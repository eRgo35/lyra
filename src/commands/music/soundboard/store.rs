//! Custom soundboard sounds, one JSON file per guild under `data/soundboard/`.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::storage::{read_json, write_json};

pub const MAX_SOUNDS: usize = 100;
const MAX_NAME: usize = 32;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sound {
    pub name: String,
    pub title: String,
    /// What yt-dlp plays: the URL resolved when the sound was added.
    pub source: String,
    pub added_by: u64,
}

#[derive(Default, Serialize, Deserialize)]
struct Board {
    sounds: Vec<Sound>,
}

pub struct SoundStore {
    dir: PathBuf,
    /// One writer at a time; files are small and edits are rare.
    lock: Mutex<()>,
}

/// Lower-cased name, or why it can't be one: 1-32 of a-z, 0-9, `-`, `_`.
pub fn sound_name(input: &str) -> Result<String, String> {
    let name = input.trim().to_lowercase();
    let valid = (1..=MAX_NAME).contains(&name.len())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
    if valid {
        Ok(name)
    } else {
        Err(format!(
            "\"{}\" can't be a sound name: use up to {MAX_NAME} letters, digits, - or _.",
            input.trim()
        ))
    }
}

impl SoundStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        SoundStore {
            dir: dir.into(),
            lock: Mutex::new(()),
        }
    }

    fn path(&self, guild_id: u64) -> PathBuf {
        self.dir.join(format!("{guild_id}.json"))
    }

    pub async fn list(&self, guild_id: u64) -> Result<Vec<Sound>, String> {
        let _guard = self.lock.lock().await;
        Ok(read_json::<Board>(&self.path(guild_id)).await?.sounds)
    }

    pub async fn get(&self, guild_id: u64, name: &str) -> Result<Sound, String> {
        self.list(guild_id)
            .await?
            .into_iter()
            .find(|s| s.name == name)
            .ok_or_else(|| format!("There is no sound called `{name}`!"))
    }

    pub async fn add(&self, guild_id: u64, sound: Sound) -> Result<(), String> {
        let _guard = self.lock.lock().await;
        let path = self.path(guild_id);
        let mut board = read_json::<Board>(&path).await?;
        if board.sounds.iter().any(|s| s.name == sound.name) {
            return Err(format!("There already is a sound called `{}`!", sound.name));
        }
        if board.sounds.len() >= MAX_SOUNDS {
            return Err(format!(
                "The soundboard is full ({MAX_SOUNDS} sounds); remove one first."
            ));
        }
        board.sounds.push(sound);
        board.sounds.sort_by(|a, b| a.name.cmp(&b.name));
        write_json(&path, &board).await
    }

    /// Removes `name` if `allowed` says the caller may remove that sound.
    pub async fn remove(
        &self,
        guild_id: u64,
        name: &str,
        allowed: impl FnOnce(&Sound) -> bool,
    ) -> Result<Sound, String> {
        let _guard = self.lock.lock().await;
        let path = self.path(guild_id);
        let mut board = read_json::<Board>(&path).await?;
        let index = board
            .sounds
            .iter()
            .position(|s| s.name == name)
            .ok_or_else(|| format!("There is no sound called `{name}`!"))?;
        if !allowed(&board.sounds[index]) {
            return Err(
                "Only whoever added a sound, or someone with Manage Server, can remove it."
                    .to_string(),
            );
        }
        let removed = board.sounds.remove(index);
        write_json(&path, &board).await?;
        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sound(name: &str, added_by: u64) -> Sound {
        Sound {
            name: name.to_string(),
            title: format!("{name} title"),
            source: format!("https://example.com/{name}"),
            added_by,
        }
    }

    /// A fresh directory under the system temp dir, removed when dropped.
    struct TempDir(PathBuf);
    impl TempDir {
        fn new(label: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "lyra-soundboard-{label}-{}-{:?}",
                std::process::id(),
                std::time::SystemTime::now()
            ));
            TempDir(dir)
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn names_are_normalised_and_validated() {
        assert_eq!(sound_name(" AirHorn "), Ok("airhorn".to_string()));
        assert_eq!(sound_name("bruh_2-x"), Ok("bruh_2-x".to_string()));
        for bad in ["", "two words", "../x", "ż", &"a".repeat(33)] {
            assert!(sound_name(bad).is_err(), "{bad:?}");
        }
    }

    #[tokio::test]
    async fn sounds_persist_per_guild_sorted_by_name() {
        let dir = TempDir::new("persist");
        let store = SoundStore::new(&dir.0);
        assert!(store.list(1).await.unwrap().is_empty());

        store.add(1, sound("zap", 7)).await.unwrap();
        store.add(1, sound("airhorn", 7)).await.unwrap();
        store.add(2, sound("other", 8)).await.unwrap();
        assert!(store
            .add(1, sound("zap", 9))
            .await
            .unwrap_err()
            .contains("already"));

        // A new store over the same directory sees the same sounds.
        let reopened = SoundStore::new(&dir.0);
        let names: Vec<_> = reopened
            .list(1)
            .await
            .unwrap()
            .into_iter()
            .map(|s| s.name)
            .collect();
        assert_eq!(names, ["airhorn", "zap"]);
        assert_eq!(reopened.get(2, "other").await.unwrap().added_by, 8);
        assert!(reopened.get(2, "zap").await.is_err());
    }

    #[tokio::test]
    async fn removing_checks_permission() {
        let dir = TempDir::new("remove");
        let store = SoundStore::new(&dir.0);
        store.add(1, sound("airhorn", 7)).await.unwrap();

        assert!(store
            .remove(1, "airhorn", |s| s.added_by == 8)
            .await
            .is_err());
        assert!(store.remove(1, "missing", |_| true).await.is_err());
        assert_eq!(
            store
                .remove(1, "airhorn", |s| s.added_by == 7)
                .await
                .unwrap()
                .name,
            "airhorn"
        );
        assert!(store.list(1).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn the_board_has_a_limit() {
        let dir = TempDir::new("limit");
        let store = SoundStore::new(&dir.0);
        for i in 0..MAX_SOUNDS {
            store.add(1, sound(&format!("s{i}"), 1)).await.unwrap();
        }
        assert!(store
            .add(1, sound("onemore", 1))
            .await
            .unwrap_err()
            .contains("full"));
    }

    #[tokio::test]
    async fn a_corrupt_file_is_reported_and_left_alone() {
        let dir = TempDir::new("corrupt");
        std::fs::create_dir_all(&dir.0).unwrap();
        std::fs::write(dir.0.join("1.json"), "not json").unwrap();
        let store = SoundStore::new(&dir.0);

        assert!(store.list(1).await.is_err());
        assert!(store.add(1, sound("airhorn", 7)).await.is_err());
        assert_eq!(
            std::fs::read_to_string(dir.0.join("1.json")).unwrap(),
            "not json"
        );
    }
}
