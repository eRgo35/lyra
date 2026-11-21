use crate::{commands::music::play::enqueue_song, Context, Error};

/// Plays a song right now; \
/// the interrupted song plays again after it; \
/// aliases: forceplay, fp, playnow
#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    aliases("fp", "playnow"),
    category = "Music"
)]
pub async fn forceplay(
    ctx: Context<'_>,
    #[description = "Provide a query or an url"]
    #[rest]
    song: String,
) -> Result<(), Error> {
    enqueue_song(ctx, song, true).await
}
