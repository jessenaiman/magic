//! Storyboard printable reference document (`storyboard.pdf`) CRUD & `PDFCraft` operations,
//! integrated with `PhotoCraft` non-destructive layer prep, `VectorCraft` tracing, and
//! `EffectCraft` animated character pose stamping.
//!
//! All section names and field rules come strictly from the `.designcraft` master (`!slot.locked`).
//! Every placed asset/project stores its flat project preview image in the Storyboard alongside
//! structured metadata linking back to the source files.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::{
    FieldState, ImageLinkMeta, PDF_TEMPLATE, PageInfo, bad, doc_path, field_key,
    load_field_state, master_sections, parse_storyboard_pages, save_field_state,
    sync_assets_into_state,
};
use crate::{Result, Session};

pub(crate) fn page_index(pages: &[PageInfo], p: &Value) -> Result<usize> {
    if let Some(n) = p.get("page").and_then(Value::as_u64) {
        let idx = n as usize;
        return if idx < pages.len() {
            Ok(idx)
        } else {
            Err(bad(&format!("no page {n}; storyboard.pdf has {} pages", pages.len())))
        };
    }
    if let Some(label) = p.get("slot").or_else(|| p.get("label")).and_then(Value::as_str) {
        return pages
            .iter()
            .find(|pg| pg.slots.iter().any(|s| !s.locked && s.label == label))
            .map(|pg| pg.index)
            .ok_or_else(|| bad(&format!("no unlocked field `{label}` in storyboard")));
    }
    if let Some(n) = p.get("scene").and_then(Value::as_u64) {
        let scene_pages: Vec<&PageInfo> = pages
            .iter()
            .filter(|pg| pg.slots.iter().any(|s| s.label == "scene_id"))
            .collect();
        let base = scene_pages
            .get((n as usize).wrapping_sub(1))
            .ok_or_else(|| bad(&format!("no scene {n}")))?;
        return Ok(base.index);
    }
    Err(bad("give `page`, `label`, or `scene`"))
}

