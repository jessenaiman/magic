//! `project.new` / `file.new`: creates a self-contained `<project-slug>/` directory owning copies
//! of all audio, assets, `storyboard.designcraft`, `effects/`, `film/`, and `exports/`.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::{cmd_project_import_file, create_initial_project_file, project_mode, recent_warning, validate_project_name, write_project_file};
use crate::cmd::project::format::Project;
use crate::{EngineError, Result, Session};

const PROJECT_SCAFFOLD_DIRS: &[&str] = &["audio", "assets", "effects", "film", "exports"];

fn clear_stale_project_artifacts(project_dir: &Path) {
    let _ = std::fs::remove_file(project_dir.join("transcript.srt"));
    for rel_dir in [".preview", "lettering/.preview", "motion/.preview", "storyboard/.preview", "film/frames"] {
        let _ = std::fs::remove_dir_all(project_dir.join(rel_dir));
    }
}

fn parent_has_other_magic_file(parent: &Path, target_file_name: &str) -> bool {
    std::fs::read_dir(parent).into_iter().flatten().flatten().map(|e| e.path()).any(|p| {
        p.is_file() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("magic")) && p.file_name().and_then(|n| n.to_str()) != Some(target_file_name)
    })
}

fn new_project_file_path(s: &Session, params: &Value, mode: Option<&str>) -> Result<PathBuf> {
    let text = |key: &str| params.get(key).and_then(Value::as_str).map(str::trim).filter(|v| !v.is_empty());
    let explicit_name = text("name").or_else(|| text("title"));

    if let Some(raw_path) = text("path") {
        let resolved = crate::workspace::resolve_workspace_path(s, raw_path);
        if params["allow_empty_audio"] == true {
            let mut scratch = resolved;
            if scratch.extension().is_none_or(|e| !e.eq_ignore_ascii_case("magic")) {
                scratch.as_mut_os_string().push(".magic");
            }
            return Ok(scratch);
        }
        let stem = explicit_name
            .map(str::to_owned)
            .or_else(|| resolved.file_stem().and_then(|st| st.to_str()).map(str::to_owned))
            .ok_or_else(|| EngineError::InvalidParams("project path must include a project name".into()))?;
        let slug = validate_project_name(&stem)?;
        let fname = format!("{stem}.magic");
        if !resolved.extension().is_some_and(|e| e.eq_ignore_ascii_case("magic")) {
            let matches_dir = resolved.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.eq_ignore_ascii_case(&slug) || n.eq_ignore_ascii_case(&stem));
            let proj_dir = if matches_dir { resolved } else { resolved.join(&slug) };
            return Ok(proj_dir.join(fname));
        }
        let parent = resolved.parent().unwrap_or(Path::new("."));
        let parent_matches = parent.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.eq_ignore_ascii_case(&slug) || n.eq_ignore_ascii_case(&stem));
        return if (parent_matches || parent.is_dir()) && !parent_has_other_magic_file(parent, &fname) {
            Ok(parent.join(fname))
        } else {
            Ok(parent.join(&slug).join(fname))
        };
    }

    let name = explicit_name.ok_or_else(|| EngineError::InvalidParams("project.new requires `name` (optionally with `root`), or `path`".into()))?;
    let slug = validate_project_name(name)?;
    let base_root = match text("root") {
        Some(raw_root) => crate::workspace::resolve_workspace_path(s, raw_root),
        None => crate::project_settings::default_root_for_mode(s, mode)?,
    };
    Ok(base_root.join(&slug).join(format!("{name}.magic")))
}

fn scaffold_project_dirs(project_dir: &Path) -> Result<Vec<String>> {
    std::fs::create_dir_all(project_dir)?;
    let mut created = Vec::with_capacity(PROJECT_SCAFFOLD_DIRS.len());
    for rel in PROJECT_SCAFFOLD_DIRS {
        std::fs::create_dir_all(project_dir.join(rel))?;
        created.push((*rel).to_owned());
    }
    let _ = magic_util::media::sweep_orphan_craft_tmp(project_dir);
    Ok(created)
}

pub(crate) fn copy_audio_into_project(project_dir: &Path, source_audio: &Path, overwrite: bool) -> Result<(PathBuf, bool)> {
    let audio_dir = project_dir.join("audio");
    std::fs::create_dir_all(&audio_dir)?;
    let file_name = source_audio.file_name().ok_or_else(|| EngineError::InvalidParams(format!("invalid audio file name: {}", source_audio.display())))?;
    let dest = audio_dir.join(file_name);
    let was_external = source_audio.canonicalize().ok() != dest.canonicalize().ok();
    crate::doc_state::atomic_copy_file(source_audio, &dest, overwrite)?;
    Ok((dest.canonicalize().unwrap_or(dest), was_external))
}

