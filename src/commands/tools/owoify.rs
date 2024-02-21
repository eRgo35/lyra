use owoify::OwOifiable;
use poise::CreateReply;

use crate::{commands::embeds::embed, Context, Error};

/// Owoifies whatever you want uwu
#[poise::command(prefix_command, slash_command, category = "Tools")]
pub async fn owoify(
    ctx: Context<'_>,
    #[description = "Text to owoify w-woify OwO"]
    #[rest]
    text: String,
) -> Result<(), Error> {
    ctx.send(CreateReply::default().embed(embed(ctx, "OwO", &text.owoify(), "").await.unwrap()))
        .await?;

    Ok(())
}
