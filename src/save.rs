//! Utilities for saving/loading data definitions for the game.
//! What do we care about?
//! - saving all generated games to localstorage (or a fixed local file)
//! - listing ^
//! - being able to save and load these to local files
//! - properly supporting saves from an older save_format
use quad_storage::STORAGE;

use crate::net::GameDefs;

/// Rev of the game definitions used by the running game.
pub const CURRENT_SAVE_FORMAT: u64 = 1;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DefsMetadata {
    pub theme: String,
    pub save_format: u64,
    /// unix time
    pub created: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Defs {
    pub metadata: DefsMetadata,
    /// Deliberately not parsing this here to allow old game data.
    pub defs: serde_json::Value,
}

pub fn load_defs() -> Vec<Defs> {
    STORAGE
        .lock()
        .unwrap()
        .get("game_defs")
        .and_then(|defs| serde_json::from_str(&defs).ok())
        .unwrap_or_default()
}

pub fn write_defs(defs: &[Defs]) {
    STORAGE
        .lock()
        .unwrap()
        .set("game_defs", &serde_json::to_string(defs).unwrap());
}

pub fn save_def(defs: &GameDefs) {
    let mut saved_defs = load_defs();
    saved_defs.push(Defs {
        metadata: DefsMetadata {
            theme: defs.theme.clone(),
            save_format: CURRENT_SAVE_FORMAT,
            created: web_time::SystemTime::now()
                .duration_since(web_time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        },
        defs: serde_json::to_value(defs).unwrap(),
    });
    write_defs(&saved_defs);
}