fn copy_supporting_input(s: &Session, project_dir: &Path, raw: &str, subdir: &str, overwrite: bool) -> Result<PathBuf> {
    let src = crate::workspace::resolve_workspace_path(s, raw);
    if !src.is_file() {
        return Err(EngineError::InvalidParams(format!("file not found: {}", src.display())));
    }
    if src.starts_with(project_dir) {
        return Ok(src);
    }
    let target_dir = project_dir.join(subdir);
    std::fs::create_dir_all(&target_dir)?;
    let fname = src.file_name().ok_or_else(|| EngineError::InvalidParams(format!("invalid file name: {}", src.display())))?;
    let dest = target_dir.join(fname);
    crate::doc_state::atomic_copy_file(&src, &dest, overwrite)?;
    Ok(dest)
}

fn copy_initial_collections(s: &mut Session, params: &Value, overwrite: bool, copied: &mut Vec<PathBuf>, steps: &mut Vec<Value>) {
    for (param_key, workshop_kind) in [("assets", "assets"), ("panels", "panels"), ("liner_notes", "liner_notes")] {
        let Some(items) = params.get(param_key).and_then(Value::as_array).filter(|a| !a.is_empty()) else {
            continue;
        };
        let result = if param_key == "assets" {
            cmd_project_import_file(s, &json!({"paths": items, "overwrite": overwrite})).map(|mut res| {
                res["added"] = res["imported"].take();
                res
            })
        } else {
            crate::cmd::project::workshop::cmd_workshop_add(s, &json!({"kind": workshop_kind, "paths": items, "overwrite": overwrite}))
        };
        match result {
            Ok(res) => {
                copied.extend(res["added"].as_array().into_iter().flatten().filter_map(Value::as_str).map(PathBuf::from));
                steps.push(json!({"command": format!("workshop.add({workshop_kind})"), "status": "copied into project directory"}));
            }
            Err(e) => steps.push(json!({"command": format!("workshop.add({workshop_kind})"), "status": "failed", "error": e.to_string()})),
        }
    }
    if let Some(files) = params.get("files").and_then(Value::as_array).filter(|a| !a.is_empty()) {
        match cmd_project_import_file(s, &json!({"paths": files, "overwrite": overwrite})) {
            Ok(res) => {
                copied.extend(res["imported"].as_array().into_iter().flatten().filter_map(Value::as_str).map(PathBuf::from));
                steps.push(json!({"command": "project.import_file", "status": "copied into project directory"}));
            }
            Err(e) => steps.push(json!({"command": "project.import_file", "status": "failed", "error": e.to_string()})),
        }
    }
}

fn bootstrap_song_workshop(s: &mut Session, path: &Path, project_dir: &Path, params: &Value, overwrite: bool, copied: &mut Vec<PathBuf>) -> Vec<Value> {
    let mut steps = Vec::new();
    let project_title =
        params.get("title").or_else(|| params.get("name")).and_then(Value::as_str).or_else(|| path.file_stem().and_then(|n| n.to_str())).unwrap_or("Untitled");

    copy_initial_collections(s, params, overwrite, copied, &mut steps);

    let sb = path.with_file_name(crate::cmd::storyboard::DEFAULT_STORYBOARD_FILE);
    if !sb.is_file() || overwrite {
        let sb_req = json!({"path": sb, "overwrite": overwrite});
        match crate::cmd::storyboard::cmd_storyboard_new(s, &sb_req) {
            Ok(_) => steps.push(json!({"command": "storyboard.new", "status": "storyboard.designcraft initialized"})),
            Err(e) => steps.push(json!({"command": "storyboard.new", "status": "failed", "error": e.to_string()})),
        }
    }

    let (Some(audio_in_project), false) = (s.song_audio().map(Path::to_path_buf), params["skip_audio_setup"] == true) else {
        if s.song_audio().is_none() {
            match crate::cmd::film::assembly::cmd_video_assemble(s, &json!({"name": project_title, "export": false})) {
                Ok(res) => {
                    steps.push(json!({"command": "video.assemble", "status": "empty FilmCraft shell initialized", "fcproj": res["project"]}));
                }
                Err(e) => steps.push(json!({"command": "video.assemble", "status": "deferred", "error": e.to_string()})),
            }
        }
        return steps;
    };

    match crate::cmd::film::assembly::ensure_song_film(s, &audio_in_project) {
        Ok(fcproj) => {
            steps.push(json!({"command": "film.ensure_song_film", "status": "audio placed on FilmCraft timeline", "fcproj": fcproj}));
        }
        Err(e) => steps.push(json!({"command": "film.ensure_song_film", "status": "deferred", "error": e.to_string()})),
    }

    let local_captions = params
        .get("captions")
        .and_then(Value::as_str)
        .filter(|v| !v.trim().is_empty())
        .and_then(|raw| copy_supporting_input(s, project_dir, raw, "film", overwrite).ok())
        .inspect(|p| copied.push(p.clone()));

    if let Some(raw_transcript) = params.get("transcript").and_then(Value::as_str).filter(|v| !v.trim().is_empty()) {
        let transcript_src = match copy_supporting_input(s, project_dir, raw_transcript, "audio", overwrite) {
            Ok(local) => {
                copied.push(local.clone());
                local.to_string_lossy().into_owned()
            }
            Err(_) => raw_transcript.to_owned(),
        };
        let mut import_req = json!({"path": transcript_src, "replace": true});
        if let Some(ref cap_path) = local_captions {
            import_req["captions"] = json!(cap_path);
        } else if !params["captions"].is_null() {
            import_req["captions"] = params["captions"].clone();
        }
        match crate::cmd::project::workshop::cmd_workshop_import_transcript(s, &import_req) {
            Ok(_) => steps.push(json!({"command": "workshop.import_transcript", "status": "transcript + FilmCraft C1 ready"})),
            Err(e) => steps.push(json!({"command": "workshop.import_transcript", "status": "failed", "error": e.to_string()})),
        }
    } else if params.get("process_audio").and_then(Value::as_bool).unwrap_or(true) && params["prepare_only"] != true {
        let model = params.get("model").and_then(Value::as_str).unwrap_or("large-v3");
        let analyze_req = json!({"analyze_beats": true, "model": model, "resolve_video_cli": params["resolve_video_cli"]});
        match crate::cmd::project::workshop::cmd_audio_analyze(s, &analyze_req) {
            Ok(_) => steps.push(json!({"command": "audio.analyze", "status": "transcribed audio and synced FilmCraft C1"})),
            Err(e) => steps.push(json!({"command": "audio.analyze", "status": "failed", "error": e.to_string()})),
        }
    }
    steps
}

