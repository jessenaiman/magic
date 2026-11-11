//! Synced Timeline Markers, Cue Points, 64-Swatch Custom Palette, Marker CSV Export, &
//! Bidirectional Timeline Interchange (`T03`, `T35`) across FilmCraft (`film/*.fcproj`) and
//! EffectCraft (`lettering/lyrics.ecproj`, `motion/actions.ecproj`, or custom `.ecproj`).

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::time::{TICKS_PER_SECOND, Tick, parse_raw_tick, parse_tick};
use crate::{CraftHub, CraftTarget, EngineError, Result, Session};

use super::{bad, execute, film, run_steps};

fn map_fc_color_to_ec_label(color: &str) -> &'static str {
    match color.trim().to_ascii_lowercase().as_str() {
        "violet" | "purple" | "lavender" => "Lavender",
        "iris" | "blue" | "cerulean" => "Blue",
        "caribbean" | "teal" | "aqua" | "sea foam" => "Aqua",
        "forest" | "green" => "Green",
        "mango" | "orange" | "peach" => "Orange",
        "yellow" | "tan" => "Yellow",
        "rose" | "magenta" | "pink" => "Pink",
        _ => "Red",
    }
}

fn map_ec_label_to_fc_color(label: &str) -> &'static str {
    match label.trim().to_ascii_lowercase().as_str() {
        "lavender" | "purple" => "Lavender",
        "blue" | "cyan" => "Iris",
        "aqua" | "sea foam" => "Caribbean",
        "green" => "Forest",
        "orange" | "peach" => "Mango",
        "yellow" => "Yellow",
        "pink" | "fuchsia" => "Rose",
        _ => "Iris",
    }
}

pub(crate) fn extract_synced_markers(dir: &Path, seq: Option<&Value>) -> Vec<Value> {
    let mut out: Vec<Value> = Vec::new();

    if let Some(seq_val) = seq
        && let Some(arr) = seq_val["markers"].as_array()
    {
        for (idx, m) in arr.iter().enumerate() {
            let start_tick = parse_raw_tick(&m["start"]).map(|t| t.0).unwrap_or(0);
            let duration_tick = parse_raw_tick(&m["duration"]).map(|t| t.0).unwrap_or(0);
            let time_sec = (start_tick as f64) / (TICKS_PER_SECOND as f64);
            let duration_sec = (duration_tick as f64) / (TICKS_PER_SECOND as f64);
            let name = m["name"].as_str().unwrap_or_default();
            let comment = m["comment"].as_str().unwrap_or(name);
            let color = m["color"].as_str().unwrap_or("Iris");
            let kind = m["kind"].as_str().unwrap_or(if duration_tick > 0 { "Range" } else { "FlashCue" });
            out.push(json!({
                "id": m["id"],
                "index": idx,
                "name": if name.is_empty() { comment } else { name },
                "comment": comment,
                "author": m.get("author").cloned().unwrap_or(Value::Null),
                "url": m.get("url").cloned().unwrap_or(Value::Null),
                "kind": kind,
                "color": color,
                "start_tick": start_tick,
                "duration_tick": duration_tick,
                "time_sec": time_sec,
                "duration_sec": duration_sec,
                "source": "filmcraft",
            }));
        }
    }

    let ecproj = dir.join("lettering/lyrics.ecproj");
    if ecproj.is_file()
        && let Ok(ec_res) = CraftHub::run_effect_steps(Some(&ecproj), false, &[("markers.list".to_string(), json!({}))])
        && let Some(ec_arr) = ec_res.first().and_then(Value::as_array)
    {
        for m in ec_arr {
            let time_sec = m["time"].as_f64().unwrap_or(0.0);
            let duration_sec = m["duration"].as_f64().unwrap_or(0.0);
            let start_tick = (time_sec * (TICKS_PER_SECOND as f64)).round() as i64;
            let duration_tick = (duration_sec * (TICKS_PER_SECOND as f64)).round() as i64;
            let cue_name = m.pointer("/cuePoint/name").and_then(Value::as_str).unwrap_or_default();
            let comment = m["comment"].as_str().unwrap_or_default();
            let name = if !cue_name.is_empty() { cue_name } else { comment };
            let label = m["label"].as_str().unwrap_or("Red");
            let fc_color = map_ec_label_to_fc_color(label);

            if let Some(existing) = out.iter_mut().find(|row| {
                let dt = (row["time_sec"].as_f64().unwrap_or(0.0) - time_sec).abs();
                dt < 0.05 && (row["name"].as_str() == Some(name) || row["comment"].as_str() == Some(comment))
            }) {
                existing["source"] = json!("synced");
                existing["ec_index"] = m["index"].clone();
                existing["ec_label"] = json!(label);
            } else {
                out.push(json!({
                    "id": Value::Null,
                    "ec_index": m["index"],
                    "index": out.len(),
                    "name": name,
                    "comment": comment,
                    "kind": if duration_sec > 0.0 { "Range" } else { "CuePoint" },
                    "color": fc_color,
                    "ec_label": label,
                    "start_tick": start_tick,
                    "duration_tick": duration_tick,
                    "time_sec": time_sec,
                    "duration_sec": duration_sec,
                    "source": "effectcraft",
                }));
            }
        }
    }

    out.sort_by_key(|m| m["start_tick"].as_i64().unwrap_or(0));
    out
}

