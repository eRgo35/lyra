use std::collections::VecDeque;

use crate::{
    commands::embeds::{embed, error_embed},
    Context, Error,
};
use poise::CreateReply;
use songbird::input::AuxMetadata;

/// Jumps to a queue position (as shown by queue), skipping the tracks before it; \
/// aliases: jump, j
#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    aliases("j"),
    category = "Music"
)]
pub async fn jump(
    ctx: Context<'_>,
    #[description = "Queue position to jump to (1 = next track)"] position: u32,
) -> Result<(), Error> {
    let position = position as usize;
    let guild_id = ctx.guild_id().unwrap();

    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird client placed at init")
        .clone();

    let Some(handler_lock) = manager.get(guild_id) else {
        return reply_error(ctx, "I am not in a voice channel!").await;
    };
    let queue = handler_lock.lock().await.queue().clone();

    if queue.is_empty() {
        return reply_error(ctx, "Nothing is playing!").await;
    }
    if position == 0 {
        return reply_error(ctx, "That track is already playing!").await;
    }
    if position >= queue.len() {
        let msg = format!(
            "There is no track {position}; the queue goes up to {}.",
            queue.len() - 1
        );
        return reply_error(ctx, &msg).await;
    }

    // Drop the tracks in between, then skip the current one so `position` plays next.
    for skipped in queue.modify_queue(|q| remove_between(q, position)) {
        let _ = skipped.stop();
    }
    let target = queue.current_queue().get(1).cloned();
    // songbird removes the skipped track asynchronously, so count before skipping.
    let remaining = queue.len() - 1;
    let _ = queue.skip();

    let title = target
        .and_then(|track| track.data::<AuxMetadata>().title.clone())
        .unwrap_or("Unknown Title".to_string());
    let description = format!("Skipped {position} track(s). Queue length is {remaining}");

    ctx.send(CreateReply::default().embed(embed(ctx, "Jumped!", &description, &title).await?))
        .await?;

    Ok(())
}

async fn reply_error(ctx: Context<'_>, msg: &str) -> Result<(), Error> {
    ctx.send(CreateReply::default().embed(error_embed(ctx, msg).await?))
        .await?;
    Ok(())
}

/// Removes the tracks strictly between the current one (index 0) and `position`.
fn remove_between<T>(queue: &mut VecDeque<T>, position: usize) -> Vec<T> {
    queue.drain(1..position.min(queue.len())).collect()
}

#[cfg(test)]
mod tests {
    use super::remove_between;
    use std::collections::VecDeque;

    #[test]
    fn removes_only_tracks_before_the_target() {
        let mut queue = VecDeque::from([0, 1, 2, 3, 4]);
        assert_eq!(remove_between(&mut queue, 3), vec![1, 2]);
        assert_eq!(queue, [0, 3, 4]);
    }

    #[test]
    fn jumping_to_the_next_track_removes_nothing() {
        let mut queue = VecDeque::from([0, 1, 2]);
        assert!(remove_between(&mut queue, 1).is_empty());
        assert_eq!(queue, [0, 1, 2]);
    }
}
