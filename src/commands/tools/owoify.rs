use poise::CreateReply;

use crate::{commands::embeds::embed, Context, Error};

/// Owoifies whatever you want uwu
#[poise::command(prefix_command, slash_command, category = "Tools")]
pub async fn owoify(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(CreateReply::default().embed(embed(ctx, "", "", "").await.unwrap()))
        .await?;

    Ok(())
}
