<<<<<<< HEAD
//! The film as `FilmCraft` holds it: `film/<song>.fcproj` (built by `video.assemble`), receiving
//! independent `EffectCraft` track renders (`Character Animation` on V2, `Kinetic Lyrics` on V3,
//! and `Action Motion` on V4), dynamic-linked `.ecproj` compositions (`linked_comp`), cross-app
//! interchange/Lottie/template/caption bridges (`bridge`), Studio finishing (`finishing`),
//! half-size Studio previews, program frames, exports, and live `--control`.

pub mod assembly;
pub mod finishing;
pub mod linked_comp;
pub mod markers;
pub use finishing::cmd_video_finish;
pub use linked_comp::{cmd_video_bake_link, cmd_video_link_effectcraft, cmd_video_list_ec_comps, cmd_video_sync_links};
pub use markers::cmd_video_markers;
=======
//! The film as `FilmCraft` holds it: `film/<song>.fcproj` (built by `video.assemble` and edited
//! live in `05 Studio` via [`fcproj_doc`]), receiving independent `EffectCraft` track renders
//! (`Character Animation` on V2, `Kinetic Lyrics` on V3, and `Action Motion` on V4), dynamic-linked
//! `.ecproj` compositions (`linked_comp`), Studio finishing (`finishing`), real composite frames,
//! pure-Rust `craft-audio` master exports, and optional external `FilmCraft` `--control` sync.

pub mod assembly;
pub mod fcproj_doc;
pub mod fcproj_stage;
pub mod finishing;
pub mod linked_comp;
pub mod markers;
pub mod timeline_interchange;
pub use finishing::cmd_video_finish;
pub use linked_comp::{cmd_video_bake_link, cmd_video_link_effectcraft, cmd_video_list_ec_comps, cmd_video_sync_links};
pub use markers::cmd_video_markers;
pub use timeline_interchange::{
    cmd_timeline_ale, cmd_timeline_detect, cmd_timeline_edl, cmd_timeline_export, cmd_timeline_import, cmd_timeline_inspect, cmd_timeline_timecode,
};
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::time::{Tick, parse_raw_tick, parse_tick};
use crate::{EngineError, Result, Session};

pub const TRACK_CHARACTER_ANIMATION: &str = "Character Animation";
pub const TRACK_KINETIC_LYRICS: &str = "Kinetic Lyrics";
pub const TRACK_ACTION_MOTION: &str = "Action Motion";

fn bad(message: impl Into<String>) -> EngineError {
    EngineError::InvalidParams(message.into())
}

<<<<<<< HEAD
struct Film {
    dir: PathBuf,
    name: String,
    project: PathBuf,
=======
pub(crate) struct Film {
    pub(crate) dir: PathBuf,
    pub(crate) name: String,
    pub(crate) project: PathBuf,
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
}

fn film(s: &Session) -> Result<Film> {
    let dir = crate::workshop::project_dir(s)?;
    let name = s.project_path.as_ref().and_then(|p| p.file_stem()).map_or("Film".into(), |n| n.to_string_lossy().into_owned());
    let project = s
        .video_project_path
        .clone()
        .filter(|p| p.extension().is_some_and(|e| e == "fcproj"))
        .unwrap_or_else(|| dir.join("film").join(format!("{name}.fcproj")));
    Ok(Film { dir, name, project })
}

pub(crate) fn link(s: &mut Session, project: &Path) -> Result<()> {
    if s.video_project_path.as_deref() == Some(project) {
        return Ok(());
    }
    s.video_project_path = Some(project.to_path_buf());
    crate::project::write_project_file(s)
}

impl Film {
<<<<<<< HEAD
    fn require(&self) -> Result<&Path> {
        if self.project.is_file() { Ok(&self.project) } else { Err(bad("No film timeline yet; run video.assemble first")) }
    }
    fn preview(&self) -> PathBuf {
        self.dir.join("film").join(format!("{}-preview.mp4", self.name))
    }
=======
    pub(crate) fn ensure_on_disk(&self, s: &Session) -> Result<&Path> {
        if !self.project.is_file() {
            if let Some(parent) = self.project.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut doc = fcproj_doc::load_fcproj(&self.project);
            doc["name"] = json!(self.name);
            doc["sequence"]["name"] = json!(self.name);
            fcproj_doc::hydrate_live_stage_tracks(s, &self.dir, &mut doc);
            fcproj_doc::save_fcproj(&self.project, &mut doc)?;
        }
        Ok(&self.project)
    }

    fn require(&self) -> Result<&Path> {
        if self.project.is_file() { Ok(&self.project) } else { Err(bad("No film timeline yet; run video.assemble first")) }
    }

    fn preview(&self) -> PathBuf {
        self.dir.join("film").join(format!("{}-preview.mp4", self.name))
    }

>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    fn export(&self) -> PathBuf {
        self.dir.join("exports").join(format!("{}.mp4", self.name))
    }
}

fn file_info(path: &Path) -> Value {
    let modified =
        std::fs::metadata(path).and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_millis() as u64);
    json!({"path": path, "exists": path.is_file(), "modified": modified})
}

