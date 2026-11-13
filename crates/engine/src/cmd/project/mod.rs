//! Self-contained `.magic` project lifecycle (`project.*` and ArtCraft `file.*` suite aliases).
//!
//! Every Magic Studio project is a self-contained `<project-slug>/` directory that owns copies of
//! its audio, assets, `storyboard.designcraft`, `effects/*.ecproj`, and `film/*.fcproj`, storing
//! relative paths inside `<slug>.magic`.

pub mod archive;
pub mod assets;
pub mod catalog;
pub mod intake;
pub mod new_project;
pub mod settings;
pub mod transcript;
pub mod trunk;
pub mod ui_state;
pub mod workshop;

pub use archive::cmd_project_migrate;
pub use assets::{
    cmd_project_add_asset, cmd_project_list_assets, cmd_project_load_asset, cmd_project_remove_asset, cmd_project_save_asset, cmd_project_update_asset,
};
pub use new_project::cmd_project_new;
pub(crate) use new_project::copy_audio_into_project;
pub use trunk::{cmd_project_import_file, cmd_project_trunk};
pub use ui_state::{cmd_project_snapshot, save_theatre_state, theatre_state};

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::{EngineError, Result, Session};

const PROJECT_FORMAT: &str = "magic.project";
const MANAGED_KEYS: [&str; 7] = ["format", "version", "name", "assets", "workshop", "mode", "song"];

fn saved_path(path: &Path, project: &Path) -> PathBuf {
    let parent = project.parent().unwrap_or(Path::new("."));
    if let Ok(rel) = path.strip_prefix(parent) {
        return rel.to_path_buf();
    }
    if let (Ok(canon_path), Ok(canon_parent)) = (path.canonicalize(), parent.canonicalize())
        && let Ok(rel) = canon_path.strip_prefix(&canon_parent)
    {
        return rel.to_path_buf();
    }
    path.to_path_buf()
}

fn opened_path(path: &str, project: &Path) -> PathBuf {
    if let Ok(resolved) = craft_docpath::resolve_absolute(path, Some(project)) {
        return resolved;
    }
    let parent = project.parent().unwrap_or(Path::new("."));
    let raw = Path::new(path);
    if raw.is_absolute() { raw.to_path_buf() } else { parent.join(raw) }
}

pub fn project_document(s: &Session) -> Value {
    let Some(path) = &s.project_path else {
        return Value::Null;
    };
    let name = path.file_stem().and_then(|n| n.to_str()).unwrap_or("Untitled");
    let mut doc = json!({
        "format": PROJECT_FORMAT,
        "version": 1,
        "name": name,
        "assets": s.project_assets,
        "workshop": {
            "audio": s.song_audio,
            "storyboard": "storyboard.designcraft",
            "draft_approved": s.draft_approved,
            "target": s.video_target,
            "video_project": s.video_project_path,
        }
    });
    let song = json!(s.song);
    if song.as_object().is_some_and(|fields| !fields.is_empty()) {
        doc["song"] = song;
    }
    if let Some(mode) = &s.project_mode {
        doc["mode"] = json!(mode);
    }
    normalize_connections(&mut doc, path);
    doc
}

fn merged_document(s: &Session, path: &Path) -> Value {
    let current = project_document(s);
    let mut doc = read_project_document(path).ok().filter(Value::is_object).unwrap_or_else(|| json!({}));
    for key in MANAGED_KEYS {
        match current.get(key) {
            Some(value) => doc[key] = value.clone(),
            None => {
                if let Some(map) = doc.as_object_mut() {
                    map.remove(key);
                }
            }
        }
    }
    doc
}

fn normalize_connections(doc: &mut Value, project: &Path) {
    for asset in doc["assets"].as_array_mut().into_iter().flatten() {
        if let Some(path) = asset["path"].as_str() {
            asset["path"] = json!(saved_path(Path::new(path), project));
        }
    }
    for key in ["audio", "video_project"] {
        if let Some(path) = doc["workshop"][key].as_str() {
            doc["workshop"][key] = json!(saved_path(Path::new(path), project));
        }
    }
}

pub fn project_is_dirty(s: &Session, saved: &Value) -> bool {
    let mut saved = saved.clone();
    if let Some(path) = &s.project_path {
        normalize_connections(&mut saved, path);
    }
    let current = project_document(s);
    ["assets", "workshop", "mode", "song"].iter().any(|key| saved[*key] != current[*key])
}

