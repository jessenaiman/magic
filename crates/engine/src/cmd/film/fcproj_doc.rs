//! Minimal `.fcproj` track/SRT/frame document helper (`crates/engine/src/cmd/film/fcproj_doc.rs`).
//!
//! Used only for inspecting track geometry, SRT captions, and composite preview frames when neither
//! live `filmcraft --control 9876` nor `filmcraft-cli` is installed. Zero fake Lumetri or mixer state.

use std::path::Path;

use serde_json::{Value, json};

use crate::Result;
use crate::time::{TICKS_PER_SECOND, parse_raw_tick};

const DEFAULT_FPS: f64 = 24.0;

pub(crate) fn sec_to_ticks(sec: f64) -> i64 {
    (sec.max(0.0) * (TICKS_PER_SECOND as f64)).round() as i64
}

fn frame_to_ticks(frame: i64, fps: f64) -> i64 {
    let rate = if fps.is_finite() && fps > 0.0 { fps } else { DEFAULT_FPS };
    ((frame.max(0) as f64 / rate) * (TICKS_PER_SECOND as f64)).round() as i64
}

pub(crate) fn new_fcproj(name: &str, width: u64, height: u64, fps: f64) -> Value {
    json!({
        "format": "filmcraft.project",
        "version": 1,
        "name": name,
        "next_item_id": 1_u64,
        "next_clip_id": 100_u64,
        "next_marker_id": 1_u64,
        "media": [],
        "sequence": {
            "name": name,
            "width": width,
            "height": height,
            "fps": fps,
            "dropFrame": false,
            "duration": 0_i64,
            "durationFrames": 0_u64,
            "in_tick": 0_i64,
            "out_tick": 0_i64,
            "video": [
                {
                    "id": 1_u64, "code": "V1", "name": "Storyboard Scenes",
                    "enabled": true, "patched": true, "target": true, "syncLock": true, "muted": false, "solo": false, "items": []
                },
                {
                    "id": 2_u64, "code": "V2", "name": "Character Animation",
                    "enabled": true, "patched": true, "target": false, "syncLock": true, "muted": false, "solo": false, "items": []
                },
                {
                    "id": 3_u64, "code": "V3", "name": "Kinetic Lyrics",
                    "enabled": true, "patched": true, "target": false, "syncLock": true, "muted": false, "solo": false, "items": []
                },
                {
                    "id": 4_u64, "code": "V4", "name": "Action Motion",
                    "enabled": true, "patched": true, "target": false, "syncLock": true, "muted": false, "solo": false, "items": []
                },
            ],
            "audio": [
                {
                    "id": 10_u64, "code": "A1", "name": "Song Master",
                    "enabled": true, "patched": true, "target": true, "syncLock": true, "muted": false, "solo": false, "items": []
                },
            ],
            "captions": [
                {"id": 20_u64, "code": "C1", "name": "Captions", "enabled": true, "muted": false, "solo": false, "captions": []},
            ],
            "markers": [],
        }
    })
}

pub(crate) fn load_fcproj(project_path: &Path) -> Value {
    let stem = project_path.file_stem().and_then(|s| s.to_str()).unwrap_or("Film");
    if let Ok(raw) = std::fs::read_to_string(project_path)
        && let Ok(parsed) = serde_json::from_str::<Value>(&raw)
        && parsed["sequence"]["video"].is_array()
    {
        return parsed;
    }
    new_fcproj(stem, 1920, 1080, DEFAULT_FPS)
}

pub(crate) fn save_fcproj(project_path: &Path, doc: &mut Value) -> Result<()> {
    recompute_sequence_duration(doc);
    let _ = crate::doc_state::write_json(project_path, doc)?;
    Ok(())
}

pub(crate) fn recompute_sequence_duration(doc: &mut Value) {
    let fps = doc["sequence"]["fps"].as_f64().unwrap_or(DEFAULT_FPS).max(1.0);
    let mut max_tick = doc["sequence"]["duration"].as_i64().unwrap_or(0);
    for section in ["video", "audio"] {
        for trk in doc["sequence"][section].as_array().into_iter().flatten() {
            for clip in trk["items"].as_array().into_iter().flatten() {
                max_tick = max_tick.max(clip["end"].as_i64().unwrap_or(0));
            }
        }
    }
    for cap_trk in doc["sequence"]["captions"].as_array().into_iter().flatten() {
        for cue in cap_trk["captions"].as_array().into_iter().flatten() {
            max_tick = max_tick.max(cue["end"].as_i64().unwrap_or(0));
        }
    }
    doc["sequence"]["duration"] = json!(max_tick);
    doc["sequence"]["durationFrames"] = json!(((max_tick.max(0) as f64 / (TICKS_PER_SECOND as f64)) * fps).round() as u64);
}

