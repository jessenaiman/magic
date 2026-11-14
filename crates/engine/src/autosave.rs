//! Rolling `Auto-Save/` `.magic` snapshots inside the project folder (`MAX_AUTOSAVES = 20`).
//!
//! Writes only when [`Session::is_dirty`] is true and keeps [`Session::is_dirty`] unchanged (`true`)
//! because an auto-save copy in `Auto-Save/` never replaces an explicit user save (`file.save`).

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde_json::{Value, json};

use crate::{Result, Session};

pub const AUTOSAVE_DIR_NAME: &str = "Auto-Save";
pub const MAX_AUTOSAVES: usize = 20;

pub fn autosave_filename(stem: &str, now: DateTime<Utc>) -> String {
    let stamp = now.format("%Y-%m-%d_%H-%M-%S");
    let unix = now.timestamp();
    format!("{stem}-{stamp}-{unix}.magic")
}

fn prune_oldest(dir: &Path, keep: usize) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<PathBuf> =
        rd.flatten().map(|e| e.path()).filter(|p| p.is_file() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("magic"))).collect();
    if files.len() <= keep {
        return;
    }
    files.sort();
    let excess = files.len().saturating_sub(keep);
    for old in files.into_iter().take(excess) {
        let _ = std::fs::remove_file(old);
    }
}

pub fn write_autosave_now(s: &mut Session) -> Result<Option<PathBuf>> {
    if !s.settings.prefs().autosave.enabled || !s.is_dirty() {
        return Ok(None);
    }
    let (Some(project), Some(project_path)) = (s.project.as_ref(), s.project_path.as_ref()) else {
        return Ok(None);
    };
    if s.last_autosave_revision == Some(s.project_revision) {
        return Ok(None);
    }
    let Some(project_dir) = project_path.parent() else {
        return Ok(None);
    };
    let autosave_dir = project_dir.join(AUTOSAVE_DIR_NAME);
    std::fs::create_dir_all(&autosave_dir)?;
    let stem = project_path.file_stem().and_then(|st| st.to_str()).unwrap_or("project");
    let mut dest = autosave_dir.join(autosave_filename(stem, Utc::now()));
    let mut suffix = 2u32;
    while dest.exists() {
        dest = autosave_dir.join(format!("{}-{suffix}.magic", dest.file_stem().and_then(|st| st.to_str()).unwrap_or(stem)));
        suffix = suffix.saturating_add(1);
    }
    let doc = project.to_persisted_value(project_path);
    crate::doc_state::write_json(&dest, &doc)?;
    prune_oldest(&autosave_dir, MAX_AUTOSAVES);
    s.last_autosave_at = Some(Instant::now());
    s.last_autosave_revision = Some(s.project_revision);
    Ok(Some(dest))
}

pub fn save_if_due(s: &mut Session) -> Result<Option<PathBuf>> {
    let prefs = s.settings.prefs().autosave;
    if !prefs.enabled || !s.is_dirty() {
        return Ok(None);
    }
    let mins = u64::try_from(prefs.interval_minutes.max(1)).unwrap_or(1);
    let interval = Duration::from_secs(mins.saturating_mul(60));
    let elapsed_ok = s.last_autosave_at.is_none_or(|prev| prev.elapsed() >= interval);
    if !elapsed_ok {
        return Ok(None);
    }
    write_autosave_now(s)
}

pub fn cmd_project_autosave_tick(s: &mut Session, params: &Value) -> Result<Value> {
    let force = params.get("force").and_then(Value::as_bool).unwrap_or(false);
    let written = if force { write_autosave_now(s)? } else { save_if_due(s)? };
    Ok(json!({
        "ok": true,
        "written": written,
        "dirty": s.is_dirty(),
        "revision": s.project_revision,
    }))
}
