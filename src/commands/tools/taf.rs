use poise::CreateReply;

use crate::{commands::embeds::embed, Context, Error};

/// Returns taf for provided airport
#[poise::command(prefix_command, slash_command, category = "Tools")]
pub async fn taf(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(
        CreateReply::default().embed(embed(ctx, "Taf", "Work in progress", "").await.unwrap()),
    )
    .await?;

    Ok(())
}
