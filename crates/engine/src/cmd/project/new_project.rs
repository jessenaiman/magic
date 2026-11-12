//! `project.new` / `file.new` dedicated directory creation, canonical Craft subfolder scaffolding,
//! exclusive `.magic` creation (`craft_atomic::write_new`), and atomic copying of all referenced
//! files into `<project_dir>/`.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::{
    cmd_project_import_file, create_initial_project_file, project_mode, project_slug, recent_warning,
    reset_for_project, validate_project_name, write_project_file,
};
use crate::{EngineError, Result, Session};

const SONG_SCAFFOLD_DIRS: &[&str] = &[
    "audio",
    "assets/characters",
    "assets/backgrounds",
    "assets/images",
    "assets/vectors",
    "assets/3d",
    "assets/.previews",
    "storyboard",
    "film",
    "animations",
    "lettering",
    "motion",
    "liner-notes",
    "exports",
    "magic/originals",
];

const ANIMATION_SCAFFOLD_DIRS: &[&str] = &["assets/parts", "animations", "exports"];

fn parent_has_other_magic_file(parent: &Path, target_file_name: &str) -> bool {
    let Ok(entries) = std::fs::read_dir(parent) else {
        return false;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_file()
            && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("magic"))
            && p.file_name().and_then(|n| n.to_str()) != Some(target_file_name)
        {
            return true;
        }
    }
    false
}

/// Resolve the target `.magic` path for `project.new` / `file.new`, guaranteeing that every new
/// project lives inside its own dedicated `<slug>/` directory.
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
        let has_magic_ext = resolved.extension().is_some_and(|e| e.eq_ignore_ascii_case("magic"));
        let stem = explicit_name
            .or_else(|| resolved.file_stem().and_then(|st| st.to_str()))
            .ok_or_else(|| EngineError::InvalidParams("project path must include a project name".into()))?;
        let slug = validate_project_name(stem)?;
        if !has_magic_ext {
            let dir_matches = resolved
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.eq_ignore_ascii_case(&slug) || n.eq_ignore_ascii_case(stem));
            let proj_dir = if dir_matches { resolved } else { resolved.join(&slug) };
            return Ok(proj_dir.join(format!("{stem}.magic")));
        }
        let fname = format!("{stem}.magic");
        let parent = resolved.parent().unwrap_or(Path::new("."));
        let parent_matches_project = parent
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.eq_ignore_ascii_case(&slug) || n.eq_ignore_ascii_case(stem));
        if parent_matches_project && !parent_has_other_magic_file(parent, &fname) {
            return Ok(parent.join(fname));
        }
        return Ok(parent.join(&slug).join(fname));
    }

    let name = explicit_name.ok_or_else(|| {
        EngineError::InvalidParams("project.new requires `name` (optionally with `root`), or `path`".into())
    })?;
    let slug = validate_project_name(name)?;
    let base_root = match text("root") {
        Some(raw_root) => {
            let resolved = crate::workspace::resolve_workspace_path(s, raw_root);
            let is_magic_ws = resolved.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.eq_ignore_ascii_case("magic"))
                || resolved.join("workspace.json").is_file();
            if is_magic_ws {
                let layout = super::workspace_layout::ensure_magic_workspace(&resolved)?;
                if mode == Some("animation") { layout.animations_dir } else { layout.projects_dir }
            } else {
                resolved
            }
        }
        None => crate::project_settings::default_root_for_mode(s, mode)?,
    };
    Ok(base_root.join(slug).join(format!("{name}.magic")))
}

fn scaffold_project_dirs(project_dir: &Path, mode: Option<&str>) -> Result<Vec<String>> {
    std::fs::create_dir_all(project_dir)?;
    let subdirs = if mode == Some("animation") { ANIMATION_SCAFFOLD_DIRS } else { SONG_SCAFFOLD_DIRS };
    let mut created = Vec::with_capacity(subdirs.len());
    for rel in subdirs {
        std::fs::create_dir_all(project_dir.join(rel))?;
        created.push((*rel).to_owned());
    }
    super::workspace_layout::ensure_sibling_magic_folders(project_dir);
    let _ = magic_util::media::sweep_orphan_craft_tmp(project_dir);
    Ok(created)
}

