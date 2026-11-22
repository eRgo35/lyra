//! Small JSON files under the data directory (soundboard sounds, reminders).

use std::path::Path;

use serde::{de::DeserializeOwned, Serialize};
use tracing::error;

/// Shown to users when a file can't be read or written; details go to the log.
pub const STORAGE_ERROR: &str = "Lyra's data storage is not working; see the bot's logs.";

/// Reads `path`, or `T::default()` if it does not exist yet. A file that exists but
/// can't be parsed is an error: callers must not overwrite it, or its data is lost.
pub async fn read_json<T: DeserializeOwned + Default>(path: &Path) -> Result<T, String> {
    match tokio::fs::read(path).await {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| {
            error!("{} is not valid: {e}", path.display());
            STORAGE_ERROR.to_string()
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(T::default()),
        Err(e) => {
            error!("Could not read {}: {e}", path.display());
            Err(STORAGE_ERROR.to_string())
        }
    }
}

/// Writes to a temporary file and renames it over `path`, so a crash never leaves
/// half a file behind.
pub async fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let fail = |e: std::io::Error| {
        error!("Could not write {}: {e}", path.display());
        STORAGE_ERROR.to_string()
    };
    if let Some(dir) = path.parent() {
        tokio::fs::create_dir_all(dir).await.map_err(fail)?;
    }
    let json = serde_json::to_vec_pretty(value).expect("data serializes");
    let temporary = path.with_extension("json.tmp");
    tokio::fs::write(&temporary, json).await.map_err(fail)?;
    tokio::fs::rename(&temporary, path).await.map_err(fail)
}
