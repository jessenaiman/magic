//! Pure-Rust `FilmCraft` `.fcproj` sequence state engine (`crates/engine/src/cmd/film/fcproj_doc.rs`).
//!
//! Single canonical `.fcproj` document model for `05 Studio` (`V1` Storyboard Scenes,
//! `V2` Character Animation, `V3` Kinetic Lyrics, `V4` Action Motion & Titles, `C1` Captions,
//! and `A1` Song Master) — zero `.steps.json` sidecars and clean native track schema.

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
            "color": {
                "workingSpace": "rec709",
                "wideGamut": false,
                "autoToneMap": true,
                "hdrWhiteNits": 203,
                "exposure": 0.0,
                "saturation": 100.0,
                "vignette": 0.0,
                "look": "builtin:warm_storybook",
            },
            "mixer": {
                "A1": {"volume": 0.0, "pan": 0.0, "mute": false, "solo": false, "mode": "Read", "inserts": []},
                "A2": {"volume": -3.0, "pan": 0.0, "mute": false, "solo": false, "mode": "Read", "inserts": []},
                "Master": {"volume": 0.0, "pan": 0.0, "mute": false, "solo": false, "mode": "Read", "inserts": []},
            },
            "video": [
                {"id": 1_u64, "code": "V1", "name": "Storyboard Scenes", "enabled": true, "patched": true, "target": true, "syncLock": true, "muted": false, "solo": false, "items": []},
                {"id": 2_u64, "code": "V2", "name": "Character Animation", "enabled": true, "patched": true, "target": false, "syncLock": true, "muted": false, "solo": false, "items": []},
                {"id": 3_u64, "code": "V3", "name": "Kinetic Lyrics", "enabled": true, "patched": true, "target": false, "syncLock": true, "muted": false, "solo": false, "items": []},
                {"id": 4_u64, "code": "V4", "name": "Action Motion", "enabled": true, "patched": true, "target": false, "syncLock": true, "muted": false, "solo": false, "items": []},
            ],
            "audio": [
                {"id": 10_u64, "code": "A1", "name": "Song Master", "enabled": true, "patched": true, "target": true, "syncLock": true, "muted": false, "solo": false, "items": []},
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
                let end = clip["end"].as_i64().unwrap_or(0);
                if end > max_tick {
                    max_tick = end;
                }
            }
        }
    }
    for cap_trk in doc["sequence"]["captions"].as_array().into_iter().flatten() {
        for cue in cap_trk["captions"].as_array().into_iter().flatten() {
            let end = cue["end"].as_i64().unwrap_or(0);
            if end > max_tick {
                max_tick = end;
            }
        }
    }
    doc["sequence"]["duration"] = json!(max_tick);
    let frames = ((max_tick.max(0) as f64 / (TICKS_PER_SECOND as f64)) * fps).round() as u64;
    doc["sequence"]["durationFrames"] = json!(frames);
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
        if one_based < tracks.len() {
            return Some(one_based);
        }
    }
    let raw = selector.as_str()?.trim();
    tracks
        .iter()
        .position(|t| t["code"].as_str().is_some_and(|c| c.eq_ignore_ascii_case(raw)) || t["name"].as_str().is_some_and(|n| n.eq_ignore_ascii_case(raw)))
}

fn import_media_paths(doc: &mut Value, paths: &[Value]) -> Vec<u64> {
    let mut ids = Vec::with_capacity(paths.len());
    for p in paths {
        let Some(raw) = p.as_str().filter(|s| !s.is_empty()) else {
            continue;
        };
        if let Some(existing_id) = doc["media"].as_array().into_iter().flatten().find(|m| m["path"].as_str() == Some(raw)).and_then(|m| m["item"].as_u64()) {
            ids.push(existing_id);
            continue;
        }
        let id = alloc_id(doc, "next_item_id", 1);
        let pb = Path::new(raw);
        let name = pb.file_name().and_then(|n| n.to_str()).unwrap_or(raw).to_owned();
        let dur_ticks = super::probe_media(pb).ok().and_then(|info| parse_raw_tick(&info["duration"])).map_or(sec_to_ticks(5.0), |t| t.0);
        if let Some(arr) = doc["media"].as_array_mut() {
            arr.push(json!({
                "item": id,
                "path": raw,
                "name": name,
                "duration_tick": dur_ticks,
            }));
        }
        ids.push(id);
    }
    ids
}