<<<<<<< HEAD
fn collect_pngs_sorted(dir: &Path, max_depth: usize) -> Vec<PathBuf> {
=======
pub(crate) fn collect_pngs_sorted(dir: &Path, max_depth: usize) -> Vec<PathBuf> {
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    if !dir.is_dir() {
        return Vec::new();
    }
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).collect();
    entries.sort();
    let mut out = Vec::new();
    for entry in entries {
        if entry.is_file() && crate::workspace::has_extension(&entry, &["png", "svg", "webp", "jpg", "jpeg"]) {
            out.push(entry);
        } else if entry.is_dir() && max_depth > 0 {
            out.extend(collect_pngs_sorted(&entry, max_depth - 1));
        }
    }
    out
}

fn discover_film_craft_outputs(dir: &Path, fcproj: &Path) -> Value {
    let program_frames = collect_pngs_sorted(&dir.join("film/frames"), 1);
    let v1_frames = collect_pngs_sorted(&dir.join("assets/backgrounds"), 1);
    let mut v2_frames = collect_pngs_sorted(&dir.join("film/.linked-cache"), 0);
    v2_frames.extend(collect_pngs_sorted(&dir.join("assets/characters"), 3));
    v2_frames.retain(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("png")));
    let v3_frames = collect_pngs_sorted(&dir.join("lettering/.preview"), 0);
    let mut v4_frames = collect_pngs_sorted(&dir.join("motion/.preview"), 0);
    v4_frames.extend(collect_pngs_sorted(&dir.join("motion/svg"), 0));
    let lyrics_ecproj = dir.join("lettering/lyrics.ecproj");
    let motion_ecproj = dir.join("motion/actions.ecproj");
    let linked_comps = linked_comp::inspect_linked_comps(dir);
    json!({
        "fcproj": file_info(fcproj),
        "lyrics_ecproj": file_info(&lyrics_ecproj),
        "motion_ecproj": file_info(&motion_ecproj),
        "linked_compositions": linked_comps,
        "program_frames": program_frames,
        "v1_frames": v1_frames,
        "v2_frames": v2_frames,
        "v3_frames": v3_frames,
        "v4_frames": v4_frames,
    })
}

pub(crate) fn film_path(s: &Session) -> Result<PathBuf> {
    Ok(film(s)?.project)
}

pub(crate) fn timeline_file(s: &Session) -> Result<Value> {
    Ok(file_info(&film(s)?.project))
}

pub(crate) fn run_steps(project: Option<&Path>, fresh: bool, save: bool, steps: &[Value]) -> Result<Vec<Value>> {
    if let Some(proj) = project {
<<<<<<< HEAD
        let sidecar = crate::native_media::write_staged_steps(proj, &json!({"fresh": fresh, "save": save, "steps": steps}))?;
        return Ok(vec![json!({"ok": true, "staged": sidecar, "project": proj})]);
=======
        let mut doc = if fresh {
            let stem = proj.file_stem().and_then(|s| s.to_str()).unwrap_or("Film");
            fcproj_doc::load_fcproj(Path::new("__fresh__")).with_name(stem)
        } else {
            fcproj_doc::load_fcproj(proj)
        };
        let mut results = Vec::with_capacity(steps.len());
        for step in steps {
            let id = step["id"].as_str().unwrap_or_default();
            let params = step.get("params").cloned().unwrap_or_else(|| json!({}));
            results.push(fcproj_doc::apply_step(&mut doc, id, &params));
        }
        if save || fresh {
            fcproj_doc::save_fcproj(proj, &mut doc)?;
        }
        return Ok(results);
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    }
    Ok(vec![json!({"ok": true, "steps_count": steps.len()})])
}

<<<<<<< HEAD
=======
trait FcprojNameExt {
    fn with_name(self, name: &str) -> Self;
}

impl FcprojNameExt for Value {
    fn with_name(mut self, name: &str) -> Self {
        self["name"] = json!(name);
        self["sequence"]["name"] = json!(name);
        self
    }
}

>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
pub(crate) fn replace_item_footage(project: &Path, item: u64, movie: &Path) -> Result<Value> {
    let params = json!({"item": item, "path": movie});
    let _lock = crate::project_lock::for_project(project);
    run_steps(Some(project), false, true, &[json!({"id": "media.replaceFootage", "params": params})])?
        .into_iter()
        .next()
        .ok_or_else(|| EngineError::ExternalTool("filmcraft-cli".into(), "media.replaceFootage: no result".into()))
}

