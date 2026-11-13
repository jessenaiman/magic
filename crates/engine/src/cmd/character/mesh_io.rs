//! `mesh.*` & shared `craft-mesh` 3D character inspection/staging helpers: pure-Rust 3D scene
//! validation, topology/skeleton/clip inspection, format conversion (`.glb`, `.gltf`, `.obj` +
//! `.mtl`, `.fbx`, `.stl`, `.ply`, `.3mf`), and `EffectCraft` `advanced3D` `layer.newModel` staging.

use std::path::{Path, PathBuf};

use craft_mesh::{Limits as MeshLimits, Scene as MeshScene};
use serde_json::{Value, json};

use super::bad;
use crate::{Result, Session};

pub(crate) fn has_matching_ext(path: &Path, exts: &[&str]) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| exts.iter().any(|w| e.eq_ignore_ascii_case(w)))
}

pub(crate) fn collect_files_with_ext(root: &Path, exts: &[&str], max_depth: usize) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![(root.to_path_buf(), 0usize)];
    while let Some((dir, depth)) = stack.pop() {
        for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() && depth < max_depth {
                stack.push((path, depth + 1));
            } else if path.is_file() && has_matching_ext(&path, exts) {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

pub(crate) fn collect_unique_ext(dirs: &[PathBuf], exts: &[&str], max_depth: usize) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for dir in dirs {
        for p in collect_files_with_ext(dir, exts, max_depth) {
            if !out.contains(&p) {
                out.push(p);
            }
        }
    }
    out
}

pub(crate) fn read_json_optional(path: &Path) -> Option<Value> {
    std::fs::read_to_string(path).ok().and_then(|t| serde_json::from_str(&t).ok())
}

/// Read any supported 3D scene/mesh file (`.glb`, `.gltf`, `.obj`, `.fbx`, `.stl`, `.ply`, `.3mf`)
/// via `craft-mesh` with bounded `Limits` and sidecar resolution confined to the file's directory.
pub(crate) fn read_craft_mesh_scene(path: &Path) -> Result<MeshScene> {
    let bytes = std::fs::read(path)?;
    let limits = MeshLimits::default();
    let parent_dir = path.parent().map(Path::to_path_buf);
    let mut resolver: craft_mesh::Resolver = &mut |rel: &str| -> Option<Vec<u8>> {
        let Some(base) = &parent_dir else {
            return None;
        };
        let Ok(resolved) = craft_docpath::resolve_absolute(rel, Some(base.as_path())) else {
            return None;
        };
        std::fs::read(resolved).ok()
    };
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase();
    let scene = match ext.as_str() {
        "glb" | "gltf" => craft_mesh::gltf::read(&bytes, &limits, &mut resolver).map_err(|e| bad(format!("{}: {e}", path.display())))?,
        "obj" => craft_mesh::obj::read(&bytes, &limits, &mut resolver).map_err(|e| bad(format!("{}: {e}", path.display())))?,
        "fbx" => craft_mesh::fbx::read(&bytes, &limits, &mut resolver).map_err(|e| bad(format!("{}: {e}", path.display())))?,
        "stl" => {
            let scene = craft_mesh::stl::read(&bytes, &limits).map_err(|e| bad(format!("{}: {e}", path.display())))?;
            MeshScene::from_meshes(scene.meshes, Vec::new())
        }
        "ply" => {
            let scene = craft_mesh::ply::read(&bytes, &limits).map_err(|e| bad(format!("{}: {e}", path.display())))?;
            MeshScene::from_meshes(scene.meshes, Vec::new())
        }
        "3mf" => craft_mesh::threemf::read(&bytes, &limits).map_err(|e| bad(format!("{}: {e}", path.display())))?,
        _ if craft_mesh::gltf::is_glb(&bytes) => {
            craft_mesh::gltf::read(&bytes, &limits, &mut craft_mesh::no_external).map_err(|e| bad(format!("{}: {e}", path.display())))?
        }
        other => return Err(bad(format!("unsupported 3D mesh format `.{other}` (expected glb, gltf, obj, fbx, stl, ply, or 3mf)"))),
    };
    scene.validate().map_err(|e| bad(format!("{} failed craft-mesh validation: {e}", path.display())))?;
    Ok(scene)
}

/// Summarize a validated [`craft_mesh::Scene`] into structured JSON metrics for UI & CLI inspection.
pub(crate) fn summarize_mesh_scene(path: &Path, scene: &MeshScene) -> Value {
    let mut vertices: u64 = 0;
    let mut triangles: u64 = 0;
    let mut faces: u64 = 0;
    let mut has_normals = false;
    let mut has_tangents = false;
    let mut uv_set_count: usize = 0;
    let mut skinned_meshes: usize = 0;
    for mesh in &scene.meshes {
        vertices = vertices.saturating_add(mesh.positions.len() as u64);
        has_normals |= !mesh.normals.is_empty();
        has_tangents |= !mesh.tangents.is_empty();
        uv_set_count = uv_set_count.max(mesh.uv_sets.len());
        if !mesh.joints.is_empty() && !mesh.weights.is_empty() {
            skinned_meshes += 1;
        }
        for part in &mesh.parts {
            faces = faces.saturating_add(part.faces().count() as u64);
            triangles = triangles.saturating_add(part.triangles().count() as u64);
        }
    }
    let max_skin_joints = scene.skins.iter().map(|sk| sk.joints.len() as u64).max().unwrap_or(0);
    let anim_names: Vec<String> =
        scene.animations.iter().enumerate().map(|(idx, a)| if a.name.is_empty() { format!("clip_{}", idx + 1) } else { a.name.clone() }).collect();
    let instance_count = scene.instances(&MeshLimits::default()).map(|v| v.len()).unwrap_or(0);
    let byte_len = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let fmt = path.extension().and_then(|e| e.to_str()).unwrap_or("glb").to_ascii_lowercase();
    json!({
        "path": path,
        "format": fmt,
        "bytes": byte_len,
        "meshes": scene.meshes.len(),
        "instances": instance_count,
        "nodes": scene.nodes.len(),
        "vertices": vertices,
        "faces": faces,
        "triangles": triangles,
        "hasNormals": has_normals,
        "hasTangents": has_tangents,
        "uvSets": uv_set_count,
        "skinnedMeshes": skinned_meshes,
        "skins": scene.skins.len(),
        "joints": max_skin_joints,
        "animations": anim_names,
        "animation_count": anim_names.len(),
        "materials": scene.materials.len(),
        "images": scene.images.len(),
        "cameras": scene.cameras.len(),
        "lights": scene.lights.len(),
        "warnings": scene.warnings,
    })
}

fn candidate_3d_dirs(project_dir: &Path, id: &str) -> Vec<PathBuf> {
    vec![
        project_dir.join("assets/characters").join(id).join("3d"),
        project_dir.join("assets").join(id).join("3d"),
        project_dir.join("3d").join(format!("{id}-actor")),
        project_dir.join("3d").join(id),
    ]
}

fn registered_model(s: &Session, id: &str) -> Option<PathBuf> {
    let norm = id.replace('-', "_");
    s.project_assets.iter().filter(|a| a.kind == "model_3d" && a.id.replace('-', "_") == norm).find_map(|a| a.path.clone().filter(|p| p.is_file()))
}

fn scan_anim_json_clips(dirs: &[PathBuf]) -> Vec<Value> {
    let mut clips = Vec::new();
    for p in collect_unique_ext(dirs, &["json"], 3) {
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        if !name.ends_with(".anim.json") {
            continue;
        }
        let Some(doc) = read_json_optional(&p) else {
            continue;
        };
        let stem = name.trim_end_matches(".anim.json");
        clips.push(json!({
            "id": doc["id"].as_str().unwrap_or(stem),
            "name": doc["name"].as_str().unwrap_or(stem),
            "source": doc["source"],
            "category": doc["category"],
            "duration": doc["duration"],
            "in_place": doc["in_place"],
            "fbx": doc["fbx"],
            "glb": doc["glb"],
            "ecproj_3d": doc["ecproj_3d"],
            "preview": doc["preview"],
            "file": p,
        }));
    }
    clips
}

/// Inspect 3D `.glb`/`.gltf`/`.fbx`/`.obj`/`.stl`/`.ply`/`.3mf` models via `craft-mesh` and
/// TextureCraft `.tcraft` materials for one character, keeping the `02 Animations` UI contract intact.
pub(crate) fn inspect_character_3d(s: &Session, project_dir: &Path, id: &str) -> Value {
    let canonical = project_dir.join("assets/characters").join(id).join("3d");
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
    let video_previews = collect_unique_ext(&dirs, &["mp4", "webm", "mov"], 2);
    let anim_clips = scan_anim_json_clips(&dirs);
    let primary_glb = all_meshes
        .iter()
        .find(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.contains("rigged") || n.contains("rigid-case")))
        .cloned()
        .or_else(|| all_meshes.first().cloned());
    let mesh_summary = primary_glb.as_deref().and_then(|p| read_craft_mesh_scene(p).ok().map(|sc| summarize_mesh_scene(p, &sc))).unwrap_or(Value::Null);
    let bone_count = mesh_summary["joints"].as_u64().unwrap_or(0);
    let model_clips: Vec<String> = mesh_summary["animations"].as_array().into_iter().flatten().filter_map(|v| v.as_str().map(str::to_owned)).collect();
    json!({
        "available": !all_meshes.is_empty(),
        "animation_style": "rig_3d",
        "directory": canonical,
        "primary_glb": primary_glb,
        "glb_files": glb_files,
        "fbx_files": fbx_files,
        "obj_files": obj_files,
        "tcraft_files": tcraft_files,
        "video_previews": video_previews,
        "mesh_summary": mesh_summary,
        "clips": anim_clips,
        "model_clips": model_clips,
        "rigged": bone_count > 0,
        "bone_count": bone_count,
    })
}

