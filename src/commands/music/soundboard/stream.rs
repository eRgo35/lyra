use crate::{commands::embeds::error_embed, Context, Error};

use poise::serenity_prelude::{
    Color, CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter, Timestamp,
};
use poise::CreateReply;
use songbird::events::TrackEvent;
use songbird::input::AuxMetadata;
use songbird::input::{Compose, YoutubeDl};
use std::time::Duration;

use crate::commands::music::notifier::TrackErrorNotifier;
use crate::http::HttpKey;

/// Hijacks output and plays audio; \
/// search by query or paste an url; \
/// aliases: stream, override, hijack
#[poise::command(
    prefix_command,
    slash_command,
    aliases("override", "hijack"),
    category = "Music"
)]
pub async fn stream(
    ctx: Context<'_>,
    #[description = "Shall output pause?"]
    #[flag]
    pause: bool,
    #[description = "Provide a query or an url"]
    #[rest]
    mut song: String,
) -> Result<(), Error> {
    let is_query = !song.starts_with("http");

    let guild_id = ctx.guild_id().unwrap();

    let http_client = {
        let data = ctx.serenity_context().data.read().await;
        data.get::<HttpKey>()
            .cloned()
            .expect("Guaranteed to exist in the typemap.")
    };

    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird Voice placed at init")
        .clone();

    if pause {
        if let Some(handler_lock) = manager.get(guild_id) {
            let handler = handler_lock.lock().await;
            let queue = handler.queue();
            let _ = queue.pause();
        }
    }

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;

        handler.add_global_event(TrackEvent::Error.into(), TrackErrorNotifier);

        if is_query {
            song = format!("ytsearch:{}", song);
        }

        let src = YoutubeDl::new_ytdl_like("yt-dlp", http_client, song);
        let embed = generate_embed(ctx, src.clone()).await;
        let response = CreateReply::default().embed(embed.unwrap());
        ctx.send(response).await?;

        let _ = handler.play_input(src.clone().into());
    } else {
        let msg = "I am not in a voice channel!";
        ctx.send(CreateReply::default().embed(error_embed(ctx, msg).await.unwrap()))
            .await?;
    }

    Ok(())
}

async fn generate_embed(ctx: Context<'_>, src: YoutubeDl) -> Result<CreateEmbed, Error> {
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

    let description = "Playing now!";

    let embed = CreateEmbed::default()
        .author(
            CreateEmbedAuthor::new("Audio output hijacked!").icon_url(ctx.author().clone().face()),
        )
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
        .timestamp(timestamp)
        .footer(
            CreateEmbedFooter::new(ctx.cache().current_user().name.to_string())
                .icon_url(ctx.cache().current_user().face()),
        );

    Ok(embed)
}
