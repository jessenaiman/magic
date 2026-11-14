//! Self-contained `.magic` project lifecycle (`project.*` and ArtCraft `file.*` suite aliases).
//!
//! Every Magic Studio project is a self-contained `<project-slug>/` directory that owns copies of
//! its audio, assets, `storyboard.designcraft`, `effects/*.ecproj`, and `film/*.fcproj`, storing
//! relative paths inside `<slug>.magic`.

pub mod assets;
pub mod catalog;
pub mod files;
pub mod intake;
pub mod new_project;
pub mod settings;
pub mod transcript;
pub mod ui_state;
pub mod workshop;

pub use assets::{
    cmd_project_add_asset, cmd_project_list_assets, cmd_project_load_asset, cmd_project_remove_asset, cmd_project_save_asset, cmd_project_update_asset,
};
pub use files::{cmd_project_files, cmd_project_import_file};
pub use new_project::cmd_project_new;
pub(crate) use new_project::copy_audio_into_project;
pub use ui_state::{cmd_project_snapshot, save_theatre_state, theatre_state};

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::{EngineError, Result, Session};

const PROJECT_FORMAT: &str = "magic.project";

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
    let sb_name = if path.with_file_name("storyboard.idml").is_file() && !path.with_file_name(crate::cmd::storyboard::DEFAULT_STORYBOARD_FILE).is_file() {
        "storyboard.idml"
    } else {
        crate::cmd::storyboard::DEFAULT_STORYBOARD_FILE
    };
    let mut doc = json!({
        "format": PROJECT_FORMAT,
        "version": 1,
        "name": name,
        "assets": s.project_assets,
        "workshop": {
            "audio": s.song_audio,
            "storyboard": sb_name,
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

fn finalize_project_write(s: &Session, stamp: Option<crate::link_watch::FileStamp>) {
    s.project_stamp.set(stamp);
    s.project_saves.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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
    let doc = project_document(s);
    let (stamp, _written) = crate::doc_state::write_json(path, &doc)?;
    finalize_project_write(s, stamp);
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
    let (stamp, _written) = match crate::doc_state::write_json_new(path, &doc) {
        Ok(res) => res,
        Err(EngineError::Io(e)) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(EngineError::InvalidParams(format!("project already exists: {} (pass \"overwrite\": true to replace it)", path.display())));
        }
        Err(e) => return Err(e),
    };
    finalize_project_write(s, stamp);
    Ok(())
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
    s.reset_for_project(Some(path), assets);
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
    let doc = read_project_document(&path)?;
    if doc.get("format").and_then(Value::as_str) != Some(PROJECT_FORMAT) {
        return Err(EngineError::InvalidParams(format!("{} is not a Magic Studio project", path.display())));
    }
    apply_opened_state(s, &path, &doc)?;
    let missing: Vec<String> = s.project_assets.iter().filter(|a| a.path.as_ref().is_some_and(|p| !p.exists())).map(|a| a.id.clone()).collect();
    let warnings = recent_warning(s, &path, params["record_recent"] != false);
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
    write_project_file(s)?;
    s.mark_saved();
    Ok(json!({
        "ok": true,
        "project": s.project_path,
        "saved": s.project_path,
        "saves": s.project_saves.load(std::sync::atomic::Ordering::Relaxed),
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
    s.mark_saved();
    Ok(json!({
        "ok": true,
        "project": s.project_path,
        "saved": s.project_path,
        "saves": s.project_saves.load(std::sync::atomic::Ordering::Relaxed),
        "relinked": relinked,
        "warnings": warnings,
    }))
}

pub fn cmd_project_close(s: &mut Session, _: &Value) -> Result<Value> {
    s.reset_for_project(None, Vec::new());
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
    let mut probe = s.clone();
    probe.record_user_state = false;
    let mut p = p.clone();
    p["record_recent"] = json!(false);
    p["read_only"] = json!(true);
    let mut value = cmd_project_open(&mut probe, &p)?;
    value["attachment"] = json!("read-only disk metadata; Studio unchanged");
    Ok(value)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::json;

    use super::{cmd_project_attach, cmd_project_new, cmd_project_open};
    use crate::{Asset, Session};

    #[test]
    fn b10_p0_project_switch_clears_stale_lyrics_cues_previews_and_selection() {
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let sandbox = std::env::temp_dir().join(format!("magic_b10_p0_{nanos}"));
        std::fs::create_dir_all(&sandbox).unwrap();

        let mut session = Session::default();
        let res_a = cmd_project_new(&mut session, &json!({"root": sandbox, "name": "Song Alpha", "mode": "animation"})).unwrap();
        let path_a = PathBuf::from(res_a["project"].as_str().unwrap());
        let dir_a = path_a.parent().unwrap().to_path_buf();

        session.song.lyrics = "Old project lyrics line 1".into();
        session.song.notes = "Old cue notes".into();
        session.lyrics_workspace = Some(dir_a.clone());
        session.video_project_path = Some(dir_a.join("film/alpha.fcproj"));
        session.draft_approved = true;
        session.active_asset = Some("hero_fox".into());
        session.assets.insert("hero_fox".into(), Asset { id: "hero_fox".into(), path: dir_a.join("fox.png"), width: 512, height: 512, has_alpha: true });
        session.record(json!({"preview": "stale_frame.png"}));

        let res_b = session.execute("project.new", json!({"root": sandbox, "name": "Song Beta", "mode": "animation"})).unwrap();
        let path_b = PathBuf::from(res_b["project"].as_str().unwrap());
        assert_eq!(session.project_path.as_deref(), Some(path_b.as_path()));
        assert!(session.song.lyrics.is_empty());
        assert!(session.song.notes.is_empty());
        assert!(session.lyrics_workspace.is_none());
        assert!(session.video_project_path.is_none());
        assert!(!session.draft_approved);
        assert!(session.active_asset.is_none());
        assert!(session.assets.is_empty());
        assert!(session.history.undo.is_empty());

        // Attaching Project A metadata must NOT clobber the active Project B session.
        let _ = cmd_project_attach(&mut session, &json!({"path": path_a})).unwrap();
        assert_eq!(session.project_path.as_deref(), Some(path_b.as_path()));
        assert!(session.song.lyrics.is_empty());

        // Re-opening Project B after staging transient state must also wipe stale session state.
        session.active_asset = Some("transient".into());
        session.lyrics_workspace = Some(dir_a);
        let _ = cmd_project_open(&mut session, &json!({"path": path_b})).unwrap();
        assert!(session.active_asset.is_none());
        assert!(session.lyrics_workspace.is_none());

        let _ = std::fs::remove_dir_all(&sandbox);
    }
}
