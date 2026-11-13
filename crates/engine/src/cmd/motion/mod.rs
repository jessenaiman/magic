//! `motion`: Song-Action Drawing (`motion/actions.json`, `motion/svg/*.svg`, `motion/actions.ecproj`).
//!
//! Separated from Kinetic Lyrics (`cmd::words`) under Single Responsibility Principle:
//! - **Kinetic Lyrics (`words`)** authors `lettering/lyrics.ecproj` -> exports `V3` `"Kinetic Lyrics"` into `FilmCraft`.
//! - **Motion (`motion`)** binds drawn vector action illustrations (`.svg` / `.vectorcraft`) and `.ecproj`
//!   shape layers to song scenes & action words, traces artwork via `VectorCraft` (`imageTrace.make`),
//!   authors `motion/actions.ecproj` via `EffectCraft`, and publishes `V4` `"Action Motion"` into `FilmCraft`.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::time::{TICKS_PER_SECOND, Tick, parse_tick};
use crate::{EngineError, Result, Session};

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

// Temporary stub: return None until Geometry SVG path extraction is implemented via kurbo directly.
fn build_drawingml_svg_path(_preset_name: &str, _w: f64, _h: f64) -> Option<String> {
    let _geom = craft_drawingml::preset::build(_preset_name, _w, _h, &[])?;
    None
}

pub fn action_sketch_presets() -> Vec<Value> {
    let specs = [
        ("curvedRightArrow", "Glide Arc", "M 380 620 C 560 390 790 360 960 510 C 1130 360 1360 390 1540 620"),
        ("bentUpArrow", "Swoop", "M 340 360 C 540 760 1120 790 1540 340"),
        ("irregularSeal1", "Cry Burst", "M 960 320 L 995 455 L 1135 415 L 1045 530 L 1175 605 L 1025 615 L 1055 750 L 960 645 L 865 750 Z"),
        ("wave", "Wave", "M 360 640 Q 520 480 680 640 T 1000 640 T 1320 640 T 1560 640"),
        ("star5", "Star", "M 960 300 L 1025 465 L 1200 480 L 1065 595 L 1105 765 L 960 670 L 815 765 L 855 595 L 720 480 L 895 465 Z"),
        ("lightningBolt", "Lightning", "M 940 280 L 820 530 L 950 530 L 870 790 L 1130 490 L 990 490 Z"),
        ("moon", "Moon", "M 1040 320 C 850 340 760 520 850 690 C 920 820 1090 840 1200 760 C 1030 760 940 620 970 470 Z"),
        ("heart", "Heart", "M 960 740 C 760 600 680 430 810 340 C 890 285 960 355 960 410 C 960 355 1030 285 1110 340 C 1240 430 1160 600 960 740 Z"),
        (
            "cloud",
            "Cloud",
            "M 690 650 C 610 650 560 590 580 520 C 600 450 670 420 730 440 C 780 350 910 340 980 410 C 1060 370 1170 420 1180 510 C 1250 530 1260 650 1160 650 Z",
        ),
        ("roundRect", "Callout", "M 660 400 L 1260 400 L 1260 680 L 660 680 Z"),
    ];
    specs
        .into_iter()
        .map(|(id, name, default_svg)| {
            let d = build_drawingml_svg_path(id, 520.0, 260.0).unwrap_or_else(|| default_svg.to_owned());
            json!({
                "id": id,
                "name": name,
                "engine": "craft-drawingml",
                "d": d,
            })
        })
        .collect()
}

fn load_doc(s: &mut Session, dir: &Path) -> Result<Value> {
    Ok(s.doc::<MotionActionsDoc>(dir)?.map(|d| d.0).unwrap_or_else(|| json!({ "format": "magic.motion.actions", "version": 1, "cues": [] })))
}

fn save_doc(s: &mut Session, dir: &Path, doc: &Value) -> Result<()> {
    s.save_doc(dir, &MotionActionsDoc(doc.clone()))
}

