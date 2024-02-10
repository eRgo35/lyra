use crate::{Context, Error};

#[poise::command(prefix_command, slash_command)]
pub async fn stop(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    
    let manager = songbird::get(&ctx.serenity_context())
        .await
        .expect("Songbird client placed at init")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        queue.stop();

        ctx.say("Playback stopped!").await?;
    } else {
        ctx.say("Not in a voice channel!").await?;
    }
    
    Ok(())
}
