//! `character.cut`: `PhotoCraft` isolates the subject (`layer.removeBackground` / `select.refineEdge`)
//! and cuts each marked part onto its own layer, while `VectorCraft` (`imageTrace.make` with
//! `ignoreWhite: true` + `imageTrace.expand`) traces clean polygon silhouettes — routed via [`CraftHub`].

use serde_json::{Value, json};

use super::{View, bad, part_list, root_part};
use crate::{CraftHub, CraftTarget, Result, Session};

fn cmd(args: &mut Vec<(&'static str, Value)>, id: &'static str, params: Option<Value>) {
    args.push((id, params.unwrap_or_else(|| json!({}))));
}

fn cut_part(args: &mut Vec<(&'static str, Value)>, part: &Value, root: &str, behind_root: bool) -> Result<()> {
    let id = part["id"].as_str().ok_or_else(|| bad("a part has no id"))?;
    let outline = part["outline"].as_array().filter(|o| o.len() >= 3).ok_or_else(|| bad(format!("part {id} needs an outline of 3 or more points")))?;
    let lasso = json!({"points": outline, "antiAlias": true});
    cmd(args, "layer.select", Some(json!({"name": root})));
    cmd(args, "select.lasso", Some(lasso));
    cmd(args, "select.refineEdge", Some(json!({"smooth": 2, "feather": 0.6, "contrast": 14, "shiftEdge": 0})));
    cmd(args, "layer.new.layerViaCopy", None);
    cmd(args, "layer.renameLayer", Some(json!({"name": id})));
    if behind_root {
        cmd(args, "layer.arrange.sendToBack", None);
    } else {
        cmd(args, "layer.arrange.bringToFront", None);
    }
    cmd(args, "select.deselect", None);
    if let Some(clone_params) = part.get("clone_stamp").filter(|v| v.is_object()) {
        cmd(args, "layer.select", Some(json!({"name": id})));
        cmd(args, "paint.cloneStamp", Some(clone_params.clone()));
    }
    if let Some(heal_params) = part.get("spot_healing").filter(|v| v.is_object()) {
        cmd(args, "layer.select", Some(json!({"name": id})));
        cmd(args, "paint.spotHealing", Some(heal_params.clone()));
    }
    Ok(())
}

fn trim_root_part(args: &mut Vec<(&'static str, Value)>, root_part_val: &Value, root: &str) -> Result<()> {
    let outline =
        root_part_val["outline"].as_array().filter(|o| o.len() >= 3).ok_or_else(|| bad(format!("root part {root} needs an outline of 3 or more points")))?;
    cmd(args, "layer.select", Some(json!({"name": root})));
    cmd(args, "select.lasso", Some(json!({"points": outline, "antiAlias": true})));
    cmd(args, "select.inverse", None);
    cmd(args, "edit.clear", None);
    cmd(args, "select.deselect", None);
    Ok(())
}

pub(crate) fn photocraft_run(input: &std::path::Path, steps: &[(&str, Value)], output: Option<&std::path::Path>) -> Result<()> {
    if let Some(out) = output
        && out == input
    {
        return Err(crate::EngineError::InvalidParams(format!("refusing to overwrite input {}", input.display())));
    }
    let _ = CraftHub::run_batch(CraftTarget::Photo, Some(input), output, steps)?;
    Ok(())
}

pub(crate) fn photocraft_dimensions(path: &std::path::Path) -> Result<(u32, u32)> {
    if let Some((w, h, _)) = craft_util::media::fast_dimensions(path) {
        return Ok((w, h));
    }
    if let Ok(dims) = image::image_dimensions(path) {
        return Ok(dims);
    }
    let doc = CraftHub::execute(CraftTarget::Photo, Some(path), "info", &json!({}))?;
    let w = doc["width"].as_u64().unwrap_or(1024) as u32;
    let h = doc["height"].as_u64().unwrap_or(1024) as u32;
    Ok((w, h))
}

/// Trace a raster image in `VectorCraft` (`imageTrace.make` with `ignoreWhite: true` + `imageTrace.expand`),
/// exporting an `.svg` silhouette and returning simplified polygon coordinates in pixel space.
pub(crate) fn vectorcraft_trace_outline(image: &std::path::Path, svg_out: &std::path::Path, offset_px: f64) -> Result<Vec<Value>> {
    let (w, h) = photocraft_dimensions(image)?;
    let steps = [
        ("file.new", json!({"width": w, "height": h})),
        ("file.place", json!({"path": image, "x": 0, "y": 0})),
        ("object.setBounds", json!({"x": 0, "y": 0, "width": w, "height": h})),
        ("imageTrace.make", json!({"preset": "silhouettes", "ignoreWhite": true, "threshold": 245, "method": "overlapping"})),
        ("imageTrace.expand", json!({})),
        ("object.ungroup", json!({})),
        ("select.all", json!({})),
        ("object.pathfinder.unite", json!({})),
        ("object.path.offsetPath", json!({"offset": offset_px})),
        ("object.path.simplify", json!({"tolerance": 4.0, "straightLines": true})),
        ("document.node", json!({"id": 14})),
    ];
    let results = CraftHub::run_batch(CraftTarget::Vector, None, Some(svg_out), &steps)?;
    let cx = f64::from(w) / 2.0;
    let cy = f64::from(h) / 2.0;
    let mut points = Vec::new();
    for v in &results {
        let anchors = v
            .pointer("/kind/children/0/kind/path/subpaths/0/anchors")
            .or_else(|| v.pointer("/result/kind/children/0/kind/path/subpaths/0/anchors"))
            .and_then(Value::as_array);
        if let Some(list) = anchors {
            for a in list {
                if let Some(p) = a["p"].as_array()
                    && p.len() == 2
                {
                    let x = (p[0].as_f64().unwrap_or(0.0) + cx).round() as i64;
                    let y = (p[1].as_f64().unwrap_or(0.0) + cy).round() as i64;
                    points.push(json!([x, y]));
                }
            }
        }
    }
    Ok(points)
}

fn layer_bounds(psd: &std::path::Path) -> Result<serde_json::Map<String, Value>> {
    let doc = CraftHub::execute(CraftTarget::Photo, Some(psd), "info", &json!({}))?;
    let mut out = serde_json::Map::new();
    for layer in doc["layers"].as_array().into_iter().flatten() {
        if let (Some(name), Some(bounds)) = (layer["name"].as_str(), layer.get("bounds")) {
            out.insert(name.into(), bounds.clone());
        }
    }
    Ok(out)
}

fn export_layers(view: &View, psd: &std::path::Path) -> Result<serde_json::Map<String, Value>> {
    let dir = view.dir.join("layers");
    if dir.is_dir() {
        std::fs::remove_dir_all(&dir)?;
    }
    std::fs::create_dir_all(&dir)?;
    let mut args = Vec::new();
    let params = json!({"dir": dir, "format": "png", "prefix": "", "trim": true});
    cmd(&mut args, "file.export.layersToFiles", Some(params));
    photocraft_run(psd, &args, None)?;
    let mut out = serde_json::Map::new();
    for entry in std::fs::read_dir(&dir)?.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if let Some(id) = name.strip_suffix(".png").and_then(|n| n.splitn(3, '_').nth(2)) {
            out.insert(id.to_string(), json!(entry.path()));
        }
    }
    Ok(out)
}

fn resolve_body_plate(view: &View, parts: &Value) -> Option<std::path::PathBuf> {
    parts["body_plate"]
        .as_str()
        .or_else(|| parts["torso_image"].as_str())
        .map(|p| {
            let path = std::path::Path::new(p);
            if path.is_absolute() { path.to_path_buf() } else { view.dir.join(path) }
        })
        .filter(|p| p.is_file())
        .or_else(|| view.dir.parent().map(|c| c.join("parts/source/body-plate_v1.png")).filter(|p| p.is_file()))
}

pub fn cmd_character_cut(s: &mut Session, p: &Value) -> Result<Value> {
    if p["parts"].is_array() {
        super::cmd_character_save_marks(s, p)?;
    }
    let view = View::from_params(s, p)?;
    let mut parts = view.parts()?;
    let list = part_list(&parts)?.clone();
    let root = root_part(&list)?.to_string();
    let psd = view.file("psd");
    let pcraft = view.file("pcraft");
    let pre_cut: Vec<(String, std::path::PathBuf)> = list
        .iter()
        .filter_map(|part| {
            let id = part["id"].as_str()?.to_string();
            let raw = part["source"].as_str()?;
            let path = std::path::Path::new(raw);
            let full = if path.is_absolute() { path.to_path_buf() } else { view.dir.join(path) };
            full.is_file().then_some((id, full))
        })
        .collect();
    let image = if pre_cut.len() == list.len() && !pre_cut.is_empty() {
        let drawing = view.dir.join(format!("{}-{}-drawing.png", view.id, view.view));
        let mut args = vec![("layer.renameLayer", json!({"name": &pre_cut[0].0}))];
        for (id, file) in &pre_cut[1..] {
            cmd(&mut args, "file.placeEmbedded", Some(json!({"path": file})));
            cmd(&mut args, "layer.renameLayer", Some(json!({"name": id})));
        }
        cmd(&mut args, "layer.rasterize.allLayers", None);
        cmd(&mut args, "file.saveACopy", Some(json!({"path": &psd, "overwrite": true})));
        cmd(&mut args, "file.saveACopy", Some(json!({"path": &drawing, "overwrite": true})));
        photocraft_run(&pre_cut[0].1, &args, Some(&pcraft))?;
        parts["image"] = json!(drawing.file_name().map(|n| n.to_string_lossy().into_owned()));
        drawing
    } else {
        let raw_image = parts["image"].as_str().map(|i| view.dir.join(i)).filter(|i| i.is_file()).ok_or_else(|| bad("parts.json image is missing"))?;
        let image = if p["remove_background"].as_bool().unwrap_or(true) {
            let isolated = view.dir.join(format!("{}-{}-isolated.png", view.id, view.view));
            if CraftHub::remove_background_photo(&raw_image, &isolated, p["chroma"].as_str(), 0.8).is_ok() && isolated.is_file() { isolated } else { raw_image }
        } else {
            raw_image
        };
        let root_idx = list.iter().position(|p| p["id"].as_str() == Some(&root)).unwrap_or(0);
        let mut args = Vec::new();
        cmd(&mut args, "layer.renameLayer", Some(json!({"name": root})));
        for part in &list[root_idx + 1..] {
            cut_part(&mut args, part, &root, false)?;
        }
        for part in list[..root_idx].iter().rev() {
            cut_part(&mut args, part, &root, true)?;
        }
        if let Some(plate) = resolve_body_plate(&view, &parts) {
            cmd(&mut args, "layer.select", Some(json!({"name": root})));
            cmd(&mut args, "file.placeEmbedded", Some(json!({"path": plate})));
            cmd(&mut args, "layer.rasterize.layer", None);
            cmd(&mut args, "layer.mergeDown", None);
            cmd(&mut args, "layer.renameLayer", Some(json!({"name": root})));
        }
        trim_root_part(&mut args, &list[root_idx], &root)?;
        cmd(&mut args, "file.saveACopy", Some(json!({"path": &psd, "overwrite": true})));
        photocraft_run(&image, &args, Some(&pcraft))?;
        image
    };
    let bounds = layer_bounds(&psd)?;
    let files = export_layers(&view, &psd)?;
    for part in parts["parts"].as_array_mut().into_iter().flatten() {
        let id = part["id"].as_str().unwrap_or_default().to_string();
        part["bounds"] = bounds.get(&id).cloned().unwrap_or(Value::Null);
        part["file"] = files.get(&id).cloned().unwrap_or(Value::Null);
    }
    let svg = view.file("svg");
    if let Ok(silhouette_pts) = vectorcraft_trace_outline(&image, &svg, 2.0)
        && !silhouette_pts.is_empty()
    {
        parts["silhouette_svg"] = json!(svg);
        parts["silhouette_points"] = json!(silhouette_pts.len());
    }
    if parts["animation_style"].is_null() {
        parts["animation_style"] = json!("cutout_2_5d");
    }
    view.write_parts(&parts)?;
    let mut reply = json!({"ok": true, "psd": psd, "pcraft": pcraft, "layers": bounds,
        "next": format!("character.rig character={} view={}", view.id, view.view)});
    if p["rig"] == true {
        let rigged = super::cmd_character_rig(s, p)?;
        reply["ecproj"] = rigged["ecproj"].clone();
        reply["comp"] = rigged["comp"].clone();
        reply["next"] = rigged["next"].clone();
    }
    Ok(reply)
}