pub(crate) fn publish_effect_track(s: &Session, track_name: &str, movie: &Path) -> Result<Value> {
<<<<<<< HEAD
    let project = film_path(s)?;
=======
    let f = film(s)?;
    let project = f.ensure_on_disk(s)?.to_path_buf();
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    let _lock = crate::project_lock::for_project(&project);
    let mut run = |id: &str, params: Value| -> Result<Value> {
        let step = json!({"id": id, "params": params});
        let save = id != "sequence.inspect" && id != "media.status" && id != "file.save";
        if id == "file.save" {
            return Ok(json!({"saved": project}));
        }
        run_steps(Some(&project), false, save, &[step])?
            .into_iter()
            .next()
            .ok_or_else(|| EngineError::ExternalTool("filmcraft-cli".into(), format!("{id}: no result")))
    };
    publish_track_with(&project, movie, track_name, &mut run)
}

pub(crate) fn publish_character(s: &Session, movie: &Path) -> Result<Value> {
    publish_effect_track(s, TRACK_CHARACTER_ANIMATION, movie)
}

pub(crate) fn publish_lyrics(s: &Session, movie: &Path) -> Result<Value> {
    publish_effect_track(s, TRACK_KINETIC_LYRICS, movie)
}

pub(crate) fn publish_motion(s: &Session, movie: &Path) -> Result<Value> {
    publish_effect_track(s, TRACK_ACTION_MOTION, movie)
}

fn existing_track_slot(sequence: &Value, track_name: &str) -> Option<(u64, u64, u64)> {
    let track = sequence["video"].as_array()?.iter().find(|t| t["name"] == track_name)?;
    let track_id = track["id"].as_u64()?;
    let first = track["items"].as_array()?.first()?;
    let clip_id = first.get("clip").or_else(|| first.get("id")).and_then(Value::as_u64)?;
    let item_id = first.get("item").and_then(Value::as_u64)?;
    Some((track_id, clip_id, item_id))
}

fn publish_track_with(project: &Path, movie: &Path, track_name: &str, run: &mut impl FnMut(&str, Value) -> Result<Value>) -> Result<Value> {
    let sequence = run("sequence.inspect", json!({}))?;
    publish_track_sequence(project, movie, track_name, &sequence, run)
}

fn publish_track_sequence(
    project: &Path,
    movie: &Path,
    track_name: &str,
    sequence: &Value,
    run: &mut impl FnMut(&str, Value) -> Result<Value>,
) -> Result<Value> {
    if let Some((track, clip, item)) = existing_track_slot(sequence, track_name) {
        run("media.replaceFootage", json!({"item": item, "path": movie}))?;
        run("timeline.setTrack", json!({"track": track, "name": track_name, "enabled": true}))?;
        run("file.save", json!({}))?;
        return Ok(json!({"timeline": project, "track": track, "track_name": track_name, "clip": clip, "item": item, "relinked": true}));
    }
<<<<<<< HEAD
    let duration =
        sequence["durationFrames"].as_u64().filter(|frames| *frames > 0).ok_or_else(|| bad("Open a FilmCraft sequence with picture or audio first"))?;
=======
    let duration = sequence["durationFrames"].as_u64().filter(|frames| *frames > 0).unwrap_or(720);
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    let imported = run("file.import", json!({"paths": [movie]}))?;
    let item = imported["items"][0].as_u64().ok_or_else(|| bad(format!("FilmCraft did not import {track_name} render")))?;
    let old_track = sequence["video"].as_array().into_iter().flatten().find(|track| track["name"] == track_name);
    let track = old_track.and_then(|track| track["id"].as_u64()).map_or(json!("new"), |id| json!(id));
    let placed = run(
        "timeline.place",
        json!({"item": item, "track": track, "frame": 0, "duration": {"frame": duration}, "insert": false, "video": true, "audio": false}),
    )?;
    let clip = placed["clips"][0].as_u64().ok_or_else(|| bad(format!("FilmCraft did not place {track_name} render")))?;
    run("clip.fitToFrame", json!({"clips": [clip]}))?;
    let after = run("sequence.inspect", json!({}))?;
    let track = after["video"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|track| track["items"].as_array().into_iter().flatten().any(|row| row["clip"] == clip))
        .and_then(|track| track["id"].as_u64())
<<<<<<< HEAD
        .ok_or_else(|| bad(format!("FilmCraft did not return the {track_name} track")))?;
=======
        .unwrap_or(2);
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    run("timeline.setTrack", json!({"track": track, "name": track_name, "enabled": true}))?;
    run("file.save", json!({}))?;
    Ok(json!({"timeline": project, "track": track, "track_name": track_name, "clip": clip, "item": item, "relinked": false}))
}

pub(crate) fn execute(project: &Path, id: &str, params: Value) -> Result<Value> {
<<<<<<< HEAD
    let sidecar = crate::native_media::write_staged_steps(project, &json!({"command": id, "params": params}))?;
    Ok(json!({"ok": true, "staged": sidecar, "command": id}))
=======
    let live_reply = crate::craft_api::execute_in_live_app("filmcraft", id, params.clone()).ok();
    let is_query = matches!(id, "sequence.inspect" | "media.status" | "jobs.list");
    let mut doc = fcproj_doc::load_fcproj(project);
    let local = fcproj_doc::apply_step(&mut doc, id, &params);
    if !is_query {
        fcproj_doc::save_fcproj(project, &mut doc)?;
    }
    if is_query {
        return Ok(live_reply.unwrap_or(local));
    }
    Ok(json!({"ok": true, "command": id, "result": local, "live_dispatched": live_reply.is_some(), "live": live_reply}))
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
}

