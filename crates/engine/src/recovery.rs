//! Crash recovery under `<MAGIC_CONFIG_DIR>/Recovery/run-<pid>-<seq>/`.
//!
//! Creates a `0700` directory and `0600` private files (`craft_atomic::NewFilePermissions::Private`)
//! only when [`Session::is_dirty`] is true and persistence is enabled (`resolved_config_dir().is_some()`).
//! Holds an advisory file lock (`lock`) while alive so orphaned recovery folders left after a crash
//! are distinguished from other running Magic Studio instances.

use std::fs::{File, OpenOptions, TryLockError};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::cmd::project::format::Project;
use crate::{EngineError, Result, Session};

static NEXT_RUN_SEQ: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryMetadata {
    pub run_id: String,
    pub pid: u32,
    pub project_path: Option<PathBuf>,
    pub name: String,
    pub saved_at_utc: String,
    pub revision: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct RecoveryEntry {
    pub run_id: String,
    pub pid: u32,
    pub project_path: Option<PathBuf>,
    pub name: String,
    pub saved_at_utc: String,
    pub revision: u64,
    pub snapshot_path: PathBuf,
}

#[derive(Debug)]
pub struct RecoveryRun {
    pub run_id: String,
    pub run_dir: PathBuf,
    _lock_file: File,
    pub last_revision: Option<u64>,
}

fn ensure_private_dir(dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

fn open_private_lock(path: &Path) -> std::io::Result<File> {
    let mut opts = OpenOptions::new();
    opts.create(true).read(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    opts.open(path)
}

fn try_lock_orphan_dir(run_dir: &Path) -> Option<File> {
    let lock_path = run_dir.join("lock");
    let file = open_private_lock(&lock_path).ok()?;
    match file.try_lock() {
        Ok(()) => Some(file),
        Err(TryLockError::WouldBlock | TryLockError::Error(_)) => None,
    }
}

pub fn recovery_root_for(s: &Session) -> Option<PathBuf> {
    if !s.settings.is_persistent() {
        return None;
    }
    crate::settings::prefs::resolved_config_dir().map(|dir| dir.join("Recovery"))
}

pub fn active_run_id(s: &Session) -> Option<&str> {
    s.recovery_run.as_ref().map(|r| r.run_id.as_str())
}

pub fn sync_dirty_in(recovery_root: &Path, s: &mut Session) -> Result<bool> {
    if !s.is_dirty() {
        clear_active(s);
        return Ok(false);
    }
    let (Some(project), Some(project_path)) = (s.project.as_ref(), s.project_path.as_ref()) else {
        return Ok(false);
    };
    if s.recovery_run.as_ref().is_some_and(|r| r.last_revision == Some(s.project_revision)) {
        return Ok(false);
    }

    ensure_private_dir(recovery_root)?;
    if s.recovery_run.is_none() {
        let seq = NEXT_RUN_SEQ.fetch_add(1, Ordering::Relaxed);
        let run_id = format!("run-{}-{seq}", std::process::id());
        let run_dir = recovery_root.join(&run_id);
        ensure_private_dir(&run_dir)?;
        let lock_file = open_private_lock(&run_dir.join("lock"))?;
        lock_file.try_lock().map_err(|e| EngineError::Io(std::io::Error::other(format!("recovery lock failed: {e}"))))?;
        s.recovery_run = Some(RecoveryRun { run_id, run_dir, _lock_file: lock_file, last_revision: None });
    }

    let Some(run) = s.recovery_run.as_mut() else {
        return Ok(false);
    };
    let meta = RecoveryMetadata {
        run_id: run.run_id.clone(),
        pid: std::process::id(),
        project_path: Some(project_path.clone()),
        name: project.name.clone(),
        saved_at_utc: Utc::now().to_rfc3339(),
        revision: s.project_revision,
    };
    let persisted = project.to_persisted_value(project_path);
    crate::doc_state::write_private_json(&run.run_dir.join("project.magic"), &persisted)?;
    crate::doc_state::write_private_json(&run.run_dir.join("recovery.json"), &meta)?;
    run.last_revision = Some(s.project_revision);
    Ok(true)
}

pub fn sync_dirty(s: &mut Session) {
    let Some(root) = recovery_root_for(s) else {
        return;
    };
    let _ = sync_dirty_in(&root, s);
}

pub fn clear_active(s: &mut Session) {
    if let Some(run) = s.recovery_run.take() {
        let dir = run.run_dir.clone();
        drop(run);
        let _ = std::fs::remove_dir_all(dir);
    }
}

pub fn list_orphans_in(recovery_root: &Path, own_run_id: Option<&str>) -> Result<Vec<RecoveryEntry>> {
    if !recovery_root.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(recovery_root)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir() && p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("run-")))
        .collect();
    dirs.sort();

    for run_dir in dirs {
        let Some(run_id) = run_dir.file_name().and_then(|n| n.to_str()).map(str::to_owned) else {
            continue;
        };
        if own_run_id == Some(run_id.as_str()) {
            continue;
        }
        let Some(_probe_lock) = try_lock_orphan_dir(&run_dir) else {
            continue;
        };
        let meta_path = run_dir.join("recovery.json");
        let snapshot_path = run_dir.join("project.magic");
        if !meta_path.is_file() || !snapshot_path.is_file() {
            continue;
        }
        let Ok(meta_bytes) = std::fs::read(&meta_path) else { continue };
        let Ok(meta) = serde_json::from_slice::<RecoveryMetadata>(&meta_bytes) else { continue };
        out.push(RecoveryEntry {
            run_id,
            pid: meta.pid,
            project_path: meta.project_path,
            name: meta.name,
            saved_at_utc: meta.saved_at_utc,
            revision: meta.revision,
            snapshot_path,
        });
    }
    out.sort_by(|a, b| b.saved_at_utc.cmp(&a.saved_at_utc));
    Ok(out)
}

pub fn restore_in(recovery_root: &Path, s: &mut Session, run_id: Option<&str>) -> Result<Value> {
    let entries = list_orphans_in(recovery_root, active_run_id(s))?;
    let entry = match run_id.map(str::trim).filter(|id| !id.is_empty()) {
        Some(wanted) => {
            entries.into_iter().find(|e| e.run_id == wanted).ok_or_else(|| EngineError::InvalidParams(format!("unknown recovery run_id `{wanted}`")))?
        }
        None => entries.into_iter().next().ok_or_else(|| EngineError::InvalidParams("no recoverable snapshots available".into()))?,
    };
    let run_dir = recovery_root.join(&entry.run_id);
    let _lock =
        try_lock_orphan_dir(&run_dir).ok_or_else(|| EngineError::InvalidParams(format!("recovery run `{}` is locked by another process", entry.run_id)))?;
    let orig_path = entry.project_path.clone().ok_or_else(|| EngineError::InvalidParams("recovery snapshot has no original project path".into()))?;
    let doc: Value = serde_json::from_slice(&std::fs::read(&entry.snapshot_path)?)
        .map_err(|e| EngineError::InvalidParams(format!("invalid recovery project snapshot: {e}")))?;
    let project = Project::from_value(&doc, &orig_path)?;

    clear_active(s);
    s.reset_for_opened_project(orig_path.clone(), project);
    let rev = entry.revision.max(1);
    s.project_revision = rev;
    s.next_revision = rev;
    s.saved_revision = 0;
    drop(_lock);
    let _ = std::fs::remove_dir_all(&run_dir);
    let _ = sync_dirty_in(recovery_root, s);

    Ok(json!({
        "ok": true,
        "restored": entry.run_id,
        "project": orig_path,
        "name": entry.name,
        "dirty": s.is_dirty(),
        "revision": s.project_revision,
    }))
}

pub fn discard_in(recovery_root: &Path, own_run_id: Option<&str>, run_id: Option<&str>, all: bool) -> Result<Value> {
    let entries = list_orphans_in(recovery_root, own_run_id)?;
    let targets: Vec<RecoveryEntry> = if all {
        entries
    } else {
        let wanted = run_id.map(str::trim).filter(|id| !id.is_empty()).ok_or_else(|| EngineError::InvalidParams("pass `run_id` or `all: true`".into()))?;
        let found =
            entries.into_iter().find(|e| e.run_id == wanted).ok_or_else(|| EngineError::InvalidParams(format!("unknown recovery run_id `{wanted}`")))?;
        vec![found]
    };

    let mut discarded = Vec::new();
    for entry in targets {
        let run_dir = recovery_root.join(&entry.run_id);
        let Some(_lock) = try_lock_orphan_dir(&run_dir) else {
            continue;
        };
        drop(_lock);
        if std::fs::remove_dir_all(&run_dir).is_ok() {
            discarded.push(entry.run_id);
        }
    }
    Ok(json!({"ok": true, "discarded": discarded}))
}

pub fn cmd_recovery_list(s: &mut Session, _params: &Value) -> Result<Value> {
    let Some(root) = recovery_root_for(s) else {
        return Ok(json!({"ok": true, "enabled": false, "count": 0, "entries": []}));
    };
    let entries = list_orphans_in(&root, active_run_id(s))?;
    Ok(json!({"ok": true, "enabled": true, "root": root, "count": entries.len(), "entries": entries}))
}

pub fn cmd_recovery_restore(s: &mut Session, params: &Value) -> Result<Value> {
    let root = recovery_root_for(s).ok_or_else(|| EngineError::InvalidParams("recovery is disabled in ephemeral config mode".into()))?;
    restore_in(&root, s, params.get("run_id").and_then(Value::as_str))
}

pub fn cmd_recovery_discard(s: &mut Session, params: &Value) -> Result<Value> {
    let Some(root) = recovery_root_for(s) else {
        return Ok(json!({"ok": true, "discarded": []}));
    };
    let all = params.get("all").and_then(Value::as_bool).unwrap_or(false);
    discard_in(&root, active_run_id(s), params.get("run_id").and_then(Value::as_str), all)
}
