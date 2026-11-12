//! Native Slide-Deck Storyboard (`storyboard.pptx` / `storyboard.deckcraft` + `storyboard/slides.json`)
//! modeled on the Barnyard 16:9 slide workflow and backed by `DeckCraft` (`:7990`, `deckcraft-cli`).
//!
//! Each Scene Beat slide (`960 × 540 pt` in DeckCraft coordinates) exposes:
//! - Tier 1 (Top-Left): Scene + Location main frame (`main_image` + `scene_location` + `prompt`)
//! - Tier 2 (Middle-Left): Active Character Models row (`character_models[]` -> `cutout.png`)
//! - Tier 3 (Bottom-Left): Simple Scene Assets-by-Type row (`scene_assets[]` / `asset_tags[]`)
//! - Right Metadata Card: `status`, `lyric_cue`, `planned_hold_sec`, `visible_cast`, `story_beat`, `continuity_lock`
//! - Bottom Slate Bar: `motion` (`magic_cli` call) + `editable_caption` (`C1` off artwork)

use std::io::Read;
use std::path::{Path, PathBuf};

use flate2::read::DeflateDecoder;
use serde_json::{Value, json};

use super::{PageInfo, Slot, bad, fail, slot_is_filled, u16_at, u32_at};
use crate::{Result, Session};

pub const SLIDE_W_PT: f64 = 960.0;
pub const SLIDE_H_PT: f64 = 540.0;

pub fn is_slide_storyboard(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or_default();
    if ext.eq_ignore_ascii_case("pptx") || ext.eq_ignore_ascii_case("deckcraft") {
        return true;
    }
    slides_json_path(path).is_file() && path.with_file_name("storyboard.pptx").is_file()
}

pub fn slides_json_path(doc_path: &Path) -> PathBuf {
    doc_path.with_file_name("storyboard").join("slides.json")
}

fn resolve_rel(base: &Path, raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() || Path::new(trimmed).is_absolute() {
        return trimmed.to_owned();
    }
    let cand = base.join(trimmed);
    if cand.exists() {
        cand.to_string_lossy().into_owned()
    } else {
        trimmed.to_owned()
    }
}

/// Extract plain `<a:t>` text runs per slide from a `.pptx` archive when `slides.json` is absent.
fn extract_pptx_slide_texts(pptx_path: &Path) -> Vec<String> {
    let Ok(bytes) = std::fs::read(pptx_path) else {
        return Vec::new();
    };
    if bytes.len() < 22 {
        return Vec::new();
    }
    let Some(eocd) = (0..=bytes.len() - 22).rev().find(|&i| &bytes[i..i + 4] == b"PK\x05\x06") else {
        return Vec::new();
    };
    let mut entries: Vec<(usize, String)> = Vec::new();
    let mut pos = u32_at(&bytes, eocd + 16);
    for _ in 0..u16_at(&bytes, eocd + 10) {
        if pos + 46 > bytes.len() || &bytes[pos..pos + 4] != b"PK\x01\x02" {
            break;
        }
        let (nlen, elen, clen) = (u16_at(&bytes, pos + 28), u16_at(&bytes, pos + 30), u16_at(&bytes, pos + 32));
        let name = std::str::from_utf8(&bytes[pos + 46..pos + 46 + nlen]).unwrap_or("");
        if let Some(rest) = name.strip_prefix("ppt/slides/slide").and_then(|s| s.strip_suffix(".xml"))
            && let Ok(num) = rest.parse::<usize>()
        {
            let local = u32_at(&bytes, pos + 42);
            let start = local + 30 + u16_at(&bytes, local + 26) + u16_at(&bytes, local + 28);
            let csize = u32_at(&bytes, pos + 20);
            if start + csize <= bytes.len() {
                let mut xml_bytes = Vec::new();
                if u16_at(&bytes, pos + 10) == 8 {
                    let _ = DeflateDecoder::new(&bytes[start..start + csize]).read_to_end(&mut xml_bytes);
                } else {
                    xml_bytes.extend_from_slice(&bytes[start..start + csize]);
                }
                let xml = String::from_utf8_lossy(&xml_bytes);
                let mut chunks = Vec::new();
                for part in xml.split("<a:t>").skip(1) {
                    if let Some((txt, _)) = part.split_once("</a:t>") {
                        let t = txt.trim();
                        if !t.is_empty() {
                            chunks.push(t.to_owned());
                        }
                    }
                }
                entries.push((num, chunks.join(" · ")));
            }
        }
        pos += 46 + nlen + elen + clen;
    }
    entries.sort_by_key(|(n, _)| *n);
    entries.into_iter().map(|(_, t)| t).collect()
}

