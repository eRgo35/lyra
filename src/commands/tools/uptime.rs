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

    ctx.send(CreateReply::default().embed(embed(ctx, "Uptime", "", &message).await?))
        .await?;

    Ok(())
}

/// "I have been awake for …", naming only the largest non-zero unit.
fn uptime_message(uptime: std::time::Duration) -> String {
    let secs = uptime.as_secs();
    let (amount, unit) = match secs {
        86400.. => (secs / 86400, "day"),
        3600.. => (secs / 3600, "hour"),
        60.. => (secs / 60, "minute"),
        _ => (secs, "second"),
    };
    let plural = if amount == 1 { "" } else { "s" };
    format!("I have been awake for {amount} {unit}{plural}!")
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
        assert_eq!(msg(61), "I have been awake for 1 minute!");
        assert_eq!(msg(86400), "I have been awake for 1 day!");
        assert_eq!(msg(7), "I have been awake for 7 seconds!");
    }

    #[test]
    fn zero_uptime_is_zero_seconds() {
        assert_eq!(
            uptime_message(Duration::ZERO),
            "I have been awake for 0 seconds!"
        );
    }
}
