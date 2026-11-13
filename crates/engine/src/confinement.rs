//! Automation file-access confinement (`--automation-read-root` and `--automation-write-root`),
//! matching the ArtCraft suite (`VectorCraft`, `DesignCraft`, `PhotoCraft`, `EffectCraft`, `FilmCraft`).
//!
//! Reference:
//! - <https://github.com/storytold/designcraft/blob/main/docs/mcp.md>

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::{EngineError, Result, Session};

/// Canonicalized read and write root allowlists for automated sessions (`magic-cli mcp` or
/// `magic --control <port> --automation-read-root ... --automation-write-root ...`).
#[derive(Debug, Clone, Default)]
pub struct AutomationRoots {
    pub read_roots: Vec<PathBuf>,
    pub write_roots: Vec<PathBuf>,
}

impl AutomationRoots {
    /// Parse repeatable `--automation-read-root` and `--automation-write-root` directory lists.
    /// Returns `Ok(None)` when neither flag was given.
    pub fn from_args(read: &[String], write: &[String]) -> std::result::Result<Option<Self>, String> {
        if read.is_empty() && write.is_empty() {
            return Ok(None);
        }
        let mut read_roots = Vec::with_capacity(read.len());
        for raw in read {
            let p = Path::new(raw);
            if !p.is_absolute() {
                return Err(format!("--automation-read-root must be an absolute existing folder: {raw}"));
            }
            let canon = fs::canonicalize(p).map_err(|e| format!("--automation-read-root {raw}: {e}"))?;
            if !canon.is_dir() {
                return Err(format!("--automation-read-root is not a directory: {raw}"));
            }
            read_roots.push(canon);
        }
        let mut write_roots = Vec::with_capacity(write.len());
        for raw in write {
            let p = Path::new(raw);
            if !p.is_absolute() {
                return Err(format!("--automation-write-root must be an absolute existing folder: {raw}"));
            }
            let canon = fs::canonicalize(p).map_err(|e| format!("--automation-write-root {raw}: {e}"))?;
            if !canon.is_dir() {
                return Err(format!("--automation-write-root is not a directory: {raw}"));
            }
            write_roots.push(canon);
        }
        Ok(Some(Self { read_roots, write_roots }))
    }

    pub fn check_read(&self, candidate: &Path) -> Result<PathBuf> {
        let resolved = resolve_no_symlink_escape(candidate)?;
        if self.read_roots.iter().any(|root| resolved.starts_with(root)) {
            Ok(resolved)
        } else {
            Err(EngineError::InvalidParams(format!("path is outside --automation-read-root: {}", candidate.display())))
        }
    }

    pub fn check_write(&self, candidate: &Path) -> Result<PathBuf> {
        let resolved = resolve_no_symlink_escape(candidate)?;
        if self.write_roots.iter().any(|root| resolved.starts_with(root)) {
            Ok(resolved)
        } else {
            Err(EngineError::InvalidParams(format!("path is outside --automation-write-root: {}", candidate.display())))
        }
    }

    /// Inspect command parameters prior to dispatch and enforce read/write root boundaries.
    pub fn check_command(&self, s: &Session, id: &str, params: &Value) -> Result<()> {
        let Some(obj) = params.as_object() else {
            return Ok(());
        };
        let is_write_cmd = matches!(
            id,
            "project.new"
                | "file.new"
                | "project.save"
                | "file.save"
                | "file.saveAs"
                | "storyboard.new"
                | "storyboard.export"
                | "storyboard.grade"
                | "film.render"
                | "video.render"
                | "words.preview"
                | "motion.render"
                | "ui.screenshot"
        );
        for (key, value) in obj {
            check_param_value(self, s, id, key, value, is_write_cmd)?;
        }
        // Saving an already-open project without `path` writes to `session.project_path`.
        if matches!(id, "project.save" | "file.save")
            && obj.get("path").is_none()
            && let Some(open_path) = &s.project_path
        {
            self.check_write(open_path)?;
        }
        Ok(())
    }
}

