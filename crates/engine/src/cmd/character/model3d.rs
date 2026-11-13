//! `character.model3d.*`: Magic Studio 3D character breakout, rig precheck, and animation pipeline
//! powered by [`craft_mesh`] (via [`super::mesh_io`]) across `02 Animations` and `05 Studio`
//! (`EffectCraft` `advanced3D` `layer.newModel`).

use std::path::{Path, PathBuf};

use craft_mesh::Limits as MeshLimits;
use serde_json::{Value, json};

use super::bad;
pub use super::mesh_io::{cmd_mesh_convert, cmd_mesh_inspect};
use super::mesh_io::{
    collect_files_with_ext, collect_unique_ext, has_matching_ext, read_craft_mesh_scene, read_json_optional, summarize_mesh_scene, write_json_pretty,
};
use crate::{Result, Session};

const MODEL3D_CATALOG: &str = include_str!("model3d_catalog.json");

pub(crate) fn catalog_json() -> Result<Value> {
    serde_json::from_str(MODEL3D_CATALOG).map_err(|e| bad(format!("model3d_catalog.json: {e}")))
}

fn character_id(p: &Value) -> Result<String> {
    let raw = p["character"].as_str().ok_or_else(|| bad("character=<id> is required"))?;
    let id = crate::workspace::sanitize_asset_id(raw);
    if id.is_empty() { Err(bad("character=<id> is required")) } else { Ok(id) }
}

/// Canonical 3D directory for a character: `<project>/assets/characters/<id>/3d/`.
pub(crate) fn character_3d_dir(project_dir: &Path, id: &str) -> PathBuf {
    project_dir.join("assets/characters").join(id).join("3d")
}

fn candidate_3d_dirs(project_dir: &Path, id: &str) -> Vec<PathBuf> {
    vec![
        character_3d_dir(project_dir, id),
        project_dir.join("assets").join(id).join("3d"),
        project_dir.join("3d").join(format!("{id}-actor")),
        project_dir.join("3d").join(id),
    ]
}

fn read_manifest(dir: &Path) -> Option<Value> {
    read_json_optional(&dir.join("model3d.json"))
}

fn anim_clip_entry(path: &Path, doc: &Value) -> Value {
    let default_stem = path.file_name().and_then(|n| n.to_str()).unwrap_or("clip").trim_end_matches(".anim.json");
    json!({
        "id": doc["id"].as_str().unwrap_or(default_stem),
        "name": doc["name"].as_str().unwrap_or(default_stem),
        "source": doc["source"],
        "category": doc["category"],
        "duration": doc["duration"],
        "in_place": doc["in_place"],
        "fbx": doc["fbx"],
        "glb": doc["glb"],
        "ecproj_3d": doc["ecproj_3d"],
        "preview": doc["preview"],
        "stages": doc["stages"],
        "file": path,
    })
}

fn scan_json_metadata(dirs: &[PathBuf]) -> (Vec<Value>, Value, Value, Value) {
    let mut clips = Vec::new();
    let mut import_meta = Value::Null;
    let mut repair_meta = Value::Null;
    let mut multistage_meta = Value::Null;
    for p in collect_unique_ext(dirs, &["json"], 3) {
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        let Some(doc) = read_json_optional(&p) else {
            continue;
        };
        if name.ends_with(".anim.json") {
            clips.push(anim_clip_entry(&p, &doc));
        } else if name == "motion-library-import.json" && import_meta.is_null() {
            import_meta = doc;
        } else if name == "rigid-case-repair.json" && repair_meta.is_null() {
            repair_meta = doc;
        } else if name == "multistage-motion-request.json" && multistage_meta.is_null() {
            multistage_meta = doc;
        }
    }
    (clips, import_meta, repair_meta, multistage_meta)
}