fn alloc_id(doc: &mut Value, key: &str, start_at: u64) -> u64 {
    let cur = doc[key].as_u64().unwrap_or(start_at);
    doc[key] = json!(cur + 1);
    cur
}

fn resolve_track_index(tracks: &[Value], selector: &Value) -> Option<usize> {
    if let Some(id_num) = selector.as_u64() {
        if let Some(pos) = tracks.iter().position(|t| t["id"].as_u64() == Some(id_num)) {
            return Some(pos);
        }
        let one_based = (id_num as usize).checked_sub(1)?;
        return (one_based < tracks.len()).then_some(one_based);
    }
    let raw = selector.as_str()?.trim();
    tracks
        .iter()
        .position(|t| t["code"].as_str().is_some_and(|c| c.eq_ignore_ascii_case(raw)) || t["name"].as_str().is_some_and(|n| n.eq_ignore_ascii_case(raw)))
}

fn import_media_paths(doc: &mut Value, paths: &[Value]) -> Vec<u64> {
    let mut ids = Vec::with_capacity(paths.len());
    for p in paths {
        let Some(raw) = p.as_str().filter(|s| !s.is_empty()) else { continue };
        if let Some(existing) = doc["media"].as_array().into_iter().flatten().find(|m| m["path"].as_str() == Some(raw)).and_then(|m| m["item"].as_u64()) {
            ids.push(existing);
            continue;
        }
        let id = alloc_id(doc, "next_item_id", 1);
        let pb = Path::new(raw);
        let name = pb.file_name().and_then(|n| n.to_str()).unwrap_or(raw).to_owned();
        let dur_ticks = super::probe_media(pb).ok().and_then(|info| parse_raw_tick(&info["duration"])).map_or(sec_to_ticks(5.0), |t| t.0);
        if let Some(arr) = doc["media"].as_array_mut() {
            arr.push(json!({"item": id, "path": raw, "name": name, "duration_tick": dur_ticks}));
        }
        ids.push(id);
    }
    ids
}

fn place_media_clip(doc: &mut Value, params: &Value) -> Vec<u64> {
    let fps = doc["sequence"]["fps"].as_f64().unwrap_or(DEFAULT_FPS);
    let item_id = params["item"].as_u64().unwrap_or(0);
    let media_entry = doc["media"].as_array().into_iter().flatten().find(|m| m["item"].as_u64() == Some(item_id)).cloned();
    let default_name = media_entry.as_ref().and_then(|m| m["name"].as_str()).unwrap_or("Clip").to_owned();
    let media_path = media_entry.as_ref().and_then(|m| m["path"].as_str()).map(str::to_owned);
    let media_dur = media_entry.as_ref().and_then(|m| m["duration_tick"].as_i64()).unwrap_or_else(|| sec_to_ticks(5.0));

    let start_tick = params["seconds"]
        .as_f64()
        .map(sec_to_ticks)
        .or_else(|| params["frame"].as_i64().map(|fr| frame_to_ticks(fr, fps)))
        .unwrap_or_else(|| params["start_tick"].as_i64().unwrap_or(0));
    let len_tick = params
        .pointer("/duration/frame")
        .and_then(Value::as_i64)
        .map(|df| frame_to_ticks(df.max(1), fps))
        .or_else(|| params["duration_sec"].as_f64().or_else(|| params.pointer("/duration/seconds").and_then(Value::as_f64)).map(|ds| sec_to_ticks(ds.max(0.1))))
        .unwrap_or_else(|| media_dur.max(sec_to_ticks(0.5)));
    let source_in = params["sourceIn"].as_i64().unwrap_or(0);
    let clip_id = alloc_id(doc, "next_clip_id", 100);
    let clip_obj = json!({
        "clip": clip_id,
        "id": clip_id,
        "item": if item_id > 0 { json!(item_id) } else { Value::Null },
        "name": params["name"].as_str().unwrap_or(&default_name),
        "start": start_tick,
        "end": start_tick + len_tick,
        "sourceIn": source_in,
        "sourceOut": source_in + len_tick,
        "source_path": media_path,
        "enabled": true,
    });

    let track_sel = &params["track"];
    let section = if params["video"].as_bool() == Some(false) || track_sel.as_str().is_some_and(|t| t.starts_with(['A', 'a'])) { "audio" } else { "video" };
    let idx = {
        let existing = doc["sequence"][section].as_array().map(Vec::as_slice).unwrap_or(&[]);
        if track_sel.as_str() == Some("new") {
            let next_idx = existing.len();
            let prefix = if section == "audio" { "A" } else { "V" };
            let new_id = if section == "audio" { 10 + next_idx as u64 } else { 1 + next_idx as u64 };
            if let Some(arr) = doc["sequence"][section].as_array_mut() {
                arr.push(json!({
                    "id": new_id, "code": format!("{prefix}{}", next_idx + 1), "name": format!("{prefix}{}", next_idx + 1),
                    "enabled": true, "patched": true, "target": false, "syncLock": true, "muted": false, "solo": false, "items": [],
                }));
            }
            next_idx
        } else {
            resolve_track_index(existing, track_sel).unwrap_or(0)
        }
    };
    if let Some(items) = doc["sequence"][section].get_mut(idx).and_then(|t| t["items"].as_array_mut()) {
        items.retain(|c| {
            !(c["start"].as_i64().unwrap_or(0) >= start_tick && c["end"].as_i64().unwrap_or(0) <= start_tick + len_tick && c["name"] == clip_obj["name"])
        });
        items.push(clip_obj);
        items.sort_by_key(|c| c["start"].as_i64().unwrap_or(0));
    }
    recompute_sequence_duration(doc);
    vec![clip_id]
}