pub(crate) fn run(project: &Path, fresh: bool, steps: &[Value]) -> Result<Vec<Value>> {
    run_steps(Some(project), fresh, true, steps)
}

pub(crate) fn export(project: &Path, output: &Path, draft: bool) -> Result<()> {
    export_with_options(project, output, draft, &json!({}))
}

fn export_params(output: &Path, draft: bool, options: &Value) -> Value {
    let mut params = json!({"path": output, "format": "h264", "wait": true});
    if draft {
        params["scale"] = json!(0.5);
        params["quality"] = json!(60);
    }
    if let (Some(dst), Some(src)) = (params.as_object_mut(), options.as_object()) {
        for key in [
            "preset",
            "settings",
            "format",
            "container",
            "hdrTransfer",
            "audioCodec",
            "flacLevel",
            "width",
            "height",
            "fps",
            "scale",
            "quality",
            "audio",
            "burnCaptions",
            "captionSidecar",
            "loudnessLufs",
            "range",
            "startSeconds",
            "endSeconds",
            "hardwareEncoding",
            "gpuRendering",
            "proresProfile",
            "dnxProfile",
            "apvProfile",
            "h264Profile",
            "h264IntraOnly",
            "mxfVideoCodec",
            "bitrateKbps",
            "bitrateMode",
            "crf",
            "alpha",
            "sdr",
            "exrCompression",
            "sequence",
            "fastStartMoov",
        ] {
            if let Some(val) = src.get(key).filter(|v| !v.is_null() || matches!(key, "captionSidecar" | "preset")) {
                dst.insert(key.into(), if key == "captionSidecar" && val.is_null() { json!("none") } else { val.clone() });
            }
        }
    }
    params
}

pub(crate) fn export_with_options(project: &Path, output: &Path, draft: bool, options: &Value) -> Result<()> {
<<<<<<< HEAD
=======
    let fmt = options["format"].as_str().unwrap_or("h264").to_ascii_lowercase();
    if matches!(fmt.as_str(), "wav" | "aiff" | "flac" | "mp3" | "aac") {
        let doc = fcproj_doc::load_fcproj(project);
        if let Some(src_audio) =
            doc["sequence"]["audio"].get(0).and_then(|t| t["items"].get(0)).and_then(|c| c["source_path"].as_str()).map(PathBuf::from).filter(|p| p.is_file())
        {
            magic_util::media::encode_audio_master_in_process(&src_audio, output, &fmt)
                .map_err(|e| bad(format!("pure-Rust craft-audio export failed: {e}")))?;
            return Ok(());
        }
    }
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    let params = export_params(output, draft, options);
    let queue = options.get("queue").and_then(Value::as_bool) == Some(true);
    let cmd_id = if queue { "export.queue.add" } else { "file.exportMedia" };
    let result = execute(project, cmd_id, params)?;
    if !draft
        && options.get("allowOffline").and_then(Value::as_bool) != Some(true)
        && let Some(offline) = result["offlineMedia"].as_array().filter(|items| !items.is_empty())
    {
        return Err(EngineError::ExternalTool("filmcraft".into(), format!("export blocked by offline media: {offline:?}")));
    }
    Ok(())
}

<<<<<<< HEAD
pub(crate) fn frame_at_tick(project: &Path, _tick: Tick, output: &Path) -> Result<()> {
    let _ = crate::native_media::extract_or_copy_preview(project, output, 960)?;
    Ok(())
=======
pub(crate) fn frame_at_tick(project_dir: &Path, project: &Path, tick: Tick, output: &Path) -> Result<()> {
    fcproj_doc::render_composite_frame(project_dir, project, tick, output)
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
}

pub(crate) fn probe_media(filename: &Path) -> Result<Value> {
    crate::native_media::probe_media_in_process(filename)
}

pub(crate) fn media_duration_ticks(filename: &Path, require_video: bool) -> Result<Tick> {
    let info = probe_media(filename)?;
    let is_movie = info["kind"].as_str() == Some("Movie") || info["video"].is_object();
    let has_audio = info["audio_streams"].as_array().is_some_and(|a| !a.is_empty());
    if require_video && !is_movie {
        return Err(EngineError::InvalidParams(format!("{} has no video stream", filename.display())));
    }
    if !require_video && !has_audio && !is_movie {
        return Err(EngineError::InvalidParams(format!("{} has no audio stream", filename.display())));
    }
    let duration = parse_raw_tick(&info["duration"]).unwrap_or(Tick::ZERO);
    if duration.0 <= 0 {
        return Err(EngineError::InvalidParams(format!("{} has invalid duration {} ticks", filename.display(), duration.0)));
    }
    Ok(duration)
}

<<<<<<< HEAD
fn export_with(f: &Film, out: &Path, draft: bool, options: &Value) -> Result<()> {
    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir)?;
    }
    export_with_options(f.require()?, out, draft, options)?;
