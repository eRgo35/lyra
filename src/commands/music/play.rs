use crate::commands::music::notifier::TrackErrorNotifier;
use crate::{commands::embeds::error_embed, Context, Error};

use poise::serenity_prelude::{
    Color, CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter, Timestamp,
};
use poise::CreateReply;
use reqwest::Client;
use serenity::all::GuildId;
use songbird::events::TrackEvent;
use songbird::input::AuxMetadata;
use songbird::input::{Compose, YoutubeDl};
use songbird::tracks::{Track, TrackQueue};
use songbird::Call;
use spotify_parser;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
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
    let guild_id = ctx.guild_id().unwrap();
    let http_client = ctx.data().http_client.clone();
    let manager = songbird::get(ctx.serenity_context()).await.unwrap().clone();

    let mut rest_playlist: VecDeque<String> = VecDeque::new();

    if manager.get(guild_id).is_none() {
        let channel_id = ctx
            .guild()
            .unwrap()
            .voice_states
            .get(&ctx.author().id)
            .and_then(|voice_state| voice_state.channel_id);

        let connect_to = match channel_id {
            Some(channel) => channel,
            None => {
                let msg = "I am not in a voice channel!";
                ctx.send(CreateReply::default().embed(error_embed(ctx, msg).await.unwrap()))
                    .await?;
                return Ok(());
            }
        };

        if let Ok(handler_lock) = manager.join(guild_id, connect_to).await {
            let mut handler = handler_lock.lock().await;
            handler.add_global_event(TrackEvent::Error.into(), TrackErrorNotifier);

            rest_playlist = handle_play(ctx, song, handler, http_client.clone()).await?;
        } else {
            let msg = "Failed to join the voice channel!";
            ctx.send(CreateReply::default().embed(error_embed(ctx, msg).await.unwrap()))
                .await?;
        }
    } else {
        let handler = manager.get(guild_id).unwrap();
        let mut handler = handler.lock().await;
        handler.add_global_event(TrackEvent::Error.into(), TrackErrorNotifier);
        rest_playlist = handle_play(ctx, song, handler, http_client.clone()).await?;
    }

    handle_playlist(rest_playlist, manager, guild_id, http_client).await?;

    Ok(())
}

async fn handle_play<'a>(
    ctx: Context<'a>,
    song: String,
    mut handler: tokio::sync::MutexGuard<'a, Call>,
    http_client: Client,
) -> Result<VecDeque<String>, Error> {
    let mut results = match parse_data(song).await {
        Ok(results) => results,
        Err(e) => {
            warn!("Failed to run yt-dlp: {e}");
            let msg = if e.kind() == std::io::ErrorKind::NotFound {
                "`yt-dlp` was not found! Install it and make sure it is on PATH.".to_string()
            } else {
                format!("Failed to run `yt-dlp`: {e}")
            };
            ctx.send(CreateReply::default().embed(error_embed(ctx, &msg).await?))
                .await?;
            return Ok(VecDeque::new());
        }
    };

    let Some(first) = results.pop_front() else {
        let msg = "No tracks found!";
        ctx.send(CreateReply::default().embed(error_embed(ctx, msg).await?))
            .await?;
        return Ok(results);
    };

    let src: YoutubeDl = YoutubeDl::new_ytdl_like("yt-dlp", http_client, first);

    let aux_metadata = match src.clone().aux_metadata().await {
        Ok(metadata) => metadata,
        Err(e) => {
            warn!("Failed to load track metadata: {e}");
            let msg = "Could not load that track!";
            ctx.send(CreateReply::default().embed(error_embed(ctx, msg).await?))
                .await?;
            return Ok(results);
        }
    };

    ctx.send(
        CreateReply::default().embed(
            generate_embed(ctx, aux_metadata.clone(), handler.queue(), results.clone()).await?,
        ),
    )
    .await?;

    handler
        .enqueue(Track::new_with_data(
            src.clone().into(),
            Arc::new(aux_metadata),
        ))
        .await;

    Ok(results)
}

async fn parse_data(data: String) -> std::io::Result<VecDeque<String>> {
    let mut tracks = VecDeque::new();

    for track in spotify_parser::retrieve_async_url(&data)
        .await
        .unwrap_or(vec![data])
    {
        if track.contains("?list=") {
            let output = Command::new("yt-dlp")
                .args(["-j", "--flat-playlist", "--", &track])
                .output()
                .await?;

            tracks.extend(playlist_urls(&String::from_utf8_lossy(&output.stdout)));
        } else if track.starts_with("http") {
            tracks.push_back(track);
        } else {
            tracks.push_back(format!("ytsearch:{}", track));
        }
    }

    Ok(tracks)
}

/// Track URLs from `yt-dlp -j --flat-playlist` output (one JSON object per line).
///
/// Only `http(s)` URLs are kept: entries are later passed to yt-dlp by songbird without
/// a `--` separator, so a value starting with `-` would be parsed as an option.
fn playlist_urls(output: &str) -> Vec<String> {
    output
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter_map(|entry| entry.get("url")?.as_str().map(str::to_string))
        .filter(|url| url.starts_with("http"))
        .collect()
}

async fn handle_playlist(
    playlist: VecDeque<String>,
    manager: std::sync::Arc<songbird::Songbird>,
    guild_id: GuildId,
    http_client: Client,
) -> Result<(), Error> {
    for song in playlist {
        if manager.get(guild_id).is_some() {
            let handler = manager.get(guild_id).unwrap();
            let mut handler = handler.lock().await;

            let src: YoutubeDl =
                YoutubeDl::new_ytdl_like("yt-dlp", http_client.clone(), song.clone());

            let aux_metadata = match src.clone().aux_metadata().await {
                Ok(metadata) => metadata,
                Err(e) => {
                    warn!("Skipping playlist entry {song}: {e}");
                    continue;
                }
            };

            handler
                .enqueue(Track::new_with_data(
                    src.clone().into(),
                    Arc::new(aux_metadata),
                ))
                .await;
        }
    }

    Ok(())
}

async fn generate_embed(
    ctx: Context<'_>,
    metadata: AuxMetadata,
    queue: &TrackQueue,
    results: VecDeque<String>,
) -> Result<CreateEmbed, Error> {
    let AuxMetadata {
        title,
        thumbnail,
        source_url,
        artist,
        duration,
        ..
    } = metadata;
    let duration_minutes = duration.unwrap_or(Duration::new(0, 0)).clone().as_secs() / 60;
    let duration_seconds = duration.unwrap_or(Duration::new(0, 0)).clone().as_secs() % 60;
    let mut description = format!("Enqueued @ {}", queue.len() + 1);
    let mut tracks = "Tracks enqueued";

    if results.len() == 1 {
        tracks = "Track enqueued";
    }

    if queue.is_empty() {
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
        .field(
            "Duration",
            format!("{:02}:{:02}", duration_minutes, duration_seconds),
            true,
        )
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
    use super::playlist_urls;

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
}
