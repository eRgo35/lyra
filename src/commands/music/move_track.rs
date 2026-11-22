use poise::CreateReply;
use songbird::input::AuxMetadata;

use super::queue_edit::{guild_queue, move_within, reply_error};
use crate::{commands::embeds::embed, Context, Error};

/// Moves a track to another queue position; aliases: move, mv
#[poise::command(
    prefix_command,
    slash_command,
    rename = "move",
    aliases("mv"),
    guild_only,
    category = "Music"
)]
pub async fn move_track(
    ctx: Context<'_>,
    #[description = "Position of the track to move (as shown by queue)"] from: u32,
    #[description = "Position to move it to (1 = next)"] to: u32,
) -> Result<(), Error> {
    let Some(queue) = guild_queue(ctx).await? else {
        return Ok(());
    };
    if let Err(msg) = queue.modify_queue(|q| move_within(q, from as usize, to as usize)) {
        return reply_error(ctx, &msg).await;
    }

    let title = queue
        .current_queue()
        .get(to as usize)
        .and_then(|track| track.data::<AuxMetadata>().title.clone())
        .unwrap_or("Unknown Title".to_string());
    let description = format!("Now at position {to}.");
    ctx.send(CreateReply::default().embed(embed(ctx, "Moved", &description, &title).await?))
        .await?;
    Ok(())
}