pub(crate) fn copy_audio_into_project(project_dir: &Path, source_audio: &Path, overwrite: bool) -> Result<(PathBuf, bool)> {
    let audio_dir = project_dir.join("audio");
    std::fs::create_dir_all(&audio_dir)?;
    let file_name = source_audio
        .file_name()
        .ok_or_else(|| EngineError::InvalidParams(format!("invalid audio file name: {}", source_audio.display())))?;
    let dest = audio_dir.join(file_name);
    let was_external = source_audio.canonicalize().ok() != dest.canonicalize().ok();
    let bytes = crate::doc_state::atomic_copy_file(source_audio, &dest, overwrite)?;
    if was_external {
        let unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let prov_path = audio_dir.join("provenance.json");
        let mut doc: Value = std::fs::read_to_string(&prov_path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_else(|| json!({"files": []}));
        if !doc["files"].is_array() {
            doc["files"] = json!([]);
        }
        if let Some(arr) = doc["files"].as_array_mut() {
            arr.push(json!({
                "file": file_name.to_string_lossy(),
                "source": source_audio,
                "bytes": bytes,
                "copied_unix": unix,
            }));
        }
        let _ = crate::doc_state::write_json(&prov_path, &doc);
    }
    let canon_dest = dest.canonicalize().unwrap_or(dest);
    Ok((canon_dest, was_external))
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
    let fname = src
        .file_name()
        .ok_or_else(|| EngineError::InvalidParams(format!("invalid file name: {}", src.display())))?;
    let dest = target_dir.join(fname);
    crate::doc_state::atomic_copy_file(&src, &dest, overwrite)?;
    Ok(dest)
}

fn copy_initial_project_collections(
    s: &mut Session,
    params: &Value,
    overwrite: bool,
    copied: &mut Vec<PathBuf>,
    steps: &mut Vec<Value>,
) {
    for (param_key, workshop_kind) in [("assets", "assets"), ("panels", "panels"), ("liner_notes", "liner_notes")] {
        let Some(items) = params.get(param_key).and_then(Value::as_array).filter(|a| !a.is_empty()) else {
            continue;
        };
        match crate::cmd::project::workshop::cmd_workshop_add(s, &json!({"kind": workshop_kind, "paths": items, "overwrite": overwrite})) {
            Ok(res) => {
                for p in res["added"].as_array().into_iter().flatten().filter_map(Value::as_str) {
                    copied.push(PathBuf::from(p));
                }
                steps.push(json!({"command": format!("workshop.add({workshop_kind})"), "status": "complete; copied into project directory"}));
            }
            Err(e) => steps.push(json!({"command": format!("workshop.add({workshop_kind})"), "status": "failed", "error": e.to_string()})),
        }
    }
    if let Some(files) = params.get("files").and_then(Value::as_array).filter(|a| !a.is_empty()) {
        match cmd_project_import_file(s, &json!({"paths": files, "overwrite": overwrite})) {
            Ok(res) => {
                for p in res["imported"].as_array().into_iter().flatten().filter_map(Value::as_str) {
                    copied.push(PathBuf::from(p));
                }
                steps.push(json!({"command": "project.import_file", "status": "complete; copied into project directory"}));
            }
            Err(e) => steps.push(json!({"command": "project.import_file", "status": "failed", "error": e.to_string()})),
        }
    }
}

fn bootstrap_new_project_workshop(
    s: &mut Session,
    path: &Path,
    project_dir: &Path,
    params: &Value,
    overwrite: bool,
    copied: &mut Vec<PathBuf>,
) -> Vec<Value> {
    let mut steps = Vec::new();
    let project_title = params
        .get("title")
        .or_else(|| params.get("name"))
        .and_then(Value::as_str)
        .or_else(|| path.file_stem().and_then(|n| n.to_str()))
        .unwrap_or("Untitled");

    copy_initial_project_collections(s, params, overwrite, copied, &mut steps);

    let sb = path.with_file_name("storyboard.pdf");
    if !sb.is_file() || overwrite {
        let mut sb_req = json!({
            "path": sb,
            "overwrite": overwrite,
            "title": project_title,
        });
        if let Some(wiz) = params.get("wizard") {
            sb_req["wizard"] = wiz.clone();
        }
        match crate::cmd::storyboard::cmd_storyboard_new(s, &sb_req) {
            Ok(_) => steps.push(json!({"command": "storyboard.new", "status": "complete; printable storyboard.pdf initialized from read-only master"})),
            Err(e) => steps.push(json!({"command": "storyboard.new", "status": "failed", "error": e.to_string()})),
        }
    }

    let Some(audio_in_project) = s.song_audio.clone() else {
        return steps;
    };
    if params["skip_audio_setup"] == true {
        return steps;
    }

    match crate::cmd::film::assembly::ensure_song_film(s, &audio_in_project) {
        Ok(fcproj) => {
            let _ = write_project_file(s);
            steps.push(json!({"command": "film.ensure_song_film", "status": "complete; audio placed on FilmCraft timeline", "fcproj": fcproj}));
        }
        Err(e) => {
            steps.push(json!({"command": "film.ensure_song_film", "status": "deferred", "error": e.to_string()}));
        }
    }

    let local_captions = params
        .get("captions")
        .and_then(Value::as_str)
        .filter(|v| !v.trim().is_empty())
        .and_then(|raw| copy_supporting_input(s, project_dir, raw, "liner-notes", overwrite).ok())
        .inspect(|p| copied.push(p.clone()));

    if let Some(raw_transcript) = params.get("transcript").and_then(Value::as_str).filter(|v| !v.trim().is_empty()) {
        let transcript_src = match copy_supporting_input(s, project_dir, raw_transcript, "liner-notes", overwrite) {
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
            Ok(_) => steps.push(json!({
                "command": "workshop.import_transcript",
                "status": "complete; timing-reference.json + synced FilmCraft lyric track ready",
            })),
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
        return Err(EngineError::InvalidParams(format!(
            "project already exists: {} (pass \"overwrite\": true to replace it)",
            path.display()
        )));
    }
    Ok(())
}

fn project_audio_source(s: &Session, params: &Value, require_audio: bool) -> Result<Option<PathBuf>> {
    match params.get("audio").and_then(Value::as_str).filter(|v| !v.trim().is_empty()) {
        Some(raw) => Ok(Some(crate::workspace::song_audio_path(s, raw)?)),
        None if require_audio => Err(EngineError::InvalidParams(
            concat!(
                "project.new requires `audio` (<song file>) so Magic can copy the recording into `<project>/audio/`, ",
                "initialize the FilmCraft timeline, and create `storyboard.pdf` (or pass mode=\"animation\" for a standalone character study)"
            ).into(),
        )),
        None => Ok(None),
    }
}

/// Create a new self-contained `.magic` project directory (`magic-cli new` / `project.new` / `file.new`),
/// copying all input files atomically into `<project_dir>/` via `craft-atomic`.
pub fn cmd_project_new(s: &mut Session, params: &Value) -> Result<Value> {
    let mode = project_mode(params)?;
    let raw_target = new_project_file_path(s, params, mode.as_deref())?;
    let overwrite = params.get("overwrite").and_then(Value::as_bool).unwrap_or(false);
    check_overwrite(&raw_target, overwrite)?;
    let is_isolated_scratch = params["allow_empty_audio"] == true;
    let require_audio = mode.is_none() && !is_isolated_scratch;
    let source_audio = project_audio_source(s, params, require_audio)?;

    let raw_dir = raw_target
        .parent()
        .ok_or_else(|| EngineError::InvalidParams("project path must reside inside a directory".into()))?;
    let directories = if is_isolated_scratch {
        std::fs::create_dir_all(raw_dir)?;
        Vec::new()
    } else {
        scaffold_project_dirs(raw_dir, mode.as_deref())?
    };

    let project_dir = std::fs::canonicalize(raw_dir).unwrap_or_else(|_| raw_dir.to_path_buf());
    let file_name = raw_target
        .file_name()
        .ok_or_else(|| EngineError::InvalidParams("missing `.magic` project file name".into()))?;
    let path = project_dir.join(file_name);
    check_overwrite(&path, overwrite)?;

    let mut copied_files: Vec<PathBuf> = Vec::new();
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
    if let Some(title) = params
        .get("title")
        .or_else(|| params.get("name"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|t| !t.is_empty())
    {
        s.song.title = Some(title.to_owned());
    }

    create_initial_project_file(s, overwrite)?;

    let mut steps = Vec::new();
    if is_isolated_scratch {
        // Internal asset scratch pad needs no storyboard or FilmCraft bootstrap.
    } else if s.project_mode.is_some() {
        copy_initial_project_collections(s, params, overwrite, &mut copied_files, &mut steps);
        write_project_file(s)?;
    } else {
        steps = bootstrap_new_project_workshop(s, &path, &project_dir, params, overwrite, &mut copied_files);
        write_project_file(s)?;
    }

    let _ = project_slug("ok");
    let warnings = recent_warning(s, &path, !is_isolated_scratch && params["record_recent"] != false);
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
        "workflow": [
            "1. Self-contained project folder created with audio and assets atomically copied inside",
            "2. Printable storyboard.pdf initialized from the read-only .designcraft master for visual reference",
            "3. Author Animation (V2), Kinetic Lyrics (V3), and Song-Action Motion (V4) in EffectCraft onto the FilmCraft timeline"
        ],
    }))
}