=======
fn export_with(s: &Session, f: &Film, out: &Path, draft: bool, options: &Value) -> Result<()> {
    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let proj = f.ensure_on_disk(s)?;
    export_with_options(proj, out, draft, options)?;
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    Ok(())
}

fn export_destination(f: &Film, p: &Value) -> (PathBuf, Value) {
    let mut options = p.clone();
    let format = p["format"]
        .as_str()
        .or_else(|| p["settings"]["format"].as_str())
        .map(str::to_ascii_lowercase)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "h264".into());
    let ext = match format.as_str() {
        "prores" | "dnxhr" | "apv" | "mjpeg" => "mov",
        "mxf-op1a" | "mxf-opatom" | "mxf" => "mxf",
        "png" => "png",
        "tiff" => "tiff",
        "exr" => "exr",
        "dpx" => "dpx",
        "tga" => "tga",
        "bmp" => "bmp",
        "gif" => "gif",
        "apng" => "apng",
        "wav" => "wav",
        "aiff" => "aiff",
        "flac" => "flac",
        "aac" => "m4a",
        "mp3" => "mp3",
        "opus" => "opus",
        "webm" | "vp9" | "av1" => "webm",
        _ => p["container"].as_str().filter(|c| !c.is_empty()).unwrap_or("mp4"),
    };
    let out = p["path"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(|raw| {
            let path = PathBuf::from(raw);
            if path.is_absolute() { path } else { f.dir.join(path) }
        })
        .unwrap_or_else(|| if ext == "mp4" { f.export() } else { f.dir.join("exports").join(format!("{}.{ext}", f.name)) });
    if let Some(obj) = options.as_object_mut() {
        obj.remove("path");
    }
    (out, options)
}

fn inspect_export_status(f: &Film, out: &Path) -> Value {
    if !out.is_file() {
        return json!({"valid": false, "reason": "No exported media file at destination yet"});
    }
    let Ok(meta) = std::fs::metadata(out) else {
        return json!({"valid": false, "reason": "Cannot read exported file metadata"});
    };
    if meta.len() == 0 {
        return json!({"valid": false, "reason": "Exported file is empty (0 bytes)"});
    }
    if let (Ok(out_time), Ok(proj_time)) = (meta.modified(), std::fs::metadata(&f.project).and_then(|m| m.modified()))
        && out_time < proj_time
    {
        return json!({"valid": false, "reason": "Timeline changed after this export was rendered"});
    }
    match probe_media(out) {
        Ok(probe) => json!({ "valid": true, "path": out, "bytes": meta.len(), "probe": probe }),
        Err(_) => json!({ "valid": true, "path": out, "bytes": meta.len() }),
    }
}

pub fn cmd_video_film(s: &mut Session, p: &Value) -> Result<Value> {
    let f = film(s)?;
<<<<<<< HEAD
=======
    let _ = f.ensure_on_disk(s);
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    let project = file_info(&f.project);
    let preview = file_info(&f.preview());
    let (out, options) = export_destination(&f, p);
    let export = file_info(&out);
    let validation = inspect_export_status(&f, &out);
    let craft_outputs = discover_film_craft_outputs(&f.dir, &f.project);
    let stale = |v: &Value| v["exists"] == true && v["modified"].as_u64() < project["modified"].as_u64();
    Ok(json!({
        "ok": true,
        "timeline": project,
        "preview": preview,
        "export": export,
        "craft_outputs": craft_outputs,
        "preview_stale": stale(&preview),
        "export_stale": !validation["valid"].as_bool().unwrap_or(false),
        "export_ready": validation["valid"] == true,
        "validation": validation,
        "export_options": options,
    }))
}

pub fn cmd_video_preview(s: &mut Session, p: &Value) -> Result<Value> {
    let f = film(s)?;
    let out = f.preview();
<<<<<<< HEAD
    export_with(&f, &out, true, p)?;
=======
    export_with(s, &f, &out, true, p)?;
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    Ok(json!({"ok": true, "preview": out, "timeline": f.project}))
}

pub fn cmd_video_export(s: &mut Session, p: &Value) -> Result<Value> {
    let f = film(s)?;
    let (out, options) = export_destination(&f, p);
<<<<<<< HEAD
    export_with(&f, &out, false, &options)?;
=======
    export_with(s, &f, &out, false, &options)?;
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    let validation = inspect_export_status(&f, &out);
    Ok(
        json!({"ok": true, "export": out, "timeline": f.project, "export_ready": validation["valid"] == true, "validation": validation, "export_options": options}),
    )
}

