use crate::{commands::embeds::{error_embed, embed}, Context, Error};
use poise::CreateReply;

/// Skips the currently playing song
#[poise::command(
    prefix_command,
    slash_command,
    category = "Music"
)]
pub async fn skip(
    ctx: Context<'_>
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let manager = songbird::get(&ctx.serenity_context())
        .await
        .expect("Songbird client placed at init")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        let _ = queue.skip();

        ctx.send(
            CreateReply::default().embed(embed(ctx, "Skipped!", "Next song: {song}", &format!("Songs left in queue: {}", queue.len())).await.unwrap())
        ).await?;
    } else {
        let msg = "I am not in a voice channel!";
        ctx.send(
            CreateReply::default().embed(error_embed(ctx, msg).await.unwrap())
        ).await?;
    }

    Ok(())
}
