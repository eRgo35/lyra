use crate::commands::music::notifier::TrackErrorNotifier;
use crate::commands::music::repeat_all::Enqueuer;
use crate::{
    commands::embeds::{error_embed, format_duration, with_detail},
    Context, Error,
};

use crate::http::Ytdl;
use poise::serenity_prelude::{
    Color, CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter, Timestamp,
};
use poise::CreateReply;
use songbird::input::AuxMetadata;
use songbird::Call;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use tokio::sync::Semaphore;
use tracing::warn;

/// Plays a song; \
/// you can search by query or paste an url; \
/// aliases: play, p, enqueue
#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    aliases("p", "enqueue"),
    category = "Music"
)]
pub async fn play(
    ctx: Context<'_>,
    #[description = "Provide a query or an url"]
    #[rest]
    song: String,
) -> Result<(), Error> {
    enqueue_song(ctx, song, false).await
}

/// Resolves `song` and enqueues it, joining the author's voice channel if needed.
///
/// With `force`, only the first resolved track is used: it starts immediately and the
/// interrupted track is queued right after it, from the beginning.
pub(super) async fn enqueue_song(ctx: Context<'_>, song: String, force: bool) -> Result<(), Error> {
    // Resolving with yt-dlp often takes longer than Discord's 3 s to acknowledge a slash command.
    ctx.defer().await?;
    let guild_id = ctx.guild_id().expect("guild_only");
    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird client placed at init")
        .clone();
    let enqueuer = Enqueuer {
        manager: manager.clone(),
        guild_id,
        http_client: ctx.data().media_client.clone(),
        repeat: ctx.data().repeat_all.clone(),
        volumes: ctx.data().volumes.clone(),
    };

    let existing = manager.get(guild_id);
    let connect_to = match &existing {
        Some(_) => None,
        None => {
            let channel = ctx.guild().and_then(|guild| {
                guild
                    .voice_states
                    .get(&ctx.author().id)
                    .and_then(|voice_state| voice_state.channel_id)
            });
            match channel {
                Some(channel) => Some(channel),
                None => {
                    let msg = "Join a voice channel first!";
                    ctx.send(CreateReply::default().embed(error_embed(ctx, msg).await?))
                        .await?;
                    return Ok(());
                }
            }
        }
    };

    // Joining and resolving at once; neither holds the call's lock, which other
    // commands (and songbird itself) need while yt-dlp runs.
    let join = async {
        match connect_to {
            Some(channel) => Some(manager.join(guild_id, channel).await),
            None => None,
        }
    };
    // Something already playing: the new track has time to be resolved later.
    let busy = match &existing {
        Some(call) => !call.lock().await.queue().is_empty(),
        None => false,
    };
    let quick = busy && !force;
    let (resolved, joined) = tokio::join!(resolve(ctx, song, &enqueuer, force, quick), join);

    let call = match (existing, joined) {
        (Some(call), _) => call,
        (None, Some(Ok(call))) => {
            TrackErrorNotifier::attach(
                &mut *call.lock().await,
                ctx.serenity_context().http.clone(),
                ctx.channel_id(),
            );
            call
        }
        _ => {
            let msg = "Failed to join the voice channel!";
            ctx.send(CreateReply::default().embed(error_embed(ctx, msg).await?))
                .await?;
            return Ok(());
        }
    };
    let (src, rest) = resolved?;
    let Some(src) = src else {
        return handle_playlist(rest, &enqueuer).await;
    };

    let metadata = src.metadata().cloned().unwrap_or_default();
    let position = {
        let mut handler = call.lock().await;
        let position = handler.queue().len();
        enqueuer
            .enqueue_source(&mut handler, src, metadata.clone())
            .await;
        if force && handler.queue().len() > 1 {
            play_now(&mut handler, &enqueuer).await;
        }
        position
    };
    ctx.send(
        CreateReply::default()
            .embed(generate_embed(ctx, metadata, position, rest.len(), force).await?),
    )
    .await?;

    handle_playlist(rest, &enqueuer).await
}

