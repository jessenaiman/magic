//! Scribbles sprite-sheet geometry and validated local character-bundle reader (`magic-scribbles`, Layer 4).
//!
//! Pure data and local image validation only: zero browser launches, zero `~/Downloads` scraping,
//! zero external CLI subprocesses, and zero network or LLM calls. All AI generation runs through
//! `storytold/craft-ai-integrations` in `magic-engine`, and all raster/vector cutout or rig builds
//! run through `PhotoCraft`, `VectorCraft`, and `EffectCraft`.

#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable))]

pub mod sheet;
pub use sheet::{ExtractedPart, PLUSH9_Z_ORDER, plush9_part_slots, sprite_cell_rect};

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, thiserror::Error)]
pub enum ScribblesError {
    #[error("{0}")]
    Invalid(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, ScribblesError>;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct IngestedCharacterBundle {
    pub character_id: String,
    pub character_name: String,
    pub view: String,
    pub action: String,
    pub grid: (u32, u32),
    pub source_dir: PathBuf,
    pub dest_dir: PathBuf,
    pub handoff_json: Option<PathBuf>,
    pub parts_json: Option<PathBuf>,
    pub character_png: Option<PathBuf>,
    pub parts_sheet_png: Option<PathBuf>,
    pub pose_sheet_png: Option<PathBuf>,
    pub metadata: Value,
}

pub fn ensure_valid_png(src: &Path, dst: &Path) -> Result<(u32, u32)> {
    let bytes = fs::read(src)?;
    if bytes.len() < 8 {
        return Err(ScribblesError::Invalid(format!("Image file {} is empty or truncated", src.display())));
    }
    let img = image::load_from_memory(&bytes)
        .map_err(|e| ScribblesError::Invalid(format!("Could not decode image {}: {e}", src.display())))?;
    let (width, height) = (img.width(), img.height());
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        if src != dst {
            fs::copy(src, dst)?;
        }
    } else {
        img.save_with_format(dst, image::ImageFormat::Png)
            .map_err(|e| ScribblesError::Invalid(format!("Failed to write PNG {}: {e}", dst.display())))?;
    }
    Ok((width, height))
}

fn newest_matching_file(dir: &Path, prefixes: &[&str], ext: &str) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else { continue };
        let lower = name.to_ascii_lowercase();
        if lower.ends_with(ext) && prefixes.iter().any(|p| lower.starts_with(p)) {
            let mtime = entry
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            if best.as_ref().is_none_or(|(t, _)| mtime > *t) {
                best = Some((mtime, path));
            }
        }
    }
    best.map(|(_, p)| p)
}

pub fn sanitize_id(raw: &str) -> String {
    let mut out = String::new();
    let mut last_sep = false;
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_sep = false;
        } else if !last_sep && !out.is_empty() {
            out.push('_');
            last_sep = true;
        }
    }
    out.trim_matches('_').to_string()
}

