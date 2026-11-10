//! Storyboard: Single-Responsibility `.designcraft` document adapter over `DesignCraft`
//! (`designcraft --control` and `designcraft-cli` via [`crate::CraftHub`]) and `craft-libs`.
//!
//! Storyboard owns CRUD operations (`new`, `pages`, `set`, `add`, `remove`, `section`, `locations`),
//! drag-and-drop / CLI image overlaying (saving a flat image inside `.designcraft` plus an optional
//! reference to the underlying Craft project in `storyboard/references.json`), traffic-light section
//! readiness (`check`), setup wizard (`wizard`), and title-page-driven bulk image generation
//! (`bulk_generate`). Every spread embeds read-only fine-print `magic-cli` instructions (`*_cli` frames
//! locked on the `"CLI Fineprint (Read-Only)"` layer).

pub mod crud;
pub mod workflow;

pub use crud::{
    cmd_storyboard_add, cmd_storyboard_export, cmd_storyboard_locations, cmd_storyboard_new, cmd_storyboard_open, cmd_storyboard_pages, cmd_storyboard_remove,
    cmd_storyboard_section, cmd_storyboard_set, cmd_storyboard_thumbnail,
};
pub use workflow::{cmd_storyboard_bulk_generate, cmd_storyboard_check, cmd_storyboard_wizard};

use std::collections::{BTreeMap, HashMap};
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::link_watch::FileStamp;
use crate::{CraftHub, CraftTarget, EngineError, Result, Session};

pub(crate) const TEMPLATE: &[u8] = include_bytes!("../../../templates/storyboard.designcraft");
pub(crate) const MAX_REFS: usize = 12;
pub const ARCHETYPES: &[(&str, &str)] =
    &[("cover", "cover_title"), ("world", "world_heading"), ("char", "char_heading"), ("motion", "motion_heading"), ("scene", "scene_id")];
pub const MAIN_TITLE_ARCHETYPES: &[&str] = &["cover", "world", "char", "motion"];
const TITLE_LABELS: &[&str] = &["cover_title", "world_heading", "char_heading", "motion_heading", "scene_title", "scene_id", "title"];
const EXPORT_ONLY: &[&str] = &["document.inspect", "file.exportPdf", "file.exportPptx"];

pub(crate) type Labels = HashMap<String, u64>;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SlotReference {
    pub page: usize,
    pub slot: String,
    pub flat_image: String,
    pub source_project: Option<String>,
    pub craft_app: Option<String>,
    pub updated_unix: u64,
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
        if path.extension().is_none_or(|e| !e.eq_ignore_ascii_case("designcraft")) {
            path.as_mut_os_string().push(".designcraft");
        }
        return Ok(path);
    }
    let project = s.project_path.as_ref().ok_or_else(|| bad("give `path`, or open a project"))?;
    Ok(project.with_file_name("storyboard.designcraft"))
}

pub(crate) fn references_path(sb_path: &Path) -> PathBuf {
    sb_path.with_file_name("storyboard").join("references.json")
}

pub(crate) fn load_slot_references(sb_path: &Path) -> BTreeMap<String, SlotReference> {
    let path = references_path(sb_path);
    std::fs::read_to_string(path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

pub(crate) fn save_slot_references(sb_path: &Path, refs: &BTreeMap<String, SlotReference>) -> Result<()> {
    let path = references_path(sb_path);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let bytes = serde_json::to_vec_pretty(refs).map_err(fail)?;
    craft_atomic::atomic_write(&path, &bytes)?;
    Ok(())
}

pub(crate) fn slot_cli_command(page: usize, label: &str, kind: &str) -> String {
    if label.starts_with("ref_") && label[4..].chars().all(|c| c.is_ascii_digit()) {
        format!("magic-cli storyboard set page={page} refs='[\"ref1.png\"]'")
    } else if kind == "text" {
        format!("magic-cli storyboard set page={page} label={label} text=\"...\"")
    } else {
        format!("magic-cli storyboard set page={page} label={label} image=\"path/to/art.png\"")
    }
}

pub(crate) fn page_cli_command(page: usize, archetype: &str) -> String {
    match archetype {
        "cover" => format!(
            "READONLY CLI · magic-cli storyboard wizard title=\"...\" style=\"...\" · magic-cli storyboard pages page={page} · magic-cli storyboard check"
        ),
        "world" => format!(
            "READONLY CLI · magic-cli storyboard locations page={page} fields='{{\"world_heading\":\"...\"}}' · magic-cli storyboard check"
        ),
        "char" => {
            "READONLY CLI · magic-cli exec character.cut character=<id> remove_background=true rig=true · magic-cli exec character.puppet.rig_face character=<id>"
                .into()
        }
        "motion" => {
            "READONLY CLI · magic-cli exec motion.save word_id=w-0001 image=\"cue.png\" · magic-cli exec motion.publish render=true".into()
        }
        _ => format!(
            "READONLY CLI · magic-cli storyboard pages page={page} · magic-cli storyboard bulk_generate · magic-cli storyboard add archetype={archetype}"
        ),
    }
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
    (min_x.is_finite() && max_x > min_x && max_y > min_y).then_some([min_x, min_y, max_x, max_y])
}

fn u16_at(bytes: &[u8], at: usize) -> usize {
    u16::from_le_bytes([bytes[at], bytes[at + 1]]) as usize
}

fn u32_at(bytes: &[u8], at: usize) -> usize {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]) as usize
}

