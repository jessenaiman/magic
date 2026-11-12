//! `character.mark`: prepares `parts.json` joint pivots and cutout polygons from an existing
//! `parts/rig.json` manifest or deterministic `plush-9` character rig slots (`magic_scribbles::plush9_part_slots`)
//! for review on the canvas before `character.cut` (`PhotoCraft`). Zero external CLI agent subprocesses.

use serde_json::{Value, json};

use super::{POSES, View, bad, point};
use crate::{Result, Session};

/// Copies the drawing beside `parts.json` and measures pixel dimensions via `PhotoCraft` or `image`.
fn stage_image(view: &View, p: &Value, s: &Session) -> Result<(std::path::PathBuf, (u32, u32))> {
    let source = p["image"]
        .as_str()
        .filter(|i| !i.trim().is_empty())
        .map(|i| crate::workspace::resolve_workspace_path(s, i))
        .filter(|i| i.is_file())
        .or_else(|| view.dir.parent().and_then(super::find_initial_cutout))
        .ok_or_else(|| bad("image=<drawing.png> is required"))?;
    std::fs::create_dir_all(&view.dir)?;
    let staged = view.dir.join(format!("{}-{}-drawing.png", view.id, view.view));
    if source != staged {
        std::fs::copy(&source, &staged)?;
    }
    let dims = super::cut::photocraft_dimensions(&staged)
        .or_else(|_| image::image_dimensions(&staged).map_err(|e| bad(e.to_string())))?;
    Ok((staged, dims))
}

/// Optional body-plate redraw when `redraw=true` is requested: delegates image generation through
/// `crate::craft_ai::cmd_image_generate` (`storytold/craft-ai-integrations`) and resizes in `PhotoCraft`.
fn ensure_redrawn_body_plate(
    s: &mut Session,
    view: &View,
    size: (u32, u32),
    p: &Value,
) -> Result<Option<std::path::PathBuf>> {
    let Some(char_dir) = view.dir.parent() else {
        return Ok(None);
    };
    let plate = char_dir.join("parts/source/body-plate_v1.png");
    if plate.is_file() && p["force_redraw"] != true {
        return Ok(Some(plate));
    }
    if p["redraw"] != true {
        return Ok(plate.is_file().then_some(plate));
    }
    let prompt = p["redraw_prompt"].as_str().map(str::to_string).unwrap_or_else(|| {
        format!(
            "Clean felt and cut-paper body-plate infill for character '{}' ({}x{} px). Keep the exact body silhouette \
             and texture while omitting overlapping foreground limbs so the torso underneath rotates cleanly.",
            view.id, size.0, size.1
        )
    });
    let generated = crate::craft_ai::cmd_image_generate(
        s,
        &json!({"prompt": prompt, "name": format!("{}-body-plate", view.id)}),
    )?;
    if let Some(raw_img) = generated["image"].as_str() {
        let src = std::path::Path::new(raw_img);
        if src.is_file() {
            if let Some(parent) = plate.parent() {
                std::fs::create_dir_all(parent)?;
            }
            super::cut::photocraft_run(
                src,
                &[
                    ("image.imageSize", json!({"width": size.0, "height": size.1})),
                    ("file.saveACopy", json!({"path": &plate, "overwrite": true})),
                ],
                None,
            )?;
            return Ok(Some(plate));
        }
    }
    Ok(plate.is_file().then_some(plate))
}

fn box_polygon(b: &[Value], size: (u32, u32)) -> Option<Value> {
    if b.len() != 4 {
        return None;
    }
    let x = b[0].as_f64()?.round() as i64;
    let y = b[1].as_f64()?.round() as i64;
    let w = b[2].as_f64()?.round() as i64;
    let h = b[3].as_f64()?.round() as i64;
    let x1 = (x + w).clamp(0, i64::from(size.0));
    let y1 = (y + h).clamp(0, i64::from(size.1));
    let x0 = x.clamp(0, x1);
    let y0 = y.clamp(0, y1);
    Some(json!([[x0, y0], [x1, y0], [x1, y1], [x0, y1]]))
}

