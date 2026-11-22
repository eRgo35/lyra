//! Reminders set with `remind`: kept in `data/reminders.json` and delivered by a
//! background task, including any that came due while the bot was offline.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use poise::serenity_prelude::{ChannelId, CreateAllowedMentions, CreateMessage, Http, UserId};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, Notify};
use tracing::{error, warn};

use crate::storage::{read_json, write_json, STORAGE_ERROR};

pub const MAX_PER_USER: usize = 25;
pub const MAX_DELAY: u64 = 365 * 24 * 3600;
pub const MAX_TEXT: usize = 1000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reminder {
    pub id: u64,
    pub user: u64,
    /// Where it was set; where it is posted unless `dm`.
    pub channel: u64,
    pub dm: bool,
    /// Unix seconds.
    pub created: u64,
    pub due: u64,
    pub text: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Saved {
    next_id: u64,
    reminders: Vec<Reminder>,
}

pub struct Reminders {
    path: PathBuf,
    /// `None` if the file could not be read: nothing is written then, so it isn't lost.
    state: Mutex<Option<Saved>>,
    /// Wakes the delivery loop when a reminder is added.
    added: Notify,
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// `90s`, `10m`, `2h30m`, `1d`, `1w` (combinable) as seconds.
pub fn parse_delay(input: &str) -> Result<u64, String> {
    let invalid = || {
        format!(
            "\"{}\" is not a time like 10m, 2h30m, 1d or 1w!",
            input.trim()
        )
    };
    let text = input.trim().to_ascii_lowercase();
    let mut total: u64 = 0;
    let mut number = String::new();
    for c in text.chars() {
        if c.is_ascii_digit() {
            number.push(c);
            continue;
        }
        let unit = match c {
            's' => 1,
            'm' => 60,
            'h' => 3600,
            'd' => 86400,
            'w' => 7 * 86400,
            _ => return Err(invalid()),
        };
        let amount: u64 = number.parse().map_err(|_| invalid())?;
        number.clear();
        total = amount
            .checked_mul(unit)
            .and_then(|s| total.checked_add(s))
            .ok_or_else(invalid)?;
    }
    if !number.is_empty() || total == 0 {
        return Err(invalid());
    }
    if total > MAX_DELAY {
        return Err("Reminders can be at most a year ahead!".to_string());
    }
    Ok(total)
}

impl Reminders {
    pub async fn load(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let state = read_json::<Saved>(&path).await.ok();
        Reminders {
            path,
            state: Mutex::new(state),
            added: Notify::new(),
        }
    }

    /// Applies `change` to a copy, saves it, and only then keeps it.
    async fn update<T>(
        &self,
        change: impl FnOnce(&mut Saved) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut state = self.state.lock().await;
        let current = state.as_ref().ok_or(STORAGE_ERROR)?;
        let mut next = current.clone();
        let result = change(&mut next)?;
        write_json(&self.path, &next).await?;
        *state = Some(next);
        Ok(result)
    }

    pub async fn add(
        &self,
        user: u64,
        channel: u64,
        dm: bool,
        delay: u64,
        text: &str,
    ) -> Result<Reminder, String> {
        let text = text.trim();
        if text.is_empty() {
            return Err("What should I remind you about?".to_string());
        }
        if text.chars().count() > MAX_TEXT {
            return Err(format!("Keep reminders under {MAX_TEXT} characters!"));
        }
        let created = now();
        let reminder = self
            .update(|saved| {
                let pending = saved.reminders.iter().filter(|r| r.user == user).count();
                if pending >= MAX_PER_USER {
                    return Err(format!(
                        "You already have {MAX_PER_USER} reminders; cancel one first."
                    ));
                }
                saved.next_id += 1;
                let reminder = Reminder {
                    id: saved.next_id,
                    user,
                    channel,
                    dm,
                    created,
                    due: created + delay,
                    text: text.to_string(),
                };
                saved.reminders.push(reminder.clone());
                Ok(reminder)
            })
            .await?;
        self.added.notify_one();
        Ok(reminder)
    }

    /// `user`'s pending reminders, soonest first.
    pub async fn list(&self, user: u64) -> Result<Vec<Reminder>, String> {
        let state = self.state.lock().await;
        let mut mine: Vec<_> = state
            .as_ref()
            .ok_or(STORAGE_ERROR)?
            .reminders
            .iter()
            .filter(|r| r.user == user)
            .cloned()
            .collect();
        mine.sort_by_key(|r| r.due);
        Ok(mine)
    }

    pub async fn cancel(&self, user: u64, id: u64) -> Result<Reminder, String> {
        self.update(|saved| {
            let index = saved
                .reminders
                .iter()
                .position(|r| r.id == id && r.user == user)
                .ok_or_else(|| format!("You have no reminder #{id}!"))?;
            Ok(saved.reminders.remove(index))
        })
        .await
    }

    /// Removes and returns the reminders due at `now`.
    async fn take_due(&self, now: u64) -> Result<Vec<Reminder>, String> {
        if !self
            .state
            .lock()
            .await
            .as_ref()
            .is_some_and(|s| s.reminders.iter().any(|r| r.due <= now))
        {
            return Ok(Vec::new());
        }
        self.update(|saved| {
            let (due, later) = saved.reminders.drain(..).partition(|r| r.due <= now);
            saved.reminders = later;
            Ok(due)
        })
        .await
    }

    async fn next_due(&self) -> Option<u64> {
        let state = self.state.lock().await;
        state.as_ref()?.reminders.iter().map(|r| r.due).min()
    }

    /// Delivers reminders as they come due. Runs for the life of the bot.
    pub async fn run(self: Arc<Self>, http: Arc<Http>) {
        if self.state.lock().await.is_none() {
            error!(
                "Reminders are disabled: could not read {}",
                self.path.display()
            );
        }
        loop {
            let current = now();
            match self.take_due(current).await {
                Ok(due) => {
                    for reminder in due {
                        deliver(&http, &reminder, current).await;
                    }
                }
                Err(e) => warn!("Could not take due reminders: {e}"),
            }
            // Check at least every minute, so clock jumps and failures recover.
            let wait = self
                .next_due()
                .await
                .map(|due| due.saturating_sub(now()).clamp(1, 60))
                .unwrap_or(60);
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(wait)) => {}
                _ = self.added.notified() => {}
            }
        }
    }
}