fn append_imported_actions(clips: &mut Vec<Value>, import_meta: &Value, fbx: Option<&PathBuf>, glb: Option<&PathBuf>, preview: Option<&PathBuf>) {
    let Some(actions) = import_meta["actions"].as_array() else {
        return;
    };
    let fps = import_meta["fps"].as_f64().unwrap_or(24.0).max(1.0);
    for (idx, act) in actions.iter().enumerate() {
        let clip_id = format!("imported_action_{}", idx + 1);
        if clips.iter().any(|c| c["id"] == clip_id) {
            continue;
        }
        let start = act["range"][0].as_f64().unwrap_or(1.0);
        let end = act["range"][1].as_f64().unwrap_or(48.0);
        clips.push(json!({
            "id": clip_id,
            "name": act["name"].as_str().unwrap_or("action"),
            "source": "motion_library",
            "category": "interactive",
            "duration": ((end - start + 1.0) / fps * 100.0).round() / 100.0,
            "in_place": true,
            "fbx": fbx,
            "glb": glb,
            "preview": preview,
        }));
    }
}

fn pick_primary_mesh(manifest: &Value, mesh_files: &[PathBuf]) -> Option<PathBuf> {
    manifest["base_model_glb"]
        .as_str()
        .map(PathBuf::from)
        .filter(|p| p.is_file())
        .or_else(|| {
            mesh_files.iter().find(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.contains("rigid-case") || n.contains("rigged"))).cloned()
        })
        .or_else(|| mesh_files.first().cloned())
}

fn registered_model(s: &Session, id: &str) -> Option<PathBuf> {
    let id = id.replace('-', "_");
    s.project_assets.iter().filter(|a| a.kind == "model_3d" && a.id.replace('-', "_") == id).find_map(|a| a.path.clone().filter(|p| p.is_file()))
}

/// Inspect 3D `.glb`/`.gltf`/`.fbx`/`.obj`/`.stl`/`.ply`/`.3mf` models via `craft-mesh`,
/// TextureCraft `.tcraft` materials, and split `.anim.json` clips for one character.
pub(crate) fn inspect_character_3d(s: &Session, project_dir: &Path, id: &str) -> Value {
    let canonical = character_3d_dir(project_dir, id);
    let mut dirs = candidate_3d_dirs(project_dir, id);
    let registered = registered_model(s, id);
    if let Some(folder) = registered.as_deref().and_then(Path::parent) {
        dirs.insert(0, folder.to_path_buf());
    }
    let mut glb_files = collect_unique_ext(&dirs, &["glb", "gltf"], 3);
    let obj_files = collect_unique_ext(&dirs, &["obj", "stl", "ply", "3mf"], 3);
    let fbx_files = collect_unique_ext(&dirs, &["fbx"], 3);
    if let Some(model) = registered.filter(|m| has_matching_ext(m, &["glb", "gltf"])) {
        glb_files.retain(|f| *f != model);
        glb_files.insert(0, model);
    }
    let mut all_meshes = glb_files.clone();
    all_meshes.extend(fbx_files.iter().cloned());
    all_meshes.extend(obj_files.iter().cloned());
    let tcraft_files = collect_unique_ext(&dirs, &["tcraft"], 3);
    let zip_files = collect_unique_ext(&dirs, &["zip"], 2);
    let video_previews = collect_unique_ext(&dirs, &["mp4", "webm", "mov"], 2);
    let (mut anim_clips, import_meta, repair_meta, multistage_meta) = scan_json_metadata(&dirs);
    append_imported_actions(&mut anim_clips, &import_meta, fbx_files.first(), glb_files.first(), video_previews.first());
    let manifest = read_manifest(&canonical).unwrap_or_else(|| json!({}));
    let primary_glb = pick_primary_mesh(&manifest, &all_meshes);
    let mesh_summary = primary_glb.as_deref().and_then(|p| read_craft_mesh_scene(p).ok().map(|sc| summarize_mesh_scene(p, &sc))).unwrap_or(Value::Null);
    let mesh_joints = mesh_summary["joints"].as_u64().unwrap_or(0);
    let model_clips: Vec<String> = mesh_summary["animations"].as_array().into_iter().flatten().filter_map(|v| v.as_str().map(str::to_owned)).collect();
    let bone_count = import_meta["bones"].as_u64().unwrap_or(0).max(mesh_joints);
    let rigged = bone_count > 0;
    let has_3d = !all_meshes.is_empty();
    let anim_style = manifest["animation_style"].as_str().unwrap_or("rig_3d");
    json!({
        "available": has_3d,
        "animation_style": anim_style,
        "directory": canonical,
        "primary_glb": primary_glb,
        "glb_files": glb_files,
        "fbx_files": fbx_files,
        "obj_files": obj_files,
        "tcraft_files": tcraft_files,
        "zip_archives": zip_files,
        "video_previews": video_previews,
        "mesh_summary": mesh_summary,
        "clips": anim_clips,
        "model_clips": model_clips,
        "rigged": rigged,
        "bone_count": bone_count,
        "rig_type": manifest["rig_type"].as_str().unwrap_or("biped"),
        "skeleton_preset": manifest["skeleton_preset"].as_str().unwrap_or("mixamo"),
        "animate_in_place": manifest["animate_in_place"].as_bool().unwrap_or(true),
        "fbx_preset": manifest["fbx_preset"].as_str().unwrap_or("unreal"),
        "import_summary": import_meta,
        "rig_repair": repair_meta,
        "multistage_request": multistage_meta,
        "breakouts": manifest.get("breakouts").cloned().unwrap_or_else(|| json!([])),
        "precheck": manifest.get("precheck").cloned().unwrap_or(Value::Null),
    })
}

