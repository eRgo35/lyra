use crate::commands::music::notifier::TrackErrorNotifier;
use crate::{
    commands::embeds::{embed, error_embed},
    Context, Error,
};
use poise::CreateReply;
use songbird::TrackEvent;

/// Joins your voice channel
#[poise::command(prefix_command, slash_command, category = "Music")]
pub async fn join(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let channel_id = ctx
        .guild()
        .unwrap()
        .voice_states
        .get(&ctx.author().id)
        .and_then(|voice_state| voice_state.channel_id);

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            let msg = "I am not in a voice channel!";
            ctx.send(CreateReply::default().embed(error_embed(ctx, msg).await.unwrap()))
                .await?;
            return Ok(());
        }
    };

    let manager = songbird::get(&ctx.serenity_context())
        .await
        .expect("Songbird client placed at init")
        .clone();

    if let Ok(handler_lock) = manager.join(guild_id, connect_to).await {
        let mut handler = handler_lock.lock().await;
        handler.add_global_event(TrackEvent::Error.into(), TrackErrorNotifier);
    }

    ctx.send(CreateReply::default().embed(embed(ctx, "Joined!", "Hi there!", "").await.unwrap()))
        .await?;

    Ok(())
}