/// The message text: who it's for (in a channel), what, when it was set, and a note if
/// the bot was offline when it came due.
fn message(reminder: &Reminder, now: u64, in_channel: bool) -> String {
    let mut text = if in_channel {
        format!("⏰ <@{}>, reminder: {}", reminder.user, reminder.text)
    } else {
        format!("⏰ Reminder: {}", reminder.text)
    };
    text.push_str(&format!("\n-# Set <t:{}:R>", reminder.created));
    if now.saturating_sub(reminder.due) > 60 {
        text.push_str(&format!(
            ", due <t:{}:R> (I was offline then)",
            reminder.due
        ));
    }
    text
}

async fn deliver(http: &Http, reminder: &Reminder, now: u64) {
    let user = UserId::new(reminder.user);
    // Only ever ping the reminder's owner, whatever the text contains.
    let mentions = || CreateAllowedMentions::new().users([user]);

    if !reminder.dm {
        let sent = ChannelId::new(reminder.channel)
            .send_message(
                http,
                CreateMessage::new()
                    .content(message(reminder, now, true))
                    .allowed_mentions(mentions()),
            )
            .await;
        match sent {
            Ok(_) => return,
            Err(e) => warn!(
                "Reminder #{} could not be posted in its channel ({e}); sending a DM",
                reminder.id
            ),
        }
    }

    let dm = match user.create_dm_channel(http).await {
        Ok(channel) => {
            channel
                .send_message(
                    http,
                    CreateMessage::new()
                        .content(message(reminder, now, false))
                        .allowed_mentions(CreateAllowedMentions::new()),
                )
                .await
        }
        Err(e) => Err(e),
    };
    if let Err(e) = dm {
        warn!("Reminder #{} could not be delivered: {e}", reminder.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempFile(PathBuf);
    impl TempFile {
        fn new(label: &str) -> Self {
            TempFile(std::env::temp_dir().join(format!(
                "lyra-reminders-{label}-{}-{:?}.json",
                std::process::id(),
                SystemTime::now()
            )))
        }
    }
    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    #[test]
    fn delays_are_parsed() {
        assert_eq!(parse_delay("90s"), Ok(90));
        assert_eq!(parse_delay("10m"), Ok(600));
        assert_eq!(parse_delay(" 2H30m "), Ok(9000));
        assert_eq!(parse_delay("1d"), Ok(86400));
        assert_eq!(parse_delay("1w2d"), Ok(9 * 86400));
        for bad in [
            "",
            "10",
            "m",
            "0m",
            "5x",
            "1h 30m",
            "-5m",
            "99999999999999999999w",
        ] {
            assert!(parse_delay(bad).is_err(), "{bad:?}");
        }
        assert!(parse_delay("53w").unwrap_err().contains("a year"));
    }

    #[tokio::test]
    async fn reminders_persist_and_are_per_user() {
        let file = TempFile::new("persist");
        let reminders = Reminders::load(&file.0).await;
        let first = reminders.add(1, 10, false, 3600, " pizza ").await.unwrap();
        let second = reminders.add(1, 10, true, 60, "tea").await.unwrap();
        reminders.add(2, 10, false, 60, "other").await.unwrap();
        assert_eq!((first.id, second.id), (1, 2));
        assert_eq!(first.text, "pizza");

        let reopened = Reminders::load(&file.0).await;
        let mine = reopened.list(1).await.unwrap();
        assert_eq!(mine.iter().map(|r| r.id).collect::<Vec<_>>(), [2, 1]);
        assert!(reopened.cancel(2, 1).await.is_err(), "someone else's");
        assert_eq!(reopened.cancel(1, 1).await.unwrap().text, "pizza");
        assert_eq!(reopened.list(1).await.unwrap().len(), 1);
        // Ids are not reused.
        assert_eq!(reopened.add(1, 10, false, 60, "again").await.unwrap().id, 4);
    }

    #[tokio::test]
    async fn due_reminders_are_taken_once() {
        let file = TempFile::new("due");
        let reminders = Reminders::load(&file.0).await;
        let soon = reminders.add(1, 10, false, 60, "soon").await.unwrap();
        reminders.add(1, 10, false, 3600, "later").await.unwrap();

        assert!(reminders.take_due(soon.due - 1).await.unwrap().is_empty());
        let due = reminders.take_due(soon.due).await.unwrap();
        assert_eq!(due, std::slice::from_ref(&soon));
        assert!(reminders.take_due(soon.due).await.unwrap().is_empty());
        assert_eq!(
            Reminders::load(&file.0).await.list(1).await.unwrap().len(),
            1
        );
    }

    #[tokio::test]
    async fn limits_and_validation() {
        let file = TempFile::new("limits");
        let reminders = Reminders::load(&file.0).await;
        assert!(reminders.add(1, 10, false, 60, "  ").await.is_err());
        assert!(reminders
            .add(1, 10, false, 60, &"x".repeat(MAX_TEXT + 1))
            .await
            .is_err());
        for i in 0..MAX_PER_USER {
            reminders
                .add(1, 10, false, 60, &format!("r{i}"))
                .await
                .unwrap();
        }
        assert!(reminders.add(1, 10, false, 60, "one more").await.is_err());
        assert!(reminders
            .add(2, 10, false, 60, "someone else")
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn an_unreadable_file_disables_reminders_without_overwriting() {
        let file = TempFile::new("corrupt");
        std::fs::write(&file.0, "not json").unwrap();
        let reminders = Reminders::load(&file.0).await;
        assert!(reminders.add(1, 10, false, 60, "x").await.is_err());
        assert!(reminders.list(1).await.is_err());
        assert_eq!(std::fs::read_to_string(&file.0).unwrap(), "not json");
    }

    #[test]
    fn messages_ping_in_channels_and_note_lateness() {
        let reminder = Reminder {
            id: 1,
            user: 42,
            channel: 10,
            dm: false,
            created: 1000,
            due: 2000,
            text: "pizza".to_string(),
        };
        assert_eq!(
            message(&reminder, 2010, true),
            "⏰ <@42>, reminder: pizza\n-# Set <t:1000:R>"
        );
        assert_eq!(
            message(&reminder, 5000, false),
            "⏰ Reminder: pizza\n-# Set <t:1000:R>, due <t:2000:R> (I was offline then)"
        );
    }

    /// The delivery loop against a mock Discord API: a due reminder is posted in its
    /// channel pinging only its owner, falls back to a DM, and is then gone.
    #[tokio::test]
    async fn due_reminders_are_posted_pinging_only_the_owner() {
        use poise::serenity_prelude::HttpBuilder;
        use wiremock::matchers::{body_partial_json, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let discord = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v10/channels/10/messages"))
            .and(body_partial_json(serde_json::json!({
                "allowed_mentions": {"parse": [], "users": ["42"]}
            })))
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&discord)
            .await;
        let http = Arc::new(
            HttpBuilder::new("Bot test")
                .proxy(discord.uri())
                .ratelimiter_disabled(true)
                .build(),
        );

        let file = TempFile::new("deliver");
        let reminders = Arc::new(Reminders::load(&file.0).await);
        reminders
            .add(42, 10, false, 1, "@everyone pizza")
            .await
            .unwrap();
        let _ = tokio::time::timeout(Duration::from_secs(4), reminders.clone().run(http)).await;

        discord.verify().await;
        let body: serde_json::Value =
            serde_json::from_slice(&discord.received_requests().await.unwrap()[0].body).unwrap();
        assert!(body["content"]
            .as_str()
            .unwrap()
            .starts_with("⏰ <@42>, reminder: @everyone pizza"));
        // The channel post failed (500), so it fell back to opening a DM.
        let paths: Vec<_> = discord
            .received_requests()
            .await
            .unwrap()
            .iter()
            .map(|r| r.url.path().to_string())
            .collect();
        assert!(
            paths.contains(&"/api/v10/users/@me/channels".to_string()),
            "{paths:?}"
        );
        assert!(reminders.list(42).await.unwrap().is_empty());
    }
}
