use serenity::framework::standard::macros::command;
use serenity::framework::standard::CommandResult;
use serenity::model::prelude::*;
use serenity::prelude::*;

// use crate::commands::misc::check_msg;

#[command]
#[only_in(guilds)]
async fn queue(_ctx: &Context, _msg: &Message) -> CommandResult {
    Ok(())
}