/// Compact `model3d` inspector summary embedded on a character view for `animation_inspector.rs`.
pub(crate) fn view_model3d_summary(info_3d: &Value) -> Value {
    if info_3d["available"].as_bool() != Some(true) {
        return Value::Null;
    }
    let ms = &info_3d["mesh_summary"];
    let anim_n = ms["animation_count"].as_u64().unwrap_or_else(|| info_3d["model_clips"].as_array().map_or(0, |a| a.len() as u64));
    json!({
        "path": info_3d["primary_glb"],
        "format": ms["format"].as_str().unwrap_or("glb"),
        "vertices": ms["vertices"].as_u64().unwrap_or(0),
        "triangles": ms["triangles"].as_u64().unwrap_or(0),
        "joints": info_3d["bone_count"].as_u64().unwrap_or(0),
        "animations": anim_n,
    })
}

pub(crate) struct GlbStageSpec<'a> {
    pub comp_name: &'a str,
    pub glb_path: &'a Path,
    pub clip_index: usize,
    pub duration: f64,
    pub speed_pct: u32,
    pub out_ecproj: &'a Path,
}

/// Stage a validated `.glb` character clip into an `EffectCraft` `advanced3D` `.ecproj` via `layer.newModel`.
pub(crate) fn stage_glb_in_effectcraft(project_dir: &Path, spec: &GlbStageSpec<'_>) -> Result<Vec<Value>> {
    if let Some(parent) = spec.out_ecproj.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let dur = spec.duration.clamp(0.2, 120.0);
    let steps = vec![
        json!({
            "id": "comp.new",
            "params": {"name": spec.comp_name, "width": 1920, "height": 1080, "frameRate": 30, "duration": dur, "renderer": "advanced3D", "background": "#20332a"},
        }),
        json!({"id": "layer.newModel", "params": {"path": spec.glb_path, "name": "Character"}}),
        json!({"id": "prop.set", "params": {"layer": "Character", "path": "transform/position", "value": [960, 860, 0]}}),
        json!({"id": "prop.set", "params": {"layer": "Character", "path": "transform/scale", "value": [140, 140, 140]}}),
        json!({"id": "prop.set", "params": {"layer": "Character", "path": "geometryOptions/animation", "value": spec.clip_index + 1}}),
        json!({"id": "prop.set", "params": {"layer": "Character", "path": "geometryOptions/loopAnimation", "value": true}}),
        json!({"id": "prop.set", "params": {"layer": "Character", "path": "geometryOptions/animationSpeed", "value": spec.speed_pct.clamp(10, 400)}}),
        json!({"command": "file.saveAs", "params": {"path": spec.out_ecproj}}),
    ];
    super::effectcraft(project_dir, None, &steps, true)
}