fn markers_add(f: &super::Film, ecproj: &Path, p: &Value) -> Result<Value> {
    let start_tick = p.get("tick").or_else(|| p.get("time")).or_else(|| p.get("seconds")).and_then(parse_tick).unwrap_or(Tick::ZERO).max(Tick::ZERO);
    let duration_tick =
        p.get("duration_tick").or_else(|| p.get("duration")).or_else(|| p.get("duration_sec")).and_then(parse_tick).unwrap_or(Tick::ZERO).max(Tick::ZERO);
    let time_sec = start_tick.seconds();
    let duration_sec = duration_tick.seconds();
    let name = p["name"].as_str().or_else(|| p["comment"].as_str()).unwrap_or("Cue").trim();
    let comment = p["comment"].as_str().unwrap_or(name).trim();
    let author = p["author"].as_str().map(str::trim).filter(|s| !s.is_empty());
    let url = p["url"].as_str().map(str::trim).filter(|s| !s.is_empty());
    let color = p["color"].as_str().unwrap_or("Iris");
    let ec_label = p["label"].as_str().unwrap_or_else(|| map_fc_color_to_ec_label(color));
    let kind = p["kind"].as_str().unwrap_or(if duration_tick.0 > 0 { "range" } else { "comment" });

    let mut fc_marker_id = Value::Null;
    if f.project.is_file() {
        let mut fc_params = json!({
            "time": start_tick.0,
            "name": name,
            "comment": comment,
            "color": color,
        });
        if let Some(a) = author {
            fc_params["author"] = json!(a);
        }
        if let Some(u) = url {
            fc_params["url"] = json!(u);
        }
        let cmd_name = if duration_tick.0 > 0 || kind.eq_ignore_ascii_case("range") {
            fc_params["duration"] = json!(duration_tick.0);
            "markers.addRange"
        } else if kind.eq_ignore_ascii_case("chapter") {
            "markers.addChapter"
        } else if kind.eq_ignore_ascii_case("flashcue") {
            "markers.addFlashCue"
        } else {
            "markers.add"
        };
        let fc_step = json!({"id": cmd_name, "params": fc_params});
        if let Ok(res) = run_steps(Some(&f.project), false, true, &[fc_step])
            && let Some(first) = res.first()
        {
            fc_marker_id = first.get("marker").cloned().unwrap_or(Value::Null);
            if matches!(kind, "segmentation" | "webLink")
                && let Some(mid) = fc_marker_id.as_u64()
            {
                let mut edit_p = json!({"marker": mid, "kind": kind});
                if let Some(u) = url {
                    edit_p["url"] = json!(u);
                }
                let _ = run_steps(Some(&f.project), false, true, &[json!({"id": "markers.edit", "params": edit_p})]);
            }
        }
    }

    if ecproj.is_file() {
        let ec_params = json!({
            "new": true,
            "time": time_sec,
            "duration": duration_sec,
            "comment": comment,
            "chapter": if kind.eq_ignore_ascii_case("chapter") { name } else { "" },
            "url": url.unwrap_or(""),
            "label": ec_label,
            "cuePoint": {
                "name": name,
                "navigation": true,
                "params": [
                    ["color", color],
                    ["kind", kind]
                ]
            }
        });
        let _ = CraftHub::run_effect_steps(Some(ecproj), true, &[("markers.set".to_string(), ec_params)]);
    }

    let seq = if f.project.is_file() { execute(&f.project, "sequence.inspect", json!({})).ok() } else { None };
    let markers = extract_synced_markers(&f.dir, seq.as_ref());
    Ok(json!({
        "ok": true,
        "marker_id": fc_marker_id,
        "name": name,
        "kind": kind,
        "time_sec": time_sec,
        "duration_sec": duration_sec,
        "markers": markers,
    }))
}