/// The first track's source (`None` after telling the user why it can't be played)
/// and the rest of the entries. With `quick`, a YouTube URL is looked up with
/// [`crate::http::quick_lookup`].
async fn resolve(
    ctx: Context<'_>,
    song: String,
    enqueuer: &Enqueuer,
    force: bool,
    quick: bool,
) -> Result<(Option<Ytdl>, VecDeque<Entry>), Error> {
    let mut results = match parse_data(song).await {
        Ok(results) => results,
        Err(msg) => {
            ctx.send(CreateReply::default().embed(error_embed(ctx, &msg).await?))
                .await?;
            return Ok((None, VecDeque::new()));
        }
    };

    let Some(first) = results.pop_front() else {
        let msg = "No tracks found!";
        ctx.send(CreateReply::default().embed(error_embed(ctx, msg).await?))
            .await?;
        return Ok((None, results));
    };

    if force {
        // forceplay takes a single track; the rest of a playlist is not enqueued.
        results.clear();
    }

    let src = match first.metadata {
        Some(metadata) => {
            crate::http::ytdl(enqueuer.http_client.clone(), first.query).with_metadata(metadata)
        }
        None if quick => {
            match crate::http::quick_lookup(enqueuer.http_client.clone(), first.query).await {
                Ok(src) => src,
                Err(e) => {
                    warn!("Failed to load track metadata: {e}");
                    let msg = with_detail("Could not load that track!", &e);
                    ctx.send(CreateReply::default().embed(error_embed(ctx, &msg).await?))
                        .await?;
                    return Ok((None, results));
                }
            }
        }
        None => match lookup_source(enqueuer, first.query).await {
            Ok(src) => src,
            Err(e) => {
                warn!("Failed to load track metadata: {e}");
                let msg = with_detail("Could not load that track!", &e);
                ctx.send(CreateReply::default().embed(error_embed(ctx, &msg).await?))
                    .await?;
                // The rest of a playlist is still worth playing.
                return Ok((None, results));
            }
        },
    };

    Ok((Some(src), results))
}

/// Moves the last enqueued track to the front and starts it, queueing a fresh copy of
/// the interrupted track right after it (songbird tracks cannot be restarted in place).
async fn play_now(handler: &mut Call, enqueuer: &Enqueuer) {
    let queue = handler.queue().clone();
    let Some(current) = queue.current() else {
        return;
    };
    let new_index = queue.len() - 1;

    let metadata = (*current.data::<AuxMetadata>()).clone();
    match metadata.source_url.clone() {
        Some(url) => {
            // The copy below replaces the interrupted track, so `repeat all` must not add another.
            enqueuer.repeat.skip_once(enqueuer.guild_id, &current);
            enqueuer.enqueue(handler, url, metadata).await;
        }
        None => warn!("Interrupted track has no source URL; it will not be resumed"),
    }

    queue.modify_queue(|q| move_to_front(q, new_index));

    let _ = queue.skip();
}

/// `[current, .., new]` or `[current, .., new, requeued]` → `[current, new, requeued?, ..]`.
fn move_to_front<T>(queue: &mut VecDeque<T>, new_index: usize) {
    let requeued = if queue.len() - 1 > new_index {
        queue.pop_back()
    } else {
        None
    };
    if let Some(new) = queue.remove(new_index) {
        queue.insert(1, new);
    }
    if let Some(requeued) = requeued {
        queue.insert(2, requeued);
    }
}

/// A track to enqueue: the yt-dlp query, plus its metadata when it is already known.
pub struct Entry {
    pub query: String,
    pub metadata: Option<AuxMetadata>,
}

impl Entry {
    fn query(query: String) -> Self {
        Entry {
            query,
            metadata: None,
        }
    }
}

/// Turns `play` input (search terms, URLs, playlists, Spotify links) into entries to
/// enqueue. Errors are messages for the user.
pub async fn parse_data(data: String) -> Result<VecDeque<Entry>, String> {
    let mut tracks = VecDeque::new();

    let inputs = match spotify_parser::retrieve_async_url(&data).await {
        Ok(tracks) => tracks,
        Err(spotify_parser::ParseError::InvalidUrl) => vec![data],
        Err(e) => {
            warn!("Spotify lookup failed: {e}");
            return Err(format!("Could not read that Spotify link! ({e})"));
        }
    };

    for track in inputs {
        if track.contains("?list=") {
            let output = Command::new("yt-dlp")
                .args(crate::http::ytdl_proxy_args())
                .args(["-j", "--flat-playlist", "--", &track])
                .output()
                .await
                .map_err(|e| {
                    warn!("Failed to run yt-dlp: {e}");
                    if e.kind() == std::io::ErrorKind::NotFound {
                        "`yt-dlp` was not found! Install it and make sure it is on PATH."
                            .to_string()
                    } else {
                        format!("Failed to run `yt-dlp`: {e}")
                    }
                })?;

            tracks.extend(playlist_entries(&String::from_utf8_lossy(&output.stdout)));
        } else if track.starts_with("http") {
            tracks.push_back(Entry::query(track));
        } else {
            tracks.push_back(Entry::query(format!("ytsearch:{}", track)));
        }
    }

    Ok(tracks)
}

