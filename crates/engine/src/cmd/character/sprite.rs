//! `character.sprite`: pure-Rust action-sheet frame cropping (`native_media`) + staged `.ecproj`
//! sprite-cycle specification on disk.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::{bad, effectcraft, step};
use crate::{Result, Session};

pub(crate) fn sprite_dir(s: &Session, id: &str, name: &str) -> Result<PathBuf> {
    Ok(crate::workshop::project_dir(s)?.join("assets/characters").join(id).join("sprites").join(name))
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
        let rect = magic_scribbles::sprite_cell_rect(w, h, (cols, rows), i);
        let file = frames_dir.join(format!("frame_{:04}.png", i + 1));
        crate::native_media::crop_and_anchor_cell(&rgba, rect, lock_baseline, &file)?;
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
        .ok_or_else(|| bad("sheet=<action sheet PNG> is required"))?;
    let grid = (p["columns"].as_u64().or_else(|| p["cols"].as_u64()).unwrap_or(4).clamp(1, 16) as u32, p["rows"].as_u64().unwrap_or(2).clamp(1, 16) as u32);
    let fps = p["fps"].as_u64().unwrap_or(12).clamp(1, 60);
    let duration = p["duration"].as_f64().unwrap_or(4.0).clamp(0.5, 600.0);
    let chroma = p["chroma"].as_str().filter(|v| !v.trim().is_empty());
    let dir = sprite_dir(s, &id, &name)?;
    std::fs::create_dir_all(&dir)?;

    let lock_baseline = !matches!(action, "jump" | "fly");
    if sheet.extension().is_some_and(|e| e.eq_ignore_ascii_case("anicraft") || e.eq_ignore_ascii_case("ani")) {
        return Err(bad("render `.anicraft` frame sequences directly with `anicraft-cli render-seq`, then pass the exported PNG sheet"));
    }
    let target_sheet = dir.join("sheet.png");
    if sheet.canonicalize().ok() != target_sheet.canonicalize().ok() {
        std::fs::copy(&sheet, &target_sheet)?;
    }
    let frames = crop_frames(&dir, &target_sheet, grid, lock_baseline, chroma)?;

    let comp = format!("{id}-{name}");
    let project = dir.join(format!("{comp}.ecproj"));
    let mut steps = vec![
        step("file.import", json!({"paths": [frames[0]], "sequence": true, "frameRate": fps, "newComp": true})),
        step("comp.settings", json!({"name": comp, "duration": duration, "frameRate": fps})),
        step("layer.quickSetAnchor", json!({"layers": [1], "point": if lock_baseline { "bottomCenter" } else { "center" }})),
        step("layer.enableTimeRemap", json!({"layers": [1], "value": true})),
        step("layer.timing", json!({"layers": [1], "out": duration})),
        // TODO(MAGIC-REVIEW): CHANGE execution to the installed EffectCraft session; this only corrects expression syntax.
        step("prop.setExpression", json!({"layer": 1, "path": "timeRemap", "expression": r#"loopOut("cycle")"#})),
    ];
    if p["mirror"] == true {
        steps.push(step("layer.setTransform", json!({"layers": [1], "prop": "scale", "value": [-100, 100]})));
    }
    steps.push(step("file.saveAs", json!({"path": project})));
    effectcraft(&dir, Some(&project), &steps, true)?;

    let preview_png = project.with_file_name(format!("{comp}-preview.png"));
    if let Some(first_frame) = frames.first() {
        let _ = std::fs::copy(first_frame, &preview_png);
    }

    let meta = json!({
        "character": id,
        "sprite": name,
        "action": action,
        "animation_style": "sprite_2d",
        "sheet": target_sheet,
        "source_sheet": sheet,
        "source_kind": "sprite_sheet",
        "frames": frames.len(),
        "fps": fps,
        "cycle_seconds": frames.len() as f64 / fps as f64,
        "duration": duration,
        "mirror": p["mirror"] == true,
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
