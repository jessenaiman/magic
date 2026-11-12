//! `character.cut`: pure-Rust background isolation, part cutout cropping, and silhouette SVG
//! extraction built on `image` and `craft-atomic` (`crate::native_media`).

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::{View, bad, part_list, root_part};
use crate::{Result, Session};

pub(crate) fn photocraft_dimensions(path: &Path) -> Result<(u32, u32)> {
    if let Some((w, h, _)) = magic_util::media::fast_dimensions(path) {
        return Ok((w, h));
    }
    if let Ok(dims) = image::image_dimensions(path) {
        return Ok(dims);
    }
    let rgba = crate::native_media::load_rgba(path)?;
    Ok(rgba.dimensions())
}

pub(crate) fn vectorcraft_trace_outline(image: &Path, svg_out: &Path, offset_px: f64) -> Result<Vec<Value>> {
    let (w, h) = photocraft_dimensions(image)?;
    let (_, rel_pts) = crate::native_media::trace_silhouette_svg(image, svg_out, offset_px, false)?;
    let cx = f64::from(w) / 2.0;
    let cy = f64::from(h) / 2.0;
    Ok(rel_pts
        .into_iter()
        .map(|pt| json!([(pt[0] + cx).round() as i64, (pt[1] + cy).round() as i64]))
        .collect())
}

fn outline_bbox(outline: &[Value], img_w: u32, img_h: u32) -> [u32; 4] {
    let mut min_x = f64::from(img_w);
    let mut min_y = f64::from(img_h);
    let mut max_x = 0.0_f64;
    let mut max_y = 0.0_f64;
    for pt in outline {
        if let Some(arr) = pt.as_array()
            && arr.len() >= 2
        {
            let x = arr[0].as_f64().unwrap_or(0.0);
            let y = arr[1].as_f64().unwrap_or(0.0);
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }
    let x0 = (min_x.floor().max(0.0) as u32).min(img_w.saturating_sub(1));
    let y0 = (min_y.floor().max(0.0) as u32).min(img_h.saturating_sub(1));
    let w = ((max_x - min_x).ceil().max(1.0) as u32).min(img_w.saturating_sub(x0));
    let h = ((max_y - min_y).ceil().max(1.0) as u32).min(img_h.saturating_sub(y0));
    [x0, y0, w.max(1), h.max(1)]
}

pub fn cmd_character_cut(s: &mut Session, p: &Value) -> Result<Value> {
    if p["parts"].is_array() {
        super::cmd_character_save_marks(s, p)?;
    }
    let view = View::from_params(s, p)?;
    let mut parts = view.parts()?;
    let list = part_list(&parts)?.clone();
    let _root = root_part(&list)?.to_string();

    let layers_dir = view.dir.join("layers");
    std::fs::create_dir_all(&layers_dir)?;

    let pre_cut: Vec<(String, PathBuf)> = list
        .iter()
        .filter_map(|part| {
            let id = part["id"].as_str()?.to_string();
            let raw = part["source"].as_str()?;
            let path = Path::new(raw);
            let full = if path.is_absolute() { path.to_path_buf() } else { view.dir.join(path) };
            full.is_file().then_some((id, full))
        })
        .collect();

    let mut bounds_map = serde_json::Map::new();
    let mut files_map = serde_json::Map::new();

    let composite_image = if pre_cut.len() == list.len() && !pre_cut.is_empty() {
        let drawing = view.dir.join(format!("{}-{}-drawing.png", view.id, view.view));
        std::fs::copy(&pre_cut[0].1, &drawing)?;
        for (id, file) in &pre_cut {
            let dst = layers_dir.join(format!("{id}.png"));
            if file != &dst {
                std::fs::copy(file, &dst)?;
            }
            if let Ok((w, h)) = photocraft_dimensions(&dst) {
                bounds_map.insert(id.clone(), json!([0, 0, w, h]));
            }
            files_map.insert(id.clone(), json!(dst));
        }
        parts["image"] = json!(drawing.file_name().map(|n| n.to_string_lossy().into_owned()));
        drawing
    } else {
        let raw_image = parts["image"]
            .as_str()
            .map(|i| view.dir.join(i))
            .filter(|i| i.is_file())
            .ok_or_else(|| bad("parts.json image is missing"))?;
        let image = if p["remove_background"].as_bool().unwrap_or(true) {
            let isolated = view.dir.join(format!("{}-{}-isolated.png", view.id, view.view));
            if crate::native_media::isolate_subject_rgba(&raw_image, &isolated, p["chroma"].as_str(), 0.8).is_ok()
                && isolated.is_file()
            {
                isolated
            } else {
                raw_image
            }
        } else {
            raw_image
        };
        let src_rgba = crate::native_media::load_rgba(&image)?;
        let (img_w, img_h) = src_rgba.dimensions();
        for part in &list {
            let id = part["id"].as_str().ok_or_else(|| bad("a part has no id"))?;
            let outline = part["outline"]
                .as_array()
                .filter(|o| o.len() >= 3)
                .ok_or_else(|| bad(format!("part {id} needs an outline of 3 or more points")))?;
            let rect = outline_bbox(outline, img_w, img_h);
            let dst = layers_dir.join(format!("{id}.png"));
            crate::native_media::crop_and_anchor_cell(&src_rgba, rect, false, &dst)?;
            bounds_map.insert(id.to_owned(), json!(rect));
            files_map.insert(id.to_owned(), json!(dst));
        }
        image
    };

    for part in parts["parts"].as_array_mut().into_iter().flatten() {
        let id = part["id"].as_str().unwrap_or_default().to_string();
        part["bounds"] = bounds_map.get(&id).cloned().unwrap_or(Value::Null);
        part["file"] = files_map.get(&id).cloned().unwrap_or(Value::Null);
    }

    let svg = view.file("svg");
    if let Ok(silhouette_pts) = vectorcraft_trace_outline(&composite_image, &svg, 2.0)
        && !silhouette_pts.is_empty()
    {
        parts["silhouette_svg"] = json!(svg);
        parts["silhouette_points"] = json!(silhouette_pts.len());
    }
    if parts["animation_style"].is_null() {
        parts["animation_style"] = json!("cutout_2_5d");
    }
    view.write_parts(&parts)?;

    let mut reply = json!({
        "ok": true,
        "image": composite_image,
        "layers": bounds_map,
        "next": format!("character.rig character={} view={}", view.id, view.view)
    });
    if p["rig"] == true {
        let rigged = super::cmd_character_rig(s, p)?;
        reply["ecproj"] = rigged["ecproj"].clone();
        reply["comp"] = rigged["comp"].clone();
        reply["next"] = rigged["next"].clone();
    }
    Ok(reply)
}