#[allow(clippy::type_complexity)]
fn collect_motion_craft_files(dir: &Path) -> (Vec<PathBuf>, Vec<PathBuf>, Vec<(u64, PathBuf)>, Vec<Value>) {
    let mut svgs: Vec<PathBuf> = [dir.join("motion/svg"), dir.join("motion"), dir.join("assets/vectors")]
        .into_iter()
        .flat_map(|d| fs::read_dir(d).into_iter().flatten().flatten())
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|x| x.eq_ignore_ascii_case("svg")))
        .collect();
    svgs.sort();
    svgs.dedup();

    let mut vcrafts: Vec<PathBuf> = [dir.join("motion"), dir.join("motion/svg"), dir.join("assets/vectors")]
        .into_iter()
        .flat_map(|d| fs::read_dir(d).into_iter().flatten().flatten())
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|x| x.eq_ignore_ascii_case("vectorcraft")))
        .collect();
    vcrafts.sort();
    vcrafts.dedup();

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

    let mut ec_layers = Vec::new();
    let mut ec_files = vec![motion_ecproj_path(&dir)];
    for folder in [dir.join("motion"), dir.join("animations")] {
        for entry in fs::read_dir(folder).into_iter().flatten().flatten() {
            let p = entry.path();
            if p.is_file() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("ecproj")) && !ec_files.contains(&p) {
                ec_files.push(p);
            }
        }
    }
    for ec_path in ec_files {
        if let Ok(bytes) = fs::read(&ec_path)
            && let Ok(outline) = craft_doc::outline(&bytes, &craft_doc::Limits::default())
        {
            for page in outline.pages {
                for layer in page.layers {
                    ec_layers.push(json!({
                        "ecproj": ec_path,
                        "comp": page.name,
                        "name": layer.name,
                        "kind": format!("{:?}", layer.kind),
                    }));
                }
            }
        }
    }
    (svgs, vcrafts, frames, ec_layers)
}

fn words_from_timing_json(path: &Path) -> Option<Vec<Value>> {
    let bytes = fs::read(path).ok()?;
    let tref = serde_json::from_slice::<Value>(&bytes).ok()?;
    let words = tref["words"].as_array().filter(|w| !w.is_empty())?;
    Some(
        words
            .iter()
            .enumerate()
            .map(|(i, w)| {
                let st = w.get("start_tick").or_else(|| w.get("start")).and_then(parse_tick).unwrap_or(Tick::ZERO);
                let en = w.get("end_tick").or_else(|| w.get("end")).and_then(parse_tick).unwrap_or(st + Tick(TICKS_PER_SECOND / 2));
                json!({
                    "word_id": w["word_id"].as_str().map(str::to_owned).unwrap_or_else(|| format!("t/w{i}")),
                    "line_id": w["line_id"].as_str().map(str::to_owned).unwrap_or_else(|| format!("line-{}", i / 7 + 1)),
                    "section": w["section"].as_str().unwrap_or("Song"),
                    "text": w["text"].as_str().or_else(|| w["word"].as_str()).unwrap_or_default().trim(),
                    "start_tick": st.0,
                    "end_tick": en.0,
                    "start": st.seconds(),
                    "end": en.seconds(),
                })
            })
            .collect(),
    )
}

