use once_cell::sync::Lazy;
use poise::CreateReply;
use std::sync::Mutex;

use crate::{commands::embeds::embed, Context, Error};

pub static PROCESS_UPTIME: Lazy<Mutex<std::time::SystemTime>> =
    Lazy::new(|| Mutex::new(std::time::SystemTime::now()));

/// Checks how long the bot has been running
#[poise::command(prefix_command, slash_command, category = "Tools")]
pub async fn uptime(ctx: Context<'_>) -> Result<(), Error> {
    let start = *PROCESS_UPTIME.lock().unwrap();
    let uptime = std::time::SystemTime::now().duration_since(start).unwrap();

    let message = uptime_message(uptime);

    ctx.send(CreateReply::default().embed(embed(ctx, "Uptime", "", &message).await.unwrap()))
        .await?;

    Ok(())
}

/// "I have been awake for …", naming only the largest non-zero unit.
fn uptime_message(uptime: std::time::Duration) -> String {
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

    message
}

#[cfg(test)]
mod tests {
    use super::uptime_message;
    use std::time::Duration;

    #[test]
    fn names_the_largest_non_zero_unit() {
        let msg = |s| uptime_message(Duration::from_secs(s));
        assert_eq!(msg(3 * 86400 + 5), "I have been awake for 3 days!");
        assert_eq!(msg(2 * 3600 + 59), "I have been awake for 2 hours!");
        assert_eq!(msg(61), "I have been awake for 1 minutes!");
        assert_eq!(msg(7), "I have been awake for 7 seconds!");
    }

    #[test]
    fn zero_uptime_lists_all_units() {
        assert_eq!(
            uptime_message(Duration::ZERO),
            "I have been awake for 0 days, 0 hours, 0 minutes and 0 seconds!"
        );
    }
}