fn razor_split_at(doc: &mut Value, split_tick: i64, track_filter: Option<&str>) -> usize {
    let mut splits = 0usize;
    let next_base = doc["next_clip_id"].as_u64().unwrap_or(500);
    let mut alloc_offset = 0u64;
    if let Some(vtracks) = doc["sequence"]["video"].as_array_mut() {
        for trk in vtracks.iter_mut() {
            let code = trk["code"].as_str().unwrap_or_default();
            if let Some(tf) = track_filter
                && !code.eq_ignore_ascii_case(tf)
                && trk["name"].as_str() != Some(tf)
            {
                continue;
            }
            let Some(items) = trk["items"].as_array_mut() else { continue };
            let mut rebuilt = Vec::with_capacity(items.len() + 1);
            for clip in items.drain(..) {
                let st = clip["start"].as_i64().unwrap_or(0);
                let en = clip["end"].as_i64().unwrap_or(0);
                if st < split_tick && en > split_tick + sec_to_ticks(0.04) {
                    let mut left = clip.clone();
                    left["end"] = json!(split_tick);
                    left["sourceOut"] = json!(clip["sourceIn"].as_i64().unwrap_or(0) + (split_tick - st));
                    let mut right = clip;
                    let right_id = next_base + alloc_offset;
                    alloc_offset += 1;
                    right["clip"] = json!(right_id);
                    right["id"] = json!(right_id);
                    right["start"] = json!(split_tick);
                    right["sourceIn"] = left["sourceOut"].clone();
                    rebuilt.push(left);
                    rebuilt.push(right);
                    splits += 1;
                } else {
                    rebuilt.push(clip);
                }
            }
            *items = rebuilt;
        }
    }
    doc["next_clip_id"] = json!(next_base + alloc_offset);
    splits
}