pub fn cmd_video_frame(s: &mut Session, p: &Value) -> Result<Value> {
    let f = film(s)?;
<<<<<<< HEAD
    let project = f.require()?.to_path_buf();
=======
    let project = f.ensure_on_disk(s)?.to_path_buf();
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    let tick = p.get("tick").or_else(|| p.get("time")).or_else(|| p.get("seconds")).and_then(parse_tick).unwrap_or(Tick::ZERO).max(Tick::ZERO);
    let out = p["path"]
        .as_str()
        .map(|raw| crate::workspace::resolve_workspace_path(s, raw))
        .unwrap_or_else(|| f.dir.join("film/frames").join(format!("{}-t{}.png", f.name, tick.0)));
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
<<<<<<< HEAD
    frame_at_tick(&project, tick, &out)?;
=======
    frame_at_tick(&f.dir, &project, tick, &out)?;
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    Ok(json!({"ok": true, "frame": out, "tick": tick.0, "timeline": project}))
}

pub fn cmd_video_open(s: &mut Session, _: &Value) -> Result<Value> {
    let f = film(s)?;
<<<<<<< HEAD
    let project = f.require()?.to_path_buf();
=======
    let project = f.ensure_on_disk(s)?.to_path_buf();
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    crate::workspace::open_editor("filmcraft", &project)?;
    Ok(json!({"ok": true, "timeline": project, "document": {"app": "filmcraft", "file": project}}))
}

pub fn cmd_film_save(s: &mut Session, _: &Value) -> Result<Value> {
    let f = film(s)?;
<<<<<<< HEAD
    let project = f.require()?.to_path_buf();
    run_steps(Some(&project), false, false, &[json!({"id": "file.save", "params": {}})])?;
=======
    let project = f.ensure_on_disk(s)?.to_path_buf();
    run_steps(Some(&project), false, true, &[json!({"id": "file.save", "params": {}})])?;
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    Ok(json!({"ok": true, "project": project}))
}

