//! `motion`: Song-Action Drawing (`motion/actions.json`, `motion/svg/*.svg`, `motion/actions.ecproj`).
//!
//! Separated from Kinetic Lyrics (`cmd::words`) under Single Responsibility Principle:
//! - **Kinetic Lyrics (`words`)** authors `lettering/lyrics.ecproj` -> exports `V3` `"Kinetic Lyrics"` into `FilmCraft`.
//! - **Motion (`motion`)** binds drawn vector action illustrations (`.svg` / `.vectorcraft`) to sung action
//!   words, traces artwork via `VectorCraft` (`imageTrace.make` with `ignoreWhite: true`), authors
//!   `motion/actions.ecproj` via `EffectCraft` (`edit.pasteSvg` + `layer.addShapeItem {"kind": "trim"}`),
//!   renders preview frames (`motion/.preview/motion-*.png`), and exports `V4` `"Action Motion"` into `FilmCraft`.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::time::{TICKS_PER_SECOND, Tick, parse_tick};
use crate::{CraftHub, CraftTarget, EngineError, Result, Session};

const COMP_NAME: &str = "SongActions";
const STAGE_W: f64 = 1920.0;
const STAGE_H: f64 = 1080.0;

fn bad(msg: impl Into<String>) -> EngineError {
    EngineError::InvalidParams(msg.into())
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub(crate) struct MotionActionsDoc(pub Value);

impl crate::doc_state::ProjectDoc for MotionActionsDoc {
    const KIND: &'static str = "motion_actions";
    fn path(dir: &Path) -> PathBuf {
        dir.join("motion/actions.json")
    }
}

pub fn action_sketch_presets() -> Vec<Value> {
    vec![
        json!({
            "id": "whale",
            "name": "Whale",
            "d": "M 520 620 C 520 430 780 350 1040 430 C 1210 480 1320 560 1420 500 L 1510 420 L 1480 540 L 1540 640 L 1410 580 C 1270 670 760 690 520 620 M 690 520 A 14 14 0 1 1 691 520 M 820 370 Q 800 260 730 230 M 835 370 Q 875 255 945 235"
        }),
        json!({
            "id": "wave",
            "name": "Wave",
            "d": "M 360 640 Q 520 480 680 640 T 1000 640 T 1320 640 T 1560 640"
        }),
        json!({
            "id": "star",
            "name": "Star",
            "d": "M 960 300 L 1025 465 L 1200 480 L 1065 595 L 1105 765 L 960 670 L 815 765 L 855 595 L 720 480 L 895 465 Z"
        }),
        json!({
            "id": "boat",
            "name": "Boat",
            "d": "M 640 650 L 1280 650 L 1160 770 L 760 770 Z M 960 650 L 960 330 L 1210 590 L 960 590"
        }),
        json!({
            "id": "moon",
            "name": "Moon",
            "d": "M 1040 320 C 850 340 760 520 850 690 C 920 820 1090 840 1200 760 C 1030 760 940 620 970 470 C 985 395 1010 350 1040 320"
        }),
        json!({
            "id": "jump",
            "name": "Jump",
            "d": "M 580 720 Q 960 240 1340 700 M 1280 680 L 1350 705 L 1315 630"
        }),
    ]
}

fn load_doc(s: &mut Session, dir: &Path) -> Result<Value> {
    Ok(s.doc::<MotionActionsDoc>(dir)?.map(|d| d.0).unwrap_or_else(|| json!({ "format": "magic.motion.actions", "version": 1, "cues": [] })))
}

fn save_doc(s: &mut Session, dir: &Path, doc: &Value) -> Result<()> {
    s.save_doc(dir, &MotionActionsDoc(doc.clone()))
}

fn collect_motion_craft_files(dir: &Path) -> (Vec<PathBuf>, Vec<PathBuf>, Vec<(u64, PathBuf)>) {
    let mut svgs: Vec<PathBuf> = fs::read_dir(dir.join("motion/svg"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|x| x.eq_ignore_ascii_case("svg")))
        .collect();
    svgs.sort();
    let mut vcrafts: Vec<PathBuf> = [dir.join("motion"), dir.join("motion/svg")]
        .into_iter()
        .flat_map(|d| fs::read_dir(d).into_iter().flatten().flatten())
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|x| x.eq_ignore_ascii_case("vectorcraft")))
        .collect();
    vcrafts.sort();
    let mut frames = Vec::new();
    for entry in fs::read_dir(dir.join("motion/.preview")).into_iter().flatten().flatten() {
        let p = entry.path();
        if p.extension().is_some_and(|x| x == "png")
            && let Some(ms_str) = p.file_stem().and_then(|s| s.to_str()).and_then(|s| s.strip_prefix("motion-"))
            && let Ok(ms) = ms_str.parse::<u64>()
        {
            frames.push((ms, p));
        }
    }
    frames.sort_by_key(|(ms, _)| *ms);
    (svgs, vcrafts, frames)
}

