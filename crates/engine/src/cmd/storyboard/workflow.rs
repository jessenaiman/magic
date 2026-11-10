//! Storyboard workflow: traffic-light section readiness (`storyboard.check`), optional guided
//! setup wizard (`storyboard.wizard`), and bulk reference-driven image generation (`storyboard.bulk_generate`).
//!
//! Workflow order:
//! 1. Complete Main Section Title Pages first (`A-Cover`, `B-World`, `C-Character`, `D-Motion`).
//! 2. Generate bulk images using the references defined on those main section title pages.
//! 3. Once all required Storyboard sections are green, proceed to video production.

use std::path::PathBuf;

use serde_json::{Value, json};

use super::crud::cmd_storyboard_set;
use super::{MAIN_TITLE_ARCHETYPES, PageInfo, Slot, bad, doc_path, fail, page_cli_command, parse_pages_and_stories, read_designcraft_doc};
use crate::{Result, Session};

fn primary_image_slot_for(archetype: &str) -> &'static str {
    match archetype {
        "cover" => "cover_key_art",
        "world" => "world_stage_image",
        "char" => "char_original_image",
        "motion" => "motion_stage_image",
        _ => "hero_image",
    }
}

fn slot_has_placeholder(slot: &Slot) -> bool {
    if slot.hidden || slot.locked || slot.is_cli_helper() {
        return false;
    }
    if slot.label.starts_with("ref_") || slot.label == "contact_strip" || slot.label.starts_with("cover_reg_") || slot.label.starts_with("cover_songmap_") {
        return false;
    }
    match slot.kind {
        "text" => slot.text.as_deref().is_none_or(|t| {
            let trimmed = t.trim();
            trimmed.is_empty() || trimmed.contains('[') && trimmed.contains(']')
        }),
        "image" => {
            slot.label == primary_image_slot_for_slot(&slot.label)
                && slot.image.as_deref().is_none_or(|img| img.trim().is_empty() || !std::path::Path::new(img).is_file())
        }
        _ => false,
    }
}

fn primary_image_slot_for_slot(label: &str) -> &str {
    match label {
        "cover_key_art" | "world_stage_image" | "char_original_image" | "motion_stage_image" | "hero_image" => label,
        _ => "",
    }
}

fn evaluate_page_light(page: &PageInfo) -> Value {
    let mut unfilled = Vec::new();
    let mut filled_count = 0usize;
    let mut total_checked = 0usize;

    for slot in &page.slots {
        if slot.hidden || slot.locked || slot.is_cli_helper() {
            continue;
        }
        if slot.label.starts_with("ref_") || slot.label == "contact_strip" || slot.label.starts_with("cover_reg_") || slot.label.starts_with("cover_songmap_") {
            continue;
        }
        let is_primary_img = slot.kind == "image" && slot.label == primary_image_slot_for(&page.archetype);
        if slot.kind != "text" && !is_primary_img {
            continue;
        }
        total_checked += 1;
        if slot_has_placeholder(slot) {
            unfilled.push(json!({
                "label": slot.label,
                "kind": slot.kind,
                "text": slot.text,
                "cli_command": slot.cli_command,
            }));
        } else {
            filled_count += 1;
        }
    }

    let (light, status) = if unfilled.is_empty() && total_checked > 0 {
        ("green", "complete")
    } else if filled_count > 0 {
        ("amber", "partial")
    } else {
        ("red", "incomplete")
    };

    json!({
        "page": page.index,
        "title": page.title(),
        "archetype": page.archetype,
        "section": page.section,
        "is_title_section": page.is_main_title_page(),
        "light": light,
        "status": status,
        "filled_count": filled_count,
        "total_required": total_checked,
        "missing_count": unfilled.len(),
        "unfilled": unfilled,
        "cli_command": page_cli_command(page.index, &page.archetype),
    })
}

