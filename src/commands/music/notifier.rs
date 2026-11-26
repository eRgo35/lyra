use std::sync::Arc;

use poise::serenity_prelude::{ChannelId, CreateMessage, Http};
use serenity::async_trait;
use songbird::events::{Event, EventContext, EventHandler as VoiceEventHandler};
use songbird::input::AuxMetadata;
use songbird::{Call, TrackEvent};
use tracing::warn;

/// Tells `channel` when a track fails, so a dead track isn't just silence.
/// Every track must carry [`AuxMetadata`] as its data.
pub struct TrackErrorNotifier {
    http: Arc<Http>,
    channel: ChannelId,
}

impl TrackErrorNotifier {
    /// Reports `call`'s track errors in `channel`, replacing any earlier notifier
    /// (joining again must not report each error twice).
    pub fn attach(call: &mut Call, http: Arc<Http>, channel: ChannelId) {
        call.remove_all_global_events();
        call.add_global_event(
            TrackEvent::Error.into(),
            TrackErrorNotifier { http, channel },
        );
    }
}

#[async_trait]
impl VoiceEventHandler for TrackErrorNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        let EventContext::Track(track_list) = ctx else {
            return None;
        };
        for (state, handle) in *track_list {
            let metadata = handle.data::<AuxMetadata>();
            let title = metadata.title.as_deref().unwrap_or("Unknown Title");
            warn!(
                "Track {title} ({}) failed: {:?}",
                handle.uuid(),
                state.playing
            );
            let text = format!("⚠️ Could not play **{title}**; skipping it.");
            if let Err(e) = self
                .channel
                .send_message(&self.http, CreateMessage::new().content(text))
                .await
            {
                warn!("Could not report the failed track: {e}");
            }
        }
        None
    }
}
