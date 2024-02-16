use poise::CreateReply;

use crate::{commands::embeds::embed, Context, Error};

/// Creates a qr code from text
#[poise::command(prefix_command, slash_command, category = "Tools")]
pub async fn qr(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(CreateReply::default().embed(embed(ctx, "", "", "").await.unwrap()))
        .await?;

    Ok(())
}
