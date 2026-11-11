//! Rust-computed UI view-models (`project.snapshot` with `drawer`, `footer`, and `storyboard`
//! traffic-light state) and theatre timeline persistence, keeping domain logic out of `.tsx` views.

use serde_json::{Value, json};

use super::assets::cmd_project_list_assets;
use super::read_project_document;
use crate::{EngineError, Result, Session};

fn style_badge(style: &str) -> &'static str {
    match style {
        "sprite_2d" => "2D Sprite",
        "cutout_2_5d" => "2.5D Rig",
        "rig_3d" => "3D Rig",
        _ => "Cutout",
    }
}

fn kind_badge(kind: &str) -> &'static str {
    match kind {
        "cutout_rig" => "2.5D Rig",
        "poses" => "2D Sprite",
        "turnaround" => "Turnaround",
        "model_3d" | "anim_3d" => "3D Rig",
        _ => "Cutout",
    }
}

fn build_drawer_model(assets_list: &[Value], characters: &[Value]) -> Value {
    let mut character_cards = Vec::new();
    let mut covered_asset_ids = std::collections::HashSet::new();

    for c in characters {
        let cid = c["id"].as_str().unwrap_or_default();
        if cid.is_empty() {
            continue;
        }
        let name = c["name"].as_str().unwrap_or(cid);
        let style = c["animation_style"].as_str().unwrap_or("cutout");
        let views = c["views"].as_array().cloned().unwrap_or_default();
        let sprites = c["sprites"].as_array().cloned().unwrap_or_default();
        let thumb = views
            .iter()
            .find_map(|v| v["drawing"].as_str())
            .or_else(|| sprites.iter().find_map(|sp| sp["frames"].as_array().and_then(|f| f.first()).and_then(Value::as_str)))
            .or_else(|| {
                assets_list
                    .iter()
                    .find(|a| a["role"] == "character" && (a["id"].as_str() == Some(cid) || a["name"].as_str() == Some(name)))
                    .and_then(|a| a["path"].as_str())
            });
        let matched_asset_id = assets_list
            .iter()
            .find(|a| a["role"] == "character" && (a["id"].as_str() == Some(cid) || a["name"].as_str() == Some(name)))
            .and_then(|a| a["id"].as_str());
        if let Some(aid) = matched_asset_id {
            covered_asset_ids.insert(aid.to_owned());
        }
        let source_project = views.iter().find_map(|v| v["ecproj"].as_str()).or_else(|| sprites.iter().find_map(|sp| sp["ecproj"].as_str()));
        let ready = views.iter().any(|v| v["ecproj"] == true) || !sprites.is_empty() || c["model_3d"]["available"] == true;
        character_cards.push(json!({
            "id": format!("char:{cid}"),
            "char_id": cid,
            "asset_id": matched_asset_id,
            "name": name,
            "role": "character",
            "badge": style_badge(style),
            "thumb": thumb,
            "source_project": source_project,
            "ready": ready,
        }));
    }

    for a in assets_list.iter().filter(|a| a["role"] != "background") {
        let Some(aid) = a["id"].as_str() else { continue };
        if covered_asset_ids.contains(aid) {
            continue;
        }
        character_cards.push(json!({
            "id": aid,
            "char_id": aid,
            "asset_id": aid,
            "name": a["name"].as_str().unwrap_or(aid),
            "role": "character",
            "badge": kind_badge(a["kind"].as_str().unwrap_or("image")),
            "thumb": a["path"],
            "ready": false,
        }));
    }

    let background_cards: Vec<Value> = assets_list
        .iter()
        .filter(|a| a["role"] == "background")
        .filter_map(|a| {
            let aid = a["id"].as_str()?;
            Some(json!({
                "id": aid,
                "asset_id": aid,
                "name": a["name"].as_str().unwrap_or(aid),
                "role": "background",
                "badge": "Plate",
                "thumb": a["path"],
                "ready": true,
            }))
        })
        .collect();

    let total = character_cards.len() + background_cards.len();
    json!({
        "total": total,
        "characters": character_cards,
        "backgrounds": background_cards,
    })
}

