//! `character.puppet.*`: Native `EffectCraft` Puppet Controls (`puppet.addPin`, `puppet.movePin`,
//! `puppet.recordPin`, `puppet.follow`) for facial expressions, lip-sync mouth movements synced to
//! `timing-reference.json`, cranium/eye `starch` stabilization, `overlap` depth pins, and secondary
//! ear/hair/tail follow-through (`https://github.com/storytold/effectcraft`).

use std::fs;
use std::path::Path;

use serde_json::{Value, json};

use super::{View, bad, effectcraft, part_list, step};
use crate::time::{TICKS_PER_SECOND, Tick, parse_tick};
use crate::{Result, Session};

/// Resolve the head layer (or fallback primary character layer) and its `[x, y, w, h]` pixel bounds.
fn resolve_face_layer(parts: &Value) -> Result<(String, [f64; 4])> {
    let list = part_list(parts)?;
    let chosen = list
        .iter()
        .find(|p| p["id"].as_str().is_some_and(|id| id.eq_ignore_ascii_case("head") || id.contains("face")))
        .or_else(|| list.iter().find(|p| p["id"].as_str() == Some("torso") || p["id"].as_str() == Some("body")))
        .or_else(|| list.first())
        .ok_or_else(|| bad("parts.json has no layers to attach facial Puppet pins"))?;
    let layer = chosen["id"].as_str().ok_or_else(|| bad("face layer has no id"))?.to_string();
    let bounds = if let Some(b) = chosen["bounds"].as_array().filter(|b| b.len() == 4) {
        [b[0].as_f64().unwrap_or(0.0), b[1].as_f64().unwrap_or(0.0), b[2].as_f64().unwrap_or(256.0).max(32.0), b[3].as_f64().unwrap_or(256.0).max(32.0)]
    } else {
        let w = parts["size"][0].as_f64().unwrap_or(512.0).max(64.0);
        let h = parts["size"][1].as_f64().unwrap_or(512.0).max(64.0);
        [0.0, 0.0, w, h]
    };
    Ok((layer, bounds))
}

/// Canonical 11-pin `EffectCraft` facial & mouth Puppet rig in layer-local `[u, v]` coordinates:
/// - 3 `starch` pins lock the cranium & eye-socket band so jaw/mouth deformations never warp the eyes.
/// - 4 mouth viseme pins (`lip_upper`, `jaw_lower`, `mouth_L`, `mouth_R`) sculpt vowels & smiles.
/// - 2 `advanced` brow pins (`brow_L`, `brow_R`) lift, furrow, and tilt eyebrows.
/// - 2 `bend` cheek/squint pins (`cheek_L`, `cheek_R`) add natural cheek rise & blink compression.
fn canonical_face_pins(w: f64, h: f64) -> Vec<(&'static str, &'static str, [f64; 2])> {
    let pt = |u: f64, v: f64| [(w * u).round(), (h * v).round()];
    vec![
        ("cranium_lock", "starch", pt(0.50, 0.22)),
        ("temple_L_lock", "starch", pt(0.28, 0.40)),
        ("temple_R_lock", "starch", pt(0.72, 0.40)),
        ("brow_L", "advanced", pt(0.35, 0.34)),
        ("brow_R", "advanced", pt(0.65, 0.34)),
        ("cheek_L", "bend", pt(0.32, 0.54)),
        ("cheek_R", "bend", pt(0.68, 0.54)),
        ("lip_upper", "position", pt(0.50, 0.66)),
        ("jaw_lower", "position", pt(0.50, 0.78)),
        ("mouth_L", "advanced", pt(0.39, 0.71)),
        ("mouth_R", "advanced", pt(0.61, 0.71)),
    ]
}

fn pin_prop_path(pin_name: &str, prop: &str) -> String {
    format!("effects/Puppet/meshes/Mesh 1/deform/{pin_name}/{prop}")
}

