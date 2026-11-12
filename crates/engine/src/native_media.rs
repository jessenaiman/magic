<<<<<<< HEAD
//! Pure-Rust in-process image, preview, silhouette, and media helpers built strictly on
//! `storytold/craft-libs` (`craft-doc`, `craft-atomic`, `craft-audio`, `craft-media`) and `image`.
//!
//! Replaces legacy subprocess/`--control` middleman calls so Magic Studio reads, crops, keys,
//! inspects, and stages project files natively without requiring sibling binaries at runtime.

use std::io::Cursor;
use std::path::{Path, PathBuf};

use image::{DynamicImage, GenericImageView, ImageFormat, Rgba, RgbaImage};
=======
//! Host Craft application media delegation & local project preview helpers (`crates/engine/src/native_media.rs`).
//!
//! Subject isolation (`PhotoCraft`) and vector contour tracing (`VectorCraft`) execute against the
//! host machine's running Craft applications via `--control` (`crate::craft_api`) or `<app>-cli`,
//! while local thumbnail/cell cropping and `craft-doc` preview extraction keep project UI cards fast.

use std::fmt::Write as _;
use std::io::Cursor;
use std::path::Path;

use image::{DynamicImage, GenericImageView, ImageFormat, RgbaImage};
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
use serde_json::{Value, json};

use crate::{EngineError, Result};

fn bad(msg: impl Into<String>) -> EngineError {
    EngineError::InvalidParams(msg.into())
}

<<<<<<< HEAD
fn parse_hex_rgb(hex: &str) -> Option<[u8; 3]> {
    let clean = hex.trim().trim_start_matches('#');
    if clean.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&clean[0..2], 16).ok()?;
    let g = u8::from_str_radix(&clean[2..4], 16).ok()?;
    let b = u8::from_str_radix(&clean[4..6], 16).ok()?;
    Some([r, g, b])
}

=======
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
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

<<<<<<< HEAD
/// Load a raster image or extract an embedded Craft preview (`.pcraft`, `.vectorcraft`, `.ecproj`) first.
=======
/// Load a raster image or extract an embedded Craft preview (`.pcraft`, `.vectorcraft`, `.ecproj`).
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
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

<<<<<<< HEAD
/// Pure-Rust chroma/corner background alpha isolation written atomically to `output` (never in-place).
=======
/// Isolate a subject to transparent RGBA via host `PhotoCraft` (`photocraft --control 7878` or
/// `photocraft-cli` `layer.removeBackground`), falling back to chroma/corner keying only when
/// generating offline preview thumbnails before `PhotoCraft` saves back.
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
pub fn isolate_subject_rgba(input: &Path, output: &Path, chroma_hex: Option<&str>, feather_px: f64) -> Result<Value> {
    if input == output {
        return Err(bad(format!("refusing to overwrite source image {}", input.display())));
    }
<<<<<<< HEAD
=======
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

>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    let mut rgba = load_rgba(input)?;
    let (w, h) = rgba.dimensions();
    if w == 0 || h == 0 {
        return Err(bad("empty image dimensions"));
    }
<<<<<<< HEAD
    let key_rgb = chroma_hex.and_then(parse_hex_rgb).unwrap_or_else(|| {
        let p = rgba.get_pixel(0, 0);
        [p[0], p[1], p[2]]
    });
=======
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
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
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
<<<<<<< HEAD
        "engine": "pure-rust-rgba"
    }))
}

/// Crop a rectangle `[x, y, cw, ch]`, optionally trim transparent bounds, and anchor inside `(cw, ch)`.
pub fn crop_and_anchor_cell(src: &RgbaImage, rect: [u32; 4], anchor_bottom: bool, out_png: &Path) -> Result<()> {
    let (img_w, img_h) = src.dimensions();
    let x = rect[0].min(img_w.saturating_sub(1));
    let y = rect[1].min(img_h.saturating_sub(1));
    let cw = rect[2].max(1).min(img_w.saturating_sub(x));
    let ch = rect[3].max(1).min(img_h.saturating_sub(y));
    let sub = image::imageops::crop_imm(src, x, y, cw, ch).to_image();

    let mut min_x = cw;
    let mut min_y = ch;
    let mut max_x = 0_u32;
    let mut max_y = 0_u32;
    let mut any_opaque = false;
    for (px_x, px_y, px) in sub.enumerate_pixels() {
        if px[3] > 8 {
            any_opaque = true;
            min_x = min_x.min(px_x);
            min_y = min_y.min(px_y);
            max_x = max_x.max(px_x);
            max_y = max_y.max(px_y);
        }
    }
    let mut canvas = RgbaImage::from_pixel(cw, ch, Rgba([0, 0, 0, 0]));
    if any_opaque {
        let tw = max_x - min_x + 1;
        let th = max_y - min_y + 1;
        let trimmed = image::imageops::crop_imm(&sub, min_x, min_y, tw, th).to_image();
        let dest_x = i64::from((cw.saturating_sub(tw)) / 2);
        let dest_y = if anchor_bottom { i64::from(ch.saturating_sub(th)) } else { i64::from((ch.saturating_sub(th)) / 2) };
        image::imageops::overlay(&mut canvas, &trimmed, dest_x, dest_y);
    }
    encode_png_atomic(&canvas, out_png)
}

