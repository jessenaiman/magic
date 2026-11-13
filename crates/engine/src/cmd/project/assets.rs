//! Self-contained project asset registration (`<slug>/assets/`), native Craft preview extraction,
//! and optional `PhotoCraft` / `VectorCraft` subject isolation. Never shares assets across projects.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::{cmd_project_save, ensure_default_asset_project};
use crate::{EngineError, Result, Session};

const IMAGE_EXTENSIONS: &[&str; 36] = &[
    "png",
    "jpg",
    "jpeg",
    "webp",
    "svg",
    "svgz",
    "gif",
    "bmp",
    "tif",
    "tiff",
    "exr",
    "hdr",
    "fits",
    "qoi",
    "tga",
    "heic",
    "heif",
    "pcraft",
    "psd",
    "psb",
    "pdn",
    "ora",
    "xcf",
    "kra",
    "af",
    "afdesign",
    "afphoto",
    "afpub",
    "vectorcraft",
    "vcraft",
    "drawcraft",
    "ai",
    "eps",
    "emf",
    "wmf",
    "dxf",
];
const MODEL_EXTENSIONS: &[&str; 2] = &["glb", "gltf"];

pub(crate) fn project_index(s: &Session, params: &Value) -> Result<usize> {
    let id = params.get("id").and_then(Value::as_str).ok_or_else(|| EngineError::InvalidParams("missing project asset 'id'".into()))?;
    s.project_assets.iter().position(|a| a.id == id).ok_or_else(|| EngineError::InvalidParams(format!("project asset '{id}' is not connected")))
}

pub(crate) fn asset_role(dir: Option<&Path>, path: Option<&Path>) -> &'static str {
    let first = dir.zip(path).and_then(|(d, p)| p.strip_prefix(d.join("assets")).ok()).and_then(|rest| rest.components().next());
    match first.map(|c| c.as_os_str()) {
        Some(folder) if folder == "backgrounds" => "background",
        Some(folder) if folder == "characters" => "character",
        _ => "image",
    }
}

fn preview_is_fresh(source: &Path, preview: &Path) -> bool {
    if !preview.is_file() {
        return false;
    }
    let src_meta = if source.is_dir() { source.join("manifest.json") } else { source.to_path_buf() };
    match (std::fs::metadata(src_meta).and_then(|m| m.modified()), std::fs::metadata(preview).and_then(|m| m.modified())) {
        (Ok(sm), Ok(pm)) => pm >= sm,
        _ => true,
    }
}

fn extract_or_fallback(asset_path: &Path, preview_png: &Path, app: &'static str) -> (PathBuf, Option<String>, Option<String>) {
    let linked = Some(asset_path.to_string_lossy().into_owned());
    if preview_is_fresh(asset_path, preview_png)
        || (magic_util::media::extract_craft_preview(asset_path, preview_png).unwrap_or(false) && preview_png.is_file())
    {
        return (preview_png.to_path_buf(), linked, Some(app.into()));
    }
    (asset_path.to_path_buf(), linked, Some(app.into()))
}

pub(crate) fn resolve_asset_project_image(project_dir: Option<&Path>, asset_path: &Path) -> (PathBuf, Option<String>, Option<String>) {
    let ext = asset_path.extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase();
    let stem = asset_path.file_stem().and_then(|n| n.to_str()).unwrap_or("asset");
    let previews_dir = project_dir.map(|d| d.join("assets/.previews")).unwrap_or_else(|| asset_path.parent().unwrap_or(Path::new(".")).join(".previews"));
    let preview_png = previews_dir.join(format!("{stem}.png"));

    match ext.as_str() {
        "pcraft" | "psd" | "psb" | "pdn" | "ora" | "xcf" | "kra" | "afphoto" | "exr" | "hdr" | "fits" | "qoi" | "tga" | "heic" | "heif" => {
            extract_or_fallback(asset_path, &preview_png, "photocraft")
        }
        "vectorcraft" | "vcraft" | "drawcraft" | "svgz" | "ai" | "eps" | "dxf" | "emf" | "wmf" | "af" | "afdesign" | "afpub" => {
            extract_or_fallback(asset_path, &preview_png, "vectorcraft")
        }
        "svg" => {
            let img = if preview_is_fresh(asset_path, &preview_png) { preview_png } else { asset_path.to_path_buf() };
            (img, Some(asset_path.to_string_lossy().into_owned()), Some("vectorcraft".into()))
        }
        "ecproj" | "ectemplate" => {
            for sib in [asset_path.with_file_name(format!("{stem}-preview.png")), asset_path.with_extension("png")] {
                if sib.is_file() {
                    return (sib, Some(asset_path.to_string_lossy().into_owned()), Some("effectcraft".into()));
                }
            }
            extract_or_fallback(asset_path, &preview_png, "effectcraft")
        }
        "fcproj" => {
            let sib = asset_path.with_extension("png");
            (if sib.is_file() { sib } else { asset_path.to_path_buf() }, Some(asset_path.to_string_lossy().into_owned()), Some("filmcraft".into()))
        }
        _ => {
            let pairs = [("pcraft", "photocraft"), ("psd", "photocraft"), ("vectorcraft", "vectorcraft"), ("ecproj", "effectcraft"), ("fcproj", "filmcraft")];
            for (cand_ext, app) in pairs {
                let cand = asset_path.with_extension(cand_ext);
                if cand.exists() {
                    return (asset_path.to_path_buf(), Some(cand.to_string_lossy().into_owned()), Some(app.into()));
                }
            }
            (asset_path.to_path_buf(), None, None)
        }
    }
}