/// `character.model3d.catalog`: Return Magic's 3D breakout & preset catalog.
pub fn cmd_model3d_catalog(_: &mut Session, p: &Value) -> Result<Value> {
    let mut cat = catalog_json()?;
    if let Some(category) = p["category"].as_str().filter(|s| !s.is_empty())
        && let Some(biped) = cat.pointer_mut("/animation/presets/biped").and_then(Value::as_array_mut)
    {
        biped.retain(|item| item["category"].as_str() == Some(category));
    }
    Ok(json!({"ok": true, "catalog": cat}))
}

fn build_breakout_record(kind: &str, p: &Value, reference: &Option<PathBuf>, breakout_dir: &Path, id: &str) -> Result<Value> {
    let catalog = catalog_json()?;
    let matched_tpl = catalog["breakout_templates"].as_array().and_then(|a| a.iter().find(|t| t["id"].as_str() == Some(kind)).cloned());
    let matched_op = catalog["breakout_operations"].as_array().and_then(|a| a.iter().find(|o| o["id"].as_str() == Some(kind)).cloned());
    let granularity = p["granularity"].as_str().unwrap_or("medium");
    if !matches!(granularity, "low" | "medium" | "high") {
        return Err(bad("granularity must be low, medium, or high"));
    }
    let completion_mode = p["completion_mode"].as_str().unwrap_or("quick_cap");
    if !matches!(completion_mode, "quick_cap" | "ai_completion") {
        return Err(bad("completion_mode must be quick_cap or ai_completion"));
    }
    let provider = p["provider"].as_str().unwrap_or("magic");
    if !["magic", "tripo", "craft_ai"].contains(&provider) {
        return Err(bad("provider must be magic, tripo, or craft_ai"));
    }
    if let Some(tpl) = matched_tpl {
        return Ok(json!({
            "kind": kind,
            "provider": provider,
            "stage": "image_template",
            "template_id": tpl["template_id"],
            "title": tpl["title"],
            "category": tpl["category"],
            "recommended_model": tpl["recommended_model"],
            "recommended_ratio": tpl["recommended_ratio"],
            "t_pose": kind == "t_pose",
            "reference": reference,
            "prompt": p["prompt"].as_str().unwrap_or_else(|| tpl["description"].as_str().unwrap_or_default()),
        }));
    }
    if let Some(op) = matched_op {
        let craft_cli = match (kind, reference.as_deref()) {
            ("split_image", Some(src)) => {
                let out_pcraft = breakout_dir.join(format!("{id}-cutout.pcraft"));
                let out_png = breakout_dir.join(format!("{id}-cutout.png"));
                let layers_dir = breakout_dir.join(format!("{id}-split-layers"));
                let _ = std::fs::create_dir_all(&layers_dir);
                let steps = [
                    ("layer.removeBackground", json!({})),
                    ("file.export.layersToFiles", json!({"dir": layers_dir, "format": "png", "prefix": id, "trim": true})),
                    ("file.saveACopy", json!({"path": out_png, "overwrite": true})),
                ];
                match crate::cmd::character::cut::photocraft_run(src, &steps, Some(&out_pcraft)) {
                    Ok(_) => {
                        let exported = collect_files_with_ext(&layers_dir, &["png"], 1);
                        json!({
                            "tool": "photocraft-cli",
                            "status": "executed_local_cutout_and_trimmed_layers",
                            "pcraft": out_pcraft,
                            "png": out_png,
                            "layers_dir": layers_dir,
                            "exported_layers": exported,
                        })
                    }
                    Err(e) => json!({"tool": "photocraft-cli", "status": "failed", "error": e.to_string()}),
                }
            }
            _ => Value::Null,
        };
        return Ok(json!({
            "kind": kind,
            "provider": provider,
            "stage": "breakout_operation",
            "label": op["label"],
            "granularity": (kind == "ai_segmentation").then_some(granularity),
            "completion_mode": (kind == "mesh_completion").then_some(completion_mode),
            "views": op.get("views"),
            "reference": reference,
            "craft_cli": craft_cli,
        }));
    }
    Err(bad(format!("unknown 3D breakout kind: {kind}")))
}

