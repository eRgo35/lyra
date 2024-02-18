use crate::{commands::embeds::embed, Context, Error};
use poise::CreateReply;

/// Hijacks current audio output and plays selected audio
#[poise::command(prefix_command, slash_command, aliases("override"), category = "Music")]
pub async fn stream(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(CreateReply::default().embed(embed(ctx, "Playing audio", "", "").await.unwrap()))
        .await?;

    Ok(())
}
