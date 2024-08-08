use crate::{
    commands::embeds::{embed, error_embed},
    Context, Error,
};
use poise::CreateReply;

/// Stops playback and destroys the queue; \
/// aliases: stop, end
#[poise::command(prefix_command, slash_command, aliases("end"), category = "Music")]
pub async fn stop(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird client placed at init")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        queue.stop();

        ctx.send(
            CreateReply::default().embed(
                embed(
                    ctx,
                    "Stopped!",
                    "Playback stopped!",
                    "Queue destroyed! Bot will stay and chill with you in a vc",
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
