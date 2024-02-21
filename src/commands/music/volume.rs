use crate::{
    commands::embeds::{embed, error_embed},
    Context, Error,
};
use poise::CreateReply;

/// Changes output volume
#[poise::command(prefix_command, slash_command, category = "Music")]
pub async fn volume(ctx: Context<'_>, #[description = "Volume"] volume: f32) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let manager = songbird::get(&ctx.serenity_context())
        .await
        .expect("Songbird client placed at init")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();

        let track = queue.current().unwrap();
        let _ = track.set_volume(volume / 100.0);

        ctx.send(
            CreateReply::default().embed(
                embed(
                    ctx,
                    "Volume changed",
                    "",
                    &format!("Set volume to {}%", volume),
                )
                .await
                .unwrap(),
            ),
        )
        .await?;
    } else {
        let msg = "I am not in a voice channel!";
        ctx.send(CreateReply::default().embed(error_embed(ctx, msg).await.unwrap()))
            .await?;
    }

    Ok(())
}
