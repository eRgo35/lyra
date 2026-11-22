//! Queue edits shared by `shuffle`, `remove`, `move` and `clear`.
//!
//! Positions are as `queue` shows them: 0 is the current track, 1 the next one.

use std::collections::VecDeque;

use poise::CreateReply;
use songbird::tracks::TrackQueue;

use crate::{commands::embeds::error_embed, Context, Error};

/// The guild's queue, or `None` after telling the user there is none.
pub(super) async fn guild_queue(ctx: Context<'_>) -> Result<Option<TrackQueue>, Error> {
    let guild_id = ctx.guild_id().unwrap();
    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird client placed at init")
        .clone();
    match manager.get(guild_id) {
        Some(call) => Ok(Some(call.lock().await.queue().clone())),
        None => {
            reply_error(ctx, "I am not in a voice channel!").await?;
            Ok(None)
        }
    }
}

pub(super) async fn reply_error(ctx: Context<'_>, msg: &str) -> Result<(), Error> {
    ctx.send(CreateReply::default().embed(error_embed(ctx, msg).await?))
        .await?;
    Ok(())
}

/// Checks that `position` is an upcoming track (not the current one).
pub(super) fn upcoming(position: usize, len: usize) -> Result<usize, String> {
    match position {
        _ if len <= 1 => Err("There are no upcoming tracks!".to_string()),
        0 => Err("Position 0 is the current track; use skip for that.".to_string()),
        p if p >= len => Err(format!(
            "There is no track {p}; the queue goes up to {}.",
            len - 1
        )),
        p => Ok(p),
    }
}

/// Moves the upcoming track at `from` so it ends up at `to`.
pub(super) fn move_within<T>(
    queue: &mut VecDeque<T>,
    from: usize,
    to: usize,
) -> Result<(), String> {
    let from = upcoming(from, queue.len())?;
    let to = upcoming(to, queue.len())?;
    let track = queue.remove(from).expect("checked above");
    queue.insert(to, track);
    Ok(())
}

/// Shuffles the upcoming tracks, keeping the current one in place.
pub(super) fn shuffle_upcoming<T>(queue: &mut VecDeque<T>, rng: &mut impl rand::Rng) {
    use rand::seq::SliceRandom;
    if queue.len() > 2 {
        queue.make_contiguous()[1..].shuffle(rng);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_upcoming_positions_are_valid() {
        assert_eq!(upcoming(1, 3), Ok(1));
        assert_eq!(upcoming(2, 3), Ok(2));
        assert!(upcoming(0, 3).unwrap_err().contains("skip"));
        assert!(upcoming(3, 3).unwrap_err().contains("goes up to 2"));
        assert!(upcoming(1, 1).unwrap_err().contains("no upcoming"));
    }

    #[test]
    fn moving_reorders_upcoming_tracks() {
        let mut queue = VecDeque::from([0, 1, 2, 3, 4]);
        move_within(&mut queue, 4, 1).unwrap();
        assert_eq!(queue, [0, 4, 1, 2, 3]);
        move_within(&mut queue, 1, 4).unwrap();
        assert_eq!(queue, [0, 1, 2, 3, 4]);
        assert!(move_within(&mut queue, 0, 2).is_err());
        assert!(move_within(&mut queue, 2, 9).is_err());
        assert_eq!(queue, [0, 1, 2, 3, 4]);
    }

    #[test]
    fn shuffling_keeps_the_current_track_and_every_track() {
        let mut rng = rand::rng();
        for _ in 0..50 {
            let mut queue: VecDeque<_> = (0..10).collect();
            shuffle_upcoming(&mut queue, &mut rng);
            assert_eq!(queue[0], 0);
            let mut sorted: Vec<_> = queue.into_iter().collect();
            sorted.sort();
            assert_eq!(sorted, (0..10).collect::<Vec<_>>());
        }
    }
}
