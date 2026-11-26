use crate::{
    commands::embeds::{embed, error_embed},
    Context, Error,
};
use poise::CreateReply;

use std::collections::HashMap;
use std::sync::Mutex;

use poise::serenity_prelude::GuildId;

/// The highest volume, in percent: beyond it the sound just distorts.
const MAX_PERCENT: f32 = 200.0;

/// Each guild's volume (1.0 = 100%), kept for every track that plays after it is set.
#[derive(Default)]
pub struct Volumes(Mutex<HashMap<GuildId, f32>>);

impl Volumes {
    pub fn get(&self, guild_id: GuildId) -> f32 {
        self.0
            .lock()
            .unwrap()
            .get(&guild_id)
            .copied()
            .unwrap_or(1.0)
    }

    pub fn set(&self, guild_id: GuildId, volume: f32) {
        let mut volumes = self.0.lock().unwrap();
        if volume == 1.0 {
            volumes.remove(&guild_id);
        } else {
            volumes.insert(guild_id, volume);
        }
    }
}

/// `percent` as a volume, or `None` if out of range.
fn parse_percent(percent: f32) -> Option<f32> {
    (percent.is_finite() && (0.0..=MAX_PERCENT).contains(&percent)).then_some(percent / 100.0)
}

/// Changes the volume (0–200%) for this and the following tracks; 100% uses the least CPU
#[poise::command(prefix_command, slash_command, guild_only, category = "Music")]
pub async fn volume(
    ctx: Context<'_>,
    #[description = "Volume in percent, 0–200 (100 = normal)"] volume: f32,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let Some(level) = parse_percent(volume) else {
        let msg = format!("Volume must be between 0 and {MAX_PERCENT}%!");
        ctx.send(CreateReply::default().embed(error_embed(ctx, &msg).await?))
            .await?;
        return Ok(());
    };

    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird client placed at init")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();

        let Some(track) = queue.current() else {
            let msg = "Nothing is playing!";
            ctx.send(CreateReply::default().embed(error_embed(ctx, msg).await?))
                .await?;
            return Ok(());
        };
        let _ = track.set_volume(level);
        ctx.data().volumes.set(guild_id, level);

        ctx.send(
            CreateReply::default().embed(
                embed(
                    ctx,
                    "Volume changed",
                    "",
                    &format!("Set volume to {}%", volume),
                )
                .await?,
            ),
        )
        .await?;
    } else {
        let msg = "I am not in a voice channel!";
        ctx.send(CreateReply::default().embed(error_embed(ctx, msg).await?))
            .await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{parse_percent, Volumes};
    use poise::serenity_prelude::GuildId;

    #[test]
    fn percent_is_limited() {
        assert_eq!(parse_percent(50.0), Some(0.5));
        assert_eq!(parse_percent(0.0), Some(0.0));
        assert_eq!(parse_percent(200.0), Some(2.0));
        for bad in [-1.0, 200.5, 5000.0, f32::NAN, f32::INFINITY] {
            assert_eq!(parse_percent(bad), None, "{bad}");
        }
    }

    #[test]
    fn volume_is_per_guild_and_defaults_to_full() {
        let volumes = Volumes::default();
        let (a, b) = (GuildId::new(1), GuildId::new(2));
        assert_eq!(volumes.get(a), 1.0);
        volumes.set(a, 0.3);
        assert_eq!(volumes.get(a), 0.3);
        assert_eq!(volumes.get(b), 1.0);
        volumes.set(a, 1.0);
        assert_eq!(volumes.get(a), 1.0);
    }
}
