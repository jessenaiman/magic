//! Deterministic compressed folder archiving (`project.archive`) powered by [`craft_zip`]
//! (`Archive`, `Writer`, `Compression::Deflated`, `Limits`, and `preflight::preflight`).

use std::path::{Path, PathBuf};

use craft_zip::preflight::{Limits as PreflightLimits, preflight};
use craft_zip::{Archive, Compression, Limits, Writer};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::{EngineError, Result, Session};

// TODO(agent): verify craft-zip README for Limits vs PreflightLimits field names; d23f5f3 has separate types
const ARCHIVE_LIMITS: Limits = Limits { max_entries: 25_000, max_total_bytes: 2 * 1024 * 1024 * 1024 };
const PREFLIGHT_LIMITS: PreflightLimits = PreflightLimits { max_entries: 25_000, max_central_directory_bytes: 1024 * 1024, max_name_bytes: 1024 };

const SKIP_DIRS: &[&str] = &[".git", "target", "node_modules", "__pycache__", ".cache"];

fn bad(msg: impl Into<String>) -> EngineError {
    EngineError::InvalidParams(msg.into())
}

fn collect_relative_files(root: &Path, cur: &Path, out: &mut Vec<(String, PathBuf)>) -> Result<()> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(cur)?.flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name == ".DS_Store" || (name.starts_with('.') && name.ends_with(".craft_tmp")) {
            continue;
        }
        if path.is_dir() {
            if !SKIP_DIRS.iter().any(|skip| name.eq_ignore_ascii_case(skip)) {
                collect_relative_files(root, &path, out)?;
            }
        } else if path.is_file()
            && let Ok(rel) = path.strip_prefix(root)
        {
            let slash = rel.to_string_lossy().replace('\\', "/");
            if !slash.is_empty() {
                out.push((slash, path));
            }
        }
    }
    Ok(())
}

/// Compress a directory into a deterministic `.zip` archive verified with `craft_zip::preflight`.
pub fn archive_directory_to_zip(source_dir: &Path, dest_zip: &Path) -> Result<Value> {
    if !source_dir.is_dir() {
        return Err(bad(format!("source directory does not exist: {}", source_dir.display())));
    }
    let mut files = Vec::new();
    collect_relative_files(source_dir, source_dir, &mut files)?;
    files.sort_by(|a, b| a.0.cmp(&b.0));

    let mut writer = Writer::new();
    let mut total_uncompressed: u64 = 0;
    for (rel_name, disk_path) in &files {
        if Some(disk_path.as_path()) == dest_zip.canonicalize().ok().as_deref() {
            continue;
        }
        let data = std::fs::read(disk_path)?;
        total_uncompressed = total_uncompressed.saturating_add(data.len() as u64);
        writer.add(rel_name, &data, Compression::Deflated).map_err(|e| EngineError::CommandFailed("project.archive".into(), e.to_string()))?;
    }
    let zip_bytes = writer.finish().map_err(|e| EngineError::CommandFailed("project.archive".into(), e.to_string()))?;

    preflight(&zip_bytes, PREFLIGHT_LIMITS).map_err(|e| EngineError::CommandFailed("project.archive".into(), format!("craft-zip preflight failed: {e}")))?;
    let zip_reader =
        Archive::open(&zip_bytes, ARCHIVE_LIMITS).map_err(|e| EngineError::CommandFailed("project.archive".into(), format!("craft-zip open failed: {e}")))?;
    if zip_reader.has_duplicate_names() {
        return Err(EngineError::CommandFailed("project.archive".into(), "duplicate entry names in archive".into()));
    }

    if let Some(parent) = dest_zip.parent() {
        std::fs::create_dir_all(parent)?;
    }
    craft_atomic::atomic_write(dest_zip, &zip_bytes)?;
    let sha256 = format!("{:x}", Sha256::digest(&zip_bytes));

    Ok(json!({
        "ok": true,
        "source_dir": source_dir,
        "archive": dest_zip,
        "entry_count": files.len(),
        "uncompressed_bytes": total_uncompressed,
        "compressed_bytes": zip_bytes.len(),
        "sha256": sha256,
        "engine": "craft-zip",
    }))
}

/// `project.archive`: package the open project directory (or an explicit `source` folder) into a
/// deterministic compressed `.zip` bundle via `craft-zip`, or inspect an existing `.zip` archive.
pub fn cmd_project_archive(s: &mut Session, p: &Value) -> Result<Value> {
    if let Some(inspect_raw) = p.get("inspect").and_then(Value::as_str).filter(|v| !v.trim().is_empty()) {
        let zip_path = crate::workspace::resolve_workspace_path(s, inspect_raw);
        let bytes = std::fs::read(&zip_path)?;
        preflight(&bytes, PREFLIGHT_LIMITS).map_err(|e| EngineError::CommandFailed("project.archive".into(), format!("preflight failed: {e}")))?;
        let archive = Archive::open(&bytes, ARCHIVE_LIMITS).map_err(|e| EngineError::CommandFailed("project.archive".into(), format!("open failed: {e}")))?;
        return Ok(json!({
            "ok": true,
            "archive": zip_path,
            "bytes": bytes.len(),
            "has_duplicate_names": archive.has_duplicate_names(),
            "engine": "craft-zip",
        }));
    }

    let source_dir = if let Some(raw) = p.get("source").and_then(Value::as_str).filter(|v| !v.trim().is_empty()) {
        crate::workspace::resolve_workspace_path(s, raw)
    } else {
        crate::workshop::project_dir(s)?
    };
    let slug = source_dir.file_name().and_then(|n| n.to_str()).unwrap_or("project");
    let dest_zip = if let Some(out_raw) = p.get("output").or_else(|| p.get("out")).and_then(Value::as_str).filter(|v| !v.trim().is_empty()) {
        let mut cand = crate::workspace::resolve_workspace_path(s, out_raw);
        if cand.extension().and_then(|e| e.to_str()).is_none_or(|e| !e.eq_ignore_ascii_case("zip")) {
            cand.set_extension("zip");
        }
        cand
    } else {
        let parent = source_dir.parent().unwrap_or_else(|| Path::new("."));
        parent.join("archives").join(format!("{slug}.zip"))
    };
    if let Some(roots) = &s.automation_roots {
        roots.check_write(&dest_zip)?;
    }
    archive_directory_to_zip(&source_dir, &dest_zip)
}
