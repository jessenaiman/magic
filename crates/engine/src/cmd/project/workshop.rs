//! The song workshop: stage status, timing reference check, studio intent, and unified project search.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::workspace::{has_extension, workspace_root};
use crate::{EngineError, Result, Session};

pub use crate::cmd::project::intake::{
    cmd_workshop_add, cmd_workshop_approve_draft, cmd_workshop_files, cmd_workshop_remove, cmd_workshop_set_audio, cmd_workshop_set_target,
    cmd_workshop_split_panels,
};
pub use crate::cmd::project::transcript::{cmd_audio_analyze, cmd_workshop_import_transcript};
pub(crate) use crate::cmd::project::transcript::{transcript_document, transcript_problems};
pub use crate::cmd::project::trunk::{cmd_project_import_file, cmd_project_trunk};

pub(crate) const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp"];
pub(crate) const STORYBOARD_DIR: &str = "storyboard";
<<<<<<< HEAD
pub(crate) const STORYBOARD_PDF: &str = "storyboard.pdf";
pub(crate) const ASSETS_DIR: &str = "assets";
pub const VIDEO_TARGETS: &[&str] = &["filmcraft"];
pub(crate) const SEQUENCE_TICKS_PER_SECOND: f64 = 254_016_000_000.0;
=======
pub(crate) const STORYBOARD_DESIGNCRAFT: &str = "storyboard.designcraft";
pub(crate) const ASSETS_DIR: &str = "assets";
pub const VIDEO_TARGETS: &[&str] = &["filmcraft"];
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)

pub(crate) fn song_seconds(dir: &Path) -> Option<f64> {
    if let Some(duration) = std::fs::read(dir.join("transcript-provenance.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .and_then(|v| v["recording_duration_sec"].as_f64())
        .filter(|v| v.is_finite() && *v > 0.0)
    {
        return Some(duration);
    }
    let fcproj = video_project_path(dir)?;
    let seq = crate::cmd::film::execute(&fcproj, "sequence.inspect", json!({})).ok()?;
<<<<<<< HEAD
    seq.get("duration").and_then(Value::as_f64).map(|t| t / SEQUENCE_TICKS_PER_SECOND)
=======
    seq.get("duration").and_then(Value::as_f64).map(|t| t / (crate::time::TICKS_PER_SECOND as f64))
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
}

pub(crate) fn song_ticks(dir: &Path) -> Option<crate::time::Tick> {
    song_seconds(dir).map(crate::time::Tick::from_seconds_f64)
}

#[derive(Clone, Default)]
pub struct Studio {
    pub name: Option<String>,
    pub intent: Option<String>,
    pub source: Option<PathBuf>,
}

pub fn studio(dir: &Path) -> Studio {
    let candidates = [Some(dir.join("intent.md")), dir.parent().map(|p| p.join("STUDIO.md"))];
    for path in candidates.into_iter().flatten() {
        if let Ok(text) = std::fs::read_to_string(&path) {
            let mut name = None;
            let mut body = Vec::new();
            for line in text.lines() {
                match line.strip_prefix("# ") {
                    Some(h) if name.is_none() => name = Some(h.trim().to_string()),
                    _ => body.push(line),
                }
            }
            let intent = body.join("\n").trim().to_string();
            return Studio { name, intent: (!intent.is_empty()).then_some(intent), source: Some(path) };
        }
    }
    Studio { name: None, intent: None, source: None }
}

fn skill_hints(s: &Session, dir: &Path, names: &[&str]) -> String {
    let roots = [Some(workspace_root(s).join(".agents/skills")), dir.parent().and_then(Path::parent).map(|p| p.join(".agents/skills"))];
    let found: Vec<String> = names
        .iter()
        .filter_map(|n| roots.iter().flatten().map(|r| r.join(n).join("SKILL.md")).find(|p| p.is_file()))
        .map(|p| p.display().to_string())
        .collect();
    if found.is_empty() { String::new() } else { format!("\nSkills: {}", found.join(", ")) }
}

fn stage_brief(s: &Session, dir: &Path, stage: &str) -> String {
    format!("{}\n{}", brief_header(dir), stage_body(s, dir, stage))
}

pub(crate) fn brief_header(dir: &Path) -> String {
    let project = dir.display();
    let studio = studio(dir);
    let intent = match (&studio.intent, &studio.source) {
        (Some(intent), Some(src)) => format!("{intent}\n(from {})\n", src.display()),
        _ => format!("No studio intent yet: write it in {project}/intent.md or STUDIO.md beside the projects.\n"),
    };
    format!("{intent}Project folder: {project}\nWorkshop state: magic-cli exec workshop.status (run project.open path=<the .magic> first)\n")
}

pub(crate) fn stage_body(s: &Session, dir: &Path, stage: &str) -> String {
    let target = s.video_target.as_str();
    match stage {
        "song" => "Stage 1 — Song. Link the song audio (`magic-cli new --audio <song>`).".to_string(),
        "transcript" => format!(
            "Stage 2 — Transcription & Timing Reference. Generates `timing-reference.json` + `transcript.srt` and syncs the FilmCraft lyric track.\n  magic-cli exec audio.analyze\nVideo target: {target}.{}",
            skill_hints(s, dir, &["timing-skill"])
        ),
        "storyboard" => {
            "Stage 3 & 4 — Storyboard Title Sections (`A-Cover`, `B-World`, `C-Character`, `D-Motion`), then `magic-cli storyboard bulk_generate`.".to_string()
        }
        _ => format!(
            "Stage 5 — Video Production: Kinetic Lyrics (`words.*`), Song-Action Motion (`motion.*`), Character Animation (`character.*`), and FilmCraft (`video.*`).{}",
            skill_hints(s, dir, &["author-motion"])
        ),
    }
}

pub(crate) fn files_in(dir: &Path, only: Option<&[&str]>) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && !p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with('.')))
        .filter(|p| only.is_none_or(|exts| has_extension(p, exts)))
        .collect();
    files.sort();
    files
}

