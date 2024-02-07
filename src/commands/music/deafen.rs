use serenity::framework::standard::macros::command;
use serenity::framework::standard::CommandResult;
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::commands::misc::check_msg;

#[command]
#[only_in(guilds)]
async fn deafen(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();

    let manager = songbird::get(ctx)
        .await
        .expect("Client placed at init")
        .clone();

    let handler_lock = match manager.get(guild_id) {
        Some(handler) => handler,
        None => {
            check_msg(msg.reply(ctx, "Not in a voice channel").await);

            return Ok(());
        }
    };

    let mut handler = handler_lock.lock().await;

    if handler.is_deaf() {
        if let Err(err) = handler.deafen(false).await {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, format!("Failed: {:?}", err))
                    .await,
            );
        }

        check_msg(msg.channel_id.say(&ctx.http, "Undeafened").await);
    } else {
        if let Err(err) = handler.deafen(true).await {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, format!("Failed: {:?}", err))
                    .await,
            );
        }

        check_msg(msg.channel_id.say(&ctx.http, "Deafened").await);
    }

    Ok(())
}