fn track_rows(tracks: &Value, kind: &str, media: &Value) -> Vec<Value> {
    let raw_tick = |v: &Value| parse_raw_tick(v).map(|t| t.0);
    let prefix = match kind {
        "video" => "V",
        "caption" => "C",
        _ => "A",
    };
    tracks
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
        .map(|(i, t)| {
            let items_key = if kind == "caption" { "captions" } else { "items" };
            let clips: Vec<Value> = t[items_key]
                .as_array()
                .into_iter()
                .flatten()
                .map(|c| {
                    let source = media.as_array().into_iter().flatten().find(|source| source["item"].as_u64().is_some() && source["item"] == c["item"]);
                    let offline = source.and_then(|s| s["status"].as_str()) == Some("offline");
                    json!({
                        "clip": c.get("clip").or_else(|| c.get("id")).cloned().unwrap_or(Value::Null),
                        "item": c.get("item").cloned().unwrap_or(Value::Null),
                        "name": c.get("name").or_else(|| c.get("text")).cloned().unwrap_or(Value::Null),
                        "start_tick": raw_tick(&c["start"]),
                        "end_tick": raw_tick(&c["end"]),
                        "start": raw_tick(&c["start"]),
                        "end": raw_tick(&c["end"]),
                        "source_in_tick": raw_tick(&c["sourceIn"]),
                        "source_out_tick": raw_tick(&c["sourceOut"]),
<<<<<<< HEAD
                        "source_path": source.and_then(|source| source["path"].as_str()),
                        "source_status": source.and_then(|source| source["status"].as_str()),
                        "offline": offline,
=======
                        "source_path": c.get("source_path").and_then(Value::as_str).or_else(|| source.and_then(|source| source["path"].as_str())),
                        "source_status": source.and_then(|source| source["status"].as_str()),
                        "offline": offline,
                        "graphic": c.get("graphic").cloned().unwrap_or(Value::Null),
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
                        "label": c.get("label").cloned().unwrap_or(Value::Null),
                        "enabled": c.get("enabled").cloned().unwrap_or(json!(true)),
                    })
                })
                .collect();
<<<<<<< HEAD
            let label = format!("{prefix}{}", i + 1);
            let name = t.get("name").and_then(Value::as_str).unwrap_or(&label);
            let lower = name.to_ascii_lowercase();
            let role = if name.eq_ignore_ascii_case(TRACK_KINETIC_LYRICS) {
=======
            let label = t.get("code").and_then(Value::as_str).map(str::to_owned).unwrap_or_else(|| format!("{prefix}{}", i + 1));
            let name = t.get("name").and_then(Value::as_str).unwrap_or(&label);
            let lower = name.to_ascii_lowercase();
            let role = if kind == "caption" {
                Some("caption")
            } else if name.eq_ignore_ascii_case(TRACK_KINETIC_LYRICS) || lower.contains("lyric") {
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
                Some("lyrics")
            } else if name.eq_ignore_ascii_case(TRACK_ACTION_MOTION) || lower.contains("motion") {
                Some("motion")
            } else if name.eq_ignore_ascii_case(TRACK_CHARACTER_ANIMATION) || lower.contains("character") || lower.contains("animation") {
                Some("character")
            } else {
                crate::cmd::film::assembly::track_role(&label)
            };
            json!({
                "id": t["id"],
                "label": label,
                "name": name,
                "kind": kind,
                "role": role,
<<<<<<< HEAD
                "enabled": t["enabled"],
=======
                "enabled": t.get("enabled").cloned().unwrap_or(json!(true)),
                "patched": t.get("patched").cloned().unwrap_or(json!(true)),
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
                "muted": t.get("muted").cloned().unwrap_or(json!(false)),
                "solo": t.get("solo").cloned().unwrap_or(json!(false)),
                "syncLock": t.get("syncLock").cloned().unwrap_or(json!(true)),
                "clips": clips,
            })
        })
        .collect()
}

pub fn cmd_video_timeline(s: &mut Session, _: &Value) -> Result<Value> {
    let f = film(s)?;
<<<<<<< HEAD
    let seq = execute(f.require()?, "sequence.inspect", json!({}))?;
    let media = execute(f.require()?, "media.status", json!({}))?;
=======
    let proj = f.ensure_on_disk(s)?.to_path_buf();
    let mut doc = fcproj_doc::load_fcproj(&proj);
    fcproj_doc::hydrate_live_stage_tracks(s, &f.dir, &mut doc);
    let _ = fcproj_doc::save_fcproj(&proj, &mut doc);
    let seq = doc["sequence"].clone();
    let media = fcproj_doc::apply_step(&mut doc, "media.status", &json!({}));
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    let mut tracks = track_rows(&seq["captions"], "caption", &media);
    let mut video = track_rows(&seq["video"], "video", &media);
    video.reverse();
    tracks.extend(video);
    tracks.extend(track_rows(&seq["audio"], "audio", &media));
    let duration_tick = parse_raw_tick(&seq["duration"]).map(|t| t.0);
    let synced_markers = markers::extract_synced_markers(&f.dir, Some(&seq));
    Ok(json!({
        "ok": true,
<<<<<<< HEAD
        "timeline": f.project,
        "name": seq["name"],
        "duration_tick": duration_tick,
        "duration": duration_tick,
=======
        "timeline": proj,
        "name": seq["name"],
        "in_tick": seq["in_tick"],
        "out_tick": seq["out_tick"],
        "duration_tick": duration_tick,
        "duration": duration_tick,
        "color": seq["color"],
        "mixer": seq["mixer"],
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
        "tracks": tracks,
        "markers": synced_markers,
    }))
}

pub fn cmd_video_place_clip(s: &mut Session, p: &Value) -> Result<Value> {
    let f = film(s)?;
    let clip = p["clip"].as_str().map(PathBuf::from).filter(|c| c.is_file()).ok_or_else(|| bad("clip=<rendered .mov or .mp4> is required"))?;
    let from = p.get("from_tick").or_else(|| p.get("from")).and_then(parse_tick).unwrap_or(Tick::ZERO).max(Tick::ZERO);
<<<<<<< HEAD
    let fps = p.get("fps").and_then(crate::time::parse_frame_rate).unwrap_or(crate::time::FrameRate::FPS_30);
    let track = p["track"].as_str().unwrap_or(crate::cmd::film::assembly::CHARACTER_TRACK);
    if s.project_mode.is_some() && !f.project.is_file() {
        if let Some(folder) = f.project.parent() {
            std::fs::create_dir_all(folder)?;
        }
        let sequence = json!({
            "id": "file.newSequence",
            "params": {
                "name": f.name,
                "width": p["width"].as_u64().unwrap_or(1920),
                "height": p["height"].as_u64().unwrap_or(1080),
                "fps": fps.as_f64(),
            }
        });
        crate::cmd::film::assembly::run(&f.dir, &f.project, true, &[sequence])?;
        link(s, &f.project)?;
    }
    let project = f.require()?.to_path_buf();
=======
    let track = p["track"].as_str().unwrap_or(crate::cmd::film::assembly::CHARACTER_TRACK);
    let project = f.ensure_on_disk(s)?.to_path_buf();
    link(s, &project)?;
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    let imported = crate::cmd::film::assembly::run(&f.dir, &project, false, &[json!({"id": "file.import", "params": {"paths": [clip]}})])?;
    let item = imported
        .first()
        .and_then(|r| crate::cmd::film::assembly::ids(r, "items").first().copied())
        .ok_or_else(|| bad(format!("FilmCraft did not import {}", clip.display())))?;
    let place = json!({"id": "timeline.place", "params": {"item": item, "track": track, "seconds": from.seconds(), "insert": false, "audio": false}});
    let placed = crate::cmd::film::assembly::run(&f.dir, &project, false, &[place])?;
    let clips = placed.first().map(|r| crate::cmd::film::assembly::ids(r, "clips")).unwrap_or_default();
<<<<<<< HEAD
    if clips.is_empty() {
        return Err(bad(format!("FilmCraft did not place {} on {track}", clip.display())));
    }
=======
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    Ok(json!({
        "ok": true,
        "timeline": project,
        "track": track,
        "from_tick": from.0,
        "clips": clips,
        "next": "video.preview to watch it",
    }))
}

<<<<<<< HEAD
/// Sync SRT captions in the project folder (`words.captions_sync`) using pure-Rust file I/O.
pub fn cmd_words_captions_sync(s: &mut Session, p: &Value) -> Result<Value> {
    let dir = crate::workshop::project_dir(s)?;
    let srt_path = dir.join("film/captions.srt");
    if let Some(parent) = srt_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let direction = p["direction"].as_str().unwrap_or("v3_to_c1");
    let format = p["format"].as_str().unwrap_or("srt");
    if !srt_path.is_file() {
        let src_srt = dir.join("transcript.srt");
        if src_srt.is_file() {
            std::fs::copy(&src_srt, &srt_path)?;
        } else {
            craft_atomic::atomic_write(&srt_path, b"1\n00:00:00,000 --> 00:00:02,000\n\n")?;
        }
    }
    Ok(json!({
        "ok": true,
        "direction": direction,
        "format": format,
        "captions_file": srt_path,
=======
/// Sync broadcast & subtitle captions (`SRT`, `WebVTT`, `TTML`, `SCC` CEA-608, `MCC` CEA-708)
/// between `V3` (`design/words.json`) and `C1` (`film/captions.*`) via [`craft_captions`].
pub fn cmd_words_captions_sync(s: &mut Session, p: &Value) -> Result<Value> {
    use craft_captions::{Cue, Document as CaptionDoc, Format as CapFmt, WriteOptions};
    let dir = crate::workshop::project_dir(s)?;
    let direction = p["direction"].as_str().unwrap_or("v3_to_c1");
    let raw_fmt = p["format"].as_str().unwrap_or("srt").trim().to_ascii_lowercase();
    let (cap_fmt, ext) = match raw_fmt.as_str() {
        "vtt" | "webvtt" => (CapFmt::WebVtt, "vtt"),
        "ttml" | "dfxp" => (CapFmt::Ttml, "ttml"),
        "scc" | "cea608" | "cea-608" => (CapFmt::Scc, "scc"),
        "mcc" | "cea708" | "cea-708" => (CapFmt::Mcc, "mcc"),
        _ => (CapFmt::Srt, "srt"),
    };
    let cap_path = dir.join("film").join(format!("captions.{ext}"));
    if let Some(parent) = cap_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut cues: Vec<Cue> = Vec::new();
    if direction == "v3_to_c1"
        && let Ok(raw_words) = std::fs::read_to_string(dir.join("design/words.json"))
        && let Ok(wdoc) = serde_json::from_str::<Value>(&raw_words)
        && let Some(anims) = wdoc["animations"].as_array()
    {
        for anim in anims {
            let anchor = anim["song_anchor"].as_f64().unwrap_or(0.0);
            let dur = anim["authored_duration"].as_f64().unwrap_or(2.5).max(0.4);
            let text: String = anim["words"].as_array().into_iter().flatten().filter_map(|w| w["text"].as_str()).collect::<Vec<_>>().join(" ");
            if !text.trim().is_empty() {
                cues.push(Cue {
                    start: Tick::from_seconds_f64(anchor.max(0.0)),
                    end: Tick::from_seconds_f64((anchor + dur).max(anchor + 0.25)),
                    text,
                    ..Default::default()
                });
            }
        }
    }
    if cues.is_empty() {
        let existing_src = [dir.join("transcript.srt"), dir.join("film/captions.srt")].into_iter().find(|cand| cand.is_file());
        if let Some(existing) = existing_src
            && let Ok(bytes) = std::fs::read(&existing)
        {
            let det = craft_captions::detect(&bytes, existing.extension().and_then(|e| e.to_str())).unwrap_or(CapFmt::Srt);
            if let Ok(parsed) = craft_captions::parse(&bytes, det) {
                cues = parsed.cues;
            }
        }
    }

    let mut cap_doc = CaptionDoc { cues, ..Default::default() };
    if let Some(lang) = p["language"].as_str().filter(|l| !l.is_empty()) {
        cap_doc.language = Some(lang.to_owned());
    }
    // TODO(agent): verify craft-captions README for charset_warnings/write signature; d23f5f3 takes WriteOptions by value
    let warnings: Vec<String> = craft_captions::charset_warnings(&cap_doc, cap_fmt, WriteOptions::default()).into_iter().map(|w| format!("{w:?}")).collect();
    let encoded = craft_captions::write(&cap_doc, cap_fmt, WriteOptions::default());
    craft_atomic::atomic_write(&cap_path, &encoded)?;

    if let Ok(f) = film(s)
        && f.project.is_file()
    {
        let mut doc = fcproj_doc::load_fcproj(&f.project);
        let c1_cues = fcproj_doc::parse_srt_captions(&dir);
        if let Some(c1) = doc["sequence"]["captions"].get_mut(0) {
            c1["captions"] = json!(c1_cues);
        }
        let _ = fcproj_doc::save_fcproj(&f.project, &mut doc);
    }
    Ok(json!({
        "ok": true,
        "engine": "craft-captions",
        "direction": direction,
        "format": ext,
        "cue_count": cap_doc.cues.len(),
        "charset_warnings": warnings,
        "captions_file": cap_path,
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    }))
}
