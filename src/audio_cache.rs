//! A disk cache of track audio, keyed by YouTube video ID (`AUDIO_CACHE_SIZE`).
//!
//! A track is downloaded once into `<id>.<ext>.part` and played from that file while
//! it grows, so playback starts on the first bytes. When the download completes it is
//! renamed to `<id>.<ext>`; later plays read it from disk. The least recently played
//! files are deleted when the total size goes over the limit.

use std::collections::HashMap;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, LazyLock, Mutex};
use std::time::SystemTime;

use symphonia::core::io::MediaSource;
use tokio::io::AsyncWriteExt;
use tracing::warn;

/// The default `AUDIO_CACHE_SIZE`: 512 MiB.
const DEFAULT_SIZE: u64 = 512 * 1024 * 1024;

/// `AUDIO_CACHE_SIZE`: bytes, or with a `K`/`M`/`G` suffix (binary units); `0` turns
/// the cache off. Unset or invalid values give 512 MiB.
pub fn parse_size(value: Option<&str>) -> u64 {
    let Some(value) = value.map(str::trim).filter(|v| !v.is_empty()) else {
        return DEFAULT_SIZE;
    };
    let (number, unit) = match value.as_bytes()[value.len() - 1].to_ascii_uppercase() {
        b'K' => (&value[..value.len() - 1], 1 << 10),
        b'M' => (&value[..value.len() - 1], 1 << 20),
        b'G' => (&value[..value.len() - 1], 1 << 30),
        _ => (value, 1),
    };
    match number.parse::<u64>() {
        Ok(n) => n.saturating_mul(unit),
        Err(_) => {
            warn!("Invalid AUDIO_CACHE_SIZE {value:?}; using 512M");
            DEFAULT_SIZE
        }
    }
}

/// The YouTube video ID in `url`, if it is a YouTube video URL.
pub fn video_id(url: &str) -> Option<String> {
    let url = url::Url::parse(url).ok()?;
    let host = url.host_str()?;
    let id = if host == "youtu.be" {
        url.path_segments()?.next()?.to_string()
    } else if host == "youtube.com" || host.ends_with(".youtube.com") {
        match url.query_pairs().find(|(k, _)| k == "v") {
            Some((_, v)) => v.into_owned(),
            None => {
                let mut segments = url.path_segments()?;
                match segments.next()? {
                    "shorts" | "live" | "embed" => segments.next()?.to_string(),
                    _ => return None,
                }
            }
        }
    } else {
        return None;
    };
    // IDs become file names, so only accept the real format.
    let valid = id.len() == 11
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
    valid.then_some(id)
}

static GLOBAL: LazyLock<Option<Arc<AudioCache>>> = LazyLock::new(|| {
    let limit = parse_size(std::env::var("AUDIO_CACHE_SIZE").ok().as_deref());
    if limit == 0 {
        return None;
    }
    AudioCache::open(crate::AUDIO_CACHE_DIR, limit)
        .inspect_err(|e| {
            warn!(
                "Audio cache disabled: could not open {}: {e}",
                crate::AUDIO_CACHE_DIR
            )
        })
        .ok()
});

/// The cache in [`crate::AUDIO_CACHE_DIR`], or `None` when `AUDIO_CACHE_SIZE=0`.
pub fn global() -> Option<Arc<AudioCache>> {
    GLOBAL.clone()
}

pub struct AudioCache {
    dir: PathBuf,
    limit: u64,
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    files: HashMap<String, CachedFile>,
    downloads: HashMap<String, Arc<Download>>,
}

struct CachedFile {
    path: PathBuf,
    size: u64,
    played: SystemTime,
}

/// A download in progress, shared by its writer task and the readers playing it.
struct Download {
    part: PathBuf,
    len: Option<u64>,
    progress: Mutex<Progress>,
    changed: Condvar,
}

#[derive(Default)]
struct Progress {
    written: u64,
    /// `Some` once the download has ended, with its error if it failed.
    done: Option<Result<(), String>>,
}

impl Download {
    fn update(&self, f: impl FnOnce(&mut Progress)) {
        f(&mut self.progress.lock().unwrap());
        self.changed.notify_all();
    }
}

