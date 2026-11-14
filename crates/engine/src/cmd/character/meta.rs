//! What Studio's Animation tab shows for one character from its native Craft documents.

use std::path::Path;

use serde_json::{Value, json};

use super::View;
use crate::{Result, Session};

/// A file's modification time, so Studio reloads images and frames after a change.
fn revision(path: &Path) -> u64 {
    std::fs::metadata(path).and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_millis() as u64)
}

fn view_summary(view: &View) -> Value {
    let drawing = super::find_initial_cutout(&view.dir);
    let ecproj = view.file("ecproj");
    let photo_doc = view.photo_doc();
    let size = drawing
        .as_ref()
        .and_then(|d| super::cut::photocraft_dimensions(d).ok().or_else(|| image::image_dimensions(d).ok()))
        .map(|(w, h)| json!([w, h]))
        .unwrap_or(Value::Null);
    json!({
        "view": view.view,
        "animation_style": if ecproj.is_file() || photo_doc.is_some() { "cutout_2_5d" } else { "cutout" },
        "drawing": drawing,
        "size": size,
        "cut": photo_doc.is_some(),
        "rigged": ecproj.is_file(),
        "ecproj": ecproj.is_file().then_some(&ecproj),
        "revision": revision(&ecproj),
    })
}

/// One character: every drawn view and what Craft step it has reached.
pub fn cmd_character_meta(s: &mut Session, p: &Value) -> Result<Value> {
    let mut views = Vec::new();
    for name in ["front", "side", "back"] {
        let mut params = p.clone();
        params["view"] = json!(name);
        let view = View::from_params(s, &params)?;
        if view.dir.is_dir() && (!view.is_flat || name == "front") {
            views.push(view_summary(&view));
        }
    }
    let project_dir = crate::workshop::project_dir(s)?;
    let raw_id = p["character"].as_str().map(crate::workspace::sanitize_asset_id).unwrap_or_default();
    let char_folder = super::character_folder(&project_dir, &raw_id);
    let char_id = char_folder.file_name().map(|n| crate::workspace::sanitize_asset_id(&n.to_string_lossy())).filter(|s| !s.is_empty()).unwrap_or(raw_id);
    if views.is_empty()
        && let Some(cutout) = super::find_initial_cutout(&char_folder)
    {
        let size = super::cut::photocraft_dimensions(&cutout)
            .ok()
            .or_else(|| image::image_dimensions(&cutout).ok())
            .map(|(w, h)| json!([w, h]))
            .unwrap_or(Value::Null);
        views.push(json!({
            "view": "front",
            "animation_style": "cutout",
            "drawing": cutout,
            "size": size,
            "status": "unmarked cutout; open in PhotoCraft/EffectCraft or run character.cut",
            "marked": false,
            "cut": false,
            "rigged": false,
            "ecproj": Value::Null,
            "revision": revision(&cutout),
        }));
    }
    let info_3d = super::mesh_io::inspect_character_3d(s, &project_dir, &char_id);
    let sprites = super::sprite_summaries(&char_folder);
    let (animation_style, animation_styles) = super::classify_animation_style(&views, &sprites, &info_3d);
    Ok(json!({
        "ok": true,
        "character": char_id,
        "animation_style": animation_style,
        "animation_styles": animation_styles,
        "views": views,
        "actions": [],
        "sprites": sprites,
        "model_3d": info_3d
    }))
}
