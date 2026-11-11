//! Storyboard readiness (`storyboard.check`), setup helper (`storyboard.wizard`), batch
//! PhotoCraft panel grading (`storyboard.grade` via `photocraft-cli batch --actions`), and bulk
//! reference-driven media generation (`storyboard.bulk_generate`) governed strictly by the
//! `.designcraft` master (`doc["sections"]` + `!slot.locked`).

use std::path::Path;

use serde_json::{Value, json};

use super::crud::cmd_storyboard_set;
use super::{
    PDF_TEMPLATE, PageInfo, bad, doc_path, fail, field_key, human_slot_title, load_field_state, master_sections, parse_storyboard_pages, save_field_state,
    storyboard_media_references_for,
};
use crate::{CraftHub, CraftTarget, Result, Session};

fn evaluate_page_light(page: &PageInfo) -> (Value, Vec<Value>) {
    let mut unfilled = Vec::new();
    let mut page_reqs = Vec::new();
    let mut filled_count = 0usize;
    let mut total_checked = 0usize;

    for slot in page.slots.iter().filter(|s| !s.locked) {
        total_checked += 1;
        let light = if slot.filled { "green" } else { "red" };
        let req = json!({
            "page": page.index,
            "page_number": page.index + 1,
            "spread": page.spread_index,
            "section": page.section,
            "label": slot.label,
            "form_field": slot.form_name,
            "title": human_slot_title(&slot.label),
            "kind": slot.kind,
            "filled": slot.filled,
            "light": light,
            "text": slot.text,
            "image": slot.image,
            "linked_file": slot.linked_file,
            "asset_id": slot.asset_id,
            "craft_app": slot.craft_app,
            "outline": slot.outline,
            "time_sec": slot.time_sec,
            "bounds": slot.bounds,
            "cli_command": slot.cli_command,
        });
        if slot.filled {
            filled_count += 1;
        } else {
            unfilled.push(req.clone());
        }
        page_reqs.push(req);
    }

    let light = if unfilled.is_empty() { "green" } else { "red" };
    let summary = json!({
        "page": page.index,
        "page_number": page.index + 1,
        "spread": page.spread_index,
        "title": page.title(),
        "section": page.section,
        "light": light,
        "filled_count": filled_count,
        "total_required": total_checked,
        "missing_count": unfilled.len(),
        "unfilled": unfilled,
        "requirements": page_reqs,
    });
    (summary, page_reqs)
}

/// `storyboard.check`: evaluate Red/Green lights across every unlocked field and section defined in the `.designcraft` master.
pub fn cmd_storyboard_check(s: &mut Session, p: &Value) -> Result<Value> {
    let path = doc_path(s, p)?;
    if !path.is_file() {
        return Ok(json!({
            "ok": true,
            "exists": false,
            "path": path,
            "format": "pdfcraft",
            "light": "red",
            "status": "missing",
            "title_sections_complete": false,
            "scenes_complete": false,
            "production_ready": false,
            "sections": [],
            "requirements": [],
            "pages": [],
            "next_step": "Run `magic-cli storyboard new` to initialize printable storyboard.pdf.",
        }));
    }
    let pages = parse_storyboard_pages(Some(s), &path);
    let sec_names = master_sections();
    let mut evaluated = Vec::with_capacity(pages.len());
    let mut requirements = Vec::new();
    for pg in &pages {
        let (pg_val, mut pg_reqs) = evaluate_page_light(pg);
        evaluated.push(pg_val);
        requirements.append(&mut pg_reqs);
    }

    let mut sections = Vec::with_capacity(sec_names.len());
    for sec_name in &sec_names {
        let sec_reqs: Vec<Value> = requirements.iter().filter(|r| r["section"].as_str() == Some(sec_name.as_str())).cloned().collect();
        let sec_pages: Vec<usize> = pages.iter().filter(|pg| &pg.section == sec_name).map(|pg| pg.index).collect();
        let total_req = sec_reqs.len();
        let filled_req = sec_reqs.iter().filter(|r| r["filled"] == true).count();
        let missing_req = total_req.saturating_sub(filled_req);
        let sec_light = if missing_req == 0 && total_req > 0 { "green" } else { "red" };
        sections.push(json!({
            "section": sec_name,
            "light": sec_light,
            "filled_count": filled_req,
            "total_required": total_req,
            "missing_count": missing_req,
            "pages": sec_pages,
            "first_page": sec_pages.first().copied().unwrap_or(0),
            "requirements": sec_reqs,
        }));
    }

    let total_required = requirements.len();
    let filled_total = requirements.iter().filter(|r| r["filled"] == true).count();
    let missing_total = total_required.saturating_sub(filled_total);
    let production_ready = missing_total == 0 && total_required > 0;
    let title_sections_complete = sections.iter().take(3).all(|sec| sec["light"] == "green");
    let scenes_complete =
        sections.iter().find(|sec| sec["section"].as_str().is_some_and(|n| n.eq_ignore_ascii_case("scenes"))).is_some_and(|sec| sec["light"] == "green");

    Ok(json!({
        "ok": true,
        "exists": true,
        "path": path,
        "format": "pdfcraft",
        "light": if production_ready { "green" } else { "red" },
        "complete": production_ready,
        "status": if production_ready { "complete" } else { "incomplete" },
        "count": missing_total,
        "filled_count": filled_total,
        "total_required": total_required,
        "page_count": pages.len(),
        "title_sections_complete": title_sections_complete,
        "scenes_complete": scenes_complete,
        "production_ready": production_ready,
        "next_step": if production_ready {
            "Printable storyboard.pdf is complete and ready as the project reference book."
        } else {
            "Click red fields directly on storyboard.pdf (or run `magic-cli storyboard set`) to complete missing fields."
        },
        "sections": sections,
        "requirements": requirements,
        "pages": evaluated,
    }))
}