fn timed_song_words(dir: &Path) -> Vec<Value> {
    if let Ok(bytes) = fs::read(dir.join("timing-reference.json"))
        && let Ok(tref) = serde_json::from_slice::<Value>(&bytes)
        && let Some(words) = tref["words"].as_array()
        && !words.is_empty()
    {
        return words
            .iter()
            .enumerate()
            .map(|(i, w)| {
                let st = w["start_tick"].as_i64().map(Tick).unwrap_or(Tick::ZERO);
                let en = w["end_tick"].as_i64().map(Tick).unwrap_or(st + Tick(TICKS_PER_SECOND / 2));
                json!({
                    "word_id": w["word_id"].as_str().map(str::to_owned).unwrap_or_else(|| format!("t/w{i}")),
                    "line_id": format!("line-{}", i / 7 + 1),
                    "section": "Song",
                    "text": w["text"].as_str().unwrap_or_default().trim(),
                    "start_tick": st.0,
                    "end_tick": en.0,
                    "start": st.seconds(),
                    "end": en.seconds(),
                })
            })
            .collect();
    }
    if let Ok(aligned) = crate::cmd::words::aligned_in(dir)
        && let Some(words) = aligned["words"].as_array()
    {
        return words.clone();
    }
    let Some(path) = crate::workshop::transcript_files(dir).into_iter().next() else {
        return Vec::new();
    };
    let Ok(bytes) = fs::read(path) else { return Vec::new() };
    let Ok(doc) = serde_json::from_slice::<Value>(&bytes) else { return Vec::new() };
    let raw_words = doc
        .as_array()
        .or_else(|| doc["words"].as_array())
        .cloned()
        .unwrap_or_else(|| doc["segments"].as_array().into_iter().flatten().flat_map(|seg| seg["words"].as_array().cloned().unwrap_or_default()).collect());
    raw_words
        .into_iter()
        .enumerate()
        .map(|(i, w)| {
            let start = w.get("start_tick").or_else(|| w.get("start")).and_then(parse_tick).unwrap_or(Tick::ZERO);
            let end = w.get("end_tick").or_else(|| w.get("end")).and_then(parse_tick).unwrap_or(start + Tick(TICKS_PER_SECOND / 2));
            let text = w["text"].as_str().or_else(|| w["word"].as_str()).unwrap_or_default().trim();
            json!({
                "word_id": format!("t/w{i}"),
                "line_id": format!("line-{}", i / 7 + 1),
                "section": "Song",
                "text": text,
                "start_tick": start.0,
                "end_tick": end.0,
                "start": start.seconds(),
                "end": end.seconds(),
            })
        })
        .collect()
}

fn format_clock(sec: f64) -> String {
    if !sec.is_finite() || sec < 0.0 {
        return "0:00.0".into();
    }
    let m = (sec / 60.0).floor() as u64;
    let rem = sec - (m as f64) * 60.0;
    format!("{m}:{rem:04.1}")
}

