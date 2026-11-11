//! `character.sprite`: a Scribbles action sheet (or headless `anicraft-cli` `.anicraft` document)
//! becomes a 2D sprite cycle in `EffectCraft` (`.ecproj`).
//! - For raster sheets, `PhotoCraft` isolates the subject (`layer.removeBackground` / `select.colorRange`
//!   + `select.refineEdge`) and crops/trims each cell (`image.crop`, `image.trim`, `image.canvasSize`).
//! - For `.anicraft` documents, headless `anicraft-cli render-seq` exports PNG frames (never opening
//!   the pre-alpha AniCraft desktop UI) so `EffectCraft` composites and puppet-pins the sequence safely.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::{bad, effectcraft, step};
use crate::{CraftHub, CraftTarget, Result, Session};

pub(crate) fn sprite_dir(s: &Session, id: &str, name: &str) -> Result<PathBuf> {
    Ok(crate::workshop::project_dir(s)?.join("assets/characters").join(id).join("sprites").join(name))
}

/// Remove background via `PhotoCraft` (`layer.removeBackground` or `select.colorRange` + `select.refineEdge`)
/// and crop/trim/pad each cell of the `cols x rows` action sheet via `PhotoCraft` `image.crop` + `image.trim`.
fn crop_frames(dir: &Path, sheet: &Path, grid: (u32, u32), lock_baseline: bool, chroma_hex: Option<&str>) -> Result<Vec<PathBuf>> {
    let clean_sheet = dir.join("sheet-cutout.png");
    CraftHub::remove_background_photo(sheet, &clean_sheet, chroma_hex, 0.8)?;
    let (w, h) = crate::cmd::character::cut::photocraft_dimensions(&clean_sheet)?;
    let (cols, rows) = (grid.0.max(1), grid.1.max(1));
    let anchor = if lock_baseline { "bottom" } else { "center" };
    let frames_dir = dir.join("frames");
    if frames_dir.is_dir() {
        std::fs::remove_dir_all(&frames_dir)?;
    }
    std::fs::create_dir_all(&frames_dir)?;
    let mut out = Vec::with_capacity((cols * rows) as usize);
    for i in 0..cols * rows {
        let [cx, cy, cw, ch] = craft_scribbles::sprite_cell_rect(w, h, (cols, rows), i);
        let file = frames_dir.join(format!("frame_{:04}.png", i + 1));
        crate::cmd::character::cut::photocraft_run(
            &clean_sheet,
            &[
                ("image.crop", json!({"x": cx, "y": cy, "width": cw, "height": ch})),
                ("image.trim", json!({"basedOn": "transparentPixels"})),
                ("image.canvasSize", json!({"width": cw, "height": ch, "anchor": anchor})),
            ],
            Some(&file),
        )?;
        out.push(file);
    }
    Ok(out)
}