/// `character.model3d.breakout`: Prepare or run a 3D character breakout step.
pub fn cmd_model3d_breakout(s: &mut Session, p: &Value) -> Result<Value> {
    let id = character_id(p)?;
    let kind = p["kind"].as_str().unwrap_or("t_pose");
    let project_dir = crate::workshop::project_dir(s)?;
    let dir = character_3d_dir(&project_dir, &id);
    let breakout_dir = dir.join("breakouts");
    std::fs::create_dir_all(&breakout_dir)?;

    let reference = p["reference"].as_str().filter(|r| !r.trim().is_empty()).map(|r| crate::workspace::resolve_path_in(s, Some(&project_dir), r));
    if let Some(path) = &reference
        && !path.is_file()
    {
        return Err(bad(format!("reference image does not exist: {}", path.display())));
    }
    let record = build_breakout_record(kind, p, &reference, &breakout_dir, &id)?;
    let receipt_path = breakout_dir.join(format!("breakout-{kind}.json"));
    write_json_pretty(&receipt_path, &record)?;

    let manifest_path = dir.join("model3d.json");
    let mut manifest = read_manifest(&dir).unwrap_or_else(|| json!({"character": id}));
    if !manifest["breakouts"].is_array() {
        manifest["breakouts"] = json!([]);
    }
    if let Some(arr) = manifest["breakouts"].as_array_mut() {
        arr.retain(|entry| entry["kind"] != kind);
        arr.push(record.clone());
    }
    write_json_pretty(&manifest_path, &manifest)?;
    Ok(json!({
        "ok": true,
        "character": id,
        "breakout": record,
        "receipt": receipt_path,
        "manifest": manifest_path,
    }))
}

fn validate_rig_options(p: &Value) -> Result<(&str, &str, &str, u64)> {
    let rig_type = p["rig_type"].as_str().unwrap_or("biped");
    if !["biped", "quadruped", "hexapod", "octopod", "avian", "serpentine", "aquatic", "others"].contains(&rig_type) {
        return Err(bad("invalid rig_type"));
    }
    let skel = p["skeleton_preset"].as_str().unwrap_or("mixamo");
    if !["mixamo", "actorcore", "unreal", "vrm", "unity"].contains(&skel) {
        return Err(bad("invalid skeleton_preset"));
    }
    let topology = p["topology"].as_str().unwrap_or("quad");
    if !["quad", "triangle"].contains(&topology) {
        return Err(bad("topology must be quad or triangle"));
    }
    let max_faces = if topology == "quad" { 50_000 } else { 80_000 };
    Ok((rig_type, skel, topology, max_faces))
}

