use poise::CreateReply;

use crate::{commands::embeds::embed, Context, Error};

/// Checks how long the bot has been running
#[poise::command(prefix_command, slash_command, category = "Tools")]
pub async fn uptime(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(CreateReply::default().embed(embed(ctx, "", "", "").await.unwrap()))
        .await?;

    Ok(())
}