pub fn cmd_character_sprite(s: &mut Session, p: &Value) -> Result<Value> {
    let id = p["character"].as_str().map(crate::workspace::sanitize_asset_id).ok_or_else(|| bad("character=<id> is required"))?;
    let action = p["action"].as_str().or_else(|| p["name"].as_str()).ok_or_else(|| bad("action=<walk|jump|…> is required"))?;
    let name = p["name"].as_str().unwrap_or(action).to_string();
    let sheet = p["sheet"]
        .as_str()
        .map(|f| crate::workspace::resolve_workspace_path(s, f))
        .filter(|f| f.is_file())
        .ok_or_else(|| bad("sheet=<Scribbles action sheet PNG or .anicraft file> is required"))?;
    let grid = (p["columns"].as_u64().or_else(|| p["cols"].as_u64()).unwrap_or(4).clamp(1, 16) as u32, p["rows"].as_u64().unwrap_or(2).clamp(1, 16) as u32);
    let fps = p["fps"].as_u64().unwrap_or(12).clamp(1, 60);
    let duration = p["duration"].as_f64().unwrap_or(4.0).clamp(0.5, 600.0);
    let chroma = p["chroma"].as_str().filter(|v| !v.trim().is_empty());
    let dir = sprite_dir(s, &id, &name)?;
    std::fs::create_dir_all(&dir)?;

    let lock_baseline = !matches!(action, "jump" | "fly");
    let is_anicraft_doc = sheet.extension().is_some_and(|e| e.eq_ignore_ascii_case("anicraft") || e.eq_ignore_ascii_case("ani"));
    let (frames, stored_sheet) = if is_anicraft_doc {
        let frames_dir = dir.join("frames");
        if frames_dir.is_dir() {
            std::fs::remove_dir_all(&frames_dir)?;
        }
        let exported = CraftHub::anicraft_render_seq(&sheet, 0, None, &frames_dir)?;
        if exported.is_empty() {
            return Err(bad("anicraft-cli render-seq produced no PNG frames"));
        }
        (exported, sheet.clone())
    } else {
        let target_sheet = dir.join("sheet.png");
        if sheet.canonicalize().ok() != target_sheet.canonicalize().ok() {
            std::fs::copy(&sheet, &target_sheet)?;
        }
        (crop_frames(&dir, &target_sheet, grid, lock_baseline, chroma)?, target_sheet)
    };

    let comp = format!("{id}-{name}");
    let project = dir.join(format!("{comp}.ecproj"));
    let mut steps = vec![
        step("file.import", json!({"paths": [frames[0]], "sequence": true, "frameRate": fps, "newComp": true})),
        step("comp.settings", json!({"name": comp, "duration": duration, "frameRate": fps})),
        step("layer.quickSetAnchor", json!({"layers": [1], "point": if lock_baseline { "bottomCenter" } else { "center" }})),
        step("layer.enableTimeRemap", json!({"layers": [1], "value": true})),
        step("layer.timing", json!({"layers": [1], "out": duration})),
        step("prop.setExpression", json!({"layer": 1, "path": "timeRemap", "expression": "loopOut(\"cycle\")"})),
    ];
    if let Some(hex) = chroma {
        steps.push(step("effect.apply", json!({"layer": 1, "effect": "Keylight (1.2)"})));
        steps.push(step("effect.pickColor", json!({"layer": 1, "effect": "Keylight (1.2)", "color": hex})));
    }
    if p["mirror"] == true {
        steps.push(step("layer.setTransform", json!({"layers": [1], "prop": "scale", "value": [-100, 100]})));
    }
    steps.push(step("file.saveAs", json!({"path": project})));
    effectcraft(&dir, None, &steps, true)?;
    let preview_png = project.with_file_name(format!("{comp}-preview.png"));
    let _ = super::effectcraft_frame(&project, &comp, 0.0, 960, &preview_png);
    if s.project_path.is_some() && project.is_file() {
        let _ = s.execute(
            "project.add_asset",
            json!({"path": project, "kind": "character", "name": format!("{id} {action}"), "if_not_exists": true, "load": false}),
        );
    }

    let anicraft_cli_available = CraftHub::is_installed(CraftTarget::Ani);
    let meta = json!({
        "character": id,
        "sprite": name,
        "action": action,
        "animation_style": "sprite_2d",
        "sheet": stored_sheet,
        "source_sheet": sheet,
        "source_kind": if is_anicraft_doc { "anicraft_headless_seq" } else { "photocraft_sprite_sheet" },
        "frames": frames.len(),
        "fps": fps,
        "cycle_seconds": frames.len() as f64 / fps as f64,
        "duration": duration,
        "mirror": p["mirror"] == true,
        "comp": comp,
        "ecproj": project,
        "anicraft_cli_available": anicraft_cli_available,
    });
    crate::doc_state::write_json(&dir.join("sprite.json"), &meta)?;
    Ok(json!({
        "ok": true,
        "sprite": meta,
        "effect_project": project,
        "next": format!("character.shot character={id} sprite={name} studio=true")
    }))
}
