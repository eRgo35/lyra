use crate::{commands::embeds::embed, Context, Error};
use poise::CreateReply;

/// Seeks a track by provided seconds
#[poise::command(prefix_command, slash_command, category = "Music")]
pub async fn seek(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(CreateReply::default().embed(embed(ctx, "", "", "").await.unwrap()))
        .await?;

    Ok(())
}