fn sync_actions_ecproj(dir: &Path, cues: &[Value], duration_sec: f64) -> Result<PathBuf> {
    let ecproj = dir.join("motion/actions.ecproj");
    if let Some(parent) = ecproj.parent() {
        fs::create_dir_all(parent)?;
    }
    let svg_dir = dir.join("motion/svg");
    fs::create_dir_all(&svg_dir)?;

    let mut steps: Vec<(String, Value)> = vec![(
        "comp.new".into(),
        json!({
            "name": COMP_NAME,
            "width": STAGE_W as u32,
            "height": STAGE_H as u32,
            "frameRate": 30.0,
            "duration": duration_sec.max(10.0),
        }),
    )];

    for (idx, cue) in cues.iter().enumerate() {
        let cue_id = cue["id"].as_str().unwrap_or("action");
        let layer_name = format!("action_{idx}_{cue_id}");
        let color = cue["color"].as_str().unwrap_or("#64CBFF");
        let width = cue["stroke_width"].as_f64().unwrap_or(8.0).clamp(1.0, 48.0);
        let start = cue["start"].as_f64().unwrap_or(0.0).max(0.0);
        let draw_dur = cue["draw_duration"].as_f64().unwrap_or(0.85).clamp(0.1, 10.0);
        let hold_dur = cue["hold_duration"].as_f64().unwrap_or(1.2).clamp(0.1, 15.0);
        let draw_end = start + draw_dur;
        let gone = draw_end + hold_dur;

        let svg = if let Some(svg_file) = cue["svg_path"].as_str().map(Path::new).filter(|p| p.is_file()) {
            fs::read_to_string(svg_file).unwrap_or_default()
        } else {
            let d = cue["d"].as_str().unwrap_or_default().trim();
            if d.is_empty() {
                continue;
            }
            let raw = format!(
                "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {STAGE_W} {STAGE_H}\" width=\"{STAGE_W}\" height=\"{STAGE_H}\"><path d=\"{d}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"{width}\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/></svg>"
            );
            craft_atomic::atomic_write(&svg_dir.join(format!("{cue_id}.svg")), raw.as_bytes())?;
            raw
        };
        if svg.trim().is_empty() {
            continue;
        }

        steps.push(("edit.pasteSvg".into(), json!({ "svg": svg, "name": layer_name })));
        let px = cue["point_x"].as_f64().unwrap_or(STAGE_W * 0.5);
        let py = cue["point_y"].as_f64().unwrap_or(STAGE_H * 0.5);
        steps.push(("prop.set".into(), json!({ "layer": layer_name, "path": "transform/position", "value": [px, py, 0.0] })));
        if let Some(bend) = cue["arc_bend"].as_f64().filter(|b| b.abs() > 0.01) {
            steps.push(("prop.set".into(), json!({ "layer": layer_name, "path": "transform/rotation", "value": bend })));
        }
        if let Some(fx_id) = cue["effect_id"].as_str()
            && let Some(mapped) = crate::craft_effects::find_motion_effect(fx_id)
            && mapped.craft_app == "effectcraft"
            && mapped.craft_command == "effect.apply"
        {
            steps.push(("effect.apply".into(), json!({ "layer": layer_name, "effect": mapped.match_name })));
        }
        steps.push(("layer.addShapeItem".into(), json!({ "layer": layer_name, "kind": "trim" })));
        steps.push(("prop.addKey".into(), json!({ "layer": layer_name, "path": "contents/trim/end", "value": 0.0, "time": start, "timeBase": "comp" })));
        steps.push(("prop.addKey".into(), json!({ "layer": layer_name, "path": "contents/trim/end", "value": 100.0, "time": draw_end, "timeBase": "comp" })));
        steps.push(("keys.selectAll".into(), json!({})));
        steps.push(("keys.easyEase".into(), json!({})));
        steps.push(("layer.select".into(), json!({ "layers": [layer_name] })));
        steps.push(("time.set".into(), json!({ "time": start })));
        steps.push(("layer.timing".into(), json!({ "op": "trimInToTime" })));
        steps.push(("time.set".into(), json!({ "time": gone })));
        steps.push(("layer.timing".into(), json!({ "op": "trimOutToTime" })));
    }

    let _ = CraftHub::run_effect_steps(Some(&ecproj), true, &steps);
    Ok(ecproj)
}

