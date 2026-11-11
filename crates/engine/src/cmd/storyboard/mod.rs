//! Storyboard: printable reference document (`storyboard.pdf`) governed strictly by the
//! `.designcraft` master (`Magic Studio Storyboard.designcraft`), with integrated `PhotoCraft`,
//! `VectorCraft`, and `EffectCraft` asset/pose pipelines.
//!
//! Storyboard Rules:
//! 1. The `.designcraft` master IS the rules (`doc["sections"]` defines section names and
//!    `item["locked"]` defines editable vs chrome items — zero hardcoded section tables or extra
//!    permission checks).
//! 2. Storyboard is a printable reference document (`612 × 792 pt` US Letter), decoupled from video.
//! 3. Storyboard supplies the canonical style & image references for all new media creation.
//! 4. `Assets` shows each asset's flat project image, and that project image is what Storyboard
//!    stores together with metadata linking back to the underlying files.

pub mod crud;
pub mod workflow;

pub use crud::{
    cmd_storyboard_add, cmd_storyboard_export, cmd_storyboard_locations, cmd_storyboard_new, cmd_storyboard_open, cmd_storyboard_pages, cmd_storyboard_remove,
    cmd_storyboard_section, cmd_storyboard_set, cmd_storyboard_thumbnail,
};
pub use workflow::{cmd_storyboard_bulk_generate, cmd_storyboard_check, cmd_storyboard_grade, cmd_storyboard_wizard};

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use flate2::read::DeflateDecoder;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{EngineError, Result, Session};