fn markers_edit_or_palette(f: &super::Film, action: &str, p: &Value) -> Result<Value> {
    let project = f.require()?.to_path_buf();
    let swatch = p["custom_swatch"].as_str().or_else(|| (action == "palette").then(|| p["color"].as_str()).flatten()).filter(|s| !s.trim().is_empty());
    if let Some(sw) = swatch {
        let res = CraftHub::execute_and_save(CraftTarget::Film, &project, "colors.addCustom", &json!({"color": sw.trim()}))?;
        return Ok(json!({"ok": true, "swatch": sw.trim(), "result": res}));
    }
    let marker_id = p["id"].as_u64().or_else(|| p["marker"].as_u64()).ok_or_else(|| bad("marker id is required for markers.edit"))?;
    let mut edit_params = json!({"marker": marker_id});
    for key in ["name", "comment", "author", "color", "kind", "url"] {
        if let Some(val) = p.get(key) {
            edit_params[key] = val.clone();
        }
    }
    let res = CraftHub::execute_and_save(CraftTarget::Film, &project, "markers.edit", &edit_params)?;
    Ok(json!({"ok": true, "marker": marker_id, "result": res}))
}

fn markers_export_csv(f: &super::Film, p: &Value) -> Result<Value> {
    let project = f.require()?.to_path_buf();
    let out = p["path"]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .map(|raw| {
            let pb = PathBuf::from(raw);
            if pb.is_absolute() { pb } else { f.dir.join(pb) }
        })
        .unwrap_or_else(|| f.dir.join("exports").join(format!("{}-markers.csv", f.name)));
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let result = CraftHub::execute(CraftTarget::Film, Some(&project), "markers.exportCsv", &json!({"path": out}))?;
    Ok(json!({
        "ok": true,
        "csv_path": out,
        "timeline": project,
        "result": result,
    }))
}

fn markers_seed(f: &super::Film, ecproj: &Path, p: &Value) -> Result<Value> {
    let words_path = f.dir.join("design/words.json");
    let doc: Value = if words_path.is_file() {
        let bytes = std::fs::read(&words_path)?;
        serde_json::from_slice(&bytes).map_err(|e| bad(e.to_string()))?
    } else {
        json!({})
    };
    let animations = doc["animations"].as_array().cloned().unwrap_or_default();

    let clear = p["clear"].as_bool().unwrap_or(true);
    let mut fc_steps: Vec<Value> = Vec::new();
    let mut ec_steps: Vec<(String, Value)> = Vec::new();

    if clear {
        fc_steps.push(json!({"id": "markers.clearAll", "params": {"target": "program"}}));
        ec_steps.push(("comp.deleteAllMarkers".to_string(), json!({})));
    }

    let mut sections: Vec<(String, i64, i64)> = Vec::new();
    let mut lines: Vec<(String, String, i64)> = Vec::new();
    for (idx, anim) in animations.iter().enumerate() {
        let sec_name = anim["scope"]["section"].as_str().or_else(|| anim["animation_id"].as_str()).unwrap_or("section").to_string();
        let anchor_tick =
            anim["song_anchor_tick"].as_i64().or_else(|| anim["song_anchor"].as_f64().map(|s| (s * (TICKS_PER_SECOND as f64)).round() as i64)).unwrap_or(0);
        let dur_tick = anim["authored_duration_tick"]
            .as_i64()
            .or_else(|| anim["authored_duration"].as_f64().map(|s| (s * (TICKS_PER_SECOND as f64)).round() as i64))
            .unwrap_or(0);
        let sec_key = format!("S{:02} · {}", idx + 1, sec_name);
        sections.push((sec_key, anchor_tick, anchor_tick + dur_tick.max(0)));

        let mut current_line_id = String::new();
        let mut current_words: Vec<String> = Vec::new();
        let mut current_start_tick: Option<i64> = None;
        for w in anim["words"].as_array().into_iter().flatten() {
            let lid = w["line_id"].as_str().unwrap_or("line");
            let wt = w["display_text"].as_str().or_else(|| w["text"].as_str()).unwrap_or("").trim();
            let ws_tick = w["start_tick"]
                .as_i64()
                .or_else(|| w["start"].as_f64().map(|s| ((anim["song_anchor"].as_f64().unwrap_or(0.0) + s) * (TICKS_PER_SECOND as f64)).round() as i64));
            if lid != current_line_id {
                if !current_line_id.is_empty()
                    && let Some(st) = current_start_tick
                {
                    lines.push((current_line_id.clone(), current_words.join(" "), st));
                }
                current_line_id = lid.to_string();
                current_words.clear();
                current_start_tick = ws_tick;
            }
            if current_start_tick.is_none() {
                current_start_tick = ws_tick;
            }
            if !wt.is_empty() {
                current_words.push(wt.to_string());
            }
        }
        if !current_line_id.is_empty()
            && let Some(st) = current_start_tick
        {
            lines.push((current_line_id, current_words.join(" "), st));
        }
    }

    for (sec_name, s_tick, e_tick) in &sections {
        let dur_tick = (e_tick - s_tick).max(0);
        let is_chorus = sec_name.to_ascii_lowercase().contains("chorus");
        let color = if is_chorus { "Mango" } else { "Caribbean" };
        let ec_label = map_fc_color_to_ec_label(color);
        let label_upper = sec_name.replace('_', " ").to_ascii_uppercase();
        fc_steps.push(json!({
            "id": "markers.addRange",
            "params": {
                "time": s_tick,
                "duration": dur_tick,
                "name": sec_name,
                "comment": label_upper,
                "color": color,
            }
        }));
        ec_steps.push((
            "markers.set".to_string(),
            json!({
                "new": true,
                "time": (*s_tick as f64) / (TICKS_PER_SECOND as f64),
                "duration": (dur_tick as f64) / (TICKS_PER_SECOND as f64),
                "comment": label_upper,
                "chapter": sec_name,
                "label": ec_label,
                "cuePoint": {
                    "name": sec_name,
                    "navigation": true,
                    "params": [["role", "section"], ["color", color]]
                }
            }),
        ));
    }

    for (line_id, text, s_tick) in &lines {
        fc_steps.push(json!({
            "id": "markers.addFlashCue",
            "params": {
                "time": s_tick,
                "name": line_id,
                "comment": text,
                "color": "Iris",
            }
        }));
        ec_steps.push((
            "markers.set".to_string(),
            json!({
                "new": true,
                "time": (*s_tick as f64) / (TICKS_PER_SECOND as f64),
                "duration": 0.0,
                "comment": text,
                "label": "Blue",
                "cuePoint": {
                    "name": line_id,
                    "navigation": false,
                    "params": [["role", "lyric_line"]]
                }
            }),
        ));
    }

    if f.project.is_file() && !fc_steps.is_empty() {
        let _ = run_steps(Some(&f.project), false, true, &fc_steps);
    }
    if ecproj.is_file() && !ec_steps.is_empty() {
        let _ = CraftHub::run_effect_steps(Some(ecproj), true, &ec_steps);
    }

    let seq = if f.project.is_file() { execute(&f.project, "sequence.inspect", json!({})).ok() } else { None };
    let markers = extract_synced_markers(&f.dir, seq.as_ref());
    Ok(json!({
        "ok": true,
        "seeded_sections": sections.len(),
        "seeded_lines": lines.len(),
        "markers": markers,
    }))
}

