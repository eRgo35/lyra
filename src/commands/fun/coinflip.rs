use poise::CreateReply;
use rand::RngExt;

use crate::{commands::embeds::embed, Context, Error};

/// Flips a coin
#[poise::command(prefix_command, slash_command, aliases("flip"), category = "Fun")]
pub async fn coinflip(ctx: Context<'_>) -> Result<(), Error> {
    let side = if rand::rng().random_bool(0.5) {
        "Heads"
    } else {
        "Tails"
    };
    ctx.send(
        CreateReply::default().embed(embed(ctx, "Coin flip", "", &format!("🪙 {side}!")).await?),
    )
    .await?;
    Ok(())
}
