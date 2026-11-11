//! Storyboard CRUD (`new`, `pages`, `set`, `add`, `remove`, `section`, `locations`),
//! Craft-project-to-flat-PNG overlaying, thumbnail rendering, PDF/PPTX export, and live `--control` launch.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::{
    Labels, MAX_REFS, PageInfo, SlotReference, TEMPLATE, bad, doc_path, fail, load_slot_references, page_cli_command, parent_labels, parse_pages_and_stories,
    read_designcraft_doc, run_design_steps, save_slot_references, slot_cli_command,
};
use crate::craft_hub::cli::render_design_pages;
use crate::time::Tick;
use crate::{CraftHub, CraftTarget, Result, Session};

fn reject_readonly_cli_slot(label: &str) -> Result<()> {
    if label.ends_with("_cli") {
        return Err(bad(&format!("slot `{label}` is a read-only `magic-cli` fine-print instruction frame and cannot be modified")));
    }
    Ok(())
}

pub(crate) fn page_index(pages: &[PageInfo], p: &Value) -> Result<usize> {
    if let Some(n) = p.get("page").and_then(Value::as_u64) {
        let idx = n as usize;
        return if idx < pages.len() { Ok(idx) } else { Err(bad(&format!("no page {n}; storyboard has {} pages", pages.len()))) };
    }
    if let Some(label) = p.get("label").and_then(Value::as_str) {
        return pages.iter().find(|pg| pg.slots.iter().any(|s| s.label == label)).map(|pg| pg.index).ok_or_else(|| bad(&format!("no slot `{label}`")));
    }
    if let Some(n) = p.get("scene").and_then(Value::as_u64) {
        return pages
            .iter()
            .filter(|pg| pg.archetype == "scene")
            .nth((n as usize).wrapping_sub(1))
            .map(|pg| pg.index)
            .ok_or_else(|| bad(&format!("no scene {n}")));
    }
    Err(bad("give `page`, `label`, or `scene`"))
}

fn scene_number_for(pages: &[PageInfo], index: usize) -> Option<usize> {
    (pages.get(index)?.archetype == "scene").then(|| pages.iter().filter(|pg| pg.archetype == "scene").position(|pg| pg.index == index).map(|n| n + 1))?
}

fn find_page_preview_png(sb_path: &Path, page_idx: usize) -> Option<PathBuf> {
    let sb_dir = sb_path.with_file_name("storyboard");
    let preview_dir = sb_dir.join(".preview");
    for candidate in [
        preview_dir.join(format!("page-{page_idx}-100.png")),
        preview_dir.join(format!("page-{page_idx}-75.png")),
        preview_dir.join(format!("page-{page_idx}-50.png")),
        preview_dir.join(format!("doc-{page_idx}-100.png")),
        preview_dir.join(format!("doc-{page_idx}-75.png")),
        preview_dir.join(format!("doc-{page_idx}-50.png")),
        sb_dir.join("spreads").join(format!("page_{page_idx:04}.png")),
        sb_dir.join("rail").join(format!("page_{page_idx:04}.png")),
    ] {
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    let mut rail_dirs: Vec<PathBuf> =
        std::fs::read_dir(&preview_dir).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    rail_dirs.sort();
    for rdir in rail_dirs {
        let mut pngs: Vec<PathBuf> = std::fs::read_dir(&rdir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "png"))
            .collect();
        pngs.sort();
        if let Some(found) = pngs.get(page_idx) {
            return Some(found.clone());
        }
    }
    let renders_dir = sb_dir.join("renders");
    let prefix = format!("p{page_idx:02}-");
    let mut slot_pngs: Vec<PathBuf> = std::fs::read_dir(&renders_dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with(&prefix) && n.ends_with(".png")))
        .collect();
    slot_pngs.sort();
    slot_pngs.into_iter().next()
}

