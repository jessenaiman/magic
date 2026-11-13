//! `project.new` / `file.new`: creates a self-contained `<project-slug>/` directory owning copies
//! of all audio, assets, `storyboard.designcraft`, `effects/`, `film/`, and `exports/` (plus a
//! single `original-project.zip` strictly when `migrate_from` is supplied).

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::{cmd_project_import_file, create_initial_project_file, project_mode, recent_warning, reset_for_project, validate_project_name, write_project_file};
use crate::{EngineError, Result, Session};

const PROJECT_SCAFFOLD_DIRS: &[&str] = &["audio", "assets", "effects", "film", "exports"];

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
        return if parent_matches && !parent_has_other_magic_file(parent, &fname) { Ok(parent.join(fname)) } else { Ok(parent.join(&slug).join(fname)) };
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

    let sb = path.with_file_name("storyboard.designcraft");
    if !sb.is_file() || overwrite {
        let mut sb_req = json!({"path": sb, "overwrite": overwrite, "title": project_title});
        if let Some(wiz) = params.get("wizard") {
            sb_req["wizard"] = wiz.clone();
        }
        match crate::cmd::storyboard::cmd_storyboard_new(s, &sb_req) {
            Ok(_) => steps.push(json!({"command": "storyboard.new", "status": "storyboard.designcraft copied from template"})),
            Err(e) => steps.push(json!({"command": "storyboard.new", "status": "failed", "error": e.to_string()})),
        }
    }

    let (Some(audio_in_project), false) = (s.song_audio.clone(), params["skip_audio_setup"] == true) else {
        return steps;
    };

    match crate::cmd::film::assembly::ensure_song_film(s, &audio_in_project) {
        Ok(fcproj) => {
            let _ = write_project_file(s);
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
            Ok(_) => steps.push(json!({"command": "workshop.import_transcript", "status": "timing-reference.json + FilmCraft C1 ready"})),
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
    let _ = crate::workspace::ensure_default_app_folders(crate::cmd::storyboard::MASTER_DESIGNCRAFT);
    let mode = project_mode(params)?;
    let raw_target = new_project_file_path(s, params, mode.as_deref())?;
    let overwrite = params.get("overwrite").and_then(Value::as_bool).unwrap_or(false);
    if raw_target.exists() && !overwrite {
        return Err(EngineError::InvalidParams(format!("project already exists: {} (pass \"overwrite\": true to replace it)", raw_target.display())));
    }
    let is_scratch = params["allow_empty_audio"] == true;
    let source_audio = match params.get("audio").and_then(Value::as_str).filter(|v| !v.trim().is_empty()) {
        Some(raw) => Some(crate::workspace::song_audio_path(s, raw)?),
        None if mode.is_none() && !is_scratch => {
            return Err(EngineError::InvalidParams("project.new requires `audio` (<song file>) or mode=\"animation\"".into()));
        }
        None => None,
    };

    let raw_dir = raw_target.parent().ok_or_else(|| EngineError::InvalidParams("project path must reside inside a directory".into()))?;
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

    reset_for_project(s, path.clone(), Vec::new());
    s.song_audio = local_audio;
    s.project_mode = mode;
    if let Some(title) = params.get("title").or_else(|| params.get("name")).and_then(Value::as_str).map(str::trim).filter(|t| !t.is_empty()) {
        s.song.title = title.to_owned();
    }
    create_initial_project_file(s, overwrite)?;

    let mut steps = Vec::new();
    if let Some(migrate_raw) = params.get("migrate_from").and_then(Value::as_str).filter(|v| !v.trim().is_empty()) {
        let legacy_dir = crate::workspace::resolve_workspace_path(s, migrate_raw);
        let zip_dest = project_dir.join("original-project.zip");
        let mig = super::archive::archive_directory_to_zip(&legacy_dir, &zip_dest)?;
        copied_files.push(zip_dest);
        steps.push(json!({"command": "project.migrate", "status": "archived original-project.zip", "archive": mig["archive"]}));
    }

    if !is_scratch {
        if s.project_mode.is_some() {
            copy_initial_collections(s, params, overwrite, &mut copied_files, &mut steps);
        } else {
            steps.extend(bootstrap_song_workshop(s, &path, &project_dir, params, overwrite, &mut copied_files));
        }
        write_project_file(s)?;
    }

    let warnings = recent_warning(s, &path, !is_scratch && params["record_recent"] != false);
    let workshop = crate::cmd::project::workshop::workshop_status(s)?;
    let storyboard = crate::cmd::storyboard::cmd_storyboard_check(s, &json!({})).unwrap_or(Value::Null);
    Ok(json!({
        "ok": true,
        "project": path,
        "project_dir": project_dir,
        "directories": directories,
        "copied_files": copied_files,
        "mode": s.project_mode,
        "warnings": warnings,
        "steps": steps,
        "assets": s.project_assets,
        "workshop": workshop,
        "storyboard": storyboard,
    }))
}

#[cfg(test)]
mod tests {
    use image::ImageEncoder;
    use serde_json::json;
    use std::path::PathBuf;

    use super::cmd_project_new;
    use crate::Session;
    use crate::cmd::project::{cmd_project_autosave, read_project_document, write_project_file};

    fn temp_sandbox(label: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("magic_project_new_{label}_{}_{nanos}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn minimal_wav_bytes() -> Vec<u8> {
        let mut wav = Vec::with_capacity(48);
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&40_u32.to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16_u32.to_le_bytes());
        wav.extend_from_slice(&1_u16.to_le_bytes());
        wav.extend_from_slice(&1_u16.to_le_bytes());
        wav.extend_from_slice(&8000_u32.to_le_bytes());
        wav.extend_from_slice(&16000_u32.to_le_bytes());
        wav.extend_from_slice(&2_u16.to_le_bytes());
        wav.extend_from_slice(&16_u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&4_u32.to_le_bytes());
        wav.extend_from_slice(&[0_u8; 4]);
        wav
    }

    #[test]
    fn project_new_creates_self_contained_dir_and_migration_zip_only_when_requested() {
        let sandbox = temp_sandbox("self_contained");
        let external_src = sandbox.join("external_inputs");
        let legacy_src = sandbox.join("legacy_proj");
        std::fs::create_dir_all(&external_src).unwrap();
        std::fs::create_dir_all(&legacy_src).unwrap();
        crate::doc_state::write_file(legacy_src.join("notes.txt"), b"legacy").unwrap();

        let ext_wav = external_src.join("song-demo.wav");
        crate::doc_state::write_file(&ext_wav, minimal_wav_bytes()).unwrap();
        let ext_png = external_src.join("cover.png");
        let mut png_bytes = Vec::new();
        image::codecs::png::PngEncoder::new(&mut png_bytes).write_image(&[255_u8, 128, 64, 255], 1, 1, image::ColorType::Rgba8.into()).unwrap();
        crate::doc_state::write_file(&ext_png, &png_bytes).unwrap();

        let mut session = Session::default();
        let res = cmd_project_new(
            &mut session,
            &json!({
                "root": sandbox.join("Magic"),
                "name": "Hey Ho",
                "audio": ext_wav,
                "assets": [ext_png],
                "migrate_from": legacy_src,
                "skip_audio_setup": true,
            }),
        )
        .unwrap();

        let created_magic = PathBuf::from(res["project"].as_str().unwrap());
        let created_dir = created_magic.parent().unwrap();
        assert_eq!(created_dir.file_name().unwrap().to_str().unwrap(), "hey-ho");
        for sub in ["audio", "assets", "effects", "film", "exports"] {
            assert!(created_dir.join(sub).is_dir(), "expected {sub}/ inside self-contained project");
        }
        assert!(created_dir.join("storyboard.designcraft").is_file());
        assert!(created_dir.join("original-project.zip").is_file());

        let saved_doc = read_project_document(&created_magic).unwrap();
        assert_eq!(saved_doc["workshop"]["audio"].as_str().unwrap(), "audio/song-demo.wav");
        let asset_rel = saved_doc["assets"][0]["path"].as_str().unwrap();
        assert!(asset_rel.starts_with("assets/"));
        assert!(created_dir.join(asset_rel).is_file());

        let auto = cmd_project_autosave(&mut session, &json!({})).unwrap();
        let journal = PathBuf::from(auto["journal"].as_str().unwrap());
        assert!(journal.is_file());
        write_project_file(&session).unwrap();
        assert!(created_dir.join(".history/Hey Ho.magic.bak.1").is_file());

        let _ = std::fs::remove_dir_all(&sandbox);
    }
}