pub fn cmd_project_new(s: &mut Session, params: &Value) -> Result<Value> {
    let _ = crate::workspace::ensure_default_app_folders();
    let mode = project_mode(params)?;
    let raw_target = new_project_file_path(s, params, mode.as_deref())?;
    let overwrite = params.get("overwrite").and_then(Value::as_bool).unwrap_or(false);
    if raw_target.exists() && !overwrite {
        return Err(EngineError::InvalidParams(format!("project already exists: {} (pass \"overwrite\": true to replace it)", raw_target.display())));
    }
    let is_scratch = params["allow_empty_audio"] == true;
    let source_audio = match params.get("audio").and_then(Value::as_str).filter(|v| !v.trim().is_empty()) {
        Some(raw) => Some(crate::workspace::song_audio_path(s, raw)?),
        None => None,
    };

    let raw_dir = raw_target.parent().ok_or_else(|| EngineError::InvalidParams("project path must reside inside a directory".into()))?;
    if overwrite && raw_dir.is_dir() {
        clear_stale_project_artifacts(raw_dir);
    }
    let directories = if is_scratch {
        std::fs::create_dir_all(raw_dir)?;
        Vec::new()
    } else {
        scaffold_project_dirs(raw_dir)?
    };
    let project_dir = std::fs::canonicalize(raw_dir).unwrap_or_else(|_| raw_dir.to_path_buf());
    let path = project_dir.join(raw_target.file_name().ok_or_else(|| EngineError::InvalidParams("missing `.magic` file name".into()))?);
    if path.exists() && !overwrite {
        return Err(EngineError::InvalidParams(format!("project already exists: {} (pass \"overwrite\": true to replace it)", path.display())));
    }

    let mut copied_files = Vec::new();
    let local_audio = match source_audio {
        Some(ref src) => {
            let (copied_audio, _) = copy_audio_into_project(&project_dir, src, overwrite)?;
            copied_files.push(copied_audio.clone());
            Some(copied_audio)
        }
        None => None,
    };

    let mut project = Project::new_for_path(&path, mode);
    project.workshop.audio = local_audio;
    if let Some(title) = params.get("title").or_else(|| params.get("name")).and_then(Value::as_str).map(str::trim).filter(|t| !t.is_empty()) {
        project.song.title = title.to_owned();
    }
    s.reset_for_opened_project(path.clone(), project);
    create_initial_project_file(s, overwrite)?;

    let mut steps = Vec::new();
    if !is_scratch {
        steps.extend(bootstrap_song_workshop(s, &path, &project_dir, params, overwrite, &mut copied_files));
        write_project_file(s)?;
        s.mark_saved();
    }

    let warnings = recent_warning(s, &path, !is_scratch && params["record_recent"] != false);
    let workshop = crate::cmd::project::workshop::workshop_status(s)?;
    let storyboard = crate::cmd::storyboard::cmd_storyboard_status(s, &json!({})).unwrap_or(Value::Null);
    Ok(json!({
        "ok": true,
        "project": path,
        "project_dir": project_dir,
        "directories": directories,
        "copied_files": copied_files,
        "mode": s.project_mode(),
        "dirty": s.is_dirty(),
        "warnings": warnings,
        "steps": steps,
        "assets": s.project_assets(),
        "workshop": workshop,
        "storyboard": storyboard,
    }))
}