pub(crate) fn transcript_files(dir: &Path) -> Vec<PathBuf> {
    let mut list: Vec<PathBuf> = files_in(dir, Some(&["json"]))
        .into_iter()
        .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n == "transcript.json" || n.ends_with(".transcript.json")))
        .collect();
    list.sort();
    list
}

pub(crate) fn panel_files(dir: &Path) -> Vec<PathBuf> {
    files_in(&dir.join(STORYBOARD_DIR), Some(IMAGE_EXTENSIONS))
        .into_iter()
        .filter(|p| p.file_stem().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("panel-")))
        .collect()
}

pub(crate) fn video_project_path(dir: &Path) -> Option<PathBuf> {
    files_in(&dir.join("film"), Some(&["fcproj"])).into_iter().next().or_else(|| files_in(dir, Some(&["fcproj"])).into_iter().next())
}

pub(crate) fn workshop_asset_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut push_unique = |paths: Vec<PathBuf>| {
        for p in paths {
            if !out.contains(&p) {
                out.push(p);
            }
        }
    };
    push_unique(files_in(&dir.join(ASSETS_DIR), Some(IMAGE_EXTENSIONS)));
    push_unique(files_in(&dir.join(ASSETS_DIR).join("images"), Some(IMAGE_EXTENSIONS)));
    push_unique(files_in(&dir.join(ASSETS_DIR).join("characters/approved-crops"), Some(IMAGE_EXTENSIONS)));
    if let Ok(chars) = std::fs::read_dir(dir.join(ASSETS_DIR).join("characters")) {
        let mut char_dirs: Vec<PathBuf> = chars.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
        char_dirs.sort();
        for cdir in char_dirs {
            push_unique(files_in(&cdir.join("parts"), Some(IMAGE_EXTENSIONS)));
        }
    }
    if let Ok(bgs) = std::fs::read_dir(dir.join(ASSETS_DIR).join("backgrounds")) {
        let mut bg_dirs: Vec<PathBuf> = bgs.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
        bg_dirs.sort();
        for bdir in bg_dirs {
            push_unique(files_in(&bdir, Some(IMAGE_EXTENSIONS)));
        }
    }
    push_unique(files_in(&dir.join("layer-sources"), Some(IMAGE_EXTENSIONS)));
    out
}

