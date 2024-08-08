use crate::{
    commands::embeds::{embed, error_embed, fail},
    Context, Error,
};
use poise::CreateReply;

/// Leaves the voice channel; \
/// aliases: leave, qa!
#[poise::command(
    prefix_command,
    slash_command,
    aliases("leave", "qa!"),
    category = "Music"
)]
pub async fn leave(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird client placed at init")
        .clone();

    if manager.get(guild_id).is_none() {
        let msg = "I am not in a voice channel!";
        ctx.send(CreateReply::default().embed(error_embed(ctx, msg).await.unwrap()))
            .await?;

        return Ok(());
    }

    if let Err(err) = manager.remove(guild_id).await {
        fail(ctx, err.to_string()).await.unwrap();
    }

    ctx.send(
        CreateReply::default().embed(
            embed(ctx, "Left!", "I left the voice channel", "")
                .await
                .unwrap(),
        ),
    )
    .await?;

    Ok(())
}
