//! Batch Studio track-to-group mapping (`video.map_tracks`, `B10-CLI`) and open multi-tool
//! animation asset recognition (`B10-ANIM`) across `Blender`, `Three.js`, `After Effects`,
//! `AniCraft`, `EffectCraft`, and `FilmCraft`-supported media formats.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::{EngineError, Result, Session};

/// Extensions imported natively as video, animated-still, or composition clips by `FilmCraft`.
pub const FILMCRAFT_ANIMATION_EXTENSIONS: &[&str] = &[
    "mov", "mp4", "m4v", "webm", "mkv", "avi", "mxf", "mpg", "mpeg", "m2ts", "mts", "ts", "ogg", "ogv", "gif", "apng", "webp", "png", "jpg", "jpeg", "tif",
    "tiff", "bmp", "exr", "dpx", "tga", "ecproj",
];

/// Extensions representing rendered moving-picture / animated-still clips (excluding static single-still formats unless part of a sequence).
const MOVING_CLIP_EXTENSIONS: &[&str] =
    &["mov", "mp4", "m4v", "webm", "mkv", "avi", "mxf", "mpg", "mpeg", "m2ts", "mts", "ts", "ogg", "ogv", "gif", "apng", "webp", "ecproj"];

const SEQUENCE_FRAME_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "tif", "tiff", "bmp", "exr", "dpx", "tga"];

fn bad(msg: impl Into<String>) -> EngineError {
    EngineError::InvalidParams(msg.into())
}

/// First frame of an image-sequence directory when `dir` holds 2+ numbered still frames.
pub fn first_image_sequence_frame(dir: &Path) -> Option<PathBuf> {
    if !dir.is_dir() {
        return None;
    }
    let mut frames: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && crate::workspace::has_extension(p, SEQUENCE_FRAME_EXTENSIONS)
                && p.file_stem().and_then(|s| s.to_str()).is_some_and(|st| st.chars().any(|c| c.is_ascii_digit()))
        })
        .collect();
    if frames.len() < 2 {
        return None;
    }
    frames.sort();
    frames.into_iter().next()
}

/// True when `path` is a `FilmCraft`-importable animation clip, `.ecproj`, or image-sequence folder.
pub fn is_supported_animation_output(path: &Path) -> bool {
    if path.is_file() {
        return crate::workspace::has_extension(path, FILMCRAFT_ANIMATION_EXTENSIONS);
    }
    first_image_sequence_frame(path).is_some()
}

/// Resolve `path` to the concrete media file `FilmCraft` imports (the file itself or frame 0 of an image sequence).
pub fn resolve_importable_animation_path(path: &Path) -> Option<PathBuf> {
    if path.is_file() && crate::workspace::has_extension(path, FILMCRAFT_ANIMATION_EXTENSIONS) {
        return Some(path.to_path_buf());
    }
    first_image_sequence_frame(path)
}

/// Identify the originating animation tool from path conventions or companion project files,
/// while treating every `FilmCraft`-supported clip identically at import time (`B10-ANIM`).
pub fn detect_animation_tool(path: &Path) -> &'static str {
    if path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("ecproj")) {
        return "EffectCraft";
    }
    let hay = path.to_string_lossy().to_ascii_lowercase();
    if hay.contains("blender") || hay.contains("cycles") || hay.contains("eevee") {
        "Blender"
    } else if hay.contains("threejs") || hay.contains("three.js") || hay.contains("three_") || hay.contains("three-") || hay.contains("webgl") {
        "Three.js"
    } else if hay.contains("aftereffects") || hay.contains("after-effects") || hay.contains("after_effects") || hay.contains("_ae") || hay.contains("-ae") {
        "After Effects"
    } else if hay.contains("anicraft") || hay.contains("ani_") || hay.contains("ani-") {
        "AniCraft"
    } else if hay.contains("effectcraft") {
        "EffectCraft"
    } else {
        "FilmCraft Media"
    }
}

fn is_reserved_stage_ecproj(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    name.eq_ignore_ascii_case("lyrics.ecproj") || name.eq_ignore_ascii_case("motion.ecproj") || name.eq_ignore_ascii_case("actions.ecproj")
}