fn file_chip(path: &Path) -> Value {
    json!({"name": path.file_name().and_then(|n| n.to_str()), "path": path, "exists": path.exists()})
}

pub(crate) fn project_dir(s: &Session) -> Result<PathBuf> {
    s.project_path
        .as_ref()
        .and_then(|p| p.parent())
        .map(Path::to_path_buf)
        .ok_or_else(|| EngineError::InvalidParams("no project is open; use project.new or project.open".into()))
}

fn workshop_transcript_summary(dir: &Path, video_target: &str, transcripts: &[PathBuf]) -> (Value, Vec<String>, bool, String, Vec<Value>) {
    let source = transcripts
        .iter()
        .find(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.ends_with(".transcript.json")))
        .or_else(|| transcripts.first())
        .cloned();
    let doc = source.as_ref().and_then(|p| transcript_document(p).ok()).unwrap_or(Value::Null);
    let errors = transcript_problems(&doc);
    let valid = source.is_some() && errors.is_empty();
    let why = if source.is_none() {
        "Transcribe the linked song or import timed lyrics to produce timing-reference.json.".to_string()
    } else if !valid {
        format!("Transcript needs repair: {}.", errors.join("; "))
    } else {
        "Transcript and timing-reference.json ready.".to_string()
    };
    let mut outputs: Vec<Value> = transcripts.iter().map(|p| file_chip(p)).collect();
    for name in ["timing-reference.json", "transcript.srt"] {
        let f = dir.join(name);
        if f.is_file() {
            outputs.push(file_chip(&f));
        }
    }
    for name in ["transcript.json", "transcript.srt", "audio-analysis.json"] {
        let file = dir.join(video_target).join(name);
        if file.is_file() {
            outputs.push(file_chip(&file));
        }
    }
    (doc, errors, valid, why, outputs)
}

fn workshop_animated_assets(s: &Session) -> Vec<Value> {
    s.project_assets.iter().map(|a| json!({"id": a.id, "name": a.name, "kind": a.kind, "path": a.path})).collect()
}

