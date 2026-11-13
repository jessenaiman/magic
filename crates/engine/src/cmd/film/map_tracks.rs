//! Batch Studio track-to-group mapping (`video.map_tracks`, `B10-CLI`) and open multi-tool
//! animation asset recognition (`B10-ANIM`) across `Blender`, `Three.js`, `After Effects`,
//! `AniCraft`, `EffectCraft`, and `FilmCraft`-supported media formats.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::fcproj_doc::{load_fcproj, save_fcproj};
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

fn parse_v_track_number(sel: &str) -> Option<usize> {
    let digits = sel.trim().strip_prefix('V').or_else(|| sel.trim().strip_prefix('v'))?;
    let num = digits.parse::<usize>().ok()?;
    (1..=32).contains(&num).then_some(num)
}

fn resolve_or_materialize_video_track(doc: &mut Value, selector: &str) -> Option<usize> {
    let trimmed = selector.trim();
    if let Some(pos) = doc["sequence"]["video"].as_array().and_then(|arr| {
        arr.iter().position(|t| {
            t["code"].as_str().is_some_and(|c| c.eq_ignore_ascii_case(trimmed))
                || t["name"].as_str().is_some_and(|n| n.eq_ignore_ascii_case(trimmed))
                || trimmed.parse::<u64>().ok().is_some_and(|id| t["id"].as_u64() == Some(id))
        })
    }) {
        return Some(pos);
    }

    let wanted_n = parse_v_track_number(trimmed)?;
    let vtracks = doc["sequence"]["video"].as_array_mut()?;
    while vtracks.len() < wanted_n {
        let next_n = vtracks.len() + 1;
        let code = format!("V{next_n}");
        vtracks.push(json!({
            "id": next_n as u64,
            "code": code,
            "name": format!("Video {next_n}"),
            "enabled": true,
            "patched": true,
            "target": false,
            "syncLock": true,
            "muted": false,
            "solo": false,
            "items": [],
        }));
    }
    Some(wanted_n - 1)
}

#[allow(clippy::cognitive_complexity)]
fn upsert_target_ecproj(ecproj_path: &Path, comp_name: &str, target: &str, fcproj_path: &Path, group_entries: &[Value]) -> Result<()> {
    if let Some(parent) = ecproj_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut doc =
        std::fs::read_to_string(ecproj_path).ok().and_then(|raw| serde_json::from_str::<Value>(&raw).ok()).filter(Value::is_object).unwrap_or_else(|| {
            json!({
                "savedBy": format!("magic-studio/{}", env!("CARGO_PKG_VERSION")),
                "activeComp": comp_name,
                "openComps": [comp_name],
                "schema": 2,
                "compositions": [{"name": comp_name, "groups": []}],
                "track_groups": [],
            })
        });

    if !doc.get("savedBy").is_some_and(Value::is_string) {
        doc["savedBy"] = json!(format!("magic-studio/{}", env!("CARGO_PKG_VERSION")));
    }
    if !doc.get("schema").is_some_and(Value::is_u64) {
        doc["schema"] = json!(2);
    }
    doc["target"] = json!(target);
    doc["source_fcproj"] = json!(fcproj_path);

    let mut groups: Vec<Value> = doc.get("track_groups").and_then(Value::as_array).cloned().unwrap_or_default();
    for entry in group_entries {
        let gname = entry["group"].as_str().unwrap_or_default();
        if let Some(slot) = groups.iter_mut().find(|g| g["group"].as_str() == Some(gname)) {
            *slot = entry.clone();
        } else {
            groups.push(entry.clone());
        }
    }
    doc["track_groups"] = Value::Array(groups.clone());

    let mut comps: Vec<Value> = doc.get("compositions").and_then(Value::as_array).cloned().unwrap_or_default();
    if let Some(primary) = comps.iter_mut().find(|c| c["name"].as_str() == Some(comp_name)) {
        primary["groups"] = Value::Array(groups);
    } else {
        comps.push(json!({"name": comp_name, "groups": groups}));
    }
    for entry in group_entries {
        if let Some(gname) = entry["group"].as_str().filter(|g| !g.is_empty() && *g != comp_name)
            && !comps.iter().any(|c| c["name"].as_str() == Some(gname))
        {
            comps.push(json!({"name": gname, "role": "mapped_group", "target": target, "tracks": entry["tracks"]}));
        }
    }
    doc["compositions"] = Value::Array(comps);

    let _ = crate::doc_state::write_json(ecproj_path, &doc)?;
    Ok(())
}

