use serenity::framework::standard::macros::command;
use serenity::framework::standard::CommandResult;
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::commands::misc::check_msg;

#[command]
#[only_in(guilds)]
async fn queue(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    
    let manager = songbird::get(ctx)
        .await
        .expect("Client placed at init")
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
        
        check_msg(
            msg.channel_id
                .say(&ctx.http, queue_res)
                .await,
        );

    } else {
        check_msg(
            msg.channel_id
                .say(&ctx.http, "Not in a voice channel!")
                .await,
        );
    }

    Ok(())
}