pub(crate) fn workshop_status(s: &mut Session) -> Result<Value> {
    let dir = project_dir(s)?;
    let target = s.video_target.clone();
    let audio_ok = s.song_audio.as_ref().is_some_and(|p| p.is_file());
    let liner = files_in(&dir.join("liner-notes"), None);
    let transcripts = transcript_files(&dir);
    let assets = workshop_asset_files(&dir);
    let timing_ref_path = dir.join("timing-reference.json");
    let timing_ref_exists = timing_ref_path.is_file();
    let sb_check = crate::cmd::storyboard::cmd_storyboard_check(s, &json!({}))
        .unwrap_or_else(|_| json!({"production_ready": false, "title_sections_complete": false, "light": "red"}));
    let sb_titles_done = sb_check["title_sections_complete"] == true;
    let sb_ready = sb_check["production_ready"] == true;

    let song_why = match (audio_ok, liner.is_empty()) {
        (false, _) => "Choose the song's audio.",
        (true, true) => "Song audio linked and ready.",
        (true, false) => "Song and liner notes are in place.",
    };
    let targets: Vec<Value> = VIDEO_TARGETS.iter().map(|t| json!({"target": t, "selected": *t == target})).collect();
    let (supplied_transcript, transcript_errors, transcript_valid, transcript_why, transcript_outputs) =
        workshop_transcript_summary(&dir, &target, &transcripts);
    let animated = workshop_animated_assets(s);
    let mut stages = vec![
        json!({"id": "song", "title": "Song", "done": audio_ok, "why": song_why,
            "inputs": s.song_audio.iter().map(|p| file_chip(p)).chain(liner.iter().map(|p| file_chip(p))).collect::<Vec<_>>(),
            "outputs": Vec::<Value>::new(), "brief": stage_brief(s, &dir, "song")}),
        json!({"id": "transcript", "title": "Transcript & Timing Ref", "done": transcript_valid, "why": transcript_why,
            "inputs": s.song_audio.iter().map(|p| file_chip(p)).collect::<Vec<_>>(), "outputs": transcript_outputs,
            "document": supplied_transcript, "problems": transcript_errors, "brief": stage_brief(s, &dir, "transcript")}),
        json!({"id": "storyboard", "title": "Storyboard Sections", "done": sb_ready, "why": sb_check["next_step"].as_str().unwrap_or("Complete storyboard sections."),
            "title_sections_complete": sb_titles_done, "light": sb_check["light"],
<<<<<<< HEAD
            "inputs": [file_chip(&dir.join("storyboard.pdf"))],
=======
            "inputs": [file_chip(&dir.join(STORYBOARD_DESIGNCRAFT))],
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
            "outputs": assets.iter().map(|p| file_chip(p)).collect::<Vec<_>>(),
            "brief": stage_brief(s, &dir, "storyboard")}),
        json!({"id": "animate", "title": "Video Production", "done": !animated.is_empty(), "optional": true,
            "why": if !sb_ready { "Complete Storyboard sections first before starting full video production." } else if animated.is_empty() { "Ready for video production." } else { "Production assets ready." },
            "inputs": assets.iter().map(|p| file_chip(p)).collect::<Vec<_>>(), "assets": animated, "brief": stage_brief(s, &dir, "animate")}),
    ];
    for stage in &mut stages {
        stage["state"] = json!(stage_state(stage));
    }
    let setup_ready = audio_ok && transcript_valid;
    let studio_info = studio(&dir);
    let effective_video_project = s.video_project_path.clone().or_else(|| video_project_path(&dir));
    let lyrics_ready = dir.join("lettering/lyrics.ecproj").is_file() || dir.join("design/words.json").is_file();
    let motion_ready = dir.join("motion/actions.ecproj").is_file() || dir.join("motion/actions.json").is_file();

    let next_steps = if !audio_ok || !transcript_valid {
        json!(["magic-cli exec audio.analyze"])
    } else if !sb_titles_done {
        json!(["magic-cli storyboard check", "magic-cli storyboard wizard"])
    } else if !sb_ready {
        json!(["magic-cli storyboard bulk_generate", "magic-cli storyboard check"])
    } else {
        json!(["magic-cli words letter", "magic-cli motion save", "magic-cli video assemble"])
    };

    let mut status = json!({
        "ok": true,
        "project_dir": dir,
        "complete": setup_ready && sb_ready,
        "setup_ready": setup_ready,
        "timing_reference": timing_ref_path,
        "timing_reference_exists": timing_ref_exists,
        "storyboard_titles_ready": sb_titles_done,
        "storyboard_production_ready": sb_ready,
        "lyrics_ready": lyrics_ready,
        "motion_ready": motion_ready,
        "completion_scope": "audio timing + storyboard sections -> video production",
        "video_target": target,
        "targets": targets,
        "film_timeline": crate::cmd::film::timeline_file(s)?,
        "asset_files": assets.iter().map(|p| file_chip(p)).collect::<Vec<_>>(),
        "studio": {"name": studio_info.name, "intent": studio_info.intent, "source": studio_info.source},
        "video_project_path": effective_video_project,
        "video_project_exists": effective_video_project.as_ref().is_some_and(|p| p.exists()),
        "stages": stages,
        "next_steps": next_steps,
    });
    status["prerequisites"] = prerequisites(&status);
    Ok(status)
}