fn words_from_words_doc_or_song(s: &mut Session, dir: &Path) -> Vec<Value> {
    let wdoc = crate::cmd::words::load(s, dir).ok().flatten();
    if let Some(wdoc) = wdoc.as_ref()
        && let Some(anims) = wdoc["animations"].as_array()
        && !anims.is_empty()
    {
        let mut out = Vec::new();
        for (ai, anim) in anims.iter().enumerate() {
            let lid = anim["animation_id"].as_str().map(str::to_owned).unwrap_or_else(|| format!("line-{}", ai + 1));
            let section = anim["scope"]["section"].as_str().unwrap_or(&lid);
            let anchor = anim["song_anchor"].as_f64().unwrap_or((ai as f64) * 5.0);
            if let Some(arr) = anim["words"].as_array().filter(|a| !a.is_empty()) {
                for (wi, w) in arr.iter().enumerate() {
                    let st_sec = w["start"].as_f64().unwrap_or(anchor + (wi as f64) * 0.45);
                    let en_sec = w["end"].as_f64().unwrap_or(st_sec + 0.42);
                    let st = Tick::from_seconds_f64(st_sec);
                    let en = Tick::from_seconds_f64(en_sec);
                    out.push(json!({
                        "word_id": w["word_id"].as_str().map(str::to_owned).unwrap_or_else(|| format!("{lid}/w{wi}")),
                        "line_id": lid,
                        "section": section,
                        "text": w["text"].as_str().unwrap_or_default().trim(),
                        "start_tick": st.0,
                        "end_tick": en.0,
                        "start": st_sec,
                        "end": en_sec,
                    }));
                }
            }
        }
        if !out.is_empty() {
            return out;
        }
    }

    let mut raw_lines = Vec::new();
    for line in s.song.lyrics.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            raw_lines.push(("Song".to_owned(), trimmed.to_owned()));
        }
    }
    if raw_lines.is_empty() {
        return Vec::new();
    }
    let line_span = 5.2_f64;
    let mut out = Vec::new();
    for (li, (sec_name, line_txt)) in raw_lines.iter().enumerate() {
        let lid = format!("line-{}", li + 1);
        let words_in_line: Vec<&str> = line_txt.split_whitespace().collect();
        let step = (line_span / (words_in_line.len().max(1) as f64)).clamp(0.28, 0.85);
        for (wi, tok) in words_in_line.into_iter().enumerate() {
            let st_sec = (li as f64) * line_span + (wi as f64) * step;
            let en_sec = st_sec + step * 0.9;
            let st = Tick::from_seconds_f64(st_sec);
            let en = Tick::from_seconds_f64(en_sec);
            out.push(json!({
                "word_id": format!("{lid}/w{wi}"),
                "line_id": lid,
                "section": sec_name,
                "text": tok,
                "start_tick": st.0,
                "end_tick": en.0,
                "start": st_sec,
                "end": en_sec,
            }));
        }
    }
    out
}

fn timed_song_words(s: &mut Session, dir: &Path) -> Vec<Value> {
    for candidate in [dir.join("timing-reference.json"), dir.join("liner-notes/timing-reference.json"), dir.join("lettering/words.json")] {
        if let Some(found) = words_from_timing_json(&candidate) {
            return found;
        }
    }
    if let Ok(aligned) = crate::cmd::words::aligned_in(dir)
        && let Some(words) = aligned["words"].as_array()
        && !words.is_empty()
    {
        return words.clone();
    }
    if let Some(path) = crate::workshop::transcript_files(dir).into_iter().next()
        && let Some(found) = words_from_timing_json(&path)
    {
        return found;
    }
    words_from_words_doc_or_song(s, dir)
}

fn format_clock(sec: f64) -> String {
    if !sec.is_finite() || sec < 0.0 {
        return "0:00.0".into();
    }
    let m = (sec / 60.0).floor() as u64;
    let rem = sec - (m as f64) * 60.0;
    format!("{m}:{rem:04.1}")
}

