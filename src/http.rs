use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::future::Future;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use poise::serenity_prelude::{async_trait, prelude::TypeMapKey};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Client as HttpClient, StatusCode};
use serde::Deserialize;
use songbird::input::metadata::YoutubeDlOutput;
use songbird::input::{
    AudioStream, AudioStreamError, AuxMetadata, Compose, HlsRequest, HttpRequest, Input,
};
use symphonia::core::io::MediaSource;
use tokio::process::Command;
use tracing::warn;

use crate::audio_cache::{video_id, AudioCache};

/// The client music sources stream through (see [`media_client`]).
pub struct HttpKey;

impl TypeMapKey for HttpKey {
    type Value = HttpClient;
}

/// `MEDIA_PROXY`: an HTTP proxy for everything that fetches music, both yt-dlp and the
/// audio stream (YouTube ties stream URLs to the IP that resolved them). Use it when
/// YouTube blocks the server's IP. Lookups such as `weather` do not use it.
static MEDIA_PROXY: LazyLock<Option<String>> =
    LazyLock::new(|| std::env::var("MEDIA_PROXY").ok().filter(|v| !v.is_empty()));

/// Connecting to any server may take this long (streams have no overall timeout).
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// The HTTP client for lookups such as `weather`: a whole request may take this long.
pub fn lookup_client() -> reqwest::Result<HttpClient> {
    HttpClient::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(Duration::from_secs(20))
        .build()
}

/// The HTTP client for music, through `MEDIA_PROXY` when it is set.
pub fn media_client() -> reqwest::Result<HttpClient> {
    let mut builder = HttpClient::builder().connect_timeout(CONNECT_TIMEOUT);
    if let Some(proxy) = MEDIA_PROXY.as_deref() {
        builder = builder.proxy(reqwest::Proxy::all(proxy)?);
    }
    builder.build()
}

/// yt-dlp arguments that send it through `MEDIA_PROXY` (empty when unset).
pub fn ytdl_proxy_args() -> Vec<String> {
    proxy_args(MEDIA_PROXY.as_deref())
}

fn proxy_args(proxy: Option<&str>) -> Vec<String> {
    proxy
        .map(|proxy| vec!["--proxy".to_string(), proxy.to_string()])
        .unwrap_or_default()
}

/// A yt-dlp source for `query`, streamed with `client` (from [`media_client`]).
pub fn ytdl(client: HttpClient, query: impl Into<String>) -> Ytdl {
    Ytdl {
        client,
        query: query.into(),
        known: None,
        carried: None,
    }
}

/// Resolves `query`: the returned source knows its metadata ([`Ytdl::metadata`]).
///
/// A YouTube URL whose audio is cached, or a search made before, needs no yt-dlp run.
/// Otherwise yt-dlp runs once and the source keeps the stream URL for when it starts.
pub async fn lookup(
    client: HttpClient,
    query: impl Into<String>,
) -> Result<Ytdl, AudioStreamError> {
    let query = query.into();
    if let Some(metadata) = known_metadata(&query) {
        let url = metadata.source_url.clone().unwrap_or(query);
        return Ok(ytdl(client, url).with_metadata(metadata));
    }
    let mut src = ytdl(client, query.clone());
    let metadata = src.aux_metadata().await?;
    if query.starts_with("ytsearch:") && metadata.source_url.is_some() {
        let mut searches = SEARCHES.lock().unwrap();
        if searches.len() >= MAX_SEARCHES {
            searches.clear();
        }
        searches.insert(query, metadata);
    }
    Ok(src)
}

/// Results of earlier `ytsearch:` queries, so the same search is not run again.
static SEARCHES: LazyLock<Mutex<HashMap<String, AuxMetadata>>> = LazyLock::new(Default::default);

/// The most searches [`SEARCHES`] keeps; it starts over when full.
const MAX_SEARCHES: usize = 1000;