fn markers_delete(f: &super::Film, ecproj: &Path, action: &str, p: &Value) -> Result<Value> {
    if action == "clear" || p["all"].as_bool() == Some(true) {
        if f.project.is_file() {
            let _ = run_steps(Some(&f.project), false, true, &[json!({"id": "markers.clearAll", "params": {"target": "program"}})]);
        }
        if ecproj.is_file() {
            let _ = CraftHub::run_effect_steps(Some(ecproj), true, &[("comp.deleteAllMarkers".to_string(), json!({}))]);
        }
    } else {
        if let Some(marker_id) = p.get("id").and_then(Value::as_u64)
            && f.project.is_file()
        {
            let _ = run_steps(Some(&f.project), false, true, &[json!({"id": "markers.delete", "params": {"target": "program", "marker": marker_id}})]);
        }
        if ecproj.is_file() {
            if let Some(ec_idx) = p.get("ec_index").or_else(|| p.get("index")).and_then(Value::as_u64) {
                let _ = CraftHub::run_effect_steps(Some(ecproj), true, &[("markers.delete".to_string(), json!({"index": ec_idx}))]);
            } else if let Some(at_sec) = p.get("time_sec").or_else(|| p.get("seconds")).and_then(Value::as_f64) {
                let _ = CraftHub::run_effect_steps(Some(ecproj), true, &[("markers.delete".to_string(), json!({"at": at_sec}))]);
            }
        }
    }
    let seq = if f.project.is_file() { execute(&f.project, "sequence.inspect", json!({})).ok() } else { None };
    let markers = extract_synced_markers(&f.dir, seq.as_ref());
    Ok(json!({"ok": true, "markers": markers}))
}