pub fn cmd_project_list_assets(s: &mut Session, _: &Value) -> Result<Value> {
    let dir = s.project_path.as_deref().and_then(Path::parent);
    let assets = s
        .project_assets
        .iter()
        .map(|asset| {
            let mut entry = serde_json::to_value(asset).map_err(|e| EngineError::InvalidParams(e.to_string()))?;
            entry["role"] = json!(asset_role(dir, asset.path.as_deref()));
            if let Some(ref p) = asset.path {
                let (proj_img, linked_file, craft_app) = resolve_asset_project_image(dir, p);
                entry["project_image"] = json!(proj_img);
                entry["linked_file"] = json!(linked_file.unwrap_or_else(|| p.to_string_lossy().into_owned()));
                entry["craft_app"] = json!(craft_app);
                entry["outline"] = json!(magic_util::media::craft_outline_summary(p));
            }
            Ok(entry)
        })
        .collect::<Result<Vec<Value>>>()?;
    Ok(json!({"assets": assets, "active_asset": s.active_asset, "project": s.project_path}))
}

struct AssetRequest {
    path: PathBuf,
    kind: String,
    name: String,
    stem: String,
    load: bool,
    if_not_exists: bool,
    explicit_id: Option<String>,
}

fn file_kind(path: &Path) -> Option<&'static str> {
    if path.is_dir() && path.extension().is_some_and(|e| e.eq_ignore_ascii_case("pcraft")) {
        return Some("image");
    }
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    if IMAGE_EXTENSIONS.contains(&ext.as_str()) {
        Some("image")
    } else if matches!(ext.as_str(), "ecproj" | "ectemplate") {
        Some("character")
    } else if MODEL_EXTENSIONS.contains(&ext.as_str()) {
        Some("model_3d")
    } else {
        None
    }
}

fn asset_kind<'a>(requested: &'a str, path: &Path) -> Result<&'a str> {
    let found = magic_util::media::is_file_or_pcraft_bundle(path).then(|| file_kind(path)).flatten();
    match (requested, found) {
        ("auto", Some(kind)) => Ok(kind),
        ("image", Some("image")) | ("model_3d", Some("model_3d")) => Ok(requested),
        ("image", _) => Err(EngineError::InvalidParams(format!("{} is not a supported image or Craft artwork", path.display()))),
        ("model_3d", _) => Err(EngineError::InvalidParams(format!("{} is not a 3D model ({})", path.display(), MODEL_EXTENSIONS.join(", ")))),
        (kind, _) => Ok(kind),
    }
}

fn asset_request(s: &Session, params: &Value) -> Result<AssetRequest> {
    let path = params
        .get("path")
        .or_else(|| params.get("manifest"))
        .and_then(Value::as_str)
        .map(|p| crate::workspace::resolve_workspace_path(s, p))
        .ok_or_else(|| EngineError::InvalidParams("missing 'path' parameter".into()))?;
    if !path.exists() {
        return Err(EngineError::InvalidParams(format!("asset path does not exist: {}", path.display())));
    }
    let requested = params.get("kind").and_then(Value::as_str).unwrap_or("auto");
    if !["auto", "image", "poses", "turnaround", "cutout_rig", "character", "model_3d", "anim_3d", "video"].contains(&requested) {
        return Err(EngineError::InvalidParams("kind must be auto, image, video, poses, turnaround, cutout_rig, character, model_3d, or anim_3d".into()));
    }
    let kind = asset_kind(requested, &path)?.to_owned();
    let stem = path.file_stem().and_then(|st| st.to_str()).unwrap_or("asset").to_owned();
    let name = params.get("name").and_then(Value::as_str).unwrap_or(&stem).trim().to_owned();
    if name.is_empty() {
        return Err(EngineError::InvalidParams("asset name cannot be empty".into()));
    }
    Ok(AssetRequest {
        path,
        kind,
        name,
        stem,
        load: params.get("load").and_then(Value::as_bool).unwrap_or(false),
        if_not_exists: params.get("if_not_exists").and_then(Value::as_bool).unwrap_or(false),
        explicit_id: params.get("id").and_then(Value::as_str).and_then(crate::workspace::try_sanitize_asset_id),
    })
}