pub fn cmd_motion_read(s: &mut Session, _p: &Value) -> Result<Value> {
    let dir = crate::workshop::project_dir(s)?;
    let doc = load_doc(s, &dir)?;
    let words = timed_song_words(&dir);
    let mut cues = doc["cues"].as_array().cloned().unwrap_or_default();
    let (svg_files, vectorcraft_files, preview_frames) = collect_motion_craft_files(&dir);
    for cue in &mut cues {
        let cue_id = cue["id"].as_str().unwrap_or("action");
        let svg_file = cue["svg_path"]
            .as_str()
            .map(PathBuf::from)
            .filter(|p| p.is_file())
            .or_else(|| {
                let candidate = dir.join("motion/svg").join(format!("{cue_id}.svg"));
                candidate.is_file().then_some(candidate)
            });
        cue["svg_file"] = json!(svg_file);
        let st_ms = (cue["start"].as_f64().unwrap_or(0.0).max(0.0) * 1000.0).round() as u64;
        let en_ms = (cue["end"].as_f64().unwrap_or(1.5).max(0.1) * 1000.0).round() as u64;
        let frame = preview_frames
            .iter()
            .find(|(ms, _)| *ms >= st_ms && *ms <= en_ms)
            .or_else(|| preview_frames.iter().min_by_key(|(ms, _)| ms.abs_diff(st_ms)))
            .map(|(_, p)| p.clone());
        cue["preview_frame"] = json!(frame);
    }
    let cue_words: std::collections::HashSet<&str> = cues.iter().filter_map(|c| c["word_id"].as_str()).collect();

    let mut lines: Vec<Value> = Vec::new();
    for w in &words {
        let lid = w["line_id"].as_str().unwrap_or("line-1");
        let wid = w["word_id"].as_str().unwrap_or_default();
        let mut word_chip = w.clone();
        word_chip["has_action"] = json!(cue_words.contains(wid));
        word_chip["clock"] = json!(format_clock(w["start"].as_f64().unwrap_or(0.0)));
        if lines.last().is_none_or(|l| l["line_id"].as_str() != Some(lid)) {
            let st = w["start"].as_f64().unwrap_or(0.0);
            lines.push(json!({
                "line_id": lid,
                "section": w["section"].as_str().unwrap_or("Song"),
                "start": st,
                "clock": format_clock(st),
                "words": [word_chip],
            }));
        } else if let Some(last) = lines.last_mut().and_then(|l| l["words"].as_array_mut()) {
            last.push(word_chip);
        }
    }

    let duration = words.last().and_then(|w| w["end"].as_f64()).unwrap_or(30.0).max(10.0) + 2.0;
    let ecproj = dir.join("motion/actions.ecproj");
    let mov = dir.join("motion/actions.mov");
    let backdrop = crate::cmd::words::stage::project_backdrop_images(s, &dir).into_iter().next().unwrap_or(Value::Null);
    let preview_paths: Vec<PathBuf> = preview_frames.into_iter().map(|(_, p)| p).collect();

    Ok(json!({
        "ok": true,
        "path": dir.join("motion/actions.json"),
        "ecproj": ecproj.is_file().then_some(ecproj),
        "render": mov.is_file().then_some(mov),
        "svg_files": svg_files,
        "vectorcraft_files": vectorcraft_files,
        "preview_frames": preview_paths,
        "anicraft_available": CraftHub::is_installed(CraftTarget::Ani),
        "duration": duration,
        "lines": lines,
        "cues": cues,
        "presets": action_sketch_presets(),
        "craft_effects": crate::craft_effects::motion_catalog_json(),
        "backdrop": backdrop,
    }))
}