#[allow(clippy::cognitive_complexity)]
fn place_media_clip(doc: &mut Value, params: &Value) -> Vec<u64> {
    let fps = doc["sequence"]["fps"].as_f64().unwrap_or(DEFAULT_FPS);
    let item_id = params["item"].as_u64().unwrap_or(0);
    let media_entry = doc["media"].as_array().into_iter().flatten().find(|m| m["item"].as_u64() == Some(item_id)).cloned();
    let default_name = media_entry.as_ref().and_then(|m| m["name"].as_str()).unwrap_or("Clip").to_owned();
    let media_path = media_entry.as_ref().and_then(|m| m["path"].as_str()).map(str::to_owned);
    let media_dur = media_entry.as_ref().and_then(|m| m["duration_tick"].as_i64()).unwrap_or_else(|| sec_to_ticks(5.0));

    let start_tick = if let Some(sec) = params["seconds"].as_f64() {
        sec_to_ticks(sec)
    } else if let Some(fr) = params["frame"].as_i64() {
        frame_to_ticks(fr, fps)
    } else {
        params["start_tick"].as_i64().unwrap_or(0)
    };
    let len_tick = if let Some(df) = params.pointer("/duration/frame").and_then(Value::as_i64) {
        frame_to_ticks(df.max(1), fps)
    } else if let Some(ds) = params["duration_sec"].as_f64().or_else(|| params.pointer("/duration/seconds").and_then(Value::as_f64)) {
        sec_to_ticks(ds.max(0.1))
    } else {
        media_dur.max(sec_to_ticks(0.5))
    };
    let source_in = params["sourceIn"].as_i64().unwrap_or(0);
    let clip_id = alloc_id(doc, "next_clip_id", 100);
    let clip_name = params["name"].as_str().unwrap_or(&default_name).to_owned();
    let clip_obj = json!({
        "clip": clip_id,
        "id": clip_id,
        "item": if item_id > 0 { json!(item_id) } else { Value::Null },
        "name": clip_name,
        "start": start_tick,
        "end": start_tick + len_tick,
        "sourceIn": source_in,
        "sourceOut": source_in + len_tick,
        "source_path": media_path,
        "enabled": true,
        "speedPct": 100.0,
        "blendMode": "Normal",
    });

    let track_sel = &params["track"];
    let want_audio_only = params["video"].as_bool() == Some(false) || track_sel.as_str().is_some_and(|t| t.starts_with('A') || t.starts_with('a'));
    let section = if want_audio_only { "audio" } else { "video" };

    let idx = {
        let existing = doc["sequence"][section].as_array().map(Vec::as_slice).unwrap_or(&[]);
        if track_sel.as_str() == Some("new") {
            let next_idx = existing.len();
            let prefix = if section == "audio" { "A" } else { "V" };
            let new_id = if section == "audio" { 10 + next_idx as u64 } else { 1 + next_idx as u64 };
            if let Some(arr) = doc["sequence"][section].as_array_mut() {
                arr.push(json!({
                    "id": new_id,
                    "code": format!("{prefix}{}", next_idx + 1),
                    "name": format!("{prefix}{}", next_idx + 1),
                    "enabled": true,
                    "patched": true,
                    "target": false,
                    "syncLock": true,
                    "muted": false,
                    "solo": false,
                    "items": [],
                }));
            }
            next_idx
        } else {
            resolve_track_index(existing, track_sel).unwrap_or(0)
        }
    };

    let insert_ripple = params["insert"].as_bool() == Some(true);
    if let Some(items) = doc["sequence"][section].get_mut(idx).and_then(|t| t["items"].as_array_mut()) {
        if insert_ripple {
            for c in items.iter_mut() {
                let st = c["start"].as_i64().unwrap_or(0);
                if st >= start_tick {
                    c["start"] = json!(st + len_tick);
                    c["end"] = json!(c["end"].as_i64().unwrap_or(st) + len_tick);
                }
            }
        } else {
            items.retain(|c| {
                let st = c["start"].as_i64().unwrap_or(0);
                let en = c["end"].as_i64().unwrap_or(0);
                !(st >= start_tick && en <= start_tick + len_tick && c["name"] == clip_obj["name"])
            });
        }
        items.push(clip_obj);
        items.sort_by_key(|c| c["start"].as_i64().unwrap_or(0));
    }
    recompute_sequence_duration(doc);
    vec![clip_id]
}

