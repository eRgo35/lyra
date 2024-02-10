use crate::{Context, Error};
use std::time::SystemTime;

#[poise::command(prefix_command, slash_command)]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    let system_now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap().as_millis() as i64;

    let message_now = ctx.created_at().timestamp_millis();

    let response = format!("Pong! (latency: {} ms)", system_now - message_now);
    ctx.say(response).await?;

    Ok(())
}