fn build_precheck_value(p: &Value, current: &Value, mesh_report: &Value, topology: &str, max_faces: u64, t_pose_ready: bool) -> Result<(bool, Value)> {
    let observed_faces = mesh_report["triangles"]
        .as_u64()
        .or_else(|| p["face_count"].as_u64())
        .or_else(|| current["import_summary"]["faces"].as_u64())
        .ok_or_else(|| bad("Cannot check polygon limit without a mesh file or explicit face_count"))?;
    let observed_vertices = mesh_report["vertices"].as_u64();
    let within_limit = observed_faces <= max_faces;
    let recommended = if !within_limit {
        Some("retopologize")
    } else if !t_pose_ready {
        Some("generate_t_pose")
    } else {
        None
    };
    let riggable = within_limit && t_pose_ready;
    Ok((
        riggable,
        json!({
            "schema": "magic.model3d.precheck.v2",
            "engine": "craft-mesh",
            "topology": topology,
            "face_count": observed_faces,
            "vertex_count": observed_vertices,
            "face_limit": max_faces,
            "within_polygon_limit": within_limit,
            "t_pose_ready": t_pose_ready,
            "riggable": riggable,
            "mesh_info": mesh_report,
            "recommended_adjustment": recommended,
        }),
    ))
}

/// `character.model3d.rig`: Validate 3D mesh topology, UV sets, normals, tangents, and skeletal
/// skins directly via `craft-mesh` (normalizing non-glTF inputs to `.glb` when needed).
pub fn cmd_model3d_rig(s: &mut Session, p: &Value) -> Result<Value> {
    let id = character_id(p)?;
    let (rig_type, skeleton_preset, topology, max_faces) = validate_rig_options(p)?;
    let project_dir = crate::workshop::project_dir(s)?;
    let current = inspect_character_3d(s, &project_dir, &id);
    let t_pose_ready = p["t_pose_ready"].as_bool().unwrap_or(true);
    let dir = character_3d_dir(&project_dir, &id);
    std::fs::create_dir_all(&dir)?;

    let mut model_path = p["model"]
        .as_str()
        .filter(|m| !m.trim().is_empty())
        .map(|m| crate::workspace::resolve_path_in(s, Some(&project_dir), m))
        .or_else(|| current["primary_glb"].as_str().map(PathBuf::from));

    let (mesh_report, scene_joints) = if let Some(ref path) = model_path {
        let scene = read_craft_mesh_scene(path)?;
        let summary = summarize_mesh_scene(path, &scene);
        let joints = summary["joints"].as_u64().unwrap_or(0);
        let limits = MeshLimits::default();
        if !has_matching_ext(path, &["glb"]) {
            let normalized_glb = dir.join(format!("{id}-normalized.glb"));
            if let Ok(glb_bytes) = craft_mesh::gltf::write_glb(&scene, &limits)
                && craft_atomic::atomic_write(&normalized_glb, &glb_bytes).is_ok()
            {
                model_path = Some(normalized_glb);
            }
        }
        (summary, joints)
    } else {
        (Value::Null, 0)
    };

    let (riggable, precheck) = build_precheck_value(p, &current, &mesh_report, topology, max_faces, t_pose_ready)?;
    let rig_spec = json!({
        "schema": "magic.model3d.rig.v2",
        "engine": "craft-mesh",
        "rig_type": rig_type,
        "skeleton_preset": (rig_type == "biped").then_some(skeleton_preset),
    });
    let bone_count = current["bone_count"].as_u64().unwrap_or(0).max(scene_joints);
    let verified_skeleton = bone_count > 0;
    let manifest_path = dir.join("model3d.json");
    let mut manifest = read_manifest(&dir).unwrap_or_else(|| json!({"character": id}));
    manifest["character"] = json!(id);
    manifest["rigged"] = json!(verified_skeleton && riggable);
    manifest["bone_count"] = json!(bone_count);
    manifest["precheck_passed"] = json!(riggable);
    manifest["rig_type"] = json!(rig_type);
    manifest["skeleton_preset"] = json!(skeleton_preset);
    manifest["topology"] = json!(topology);
    manifest["precheck"] = precheck.clone();
    manifest["rig_spec"] = rig_spec.clone();
    if let Some(model) = &model_path {
        manifest["base_model_glb"] = json!(model);
    }
    write_json_pretty(&manifest_path, &manifest)?;
    Ok(json!({
        "ok": true,
        "character": id,
        "rigged": verified_skeleton && riggable,
        "bone_count": bone_count,
        "precheck_passed": riggable,
        "rigging_execution": if verified_skeleton {
            "verified_craft_mesh_skeleton"
        } else {
            "precheck_complete_needs_rigged_glb_skeleton"
        },
        "precheck": precheck,
        "rig_spec": rig_spec,
        "manifest": manifest_path,
    }))
}