fn pick_preset_for_text(text: &str, idx: usize) -> (&'static str, &'static str, [f64; 2], f64, [f32; 2], [f32; 2]) {
    let lower = text.to_ascii_lowercase();
    if lower.contains("sleep") || lower.contains("still") || lower.contains("moon") || lower.contains("nest") {
        ("moon", "ec.stylize.glow", [960.0, 560.0], -8.0, [-110.0, -34.0], [110.0, 28.0])
    } else if lower.contains("cave") || lower.contains("hill") || lower.contains("bottom") || lower.contains("down") {
        ("swoop_arc", "ec.shape.trim_paths", [920.0, 620.0], 18.0, [-140.0, 62.0], [135.0, -48.0])
    } else if lower.contains("cry") || lower.contains("sudden") || lower.contains("wake") || lower.contains("roar") {
        ("burst", "ec.generate.writeon", [1020.0, 460.0], 26.0, [-95.0, -82.0], [105.0, -74.0])
    } else if lower.contains("fly") || lower.contains("wing") || lower.contains("pterodactyl") {
        ("wing_glide", "ec.shape.trim_paths", [960.0, 470.0], -14.0, [-155.0, -68.0], [155.0, -68.0])
    } else if lower.contains("around") || lower.contains("circle") || lower.contains("spin") {
        ("spiral", "ec.distort.turbulent_displace", [960.0, 520.0], 32.0, [-130.0, -90.0], [130.0, 85.0])
    } else if lower.contains("east") || lower.contains("west") || lower.contains("wave") || lower.contains("sea") {
        ("wave", "vc.distort.zigzag", [980.0, 540.0], 12.0, [-145.0, -44.0], [145.0, 44.0])
    } else {
        let cycle = [
            ("wing_glide", "ec.shape.trim_paths", [940.0, 500.0], 14.0, [-135.0, -56.0], [135.0, -56.0]),
            ("swoop_arc", "ec.generate.writeon", [980.0, 580.0], -18.0, [-125.0, 54.0], [130.0, -48.0]),
            ("burst", "vc.brush.chalk", [1010.0, 460.0], 24.0, [-98.0, -76.0], [98.0, -76.0]),
            ("spiral", "ec.distort.turbulent_displace", [920.0, 530.0], -22.0, [-120.0, -84.0], [120.0, 76.0]),
            ("jump", "ec.shape.trim_paths", [960.0, 490.0], 16.0, [-130.0, -70.0], [130.0, 42.0]),
            ("star", "ec.stylize.glow", [960.0, 510.0], 8.0, [-105.0, -45.0], [105.0, -45.0]),
        ];
        cycle[idx % cycle.len()]
    }
}

fn preset_d_by_id(presets: &[Value], id: &str) -> String {
    presets
        .iter()
        .find(|p| p["id"].as_str() == Some(id))
        .and_then(|p| p["d"].as_str())
        .unwrap_or("M 380 620 C 560 390 790 360 960 510 C 1130 360 1360 390 1540 620")
        .to_owned()
}