fn find_page_preview_png(pdf_path: &Path, page_idx: usize) -> Option<PathBuf> {
    let sb_dir = pdf_path.with_file_name("storyboard");
    let preview_dir = sb_dir.join(".preview");
    for candidate in [
        preview_dir.join(format!("slide-{page_idx}-200.png")),
        preview_dir.join(format!("page-{page_idx}-200.png")),
        preview_dir.join(format!("page-{page_idx}-150.png")),
        preview_dir.join(format!("page-{page_idx}-100.png")),
        sb_dir.join("spreads").join(format!("page_{page_idx:04}.png")),
    ] {
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    let mut rail_dirs: Vec<PathBuf> = std::fs::read_dir(&preview_dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    rail_dirs.sort();
    rail_dirs.reverse();
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
    None
}

fn has_photocraft_slot_ops(p: &Value) -> bool {
    p["remove_background"].as_bool() == Some(true)
        || p["auto_tone"].as_bool() == Some(true)
        || p["auto_contrast"].as_bool() == Some(true)
        || p["auto_color"].as_bool() == Some(true)
        || p.get("vibrance").is_some_and(|v| v.is_number())
        || p.get("curves").is_some_and(Value::is_array)
        || p.get("lut").and_then(Value::as_str).is_some_and(|s| !s.trim().is_empty())
        || p.get("sharpen").is_some_and(|v| v.is_number())
        || p.get("stroke").is_some_and(Value::is_object)
        || p.get("drop_shadow").is_some_and(Value::is_object)
}

/// Stamp a character pose frame directly from an `EffectCraft` `.ecproj` (or `.pcraft` cutout)
/// into `storyboard/craft/<fkey>.png`, linking back to the live character rig file.
fn resolve_character_pose_for_storyboard(
    s: &Session,
    pdf_path: &Path,
    p: &Value,
    fkey: &str,
) -> Result<ImageLinkMeta> {
    let view = crate::cmd::character::View::from_params(s, p)?;
    let time_sec = p["effect_time"].as_f64().or_else(|| p["time"].as_f64()).unwrap_or(0.0).max(0.0);
    let action_opt = p["action"].as_str().map(str::trim).filter(|a| !a.is_empty());
    let sprite_ec = action_opt.and_then(|act| {
        let sp_dir = crate::workshop::project_dir(s)
            .ok()?
            .join("assets/characters")
            .join(&view.id)
            .join("sprites")
            .join(act);
        let cand = sp_dir.join(format!("{}-{act}.ecproj", view.id));
        cand.is_file().then_some(cand)
    });
    let chosen_ec = sprite_ec.or_else(|| {
        let ec = view.file("ecproj");
        ec.is_file().then_some(ec)
    });

    if let Some(ecproj) = chosen_ec {
        let craft_dir = pdf_path.with_file_name("storyboard").join("craft");
        std::fs::create_dir_all(&craft_dir)?;
        let pose_png = craft_dir.join(format!("{fkey}-pose.png"));
        let _ = crate::native_media::extract_or_copy_preview(&ecproj, &pose_png, 1024)?;
        return Ok(ImageLinkMeta {
            image: pose_png.to_string_lossy().into_owned(),
            linked_file: Some(ecproj.to_string_lossy().into_owned()),
            asset_id: Some(view.id),
            craft_app: Some("effectcraft".into()),
            outline: magic_util::media::craft_outline_summary(&ecproj),
            time_sec: Some(time_sec),
        });
    }

    let photo_doc = view.require_photo_doc("character.cut")?;
    let project_dir = s.project_path.as_deref().and_then(Path::parent);
    let (proj_img, linked_file, craft_app) =
        crate::cmd::project::assets::resolve_asset_project_image(project_dir, &photo_doc);
    Ok(ImageLinkMeta {
        image: proj_img.to_string_lossy().into_owned(),
        linked_file: linked_file.or_else(|| Some(photo_doc.to_string_lossy().into_owned())),
        asset_id: Some(view.id),
        craft_app,
        outline: magic_util::media::craft_outline_summary(&photo_doc),
        time_sec: None,
    })
}

/// Rule 4: Resolve any project asset or Craft document into `(flat_project_image_png, linked_file, asset_id, craft_app)`
/// and run optional `PhotoCraft` / `VectorCraft` / `EffectCraft` slot pipelines when requested via CLI.
fn resolve_storyboard_image_with_link(
    s: &Session,
    pdf_path: &Path,
    raw: &str,
    p: &Value,
    fkey: &str,
) -> Result<ImageLinkMeta> {
    let explicit_link = p.get("source_project").or_else(|| p.get("linked_file")).and_then(Value::as_str);
    let project_dir = s.project_path.as_deref().and_then(Path::parent);
    let matched_asset = s.project_assets.iter().find(|a| a.id == raw || a.name == raw);
    let input = if let Some(asset) = matched_asset
        && let Some(ref apath) = asset.path
    {
        apath.clone()
    } else {
        crate::workspace::resolve_workspace_path(s, raw)
    };
    if !input.exists() {
        return Err(bad(&format!("file not found: {}", input.display())));
    }

    let craft_dir = pdf_path.with_file_name("storyboard").join("craft");
    let matched_asset_id = matched_asset
        .map(|a| a.id.clone())
        .or_else(|| s.project_assets.iter().find(|a| a.path.as_deref() == Some(input.as_path())).map(|a| a.id.clone()));

    if input.extension().is_some_and(|e| e.eq_ignore_ascii_case("ecproj"))
        && let Some(t_sec) = p["effect_time"].as_f64().or_else(|| p["time"].as_f64())
    {
        std::fs::create_dir_all(&craft_dir)?;
        let pose_png = craft_dir.join(format!("{fkey}.png"));
        let _ = crate::native_media::extract_or_copy_preview(&input, &pose_png, 1024)?;
        return Ok(ImageLinkMeta {
            image: pose_png.to_string_lossy().into_owned(),
            linked_file: Some(explicit_link.map(str::to_owned).unwrap_or_else(|| input.to_string_lossy().into_owned())),
            asset_id: matched_asset_id,
            craft_app: Some("effectcraft".into()),
            outline: magic_util::media::craft_outline_summary(&input),
            time_sec: Some(t_sec.max(0.0)),
        });
    }

    if has_photocraft_slot_ops(p) {
        std::fs::create_dir_all(&craft_dir)?;
        let out_pcraft = craft_dir.join(format!("{fkey}.pcraft"));
        let out_png = craft_dir.join(format!("{fkey}.png"));
        let _ = crate::native_media::isolate_subject_rgba(&input, &out_png, p["chroma"].as_str(), 0.8);
        return Ok(ImageLinkMeta {
            image: out_png.to_string_lossy().into_owned(),
            linked_file: Some(explicit_link.map(str::to_owned).unwrap_or_else(|| out_pcraft.to_string_lossy().into_owned())),
            asset_id: matched_asset_id,
            craft_app: Some("photocraft".into()),
            outline: magic_util::media::craft_outline_summary(&out_pcraft),
            time_sec: None,
        });
    }

    if p["vectorize"].as_bool() == Some(true) {
        std::fs::create_dir_all(&craft_dir)?;
        let out_vcraft = craft_dir.join(format!("{fkey}.svg"));
        let out_png = craft_dir.join(format!("{fkey}.png"));
        crate::native_media::trace_silhouette_svg(&input, &out_vcraft, 1.5, p["strokes"].as_bool().unwrap_or(false))?;
        let _ = crate::native_media::extract_or_copy_preview(&input, &out_png, 1024);
        let preview = if out_png.is_file() { out_png } else { input.clone() };
        return Ok(ImageLinkMeta {
            image: preview.to_string_lossy().into_owned(),
            linked_file: Some(explicit_link.map(str::to_owned).unwrap_or_else(|| out_vcraft.to_string_lossy().into_owned())),
            asset_id: matched_asset_id,
            craft_app: Some("vectorcraft".into()),
            outline: magic_util::media::craft_outline_summary(&out_vcraft),
            time_sec: None,
        });
    }

    let dir_for_preview = project_dir.or_else(|| pdf_path.parent());
    let (proj_img, linked_file, craft_app) =
        crate::cmd::project::assets::resolve_asset_project_image(dir_for_preview, &input);
    let final_linked = explicit_link
        .map(str::to_owned)
        .or(linked_file)
        .or_else(|| Some(input.to_string_lossy().into_owned()));
    let outline = final_linked
        .as_deref()
        .and_then(|lf| magic_util::media::craft_outline_summary(Path::new(lf)))
        .or_else(|| magic_util::media::craft_outline_summary(&input));

    Ok(ImageLinkMeta {
        image: proj_img.to_string_lossy().into_owned(),
        linked_file: final_linked,
        asset_id: matched_asset_id,
        craft_app,
        outline,
        time_sec: None,
    })
}

fn push_image_step(pdf_steps: &mut Vec<(&'static str, Value)>, page_idx: usize, bounds: [f64; 4], meta: &ImageLinkMeta) {
    pdf_steps.push((
        "annot_image",
        json!({
            "page": page_idx,
            "bounds": bounds,
            "path": meta.image,
            "fit": "fillProportionally",
            "metadata": {
                "linked_file": meta.linked_file,
                "asset_id": meta.asset_id,
                "craft_app": meta.craft_app,
                "outline": meta.outline,
                "time_sec": meta.time_sec,
            }
        }),
    ));
}

fn apply_page_edits(
    s: &Session,
    pdf_path: &Path,
    page: &PageInfo,
    p: &Value,
    state: &mut FieldState,
    pdf_steps: &mut Vec<(&'static str, Value)>,
) -> Result<()> {
    let page_idx = page.index;
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
        let slot = page
            .slots
            .iter()
            .find(|sl| sl.label == label && !sl.locked)
            .ok_or_else(|| bad(&format!("page {page_idx} has no unlocked field `{label}`")))?;
        let fkey = slot.form_name.clone();
        state.text.insert(fkey.clone(), text.clone());
        pdf_steps.push(("form_fill", json!({"field": fkey, "value": text})));
    }

    if p.get("character").and_then(Value::as_str).is_some_and(|c| !c.trim().is_empty())
        && p.get("image").is_none()
        && let Some(label) = slot_or_label
    {
        let slot = page
            .slots
            .iter()
            .find(|sl| sl.label == label && !sl.locked)
            .ok_or_else(|| bad(&format!("page {page_idx} has no unlocked image slot `{label}`")))?;
        let fkey = field_key(page_idx, label);
        let meta = resolve_character_pose_for_storyboard(s, pdf_path, p, &fkey)?;
        push_image_step(pdf_steps, page_idx, slot.bounds, &meta);
        state.images.insert(fkey, meta);
        return Ok(());
    }

    let mut image_entries: Vec<(String, String)> = Vec::new();
    for key in ["images", "image"] {
        if let Some(obj) = p.get(key).and_then(Value::as_object) {
            for (label, val) in obj {
                let raw = val.as_str().ok_or_else(|| bad(&format!("`{key}` values must be file paths or asset IDs")))?;
                image_entries.push((label.clone(), raw.to_string()));
            }
        }
    }
    if let (Some(label), Some(raw)) = (slot_or_label, p.get("image").and_then(Value::as_str)) {
        image_entries.push((label.to_string(), raw.to_string()));
    }
    for (label, raw) in image_entries {
        let slot = page
            .slots
            .iter()
            .find(|sl| sl.label == label && !sl.locked)
            .ok_or_else(|| bad(&format!("page {page_idx} has no unlocked image slot `{label}`")))?;
        let fkey = field_key(page_idx, &label);
        let meta = resolve_storyboard_image_with_link(s, pdf_path, &raw, p, &fkey)?;
        push_image_step(pdf_steps, page_idx, slot.bounds, &meta);
        state.images.insert(fkey, meta);
    }
    Ok(())
}

/// `storyboard.new`: initialize printable `<project_dir>/storyboard.pdf` from `storyboard-template.pdf`.
pub fn cmd_storyboard_new(s: &mut Session, p: &Value) -> Result<Value> {
    let path = doc_path(s, p)?;
    let overwrite = p.get("overwrite").and_then(Value::as_bool).unwrap_or(false);
    if overwrite {
        crate::doc_state::write_file(&path, PDF_TEMPLATE)?;
    } else if let Err(e) = crate::doc_state::write_file_new(&path, PDF_TEMPLATE) {
        return match e {
            EngineError::Io(io_err) if io_err.kind() == std::io::ErrorKind::AlreadyExists => {
                Err(bad(&format!("{} exists; pass overwrite=true to replace it", path.display())))
            }
            other => Err(other),
        };
    }
    let mut state = FieldState::default();
    if let Some(title) = p.get("title").and_then(Value::as_str).filter(|t| !t.trim().is_empty()) {
        state.text.insert("p00_cover_title".into(), title.trim().to_owned());
    }
    sync_assets_into_state(s, &mut state);
    save_field_state(&path, &state)?;
    if let Some(wiz) = p.get("wizard") {
        let mut wiz_req = wiz.clone();
        wiz_req["path"] = json!(path);
        let _ = super::workflow::cmd_storyboard_wizard(s, &wiz_req);
    }
    Ok(json!({"ok": true, "path": path, "format": "pdfcraft"}))
}

fn attach_preview_png(pdf_path: &Path, mut page_val: Value, idx: usize) -> Value {
    page_val["preview_png"] = json!(find_page_preview_png(pdf_path, idx));
    page_val
}

/// `storyboard.pages`: inspect printable `storyboard.pdf` pages, master sections, and unlocked fields.
pub fn cmd_storyboard_pages(s: &mut Session, p: &Value) -> Result<Value> {
    let path = doc_path(s, p)?;
    if !path.is_file() {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        craft_atomic::atomic_write(&path, PDF_TEMPLATE)?;
    }
    let is_slides = super::slides::is_slide_storyboard(&path);
    let pages = parse_storyboard_pages(Some(s), &path);
    let sections = if is_slides { super::slides::slide_sections(&path) } else { master_sections() };
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
                let req = pg.slots.iter().filter(|sl| !sl.locked).count();
                let filled = pg.slots.iter().filter(|sl| !sl.locked && sl.filled).count();
                json!({
                    "page": pg.index,
                    "page_number": pg.index + 1,
                    "spread": pg.spread_index,
                    "section": pg.section,
                    "title": pg.title(),
                    "preview_png": find_page_preview_png(&path, pg.index),
                    "required_slots": req,
                    "filled_slots": filled,
                    "light": if req == 0 || filled == req { "green" } else { "red" },
                })
            })
            .collect()
    };
    Ok(json!({
        "ok": true,
        "path": path,
        "format": if is_slides { "deckcraft-slides" } else { "pdfcraft" },
        "page_width": if is_slides { super::slides::SLIDE_W_PT } else { 612.0 },
        "page_height": if is_slides { super::slides::SLIDE_H_PT } else { 792.0 },
        "sections": sections,
        "pages": list,
    }))
}

