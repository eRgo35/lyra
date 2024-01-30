use serenity::{all::Message, client::Context, framework::standard::{macros::command, CommandResult}};

use crate::commands::misc::check_msg;

#[command]
async fn kashi(ctx: &Context, msg: &Message) -> CommandResult {
    check_msg(msg.reply(ctx, "Kashi lyrics platform integration").await);

    Ok(())
}