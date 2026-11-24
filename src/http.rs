use std::collections::HashMap;
use std::error::Error;
use std::sync::LazyLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use poise::serenity_prelude::{async_trait, prelude::TypeMapKey};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::Client as HttpClient;
use serde::Deserialize;
use songbird::input::metadata::YoutubeDlOutput;
use songbird::input::{
    AudioStream, AudioStreamError, AuxMetadata, Compose, HlsRequest, HttpRequest, Input,
};
use symphonia::core::io::MediaSource;
use tokio::process::Command;

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

/// The HTTP client for music, through `MEDIA_PROXY` when it is set.
pub fn media_client() -> reqwest::Result<HttpClient> {
    let mut builder = HttpClient::builder();
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

/// Resolves `query` with one yt-dlp run: the returned source knows its metadata
/// ([`Ytdl::metadata`]) and keeps the stream URL for when it starts playing.
pub async fn lookup(
    client: HttpClient,
    query: impl Into<String>,
) -> Result<Ytdl, AudioStreamError> {
    let mut src = ytdl(client, query);
    src.aux_metadata().await?;
    Ok(src)
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
            // The same format choice as songbird's YoutubeDl.
            .args([
                "-j",
                "-f",
                "ba[abr>0][vcodec=none]/best",
                "--no-playlist",
                "--",
            ])
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
        let resolved = self.stream_info().await?;
        if *WAIT_FOR_STREAM {
            tokio::time::sleep(wait_for(resolved.available_at, SystemTime::now())).await;
        }

        let headers = resolved
            .http_headers
            .iter()
            .filter_map(|(k, v)| {
                Some((
                    HeaderName::from_bytes(k.as_bytes()).ok()?,
                    HeaderValue::from_str(v).ok()?,
                ))
            })
            .collect::<HeaderMap>();

        if resolved.protocol.as_deref() == Some("m3u8_native") {
            HlsRequest::new_with_headers(self.client.clone(), resolved.url, headers).create()
        } else {
            HttpRequest {
                client: self.client.clone(),
                request: resolved.url,
                headers,
                content_length: resolved.filesize,
            }
            .create_async()
            .await
        }
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
    use super::{parse_line, proxy_args, url_fresh, wait_enabled, wait_for, ytdl, MAX_WAIT};
    use songbird::input::{AuxMetadata, Compose};
    use std::time::{Duration, UNIX_EPOCH};

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