/// `video.map_tracks`: batch-map selected `FilmCraft` tracks from `<project>/film/<slug>.fcproj`
/// into reusable `Lyrics` (`effects/lyrics.ecproj`) and `Motion` (`effects/motion.ecproj`) groups.
// shortcut: writes schema-2 track_groups metadata into effects/{lyrics,motion}.ecproj; upgrade to full layer timeline serialization when EffectCraft imports per-clip keyframes directly
#[allow(clippy::cognitive_complexity)]
pub fn cmd_video_map_tracks(s: &mut Session, p: &Value) -> Result<Value> {
    let mappings = parse_and_validate_mappings(p)?;
    let f = super::film(s)?;
    let fcproj_path = f.ensure_on_disk(s)?.to_path_buf();
    super::link(s, &fcproj_path)?;
    let mut fc_doc = load_fcproj(&fcproj_path);

    // Pre-verify all track selectors before mutating `.fcproj` or `.ecproj` files
    let mut resolved_indices: Vec<Vec<(String, usize)>> = Vec::with_capacity(mappings.len());
    for m in &mappings {
        let mut idxs = Vec::with_capacity(m.tracks.len());
        for sel in &m.tracks {
            let idx =
                resolve_or_materialize_video_track(&mut fc_doc, sel).ok_or_else(|| bad(format!("track `{sel}` not found in {}", fcproj_path.display())))?;
            let code = fc_doc["sequence"]["video"][idx]["code"].as_str().unwrap_or(sel.as_str()).to_owned();
            idxs.push((code, idx));
        }
        resolved_indices.push(idxs);
    }

    let mut lyrics_groups: Vec<Value> = Vec::new();
    let mut motion_groups: Vec<Value> = Vec::new();
    let mut summaries: Vec<Value> = Vec::with_capacity(mappings.len());

    for (m, track_pairs) in mappings.iter().zip(resolved_indices.iter()) {
        let mut collected_clips = Vec::new();
        let mut codes = Vec::with_capacity(track_pairs.len());
        let label_color = if m.target == "lyrics" { "Iris" } else { "Mango" };

        for (code, idx) in track_pairs {
            codes.push(code.clone());
            if let Some(trk) = fc_doc["sequence"]["video"].get_mut(*idx) {
                trk["mapped_target"] = json!(m.target);
                trk["mapped_group"] = json!(m.group);
                trk["role"] = json!(m.target);
                if let Some(items) = trk["items"].as_array_mut() {
                    for clip in items.iter_mut() {
                        clip["mapped_target"] = json!(m.target);
                        clip["mapped_group"] = json!(m.group);
                        if clip.get("label").is_none_or(Value::is_null) {
                            clip["label"] = json!(label_color);
                        }
                        collected_clips.push(json!({
                            "track": code,
                            "clip": clip.get("clip").or_else(|| clip.get("id")).cloned().unwrap_or(Value::Null),
                            "name": clip.get("name").cloned().unwrap_or(Value::Null),
                            "start_tick": clip.get("start").cloned().unwrap_or(json!(0)),
                            "end_tick": clip.get("end").cloned().unwrap_or(json!(0)),
                            "source_path": clip.get("source_path").cloned().unwrap_or(Value::Null),
                        }));
                    }
                }
            }
        }

        let ecproj_rel = if m.target == "lyrics" { "effects/lyrics.ecproj" } else { "effects/motion.ecproj" };
        let group_entry = json!({
            "group": m.group,
            "target": m.target,
            "tracks": codes,
            "clip_count": collected_clips.len(),
            "clips": collected_clips,
        });
        if m.target == "lyrics" {
            lyrics_groups.push(group_entry);
        } else {
            motion_groups.push(group_entry);
        }
        summaries.push(json!({
            "target": m.target,
            "group": m.group,
            "tracks": codes,
            "clip_count": collected_clips.len(),
            "ecproj": f.dir.join(ecproj_rel),
        }));
    }

    save_fcproj(&fcproj_path, &mut fc_doc)?;

    let lyrics_ecproj = if !lyrics_groups.is_empty() {
        let p_lyr = f.dir.join("effects/lyrics.ecproj");
        upsert_target_ecproj(&p_lyr, "Lyrics", "lyrics", &fcproj_path, &lyrics_groups)?;
        Some(p_lyr)
    } else {
        None
    };
    let motion_ecproj = if !motion_groups.is_empty() {
        let p_mot = f.dir.join("effects/motion.ecproj");
        upsert_target_ecproj(&p_mot, "Motion", "motion", &fcproj_path, &motion_groups)?;
        Some(p_mot)
    } else {
        None
    };

    Ok(json!({
        "ok": true,
        "timeline": fcproj_path,
        "mapped_count": summaries.len(),
        "mappings": summaries,
        "lyrics_ecproj": lyrics_ecproj,
        "motion_ecproj": motion_ecproj,
    }))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::{Value, json};

    use super::{cmd_video_map_tracks, detect_animation_tool, discover_project_animations};
    use crate::cmd::film::fcproj_doc::{load_fcproj, new_fcproj, save_fcproj};
    use crate::{EngineError, Session};

    fn temp_project_session(label: &str) -> (PathBuf, Session) {
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("magic_map_tracks_{label}_{}_{nanos}", std::process::id()));
        for sub in ["audio", "assets", "effects", "film", "exports"] {
            std::fs::create_dir_all(dir.join(sub)).unwrap();
        }
        let magic_file = dir.join("demo-song.magic");
        crate::doc_state::write_json(&magic_file, &json!({"name": "Demo Song", "workshop": {}})).unwrap();
        let session = Session { project_path: Some(magic_file), ..Default::default() };
        (dir, session)
    }

    #[test]
    fn batch_maps_lyrics_and_motion_tracks_in_one_call_and_rejects_animation() {
        let (dir, mut session) = temp_project_session("batch");
        let fcproj_path = dir.join("film/demo-song.fcproj");
        let mut doc = new_fcproj("demo-song", 1920, 1080, 24.0);
        doc["sequence"]["video"][2]["items"] = json!([
            {"clip": 301, "id": 301, "name": "Hook Line 1", "start": 0, "end": 508_032_000_000_i64, "enabled": true}
        ]);
        doc["sequence"]["video"][3]["items"] = json!([
            {"clip": 401, "id": 401, "name": "Beat Burst", "start": 254_016_000_000_i64, "end": 762_048_000_000_i64, "enabled": true}
        ]);
        doc["sequence"]["video"].as_array_mut().unwrap().push(json!({
            "id": 5_u64,
            "code": "V5",
            "name": "Chorus Backing Words",
            "enabled": true,
            "patched": true,
            "target": false,
            "syncLock": true,
            "muted": false,
            "solo": false,
            "items": [
                {"clip": 501, "id": 501, "name": "Echo Line", "start": 508_032_000_000_i64, "end": 1_016_064_000_000_i64, "enabled": true}
            ]
        }));
        save_fcproj(&fcproj_path, &mut doc).unwrap();
        session.video_project_path = Some(fcproj_path.clone());

        let res = cmd_video_map_tracks(
            &mut session,
            &json!({
                "mappings": [
                    {"tracks": ["V3", "V5"], "target": "lyrics", "group": "chorus"},
                    {"tracks": ["V4"], "target": "motion", "group": "beats"}
                ]
            }),
        )
        .unwrap();

        assert_eq!(res["ok"], json!(true));
        assert_eq!(res["mapped_count"], json!(2));

        let lyr_path = dir.join("effects/lyrics.ecproj");
        let mot_path = dir.join("effects/motion.ecproj");
        assert!(lyr_path.is_file(), "expected effects/lyrics.ecproj to be written");
        assert!(mot_path.is_file(), "expected effects/motion.ecproj to be written");

        let lyr_head = magic_util::media::inspect_ecproj_head(&lyr_path).unwrap();
        assert_eq!(lyr_head.schema, Some(2));
        let lyr_doc: Value = serde_json::from_slice(&std::fs::read(&lyr_path).unwrap()).unwrap();
        assert_eq!(lyr_doc["track_groups"][0]["group"], json!("chorus"));
        assert_eq!(lyr_doc["track_groups"][0]["tracks"], json!(["V3", "V5"]));
        assert_eq!(lyr_doc["track_groups"][0]["clip_count"], json!(2));

        let mot_doc: Value = serde_json::from_slice(&std::fs::read(&mot_path).unwrap()).unwrap();
        assert_eq!(mot_doc["track_groups"][0]["group"], json!("beats"));
        assert_eq!(mot_doc["track_groups"][0]["tracks"], json!(["V4"]));
        assert_eq!(mot_doc["track_groups"][0]["clip_count"], json!(1));

        let saved_fc = load_fcproj(&fcproj_path);
        assert_eq!(saved_fc["sequence"]["video"][2]["mapped_group"], json!("chorus"));
        assert_eq!(saved_fc["sequence"]["video"][4]["mapped_group"], json!("chorus"));
        assert_eq!(saved_fc["sequence"]["video"][3]["mapped_group"], json!("beats"));

        // Explicitly reject `target == "animation"` (and `"animations"`)
        for forbidden in ["animation", "animations"] {
            let err = cmd_video_map_tracks(
                &mut session,
                &json!({
                    "mappings": [
                        {"tracks": ["V2"], "target": forbidden, "group": "hero_rig"}
                    ]
                }),
            )
            .unwrap_err();
            match err {
                EngineError::InvalidParams(msg) => {
                    assert!(msg.contains("lyrics") && msg.contains("motion") && msg.contains("animation"), "unexpected message: {msg}");
                }
                other => panic!("expected InvalidParams, got {other:?}"),
            }
        }
    }

    #[test]
    fn recognizes_multi_tool_saved_animations_without_live_effectcraft() {
        let (dir, _) = temp_project_session("open_anim");
        std::fs::create_dir_all(dir.join("assets/animations/blender_turntable_seq")).unwrap();
        std::fs::write(dir.join("assets/animations/blender_turntable_seq/frame_0001.png"), b"png1").unwrap();
        std::fs::write(dir.join("assets/animations/blender_turntable_seq/frame_0002.png"), b"png2").unwrap();
        std::fs::write(dir.join("assets/animations/threejs_orbit.webm"), b"webm").unwrap();
        std::fs::write(dir.join("assets/animations/after_effects_comp.mov"), b"mov").unwrap();
        std::fs::write(dir.join("assets/animations/anicraft_walk.gif"), b"gif").unwrap();
        std::fs::write(dir.join("effects/character_rig.ecproj"), br#"{"savedBy":"effectcraft/1.0","schema":2}"#).unwrap();

        let discovered = discover_project_animations(&dir);
        assert_eq!(discovered.len(), 5);
        let tools: Vec<&str> = discovered.iter().map(|p| detect_animation_tool(p)).collect();
        assert!(tools.contains(&"Blender"));
        assert!(tools.contains(&"Three.js"));
        assert!(tools.contains(&"After Effects"));
        assert!(tools.contains(&"AniCraft"));
        assert!(tools.contains(&"EffectCraft"));
    }
}
