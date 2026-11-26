//! A disk cache of track audio, keyed by YouTube video ID (`AUDIO_CACHE_SIZE`).
//!
//! A track is downloaded once into `<id>.<ext>.part` and played from that file while
//! it grows, so playback starts on the first bytes. When the download completes it is
//! renamed to `<id>.<ext>`; later plays read it from disk. The least recently played
//! files are deleted when the total size goes over the limit.
//!
//! Next to each file, `<id>.json` keeps the track's metadata, so a cached track can
//! be enqueued and played without running yt-dlp at all.

use std::collections::HashMap;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, LazyLock, Mutex};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use songbird::input::AuxMetadata;
use symphonia::core::io::MediaSource;
use tokio::io::AsyncWriteExt;
use tracing::warn;

/// The default `AUDIO_CACHE_SIZE`: 1 GiB.
const DEFAULT_SIZE: u64 = 1024 * 1024 * 1024;

/// `AUDIO_CACHE_SIZE`: bytes, or with a `K`/`M`/`G` suffix (binary units); `0` turns
/// the cache off. Unset or invalid values give 1 GiB.
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
            warn!("Invalid AUDIO_CACHE_SIZE {value:?}; using 1G");
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
    /// A download that receives nothing for this long is treated as dropped.
    stall: Duration,
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

/// The metadata kept in `<id>.json` (songbird's [`AuxMetadata`] is not serializable).
#[derive(Serialize, Deserialize)]
struct StoredMetadata {
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    date: Option<String>,
    channel: Option<String>,
    duration: Option<f64>,
    source_url: Option<String>,
    thumbnail: Option<String>,
}

impl From<&AuxMetadata> for StoredMetadata {
    fn from(m: &AuxMetadata) -> Self {
        StoredMetadata {
            title: m.title.clone(),
            artist: m.artist.clone(),
            album: m.album.clone(),
            date: m.date.clone(),
            channel: m.channel.clone(),
            duration: m.duration.map(|d| d.as_secs_f64()),
            source_url: m.source_url.clone(),
            thumbnail: m.thumbnail.clone(),
        }
    }
}

impl From<StoredMetadata> for AuxMetadata {
    fn from(m: StoredMetadata) -> Self {
        AuxMetadata {
            title: m.title,
            artist: m.artist,
            album: m.album,
            date: m.date,
            channel: m.channel,
            duration: m
                .duration
                .filter(|d| d.is_finite() && *d >= 0.0)
                .map(Duration::from_secs_f64),
            source_url: m.source_url,
            thumbnail: m.thumbnail,
            ..AuxMetadata::default()
        }
    }
}