/// Split a 3x3 Moppet anatomical parts sheet into individual transparent PNG part cutouts.
pub fn split_parts_sheet(sheet_png: &Path, view_dir: &Path, char_id: &str, view: &str, chroma_hex: Option<&str>) -> Result<Value> {
    std::fs::create_dir_all(view_dir)?;
    let clean_path = view_dir.join(format!("{char_id}-{view}-clean.png"));
    let _ = isolate_subject_rgba(sheet_png, &clean_path, chroma_hex, 0.8)?;
    let clean = load_rgba(&clean_path)?;
    let (w, h) = clean.dimensions();
    let names = ["head", "torso", "hips", "arm_l", "hand_l", "leg_l", "arm_r", "hand_r", "leg_r"];
    let mut parts_out = Vec::new();
    for (idx, part_name) in names.iter().enumerate() {
        let rect = magic_scribbles::sprite_cell_rect(w, h, (3, 3), idx as u32);
        let part_file = view_dir.join(format!("{part_name}.png"));
        crop_and_anchor_cell(&clean, rect, false, &part_file)?;
        parts_out.push(json!({"id": part_name, "file": part_file, "rect": rect}));
    }
    Ok(json!({
        "ok": true,
        "character": char_id,
        "view": view,
        "sheet": sheet_png,
        "parts": parts_out
    }))
}

/// Extract a clean bounding-contour polygon SVG (and return normalized silhouette points around center).
pub fn trace_silhouette_svg(input: &Path, out_svg: &Path, offset_px: f64, strokes: bool) -> Result<(Value, Vec<[f64; 2]>)> {
    let rgba = load_rgba(input)?;
    let (w, h) = rgba.dimensions();
    let mut min_x = w;
    let mut min_y = h;
    let mut max_x = 0_u32;
    let mut max_y = 0_u32;
    let mut found = false;
    for (x, y, px) in rgba.enumerate_pixels() {
        let opaque = px[3] > 16 && !(px[0] > 248 && px[1] > 248 && px[2] > 248);
        if opaque {
            found = true;
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }
    if !found {
        min_x = 0;
        min_y = 0;
        max_x = w.saturating_sub(1);
        max_y = h.saturating_sub(1);
    }
    let pad = offset_px.max(0.0);
    let x0 = (f64::from(min_x) - pad).max(0.0);
    let y0 = (f64::from(min_y) - pad).max(0.0);
    let x1 = (f64::from(max_x) + pad).min(f64::from(w));
    let y1 = (f64::from(max_y) + pad).min(f64::from(h));
    let cx = f64::from(w) / 2.0;
    let cy = f64::from(h) / 2.0;
    let pts = vec![[x0 - cx, y0 - cy], [x1 - cx, y0 - cy], [x1 - cx, y1 - cy], [x0 - cx, y1 - cy]];
    let paint_attr = if strokes { "fill=\"none\" stroke=\"#111827\" stroke-width=\"2\"" } else { "fill=\"#111827\" stroke=\"none\"" };
    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {w} {h}\" width=\"{w}\" height=\"{h}\">\n  <path d=\"M {x0:.1} {y0:.1} L {x1:.1} {y0:.1} L {x1:.1} {y1:.1} L {x0:.1} {y1:.1} Z\" {paint_attr}/>\n</svg>\n"
    );
    if let Some(parent) = out_svg.parent() {
        std::fs::create_dir_all(parent)?;
    }
    craft_atomic::atomic_write(out_svg, svg.as_bytes())?;
=======
        "engine": "preview-keyer",
    }))
}

