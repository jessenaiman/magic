//! Project lifecycle: `.magic` create/open/save/close (`project.*` and ArtCraft `file.*` suite aliases)
//! and sub-module re-exports.
//!
//! Includes atomic writes (`craft_atomic::atomic_write`), rolling `.history/<stem>.magic.bak.{1..3}`
//! backups (`storytold/filmcraft/docs/project-files.md`), and crash-recovery `.autosave` journals.

pub mod assets;
pub mod catalog;
pub mod intake;
pub mod settings;
pub mod transcript;
pub mod trunk;
pub mod ui_state;
pub mod workshop;

pub use assets::{cmd_project_add_asset, cmd_project_list_assets, cmd_project_load_asset, cmd_project_remove_asset, cmd_project_update_asset};
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
            "storyboard": "storyboard.pdf",
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

/// Rotate up to 3 rolling `.magic.bak.{1..3}` snapshots inside `<project_dir>/.history/` using `craft_atomic`.
fn rotate_project_backups(project: &Path) {
    if !project.is_file() {
        return;
    }
    let Ok(existing_bytes) = std::fs::read(project) else {
        return;
    };
    let hdir = history_dir(project);
    if std::fs::create_dir_all(&hdir).is_err() {
        return;
    }
    let fname = project.file_name().and_then(|n| n.to_str()).unwrap_or("project.magic");
    let bak1 = hdir.join(format!("{fname}.bak.1"));
    let bak2 = hdir.join(format!("{fname}.bak.2"));
    let bak3 = hdir.join(format!("{fname}.bak.3"));
    if let Ok(b2) = std::fs::read(&bak2) {
        let _ = craft_atomic::atomic_write(&bak3, &b2);
    }
    if let Ok(b1) = std::fs::read(&bak1) {
        let _ = craft_atomic::atomic_write(&bak2, &b1);
    }
    let _ = craft_atomic::atomic_write(&bak1, &existing_bytes);
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
    s.project_stamp.set(stamp);
    let journal = autosave_journal_path(path);
    if journal.is_file() {
        let _ = std::fs::remove_file(&journal);
    }
    if s.record_user_state {
        crate::change_marker::invalidate();
    }
    s.project_saves.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if s.record_user_state {
        crate::activity::stored(path, written);
    }
    Ok(())
}

/// Write crash-recovery `.autosave` journal (~1s debounce from UI) without overwriting the user's primary `.magic` file.
pub fn cmd_project_autosave(s: &mut Session, _: &Value) -> Result<Value> {
    let Some(path) = &s.project_path else {
        return Ok(json!({"ok": false, "reason": "no_project"}));
    };
    let journal = autosave_journal_path(path);
    let doc = merged_document(s, path);
    let (_, bytes) = crate::doc_state::write_json(&journal, &doc)?;
    Ok(json!({
        "ok": true,
        "journal": journal,
        "bytes": bytes,
    }))
}

