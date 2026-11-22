use std::sync::LazyLock;

use poise::serenity_prelude::prelude::TypeMapKey;
use reqwest::Client as HttpClient;
use songbird::input::YoutubeDl;

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
pub fn ytdl(client: HttpClient, query: impl Into<String>) -> YoutubeDl<'static> {
    YoutubeDl::new_ytdl_like("yt-dlp", client, query.into()).user_args(ytdl_proxy_args())
}

#[cfg(test)]
mod tests {
    use super::proxy_args;

    #[test]
    fn proxy_args_only_when_set() {
        assert!(proxy_args(None).is_empty());
        assert_eq!(
            proxy_args(Some("http://10.0.0.1:8888")),
            ["--proxy", "http://10.0.0.1:8888"]
        );
    }
}
