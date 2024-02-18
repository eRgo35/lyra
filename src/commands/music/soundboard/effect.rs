use crate::{commands::embeds::embed, Context, Error};
use poise::CreateReply;

/// Plays one of available audio effects
#[poise::command(prefix_command, slash_command, category = "Music")]
pub async fn effect(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(CreateReply::default().embed(embed(ctx, "Playing an effect", "", "").await.unwrap()))
        .await?;

    Ok(())
}