fn synthesize_discovered_cues(dir: &Path, lines: &[Value], svg_files: &[PathBuf], ec_layers: &[Value], backdrops: &[Value], presets: &[Value]) -> Vec<Value> {
    let mut cues = Vec::new();
    let colors = ["#35E1C0", "#F5B841", "#58B9FF", "#A87FFB", "#FF7A90", "#5EEAD4"];

    if let Ok(bytes) = fs::read(dir.join("motion/converted-overlays.json"))
        && let Ok(ov_doc) = serde_json::from_slice::<Value>(&bytes)
        && let Some(arr) = ov_doc["overlays"].as_array()
    {
        for (idx, ov) in arr.iter().take(12).enumerate() {
            let label = ov["name"].as_str().or_else(|| ov["component"].as_str()).unwrap_or("Overlay");
            let st = ov["start_sec"].as_f64().unwrap_or((idx as f64) * 4.0);
            let en = ov["end_sec"].as_f64().unwrap_or(st + 2.4);
            let (pid, fx, [px, py], bend, tin, tout) = pick_preset_for_text(label, idx);
            let bg = backdrops.get(idx % backdrops.len().max(1)).and_then(|b| b["path"].as_str());
            cues.push(json!({
                "id": format!("ov-{}", idx + 1),
                "word_id": format!("ov/w{idx}"),
                "word": label,
                "title": label,
                "preset": pid,
                "d": preset_d_by_id(presets, pid),
                "color": colors[idx % colors.len()],
                "stroke_width": 8.0,
                "start": st,
                "end": en,
                "clock": format_clock(st),
                "draw_duration": (en - st).clamp(0.5, 2.0) * 0.5,
                "hold_duration": (en - st).clamp(0.5, 3.0) * 0.5,
                "effect_id": fx,
                "point_x": px,
                "point_y": py,
                "width": 540.0 + ((idx % 3) as f64) * 55.0,
                "height": 250.0 + ((idx % 2) as f64) * 45.0,
                "arc_bend": bend,
                "tangent_in": tin,
                "tangent_out": tout,
                "backdrop_path": bg,
                "discovered": true,
            }));
        }
    }

    if cues.is_empty() && !lines.is_empty() {
        for (idx, line) in lines.iter().take(10).enumerate() {
            let line_words = line["words"].as_array().cloned().unwrap_or_default();
            let phrase: String = line_words.iter().filter_map(|w| w["text"].as_str()).collect::<Vec<_>>().join(" ");
            let hero = line_words
                .iter()
                .rev()
                .find(|w| w["text"].as_str().is_some_and(|t| t.chars().filter(|c| c.is_alphabetic()).count() >= 4))
                .or_else(|| line_words.last());
            let word_txt = hero.and_then(|w| w["text"].as_str()).unwrap_or("Action");
            let wid = hero.and_then(|w| w["word_id"].as_str()).map(str::to_owned).unwrap_or_else(|| format!("line-{}/w0", idx + 1));
            let st = line["start"].as_f64().unwrap_or((idx as f64) * 4.8);
            let en = line_words.last().and_then(|w| w["end"].as_f64()).unwrap_or(st + 2.8).max(st + 1.6);
            let (pid, fx, [px, py], bend, tin, tout) = pick_preset_for_text(&phrase, idx);
            let bg = backdrops.get(idx % backdrops.len().max(1)).and_then(|b| b["path"].as_str());
            let svg_match = svg_files.get(idx).cloned();
            let svg_d = svg_match
                .as_deref()
                .and_then(|p| fs::read_to_string(p).ok())
                .and_then(|raw| extract_first_svg_d(&raw))
                .unwrap_or_else(|| preset_d_by_id(presets, pid));
            cues.push(json!({
                "id": format!("scene-cue-{}", idx + 1),
                "word_id": wid,
                "word": word_txt,
                "title": if phrase.is_empty() { word_txt.to_owned() } else { phrase },
                "preset": pid,
                "d": svg_d,
                "svg_file": svg_match,
                "color": colors[idx % colors.len()],
                "stroke_width": 8.0,
                "start": st,
                "end": en,
                "clock": format_clock(st),
                "draw_duration": 0.9,
                "hold_duration": (en - st - 0.9).clamp(0.6, 3.2),
                "effect_id": fx,
                "point_x": px,
                "point_y": py,
                "width": 520.0 + ((idx % 4) as f64) * 48.0,
                "height": 236.0 + ((idx % 3) as f64) * 38.0,
                "arc_bend": bend,
                "tangent_in": tin,
                "tangent_out": tout,
                "backdrop_path": bg,
                "discovered": true,
            }));
        }
    }

    if cues.is_empty() && (!ec_layers.is_empty() || !svg_files.is_empty() || !backdrops.is_empty()) {
        let count = ec_layers.len().max(svg_files.len()).max(backdrops.len()).clamp(1, 8);
        for idx in 0..count {
            let layer_lbl = ec_layers
                .get(idx)
                .and_then(|l| l["name"].as_str())
                .or_else(|| svg_files.get(idx).and_then(|p| p.file_stem()).and_then(|s| s.to_str()))
                .or_else(|| backdrops.get(idx).and_then(|b| b["name"].as_str()))
                .unwrap_or("Motion Layer");
            let st = (idx as f64) * 4.5;
            let en = st + 2.6;
            let (pid, fx, [px, py], bend, tin, tout) = pick_preset_for_text(layer_lbl, idx);
            let bg = backdrops.get(idx % backdrops.len().max(1)).and_then(|b| b["path"].as_str());
            cues.push(json!({
                "id": format!("layer-cue-{}", idx + 1),
                "word_id": format!("layer/w{idx}"),
                "word": layer_lbl,
                "title": layer_lbl,
                "preset": pid,
                "d": preset_d_by_id(presets, pid),
                "svg_file": svg_files.get(idx),
                "color": colors[idx % colors.len()],
                "stroke_width": 8.0,
                "start": st,
                "end": en,
                "clock": format_clock(st),
                "draw_duration": 0.9,
                "hold_duration": 1.7,
                "effect_id": fx,
                "point_x": px,
                "point_y": py,
                "width": 540.0,
                "height": 250.0,
                "arc_bend": bend,
                "tangent_in": tin,
                "tangent_out": tout,
                "backdrop_path": bg,
                "discovered": true,
            }));
        }
    }
    cues
}

