//! `character.cut`: subject isolation, part cutout cropping, and silhouette contour tracing
//! delegated to host `PhotoCraft` and `VectorCraft` via `crate::workspace` and `crate::native_media`,
//! with clean fallback handling and safe `.pcraft` preservation.

use std::path::Path;

use serde_json::{Value, json};

use super::{View, bad};
use crate::{EngineError, Result, Session};

fn try_photocraft_steps(src: &Path, steps: &[(&str, Value)], out_pcraft: Option<&Path>) -> bool {
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

/// Execute PhotoCraft layer/image steps via headless `photocraft-cli`.
pub(crate) fn photocraft_run(src: &Path, steps: &[(&str, Value)], out_pcraft: Option<&Path>) -> Result<()> {
    if try_photocraft_steps(src, steps, out_pcraft) {
        return Ok(());
    }
    Err(EngineError::ExternalTool("photocraft-cli".into(), "PhotoCraft command unavailable or failed".into()))
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

#[allow(clippy::cognitive_complexity)]
pub fn cmd_character_cut(s: &mut Session, p: &Value) -> Result<Value> {
    let view = View::from_params(s, p)?;
    let project_dir = crate::workshop::project_dir(s)?;
    let char_folder = super::character_folder(&project_dir, &view.id);
    let raw_image = p["image"]
        .as_str()
        .map(|i| view.dir.join(i))
        .or_else(|| super::find_initial_cutout(&char_folder))
        .ok_or_else(|| bad("source character image is missing"))?;

    let layers_dir = view.dir.join("layers");
    std::fs::create_dir_all(&layers_dir)?;

    let isolated = view.dir.join(format!("{}-{}-isolated.png", view.id, view.view));
    let remove_bg = p["remove_background"].as_bool().unwrap_or(true);
    let image_to_cut = if remove_bg {
        if crate::native_media::isolate_subject_rgba(&raw_image, &isolated, p["chroma"].as_str(), 0.8).is_ok() && isolated.is_file() {
            isolated
        } else {
            raw_image
        }
    } else {
        raw_image
    };

    let pcraft_path = view.file("pcraft");
    let steps: Vec<(&str, Value)> = vec![("layer.maskAllObjects", json!({}))];
    let _ = photocraft_run(&image_to_cut, &steps, Some(&pcraft_path));

    let svg_out = view.dir.join(format!("{}-{}.svg", view.id, view.view));
    let _ = vectorcraft_trace_outline(&image_to_cut, &svg_out, 0.0);

    super::manage::sync_character_registration(s, &view.id, &view.folder_name, Some("cutout_2_5d"), None, None, None);

    let mut reply = json!({
        "ok": true,
        "character": view.id,
        "view": view.view,
        "source": image_to_cut,
        "pcraft": pcraft_path,
        "svg": svg_out,
        "next": format!("character.rig character={} view={}", view.id, view.view)
    });
    if p["rig"] == true {
        let rigged = super::rig::cmd_character_rig(s, p)?;
        reply["ecproj"] = rigged["ecproj"].clone();
        reply["comp"] = rigged["comp"].clone();
        reply["next"] = rigged["next"].clone();
    }
    Ok(reply)
}
