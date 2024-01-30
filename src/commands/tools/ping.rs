use std::time::SystemTime;

use serenity::{all::Message, client::Context, framework::standard::{macros::command, CommandResult}};

use crate::commands::misc::check_msg;

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    let system_now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis() as i64;
    let message_now = msg.timestamp.timestamp_millis();

    // println!("System Time: {} ||| Message Time: {}", system_now, message_now);

    check_msg(msg.reply(ctx, format!("Pong! (latency: {} ms)", system_now - message_now)).await);

    Ok(())
}