//! `character.puppet.*`: Chained `EffectCraft` Puppet Controls (`puppet.addPin`, `puppet.movePin`,
//! `puppet.setPin`, `puppet.recordPin`, `puppet.follow`, `file.runScript` / `script.run`).

use std::path::Path;

use serde_json::{Value, json};

use super::{View, bad, effectcraft, step};
use crate::{Result, Session};

/// Copy supported EffectCraft `puppet.setPin` parameters from `src` to `target`,
/// mapping legacy `stiffness` to `amount` without emitting `stiffness`.
pub(super) fn apply_pin_properties(target: &mut Value, src: &Value) -> bool {
    let mut changed = false;
    for key in ["amount", "extent", "rotation", "scale", "inFront", "position", "time", "frame", "merge"] {
        if let Some(val) = src.get(key) {
            target[key] = val.clone();
            changed = true;
        }
    }
    if target.get("amount").is_none()
        && let Some(stiffness) = src.get("stiffness")
    {
        target["amount"] = stiffness.clone();
        changed = true;
    }
    changed
}

/// `character.puppet.rig_face`: chain EffectCraft facial puppetry (`comp.open`, optional
/// `track.mask` / `track.extractFaceMeasurements`, `puppet.mesh`, `puppet.addPin` / `puppet.setPin`
/// for caller-supplied pins, `paths.pointsFollowNulls` / `paths.nullsFollowPoints`, or delegate
/// to `file.runScript` / `script.run`). Zero hardcoded face pin coordinates!
pub fn cmd_puppet_rig_face(s: &mut Session, p: &Value) -> Result<Value> {
    let view = View::from_params(s, p)?;
    let project = view.require("ecproj", "character.rig")?;
    let layer = p["layer"].as_str().unwrap_or("head");

    if let Some(code) = p["code"].as_str() {
        let steps = vec![step("comp.open", json!({"comp": view.comp()})), step("script.run", json!({"code": code}))];
        let out = effectcraft(&view.dir, Some(&project), &steps, true)?;
        return Ok(json!({"ok": true, "ecproj": project, "layer": layer, "script_run": true, "result": out.last()}));
    }
    if let Some(script_path) = p["script"].as_str().or_else(|| p["script_file"].as_str()) {
        let steps = vec![step("comp.open", json!({"comp": view.comp()})), step("file.runScript", json!({"path": script_path}))];
        let out = effectcraft(&view.dir, Some(&project), &steps, true)?;
        return Ok(json!({"ok": true, "ecproj": project, "layer": layer, "script_file": script_path, "result": out.last()}));
    }

    let mut steps = vec![step("comp.open", json!({"comp": view.comp()}))];

    if p.get("track").is_some() || p.get("tracking_target").is_some() {
        steps.push(step("track.mask", json!({"layer": layer, "mode": "faceDetailed", "method": "faceDetailed"})));
        steps.push(step("track.extractFaceMeasurements", json!({"layer": layer})));
    }

    if p.get("mesh").is_some() || p.get("density").is_some() {
        let mut mesh_opts = json!({"layer": layer});
        for key in ["density", "expansion", "triangles", "showMesh"] {
            if let Some(v) = p.get(key) {
                mesh_opts[key] = v.clone();
            }
        }
        steps.push(step("puppet.mesh", mesh_opts));
    }

    if let Some(pins) = p["pins"].as_array() {
        for (i, pin_val) in pins.iter().enumerate() {
            if let Some(pos) = pin_val.get("position").and_then(super::point) {
                let kind = pin_val["kind"].as_str().unwrap_or("position");
                let mut add = json!({"layer": layer, "kind": kind, "position": pos});
                if let Some(name) = pin_val["name"].as_str() {
                    add["name"] = json!(name);
                }
                steps.push(step("puppet.addPin", add));
            } else if let Some(pos) = super::point(pin_val) {
                let name = format!("Face Pin {}", i + 1);
                steps.push(step("puppet.addPin", json!({"layer": layer, "kind": "position", "position": pos, "name": name})));
            } else if let Some(pin_name) = pin_val["pin"].as_str().or_else(|| pin_val["name"].as_str()) {
                let mut edit = json!({"layer": layer, "pin": pin_name});
                apply_pin_properties(&mut edit, pin_val);
                steps.push(step("puppet.setPin", edit));
            }
        }
    }

    let mode = p["mode"].as_str().unwrap_or_default();
    if p["points_follow_nulls"].as_bool() == Some(true) || mode == "points_follow_nulls" {
        steps.push(step("paths.pointsFollowNulls", json!({"layer": layer})));
    } else if p["nulls_follow_points"].as_bool() == Some(true) || mode == "nulls_follow_points" {
        steps.push(step("paths.nullsFollowPoints", json!({"layer": layer})));
    }

    let out = effectcraft(&view.dir, Some(&project), &steps, true)?;
    Ok(json!({"ok": true, "ecproj": project, "layer": layer, "steps_executed": steps.len(), "result": out.last()}))
}

