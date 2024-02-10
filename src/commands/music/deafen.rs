use crate::{Context, Error};

#[poise::command(prefix_command, slash_command)]
pub async fn deafen(ctx: Context<'_>) -> Result<(), Error> {
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

    if handler.is_deaf() {
        if let Err(err) = handler.deafen(false).await {
            ctx.say(format!("Failed: {:?}", err)).await?;
        }

        ctx.say("Undeafened").await?;
    } else {
        if let Err(err) = handler.deafen(true).await {
            ctx.say(format!("Failed: {:?}", err)).await?;
        }

        ctx.say("Deafened").await?;
    }

    Ok(())
}