pub fn cmd_query_list(s: &mut Session, params: &Value) -> Result<Value> {
    let f = params.get("filter").or_else(|| params.get("query")).and_then(Value::as_str).map(str::to_ascii_lowercase).unwrap_or_default();
    let list: Vec<Value> = crate::commands::command_specs()
        .iter()
        .filter(|c| {
            f.is_empty() || c.id.to_ascii_lowercase().contains(&f) || c.label.to_ascii_lowercase().contains(&f) || c.params.to_ascii_lowercase().contains(&f)
        })
        .map(|c| {
            let mut description = c.describe();
            let enabled = (c.enabled)(s);
            description["enabled"] = json!(enabled.is_ok());
            if let Err(reason) = enabled {
                description["disabled_reason"] = json!(reason);
            }
            description
        })
        .collect();
    Ok(json!({"count": list.len(), "commands": list}))
}

pub fn cmd_project_search(s: &mut Session, params: &Value) -> Result<Value> {
    let q = params.get("query").or_else(|| params.get("query")).or_else(|| params.get("filter")).and_then(Value::as_str).map(str::trim).unwrap_or_default();
    let q_lower = q.to_ascii_lowercase();
    let commands: Vec<Value> = crate::commands::command_specs()
        .iter()
        .filter(|c| {
            q_lower.is_empty()
                || c.id.to_ascii_lowercase().contains(&q_lower)
                || c.label.to_ascii_lowercase().contains(&q_lower)
                || c.params.to_ascii_lowercase().contains(&q_lower)
        })
        .map(|c| json!({"id": c.id, "label": c.label, "params": c.params}))
        .collect();
    let subjects: Vec<Value> = s
        .project_assets
        .iter()
        .filter(|a| {
            q_lower.is_empty()
                || a.id.to_ascii_lowercase().contains(&q_lower)
                || a.name.to_ascii_lowercase().contains(&q_lower)
                || a.path.as_ref().is_some_and(|p| p.display().to_string().to_ascii_lowercase().contains(&q_lower))
        })
        .map(|a| json!({"id": a.id, "name": a.name, "kind": a.kind, "path": a.path}))
        .collect();
    let files = cmd_workshop_files(s, &json!({"filter": q})).ok().and_then(|v| v.get("files").cloned()).unwrap_or_else(|| json!([]));
    Ok(json!({
        "ok": true,
        "query": q,
        "project": s.project_path,
        "active_asset": s.active_asset,
        "subjects": subjects,
        "files": files,
        "commands": commands,
    }))
}

pub fn cmd_workshop_status(s: &mut Session, _: &Value) -> Result<Value> {
    workshop_status(s)
}

fn prerequisites(status: &Value) -> Value {
    let stage = |id: &str| status["stages"].as_array().and_then(|stages| stages.iter().find(|s| s["id"] == id));
    let p = |id: &str| {
        stage(id)
            .map(|s| json!({"state": s["state"], "detail": s["why"].as_str().unwrap_or("").to_string()}))
            .unwrap_or_else(|| json!({"state": "not-started", "detail": "not started"}))
    };
    let timeline = &status["film_timeline"];
    let studio = match timeline["exists"].as_bool() {
        Some(true) => ("ready", timeline["path"].as_str().unwrap_or("film timeline linked").to_string()),
        _ => ("needs-work", "no film timeline linked".to_string()),
    };
    let lyrics = if status["lyrics_ready"] == json!(true) { ("ready", "kinetic lyrics authored") } else { ("not-started", "design kinetic teaching lyrics") };
    let motion = if status["motion_ready"] == json!(true) { ("ready", "song action drawings authored") } else { ("not-started", "draw song action cues") };
    json!({
        "song": p("song"),
        "transcript": p("transcript"),
        "storyboard": p("storyboard"),
        "lyrics": {"state": lyrics.0, "detail": lyrics.1},
        "motion": {"state": motion.0, "detail": motion.1},
        "animate": p("animate"),
        "studio": {"state": studio.0, "detail": studio.1},
    })
}

fn stage_state(s: &Value) -> &'static str {
    if s["done"] == json!(true) {
        "ready"
    } else if s["problems"].as_array().is_some_and(|problems| !problems.is_empty()) {
        "failed"
    } else {
        "needs-work"
    }
}
