//! Canonical top-level `magic/` workspace hierarchy (`magic/projects`, `magic/assets`,
//! `magic/animations`) and shared asset catalog (`magic/assets/catalog.json`) management.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{EngineError, Result, Session};

pub const WORKSPACE_FORMAT: &str = "magic.workspace";
pub const SHARED_ASSET_BUCKETS: &[&str] = &["characters", "backgrounds", "images", "vectors", "3d"];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MagicWorkspaceLayout {
    pub magic_root: PathBuf,
    pub projects_dir: PathBuf,
    pub assets_dir: PathBuf,
    pub animations_dir: PathBuf,
    pub workspace_file: PathBuf,
    pub catalog_file: PathBuf,
}

/// Normalize a caller-provided workspace base so `.../magic`, `.../magic/projects`, or a parent
/// directory resolves cleanly to `<magic_root>/` containing `projects/`, `assets/`, and `animations/`.
pub fn resolve_magic_root(raw: &Path, create_named_magic_subdir: bool) -> PathBuf {
    let name = raw.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    if name.eq_ignore_ascii_case("projects") || name.eq_ignore_ascii_case("animations") || name.eq_ignore_ascii_case("assets") {
        if let Some(parent) = raw.parent().filter(|p| !p.as_os_str().is_empty()) {
            return parent.to_path_buf();
        }
    }
    if name.eq_ignore_ascii_case("magic") || raw.join("workspace.json").is_file() {
        return raw.to_path_buf();
    }
    if raw.join("projects").is_dir() && (raw.join("assets").is_dir() || raw.join("animations").is_dir()) {
        return raw.to_path_buf();
    }
    if create_named_magic_subdir { raw.join("magic") } else { raw.to_path_buf() }
}

/// Ensure `<magic_root>/projects`, `<magic_root>/assets` (with typed buckets + `catalog.json`),
/// `<magic_root>/animations`, and `<magic_root>/workspace.json` exist on disk via `craft-atomic`.
pub fn ensure_magic_workspace(magic_root: &Path) -> Result<MagicWorkspaceLayout> {
    let projects_dir = magic_root.join("projects");
    let assets_dir = magic_root.join("assets");
    let animations_dir = magic_root.join("animations");

    std::fs::create_dir_all(&projects_dir)?;
    std::fs::create_dir_all(&assets_dir)?;
    for bucket in SHARED_ASSET_BUCKETS {
        std::fs::create_dir_all(assets_dir.join(bucket))?;
    }
    std::fs::create_dir_all(&animations_dir)?;

    let workspace_file = magic_root.join("workspace.json");
    if !workspace_file.is_file() {
        let ws_doc = json!({
            "format": WORKSPACE_FORMAT,
            "version": 1,
            "projects_dir": "projects",
            "assets_dir": "assets",
            "animations_dir": "animations",
        });
        let _ = crate::doc_state::write_json(&workspace_file, &ws_doc)?;
    }

    let catalog_file = assets_dir.join("catalog.json");
    if !catalog_file.is_file() {
        let cat_doc = json!({
            "format": "magic.asset_catalog",
            "version": 1,
            "assets": [],
        });
        let _ = crate::doc_state::write_json(&catalog_file, &cat_doc)?;
    }

    let canon_root = std::fs::canonicalize(magic_root).unwrap_or_else(|_| magic_root.to_path_buf());
    Ok(MagicWorkspaceLayout {
        projects_dir: canon_root.join("projects"),
        assets_dir: canon_root.join("assets"),
        animations_dir: canon_root.join("animations"),
        workspace_file: canon_root.join("workspace.json"),
        catalog_file: canon_root.join("assets/catalog.json"),
        magic_root: canon_root,
    })
}

/// When creating a project inside `<root>/projects/<slug>` or `<root>/animations/<slug>`, ensure
/// the sibling `projects`, `assets` (`catalog.json`), and `animations` directories also exist.
pub fn ensure_sibling_magic_folders(project_dir: &Path) {
    let Some(tier_dir) = project_dir.parent() else {
        return;
    };
    let Some(tier_name) = tier_dir.file_name().and_then(|n| n.to_str()) else {
        return;
    };
    if (tier_name.eq_ignore_ascii_case("projects") || tier_name.eq_ignore_ascii_case("animations"))
        && let Some(magic_root) = tier_dir.parent().filter(|p| !p.as_os_str().is_empty())
    {
        let _ = ensure_magic_workspace(magic_root);
    }
}

