//! App-level default folders (`~/Magic` projects directory and `~/.local/share/magic/templates/storyboard.designcraft`)
//! backed by [`crate::settings::SessionSettings`] (`config.toml`, `craft-settings#12`).

use std::{fs, path::PathBuf};

use serde_json::{Value, json};

use crate::settings::MagicPrefs;
use crate::settings::prefs::open_store;
use crate::{EngineError, Result, Session};

fn invalid(message: impl Into<String>) -> EngineError {
    EngineError::InvalidParams(message.into())
}

fn absolute(value: &str) -> Result<PathBuf> {
    let expanded = crate::workspace::expand_tilde(value.trim());
    if value.trim().is_empty() || !expanded.is_absolute() || value.chars().any(char::is_control) {
        return Err(invalid("Project root must be a nonempty absolute directory path"));
    }
    Ok(expanded)
}

fn default_projects_dir() -> Result<PathBuf> {
    if let Some(override_dir) = std::env::var_os("MAGIC_PROJECTS_DIR").filter(|v| !v.is_empty()).map(PathBuf::from).filter(|p| p.is_absolute()) {
        return Ok(override_dir);
    }
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .map(|home| home.join("Magic"))
        .ok_or_else(|| invalid("Project root unavailable; configure HOME, MAGIC_PROJECTS_DIR, or project.root in config.toml"))
}

fn configured_root(prefs: &MagicPrefs) -> Result<PathBuf> {
    match prefs.project.root {
        Some(ref root) => absolute(root),
        None => default_projects_dir(),
    }
}

pub(crate) fn default_root_for_mode(s: &Session, _mode: Option<&str>) -> Result<PathBuf> {
    configured_root(&s.settings.prefs())
}

pub fn root_for(s: &Session) -> Result<PathBuf> {
    configured_root(&s.settings.prefs())
}

pub fn root() -> Result<PathBuf> {
    configured_root(open_store().get())
}

/// Resolve the configured custom `.designcraft` template or the seeded system template in `~/.local/share/magic/templates/`.
pub fn storyboard_template_path(s: &Session) -> Option<PathBuf> {
    let raw = s.settings.prefs().project.storyboard_template;
    let trimmed = raw.trim();
    if !trimmed.is_empty() && trimmed != "magic" {
        let custom = crate::workspace::expand_tilde(trimmed);
        if custom.is_file() {
            return Some(custom);
        }
    }
    crate::workspace::ensure_default_app_folders(crate::cmd::storyboard::MASTER_DESIGNCRAFT)
        .map(|d| d.join("templates/storyboard.designcraft"))
        .filter(|p| p.is_file())
}

pub(crate) fn stored_settings() -> Value {
    let store = open_store();
    let prefs = store.get();
    let mut val = serde_json::to_value(prefs).unwrap_or_else(|_| json!({}));
    if let Some(ref root) = prefs.project.root {
        val["projects_root"] = json!(root);
    }
    val
}

pub fn saved_path(key: &str) -> Option<PathBuf> {
    let store = open_store();
    let prefs = store.get();
    match key {
        "projects_root" | "project.root" => prefs.project.root.as_deref().map(crate::workspace::expand_tilde),
        _ => stored_settings().get(key)?.as_str().map(crate::workspace::expand_tilde),
    }
}

pub fn command(s: &mut Session, params: &Value) -> Result<Value> {
    if let Some(value) = params.get("root") {
        let root = absolute(value.as_str().ok_or_else(|| invalid("Project root must be a string"))?)?;
        if !root.is_dir() {
            return Err(invalid("Project root directory is unavailable; create or choose an existing directory first"));
        }
        let root = fs::canonicalize(root)?;
        let flush_now = s.control_address.is_none();
        s.settings.mutate(|prefs| prefs.project.root = Some(root.to_string_lossy().into_owned()), flush_now)?;
    }
    let prefs = s.settings.prefs();
    let persisted = prefs.project.root.is_some();
    Ok(json!({
        "root": configured_root(&prefs)?,
        "data_dir": crate::workspace::magic_data_dir(),
        "storyboard_template": storyboard_template_path(s),
        "config": s.settings.path(),
        "persisted": persisted,
        "source": if persisted { "saved production setting" } else { "default ~/Magic" },
    }))
}