/// Read `document.json` from a `.designcraft` OpenDocument-style ZIP archive (preferring live
/// `DesignCraft --control` `document.inspect` when an active DesignCraft window holds the file).
pub(crate) fn read_designcraft_doc(path: &Path) -> Result<Value> {
    if !path.is_file() {
        return Err(bad(&format!("{} does not exist", path.display())));
    }
    if let Some(Ok(live)) = CraftHub::try_live(CraftTarget::Design, path, "document.inspect", &json!({}), false)
        && live["spreads"].is_array()
    {
        return Ok(live);
    }
    let bytes = std::fs::read(path)?;
    if bytes.len() < 22 || &bytes[0..4] != b"PK\x03\x04" {
        return Err(fail("not a valid .designcraft archive"));
    }
    let eocd = (0..=bytes.len() - 22).rev().find(|&i| &bytes[i..i + 4] == b"PK\x05\x06").ok_or_else(|| fail("corrupt .designcraft central directory"))?;
    let mut pos = u32_at(&bytes, eocd + 16);
    for _ in 0..u16_at(&bytes, eocd + 10) {
        if pos + 46 > bytes.len() || &bytes[pos..pos + 4] != b"PK\x01\x02" {
            break;
        }
        let (name_len, extra_len, comment_len) = (u16_at(&bytes, pos + 28), u16_at(&bytes, pos + 30), u16_at(&bytes, pos + 32));
        if std::str::from_utf8(&bytes[pos + 46..pos + 46 + name_len]).unwrap_or("") == "document.json" {
            let local = u32_at(&bytes, pos + 42);
            let start = local + 30 + u16_at(&bytes, local + 26) + u16_at(&bytes, local + 28);
            let data = &bytes[start..start + u32_at(&bytes, pos + 20)];
            let raw = match u16_at(&bytes, pos + 10) {
                0 => data.to_vec(),
                8 => {
                    let mut out = Vec::new();
                    flate2::read::DeflateDecoder::new(data).read_to_end(&mut out).map_err(fail)?;
                    out
                }
                other => return Err(fail(format!("unsupported zip compression method {other}"))),
            };
            return serde_json::from_slice(&raw).map_err(fail);
        }
        pos += 46 + name_len + extra_len + comment_len;
    }
    Err(fail("document.json not found in .designcraft archive"))
}

pub(crate) fn parent_labels(doc: &Value) -> HashMap<u64, String> {
    doc["parents"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|p| {
            let prefix = p["parent"]["prefix"].as_str().or_else(|| p["prefix"].as_str())?;
            let name = p["parent"]["name"].as_str().or_else(|| p["name"].as_str())?;
            let label = if prefix.is_empty() { name.to_string() } else { format!("{prefix}-{name}") };
            Some((p["id"].as_u64()?, label))
        })
        .collect()
}

pub(crate) struct Slot {
    pub label: String,
    pub id: u64,
    pub kind: &'static str,
    pub text: Option<String>,
    pub image: Option<String>,
    pub source_project: Option<String>,
    pub craft_app: Option<String>,
    pub bounds: Option<[f64; 4]>,
    pub hidden: bool,
    pub locked: bool,
    pub readonly: bool,
    pub cli_command: String,
}

impl Slot {
    pub fn is_cli_helper(&self) -> bool {
        self.label.ends_with("_cli")
    }

    pub fn to_json(&self) -> Value {
        json!({
            "label": self.label,
            "id": self.id,
            "kind": self.kind,
            "text": self.text,
            "image": self.image,
            "source_project": self.source_project,
            "craft_app": self.craft_app,
            "bounds": self.bounds,
            "hidden": self.hidden,
            "locked": self.locked,
            "readonly": self.readonly,
            "cli_command": self.cli_command,
        })
    }
}

pub(crate) struct PageInfo {
    pub index: usize,
    pub archetype: String,
    pub section: Option<String>,
    pub bounds: [f64; 4],
    pub slots: Vec<Slot>,
}

impl PageInfo {
    pub fn title(&self) -> String {
        TITLE_LABELS
            .iter()
            .find_map(|k| self.slots.iter().find(|s| s.label == *k)?.text.as_deref().map(str::trim).filter(|t| !t.is_empty()))
            .map_or_else(|| format!("{} {}", self.archetype, self.index + 1), str::to_string)
    }

