//! `AUTO_LEAVE`: leaves a voice channel once the bot has had nothing to do there (an
//! empty queue, or nobody else in the channel) for that long. Off by default: the bot
//! stays until told to `leave`.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::time::Instant;

use poise::serenity_prelude::{Cache, ChannelId, GuildId, UserId};
use songbird::Songbird;
use tracing::{info, warn};

use crate::commands::music::repeat_all::RepeatAll;

/// Parses `AUTO_LEAVE` (`10m`, `1h30m`, ...). Unset, empty, `0` or `off` keeps the bot
/// in its channel for good.
pub fn parse_auto_leave(raw: Option<&str>) -> Result<Option<Duration>, String> {
    let Some(raw) = raw.map(str::trim).filter(|r| !r.is_empty()) else {
        return Ok(None);
    };
    if matches!(raw.to_ascii_lowercase().as_str(), "0" | "off" | "never") {
        return Ok(None);
    }
    crate::reminders::parse_delay(raw)
        .map(|secs| Some(Duration::from_secs(secs)))
        .map_err(|_| {
            format!("Environment variable `AUTO_LEAVE` is not a time like 10m or 1h: {raw:?}")
        })
}

/// Whether the bot has nothing to do in a channel.
fn is_idle(queue_empty: bool, listeners: usize) -> bool {
    queue_empty || listeners == 0
}

/// People (not bots) in `channel`, or `None` if the guild is not cached.
fn listeners(cache: &Cache, guild: GuildId, channel: ChannelId, bot: UserId) -> Option<usize> {
    let guild = cache.guild(guild)?;
    Some(
        guild
            .voice_states
            .values()
            .filter(|v| v.channel_id == Some(channel) && v.user_id != bot)
            .filter(|v| !v.member.as_ref().is_some_and(|m| m.user.bot))
            .count(),
    )
}

/// Checks every call every half minute and leaves those idle for `after`. Runs for the
/// life of the bot.
pub async fn run(
    cache: Arc<Cache>,
    manager: Arc<Songbird>,
    repeat: Arc<RepeatAll>,
    after: Duration,
) {
    let mut idle_since: HashMap<GuildId, Instant> = HashMap::new();
    let mut tick = tokio::time::interval(Duration::from_secs(30).min(after));
    loop {
        // Times are taken at the tick, so idle time is a whole number of ticks and
        // the check after `after` is not missed by a few microseconds.
        let now = tick.tick().await;
        let bot = cache.current_user().id;
        // Collected first: the iterator holds the manager's map while it lives.
        let calls: Vec<_> = manager.iter().collect();
        for (guild, call) in &calls {
            let guild = GuildId::new(guild.0.get());
            let (channel, queue_empty) = {
                let call = call.lock().await;
                (call.current_channel(), call.queue().is_empty())
            };
            let listeners = match channel {
                Some(channel) => {
                    let channel = ChannelId::new(channel.0.get());
                    // Unknown: assume someone is there rather than leave wrongly.
                    listeners(&cache, guild, channel, bot).unwrap_or(1)
                }
                None => 0,
            };
            if !is_idle(queue_empty, listeners) {
                idle_since.remove(&guild);
                continue;
            }
            let since = *idle_since.entry(guild).or_insert(now);
            if now - since < after {
                continue;
            }
            info!("Leaving the voice channel in {guild}: idle for {after:?}");
            repeat.set(guild, false);
            call.lock().await.queue().stop();
            if let Err(e) = manager.remove(guild).await {
                warn!("Could not leave the voice channel in {guild}: {e}");
            }
            idle_since.remove(&guild);
        }
        idle_since.retain(|guild, _| manager.get(*guild).is_some());
    }
}

#[cfg(test)]
mod tests {
    use super::{is_idle, parse_auto_leave};
    use std::time::Duration;

    #[test]
    fn auto_leave_is_off_unless_set() {
        for off in [
            None,
            Some(""),
            Some(" "),
            Some("0"),
            Some("off"),
            Some("Never"),
        ] {
            assert_eq!(parse_auto_leave(off), Ok(None), "{off:?}");
        }
        assert_eq!(
            parse_auto_leave(Some("10m")),
            Ok(Some(Duration::from_secs(600)))
        );
        assert_eq!(
            parse_auto_leave(Some("1h30m")),
            Ok(Some(Duration::from_secs(5400)))
        );
        let err = parse_auto_leave(Some("soon")).unwrap_err();
        assert!(
            err.contains("AUTO_LEAVE") && err.contains("\"soon\""),
            "{err}"
        );
    }

    #[test]
    fn idle_when_nothing_queued_or_nobody_listening() {
        assert!(!is_idle(false, 2));
        assert!(is_idle(true, 2));
        assert!(is_idle(false, 0));
    }
}
