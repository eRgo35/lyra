use crate::commands::music::metadata::Metadata;
use std::time::Duration;

use crate::{
    commands::embeds::{embed, error_embed},
    Context, Error,
};
use poise::serenity_prelude::{
    Color, CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter, Timestamp,
};
use poise::CreateReply;
use songbird::{input::AuxMetadata, tracks::TrackHandle};

/// Skips the currently playing song; \
/// aliases: skip, :skipper:
#[poise::command(prefix_command, slash_command, aliases("skipper:"), category = "Music")]
pub async fn skip(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird client placed at init")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        let _ = queue.clone().skip();
        let track_raw = queue.clone().current_queue();
        let track = track_raw.get(1);
        let queue_length = queue.len() - 1;

        let mut response = CreateReply::default().embed(
            embed(ctx, "Skipped!", "The queue is empty!", "")
                .await
                .unwrap(),
        );

        if let Some(track) = track {
            response = CreateReply::default().embed(
                generate_embed(ctx, track.clone(), queue_length)
                    .await
                    .unwrap(),
            );
        };

        ctx.send(response).await?;
    } else {
        let msg = "I am not in a voice channel!";
        ctx.send(CreateReply::default().embed(error_embed(ctx, msg).await.unwrap()))
            .await?;
    }

    Ok(())
}

async fn generate_embed(
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

    let description = format!("Song skipped! Queue length is {}", queue_length);

    let embed = CreateEmbed::default()
        .author(CreateEmbedAuthor::new("Skipped!").icon_url(ctx.author().clone().face()))
        .colour(Color::from_rgb(255, 58, 97))
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