/// `storyboard.set`: update text or store a project image + file-link metadata on `storyboard.pdf`.
pub fn cmd_storyboard_set(s: &mut Session, p: &Value) -> Result<Value> {
    let path = doc_path(s, p)?;
    if super::slides::is_slide_storyboard(&path) {
        let pages = parse_storyboard_pages(Some(s), &path);
        let idx = page_index(&pages, p)?;
        return super::slides::update_slide_slot(&path, idx, p);
    }
    if !path.is_file() {
        craft_atomic::atomic_write(&path, PDF_TEMPLATE)?;
    }
    let pages = parse_storyboard_pages(Some(s), &path);
    let mut state = load_field_state(&path);
    let mut pdf_steps = Vec::new();

    if let Some(batch) = p.get("pages").and_then(Value::as_array) {
        let mut updated = Vec::new();
        for entry in batch {
            let idx = page_index(&pages, entry)?;
            apply_page_edits(s, &path, &pages[idx], entry, &mut state, &mut pdf_steps)?;
            updated.push(idx);
        }
        save_field_state(&path, &state)?;
        if !pdf_steps.is_empty() {
        }
        return Ok(json!({"ok": true, "pages": updated, "path": path}));
    }

    let idx = page_index(&pages, p)?;
    apply_page_edits(s, &path, &pages[idx], p, &mut state, &mut pdf_steps)?;
    save_field_state(&path, &state)?;
    if !pdf_steps.is_empty() {
    }
    Ok(json!({
        "ok": true,
        "page": idx,
        "section": pages[idx].section,
        "path": path,
    }))
}

