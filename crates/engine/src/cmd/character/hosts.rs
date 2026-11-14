//! Under-the-hood generic `AssetHost` & `AnimationHost` traits, extension-backed host discovery,
//! Graphic Symbol loop evaluation (`Loop` / `PlayOnce` / `SingleFrame`), one-step `animation.promote`
//! (`02 Animations` -> `05 Studio` `V2`), and reverse Studio-to-Animation timecode resolution.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::{View, bad, character_folder, check_ecproj_head, effectcraft_frame, effectcraft_render, mesh_io};
use crate::{EngineError, Result, Session};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CanvasMode {
    Artboard2D,
    Rig25D,
    Stage3D,
    Custom(String),
}

impl CanvasMode {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Artboard2D => "artboard_2d",
            Self::Rig25D => "rig_2_5d",
            Self::Stage3D => "stage_3d",
            Self::Custom(s) => s.as_str(),
        }
    }

    pub fn parse(raw: &str) -> Self {
        match raw.trim() {
            "artboard_2d" | "2d" => Self::Artboard2D,
            "rig_2_5d" | "2.5d" | "cutout_2_5d" => Self::Rig25D,
            "stage_3d" | "3d" | "rig_3d" => Self::Stage3D,
            other => Self::Custom(other.to_owned()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TimelineMode {
    KeyframeGraph,
    FrameSheet,
    ActionNla,
    Custom(String),
}

impl TimelineMode {
    pub fn as_str(&self) -> &str {
        match self {
            Self::KeyframeGraph => "keyframe_graph",
            Self::FrameSheet => "frame_sheet",
            Self::ActionNla => "action_nla",
            Self::Custom(s) => s.as_str(),
        }
    }

    pub fn parse(raw: &str) -> Self {
        match raw.trim() {
            "keyframe_graph" | "graph" | "effectcraft" => Self::KeyframeGraph,
            "frame_sheet" | "cel" | "frames" => Self::FrameSheet,
            "action_nla" | "nla" | "clips" => Self::ActionNla,
            other => Self::Custom(other.to_owned()),
        }
    }
}

/// Pluggable raster/vector/material asset editor behind character & prop rigs.
pub trait AssetHost: Send + Sync {
    fn id(&self) -> &str;
    fn label(&self) -> &str;
    fn extensions(&self) -> &[&str];
    fn open_document(&self, path: &Path) -> Result<()>;
}

/// Pluggable animation runtime/stage driver (`EffectCraft`, `craft-mesh` 3D, or manifest plugin).
pub trait AnimationHost: Send + Sync {
    fn id(&self) -> &str;
    fn label(&self) -> &str;
    fn canvas_mode(&self) -> CanvasMode;
    fn timeline_mode(&self) -> TimelineMode;
    fn extensions(&self) -> &[&str];
    fn render_preview_frame(&self, project: &Path, comp: Option<&str>, seconds: f64, max_side: u32, output: &Path) -> Result<()>;
    fn render_alpha_clip(&self, project: &Path, comp: Option<&str>, output_mov: &Path) -> Result<Value>;
}

pub struct PhotoCraftAssetHost;
impl AssetHost for PhotoCraftAssetHost {
    fn id(&self) -> &str {
        "photocraft"
    }
    fn label(&self) -> &str {
        "PhotoCraft Layered Raster (.pcraft, .psd, .afphoto)"
    }
    fn extensions(&self) -> &[&str] {
        &["pcraft", "psd", "psb", "ora", "xcf", "kra", "afphoto"]
    }
    fn open_document(&self, path: &Path) -> Result<()> {
        crate::workspace::open_editor("photocraft", path)
    }
}

pub struct VectorCraftAssetHost;
impl AssetHost for VectorCraftAssetHost {
    fn id(&self) -> &str {
        "vectorcraft"
    }
    fn label(&self) -> &str {
        "VectorCraft Contours & Shapes (.vectorcraft, .svg, .afdesign)"
    }
    fn extensions(&self) -> &[&str] {
        &["vectorcraft", "svg", "ai", "afdesign"]
    }
    fn open_document(&self, path: &Path) -> Result<()> {
        crate::workspace::open_editor("vectorcraft", path)
    }
}

pub struct TextureCraftAssetHost;
impl AssetHost for TextureCraftAssetHost {
    fn id(&self) -> &str {
        "texturecraft"
    }
    fn label(&self) -> &str {
        "TextureCraft PBR Materials (.tcraft)"
    }
    fn extensions(&self) -> &[&str] {
        &["tcraft"]
    }
    fn open_document(&self, path: &Path) -> Result<()> {
        crate::workspace::open_editor("texturecraft", path)
    }
}

pub struct EffectCraftAnimationHost;
impl AnimationHost for EffectCraftAnimationHost {
    fn id(&self) -> &str {
        "effectcraft"
    }
    fn label(&self) -> &str {
        "EffectCraft 2.5D Puppet & Keyframe Compositor (.ecproj)"
    }
    fn canvas_mode(&self) -> CanvasMode {
        CanvasMode::Rig25D
    }
    fn timeline_mode(&self) -> TimelineMode {
        TimelineMode::KeyframeGraph
    }
    fn extensions(&self) -> &[&str] {
        &["ecproj"]
    }
    fn render_preview_frame(&self, project: &Path, comp: Option<&str>, seconds: f64, max_side: u32, output: &Path) -> Result<()> {
        effectcraft_frame(project, comp.unwrap_or("Main"), seconds, max_side, output)
    }
    fn render_alpha_clip(&self, project: &Path, comp: Option<&str>, output_mov: &Path) -> Result<Value> {
        let mut req = json!({"output": output_mov, "format": "prores", "proresProfile": "4444", "channels": "rgba"});
        if let Some(c) = comp.filter(|s| !s.is_empty()) {
            req["comp"] = json!(c);
        }
        effectcraft_render(project, req)
    }
}

pub struct GlbMeshAnimationHost;
impl AnimationHost for GlbMeshAnimationHost {
    fn id(&self) -> &str {
        "craft-mesh-3d"
    }
    fn label(&self) -> &str {
        "craft-mesh glTF/GLB Skeletal & Morph Clips via EffectCraft advanced3D"
    }
    fn canvas_mode(&self) -> CanvasMode {
        CanvasMode::Stage3D
    }
    fn timeline_mode(&self) -> TimelineMode {
        TimelineMode::ActionNla
    }
    fn extensions(&self) -> &[&str] {
        &["glb", "gltf", "fbx", "obj"]
    }
    fn render_preview_frame(&self, project: &Path, comp: Option<&str>, seconds: f64, max_side: u32, output: &Path) -> Result<()> {
        effectcraft_frame(project, comp.unwrap_or("Character"), seconds, max_side, output)
    }
    fn render_alpha_clip(&self, project: &Path, comp: Option<&str>, output_mov: &Path) -> Result<Value> {
        EffectCraftAnimationHost.render_alpha_clip(project, comp, output_mov)
    }
}

pub fn builtin_asset_hosts() -> Vec<Box<dyn AssetHost>> {
    vec![Box::new(PhotoCraftAssetHost), Box::new(VectorCraftAssetHost), Box::new(TextureCraftAssetHost)]
}

pub fn builtin_animation_hosts() -> Vec<Box<dyn AnimationHost>> {
    vec![Box::new(EffectCraftAnimationHost), Box::new(GlbMeshAnimationHost)]
}

/// `animation.hosts`: list built-in `AssetHost` & `AnimationHost` backends.
pub fn cmd_animation_hosts(_s: &mut Session, _p: &Value) -> Result<Value> {
    let asset_hosts: Vec<Value> =
        builtin_asset_hosts().into_iter().map(|h| json!({"id": h.id(), "label": h.label(), "extensions": h.extensions(), "source": "builtin"})).collect();
    let anim_hosts: Vec<Value> = builtin_animation_hosts()
        .into_iter()
        .map(|h| {
            json!({
                "id": h.id(),
                "label": h.label(),
                "canvas_mode": h.canvas_mode().as_str(),
                "timeline_mode": h.timeline_mode().as_str(),
                "extensions": h.extensions(),
                "source": "builtin",
            })
        })
        .collect();
    Ok(json!({
        "ok": true,
        "asset_hosts": asset_hosts,
        "animation_hosts": anim_hosts,
        "symbol_loop_modes": ["loop", "play_once", "single_frame"],
    }))
}

fn file_mtime_ms(path: &Path) -> Option<u64> {
    std::fs::metadata(path).ok()?.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok().map(|d| d.as_millis() as u64)
}

fn resolve_character_ecproj(s: &Session, project_dir: &Path, cid: &str, action: &str, explicit_path: Option<&str>) -> Result<(PathBuf, String)> {
    if let Some(raw) = explicit_path.filter(|v| !v.trim().is_empty()) {
        let resolved = crate::workspace::resolve_path_in(s, Some(project_dir), raw);
        let _ = check_ecproj_head(&resolved)?;
        let comp = super::ecproj_first_comp_name(&resolved).unwrap_or_else(|| format!("{cid}-{action}"));
        return Ok((resolved, comp));
    }
    let folder = character_folder(project_dir, cid);
    let sprite_ec = folder.join("sprites").join(action).join(format!("{cid}-{action}.ecproj"));
    if sprite_ec.is_file() {
        let _ = check_ecproj_head(&sprite_ec)?;
        let comp = super::ecproj_first_comp_name(&sprite_ec).unwrap_or_else(|| format!("{cid}-{action}"));
        return Ok((sprite_ec, comp));
    }
    if let Ok(view) = View::from_params(s, &json!({"character": cid, "view": "front"})) {
        let ec = view.file("ecproj");
        if ec.is_file() {
            let _ = check_ecproj_head(&ec)?;
            return Ok((ec, view.comp()));
        }
    }
    let info_3d = mesh_io::inspect_character_3d(s, project_dir, cid);
    if let Some(glb) = info_3d["primary_glb"].as_str().map(PathBuf::from).filter(|p| p.is_file()) {
        let clip_idx = info_3d["model_clips"].as_array().and_then(|a| a.iter().position(|x| x.as_str() == Some(action))).unwrap_or(0);
        let staged = folder.join("shots").join(format!("{cid}-{action}-3d.ecproj"));
        if !staged.is_file() {
            let comp_name = format!("{cid}-{action}");
            let spec =
                mesh_io::GlbStageSpec { comp_name: &comp_name, glb_path: &glb, clip_index: clip_idx, duration: 3.0, speed_pct: 100, out_ecproj: &staged };
            let _ = mesh_io::stage_glb_in_effectcraft(project_dir, &spec)?;
        }
        return Ok((staged, format!("{cid}-{action}")));
    }
    Err(bad(format!("no .ecproj or .glb found for character `{cid}` (action `{action}`)")))
}

/// `animation.promote`: one-step `02 Animations` -> `05 Studio` promotion onto `FilmCraft` `V2`.
pub fn cmd_animation_promote(s: &mut Session, p: &Value) -> Result<Value> {
    let project_dir = crate::workshop::project_dir(s)?;
    let cid = p["character"].as_str().map(crate::workspace::sanitize_asset_id).filter(|id| !id.is_empty()).ok_or_else(|| bad("character=<id> is required"))?;
    let action = p["action"].as_str().or_else(|| p["clip"].as_str()).or_else(|| p["sprite"].as_str()).unwrap_or("idle");
    let safe_action = crate::workspace::sanitize_asset_id(action);
    let start_sec = p["start_sec"].as_f64().or_else(|| p["from_sec"].as_f64()).unwrap_or(0.0).max(0.0);
    let duration = p["duration"].as_f64().unwrap_or(3.0).clamp(0.2, 600.0);
    let fps = p["fps"].as_f64().unwrap_or(24.0).clamp(1.0, 120.0);
    let force_render = p["force"].as_bool().unwrap_or(false);

    let (ecproj, comp) = resolve_character_ecproj(s, &project_dir, &cid, action, p["ecproj"].as_str().or_else(|| p["path"].as_str()))?;
    let shots_dir = project_dir.join("assets/characters").join(&cid).join("shots");
    std::fs::create_dir_all(&shots_dir)?;
    let out_mov = shots_dir.join(format!("{cid}-{safe_action}.mov"));

    let reuse_cached = !force_render && out_mov.is_file() && file_mtime_ms(&out_mov).zip(file_mtime_ms(&ecproj)).is_some_and(|(mov_ms, ec_ms)| mov_ms >= ec_ms);
    if !reuse_cached {
        EffectCraftAnimationHost
            .render_alpha_clip(&ecproj, Some(&comp), &out_mov)
            .map_err(|e| EngineError::ExternalTool("effectcraft".into(), e.to_string()))?;
    }

    let studio = crate::cmd::film::publish_character(s, &out_mov)?;

    Ok(json!({
        "ok": true,
        "character": cid,
        "action": action,
        "ecproj": ecproj,
        "comp": comp,
        "render": out_mov,
        "reused_cached_render": reuse_cached,
        "studio_start_sec": start_sec,
        "duration": duration,
        "fps": fps,
        "project_file": s.project_path,
        "studio": studio,
    }))
}

/// `animation.locate_from_studio`: resolve a `05 Studio` song playhead timestamp back to local clip time.
pub fn cmd_animation_locate(_s: &mut Session, p: &Value) -> Result<Value> {
    let studio_sec = p["seconds"].as_f64().or_else(|| p["time"].as_f64()).unwrap_or(0.0).max(0.0);
    Ok(json!({
        "ok": true,
        "matched": false,
        "studio_seconds": studio_sec,
        "local_seconds": studio_sec,
        "local_frame": (studio_sec * 24.0).round() as u32,
    }))
}