pub fn cmd_storyboard_wizard(s: &mut Session, p: &Value) -> Result<Value> {
    let path = doc_path(s, p)?;
    if !path.is_file() {
        craft_atomic::atomic_write(&path, PDF_TEMPLATE)?;
    }
    let pages = parse_storyboard_pages(Some(s), &path);
    let find_slot_page = |lbl: &str| pages.iter().find(|pg| pg.slots.iter().any(|sl| !sl.locked && sl.label == lbl)).map(|pg| pg.index);
    let mut batch = Vec::new();
    for (param, slot) in [
        ("title", "cover_title"),
        ("subtitle", "cover_subtitle"),
        ("tagline", "cover_tagline"),
        ("style", "cover_overall_design"),
        ("world", "world_subheading"),
        ("character", "char_exact_prompt"),
        ("motion", "motion_subheading"),
    ] {
        if let (Some(val), Some(pg_idx)) = (p.get(param).and_then(Value::as_str).filter(|v| !v.trim().is_empty()), find_slot_page(slot)) {
            let mut fields = serde_json::Map::new();
            fields.insert(slot.into(), json!(val));
            batch.push(json!({"page": pg_idx, "fields": fields}));
        }
    }
    if !batch.is_empty() {
        cmd_storyboard_set(s, &json!({"path": path, "pages": batch}))?;
    }
    cmd_storyboard_check(s, &json!({"path": path}))
}

fn build_grade_actions(p: &Value) -> Result<Value> {
    if let Some(arr) = p.get("actions").filter(|v| v.is_array()) {
        return Ok(arr.clone());
    }
    let mut actions = Vec::new();
    if p["auto_tone"].as_bool() == Some(true) {
        actions.push(json!({"command": "image.autoTone", "params": {}}));
    }
    if let Some(vib) = p["vibrance"].as_f64() {
        actions
            .push(json!({"command": "layer.newAdjustmentLayer.vibrance", "params": {"vibrance": vib, "saturation": p["saturation"].as_f64().unwrap_or(0.0)}}));
    }
    if let Some(pts) = p["curves"].as_array().filter(|a| !a.is_empty()) {
        actions.push(json!({"command": "layer.newAdjustmentLayer.curves", "params": {"points": pts}}));
    }
    if let Some(lut) = p["lut"].as_str().map(str::trim).filter(|l| !l.is_empty()) {
        actions.push(json!({"command": "layer.newAdjustmentLayer.colorLookup", "params": {"path": lut}}));
    }
    if let Some(amt) = p["sharpen"].as_f64().filter(|&v| v > 0.0) {
        actions.push(json!({"command": "filter.sharpen.smartSharpen", "params": {"amount": amt}}));
    }
    if actions.is_empty() {
        return Err(bad("pass `actions=[...]`, `lut`, `curves`, `vibrance`, `sharpen`, or `auto_tone=true`"));
    }
    Ok(json!(actions))
}