/// A download in progress, shared by its writer task and the readers playing it.
struct Download {
    part: PathBuf,
    len: Option<u64>,
    /// Written to `<id>.json` once the download is kept.
    metadata: Option<AuxMetadata>,
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
        Self::open_with(dir, limit, STALL_TIMEOUT)
    }

    fn open_with(dir: impl AsRef<Path>, limit: u64, stall: Duration) -> io::Result<Arc<Self>> {
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
            if name.ends_with(".json") {
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
        // Metadata whose audio is gone (e.g. deleted by hand).
        for entry in std::fs::read_dir(&dir)?.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(id) = name.strip_suffix(".json") {
                if !state.files.contains_key(id) {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
        let cache = Arc::new(AudioCache {
            dir,
            limit,
            stall,
            state: Mutex::new(state),
        });
        cache.evict(&mut cache.state.lock().unwrap());
        Ok(cache)
    }

    /// The audio for `id`: the cached file, or the file still being downloaded.
    pub fn get(&self, id: &str) -> Option<Box<dyn MediaSource>> {
        Self::existing(&mut self.state.lock().unwrap(), id)
    }

    /// Whether `id` is cached or downloading.
    pub fn contains(&self, id: &str) -> bool {
        let state = self.state.lock().unwrap();
        state.files.contains_key(id) || state.downloads.contains_key(id)
    }

    /// The metadata saved with `id`, if its audio is cached.
    pub fn metadata(&self, id: &str) -> Option<AuxMetadata> {
        if !self.state.lock().unwrap().files.contains_key(id) {
            return None;
        }
        let bytes = std::fs::read(self.metadata_path(id)).ok()?;
        let stored: StoredMetadata = serde_json::from_slice(&bytes).ok()?;
        Some(stored.into())
    }

    fn metadata_path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.json"))
    }

    fn existing(state: &mut State, id: &str) -> Option<Box<dyn MediaSource>> {
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

    /// Downloads `resp` (the answer to `request`) into the cache as `id`, returning a
    /// reader that plays it while it downloads. The download goes on if the reader is
    /// dropped, and resumes with a range of `request` if the connection drops. If `id`
    /// was cached or started meanwhile, `resp` is dropped and that is returned instead.
    /// `metadata` is saved with the file when it is kept.
    pub fn download(
        self: &Arc<Self>,
        id: &str,
        ext: &str,
        metadata: Option<AuxMetadata>,
        client: reqwest::Client,
        request: reqwest::Request,
        resp: reqwest::Response,
    ) -> io::Result<Box<dyn MediaSource>> {
        let mut state = self.state.lock().unwrap();
        if let Some(existing) = Self::existing(&mut state, id) {
            return Ok(existing);
        }
        let part = self.dir.join(format!("{id}.{ext}.part"));
        let file = std::fs::File::create(&part)?;
        let download = Arc::new(Download {
            part,
            len: resp.content_length(),
            metadata,
            progress: Mutex::default(),
            changed: Condvar::new(),
        });
        let reader = GrowingFile::open(download.clone())?;
        state.downloads.insert(id.to_string(), download.clone());
        drop(state);

        let cache = self.clone();
        let id = id.to_string();
        let path = self.dir.join(format!("{id}.{ext}"));
        tokio::spawn(async move {
            let body = Body {
                client,
                request,
                stall: cache.stall,
            };
            let result = body
                .write(resp, tokio::fs::File::from_std(file), &download)
                .await;
            cache.finish(&id, &path, &download, result);
        });
        Ok(Box::new(reader))
    }

    fn finish(&self, id: &str, path: &Path, download: &Arc<Download>, result: Result<(), String>) {
        let mut state = self.state.lock().unwrap();
        if state
            .downloads
            .get(id)
            .is_some_and(|d| Arc::ptr_eq(d, download))
        {
            state.downloads.remove(id);
        }
        let size = download.progress.lock().unwrap().written;
        let result = result.and_then(|()| {
            if size > self.limit {
                // Too big to keep; open readers still have the file.
                let _ = std::fs::remove_file(&download.part);
                return Ok(());
            }
            std::fs::rename(&download.part, path).map_err(|e| e.to_string())?;
            if let Some(metadata) = &download.metadata {
                let json = serde_json::to_vec(&StoredMetadata::from(metadata))
                    .expect("metadata serializes");
                if let Err(e) = std::fs::write(self.metadata_path(id), json) {
                    warn!("Could not save metadata of {id}: {e}");
                }
            }
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
            let _ = std::fs::remove_file(self.metadata_path(&oldest));
            total -= file.size;
        }
    }
}

/// How long a download may receive nothing before it is resumed (or failed).
const STALL_TIMEOUT: Duration = Duration::from_secs(30);

/// How many times a dropped or stalled download is resumed before it fails.
const RESUMES: usize = 3;

/// What a download needs to resume where it stopped.
struct Body {
    client: reqwest::Client,
    request: reqwest::Request,
    stall: Duration,
}

impl Body {
    async fn write(
        &self,
        mut resp: reqwest::Response,
        mut file: tokio::fs::File,
        download: &Download,
    ) -> Result<(), String> {
        let mut resumes = 0;
        loop {
            let interrupted = loop {
                let chunk = match tokio::time::timeout(self.stall, resp.chunk()).await {
                    Err(_) => break format!("nothing received for {:?}", self.stall),
                    Ok(Err(e)) => break e.to_string(),
                    Ok(Ok(chunk)) => chunk,
                };
                let Some(chunk) = chunk else {
                    let written = download.progress.lock().unwrap().written;
                    match download.len {
                        Some(len) if written < len => {
                            break format!("ended after {written} of {len} bytes")
                        }
                        _ => return Ok(()),
                    }
                };
                file.write_all(&chunk).await.map_err(|e| e.to_string())?;
                // Readers use their own handle, so the bytes must reach the file first.
                file.flush().await.map_err(|e| e.to_string())?;
                download.update(|p| p.written += chunk.len() as u64);
            };
            resp = loop {
                if resumes == RESUMES {
                    return Err(interrupted);
                }
                resumes += 1;
                warn!("Download interrupted ({interrupted}); resuming");
                if let Some(resp) = self.resume(download).await {
                    break resp;
                }
            };
        }
    }

    /// The rest of the body, from the bytes written so far.
    async fn resume(&self, download: &Download) -> Option<reqwest::Response> {
        let written = download.progress.lock().unwrap().written;
        let end = download
            .len
            .map(|len| (len - 1).to_string())
            .unwrap_or_default();
        let mut request = self.request.try_clone()?;
        let range = format!("bytes={written}-{end}").parse().ok()?;
        request.headers_mut().insert(reqwest::header::RANGE, range);
        let resp = tokio::time::timeout(self.stall, self.client.execute(request))
            .await
            .ok()?
            .ok()?;
        (resp.status() == reqwest::StatusCode::PARTIAL_CONTENT).then_some(resp)
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

    /// A response from `url`, with the client and request a download resumes with.
    struct Fetched {
        client: reqwest::Client,
        request: reqwest::Request,
        resp: reqwest::Response,
    }

    async fn fetch(url: &str) -> Fetched {
        let client = reqwest::Client::new();
        let request = client.get(url).build().unwrap();
        let resp = client.execute(request.try_clone().unwrap()).await.unwrap();
        Fetched {
            client,
            request,
            resp,
        }
    }

    fn download(cache: &Arc<AudioCache>, f: Fetched) -> Box<dyn MediaSource> {
        cache
            .download(ID, "webm", None, f.client, f.request, f.resp)
            .unwrap()
    }

    async fn serve(bytes: Vec<u8>) -> (MockServer, Fetched) {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(bytes))
            .mount(&server)
            .await;
        let fetched = fetch(&server.uri()).await;
        (server, fetched)
    }

    /// A raw HTTP server: connection `n` gets `respond(n, request)` written in parts,
    /// waiting `pause` between them, then is held open for `hold`.
    async fn raw_server(
        respond: impl Fn(usize, &str) -> Vec<Vec<u8>> + Send + Sync + 'static,
        pause: Duration,
        hold: Duration,
    ) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let respond = Arc::new(respond);
        tokio::spawn(async move {
            let mut n = 0;
            while let Ok((mut sock, _)) = listener.accept().await {
                let respond = respond.clone();
                n += 1;
                let conn = n;
                tokio::spawn(async move {
                    let mut req = [0u8; 4096];
                    let len = tokio::io::AsyncReadExt::read(&mut sock, &mut req)
                        .await
                        .unwrap_or(0);
                    let req = String::from_utf8_lossy(&req[..len]).to_lowercase();
                    for part in respond(conn, &req) {
                        if sock.write_all(&part).await.is_err() {
                            return;
                        }
                        tokio::time::sleep(pause).await;
                    }
                    tokio::time::sleep(hold).await;
                });
            }
        });
        format!("http://{addr}/")
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
        assert_eq!(parse_size(None), 1_073_741_824);
        assert_eq!(parse_size(Some("0")), 0);
        assert_eq!(parse_size(Some("1024")), 1024);
        assert_eq!(parse_size(Some("2K")), 2048);
        assert_eq!(parse_size(Some("512M")), 536_870_912);
        assert_eq!(parse_size(Some("1G")), 1_073_741_824);
        assert_eq!(parse_size(Some("junk")), 1_073_741_824);
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
        let (_server, fetched) = serve(expected.clone()).await;

        let src = download(&cache, fetched);
        assert_eq!(read_all(src).await.unwrap(), expected);

        wait_until(|| dir.join(format!("{ID}.webm")).exists()).await;
        let cached = cache.get(ID).expect("cached after download");
        assert_eq!(read_all(cached).await.unwrap(), expected);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn metadata_is_kept_with_the_file() {
        let dir = temp_dir("metadata");
        let cache = AudioCache::open(&dir, 1 << 20).unwrap();
        let (_server, f) = serve(body(100)).await;
        let metadata = songbird::input::AuxMetadata {
            title: Some("Never Gonna Give You Up".into()),
            duration: Some(Duration::from_secs(213)),
            source_url: Some(format!("https://www.youtube.com/watch?v={ID}")),
            ..Default::default()
        };
        assert!(cache.metadata(ID).is_none());
        let src = cache
            .download(
                ID,
                "webm",
                Some(metadata.clone()),
                f.client,
                f.request,
                f.resp,
            )
            .unwrap();
        assert!(cache.contains(ID));
        read_all(src).await.unwrap();
        wait_until(|| cache.metadata(ID).is_some()).await;

        // Indexed again after a restart, but not as a second audio file.
        let reopened = AudioCache::open(&dir, 1 << 20).unwrap();
        assert_eq!(reopened.metadata(ID), Some(metadata));
        assert_eq!(
            read_all(reopened.get(ID).unwrap()).await.unwrap(),
            body(100)
        );
    }

    #[test]
    fn orphaned_metadata_is_removed() {
        let dir = temp_dir("orphan");
        std::fs::write(dir.join(format!("{ID}.json")), b"{}").unwrap();
        let cache = AudioCache::open(&dir, 1 << 20).unwrap();
        assert!(!dir.join(format!("{ID}.json")).exists());
        assert!(!cache.contains(ID));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn download_completes_after_reader_dropped() {
        let dir = temp_dir("dropped");
        let cache = AudioCache::open(&dir, 1 << 20).unwrap();
        let (_server, fetched) = serve(body(32 * 1024)).await;

        let mut src = download(&cache, fetched);
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
        let fetched = fetch(&format!("http://{addr}/")).await;

        let src = download(&cache, fetched);
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
        let (_server, fetched) = serve(body(100)).await;
        read_all(download(&cache, fetched)).await.unwrap();

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
        let (_server, fetched) = serve(expected.clone()).await;

        let src = download(&cache, fetched);
        assert_eq!(read_all(src).await.unwrap(), expected);

        wait_until(|| !dir.join(format!("{ID}.webm.part")).exists()).await;
        assert!(!dir.join(format!("{ID}.webm")).exists());
        assert!(cache.get(ID).is_none());
        assert!(dir.join("aaaaaaaaaaa.webm").exists(), "others not evicted");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn second_download_of_a_video_attaches_to_the_first() {
        let dir = temp_dir("twice");
        let cache = AudioCache::open(&dir, 1 << 20).unwrap();
        let expected = body(100);
        let slow = expected.clone();
        let url = raw_server(
            move |_, _| {
                let mut parts = vec![b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\n".to_vec()];
                parts.extend(slow.chunks(10).map(<[u8]>::to_vec));
                parts
            },
            Duration::from_millis(30),
            Duration::ZERO,
        )
        .await;
        let first = download(&cache, fetch(&url).await);
        let (_server, other) = serve(vec![9u8; 50]).await;
        let second = download(&cache, other);

        assert_eq!(read_all(first).await.unwrap(), expected);
        assert_eq!(read_all(second).await.unwrap(), expected);
        wait_until(|| dir.join(format!("{ID}.webm")).exists()).await;
        assert_eq!(
            std::fs::read(dir.join(format!("{ID}.webm"))).unwrap(),
            expected
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn stalled_download_fails_instead_of_hanging() {
        let dir = temp_dir("stall");
        let cache = AudioCache::open_with(&dir, 1 << 20, Duration::from_millis(200)).unwrap();
        // Every connection sends 10 of 100 bytes, then goes quiet.
        let url = raw_server(
            |_, _| vec![b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\n0123456789".to_vec()],
            Duration::ZERO,
            Duration::from_secs(60),
        )
        .await;
        let src = download(&cache, fetch(&url).await);

        let read = tokio::time::timeout(Duration::from_secs(5), read_all(src)).await;
        assert!(read.expect("read did not hang").is_err());
        wait_until(|| cache.get(ID).is_none()).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn dropped_connection_resumes_from_written_bytes() {
        let dir = temp_dir("resume");
        let cache = AudioCache::open(&dir, 1 << 20).unwrap();
        let expected = body(100);
        let rest = expected[10..].to_vec();
        let start = expected[..10].to_vec();
        let url = raw_server(
            move |conn, req| match conn {
                1 => vec![
                    b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\n".to_vec(),
                    start.clone(),
                ],
                _ if req.contains("range: bytes=10-99") => vec![
                    b"HTTP/1.1 206 Partial Content\r\nContent-Length: 90\r\n\r\n".to_vec(),
                    rest.clone(),
                ],
                _ => vec![
                    b"HTTP/1.1 416 Range Not Satisfiable\r\nContent-Length: 0\r\n\r\n".to_vec(),
                ],
            },
            Duration::ZERO,
            Duration::ZERO,
        )
        .await;
        let src = download(&cache, fetch(&url).await);

        assert_eq!(read_all(src).await.unwrap(), expected);
        wait_until(|| dir.join(format!("{ID}.webm")).exists()).await;
    }
}