/// `storyboard.check`: Traffic-light section readiness (`red` / `amber` / `green`) across Main
/// Section Title Pages (`A-Cover`, `B-World`, `C-Character`, `D-Motion`) and Scene spreads (`S01..`).
pub fn cmd_storyboard_check(s: &mut Session, p: &Value) -> Result<Value> {
    let path = doc_path(s, p)?;
    if !path.is_file() {
        return Ok(json!({
            "ok": true,
            "exists": false,
            "path": path,
            "light": "red",
            "status": "missing",
            "title_sections_complete": false,
            "scenes_complete": false,
            "production_ready": false,
            "pages": [],
            "title_sections": [],
            "scene_sections": [],
            "next_step": "Run `magic-cli storyboard new` to create storyboard.designcraft.",
        }));
    }
    let doc = read_designcraft_doc(&path)?;
    let (pages, _) = parse_pages_and_stories(&path, &doc);
    let evaluated: Vec<Value> = pages.iter().map(evaluate_page_light).collect();

    let title_sections: Vec<Value> = evaluated.iter().filter(|pg| pg["is_title_section"] == true).cloned().collect();
    let scene_sections: Vec<Value> = evaluated.iter().filter(|pg| pg["is_title_section"] != true).cloned().collect();

    let has_all_title_archetypes = MAIN_TITLE_ARCHETYPES.iter().all(|arch| title_sections.iter().any(|pg| pg["archetype"] == *arch));
    let title_sections_complete = has_all_title_archetypes && !title_sections.is_empty() && title_sections.iter().all(|pg| pg["light"] == "green");
    let scenes_complete = !scene_sections.is_empty() && scene_sections.iter().all(|pg| pg["light"] == "green");
    let production_ready = title_sections_complete && scenes_complete;

    let missing_total: usize = evaluated.iter().filter_map(|pg| pg["missing_count"].as_u64()).sum::<u64>() as usize;
    let any_progress = evaluated.iter().any(|pg| pg["filled_count"].as_u64().unwrap_or(0) > 0);
    let overall_light = if production_ready {
        "green"
    } else if any_progress || title_sections_complete {
        "amber"
    } else {
        "red"
    };

    let next_step = if !title_sections_complete {
        "Step 3: Complete Main Section Title Pages first (A-Cover, B-World, C-Character, D-Motion)."
    } else if !scenes_complete {
        "Step 4: Run `magic-cli storyboard bulk_generate` and complete remaining Scene spreads."
    } else {
        "Step 5: All required Storyboard sections are Green — ready for Video Production (Lyrics, Motion, Animation, Studio)."
    };

    Ok(json!({
        "ok": true,
        "exists": true,
        "path": path,
        "light": overall_light,
        "complete": production_ready,
        "status": if production_ready { "complete" } else { "incomplete" },
        "count": missing_total,
        "page_count": pages.len(),
        "title_sections_complete": title_sections_complete,
        "scenes_complete": scenes_complete,
        "production_ready": production_ready,
        "next_step": next_step,
        "title_sections": title_sections,
        "scene_sections": scene_sections,
        "pages": evaluated,
    }))
}