#[allow(clippy::cognitive_complexity)]
fn check_param_value(roots: &AutomationRoots, s: &Session, id: &str, key: &str, value: &Value, is_write_cmd: bool) -> Result<()> {
    const READ_KEYS: &[&str] = &["audio", "transcript", "captions", "image", "source", "input", "background", "source_project", "reference"];
    const WRITE_KEYS: &[&str] = &["output", "out", "out_dir", "pdf", "pptx", "png_dir", "root", "save_as"];

    match value {
        Value::String(raw) if !raw.trim().is_empty() => {
            if key == "path" {
                let resolved = crate::workspace::resolve_workspace_path(s, raw);
                if is_write_cmd {
                    roots.check_write(&resolved)?;
                } else {
                    roots.check_read(&resolved)?;
                }
            } else if WRITE_KEYS.contains(&key) {
                let resolved = crate::workspace::resolve_workspace_path(s, raw);
                roots.check_write(&resolved)?;
            } else if READ_KEYS.contains(&key) {
                let resolved = crate::workspace::resolve_workspace_path(s, raw);
                roots.check_read(&resolved)?;
            }
        }
        Value::Array(items) if key == "paths" || key == "refs" || key == "references" => {
            for item in items {
                if let Some(raw) = item.as_str().filter(|v| !v.trim().is_empty()) {
                    let resolved = crate::workspace::resolve_workspace_path(s, raw);
                    roots.check_read(&resolved)?;
                }
            }
        }
        Value::Object(nested) if key == "images" => {
            for (_, v) in nested {
                if let Some(raw) = v.as_str().filter(|x| !x.trim().is_empty()) {
                    let resolved = crate::workspace::resolve_workspace_path(s, raw);
                    roots.check_read(&resolved)?;
                }
            }
        }
        _ => {
            let _ = id;
        }
    }
    Ok(())
}

/// Resolve a candidate path (following OS symlinks on existing ancestors) so containment checks match DesignCraft (`designcraft/docs/mcp.md`).
fn resolve_no_symlink_escape(candidate: &Path) -> Result<PathBuf> {
    let abs = if candidate.is_absolute() { candidate.to_path_buf() } else { std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join(candidate) };
    if let Ok(canon) = fs::canonicalize(&abs) {
        return Ok(canon);
    }
    let mut missing_tail = Vec::new();
    let mut cur: &Path = &abs;
    while let Some(parent) = cur.parent() {
        if let Some(name) = cur.file_name() {
            missing_tail.push(name.to_os_string());
        }
        if let Ok(canon_parent) = fs::canonicalize(parent) {
            let mut rebuilt = canon_parent;
            for part in missing_tail.into_iter().rev() {
                rebuilt.push(part);
            }
            return Ok(rebuilt);
        }
        cur = parent;
    }
    Ok(abs)
}

/// `file.list {path}` — list files and folders in `path` (subject to `--automation-read-root` when set),
/// matching `VectorCraft` and `DesignCraft` (`run_command {command: "file.list", params: {path}}`).
pub fn cmd_file_list(s: &mut Session, params: &Value) -> Result<Value> {
    let raw = params["path"].as_str().map(str::trim).filter(|p| !p.is_empty()).ok_or_else(|| EngineError::InvalidParams("file.list requires `path`".into()))?;
    let resolved = crate::workspace::resolve_workspace_path(s, raw);
    let checked = if let Some(roots) = &s.automation_roots { roots.check_read(&resolved)? } else { resolved };
    if !checked.is_dir() {
        return Err(EngineError::InvalidParams(format!("not a directory: {}", checked.display())));
    }
    let mut entries = Vec::new();
    let read_dir = fs::read_dir(&checked)?;
    for entry in read_dir.flatten() {
        let p = entry.path();
        let Ok(meta) = fs::symlink_metadata(&p) else {
            continue;
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        let kind = if meta.is_dir() { "directory" } else { "file" };
        entries.push(json!({
            "name": name,
            "path": p,
            "kind": kind,
            "bytes": if meta.is_file() { Some(meta.len()) } else { None },
        }));
    }
    entries.sort_by(|a, b| a["name"].as_str().unwrap_or_default().cmp(b["name"].as_str().unwrap_or_default()));
    Ok(json!({
        "ok": true,
        "path": checked,
        "count": entries.len(),
        "entries": entries,
    }))
}