/// If `input` is a Craft project (`.ecproj`, `.pcraft`, `.psd`, `.vectorcraft`, `.anicraft`, `.fcproj`),
/// render a flat PNG snapshot into `storyboard/renders/` via [`CraftHub`] and return
/// `(flat_png_path, Some(source_project), Some(craft_app))`. Otherwise keep `input` as the flat image
/// and attach any explicit or adjacent Craft project reference.
fn resolve_flat_image_and_ref(
    sb_path: &Path,
    page: usize,
    slot: &str,
    input: &Path,
    explicit_ref: Option<&Path>,
) -> Result<(PathBuf, Option<String>, Option<String>)> {
    let ext = input.extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase();
    let renders_dir = sb_path.with_file_name("storyboard").join("renders");
    let flat_out = renders_dir.join(format!("p{page:02}-{slot}.png"));

    match ext.as_str() {
        "ecproj" => {
            std::fs::create_dir_all(&renders_dir)?;
            CraftHub::render_effect_frame(input, None, 0.5, 1920, &flat_out)?;
            return Ok((flat_out, Some(input.to_string_lossy().into_owned()), Some("effectcraft".into())));
        }
        "fcproj" => {
            std::fs::create_dir_all(&renders_dir)?;
            CraftHub::render_film_frame(input, Tick::ZERO, 1.0, &flat_out)?;
            return Ok((flat_out, Some(input.to_string_lossy().into_owned()), Some("filmcraft".into())));
        }
        "vectorcraft" | "svg" | "ai" => {
            std::fs::create_dir_all(&renders_dir)?;
            if craft_util::media::extract_craft_preview(input, &flat_out).unwrap_or(false) && flat_out.is_file() {
                return Ok((flat_out, Some(input.to_string_lossy().into_owned()), Some("vectorcraft".into())));
            }
            let steps = [("export", json!({"path": flat_out, "format": "png", "scale": 2.0}))];
            if CraftHub::run_batch(CraftTarget::Vector, Some(input), None, &steps).is_ok() && flat_out.is_file() {
                return Ok((flat_out, Some(input.to_string_lossy().into_owned()), Some("vectorcraft".into())));
            }
        }
        "pcraft" | "psd" => {
            std::fs::create_dir_all(&renders_dir)?;
            if craft_util::media::extract_craft_preview(input, &flat_out).unwrap_or(false) && flat_out.is_file() {
                return Ok((flat_out, Some(input.to_string_lossy().into_owned()), Some("photocraft".into())));
            }
            let steps = [("file.export", json!({"path": flat_out, "format": "png"}))];
            if CraftHub::run_batch(CraftTarget::Photo, Some(input), None, &steps).is_ok() && flat_out.is_file() {
                return Ok((flat_out, Some(input.to_string_lossy().into_owned()), Some("photocraft".into())));
            }
        }
        "anicraft" => {
            std::fs::create_dir_all(&renders_dir)?;
            if CraftHub::anicraft_render_frame(input, 0, &flat_out).is_ok() && flat_out.is_file() {
                return Ok((flat_out, Some(input.to_string_lossy().into_owned()), Some("anicraft".into())));
            }
        }
        _ => {}
    }

    let source_proj = explicit_ref.map(|p| p.to_string_lossy().into_owned()).or_else(|| infer_adjacent_craft_project(input));
    let app = source_proj.as_deref().and_then(craft_app_for_path);
    Ok((input.to_path_buf(), source_proj, app))
}

fn craft_app_for_path(path: &str) -> Option<String> {
    let ext = Path::new(path).extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "ecproj" => Some("effectcraft".into()),
        "pcraft" | "psd" => Some("photocraft".into()),
        "vectorcraft" | "svg" => Some("vectorcraft".into()),
        "anicraft" => Some("anicraft".into()),
        "fcproj" => Some("filmcraft".into()),
        "designcraft" => Some("designcraft".into()),
        _ => None,
    }
}

fn infer_adjacent_craft_project(flat_img: &Path) -> Option<String> {
    let dir = flat_img.parent()?;
    for ext in ["ecproj", "pcraft", "psd", "vectorcraft", "anicraft"] {
        let candidate = flat_img.with_extension(ext);
        if candidate.is_file() {
            return Some(candidate.to_string_lossy().into_owned());
        }
    }
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()).is_some_and(|x| matches!(x, "ecproj" | "pcraft" | "vectorcraft" | "anicraft")))
        .collect();
    found.sort();
    found.first().map(|p| p.to_string_lossy().into_owned())
}

