use crate::{
    commands::embeds::{error_embed, with_detail},
    Context, Error,
};

use poise::serenity_prelude::{
    Color, CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter, Timestamp,
};
use poise::CreateReply;
use songbird::input::AuxMetadata;
use songbird::tracks::Track;
use std::sync::Arc;
use std::time::Duration;

use crate::http::HttpKey;

/// Plays one of available audio effects
#[poise::command(prefix_command, slash_command, guild_only, category = "Music")]
pub async fn effect(
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

    if manager.get(guild_id).is_none() {
        let msg = "I am not in a voice channel!";
        ctx.send(CreateReply::default().embed(error_embed(ctx, msg).await?))
            .await?;
        return Ok(());
    }

    if is_query {
        song = format!("ytsearch:{}", song);
    }

    // Resolved before locking the call: yt-dlp can take seconds, and other commands
    // (and songbird itself) need the lock meanwhile.
    let src = match crate::http::lookup(http_client, song).await {
        Ok(src) => src,
        Err(e) => {
            let msg = with_detail("Could not load that track!", &e);
            ctx.send(CreateReply::default().embed(error_embed(ctx, &msg).await?))
                .await?;
            return Ok(());
        }
    };
    let metadata = src.metadata().cloned().unwrap_or_default();
    ctx.send(CreateReply::default().embed(generate_embed(ctx, metadata.clone()).await?))
        .await?;

    // The bot may have left while the track was loading.
    if let Some(handler_lock) = manager.get(guild_id) {
        let _ = handler_lock
            .lock()
            .await
            .play(Track::new_with_data(src.into(), Arc::new(metadata)));
    }

    Ok(())
}

async fn generate_embed(ctx: Context<'_>, metadata: AuxMetadata) -> Result<CreateEmbed, Error> {
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
        .author(CreateEmbedAuthor::new("Playing an effect!").icon_url(ctx.author().clone().face()))
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
        .timestamp(timestamp)
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