pub fn slide_sections(doc_path: &Path) -> Vec<String> {
    let sj_path = slides_json_path(doc_path);
    if let Ok(raw) = std::fs::read_to_string(&sj_path)
        && let Ok(val) = serde_json::from_str::<Value>(&raw)
        && let Some(arr) = val["slides"].as_array()
    {
        let mut secs = Vec::new();
        for sl in arr {
            if let Some(sec) = sl["section"].as_str()
                && !secs.iter().any(|x| x == sec)
            {
                secs.push(sec.to_owned());
            }
        }
        if !secs.is_empty() {
            return secs;
        }
    }
    vec!["Overview".into(), "Timeline".into(), "Scenes".into(), "Handoff".into()]
}

pub(crate) fn parse_slide_storyboard(_s: Option<&Session>, doc_path: &Path) -> Vec<PageInfo> {
    let base_dir = doc_path.parent().unwrap_or_else(|| Path::new("."));
    let sj_path = slides_json_path(doc_path);
    if let Ok(raw) = std::fs::read_to_string(&sj_path)
        && let Ok(val) = serde_json::from_str::<Value>(&raw)
        && let Some(slides) = val["slides"].as_array()
    {
        return slides
            .iter()
            .enumerate()
            .map(|(idx, sl)| build_page_from_slide_json(idx, sl, base_dir))
            .collect();
    }

    // Fallback when a raw Google Slides / DeckCraft `.pptx` is placed without `slides.json`
    extract_pptx_slide_texts(doc_path)
        .into_iter()
        .enumerate()
        .map(|(idx, text)| {
            let section = if idx == 0 { "Overview" } else { "Scenes" }.to_owned();
            let filled = slot_is_filled("text", Some(&text), None);
            let slot = Slot {
                label: "slide_summary".into(),
                form_name: format!("s{idx:02}_slide_summary"),
                id: (idx as u64 + 1) * 100,
                kind: "text",
                text: Some(text.clone()),
                image: None,
                linked_file: None,
                asset_id: None,
                craft_app: Some("DeckCraft".into()),
                outline: Some("960x540 16:9 Slide".into()),
                time_sec: None,
                bounds: [24.0, 62.0, 936.0, 512.0],
                locked: false,
                filled,
                cli_command: format!("magic-cli storyboard set page={idx} label=slide_summary text=\"...\""),
            };
            PageInfo {
                index: idx,
                spread_index: idx,
                page_in_spread: 0,
                spread_page_count: 1,
                section,
                page_role: if idx == 0 { "cover" } else { "subpage" },
                sub_code: format!("S{:02}", idx + 1),
                bounds: [0.0, 0.0, SLIDE_W_PT, SLIDE_H_PT],
                heading: Some(text.chars().take(72).collect()),
                slots: vec![slot],
                layout_items: Vec::new(),
            }
        })
        .collect()
}