fn razor_split_at(doc: &mut Value, split_tick: i64, track_filter: Option<&str>) -> usize {
    let mut splits = 0usize;
    let total_candidates: usize = doc["sequence"]["video"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|t| t["items"].as_array().into_iter().flatten())
        .filter(|c| c["start"].as_i64().unwrap_or(0) < split_tick && c["end"].as_i64().unwrap_or(0) > split_tick)
        .count();
    let mut new_ids = Vec::with_capacity(total_candidates.max(1));
    for _ in 0..total_candidates.max(1) {
        new_ids.push(alloc_id(doc, "next_clip_id", 100));
    }
    let mut id_iter = new_ids.into_iter();
    if let Some(vtracks) = doc["sequence"]["video"].as_array_mut() {
        for trk in vtracks.iter_mut() {
            let code = trk["code"].as_str().unwrap_or_default();
            let patched = trk["patched"].as_bool().unwrap_or(true);
            if let Some(tf) = track_filter {
                if !code.eq_ignore_ascii_case(tf) && trk["name"].as_str() != Some(tf) {
                    continue;
                }
            } else if !patched {
                continue;
            }
            let Some(items) = trk["items"].as_array_mut() else { continue };
            let mut rebuilt = Vec::with_capacity(items.len() + 1);
            for clip in items.drain(..) {
                let st = clip["start"].as_i64().unwrap_or(0);
                let en = clip["end"].as_i64().unwrap_or(0);
                if st < split_tick && en > split_tick + sec_to_ticks(0.04) {
                    let mut left = clip.clone();
                    let delta = split_tick - st;
                    left["end"] = json!(split_tick);
                    left["sourceOut"] = json!(clip["sourceIn"].as_i64().unwrap_or(0) + delta);
                    let mut right = clip;
                    let right_id = id_iter.next().unwrap_or(9999);
                    right["clip"] = json!(right_id);
                    right["id"] = json!(right_id);
                    right["start"] = json!(split_tick);
                    right["sourceIn"] = json!(left["sourceOut"].as_i64().unwrap_or(0));
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
    splits
}

fn lift_or_extract_range(doc: &mut Value, ripple: bool) {
    let in_t = doc["sequence"]["in_tick"].as_i64().unwrap_or(0);
    let out_t = doc["sequence"]["out_tick"].as_i64().unwrap_or(0);
    if out_t <= in_t {
        return;
    }
    let span = out_t - in_t;
    let _ = razor_split_at(doc, in_t, None);
    let _ = razor_split_at(doc, out_t, None);
    if let Some(vtracks) = doc["sequence"]["video"].as_array_mut() {
        for trk in vtracks.iter_mut() {
            if trk["patched"].as_bool() == Some(false) {
                continue;
            }
            if let Some(items) = trk["items"].as_array_mut() {
                items.retain(|c| !(c["start"].as_i64().unwrap_or(0) >= in_t && c["end"].as_i64().unwrap_or(0) <= out_t));
                if ripple {
                    for c in items.iter_mut() {
                        let st = c["start"].as_i64().unwrap_or(0);
                        if st >= out_t {
                            c["start"] = json!((st - span).max(0));
                            c["end"] = json!((c["end"].as_i64().unwrap_or(st) - span).max(0));
                        }
                    }
                }
            }
        }
    }
    recompute_sequence_duration(doc);
}

#[allow(clippy::cognitive_complexity)]
fn apply_direct_clip_edit(doc: &mut Value, id: &str, params: &Value) -> Value {
    let target_clip = params["clip"].as_u64().or_else(|| params["id"].as_u64());
    let mut updated = false;
    for section in ["video", "audio"] {
        let Some(tracks) = doc["sequence"][section].as_array_mut() else { continue };
        for trk in tracks.iter_mut() {
            let Some(items) = trk["items"].as_array_mut() else { continue };
            if id == "clip.delete" {
                let before = items.len();
                if let Some(cid) = target_clip {
                    let mut removed_span: Option<(i64, i64)> = None;
                    items.retain(|c| {
                        let hit = c["clip"].as_u64() == Some(cid) || c["id"].as_u64() == Some(cid);
                        if hit {
                            removed_span = Some((c["start"].as_i64().unwrap_or(0), c["end"].as_i64().unwrap_or(0)));
                        }
                        !hit
                    });
                    if params["ripple"].as_bool() == Some(true)
                        && let Some((st, en)) = removed_span
                    {
                        let gap = (en - st).max(0);
                        for c in items.iter_mut() {
                            let cst = c["start"].as_i64().unwrap_or(0);
                            if cst >= en {
                                c["start"] = json!((cst - gap).max(0));
                                c["end"] = json!((c["end"].as_i64().unwrap_or(cst) - gap).max(0));
                            }
                        }
                    }
                }
                if items.len() != before {
                    updated = true;
                }
                continue;
            }
            for clip in items.iter_mut() {
                let matches_id = target_clip.is_some_and(|cid| clip["clip"].as_u64() == Some(cid) || clip["id"].as_u64() == Some(cid));
                if !matches_id && target_clip.is_some() {
                    continue;
                }
                match id {
                    "clip.rename" => {
                        if let Some(name) = params["name"].as_str().filter(|s| !s.trim().is_empty()) {
                            clip["name"] = json!(name.trim());
                            updated = true;
                        }
                    }
                    "clip.trim" | "clip.move" => {
                        let old_len = (clip["end"].as_i64().unwrap_or(0) - clip["start"].as_i64().unwrap_or(0)).max(sec_to_ticks(0.2));
                        if let Some(st_s) = params["start_sec"].as_f64().or_else(|| params["seconds"].as_f64()) {
                            let new_st = sec_to_ticks(st_s);
                            clip["start"] = json!(new_st);
                            if id == "clip.move" && params.get("end_sec").is_none() {
                                clip["end"] = json!(new_st + old_len);
                            }
                        }
                        if let Some(en_s) = params["end_sec"].as_f64() {
                            let min_en = clip["start"].as_i64().unwrap_or(0) + sec_to_ticks(0.15);
                            clip["end"] = json!(sec_to_ticks(en_s).max(min_en));
                        }
                        if let Some(nm) = params["name"].as_str().filter(|s| !s.trim().is_empty()) {
                            clip["name"] = json!(nm.trim());
                        }
                        updated = true;
                    }
                    "clip.rateStretch" => {
                        let pct = params["speedPct"].as_f64().unwrap_or(100.0).clamp(10.0, 800.0);
                        let st = clip["start"].as_i64().unwrap_or(0);
                        let cur_len = (clip["end"].as_i64().unwrap_or(st) - st).max(sec_to_ticks(0.2));
                        let scaled = ((cur_len as f64) * (100.0 / pct)).round() as i64;
                        clip["end"] = json!(st + scaled.max(sec_to_ticks(0.15)));
                        clip["speedPct"] = json!(pct);
                        updated = true;
                        if target_clip.is_none() {
                            break;
                        }
                    }
                    "clip.setBlendMode" => {
                        clip["blendMode"] = json!(params["mode"].as_str().unwrap_or("Normal"));
                        updated = true;
                    }
                    "clip.motion" => {
                        clip["motion"] = params.clone();
                        updated = true;
                    }
                    _ => {}
                }
                if matches_id {
                    break;
                }
            }
            items.sort_by_key(|c| c["start"].as_i64().unwrap_or(0));
        }
    }
    recompute_sequence_duration(doc);
    json!({"ok": true, "updated": updated})
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
        "file.import" => {
            let paths = params["paths"].as_array().cloned().unwrap_or_default();
            let items = import_media_paths(doc, &paths);
            json!({"ok": true, "items": items})
        }
        "timeline.place" => {
            let clips = place_media_clip(doc, params);
            json!({"ok": true, "clips": clips})
        }
        "sequence.inspect" => doc["sequence"].clone(),
        "media.status" => {
            let statuses: Vec<Value> = doc["media"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|m| {
                    let exists = m["path"].as_str().is_some_and(|p| Path::new(p).exists());
                    json!({
                        "item": m["item"],
                        "path": m["path"],
                        "name": m["name"],
                        "status": if exists { "online" } else { "offline" },
                    })
                })
                .collect();
            Value::Array(statuses)
        }
        "media.replaceFootage" => {
            let target = params["item"].as_u64();
            let new_path = params["path"].as_str().unwrap_or_default();
            if let Some(arr) = doc["media"].as_array_mut() {
                for m in arr.iter_mut() {
                    if m["item"].as_u64() == target {
                        m["path"] = json!(new_path);
                    }
                }
            }
            for section in ["video", "audio"] {
                if let Some(tracks) = doc["sequence"][section].as_array_mut() {
                    for trk in tracks.iter_mut() {
                        if let Some(items) = trk["items"].as_array_mut() {
                            for c in items.iter_mut() {
                                if c["item"].as_u64() == target {
                                    c["source_path"] = json!(new_path);
                                }
                            }
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
            let tf = params["track"].as_str();
            let splits = razor_split_at(doc, tick, tf);
            json!({"ok": true, "splits": splits, "tick": tick})
        }
        "source.markIn" => {
            doc["sequence"]["in_tick"] = json!(sec_to_ticks(params["seconds"].as_f64().unwrap_or(0.0)));
            json!({"ok": true, "in_tick": doc["sequence"]["in_tick"]})
        }
        "source.markOut" => {
            doc["sequence"]["out_tick"] = json!(sec_to_ticks(params["seconds"].as_f64().unwrap_or(5.0)));
            json!({"ok": true, "out_tick": doc["sequence"]["out_tick"]})
        }
        "sequence.lift" => {
            lift_or_extract_range(doc, false);
            json!({"ok": true})
        }
        "sequence.extract" => {
            lift_or_extract_range(doc, params["ripple"].as_bool().unwrap_or(true));
            json!({"ok": true})
        }
        "timeline.insert" | "timeline.overwrite" => {
            let start_tick = doc["sequence"]["in_tick"].as_i64().unwrap_or(0);
            let end_tick = doc["sequence"]["out_tick"].as_i64().unwrap_or(start_tick + sec_to_ticks(3.0));
            let dur_sec = ((end_tick - start_tick).max(sec_to_ticks(0.5)) as f64) / (TICKS_PER_SECOND as f64);
            let clips = place_media_clip(
                doc,
                &json!({
                    "track": params["track"].as_str().unwrap_or("V1"),
                    "start_tick": start_tick,
                    "duration_sec": dur_sec,
                    "insert": id == "timeline.insert",
                    "name": params["name"].as_str().unwrap_or("Edited Clip"),
                }),
            );
            json!({"ok": true, "clips": clips})
        }
        "trim.nudge" => {
            let fps = doc["sequence"]["fps"].as_f64().unwrap_or(DEFAULT_FPS);
            let delta = frame_to_ticks(params["frames"].as_i64().unwrap_or(1), fps) * if params["frames"].as_i64().unwrap_or(1) < 0 { -1 } else { 1 };
            let target = params["clip"].as_u64();
            if let Some(vtracks) = doc["sequence"]["video"].as_array_mut() {
                for trk in vtracks.iter_mut() {
                    if let Some(items) = trk["items"].as_array_mut()
                        && let Some(c) = items.iter_mut().find(|c| target.is_none_or(|cid| c["clip"].as_u64() == Some(cid)))
                    {
                        let st = c["start"].as_i64().unwrap_or(0);
                        let en = (c["end"].as_i64().unwrap_or(st) + delta).max(st + sec_to_ticks(0.15));
                        c["end"] = json!(en);
                        break;
                    }
                }
            }
            recompute_sequence_duration(doc);
            json!({"ok": true})
        }
        "trim.extendToPlayhead" => {
            let playhead = sec_to_ticks(params["seconds"].as_f64().unwrap_or(0.0));
            let side = params["side"].as_str().unwrap_or("prev");
            if let Some(vtracks) = doc["sequence"]["video"].as_array_mut() {
                for trk in vtracks.iter_mut() {
                    if trk["patched"].as_bool() == Some(false) {
                        continue;
                    }
                    if let Some(items) = trk["items"].as_array_mut() {
                        for c in items.iter_mut() {
                            let st = c["start"].as_i64().unwrap_or(0);
                            let en = c["end"].as_i64().unwrap_or(0);
                            if side == "prev" && st < playhead && en > st && (en - playhead).abs() < sec_to_ticks(8.0) {
                                c["end"] = json!(playhead.max(st + sec_to_ticks(0.15)));
                                break;
                            } else if side == "next" && en > playhead && (playhead - st).abs() < sec_to_ticks(8.0) {
                                c["start"] = json!(playhead.min(en - sec_to_ticks(0.15)));
                                break;
                            }
                        }
                    }
                }
            }
            recompute_sequence_duration(doc);
            json!({"ok": true})
        }
        "clip.rename" | "clip.trim" | "clip.move" | "clip.delete" | "clip.rateStretch" | "clip.setBlendMode" | "clip.motion" => {
            apply_direct_clip_edit(doc, id, params)
        }
        "graphics.newText" | "graphics.newVerticalText" | "graphics.newShape" => {
            let start_sec = params["at_seconds"].as_f64().unwrap_or(0.0);
            let dur_sec = params["seconds"].as_f64().unwrap_or(5.0).max(0.5);
            let label = params["text"].as_str().or_else(|| params["shape"].as_str()).unwrap_or("Title Graphic");
            let clip_id = alloc_id(doc, "next_clip_id", 100);
            let start_tick = sec_to_ticks(start_sec);
            let end_tick = start_tick + sec_to_ticks(dur_sec);
            if let Some(v4) = doc["sequence"]["video"].get_mut(3).and_then(|t| t["items"].as_array_mut()) {
                v4.push(json!({
                    "clip": clip_id,
                    "id": clip_id,
                    "item": Value::Null,
                    "name": label,
                    "start": start_tick,
                    "end": end_tick,
                    "enabled": true,
                    "graphic": params,
                }));
                v4.sort_by_key(|c| c["start"].as_i64().unwrap_or(0));
            }
            recompute_sequence_duration(doc);
            json!({"ok": true, "clip": clip_id})
        }
        "sequence.colorSettings" => {
            for (k, v) in params.as_object().into_iter().flatten() {
                doc["sequence"]["color"][k] = v.clone();
            }
            json!({"ok": true})
        }
        "lumetri.setLook" => {
            doc["sequence"]["color"]["look"] = params["lut"].clone();
            json!({"ok": true})
        }
        "effects.setParam" | "effects.apply" => {
            if params["effect"].as_str() == Some("lumetri")
                && let Some(param) = params["param"].as_str()
            {
                doc["sequence"]["color"][param] = params["value"].clone();
            }
            json!({"ok": true})
        }
        "mixer.setStrip" => {
            let strip = params["strip"].as_str().unwrap_or("A1");
            for k in ["volume", "pan", "mute", "solo", "mode"] {
                if let Some(v) = params.get(k) {
                    doc["sequence"]["mixer"][strip][k] = v.clone();
                }
            }
            json!({"ok": true})
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
            if let Some(arr) = doc["sequence"]["markers"].as_array_mut() {
                for m in arr.iter_mut() {
                    if m["id"].as_u64() == target {
                        for k in ["name", "comment", "author", "color", "kind", "url"] {
                            if let Some(v) = params.get(k) {
                                m[k] = v.clone();
                            }
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