/// Metadata for `query` without yt-dlp: from an earlier search, or saved with the
/// cached audio of a YouTube URL.
fn known_metadata(query: &str) -> Option<AuxMetadata> {
    if let Some(found) = SEARCHES.lock().unwrap().get(query) {
        return Some(found.clone());
    }
    let cache = crate::audio_cache::global()?;
    let mut metadata = cache.metadata(&video_id(query)?)?;
    metadata.source_url.get_or_insert_with(|| query.to_string());
    Some(metadata)
}

/// YouTube videos being fetched by [`prefetch`].
static PREFETCHING: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(Default::default);

/// Starts downloading the track with `metadata` into the audio cache, so it starts at
/// once when it comes up. Does nothing without a cache, or if it is already there.
pub async fn prefetch(client: HttpClient, metadata: AuxMetadata) {
    let Some(cache) = crate::audio_cache::global() else {
        return;
    };
    let Some(url) = metadata.source_url.clone() else {
        return;
    };
    let Some(id) = video_id(&url) else {
        return;
    };
    if cache.contains(&id) || !PREFETCHING.lock().unwrap().insert(id.clone()) {
        return;
    }
    let mut src = ytdl(client, url).with_metadata(metadata);
    // The download goes on in the cache after the reader is dropped.
    if let Err(e) = src.start(Some(cache)).await {
        warn!("Could not prefetch {id}: {e}");
    }
    PREFETCHING.lock().unwrap().remove(&id);
}

/// A yt-dlp source, like songbird's [`YoutubeDl`](songbird::input::YoutubeDl) but run once per track.
///
/// The `yt-dlp -j` output gives both the metadata and the stream URL; the URL is kept
/// and used when the track starts, unless it is about to expire.
///
/// YouTube hands out some stream URLs a few seconds before it will serve them
/// (yt-dlp's `available_at`), and fetching one early gets 403 Forbidden. With
/// `WAIT_FOR_STREAM` the fetch waits until then.
#[derive(Clone)]
pub struct Ytdl {
    client: HttpClient,
    query: String,
    /// Metadata already looked up, returned instead of running yt-dlp again.
    known: Option<AuxMetadata>,
    /// Stream info from the yt-dlp run that produced `known`.
    carried: Option<Resolved>,
}

/// The fields of yt-dlp's `-j` output needed to stream the chosen format.
#[derive(Clone, Deserialize)]
struct Resolved {
    url: String,
    #[serde(default)]
    http_headers: HashMap<String, String>,
    filesize: Option<u64>,
    protocol: Option<String>,
    ext: Option<String>,
    available_at: Option<f64>,
}

/// `WAIT_FOR_STREAM=true` (or `1`): wait for `available_at` before fetching a stream.
/// Off by default; a refused fetch is retried after that wait either way.
static WAIT_FOR_STREAM: LazyLock<bool> =
    LazyLock::new(|| wait_enabled(std::env::var("WAIT_FOR_STREAM").ok().as_deref()));

fn wait_enabled(value: Option<&str>) -> bool {
    matches!(value, Some("1" | "true"))
}

/// The yt-dlp format: Opus audio, else the best audio-only, else anything.
const FORMAT: &str = "ba[acodec=opus]/ba[abr>0][vcodec=none]/best";

/// A kept stream URL is only reused while it has at least this long left.
const MIN_URL_LIFE: Duration = Duration::from_secs(60);

/// Whether `url` (with YouTube's `expire` query parameter, Unix seconds) is still
/// valid for more than [`MIN_URL_LIFE`].
fn url_fresh(url: &str, now: SystemTime) -> bool {
    let Ok(url) = url::Url::parse(url) else {
        return false;
    };
    let Some(expire) = url
        .query_pairs()
        .find(|(k, _)| k == "expire")
        .and_then(|(_, v)| v.parse::<u64>().ok())
    else {
        return false;
    };
    let now = now.duration_since(UNIX_EPOCH).unwrap_or_default();
    Duration::from_secs(expire) > now + MIN_URL_LIFE
}