pub fn cmd_storyboard_add(s: &mut Session, p: &Value) -> Result<Value> {
    let path = doc_path(s, p)?;
    if super::slides::is_slide_storyboard(&path) {
        return super::slides::append_slide_beat(&path, p);
    }
    Err(bad("master storyboard.pdf layout is immutable; use a slide storyboard (.pptx / slides.json) to append beats"))
}

pub fn cmd_storyboard_remove(s: &mut Session, p: &Value) -> Result<Value> {
    let _ = (s, p);
    Err(bad("master storyboard.pdf pages cannot be deleted directly"))
}

pub fn cmd_storyboard_section(s: &mut Session, p: &Value) -> Result<Value> {
    let path = doc_path(s, p)?;
    let pages = parse_storyboard_pages(Some(s), &path);
    Ok(json!({"ok": true, "path": path, "page_count": pages.len()}))
}

pub fn cmd_storyboard_locations(s: &mut Session, p: &Value) -> Result<Value> {
    let path = doc_path(s, p)?;
    if !path.is_file() {
        craft_atomic::atomic_write(&path, PDF_TEMPLATE)?;
    }
    let pages = parse_storyboard_pages(Some(s), &path);
    if p.get("fields").is_some() || p.get("images").is_some() || p.get("text").is_some() || p.get("image").is_some() {
        let mut set_req = p.clone();
        if set_req.get("page").is_none()
            && set_req.get("label").is_none()
            && let Some(pg) = pages.iter().find(|pg| pg.slots.iter().any(|sl| sl.label == "world_subheading"))
        {
            set_req["page"] = json!(pg.index);
        }
        cmd_storyboard_set(s, &set_req)?;
    }
    let refreshed = parse_storyboard_pages(Some(s), &path);
    let locations: Vec<Value> = refreshed
        .iter()
        .filter(|pg| pg.slots.iter().any(|sl| sl.label.starts_with("world_")))
        .map(|pg| attach_preview_png(&path, pg.to_json(), pg.index))
        .collect();
    Ok(json!({"ok": true, "path": path, "locations": locations}))
}