/// `storyboard.wizard`: Guided setup for Main Section Title Pages (`A-Cover`, `B-World`, `C-Character`, `D-Motion`).
pub fn cmd_storyboard_wizard(s: &mut Session, p: &Value) -> Result<Value> {
    let path = doc_path(s, p)?;
    if !path.is_file() {
        super::crud::cmd_storyboard_new(s, &json!({"path": path, "overwrite": false}))?;
    }
    let doc = read_designcraft_doc(&path)?;
    let (pages, _) = parse_pages_and_stories(&path, &doc);

    let find_arch = |arch: &str| pages.iter().find(|pg| pg.archetype == arch).map(|pg| pg.index);
    let mut batch = Vec::new();

    if let Some(idx) = find_arch("cover") {
        let mut fields = serde_json::Map::new();
        if let Some(v) = p.get("title").and_then(Value::as_str).filter(|v| !v.trim().is_empty()) {
            fields.insert("cover_title".into(), json!(v));
        }
        if let Some(v) = p.get("subtitle").and_then(Value::as_str).filter(|v| !v.trim().is_empty()) {
            fields.insert("cover_subtitle".into(), json!(v));
        }
        if let Some(v) = p.get("tagline").and_then(Value::as_str).filter(|v| !v.trim().is_empty()) {
            fields.insert("cover_tagline".into(), json!(v));
        }
        if let Some(v) = p.get("style").or_else(|| p.get("overall_design")).and_then(Value::as_str).filter(|v| !v.trim().is_empty()) {
            fields.insert("cover_overall_design".into(), json!(v));
        }
        if !fields.is_empty() {
            batch.push(json!({"page": idx, "fields": fields}));
        }
    }
    if let Some(idx) = find_arch("world") {
        let mut fields = serde_json::Map::new();
        if let Some(v) = p.get("world").or_else(|| p.get("world_subheading")).and_then(Value::as_str).filter(|v| !v.trim().is_empty()) {
            fields.insert("world_subheading".into(), json!(v));
        }
        if let Some(v) = p.get("world_prompt").and_then(Value::as_str).filter(|v| !v.trim().is_empty()) {
            fields.insert("world_location_prompt".into(), json!(v));
        }
        if !fields.is_empty() {
            batch.push(json!({"page": idx, "fields": fields}));
        }
    }
    if let Some(idx) = find_arch("char") {
        let mut fields = serde_json::Map::new();
        if let Some(v) = p.get("character").or_else(|| p.get("char_prompt")).and_then(Value::as_str).filter(|v| !v.trim().is_empty()) {
            fields.insert("char_exact_prompt".into(), json!(v));
        }
        if !fields.is_empty() {
            batch.push(json!({"page": idx, "fields": fields}));
        }
    }
    if let Some(idx) = find_arch("motion") {
        let mut fields = serde_json::Map::new();
        if let Some(v) = p.get("motion").or_else(|| p.get("motion_spec")).and_then(Value::as_str).filter(|v| !v.trim().is_empty()) {
            fields.insert("motion_subheading".into(), json!(v));
            fields.insert("motion_drawn_spec".into(), json!(v));
        }
        if !fields.is_empty() {
            batch.push(json!({"page": idx, "fields": fields}));
        }
    }

    if !batch.is_empty() {
        cmd_storyboard_set(s, &json!({"path": path, "pages": batch}))?;
    }

    let check = cmd_storyboard_check(s, &json!({"path": path}))?;
    let steps = json!([
        {
            "step": 1,
            "section": "A-Cover",
            "goal": "Set project title, format subtitle, logline, and D01 overall visual style.",
            "cli": "magic-cli storyboard set page=0 fields='{\"cover_title\":\"...\",\"cover_overall_design\":\"...\"}'"
        },
        {
            "step": 2,
            "section": "B-World",
            "goal": "Define L01 stage location prompt, material scale, and master world plate.",
            "cli": "magic-cli storyboard set page=1 fields='{\"world_subheading\":\"...\",\"world_location_prompt\":\"...\"}'"
        },
        {
            "step": 3,
            "section": "C-Character",
            "goal": "Define C01 lead character exact prompt, silhouette rules, and reference drawing.",
            "cli": "magic-cli storyboard set page=2 fields='{\"char_exact_prompt\":\"...\"}'"
        },
        {
            "step": 4,
            "section": "D-Motion",
            "goal": "Define M01 song-action vector drawing cues and kinetic lettering direction.",
            "cli": "magic-cli storyboard set page=3 fields='{\"motion_subheading\":\"...\",\"motion_drawn_spec\":\"...\"}'"
        },
        {
            "step": 5,
            "section": "Bulk Generation & Scenes",
            "goal": "Generate scene plates in bulk from title-page references, then start video production.",
            "cli": "magic-cli storyboard bulk_generate"
        }
    ]);

    Ok(json!({
        "ok": true,
        "path": path,
        "applied_updates": batch.len(),
        "wizard_steps": steps,
        "readiness": check,
    }))
}

fn slot_text<'a>(pg: &'a PageInfo, label: &str) -> &'a str {
    pg.slots.iter().find(|s| s.label == label).and_then(|s| s.text.as_deref()).map(str::trim).unwrap_or("")
}

fn slot_img(pg: &PageInfo, label: &str) -> Option<String> {
    pg.slots.iter().find(|s| s.label == label).and_then(|s| s.image.clone()).filter(|p| PathBuf::from(p).is_file())
}