fn history_dir(project: &Path) -> PathBuf {
    project.parent().unwrap_or(Path::new(".")).join(".history")
}

fn autosave_journal_path(project: &Path) -> PathBuf {
    let fname = project.file_name().and_then(|n| n.to_str()).unwrap_or("project.magic");
    history_dir(project).join(format!("{fname}.autosave"))
}

fn rotate_project_backups(project: &Path) {
    let Ok(existing_bytes) = std::fs::read(project) else {
        return;
    };
    let hdir = history_dir(project);
    if std::fs::create_dir_all(&hdir).is_err() {
        return;
    }
    let fname = project.file_name().and_then(|n| n.to_str()).unwrap_or("project.magic");
    let bak = |n: u8| hdir.join(format!("{fname}.bak.{n}"));
    if let Ok(b2) = std::fs::read(bak(2)) {
        let _ = crate::doc_state::write_private_file(bak(3), &b2);
    }
    if let Ok(b1) = std::fs::read(bak(1)) {
        let _ = crate::doc_state::write_private_file(bak(2), &b1);
    }
    let _ = crate::doc_state::write_private_file(bak(1), &existing_bytes);
}

fn finalize_project_write(s: &Session, path: &Path, stamp: Option<crate::doc_state::FileStamp>, written: usize) {
    s.project_stamp.set(stamp);
    let journal = autosave_journal_path(path);
    if journal.is_file() {
        let _ = std::fs::remove_file(&journal);
    }
    s.project_saves.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if s.record_user_state {
        crate::change_marker::invalidate();
        crate::activity::stored(path, written);
    }
}

pub fn write_project_file(s: &Session) -> Result<()> {
    let Some(path) = &s.project_path else {
        return Ok(());
    };
    if path.exists() && !path.is_file() {
        return Err(EngineError::InvalidParams("Project destination must be a regular file; no write performed".into()));
    }
    if s.project_stamp.get().is_some() {
        crate::link_watch::save_guard(path, s.project_stamp.get())?;
    }
    rotate_project_backups(path);
    let doc = merged_document(s, path);
    let (stamp, written) = crate::doc_state::write_json(path, &doc)?;
    finalize_project_write(s, path, stamp, written);
    Ok(())
}

pub(crate) fn create_initial_project_file(s: &Session, overwrite: bool) -> Result<()> {
    if overwrite {
        return write_project_file(s);
    }
    let Some(path) = &s.project_path else {
        return Ok(());
    };
    let doc = project_document(s);
    let (stamp, written) = match crate::doc_state::write_json_new(path, &doc) {
        Ok(res) => res,
        Err(EngineError::Io(e)) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(EngineError::InvalidParams(format!("project already exists: {} (pass \"overwrite\": true to replace it)", path.display())));
        }
        Err(e) => return Err(e),
    };
    finalize_project_write(s, path, stamp, written);
    Ok(())
}

pub fn cmd_project_autosave(s: &mut Session, _: &Value) -> Result<Value> {
    let Some(path) = &s.project_path else {
        return Ok(json!({"ok": false, "reason": "no_project"}));
    };
    let journal = autosave_journal_path(path);
    let (_, bytes) = crate::doc_state::write_private_json(&journal, &merged_document(s, path))?;
    Ok(json!({"ok": true, "journal": journal, "bytes": bytes}))
}

pub(crate) fn project_slug(name: &str) -> String {
    let mut slug = String::new();
    for c in name.trim().chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }
    slug.trim_end_matches('-').to_string()
}

pub(crate) fn validate_project_name(name: &str) -> Result<String> {
    if name.contains(['/', '\\']) {
        return Err(EngineError::InvalidParams("a project name cannot contain / or \\".into()));
    }
    let slug = project_slug(name);
    if slug.is_empty() {
        return Err(EngineError::InvalidParams("a project name needs at least one letter or digit".into()));
    }
    Ok(slug)
}