fn now_unix() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

pub(crate) struct PageTarget<'a> {
    pub sb_path: &'a Path,
    pub page_idx: usize,
    pub stories: &'a HashMap<u64, u64>,
    pub frames: &'a Labels,
}

pub(crate) fn push_slot_steps(
    s: &Session,
    target: &PageTarget<'_>,
    p: &Value,
    refs_map: &mut BTreeMap<String, SlotReference>,
    steps: &mut Vec<(&'static str, Value)>,
) -> Result<()> {
    let page_idx = target.page_idx;
    let frames = target.frames;
    let stories = target.stories;
    let sb_path = target.sb_path;
    let lock = p.get("lock").and_then(Value::as_bool).unwrap_or(false);
    let mut text_entries: Vec<(String, String)> = Vec::new();
    for key in ["fields", "text"] {
        if let Some(obj) = p.get(key).and_then(Value::as_object) {
            for (label, val) in obj {
                let text = val.as_str().ok_or_else(|| bad(&format!("`{key}` values must be strings")))?;
                text_entries.push((label.clone(), text.to_string()));
            }
        }
    }
    let slot_or_label = p.get("slot").or_else(|| p.get("label")).and_then(Value::as_str);
    if let (Some(label), Some(text)) = (slot_or_label, p.get("text").and_then(Value::as_str)) {
        text_entries.push((label.to_string(), text.to_string()));
    }
    for (label, text) in text_entries {
        reject_readonly_cli_slot(&label)?;
        let frame = *frames.get(&label).ok_or_else(|| bad(&format!("page {page_idx} has no slot `{label}`")))?;
        let story = *stories.get(&frame).ok_or_else(|| fail(format!("slot `{label}` has no story")))?;
        steps.push(("object.setFlags", json!({"ids": [frame], "locked": false})));
        steps.push(("story.setText", json!({"story": story, "text": text})));
        if lock {
            steps.push(("object.setFlags", json!({"ids": [frame], "locked": true})));
        }
    }

    let fit = p.get("fit").and_then(Value::as_str).unwrap_or("fillProportionally");
    let mut image_entries: Vec<(String, String)> = Vec::new();
    for key in ["images", "image"] {
        if let Some(obj) = p.get(key).and_then(Value::as_object) {
            for (label, val) in obj {
                let raw = val.as_str().ok_or_else(|| bad(&format!("`{key}` values must be file paths")))?;
                image_entries.push((label.clone(), raw.to_string()));
            }
        }
    }
    if let (Some(label), Some(raw)) = (slot_or_label, p.get("image").and_then(Value::as_str)) {
        image_entries.push((label.to_string(), raw.to_string()));
    }
    for (label, raw) in image_entries {
        reject_readonly_cli_slot(&label)?;
        let resolved = crate::workspace::resolve_workspace_path(s, &raw);
        let explicit_ref = p
            .pointer(&format!("/references/{label}"))
            .or_else(|| p.get("source_project"))
            .and_then(Value::as_str)
            .filter(|v| !v.trim().is_empty())
            .map(|r| crate::workspace::resolve_workspace_path(s, r));
        let (flat_img, source_project, craft_app) = resolve_flat_image_and_ref(sb_path, page_idx, &label, &resolved, explicit_ref.as_deref())?;
        let frame = *frames.get(&label).ok_or_else(|| bad(&format!("page {page_idx} has no slot `{label}`")))?;
        steps.push(("object.setFlags", json!({"ids": [frame], "locked": false, "hidden": false})));
        steps.push(("place.load", json!({"path": flat_img})));
        steps.push(("place.drop", json!({"x": 0, "y": 0, "frame": frame})));
        steps.push(("object.fit", json!({"mode": fit, "ids": [frame]})));
        if let Some(&hint) = frames.get(&format!("{label}_hint")) {
            steps.push(("object.setFlags", json!({"ids": [hint], "locked": false, "hidden": true})));
        }
        refs_map.insert(
            format!("{page_idx}:{label}"),
            SlotReference {
                page: page_idx,
                slot: label,
                flat_image: flat_img.to_string_lossy().into_owned(),
                source_project,
                craft_app,
                updated_unix: now_unix(),
            },
        );
    }

    if let Some(refs) = p.get("refs").and_then(Value::as_array) {
        if refs.len() > MAX_REFS {
            return Err(bad("at most 12 `refs`"));
        }
        for n in 1..=MAX_REFS {
            let slot_name = format!("ref_{n}");
            let Some(&frame) = frames.get(&slot_name) else { continue };
            if let Some(raw) = refs.get(n - 1).and_then(Value::as_str) {
                let resolved = crate::workspace::resolve_workspace_path(s, raw);
                let (flat_img, source_project, craft_app) = resolve_flat_image_and_ref(sb_path, page_idx, &slot_name, &resolved, None)?;
                steps.push(("object.setFlags", json!({"ids": [frame], "locked": false, "hidden": false})));
                steps.push(("place.load", json!({"path": flat_img})));
                steps.push(("place.drop", json!({"x": 0, "y": 0, "frame": frame})));
                steps.push(("object.fit", json!({"mode": "fillProportionally", "ids": [frame]})));
                refs_map.insert(
                    format!("{page_idx}:{slot_name}"),
                    SlotReference {
                        page: page_idx,
                        slot: slot_name,
                        flat_image: flat_img.to_string_lossy().into_owned(),
                        source_project,
                        craft_app,
                        updated_unix: now_unix(),
                    },
                );
            } else {
                steps.push(("object.setFlags", json!({"ids": [frame], "locked": false, "hidden": true})));
            }
        }
    }
    Ok(())
}

/// `storyboard.new`: create `storyboard.designcraft` from the master template and optionally apply wizard fields.
pub fn cmd_storyboard_new(s: &mut Session, p: &Value) -> Result<Value> {
    let path = doc_path(s, p)?;
    if path.exists() && !p.get("overwrite").and_then(Value::as_bool).unwrap_or(false) {
        return Err(bad(&format!("{} exists; pass overwrite=true to replace it", path.display())));
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    craft_atomic::atomic_write(&path, TEMPLATE)?;
    if p.get("wizard").is_some() || p.get("title").is_some() {
        let mut wiz = p.get("wizard").cloned().unwrap_or_else(|| json!({}));
        if let Some(t) = p.get("title").and_then(Value::as_str)
            && wiz.get("title").is_none()
        {
            wiz["title"] = json!(t);
        }
        wiz["path"] = json!(path);
        let _ = super::workflow::cmd_storyboard_wizard(s, &wiz);
    }
    let pdf = path.with_file_name("storyboard.pdf");
    let pptx = path.with_file_name("storyboard.pptx");
    let _ = run_design_steps(&path, &[("file.exportPdf", json!({"path": pdf})), ("file.exportPptx", json!({"path": pptx}))]);
    Ok(json!({"ok": true, "path": path, "pdf": pdf, "pptx": pptx}))
}

fn attach_preview_png(sb_path: &Path, mut page_val: Value, idx: usize) -> Value {
    page_val["preview_png"] = json!(find_page_preview_png(sb_path, idx));
    page_val
}

/// `storyboard.pages`: inspect pages, labelled slots, rendered preview PNGs, and read-only `cli_instructions`.
pub fn cmd_storyboard_pages(s: &mut Session, p: &Value) -> Result<Value> {
    let path = doc_path(s, p)?;
    let doc = read_designcraft_doc(&path)?;
    let (pages, _) = parse_pages_and_stories(&path, &doc);
    let filter_label = p.get("label").and_then(Value::as_str).filter(|l| !l.is_empty());
    let list: Vec<Value> = if p.get("full").and_then(Value::as_bool) == Some(true) {
        pages.iter().map(|pg| attach_preview_png(&path, pg.to_json(), pg.index)).collect()
    } else if p.get("page").is_some() || p.get("scene").is_some() || filter_label.is_some() {
        let idx = page_index(&pages, p)?;
        let mut pg = attach_preview_png(&path, pages[idx].to_json(), idx);
        if let Some(lbl) = filter_label
            && let Some(slots) = pg["slots"].as_array_mut()
        {
            slots.retain(|sl| sl["label"].as_str() == Some(lbl));
        }
        vec![pg]
    } else {
        pages
            .iter()
            .map(|pg| {
                json!({
                    "page": pg.index,
                    "archetype": pg.archetype,
                    "is_title_section": pg.is_main_title_page(),
                    "section": pg.section,
                    "title": pg.title(),
                    "preview_png": find_page_preview_png(&path, pg.index),
                    "slots": pg.slots.iter().filter(|s| !s.is_cli_helper()).count(),
                    "cli_instructions": pg.slots.iter().filter(|s| s.is_cli_helper()).count(),
                    "cli_command": page_cli_command(pg.index, &pg.archetype),
                })
            })
            .collect()
    };
    Ok(json!({"ok": true, "path": path, "pages": list}))
}

/// `storyboard.set`: update text, flat images, or Craft-project snapshots on one or more pages.
pub fn cmd_storyboard_set(s: &mut Session, p: &Value) -> Result<Value> {
    let path = doc_path(s, p)?;
    let doc = read_designcraft_doc(&path)?;
    let (pages, stories) = parse_pages_and_stories(&path, &doc);
    let mut refs_map = load_slot_references(&path);
    let mut steps = Vec::new();
    if let Some(batch) = p.get("pages").and_then(Value::as_array) {
        let mut updated = Vec::new();
        for entry in batch {
            let idx = page_index(&pages, entry)?;
            let labels = pages[idx].labels();
            let target = PageTarget { sb_path: &path, page_idx: idx, stories: &stories, frames: &labels };
            push_slot_steps(s, &target, entry, &mut refs_map, &mut steps)?;
            updated.push(idx);
        }
        if !steps.is_empty() {
            run_design_steps(&path, &steps)?;
            save_slot_references(&path, &refs_map)?;
        }
        return Ok(json!({"ok": true, "pages": updated, "path": path}));
    }
    let idx = page_index(&pages, p)?;
    let labels = pages[idx].labels();
    let target = PageTarget { sb_path: &path, page_idx: idx, stories: &stories, frames: &labels };
    push_slot_steps(s, &target, p, &mut refs_map, &mut steps)?;
    if !steps.is_empty() {
        run_design_steps(&path, &steps)?;
        save_slot_references(&path, &refs_map)?;
    }
    Ok(json!({
        "ok": true,
        "page": idx,
        "scene": scene_number_for(&pages, idx),
        "archetype": pages[idx].archetype,
        "path": path,
    }))
}

/// `storyboard.add`: duplicate a template spread and rewrite its embedded read-only `*_cli` labels to the new `page` index.
pub fn cmd_storyboard_add(s: &mut Session, p: &Value) -> Result<Value> {
    let path = doc_path(s, p)?;
    let doc = read_designcraft_doc(&path)?;
    let (pages, _) = parse_pages_and_stories(&path, &doc);
    let archetype = p.get("archetype").and_then(Value::as_str).unwrap_or("scene");
    let from = if p.get("page").is_some() || p.get("scene").is_some() || p.get("label").is_some() {
        page_index(&pages, p)?
    } else if let Some(n) = p.get("from").and_then(Value::as_u64) {
        let idx = n as usize;
        if idx < pages.len() { idx } else { return Err(bad(&format!("no page {n}"))) }
    } else {
        pages.iter().filter(|pg| pg.archetype == archetype).map(|pg| pg.index).next_back().ok_or_else(|| bad(&format!("no `{archetype}` template page")))?
    };
    let mut dup_params = json!({"spread": from});
    match p.get("at").and_then(Value::as_u64) {
        Some(at) => dup_params["to"] = json!(at),
        None => dup_params["atEnd"] = json!(true),
    }
    let dup = run_design_steps(&path, &[("layout.pages.duplicateSpread", dup_params)])?;
    let index = dup.first().and_then(|v| v["page"].as_u64()).unwrap_or(pages.len() as u64) as usize;

    let doc_after = read_designcraft_doc(&path)?;
    let (pages_after, stories) = parse_pages_and_stories(&path, &doc_after);
    let scene_no = scene_number_for(&pages_after, index);
    if let Some(pg) = pages_after.get(index) {
        let mut refs_map = load_slot_references(&path);
        let mut steps = Vec::new();
        let labels = pg.labels();
        let target = PageTarget { sb_path: &path, page_idx: index, stories: &stories, frames: &labels };
        push_slot_steps(s, &target, p, &mut refs_map, &mut steps)?;
        if archetype == "scene"
            && p.get("fields").and_then(Value::as_object).is_none_or(|f| !f.contains_key("scene_id"))
            && let (Some(n), Some(&fid)) = (scene_no, labels.get("scene_id"))
            && let Some(&sid) = stories.get(&fid)
        {
            steps.push(("object.setFlags", json!({"ids": [fid], "locked": false})));
            steps.push(("story.setText", json!({"story": sid, "text": format!("SCENE {n:02}")})));
        }
        for sl in pg.slots.iter().filter(|sl| sl.is_cli_helper()) {
            let Some(&sid) = stories.get(&sl.id) else { continue };
            let updated_cli = if sl.label.starts_with("page_") {
                page_cli_command(index, &pg.archetype)
            } else {
                let target_lbl = sl.label.trim_end_matches("_cli");
                let target_kind = pg.slots.iter().find(|x| x.label == target_lbl).map_or("image", |x| x.kind);
                slot_cli_command(index, target_lbl, target_kind)
            };
            steps.push(("object.setFlags", json!({"ids": [sl.id], "locked": false})));
            steps.push(("story.setText", json!({"story": sid, "text": updated_cli})));
            steps.push(("object.setFlags", json!({"ids": [sl.id], "locked": true})));
        }
        if !steps.is_empty() {
            run_design_steps(&path, &steps)?;
            save_slot_references(&path, &refs_map)?;
        }
    }
    Ok(json!({"ok": true, "page": index, "scene": scene_no, "archetype": archetype, "path": path}))
}

/// `storyboard.remove`: delete a storyboard spread by `page` or `scene` (completing full CRUD).
pub fn cmd_storyboard_remove(s: &mut Session, p: &Value) -> Result<Value> {
    let path = doc_path(s, p)?;
    let doc = read_designcraft_doc(&path)?;
    let (pages, _) = parse_pages_and_stories(&path, &doc);
    if pages.len() <= 1 {
        return Err(bad("cannot remove the last remaining storyboard spread"));
    }
    let idx = page_index(&pages, p)?;
    let removed_arch = pages[idx].archetype.clone();
    run_design_steps(&path, &[("layout.pages.deleteSpread", json!({"spread": idx}))])?;
    let mut refs = load_slot_references(&path);
    refs.retain(|_, r| r.page != idx);
    save_slot_references(&path, &refs)?;
    Ok(json!({"ok": true, "removed_page": idx, "archetype": removed_arch, "remaining_pages": pages.len() - 1, "path": path}))
}

/// `storyboard.locations`: inspect or edit `world` / location spreads.
pub fn cmd_storyboard_locations(s: &mut Session, p: &Value) -> Result<Value> {
    let path = doc_path(s, p)?;
    let doc = read_designcraft_doc(&path)?;
    let (pages, _) = parse_pages_and_stories(&path, &doc);
    let world_pages: Vec<usize> = pages.iter().filter(|pg| pg.archetype == "world").map(|pg| pg.index).collect();
    if p.get("fields").is_some() || p.get("images").is_some() || p.get("text").is_some() || p.get("image").is_some() || p.get("refs").is_some() {
        let mut set_req = p.clone();
        if set_req.get("page").is_none() && set_req.get("label").is_none() {
            let first_world = *world_pages.first().ok_or_else(|| bad("storyboard has no `world` page"))?;
            set_req["page"] = json!(first_world);
        }
        cmd_storyboard_set(s, &set_req)?;
    }
    let refreshed = read_designcraft_doc(&path)?;
    let (pages_after, _) = parse_pages_and_stories(&path, &refreshed);
    let locations: Vec<Value> = pages_after.iter().filter(|pg| pg.archetype == "world").map(|pg| attach_preview_png(&path, pg.to_json(), pg.index)).collect();
    Ok(json!({"ok": true, "path": path, "locations": locations}))
}

/// `storyboard.section`: add, rename, or apply a parent section in DesignCraft.
pub fn cmd_storyboard_section(s: &mut Session, p: &Value) -> Result<Value> {
    let path = doc_path(s, p)?;
    let doc = read_designcraft_doc(&path)?;
    let existing = parent_labels(&doc);
    let raw_name = p.get("name").and_then(Value::as_str).ok_or_else(|| bad("give `name` (e.g. \"B\" or \"B-World\")"))?.trim();
    let prefix = raw_name.split_once('-').map_or(raw_name, |(pfx, _)| pfx).trim();
    let title = raw_name.split_once('-').map(|(_, rest)| rest.trim()).filter(|r| !r.is_empty()).unwrap_or("Parent");
    let mut steps = Vec::new();
    let label = if let Some((_, found)) = existing.iter().find(|(_, lbl)| *lbl == raw_name || lbl.starts_with(&format!("{prefix}-"))) {
        if raw_name.contains('-') && found != raw_name {
            steps.push(("layout.parents.options", json!({"parent": found, "prefix": prefix, "name": title})));
            format!("{prefix}-{title}")
        } else {
            found.clone()
        }
    } else {
        steps.push(("layout.parents.new", json!({"prefix": prefix, "name": title})));
        format!("{prefix}-{title}")
    };
    if let Some(base) = p.get("based_on").and_then(Value::as_str).filter(|b| !b.is_empty()) {
        steps.push(("layout.parents.options", json!({"parent": label, "basedOn": base})));
    }
    let mut target_pages: Vec<u64> = p.get("pages").and_then(Value::as_array).map(|l| l.iter().filter_map(Value::as_u64).collect()).unwrap_or_default();
    if let Some(single) = p.get("page").and_then(Value::as_u64)
        && !target_pages.contains(&single)
    {
        target_pages.push(single);
    }
    if !target_pages.is_empty() {
        steps.push(("layout.pages.applyParent", json!({"pages": target_pages, "parent": label})));
    }
    if !steps.is_empty() {
        run_design_steps(&path, &steps)?;
    }
    Ok(json!({"ok": true, "section": label, "pages": target_pages, "path": path}))
}

fn modified_ms(path: &Path) -> Result<u64> {
    let at = std::fs::metadata(path)?.modified()?;
    Ok(at.duration_since(std::time::UNIX_EPOCH).map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX)))
}

