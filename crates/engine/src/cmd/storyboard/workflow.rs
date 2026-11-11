//! Storyboard readiness (`storyboard.check`), setup helper (`storyboard.wizard`), and bulk
//! reference-driven media generation (`storyboard.bulk_generate`) governed strictly by the
//! `.designcraft` master (`doc["sections"]` + `!slot.locked`).

use serde_json::{Value, json};

use super::crud::cmd_storyboard_set;
use super::{PDF_TEMPLATE, PageInfo, doc_path, fail, human_slot_title, master_sections, parse_storyboard_pages, storyboard_media_references};
use crate::{Result, Session};

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

/// Rule 3: Storyboard supplies all style prompts & image references for new media creation.
pub fn cmd_storyboard_bulk_generate(s: &mut Session, p: &Value) -> Result<Value> {
    let path = doc_path(s, p)?;
    if !path.is_file() {
        craft_atomic::atomic_write(&path, PDF_TEMPLATE)?;
    }
    let pages = parse_storyboard_pages(Some(s), &path);
    let (style_ctx, sb_refs) = storyboard_media_references(s);
    let base_style = style_ctx.unwrap_or_default();
    let mut jobs = Vec::new();
    for pg in &pages {
        for sl in pg.slots.iter().filter(|sl| !sl.locked && sl.kind == "image" && !sl.filled) {
            jobs.push(json!({
                "page": pg.index,
                "section": pg.section,
                "slot": sl.label,
                "prompt": format!("{base_style} | {}", pg.title()).trim_matches(|c: char| c == ' ' || c == '|').trim(),
                "references": sb_refs,
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
