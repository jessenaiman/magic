//! `character.sprite`: sprite-sheet frame cropping (`PhotoCraft` CLI alignment with alpha-baseline
//! fallback, `B10-02`) and explicit `front` / `side` / `back` directional `.ecproj` cycles (`B10-03`).

use std::path::{Path, PathBuf};

use image::RgbaImage;
use serde_json::{Value, json};

use super::{bad, effectcraft, step};
use crate::{Result, Session};

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
    let align_mode = if lock_baseline { "bottomCenter" } else { "center" };
    crate::craft_api::execute_on_host_app("photocraft", Some(sheet), "image.crop", json!({"x": x, "y": y, "width": w, "height": h})).is_ok()
        && crate::craft_api::execute_on_host_app("photocraft", Some(sheet), "layer.align", json!({"anchor": align_mode})).is_ok()
        && crate::craft_api::execute_on_host_app("photocraft", Some(sheet), "file.saveACopy", json!({"path": out, "overwrite": true})).is_ok()
        && out.is_file()
}

/// Deterministic alpha-bbox baseline fallback (`B10-02`): keeps uniform `(cw, ch)` frame dimensions
/// while centering horizontally and locking feet (`max_y`) to the cell floor when `lock_baseline` is true.
pub(crate) fn align_cell_alpha_baseline(rgba: &RgbaImage, rect: [u32; 4], lock_baseline: bool, out: &Path) -> Result<()> {
    let [cx, cy, cw, ch] = rect;
    let (iw, ih) = rgba.dimensions();
    let x0 = cx.min(iw.saturating_sub(1));
    let y0 = cy.min(ih.saturating_sub(1));
    let w = cw.min(iw.saturating_sub(x0)).max(1);
    let h = ch.min(ih.saturating_sub(y0)).max(1);
    let cell = image::imageops::crop_imm(rgba, x0, y0, w, h).to_image();

    let (mut min_x, mut min_y, mut max_x, mut max_y) = (w, h, 0_u32, 0_u32);
    for (px, py, pixel) in cell.enumerate_pixels() {
        if pixel[3] > 8 {
            min_x = min_x.min(px);
            min_y = min_y.min(py);
            max_x = max_x.max(px + 1);
            max_y = max_y.max(py + 1);
        }
    }

    let mut aligned = RgbaImage::new(w, h);
    if max_x > min_x && max_y > min_y {
        let bw = max_x - min_x;
        let bh = max_y - min_y;
        let sprite = image::imageops::crop_imm(&cell, min_x, min_y, bw, bh).to_image();
        let dst_x = i64::from(w.saturating_sub(bw) / 2);
        let dst_y = if lock_baseline { i64::from(h.saturating_sub(bh)) } else { i64::from(h.saturating_sub(bh) / 2) };
        image::imageops::replace(&mut aligned, &sprite, dst_x, dst_y);
    }
    crate::native_media::encode_png_atomic(&aligned, out)
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
            align_cell_alpha_baseline(&rgba, rect, lock_baseline, &file)?;
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
    crate::doc_state::write_json(&dir.join("sprite.json"), &meta)?;
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

    use super::{align_cell_alpha_baseline, cmd_character_sprite, sprite_cell_rect};
    use crate::Session;

    fn lowest_opaque_row(img: &RgbaImage) -> Option<u32> {
        let mut lowest = None;
        for (_, y, px) in img.enumerate_pixels() {
            if px[3] > 8 {
                lowest = Some(lowest.map_or(y, |prev: u32| prev.max(y)));
            }
        }
        lowest
    }

    #[test]
    fn b10_02_alpha_baseline_fallback_aligns_floating_frames() {
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("magic_b10_02_{nanos}"));
        std::fs::create_dir_all(&dir).unwrap();

        let mut sheet = RgbaImage::from_pixel(64, 32, Rgba([0, 0, 0, 0]));
        // Frame 0 floats high (y = 4..12), Frame 1 sits lower (y = 14..24).
        for y in 4..12 {
            for x in 6..14 {
                sheet.put_pixel(x, y, Rgba([220, 80, 60, 255]));
            }
        }
        for y in 14..24 {
            for x in 42..54 {
                sheet.put_pixel(x, y, Rgba([220, 80, 60, 255]));
            }
        }

        let f1 = dir.join("frame_0001.png");
        let f2 = dir.join("frame_0002.png");
        align_cell_alpha_baseline(&sheet, sprite_cell_rect(64, 32, (2, 1), 0), true, &f1).unwrap();
        align_cell_alpha_baseline(&sheet, sprite_cell_rect(64, 32, (2, 1), 1), true, &f2).unwrap();

        let img1 = image::open(&f1).unwrap().to_rgba8();
        let img2 = image::open(&f2).unwrap().to_rgba8();
        assert_eq!(img1.dimensions(), (32, 32));
        assert_eq!(img2.dimensions(), (32, 32));
        assert_eq!(lowest_opaque_row(&img1), Some(31));
        assert_eq!(lowest_opaque_row(&img2), Some(31));

        let _ = std::fs::remove_dir_all(&dir);
    }

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
