use std::time::Duration;

use crate::{
    commands::embeds::{embed, error_embed},
    Context, Error,
};
use poise::CreateReply;

/// Seeks a track by provided seconds
#[poise::command(prefix_command, slash_command, category = "Music")]
pub async fn seek(
    ctx: Context<'_>,
    #[description = "How many seconds shall I seek"] seek: u64,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let manager = songbird::get(&ctx.serenity_context())
        .await
        .expect("Songbird client placed at init")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();

        let seek_duration = Duration::from_secs(seek);

        let track = queue.current().unwrap();
        let _ = track.seek(seek_duration);

        ctx.send(
            CreateReply::default().embed(
                embed(
                    ctx,
                    "Track seeked!",
                    &format!("Track seeked by: {} seconds", seek),
                    "",
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