/// Entries from `yt-dlp -j --flat-playlist` output (one JSON object per line), with
/// metadata taken from the same output so no per-track yt-dlp call is needed.
///
/// Only `http(s)` URLs are kept: entries are later passed to yt-dlp by songbird without
/// a `--` separator, so a value starting with `-` would be parsed as an option.
fn playlist_entries(output: &str) -> Vec<Entry> {
    output
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter_map(|entry| {
            let url = entry.get("url")?.as_str()?.to_string();
            if !url.starts_with("http") {
                return None;
            }

            let text = |key: &str| entry.get(key)?.as_str().map(str::to_string);
            let channel = text("channel").or_else(|| text("uploader"));
            let thumbnail = entry
                .get("thumbnails")
                .and_then(|thumbnails| thumbnails.as_array()?.last()?.get("url")?.as_str())
                .map(str::to_string)
                .or_else(|| text("thumbnail"));

            let metadata = AuxMetadata {
                title: text("title"),
                artist: text("artist").or_else(|| channel.clone()),
                channel,
                duration: entry
                    .get("duration")
                    .and_then(|d| d.as_f64())
                    .filter(|d| d.is_finite() && *d >= 0.0)
                    .map(Duration::from_secs_f64),
                source_url: Some(url.clone()),
                thumbnail,
                ..AuxMetadata::default()
            };

            Some(Entry {
                query: url,
                metadata: Some(metadata),
            })
        })
        .collect()
}

/// How many yt-dlp metadata lookups run at once for entries without known metadata
/// (e.g. a Spotify playlist): one per two CPU cores, from 1 to 4. Each is a Python
/// process (often with deno), so more would only slow a small machine down.
fn metadata_lookups() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get() / 2)
        .unwrap_or(1)
        .clamp(1, 4)
}

/// The source for `query` with its metadata, from one yt-dlp run.
async fn lookup_source(
    enqueuer: &Enqueuer,
    query: String,
) -> Result<Ytdl, songbird::input::AudioStreamError> {
    crate::http::lookup(enqueuer.http_client.clone(), query).await
}

async fn handle_playlist(playlist: VecDeque<Entry>, enqueuer: &Enqueuer) -> Result<(), Error> {
    let lookups = Arc::new(Semaphore::new(metadata_lookups()));

    // Start all lookups up front (bounded by the semaphore) but enqueue in playlist order.
    let pending = playlist
        .into_iter()
        .map(|entry| {
            let lookups = lookups.clone();
            let enqueuer = enqueuer.clone();
            tokio::spawn(async move {
                let src = match entry.metadata {
                    Some(metadata) => Ok(crate::http::ytdl(
                        enqueuer.http_client.clone(),
                        entry.query.clone(),
                    )
                    .with_metadata(metadata)),
                    None => {
                        let _permit = lookups.acquire().await;
                        lookup_source(&enqueuer, entry.query.clone()).await
                    }
                };
                (entry.query, src)
            })
        })
        .collect::<Vec<_>>();

    for lookup in pending {
        let (query, src) = lookup.await?;
        let src = match src {
            Ok(src) => src,
            Err(e) => {
                warn!("Skipping playlist entry {query}: {e}");
                continue;
            }
        };

        // The bot may have left the channel while the playlist was loading.
        let Some(handler) = enqueuer.manager.get(enqueuer.guild_id) else {
            break;
        };
        let metadata = src.metadata().cloned().unwrap_or_default();
        enqueuer
            .enqueue_source(&mut *handler.lock().await, src, metadata)
            .await;
    }

    Ok(())
}

/// `position` is the queue length before the track was added.
async fn generate_embed(
    ctx: Context<'_>,
    metadata: AuxMetadata,
    position: usize,
    remaining: usize,
    force: bool,
) -> Result<CreateEmbed, Error> {
    let AuxMetadata {
        title,
        thumbnail,
        source_url,
        artist,
        duration,
        ..
    } = metadata;
    let mut description = format!("Enqueued @ {}", position + 1);
    let mut tracks = "Tracks enqueued";

    if remaining == 0 {
        tracks = "Track enqueued";
    }

    if position == 0 || force {
        description = "Playing now!".to_string();
    }

    let embed = CreateEmbed::default()
        .author(CreateEmbedAuthor::new(tracks).icon_url(ctx.author().clone().face()))
        .colour(Color::from_rgb(255, 58, 97))
        .title(title.unwrap_or("Unknown Title".to_string()))
        .thumbnail(thumbnail.unwrap_or(ctx.cache().current_user().face()))
        .field(
            "Artist",
            artist.unwrap_or("Unknown Artist".to_string()),
            true,
        )
        .field("Duration", format_duration(duration), true)
        .field("DJ", ctx.author().name.clone(), true)
        .description(description)
        .timestamp(Timestamp::now())
        .footer(
            CreateEmbedFooter::new(ctx.cache().current_user().name.to_string())
                .icon_url(ctx.cache().current_user().face()),
        );

    let embed = match source_url {
        Some(url) => embed.url(url),
        None => embed,
    };

    Ok(embed)
}

