//! `character.puppet.*`: Chained `EffectCraft` Puppet Controls (`puppet.addPin`, `puppet.movePin`,
//! `puppet.setPin`, `puppet.recordPin`, `puppet.follow`, `file.runScript` / `script.run`).

use serde_json::{Value, json};

use super::{View, bad, effectcraft, step};
use crate::{Result, Session};

pub fn cmd_puppet_rig_face(_s: &mut Session, _p: &Value) -> Result<Value> {
    Err(bad("pending Step 2 EffectCraft CLI chain"))
}

pub fn cmd_puppet_expression(_s: &mut Session, _p: &Value) -> Result<Value> {
    Err(bad("pending Step 2 EffectCraft CLI chain"))
}

pub fn cmd_puppet_lipsync(_s: &mut Session, _p: &Value) -> Result<Value> {
    Err(bad("pending Step 2 EffectCraft CLI chain"))
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
    let smoothing = p["smoothing"].as_f64().or_else(|| p["smooth"].as_f64()).unwrap_or(2.0);
    let rec_opts = json!({
        "speed": p["speed"].as_f64().unwrap_or(100.0),
        "smoothing": smoothing,
        "useDraftDeformation": p["draft_deform"].as_bool().or_else(|| p["useDraftDeformation"].as_bool()).unwrap_or(true),
        "showMesh": p["show_mesh"].as_bool().or_else(|| p["showMesh"].as_bool()).unwrap_or(true),
    });
    let params = json!({
        "layer": layer,
        "pin": pin,
        "samples": samples,
        "smoothing": smoothing,
    });
    let steps = vec![step("comp.open", json!({"comp": view.comp()})), step("puppet.recordOptions", rec_opts), step("puppet.recordPin", params)];
    let out = effectcraft(&view.dir, Some(&project), &steps, true)?;
    Ok(json!({"ok": true, "ecproj": project, "layer": layer, "pin": pin, "samples": samples.len(), "result": out.last()}))
}