/// `character.puppet.rig_face`: install the 11-pin `EffectCraft` facial & mouth Puppet rig on the
/// character's head layer, plus optional `puppet.follow` secondary cascade on ears/hair/tail.
pub fn cmd_puppet_rig_face(s: &mut Session, p: &Value) -> Result<Value> {
    let view = View::from_params(s, p)?;
    let project = view.require("ecproj", "character.rig")?;
    let mut parts = view.parts()?;
    let (default_layer, bounds) = resolve_face_layer(&parts)?;
    let layer = p["layer"].as_str().filter(|v| !v.trim().is_empty()).unwrap_or(&default_layer).to_string();
    let w = p["width"].as_f64().unwrap_or(bounds[2]);
    let h = p["height"].as_f64().unwrap_or(bounds[3]);

    let pins = canonical_face_pins(w, h);
    let mut steps = vec![step("comp.open", json!({"comp": view.comp()}))];
    let mut rest_map = serde_json::Map::new();
    for (name, kind, pos) in &pins {
        steps.push(step("puppet.addPin", json!({"layer": layer, "kind": kind, "name": name, "position": pos})));
        rest_map.insert((*name).to_string(), json!({"kind": kind, "rest": pos}));
    }

    // Wire cascade follow-through on ears or tail when present in parts.json.
    for follow_id in ["ear_L", "ear_R", "tail"] {
        if let Some(part) = parts["parts"].as_array().into_iter().flatten().find(|pt| pt["id"].as_str() == Some(follow_id))
            && let Some(b) = part["bounds"].as_array().filter(|b| b.len() == 4)
        {
            let fw = b[2].as_f64().unwrap_or(80.0).max(16.0);
            let fh = b[3].as_f64().unwrap_or(80.0).max(16.0);
            let root_pin = format!("{follow_id}_base");
            let mid_pin = format!("{follow_id}_mid");
            let tip_pin = format!("{follow_id}_tip");
            steps.push(step("puppet.addPin", json!({"layer": follow_id, "kind": "starch", "name": &root_pin, "position": [fw * 0.5, fh * 0.85]})));
            steps.push(step("puppet.addPin", json!({"layer": follow_id, "kind": "bend", "name": &mid_pin, "position": [fw * 0.5, fh * 0.50]})));
            steps.push(step("puppet.addPin", json!({"layer": follow_id, "kind": "position", "name": &tip_pin, "position": [fw * 0.5, fh * 0.15]})));
            steps.push(step(
                "puppet.follow",
                json!({"layer": follow_id, "leader": root_pin, "pins": [mid_pin, tip_pin], "delay": 0.07, "amount": 90, "cascade": true}),
            ));
        }
    }

    effectcraft(&view.dir, Some(&project), &steps, true)?;
    parts["face_puppet"] = json!({
        "layer": layer,
        "width": w,
        "height": h,
        "pins": rest_map,
    });
    view.write_parts(&parts)?;
    Ok(json!({
        "ok": true,
        "ecproj": project,
        "layer": layer,
        "pins_added": pins.len(),
        "face_puppet": parts["face_puppet"],
    }))
}

/// Pin delta offsets `(pin_name, [dx_frac_of_w, dy_frac_of_h])` for expressive facial states.
fn expression_deltas(expr: &str) -> &'static [(&'static str, [f64; 2])] {
    match expr {
        "happy" | "smile" => &[
            ("brow_L", [0.0, -0.03]),
            ("brow_R", [0.0, -0.03]),
            ("cheek_L", [-0.015, -0.035]),
            ("cheek_R", [0.015, -0.035]),
            ("mouth_L", [-0.04, -0.035]),
            ("mouth_R", [0.04, -0.035]),
            ("jaw_lower", [0.0, 0.025]),
        ],
        "joy" | "sing_wide" => &[
            ("brow_L", [0.0, -0.045]),
            ("brow_R", [0.0, -0.045]),
            ("cheek_L", [-0.02, -0.04]),
            ("cheek_R", [0.02, -0.04]),
            ("lip_upper", [0.0, -0.02]),
            ("jaw_lower", [0.0, 0.085]),
            ("mouth_L", [-0.045, -0.025]),
            ("mouth_R", [0.045, -0.025]),
        ],
        "oh" | "surprised" => &[
            ("brow_L", [0.0, -0.06]),
            ("brow_R", [0.0, -0.06]),
            ("lip_upper", [0.0, -0.025]),
            ("jaw_lower", [0.0, 0.095]),
            ("mouth_L", [0.03, 0.01]),
            ("mouth_R", [-0.03, 0.01]),
        ],
        "sad" => &[("brow_L", [0.02, 0.03]), ("brow_R", [-0.02, 0.03]), ("mouth_L", [-0.015, 0.035]), ("mouth_R", [0.015, 0.035]), ("jaw_lower", [0.0, 0.01])],
        "angry" | "focused" => &[("brow_L", [0.03, 0.045]), ("brow_R", [-0.03, 0.045]), ("lip_upper", [0.0, 0.01]), ("jaw_lower", [0.0, -0.015])],
        "blink" => &[("brow_L", [0.0, 0.025]), ("brow_R", [0.0, 0.025]), ("cheek_L", [0.0, -0.03]), ("cheek_R", [0.0, -0.03])],
        "wink_L" => &[("brow_L", [0.0, 0.03]), ("cheek_L", [0.0, -0.035]), ("mouth_L", [-0.025, -0.02])],
        "wink_R" => &[("brow_R", [0.0, 0.03]), ("cheek_R", [0.0, -0.035]), ("mouth_R", [0.025, -0.02])],
        _ => &[],
    }
}

