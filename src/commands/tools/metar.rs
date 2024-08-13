use poise::CreateReply;

use crate::{commands::embeds::embed, Context, Error};

/// Prints metar for provided airport
#[poise::command(prefix_command, slash_command, category = "Tools")]
pub async fn metar(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(
        CreateReply::default().embed(embed(ctx, "Metar", "Work in progress", "").await.unwrap()),
    )
    .await?;

    Ok(())
}
