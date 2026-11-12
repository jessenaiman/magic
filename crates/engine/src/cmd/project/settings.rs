//! Production project root & workspace directory resolution (`magic/projects`, `magic/assets`,
//! `magic/animations`) backed by [`crate::settings::SessionSettings`] (`config.toml`).

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

fn default_magic_workspace_base() -> Result<PathBuf> {
    crate::workspace::user_data_dir().map(|root| root.join("magic")).ok_or_else(|| invalid("Project root unavailable; configure XDG_DATA_HOME or HOME"))
}

fn configured_root(prefs: &MagicPrefs) -> Result<PathBuf> {
    if let Some(ref root) = prefs.project.root {
        return absolute(root);
    }
    Ok(default_magic_workspace_base()?.join("projects"))
}

/// Resolve the default parent folder for `project.new` when neither `path` nor `root` is given:
/// `magic/projects` for song projects (`mode == None`), or `magic/animations` (or configured
/// `project.characters_root`) for standalone character studies (`mode == Some("animation")`).
pub(crate) fn default_root_for_mode(s: &Session, mode: Option<&str>) -> Result<PathBuf> {
    let prefs = s.settings.prefs();
    if mode == Some("animation") {
        if let Some(ref chars) = prefs.project.characters_root {
            return absolute(chars);
        }
        if let Some(ref custom_proj_root) = prefs.project.root {
            let proj_root = absolute(custom_proj_root)?;
            if proj_root.file_name().and_then(|n| n.to_str()) == Some("projects")
                && let Some(parent) = proj_root.parent()
            {
                return Ok(parent.join("animations"));
            }
            return Ok(proj_root.join("animations"));
        }
        return Ok(default_magic_workspace_base()?.join("animations"));
    }
    configured_root(&prefs)
}

pub fn root_for(s: &Session) -> Result<PathBuf> {
    configured_root(&s.settings.prefs())
}

pub fn root() -> Result<PathBuf> {
    configured_root(open_store().get())
}

/// Read the saved Magic Studio settings object from `config.toml` (`craft-settings`).
pub(crate) fn stored_settings() -> Value {
    let store = open_store();
    let prefs = store.get();
    let mut val = serde_json::to_value(prefs).unwrap_or_else(|_| json!({}));
    if let Some(ref root) = prefs.project.root {
        val["projects_root"] = json!(root);
    }
    if let Some(ref chars) = prefs.project.characters_root {
        val["characters_root"] = json!(chars);
    }
    val
}

/// A folder path saved under `key` (`projects_root` or `characters_root`) in `config.toml`; GUI
/// windows launched by the desktop do not inherit a terminal's environment, so this is where they look.
pub fn saved_path(key: &str) -> Option<PathBuf> {
    let store = open_store();
    let prefs = store.get();
    match key {
        "projects_root" | "project.root" => prefs.project.root.as_deref().map(|r| crate::workspace::expand_tilde(r)),
        "characters_root" | "project.characters_root" => prefs.project.characters_root.as_deref().map(|r| crate::workspace::expand_tilde(r)),
        _ => stored_settings().get(key)?.as_str().map(|r| crate::workspace::expand_tilde(r)),
    }
}

/// Optional external character/animations library (`MAGIC_CHARACTERS_ROOT` or `config.toml`).
pub fn characters_root() -> Option<PathBuf> {
    std::env::var_os("MAGIC_CHARACTERS_ROOT")
        .or_else(|| std::env::var_os("magic_CHARACTERS_ROOT"))
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| saved_path("characters_root"))
        .filter(|path| path.is_absolute())
}

fn persist_root(s: &mut Session, value: &Value) -> Result<()> {
    let root = absolute(value.as_str().ok_or_else(|| invalid("Project root must be a string"))?)?;
    if !root.is_dir() {
        return Err(invalid("Project root directory is unavailable; mount or choose the intended directory first"));
    }
    let root = fs::canonicalize(root)?;
    let flush_now = s.control_address.is_none();
    s.settings.mutate(
        |prefs| {
            prefs.project.root = Some(root.to_string_lossy().into_owned());
        },
        flush_now,
    )
}

/// Without `root`, inspect the setting. Only this explicit command changes it.
pub fn command(s: &mut Session, params: &Value) -> Result<Value> {
    if let Some(value) = params.get("root") {
        persist_root(s, value)?;
    }
    let prefs = s.settings.prefs();
    let persisted = prefs.project.root.is_some();
    Ok(json!({
        "root": configured_root(&prefs)?,
        "animations_root": default_root_for_mode(s, Some("animation"))?,
        "config": s.settings.path(),
        "persisted": persisted,
        "source": if persisted { "saved production setting" } else { "production default" },
        "overrides": "project.new root= or path= applies only to that request; recent history never changes this setting",
    }))
}
