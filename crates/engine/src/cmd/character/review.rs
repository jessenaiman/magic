//! Project-local, append-only animation revisions stored inside `<slug>.magic` (`"animation_review"`,
//! reading legacy `animation-review.json` on import). Source GLBs and `.ecproj` rigs are never overwritten.

use std::path::PathBuf;

use serde_json::{Value, json};

use super::{bad, mesh_io};
use crate::{Result, Session};

fn legacy_path(s: &Session) -> Result<PathBuf> {
    Ok(crate::workshop::project_dir(s)?.join("animation-review.json"))
}

fn read(s: &mut Session) -> Result<Value> {
    let dir = crate::workshop::project_dir(s)?;
    if let Some(v) = crate::cmd::project::read_session_section(s, &dir, "animation_review", "animation-review.json")? {
        if v["version"] == 1 && v["revisions"].is_array() {
            return Ok(v);
        }
        return Err(bad("Invalid animation_review section"));
    }
    Ok(json!({"version": 1, "revision": 0, "revisions": []}))
}

fn fingerprint(p: &std::path::Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    Ok(format!("{:x}", Sha256::digest(std::fs::read(p)?)))
}

pub fn cmd_review(s: &mut Session, _: &Value) -> Result<Value> {
    let dir = crate::workshop::project_dir(s)?;
    let chars = super::cmd_character_list(s, &json!({}))?;
    let mut models = Vec::new();
    for c in chars["characters"].as_array().into_iter().flatten() {
        let cid = c["id"].as_str().unwrap_or_default();
        let name =
            s.project_assets.iter().find(|a| a.id == cid).map(|a| a.name.clone()).unwrap_or_else(|| c["name"].as_str().unwrap_or(cid).replace(['-', '_'], " "));
        let style = c["animation_style"].as_str().unwrap_or("cutout_2_5d");

        let m = &c["model_3d"];
        if m["available"].as_bool() == Some(true) {
            let file = m["primary_glb"].as_str().unwrap_or_default();
            let mut clip_names: Vec<Value> = m["model_clips"].as_array().cloned().unwrap_or_default();
            for clip_obj in m["clips"].as_array().into_iter().flatten() {
                if let Some(cn) = clip_obj["id"].as_str()
                    && !clip_names.iter().any(|x| x.as_str() == Some(cn))
                {
                    clip_names.push(json!(cn));
                }
            }
            models.push(json!({
                "id": cid,
                "name": name,
                "animation_style": m["animation_style"].as_str().unwrap_or("rig_3d"),
                "path": file,
                "rigged": m["rigged"],
                "clips": clip_names,
                "version": std::fs::metadata(file)
                    .ok()
                    .and_then(|md| md.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_millis().to_string()),
                "qa_status": "needs-visual-review"
            }));
        }

        let sprites = c["sprites"].as_array().cloned().unwrap_or_default();
        if !sprites.is_empty() {
            let clip_names: Vec<Value> = sprites.iter().filter_map(|sp| sp["name"].as_str().map(|n| json!(n))).collect();
            let first_ecproj = sprites.first().and_then(|sp| sp["ecproj"].as_str()).unwrap_or_default();
            models.push(json!({
                "id": cid,
                "name": name,
                "animation_style": style,
                "path": first_ecproj,
                "rigged": true,
                "clips": clip_names,
                "sprites": sprites,
                "qa_status": "needs-visual-review"
            }));
        }
    }
    let review_doc = read(s)?;
    let target_file = s.project_path.clone().unwrap_or_else(|| legacy_path(s).unwrap_or_else(|_| dir.join("project.magic")));
    Ok(json!({
        "ok": true, "project": s.project_path, "directory": dir, "models": models,
        "review": review_doc, "motion_options": motion_options(), "file": target_file,
    }))
}