/// `character.puppet.expression`: chain EffectCraft expression setup and pin edits (`comp.open`,
/// `prop.setExpression`, `puppet.setPin` with amount/extent/rotation/scale/inFront,
/// `puppet.movePin`, or `file.runScript` / `script.run`). Zero hardcoded expression delta tables!
pub fn cmd_puppet_expression(s: &mut Session, p: &Value) -> Result<Value> {
    let view = View::from_params(s, p)?;
    let project = view.require("ecproj", "character.rig")?;
    let layer = p["layer"].as_str().unwrap_or("head");

    if let Some(code) = p["code"].as_str() {
        let steps = vec![step("comp.open", json!({"comp": view.comp()})), step("script.run", json!({"code": code}))];
        let out = effectcraft(&view.dir, Some(&project), &steps, true)?;
        return Ok(json!({"ok": true, "ecproj": project, "layer": layer, "script_run": true, "result": out.last()}));
    }
    if let Some(script_path) = p["script"].as_str().or_else(|| p["script_file"].as_str()) {
        let steps = vec![step("comp.open", json!({"comp": view.comp()})), step("file.runScript", json!({"path": script_path}))];
        let out = effectcraft(&view.dir, Some(&project), &steps, true)?;
        return Ok(json!({"ok": true, "ecproj": project, "layer": layer, "script_file": script_path, "result": out.last()}));
    }

    let mut steps = vec![step("comp.open", json!({"comp": view.comp()}))];

    if let Some(expr) = p["expression"].as_str()
        && let Some(path) = p["path"].as_str()
    {
        steps.push(step("prop.setExpression", json!({"layer": layer, "path": path, "expression": expr})));
    }

    if let Some(pin_name) = p["pin"].as_str() {
        let mut set_params = json!({"layer": layer, "pin": pin_name});
        if apply_pin_properties(&mut set_params, p) {
            steps.push(step("puppet.setPin", set_params));
        } else if let Some(pos) = p.get("position").and_then(super::point) {
            steps.push(step("puppet.movePin", json!({"layer": layer, "pin": pin_name, "position": pos})));
        }
    }

    if let Some(pins) = p["pins"].as_array() {
        for pin_val in pins {
            if let Some(pin_name) = pin_val["pin"].as_str().or_else(|| pin_val["name"].as_str()) {
                let mut set_params = json!({"layer": layer, "pin": pin_name});
                apply_pin_properties(&mut set_params, pin_val);
                steps.push(step("puppet.setPin", set_params));
            }
        }
    }

    let out = effectcraft(&view.dir, Some(&project), &steps, true)?;
    Ok(json!({"ok": true, "ecproj": project, "layer": layer, "steps_executed": steps.len(), "result": out.last()}))
}

/// `character.puppet.lipsync`: chain EffectCraft audio amplitude driving (`comp.open`,
/// `keys.audioToKeyframes`, `prop.setExpression` driving target pin/property from
/// `[Audio Amplitude]` keyframes, or `file.runScript` / `script.run`). Zero vowel/viseme tables!
pub fn cmd_puppet_lipsync(s: &mut Session, p: &Value) -> Result<Value> {
    let view = View::from_params(s, p)?;
    let project = view.require("ecproj", "character.rig")?;
    let layer = p["layer"].as_str().unwrap_or("mouth");

    if let Some(code) = p["code"].as_str() {
        let steps = vec![step("comp.open", json!({"comp": view.comp()})), step("script.run", json!({"code": code}))];
        let out = effectcraft(&view.dir, Some(&project), &steps, true)?;
        return Ok(json!({"ok": true, "ecproj": project, "layer": layer, "script_run": true, "result": out.last()}));
    }
    if let Some(script_path) = p["script"].as_str().or_else(|| p["script_file"].as_str()) {
        let steps = vec![step("comp.open", json!({"comp": view.comp()})), step("file.runScript", json!({"path": script_path}))];
        let out = effectcraft(&view.dir, Some(&project), &steps, true)?;
        return Ok(json!({"ok": true, "ecproj": project, "layer": layer, "script_file": script_path, "result": out.last()}));
    }

    let mut steps = vec![step("comp.open", json!({"comp": view.comp()})), step("keys.audioToKeyframes", json!({"comp": view.comp()}))];

    let path = p["path"].as_str().unwrap_or("transform/scale");
    let default_expr = "thisComp.layer('Audio Amplitude').effect('Both Channels')('Slider')";
    let expr = p["expression"].as_str().unwrap_or(default_expr);
    steps.push(step("prop.setExpression", json!({"layer": layer, "path": path, "expression": expr})));

    let out = effectcraft(&view.dir, Some(&project), &steps, true)?;
    Ok(json!({"ok": true, "ecproj": project, "layer": layer, "path": path, "result": out.last()}))
}