fn ensure_face_rig(s: &mut Session, p: &Value) -> Result<(View, std::path::PathBuf, Value)> {
    let view = View::from_params(s, p)?;
    let project = view.require("ecproj", "character.rig")?;
    let mut parts = view.parts()?;
    if !parts["face_puppet"]["pins"].is_object() {
        let _ = cmd_puppet_rig_face(s, p)?;
        parts = view.parts()?;
    }
    Ok((view, project, parts))
}

/// `character.puppet.expression`: keyframe a facial expression (`happy`, `joy`, `surprised`, `oh`,
/// `sad`, `focused`, `blink`, `wink_L`, `wink_R`, `neutral`) using `EffectCraft` Puppet Pins.
pub fn cmd_puppet_expression(s: &mut Session, p: &Value) -> Result<Value> {
    let (view, project, parts) = ensure_face_rig(s, p)?;
    let expr = p["expression"].as_str().unwrap_or("happy");
    let at = p["at"].as_f64().unwrap_or(0.0).max(0.0);
    let dur = p["duration"].as_f64().unwrap_or(if expr == "blink" { 0.22 } else { 0.9 }).clamp(0.08, 30.0);
    let intensity = p["intensity"].as_f64().unwrap_or(1.0).clamp(0.1, 2.5);

    let face = &parts["face_puppet"];
    let layer = face["layer"].as_str().ok_or_else(|| bad("face_puppet missing layer"))?;
    let w = face["width"].as_f64().unwrap_or(256.0);
    let h = face["height"].as_f64().unwrap_or(256.0);
    let pins = face["pins"].as_object().ok_or_else(|| bad("face_puppet missing pins"))?;

    let deltas = expression_deltas(expr);
    let lead_in = (at + (dur * 0.18).min(0.14)).min(at + dur * 0.45);
    let hold_out = (at + dur - (dur * 0.2).min(0.16)).max(lead_in + 0.02);
    let end = at + dur;

    let mut steps = vec![step("comp.open", json!({"comp": view.comp()}))];
    for (pin_name, info) in pins {
        let Some(rest) = info["rest"].as_array().filter(|a| a.len() == 2) else { continue };
        let rx = rest[0].as_f64().unwrap_or(0.0);
        let ry = rest[1].as_f64().unwrap_or(0.0);
        let [dx, dy] = deltas.iter().find(|(n, _)| *n == pin_name).map(|(_, d)| *d).unwrap_or([0.0, 0.0]);
        if dx.abs() < 1e-6 && dy.abs() < 1e-6 && expr != "neutral" {
            continue;
        }
        let target = [rx + dx * w * intensity, ry + dy * h * intensity];
        let path = pin_prop_path(pin_name, "position");
        for (t, val) in [(at, [rx, ry]), (lead_in, target), (hold_out, target), (end, [rx, ry])] {
            steps.push(step("prop.addKey", json!({"layer": layer, "path": path, "time": t, "timeBase": "comp", "value": val})));
        }
    }
    steps.push(step("keys.selectAll", json!({})));
    steps.push(step("keys.easyEase", json!({})));
    effectcraft(&view.dir, Some(&project), &steps, true)?;

    Ok(json!({
        "ok": true,
        "ecproj": project,
        "layer": layer,
        "expression": expr,
        "start": at,
        "end": end,
    }))
}

/// Classify a sung word into mouth viseme deformation `(jaw_drop, lip_lift, width_spread)` in normalized face units.
fn word_viseme_shape(word: &str) -> (f64, f64, f64) {
    let lower: String = word.chars().filter(char::is_ascii_alphabetic).flat_map(char::to_lowercase).collect();
    if lower.contains('o') || lower.contains('u') || lower.contains('w') {
        (0.085, -0.015, -0.022) // rounded "oh / oo / whale"
    } else if lower.contains('a') || lower.contains("ai") || lower.contains("ay") {
        (0.092, -0.022, 0.025) // wide open "ah / ai"
    } else if lower.contains('e') || lower.contains('i') || lower.contains('y') {
        (0.055, -0.012, 0.038) // spread smile "ee / ih"
    } else if lower.starts_with(['m', 'b', 'p']) {
        (0.018, 0.006, -0.010) // bilabial press -> release
    } else {
        (0.062, -0.010, 0.012) // general sung syllable
    }
}