fn build_page_from_slide_json(idx: usize, sl: &Value, base_dir: &Path) -> PageInfo {
    let section = sl["section"].as_str().unwrap_or("Scenes").to_owned();
    let title = sl["title"].as_str().unwrap_or("Storyboard Slide").to_owned();
    let mut slots = Vec::new();
    let mut next_id = (idx as u64 + 1) * 1000;

    // 1. Main Scene + Location Image Slot (Tier 1 Top-Left)
    if let Some(main_raw) = sl["main_image"].as_str() {
        let resolved = resolve_rel(base_dir, main_raw);
        let linked = sl["linked_media"].as_str().map(|l| resolve_rel(base_dir, l)).or_else(|| Some(resolved.clone()));
        let filled = slot_is_filled("image", None, Some(&resolved));
        next_id += 1;
        slots.push(Slot {
            label: "main_scene_image".into(),
            form_name: format!("s{idx:02}_main_scene_image"),
            id: next_id,
            kind: "image",
            text: sl["scene_location"].as_str().map(str::to_owned),
            image: Some(resolved),
            linked_file: linked,
            asset_id: sl["shot_id"].as_str().map(str::to_owned),
            craft_app: Some("FilmCraft".into()),
            outline: Some("16:9 Scene + Location".into()),
            time_sec: sl["planned_hold_sec"].as_f64(),
            bounds: [27.0, 70.0, 566.0, 290.0],
            locked: false,
            filled,
            cli_command: format!("magic-cli storyboard set page={idx} label=main_scene_image image=PATH"),
        });
    }

    // 2. Active Character Models Row (Tier 2 Middle-Left, directly below Main Image!)
    for (ci, ch_val) in sl["character_models"].as_array().into_iter().flatten().enumerate() {
        if let Some(ch_raw) = ch_val.as_str() {
            let resolved = resolve_rel(base_dir, ch_raw);
            let filled = slot_is_filled("image", None, Some(&resolved));
            next_id += 1;
            let x0 = 27.0 + (ci as f64) * 106.0;
            slots.push(Slot {
                label: format!("character_model_{}", ci + 1),
                form_name: format!("s{idx:02}_character_model_{}", ci + 1),
                id: next_id,
                kind: "image",
                text: None,
                image: Some(resolved.clone()),
                linked_file: Some(resolved),
                asset_id: Some(format!("char_{}", ci + 1)),
                craft_app: Some("EffectCraft".into()),
                outline: Some("RGBA8 Cutout Model".into()),
                time_sec: None,
                bounds: [x0, 296.0, (x0 + 100.0).min(566.0), 374.0],
                locked: false,
                filled,
                cli_command: format!("magic-cli storyboard set page={idx} label=character_model_{} image=PATH", ci + 1),
            });
        }
    }

    // 3. Scene Assets Row + Barnyard Text Fields (Tier 3 Bottom-Left + Right Metadata Card + Bottom Slate Bar)
    let text_fields: &[(&str, &str, [f64; 4])] = &[
        ("scene_location", "scene_location", [414.0, 76.0, 560.0, 140.0]),
        ("prompt", "prompt", [414.0, 144.0, 560.0, 286.0]),
        ("lyric_cue", "lyric_cue", [580.0, 112.0, 928.0, 168.0]),
        ("visible_cast", "visible_cast", [740.0, 174.0, 928.0, 218.0]),
        ("story_beat", "story_beat", [580.0, 224.0, 928.0, 308.0]),
        ("continuity_lock", "continuity_lock", [580.0, 314.0, 928.0, 382.0]),
        ("magic_cli", "magic_cli", [580.0, 388.0, 928.0, 434.0]),
        ("motion", "motion", [32.0, 444.0, 470.0, 520.0]),
        ("editable_caption", "editable_caption", [480.0, 444.0, 928.0, 520.0]),
    ];
    for &(lbl, key, bnds) in text_fields {
        if let Some(val_str) = sl[key].as_str() {
            let filled = slot_is_filled("text", Some(val_str), None);
            next_id += 1;
            slots.push(Slot {
                label: lbl.into(),
                form_name: format!("s{idx:02}_{lbl}"),
                id: next_id,
                kind: "text",
                text: Some(val_str.to_owned()),
                image: None,
                linked_file: None,
                asset_id: None,
                craft_app: Some("DeckCraft".into()),
                outline: None,
                time_sec: sl["planned_hold_sec"].as_f64(),
                bounds: bnds,
                locked: false,
                filled,
                cli_command: format!("magic-cli storyboard set page={idx} label={lbl} text=\"...\""),
            });
        }
    }

    PageInfo {
        index: idx,
        spread_index: idx,
        page_in_spread: 0,
        spread_page_count: 1,
        section,
        page_role: "subpage",
        sub_code: format!("SC-{:02}", idx + 1),
        bounds: [0.0, 0.0, SLIDE_W_PT, SLIDE_H_PT],
        heading: Some(title),
        slots,
        layout_items: sl.get("asset_tags").cloned().and_then(|v| v.as_array().cloned()).unwrap_or_default(),
    }
}

