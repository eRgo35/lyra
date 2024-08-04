use crate::commands::music::metadata::Metadata;
use crate::{commands::embeds::error_embed, Context, Error};

use fancy_regex::Regex;
use lib_spotify_parser;
use poise::serenity_prelude::model::Timestamp;
use poise::serenity_prelude::Colour;
use poise::serenity_prelude::CreateEmbed;
use poise::CreateReply;
use regex::Regex as Regex_Classic;
use serenity::builder::CreateEmbedAuthor;
use serenity::builder::CreateEmbedFooter;
use songbird::events::TrackEvent;
use songbird::input::AuxMetadata;
use songbird::input::{Compose, YoutubeDl};
use songbird::tracks::{TrackHandle, TrackQueue};
use std::process::Command;
use std::time::Duration;

use crate::commands::music::notifier::TrackErrorNotifier;
use crate::http::HttpKey;

/// Plays a song; \
/// you can search by query or paste an url; \
/// aliases: play, p, enqueue
#[poise::command(
    prefix_command,
    slash_command,
    aliases("p", "enqueue"),
    category = "Music"
)]
pub async fn play(
    ctx: Context<'_>,
    #[description = "Provide a query or an url"]
    #[rest]
    mut song: String,
) -> Result<(), Error> {
    let regex_spotify = Regex::new(r"https?:\/\/(?:embed\.|open\.)(?:spotify\.com\/)(?:track\/|\?uri=spotify:track:)((\w|-)+)(?:(?=\?)(?:[?&]foo=(\d*)(?=[&#]|$)|(?![?&]foo=)[^#])+)?(?=#|$)").unwrap();
    let regex_youtube =
        Regex_Classic::new(r#""url": "(https://www.youtube.com/watch\?v=[A-Za-z0-9]{11})""#)
            .unwrap();
    let regex_youtube_playlist = Regex::new(
        r"^((?:https?:)\/\/)?((?:www|m)\.)?((?:youtube\.com)).*(youtu.be\/|list=)([^#&?]*).*",
    )
    .unwrap();
    let regex_spotify_playlist = Regex::new(r"https?:\/\/(?:embed\.|open\.)(?:spotify\.com\/)(?:(album|playlist)\/|\?uri=spotify:playlist:)((\w|-)+)(?:(?=\?)(?:[?&]foo=(\d*)(?=[&#]|$)|(?![?&]foo=)[^#])+)?(?=#|$)").unwrap();

    let is_playlist = regex_youtube_playlist.is_match(&song).unwrap()
        || regex_spotify_playlist.is_match(&song).unwrap();
    let is_spotify =
        regex_spotify.is_match(&song).unwrap() || regex_spotify_playlist.is_match(&song).unwrap();
    let is_query = !song.starts_with("http");

    let guild_id = ctx.guild_id().unwrap();
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

    let http_client = {
        let data = ctx.serenity_context().data.read().await;
        data.get::<HttpKey>()
            .cloned()
            .expect("Guaranteed to exist in the typemap.")
    };

    let manager = songbird::get(&ctx.serenity_context())
        .await
        .expect("Songbird Voice placed at init")
        .clone();

    if let Ok(handler_lock) = manager.join(guild_id, connect_to).await {
        let mut handler = handler_lock.lock().await;

        handler.add_global_event(TrackEvent::Error.into(), TrackErrorNotifier);

        if is_playlist && is_spotify {
            let tracks: Vec<String> = lib_spotify_parser::retrieve_async_url(&song).await.unwrap();

            for (index, url) in tracks.clone().iter().enumerate() {
                if url.is_empty() {
                    break;
                }
                let src = YoutubeDl::new_ytdl_like(
                    "yt-dlp",
                    http_client.clone(),
                    format!("ytsearch:{}", url.to_string()),
                );
                let aux_metadata = src.clone().aux_metadata().await.unwrap();
                let track = handler.enqueue_input(src.clone().into()).await;
                let _ = track
                    .typemap()
                    .write()
                    .await
                    .insert::<Metadata>(aux_metadata);

                if index == 0 {
                    let embed = generate_playlist_embed(ctx, track, tracks.len()).await;
                    let response = CreateReply::default().embed(embed.unwrap());
                    ctx.send(response).await?;
                }
            }

            return Ok(());
        }

        if is_playlist {
            let raw_list = Command::new("yt-dlp")
                .args(["-j", "--flat-playlist", &song])
                .output()
                .expect("failed to execute process")
                .stdout;

            let list = String::from_utf8(raw_list.clone()).expect("Invalid UTF-8");

            let urls: Vec<String> = regex_youtube
                .captures_iter(&list)
                .map(|capture| capture[1].to_string())
                .collect();

            for (index, url) in urls.clone().iter().enumerate() {
                if url.is_empty() {
                    break;
                }
                let src = YoutubeDl::new_ytdl_like("yt-dlp", http_client.clone(), url.to_string());
                let aux_metadata = src.clone().aux_metadata().await.unwrap();
                let track = handler.enqueue_input(src.clone().into()).await;
                let _ = track
                    .typemap()
                    .write()
                    .await
                    .insert::<Metadata>(aux_metadata);

                if index == 0 {
                    let embed = generate_playlist_embed(ctx, track, urls.len()).await;
                    let response = CreateReply::default().embed(embed.unwrap());
                    ctx.send(response).await?;
                }
            }

            return Ok(());
        }

        if is_spotify {
            song = format!(
                "ytsearch:{}",
                lib_spotify_parser::retrieve_async_url(&song)
                    .await
                    .unwrap()
                    .first()
                    .unwrap()
            );
        }

        if is_query {
            song = format!("ytsearch:{}", song);
        }

        let src = YoutubeDl::new_ytdl_like("yt-dlp", http_client, song);
        let embed = generate_embed(ctx, src.clone(), handler.queue()).await;
        let response = CreateReply::default().embed(embed.unwrap());
        ctx.send(response).await?;

        let aux_metadata = src.clone().aux_metadata().await.unwrap();

        let track = handler.enqueue_input(src.clone().into()).await;
        let _ = track
            .typemap()
            .write()
            .await
            .insert::<Metadata>(aux_metadata);
    }

    Ok(())
}

async fn generate_embed(
    ctx: Context<'_>,
    src: YoutubeDl,
    queue: &TrackQueue,
) -> Result<CreateEmbed, Error> {
    let metadata = src.clone().aux_metadata().await.unwrap();
    let AuxMetadata {
        title,
        thumbnail,
        source_url,
        artist,
        duration,
        ..
    } = metadata;
    let timestamp = Timestamp::now();
    let duration_minutes = duration.unwrap_or(Duration::new(0, 0)).clone().as_secs() / 60;
    let duration_seconds = duration.unwrap_or(Duration::new(0, 0)).clone().as_secs() % 60;
    let mut description = format!("Song added to queue @ {}", queue.len() + 1);

    if queue.len() == 0 {
        description = format!("Playing now!");
    }

    let embed = CreateEmbed::default()
        .author(CreateEmbedAuthor::new("Track enqueued").icon_url(ctx.author().clone().face()))
        .colour(Colour::from_rgb(255, 58, 97))
        .title(title.unwrap())
        .url(source_url.unwrap())
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
        .timestamp(timestamp)
        .footer(
            CreateEmbedFooter::new(ctx.cache().current_user().name.to_string())
                .icon_url(ctx.cache().current_user().face()),
        );

    Ok(embed)
}

async fn generate_playlist_embed(
    ctx: Context<'_>,
    track: TrackHandle,
    queue_length: usize,
) -> Result<CreateEmbed, Error> {
    let meta_typemap = track.typemap().read().await;
    let metadata = meta_typemap.get::<Metadata>().unwrap();
    let AuxMetadata {
        title,
        thumbnail,
        source_url,
        artist,
        duration,
        ..
    } = metadata;
    let timestamp = Timestamp::now();
    let duration_minutes = duration.unwrap_or(Duration::new(0, 0)).clone().as_secs() / 60;
    let duration_seconds = duration.unwrap_or(Duration::new(0, 0)).clone().as_secs() % 60;

    let description = format!("Enqueued tracks: {}", queue_length - 1);

    let embed = CreateEmbed::default()
        .author(CreateEmbedAuthor::new("Playlist enqueued").icon_url(ctx.author().clone().face()))
        .colour(Colour::from_rgb(255, 58, 97))
        .title(title.as_ref().unwrap())
        .url(source_url.as_ref().unwrap())
        .thumbnail(
            thumbnail
                .as_ref()
                .unwrap_or(&ctx.cache().current_user().face()),
        )
        .field(
            "Artist",
            artist.as_ref().unwrap_or(&"Unknown Artist".to_string()),
            true,
        )
        .field(
            "Duration",
            format!("{:02}:{:02}", duration_minutes, duration_seconds),
            true,
        )
        .field("DJ", ctx.author().name.clone(), true)
        .description(description)
        .timestamp(timestamp)
        .footer(
            CreateEmbedFooter::new(ctx.cache().current_user().name.to_string())
                .icon_url(ctx.cache().current_user().face()),
        );

    Ok(embed)
}