impl AudioCache {
    /// Opens (creating) `dir`, deleting unfinished downloads and indexing the rest.
    pub fn open(dir: impl AsRef<Path>, limit: u64) -> io::Result<Arc<Self>> {
        let dir = dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&dir)?;
        let mut state = State::default();
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.ends_with(".part") {
                let _ = std::fs::remove_file(&path);
                continue;
            }
            let meta = entry.metadata()?;
            let Some((id, _)) = name.split_once('.') else {
                continue;
            };
            if !meta.is_file() {
                continue;
            }
            state.files.insert(
                id.to_string(),
                CachedFile {
                    path,
                    size: meta.len(),
                    played: meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                },
            );
        }
        let cache = Arc::new(AudioCache {
            dir,
            limit,
            state: Mutex::new(state),
        });
        cache.evict(&mut cache.state.lock().unwrap());
        Ok(cache)
    }

    /// The audio for `id`: the cached file, or the file still being downloaded.
    pub fn get(&self, id: &str) -> Option<Box<dyn MediaSource>> {
        let mut state = self.state.lock().unwrap();
        if let Some(cached) = state.files.get_mut(id) {
            match std::fs::File::open(&cached.path) {
                Ok(file) => {
                    cached.played = SystemTime::now();
                    // The mtime records the last play across restarts.
                    let _ = file.set_modified(cached.played).or_else(|_| {
                        std::fs::File::options()
                            .append(true)
                            .open(&cached.path)?
                            .set_modified(cached.played)
                    });
                    return Some(Box::new(file));
                }
                Err(_) => {
                    state.files.remove(id);
                }
            }
        }
        let download = state.downloads.get(id)?.clone();
        GrowingFile::open(download)
            .ok()
            .map(|f| Box::new(f) as Box<dyn MediaSource>)
    }

    /// Downloads `resp` into the cache as `id`, returning a reader that plays it while
    /// it downloads. The download goes on if the reader is dropped.
    pub fn download(
        self: &Arc<Self>,
        id: &str,
        ext: &str,
        resp: reqwest::Response,
    ) -> io::Result<Box<dyn MediaSource>> {
        let part = self.dir.join(format!("{id}.{ext}.part"));
        let file = std::fs::File::create(&part)?;
        let download = Arc::new(Download {
            part,
            len: resp.content_length(),
            progress: Mutex::default(),
            changed: Condvar::new(),
        });
        let reader = GrowingFile::open(download.clone())?;
        self.state
            .lock()
            .unwrap()
            .downloads
            .insert(id.to_string(), download.clone());

        let cache = self.clone();
        let id = id.to_string();
        let path = self.dir.join(format!("{id}.{ext}"));
        tokio::spawn(async move {
            let result = write_body(resp, tokio::fs::File::from_std(file), &download).await;
            cache.finish(&id, &path, &download, result);
        });
        Ok(Box::new(reader))
    }

    fn finish(&self, id: &str, path: &Path, download: &Download, result: Result<(), String>) {
        let mut state = self.state.lock().unwrap();
        state.downloads.remove(id);
        let size = download.progress.lock().unwrap().written;
        let result = result.and_then(|()| {
            if size > self.limit {
                // Too big to keep; open readers still have the file.
                let _ = std::fs::remove_file(&download.part);
                return Ok(());
            }
            std::fs::rename(&download.part, path).map_err(|e| e.to_string())?;
            state.files.insert(
                id.to_string(),
                CachedFile {
                    path: path.to_path_buf(),
                    size,
                    played: SystemTime::now(),
                },
            );
            Ok(())
        });
        if let Err(e) = &result {
            warn!("Download of {id} failed: {e}");
            let _ = std::fs::remove_file(&download.part);
        }
        self.evict(&mut state);
        drop(state);
        download.update(|p| p.done = Some(result));
    }

    /// Deletes the least recently played files until the total fits the limit.
    fn evict(&self, state: &mut State) {
        let mut total: u64 = state.files.values().map(|f| f.size).sum();
        while total > self.limit {
            let Some(oldest) = state
                .files
                .iter()
                .min_by_key(|(_, f)| f.played)
                .map(|(id, _)| id.clone())
            else {
                break;
            };
            let file = state.files.remove(&oldest).unwrap();
            let _ = std::fs::remove_file(&file.path);
            total -= file.size;
        }
    }
}

async fn write_body(
    mut resp: reqwest::Response,
    mut file: tokio::fs::File,
    download: &Download,
) -> Result<(), String> {
    while let Some(chunk) = resp.chunk().await.map_err(|e| e.to_string())? {
        file.write_all(&chunk).await.map_err(|e| e.to_string())?;
        // Readers use their own handle, so the bytes must reach the file first.
        file.flush().await.map_err(|e| e.to_string())?;
        download.update(|p| p.written += chunk.len() as u64);
    }
    let written = download.progress.lock().unwrap().written;
    match download.len {
        Some(len) if written < len => Err(format!("ended after {written} of {len} bytes")),
        _ => Ok(()),
    }
}