/// Discover the enclosing `magic/` workspace root for an open `.magic` project if it sits inside
/// `<magic_root>/projects/<slug>/` or `<magic_root>/animations/<slug>/`.
pub fn enclosing_magic_root(project_dir: &Path) -> Option<PathBuf> {
    let tier = project_dir.parent()?;
    let tier_name = tier.file_name()?.to_str()?;
    if tier_name.eq_ignore_ascii_case("projects") || tier_name.eq_ignore_ascii_case("animations") {
        let candidate = tier.parent()?;
        if !candidate.as_os_str().is_empty() {
            return Some(candidate.to_path_buf());
        }
    }
    None
}

/// Copy and register a project asset into `<magic_root>/assets/<bucket>/` and `catalog.json`.
pub fn promote_asset_to_workspace(
    magic_root: &Path,
    asset_id: &str,
    name: &str,
    kind: &str,
    bucket: &str,
    source_file: &Path,
    overwrite: bool,
) -> Result<PathBuf> {
    let layout = ensure_magic_workspace(magic_root)?;
    let safe_bucket = if SHARED_ASSET_BUCKETS.contains(&bucket) { bucket } else { "images" };
    let dest_dir = layout.assets_dir.join(safe_bucket).join(asset_id);
    std::fs::create_dir_all(&dest_dir)?;
    let fname = source_file.file_name().ok_or_else(|| EngineError::InvalidParams(format!("invalid asset file name: {}", source_file.display())))?;
    let dest = dest_dir.join(fname);
    if source_file.is_dir() {
        crate::doc_state::atomic_copy_dir(source_file, &dest, overwrite)?;
    } else {
        crate::doc_state::atomic_copy_file(source_file, &dest, overwrite)?;
    }
    let rel = dest.strip_prefix(&layout.assets_dir).unwrap_or(&dest).to_string_lossy().replace('\\', "/");

    let mut cat: Value = std::fs::read_to_string(&layout.catalog_file)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_else(|| json!({"format": "magic.asset_catalog", "version": 1, "assets": []}));
    if !cat["assets"].is_array() {
        cat["assets"] = json!([]);
    }
    let entry = json!({
        "id": asset_id,
        "name": name,
        "kind": kind,
        "bucket": safe_bucket,
        "path": rel,
    });
    if let Some(arr) = cat["assets"].as_array_mut() {
        if let Some(slot) = arr.iter_mut().find(|v| v["id"].as_str() == Some(asset_id)) {
            *slot = entry;
        } else {
            arr.push(entry);
        }
    }
    let _ = crate::doc_state::write_json(&layout.catalog_file, &cat)?;
    Ok(dest)
}

/// Resolve an asset entry by `id` from `<magic_root>/assets/catalog.json`.
pub fn find_workspace_catalog_asset(magic_root: &Path, asset_id: &str) -> Option<(PathBuf, Value)> {
    let catalog_path = magic_root.join("assets/catalog.json");
    let raw = std::fs::read_to_string(catalog_path).ok()?;
    let doc: Value = serde_json::from_str(&raw).ok()?;
    let entry = doc["assets"].as_array()?.iter().find(|v| v["id"].as_str() == Some(asset_id))?.clone();
    let rel = entry["path"].as_str()?;
    let abs = magic_root.join("assets").join(rel);
    abs.exists().then_some((abs, entry))
}

/// `project.workspace_init`: initialize or inspect a canonical `magic/` workspace folder
/// (`magic/projects`, `magic/assets`, `magic/animations`, `magic/workspace.json`, `magic/assets/catalog.json`).
pub fn cmd_workspace_init(s: &mut Session, params: &Value) -> Result<Value> {
    let base = match params.get("path").or_else(|| params.get("root")).and_then(Value::as_str).map(str::trim).filter(|v| !v.is_empty()) {
        Some(raw) => {
            let resolved = crate::workspace::resolve_workspace_path(s, raw);
            let create_sub = params["create_magic_subdir"].as_bool().unwrap_or(false);
            resolve_magic_root(&resolved, create_sub)
        }
        None => {
            let projects = crate::project_settings::root_for(s)?;
            resolve_magic_root(&projects, false)
        }
    };
    let layout = ensure_magic_workspace(&base)?;
    if params["set_default"].as_bool() == Some(true) {
        let _ = crate::project_settings::command(s, &json!({"root": layout.projects_dir}));
    }
    Ok(json!({
        "ok": true,
        "magic_root": layout.magic_root,
        "projects_dir": layout.projects_dir,
        "assets_dir": layout.assets_dir,
        "animations_dir": layout.animations_dir,
        "workspace_file": layout.workspace_file,
        "catalog_file": layout.catalog_file,
    }))
}
