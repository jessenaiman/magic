//! `character.cut`: subject isolation, part cutout cropping, and silhouette contour tracing
//! delegated to host `PhotoCraft` and `VectorCraft` via `crate::workspace` and `crate::native_media`,
//! with clean fallback handling and safe `.pcraft` preservation.

use std::path::Path;

use serde_json::{Value, json};

use super::bad;
use crate::{EngineError, Result, Session};

fn try_photocraft_live(src: &Path, steps: &[(&str, Value)], out_pcraft: Option<&Path>) -> bool {
    if !crate::workspace::control_probe("photocraft") || crate::workspace::control_exec("photocraft", "file.open", json!({"path": src})).is_err() {
        return false;
    }
    for (op, params) in steps {
        if crate::workspace::control_exec("photocraft", op, params.clone()).is_err() {
            return false;
        }
    }
    if let Some(pcraft) = out_pcraft
        && crate::workspace::control_exec("photocraft", "file.saveAs", json!({"path": pcraft})).is_err()
    {
        return false;
    }
    true
}

fn try_photocraft_steps(src: &Path, steps: &[(&str, Value)], out_pcraft: Option<&Path>) -> bool {
    if try_photocraft_live(src, steps, out_pcraft) {
        return true;
    }
    let Some(src_s) = src.to_str() else {
        return false;
    };
    let mut args = vec!["run".to_owned(), src_s.to_owned()];
    for (op, params) in steps {
        args.extend(["--cmd".to_owned(), (*op).to_owned(), "--params".to_owned(), params.to_string()]);
    }
    if let Some(pcraft) = out_pcraft {
        args.extend(["--out".to_owned(), pcraft.to_string_lossy().into_owned()]);
    }
    if crate::workspace::run_cli("photocraft-cli", &args, None).is_err() {
        return false;
    }
    out_pcraft.is_none_or(Path::is_file)
        && steps.iter().filter(|(op, _)| *op == "file.saveACopy").all(|(_, p)| p["path"].as_str().is_none_or(|dst| Path::new(dst).is_file()))
}

/// Execute PhotoCraft layer/image steps via host `PhotoCraft` (`--control` / `photocraft-cli`).
pub(crate) fn photocraft_run(src: &Path, steps: &[(&str, Value)], out_pcraft: Option<&Path>) -> Result<()> {
    if try_photocraft_steps(src, steps, out_pcraft) {
        return Ok(());
    }
    Err(EngineError::ExternalTool("photocraft".into(), "PhotoCraft command unavailable or failed".into()))
}

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
    Ok(rel_pts.into_iter().map(|pt| json!([(pt[0] + cx).round() as i64, (pt[1] + cy).round() as i64])).collect())
}

pub fn cmd_character_cut(_s: &mut Session, _p: &Value) -> Result<Value> {
    Err(bad("pending Step 2 PhotoCraft/VectorCraft CLI chain"))
}
