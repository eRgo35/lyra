use poise::CreateReply;

use crate::{commands::embeds::embed, Context, Error};

/// Shows weather for provided location
#[poise::command(prefix_command, slash_command, category = "Tools")]
pub async fn weather(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(CreateReply::default().embed(embed(ctx, "", "", "").await.unwrap()))
        .await?;

    Ok(())
}
