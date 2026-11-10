//! First-class `PhotoCraft` subject isolation (`photo.remove_background`), `VectorCraft`
//! subject/stroke tracing (`vector.trace`), and headless `AniCraft` CLI (`anicraft.exec`) commands.

use std::path::PathBuf;

use serde_json::{Value, json};

use crate::{CraftHub, CraftTarget, EngineError, Result, Session};

fn bad(msg: impl Into<String>) -> EngineError {
    EngineError::InvalidParams(msg.into())
}

/// `photo.remove_background`: isolate the subject and remove background via `PhotoCraft`
/// (`layer.removeBackground` + `select.refineEdge`, or `select.colorRange` for `#00FF00` chroma).
pub fn cmd_photo_remove_background(s: &mut Session, p: &Value) -> Result<Value> {
    let raw = p["path"]
        .as_str()
        .or_else(|| p["image"].as_str())
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| bad("photo.remove_background requires path=<image>"))?;
    let input = crate::workspace::resolve_workspace_path(s, raw);
    if !input.is_file() {
        return Err(bad(format!("image does not exist: {}", input.display())));
    }
    let stem = input.file_stem().and_then(|n| n.to_str()).unwrap_or("subject");
    let out = p["out"]
        .as_str()
        .or_else(|| p["output"].as_str())
        .filter(|v| !v.trim().is_empty())
        .map(|raw_out| crate::workspace::resolve_workspace_path(s, raw_out))
        .unwrap_or_else(|| {
            let folder = input.parent().unwrap_or_else(|| std::path::Path::new("."));
            folder.join(format!("{stem}-cutout.png"))
        });
    let chroma = p["chroma"].as_str().filter(|v| !v.trim().is_empty());
    let feather = p["feather"].as_f64().unwrap_or(0.8);
    let mut res = CraftHub::remove_background_photo(&input, &out, chroma, feather)?;
    if p["register_asset"].as_bool().unwrap_or(s.project_path.is_some())
        && s.project_path.is_some()
        && let Ok(attached) = s.execute(
            "project.add_asset",
            json!({
                "path": out,
                "kind": "image",
                "name": p["name"].as_str().unwrap_or(&format!("{stem} Cutout")),
                "if_not_exists": true,
                "load": false,
            }),
        )
    {
        res["asset_id"] = attached["asset_id"].clone();
    }
    Ok(res)
}

/// `vector.trace`: trace an AI-generated or drawn raster image into clean, background-free vector
/// artwork or stroke paths via `VectorCraft` (`imageTrace.make` with `ignoreWhite: true` + `imageTrace.expand`).
pub fn cmd_vector_trace(s: &mut Session, p: &Value) -> Result<Value> {
    let raw = p["path"].as_str().or_else(|| p["image"].as_str()).filter(|v| !v.trim().is_empty()).ok_or_else(|| bad("vector.trace requires path=<image>"))?;
    let input = crate::workspace::resolve_workspace_path(s, raw);
    if !input.is_file() {
        return Err(bad(format!("image does not exist: {}", input.display())));
    }
    let stem = input.file_stem().and_then(|n| n.to_str()).unwrap_or("traced");
    let out: PathBuf = p["out"]
        .as_str()
        .or_else(|| p["output"].as_str())
        .filter(|v| !v.trim().is_empty())
        .map(|raw_out| crate::workspace::resolve_workspace_path(s, raw_out))
        .unwrap_or_else(|| {
            let folder = input.parent().unwrap_or_else(|| std::path::Path::new("."));
            folder.join(format!("{stem}.svg"))
        });
    let preset = p["preset"].as_str().filter(|v| !v.trim().is_empty());
    let ignore_white = p["ignore_white"].as_bool().unwrap_or(true);
    let strokes = p["strokes"].as_bool().unwrap_or(false);
    let mut res = CraftHub::trace_subject_vector(&input, &out, preset, ignore_white, strokes)?;
    if p["register_asset"].as_bool().unwrap_or(s.project_path.is_some())
        && s.project_path.is_some()
        && let Ok(attached) = s.execute(
            "project.add_asset",
            json!({
                "path": out,
                "kind": "image",
                "name": p["name"].as_str().unwrap_or(&format!("{stem} Vector")),
                "if_not_exists": true,
                "load": false,
            }),
        )
    {
        res["asset_id"] = attached["asset_id"].clone();
    }
    Ok(res)
}

/// `anicraft.exec`: invoke `anicraft-cli` strictly headlessly (`info`, `commands`, `serve`,
/// `render`, or `render-seq` -> PNG frames for `EffectCraft`). Never opens the pre-alpha AniCraft GUI.
pub fn cmd_anicraft_exec(s: &mut Session, p: &Value) -> Result<Value> {
    let doc = p["path"].as_str().map(str::trim).filter(|v| !v.is_empty()).map(|raw| crate::workspace::resolve_workspace_path(s, raw));
    let mode = p["mode"].as_str().unwrap_or("exec");
    match mode {
        "render_frame" => {
            let src = doc.ok_or_else(|| bad("anicraft.exec mode=render_frame requires path=<.anicraft>"))?;
            let out_raw = p["out"].as_str().ok_or_else(|| bad("anicraft.exec mode=render_frame requires out=<.png>"))?;
            let out = crate::workspace::resolve_workspace_path(s, out_raw);
            let frame = p["frame"].as_u64().unwrap_or(0) as u32;
            CraftHub::anicraft_render_frame(&src, frame, &out)?;
            Ok(json!({"ok": true, "mode": "render_frame", "frame": out, "via": "anicraft-cli"}))
        }
        "render_seq" => {
            let src = doc.ok_or_else(|| bad("anicraft.exec mode=render_seq requires path=<.anicraft>"))?;
            let out_raw = p["out_dir"].as_str().or_else(|| p["out"].as_str()).ok_or_else(|| bad("anicraft.exec mode=render_seq requires out_dir=<dir>"))?;
            let out_dir = crate::workspace::resolve_workspace_path(s, out_raw);
            let start = p["start"].as_u64().unwrap_or(0) as u32;
            let end = p["end"].as_u64().map(|n| n as u32);
            let frames = CraftHub::anicraft_render_seq(&src, start, end, &out_dir)?;
            Ok(json!({"ok": true, "mode": "render_seq", "out_dir": out_dir, "frames": frames, "via": "anicraft-cli"}))
        }
        _ => {
            let cmd = p["id"].as_str().or_else(|| p["command"].as_str()).unwrap_or("info");
            let params = p.get("params").cloned().unwrap_or_else(|| json!({}));
            let save = p["save"].as_bool().unwrap_or(false);
            let res = if save {
                let src = doc.ok_or_else(|| bad("anicraft.exec save=true requires path=<.anicraft>"))?;
                CraftHub::execute_and_save(CraftTarget::Ani, &src, cmd, &params)?
            } else {
                CraftHub::execute(CraftTarget::Ani, doc.as_deref(), cmd, &params)?
            };
            Ok(json!({"ok": true, "command": cmd, "result": res, "via": "anicraft-cli"}))
        }
    }
}
