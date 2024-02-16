use poise::CreateReply;

use crate::{commands::embeds::embed, Context, Error};

/// Reference Bible by verse
#[poise::command(prefix_command, slash_command, category = "Tools")]
pub async fn verse(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(CreateReply::default().embed(embed(ctx, "", "", "").await.unwrap()))
        .await?;

    Ok(())
}
