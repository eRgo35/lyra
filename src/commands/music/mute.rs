use crate::{Context, Error};

#[poise::command(prefix_command, slash_command)]
pub async fn mute(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let manager = songbird::get(&ctx.serenity_context())
        .await
        .expect("Songbird client placed at init")
        .clone();

    let handler_lock = match manager.get(guild_id) {
        Some(handler) => handler,
        None => {
            ctx.say("Not in a voice channel").await?;

            return Ok(());
        }
    };

    let mut handler = handler_lock.lock().await;

    if handler.is_mute() { 
        if let Err(e) = handler.mute(false).await {
            ctx.say(format!("failed: {:?}", e)).await?;
        }

        ctx.say("Unmuted").await?;
    } else {
        if let Err(err) = handler.mute(true).await {
            ctx.say(format!("Failed: {:?}", err)).await?;
        }

        ctx.say("Muted").await?;
    }

    Ok(())
}