fn load_timed_words_for_lipsync(project_dir: &Path, start: f64, end: Option<f64>) -> Vec<(String, f64, f64)> {
    let mut out = Vec::new();
    let tref_path = project_dir.join("timing-reference.json");
    if let Ok(bytes) = fs::read(&tref_path)
        && let Ok(doc) = serde_json::from_slice::<Value>(&bytes)
        && let Some(words) = doc["words"].as_array()
    {
        for w in words {
            let text = w["text"].as_str().unwrap_or_default().trim();
            let ws = w["start_sec"].as_f64().or_else(|| w.get("start_tick").and_then(parse_tick).map(Tick::seconds)).unwrap_or(0.0);
            let we = w["end_sec"].as_f64().or_else(|| w.get("end_tick").and_then(parse_tick).map(Tick::seconds)).unwrap_or(ws + 0.25);
            if !text.is_empty() && we >= start && end.is_none_or(|max_e| ws <= max_e) {
                out.push((text.to_string(), ws.max(0.0), we.max(ws + 0.08)));
            }
        }
        return out;
    }
    if let Some(t_file) = crate::workshop::transcript_files(project_dir).into_iter().next()
        && let Ok(bytes) = fs::read(t_file)
        && let Ok(doc) = serde_json::from_slice::<Value>(&bytes)
    {
        let arr = doc.as_array().or_else(|| doc["words"].as_array());
        for w in arr.into_iter().flatten() {
            let text = w["text"].as_str().unwrap_or_default().trim();
            let ws = w.get("start_tick").or_else(|| w.get("start")).and_then(parse_tick).unwrap_or(Tick::ZERO).seconds();
            let we = w.get("end_tick").or_else(|| w.get("end")).and_then(parse_tick).unwrap_or(Tick(TICKS_PER_SECOND / 4)).seconds();
            if !text.is_empty() && we >= start && end.is_none_or(|max_e| ws <= max_e) {
                out.push((text.to_string(), ws.max(0.0), we.max(ws + 0.08)));
            }
        }
    }
    out
}

/// `character.puppet.lipsync`: animate `jaw_lower`, `lip_upper`, `mouth_L`, and `mouth_R` Puppet Pins
/// across sung words from `timing-reference.json` so the character performs natural vowel/consonant
/// mouth movements in `EffectCraft`.
pub fn cmd_puppet_lipsync(s: &mut Session, p: &Value) -> Result<Value> {
    let project_dir = crate::workshop::project_dir(s)?;
    let (view, project, parts) = ensure_face_rig(s, p)?;
    let start = p["start"].as_f64().unwrap_or(0.0).max(0.0);
    let end = p["end"].as_f64().filter(|v| *v > start);
    let intensity = p["intensity"].as_f64().unwrap_or(1.0).clamp(0.2, 2.5);

    let words: Vec<(String, f64, f64)> = if let Some(explicit) = p["words"].as_array() {
        explicit
            .iter()
            .filter_map(|w| {
                let text = w["text"].as_str()?.trim().to_string();
                let ws = w["start"].as_f64()?;
                let we = w["end"].as_f64().unwrap_or(ws + 0.28);
                (!text.is_empty()).then_some((text, ws, we.max(ws + 0.08)))
            })
            .collect()
    } else {
        load_timed_words_for_lipsync(&project_dir, start, end)
    };
    if words.is_empty() {
        return Err(bad("No timed words in timing-reference.json for lip-sync; transcribe or import lyrics first"));
    }

    let face = &parts["face_puppet"];
    let layer = face["layer"].as_str().ok_or_else(|| bad("face_puppet missing layer"))?;
    let w = face["width"].as_f64().unwrap_or(256.0);
    let h = face["height"].as_f64().unwrap_or(256.0);
    let rest_of = |pin: &str| -> [f64; 2] {
        face["pins"][pin]["rest"]
            .as_array()
            .filter(|a| a.len() == 2)
            .map(|a| [a[0].as_f64().unwrap_or(0.0), a[1].as_f64().unwrap_or(0.0)])
            .unwrap_or([w * 0.5, h * 0.7])
    };
    let jaw_rest = rest_of("jaw_lower");
    let up_rest = rest_of("lip_upper");
    let ml_rest = rest_of("mouth_L");
    let mr_rest = rest_of("mouth_R");

    let mut steps = vec![step("comp.open", json!({"comp": view.comp()}))];
    let last_end = words.last().map(|(_, _, we)| *we + 0.6).unwrap_or(10.0);
    steps.push(step("comp.settings", json!({"duration": last_end.max(10.0)})));

    for (word, ws, we) in &words {
        let (jaw_dy, up_dy, spread_dx) = word_viseme_shape(word);
        let pre = (ws - 0.04).max(0.0);
        let peak = ws + ((we - ws) * 0.38).clamp(0.05, 0.22);
        let close = *we;

        let key_pin = |steps: &mut Vec<Value>, pin: &str, t: f64, val: [f64; 2]| {
            steps.push(step("prop.addKey", json!({"layer": layer, "path": pin_prop_path(pin, "position"), "time": t, "timeBase": "comp", "value": val})));
        };

        // Closed rest right before consonant onset -> vowel apex -> consonant closure
        key_pin(&mut steps, "jaw_lower", pre, jaw_rest);
        key_pin(&mut steps, "jaw_lower", peak, [jaw_rest[0], jaw_rest[1] + jaw_dy * h * intensity]);
        key_pin(&mut steps, "jaw_lower", close, jaw_rest);

        key_pin(&mut steps, "lip_upper", pre, up_rest);
        key_pin(&mut steps, "lip_upper", peak, [up_rest[0], up_rest[1] + up_dy * h * intensity]);
        key_pin(&mut steps, "lip_upper", close, up_rest);

        key_pin(&mut steps, "mouth_L", pre, ml_rest);
        key_pin(&mut steps, "mouth_L", peak, [ml_rest[0] - spread_dx * w * intensity, ml_rest[1]]);
        key_pin(&mut steps, "mouth_L", close, ml_rest);

        key_pin(&mut steps, "mouth_R", pre, mr_rest);
        key_pin(&mut steps, "mouth_R", peak, [mr_rest[0] + spread_dx * w * intensity, mr_rest[1]]);
        key_pin(&mut steps, "mouth_R", close, mr_rest);
    }
    steps.push(step("keys.selectAll", json!({})));
    steps.push(step("keys.easyEase", json!({})));
    effectcraft(&view.dir, Some(&project), &steps, true)?;

    Ok(json!({
        "ok": true,
        "ecproj": project,
        "layer": layer,
        "words_keyed": words.len(),
        "duration": last_end,
    }))
}

