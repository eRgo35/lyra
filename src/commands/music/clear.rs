use poise::CreateReply;

use super::queue_edit::{guild_queue, reply_error};
use crate::{commands::embeds::embed, Context, Error};

/// Empties the queue but keeps the current track playing
#[poise::command(prefix_command, slash_command, guild_only, category = "Music")]
pub async fn clear(ctx: Context<'_>) -> Result<(), Error> {
    let Some(queue) = guild_queue(ctx).await? else {
        return Ok(());
    };
    // drain(1..) would panic on an empty queue.
    let removed = queue.modify_queue(|q| q.drain(1.min(q.len())..).collect::<Vec<_>>());
    if removed.is_empty() {
        return reply_error(ctx, "There are no upcoming tracks!").await;
    }

    let guild_id = ctx.guild_id().unwrap();
    for track in &removed {
        // Removed for good, even with `repeat all` on.
        ctx.data().repeat_all.skip_once(guild_id, track);
        let _ = track.stop();
    }

    let description = format!("Removed {} upcoming tracks.", removed.len());
    ctx.send(CreateReply::default().embed(embed(ctx, "Cleared!", &description, "").await?))
        .await?;
    Ok(())
}
