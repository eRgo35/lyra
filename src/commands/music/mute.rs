use crate::{commands::embeds::{error_embed, embed, fail}, Context, Error};
use poise::CreateReply;

/// Mutes itself while in a voice channel; \
/// aliases: mute, unmute, shhh
#[poise::command(
    prefix_command, 
    slash_command,
    aliases("shhh", "unmute"),
    category = "Music"
)]
pub async fn mute(
    ctx: Context<'_>
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let manager = songbird::get(&ctx.serenity_context())
        .await
        .expect("Songbird client placed at init")
        .clone();

    let handler_lock = match manager.get(guild_id) {
        Some(handler) => handler,
        None => { 
            let msg = "I am not in a voice channel!";
            ctx.send(
                CreateReply::default().embed(error_embed(ctx, msg).await.unwrap())
            ).await?;
            return Ok(());
        }
    };

    let mut handler = handler_lock.lock().await;

    if handler.is_mute() { 
        if let Err(err) = handler.mute(false).await {
            fail(ctx, err.to_string()).await.unwrap();
        }
 
        ctx.send(
            CreateReply::default().embed(embed(ctx, "Unmuted!", "", "").await.unwrap())
        ).await?;
    } else {
        if let Err(err) = handler.mute(true).await {
            fail(ctx, err.to_string()).await.unwrap();
        }
 
        ctx.send(
            CreateReply::default().embed(embed(ctx, "Muted!", "", "").await.unwrap())
        ).await?;
    }

    Ok(())
}