const MASTER_DESIGNCRAFT: &[u8] = include_bytes!("../../../templates/storyboard.designcraft");
pub(crate) const PDF_TEMPLATE: &[u8] = include_bytes!("../../../templates/storyboard-template.pdf");

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ImageLinkMeta {
    pub image: String,
    pub linked_file: Option<String>,
    pub asset_id: Option<String>,
    pub craft_app: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outline: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_sec: Option<f64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FieldState {
    pub text: BTreeMap<String, String>,
    pub images: BTreeMap<String, ImageLinkMeta>,
}

pub(crate) fn bad(msg: &str) -> EngineError {
    EngineError::InvalidParams(msg.into())
}

pub(crate) fn fail(e: impl std::fmt::Display) -> EngineError {
    EngineError::CommandFailed("storyboard".into(), e.to_string())
}

pub(crate) fn doc_path(s: &Session, p: &Value) -> Result<PathBuf> {
    if let Some(raw) = p.get("path").and_then(Value::as_str).filter(|r| !r.trim().is_empty()) {
        let mut path = crate::workspace::resolve_workspace_path(s, raw);
        if path.extension().is_none_or(|e| !e.eq_ignore_ascii_case("pdf")) {
            path.set_extension("pdf");
        }
        return Ok(path);
    }
    let project = s.project_path.as_ref().ok_or_else(|| bad("give `path`, or open a .magic project"))?;
    Ok(project.with_file_name("storyboard.pdf"))
}

pub(crate) fn fields_state_path(pdf_path: &Path) -> PathBuf {
    pdf_path.with_file_name("storyboard").join("fields.json")
}

pub(crate) fn load_field_state(pdf_path: &Path) -> FieldState {
    std::fs::read_to_string(fields_state_path(pdf_path)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

pub(crate) fn save_field_state(pdf_path: &Path, state: &FieldState) -> Result<()> {
    let path = fields_state_path(pdf_path);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    craft_atomic::atomic_write(&path, &serde_json::to_vec_pretty(state).map_err(fail)?)?;
    Ok(())
}

pub(crate) fn field_key(page: usize, label: &str) -> String {
    format!("p{page:02}_{label}")
}

pub(crate) fn slot_cli_command(page: usize, label: &str, kind: &str) -> String {
    if kind == "text" {
        format!("magic-cli storyboard set page={page} label={label} text=\"...\"")
    } else {
        format!("magic-cli storyboard set page={page} label={label} image=\"path/to/artwork.pcraft\"")
    }
}

pub(crate) fn human_slot_title(label: &str) -> String {
    label
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn slot_is_filled(kind: &str, text: Option<&str>, image: Option<&str>) -> bool {
    match kind {
        "text" => text.is_some_and(|raw| {
            let t = raw.trim();
            !t.is_empty() && !(t.contains('[') && t.contains(']')) && !t.contains("____")
        }),
        "image" => image.is_some_and(|img| {
            let p = img.trim();
            !p.is_empty() && Path::new(p).is_file()
        }),
        _ => false,
    }
}

fn u16_at(bytes: &[u8], at: usize) -> usize {
    u16::from_le_bytes([bytes[at], bytes[at + 1]]) as usize
}

fn u32_at(bytes: &[u8], at: usize) -> usize {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]) as usize
}

/// Read `document.json` from the embedded `.designcraft` master once in memory.
pub(crate) fn master_template_doc() -> &'static Value {
    static DOC: OnceLock<Value> = OnceLock::new();
    DOC.get_or_init(|| {
        let bytes = MASTER_DESIGNCRAFT;
        let eocd = (0..=bytes.len().saturating_sub(22)).rev().find(|&i| &bytes[i..i + 4] == b"PK\x05\x06").unwrap_or(0);
        let mut pos = u32_at(bytes, eocd + 16);
        for _ in 0..u16_at(bytes, eocd + 10) {
            if pos + 46 > bytes.len() || &bytes[pos..pos + 4] != b"PK\x01\x02" {
                break;
            }
            let (name_len, extra_len, comment_len) = (u16_at(bytes, pos + 28), u16_at(bytes, pos + 30), u16_at(bytes, pos + 32));
            if std::str::from_utf8(&bytes[pos + 46..pos + 46 + name_len]).unwrap_or("") == "document.json" {
                let local = u32_at(bytes, pos + 42);
                let start = local + 30 + u16_at(bytes, local + 26) + u16_at(bytes, local + 28);
                let data = &bytes[start..start + u32_at(bytes, pos + 20)];
                let mut out = Vec::new();
                if u16_at(bytes, pos + 10) == 8 {
                    let _ = DeflateDecoder::new(data).read_to_end(&mut out);
                } else {
                    out.extend_from_slice(data);
                }
                return serde_json::from_slice(&out).unwrap_or_else(|_| json!({}));
            }
            pos += 46 + name_len + extra_len + comment_len;
        }
        json!({})
    })
}

/// Rule 1: Section names come strictly from `doc["sections"]` in the `.designcraft` master.
pub(crate) fn master_sections() -> Vec<String> {
    let doc = master_template_doc();
    doc["sections"].as_array().into_iter().flatten().filter_map(|s| s["marker"].as_str().map(str::trim).filter(|m| !m.is_empty()).map(str::to_owned)).collect()
}

fn item_bounds(item: &Value) -> Option<[f64; 4]> {
    if let Some(b) = item["bounds"].as_array().filter(|b| b.len() == 4)
        && let (Some(x0), Some(y0), Some(x1), Some(y1)) = (b[0].as_f64(), b[1].as_f64(), b[2].as_f64(), b[3].as_f64())
    {
        return Some([x0, y0, x1, y1]);
    }
    let xf = item.get("xf").and_then(Value::as_array);
    let (a, b, c, d, tx, ty) = match xf {
        Some(v) if v.len() == 6 => (
            v[0].as_f64().unwrap_or(1.0),
            v[1].as_f64().unwrap_or(0.0),
            v[2].as_f64().unwrap_or(0.0),
            v[3].as_f64().unwrap_or(1.0),
            v[4].as_f64().unwrap_or(0.0),
            v[5].as_f64().unwrap_or(0.0),
        ),
        _ => (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
    };
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for sp in item["path"]["subpaths"].as_array().into_iter().flatten() {
        for anchor in sp["anchors"].as_array().into_iter().flatten() {
            if let Some(p) = anchor["p"].as_array()
                && p.len() >= 2
                && let (Some(px), Some(py)) = (p[0].as_f64(), p[1].as_f64())
            {
                let (x, y) = (a * px + c * py + tx, b * px + d * py + ty);
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }
    (min_x.is_finite() && max_x >= min_x && max_y >= min_y && (max_x > min_x || max_y > min_y)).then_some([min_x, min_y, max_x, max_y])
}

fn section_for_doc_page(doc: &Value, page_idx: usize) -> String {
    let mut matched: Option<&str> = None;
    for sec in doc["sections"].as_array().into_iter().flatten() {
        let start = sec["start"].as_u64().unwrap_or(0) as usize;
        if page_idx >= start
            && let Some(marker) = sec["marker"].as_str().filter(|m| !m.trim().is_empty())
        {
            matched = Some(marker);
        }
    }
    matched.unwrap_or("Storyboard").to_owned()
}

#[derive(Clone)]
pub(crate) struct Slot {
    pub label: String,
    pub form_name: String,
    pub id: u64,
    pub kind: &'static str,
    pub text: Option<String>,
    pub image: Option<String>,
    pub linked_file: Option<String>,
    pub asset_id: Option<String>,
    pub craft_app: Option<String>,
    pub outline: Option<String>,
    pub time_sec: Option<f64>,
    pub bounds: [f64; 4],
    pub locked: bool,
    pub filled: bool,
    pub cli_command: String,
}

impl Slot {
    pub fn to_json(&self) -> Value {
        json!({
            "label": self.label,
            "form_field": self.form_name,
            "title": human_slot_title(&self.label),
            "id": self.id,
            "kind": self.kind,
            "text": self.text,
            "image": self.image,
            "linked_file": self.linked_file,
            "asset_id": self.asset_id,
            "craft_app": self.craft_app,
            "outline": self.outline,
            "time_sec": self.time_sec,
            "bounds": self.bounds,
            "locked": self.locked,
            "required": !self.locked,
            "filled": self.filled,
            "light": if self.filled { "green" } else { "red" },
            "cli_command": self.cli_command,
        })
    }
}

pub(crate) struct PageInfo {
    pub index: usize,
    pub spread_index: usize,
    pub page_in_spread: usize,
    pub spread_page_count: usize,
    pub section: String,
    pub bounds: [f64; 4],
    pub heading: Option<String>,
    pub slots: Vec<Slot>,
    pub layout_items: Vec<Value>,
}

impl PageInfo {
    pub fn title(&self) -> String {
        if let Some(h) = self.heading.as_deref().filter(|t| !t.trim().is_empty()) {
            return h.trim().to_owned();
        }
        format!("{} · Page {:02}", self.section, self.index + 1)
    }

    pub fn to_json(&self) -> Value {
        let slots: Vec<Value> = self.slots.iter().filter(|s| !s.locked).map(Slot::to_json).collect();
        json!({
            "page": self.index,
            "page_number": self.index + 1,
            "spread": self.spread_index,
            "page_in_spread": self.page_in_spread,
            "spread_page_count": self.spread_page_count,
            "section": self.section,
            "title": self.title(),
            "bounds": self.bounds,
            "slots": slots,
            "layout_items": self.layout_items,
        })
    }
}

/// Rule 4: Sync connected project assets (`s.project_assets`) into the Storyboard's `Assets`
/// inventory slots (`assets_thumb_N`, `assets_name_N`, `assets_details_N`) using each asset's
/// flat project image + linked-file metadata.
pub(crate) fn sync_assets_into_state(s: &Session, state: &mut FieldState) {
    let Some(project_dir) = s.project_path.as_deref().and_then(Path::parent) else {
        return;
    };
    let doc = master_template_doc();
    let mut assets_page_idx: Option<usize> = None;
    let mut doc_pg = 0usize;
    for spread in doc["spreads"].as_array().into_iter().flatten() {
        let sp_pages = spread["pages"].as_array().map_or(1, Vec::len).max(1);
        if spread["items"].as_array().into_iter().flatten().any(|it| it["label"].as_str() == Some("assets_thumb_1")) {
            assets_page_idx = Some(doc_pg);
            break;
        }
        doc_pg += sp_pages;
    }
    let Some(pg_idx) = assets_page_idx else { return };

    for (i, asset) in s.project_assets.iter().take(6).enumerate() {
        let n = i + 1;
        let Some(ref raw_path) = asset.path else { continue };
        let (proj_img, linked_file, craft_app) = crate::cmd::project::assets::resolve_asset_project_image(Some(project_dir), raw_path);
        let outline = craft_util::media::craft_outline_summary(raw_path);
        let thumb_key = field_key(pg_idx, &format!("assets_thumb_{n}"));
        state.images.entry(thumb_key).or_insert_with(|| ImageLinkMeta {
            image: proj_img.to_string_lossy().into_owned(),
            linked_file: linked_file.clone().or_else(|| Some(raw_path.to_string_lossy().into_owned())),
            asset_id: Some(asset.id.clone()),
            craft_app: craft_app.clone(),
            outline: outline.clone(),
            time_sec: None,
        });
        let name_key = field_key(pg_idx, &format!("assets_name_{n}"));
        state.text.entry(name_key).or_insert_with(|| asset.name.clone());
        let details_key = field_key(pg_idx, &format!("assets_details_{n}"));
        state.text.entry(details_key).or_insert_with(|| {
            let rel = raw_path.strip_prefix(project_dir).unwrap_or(raw_path).to_string_lossy();
            let app_tag = craft_app.as_deref().unwrap_or(asset.kind.as_str());
            match outline.as_deref() {
                Some(info) => format!("{app_tag} ({info}) · {rel}"),
                None => format!("{app_tag} · {rel}"),
            }
        });
    }
}

/// Rule 1: Parse Storyboard pages strictly from the `.designcraft` master (`doc["sections"]` + `item["locked"]`).
pub(crate) fn parse_storyboard_pages(s: Option<&Session>, pdf_path: &Path) -> Vec<PageInfo> {
    let doc = master_template_doc();
    let mut state = load_field_state(pdf_path);
    if let Some(sess) = s {
        sync_assets_into_state(sess, &mut state);
    }
    let default_w = doc["settings"]["pageWidth"].as_f64().unwrap_or(612.0);
    let default_h = doc["settings"]["pageHeight"].as_f64().unwrap_or(792.0);
    let mut pages = Vec::new();
    let mut doc_page_idx = 0usize;

    for (spread_idx, spread) in doc["spreads"].as_array().into_iter().flatten().enumerate() {
        let raw_pages = spread["pages"].as_array().cloned().unwrap_or_default();
        let spread_pages: Vec<(f64, f64, f64, f64)> = if raw_pages.is_empty() {
            vec![(0.0, 0.0, default_w, default_h)]
        } else {
            raw_pages
                .iter()
                .map(|pg| {
                    (
                        pg["x"].as_f64().unwrap_or(0.0),
                        pg["y"].as_f64().unwrap_or(0.0),
                        pg["width"].as_f64().unwrap_or(default_w),
                        pg["height"].as_f64().unwrap_or(default_h),
                    )
                })
                .collect()
        };
        let sp_count = spread_pages.len();
        let mut per_page_slots: Vec<Vec<Slot>> = vec![Vec::new(); sp_count];
        let mut per_page_layout: Vec<Vec<Value>> = vec![Vec::new(); sp_count];
        let mut per_page_headings: Vec<Option<String>> = vec![None; sp_count];

        for item in spread["items"].as_array().into_iter().flatten() {
            let Some(id) = item["id"].as_u64() else { continue };
            if item["hidden"].as_bool().unwrap_or(false) {
                continue;
            }
            let Some(wb) = item_bounds(item) else { continue };
            let cx = (wb[0] + wb[2]) * 0.5;
            let page_in_spread = spread_pages.iter().position(|(px, _, pw, _)| cx >= *px - 2.0 && cx <= *px + *pw + 2.0).unwrap_or(0);
            let (px, py, _, _) = spread_pages[page_in_spread];
            let local_bounds = [wb[0] - px, wb[1] - py, wb[2] - px, wb[3] - py];
            let target_page_idx = doc_page_idx + page_in_spread;

            let ctype = item["content"]["type"].as_str().unwrap_or("unassigned");
            let label_opt = item["label"].as_str().filter(|l| !l.is_empty());
            let locked = item["locked"].as_bool().unwrap_or(false);

            let story_id = item["content"]["story"].as_u64();
            let story_obj = story_id.and_then(|sid| doc["stories"].get(sid.to_string()));
            let fkey = label_opt.map(|lbl| field_key(target_page_idx, lbl));
            let text = fkey.as_ref().and_then(|k| state.text.get(k).cloned()).or_else(|| story_obj.and_then(|st| st["text"].as_str()).map(str::to_owned));
            let img_meta = fkey.as_ref().and_then(|k| state.images.get(k)).cloned();
            let image = img_meta.as_ref().map(|m| m.image.clone());

            if locked
                && ctype == "text"
                && label_opt.is_some_and(|l| l.ends_with("_heading") || l == "section_title" || l == "cover_badge")
                && per_page_headings[page_in_spread].is_none()
            {
                per_page_headings[page_in_spread] = text.clone();
            }

            let over = story_obj.and_then(|st| st["chars"].as_array()?.first()).and_then(|ch| ch.get("over"));
            let align = story_obj.and_then(|st| st["paras"].as_array()?.first()).and_then(|pa| pa["para"]["align"].as_str()).unwrap_or("left");
            let item_kind = if ctype == "text" {
                "text"
            } else if !locked && label_opt.is_some() {
                "image"
            } else {
                "shape"
            };
            let hide_hint = label_opt.and_then(|l| l.strip_suffix("_hint")).is_some_and(|base| state.images.contains_key(&field_key(target_page_idx, base)));
            if !hide_hint {
                per_page_layout[page_in_spread].push(json!({
                    "id": id,
                    "label": label_opt,
                    "kind": item_kind,
                    "locked": locked,
                    "bounds": local_bounds,
                    "fill": item["fill"]["swatch"].as_str().unwrap_or("[None]"),
                    "stroke": item["stroke"]["swatch"].as_str().unwrap_or("[None]"),
                    "stroke_weight": item["stroke"]["weight"].as_f64().unwrap_or(0.0),
                    "text": text,
                    "font_size": over.and_then(|o| o["size"].as_f64()).unwrap_or(9.5),
                    "font_style": over.and_then(|o| o["fontStyle"].as_str()).unwrap_or("Regular"),
                    "text_fill": over.and_then(|o| o["fill"].as_str()).unwrap_or("Ink"),
                    "align": align,
                    "image": image,
                    "linked_file": img_meta.as_ref().and_then(|m| m.linked_file.as_deref()),
                    "asset_id": img_meta.as_ref().and_then(|m| m.asset_id.as_deref()),
                    "craft_app": img_meta.as_ref().and_then(|m| m.craft_app.as_deref()),
                    "outline": img_meta.as_ref().and_then(|m| m.outline.as_deref()),
                    "time_sec": img_meta.as_ref().and_then(|m| m.time_sec),
                }));
            }

            let Some(label) = label_opt else { continue };
            let kind = if ctype == "text" { "text" } else { "image" };
            let filled = slot_is_filled(kind, text.as_deref(), image.as_deref());
            per_page_slots[page_in_spread].push(Slot {
                label: label.to_string(),
                form_name: field_key(target_page_idx, label),
                id,
                kind,
                text,
                image,
                linked_file: img_meta.as_ref().and_then(|m| m.linked_file.clone()),
                asset_id: img_meta.as_ref().and_then(|m| m.asset_id.clone()),
                craft_app: img_meta.as_ref().and_then(|m| m.craft_app.clone()),
                outline: img_meta.as_ref().and_then(|m| m.outline.clone()),
                time_sec: img_meta.as_ref().and_then(|m| m.time_sec),
                bounds: local_bounds,
                locked,
                filled,
                cli_command: slot_cli_command(target_page_idx, label, kind),
            });
        }

        for (p_in_sp, (((_, _, pw, ph), slots), (layout_items, heading))) in
            spread_pages.into_iter().zip(per_page_slots).zip(per_page_layout.into_iter().zip(per_page_headings)).enumerate()
        {
            let index = doc_page_idx + p_in_sp;
            pages.push(PageInfo {
                index,
                spread_index: spread_idx,
                page_in_spread: p_in_sp,
                spread_page_count: sp_count,
                section: section_for_doc_page(doc, index),
                bounds: [0.0, 0.0, pw, ph],
                heading,
                slots,
                layout_items,
            });
        }
        doc_page_idx += sp_count;
    }
    pages
}

/// Rule 3: Gather Storyboard visual design prompts and placed reference images, optionally
/// prioritizing `Characters` prompts and character images for character sheet/rig generation.
pub(crate) fn storyboard_media_references_for(s: &Session, prefer_character: bool) -> (Option<String>, Vec<PathBuf>) {
    let Ok(pdf_path) = doc_path(s, &json!({})) else {
        return (None, Vec::new());
    };
    let pages = parse_storyboard_pages(Some(s), &pdf_path);
    let keys: &[&str] = if prefer_character {
        &["char_exact_prompt", "cover_overall_design", "world_location_prompt"]
    } else {
        &["cover_overall_design", "world_location_prompt", "char_exact_prompt"]
    };
    let mut style_parts = Vec::new();
    for key in keys {
        if let Some(txt) =
            pages.iter().find_map(|pg| pg.slots.iter().find(|sl| sl.label == *key && sl.filled)?.text.as_deref()).map(str::trim).filter(|t| !t.is_empty())
        {
            style_parts.push(txt.to_owned());
        }
    }
    let style_context = (!style_parts.is_empty()).then(|| style_parts.join(" | "));

    let mut refs = Vec::new();
    if prefer_character {
        for pg in &pages {
            for sl in pg.slots.iter().filter(|sl| sl.kind == "image" && sl.filled && sl.label.starts_with("char_")) {
                if let Some(p) = sl.image.as_deref().map(PathBuf::from).filter(|p| p.is_file())
                    && !refs.contains(&p)
                    && refs.len() < 5
                {
                    refs.push(p);
                }
            }
        }
    }
    for pg in &pages {
        for sl in pg.slots.iter().filter(|sl| sl.kind == "image" && sl.filled) {
            if let Some(p) = sl.image.as_deref().map(PathBuf::from).filter(|p| p.is_file())
                && !refs.contains(&p)
                && refs.len() < 5
            {
                refs.push(p);
            }
        }
    }
    (style_context, refs)
}

pub(crate) fn storyboard_media_references(s: &Session) -> (Option<String>, Vec<PathBuf>) {
    storyboard_media_references_for(s, false)
}
