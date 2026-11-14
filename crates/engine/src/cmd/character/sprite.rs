//! `character.sprite`: sprite-sheet frame cropping (`PhotoCraft` CLI alignment with alpha-baseline
//! fallback, `B10-02`) and explicit `front` / `side` / `back` directional `.ecproj` cycles (`B10-03`).

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::{bad, effectcraft, step};
use crate::{EngineError, Result, Session};

/// Rectangular crop cell `[x, y, width, height]` inside a `cols x rows` sprite sheet of `(width, height)`.
pub fn sprite_cell_rect(width: u32, height: u32, grid: (u32, u32), index: u32) -> [u32; 4] {
    let cols = grid.0.max(1);
    let rows = grid.1.max(1);
    let cw = (width / cols).max(1);
    let ch = (height / rows).max(1);
    let col = index % cols;
    let row = index / cols;
    [col * cw, row * ch, cw, ch]
}

pub(crate) fn sprite_dir(s: &Session, id: &str, name: &str) -> Result<PathBuf> {
    Ok(crate::workshop::project_dir(s)?.join("assets/characters").join(id).join("sprites").join(name))
}

fn try_photocraft_align_cell(sheet: &Path, rect: [u32; 4], lock_baseline: bool, out: &Path) -> bool {
    let [x, y, w, h] = rect;
    let crop_params = json!({"x": x, "y": y, "width": w, "height": h});
    let align_cmd = if lock_baseline { "layer.align.bottomEdges" } else { "layer.align.horizontalCenters" };
    let Some(sheet_s) = sheet.to_str() else {
        return false;
    };
    let Some(out_s) = out.to_str() else {
        return false;
    };
    let crop_s = crop_params.to_string();
    crate::workspace::run_cli(
        "photocraft-cli",
        &["run", sheet_s, "--cmd", "image.crop", "--params", &crop_s, "--cmd", align_cmd, "--params", "{}", "--out", out_s],
        None,
    )
    .is_ok()
        && out.is_file()
}

fn crop_frames(dir: &Path, sheet: &Path, grid: (u32, u32), lock_baseline: bool, chroma_hex: Option<&str>) -> Result<Vec<PathBuf>> {
    let clean_sheet = dir.join("sheet-cutout.png");
    crate::native_media::isolate_subject_rgba(sheet, &clean_sheet, chroma_hex, 0.8)?;
    let rgba = crate::native_media::load_rgba(&clean_sheet)?;
    let (w, h) = rgba.dimensions();
    let (cols, rows) = (grid.0.max(1), grid.1.max(1));
    let frames_dir = dir.join("frames");
    if frames_dir.is_dir() {
        std::fs::remove_dir_all(&frames_dir)?;
    }
    std::fs::create_dir_all(&frames_dir)?;
    let mut out = Vec::with_capacity((cols * rows) as usize);
    for i in 0..cols * rows {
        let rect = sprite_cell_rect(w, h, (cols, rows), i);
        let file = frames_dir.join(format!("frame_{:04}.png", i + 1));
        if !try_photocraft_align_cell(&clean_sheet, rect, lock_baseline, &file) {
            return Err(EngineError::ExternalTool("photocraft".into(), "PhotoCraft sprite cell alignment unavailable or failed".into()));
        }
        out.push(file);
    }
    Ok(out)
}

fn resolve_sprite_direction<'a>(p: &'a Value, action: &str) -> Result<&'a str> {
    if p["mirror"] == true {
        return Err(bad("horizontal scale mirroring (`mirror=true`) is disabled; pass an explicit `direction` (`front`, `side`, or `back`) sprite sheet"));
    }
    let default_dir = if matches!(action, "walk" | "run") { "side" } else { "front" };
    let direction = p["direction"].as_str().or_else(|| p["view"].as_str()).map(str::trim).filter(|d| !d.is_empty()).unwrap_or(default_dir);
    if !matches!(direction, "front" | "side" | "back") {
        return Err(bad("direction must be front, side, or back"));
    }
    Ok(direction)
}