#[cfg(test)]
mod tests {
    use image::ImageEncoder;

    use std::path::PathBuf;
    use serde_json::json;
    use super::cmd_project_new;
    use crate::Session;
    use crate::cmd::project::write_project_file;
    use crate::cmd::project::{cmd_project_autosave, read_project_document};

    fn temp_sandbox(label: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
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
    fn project_new_creates_dedicated_dir_and_copies_all_files() {
        let sandbox = temp_sandbox("dedicated_copy");
        let external_src = sandbox.join("external_inputs");
        let shared_projects = sandbox.join("projects");
        std::fs::create_dir_all(&external_src).unwrap();
        std::fs::create_dir_all(&shared_projects).unwrap();

        let ext_wav = external_src.join("song-demo.wav");
        crate::doc_state::write_file(&ext_wav, minimal_wav_bytes()).unwrap();
        let ext_png = external_src.join("cover.png");
        let mut png_bytes = Vec::new();
        image::codecs::png::PngEncoder::new(&mut png_bytes)
            .write_image(&[255_u8, 128, 64, 255], 1, 1, image::ColorType::Rgba8.into())
            .unwrap();
        crate::doc_state::write_file(&ext_png, &png_bytes).unwrap();

        let mut session = Session::default();
        assert!(!session.settings.is_persistent(), "default Session must use Location::Memory");

        let res = cmd_project_new(
            &mut session,
            &json!({
                "path": shared_projects.join("Hey Ho.magic"),
                "audio": ext_wav,
                "assets": [ext_png],
                "skip_audio_setup": true,
            }),
        )
        .unwrap();

        let created_magic = PathBuf::from(res["project"].as_str().unwrap());
        let created_dir = created_magic.parent().unwrap();
        assert_eq!(created_dir.file_name().unwrap().to_str().unwrap(), "hey-ho");
        assert!(created_dir.join("audio/song-demo.wav").is_file());
        assert!(created_dir.join("storyboard.pdf").is_file());
        assert!(created_dir.join("film").is_dir());
        assert!(created_dir.join("animations").is_dir());

        let saved_doc = read_project_document(&created_magic).unwrap();
        assert_eq!(saved_doc["workshop"]["audio"].as_str().unwrap(), "audio/song-demo.wav");
        let asset_rel = saved_doc["assets"][0]["path"].as_str().unwrap();
        assert!(asset_rel.starts_with("assets/"), "asset path must be stored inside project: {asset_rel}");
        assert!(created_dir.join(asset_rel).is_file());

        let err = cmd_project_new(
            &mut session,
            &json!({
                "path": created_magic,
                "audio": ext_wav,
                "skip_audio_setup": true,
            }),
        )
        .unwrap_err();
        assert!(err.to_string().contains("already exists"));

        let _ = std::fs::remove_dir_all(&sandbox);
    }

    #[test]
    fn autosave_and_backups_use_private_permissions() {
        let sandbox = temp_sandbox("private_perms");
        let ext_wav = sandbox.join("tune.wav");
        crate::doc_state::write_file(&ext_wav, minimal_wav_bytes()).unwrap();

        let mut session = Session::default();
        let res = cmd_project_new(
            &mut session,
            &json!({
                "root": sandbox.join("projects"),
                "name": "Private Check",
                "audio": ext_wav,
                "skip_audio_setup": true,
            }),
        )
        .unwrap();
        let magic_path = PathBuf::from(res["project"].as_str().unwrap());

        let auto = cmd_project_autosave(&mut session, &json!({})).unwrap();
        let journal = PathBuf::from(auto["journal"].as_str().unwrap());
        assert!(journal.is_file());

        write_project_file(&session).unwrap();
        let bak1 = magic_path.parent().unwrap().join(".history/Private Check.magic.bak.1");
        assert!(bak1.is_file());

        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            assert_eq!(std::fs::metadata(&journal).unwrap().mode() & 0o077, 0);
            assert_eq!(std::fs::metadata(&bak1).unwrap().mode() & 0o077, 0);
        }

        let _ = std::fs::remove_dir_all(&sandbox);
    }

