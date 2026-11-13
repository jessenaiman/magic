//! Typed coordinator preferences persisted in `config.toml` via [`craft_settings::SettingsStore`].
//!
//! Follows the `craft-settings` contract:
//! - One in-memory [`SettingsStore<MagicPrefs>`] per [`crate::Session`] (`Location::Memory` for
//!   isolated/test sessions so tests never touch user profiles; `Location::Auto` for user sessions,
//!   retaining `LoadReport` diagnostics).
//! - Every struct carries `#[serde(default)]` without `#[serde(deny_unknown_fields)]` so unknown or
//!   newer keys in `config.toml` survive untouched.
//! - Desktop frames call [`SessionSettings::tick`] (`500 ms` debounced atomic save with retry) and
//!   [`SessionSettings::flush`] on exit; headless CLI commands flush on mutation.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use craft_settings::{Location, SaveStatus, Settings, SettingsStore};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::schema::{Kind, fields_with_cat};
use crate::{EngineError, Result};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AiPrefs {
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FidgetPrefs {
    pub enabled: bool,
    pub seamless: bool,
    pub connection: Option<String>,
    pub model: Option<String>,
}

impl Default for FidgetPrefs {
    fn default() -> Self {
        Self { enabled: true, seamless: false, connection: None, model: None }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScribblesPrefs {
    pub enabled: bool,
    pub connection: Option<String>,
    pub model: Option<String>,
    pub endpoint: Option<String>,
    pub template: Option<String>,
}

impl Default for ScribblesPrefs {
    fn default() -> Self {
        Self { enabled: true, connection: None, model: None, endpoint: None, template: Some("new".into()) }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectPrefs {
    pub root: Option<String>,
    pub storyboard_template: String,
    pub storyboard_format: String,
}

impl Default for ProjectPrefs {
    fn default() -> Self {
        Self { root: None, storyboard_template: "magic".into(), storyboard_format: "designcraft".into() }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LyricsPrefs {
    pub default_size: i64,
    pub outline: bool,
    pub font_dirs: Option<String>,
}

impl Default for LyricsPrefs {
    fn default() -> Self {
        Self { default_size: 72, outline: true, font_dirs: None }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AnimationPrefs {
    pub loop_preview: bool,
    pub duration: f64,
}

impl Default for AnimationPrefs {
    fn default() -> Self {
        Self { loop_preview: true, duration: 4.0 }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppearancePrefs {
    pub interface_style: String,
    pub theme: String,
    pub accessible_contrast: bool,
    pub accent: String,
    pub ui_scale: f64,
}

impl Default for AppearancePrefs {
    fn default() -> Self {
        Self { interface_style: "aurora".into(), theme: "dark".into(), accessible_contrast: false, accent: "teal".into(), ui_scale: 1.0 }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralPrefs {
    pub at_startup: String,
    pub when_opening_project: String,
    pub show_tooltips: bool,
    pub show_rich_tooltips: bool,
    pub recent_projects: String,
}

impl Default for GeneralPrefs {
    fn default() -> Self {
        Self {
            at_startup: "showHome".into(),
            when_opening_project: "showOpenDialog".into(),
            show_tooltips: true,
            show_rich_tooltips: true,
            recent_projects: String::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AutosavePrefs {
    pub enabled: bool,
    pub interval_minutes: i64,
}

impl Default for AutosavePrefs {
    fn default() -> Self {
        Self { enabled: true, interval_minutes: 5 }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MagicPrefs {
    pub ai: AiPrefs,
    pub fidget: FidgetPrefs,
    pub scribbles: ScribblesPrefs,
    pub project: ProjectPrefs,
    pub lyrics: LyricsPrefs,
    pub animation: AnimationPrefs,
    pub appearance: AppearancePrefs,
    pub general: GeneralPrefs,
    pub autosave: AutosavePrefs,
    pub tools: BTreeMap<String, String>,
}

impl Settings for MagicPrefs {
    const APP_ID: &'static str = "magic";
    const SCHEMA_VERSION: u32 = 1;
}

fn opt_str_json(opt: &Option<String>) -> Value {
    match opt {
        Some(s) => json!(s),
        None => Value::Null,
    }
}

fn bad(msg: impl Into<String>) -> EngineError {
    EngineError::InvalidParams(msg.into())
}

fn validate_schema_value(key: &str, value: &Value) -> Result<()> {
    let (_, field) = fields_with_cat().into_iter().find(|(_, f)| f.key == key).ok_or_else(|| bad(format!("unknown setting `{key}`; see prefs.schema")))?;
    match field.kind {
        Kind::Bool => {
            if !value.is_boolean() {
                return Err(bad(format!("`{key}` must be a boolean")));
            }
        }
        Kind::Int { min, max, .. } => {
            let n = value.as_i64().ok_or_else(|| bad(format!("`{key}` must be an integer")))?;
            if (n as f64) < min || (n as f64) > max {
                return Err(bad(format!("`{key}` must be between {min} and {max}")));
            }
        }
        Kind::Float { min, max, .. } => {
            let n = value.as_f64().filter(|v| v.is_finite()).ok_or_else(|| bad(format!("`{key}` must be a number")))?;
            if n < min || n > max {
                return Err(bad(format!("`{key}` must be between {min} and {max}")));
            }
        }
        Kind::Choice(choices) => {
            let s = value.as_str().ok_or_else(|| bad(format!("`{key}` must be a string")))?;
            if !choices.iter().any(|(c, _)| *c == s) {
                return Err(bad(format!("invalid value `{s}` for `{key}`")));
            }
        }
        Kind::Text | Kind::Path => {
            if !value.is_null() && !value.is_string() {
                return Err(bad(format!("`{key}` must be a string or null")));
            }
        }
    }
    Ok(())
}

fn opt_str_from_value(value: &Value) -> Option<String> {
    value.as_str().map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned)
}

impl MagicPrefs {
    pub fn get_value(&self, key: &str) -> Value {
        match key {
            "ai.enabled" => json!(self.ai.enabled),
            "fidget.enabled" => json!(self.fidget.enabled),
            "fidget.seamless" => json!(self.fidget.seamless),
            "fidget.connection" => opt_str_json(&self.fidget.connection),
            "fidget.model" => opt_str_json(&self.fidget.model),
            "scribbles.enabled" => json!(self.scribbles.enabled),
            "scribbles.connection" => opt_str_json(&self.scribbles.connection),
            "scribbles.model" => opt_str_json(&self.scribbles.model),
            "scribbles.endpoint" => opt_str_json(&self.scribbles.endpoint),
            "scribbles.template" => opt_str_json(&self.scribbles.template),
            "appearance.interface_style" => json!(self.appearance.interface_style),
            "appearance.theme" => json!(self.appearance.theme),
            "appearance.accessible_contrast" => json!(self.appearance.accessible_contrast),
            "appearance.accent" => json!(self.appearance.accent),
            "appearance.ui_scale" => json!(self.appearance.ui_scale),
            "project.root" => opt_str_json(&self.project.root),
            "project.storyboard_template" => json!(self.project.storyboard_template),
            "project.storyboard_format" => json!(self.project.storyboard_format),
            "lyrics.default_size" => json!(self.lyrics.default_size),
            "lyrics.outline" => json!(self.lyrics.outline),
            "lyrics.font_dirs" => opt_str_json(&self.lyrics.font_dirs),
            "animation.loop_preview" => json!(self.animation.loop_preview),
            "animation.duration" => json!(self.animation.duration),
            "general.at_startup" => json!(self.general.at_startup),
            "general.when_opening_project" => json!(self.general.when_opening_project),
            "general.show_tooltips" => json!(self.general.show_tooltips),
            "general.show_rich_tooltips" => json!(self.general.show_rich_tooltips),
            "general.recent_projects" => json!(self.general.recent_projects),
            "autosave.enabled" => json!(self.autosave.enabled),
            "autosave.interval_minutes" => json!(self.autosave.interval_minutes),
            _ => Value::Null,
        }
    }

    pub fn set_value(&mut self, key: &str, value: &Value) -> Result<()> {
        validate_schema_value(key, value)?;
        match key {
            "ai.enabled" => self.ai.enabled = value.as_bool().unwrap_or(false),
            "fidget.enabled" => self.fidget.enabled = value.as_bool().unwrap_or(true),
            "fidget.seamless" => self.fidget.seamless = value.as_bool().unwrap_or(false),
            "fidget.connection" => self.fidget.connection = opt_str_from_value(value),
            "fidget.model" => self.fidget.model = opt_str_from_value(value),
            "scribbles.enabled" => self.scribbles.enabled = value.as_bool().unwrap_or(true),
            "scribbles.connection" => self.scribbles.connection = opt_str_from_value(value),
            "scribbles.model" => self.scribbles.model = opt_str_from_value(value),
            "scribbles.endpoint" => self.scribbles.endpoint = opt_str_from_value(value),
            "scribbles.template" => self.scribbles.template = opt_str_from_value(value),
            "appearance.interface_style" => self.appearance.interface_style = value.as_str().unwrap_or("aurora").to_owned(),
            "appearance.theme" => self.appearance.theme = value.as_str().unwrap_or("dark").to_owned(),
            "appearance.accessible_contrast" => self.appearance.accessible_contrast = value.as_bool().unwrap_or(false),
            "appearance.accent" => self.appearance.accent = value.as_str().unwrap_or("teal").to_owned(),
            "appearance.ui_scale" => self.appearance.ui_scale = value.as_f64().unwrap_or(1.0),
            "project.root" => self.project.root = opt_str_from_value(value),
            "project.storyboard_template" => self.project.storyboard_template = value.as_str().unwrap_or("magic").to_owned(),
            "project.storyboard_format" => self.project.storyboard_format = value.as_str().unwrap_or("designcraft").to_owned(),
            "lyrics.default_size" => self.lyrics.default_size = value.as_i64().unwrap_or(72),
            "lyrics.outline" => self.lyrics.outline = value.as_bool().unwrap_or(true),
            "lyrics.font_dirs" => self.lyrics.font_dirs = opt_str_from_value(value),
            "animation.loop_preview" => self.animation.loop_preview = value.as_bool().unwrap_or(true),
            "animation.duration" => self.animation.duration = value.as_f64().unwrap_or(4.0),
            "general.at_startup" => self.general.at_startup = value.as_str().unwrap_or("showHome").to_owned(),
            "general.when_opening_project" => self.general.when_opening_project = value.as_str().unwrap_or("showOpenDialog").to_owned(),
            "general.show_tooltips" => self.general.show_tooltips = value.as_bool().unwrap_or(true),
            "general.show_rich_tooltips" => self.general.show_rich_tooltips = value.as_bool().unwrap_or(true),
            "general.recent_projects" => self.general.recent_projects = value.as_str().unwrap_or_default().to_owned(),
            "autosave.enabled" => self.autosave.enabled = value.as_bool().unwrap_or(true),
            "autosave.interval_minutes" => self.autosave.interval_minutes = value.as_i64().unwrap_or(5),
            _ => return Err(bad(format!("unknown setting `{key}`; see prefs.schema"))),
        }
        Ok(())
    }
}

struct SettingsInner {
    store: SettingsStore<MagicPrefs>,
    has_problems: bool,
    read_only: bool,
    broken_moved_to: Option<PathBuf>,
    last_save_error: Option<String>,
}

/// Long-lived in-memory [`SettingsStore<MagicPrefs>`] handle owned by a [`crate::Session`].
#[derive(Clone)]
pub struct SessionSettings {
    inner: Arc<Mutex<SettingsInner>>,
}

impl fmt::Debug for SessionSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SessionSettings").field("path", &self.path()).field("persistent", &self.is_persistent()).finish()
    }
}

impl Default for SessionSettings {
    fn default() -> Self {
        Self::memory()
    }
}

impl SessionSettings {
    pub fn open_at(loc: Location) -> Self {
        let (store, report) = SettingsStore::<MagicPrefs>::open(loc);
        Self {
            inner: Arc::new(Mutex::new(SettingsInner {
                store,
                has_problems: report.has_problems(),
                read_only: report.read_only.is_some(),
                broken_moved_to: report.broken_moved_to,
                last_save_error: None,
            })),
        }
    }

    /// In-memory store (`Location::Memory`) for tests and isolated headless sessions.
    pub fn memory() -> Self {
        Self::open_at(Location::Memory)
    }

    /// Platform/portable persistent store (`Location::Auto`) opened once per user session.
    pub fn auto() -> Self {
        Self::open_at(Location::Auto)
    }

    pub fn prefs(&self) -> MagicPrefs {
        match self.inner.lock() {
            Ok(guard) => guard.store.get().clone(),
            Err(p) => p.into_inner().store.get().clone(),
        }
    }

    pub fn path(&self) -> Option<PathBuf> {
        match self.inner.lock() {
            Ok(guard) => guard.store.path().map(Path::to_path_buf),
            Err(p) => p.into_inner().store.path().map(Path::to_path_buf),
        }
    }

    pub fn is_persistent(&self) -> bool {
        match self.inner.lock() {
            Ok(guard) => guard.store.is_persistent(),
            Err(p) => p.into_inner().store.is_persistent(),
        }
    }

    pub fn report_json(&self) -> Value {
        let guard = match self.inner.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        json!({
            "persistent": guard.store.is_persistent(),
            "path": guard.store.path(),
            "has_problems": guard.has_problems,
            "read_only": guard.read_only,
            "broken_moved_to": guard.broken_moved_to,
            "last_save_error": guard.last_save_error,
        })
    }

    pub fn get_value(&self, key: &str) -> Value {
        self.prefs().get_value(key)
    }

    pub fn mutate(&self, f: impl FnOnce(&mut MagicPrefs), flush_now: bool) -> Result<()> {
        let mut guard = match self.inner.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.store.update(f);
        if flush_now {
            guard.store.flush().map_err(|e| {
                guard.last_save_error = Some(e.to_string());
                bad(format!("failed to save config.toml: {e}"))
            })?;
            guard.last_save_error = None;
        }
        Ok(())
    }

    pub fn set_value(&self, key: &str, value: &Value, flush_now: bool) -> Result<Value> {
        validate_schema_value(key, value)?;
        let mut guard = match self.inner.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let mut err = None;
        guard.store.update(|prefs| {
            if let Err(e) = prefs.set_value(key, value) {
                err = Some(e);
            }
        });
        if let Some(e) = err {
            return Err(e);
        }
        if flush_now {
            guard.store.flush().map_err(|e| {
                guard.last_save_error = Some(e.to_string());
                bad(format!("failed to save config.toml: {e}"))
            })?;
            guard.last_save_error = None;
        }
        Ok(guard.store.get().get_value(key))
    }

    /// Drive debounced `craft-settings` persistence from the desktop event loop (`500 ms` after
    /// the last change), returning `next_deadline` so `eframe` schedules a repaint when idle.
    pub fn tick(&self, now: Instant) -> Option<Instant> {
        let mut guard = match self.inner.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let res = guard.store.tick(now);
        if let SaveStatus::Failed(err) = &res.status {
            guard.last_save_error = Some(err.to_string());
        } else {
            guard.last_save_error = None;
        }
        res.next_deadline
    }

    pub fn flush(&self) -> Result<()> {
        let mut guard = match self.inner.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.store.flush().map_err(|e| {
            guard.last_save_error = Some(e.to_string());
            bad(format!("failed to save config.toml: {e}"))
        })?;
        guard.last_save_error = None;
        Ok(())
    }
}

/// Process-level compatibility accessor for stateless callers outside a [`crate::Session`].
pub fn open_store() -> SettingsStore<MagicPrefs> {
    SettingsStore::<MagicPrefs>::open(Location::Auto).0
}