pub fn cmd_motion_save(s: &mut Session, p: &Value) -> Result<Value> {
    let dir = crate::workshop::project_dir(s)?;
    let mut doc = load_doc(s, &dir)?;
    if !doc["cues"].is_array() {
        doc["cues"] = json!([]);
    }
    let words = timed_song_words(&dir);
    let duration = words.last().and_then(|w| w["end"].as_f64()).unwrap_or(30.0).max(10.0) + 4.0;

    if let Some(remove_id) = p.get("remove_id").and_then(Value::as_str) {
        if let Some(cues) = doc["cues"].as_array_mut() {
            cues.retain(|c| c["id"].as_str() != Some(remove_id));
        }
        save_doc(s, &dir, &doc)?;
        let cues = doc["cues"].as_array().cloned().unwrap_or_default();
        let ecproj = sync_actions_ecproj(&dir, &cues, duration)?;
        return Ok(json!({ "ok": true, "removed": remove_id, "ecproj": ecproj, "cues": cues }));
    }

    let word_id = p["word_id"].as_str().ok_or_else(|| bad("word_id is required to anchor an action drawing to a sung word"))?;
    let word_entry = words.iter().find(|w| w["word_id"].as_str() == Some(word_id));
    let word_text = p["word"].as_str().or_else(|| word_entry.and_then(|w| w["text"].as_str())).unwrap_or("Action");
    let start_tick = p
        .get("start_tick")
        .or_else(|| p.get("start"))
        .and_then(parse_tick)
        .or_else(|| word_entry.and_then(|w| w.get("start_tick").or_else(|| w.get("start"))).and_then(parse_tick))
        .unwrap_or(Tick::ZERO);

    let slug = crate::workspace::sanitize_asset_id(word_text);
    let cue_id =
        p["id"].as_str().map(str::to_owned).unwrap_or_else(|| format!("act-{}-{}", if slug.is_empty() { "word" } else { &slug }, word_id.replace('/', "-")));

    let mut traced_svg: Option<PathBuf> = None;
    let d = if let Some(img_raw) = p["image"].as_str().filter(|v| !v.trim().is_empty()) {
        let img = crate::workspace::resolve_workspace_path(s, img_raw);
        let svg_out = dir.join("motion/svg").join(format!("{cue_id}.svg"));
        let trace_preset = if p["effect_id"].as_str() == Some("vc.trace.silhouette") { "silhouettes" } else { "lineArt" };
        let use_strokes = trace_preset == "lineArt";
        CraftHub::trace_subject_vector(&img, &svg_out, Some(trace_preset), true, use_strokes)?;
        traced_svg = Some(svg_out);
        String::new()
    } else if let Some(preset_id) = p.get("preset").and_then(Value::as_str) {
        action_sketch_presets()
            .into_iter()
            .find(|pr| pr["id"].as_str() == Some(preset_id))
            .and_then(|pr| pr["d"].as_str().map(str::to_owned))
            .ok_or_else(|| bad(format!("unknown action preset `{preset_id}`")))?
    } else {
        p["d"].as_str().map(str::trim).filter(|s| !s.is_empty()).ok_or_else(|| bad("pass SVG path `d`, `preset`, or `image`"))?.to_owned()
    };

    let draw_dur = p["draw_duration"].as_f64().unwrap_or(0.85).clamp(0.1, 10.0);
    let hold_dur = p["hold_duration"].as_f64().unwrap_or(1.2).clamp(0.1, 15.0);
    let end_tick = start_tick + Tick::from_seconds_f64(draw_dur + hold_dur);
    let cue = json!({
        "id": cue_id,
        "word_id": word_id,
        "word": word_text,
        "title": p["title"].as_str().unwrap_or(word_text),
        "d": d,
        "svg_path": traced_svg,
        "color": p["color"].as_str().unwrap_or("#64CBFF"),
        "stroke_width": p["stroke_width"].as_f64().unwrap_or(8.0),
        "start_tick": start_tick.0,
        "end_tick": end_tick.0,
        "start": start_tick.seconds(),
        "end": end_tick.seconds(),
        "clock": format_clock(start_tick.seconds()),
        "draw_duration": draw_dur,
        "hold_duration": hold_dur,
        "effect_id": p["effect_id"].as_str().unwrap_or("ec.shape.trim_paths"),
        "point_x": p["point_x"].as_f64().unwrap_or(STAGE_W * 0.5),
        "point_y": p["point_y"].as_f64().unwrap_or(STAGE_H * 0.5),
        "arc_bend": p["arc_bend"].as_f64().unwrap_or(0.0),
        "taper": p["taper"].as_f64().unwrap_or(65.0),
        "craft_app": p["effect_id"]
            .as_str()
            .and_then(crate::craft_effects::find_motion_effect)
            .map(|m| m.craft_app)
            .or_else(|| p["craft_app"].as_str())
            .unwrap_or("vectorcraft"),
    });

    if let Some(cues) = doc["cues"].as_array_mut() {
        if let Some(existing) = cues.iter_mut().find(|c| c["id"] == cue["id"] || c["word_id"] == word_id) {
            *existing = cue.clone();
        } else {
            cues.push(cue.clone());
        }
        cues.sort_by_key(|c| c["start_tick"].as_i64().unwrap_or(0));
    }
    save_doc(s, &dir, &doc)?;
    let cues = doc["cues"].as_array().cloned().unwrap_or_default();
    let ecproj = sync_actions_ecproj(&dir, &cues, duration)?;

    let mut studio = None;
    if p["render"].as_bool() == Some(true) || p["studio"].as_bool() == Some(true) {
        let out_mov = dir.join("motion/actions.mov");
        CraftHub::render_effect_clip(
            &ecproj,
            &json!({ "comp": COMP_NAME, "output": out_mov, "format": "prores", "proresProfile": "4444", "channels": "rgba" }),
        )?;
        if p["studio"].as_bool() == Some(true) {
            studio = Some(crate::cmd::film::publish_motion(s, &out_mov)?);
        }
    }

    Ok(json!({ "ok": true, "cue": cue, "ecproj": ecproj, "cues": cues, "studio": studio }))
}