pub fn cmd_storyboard_thumbnail(s: &mut Session, p: &Value) -> Result<Value> {
    let pdf = doc_path(s, p)?;
    if !pdf.is_file() {
        craft_atomic::atomic_write(&pdf, PDF_TEMPLATE)?;
    }
    let scale = p.get("scale").and_then(Value::as_f64).unwrap_or(1.5);
    let preview_root = pdf.with_file_name("storyboard").join(".preview");
    if p.get("rail").and_then(Value::as_bool) == Some(true) {
        let dir = preview_root.join(format!("rail-{}", (scale * 100.0).round()));
        std::fs::create_dir_all(&dir)?;
        let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "png"))
            .collect();
        files.sort();
        return Ok(json!({"ok": true, "rail": true, "files": files}));
    }
    let pages = parse_storyboard_pages(Some(s), &pdf);
    let index = page_index(&pages, p)?;
    let out = find_page_preview_png(&pdf, index)
        .unwrap_or_else(|| preview_root.join(format!("page-{index}-{}.png", (scale * 100.0).round())));
    Ok(json!({"ok": true, "page": index, "out": out}))
}

pub fn cmd_storyboard_export(s: &mut Session, p: &Value) -> Result<Value> {
    let src_pdf = doc_path(s, p)?;
    if !src_pdf.is_file() {
        craft_atomic::atomic_write(&src_pdf, PDF_TEMPLATE)?;
    }
    let mut done = serde_json::Map::new();
    if let Some(raw_pdf) = p.get("pdf").and_then(Value::as_str).filter(|s| !s.trim().is_empty()) {
        let dst = crate::workspace::resolve_workspace_path(s, raw_pdf);
        if dst != src_pdf {
            if let Some(dir) = dst.parent() {
                std::fs::create_dir_all(dir)?;
            }
            let bytes = std::fs::read(&src_pdf)?;
            craft_atomic::atomic_write(&dst, &bytes)?;
        }
        done.insert("pdf".into(), json!(dst));
    } else {
        done.insert("pdf".into(), json!(src_pdf));
    }
    if let Some(raw_dir) = p.get("png_dir").and_then(Value::as_str).filter(|s| !s.trim().is_empty()) {
        let dir = crate::workspace::resolve_workspace_path(s, raw_dir);
        std::fs::create_dir_all(&dir)?;
        done.insert("png_dir".into(), json!(dir));
    }
    Ok(json!({"ok": true, "exported": done}))
}

pub fn cmd_storyboard_open(s: &mut Session, p: &Value) -> Result<Value> {
    let pdf_path = doc_path(s, p)?;
    if !pdf_path.is_file() {
        if let Some(dir) = pdf_path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        craft_atomic::atomic_write(&pdf_path, PDF_TEMPLATE)?;
    }
    let app = if super::slides::is_slide_storyboard(&pdf_path) { "deckcraft" } else { "pdfcraft" };
    crate::workspace::open_editor(app, &pdf_path)?;
    Ok(json!({
        "ok": true,
        "app": app,
        "path": pdf_path,
        "focused_page": p.get("page").and_then(Value::as_u64),
    }))
}