fn destination_folder(dir: &Path, params: &Value, source: &Path, kind: &str) -> Result<PathBuf> {
    let bucket = |key: &str| {
        params
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|n| !n.is_empty() && !n.contains(['/', '\\']) && *n != "." && *n != "..")
            .map(str::to_owned)
    };
    match (bucket("character"), bucket("background"), bucket("image")) {
        (Some(c), None, None) => return Ok(dir.join("assets/characters").join(c).join("parts")),
        (None, Some(b), None) => return Ok(dir.join("assets/backgrounds").join(b)),
        (None, None, Some(i)) => return Ok(dir.join("assets/images").join(i)),
        (None, None, None) => {}
        _ => return Err(EngineError::InvalidParams("specify at most one destination bucket (`character`, `background`, or `image`)".into())),
    }
    let ext = source.extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase();
    if kind == "model_3d" || matches!(ext.as_str(), "glb" | "gltf") {
        Ok(dir.join("assets/3d"))
    } else if matches!(ext.as_str(), "ecproj" | "ectemplate") {
        Ok(dir.join("effects"))
    } else if kind == "video" || matches!(ext.as_str(), "mp4" | "mov" | "webm" | "mkv") {
        Ok(dir.join("film"))
    } else if matches!(ext.as_str(), "vectorcraft" | "vcraft" | "drawcraft" | "svg" | "svgz" | "ai" | "eps" | "emf" | "wmf" | "dxf") {
        Ok(dir.join("assets/vectors"))
    } else {
        Ok(dir.join("assets"))
    }
}

fn copy_into_owned_assets(dir: &Path, params: &Value, source: &Path, kind: &str, overwrite: bool) -> Result<Option<PathBuf>> {
    let canon_dir = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    let canon_src = source.canonicalize().unwrap_or_else(|_| source.to_path_buf());
    if canon_src.starts_with(&canon_dir) {
        return Ok(None);
    }
    let files_dir = destination_folder(dir, params, source, kind)?;
    std::fs::create_dir_all(&files_dir)?;
    let copy = files_dir.join(source.file_name().unwrap_or_default());
    if source.is_dir() && source.extension().is_some_and(|e| e.eq_ignore_ascii_case("pcraft")) {
        crate::doc_state::atomic_copy_dir(source, &copy, overwrite)?;
    } else {
        crate::doc_state::atomic_copy_file(source, &copy, overwrite)?;
    }
    Ok(Some(copy))
}

fn unique_asset_id(s: &Session, base_id: &str, explicit: bool) -> Result<String> {
    let mut id = base_id.to_owned();
    let mut suffix = 2;
    while s.project_assets.iter().any(|a| a.id == id) || s.assets.contains_key(&id) {
        if explicit {
            return Err(EngineError::InvalidParams(format!("asset id '{id}' already exists")));
        }
        id = format!("{base_id}_{suffix}");
        suffix += 1;
    }
    Ok(id)
}

