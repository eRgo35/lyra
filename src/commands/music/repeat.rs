use crate::{
    commands::embeds::{embed, error_embed},
    Context, Error,
};
use nonmax::NonMaxU32;
use poise::CreateReply;

/// Repeat: `one` loops this track, `all` the queue, `<n>` n times, `off` stops; \
/// aliases: repeat, loop
#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    aliases("loop", "while", "for"),
    category = "Music"
)]
pub async fn repeat(
    ctx: Context<'_>,
    #[description = "off, one, all, or how many times to loop this track"] mode: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird client placed at init")
        .clone();

    let Some(mode) = Mode::parse(&mode) else {
        return reply_error(ctx, "Use `off`, `one`, `all` or a number of times.").await;
    };

    let Some(handler_lock) = manager.get(guild_id) else {
        return reply_error(ctx, "I am not in a voice channel!").await;
    };
    let current = handler_lock.lock().await.queue().current();
    let repeat_all = &ctx.data().repeat_all;

    // `all` works on an empty queue (it applies to what is played next); the rest need a track.
    let (title, description) = match (mode, current) {
        (Mode::All, current) => {
            if let Some(track) = current {
                let _ = track.disable_loop();
            }
            repeat_all.set(guild_id, true);
            (
                "Repeating the queue!",
                "Finished tracks go back to the end.",
            )
        }
        (Mode::Off, current) => {
            if let Some(track) = current {
                let _ = track.disable_loop();
            }
            repeat_all.set(guild_id, false);
            ("Repeat off!", "")
        }
        (_, None) => return reply_error(ctx, "Nothing is playing!").await,
        (Mode::One, Some(track)) => {
            repeat_all.set(guild_id, false);
            if track.enable_loop().is_err() {
                return reply_error(ctx, "The current track has already finished!").await;
            }
            ("Song looped forever!", "You definitely love this song!")
        }
        (Mode::Times(times), Some(track)) => {
            repeat_all.set(guild_id, false);
            if track.loop_for(times).is_err() {
                return reply_error(ctx, "The current track has already finished!").await;
            }
            let msg = format!("Song looped {times} more times!");
            ctx.send(CreateReply::default().embed(embed(ctx, &msg, "", "").await?))
                .await?;
            return Ok(());
        }
    };

    ctx.send(CreateReply::default().embed(embed(ctx, title, description, "").await?))
        .await?;

    Ok(())
}

async fn reply_error(ctx: Context<'_>, msg: &str) -> Result<(), Error> {
    ctx.send(CreateReply::default().embed(error_embed(ctx, msg).await?))
        .await?;
    Ok(())
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum Mode {
    Off,
    One,
    All,
    Times(NonMaxU32),
}

impl Mode {
    fn parse(input: &str) -> Option<Self> {
        match input.trim().to_lowercase().as_str() {
            "off" | "0" => Some(Self::Off),
            "one" => Some(Self::One),
            "all" => Some(Self::All),
            n => n
                .parse::<u32>()
                .ok()
                .and_then(NonMaxU32::new)
                .map(Self::Times),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Mode;
    use nonmax::NonMaxU32;

    #[test]
    fn parses_modes_and_counts() {
        assert_eq!(Mode::parse("off"), Some(Mode::Off));
        assert_eq!(Mode::parse("0"), Some(Mode::Off));
        assert_eq!(Mode::parse(" One "), Some(Mode::One));
        assert_eq!(Mode::parse("ALL"), Some(Mode::All));
        assert_eq!(
            Mode::parse("3"),
            Some(Mode::Times(NonMaxU32::new(3).unwrap()))
        );
    }

    #[test]
    fn rejects_other_input() {
        for input in ["", "forever", "-1", "1.5", &u32::MAX.to_string()] {
            assert_eq!(Mode::parse(input), None, "{input:?}");
        }
    }
}
