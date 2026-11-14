//! `character.rig` & `character.bone.*`: EffectCraft builds 2.5D puppet rigs from cut PhotoCraft documents,
//! while `AniCraft` rigs and controls 2D artboard armatures (`bone.add`, `bone.remove`, `bone.set_limits`,
//! `bone.pin`, `bone.pose`, `bone.solve`, `bone.info`).

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::{View, bad, effectcraft, step};
use crate::{EngineError, Result, Session};

fn anicraft_exec(target: &Path, command_id: &str, params: &Value, save: bool) -> Result<Value> {
    if !target.is_file() {
        return Err(bad(format!("anicraft file not found: {}", target.display())));
    }
    let target_s = target.to_string_lossy().into_owned();
    let params_s = serde_json::to_string(params).map_err(|e| bad(e.to_string()))?;
    let mut args = vec!["exec", target_s.as_str(), command_id, params_s.as_str()];
    if save {
        args.extend(["--save", target_s.as_str()]);
    }
    crate::workspace::run_cli_json("anicraft-cli", &args, None).map_err(|e| EngineError::ExternalTool("anicraft-cli".into(), e))
}

fn resolve_anicraft_doc(s: &Session, p: &Value) -> Result<(String, String, PathBuf)> {
    if let Some(path_str) = p["path"].as_str().or_else(|| p["file"].as_str()) {
        let resolved = crate::workspace::resolve_workspace_path(s, path_str);
        let name = resolved.file_stem().and_then(|n| n.to_str()).unwrap_or("character");
        let cid = p["character"].as_str().map(crate::workspace::sanitize_asset_id).unwrap_or_else(|| crate::workspace::sanitize_asset_id(name));
        return Ok((cid, name.to_string(), resolved));
    }
    let view = View::from_params(s, p)?;
    let target = view.file("anicraft");
    if !target.is_file() {
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let target_s = target.to_string_lossy().into_owned();
        let _ = crate::workspace::run_cli("anicraft-cli", &["new", "--out", target_s.as_str(), "--width", "1280", "--height", "720", "--fps", "24"], None);
    }
    Ok((view.id, view.folder_name, target))
}

/// `character.rig`: build a layered puppet rig in EffectCraft (.ecproj) from a cut PhotoCraft document.
pub fn cmd_character_rig(s: &mut Session, p: &Value) -> Result<Value> {
    let view = View::from_params(s, p)?;
    let photo_source = view.require_photo_doc("character.cut")?;
    let project = view.file("ecproj");

    let mut steps = vec![
        step("file.import", json!({"paths": [photo_source], "importAs": "compositionLayerSizes"})),
        step("comp.open", json!({"comp": view.comp()})),
        step("comp.settings", json!({"frameRate": 24})),
    ];
    if let Some(script_path) = p["script"].as_str() {
        steps.push(step("file.runScript", json!({"path": script_path})));
    }
    if let Some(code) = p["code"].as_str() {
        steps.push(step("script.run", json!({"code": code})));
    }

    effectcraft(&view.dir, Some(&project), &steps, true)?;
    super::manage::sync_character_registration(s, &view.id, &view.folder_name, Some("rig_2_5d"), Some(&project), None, None);

    Ok(json!({
        "ok": true,
        "ecproj": project,
        "source": photo_source,
        "comp": view.comp(),
        "next": format!("character.action character={} view={} action=idle", view.id, view.view)
    }))
}

/// `character.bone.add`: add an IK/FK bone between two element instances (`bone.add` in AniCraft).
pub fn cmd_bone_add(s: &mut Session, p: &Value) -> Result<Value> {
    let (cid, name, anicraft_path) = resolve_anicraft_doc(s, p)?;
    let from = p.get("from").ok_or_else(|| bad("from=<instance id> is required"))?;
    let to = p.get("to").ok_or_else(|| bad("to=<instance id> is required"))?;
    let mut params = json!({"from": from, "to": to});
    if let Some(head) = p.get("head") {
        params["head"] = head.clone();
    }
    if let Some(tail) = p.get("tail") {
        params["tail"] = tail.clone();
    }
    if let Some(bone_name) = p.get("name") {
        params["name"] = bone_name.clone();
    }
    let res = anicraft_exec(&anicraft_path, "bone.add", &params, true)?;
    super::manage::sync_character_registration(s, &cid, &name, Some("artboard_2d"), None, Some(&anicraft_path), None);
    Ok(json!({"ok": true, "character": cid, "anicraft": anicraft_path, "result": res}))
}

