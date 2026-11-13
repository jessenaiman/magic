//! Migration-only `original-project.zip` bundler (`project.migrate`) powered by [`craft_zip`].
//!
//! Per Magic's self-contained project rule, a `.zip` archive exists **only** when migrating an
//! existing external project into `<project-slug>/original-project.zip`. `Archive::open` runs
//! raw-byte entry-count and size verification directly (`craft-libs/crates/zip/README.md`).

use std::path::{Path, PathBuf};

use craft_zip::{Archive, Compression, Limits, Writer};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::{EngineError, Result, Session};

const ARCHIVE_LIMITS: Limits = Limits { max_entries: 25_000, max_total_bytes: 2 * 1024 * 1024 * 1024 };
const SKIP_DIRS: &[&str] = &[".git", "target", "node_modules", "__pycache__", ".cache"];

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
        if path.is_dir() && !SKIP_DIRS.iter().any(|skip| name.eq_ignore_ascii_case(skip)) {
            collect_relative_files(root, &path, out)?;
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

/// Bundle a migrated legacy project folder into `<project_dir>/original-project.zip`.
pub fn archive_directory_to_zip(source_dir: &Path, dest_zip: &Path) -> Result<Value> {
    if !source_dir.is_dir() {
        return Err(EngineError::InvalidParams(format!("migration source directory does not exist: {}", source_dir.display())));
    }
    let mut files = Vec::new();
    collect_relative_files(source_dir, source_dir, &mut files)?;
    files.sort_by(|a, b| a.0.cmp(&b.0));

    let dest_canon = dest_zip.canonicalize().ok();
    let mut writer = Writer::new();
    let mut total_uncompressed: u64 = 0;
    for (rel_name, disk_path) in &files {
        if dest_canon.as_deref() == disk_path.canonicalize().ok().as_deref() {
            continue;
        }
        let data = std::fs::read(disk_path)?;
        total_uncompressed = total_uncompressed.saturating_add(data.len() as u64);
        writer.add(rel_name, &data, Compression::Deflated).map_err(|e| EngineError::CommandFailed("project.migrate".into(), e.to_string()))?;
    }
    let zip_bytes = writer.finish().map_err(|e| EngineError::CommandFailed("project.migrate".into(), e.to_string()))?;
    let reader = Archive::open(&zip_bytes, ARCHIVE_LIMITS).map_err(|e| EngineError::CommandFailed("project.migrate".into(), e.to_string()))?;
    if reader.has_duplicate_names() {
        return Err(EngineError::CommandFailed("project.migrate".into(), "duplicate entry names in original-project.zip".into()));
    }
    if let Some(parent) = dest_zip.parent() {
        std::fs::create_dir_all(parent)?;
    }
    craft_atomic::atomic_write(dest_zip, &zip_bytes)?;
    Ok(json!({
        "ok": true,
        "source_dir": source_dir,
        "archive": dest_zip,
        "entry_count": files.len(),
        "uncompressed_bytes": total_uncompressed,
        "compressed_bytes": zip_bytes.len(),
        "sha256": format!("{:x}", Sha256::digest(&zip_bytes)),
    }))
}

/// `project.migrate`: preserve a single `original-project.zip` inside the open `<project-slug>/` directory.
pub fn cmd_project_migrate(s: &mut Session, p: &Value) -> Result<Value> {
    let raw_src = p
        .get("source")
        .or_else(|| p.get("migrate_from"))
        .and_then(Value::as_str)
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| EngineError::InvalidParams("project.migrate requires `source` (legacy project directory)".into()))?;
    let source_dir = crate::workspace::resolve_workspace_path(s, raw_src);
    let project_dir = crate::workshop::project_dir(s)?;
    let dest_zip = project_dir.join("original-project.zip");
    if let Some(roots) = &s.automation_roots {
        roots.check_write(&dest_zip)?;
    }
    archive_directory_to_zip(&source_dir, &dest_zip)
}