/// Ingests an explicitly specified character bundle directory (`character.png`, `parts-sheet.png`,
/// `pose-sheet.png`, optional `handoff.json` / `parts.json`) into `<project>/assets/characters/<id>/`.
pub fn ingest_bundle_dir(
    project_root: &Path,
    source_dir: &Path,
    requested_character_id: Option<&str>,
) -> Result<IngestedCharacterBundle> {
    if !source_dir.is_dir() {
        return Err(ScribblesError::Invalid(format!(
            "Character bundle directory {} does not exist",
            source_dir.display()
        )));
    }

    let handoff_src = newest_matching_file(source_dir, &["handoff", "bundle"], ".json");
    let mut meta = Value::Object(Default::default());
    if let Some(ref h_path) = handoff_src
        && let Ok(raw) = fs::read_to_string(h_path)
        && let Ok(parsed) = serde_json::from_str::<Value>(&raw)
    {
        meta = parsed;
    }

    let inferred_name = requested_character_id
        .filter(|s| !s.trim().is_empty())
        .or_else(|| meta["characterName"].as_str())
        .or_else(|| meta["name"].as_str())
        .unwrap_or("character");
    let char_id = {
        let clean = sanitize_id(inferred_name);
        if clean.is_empty() { "character".to_string() } else { clean }
    };

    let dest_dir = project_root.join("assets/characters").join(&char_id);
    fs::create_dir_all(&dest_dir)?;

    let view = meta["view"]
        .as_str()
        .or_else(|| meta["direction"].as_str())
        .map(|v| {
            let l = v.to_ascii_lowercase();
            if l.contains("right") || l.contains("left") || l.contains("side") {
                "side"
            } else if l.contains("back") || l.contains("up") {
                "back"
            } else {
                "front"
            }
        })
        .unwrap_or("front")
        .to_string();

    let action = meta["action"]
        .as_str()
        .map(sanitize_id)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| if view == "side" { "walk_right".into() } else { "idle".into() });

    let cols = meta["columns"].as_u64().or_else(|| meta["grid"][0].as_u64()).unwrap_or(2) as u32;
    let rows = meta["rows"].as_u64().or_else(|| meta["grid"][1].as_u64()).unwrap_or(2) as u32;

    let mut bundle = IngestedCharacterBundle {
        character_id: char_id.clone(),
        character_name: inferred_name.to_string(),
        view: view.clone(),
        action,
        grid: (cols.max(1), rows.max(1)),
        source_dir: source_dir.to_path_buf(),
        dest_dir: dest_dir.clone(),
        metadata: meta.clone(),
        ..Default::default()
    };

    if let Some(src) = handoff_src {
        let dst = dest_dir.join("handoff.json");
        if src != dst {
            let _ = fs::copy(&src, &dst);
        }
        bundle.handoff_json = Some(dst);
    }

    let view_dir = dest_dir.join(&view);
    fs::create_dir_all(&view_dir)?;
    if let Some(src) = newest_matching_file(source_dir, &["parts"], ".json") {
        let dst = view_dir.join("parts.json");
        if src != dst {
            let _ = fs::copy(&src, &dst);
        }
        bundle.parts_json = Some(dst);
    } else if meta.get("parts").and_then(Value::as_array).is_some() {
        let dst = view_dir.join("parts.json");
        let parts_doc = json!({
            "image": format!("{char_id}-{view}-drawing.png"),
            "view": view,
            "size": meta.get("size").cloned().unwrap_or_else(|| json!([1024, 1024])),
            "skeleton": meta.get("skeleton").and_then(Value::as_str).unwrap_or("plush-9"),
            "status": "ingested bundle; ready for character.cut and character.rig",
            "parts": meta["parts"],
        });
        if let Ok(bytes) = serde_json::to_vec_pretty(&parts_doc) {
            let _ = craft_atomic::atomic_write(&dst, &bytes);
            bundle.parts_json = Some(dst);
        }
    }

    if let Some(src) = newest_matching_file(source_dir, &["character"], ".png") {
        let dst = dest_dir.join("character.png");
        if ensure_valid_png(&src, &dst).is_ok() {
            let _ = fs::copy(&dst, view_dir.join(format!("{char_id}-{view}-drawing.png")));
            bundle.character_png = Some(dst);
        }
    }

    if let Some(src) = newest_matching_file(source_dir, &["parts-sheet", "parts_sheet"], ".png") {
        let dst = dest_dir.join("parts-sheet.png");
        if ensure_valid_png(&src, &dst).is_ok() {
            bundle.parts_sheet_png = Some(dst);
        }
    }

    if let Some(src) = newest_matching_file(source_dir, &["pose-sheet", "pose_sheet", "sprite-sheet"], ".png") {
        let dst = dest_dir.join("pose-sheet.png");
        if ensure_valid_png(&src, &dst).is_ok() {
            bundle.pose_sheet_png = Some(dst);
        }
    }

    Ok(bundle)
}
