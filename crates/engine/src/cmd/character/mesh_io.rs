//! `mesh.*` & shared `craft-mesh` I/O helpers: pure-Rust 3D scene/mesh inspection, validation,
//! and format conversion (`.glb`, `.gltf`, `.obj` + `.mtl`, `.fbx`, `.stl`, `.ply`, `.3mf`)
//! shared across `02 Animations` (`character.model3d.*`) and `05 Studio` (`EffectCraft` `advanced3D`).

use std::path::{Path, PathBuf};

use craft_mesh::{Limits as MeshLimits, Scene as MeshScene};
use serde_json::{Value, json};

use super::bad;
use crate::{Result, Session};

pub(crate) fn resolve_project_path(s: &Session, project_dir: &Path, raw: &str) -> PathBuf {
    let p = Path::new(raw);
    if p.is_absolute() {
        return p.to_path_buf();
    }
    let in_proj = project_dir.join(p);
    if in_proj.exists() {
        return in_proj;
    }
    crate::workspace::resolve_workspace_path(s, raw)
}

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

pub(crate) fn write_json_pretty(path: &Path, value: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(value).map_err(|e| bad(e.to_string()))?;
    crate::doc_state::write_file(path, format!("{text}\n"))?;
    Ok(())
}

/// Read any supported 3D scene/mesh file (`.glb`, `.gltf`, `.obj`, `.fbx`, `.stl`, `.ply`, `.3mf`)
/// via `craft-mesh` with bounded `Limits` and sidecar resolution confined to the file's directory.
pub(crate) fn read_craft_mesh_scene(path: &Path) -> Result<MeshScene> {
    let bytes = std::fs::read(path)?;
    let limits = MeshLimits::default();
    // TODO(agent): verify craft-mesh README for resolver signature and no_external availability; temporary adapter for d23f5f3 API
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
    json!({
        "path": path,
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
        "materials": scene.materials.len(),
        "images": scene.images.len(),
        "cameras": scene.cameras.len(),
        "lights": scene.lights.len(),
        "warnings": scene.warnings,
    })
}

/// `mesh.inspect`: Inspect any 3D mesh/scene file (`.glb`, `.gltf`, `.obj`, `.fbx`, `.stl`, `.ply`, `.3mf`) via `craft-mesh`.
pub fn cmd_mesh_inspect(s: &mut Session, p: &Value) -> Result<Value> {
    let raw = p["path"].as_str().filter(|v| !v.trim().is_empty()).ok_or_else(|| bad("path=<mesh file> is required"))?;
    let resolved =
        if let Ok(dir) = crate::workshop::project_dir(s) { resolve_project_path(s, &dir, raw) } else { crate::workspace::resolve_workspace_path(s, raw) };
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
    let out_raw =
        p["output"].as_str().or_else(|| p["out"].as_str()).filter(|v| !v.trim().is_empty()).ok_or_else(|| bad("output=<target .glb/.obj/.stl> is required"))?;
    let src = if let Ok(dir) = crate::workshop::project_dir(s) {
        resolve_project_path(s, &dir, src_raw)
    } else {
        crate::workspace::resolve_workspace_path(s, src_raw)
    };
    let out = if let Ok(dir) = crate::workshop::project_dir(s) {
        let cand = Path::new(out_raw);
        if cand.is_absolute() { cand.to_path_buf() } else { dir.join(cand) }
    } else {
        crate::workspace::resolve_workspace_path(s, out_raw)
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
    // TODO(agent): verify craft-mesh write API README for limits and mtl sidecar contract; temporary adapter for d23f5f3
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