pub fn cmd_project_add_asset(s: &mut Session, params: &Value) -> Result<Value> {
    if s.project_path.is_none() && (params["isolated"].as_bool() == Some(true) || params["use_default_project"].as_bool() == Some(true)) {
        let _ = ensure_default_asset_project(s)?;
    }
    let req = asset_request(s, params)?;
    if req.if_not_exists
        && let Some(existing_id) = s.project_assets.iter().find(|a| a.path.as_deref() == Some(req.path.as_path())).map(|a| a.id.clone())
    {
        if req.load {
            cmd_project_load_asset(s, &json!({"id": existing_id}))?;
        }
        return Ok(json!({"ok": true, "asset_id": existing_id, "loaded": req.load, "assets": s.project_assets}));
    }
    let base_id = req
        .explicit_id
        .clone()
        .or_else(|| crate::workspace::try_sanitize_asset_id(&req.name))
        .unwrap_or_else(|| crate::workspace::sanitize_asset_id(&req.stem));
    let id = unique_asset_id(s, &base_id, req.explicit_id.is_some())?;
    let overwrite = params.get("overwrite").and_then(Value::as_bool).unwrap_or(false);
    let copied = match s.project_path.as_deref().and_then(Path::parent) {
        Some(dir) => copy_into_owned_assets(dir, params, &req.path, &req.kind, overwrite)?,
        None => None,
    };
    let mut stored = copied.clone().unwrap_or_else(|| req.path.clone());
    if params["remove_background"].as_bool() == Some(true) && req.kind == "image" {
        let cutout_path = stored.with_file_name(format!("{}-cutout.png", req.stem));
        crate::native_media::isolate_subject_rgba(&stored, &cutout_path, params["chroma"].as_str(), 0.8)?;
        stored = cutout_path;
    }
    if params["vectorize"].as_bool() == Some(true) && req.kind == "image" {
        let svg_path = stored.with_file_name(format!("{}.svg", req.stem));
        crate::native_media::trace_silhouette_svg(&stored, &svg_path, 1.5, params["strokes"].as_bool().unwrap_or(false))?;
        stored = svg_path;
    }
    s.project_assets.push(crate::ProjectAsset {
        id: id.clone(),
        name: req.name,
        preset: None,
        path: Some(stored.clone()),
        kind: req.kind,
        extra: Default::default(),
    });
    if req.load {
        cmd_project_load_asset(s, &json!({"id": id}))?;
    }
    if let Ok(sb_dc) = crate::cmd::storyboard::doc_path(s, &json!({}))
        && sb_dc.is_file()
    {
        let mut sb_state = crate::cmd::storyboard::load_field_state(&sb_dc);
        crate::cmd::storyboard::sync_assets_into_state(s, &mut sb_state);
        let _ = crate::cmd::storyboard::save_field_state(&sb_dc, &sb_state);
    }
    if params["save"].as_bool() == Some(true) {
        cmd_project_save(s, &json!({}))?;
    }
    Ok(json!({"ok": true, "asset_id": id, "path": stored, "loaded": req.load, "assets": s.project_assets, "copied_to": copied}))
}

pub fn cmd_project_remove_asset(s: &mut Session, params: &Value) -> Result<Value> {
    let index = project_index(s, params)?;
    let entry = s.project_assets.remove(index);
    s.assets.remove(&entry.id);
    if s.active_asset.as_deref() == Some(&entry.id) {
        s.active_asset = s.project_assets.iter().find(|e| s.assets.contains_key(&e.id)).map(|e| e.id.clone());
    }
    Ok(json!({"ok": true, "removed": entry.id, "active_asset": s.active_asset}))
}

pub fn cmd_project_update_asset(s: &mut Session, params: &Value) -> Result<Value> {
    let index = project_index(s, params)?;
    if let Some(raw_name) = params.get("name") {
        let name = raw_name.as_str().map(str::trim).filter(|n| !n.is_empty()).ok_or_else(|| EngineError::InvalidParams("asset name cannot be empty".into()))?;
        s.project_assets[index].name = name.into();
    }
    if let Some(raw_idx) = params.get("index") {
        let target = raw_idx
            .as_u64()
            .filter(|i| (*i as usize) < s.project_assets.len())
            .map(|i| i as usize)
            .ok_or_else(|| EngineError::InvalidParams("asset index out of range".into()))?;
        let entry = s.project_assets.remove(index);
        s.project_assets.insert(target, entry);
    }
    cmd_project_list_assets(s, params)
}

pub fn cmd_project_load_asset(s: &mut Session, params: &Value) -> Result<Value> {
    let entry = s.project_assets[project_index(s, params)?].clone();
    let reload = params.get("reload").and_then(Value::as_bool).unwrap_or(false);
    if !s.assets.contains_key(&entry.id) || reload {
        let path = entry.path.as_ref().ok_or_else(|| EngineError::InvalidParams(format!("project asset '{}' has no file path", entry.id)))?;
        if !path.exists() {
            return Err(EngineError::InvalidParams(format!("project asset file is missing: {}", path.display())));
        }
        let mut asset = crate::Asset { id: entry.id.clone(), path: path.clone(), ..Default::default() };
        if let Some((w, h, has_alpha)) = magic_util::media::fast_dimensions(path) {
            asset.width = w;
            asset.height = h;
            asset.has_alpha = has_alpha;
        } else if let Ok(reader) = image::ImageReader::open(path).and_then(|r| r.with_guessed_format())
            && let Ok((w, h)) = reader.into_dimensions()
        {
            asset.width = w;
            asset.height = h;
        }
        s.assets.insert(entry.id.clone(), asset);
    }
    s.active_asset = Some(entry.id.clone());
    Ok(json!({"ok": true, "asset_id": entry.id}))
}

/// Copy an external file into the open project's owned `<slug>/assets/` directory and save `<slug>.magic`.
pub fn cmd_project_save_asset(s: &mut Session, params: &Value) -> Result<Value> {
    if s.project_path.is_none() {
        return Err(EngineError::InvalidParams("no project is open; open or create a .magic project first".into()));
    }
    let mut merged = params.clone();
    merged["save"] = json!(true);
    cmd_project_add_asset(s, &merged)
}