    pub fn is_main_title_page(&self) -> bool {
        MAIN_TITLE_ARCHETYPES.contains(&self.archetype.as_str())
    }

    pub fn labels(&self) -> Labels {
        self.slots.iter().map(|s| (s.label.clone(), s.id)).collect()
    }

    pub fn to_json(&self) -> Value {
        let slots: Vec<Value> = self.slots.iter().filter(|s| !s.is_cli_helper()).map(Slot::to_json).collect();
        let cli_instructions: Vec<Value> = self
            .slots
            .iter()
            .filter(|s| s.is_cli_helper())
            .map(|s| {
                json!({
                    "label": s.label,
                    "id": s.id,
                    "text": s.text,
                    "readonly": true,
                    "locked": true,
                    "style": "CLI Fineprint",
                })
            })
            .collect();
        let left_no = self.index * 2 + 1;
        let right_no = left_no + 1;
        json!({
            "page": self.index,
            "spread_pages": [format!("{left_no:02}"), format!("{right_no:02}")],
            "archetype": self.archetype,
            "is_title_section": self.is_main_title_page(),
            "section": self.section,
            "title": self.title(),
            "bounds": self.bounds,
            "cli_command": page_cli_command(self.index, &self.archetype),
            "cli_instructions": cli_instructions,
            "slots": slots,
        })
    }
}

pub(crate) fn parse_pages_and_stories(path: &Path, doc: &Value) -> (Vec<PageInfo>, HashMap<u64, u64>) {
    let parents = parent_labels(doc);
    let refs = load_slot_references(path);
    let default_w = doc["settings"]["pageWidth"].as_f64().unwrap_or(1600.0);
    let default_h = doc["settings"]["pageHeight"].as_f64().unwrap_or(1000.0);
    let mut stories = HashMap::new();
    let mut pages = Vec::new();
    for (index, spread) in doc["spreads"].as_array().into_iter().flatten().enumerate() {
        let mut slots = Vec::new();
        for item in spread["items"].as_array().into_iter().flatten() {
            let Some(id) = item["id"].as_u64() else { continue };
            if let Some(story_id) = item["content"]["story"].as_u64().or_else(|| item["story"].as_u64()) {
                stories.insert(id, story_id);
            }
            let Some(label) = item["label"].as_str().filter(|l| !l.is_empty()) else { continue };
            let ctype = item["content"]["type"].as_str().or_else(|| item["kind"].as_str()).unwrap_or("image");
            let kind = if ctype == "text" { "text" } else { "image" };
            let text = if kind == "text" {
                item["text"]
                    .as_str()
                    .map(str::to_string)
                    .or_else(|| item["content"]["story"].as_u64().and_then(|sid| doc["stories"][sid.to_string()]["text"].as_str()).map(str::to_string))
            } else {
                None
            };
            let image = if kind == "image" {
                item["content"]["asset"]
                    .as_u64()
                    .and_then(|aid| doc["assets"][aid.to_string()]["link"].as_str())
                    .or_else(|| item["link"].as_str())
                    .map(|raw| craft_docpath::resolve_absolute(raw, Some(path)).map_or_else(|_| raw.to_string(), |p| p.to_string_lossy().into_owned()))
            } else {
                None
            };
            let ref_entry = refs.get(&format!("{index}:{label}"));
            let is_cli = label.ends_with("_cli");
            slots.push(Slot {
                label: label.to_string(),
                id,
                kind,
                text,
                image,
                source_project: ref_entry.and_then(|r| r.source_project.clone()),
                craft_app: ref_entry.and_then(|r| r.craft_app.clone()),
                bounds: item_bounds(item),
                hidden: item["hidden"].as_bool().unwrap_or(false),
                locked: item["locked"].as_bool().unwrap_or(is_cli),
                readonly: item["readonly"].as_bool().unwrap_or(is_cli),
                cli_command: slot_cli_command(index, label, kind),
            });
        }
        let archetype = ARCHETYPES.iter().find(|(_, marker)| slots.iter().any(|s| s.label == *marker)).map_or("scene", |(name, _)| *name).to_string();
        let section = spread["pages"][0]["parent"].as_u64().and_then(|pid| parents.get(&pid).cloned());
        pages.push(PageInfo { index, archetype, section, bounds: [0.0, 0.0, default_w, default_h], slots });
    }
    (pages, stories)
}

pub(crate) fn run_design_steps(path: &Path, steps: &[(&str, Value)]) -> Result<Vec<Value>> {
    let mutates = steps.iter().any(|(id, _)| !EXPORT_ONLY.contains(id));
    if mutates {
        crate::link_watch::save_guard(path, FileStamp::of(path))?;
    }
    CraftHub::run_batch(CraftTarget::Design, Some(path), mutates.then_some(path), steps)
}