fn import_rig_marks(s: &mut Session, view: &View, p: &Value) -> Result<Option<Value>> {
    let Some(char_dir) = view.dir.parent() else {
        return Ok(None);
    };
    let rig_path = char_dir.join("parts/rig.json");
    if !rig_path.is_file() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&rig_path)?;
    let rig: Value = serde_json::from_str(&text).map_err(|e| bad(format!("{}: {e}", rig_path.display())))?;
    let (staged, size) = stage_image(view, p, s)?;
    let plate = ensure_redrawn_body_plate(s, view, size, p)?;
    let anim_dir = char_dir.join("parts/animation-layers");
    let layers = rig["layers"].as_array().cloned().unwrap_or_default();
    let seg_layers = &rig["segmented_source"]["layers"];
    let skeleton: Value = serde_json::from_str(POSES).map_err(|e| bad(e.to_string()))?;
    let known = skeleton["parts"].as_array().cloned().unwrap_or_default();
    let mut parts_arr = Vec::new();
    for layer in &layers {
        let Some(id) = layer["id"].as_str() else { continue };
        if !known.contains(&json!(id)) {
            continue;
        }
        let parent = layer.get("parent").cloned().unwrap_or(Value::Null);
        let joint = point(&layer["pivot"])
            .or_else(|| point(&seg_layers[id]["pivot"]))
            .unwrap_or([f64::from(size.0) / 2.0, f64::from(size.1) / 2.0]);
        let mut entry = json!({
            "id": id,
            "parent": parent,
            "joint": [joint[0].round(), joint[1].round()],
        });
        let candidate = anim_dir.join(format!("{id}.png"));
        if candidate.is_file() {
            entry["source"] = json!(candidate);
        }
        let outline = seg_layers[id]["box"]
            .as_array()
            .and_then(|b| box_polygon(b, size))
            .unwrap_or_else(|| json!([[0, 0], [size.0, 0], [size.0, size.1], [0, size.1]]));
        entry["outline"] = outline;
        parts_arr.push(entry);
    }
    if parts_arr.is_empty() {
        return Ok(None);
    }
    let mut doc = json!({
        "image": staged.file_name().map(|n| n.to_string_lossy().into_owned()),
        "view": view.view,
        "size": [size.0, size.1],
        "skeleton": skeleton["skeleton"],
        "status": "imported from parts/rig.json; ready for character.cut",
        "parts": parts_arr,
    });
    if let Some(plate_path) = plate {
        doc["body_plate"] = json!(plate_path);
    }
    view.write_parts(&doc)?;
    Ok(Some(doc))
}

fn build_plush9_draft_marks(view: &View, staged: &std::path::Path, size: (u32, u32)) -> Result<Value> {
    let skeleton: Value = serde_json::from_str(POSES).map_err(|e| bad(e.to_string()))?;
    let slots = magic_scribbles::plush9_part_slots(size.0, size.1);
    let (w, h) = (f64::from(size.0.max(1)), f64::from(size.1.max(1)));
    let pad_x = (w * 0.12).round() as i64;
    let pad_y = (h * 0.10).round() as i64;
    let parts_arr: Vec<Value> = slots
        .into_iter()
        .map(|mut slot| {
            let jx = slot["joint"][0].as_f64().unwrap_or(w * 0.5).round() as i64;
            let jy = slot["joint"][1].as_f64().unwrap_or(h * 0.5).round() as i64;
            let x0 = (jx - pad_x).clamp(0, i64::from(size.0));
            let y0 = (jy - pad_y).clamp(0, i64::from(size.1));
            let x1 = (jx + pad_x).clamp(x0, i64::from(size.0));
            let y1 = (jy + pad_y).clamp(y0, i64::from(size.1));
            slot["outline"] = json!([[x0, y0], [x1, y0], [x1, y1], [x0, y1]]);
            slot
        })
        .collect();
    let doc = json!({
        "image": staged.file_name().map(|n| n.to_string_lossy().into_owned()),
        "view": view.view,
        "size": [size.0, size.1],
        "skeleton": skeleton["skeleton"],
        "status": "draft plush-9 rig marks; adjust joints/outlines before character.cut",
        "parts": parts_arr,
    });
    view.write_parts(&doc)?;
    Ok(doc)
}

pub fn cmd_character_mark(s: &mut Session, p: &Value) -> Result<Value> {
    let view = View::from_params(s, p)?;
    let parts = if let Some(imported) = import_rig_marks(s, &view, p)? {
        imported
    } else {
        let (staged, size) = stage_image(&view, p, s)?;
        let _ = ensure_redrawn_body_plate(s, &view, size, p)?;
        build_plush9_draft_marks(&view, &staged, size)?
    };
    let mut reply = json!({
        "ok": true,
        "parts": view.dir.join("parts.json"),
        "marked": parts["parts"],
        "body_plate": parts.get("body_plate"),
        "next": format!("review the marks, then character.cut character={} view={}", view.id, view.view),
    });
    if p["cut"] == true || p["rig"] == true {
        let mut cut_params = p.clone();
        if let Some(obj) = cut_params.as_object_mut() {
            obj.remove("parts");
        }
        let cut_res = super::cmd_character_cut(s, &cut_params)?;
        reply["psd"] = cut_res["psd"].clone();
        reply["pcraft"] = cut_res["pcraft"].clone();
        reply["ecproj"] = cut_res["ecproj"].clone();
        reply["next"] = cut_res["next"].clone();
    }
    Ok(reply)
}