pub(crate) fn project_file_path(s: &Session, params: &Value) -> Result<PathBuf> {
    let text = |key: &str| params.get(key).and_then(Value::as_str).map(str::trim).filter(|v| !v.is_empty());
    let raw = match (text("path"), text("root"), text("name")) {
        (Some(path), _, _) => path.to_string(),
        (None, Some(root), Some(name)) => {
            let slug = validate_project_name(name)?;
            crate::workspace::expand_tilde(root).join(&slug).join(format!("{slug}.magic")).to_string_lossy().into_owned()
        }
        (None, None, _) => return Err(EngineError::InvalidParams("give the project `path`, or a `root` folder and a `name`".into())),
        (None, Some(_), None) => return Err(EngineError::InvalidParams("give the project `name` to create it in `root`".into())),
    };
    let mut path = crate::workspace::resolve_workspace_path(s, &raw);
    if path.extension().is_none_or(|e| !e.eq_ignore_ascii_case("magic")) {
        if path.is_dir()
            && let Some(found) = std::fs::read_dir(&path)
                .into_iter()
                .flatten()
                .flatten()
                .map(|e| e.path())
                .find(|p| p.is_file() && p.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("magic")))
        {
            return std::fs::canonicalize(found).map_err(EngineError::from);
        }
        path.as_mut_os_string().push(".magic");
    }
    if path.exists() { std::fs::canonicalize(path).map_err(EngineError::from) } else { Ok(path) }
}

pub(crate) fn reset_for_project(s: &mut Session, path: PathBuf, assets: Vec<crate::ProjectAsset>) {
    let stamp = crate::link_watch::FileStamp::of(&path);
    *s = Session {
        cwd: s.cwd.clone(),
        settings: s.settings.clone(),
        record_user_state: s.record_user_state,
        control_address: s.control_address,
        automation_roots: s.automation_roots.clone(),
        project_assets: assets,
        project_path: Some(path),
        project_stamp: std::cell::Cell::new(stamp),
        ..Session::default()
    };
}

pub(crate) fn recent_warning(s: &Session, path: &Path, requested: bool) -> Vec<Value> {
    if !s.record_user_state || !requested {
        return Vec::new();
    }
    match crate::cmd::project::catalog::record_for(s, &path.to_string_lossy()) {
        Ok(()) => Vec::new(),
        Err(error) => vec![json!({
            "code": "recent_history_unavailable",
            "path": crate::cmd::project::catalog::file_for(s),
            "error": error,
            "project": path,
        })],
    }
}

pub(crate) fn project_mode(value: &Value) -> Result<Option<String>> {
    match value.get("mode").and_then(Value::as_str) {
        None | Some("project") => Ok(None),
        Some("animation") => Ok(Some("animation".into())),
        Some(other) => Err(EngineError::InvalidParams(format!("mode must be animation or project, not {other}"))),
    }
}

pub(crate) fn read_project_document(path: &Path) -> Result<Value> {
    serde_json::from_str(&std::fs::read_to_string(path)?).map_err(|e| EngineError::InvalidParams(format!("invalid project file: {e}")))
}

fn apply_opened_state(s: &mut Session, path: &Path, doc: &Value) -> Result<()> {
    let mut assets: Vec<crate::ProjectAsset> = serde_json::from_value(doc.get("assets").cloned().unwrap_or_else(|| json!([])))
        .map_err(|e| EngineError::InvalidParams(format!("invalid project assets: {e}")))?;
    for asset in &mut assets {
        if let Some(source) = asset.path.as_ref().and_then(|p| p.to_str()).map(str::to_owned) {
            asset.path = Some(opened_path(&source, path));
        }
    }
    let song = serde_json::from_value(doc.get("song").cloned().unwrap_or_else(|| json!({})))
        .map_err(|e| EngineError::InvalidParams(format!("invalid song metadata: {e}")))?;
    reset_for_project(s, path.to_path_buf(), assets);
    s.project_mode = project_mode(doc)?;
    s.song = song;
    s.song_audio = doc.pointer("/workshop/audio").and_then(Value::as_str).map(|src| opened_path(src, path));
    s.draft_approved = doc.pointer("/workshop/draft_approved").and_then(Value::as_bool).unwrap_or(false);
    if let Some(target) = doc.pointer("/workshop/target").and_then(Value::as_str).filter(|t| crate::cmd::project::workshop::VIDEO_TARGETS.contains(t)) {
        s.video_target = target.into();
    }
    s.video_project_path = doc.pointer("/workshop/video_project").and_then(Value::as_str).map(|src| opened_path(src, path));
    Ok(())
}

