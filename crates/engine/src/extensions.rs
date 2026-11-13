//! Shared ArtCraft extension & theme directory inspection (`$CRAFT_EXTENSIONS_DIR` or app data root).
//!
//! Inspects local extension packages on disk with third-party code execution disabled (`U19`/`F8-06`).

use std::path::PathBuf;

use serde_json::{Value, json};

use crate::{Result, Session};

/// Default shared extensions directory (`$CRAFT_EXTENSIONS_DIR` or user data root).
pub fn shared_extensions_dir() -> Option<PathBuf> {
    if let Some(explicit) = std::env::var_os("CRAFT_EXTENSIONS_DIR").filter(|v| !v.is_empty()) {
        let path = PathBuf::from(explicit);
        if path.is_absolute() {
            return Some(path);
        }
    }
    crate::workspace::user_data_dir().map(|base| base.join("craft-extensions"))
}

/// Inspect the local shared extensions directory on disk without touching the network.
pub fn extensions_status() -> Value {
    let Some(root) = shared_extensions_dir() else {
        return json!({
            "configured": false,
            "third_party_enabled": false,
            "packages": [],
        });
    };
    let state_file = root.join("state.json");
    let store_exists = root.is_dir();
    let installed_count = std::fs::read_dir(root.join("packages")).map_or(0usize, |rd| rd.flatten().count());
    json!({
        "configured": true,
        "store_dir": root,
        "exists": store_exists,
        "state_file_present": state_file.is_file(),
        "installed_packages": installed_count,
        "third_party_enabled": false,
    })
}

/// `app.extensions`: report local ArtCraft extension directory status for Magic Studio.
pub fn cmd_app_extensions(_s: &mut Session, _p: &Value) -> Result<Value> {
    Ok(json!({
        "ok": true,
        "extensions": extensions_status(),
    }))
}