pub fn cmd_character_sprite(s: &mut Session, p: &Value) -> Result<Value> {
    let id = p["character"].as_str().map(crate::workspace::sanitize_asset_id).ok_or_else(|| bad("character=<id> is required"))?;
    let action = p["action"].as_str().or_else(|| p["name"].as_str()).ok_or_else(|| bad("action=<walk|jump|…> is required"))?;
    let direction = resolve_sprite_direction(p, action)?;
    let name = p["name"].as_str().map(str::to_owned).unwrap_or_else(|| {
        if p.get("direction").is_some() && !action.ends_with(&format!("_{direction}")) { format!("{action}_{direction}") } else { action.to_owned() }
    });
    let sheet = p["sheet"]
        .as_str()
        .map(|f| crate::workspace::resolve_workspace_path(s, f))
        .filter(|f| f.is_file())
        .ok_or_else(|| bad("sheet=<action sheet PNG> is required"))?;
    let grid = (p["columns"].as_u64().or_else(|| p["cols"].as_u64()).unwrap_or(4).clamp(1, 16) as u32, p["rows"].as_u64().unwrap_or(2).clamp(1, 16) as u32);
    let fps = p["fps"].as_u64().unwrap_or(12).clamp(1, 60);
    let duration = p["duration"].as_f64().unwrap_or(4.0).clamp(0.5, 600.0);
    let chroma = p["chroma"].as_str().filter(|v| !v.trim().is_empty());
    let dir = sprite_dir(s, &id, &name)?;
    std::fs::create_dir_all(&dir)?;

    let lock_baseline = !matches!(action, "jump" | "fly");
    let target_sheet = dir.join("sheet.png");
    if sheet.canonicalize().ok() != target_sheet.canonicalize().ok() {
        std::fs::copy(&sheet, &target_sheet)?;
    }
    let frames = crop_frames(&dir, &target_sheet, grid, lock_baseline, chroma)?;

    let comp = format!("{id}-{name}");
    let project = dir.join(format!("{comp}.ecproj"));
    let steps = vec![
        step("file.import", json!({"paths": [frames[0]], "sequence": true, "frameRate": fps, "newComp": true})),
        step("comp.settings", json!({"name": comp, "duration": duration, "frameRate": fps})),
        step("layer.quickSetAnchor", json!({"layers": [1], "point": if lock_baseline { "bottom" } else { "center" }})),
        step("layer.enableTimeRemap", json!({"layers": [1], "value": true})),
        step("layer.timing", json!({"layers": [1], "out": duration})),
        step("prop.setExpression", json!({"layer": 1, "path": "timeRemap", "expression": r#"loopOut("cycle")"#})),
        step("file.saveAs", json!({"path": project})),
    ];
    effectcraft(&dir, Some(&project), &steps, true)?;

    let preview_png = project.with_file_name(format!("{comp}-preview.png"));
    if let Some(first_frame) = frames.first() {
        let _ = std::fs::copy(first_frame, &preview_png);
    }

    let meta = json!({
        "character": id,
        "sprite": name,
        "action": action,
        "direction": direction,
        "animation_style": "sprite_2d",
        "sheet": target_sheet,
        "source_sheet": sheet,
        "source_kind": "sprite_sheet",
        "frames": frames.len(),
        "fps": fps,
        "cycle_seconds": frames.len() as f64 / fps as f64,
        "duration": duration,
        "comp": comp,
        "ecproj": project,
        "preview": preview_png,
    });
    Ok(json!({
        "ok": true,
        "sprite": meta,
        "effect_project": project,
        "next": format!("character.shot character={id} sprite={name} studio=true")
    }))
}

#[cfg(test)]
mod tests {
    use image::{Rgba, RgbaImage};
    use serde_json::json;

    use super::cmd_character_sprite;
    use crate::Session;

    #[test]
    fn b10_03_explicit_direction_replaces_horizontal_mirroring() {
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("magic_b10_03_{nanos}"));
        std::fs::create_dir_all(&dir).unwrap();

        let sheet_path = dir.join("walk-sheet.png");
        let mut sheet = RgbaImage::from_pixel(32, 16, Rgba([0, 255, 0, 255]));
        sheet.put_pixel(8, 8, Rgba([240, 100, 50, 255]));
        sheet.put_pixel(24, 8, Rgba([240, 100, 50, 255]));
        crate::native_media::encode_png_atomic(&sheet, &sheet_path).unwrap();

        let mut session = Session { project_path: Some(dir.join("song.magic")), ..Default::default() };

        let mirror_err =
            cmd_character_sprite(&mut session, &json!({"character": "pip", "action": "walk", "sheet": sheet_path, "columns": 2, "rows": 1, "mirror": true}))
                .unwrap_err()
                .to_string();
        assert!(mirror_err.contains("direction"), "expected direction guidance, got: {mirror_err}");

        for dir_name in ["front", "side", "back"] {
            let res = cmd_character_sprite(
                &mut session,
                &json!({"character": "pip", "action": "walk", "direction": dir_name, "sheet": sheet_path, "columns": 2, "rows": 1, "chroma": "#00FF00"}),
            )
            .unwrap();
            assert_eq!(res["sprite"]["direction"], dir_name);
            assert!(res["sprite"].get("mirror").is_none());
        }

        let _ = std::fs::remove_dir_all(&dir);
    }
}