pub fn update_slide_slot(doc_path: &Path, slide_idx: usize, p: &Value) -> Result<Value> {
    let sj_path = slides_json_path(doc_path);
    let mut root: Value = std::fs::read_to_string(&sj_path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_else(|| json!({"schema": "magic.storyboard.slides.v1", "slides": []}));
    let arr = root["slides"].as_array_mut().ok_or_else(|| bad("invalid slides.json"))?;
    let slide = arr.get_mut(slide_idx).ok_or_else(|| bad(&format!("no slide {slide_idx}")))?;
    if let Some(lbl) = p.get("label").or_else(|| p.get("slot")).and_then(Value::as_str) {
        if let Some(txt) = p.get("text").and_then(Value::as_str) {
            slide[lbl] = json!(txt);
        }
        if let Some(img) = p.get("image").and_then(Value::as_str) {
            if lbl == "main_scene_image" {
                slide["main_image"] = json!(img);
            } else {
                slide[lbl] = json!(img);
            }
        }
    }
    if let Some(dir) = sj_path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    craft_atomic::atomic_write(&sj_path, &serde_json::to_vec_pretty(&root).map_err(fail)?)?;
    Ok(json!({"ok": true, "format": "deckcraft-slides", "slide": slide_idx, "path": doc_path}))
}

pub fn append_slide_beat(doc_path: &Path, p: &Value) -> Result<Value> {
    let sj_path = slides_json_path(doc_path);
    let mut root: Value = std::fs::read_to_string(&sj_path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_else(|| json!({"schema": "magic.storyboard.slides.v1", "slides": []}));
    let arr = root["slides"].as_array_mut().ok_or_else(|| bad("invalid slides.json"))?;
    let idx = arr.len();
    let shot_id = p.get("shot_id").and_then(Value::as_str).unwrap_or("NEW");
    let title = p.get("heading").or_else(|| p.get("title")).and_then(Value::as_str).unwrap_or("New Scene Beat");
    arr.push(json!({
        "slide": idx,
        "section": p.get("section").and_then(Value::as_str).unwrap_or("Scenes"),
        "shot_id": shot_id,
        "title": format!("{shot_id} · {title}"),
        "scene_location": p.get("scene_location").and_then(Value::as_str).unwrap_or("L01 Proscenium Stage"),
        "main_image": p.get("image").and_then(Value::as_str).unwrap_or("cover_key_art.png"),
        "character_models": p.get("character_models").cloned().unwrap_or_else(|| json!([])),
        "scene_assets": p.get("scene_assets").cloned().unwrap_or_else(|| json!([])),
        "status": "NEW · IDENTITY CHECKED",
        "lyric_cue": p.get("lyric_cue").and_then(Value::as_str).unwrap_or("TBD"),
        "planned_hold_sec": p.get("planned_hold_sec").and_then(Value::as_u64).unwrap_or(7),
        "visible_cast": p.get("visible_cast").and_then(Value::as_str).unwrap_or("Cast"),
        "story_beat": p.get("story_beat").and_then(Value::as_str).unwrap_or(title),
        "prompt": p.get("prompt").and_then(Value::as_str).unwrap_or(title),
        "continuity_lock": p.get("continuity_lock").and_then(Value::as_str).unwrap_or("Preserve character cutouts."),
        "motion": p.get("motion").and_then(Value::as_str).unwrap_or("Gentle push-in"),
        "editable_caption": p.get("editable_caption").and_then(Value::as_str).unwrap_or(""),
        "magic_cli": "magic-cli storyboard check",
    }));
    if let Some(dir) = sj_path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    craft_atomic::atomic_write(&sj_path, &serde_json::to_vec_pretty(&root).map_err(fail)?)?;
    Ok(json!({"ok": true, "format": "deckcraft-slides", "added_slide": idx, "path": doc_path}))
}
