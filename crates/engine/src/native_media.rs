//! Host Craft application media delegation & local project preview helpers (`crates/engine/src/native_media.rs`).
//!
//! Subject isolation (`PhotoCraft`) and vector contour tracing (`VectorCraft`) execute against the
//! host machine's running Craft applications via `--control` or `<app>-cli`,
//! while local thumbnail/cell cropping and `craft-doc` preview extraction keep project UI cards fast.

use std::io::Cursor;
use std::path::Path;

use craft_doc::{Limits, RenditionKind};

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
    if input.is_dir() {
        for rel in ["preview.png", "QuickLook/Thumbnail.png"] {
            let p = input.join(rel);
            if p.is_file()
                && let Ok(img) = image::open(&p)
            {
                return Ok(img.to_rgba8());
            }
        }
    } else if input.is_file()
        && let Ok(bytes) = std::fs::read(input)
        && craft_doc::sniff(&bytes).is_some()
    {
        let limits = Limits::default();
        if let Ok(rend) =
            craft_doc::rendition(&bytes, RenditionKind::Composite, &limits).or_else(|_| craft_doc::rendition(&bytes, RenditionKind::Thumbnail, &limits))
            && let Ok(img) = image::load_from_memory(&rend.bytes)
        {
            return Ok(img.to_rgba8());
        }
    }
    Err(bad(format!("cannot decode raster or embedded Craft preview from {}", input.display())))
}

fn try_photocraft_isolate(input: &Path, output: &Path) -> bool {
    if crate::workspace::control_probe("photocraft") {
        let ok = crate::workspace::control_exec("photocraft", "file.open", json!({"path": input})).is_ok()
            && crate::workspace::control_exec("photocraft", "layer.removeBackground", json!({})).is_ok()
            && crate::workspace::control_exec("photocraft", "file.saveACopy", json!({"path": output, "overwrite": true})).is_ok();
        if ok && output.is_file() {
            return true;
        }
    }
    let Some(in_s) = input.to_str() else {
        return false;
    };
    let Some(out_s) = output.to_str() else {
        return false;
    };
    crate::workspace::run_cli("photocraft-cli", &["run", in_s, "--cmd", "layer.removeBackground", "--params", "{}", "--out", out_s], None).is_ok()
        && output.is_file()
}

/// Isolate a subject to transparent RGBA via host `PhotoCraft` (`photocraft --control` or `photocraft-cli`).
pub fn isolate_subject_rgba(input: &Path, output: &Path, _chroma_hex: Option<&str>, _feather_px: f64) -> Result<Value> {
    if input == output {
        return Err(bad(format!("refusing to overwrite source image {}", input.display())));
    }
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }

    if try_photocraft_isolate(input, output) {
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

    Err(EngineError::ExternalTool("photocraft".into(), "PhotoCraft subject isolation unavailable or failed".into()))
}

fn try_vectorcraft_trace(input: &Path, output: &Path) -> bool {
    if crate::workspace::control_probe("vectorcraft") {
        let ok = crate::workspace::control_exec("vectorcraft", "document.open", json!({"path": input})).is_ok()
            && crate::workspace::control_exec("vectorcraft", "imageTrace.makeAndExpand", json!({})).is_ok()
            && crate::workspace::control_exec("vectorcraft", "document.export", json!({"path": output, "format": "svg"})).is_ok();
        if ok && output.is_file() {
            return true;
        }
    }
    let Some(in_s) = input.to_str() else {
        return false;
    };
    let Some(out_s) = output.to_str() else {
        return false;
    };
    crate::workspace::run_cli("vectorcraft-cli", &["run", "--in", in_s, "--cmd", "imageTrace.makeAndExpand", "--export", out_s], None).is_ok()
        && output.is_file()
}

fn try_photocraft_split_layers(src: &Path, layers_dir: &Path) -> bool {
    let export_params = json!({"dir": layers_dir, "prefix": "part"});
    if crate::workspace::control_probe("photocraft") {
        let ok = crate::workspace::control_exec("photocraft", "file.open", json!({"path": src})).is_ok()
            && crate::workspace::control_exec("photocraft", "layer.maskAllObjects", json!({})).is_ok()
            && crate::workspace::control_exec("photocraft", "file.export.layersToFiles", export_params.clone()).is_ok();
        if ok {
            return true;
        }
    }
    let Some(src_s) = src.to_str() else {
        return false;
    };
    let exp_s = export_params.to_string();
    crate::workspace::run_cli(
        "photocraft-cli",
        &["run", src_s, "--cmd", "layer.maskAllObjects", "--params", "{}", "--cmd", "file.export.layersToFiles", "--params", &exp_s],
        None,
    )
    .is_ok()
}
pub fn trace_silhouette_svg(input: &Path, output: &Path, _simplify_px: f64, _strokes: bool) -> Result<(Value, Vec<[f64; 2]>)> {
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if try_vectorcraft_trace(input, output) {
        let (w, h, _) = magic_util::media::fast_dimensions(input).unwrap_or((0, 0, true));
        return Ok((json!({ "ok": true, "input": input, "output": output, "width": w, "height": h, "engine": "vectorcraft" }), Vec::new()));
    }
    Err(EngineError::ExternalTool("vectorcraft".into(), "VectorCraft silhouette trace unavailable or failed".into()))
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

/// Separate a character `parts-sheet.png` via host `PhotoCraft`.
pub fn split_parts_sheet(sheet: &Path, view_dir: &Path, char_id: &str, view: &str, chroma: Option<&str>) -> Result<Value> {
    let layers_dir = view_dir.join("layers");
    std::fs::create_dir_all(&layers_dir)?;
    let isolated = view_dir.join(format!("{char_id}-{view}-sheet-isolated.png"));
    let src_for_cells = if isolate_subject_rgba(sheet, &isolated, chroma, 1.0).is_ok() && isolated.is_file() { isolated } else { sheet.to_path_buf() };

    if !try_photocraft_split_layers(&src_for_cells, &layers_dir) {
        return Err(EngineError::ExternalTool("photocraft".into(), "PhotoCraft parts separation unavailable or failed".into()));
    }
    Ok(json!({
        "ok": true,
        "character": char_id,
        "view": view,
        "layers_dir": layers_dir,
        "engine": "photocraft",
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
