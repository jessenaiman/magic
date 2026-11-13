//! Host Craft application media delegation & local project preview helpers (`crates/engine/src/native_media.rs`).
//!
//! Subject isolation (`PhotoCraft`) and vector contour tracing (`VectorCraft`) execute against the
//! host machine's running Craft applications via `--control` (`crate::craft_api`) or `<app>-cli`,
//! while local thumbnail/cell cropping and `craft-doc` preview extraction keep project UI cards fast.

use std::fmt::Write as _;
use std::io::Cursor;
use std::path::Path;

use image::{DynamicImage, GenericImageView, ImageFormat, RgbaImage};
use serde_json::{Value, json};

use crate::{EngineError, Result};

fn bad(msg: impl Into<String>) -> EngineError {
    EngineError::InvalidParams(msg.into())
}

pub fn encode_png_atomic(img: &RgbaImage, out: &Path) -> Result<()> {
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut buf = Vec::new();
    DynamicImage::ImageRgba8(img.clone())
        .write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
        .map_err(|e| bad(format!("encode PNG {}: {e}", out.display())))?;
    craft_atomic::atomic_write(out, &buf)?;
    Ok(())
}

/// Load a raster image or extract an embedded Craft preview (`.pcraft`, `.vectorcraft`, `.ecproj`).
pub fn load_rgba(input: &Path) -> Result<RgbaImage> {
    if let Ok(img) = image::open(input) {
        return Ok(img.to_rgba8());
    }
    let tmp = std::env::temp_dir().join(format!("magic-preview-{}.png", std::process::id()));
    let extracted = magic_util::media::extract_craft_preview(input, &tmp).unwrap_or(false);
    if extracted && let Ok(img) = image::open(&tmp) {
        let _ = std::fs::remove_file(&tmp);
        return Ok(img.to_rgba8());
    }
    let _ = std::fs::remove_file(&tmp);
    Err(bad(format!("cannot decode raster or embedded Craft preview from {}", input.display())))
}

/// Isolate a subject to transparent RGBA via host `PhotoCraft` (`photocraft --control 7878` or
/// `photocraft-cli` `layer.removeBackground`), falling back to chroma/corner keying only when
/// generating offline preview thumbnails before `PhotoCraft` saves back.
pub fn isolate_subject_rgba(input: &Path, output: &Path, chroma_hex: Option<&str>, feather_px: f64) -> Result<Value> {
    if input == output {
        return Err(bad(format!("refusing to overwrite source image {}", input.display())));
    }
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }

    if crate::craft_api::execute_on_host_app("photocraft", Some(input), "layer.removeBackground", json!({})).is_ok()
        && crate::craft_api::execute_on_host_app("photocraft", Some(input), "file.saveACopy", json!({"path": output, "overwrite": true})).is_ok()
        && output.is_file()
    {
        let (w, h, _) = magic_util::media::fast_dimensions(output).unwrap_or((0, 0, true));
        return Ok(json!({
            "ok": true,
            "input": input,
            "output": output,
            "width": w,
            "height": h,
            "engine": "photocraft",
        }));
    }

    let mut rgba = load_rgba(input)?;
    let (w, h) = rgba.dimensions();
    if w == 0 || h == 0 {
        return Err(bad("empty image dimensions"));
    }
    let key_rgb = chroma_hex
        .and_then(|hex| {
            let clean = hex.trim().trim_start_matches('#');
            (clean.len() == 6).then(|| {
                Some([u8::from_str_radix(&clean[0..2], 16).ok()?, u8::from_str_radix(&clean[2..4], 16).ok()?, u8::from_str_radix(&clean[4..6], 16).ok()?])
            })?
        })
        .unwrap_or_else(|| {
            let p = rgba.get_pixel(0, 0);
            [p[0], p[1], p[2]]
        });
    let low_thresh = 26.0_f64;
    let high_thresh = (low_thresh + feather_px.clamp(0.2, 16.0) * 18.0).max(low_thresh + 4.0);
    for px in rgba.pixels_mut() {
        if px[3] == 0 {
            continue;
        }
        let dr = f64::from(px[0]) - f64::from(key_rgb[0]);
        let dg = f64::from(px[1]) - f64::from(key_rgb[1]);
        let db = f64::from(px[2]) - f64::from(key_rgb[2]);
        let dist = (dr * dr + dg * dg + db * db).sqrt();
        if dist <= low_thresh {
            px[3] = 0;
        } else if dist < high_thresh {
            let t = ((dist - low_thresh) / (high_thresh - low_thresh)).clamp(0.0, 1.0);
            px[3] = (f64::from(px[3]) * t).round() as u8;
        }
    }
    encode_png_atomic(&rgba, output)?;
    Ok(json!({
        "ok": true,
        "input": input,
        "output": output,
        "width": w,
        "height": h,
        "engine": "preview-keyer",
    }))
}

