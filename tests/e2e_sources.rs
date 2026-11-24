//! End-to-end: `play` input through the real `yt-dlp` and Spotify, as the bot resolves it.
//!
//! Needs network access and `yt-dlp` on PATH, so these are ignored by default:
//! `cargo test --test e2e_sources -- --ignored`

use lyra::commands::music::play::parse_data;
use songbird::input::{Compose, YoutubeDl};

/// A large public YouTube playlist (official music videos).
const PLAYLIST_ID: &str = "PLMC9KNkIncKtPzgY-5rmhvj7fax8fdxoj";

async fn metadata(query: &str) -> Result<songbird::input::AuxMetadata, String> {
    YoutubeDl::new_ytdl_like("yt-dlp", reqwest::Client::new(), query.to_string())
        .aux_metadata()
        .await
        .map_err(|e| e.to_string())
}

#[tokio::test]
#[ignore = "network: YouTube via yt-dlp"]
async fn youtube_playlist_expands_with_metadata_and_no_per_track_lookup() {
    let url = format!("https://www.youtube.com/playlist?list={PLAYLIST_ID}");
    let entries = parse_data(url).await.expect("yt-dlp runs");

    assert!(entries.len() >= 50, "only {} entries", entries.len());
    for entry in &entries {
        assert!(entry.query.starts_with("https://www.youtube.com/watch?v="));
        let metadata = entry
            .metadata
            .as_ref()
            .expect("metadata from --flat-playlist");
        assert!(metadata.title.is_some(), "{} has no title", entry.query);
    }
    assert!(
        entries
            .iter()
            .any(|e| e.query.contains('-') || e.query.contains('_')),
        "expected some IDs with - or _ (the old regex dropped them)"
    );
}

#[tokio::test]
#[ignore = "network: YouTube Music via yt-dlp"]
async fn youtube_music_playlist_expands() {
    let url = format!("https://music.youtube.com/playlist?list={PLAYLIST_ID}");
    let entries = parse_data(url).await.expect("yt-dlp runs");

    assert!(entries.len() >= 50, "only {} entries", entries.len());
    assert!(entries
        .iter()
        .all(|e| e.query.starts_with("https://music.youtube.com/watch?v=")));
}

#[tokio::test]
#[ignore = "network: YouTube via yt-dlp"]
async fn single_url_resolves_its_metadata_when_enqueued() {
    let url = "https://www.youtube.com/watch?v=kPa7bsKwL-c";
    let entries = parse_data(url.to_string()).await.unwrap();

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].query, url);
    assert!(
        entries[0].metadata.is_none(),
        "looked up later, 4 at a time"
    );

    let metadata = metadata(url).await.unwrap();
    assert!(metadata.title.unwrap().contains("Die With A Smile"));
    assert!(metadata.duration.is_some());
}

#[tokio::test]
#[ignore = "network: YouTube search via yt-dlp"]
async fn search_terms_become_a_youtube_search() {
    let entries = parse_data("rick astley never gonna give you up".to_string())
        .await
        .unwrap();

    assert_eq!(entries.len(), 1);
    assert_eq!(
        entries[0].query,
        "ytsearch:rick astley never gonna give you up"
    );
    let metadata = metadata(&entries[0].query).await.unwrap();
    assert!(metadata
        .source_url
        .unwrap()
        .starts_with("https://www.youtube.com/watch?v="));
}

#[tokio::test]
#[ignore = "network: YouTube via yt-dlp"]
async fn unavailable_video_is_an_error_not_a_panic() {
    assert!(metadata("https://www.youtube.com/watch?v=xxxxxxxxxxx")
        .await
        .is_err());
}

#[tokio::test]
#[ignore = "runs yt-dlp"]
async fn option_like_playlist_input_is_not_parsed_as_a_yt_dlp_option() {
    // Passed after `--`, yt-dlp treats it as a (bad) URL and returns nothing.
    let entries = parse_data("--version ?list=x".to_string()).await.unwrap();
    assert!(entries.is_empty());
}

#[tokio::test]
#[ignore = "network: Spotify embed page"]
async fn spotify_track_becomes_a_youtube_search() {
    let entries = parse_data("https://open.spotify.com/track/4cOdK2wGLETKBW3PvgPWqT".to_string())
        .await
        .unwrap();

    assert_eq!(entries.len(), 1);
    let query = &entries[0].query;
    assert!(query.starts_with("ytsearch:"), "{query}");
    assert!(
        query.to_lowercase().contains("never gonna give you up"),
        "{query}"
    );
}

/// One yt-dlp run, then songbird decodes the stream while it downloads into the
/// audio cache; once downloaded, a second play comes from disk.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "network: YouTube via yt-dlp"]
async fn looked_up_track_plays_through_the_audio_cache() {
    use songbird::input::codecs::{get_codec_registry, get_probe};
    use songbird::input::Input;
    use std::time::{Duration, Instant};

    // The cache lives under the working directory; keep it out of the repository.
    let dir = std::env::temp_dir().join(format!("lyra-e2e-cache-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::env::set_current_dir(&dir).unwrap();
    let cached = dir.join(lyra::AUDIO_CACHE_DIR).join("kPa7bsKwL-c.webm");

    async fn first_packet(src: lyra::http::Ytdl) -> Duration {
        let start = Instant::now();
        let input = Input::from(src)
            .make_playable_async(get_codec_registry(), get_probe())
            .await
            .expect("playable");
        let Input::Live(live, _) = input else {
            panic!("not live")
        };
        let mut live = live.promote(get_codec_registry(), get_probe()).unwrap();
        tokio::task::spawn_blocking(move || {
            let parsed = live.parsed_mut().unwrap();
            for _ in 0..50 {
                parsed.format.next_packet().expect("packet");
            }
        })
        .await
        .unwrap();
        start.elapsed()
    }

    let url = "https://www.youtube.com/watch?v=kPa7bsKwL-c";
    let lookup = Instant::now();
    let src = lyra::http::lookup(reqwest::Client::new(), url)
        .await
        .expect("lookup");
    println!("lookup: {:?}", lookup.elapsed());
    println!(
        "first play, to 50 packets: {:?}",
        first_packet(src.clone()).await
    );

    for _ in 0..300 {
        if cached.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(cached.exists(), "downloaded into the cache");
    println!("cached play, to 50 packets: {:?}", first_packet(src).await);
    let _ = std::fs::remove_dir_all(&dir);
}
