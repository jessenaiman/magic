//! Shared Craft App Suite extension & theme store integration (`storytold/craft-extensions`:
//! `craft-ext-core` + `craft-ext-store`).
//!
//! Reads the shared on-disk extensions store (`third-party` cargo feature stays disabled per
//! `craft-extensions/README.md`) so themes, plug-ins, and resources installed once in ArtCraft
//! Launcher are visible to Magic Studio without network calls.

use std::path::PathBuf;

use serde_json::{Value, json};

use crate::{Result, Session};

/// Default shared `craft-extensions` store directory (`$CRAFT_EXTENSIONS_DIR` or user data root).
pub fn shared_extensions_dir() -> Option<PathBuf> {
    if let Some(explicit) = std::env::var_os("CRAFT_EXTENSIONS_DIR").filter(|v| !v.is_empty()) {
        let path = PathBuf::from(explicit);
        if path.is_absolute() {
            return Some(path);
        }
    }
    crate::workspace::user_data_dir().map(|base| base.join("craft-extensions"))
}

// TODO(agent): verify craft-extensions README for public type paths; d23f5f3 exposes ExtensionId at craft_ext_core::id::ExtensionId and store::Store at craft_ext_store::store::Store.
/// Inspect the shared `craft-ext-store` on disk for `"magic"` without touching the network.
pub fn extensions_status() -> Value {
    // Type-name checks verify the crates link; replaced with TODO pending correct public paths.
    let _ = std::any::type_name::<craft_ext_core::id::ExtensionId>();
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
    let _ = std::any::type_name::<craft_ext_store::store::Store>();
    json!({
        "configured": true,
        "store_dir": root,
        "exists": store_exists,
        "state_file_present": state_file.is_file(),
        "installed_packages": installed_count,
        "third_party_enabled": false,
    })
}

/// `app.extensions`: report shared `storytold/craft-extensions` store status for Magic Studio.
pub fn cmd_app_extensions(_s: &mut Session, _p: &Value) -> Result<Value> {
    Ok(json!({
        "ok": true,
        "extensions": extensions_status(),
    }))
}