fn build_footer_model(status: &Value, storyboard: &Value, characters_len: usize) -> Value {
    let stage_done = |id: &str| status["stages"].as_array().into_iter().flatten().any(|st| st["id"] == id && st["done"] == true);
    let song_done = stage_done("song");
    let transcript_done = stage_done("transcript");
    let sb_light = storyboard["light"].as_str().unwrap_or("red");
    let sb_done = storyboard["production_ready"] == true;
    let lyrics_done = status["lyrics_ready"] == true;
    let motion_done = status["motion_ready"] == true;
    let animate_done = characters_len > 0;
    let studio_done = status["video_project_exists"] == true;

    let mut summary = Vec::new();
    if status["timing_reference_exists"] == true {
        summary.push("Timing Ref ✓".to_string());
    }
    if storyboard["title_sections_complete"] == true {
        summary.push("Titles Green".to_string());
    }
    if characters_len > 0 {
        summary.push(format!("{characters_len} Cast"));
    }

    json!({
        "stages": [
            {"id": "song", "label": "Song", "view": "project", "done": song_done, "light": if song_done && transcript_done { "green" } else if song_done { "amber" } else { "red" }},
            {"id": "storyboard", "label": "Board", "view": "storyboard", "done": sb_done, "light": sb_light},
            {"id": "lyrics", "label": "Lyrics", "view": "design", "done": lyrics_done, "light": if lyrics_done { "green" } else { "red" }},
            {"id": "motion", "label": "Motion", "view": "motion", "done": motion_done, "light": if motion_done { "green" } else { "red" }},
            {"id": "animation", "label": "Animate", "view": "animation", "done": animate_done, "light": if animate_done { "green" } else { "red" }},
            {"id": "studio", "label": "Studio", "view": "studio", "done": studio_done, "light": if studio_done { "green" } else { "red" }},
        ],
        "production_ready": sb_done,
        "summary": summary.join(" · "),
    })
}

/// `project.snapshot`: everything a view needs to draw the open project in one reply.
pub fn cmd_project_snapshot(s: &mut Session, _: &Value) -> Result<Value> {
    let Some(project) = s.project_path.clone() else {
        return Ok(json!({"project": null, "data": null}));
    };
    let status = crate::cmd::project::workshop::workshop_status(s)?;
    let storyboard = crate::cmd::storyboard::cmd_storyboard_check(s, &json!({})).unwrap_or_else(|_| json!({"light": "red", "production_ready": false}));
    let assets = cmd_project_list_assets(s, &json!({}))?;
    let trunk = s.execute("project.trunk", json!({})).unwrap_or_else(|error| json!({"files": [], "error": error.to_string()}));
    let characters_arr =
        s.execute("character.list", json!({})).ok().and_then(|res| res.get("characters").and_then(Value::as_array).cloned()).unwrap_or_default();
    let assets_arr = assets.get("assets").and_then(Value::as_array).cloned().unwrap_or_default();
    let drawer = build_drawer_model(&assets_arr, &characters_arr);
    let footer = build_footer_model(&status, &storyboard, characters_arr.len());
    Ok(json!({
        "project": project,
        "data": {
            "project": project,
            "status": status,
            "storyboard": storyboard,
            "assets": assets,
            "trunk": trunk,
            "characters": characters_arr,
            "drawer": drawer,
            "footer": footer,
        }
    }))
}

pub fn theatre_state(s: &mut Session, params: &Value) -> Result<Value> {
    let path = s.project_path.as_ref().ok_or_else(|| EngineError::InvalidParams("Open a project".into()))?;
    if params["expected_project"].as_str() != path.to_str() {
        return Err(EngineError::InvalidParams("Project changed".into()));
    }
    let doc = read_project_document(path)?;
    Ok(json!({"state": doc["theatre"], "status": if doc["theatre"].is_object() { "saved" } else { "empty" }}))
}

pub fn save_theatre_state(s: &mut Session, params: &Value) -> Result<Value> {
    theatre_state(s, params)?;
    if !params["state"]["sheetsById"].is_object() {
        return Err(EngineError::InvalidParams("Invalid Theatre snapshot".into()));
    }
    let path = s.project_path.as_ref().ok_or_else(|| EngineError::InvalidParams("Open a project".into()))?;
    if s.project_stamp.get().is_some() {
        crate::link_watch::save_guard(path, s.project_stamp.get())?;
    }
    let mut doc = read_project_document(path)?;
    doc["theatre"] = params["state"].clone();
    let bytes = serde_json::to_vec_pretty(&doc).map_err(|e| EngineError::InvalidParams(e.to_string()))?;
    craft_atomic::atomic_write(path, &bytes)?;
    s.project_stamp.set(crate::link_watch::FileStamp::of(path));
    Ok(json!({"saved": true, "project": path}))
}
