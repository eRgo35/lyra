use crate::{commands::embeds::{error_embed, embed, fail}, Context, Error};
use poise::CreateReply;

/// Deafens itself while in a voice channel; \
/// aliases: deafen, undeaden, shuush
#[poise::command(
    prefix_command,
    slash_command,
    aliases("shuush", "undeafen"),
    category = "Music"
)]
pub async fn deafen(
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

    if handler.is_deaf() {
        if let Err(err) = handler.deafen(false).await {
            fail(ctx, err.to_string()).await.unwrap();
        }

        ctx.send(
            CreateReply::default().embed(embed(ctx, "Undeafened!", "", "").await.unwrap())
        ).await?;
    } else {
        if let Err(err) = handler.deafen(true).await {  
            fail(ctx, err.to_string()).await.unwrap();
        }
       
        ctx.send(
            CreateReply::default().embed(embed(ctx, "Deafened!", "", "").await.unwrap())
        ).await?;
    }

    Ok(())
}