fn parse_stage_waypoints(stage: &Value, prev_end: Option<[f64; 2]>) -> Result<([f64; 2], Value)> {
    let Some(wp) = stage.get("waypoints").and_then(Value::as_array) else {
        let end = [stage["x"].as_f64().unwrap_or(0.0), stage["z"].as_f64().unwrap_or(0.0)];
        let start = prev_end.unwrap_or([0.0, 0.0]);
        return Ok((end, json!([[start[0], start[1]], [end[0], end[1]]])));
    };
    if wp.len() != 2 {
        return Err(bad("waypoints must be [[x0, z0], [x1, z1]]"));
    }
    let p0 = [
        wp[0].get(0).and_then(Value::as_f64).ok_or_else(|| bad("invalid waypoint x0"))?,
        wp[0].get(1).and_then(Value::as_f64).ok_or_else(|| bad("invalid waypoint z0"))?,
    ];
    let p1 = [
        wp[1].get(0).and_then(Value::as_f64).ok_or_else(|| bad("invalid waypoint x1"))?,
        wp[1].get(1).and_then(Value::as_f64).ok_or_else(|| bad("invalid waypoint z1"))?,
    ];
    if let Some(prev) = prev_end
        && ((prev[0] - p0[0]).abs() > 1e-6 || (prev[1] - p0[1]).abs() > 1e-6)
    {
        return Err(bad("stage start waypoint must match previous stage end waypoint"));
    }
    Ok((p1, json!([[p0[0], p0[1]], [p1[0], p1[1]]])))
}

fn validate_multistage_segments(stages: &[Value]) -> Result<Vec<Value>> {
    if stages.is_empty() || stages.len() > 5 {
        return Err(bad("Multi-Stage Motion requires 1 to 5 stages"));
    }
    let mut normalized = Vec::with_capacity(stages.len());
    let mut prev_end: Option<[f64; 2]> = None;
    for (idx, stage) in stages.iter().enumerate() {
        let prompt = stage["prompt"]
            .as_str()
            .or_else(|| stage["action"].as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty() && s.chars().count() <= 1000)
            .ok_or_else(|| bad("every stage needs a prompt/action (1..=1000 chars)"))?;
        let duration = stage["duration_seconds"]
            .as_u64()
            .or_else(|| stage["duration"].as_u64())
            .filter(|d| (1..=10).contains(d))
            .ok_or_else(|| bad("duration_seconds must be 1..=10"))?;
        let (end_pt, waypoints) = parse_stage_waypoints(stage, prev_end)?;
        prev_end = Some(end_pt);
        normalized.push(json!({
            "name": stage["name"].as_str().unwrap_or(&format!("Stage {}", idx + 1)),
            "prompt": prompt,
            "duration_seconds": duration,
            "waypoints": waypoints,
        }));
    }
    Ok(normalized)
}

struct ClipSpec {
    id: String,
    name: String,
    source: &'static str,
    category: String,
    duration: f64,
    stages: Value,
    motion_spec: Value,
}