/// `storyboard.thumbnail`: render page or rail PNGs via `designcraft-cli run`.
pub fn cmd_storyboard_thumbnail(s: &mut Session, p: &Value) -> Result<Value> {
    let doc = doc_path(s, p)?;
    let scale = p.get("scale").and_then(Value::as_f64).unwrap_or(0.5);
    let preview_root = doc.with_file_name("storyboard").join(".preview");
    if p.get("rail").and_then(Value::as_bool) == Some(true) {
        let dir = preview_root.join(format!("rail-{}", (scale * 100.0).round()));
        let source = modified_ms(&doc)?;
        let marker = dir.join(".source-modified");
        let fresh = std::fs::read_to_string(&marker).ok().and_then(|t| t.trim().parse::<u64>().ok()) == Some(source);
        if !fresh {
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir)?;
            render_design_pages(&doc, None, scale, &dir)?;
            craft_atomic::atomic_write(&marker, source.to_string().as_bytes())?;
        }
        let mut files: Vec<PathBuf> =
            std::fs::read_dir(&dir)?.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|x| x == "png")).collect();
        files.sort();
        return Ok(json!({"ok": true, "rail": true, "files": files, "modified": source, "cached": fresh}));
    }
    let parsed_doc = read_designcraft_doc(&doc)?;
    let (pages, _) = parse_pages_and_stories(&doc, &parsed_doc);
    let index = page_index(&pages, p)?;
    let out = match p.get("out").and_then(Value::as_str).filter(|v| !v.trim().is_empty()) {
        Some(raw) => crate::workspace::resolve_workspace_path(s, raw),
        None => preview_root.join(format!("page-{index}-{}.png", (scale * 100.0).round())),
    };
    let source = modified_ms(&doc)?;
    let fresh = modified_ms(&out).is_ok_and(|rendered| rendered >= source);
    if !fresh {
        if let Some(dir) = out.parent() {
            std::fs::create_dir_all(dir)?;
        }
        render_design_pages(&doc, Some(index), scale, &out)?;
    }
    Ok(json!({"ok": true, "page": index, "out": out, "modified": modified_ms(&out)?, "cached": fresh}))
}

