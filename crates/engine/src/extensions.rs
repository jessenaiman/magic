//! L0 integration with `storytold/craft-extensions` (`craft-ext-core` + `craft-ext-store`).
//!
//! Reads installed Themes, Plugins, and Resources enabled for `magic` without requiring a launcher
//! or mutating store state.

use crate::Session;
use crate::error::EngineError;
use craft_ext_core::id::AppId;
use craft_ext_core::manifest::Kind;
use craft_ext_store::location::default_root;
use craft_ext_store::reader::{Found, Reader};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// Returns the canonical `AppId` for Magic Studio.
pub fn magic_app_id() -> crate::Result<AppId> {
    AppId::new("magic").map_err(|err| EngineError::CommandFailed("app.extensions".into(), format!("invalid app id: {err}")))
}

/// Returns the `craft-ext-store` root directory on disk.
pub fn extensions_root() -> crate::Result<PathBuf> {
    default_root().map_err(|err| EngineError::CommandFailed("app.extensions".into(), format!("extensions root unavailable: {err}")))
}

/// Opens a read-only `craft-ext-store` reader at the default store root.
pub fn store_reader() -> crate::Result<Reader> {
    Ok(Reader::open(extensions_root()?))
}

fn serialize_found(found: &Found) -> Value {
    json!({
        "id": found.installed.id.as_str(),
        "name": found.installed.name,
        "version": found.installed.version.to_string(),
        "kind": found.installed.kind,
        "tier": found.installed.tier,
        "dir": found.dir.to_string_lossy(),
    })
}

/// Inspects installed extensions and active theme for `magic` at `root`.
pub fn extensions_report_at(root: &Path) -> Value {
    let reader = Reader::open(root);
    let Ok(app_id) = magic_app_id() else {
        return json!({ "error": "invalid app id" });
    };
    let generation = reader.generation();
    let active_theme = reader.active_theme(&app_id).map(|theme| {
        json!({
            "id": theme.id.as_str(),
            "name": theme.name,
        })
    });
    let themes: Vec<Value> = reader.installed_for(&app_id, Kind::Theme).iter().map(serialize_found).collect();
    let plugins: Vec<Value> = reader.installed_for(&app_id, Kind::Plugin).iter().map(serialize_found).collect();
    let resources: Vec<Value> = reader.installed_for(&app_id, Kind::Resource).iter().map(serialize_found).collect();
    let total = themes.len() + plugins.len() + resources.len();
    json!({
        "root": root.to_string_lossy(),
        "generation": generation,
        "active_theme": active_theme,
        "counts": {
            "themes": themes.len(),
            "plugins": plugins.len(),
            "resources": resources.len(),
            "total": total,
        },
        "themes": themes,
        "plugins": plugins,
        "resources": resources,
    })
}

/// Inspects installed extensions and active theme at the default `craft-ext-store` root.
pub fn extensions_report() -> Value {
    match extensions_root() {
        Ok(root) => extensions_report_at(&root),
        Err(err) => json!({
            "root": Value::Null,
            "generation": 0,
            "active_theme": Value::Null,
            "counts": { "themes": 0, "plugins": 0, "resources": 0, "total": 0 },
            "themes": [],
            "plugins": [],
            "resources": [],
            "error": err.to_string(),
        }),
    }
}

/// Engine query handler for `app.extensions`.
pub fn cmd_app_extensions(_: &mut Session, params: &Value) -> crate::Result<Value> {
    if let Some(root) = params.get("root").and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()) {
        return Ok(extensions_report_at(Path::new(root)));
    }
    Ok(extensions_report())
}