/// `mesh.inspect`: Inspect any 3D mesh/scene file (`.glb`, `.gltf`, `.obj`, `.fbx`, `.stl`, `.ply`, `.3mf`) via `craft-mesh`.
pub fn cmd_mesh_inspect(s: &mut Session, p: &Value) -> Result<Value> {
    let raw = p["path"].as_str().filter(|v| !v.trim().is_empty()).ok_or_else(|| bad("path=<mesh file> is required"))?;
    let resolved = if let Ok(dir) = crate::workshop::project_dir(s) {
        crate::workspace::resolve_path_in(s, Some(&dir), raw)
    } else {
        crate::workspace::resolve_workspace_path(s, raw)
    };
    if !resolved.is_file() {
        return Err(bad(format!("mesh file not found: {}", resolved.display())));
    }
    let scene = read_craft_mesh_scene(&resolved)?;
    Ok(json!({
        "ok": true,
        "engine": "craft-mesh",
        "summary": summarize_mesh_scene(&resolved, &scene),
    }))
}

/// `mesh.convert`: Convert a 3D mesh/scene between `.glb`, `.obj` (+ `.mtl`), and `.stl` via `craft-mesh`.
pub fn cmd_mesh_convert(s: &mut Session, p: &Value) -> Result<Value> {
    let src_raw = p["path"].as_str().or_else(|| p["source"].as_str()).filter(|v| !v.trim().is_empty()).ok_or_else(|| bad("path=<source mesh> is required"))?;
    let opt_fmt = p["format"].as_str().filter(|v| !v.trim().is_empty());
    let src = if let Ok(dir) = crate::workshop::project_dir(s) {
        crate::workspace::resolve_path_in(s, Some(&dir), src_raw)
    } else {
        crate::workspace::resolve_workspace_path(s, src_raw)
    };
    let out = if let Some(out_raw) = p["output"].as_str().or_else(|| p["out"].as_str()).filter(|v| !v.trim().is_empty()) {
        if let Ok(dir) = crate::workshop::project_dir(s) {
            let cand = Path::new(out_raw);
            if cand.is_absolute() { cand.to_path_buf() } else { dir.join(cand) }
        } else {
            crate::workspace::resolve_workspace_path(s, out_raw)
        }
    } else if let Some(fmt) = opt_fmt {
        src.with_extension(fmt.trim_start_matches('.').to_ascii_lowercase())
    } else {
        return Err(bad("output=<target .glb/.obj/.stl> or format=<glb|obj|stl> is required"));
    };
    if let Some(roots) = &s.automation_roots {
        roots.check_write(&out)?;
    }
    let scene = read_craft_mesh_scene(&src)?;
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let ext = out.extension().and_then(|e| e.to_str()).unwrap_or("glb").to_ascii_lowercase();
    let mut sidecar_mtl = None;
    let limits = MeshLimits::default();
    match ext.as_str() {
        "glb" => {
            let glb_bytes = craft_mesh::gltf::write_glb(&scene, &limits).map_err(|e| bad(format!("craft-mesh write_glb failed: {e}")))?;
            craft_atomic::atomic_write(&out, &glb_bytes)?;
        }
        "obj" => {
            let mtl_path = out.with_extension("mtl");
            let mtl_name = mtl_path.file_name().and_then(|n| n.to_str());
            let obj_text = craft_mesh::obj::write(&scene, mtl_name, &limits).map_err(|e| bad(format!("craft-mesh write_obj failed: {e}")))?;
            craft_atomic::atomic_write(&out, obj_text.as_bytes())?;
            if !scene.materials.is_empty() {
                let mtl_text = craft_mesh::obj::write_mtl(&scene);
                craft_atomic::atomic_write(&mtl_path, mtl_text.as_bytes())?;
                sidecar_mtl = Some(mtl_path);
            }
        }
        "stl" => {
            let name = out.file_name().and_then(|n| n.to_str()).unwrap_or("scene");
            let stl_bytes = craft_mesh::stl::write_binary(&scene, name, &limits).map_err(|e| bad(format!("craft-mesh write_stl failed: {e}")))?;
            craft_atomic::atomic_write(&out, &stl_bytes)?;
        }
        other => return Err(bad(format!("unsupported target mesh format `.{other}` (expected glb, obj, or stl)"))),
    }
    Ok(json!({
        "ok": true,
        "engine": "craft-mesh",
        "source": src,
        "output": out,
        "mtl": sidecar_mtl,
        "summary": summarize_mesh_scene(&out, &scene),
    }))
}