/// Trace a vector contour SVG via host `VectorCraft` (`vectorcraft --control` or `vectorcraft-cli`),
/// returning `(summary_json, centered_outline_points)`.
fn scan_alpha_contour(rgba: &image::RgbaImage, step: u32) -> (Vec<(u32, u32)>, Vec<(u32, u32)>) {
    let (w, h) = rgba.dimensions();
    let mut left_pts = Vec::new();
    let mut right_pts = Vec::new();
    let mut y = 0;
    while y < h {
        let first_opaque = (0..w).find(|&x| rgba.get_pixel(x, y)[3] > 24);
        let last_opaque = (0..w).rev().find(|&x| rgba.get_pixel(x, y)[3] > 24);
        if let (Some(lx), Some(rx)) = (first_opaque, last_opaque) {
            left_pts.push((lx, y));
            right_pts.push((rx + 1, y));
        }
        y = y.saturating_add(step);
    }
    (left_pts, right_pts)
}

fn contour_svg_path_data(w: u32, h: u32, left_pts: &[(u32, u32)], right_pts: &[(u32, u32)]) -> String {
    let mut d = String::new();
    if left_pts.is_empty() {
        let _ = write!(d, "M 0 0 L {w} 0 L {w} {h} L 0 {h} Z");
        return d;
    }
    for (idx, (x, py)) in left_pts.iter().enumerate() {
        let cmd = if idx == 0 { 'M' } else { 'L' };
        let _ = write!(d, "{cmd} {x} {py} ");
    }
    for (x, py) in right_pts.iter().rev() {
        let _ = write!(d, "L {x} {py} ");
    }
    d.push('Z');
    d
}

pub fn trace_silhouette_svg(input: &Path, output: &Path, simplify_px: f64, strokes: bool) -> Result<(Value, Vec<[f64; 2]>)> {
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let rgba = load_rgba(input)?;
    let (w, h) = rgba.dimensions();
    if w == 0 || h == 0 {
        return Err(bad("empty image dimensions"));
    }
    let step = (simplify_px.round() as u32).clamp(1, 8);
    let (left_pts, right_pts) = scan_alpha_contour(&rgba, step);
    let (cx, cy) = (f64::from(w) * 0.5, f64::from(h) * 0.5);
    let centered: Vec<[f64; 2]> = if left_pts.is_empty() {
        vec![[-cx, -cy], [cx, -cy], [cx, cy], [-cx, cy]]
    } else {
        left_pts
            .iter()
            .chain(right_pts.iter().rev())
            .map(|&(x, py)| [f64::from(x) - cx, f64::from(py) - cy])
            .collect()
    };

    if crate::craft_api::execute_on_host_app(
        "vectorcraft",
        Some(input),
        "image.trace",
        json!({"simplify": simplify_px, "strokes": strokes, "ignoreWhite": true}),
    )
    .is_ok()
        && crate::craft_api::execute_on_host_app("vectorcraft", Some(input), "file.export", json!({"path": output, "format": "svg"})).is_ok()
        && output.is_file()
    {
        return Ok((
            json!({ "ok": true, "input": input, "output": output, "width": w, "height": h, "engine": "vectorcraft" }),
            centered,
        ));
    }

    let d = contour_svg_path_data(w, h, &left_pts, &right_pts);
    let attr = if strokes { "fill=\"none\" stroke=\"#111827\" stroke-width=\"2\"" } else { "fill=\"#111827\"" };
    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {w} {h}\" width=\"{w}\" height=\"{h}\">\n  <path d=\"{}\" {attr}/>\n</svg>\n",
        d.trim()
    );
    craft_atomic::atomic_write(output, svg.as_bytes())?;
    Ok((
        json!({
            "ok": true,
            "input": input,
            "output": output,
            "width": w,
            "height": h,
            "engine": "contour-outline",
        }),
        centered,
    ))
}