fn markers_interchange(f: &super::Film, default_ecproj: &Path, p: &Value) -> Result<Value> {
    let format = p["format"].as_str().unwrap_or("otio");
    let raw_dir = p["direction"].as_str().unwrap_or("film_to_effect");
    let direction = match raw_dir {
        "import" | "effect_to_film" => "effect_to_film",
        _ => "film_to_effect",
    };
    let prerender = p["prerender"].as_str().unwrap_or_else(|| if p["prerender"].as_bool() == Some(false) { "none" } else { "unsupported" });
    let ext = match format {
        "fcpxml" => "fcpxml",
        "otio" => "otio",
        "edl" => "edl",
        "aaf" => "aaf",
        "omf" => "omf",
        "ale" => "ale",
        _ => "xml",
    };
    let ecproj = p["ecproj"]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .map(|raw| {
            let pb = PathBuf::from(raw);
            if pb.is_absolute() { pb } else { f.dir.join(pb) }
        })
        .unwrap_or_else(|| default_ecproj.to_path_buf());
    let interchange_path = p["path"]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .map(|raw| {
            let pb = PathBuf::from(raw);
            if pb.is_absolute() { pb } else { f.dir.join(pb) }
        })
        .unwrap_or_else(|| f.dir.join("film/interchange").join(format!("{}-interchange.{ext}", f.name)));
    if let Some(parent) = interchange_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    if direction == "effect_to_film" {
        if !ecproj.is_file() {
            return Err(bad(format!("No EffectCraft project found at {}", ecproj.display())));
        }
        let mut export_p = json!({
            "path": interchange_path,
            "format": format,
            "prerender": prerender,
        });
        if let Some(comp) = p["comp"].as_str().filter(|s| !s.trim().is_empty()) {
            export_p["comp"] = json!(comp);
        }
        let exported = CraftHub::run_effect_steps(Some(&ecproj), false, &[("file.exportTimeline".to_string(), export_p)])?;
        let mut imported_into_filmcraft = Value::Null;
        if f.project.is_file()
            && p["import_target"].as_bool().unwrap_or(true)
            && let Ok(res) = CraftHub::execute_and_save(CraftTarget::Film, &f.project, "file.import", &json!({"paths": [interchange_path]}))
        {
            imported_into_filmcraft = res;
        }
        Ok(json!({
            "ok": true,
            "direction": direction,
            "format": format,
            "prerender": prerender,
            "path": interchange_path,
            "exported": exported.first(),
            "imported_into_filmcraft": imported_into_filmcraft,
        }))
    } else {
        let project = f.require()?.to_path_buf();
        let exported = CraftHub::execute(CraftTarget::Film, Some(&project), "file.exportInterchange", &json!({"format": format, "path": interchange_path}))?;
        let mut imported = Value::Null;
        if ecproj.is_file()
            && format != "ale"
            && p["import_target"].as_bool().unwrap_or(true)
            && let Ok(res) =
                CraftHub::run_effect_steps(Some(&ecproj), true, &[("file.importTimeline".to_string(), json!({"path": interchange_path, "format": format}))])
        {
            imported = res.into_iter().next().unwrap_or(Value::Null);
        }
        Ok(json!({
            "ok": true,
            "direction": direction,
            "format": format,
            "path": interchange_path,
            "exported": exported,
            "imported_into_effectcraft": imported,
        }))
    }
}

pub fn cmd_video_markers(s: &mut Session, p: &Value) -> Result<Value> {
    let f = film(s)?;
    let ecproj = f.dir.join("lettering/lyrics.ecproj");
    let has_time_or_name = p.get("time").is_some() || p.get("seconds").is_some() || p.get("tick").is_some() || p.get("name").is_some();
    let action = p["action"].as_str().unwrap_or(if has_time_or_name { "add" } else { "list" });

    match action {
        "list" => {
            let seq = if f.project.is_file() { execute(&f.project, "sequence.inspect", json!({})).ok() } else { None };
            let markers = extract_synced_markers(&f.dir, seq.as_ref());
            Ok(json!({
                "ok": true,
                "timeline": f.project,
                "ecproj": ecproj,
                "markers": markers,
                "interchange_formats": ["otio", "fcpxml", "xml", "edl", "aaf", "omf", "ale"],
            }))
        }
        "add" | "set" => markers_add(&f, &ecproj, p),
        "edit" | "palette" => markers_edit_or_palette(&f, action, p),
        "export_csv" | "csv" => markers_export_csv(&f, p),
        "seed" | "seed_from_lyrics" => markers_seed(&f, &ecproj, p),
        "delete" | "clear" => markers_delete(&f, &ecproj, action, p),
        "interchange" => markers_interchange(&f, &ecproj, p),
        other => Err(EngineError::InvalidParams(format!(
            "unknown video.markers action `{other}`; expected list, add, edit, palette, export_csv, seed, delete, clear, or interchange"
        ))),
    }
}