/// `character.bone.remove`: remove a bone and descendant joints (`bone.remove` in AniCraft).
pub fn cmd_bone_remove(s: &mut Session, p: &Value) -> Result<Value> {
    let (cid, _name, anicraft_path) = resolve_anicraft_doc(s, p)?;
    let bone = p.get("bone").ok_or_else(|| bad("bone=<bone id or name> is required"))?;
    let params = json!({"bone": bone});
    let res = anicraft_exec(&anicraft_path, "bone.remove", &params, true)?;
    Ok(json!({"ok": true, "character": cid, "anicraft": anicraft_path, "bone": bone, "result": res}))
}

/// `character.bone.setLimits`: configure bone joint angle constraints (`bone.set_limits` in AniCraft).
pub fn cmd_bone_set_limits(s: &mut Session, p: &Value) -> Result<Value> {
    let (cid, _name, anicraft_path) = resolve_anicraft_doc(s, p)?;
    let bone = p.get("bone").ok_or_else(|| bad("bone=<bone id or name> is required"))?;
    let mut params = json!({"bone": bone});
    if let Some(c) = p.get("constrain") {
        params["constrain"] = c.clone();
    }
    if let Some(min) = p.get("min") {
        params["min"] = min.clone();
    }
    if let Some(max) = p.get("max") {
        params["max"] = max.clone();
    }
    let res = anicraft_exec(&anicraft_path, "bone.set_limits", &params, true)?;
    Ok(json!({"ok": true, "character": cid, "anicraft": anicraft_path, "bone": bone, "result": res}))
}

/// `character.bone.pin`: pin or unpin an armature joint (`bone.pin` in AniCraft).
pub fn cmd_bone_pin(s: &mut Session, p: &Value) -> Result<Value> {
    let (cid, _name, anicraft_path) = resolve_anicraft_doc(s, p)?;
    let bone = p.get("bone").ok_or_else(|| bad("bone=<bone id or name> is required"))?;
    let pinned = p["pinned"].as_bool().unwrap_or(true);
    let params = json!({"bone": bone, "pinned": pinned});
    let res = anicraft_exec(&anicraft_path, "bone.pin", &params, true)?;
    Ok(json!({"ok": true, "character": cid, "anicraft": anicraft_path, "bone": bone, "pinned": pinned, "result": res}))
}

/// `character.bone.pose`: keyframe bone joint pose or translation at a timeline frame (`bone.pose` in AniCraft).
pub fn cmd_bone_pose(s: &mut Session, p: &Value) -> Result<Value> {
    let (cid, _name, anicraft_path) = resolve_anicraft_doc(s, p)?;
    let mut params = json!({});
    for k in ["bone", "angle", "rotate_by", "dx", "dy", "layer", "frame"] {
        if let Some(v) = p.get(k) {
            params[k] = v.clone();
        }
    }
    let res = anicraft_exec(&anicraft_path, "bone.pose", &params, true)?;
    Ok(json!({"ok": true, "character": cid, "anicraft": anicraft_path, "result": res}))
}

/// `character.bone.solve`: perform inverse kinematics (IK) solve by dragging a bone end (`bone.solve` in AniCraft).
pub fn cmd_bone_solve(s: &mut Session, p: &Value) -> Result<Value> {
    let (cid, _name, anicraft_path) = resolve_anicraft_doc(s, p)?;
    let bone = p.get("bone").ok_or_else(|| bad("bone=<bone id or name> is required"))?;
    let x = p.get("x").ok_or_else(|| bad("x=<stage px> is required"))?;
    let y = p.get("y").ok_or_else(|| bad("y=<stage px> is required"))?;
    let mut params = json!({"bone": bone, "x": x, "y": y});
    if let Some(local) = p.get("local") {
        params["local"] = local.clone();
    }
    if let Some(frame) = p.get("frame") {
        params["frame"] = frame.clone();
    }
    let res = anicraft_exec(&anicraft_path, "bone.solve", &params, true)?;
    Ok(json!({"ok": true, "character": cid, "anicraft": anicraft_path, "result": res}))
}

/// `character.bone.info`: inspect all armatures, bones, joints, limits, and pin states (`bone.info` in AniCraft).
pub fn cmd_bone_info(s: &mut Session, p: &Value) -> Result<Value> {
    let (cid, _name, anicraft_path) = resolve_anicraft_doc(s, p)?;
    let mut params = json!({});
    if let Some(frame) = p.get("frame") {
        params["frame"] = frame.clone();
    }
    if let Some(layer) = p.get("layer") {
        params["layer"] = layer.clone();
    }
    let res = anicraft_exec(&anicraft_path, "bone.info", &params, false)?;
    Ok(json!({"ok": true, "character": cid, "anicraft": anicraft_path, "info": res}))
}