fn headers(resolved: &Resolved) -> HeaderMap {
    resolved
        .http_headers
        .iter()
        .filter_map(|(k, v)| {
            Some((
                HeaderName::from_bytes(k.as_bytes()).ok()?,
                HeaderValue::from_str(v).ok()?,
            ))
        })
        .collect()
}

fn http_request(client: &HttpClient, resolved: &Resolved) -> HttpRequest {
    HttpRequest {
        client: client.clone(),
        request: resolved.url.clone(),
        headers: headers(resolved),
        content_length: resolved.filesize,
    }
}

/// GETs the stream. On 403 (the URL is not served yet), waits for `available_at` of
/// the info `refresh` gives and tries once more.
async fn get_with_retry(
    client: &HttpClient,
    first: Resolved,
    refresh: impl Future<Output = Result<Resolved, AudioStreamError>>,
) -> Result<(Resolved, reqwest::Request, reqwest::Response), AudioStreamError> {
    let build = |resolved: &Resolved| {
        let mut req = client.get(&resolved.url).headers(headers(resolved));
        // YouTube throttles open-ended requests to about real time; a range is fast.
        if let Some(len) = resolved.filesize.filter(|len| *len > 0) {
            req = req.header(reqwest::header::RANGE, format!("bytes=0-{}", len - 1));
        }
        req.build().map_err(fail)
    };
    let send = |req: &reqwest::Request| {
        let req = req
            .try_clone()
            .expect("GET requests have no streaming body");
        client.execute(req)
    };
    let mut resolved = first;
    let mut req = build(&resolved)?;
    let mut resp = send(&req).await.map_err(fail)?;
    if resp.status() == StatusCode::FORBIDDEN {
        warn!("Stream refused (403), retrying once");
        resolved = refresh.await?;
        tokio::time::sleep(wait_for(resolved.available_at, SystemTime::now())).await;
        req = build(&resolved)?;
        resp = send(&req).await.map_err(fail)?;
    }
    if !resp.status().is_success() {
        return Err(fail(format!(
            "failed with http status code: {}",
            resp.status()
        )));
    }
    Ok((resolved, req, resp))
}

/// Metadata and stream info from one line of `yt-dlp -j` output.
fn parse_line(line: &str) -> Result<(AuxMetadata, Resolved), AudioStreamError> {
    let read = |e| fail(format!("could not read yt-dlp output: {e}"));
    let output: YoutubeDlOutput = serde_json::from_str(line).map_err(read)?;
    let resolved: Resolved = serde_json::from_str(line).map_err(read)?;
    Ok((output.as_aux_metadata(), resolved))
}

/// The longest wait for `available_at`, in case a value is far off.
const MAX_WAIT: Duration = Duration::from_secs(30);

/// Extra time after `available_at`: fetching right at it can still be refused.
const MARGIN: Duration = Duration::from_secs(1);

/// How long until `available_at` (Unix seconds), given the current time.
fn wait_for(available_at: Option<f64>, now: SystemTime) -> Duration {
    let Some(at) = available_at.filter(|at| at.is_finite() && *at > 0.0) else {
        return Duration::ZERO;
    };
    let now = now
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64();
    if at <= now {
        return Duration::ZERO;
    }
    (Duration::from_secs_f64(at - now) + MARGIN).min(MAX_WAIT)
}

fn fail(msg: impl Into<Box<dyn Error + Send + Sync>>) -> AudioStreamError {
    AudioStreamError::Fail(msg.into())
}

impl Ytdl {
    /// Uses `metadata` for [`Compose::aux_metadata`], e.g. when songbird asks for the
    /// duration on enqueue, so yt-dlp is not run just to look it up again.
    pub fn with_metadata(mut self, metadata: AuxMetadata) -> Self {
        self.known = Some(metadata);
        self
    }

    /// The metadata, when it is already known (always, for a [`lookup`] result).
    pub fn metadata(&self) -> Option<&AuxMetadata> {
        self.known.as_ref()
    }

