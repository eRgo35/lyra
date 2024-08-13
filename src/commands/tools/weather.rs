use poise::CreateReply;

use crate::{commands::embeds::embed, Context, Error};

/// Shows weather for provided location
#[poise::command(prefix_command, slash_command, category = "Tools")]
pub async fn weather(
    ctx: Context<'_>,
    #[description = "Provide a city name"]
    #[rest]
    _location: String,
) -> Result<(), Error> {
    ctx.send(
        CreateReply::default().embed(embed(ctx, "Weather", "Work in progress", "").await.unwrap()),
    )
    .await?;

    Ok(())
}
