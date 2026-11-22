use poise::CreateReply;
use songbird::input::AuxMetadata;

use super::queue_edit::{guild_queue, reply_error, upcoming};
use crate::{commands::embeds::embed, Context, Error};

/// Removes a track from the queue (position as shown by queue); aliases: remove, rm
#[poise::command(
    prefix_command,
    slash_command,
    aliases("rm"),
    guild_only,
    category = "Music"
)]
pub async fn remove(
    ctx: Context<'_>,
    #[description = "Queue position to remove (1 = next track)"] position: u32,
) -> Result<(), Error> {
    let Some(queue) = guild_queue(ctx).await? else {
        return Ok(());
    };
    let removed = queue.modify_queue(|q| {
        upcoming(position as usize, q.len()).map(|p| q.remove(p).expect("checked"))
    });
    let track = match removed {
        Ok(track) => track,
        Err(msg) => return reply_error(ctx, &msg).await,
    };

    // Removed for good, even with `repeat all` on.
    ctx.data()
        .repeat_all
        .skip_once(ctx.guild_id().unwrap(), &track);
    let _ = track.stop();

    let title = track
        .data::<AuxMetadata>()
        .title
        .clone()
        .unwrap_or("Unknown Title".to_string());
    ctx.send(CreateReply::default().embed(embed(ctx, "Removed", "", &title).await?))
        .await?;
    Ok(())
}
