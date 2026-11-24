//! `repeat all`: tracks that leave the queue (finished or skipped) are enqueued again at the end.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use reqwest::Client;
use serenity::all::GuildId;
use serenity::async_trait;
use songbird::events::{Event, EventContext, EventHandler, TrackEvent};
use songbird::input::AuxMetadata;
use songbird::tracks::{Track, TrackHandle};
use songbird::{Call, Songbird};

/// Per-guild `repeat all` state, shared by commands and track event handlers.
#[derive(Default)]
pub struct RepeatAll {
    guilds: Mutex<HashSet<GuildId>>,
    /// Tracks (by UUID) that should not be requeued once, e.g. the copy `forceplay` already re-added.
    skip_once: Mutex<HashSet<u128>>,
}

impl RepeatAll {
    pub fn set(&self, guild_id: GuildId, enabled: bool) {
        let mut guilds = self.guilds.lock().unwrap();
        if enabled {
            guilds.insert(guild_id);
        } else {
            guilds.remove(&guild_id);
        }
    }

    pub fn is_enabled(&self, guild_id: GuildId) -> bool {
        self.guilds.lock().unwrap().contains(&guild_id)
    }

    /// Don't requeue `track` when it next ends (only recorded while `repeat all` is on).
    pub fn skip_once(&self, guild_id: GuildId, track: &TrackHandle) {
        self.mark_skip(guild_id, track.uuid().as_u128());
    }

    fn mark_skip(&self, guild_id: GuildId, track: u128) {
        if self.is_enabled(guild_id) {
            self.skip_once.lock().unwrap().insert(track);
        }
    }

    fn take_skip(&self, track: &TrackHandle) -> bool {
        self.take_skip_id(track.uuid().as_u128())
    }

    fn take_skip_id(&self, track: u128) -> bool {
        self.skip_once.lock().unwrap().remove(&track)
    }
}

/// What is needed to put a track back into a guild's queue from an event handler.
#[derive(Clone)]
pub struct Enqueuer {
    pub manager: Arc<Songbird>,
    pub guild_id: GuildId,
    pub http_client: Client,
    pub repeat: Arc<RepeatAll>,
}

impl Enqueuer {
    /// Enqueues `query` with `metadata`, requeueing it at the end when it leaves the queue
    /// while `repeat all` is on. All queue insertions should go through this.
    pub async fn enqueue(
        &self,
        call: &mut Call,
        query: String,
        metadata: AuxMetadata,
    ) -> TrackHandle {
        let src =
            crate::http::ytdl(self.http_client.clone(), query).with_metadata(metadata.clone());
        self.enqueue_source(call, src, metadata).await
    }

    /// Enqueues `src` (e.g. from [`crate::http::lookup`], keeping its stream URL), as
    /// [`Self::enqueue`] does.
    pub async fn enqueue_source(
        &self,
        call: &mut Call,
        src: crate::http::Ytdl,
        metadata: AuxMetadata,
    ) -> TrackHandle {
        let handle = call
            .enqueue(Track::new_with_data(src.into(), Arc::new(metadata.clone())))
            .await;

        let _ = handle.add_event(
            Event::Track(TrackEvent::End),
            RequeueOnEnd {
                enqueuer: self.clone(),
                metadata,
            },
        );

        handle
    }
}

struct RequeueOnEnd {
    enqueuer: Enqueuer,
    metadata: AuxMetadata,
}

#[async_trait]
impl EventHandler for RequeueOnEnd {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        let EventContext::Track(tracks) = ctx else {
            return None;
        };
        let repeat = &self.enqueuer.repeat;
        let guild_id = self.enqueuer.guild_id;

        for (_, handle) in *tracks {
            if repeat.take_skip(handle) || !repeat.is_enabled(guild_id) {
                continue;
            }
            let Some(url) = self.metadata.source_url.clone() else {
                continue;
            };

            let enqueuer = self.enqueuer.clone();
            let metadata = self.metadata.clone();
            tokio::spawn(async move {
                if let Some(call) = enqueuer.manager.get(enqueuer.guild_id) {
                    enqueuer
                        .enqueue(&mut *call.lock().await, url, metadata)
                        .await;
                }
            });
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::RepeatAll;
    use serenity::all::GuildId;

    const GUILD: GuildId = GuildId::new(1);
    const OTHER: GuildId = GuildId::new(2);

    #[test]
    fn enabling_is_per_guild() {
        let repeat = RepeatAll::default();
        assert!(!repeat.is_enabled(GUILD));

        repeat.set(GUILD, true);
        assert!(repeat.is_enabled(GUILD));
        assert!(!repeat.is_enabled(OTHER));

        repeat.set(GUILD, false);
        assert!(!repeat.is_enabled(GUILD));
    }

    #[test]
    fn skip_marker_is_consumed_once() {
        let repeat = RepeatAll::default();
        repeat.set(GUILD, true);
        repeat.mark_skip(GUILD, 42);

        assert!(repeat.take_skip_id(42));
        assert!(!repeat.take_skip_id(42));
    }

    #[test]
    fn skip_marker_is_not_recorded_while_disabled() {
        // Otherwise it would leak, and wrongly suppress a later requeue once enabled.
        let repeat = RepeatAll::default();
        repeat.mark_skip(GUILD, 42);
        repeat.set(GUILD, true);

        assert!(!repeat.take_skip_id(42));
    }
}