/// Reads a file that is still being downloaded, waiting for bytes not yet written.
struct GrowingFile {
    file: std::fs::File,
    download: Arc<Download>,
    pos: u64,
}

impl GrowingFile {
    fn open(download: Arc<Download>) -> io::Result<Self> {
        Ok(GrowingFile {
            file: std::fs::File::open(&download.part)?,
            download,
            pos: 0,
        })
    }

    /// The total length, waiting for the download to end if it is not known.
    fn total_len(&self) -> io::Result<u64> {
        if let Some(len) = self.download.len {
            return Ok(len);
        }
        let mut progress = self.download.progress.lock().unwrap();
        loop {
            match &progress.done {
                Some(Ok(())) => return Ok(progress.written),
                Some(Err(e)) => return Err(io::Error::other(e.clone())),
                None => progress = self.download.changed.wait(progress).unwrap(),
            }
        }
    }
}

impl Read for GrowingFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let available = {
            let mut progress = self.download.progress.lock().unwrap();
            loop {
                if progress.written > self.pos {
                    break progress.written - self.pos;
                }
                match &progress.done {
                    Some(Ok(())) => return Ok(0),
                    Some(Err(e)) => return Err(io::Error::other(e.clone())),
                    None => progress = self.download.changed.wait(progress).unwrap(),
                }
            }
        };
        let len = buf.len().min(available.try_into().unwrap_or(usize::MAX));
        self.file.seek(SeekFrom::Start(self.pos))?;
        let n = self.file.read(&mut buf[..len])?;
        self.pos += n as u64;
        Ok(n)
    }
}

impl Seek for GrowingFile {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let target = match pos {
            SeekFrom::Start(n) => Some(n),
            SeekFrom::Current(d) => self.pos.checked_add_signed(d),
            SeekFrom::End(d) => self.total_len()?.checked_add_signed(d),
        };
        self.pos = target
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "seek before the start"))?;
        Ok(self.pos)
    }
}