pub fn cmd_project_open(s: &mut Session, params: &Value) -> Result<Value> {
    let path = project_file_path(s, params)?;
    if params["read_only"] != true
        && let Some(project_dir) = path.parent()
    {
        let _ = magic_util::media::sweep_orphan_craft_tmp(project_dir);
    }
    let journal = autosave_journal_path(&path);
    let use_journal = params["recover_autosave"] == true && journal.is_file();
    let doc = read_project_document(if use_journal { &journal } else { &path })?;
    if doc.get("format").and_then(Value::as_str) != Some(PROJECT_FORMAT) {
        return Err(EngineError::InvalidParams(format!("{} is not a Magic Studio project", path.display())));
    }
    apply_opened_state(s, &path, &doc)?;
    let missing: Vec<String> = s.project_assets.iter().filter(|a| a.path.as_ref().is_some_and(|p| !p.exists())).map(|a| a.id.clone()).collect();
    if params["read_only"] != true
        && let Ok(disk_doc) = read_project_document(&path)
        && project_is_dirty(s, &disk_doc)
    {
        let _ = write_project_file(s);
    }
    let mut warnings = recent_warning(s, &path, params["record_recent"] != false);
    if journal.is_file() && !use_journal {
        warnings.push(json!({
            "code": "autosave_journal_available",
            "journal": journal,
            "hint": "Pass recover_autosave=true to restore crash-recovery journal",
        }));
    }
    let workshop = crate::cmd::project::workshop::workshop_status(s)?;
    Ok(json!({
        "ok": true,
        "project": path,
        "mode": s.project_mode,
        "warnings": warnings,
        "assets": s.project_assets,
        "missing": missing,
        "workshop": workshop,
        "next_steps": workshop["next_steps"],
    }))
}

fn flush_dirty_documents(s: &mut Session) -> Result<Vec<Value>> {
    let mut saved = Vec::new();
    for doc in &mut s.docs {
        if doc.dirty {
            doc.save()?;
            saved.push(json!({"kind": doc.kind, "path": doc.path, "revision": doc.revision}));
        }
    }
    Ok(saved)
}

pub fn cmd_project_save(s: &mut Session, params: &Value) -> Result<Value> {
    if params.get("expected_project").is_some() {
        let selected = s.project_path.as_ref().ok_or_else(|| EngineError::InvalidParams("Select the intended project".into()))?.canonicalize()?;
        let expected = project_file_path(s, &json!({"path": params["expected_project"]}))?.canonicalize()?;
        if selected != expected {
            return Err(EngineError::InvalidParams("expected_project differs from selected project".into()));
        }
    }
    if s.project_path.is_none() {
        return Err(EngineError::InvalidParams("no project is open; use project.new".into()));
    }
    if let Some(target) = params.get("path").map(|_| project_file_path(s, params)).transpose()?
        && s.project_path.as_ref() != Some(&target)
    {
        return save_as(s, target);
    }
    let documents = flush_dirty_documents(s)?;
    write_project_file(s)?;
    s.edits_since_save = 0;
    Ok(json!({
        "ok": true,
        "project": s.project_path,
        "saved": s.project_path,
        "saves": s.project_saves.load(std::sync::atomic::Ordering::Relaxed),
        "documents": documents,
        "relinked": [],
        "warnings": [],
    }))
}

pub fn cmd_file_save_as(s: &mut Session, params: &Value) -> Result<Value> {
    if params.get("path").and_then(Value::as_str).map(str::trim).filter(|p| !p.is_empty()).is_none() {
        return Err(EngineError::InvalidParams("file.saveAs requires `path`".into()));
    }
    if s.project_path.is_none() {
        return Err(EngineError::InvalidParams("no project is open; use file.new or project.new".into()));
    }
    save_as(s, project_file_path(s, params)?)
}

fn copy_project_tree(old_dir: &Path, new_dir: &Path) -> Result<Vec<PathBuf>> {
    if old_dir.canonicalize().ok() == new_dir.canonicalize().ok() {
        return Ok(Vec::new());
    }
    let mut relinked = Vec::new();
    for entry in std::fs::read_dir(old_dir).into_iter().flatten().flatten().map(|e| e.path()) {
        let Some(fname) = entry.file_name().and_then(|n| n.to_str()) else { continue };
        if fname.starts_with('.') || entry.extension().is_some_and(|e| e.eq_ignore_ascii_case("magic")) {
            continue;
        }
        let dst = new_dir.join(fname);
        if entry.is_dir() {
            crate::doc_state::atomic_copy_dir(&entry, &dst, false)?;
            relinked.push(dst);
        } else if entry.is_file() {
            crate::doc_state::atomic_copy_file(&entry, &dst, false)?;
            relinked.push(dst);
        }
    }
    Ok(relinked)
}