/// `character.puppet.follow`: attach EffectCraft's native `puppet.follow` secondary cascade
/// (`layer?`, `leader?`, `pins?`, `delay`, `amount`, `cascade`) to any hair, ear, tail, cape, or limb layer.
pub fn cmd_puppet_follow(s: &mut Session, p: &Value) -> Result<Value> {
    let view = View::from_params(s, p)?;
    let project = view.require("ecproj", "character.rig")?;
    let mut params = json!({
        "delay": p["delay"].as_f64().unwrap_or(0.1),
        "amount": p["amount"].as_u64().unwrap_or(100),
        "cascade": p["cascade"].as_bool().unwrap_or(true),
    });
    if let Some(layer) = p["layer"].as_str().filter(|v| !v.trim().is_empty()) {
        params["layer"] = json!(layer);
    }
    if let Some(leader) = p["leader"].as_str().filter(|v| !v.trim().is_empty()) {
        params["leader"] = json!(leader);
    }
    if let Some(pins) = p.get("pins").and_then(Value::as_array) {
        if pins.is_empty() {
            return Err(bad("pins must not be empty when specified"));
        }
        params["pins"] = json!(pins);
    }
    let steps = vec![step("comp.open", json!({"comp": view.comp()})), step("puppet.follow", params.clone())];
    let out = effectcraft(&view.dir, Some(&project), &steps, true)?;
    Ok(json!({
        "ok": true,
        "ecproj": project,
        "layer": params.get("layer"),
        "leader": params.get("leader"),
        "result": out.last(),
    }))
}

/// `character.puppet.record`: record a real-time Puppet pin gesture path (`samples: [[t, x, y], ...]`)
/// onto a pin, pins, or selected pins via `EffectCraft` `puppet.recordPin`.
pub fn cmd_puppet_record(s: &mut Session, p: &Value) -> Result<Value> {
    let view = View::from_params(s, p)?;
    let project = view.require("ecproj", "character.rig")?;
    let samples = p["samples"].as_array().filter(|a| !a.is_empty()).ok_or_else(|| bad("samples=[[time, x, y], ...] is required"))?;
    let smoothing = p["smoothing"].as_f64().or_else(|| p["smooth"].as_f64()).unwrap_or(2.0);
    let rec_opts = json!({
        "speed": p["speed"].as_f64().unwrap_or(100.0),
        "smoothing": smoothing,
        "useDraftDeformation": p["draft_deform"].as_bool().or_else(|| p["useDraftDeformation"].as_bool()).unwrap_or(true),
        "showMesh": p["show_mesh"].as_bool().or_else(|| p["showMesh"].as_bool()).unwrap_or(true),
    });
    let mut params = json!({
        "samples": samples,
        "smoothing": smoothing,
    });
    if let Some(layer) = p["layer"].as_str().filter(|v| !v.trim().is_empty()) {
        params["layer"] = json!(layer);
    }
    if let Some(pin) = p["pin"].as_str().filter(|v| !v.trim().is_empty()) {
        params["pin"] = json!(pin);
    }
    if let Some(pins) = p["pins"].as_array().filter(|a| !a.is_empty()) {
        params["pins"] = json!(pins);
    }
    if let Some(start) = p["start"].as_f64() {
        params["start"] = json!(start);
    }
    if let Some(speed) = p["speed"].as_f64() {
        params["speed"] = json!(speed);
    }
    let steps = vec![step("comp.open", json!({"comp": view.comp()})), step("puppet.recordOptions", rec_opts), step("puppet.recordPin", params.clone())];
    let out = effectcraft(&view.dir, Some(&project), &steps, true)?;
    Ok(json!({
        "ok": true,
        "ecproj": project,
        "layer": params.get("layer"),
        "pin": params.get("pin"),
        "samples": samples.len(),
        "result": out.last(),
    }))
}