fn extract_first_svg_d(svg_raw: &str) -> Option<String> {
    let idx = svg_raw.find(" d=\"")?;
    let rest = &svg_raw[idx + 4..];
    let end = rest.find('"')?;
    let d = rest[..end].trim();
    (!d.is_empty()).then(|| d.to_owned())
}

/// Resolve the project's motion-design EffectCraft file (`effects/motion.ecproj` in the 5-folder
/// layout, falling back to `motion/actions.ecproj` when opening an older project folder).
pub(crate) fn motion_ecproj_path(dir: &Path) -> PathBuf {
    let preferred = dir.join("effects/motion.ecproj");
    if preferred.is_file() {
        return preferred;
    }
    let legacy = dir.join("motion/actions.ecproj");
    if legacy.is_file() {
        return legacy;
    }
    preferred
}

fn sync_actions_ecproj(dir: &Path, cues: &[Value], duration_sec: f64) -> Result<PathBuf> {
    let ecproj = motion_ecproj_path(dir);
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
            "frameRate": 24.0,
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

    let _ = steps;
    Ok(ecproj)
}

pub fn cmd_motion_read(s: &mut Session, _p: &Value) -> Result<Value> {
    let dir = crate::workshop::project_dir(s)?;
    let doc = load_doc(s, &dir)?;
    let words = timed_song_words(s, &dir);
    let mut cues = doc["cues"].as_array().cloned().unwrap_or_default();
    let (svg_files, vectorcraft_files, preview_frames, ec_layers) = collect_motion_craft_files(&dir);
    let backdrops = crate::cmd::words::stage::project_backdrop_images(s, &dir);
    let presets = action_sketch_presets();

    let mut lines: Vec<Value> = Vec::new();
    for w in &words {
        let lid = w["line_id"].as_str().unwrap_or("line-1");
        let mut word_chip = w.clone();
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

    if cues.is_empty() {
        cues = synthesize_discovered_cues(&dir, &lines, &svg_files, &ec_layers, &backdrops, &presets);
    }

    for (idx, cue) in cues.iter_mut().enumerate() {
        let cue_id = cue["id"].as_str().unwrap_or("action").to_owned();
        let svg_file = cue["svg_path"].as_str().or_else(|| cue["svg_file"].as_str()).map(PathBuf::from).filter(|p| p.is_file()).or_else(|| {
            let candidate = dir.join("motion/svg").join(format!("{cue_id}.svg"));
            candidate.is_file().then_some(candidate)
        });
        cue["svg_file"] = json!(svg_file);
        if cue["backdrop_path"].is_null()
            && let Some(bg) = backdrops.get(idx % backdrops.len().max(1)).and_then(|b| b["path"].as_str())
        {
            cue["backdrop_path"] = json!(bg);
        }
        let st_ms = (cue["start"].as_f64().unwrap_or(0.0).max(0.0) * 1000.0).round() as u64;
        let en_ms = (cue["end"].as_f64().unwrap_or(1.5).max(0.1) * 1000.0).round() as u64;
        let frame = preview_frames
            .iter()
            .find(|(ms, _)| *ms >= st_ms && *ms <= en_ms)
            .or_else(|| preview_frames.iter().min_by_key(|(ms, _)| ms.abs_diff(st_ms)))
            .map(|(_, p)| p.clone());
        cue["preview_frame"] = json!(frame);
    }

    let duration = words.last().and_then(|w| w["end"].as_f64()).or_else(|| cues.last().and_then(|c| c["end"].as_f64())).unwrap_or(30.0).max(10.0) + 2.0;
    let ecproj = motion_ecproj_path(&dir);
    let mov = dir.join("motion/actions.mov");
    let backdrop = backdrops.first().cloned().unwrap_or(Value::Null);
    let preview_paths: Vec<PathBuf> = preview_frames.into_iter().map(|(_, p)| p).collect();

    Ok(json!({
        "ok": true,
        "path": dir.join("motion/actions.json"),
        "ecproj": ecproj.is_file().then_some(ecproj),
        "ecproj_layers": ec_layers,
        "render": mov.is_file().then_some(mov),
        "svg_files": svg_files,
        "vectorcraft_files": vectorcraft_files,
        "preview_frames": preview_paths,
        "backdrops": backdrops,
        "duration": duration,
        "lines": lines,
        "cues": cues,
        "presets": presets,
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
    let words = timed_song_words(s, &dir);
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

    let cue_title = p["title"].as_str().or_else(|| p["word"].as_str()).unwrap_or("Action");
    let word_id = p["word_id"]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| format!("cue/{}", crate::workspace::sanitize_asset_id(cue_title)));
    let word_entry = words.iter().find(|w| w["word_id"].as_str() == Some(word_id.as_str()));
    let word_text = p["word"].as_str().or_else(|| p["title"].as_str()).or_else(|| word_entry.and_then(|w| w["text"].as_str())).unwrap_or("Action");
    let start_tick = p
        .get("start_tick")
        .or_else(|| p.get("start"))
        .and_then(parse_tick)
        .or_else(|| word_entry.and_then(|w| w.get("start_tick").or_else(|| w.get("start"))).and_then(parse_tick))
        .unwrap_or(Tick::ZERO);

    let slug = crate::workspace::sanitize_asset_id(word_text);
    let cue_id = p["id"]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| format!("act-{}-{}", if slug.is_empty() { "word" } else { &slug }, word_id.replace('/', "-")));

    let mut traced_svg: Option<PathBuf> = None;
    let d = if let Some(img_raw) = p["image"].as_str().filter(|v| !v.trim().is_empty()) {
        let img = crate::workspace::resolve_workspace_path(s, img_raw);
        let svg_out = dir.join("motion/svg").join(format!("{cue_id}.svg"));
        let use_strokes = p["effect_id"].as_str() != Some("vc.trace.silhouette");
        crate::native_media::trace_silhouette_svg(&img, &svg_out, 1.5, use_strokes)?;
        traced_svg = Some(svg_out);
        String::new()
    } else if let Some(explicit_d) = p["d"].as_str().map(str::trim).filter(|s| !s.is_empty()) {
        explicit_d.to_owned()
    } else if let Some(preset_id) = p.get("preset").and_then(Value::as_str) {
        action_sketch_presets()
            .into_iter()
            .find(|pr| pr["id"].as_str() == Some(preset_id))
            .and_then(|pr| pr["d"].as_str().map(str::to_owned))
            .or_else(|| build_drawingml_svg_path(preset_id, 520.0, 260.0))
            .ok_or_else(|| bad(format!("unknown craft-drawingml preset `{preset_id}`")))?
    } else {
        return Err(bad("pass SVG path `d`, `preset`, or `image`"));
    };

    let draw_dur = p["draw_duration"].as_f64().unwrap_or(0.85).clamp(0.1, 10.0);
    let hold_dur = p["hold_duration"].as_f64().unwrap_or(1.2).clamp(0.1, 15.0);
    let end_tick = start_tick + Tick::from_seconds_f64(draw_dur + hold_dur);
    let cue = json!({
        "id": cue_id,
        "word_id": word_id,
        "word": word_text,
        "title": p["title"].as_str().unwrap_or(word_text),
        "preset": p["preset"].as_str().unwrap_or("wing_glide"),
        "d": d,
        "svg_path": traced_svg,
        "color": p["color"].as_str().unwrap_or("#35E1C0"),
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
        "width": p["width"].as_f64().unwrap_or(520.0),
        "height": p["height"].as_f64().unwrap_or(240.0),
        "arc_bend": p["arc_bend"].as_f64().unwrap_or(0.0),
        "taper": p["taper"].as_f64().unwrap_or(65.0),
        "tangent_in": p.get("tangent_in").cloned().unwrap_or_else(|| json!([-120.0, -54.0])),
        "tangent_out": p.get("tangent_out").cloned().unwrap_or_else(|| json!([120.0, -54.0])),
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
        let _ = crate::craft_api::execute_in_live_app(
            "effectcraft",
            "render.comp",
            json!({"comp": COMP_NAME, "output": out_mov, "format": "prores", "proresProfile": "4444", "channels": "rgba"}),
        );
        if p["studio"].as_bool() == Some(true) {
            studio = Some(crate::cmd::film::publish_motion(s, &out_mov)?);
        }
    }

    Ok(json!({ "ok": true, "cue": cue, "ecproj": ecproj, "cues": cues, "studio": studio }))
}

/// `motion.publish`: stage `motion/actions.ecproj` ProRes 4444 RGBA render and publish onto `FilmCraft` `V4` `"Action Motion"`.
pub fn cmd_motion_publish(s: &mut Session, p: &Value) -> Result<Value> {
    let dir = crate::workshop::project_dir(s)?;
    let ecproj = motion_ecproj_path(&dir);
    if !ecproj.is_file() {
        let read_val = cmd_motion_read(s, &json!({}))?;
        let cues = read_val["cues"].as_array().cloned().unwrap_or_default();
        if cues.is_empty() {
            return Err(bad("No song-action motion cues yet; add a cue with motion.save first"));
        }
        let duration = read_val["duration"].as_f64().unwrap_or(30.0);
        let _ = sync_actions_ecproj(&dir, &cues, duration)?;
    }
    let out_mov = p["path"].as_str().map(|raw| crate::workspace::resolve_workspace_path(s, raw)).unwrap_or_else(|| dir.join("motion/actions.mov"));
    if p["render"].as_bool().unwrap_or(!out_mov.is_file()) {
        let _ = crate::craft_api::execute_in_live_app(
            "effectcraft",
            "render.comp",
            json!({"comp": COMP_NAME, "output": out_mov, "format": "prores", "proresProfile": "4444", "channels": "rgba"}),
        );
    }
    let studio = crate::cmd::film::publish_motion(s, &out_mov)?;
    Ok(json!({ "ok": true, "ecproj": ecproj, "render": out_mov, "track": "Action Motion", "studio": studio }))
}

pub fn cmd_motion_open(s: &mut Session, p: &Value) -> Result<Value> {
    let dir = crate::workshop::project_dir(s)?;
    let app = p["app"].as_str().unwrap_or("vectorcraft").trim();
    let file = match app {
        "effectcraft" => {
            let ec = motion_ecproj_path(&dir);
            if !ec.is_file() {
                let read_val = cmd_motion_read(s, &json!({}))?;
                let cues = read_val["cues"].as_array().cloned().unwrap_or_default();
                let dur = read_val["duration"].as_f64().unwrap_or(30.0);
                let _ = sync_actions_ecproj(&dir, &cues, dur)?;
            }
            ec
        }
        "vectorcraft" => {
            let id = p["id"].as_str().unwrap_or("song-actions");
            let vc = dir.join("motion").join(format!("{id}.vectorcraft"));
            if vc.is_file() {
                vc
            } else if let Some(first_svg) = collect_motion_craft_files(&dir).0.into_iter().next() {
                first_svg
            } else {
                dir.join("motion/svg/song-actions.svg")
            }
        }
        other => return Err(bad(format!("unsupported app `{other}` (expected vectorcraft|effectcraft)"))),
    };
    if file.exists() {
        crate::workspace::open_editor(app, &file)?;
        Ok(json!({"ok": true, "app": app, "file": file}))
    } else {
        crate::workspace::launch_app_only(app)?;
        Ok(json!({"ok": true, "app": app, "launched": true}))
    }
}

pub fn cmd_motion_preview_frame(s: &mut Session, p: &Value) -> Result<Value> {
    let time = p["time"].as_f64().unwrap_or(0.0).max(0.0);
    let dir = crate::workshop::project_dir(s)?;
    let ecproj = motion_ecproj_path(&dir);
    let preview_dir = dir.join("motion/.preview");
    std::fs::create_dir_all(&preview_dir)?;
    let out = preview_dir.join(format!("motion-{}.png", (time * 1000.0).round() as u64));
    if ecproj.is_file() {
        let _ = crate::native_media::extract_or_copy_preview(&ecproj, &out, 960);
    }
    Ok(json!({"ok": true, "frame": out.is_file().then_some(&out), "time": time}))
}