#[allow(clippy::cognitive_complexity)]
pub(crate) fn apply_step(doc: &mut Value, id: &str, params: &Value) -> Value {
    match id {
        "file.newProject" => {
            if let Some(name) = params["name"].as_str() {
                doc["name"] = json!(name);
                doc["sequence"]["name"] = json!(name);
            }
            json!({"ok": true})
        }
        "file.newSequence" => {
            if let Some(name) = params["name"].as_str() {
                doc["sequence"]["name"] = json!(name);
            }
            if let Some(w) = params["width"].as_u64() {
                doc["sequence"]["width"] = json!(w);
            }
            if let Some(h) = params["height"].as_u64() {
                doc["sequence"]["height"] = json!(h);
            }
            if let Some(fps) = params["fps"].as_f64().filter(|&v| v > 0.0) {
                doc["sequence"]["fps"] = json!(fps);
            }
            json!({"ok": true})
        }
        "file.import" => json!({"ok": true, "items": import_media_paths(doc, &params["paths"].as_array().cloned().unwrap_or_default())}),
        "timeline.place" => json!({"ok": true, "clips": place_media_clip(doc, params)}),
        "sequence.inspect" => doc["sequence"].clone(),
        "media.status" => Value::Array(
            doc["media"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|m| {
                    let exists = m["path"].as_str().is_some_and(|p| Path::new(p).exists());
                    json!({"item": m["item"], "path": m["path"], "name": m["name"], "status": if exists { "online" } else { "offline" }})
                })
                .collect(),
        ),
        "media.replaceFootage" => {
            let target = params["item"].as_u64();
            let new_path = params["path"].as_str().unwrap_or_default();
            for m in doc["media"].as_array_mut().into_iter().flatten() {
                if m["item"].as_u64() == target {
                    m["path"] = json!(new_path);
                }
            }
            for section in ["video", "audio"] {
                for trk in doc["sequence"][section].as_array_mut().into_iter().flatten() {
                    for c in trk["items"].as_array_mut().into_iter().flatten() {
                        if c["item"].as_u64() == target {
                            c["source_path"] = json!(new_path);
                        }
                    }
                }
            }
            json!({"ok": true, "item": target, "path": new_path})
        }
        "timeline.setTrack" => {
            for section in ["video", "audio", "captions"] {
                let idx_opt = doc["sequence"][section].as_array().and_then(|arr| resolve_track_index(arr, &params["track"]));
                if let Some(idx) = idx_opt
                    && let Some(trk) = doc["sequence"][section].get_mut(idx)
                {
                    for key in ["name", "enabled", "patched", "target", "syncLock", "muted", "solo"] {
                        if let Some(v) = params.get(key) {
                            trk[key] = v.clone();
                        }
                    }
                    break;
                }
            }
            json!({"ok": true})
        }
        "timeline.razor" | "sequence.addEdit" => {
            let tick = sec_to_ticks(params["seconds"].as_f64().unwrap_or(0.0));
            json!({"ok": true, "splits": razor_split_at(doc, tick, params["track"].as_str()), "tick": tick})
        }
        "markers.add" | "markers.addRange" | "markers.addChapter" | "markers.addFlashCue" => {
            let mid = alloc_id(doc, "next_marker_id", 1);
            let kind = match id {
                "markers.addRange" => "Range",
                "markers.addChapter" => "Chapter",
                "markers.addFlashCue" => "FlashCue",
                _ => "Comment",
            };
            if let Some(arr) = doc["sequence"]["markers"].as_array_mut() {
                arr.push(json!({
                    "id": mid,
                    "name": params["name"].as_str().unwrap_or("Cue"),
                    "comment": params["comment"].as_str().unwrap_or(""),
                    "color": params["color"].as_str().unwrap_or("Iris"),
                    "kind": kind,
                    "start": params["time"].as_i64().unwrap_or(0),
                    "duration": params["duration"].as_i64().unwrap_or(0),
                    "author": params.get("author").cloned().unwrap_or(Value::Null),
                    "url": params.get("url").cloned().unwrap_or(Value::Null),
                }));
            }
            json!({"ok": true, "marker": mid})
        }
        "markers.delete" => {
            let target = params["marker"].as_u64();
            if let Some(arr) = doc["sequence"]["markers"].as_array_mut() {
                arr.retain(|m| m["id"].as_u64() != target);
            }
            json!({"ok": true})
        }
        "markers.clearAll" => {
            doc["sequence"]["markers"] = json!([]);
            json!({"ok": true})
        }
        "markers.edit" => {
            let target = params["marker"].as_u64();
            for m in doc["sequence"]["markers"].as_array_mut().into_iter().flatten() {
                if m["id"].as_u64() == target {
                    for k in ["name", "comment", "author", "color", "kind", "url"] {
                        if let Some(v) = params.get(k) {
                            m[k] = v.clone();
                        }
                    }
                }
            }
            json!({"ok": true, "marker": target})
        }
        _ => json!({"ok": true, "command": id}),
    }
}

pub(crate) use super::fcproj_stage::{hydrate_live_stage_tracks, parse_srt_captions, render_composite_frame};