/// `motion.publish`: render `motion/actions.ecproj` via `EffectCraft` ProRes 4444 RGBA (if needed)
/// and publish it onto `FilmCraft`'s dedicated `V4` `"Action Motion"` track.
pub fn cmd_motion_publish(s: &mut Session, p: &Value) -> Result<Value> {
    let dir = crate::workshop::project_dir(s)?;
    let ecproj = dir.join("motion/actions.ecproj");
    if !ecproj.is_file() {
        let doc = load_doc(s, &dir)?;
        let cues = doc["cues"].as_array().cloned().unwrap_or_default();
        if cues.is_empty() {
            return Err(bad("No song-action motion cues yet; add a cue with motion.save first"));
        }
        let words = timed_song_words(&dir);
        let duration = words.last().and_then(|w| w["end"].as_f64()).unwrap_or(30.0).max(10.0) + 4.0;
        let _ = sync_actions_ecproj(&dir, &cues, duration)?;
    }
    let out_mov = p["path"].as_str().map(|raw| crate::workspace::resolve_workspace_path(s, raw)).unwrap_or_else(|| dir.join("motion/actions.mov"));
    if p["render"].as_bool().unwrap_or(!out_mov.is_file()) {
        CraftHub::render_effect_clip(
            &ecproj,
            &json!({ "comp": COMP_NAME, "output": out_mov, "format": "prores", "proresProfile": "4444", "channels": "rgba" }),
        )?;
    }
    let studio = crate::cmd::film::publish_motion(s, &out_mov)?;
    Ok(json!({ "ok": true, "ecproj": ecproj, "render": out_mov, "track": "Action Motion", "studio": studio }))
}

pub fn cmd_motion_open(s: &mut Session, p: &Value) -> Result<Value> {
    let dir = crate::workshop::project_dir(s)?;
    let app = p["app"].as_str().unwrap_or("vectorcraft");
    let target = CraftTarget::from_name(app).ok_or_else(|| bad(format!("unknown Craft app `{app}`")))?;
    if target == CraftTarget::Ani {
        return Err(bad(
            "AniCraft desktop UI is disabled for safety while in early alpha; use `anicraft.exec` for headless CLI steps and inspect live motion inside EffectCraft (`app=\"effectcraft\"`).",
        ));
    }
    let file = match target {
        CraftTarget::Effect => dir.join("motion/actions.ecproj"),
        CraftTarget::Ani => {
            let id = p["id"].as_str().unwrap_or("song-actions");
            dir.join("motion").join(format!("{id}.anicraft"))
        }
        _ => {
            let id = p["id"].as_str().unwrap_or("action");
            let svg = dir.join("motion/svg").join(format!("{id}.svg"));
            if svg.is_file() { svg } else { dir.join("motion/actions.vectorcraft") }
        }
    };
    if file.is_file() { CraftHub::open_live(target, &file) } else { CraftHub::launch_with_control(target, None, None, Some(&dir)) }
}

pub fn cmd_motion_preview_frame(s: &mut Session, p: &Value) -> Result<Value> {
    let dir = crate::workshop::project_dir(s)?;
    let time = p.get("time").or_else(|| p.get("seconds")).and_then(Value::as_f64).unwrap_or(0.0).max(0.0);
    let ecproj = dir.join("motion/actions.ecproj");
    if !ecproj.is_file() {
        return Ok(json!({ "empty": true, "kind": "motion", "time": time }));
    }
    let preview_dir = dir.join("motion/.preview");
    fs::create_dir_all(&preview_dir)?;
    let out = preview_dir.join(format!("motion-{}.png", (time * 1000.0).round() as u64));
    if out.is_file()
        && let (Ok(om), Ok(pm)) = (fs::metadata(&out), fs::metadata(&ecproj))
        && om.len() > 0
        && let (Ok(ot), Ok(pt)) = (om.modified(), pm.modified())
        && ot >= pt
    {
        return Ok(json!({ "frame": out, "source": ecproj, "kind": "motion", "time": time }));
    }
    CraftHub::render_effect_frame(&ecproj, Some(COMP_NAME), time, 640, &out)?;
    Ok(json!({ "frame": out, "source": ecproj, "kind": "motion", "time": time }))
}

/// `motion.effects`: return distinct CLI-mapped VectorCraft, EffectCraft, and PhotoCraft motion effects.
pub fn cmd_motion_effects(_s: &mut Session, p: &Value) -> Result<Value> {
    Ok(crate::craft_effects::query_motion_effects(p))
}