/// `character.script.list`: list installed and sample scripts in EffectCraft (`file.scripts.list`).
pub fn cmd_script_list(s: &mut Session, p: &Value) -> Result<Value> {
    let view = View::from_params(s, p).ok();
    let dir = view.as_ref().map(|v| v.dir.as_path()).unwrap_or_else(|| Path::new("."));
    let project = view.as_ref().and_then(|v| v.file("ecproj").is_file().then(|| v.file("ecproj")));
    let steps = vec![step("file.scripts.list", json!({}))];
    let out = effectcraft(dir, project.as_deref(), &steps, false)?;
    Ok(json!({"ok": true, "scripts": out.last()}))
}

/// `character.script.install`: install a script or ScriptUI panel into EffectCraft
/// (`file.installScript` or `file.installScriptUIPanel`).
pub fn cmd_script_install(_s: &mut Session, p: &Value) -> Result<Value> {
    let path = p["path"].as_str().ok_or_else(|| bad("path=<script file> is required"))?;
    let is_panel = p["ui_panel"].as_bool().or_else(|| p["panel"].as_bool()).unwrap_or(false);
    let cmd_id = if is_panel { "file.installScriptUIPanel" } else { "file.installScript" };
    let steps = vec![step(cmd_id, json!({"path": path}))];
    let out = effectcraft(Path::new("."), None, &steps, false)?;
    Ok(json!({"ok": true, "path": path, "panel": is_panel, "result": out.last()}))
}

/// `character.script.run`: execute JavaScript code or run installed/file/recent scripts or step lists
/// in EffectCraft (`script.run` or `file.runScript`).
pub fn cmd_script_run(s: &mut Session, p: &Value) -> Result<Value> {
    let view = View::from_params(s, p).ok();
    let dir = view.as_ref().map(|v| v.dir.as_path()).unwrap_or_else(|| Path::new("."));
    let project = view.as_ref().and_then(|v| v.file("ecproj").is_file().then(|| v.file("ecproj")));
    let mut steps = Vec::new();
    if let Some(comp) = view.as_ref().map(|v| v.comp()) {
        steps.push(step("comp.open", json!({"comp": comp})));
    }
    if let Some(code) = p["code"].as_str().filter(|c| !c.trim().is_empty()) {
        let mut req = json!({"code": code});
        if let Some(undo_name) = p["undoName"].as_str().or_else(|| p["undo_name"].as_str()) {
            req["undoName"] = json!(undo_name);
        }
        steps.push(step("script.run", req));
    } else {
        let mut req = json!({});
        let mut has_target = false;
        if let Some(path) = p["path"].as_str().or_else(|| p["file"].as_str()).filter(|v| !v.trim().is_empty()) {
            req["path"] = json!(path);
            has_target = true;
        }
        if let Some(name) = p["name"].as_str().filter(|v| !v.trim().is_empty()) {
            req["name"] = json!(name);
            has_target = true;
        }
        if let Some(recent) = p["recent"].as_u64() {
            req["recent"] = json!(recent);
            has_target = true;
        }
        if let Some(steps_arr) = p["steps"].as_array() {
            req["steps"] = json!(steps_arr);
            has_target = true;
        }
        if !has_target {
            return Err(bad("script.run requires 'code', 'path', 'name', 'recent', or 'steps'"));
        }
        steps.push(step("file.runScript", req));
    }
    let out = effectcraft(dir, project.as_deref(), &steps, true)?;
    Ok(json!({"ok": true, "result": out.last()}))
}

/// `character.puppet.info`: inspect puppet mesh status and deformation details (`puppet.info` / `puppet.mesh`).
pub fn cmd_puppet_info(s: &mut Session, p: &Value) -> Result<Value> {
    let view = View::from_params(s, p)?;
    let project = view.require("ecproj", "character.rig")?;
    let mut steps = vec![step("comp.open", json!({"comp": view.comp()}))];
    let mut info_params = json!({});
    if let Some(l) = p["layer"].as_str() {
        info_params["layer"] = json!(l);
    }
    if let Some(t) = p.get("time") {
        info_params["time"] = t.clone();
    }
    if let Some(f) = p.get("frame") {
        info_params["frame"] = f.clone();
    }
    steps.push(step("puppet.info", info_params));
    let out = effectcraft(&view.dir, Some(&project), &steps, false)?;
    Ok(json!({"ok": true, "ecproj": project, "comp": view.comp(), "info": out.last()}))
}
