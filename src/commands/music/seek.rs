use std::time::Duration;

use crate::{
    commands::embeds::{embed, error_embed},
    Context, Error,
};
use poise::CreateReply;
use songbird::input::AuxMetadata;
use tracing::warn;

/// Seeks the current track: `90` or `1:30` jumps there, `+30` / `-15` moves by seconds
#[poise::command(prefix_command, slash_command, guild_only, category = "Music")]
pub async fn seek(
    ctx: Context<'_>,
    #[description = "Position (90, 1:30) or offset (+30, -15)"] position: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird client placed at init")
        .clone();

    let Some(handler_lock) = manager.get(guild_id) else {
        return reply_error(ctx, "I am not in a voice channel!").await;
    };

    let Some(target) = SeekTarget::parse(&position) else {
        return reply_error(
            ctx,
            "Use a position like `90` or `1:30`, or an offset like `+30` or `-15`.",
        )
        .await;
    };

    let Some(track) = handler_lock.lock().await.queue().current() else {
        return reply_error(ctx, "Nothing is playing!").await;
    };

    let Ok(info) = track.get_info().await else {
        return reply_error(ctx, "The current track has already finished!").await;
    };

    let destination = target.resolve(info.position);

    if let Some(length) = track.data::<AuxMetadata>().duration {
        if destination >= length {
            let msg = format!("The track is only {} long!", format_time(length));
            return reply_error(ctx, &msg).await;
        }
    }

    match track.seek(destination).result_async().await {
        Ok(position) => {
            ctx.send(
                CreateReply::default().embed(
                    embed(
                        ctx,
                        "Track seeked!",
                        &format!("Now at {}", format_time(position)),
                        "",
                    )
                    .await?,
                ),
            )
            .await?;
        }
        Err(e) => {
            warn!("Seek to {destination:?} failed: {e}");
            reply_error(ctx, "This track cannot be seeked!").await?;
        }
    }

    Ok(())
}

async fn reply_error(ctx: Context<'_>, msg: &str) -> Result<(), Error> {
    ctx.send(CreateReply::default().embed(error_embed(ctx, msg).await?))
        .await?;
    Ok(())
}

#[derive(Debug, PartialEq)]
enum SeekTarget {
    To(Duration),
    Forward(Duration),
    Back(Duration),
}

impl SeekTarget {
    /// `90`, `1:30`, `1:02:03` are absolute; a leading `+` or `-` makes them relative.
    fn parse(input: &str) -> Option<Self> {
        let input = input.trim();
        let (make, rest): (fn(Duration) -> Self, &str) = match input.as_bytes().first()? {
            b'+' => (Self::Forward, &input[1..]),
            b'-' => (Self::Back, &input[1..]),
            _ => (Self::To, input),
        };

        let parts = rest
            .split(':')
            .map(|part| part.parse::<u64>().ok())
            .collect::<Option<Vec<_>>>()?;

        let seconds = match parts[..] {
            [s] => s,
            [m, s] if s < 60 => m.checked_mul(60)?.checked_add(s)?,
            [h, m, s] if m < 60 && s < 60 => {
                h.checked_mul(3600)?.checked_add(m * 60)?.checked_add(s)?
            }
            _ => return None,
        };

        Some(make(Duration::from_secs(seconds)))
    }

    fn resolve(&self, current: Duration) -> Duration {
        match *self {
            Self::To(position) => position,
            Self::Forward(offset) => current.saturating_add(offset),
            Self::Back(offset) => current.saturating_sub(offset),
        }
    }
}

fn format_time(duration: Duration) -> String {
    let seconds = duration.as_secs();
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}

#[cfg(test)]
mod tests {
    use super::format_time;
    use super::SeekTarget::{self, Back, Forward, To};
    use std::time::Duration;

    fn secs(s: u64) -> Duration {
        Duration::from_secs(s)
    }

    #[test]
    fn parses_absolute_and_relative_forms() {
        assert_eq!(SeekTarget::parse("90"), Some(To(secs(90))));
        assert_eq!(SeekTarget::parse(" 1:30 "), Some(To(secs(90))));
        assert_eq!(SeekTarget::parse("1:02:03"), Some(To(secs(3723))));
        assert_eq!(SeekTarget::parse("+30"), Some(Forward(secs(30))));
        assert_eq!(SeekTarget::parse("-1:00"), Some(Back(secs(60))));
        assert_eq!(SeekTarget::parse("0"), Some(To(secs(0))));
    }

    #[test]
    fn rejects_malformed_input() {
        for input in [
            "", "+", "abc", "1:60", "1::2", "1:2:3:4", "--5", "+-5", "1.5",
        ] {
            assert_eq!(SeekTarget::parse(input), None, "{input:?}");
        }
    }

    #[test]
    fn resolves_relative_to_current_position_and_clamps_at_zero() {
        assert_eq!(To(secs(90)).resolve(secs(10)), secs(90));
        assert_eq!(Forward(secs(30)).resolve(secs(10)), secs(40));
        assert_eq!(Back(secs(15)).resolve(secs(10)), secs(0));
    }

    #[test]
    fn formats_minutes_and_seconds() {
        assert_eq!(format_time(secs(0)), "00:00");
        assert_eq!(format_time(secs(90)), "01:30");
        assert_eq!(format_time(secs(3723)), "62:03");
    }
}
