use serenity::framework::standard::macros::command;
use serenity::framework::standard::CommandResult;
use serenity::model::prelude::*;
use serenity::prelude::*;
use songbird::events::TrackEvent;

use crate::commands::{misc::check_msg, music::misc::TrackErrorNotifier};

#[command]
#[only_in(guilds)]
async fn join(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let channel_id = msg.guild(&ctx.cache).unwrap().voice_states.get(&msg.author.id).and_then(|voice_state| voice_state.channel_id);

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            check_msg(msg.reply(ctx, "Not in a voice channel").await);

            return Ok(());
        }
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice placed at init")
        .clone();

    if let Ok(handler_lock) = manager.join(guild_id, connect_to).await {
        let mut handler = handler_lock.lock().await;
        handler.add_global_event(TrackEvent::Error.into(), TrackErrorNotifier);
    }

    Ok(())
}
