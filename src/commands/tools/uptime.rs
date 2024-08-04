use once_cell::sync::Lazy;
use poise::CreateReply;
use std::sync::Mutex;

use crate::{commands::embeds::embed, Context, Error};

pub static PROCESS_UPTIME: Lazy<Mutex<std::time::SystemTime>> =
    Lazy::new(|| Mutex::new(std::time::SystemTime::now()));

/// Checks how long the bot has been running
#[poise::command(prefix_command, slash_command, category = "Tools")]
pub async fn uptime(ctx: Context<'_>) -> Result<(), Error> {
    let start = PROCESS_UPTIME.lock().unwrap().clone();
    let uptime = std::time::SystemTime::now().duration_since(start).unwrap();

    let (days, hours, minutes, seconds) = (
        uptime.as_secs() / 86400,
        (uptime.as_secs() / 3600) % 24,
        (uptime.as_secs() / 60) % 60,
        uptime.as_secs() % 60,
    );

    let mut message = format!(
        "I have been awake for {} days, {} hours, {} minutes and {} seconds!",
        days, hours, minutes, seconds
    );

    if days != 0 {
        message = format!("I have been awake for {} days!", days);
    }

    if days == 0 && hours != 0 {
        message = format!("I have been awake for {} hours!", hours);
    }

    if days == 0 && hours == 0 && minutes != 0 {
        message = format!("I have been awake for {} minutes!", minutes);
    }

    if days == 0 && hours == 0 && minutes == 0 && seconds != 0 {
        message = format!("I have been awake for {} seconds!", seconds);
    }

    ctx.send(CreateReply::default().embed(embed(ctx, "Uptime", "", &message).await.unwrap()))
        .await?;

    Ok(())
}