    async fn resolve(&self) -> Result<(AuxMetadata, Resolved), AudioStreamError> {
        let output = Command::new("yt-dlp")
            .args(ytdl_proxy_args())
            // Opus first: songbird sends Opus to Discord as is instead of decoding
            // and re-encoding it (one track at 100% volume). Then songbird's choice.
            .args(["-j", "-f", FORMAT, "--no-playlist", "--"])
            .arg(&self.query)
            .output()
            .await
            .map_err(|e| fail(format!("could not run yt-dlp: {e}")))?;
        if !output.status.success() {
            return Err(fail(format!(
                "yt-dlp failed with non-zero status code: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let line = stdout
            .lines()
            .next()
            .ok_or_else(|| fail(format!("no results found for '{}'", self.query)))?;
        parse_line(line)
    }

    /// Starts the stream: from `cache` when the video is there (or downloading),
    /// otherwise from YouTube, downloading into `cache` while it plays.
    async fn start(
        &mut self,
        cache: Option<Arc<AudioCache>>,
    ) -> Result<AudioStream<Box<dyn MediaSource>>, AudioStreamError> {
        let id = self
            .known
            .as_ref()
            .and_then(|m| m.source_url.as_deref())
            .and_then(video_id)
            .or_else(|| video_id(&self.query));
        let cache = cache.zip(id);
        if let Some((cache, id)) = &cache {
            if let Some(input) = cache.get(id) {
                return Ok(AudioStream { input });
            }
        }

        let carried = self
            .carried
            .as_ref()
            .is_some_and(|c| url_fresh(&c.url, SystemTime::now()));
        let resolved = self.stream_info().await?;
        if *WAIT_FOR_STREAM {
            tokio::time::sleep(wait_for(resolved.available_at, SystemTime::now())).await;
        }

        if resolved.protocol.as_deref() == Some("m3u8_native") {
            let headers = headers(&resolved);
            return HlsRequest::new_with_headers(self.client.clone(), resolved.url, headers)
                .create();
        }

        // A refused kept URL is replaced by a fresh one; a fresh one is retried as is.
        let this = &*self;
        let refresh = |resolved: &Resolved| {
            let resolved = resolved.clone();
            async move {
                if carried {
                    Ok(this.resolve().await?.1)
                } else {
                    Ok(resolved)
                }
            }
        };

        let Some((cache, id)) = cache else {
            return match http_request(&self.client, &resolved).create_async().await {
                Ok(stream) => Ok(stream),
                Err(e) => {
                    warn!("Stream fetch failed, retrying once: {e}");
                    let next = refresh(&resolved).await?;
                    tokio::time::sleep(wait_for(next.available_at, SystemTime::now())).await;
                    http_request(&self.client, &next).create_async().await
                }
            };
        };

        let (resolved, request, resp) =
            get_with_retry(&self.client, resolved.clone(), refresh(&resolved)).await?;
        let ext = resolved
            .ext
            .clone()
            .filter(|e| !e.is_empty() && e.bytes().all(|b| b.is_ascii_alphanumeric()))
            .unwrap_or_else(|| "bin".to_string());
        let metadata = self.known.clone();
        match cache.download(&id, &ext, metadata, self.client.clone(), request, resp) {
            Ok(input) => Ok(AudioStream { input }),
            Err(e) => {
                // E.g. a full disk: play without caching rather than not at all.
                warn!("Could not cache {id}, streaming instead: {e}");
                http_request(&self.client, &resolved).create_async().await
            }
        }
    }

    /// The kept stream info if it is still fresh, otherwise a new yt-dlp run.
    async fn stream_info(&mut self) -> Result<Resolved, AudioStreamError> {
        match self.carried.take() {
            Some(carried) if url_fresh(&carried.url, SystemTime::now()) => Ok(carried),
            _ => Ok(self.resolve().await?.1),
        }
    }
}

#[async_trait]
impl Compose for Ytdl {
    fn create(&mut self) -> Result<AudioStream<Box<dyn MediaSource>>, AudioStreamError> {
        Err(AudioStreamError::Unsupported)
    }

    async fn create_async(
        &mut self,
    ) -> Result<AudioStream<Box<dyn MediaSource>>, AudioStreamError> {
        self.start(crate::audio_cache::global()).await
    }

    fn should_create_async(&self) -> bool {
        true
    }

    async fn aux_metadata(&mut self) -> Result<AuxMetadata, AudioStreamError> {
        if let Some(known) = &self.known {
            return Ok(known.clone());
        }
        let (metadata, resolved) = self.resolve().await?;
        self.known = Some(metadata.clone());
        self.carried = Some(resolved);
        Ok(metadata)
    }
}

impl From<Ytdl> for Input {
    fn from(source: Ytdl) -> Self {
        Input::Lazy(Box::new(source))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        get_with_retry, parse_line, proxy_args, url_fresh, wait_enabled, wait_for, ytdl, Resolved,
        Ytdl, MAX_WAIT,
    };
    use crate::audio_cache::AudioCache;
    use songbird::input::{AuxMetadata, Compose};
    use std::io::Read;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const SOURCE: &str = "https://www.youtube.com/watch?v=dQw4w9WgXcQ";

    fn resolved(url: String) -> Resolved {
        Resolved {
            url,
            http_headers: Default::default(),
            filesize: None,
            protocol: Some("https".into()),
            ext: Some("webm".into()),
            available_at: None,
        }
    }

    /// A looked-up source whose kept stream URL points at `server`.
    fn looked_up(server: &MockServer) -> Ytdl {
        let mut src = ytdl(reqwest::Client::new(), SOURCE).with_metadata(AuxMetadata {
            source_url: Some(SOURCE.into()),
            ..AuxMetadata::default()
        });
        src.carried = Some(resolved(format!("{}/s?expire=99999999999", server.uri())));
        src
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "lyra-http-test-{name}-{}-{:?}",
            std::process::id(),
            SystemTime::now()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    async fn read_stream(mut src: Ytdl, cache: std::sync::Arc<AudioCache>) -> Vec<u8> {
        let mut stream = src.start(Some(cache)).await.unwrap();
        tokio::task::spawn_blocking(move || {
            let mut out = Vec::new();
            stream.input.read_to_end(&mut out).unwrap();
            out
        })
        .await
        .unwrap()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn cached_track_starts_without_network() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&server)
            .await;
        let dir = temp_dir("cached");
        std::fs::write(dir.join("dQw4w9WgXcQ.webm"), b"cached audio").unwrap();
        let cache = AudioCache::open(&dir, 1 << 20).unwrap();

        assert_eq!(
            read_stream(looked_up(&server), cache).await,
            b"cached audio"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn second_start_attaches_to_running_download() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![7u8; 256 * 1024]))
            .expect(1)
            .mount(&server)
            .await;
        let cache = AudioCache::open(temp_dir("attach"), 1 << 20).unwrap();
        let src = looked_up(&server);

        let mut second = src.clone();
        let first = read_stream(src, cache.clone());
        let second_stream = second.start(Some(cache.clone())).await;
        assert!(second_stream.is_ok());
        assert_eq!(first.await.len(), 256 * 1024);
    }

    #[tokio::test]
    async fn refused_url_is_retried_once() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(403))
            .up_to_n_times(1)
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"ok".to_vec()))
            .expect(1)
            .mount(&server)
            .await;
        let client = reqwest::Client::new();
        let first = resolved(server.uri());
        let refresh = first.clone();

