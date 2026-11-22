use std::collections::HashMap;
use std::error::Error;
use std::sync::LazyLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use poise::serenity_prelude::{async_trait, prelude::TypeMapKey};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::Client as HttpClient;
use serde::Deserialize;
use songbird::input::{
    AudioStream, AudioStreamError, AuxMetadata, Compose, HlsRequest, HttpRequest, Input, YoutubeDl,
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
    let query = query.into();
    Ytdl {
        metadata: YoutubeDl::new_ytdl_like("yt-dlp", client.clone(), query.clone())
            .user_args(ytdl_proxy_args()),
        client,
        query,
    }
}

/// Like songbird's [`YoutubeDl`], but it waits for the stream to become available.
///
/// YouTube now hands out some stream URLs a few seconds before it will serve them
/// (yt-dlp's `available_at`; yt-dlp itself sleeps until then), and fetching one early
/// gets 403 Forbidden. Metadata still comes from songbird's [`YoutubeDl`].
#[derive(Clone)]
pub struct Ytdl {
    client: HttpClient,
    query: String,
    metadata: YoutubeDl<'static>,
}

/// The fields of yt-dlp's `-j` output needed to stream the chosen format.
#[derive(Deserialize)]
struct Resolved {
    url: String,
    #[serde(default)]
    http_headers: HashMap<String, String>,
    filesize: Option<u64>,
    protocol: Option<String>,
    available_at: Option<f64>,
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
    async fn resolve(&self) -> Result<Resolved, AudioStreamError> {
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
        serde_json::from_str(line).map_err(|e| fail(format!("could not read yt-dlp output: {e}")))
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
        let resolved = self.resolve().await?;
        tokio::time::sleep(wait_for(resolved.available_at, SystemTime::now())).await;

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
        self.metadata.aux_metadata().await
    }
}

impl From<Ytdl> for Input {
    fn from(source: Ytdl) -> Self {
        Input::Lazy(Box::new(source))
    }
}

#[cfg(test)]
mod tests {
    use super::{proxy_args, wait_for, MAX_WAIT};
    use std::time::{Duration, UNIX_EPOCH};

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