/// `storyboard.bulk_generate`: Workflow Step 4 — requires Main Section Title Pages (`A-Cover`,
/// `B-World`, `C-Character`, `D-Motion`) to be completed first (unless `force=true`), gathers their
/// locked visual prompts & reference plates, builds `storyboard/bulk-generation-plan.json`, and
/// optionally executes batch image generation + flat-image placement across scene pages.
// TODO: Auto-promote generated `C-Character` and `D-Motion` title-page cutouts directly into
// `character.rig` (`EffectCraft`) and `motion.save` (`VectorCraft` -> `EffectCraft`) once approved.
pub fn cmd_storyboard_bulk_generate(s: &mut Session, p: &Value) -> Result<Value> {
    let path = doc_path(s, p)?;
    let check = cmd_storyboard_check(s, &json!({"path": path}))?;
    let force = p.get("force").and_then(Value::as_bool).unwrap_or(false);
    if check["title_sections_complete"] != true && !force {
        return Err(bad(
            "Complete the Main Section Title Pages (A-Cover, B-World, C-Character, D-Motion) before running bulk generation (inspect `magic-cli storyboard check`, or pass `force=true` to preview the plan).",
        ));
    }

    let doc = read_designcraft_doc(&path)?;
    let (pages, _) = parse_pages_and_stories(&path, &doc);

    let cover = pages.iter().find(|pg| pg.archetype == "cover");
    let world = pages.iter().find(|pg| pg.archetype == "world");
    let char_pg = pages.iter().find(|pg| pg.archetype == "char");
    let motion_pg = pages.iter().find(|pg| pg.archetype == "motion");

    let core_style = cover.map(|pg| slot_text(pg, "cover_overall_design")).unwrap_or_default();
    let world_prompt = world.map(|pg| slot_text(pg, "world_location_prompt")).unwrap_or_default();
    let char_prompt = char_pg.map(|pg| slot_text(pg, "char_exact_prompt")).unwrap_or_default();
    let motion_spec = motion_pg.map(|pg| slot_text(pg, "motion_drawn_spec")).unwrap_or_default();

    let mut title_refs: Vec<String> = Vec::new();
    for img in [
        cover.and_then(|pg| slot_img(pg, "cover_key_art")),
        world.and_then(|pg| slot_img(pg, "world_stage_image")),
        char_pg.and_then(|pg| slot_img(pg, "char_original_image")),
        char_pg.and_then(|pg| slot_img(pg, "char_pose_image")),
        motion_pg.and_then(|pg| slot_img(pg, "motion_stage_image")),
    ]
    .into_iter()
    .flatten()
    {
        if !title_refs.contains(&img) && title_refs.len() < 5 {
            title_refs.push(img);
        }
    }

    let mut jobs = Vec::new();
    for pg in &pages {
        let slot = primary_image_slot_for(&pg.archetype);
        let existing = slot_img(pg, slot);
        if existing.is_some() && !p.get("overwrite").and_then(Value::as_bool).unwrap_or(false) {
            continue;
        }
        let page_prompt = match pg.archetype.as_str() {
            "cover" => format!("{core_style} | {world_prompt}"),
            "world" => format!("{core_style} | {world_prompt}"),
            "char" => format!("{core_style} | {char_prompt}"),
            "motion" => format!("{core_style} | {motion_spec}"),
            _ => {
                let exact = slot_text(pg, "exact_prompt");
                let design = slot_text(pg, "overall_design");
                if !exact.is_empty() && !exact.starts_with('[') {
                    format!("{core_style} | {exact}")
                } else {
                    format!("{core_style} | {world_prompt} | {design}")
                }
            }
        };
        jobs.push(json!({
            "page": pg.index,
            "archetype": pg.archetype,
            "title": pg.title(),
            "slot": slot,
            "prompt": page_prompt.trim_matches(|c: char| c == ' ' || c == '|').trim(),
            "references": title_refs,
            "cli_set": format!("magic-cli storyboard set page={} label={slot} image=\"<generated.png>\"", pg.index),
        }));
    }

    let sb_dir = path.with_file_name("storyboard");
    std::fs::create_dir_all(&sb_dir)?;
    let plan_path = sb_dir.join("bulk-generation-plan.json");
    let mut generated_receipts = Vec::new();
    let run_live = p.get("execute").and_then(Value::as_bool).unwrap_or(false);

    if run_live {
        let mut batch_updates = Vec::new();
        for job in &jobs {
            let mut gen_req = json!({
                "prompt": job["prompt"],
                "references": job["references"],
                "transparent_background": job["archetype"] == "char",
                "name": format!("sb-p{:02}-{}", job["page"].as_u64().unwrap_or(0), job["slot"].as_str().unwrap_or("image")),
            });
            if let Some(prov) = p.get("provider") {
                gen_req["provider"] = prov.clone();
            }
            if let Some(model) = p.get("model") {
                gen_req["model"] = model.clone();
            }
            if let Ok(res) = crate::image_generation::generate(s, &gen_req)
                && let Some(img) = res["image"].as_str()
            {
                let slot_name = job["slot"].as_str().unwrap_or("hero_image");
                batch_updates.push(json!({
                    "page": job["page"],
                    "images": { slot_name: img }
                }));
                generated_receipts.push(json!({"page": job["page"], "slot": slot_name, "image": img}));
            }
        }
        if !batch_updates.is_empty() {
            cmd_storyboard_set(s, &json!({"path": path, "pages": batch_updates}))?;
        }
    }

    let plan_doc = json!({
        "ok": true,
        "path": path,
        "plan_path": plan_path,
        "title_references": title_refs,
        "job_count": jobs.len(),
        "jobs": jobs,
        "executed": run_live,
        "generated": generated_receipts,
    });
    craft_atomic::atomic_write(&plan_path, &serde_json::to_vec_pretty(&plan_doc).map_err(fail)?)?;
    Ok(plan_doc)
}