/// `storyboard.export`: export `.designcraft` to PDF (default `storyboard.pdf`), PPTX/DeckCraft, or PNG spreads.
pub fn cmd_storyboard_export(s: &mut Session, p: &Value) -> Result<Value> {
    let path = doc_path(s, p)?;
    let mut done = serde_json::Map::new();
    let mut steps = Vec::new();
    let has_any = p.get("pdf").is_some() || p.get("pptx").is_some() || p.get("deck").is_some() || p.get("png_dir").is_some();
    let default_pdf = (!has_any).then(|| path.with_file_name("storyboard.pdf"));
    if let Some(raw_pdf) = p.get("pdf").and_then(Value::as_str).filter(|s| !s.trim().is_empty()) {
        let pdf = crate::workspace::resolve_workspace_path(s, raw_pdf);
        if let Some(dir) = pdf.parent() {
            std::fs::create_dir_all(dir)?;
        }
        steps.push(("file.exportPdf", json!({"path": pdf})));
        done.insert("pdf".into(), json!(pdf));
    } else if let Some(pdf) = default_pdf {
        steps.push(("file.exportPdf", json!({"path": pdf})));
        done.insert("pdf".into(), json!(pdf));
    }
    if let Some(raw_pptx) = p.get("pptx").or_else(|| p.get("deck")).and_then(Value::as_str).filter(|s| !s.trim().is_empty()) {
        let pptx = crate::workspace::resolve_workspace_path(s, raw_pptx);
        if let Some(dir) = pptx.parent() {
            std::fs::create_dir_all(dir)?;
        }
        steps.push(("file.exportPptx", json!({"path": pptx})));
        done.insert("pptx".into(), json!(pptx));
    }
    if !steps.is_empty() {
        run_design_steps(&path, &steps)?;
    }
    if let Some(raw_dir) = p.get("png_dir").and_then(Value::as_str).filter(|s| !s.trim().is_empty()) {
        let dir = crate::workspace::resolve_workspace_path(s, raw_dir);
        std::fs::create_dir_all(&dir)?;
        render_design_pages(&path, None, p.get("scale").and_then(Value::as_f64).unwrap_or(1.0), &dir)?;
        done.insert("png_dir".into(), json!(dir));
    }
    if done.is_empty() {
        return Err(bad("give `pdf`, `pptx`, or `png_dir`"));
    }
    Ok(json!({"ok": true, "exported": done}))
}

/// `storyboard.open`: launch or focus `DesignCraft --control` with `storyboard.designcraft`.
pub fn cmd_storyboard_open(s: &mut Session, p: &Value) -> Result<Value> {
    let path = doc_path(s, p)?;
    if !path.is_file() {
        return Err(bad("Create storyboard.designcraft first with storyboard.new"));
    }
    let port = match p.get("control") {
        None | Some(Value::Null) => None,
        Some(v) => Some(v.as_u64().and_then(|n| u16::try_from(n).ok()).filter(|&n| n > 0).ok_or_else(|| bad("control must be 1..=65535"))?),
    };
    CraftHub::launch_with_control(CraftTarget::Design, Some(&path), port, s.project_path.as_deref().and_then(Path::parent))
}
