use crate::{Context, Error};

#[poise::command(prefix_command, slash_command)]
pub async fn queue(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    
    let manager = songbird::get(&ctx.serenity_context())
        .await
        .expect("Songbird client placed at init")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        let mut queue_res = String::from("Queue: \n");

        for (i, song) in queue.current_queue().iter().enumerate() {
            queue_res.push_str(&format!(
                "{}. {} - {}\n",
                i + 1,
                song.uuid(),
                "Artist"
                // song.metadata().artist.clone().unwrap_or_else(|| String::from("Unknown"))
        ));
        }
        
        ctx.say(queue_res).await?;

    } else {
        ctx.say("Not in a voice channel!").await?;
    }

    Ok(())
}