#[cfg(test)]
mod tests {
    use super::{move_to_front, playlist_entries};
    use std::collections::VecDeque;
    use std::time::Duration;

    fn playlist_urls(output: &str) -> Vec<String> {
        playlist_entries(output)
            .into_iter()
            .map(|entry| entry.query)
            .collect()
    }

    #[test]
    fn playlist_urls_reads_url_field_including_dash_and_underscore_ids() {
        let output = concat!(
            r#"{"_type": "url", "ie_key": "Youtube", "id": "kPa7bsKwL-c", "url": "https://www.youtube.com/watch?v=kPa7bsKwL-c"}"#,
            "\n",
            r#"{"_type": "url", "id": "a_b-c_d-e_f", "url": "https://music.youtube.com/watch?v=a_b-c_d-e_f"}"#,
            "\n",
        );
        assert_eq!(
            playlist_urls(output),
            vec![
                "https://www.youtube.com/watch?v=kPa7bsKwL-c",
                "https://music.youtube.com/watch?v=a_b-c_d-e_f",
            ]
        );
    }

    #[test]
    fn playlist_urls_skips_non_json_and_entries_without_url() {
        assert!(playlist_urls("").is_empty());
        assert_eq!(
            playlist_urls("WARNING: something\n{\"id\": \"x\"}\n{\"url\": \"https://u\"}\n"),
            vec!["https://u"]
        );
    }

    #[test]
    fn playlist_urls_drops_values_that_yt_dlp_would_parse_as_options() {
        let output = concat!(
            r#"{"url": "--exec=touch /tmp/pwned"}"#,
            "\n",
            r#"{"url": "-o/tmp/x"}"#,
            "\n",
            r#"{"url": "relative/path"}"#,
            "\n",
        );
        assert!(playlist_urls(output).is_empty());
    }

    #[test]
    fn playlist_entries_carry_metadata_from_flat_playlist_output() {
        let output = r#"{"url": "https://www.youtube.com/watch?v=kPa7bsKwL-c", "title": "Die With A Smile", "duration": 253.0, "channel": "Lady Gaga", "thumbnails": [{"url": "https://i.ytimg.com/small.jpg"}, {"url": "https://i.ytimg.com/large.jpg"}]}"#;
        let entries = playlist_entries(output);
        let metadata = entries[0].metadata.as_ref().unwrap();

        assert_eq!(metadata.title.as_deref(), Some("Die With A Smile"));
        assert_eq!(metadata.artist.as_deref(), Some("Lady Gaga"));
        assert_eq!(metadata.duration, Some(Duration::from_secs(253)));
        assert_eq!(
            metadata.source_url.as_deref(),
            Some("https://www.youtube.com/watch?v=kPa7bsKwL-c")
        );
        assert_eq!(
            metadata.thumbnail.as_deref(),
            Some("https://i.ytimg.com/large.jpg")
        );
    }

    #[test]
    fn playlist_entries_tolerate_missing_metadata_fields() {
        let entries = playlist_entries(r#"{"url": "https://u", "duration": null}"#);
        let metadata = entries[0].metadata.as_ref().unwrap();

        assert_eq!(metadata.title, None);
        assert_eq!(metadata.artist, None);
        assert_eq!(metadata.duration, None);
    }

    #[test]
    fn move_to_front_puts_new_track_next_and_requeued_after_it() {
        // 0 = current, 9 = forceplayed, 10 = fresh copy of the interrupted track.
        let mut queue = VecDeque::from([0, 1, 2, 9, 10]);
        move_to_front(&mut queue, 3);
        assert_eq!(queue, [0, 9, 10, 1, 2]);

        let mut queue = VecDeque::from([0, 9, 10]);
        move_to_front(&mut queue, 1);
        assert_eq!(queue, [0, 9, 10]);
    }

    #[test]
    fn move_to_front_without_requeued_track() {
        let mut queue = VecDeque::from([0, 1, 2, 9]);
        move_to_front(&mut queue, 3);
        assert_eq!(queue, [0, 9, 1, 2]);
    }
}