fn project_slug(name: &str) -> String {
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

pub(crate) fn project_file_path(s: &Session, params: &Value) -> Result<PathBuf> {
    let text = |key: &str| params.get(key).and_then(Value::as_str).map(str::trim).filter(|v| !v.is_empty());
    let raw = match (text("path"), text("root"), text("name")) {
        (Some(path), _, _) => path.to_string(),
        (None, Some(root), Some(name)) => {
            if name.contains(['/', '\\']) {
                return Err(EngineError::InvalidParams("a project name cannot contain / or \\".into()));
            }
            let slug = project_slug(name);
            if slug.is_empty() {
                return Err(EngineError::InvalidParams("a project name needs at least one letter or digit".into()));
            }
            Path::new(root).join(slug).join(format!("{name}.magic")).to_string_lossy().into_owned()
        }
        (None, None, _) => return Err(EngineError::InvalidParams("give the project `path`, or a `root` folder and a `name`".into())),
        (None, Some(_), None) => return Err(EngineError::InvalidParams("give the project `name` to create it in `root`".into())),
    };
    let mut path = crate::workspace::resolve_workspace_path(s, &raw);
    if path.extension().is_none_or(|e| !e.eq_ignore_ascii_case("magic")) {
        path.as_mut_os_string().push(".magic");
    }
    if path.exists() {
        return std::fs::canonicalize(&path).map_err(EngineError::from);
    }
    Ok(path)
}

fn reset_for_project(s: &mut Session, path: PathBuf, assets: Vec<crate::ProjectAsset>) {
    let stamp = crate::link_watch::FileStamp::of(&path);
    *s = Session {
        cwd: s.cwd.clone(),
        record_user_state: s.record_user_state,
        control_address: s.control_address,
        automation_roots: s.automation_roots.clone(),
        project_assets: assets,
        project_path: Some(path),
        project_stamp: std::cell::Cell::new(stamp),
        ..Session::default()
    };
}

fn recent_warning(s: &Session, path: &Path, requested: bool) -> Vec<Value> {
    if !s.record_user_state || !requested {
        return Vec::new();
    }
    match crate::cmd::project::catalog::record(&path.to_string_lossy()) {
        Ok(()) => Vec::new(),
        Err(error) => vec![json!({
            "code": "recent_history_unavailable",
            "stage": "update_recent_projects",
            "path": crate::cmd::project::catalog::file(),
            "error": error,
            "project": path,
            "effect": "Project is available; Home recent history was not updated",
            "next_action": "Continue using the returned project path; repair only the named navigation-history location if needed",
        })],
    }
}

fn bootstrap_new_project_workshop(s: &mut Session, path: &Path, params: &Value) -> Vec<Value> {
    let mut steps = Vec::new();
    let project_title =
        params.get("title").or_else(|| params.get("name")).and_then(Value::as_str).or_else(|| path.file_stem().and_then(|n| n.to_str())).unwrap_or("Untitled");

    let sb = path.with_file_name("storyboard.pdf");
    if !sb.is_file() || params["overwrite"] == true {
        let mut sb_req = json!({
            "path": sb,
            "overwrite": params["overwrite"] == true,
            "title": project_title,
        });
        if let Some(wiz) = params.get("wizard") {
            sb_req["wizard"] = wiz.clone();
        }
        match crate::cmd::storyboard::cmd_storyboard_new(s, &sb_req) {
            Ok(_) => steps.push(json!({"command": "storyboard.new", "status": "complete; storyboard.pdf initialized from read-only master template"})),
            Err(e) => steps.push(json!({"command": "storyboard.new", "status": "failed", "error": e.to_string()})),
        }
    }

    let Some(raw_audio) = params.get("audio").and_then(Value::as_str).filter(|v| !v.is_empty()) else {
        return steps;
    };
    if params["skip_audio_setup"] == true {
        return steps;
    }
    match crate::cmd::project::workshop::cmd_workshop_set_audio(s, &json!({"path": raw_audio, "resolve_video_cli": params["resolve_video_cli"]})) {
        Ok(_) => steps.push(json!({"command": "workshop.set_audio", "status": "complete; audio linked into FilmCraft sequence"})),
        Err(e) => {
            steps.push(json!({"command": "workshop.set_audio", "status": "failed", "error": e.to_string()}));
            return steps;
        }
    }
    if let Some(t) = params.get("transcript").and_then(Value::as_str).filter(|v| !v.is_empty()) {
        match crate::cmd::project::workshop::cmd_workshop_import_transcript(s, &json!({"path": t, "captions": params["captions"], "replace": true})) {
            Ok(_) => {
                steps.push(json!({"command": "workshop.import_transcript", "status": "complete; timing-reference.json + synced FilmCraft lyric track ready"}))
            }
            Err(e) => steps.push(json!({"command": "workshop.import_transcript", "status": "failed", "error": e.to_string()})),
        }
    } else if params.get("process_audio").and_then(Value::as_bool).unwrap_or(true) && params["prepare_only"] != true {
        let model = params.get("model").and_then(Value::as_str).unwrap_or("large-v3");
        match crate::cmd::project::workshop::cmd_audio_analyze(
            s,
            &json!({"analyze_beats": true, "model": model, "resolve_video_cli": params["resolve_video_cli"]}),
        ) {
            Ok(_) => steps.push(
                json!({"command": "audio.analyze", "status": "complete; transcribed audio, wrote timing-reference.json, and synced FilmCraft lyric track"}),
            ),
            Err(e) => steps.push(json!({"command": "audio.analyze", "status": "failed", "error": e.to_string()})),
        }
    }
    steps
}

fn check_overwrite(path: &Path, overwrite: bool) -> Result<()> {
    if path.exists() && !overwrite {
        return Err(EngineError::InvalidParams(format!("project already exists: {} (pass \"overwrite\": true to replace it)", path.display())));
    }
    Ok(())
}

fn project_audio_source(s: &Session, params: &Value, require_audio: bool) -> Result<Option<PathBuf>> {
    match params.get("audio").and_then(Value::as_str).filter(|v| !v.trim().is_empty()) {
        Some(raw) => Ok(Some(crate::workspace::song_audio_path(s, raw)?)),
        None if require_audio => Err(EngineError::InvalidParams(
            "project.new requires `audio` (<song file>) so Magic can transcribe timestamps (`timing-reference.json`), sync the FilmCraft lyric track, and initialize the storyboard (or pass mode=\"animation\" for a standalone character study)".into(),
        )),
        None => Ok(None),
    }
}

fn project_mode(value: &Value) -> Result<Option<String>> {
    match value.get("mode").and_then(Value::as_str) {
        None | Some("project") => Ok(None),
        Some("animation") => Ok(Some("animation".into())),
        Some(other) => Err(EngineError::InvalidParams(format!("mode must be animation or project, not {other}"))),
    }
}

/// Create a new `.magic` session (`magic-cli new` / `project.new` / `file.new`).
pub fn cmd_project_new(s: &mut Session, params: &Value) -> Result<Value> {
    let path = project_file_path(s, params)?;
    let mode = project_mode(params)?;
    let overwrite = params.get("overwrite").and_then(Value::as_bool).unwrap_or(false);
    check_overwrite(&path, overwrite)?;
    let is_isolated_scratch = params["allow_empty_audio"] == true;
    let require_audio = mode.is_none() && !is_isolated_scratch;
    let audio = project_audio_source(s, params, require_audio)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let path = match (path.parent().and_then(|p| std::fs::canonicalize(p).ok()), path.file_name()) {
        (Some(parent), Some(name)) => parent.join(name),
        _ => path,
    };
    reset_for_project(s, path.clone(), Vec::new());
    s.song_audio = audio;
    s.project_mode = mode;
    write_project_file(s)?;
    let steps = if s.project_mode.is_some() || is_isolated_scratch { Vec::new() } else { bootstrap_new_project_workshop(s, &path, params) };
    write_project_file(s)?;
    let warnings = recent_warning(s, &path, true);
    let workshop = crate::cmd::project::workshop::workshop_status(s)?;
    let storyboard = crate::cmd::storyboard::cmd_storyboard_check(s, &json!({})).unwrap_or(Value::Null);
    Ok(json!({
        "ok": true,
        "project": path,
        "mode": s.project_mode,
        "warnings": warnings,
        "steps": steps,
        "assets": s.project_assets,
        "workshop": workshop,
        "storyboard": storyboard,
        "workflow": [
            "1. Audio transcribed -> FilmCraft project with synced lyric track + timing-reference.json",
            "2. Storyboard created -> run `magic-cli storyboard wizard` or edit Main Section Title Pages",
            "3. Complete Main Section Title Pages first (A-Cover, B-World, C-Character, D-Motion)",
            "4. Generate bulk images from title-page references via `magic-cli storyboard bulk_generate`",
            "5. Once all required Storyboard sections are Green, proceed to Video Production (Lyrics, Motion, Animation, Studio)"
        ],
    }))
}

pub(crate) fn read_project_document(path: &Path) -> Result<Value> {
    let text = std::fs::read_to_string(path)?;
    serde_json::from_str(&text).map_err(|e| EngineError::InvalidParams(format!("invalid project file: {e}")))
}

fn project_assets_from_doc(path: &Path, doc: &Value) -> Result<Vec<crate::ProjectAsset>> {
    let mut assets: Vec<crate::ProjectAsset> = serde_json::from_value(doc.get("assets").cloned().unwrap_or_else(|| json!([])))
        .map_err(|e| EngineError::InvalidParams(format!("invalid project assets: {e}")))?;
    for asset in &mut assets {
        if let Some(source) = asset.path.as_ref().and_then(|p| p.to_str()).map(str::to_owned) {
            asset.path = Some(opened_path(&source, path));
        }
    }
    Ok(assets)
}

fn apply_opened_state(s: &mut Session, path: &Path, doc: &Value, assets: Vec<crate::ProjectAsset>) -> Result<()> {
    let song = serde_json::from_value(doc.get("song").cloned().unwrap_or_else(|| json!({})))
        .map_err(|e| EngineError::InvalidParams(format!("invalid song metadata: {e}")))?;
    reset_for_project(s, path.to_path_buf(), assets);
    s.project_mode = project_mode(doc)?;
    s.song = song;
    s.song_audio = doc.pointer("/workshop/audio").and_then(Value::as_str).map(|source| opened_path(source, path));
    s.draft_approved = doc.pointer("/workshop/draft_approved").and_then(Value::as_bool).unwrap_or(false);
    if let Some(target) = doc.pointer("/workshop/target").and_then(Value::as_str).filter(|t| crate::cmd::project::workshop::VIDEO_TARGETS.contains(t)) {
        s.video_target = target.into();
    }
    s.video_project_path = doc.pointer("/workshop/video_project").and_then(Value::as_str).map(|source| opened_path(source, path));
    Ok(())
}

pub fn cmd_project_open(s: &mut Session, params: &Value) -> Result<Value> {
    let path = project_file_path(s, params)?;
    if params["read_only"] != true
        && let Some(project_dir) = path.parent()
    {
        let _ = craft_util::media::sweep_orphan_craft_tmp(project_dir);
    }
    let journal = autosave_journal_path(&path);
    let use_journal = params["recover_autosave"] == true && journal.is_file();
    let doc = if use_journal {
        read_project_document(&journal)?
    } else {
        read_project_document(&path)?
    };
    if doc.get("format").and_then(Value::as_str) != Some(PROJECT_FORMAT) {
        return Err(EngineError::InvalidParams(format!("{} is not a Magic Studio project", path.display())));
    }
    let assets = project_assets_from_doc(&path, &doc)?;
    let missing: Vec<&str> = assets.iter().filter(|a| a.path.as_ref().is_some_and(|p| !p.exists())).map(|a| a.id.as_str()).collect::<Vec<_>>();
    let missing = json!(missing);
    apply_opened_state(s, &path, &doc, assets)?;
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

fn project_guard(s: &Session, p: &Value) -> Result<()> {
    let selected = s.project_path.as_ref().ok_or_else(|| EngineError::InvalidParams("Select the intended project".into()))?.canonicalize()?;
    let expected = project_file_path(s, &json!({"path": p["expected_project"]}))?.canonicalize()?;
    if selected != expected {
        return Err(EngineError::InvalidParams("expected_project differs from selected project".into()));
    }
    Ok(())
}

fn flush_dirty_documents(s: &mut Session) -> Result<Vec<Value>> {
    let mut saved = Vec::new();
    for doc in &mut s.docs {
        if doc.dirty {
            doc.save()?;
            saved.push(json!({
                "kind": doc.kind,
                "path": doc.path,
                "revision": doc.revision,
            }));
        }
    }
    Ok(saved)
}

pub fn cmd_project_save(s: &mut Session, params: &Value) -> Result<Value> {
    if params.get("expected_project").is_some() {
        project_guard(s, params)?;
    }
    if s.project_path.is_none() {
        return Err(EngineError::InvalidParams("no project is open; use project.new".into()));
    }
    let target = match params.get("path") {
        Some(_) => Some(project_file_path(s, params)?),
        None => None,
    };
    match target {
        Some(target) if s.project_path.as_ref() != Some(&target) => save_as(s, target),
        _ => {
            let documents = flush_dirty_documents(s)?;
            write_project_file(s)?;
            s.edits_since_save = 0;
            Ok(json!({
                "ok": true,
                "project": s.project_path,
                "saved": s.project_path,
                "saves": s.project_saves.load(std::sync::atomic::Ordering::Relaxed),
                "documents": documents,
                "native": [],
                "derivatives": [],
                "relinked": [],
                "warnings": [],
            }))
        }
    }
}

/// Explicit `file.saveAs` handler (`path` required), mirroring `FilmCraft` and `VectorCraft`.
pub fn cmd_file_save_as(s: &mut Session, params: &Value) -> Result<Value> {
    if params.get("path").and_then(Value::as_str).map(str::trim).filter(|p| !p.is_empty()).is_none() {
        return Err(EngineError::InvalidParams("file.saveAs requires `path`".into()));
    }
    if s.project_path.is_none() {
        return Err(EngineError::InvalidParams("no project is open; use file.new or project.new".into()));
    }
    let target = project_file_path(s, params)?;
    save_as(s, target)
}

fn save_as(s: &mut Session, target: PathBuf) -> Result<Value> {
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if target.exists() && !target.is_file() {
        return Err(EngineError::InvalidParams("Project destination must be a regular file; no write performed".into()));
    }
    let target = match (target.parent().and_then(|p| std::fs::canonicalize(p).ok()), target.file_name()) {
        (Some(parent), Some(name)) => parent.join(name),
        _ => target,
    };
    let documents = flush_dirty_documents(s)?;
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
    let warnings = match &s.project_path {
        Some(saved) => recent_warning(s, saved, true),
        None => Vec::new(),
    };
    s.edits_since_save = 0;
    Ok(json!({
        "ok": true,
        "project": s.project_path,
        "saved": s.project_path,
        "saves": s.project_saves.load(std::sync::atomic::Ordering::Relaxed),
        "documents": documents,
        "native": [],
        "derivatives": [],
        "relinked": [],
        "warnings": warnings,
    }))
}

pub fn cmd_project_close(s: &mut Session, _: &Value) -> Result<Value> {
    *s = Session {
        cwd: s.cwd.clone(),
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
    let root = crate::workspace::workspace_root(s).join("exports/scratch");
    std::fs::create_dir_all(&root)?;
    let default_proj = root.join("scratch.magic");
    if default_proj.is_file() {
        let _ = cmd_project_open(s, &json!({"path": default_proj, "record_recent": false}));
    } else {
        let _ = cmd_project_new(s, &json!({"path": default_proj, "overwrite": true, "allow_empty_audio": true}));
    }
    s.project_path.clone().ok_or_else(|| EngineError::InvalidParams("could not initialize default asset project".into()))
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
