use poise::CreateReply;

use crate::{commands::embeds::embed, Context, Error};

/// Explains provided query
#[poise::command(prefix_command, slash_command, category = "Tools")]
pub async fn dictionary(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(CreateReply::default().embed(embed(ctx, "", "", "").await.unwrap()))
        .await?;

    Ok(())
}
