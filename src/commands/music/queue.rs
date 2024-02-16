use std::time::Duration;

use crate::commands::music::metadata::Metadata;
use crate::{commands::embeds::error_embed, Context, Error};
use poise::serenity_prelude::CreateEmbed;
use poise::CreateReply;
use serenity::{
    builder::{CreateEmbedAuthor, CreateEmbedFooter},
    model::{Colour, Timestamp},
};
use songbird::input::AuxMetadata;

/// Shows next tracks in queue; \
/// aliases: queue, q
#[poise::command(prefix_command, slash_command, aliases("q"), category = "Music")]
pub async fn queue(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let manager = songbird::get(&ctx.serenity_context())
        .await
        .expect("Songbird client placed at init")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        let mut queue_res = String::from("");

        for (index, song) in queue.current_queue().iter().enumerate() {
            let meta_typemap = song.typemap().read().await;
            let metadata = meta_typemap.get::<Metadata>().unwrap();
            let AuxMetadata {
                title,
                artist,
                duration,
                ..
            } = metadata;

            let duration_minutes = duration.unwrap_or(Duration::new(0, 0)).clone().as_secs() / 60;
            let duration_seconds = duration.unwrap_or(Duration::new(0, 0)).clone().as_secs() % 60;

            // println!("{:?}", metadata.clone());

            queue_res.push_str(&format!(
                "{}. {} - {} [{:02}:{:02}] \n",
                index,
                title.as_ref().unwrap(),
                artist.as_ref().unwrap(),
                duration_minutes,
                duration_seconds
            ));
        }

        ctx.send(CreateReply::default().embed(embed(ctx, queue_res).await.unwrap()))
            .await?;
    } else {
        let msg = "I am not in a voice channel!";
        ctx.send(CreateReply::default().embed(error_embed(ctx, msg).await.unwrap()))
            .await?;
    }

    Ok(())
}

async fn embed(ctx: Context<'_>, queue: String) -> Result<CreateEmbed, Error> {
    let title = "Now playing";
    let timestamp = Timestamp::now();

    let embed = CreateEmbed::default()
        .author(CreateEmbedAuthor::new("Queue").icon_url(ctx.author().clone().face()))
        .colour(Colour::from_rgb(255, 58, 97))
        .title(title)
        .description(queue)
        .timestamp(timestamp)
        .footer(
            CreateEmbedFooter::new(ctx.cache().current_user().name.to_string())
                .icon_url(ctx.cache().current_user().face()),
        );

    Ok(embed)
}