pub fn crop_and_anchor_cell(rgba: &RgbaImage, cell: [u32; 4], trim_alpha: bool, out: &Path) -> Result<[f64; 2]> {
    let [cx, cy, cw, ch] = cell;
    let (iw, ih) = rgba.dimensions();
    let x0 = cx.min(iw.saturating_sub(1));
    let y0 = cy.min(ih.saturating_sub(1));
    let w = cw.min(iw.saturating_sub(x0)).max(1);
    let h = ch.min(ih.saturating_sub(y0)).max(1);
    let sub = image::imageops::crop_imm(rgba, x0, y0, w, h).to_image();

    if !trim_alpha {
        encode_png_atomic(&sub, out)?;
        return Ok([f64::from(w) * 0.5, f64::from(h) * 0.5]);
    }

    let (mut min_x, mut min_y, mut max_x, mut max_y) = (w, h, 0, 0);
    for (x, y, p) in sub.enumerate_pixels() {
        if p[3] > 8 {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x + 1);
            max_y = max_y.max(y + 1);
        }
    }
    if max_x <= min_x || max_y <= min_y {
        encode_png_atomic(&sub, out)?;
        return Ok([f64::from(w) * 0.5, f64::from(h) * 0.5]);
    }
    let tw = (max_x - min_x).max(1);
    let th = (max_y - min_y).max(1);
    let trimmed = image::imageops::crop_imm(&sub, min_x, min_y, tw, th).to_image();
    encode_png_atomic(&trimmed, out)?;
    Ok([f64::from(tw) * 0.5, f64::from(th) * 0.5])
}

/// Separate a character `parts-sheet.png` via host `PhotoCraft` (`layer.maskAllObjects` +
/// `file.export.layersToFiles`) or 3x2 grid slicing into `<view_dir>/layers/` + `parts.json`.
pub fn split_parts_sheet(sheet: &Path, view_dir: &Path, char_id: &str, view: &str, chroma: Option<&str>) -> Result<Value> {
    let layers_dir = view_dir.join("layers");
    std::fs::create_dir_all(&layers_dir)?;
    let isolated = view_dir.join(format!("{char_id}-{view}-sheet-isolated.png"));
    let src_for_cells = if isolate_subject_rgba(sheet, &isolated, chroma, 1.0).is_ok() && isolated.is_file() { isolated } else { sheet.to_path_buf() };

    let used_photocraft = crate::craft_api::execute_on_host_app("photocraft", Some(&src_for_cells), "layer.maskAllObjects", json!({})).is_ok()
        && crate::craft_api::execute_on_host_app("photocraft", Some(&src_for_cells), "file.export.layersToFiles", json!({"dir": layers_dir, "prefix": "part"}))
            .is_ok();

    let rgba = load_rgba(&src_for_cells)?;
    let (w, h) = rgba.dimensions();
    let cols = 3_u32;
    let rows = 2_u32;
    let cw = (w / cols).max(1);
    let ch = (h / rows).max(1);
    let default_parts = ["torso", "head", "arm_l", "arm_r", "leg_l", "leg_r"];
    let mut part_entries = Vec::new();
    for (idx, part_id) in default_parts.iter().enumerate() {
        let c = (idx as u32) % cols;
        let r = (idx as u32) / cols;
        let out_png = layers_dir.join(format!("{part_id}.png"));
        let anchor = crop_and_anchor_cell(&rgba, [c * cw, r * ch, cw, ch], true, &out_png)?;
        part_entries.push(json!({
            "id": part_id,
            "parent": if *part_id == "torso" { Value::Null } else { json!("torso") },
            "source": out_png,
            "pivot": anchor,
        }));
    }
    let parts_doc = json!({
        "character": char_id,
        "view": view,
        "image": src_for_cells.file_name().and_then(|n| n.to_str()).unwrap_or("parts-sheet.png"),
        "parts": part_entries,
    });
    let parts_json_path = view_dir.join("parts.json");
    craft_atomic::atomic_write(&parts_json_path, serde_json::to_string_pretty(&parts_doc).unwrap_or_default().as_bytes())?;
    Ok(json!({
        "ok": true,
        "character": char_id,
        "view": view,
        "parts_json": parts_json_path,
        "layers_dir": layers_dir,
        "count": default_parts.len(),
        "engine": if used_photocraft { "photocraft" } else { "grid-slicer" },
    }))
}

pub fn extract_or_copy_preview(input: &Path, out_png: &Path, max_side: u32) -> Result<bool> {
    if magic_util::media::extract_craft_preview(input, out_png).unwrap_or(false) {
        return Ok(true);
    }
    let Ok(img) = image::open(input) else {
        return Ok(false);
    };
    let (w, h) = img.dimensions();
    let max_side = max_side.max(64);
    let resized = if w > max_side || h > max_side { img.resize(max_side, max_side, image::imageops::FilterType::Triangle).to_rgba8() } else { img.to_rgba8() };
    encode_png_atomic(&resized, out_png)?;
    Ok(true)
}

pub fn probe_media_in_process(path: &Path) -> Result<Value> {
    if magic_util::media::is_pure_audio_file(path) {
        magic_util::media::probe_audio_in_process(path).map_err(|e| bad(e.to_string()))
    } else {
        magic_util::media::probe_container_in_process(path).or_else(|_| magic_util::media::probe_audio_in_process(path)).map_err(|e| bad(e.to_string()))
    }
}
