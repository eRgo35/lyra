use poise::CreateReply;

use super::queue_edit::{guild_queue, reply_error, shuffle_upcoming};
use crate::{commands::embeds::embed, Context, Error};

/// Shuffles the upcoming tracks
#[poise::command(prefix_command, slash_command, guild_only, category = "Music")]
pub async fn shuffle(ctx: Context<'_>) -> Result<(), Error> {
    let Some(queue) = guild_queue(ctx).await? else {
        return Ok(());
    };
    if queue.len() <= 2 {
        return reply_error(ctx, "There is nothing to shuffle!").await;
    }
    queue.modify_queue(|q| shuffle_upcoming(q, &mut rand::rng()));
    let description = format!("Shuffled {} upcoming tracks.", queue.len() - 1);
    ctx.send(CreateReply::default().embed(embed(ctx, "Shuffled!", &description, "").await?))
        .await?;
    Ok(())
}