fn build_clip_spec(p: &Value, rig_type: &str) -> Result<ClipSpec> {
    if let Some(stages_arr) = p.get("stages").and_then(Value::as_array) {
        if rig_type != "biped" {
            return Err(bad("Multi-Stage Motion requires rig_type=biped"));
        }
        let stages = validate_multistage_segments(stages_arr)?;
        let dur: u64 = stages.iter().filter_map(|s| s["duration_seconds"].as_u64()).sum();
        let raw_name = p["name"].as_str().unwrap_or("multi_stage_motion");
        let cid = crate::workspace::sanitize_asset_id(raw_name);
        return Ok(ClipSpec {
            id: if cid.is_empty() { "multi_stage".into() } else { cid },
            name: raw_name.to_string(),
            source: "multi_stage_motion",
            category: "interactive".into(),
            duration: dur as f64,
            stages: json!(stages),
            motion_spec: json!({"kind": "multi_stage", "rig_type": rig_type, "segments": stages}),
        });
    }
    if let Some(prompt) = p["prompt"].as_str().filter(|s| !s.trim().is_empty()) {
        if rig_type != "biped" {
            return Err(bad("AI Text-to-Motion requires rig_type=biped"));
        }
        let dur = p["duration"].as_u64().unwrap_or(4).clamp(1, 10);
        let stages = validate_multistage_segments(&[json!({"name": "Start", "prompt": prompt, "duration_seconds": dur})])?;
        let raw_name = p["name"].as_str().unwrap_or(prompt);
        let cid = crate::workspace::sanitize_asset_id(raw_name);
        return Ok(ClipSpec {
            id: if cid.is_empty() { "ai_motion".into() } else { cid },
            name: raw_name.chars().take(64).collect(),
            source: "ai_animation",
            category: "interactive".into(),
            duration: dur as f64,
            stages: json!(stages),
            motion_spec: json!({"kind": "prompt_motion", "rig_type": rig_type, "segments": stages}),
        });
    }
    let catalog = catalog_json()?;
    let presets = catalog
        .pointer(&format!("/animation/presets/{rig_type}"))
        .and_then(Value::as_array)
        .ok_or_else(|| bad(format!("no presets for rig_type={rig_type}")))?;
    let default_preset = if rig_type == "biped" { "wave_goodbye_01" } else { presets.first().and_then(|item| item["name"].as_str()).unwrap_or("walk") };
    let preset_name = p["preset"].as_str().unwrap_or(default_preset);
    let found = presets.iter().find(|item| item["name"].as_str() == Some(preset_name)).ok_or_else(|| bad(format!("unknown preset '{preset_name}'")))?;
    Ok(ClipSpec {
        id: crate::workspace::sanitize_asset_id(preset_name),
        name: preset_name.replace('_', " "),
        source: "preset",
        category: found["category"].as_str().unwrap_or("basic").into(),
        duration: p["duration"].as_f64().unwrap_or(3.0).max(0.5),
        stages: Value::Null,
        motion_spec: json!({"kind": "preset_retarget", "rig_type": rig_type, "animations": [format!("preset:{rig_type}:{preset_name}")]}),
    })
}

fn build_effectcraft_3d_comp(clips_dir: &Path, id: &str, clip_id: &str, duration: f64, model_file: Option<&PathBuf>) -> (Value, Value) {
    let Some(file) = model_file.filter(|f| f.is_file()) else {
        return (Value::Null, Value::Null);
    };
    let staged_glb = if has_matching_ext(file, &["glb"]) {
        file.clone()
    } else if let Ok(scene) = read_craft_mesh_scene(file)
        && let Ok(glb_bytes) = craft_mesh::gltf::write_glb(&scene, &MeshLimits::default())
    {
        let converted = clips_dir.join(format!("{id}-{clip_id}-mesh.glb"));
        if craft_atomic::atomic_write(&converted, &glb_bytes).is_ok() { converted } else { file.clone() }
    } else {
        file.clone()
    };
    let comp_name = format!("{id}-{clip_id}-3d");
    let ec_path = clips_dir.join(format!("{comp_name}.ecproj"));
    let frame_path = clips_dir.join(format!("{comp_name}-frame.png"));
    let steps = vec![
        super::step(
            "comp.new",
            json!({
                "name": comp_name,
                "width": 1280,
                "height": 720,
                "frameRate": 24,
                "duration": duration,
                "renderer": "advanced3D",
                "open": true
            }),
        ),
        super::step("layer.newModel", json!({"path": staged_glb, "name": id, "position": [640, 360]})),
        super::step("file.saveAs", json!({"path": ec_path})),
    ];
    if super::effectcraft(clips_dir, None, &steps, true).is_err() {
        return (Value::Null, Value::Null);
    }
    let rendered = crate::cmd::character::effectcraft_frame(&ec_path, &comp_name, (duration * 0.5).min(1.0), 900, &frame_path)
        .ok()
        .filter(|_| frame_path.is_file())
        .map(|_| json!(frame_path))
        .unwrap_or(Value::Null);
    (json!(ec_path), rendered)
}

fn first_or_param_path(s: &Session, project_dir: &Path, p: &Value, key: &str, default_val: &Value) -> Option<PathBuf> {
    p[key]
        .as_str()
        .filter(|v| !v.is_empty())
        .map(|v| crate::workspace::resolve_path_in(s, Some(project_dir), v))
        .or_else(|| default_val.as_str().or_else(|| default_val.as_array()?.first()?.as_str()).map(PathBuf::from))
}