/// `storyboard.grade`: run `photocraft-cli batch --actions <grade.json> --in <raw> --out <graded>`
/// across filled Storyboard image slots (optionally filtered by `section`), updating `storyboard.pdf`
/// while preserving each slot's `linked_file` metadata.
pub fn cmd_storyboard_grade(s: &mut Session, p: &Value) -> Result<Value> {
    let path = doc_path(s, p)?;
    if !path.is_file() {
        return Err(bad("storyboard.pdf does not exist yet; run `storyboard.new` first"));
    }
    let actions = build_grade_actions(p)?;
    let filter_sec = p["section"].as_str().map(str::trim).filter(|sec| !sec.is_empty());
    let pages = parse_storyboard_pages(Some(s), &path);
    let mut state = load_field_state(&path);

    let sb_dir = path.with_file_name("storyboard");
    let raw_dir = sb_dir.join("grade_in");
    let graded_dir = sb_dir.join("graded");
    if raw_dir.is_dir() {
        let _ = std::fs::remove_dir_all(&raw_dir);
    }
    std::fs::create_dir_all(&raw_dir)?;
    std::fs::create_dir_all(&graded_dir)?;

    let mut targets = Vec::new();
    for pg in &pages {
        if filter_sec.is_some_and(|wanted| !pg.section.eq_ignore_ascii_case(wanted)) {
            continue;
        }
        for sl in pg.slots.iter().filter(|sl| !sl.locked && sl.kind == "image" && sl.filled) {
            if let Some(ref src_str) = sl.image {
                let src = Path::new(src_str);
                if src.is_file() {
                    let fkey = field_key(pg.index, &sl.label);
                    let staged = raw_dir.join(format!("{fkey}.png"));
                    if std::fs::copy(src, &staged).is_ok() {
                        targets.push((pg.index, sl.bounds, fkey));
                    }
                }
            }
        }
    }
    if targets.is_empty() {
        return Ok(json!({"ok": true, "graded_count": 0, "path": path}));
    }

    let report = CraftHub::batch_grade_photo_folder(&actions, &raw_dir, &graded_dir)?;
    let mut pdf_steps = Vec::new();
    let mut graded_count = 0usize;
    for (page_idx, bounds, fkey) in targets {
        let out_png = graded_dir.join(format!("{fkey}.png"));
        if out_png.is_file()
            && let Some(meta) = state.images.get_mut(&fkey)
        {
            meta.image = out_png.to_string_lossy().into_owned();
            if meta.craft_app.is_none() {
                meta.craft_app = Some("photocraft".into());
            }
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
            graded_count += 1;
        }
    }
    let _ = std::fs::remove_dir_all(&raw_dir);
    save_field_state(&path, &state)?;
    if !pdf_steps.is_empty() {
        let _ = CraftHub::run_batch(CraftTarget::Pdf, Some(&path), Some(&path), &pdf_steps);
    }
    Ok(json!({
        "ok": true,
        "engine": "PhotoCraft",
        "graded_count": graded_count,
        "path": path,
        "report": report,
    }))
}

/// Rule 3: Storyboard supplies all style prompts & image references for new media creation.
pub fn cmd_storyboard_bulk_generate(s: &mut Session, p: &Value) -> Result<Value> {
    let path = doc_path(s, p)?;
    if !path.is_file() {
        craft_atomic::atomic_write(&path, PDF_TEMPLATE)?;
    }
    let pages = parse_storyboard_pages(Some(s), &path);
    let (style_ctx, sb_refs) = storyboard_media_references_for(s, false);
    let (char_style_ctx, char_refs) = storyboard_media_references_for(s, true);
    let base_style = style_ctx.unwrap_or_default();
    let char_style = char_style_ctx.unwrap_or_else(|| base_style.clone());

    let mut jobs = Vec::new();
    for pg in &pages {
        let is_char_page = pg.section.eq_ignore_ascii_case("Characters");
        let refs_for_page = if is_char_page && !char_refs.is_empty() { &char_refs } else { &sb_refs };
        let style_for_page = if is_char_page { &char_style } else { &base_style };
        for sl in pg.slots.iter().filter(|sl| !sl.locked && sl.kind == "image" && !sl.filled) {
            jobs.push(json!({
                "page": pg.index,
                "section": pg.section,
                "slot": sl.label,
                "remove_background": is_char_page && sl.label.starts_with("char_"),
                "prompt": format!("{style_for_page} | {}", pg.title()).trim_matches(|c: char| c == ' ' || c == '|').trim(),
                "references": refs_for_page,
                "cli_set": sl.cli_command,
            }));
        }
    }
    if p.get("execute").and_then(Value::as_bool) == Some(true) {
        let mut updates = Vec::new();
        for job in &jobs {
            let gen_req = json!({
                "prompt": job["prompt"],
                "references": job["references"],
                "remove_background": job["remove_background"],
                "name": format!("sb-p{:02}-{}", job["page"].as_u64().unwrap_or(0), job["slot"].as_str().unwrap_or("image")),
            });
            if let Ok(res) = crate::craft_ai::cmd_image_generate(s, &gen_req)
                && let Some(img) = res["image"].as_str()
                && let Some(slot_lbl) = job["slot"].as_str()
            {
                updates.push(json!({"page": job["page"], "images": { slot_lbl: img }}));
            }
        }
        if !updates.is_empty() {
            cmd_storyboard_set(s, &json!({"path": path, "pages": updates}))?;
        }
    }
    let sb_dir = path.with_file_name("storyboard");
    std::fs::create_dir_all(&sb_dir)?;
    let plan_path = sb_dir.join("bulk-generation-plan.json");
    let plan_doc = json!({
        "ok": true,
        "path": path,
        "plan_path": plan_path,
        "storyboard_references": sb_refs,
        "job_count": jobs.len(),
        "jobs": jobs,
    });
    craft_atomic::atomic_write(&plan_path, &serde_json::to_vec_pretty(&plan_doc).map_err(fail)?)?;
    Ok(plan_doc)
}