        let (_, _, resp) = get_with_retry(&client, first, async { Ok(refresh) })
            .await
            .unwrap();
        assert_eq!(resp.bytes().await.unwrap().as_ref(), b"ok");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn unwritable_cache_falls_back_to_streaming() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"streamed".to_vec()))
            .mount(&server)
            .await;
        let dir = temp_dir("unwritable");
        let cache = AudioCache::open(&dir, 1 << 20).unwrap();
        // Like a full disk: the cache can no longer create files.
        std::fs::remove_dir_all(&dir).unwrap();

        assert_eq!(read_stream(looked_up(&server), cache).await, b"streamed");
    }

    #[tokio::test]
    async fn known_size_is_requested_as_a_range() {
        // YouTube throttles open-ended requests to about real time; ranged ones are fast.
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(wiremock::matchers::header("range", "bytes=0-9"))
            .respond_with(ResponseTemplate::new(206).set_body_bytes(vec![1u8; 10]))
            .expect(1)
            .mount(&server)
            .await;
        let mut first = resolved(server.uri());
        first.filesize = Some(10);
        let refresh = first.clone();

        let (_, _, resp) = get_with_retry(&reqwest::Client::new(), first, async { Ok(refresh) })
            .await
            .unwrap();
        assert_eq!(resp.status(), 206);
    }

    #[tokio::test]
    async fn refused_twice_fails() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(403))
            .expect(2)
            .mount(&server)
            .await;
        let client = reqwest::Client::new();
        let first = resolved(server.uri());
        let refresh = first.clone();

        assert!(get_with_retry(&client, first, async { Ok(refresh) })
            .await
            .is_err());
    }

    #[test]
    fn resolve_json_yields_metadata_and_stream() {
        let line = r#"{"url":"https://a/b?expire=2000","webpage_url":"https://www.youtube.com/watch?v=abc","title":"T","uploader":"U","duration":61.0,"ext":"webm","protocol":"https","filesize":10}"#;
        let (metadata, resolved) = parse_line(line).unwrap();
        assert_eq!(metadata.title.as_deref(), Some("T"));
        assert_eq!(metadata.artist.as_deref(), Some("U"));
        assert_eq!(metadata.duration, Some(Duration::from_secs(61)));
        assert_eq!(
            metadata.source_url.as_deref(),
            Some("https://www.youtube.com/watch?v=abc")
        );
        assert_eq!(resolved.url, "https://a/b?expire=2000");
        assert_eq!(resolved.ext.as_deref(), Some("webm"));
        assert_eq!(resolved.filesize, Some(10));
    }

    #[test]
    fn url_fresh_uses_expire() {
        let now = UNIX_EPOCH + Duration::from_secs(1_000);
        assert!(url_fresh("https://a/b?x=1&expire=1061", now));
        assert!(!url_fresh("https://a/b?expire=1060", now));
        assert!(!url_fresh("https://a/b", now));
        assert!(!url_fresh("not a url", now));
    }

    #[test]
    fn wait_flag_off_by_default() {
        assert!(!wait_enabled(None));
        assert!(!wait_enabled(Some("")));
        assert!(!wait_enabled(Some("false")));
        assert!(wait_enabled(Some("true")));
        assert!(wait_enabled(Some("1")));
    }

    #[tokio::test]
    async fn known_metadata_is_returned_without_running_yt_dlp() {
        let known = AuxMetadata {
            title: Some("Known".into()),
            ..AuxMetadata::default()
        };
        // yt-dlp would fail on this query, so Ok means it was not run.
        let mut src = ytdl(reqwest::Client::new(), "not a url").with_metadata(known);
        let metadata = src.aux_metadata().await.unwrap();
        assert_eq!(metadata.title.as_deref(), Some("Known"));
    }

    #[test]
    fn waits_until_available_at() {
        let now = UNIX_EPOCH + Duration::from_secs(1_000);
        assert_eq!(wait_for(None, now), Duration::ZERO);
        assert_eq!(wait_for(Some(990.0), now), Duration::ZERO);
        assert_eq!(wait_for(Some(1_004.5), now), Duration::from_millis(5_500));
        assert_eq!(wait_for(Some(1e12), now), MAX_WAIT);
        assert_eq!(wait_for(Some(f64::NAN), now), Duration::ZERO);
    }

    #[test]
    fn proxy_args_only_when_set() {
        assert!(proxy_args(None).is_empty());
        assert_eq!(
            proxy_args(Some("http://10.0.0.1:8888")),
            ["--proxy", "http://10.0.0.1:8888"]
        );
    }
}