    #[test]
    fn magic_root_creates_projects_assets_animations_and_saves_project_assets_locally() {
        let sandbox = temp_sandbox("magic_workspace_tiers");
        let magic_root = sandbox.join("magic");
        let ext_src = sandbox.join("incoming");
        std::fs::create_dir_all(&ext_src).unwrap();

        let ext_wav = ext_src.join("track.wav");
        crate::doc_state::write_file(&ext_wav, minimal_wav_bytes()).unwrap();
        let ext_png = ext_src.join("hero.png");
        let mut png_bytes = Vec::new();
        image::codecs::png::PngEncoder::new(&mut png_bytes)
            .write_image(&[40_u8, 200, 160, 255], 1, 1, image::ColorType::Rgba8.into())
            .unwrap();
        crate::doc_state::write_file(&ext_png, &png_bytes).unwrap();

        let mut session = Session::default();
        let res = cmd_project_new(
            &mut session,
            &json!({
                "root": magic_root,
                "name": "Moppet Test",
                "audio": ext_wav,
                "skip_audio_setup": true,
            }),
        )
        .unwrap();

        assert!(magic_root.join("projects/moppet-test/Moppet Test.magic").is_file());
        assert!(magic_root.join("assets/catalog.json").is_file());
        assert!(magic_root.join("animations").is_dir());
        assert!(magic_root.join("workspace.json").is_file());
        assert!(magic_root.join("projects/moppet-test/magic/originals").is_dir());

        let saved_asset = crate::cmd::project::assets::cmd_project_save_asset(
            &mut session,
            &json!({
                "path": ext_png,
                "name": "Hero Cutout",
                "character": "barnaby",
                "promote_to_workspace": true,
            }),
        )
        .unwrap();
        let local_path = PathBuf::from(saved_asset["path"].as_str().unwrap());
        assert!(local_path.starts_with(magic_root.canonicalize().unwrap().join("projects/moppet-test/assets/characters/barnaby")));
        assert!(local_path.is_file());
        assert!(saved_asset["workspace_asset"].as_str().is_some());

        let magic_doc = read_project_document(&PathBuf::from(res["project"].as_str().unwrap())).unwrap();
        let rel = magic_doc["assets"][0]["path"].as_str().unwrap();
        assert!(rel.starts_with("assets/characters/barnaby/"), "expected relative asset path under project dir, got {rel}");

        let _ = std::fs::remove_dir_all(&sandbox);
    }
}
