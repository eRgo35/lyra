use poise::CreateReply;
use serde::Deserialize;
use songbird::input::AuxMetadata;

use crate::commands::tools::lookup::{get_json, reply};
use crate::{commands::embeds::error_embed, Context, Error};

/// Shows lyrics for the current track, or for a song you name
#[poise::command(prefix_command, slash_command, guild_only, category = "Music")]
pub async fn lyrics(
    ctx: Context<'_>,
    #[description = "Song to look up (default: the current track)"]
    #[rest]
    song: Option<String>,
) -> Result<(), Error> {
    let query = match song.filter(|s| !s.trim().is_empty()) {
        Some(song) => song.trim().to_string(),
        None => match current_track(ctx).await {
            Some(metadata) => search_terms(
                metadata.title.as_deref().unwrap_or_default(),
                metadata.artist.as_deref(),
            ),
            None => {
                let msg = "Nothing is playing; name a song, e.g. `lyrics never gonna give you up`";
                ctx.send(CreateReply::default().embed(error_embed(ctx, msg).await?))
                    .await?;
                return Ok(());
            }
        },
    };

    let result = fetch_lyrics(&ctx.data().http_client, LRCLIB_API, &query).await;
    reply(ctx, "Lyrics", result).await
}

async fn current_track(ctx: Context<'_>) -> Option<AuxMetadata> {
    let manager = songbird::get(ctx.serenity_context()).await?;
    let call = manager.get(ctx.guild_id()?)?;
    let track = call.lock().await.queue().current()?;
    let metadata = track.data::<AuxMetadata>().as_ref().clone();
    Some(metadata)
}

/// lrclib.net, no key needed.
pub const LRCLIB_API: &str = "https://lrclib.net";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Song {
    track_name: String,
    artist_name: String,
    #[serde(default)]
    instrumental: bool,
    plain_lyrics: Option<String>,
}

/// Lyrics for the best match of `query` as an embed title and description.
pub async fn fetch_lyrics(
    client: &reqwest::Client,
    base_url: &str,
    query: &str,
) -> Result<(String, String), String> {
    let not_found = || format!("No lyrics found for \"{query}\"!");
    if query.trim().is_empty() {
        return Err(not_found());
    }
    let songs: Vec<Song> = get_json(
        client,
        &format!("{base_url}/api/search"),
        &[("q", query)],
        "lrclib.net",
    )
    .await?
    .unwrap_or_default();

    let song = songs
        .into_iter()
        .find(|s| {
            s.instrumental
                || s.plain_lyrics
                    .as_deref()
                    .is_some_and(|l| !l.trim().is_empty())
        })
        .ok_or_else(not_found)?;
    let title = if song.track_name.contains(&song.artist_name) {
        song.track_name.clone()
    } else {
        format!("{} – {}", song.artist_name, song.track_name)
    };
    let lyrics = match song.plain_lyrics {
        Some(lyrics) if !song.instrumental => truncate(lyrics.trim(), DESCRIPTION_LIMIT),
        _ => "🎼 Instrumental".to_string(),
    };
    Ok((title, lyrics))
}

/// Discord's embed description limit is 4096; leave room for the ellipsis line.
const DESCRIPTION_LIMIT: usize = 4000;

/// Cuts at a line break before `limit` characters and marks the cut.
fn truncate(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let cut: String = text.chars().take(limit).collect();
    let end = cut.rfind('\n').unwrap_or(cut.len());
    format!("{}\n…", &cut[..end])
}

/// A search query from a YouTube-style title: drops "(Official Video)", "[HD]" and
/// the like, and adds the artist (minus "VEVO"/"- Topic") unless the title has one.
fn search_terms(title: &str, artist: Option<&str>) -> String {
    let mut cleaned = String::new();
    let mut depth = 0usize;
    for c in title.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            _ if depth == 0 => cleaned.push(c),
            _ => {}
        }
    }
    let cleaned = cleaned.split('|').next().unwrap_or_default();
    let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");

    let artist = artist
        .map(|a| {
            a.trim_end_matches(" - Topic")
                .trim_end_matches("VEVO")
                .trim()
        })
        .filter(|a| !a.is_empty());
    match artist {
        Some(artist)
            if !cleaned.contains(" - ")
                && !cleaned.to_lowercase().contains(&artist.to_lowercase()) =>
        {
            format!("{artist} {cleaned}")
        }
        _ => cleaned,
    }
}

#[cfg(test)]
mod tests {
    use super::{search_terms, truncate};

    #[test]
    fn youtube_titles_become_search_terms() {
        assert_eq!(
            search_terms(
                "Rick Astley - Never Gonna Give You Up (Official Music Video)",
                Some("Rick Astley")
            ),
            "Rick Astley - Never Gonna Give You Up"
        );
        assert_eq!(
            search_terms("Die With A Smile [4K] | Live", Some("LadyGagaVEVO")),
            "LadyGaga Die With A Smile"
        );
        assert_eq!(
            search_terms("Bohemian Rhapsody", Some("Queen - Topic")),
            "Queen Bohemian Rhapsody"
        );
        assert_eq!(search_terms("Song (feat. X (remix))", None), "Song");
    }

    #[test]
    fn long_lyrics_are_cut_at_a_line() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(
            truncate("line one\nline two\nline three", 20),
            "line one\nline two\n…"
        );
        let long = "ą".repeat(5000);
        assert_eq!(truncate(&long, 4000).chars().count(), 4002);
    }
}
