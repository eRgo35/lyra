use std::time::Duration;

use poise::CreateReply;
use songbird::input::AuxMetadata;

use super::seek::format_time;
use crate::{
    commands::embeds::{embed, error_embed},
    Context, Error,
};

/// Shows the current track and how far into it we are; aliases: nowplaying, np
#[poise::command(
    prefix_command,
    slash_command,
    aliases("np"),
    guild_only,
    category = "Music"
)]
pub async fn nowplaying(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird client placed at init")
        .clone();

    let current = match manager.get(guild_id) {
        Some(call) => call.lock().await.queue().current(),
        None => None,
    };
    let Some(track) = current else {
        ctx.send(CreateReply::default().embed(error_embed(ctx, "Nothing is playing!").await?))
            .await?;
        return Ok(());
    };

    let metadata = crate::http::with_cached_details(&track.data::<AuxMetadata>());
    let position = track
        .get_info()
        .await
        .map(|info| info.position)
        .unwrap_or_default();

    let title = metadata.title.as_deref().unwrap_or("Unknown Title");
    let artist = metadata.artist.as_deref().unwrap_or("Unknown Artist");
    let description = format!("by {artist}\n\n{}", progress(position, metadata.duration));

    let mut embed = embed(ctx, "Now playing", &description, title).await?;
    if let Some(url) = &metadata.source_url {
        embed = embed.url(url);
    }
    if let Some(thumbnail) = &metadata.thumbnail {
        embed = embed.thumbnail(thumbnail);
    }
    ctx.send(CreateReply::default().embed(embed)).await?;
    Ok(())
}

const BAR_WIDTH: usize = 20;

/// `▬▬▬🔘▬▬▬ 01:02 / 03:45`, or a live marker when the duration is unknown.
fn progress(position: Duration, duration: Option<Duration>) -> String {
    let Some(duration) = duration.filter(|d| !d.is_zero()) else {
        return format!("🔴 Live · {}", format_time(position));
    };
    let position = position.min(duration);
    let filled = (position.as_secs_f64() / duration.as_secs_f64() * BAR_WIDTH as f64) as usize;
    let filled = filled.min(BAR_WIDTH - 1);
    format!(
        "{}🔘{} {} / {}",
        "▬".repeat(filled),
        "▬".repeat(BAR_WIDTH - 1 - filled),
        format_time(position),
        format_time(duration)
    )
}

#[cfg(test)]
mod tests {
    use super::progress;
    use std::time::Duration;

    fn secs(s: u64) -> Duration {
        Duration::from_secs(s)
    }

    #[test]
    fn progress_bar_tracks_the_position() {
        assert_eq!(
            progress(secs(0), Some(secs(200))),
            format!("🔘{} 00:00 / 03:20", "▬".repeat(19))
        );
        assert_eq!(
            progress(secs(100), Some(secs(200))),
            format!("{}🔘{} 01:40 / 03:20", "▬".repeat(10), "▬".repeat(9))
        );
        // Past the end (or exactly at it) the knob stays on the bar.
        assert_eq!(
            progress(secs(500), Some(secs(200))),
            format!("{}🔘 03:20 / 03:20", "▬".repeat(19))
        );
    }

    #[test]
    fn streams_without_a_duration_are_live() {
        assert_eq!(progress(secs(65), None), "🔴 Live · 01:05");
        assert_eq!(progress(secs(65), Some(Duration::ZERO)), "🔴 Live · 01:05");
    }
}