#[allow(clippy::cognitive_complexity)]
pub fn cmd_save_revision(s: &mut Session, p: &Value) -> Result<Value> {
    let project = s.project_path.clone().ok_or_else(|| bad("Open a project first"))?;
    if p["expected_project"].as_str() != project.to_str() {
        return Err(bad("Project changed; reopen animation review"));
    }
    let mut doc = read(s)?;
    if p["expected_revision"].as_u64() != doc["revision"].as_u64() {
        return Err(bad("Review changed on disk; reload before saving"));
    }
    let id = p["character"]
        .as_str()
        .filter(|v| !v.is_empty() && v.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_')))
        .ok_or_else(|| bad("Invalid character"))?;
    let clip = p["clip"].as_str().ok_or_else(|| bad("clip is required"))?;
    let project_dir = crate::workshop::project_dir(s)?;
    let model = mesh_io::inspect_character_3d(s, &project_dir, id);
    let char_folder = super::character_folder(&project_dir, id);
    let sprites = super::sprite_summaries(&char_folder);
    let source_path = if let Some(sp) = sprites.iter().find(|sp| sp["name"].as_str() == Some(clip)) {
        sp["ecproj"].as_str().ok_or_else(|| bad("Sprite cycle has no .ecproj"))?.to_owned()
    } else if model["model_clips"].as_array().is_some_and(|a| a.contains(&json!(clip)))
        || model["clips"].as_array().is_some_and(|a| a.iter().any(|c| c["id"].as_str() == Some(clip)))
    {
        model["primary_glb"].as_str().ok_or_else(|| bad("Character has no GLB file"))?.to_owned()
    } else {
        return Err(bad("Clip is not in this character's GLB or sprite/EffectCraft cycles"));
    };
    let d = &p["draft"];
    let start = number(d, "trim_start", 0.0, 3600.0)?;
    let end = number(d, "trim_end", 0.0, 3600.0)?;
    if end <= start {
        return Err(bad("Trim end must be after trim start"));
    }
    number(d, "speed", 0.1, 4.0)?;
    for k in ["loop", "in_place"] {
        if !d[k].is_boolean() {
            return Err(bad(format!("{k} must be boolean")));
        }
    }
    if !matches!(d["status"].as_str(), Some("needs-visual-review" | "issues-found" | "approved")) {
        return Err(bad("Invalid review status"));
    }
    let keys = d["keys"].as_array().filter(|a| a.len() <= 10000).ok_or_else(|| bad("keys must be an array of at most 10000 corrections"))?;
    for key in keys {
        if key["bone"].as_str().is_none_or(|b| b.is_empty() || b.len() > 200) {
            return Err(bad("Invalid bone name"));
        }
        number(key, "time", 0.0, 3600.0)?;
        if !key["rotation"].as_array().is_some_and(|v| v.len() == 3 && v.iter().all(|n| n.as_f64().is_some_and(|n| n.is_finite() && n.abs() <= 180.0))) {
            return Err(bad("Correction rotation must have three angles within ±180 degrees"));
        }
    }
    if !d["issues"].as_array().is_some_and(|v| {
        v.len() <= 1000
            && v.iter().all(|i| {
                i["time"].as_f64().is_some_and(|n| n.is_finite() && (0.0..=3600.0).contains(&n))
                    && i["text"].as_str().is_some_and(|t| !t.trim().is_empty() && t.len() <= 4000)
            })
    }) {
        return Err(bad("Invalid issue notes"));
    }
    if d["notes"].as_str().is_none_or(|n| n.len() > 16000) {
        return Err(bad("Notes must be text under 16000 bytes"));
    }
    let revision = doc["revision"].as_u64().unwrap_or(0) + 1;
    let created_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64;
    let entry = json!({
        "id": revision,
        "character": id,
        "clip": clip,
        "created_unix_ms": created_ms,
        "source_sha256": fingerprint(std::path::Path::new(&source_path))?,
        "parent": p["parent"],
        "draft": d,
    });
    doc["revisions"].as_array_mut().ok_or_else(|| bad("Invalid revision list"))?.push(entry.clone());
    doc["revision"] = json!(revision);
    let out = crate::cmd::project::write_session_section(s, &project_dir, "animation_review", &doc)?;
    Ok(json!({"ok": true, "file": out, "revision": revision, "entry": entry}))
}

fn number(p: &Value, key: &str, min: f64, max: f64) -> Result<f64> {
    p[key].as_f64().filter(|v| v.is_finite() && *v >= min && *v <= max).ok_or_else(|| bad(format!("{key} must be between {min} and {max}")))
}

fn motion_options() -> Value {
    json!([
        {
            "id": "baseline",
            "label": "Keep baseline motion",
            "description": "Use the selected GLB clip unchanged in EffectCraft.",
            "load": "One character. Project preparation only; no rendering.",
        },
        {
            "id": "paced",
            "label": "Gentler pacing",
            "description": "Play the existing clip at 85% speed; does not change skin weights or collision poses.",
            "load": "One character. Project preparation only; no rendering.",
        },
    ])
}

pub fn cmd_motion_options(_: &mut Session, _: &Value) -> Result<Value> {
    Ok(json!({
        "ok": true, "options": motion_options(), "scope": "single-subject",
        "render": "explicit separate command", "baseline": "source GLBs are preserved",
    }))
}

pub fn cmd_prepare_motion(s: &mut Session, p: &Value) -> Result<Value> {
    let dir = crate::workshop::project_dir(s)?;
    let character = p["character"]
        .as_str()
        .filter(|v| !v.is_empty() && v.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_')))
        .ok_or_else(|| bad("Choose one character"))?;
    let clip = p["clip"].as_str().ok_or_else(|| bad("Choose one animation"))?;
    let option = p["option"].as_str().unwrap_or("baseline");
    if !matches!(option, "baseline" | "paced") {
        return Err(bad("Unknown motion option"));
    }
    let model = mesh_io::inspect_character_3d(s, &dir, character);
    let source = model["primary_glb"].as_str().ok_or_else(|| bad("Character needs a GLB"))?;
    let clips = model["model_clips"].as_array().ok_or_else(|| bad("Character has no animation clips"))?;
    let index = clips.iter().position(|v| v.as_str() == Some(clip)).ok_or_else(|| bad("This character does not have that clip"))?;
    let sha = fingerprint(std::path::Path::new(source))?;
    let safe_clip = crate::workspace::sanitize_asset_id(clip);
    let duration = p["duration"].as_f64().unwrap_or(3.0).clamp(0.1, 30.0);
    let folder = dir.join("animation-options").join(character).join(format!("{}-{}-{}-{:.3}s-v2", safe_clip, option, &sha[..10], duration));
    std::fs::create_dir_all(&folder)?;
    let project = folder.join("motion.ecproj");
    if project.exists() {
        return Ok(json!({"ok": true, "reused": true, "project": project, "option": option, "source_sha256": sha, "rendered": false}));
    }
    let comp_name = format!("{character} / {clip} / {option}");
    let speed = if option == "paced" { 85 } else { 100 };
    let spec = mesh_io::GlbStageSpec {
        comp_name: &comp_name,
        glb_path: std::path::Path::new(source),
        clip_index: index,
        duration,
        speed_pct: speed,
        out_ecproj: &project,
    };
    let result = mesh_io::stage_glb_in_effectcraft(&dir, &spec)?;
    Ok(json!({"ok": true, "project": project, "option": option, "source_sha256": sha, "rendered": false, "effectcraft": result}))
}