fn clip_specific_glb(clips_dir: &Path, id: &str, clip_id: &str, rig_type: &str) -> Option<PathBuf> {
    let candidates = [
        clips_dir.join(format!("{id}-preset_{rig_type}_{clip_id}.glb")),
        clips_dir.join(format!("{id}-{clip_id}.glb")),
        clips_dir.join(format!("{clip_id}.glb")),
    ];
    candidates.into_iter().find(|p| p.is_file())
}

/// `character.model3d.animate`: Author a split `.anim.json` clip, normalize non-glTF meshes to `.glb`
/// via `craft-mesh`, and build an `EffectCraft` `advanced3D` composition (`.ecproj`) via `layer.newModel`.
pub fn cmd_model3d_animate(s: &mut Session, p: &Value) -> Result<Value> {
    let id = character_id(p)?;
    let project_dir = crate::workshop::project_dir(s)?;
    let current = inspect_character_3d(s, &project_dir, &id);
    let rig_type = p["rig_type"].as_str().or_else(|| current["rig_type"].as_str()).unwrap_or("biped");
    let animate_in_place = p["animate_in_place"].as_bool().unwrap_or(true);
    let fbx_preset = p["fbx_preset"].as_str().unwrap_or("unreal");
    if !["unreal", "unity", "maya", "3dsmax", "blender"].contains(&fbx_preset) {
        return Err(bad("invalid fbx_preset"));
    }
    let spec = build_clip_spec(p, rig_type)?;
    let dir = character_3d_dir(&project_dir, &id);
    let clips_dir = dir.join("clips");
    std::fs::create_dir_all(&clips_dir)?;

    let fbx_path = first_or_param_path(s, &project_dir, p, "fbx", &current["fbx_files"]);
    let glb_path = p["glb"]
        .as_str()
        .filter(|v| !v.is_empty())
        .map(|v| crate::workspace::resolve_path_in(s, Some(&project_dir), v))
        .or_else(|| clip_specific_glb(&clips_dir, &id, &spec.id, rig_type))
        .or_else(|| first_or_param_path(s, &project_dir, p, "glb", &current["primary_glb"]));
    let preview_path = first_or_param_path(s, &project_dir, p, "preview", &current["video_previews"]);
    let (ecproj_3d, rendered_frame) = build_effectcraft_3d_comp(&clips_dir, &id, &spec.id, spec.duration, glb_path.as_ref().or(fbx_path.as_ref()));
    let clip_doc = json!({
        "schema": "magic.model3d.anim.v2",
        "engine": "craft-mesh",
        "id": spec.id,
        "character": id,
        "name": spec.name,
        "source": spec.source,
        "category": spec.category,
        "rig_type": rig_type,
        "duration": spec.duration,
        "in_place": animate_in_place,
        "fbx_preset": fbx_preset,
        "glb": glb_path,
        "fbx": fbx_path,
        "ecproj_3d": ecproj_3d,
        "rendered_frame": rendered_frame,
        "retarget_execution": if ecproj_3d.is_string() {
            "composed_in_effectcraft_advanced3d"
        } else {
            "manifest_recorded_awaiting_rigged_glb"
        },
        "preview": preview_path,
        "stages": spec.stages,
        "motion_spec": spec.motion_spec,
    });
    let clip_file = clips_dir.join(format!("{}.anim.json", spec.id));
    write_json_pretty(&clip_file, &clip_doc)?;

    let manifest_path = dir.join("model3d.json");
    let mut manifest = read_manifest(&dir).unwrap_or_else(|| json!({"character": id}));
    manifest["character"] = json!(id);
    manifest["animate_in_place"] = json!(animate_in_place);
    manifest["fbx_preset"] = json!(fbx_preset);
    if let Some(g) = &glb_path {
        manifest["base_model_glb"] = json!(g);
    }
    write_json_pretty(&manifest_path, &manifest)?;
    Ok(json!({
        "ok": true,
        "character": id,
        "clip": clip_doc,
        "clip_file": clip_file,
        "manifest": manifest_path,
    }))
}
