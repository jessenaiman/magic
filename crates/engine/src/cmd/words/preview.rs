//! Saved kinetic-lyric thumbnails rendered directly by `EffectCraft` (`effectcraft-cli`), with
//! `kind="motion"` delegated to [`crate::cmd::motion::cmd_motion_preview_frame`] so Lyrics and Motion
//! never share the same `.ecproj` filmstrip frames and Magic never performs in-process pixel post-processing.

use serde_json::{Value, json};

use crate::{EngineError, Result, Session};

fn choose_sample_time(layers: &[Value], layer_type: &str, requested: f64) -> f64 {
    let mut best_span: Option<(f64, f64, f64)> = None;
    for l in layers.iter().filter(|l| l["type"] == layer_type) {
        let l_in = l["in"].as_f64().unwrap_or(0.0);
        let l_out = l["out"].as_f64().unwrap_or(l_in + 1.0).max(l_in + 0.05);
        let dist = if requested < l_in {
            l_in - requested
        } else if requested > l_out {
            requested - l_out
        } else {
            0.0
        };
        if best_span.is_none_or(|(_, _, best_dist)| dist < best_dist) {
            best_span = Some((l_in, l_out, dist));
        }
    }
    let Some((l_in, l_out, dist)) = best_span else {
        return requested;
    };
    let span = (l_out - l_in).max(0.1);
    let enter_done = (l_in + (span * 0.35).min(0.45)).min(l_out - 0.04);
    if dist > 0.0 { enter_done } else { requested.clamp(enter_done, (l_out - 0.04).max(enter_done)) }
}

pub fn frame(s: &mut Session, p: &Value) -> Result<Value> {
    let kind = p["kind"].as_str().unwrap_or("lyrics");
    if kind == "motion" {
        return crate::cmd::motion::cmd_motion_preview_frame(s, p);
    }
    let time =
        p["time"].as_f64().filter(|v| v.is_finite() && *v >= 0.0).ok_or_else(|| EngineError::InvalidParams("time must be nonnegative seconds".into()))?;
    if !matches!(kind, "lyrics" | "stage") {
        return Err(EngineError::InvalidParams("kind must be lyrics, motion, or stage".into()));
    }
    let dir = super::workspace_dir(s)?;
    let project = dir.join("lettering/lyrics.ecproj");
    if !project.is_file() {
        let _ = super::lettering::cmd_words_letter(s, &json!({"render": false}));
    }
    if !project.is_file() {
        return Err(EngineError::InvalidParams("No saved EffectCraft lyrics composition; save the lyric design first".into()));
    }
    let folder = dir.join("lettering/.preview");
    std::fs::create_dir_all(&folder)?;
    let out = folder.join(format!("{kind}-{}.png", (time * 1000.0).round() as u64));
    if out.is_file()
        && let (Ok(out_meta), Ok(proj_meta)) = (std::fs::metadata(&out), std::fs::metadata(&project))
        && out_meta.len() > 0
        && let (Ok(out_time), Ok(proj_time)) = (out_meta.modified(), proj_meta.modified())
        && out_time >= proj_time
    {
        return Ok(json!({"frame": out, "source": project, "kind": kind, "time": time, "sample_time": time, "native": "effectcraft"}));
    }
    if kind == "stage" {
        let scale = p["scale"].as_f64().unwrap_or(0.67).clamp(0.25, 1.0);
        crate::CraftHub::render_effect_frame(&project, Some("Lyrics"), time, (1920.0 * scale).round() as u32, &out)?;
        return Ok(json!({"frame": out, "source": project, "kind": "stage", "time": time, "sample_time": time, "native": "effectcraft"}));
    }
    let comp: Value = crate::CraftHub::execute(crate::CraftTarget::Effect, Some(&project), "comp.info", &json!({"comp": "Lyrics"}))
        .or_else(|_| crate::CraftHub::execute(crate::CraftTarget::Effect, Some(&project), "comp.info", &json!({})))?;
    let layers = comp["layers"].as_array().ok_or_else(|| EngineError::InvalidParams("EffectCraft returned no active composition layers".into()))?;
    let ids: Vec<Value> = layers.iter().filter(|l| l["type"] == "Text").map(|l| l["id"].clone()).collect();
    if ids.is_empty() {
        return Ok(json!({"empty": true, "kind": "lyrics", "time": time, "source": project}));
    }
    let sample_time = choose_sample_time(layers, "Text", time);
    crate::workspace::cmd_effect_exec(
        s,
        &json!({
            "path": project,
            "save": false,
            "steps": [
                {"id": "comp.open", "params": {"comp": comp["id"]}},
                {"id": "layer.setSwitch", "params": {"layers": ids, "switch": "video", "value": false, "others": true}},
                {"id": "comp.saveFrameAs", "params": {"path": out, "time": sample_time, "scale": 0.35, "transparent": true}},
                {"id": "layer.setSwitch", "params": {"layers": ids, "switch": "video", "value": true, "others": false}},
            ]
        }),
    )?;
    if !out.is_file() {
        return Err(EngineError::InvalidParams("EffectCraft did not produce the preview frame".into()));
    }
    Ok(json!({"frame": out, "source": project, "kind": "lyrics", "time": time, "sample_time": sample_time, "native": "effectcraft"}))
}