fn scan_animation_dir(dir: &Path, depth: usize, allow_stills: bool, out: &mut Vec<PathBuf>) {
    if depth > 3 || !dir.is_dir() {
        return;
    }
    if depth > 0
        && let Some(seq_first) = first_image_sequence_frame(dir)
    {
        out.push(seq_first);
        return;
    }
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).collect();
    entries.sort();
    for entry in entries {
        if entry.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with('.')) {
            continue;
        }
        if entry.is_file() {
            if is_reserved_stage_ecproj(&entry) {
                continue;
            }
            let exts = if allow_stills { FILMCRAFT_ANIMATION_EXTENSIONS } else { MOVING_CLIP_EXTENSIONS };
            if crate::workspace::has_extension(&entry, exts) {
                out.push(entry);
            }
        } else if entry.is_dir() {
            let name = entry.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_ascii_lowercase();
            if matches!(name.as_str(), "backgrounds" | "slides" | "audio" | "liner_notes") {
                continue;
            }
            scan_animation_dir(&entry, depth + 1, allow_stills, out);
        }
    }
}

/// Discover saved animation renders, image sequences, and `.ecproj` compositions across
/// `<project>/assets/` and `<project>/effects/` from any animation tool (`B10-ANIM`).
pub(crate) fn discover_project_animations(project_dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for (sub, allow_stills) in [("assets/animations", true), ("assets/renders", true), ("assets", false), ("effects", false)] {
        scan_animation_dir(&project_dir.join(sub), 0, allow_stills, &mut found);
    }
    found.sort();
    found.dedup();
    found
}

/// Metadata summary of discovered multi-tool animation outputs for `video.film`.
pub(crate) fn discover_saved_animation_records(project_dir: &Path) -> Vec<Value> {
    discover_project_animations(project_dir)
        .into_iter()
        .map(|path| {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase();
            json!({
                "path": path,
                "tool": detect_animation_tool(&path),
                "format": ext,
                "live_link_required": false,
            })
        })
        .collect()
}

#[derive(Clone, Debug)]
struct ValidatedMapping {
    target: String,
    group: String,
    tracks: Vec<String>,
}

fn parse_and_validate_mappings(p: &Value) -> Result<Vec<ValidatedMapping>> {
    let raw_list = p
        .get("mappings")
        .and_then(Value::as_array)
        .filter(|arr| !arr.is_empty())
        .ok_or_else(|| bad("video.map_tracks requires a non-empty `mappings` array"))?;

    let mut validated = Vec::with_capacity(raw_list.len());
    for (idx, entry) in raw_list.iter().enumerate() {
        let raw_target = entry
            .get("target")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| bad(format!("mappings[{idx}] is missing `target` (`lyrics` or `motion`)")))?;
        let target = raw_target.to_ascii_lowercase();
        if target == "animation" || target == "animations" {
            return Err(bad("Studio track-to-group batch mapping only targets `lyrics` and `motion` (never `animation`)"));
        }
        if target != "lyrics" && target != "motion" {
            return Err(bad(format!(
                "unsupported target `{raw_target}` in mappings[{idx}]: Studio track-to-group batch mapping only targets `lyrics` and `motion` (never `animation`)"
            )));
        }

        let mut tracks: Vec<String> = entry
            .get("tracks")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect();
        if tracks.is_empty()
            && let Some(single) = entry.get("track").and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty())
        {
            tracks.push(single.to_owned());
        }
        if tracks.is_empty() {
            return Err(bad(format!("mappings[{idx}] requires at least one track in `tracks` (e.g. [\"V3\", \"V5\"])")));
        }

        let group = entry
            .get("group")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(if target == "lyrics" { "lyrics" } else { "motion" })
            .to_owned();

        validated.push(ValidatedMapping { target, group, tracks });
    }
    Ok(validated)
}

/// `video.map_tracks`: batch-map selected `FilmCraft` tracks from `<project>/film/<slug>.fcproj`.
pub fn cmd_video_map_tracks(s: &mut Session, p: &Value) -> Result<Value> {
    let mappings = parse_and_validate_mappings(p)?;
    let f = super::film(s)?;
    let fcproj_path = f.ensure_on_disk(s)?.to_path_buf();
    super::link(s, &fcproj_path)?;

    let summaries: Vec<Value> = mappings
        .iter()
        .map(|m| {
            let ecproj_rel = if m.target == "lyrics" { "effects/lyrics.ecproj" } else { "effects/motion.ecproj" };
            json!({
                "target": m.target,
                "group": m.group,
                "tracks": m.tracks,
                "ecproj": f.dir.join(ecproj_rel),
            })
        })
        .collect();

    Ok(json!({
        "ok": true,
        "timeline": fcproj_path,
        "mapped_count": summaries.len(),
        "mappings": summaries,
    }))
}