/// `character.puppet.follow`: attach EffectCraft's native `puppet.follow` secondary cascade
/// (`leader`, `pins`, `delay`, `amount`, `cascade`) to any hair, ear, tail, cape, or limb layer.
pub fn cmd_puppet_follow(s: &mut Session, p: &Value) -> Result<Value> {
    let view = View::from_params(s, p)?;
    let project = view.require("ecproj", "character.rig")?;
    let layer = p["layer"].as_str().ok_or_else(|| bad("layer=<layer name> is required"))?;
    let leader = p["leader"].as_str().ok_or_else(|| bad("leader=<leader pin name> is required"))?;
    let pins = p["pins"].as_array().filter(|a| !a.is_empty()).ok_or_else(|| bad("pins=[\"Pin 2\", ...] is required"))?;
    let params = json!({
        "layer": layer,
        "leader": leader,
        "pins": pins,
        "delay": p["delay"].as_f64().unwrap_or(0.08),
        "amount": p["amount"].as_u64().unwrap_or(100),
        "cascade": p["cascade"].as_bool().unwrap_or(true),
    });
    let steps = vec![step("comp.open", json!({"comp": view.comp()})), step("puppet.follow", params)];
    let out = effectcraft(&view.dir, Some(&project), &steps, true)?;
    Ok(json!({"ok": true, "ecproj": project, "layer": layer, "leader": leader, "result": out.last()}))
}

/// `character.puppet.record`: record a real-time Puppet pin gesture path (`samples: [[t, x, y], ...]`)
/// onto a pin via `EffectCraft` `puppet.recordPin`.
pub fn cmd_puppet_record(s: &mut Session, p: &Value) -> Result<Value> {
    let view = View::from_params(s, p)?;
    let project = view.require("ecproj", "character.rig")?;
    let layer = p["layer"].as_str().ok_or_else(|| bad("layer=<layer name> is required"))?;
    let pin = p["pin"].as_str().ok_or_else(|| bad("pin=<pin name> is required"))?;
    let samples = p["samples"].as_array().filter(|a| !a.is_empty()).ok_or_else(|| bad("samples=[[time, x, y], ...] is required"))?;
    let params = json!({
        "layer": layer,
        "pin": pin,
        "samples": samples,
        "smooth": p["smooth"].as_f64().unwrap_or(2.0),
    });
    let steps = vec![step("comp.open", json!({"comp": view.comp()})), step("puppet.recordPin", params)];
    let out = effectcraft(&view.dir, Some(&project), &steps, true)?;
    Ok(json!({"ok": true, "ecproj": project, "layer": layer, "pin": pin, "samples": samples.len(), "result": out.last()}))
}
