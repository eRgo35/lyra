use crate::{Context, Error};

#[poise::command(prefix_command, slash_command)]
pub async fn resume(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let manager = songbird::get(&ctx.serenity_context())
        .await
        .expect("Songbird client placed at init")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        let _ = queue.resume();

        ctx.say(format!("Song resumed.")).await?;
    } else {
        ctx.say("Not in a voice channel to play in").await?;
    }

    Ok(())
}
