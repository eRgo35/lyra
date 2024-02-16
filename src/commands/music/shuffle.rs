use crate::{commands::embeds::embed, Context, Error};
use poise::CreateReply;

/// Shuffles the playlist
#[poise::command(prefix_command, slash_command, category = "Music")]
pub async fn shuffle(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(CreateReply::default().embed(embed(ctx, "", "", "").await.unwrap()))
        .await?;

    Ok(())
}