/// Trace a vector contour SVG via host `VectorCraft` (`vectorcraft --control` or `vectorcraft-cli`),
/// returning `(summary_json, centered_outline_points)`.
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
    let mut left_pts = Vec::new();
    let mut right_pts = Vec::new();
    let mut y = 0;
    while y < h {
        let mut min_x = None;
        let mut max_x = None;
        for x in 0..w {
            if rgba.get_pixel(x, y)[3] > 24 {
                if min_x.is_none() {
                    min_x = Some(x);
                }
                max_x = Some(x);
            }
        }
        if let (Some(lx), Some(rx)) = (min_x, max_x) {
            left_pts.push((lx, y));
            right_pts.push((rx + 1, y));
        }
        y = y.saturating_add(step);
    }
    let cx = f64::from(w) * 0.5;
    let cy = f64::from(h) * 0.5;
    let mut centered: Vec<[f64; 2]> = Vec::new();
    if left_pts.is_empty() {
        centered.extend([[-cx, -cy], [cx, -cy], [cx, cy], [-cx, cy]]);
    } else {
        for &(x, py) in &left_pts {
            centered.push([f64::from(x) - cx, f64::from(py) - cy]);
        }
        for &(x, py) in right_pts.iter().rev() {
            centered.push([f64::from(x) - cx, f64::from(py) - cy]);
        }
    }

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
            json!({
                "ok": true,
                "input": input,
                "output": output,
                "width": w,
                "height": h,
                "engine": "vectorcraft",
            }),
            centered,
        ));
    }

    let mut d = String::new();
    if left_pts.is_empty() {
        let _ = write!(d, "M 0 0 L {w} 0 L {w} {h} L 0 {h} Z");
    } else {
        for (idx, (x, py)) in left_pts.iter().enumerate() {
            let cmd = if idx == 0 { 'M' } else { 'L' };
            let _ = write!(d, "{cmd} {x} {py} ");
        }
        for (x, py) in right_pts.iter().rev() {
            let _ = write!(d, "L {x} {py} ");
        }
        d.push('Z');
    }
    let attr = if strokes { "fill=\"none\" stroke=\"#111827\" stroke-width=\"2\"" } else { "fill=\"#111827\"" };
    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {w} {h}\" width=\"{w}\" height=\"{h}\">\n  <path d=\"{}\" {attr}/>\n</svg>\n",
        d.trim()
    );
    craft_atomic::atomic_write(output, svg.as_bytes())?;
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    Ok((
        json!({
            "ok": true,
            "input": input,
<<<<<<< HEAD
            "output": out_svg,
            "width": w,
            "height": h,
            "bounds": [x0, y0, x1 - x0, y1 - y0]
        }),
        pts,
    ))
}

/// Extract embedded PNG preview from a Craft document (`.pcraft`, `.vectorcraft`, `.ecproj`) or resize raster image.
pub fn extract_or_copy_preview(src: &Path, out_png: &Path, max_side: u32) -> Result<bool> {
    if let Some(parent) = out_png.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if magic_util::media::extract_craft_preview(src, out_png).unwrap_or(false) && out_png.is_file() {
        return Ok(true);
    }
    if let Ok(img) = image::open(src) {
        let (w, h) = img.dimensions();
        let out_img = if max_side > 0 && w.max(h) > max_side { img.thumbnail(max_side, max_side).to_rgba8() } else { img.to_rgba8() };
        encode_png_atomic(&out_img, out_png)?;
        return Ok(true);
    }
    let stem_png = src.with_extension("png");
    if stem_png != src && stem_png.is_file() {
        std::fs::copy(&stem_png, out_png)?;
        return Ok(true);
    }
    Ok(false)
}

/// Probe audio or media container metadata in-process using `craft-audio` and `craft-media`.
pub fn probe_media_in_process(path: &Path) -> Result<Value> {
    if magic_util::media::is_pure_audio_file(path)
        && let Ok(audio) = magic_util::media::probe_audio_in_process(path)
    {
        return Ok(audio);
    }
    if let Ok(container) = magic_util::media::probe_container_in_process(path) {
        return Ok(container);
    }
    magic_util::media::probe_audio_in_process(path).map_err(|e| bad(format!("unable to probe media {}: {e}", path.display())))
}

/// Persist staged Craft command steps alongside a target document path so the user or target app's
/// own MCP server (`effectcraft-cli mcp`, `filmcraft-cli mcp`, etc.) can inspect or apply them.
pub fn write_staged_steps(target_doc: &Path, steps: &Value) -> Result<PathBuf> {
    let file_name = target_doc.file_name().and_then(|n| n.to_str()).unwrap_or("staged");
    let sidecar = target_doc.with_file_name(format!(".{file_name}.steps.json"));
    if let Some(parent) = sidecar.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(steps).map_err(|e| bad(e.to_string()))?;
    craft_atomic::atomic_write(&sidecar, &bytes)?;
    Ok(sidecar)
=======
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
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
}