fn save_as(s: &mut Session, target: PathBuf) -> Result<Value> {
    let old_dir = s.project_path.as_deref().and_then(Path::parent).map(Path::to_path_buf);
    let target_parent = target.parent().ok_or_else(|| EngineError::InvalidParams("Save As target must reside in a directory".into()))?;
    std::fs::create_dir_all(target_parent)?;
    let target_dir = std::fs::canonicalize(target_parent).unwrap_or_else(|_| target_parent.to_path_buf());
    let target = target.file_name().map_or_else(|| target.clone(), |name| target_dir.join(name));
    let documents = flush_dirty_documents(s)?;
    let relinked = match old_dir {
        Some(ref prev_dir) if *prev_dir != target_dir => copy_project_tree(prev_dir, &target_dir)?,
        _ => Vec::new(),
    };
    if let Some(ref prev_dir) = old_dir {
        if let Some(rel) = s.song_audio.as_ref().and_then(|a| a.strip_prefix(prev_dir).ok()).map(Path::to_path_buf) {
            s.song_audio = Some(target_dir.join(rel));
        }
        if let Some(rel) = s.video_project_path.as_ref().and_then(|v| v.strip_prefix(prev_dir).ok()).map(Path::to_path_buf) {
            s.video_project_path = Some(target_dir.join(rel));
        }
        for asset in &mut s.project_assets {
            if let Some(rel) = asset.path.as_ref().and_then(|p| p.strip_prefix(prev_dir).ok()).map(Path::to_path_buf) {
                asset.path = Some(target_dir.join(rel));
            }
        }
    }
    let previous_path = s.project_path.replace(target);
    let previous_stamp = s.project_stamp.replace(None);
    if let Err(error) = write_project_file(s) {
        s.project_path = previous_path;
        s.project_stamp.set(previous_stamp);
        return Err(error);
    }
    if let Some(path) = s.project_path.as_ref().and_then(|p| std::fs::canonicalize(p).ok()) {
        s.project_path = Some(path);
    }
    let warnings = s.project_path.as_deref().map_or_else(Vec::new, |saved| recent_warning(s, saved, true));
    s.edits_since_save = 0;
    Ok(json!({
        "ok": true,
        "project": s.project_path,
        "saved": s.project_path,
        "saves": s.project_saves.load(std::sync::atomic::Ordering::Relaxed),
        "documents": documents,
        "relinked": relinked,
        "warnings": warnings,
    }))
}

pub fn cmd_project_close(s: &mut Session, _: &Value) -> Result<Value> {
    *s = Session {
        cwd: s.cwd.clone(),
        settings: s.settings.clone(),
        record_user_state: s.record_user_state,
        control_address: s.control_address,
        automation_roots: s.automation_roots.clone(),
        ..Session::default()
    };
    Ok(json!({"ok": true}))
}

pub fn ensure_default_asset_project(s: &mut Session) -> Result<PathBuf> {
    if let Some(existing) = &s.project_path {
        return Ok(existing.clone());
    }
    let root = crate::workspace::magic_data_dir().unwrap_or_else(|| crate::workspace::workspace_root(s)).join("recovery/scratch");
    std::fs::create_dir_all(&root)?;
    let default_proj = root.join("scratch.magic");
    let _ = if default_proj.is_file() {
        cmd_project_open(s, &json!({"path": default_proj, "record_recent": false}))
    } else {
        cmd_project_new(s, &json!({"path": default_proj, "overwrite": true, "allow_empty_audio": true}))
    };
    s.project_path.clone().ok_or_else(|| EngineError::InvalidParams("could not initialize scratch asset project".into()))
}

pub fn cmd_project_identity(s: &mut Session, p: &Value) -> Result<Value> {
    let path = if p.get("path").is_some() {
        project_file_path(s, p)?
    } else {
        s.project_path.clone().ok_or_else(|| EngineError::InvalidParams("No project is open".into()))?
    };
    let path = std::fs::canonicalize(path)?;
    Ok(json!({"ok": true, "project": path, "project_id": path, "identity_kind": "canonical path"}))
}

pub fn cmd_project_attach(s: &mut Session, p: &Value) -> Result<Value> {
    let mut p = p.clone();
    p["record_recent"] = json!(false);
    p["read_only"] = json!(true);
    let mut value = cmd_project_open(s, &p)?;
    value["attachment"] = json!("read-only disk metadata; Studio unchanged");
    Ok(value)
}