impl MediaSource for GrowingFile {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        self.download.len
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_size, video_id, AudioCache};
    use std::io::Read;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::{Duration, SystemTime};
    use symphonia::core::io::MediaSource;
    use tokio::io::AsyncWriteExt;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const ID: &str = "dQw4w9WgXcQ";

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "lyra-cache-test-{name}-{}-{:?}",
            std::process::id(),
            SystemTime::now()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn body(len: usize) -> Vec<u8> {
        (0..len).map(|i| (i % 251) as u8).collect()
    }

    async fn serve(bytes: Vec<u8>) -> (MockServer, reqwest::Response) {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(bytes))
            .mount(&server)
            .await;
        let resp = reqwest::get(server.uri()).await.unwrap();
        (server, resp)
    }

    async fn read_all(mut src: Box<dyn MediaSource>) -> std::io::Result<Vec<u8>> {
        tokio::task::spawn_blocking(move || {
            let mut out = Vec::new();
            src.read_to_end(&mut out).map(|_| out)
        })
        .await
        .unwrap()
    }

    async fn wait_until(mut cond: impl FnMut() -> bool) {
        for _ in 0..100 {
            if cond() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("condition not met within 5 s");
    }

    #[test]
    fn parse_size_units() {
        assert_eq!(parse_size(None), 536_870_912);
        assert_eq!(parse_size(Some("0")), 0);
        assert_eq!(parse_size(Some("1024")), 1024);
        assert_eq!(parse_size(Some("2K")), 2048);
        assert_eq!(parse_size(Some("512M")), 536_870_912);
        assert_eq!(parse_size(Some("1G")), 1_073_741_824);
        assert_eq!(parse_size(Some("junk")), 536_870_912);
    }

    #[test]
    fn video_id_forms() {
        for url in [
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=1",
            "https://youtu.be/dQw4w9WgXcQ",
            "https://music.youtube.com/watch?v=dQw4w9WgXcQ",
            "https://www.youtube.com/shorts/dQw4w9WgXcQ",
        ] {
            assert_eq!(video_id(url).as_deref(), Some(ID), "{url}");
        }
        assert_eq!(video_id("https://example.com/x.mp3"), None);
        assert_eq!(video_id("ytsearch:foo"), None);
        assert_eq!(video_id("https://www.youtube.com/watch?v=../../etc"), None);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn download_streams_then_serves_from_disk() {
        let dir = temp_dir("serve");
        let cache = AudioCache::open(&dir, 1 << 20).unwrap();
        let expected = body(64 * 1024);
        let (_server, resp) = serve(expected.clone()).await;

        let src = cache.download(ID, "webm", resp).unwrap();
        assert_eq!(read_all(src).await.unwrap(), expected);

        wait_until(|| dir.join(format!("{ID}.webm")).exists()).await;
        let cached = cache.get(ID).expect("cached after download");
        assert_eq!(read_all(cached).await.unwrap(), expected);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn download_completes_after_reader_dropped() {
        let dir = temp_dir("dropped");
        let cache = AudioCache::open(&dir, 1 << 20).unwrap();
        let (_server, resp) = serve(body(32 * 1024)).await;

        let mut src = cache.download(ID, "webm", resp).unwrap();
        tokio::task::spawn_blocking(move || {
            let mut one = [0u8; 1];
            src.read_exact(&mut one).unwrap();
        })
        .await
        .unwrap();

        wait_until(|| dir.join(format!("{ID}.webm")).exists()).await;
        assert!(cache.get(ID).is_some());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn failed_download_errors_reader_and_removes_part() {
        let dir = temp_dir("failed");
        let cache = AudioCache::open(&dir, 1 << 20).unwrap();

        // Promises 100 bytes, sends 10, then closes.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut req = [0u8; 1024];
            let _ = tokio::io::AsyncReadExt::read(&mut sock, &mut req).await;
            sock.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\n0123456789")
                .await
                .unwrap();
        });
        let resp = reqwest::get(format!("http://{addr}/")).await.unwrap();

        let src = cache.download(ID, "webm", resp).unwrap();
        assert!(read_all(src).await.is_err());

        wait_until(|| !dir.join(format!("{ID}.webm.part")).exists()).await;
        assert!(cache.get(ID).is_none());
        assert!(!dir.join(format!("{ID}.webm")).exists());
    }

    #[test]
    fn open_deletes_part_files_and_indexes_complete() {
        let dir = temp_dir("open");
        std::fs::write(dir.join("aaaaaaaaaaa.webm.part"), b"partial").unwrap();
        std::fs::write(dir.join(format!("{ID}.webm")), b"done").unwrap();

        let cache = AudioCache::open(&dir, 1 << 20).unwrap();
        assert!(!dir.join("aaaaaaaaaaa.webm.part").exists());
        assert!(cache.get("aaaaaaaaaaa").is_none());
        assert!(cache.get(ID).is_some());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn evicts_least_recently_played() {
        let dir = temp_dir("evict");
        let now = SystemTime::now();
        for (i, id) in ["aaaaaaaaaaa", "bbbbbbbbbbb", "ccccccccccc"]
            .iter()
            .enumerate()
        {
            let path = dir.join(format!("{id}.webm"));
            std::fs::write(&path, body(100)).unwrap();
            let file = std::fs::File::options().append(true).open(&path).unwrap();
            file.set_modified(now - Duration::from_secs(100 - i as u64 * 10))
                .unwrap();
        }
        let cache = AudioCache::open(&dir, 300).unwrap();
        let (_server, resp) = serve(body(100)).await;
        read_all(cache.download(ID, "webm", resp).unwrap())
            .await
            .unwrap();

        wait_until(|| dir.join(format!("{ID}.webm")).exists()).await;
        wait_until(|| !dir.join("aaaaaaaaaaa.webm").exists()).await;
        for id in ["bbbbbbbbbbb", "ccccccccccc", ID] {
            assert!(dir.join(format!("{id}.webm")).exists(), "{id} kept");
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn oversized_file_is_not_kept() {
        let dir = temp_dir("oversized");
        std::fs::write(dir.join("aaaaaaaaaaa.webm"), b"small").unwrap();
        let cache: Arc<AudioCache> = AudioCache::open(&dir, 10).unwrap();
        let expected = body(100);
        let (_server, resp) = serve(expected.clone()).await;

        let src = cache.download(ID, "webm", resp).unwrap();
        assert_eq!(read_all(src).await.unwrap(), expected);

        wait_until(|| !dir.join(format!("{ID}.webm.part")).exists()).await;
        assert!(!dir.join(format!("{ID}.webm")).exists());
        assert!(cache.get(ID).is_none());
        assert!(dir.join("aaaaaaaaaaa.webm").exists(), "others not evicted");
    }
}
