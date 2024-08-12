use crate::commands::music::metadata::Metadata;
use crate::commands::music::notifier::TrackErrorNotifier;
use crate::{commands::embeds::error_embed, Context, Error};

use poise::serenity_prelude::{
    Color, CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter, Timestamp,
};
use poise::CreateReply;
use regex::Regex as Regex_Classic;
use reqwest::Client;
use serenity::all::GuildId;
use songbird::events::TrackEvent;
use songbird::input::AuxMetadata;
use songbird::input::{Compose, YoutubeDl};
use songbird::tracks::TrackQueue;
use songbird::Call;
use spotify_parser;
use std::collections::VecDeque;
use std::process::Command;
use std::time::Duration;

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

            rest_playlist = handle_play(ctx, song, handler, http_client.clone())
                .await
                .unwrap();
        } else {
            let msg = "Failed to join the voice channel!";
            ctx.send(CreateReply::default().embed(error_embed(ctx, msg).await.unwrap()))
                .await?;
        }
    } else {
        let handler = manager.get(guild_id).unwrap();
        let mut handler = handler.lock().await;
        handler.add_global_event(TrackEvent::Error.into(), TrackErrorNotifier);
        rest_playlist = handle_play(ctx, song, handler, http_client.clone())
            .await
            .unwrap();
    }

    handle_playlist(rest_playlist, manager, guild_id, http_client)
        .await
        .unwrap();

    Ok(())
}

async fn handle_play<'a>(
    ctx: Context<'a>,
    song: String,
    mut handler: tokio::sync::MutexGuard<'a, Call>,
    http_client: Client,
) -> Result<VecDeque<String>, Error> {
    let mut results = parse_data(song).await;

    let src: YoutubeDl =
        YoutubeDl::new_ytdl_like("yt-dlp", http_client, results.pop_front().unwrap());

    ctx.send(
        CreateReply::default().embed(
            generate_embed(ctx, src.clone(), handler.queue(), results.clone())
                .await
                .unwrap(),
        ),
    )
    .await?;

    let aux_metadata = src.clone().aux_metadata().await.unwrap();

    handler
        .enqueue_input(src.clone().into())
        .await
        .typemap()
        .write()
        .await
        .insert::<Metadata>(aux_metadata);

    Ok(results)
}

async fn parse_data(data: String) -> VecDeque<String> {
    let tracks = spotify_parser::retrieve_async_url(&data)
        .await
        .unwrap_or(vec![data])
        .iter()
        .flat_map(|track| {
            if track.contains("?list=") {
                let regex_youtube = Regex_Classic::new(
                    r#""url": "(https://www.youtube.com/watch\?v=[A-Za-z0-9]{11})""#,
                )
                .unwrap();

                let list = Command::new("yt-dlp")
                    .args(["-j", "--flat-playlist", track])
                    .output()
                    .expect("Failed to execute process")
                    .stdout;
                let list = String::from_utf8(list).unwrap();

                regex_youtube
                    .captures_iter(&list)
                    .map(|capture| capture.get(1).unwrap().as_str().to_string())
                    .collect::<Vec<String>>()
            } else if track.starts_with("http") {
                vec![track.clone()]
            } else {
                vec![format!("ytsearch:{}", track)]
            }
        })
        .collect();

    tracks
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

            let aux_metadata = src.clone().aux_metadata().await.unwrap();

            handler
                .enqueue_input(src.clone().into())
                .await
                .typemap()
                .write()
                .await
                .insert::<Metadata>(aux_metadata);
        }
    }

    Ok(())
}

async fn generate_embed(
    ctx: Context<'_>,
    src: YoutubeDl,
    queue: &TrackQueue,
    results: VecDeque<String>,
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
        .timestamp(Timestamp::now())
        .footer(
            CreateEmbedFooter::new(ctx.cache().current_user().name.to_string())
                .icon_url(ctx.cache().current_user().face()),
        );

    Ok(embed)
}
