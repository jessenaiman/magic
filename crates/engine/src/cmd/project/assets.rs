//! Project asset registration, provenance tracking, native `PhotoCraft`/`VectorCraft` subject isolation,
//! and lightweight dimension inspection.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::{cmd_project_save, ensure_default_asset_project};
use crate::{CraftHub, EngineError, Result, Session};

const IMAGE_EXTENSIONS: &[&str; 10] = &["png", "jpg", "jpeg", "webp", "svg", "gif", "bmp", "tif", "tiff", "exr"];
const MODEL_EXTENSIONS: &[&str; 2] = &["glb", "gltf"];

pub(crate) fn project_index(s: &Session, params: &Value) -> Result<usize> {
    let id = params.get("id").and_then(Value::as_str).ok_or_else(|| EngineError::InvalidParams("missing project asset 'id'".into()))?;
    s.project_assets.iter().position(|a| a.id == id).ok_or_else(|| EngineError::InvalidParams(format!("project asset '{id}' is not connected")))
}

pub(crate) fn asset_role(dir: Option<&Path>, path: Option<&Path>) -> &'static str {
    let first = dir.zip(path).and_then(|(dir, path)| path.strip_prefix(dir.join("assets")).ok()).and_then(|rest| rest.components().next());
    match first.map(|c| c.as_os_str()) {
        Some(folder) if folder == "backgrounds" => "background",
        Some(folder) if folder == "characters" => "character",
        _ => "image",
    }
}

/// Resolve the flat project preview `.png` and linked Craft file metadata for any project asset
/// (`.pcraft` ZIP or directory bundle, `.vectorcraft`, `.ecproj`, `.fcproj`, or image with sibling Craft project).
pub(crate) fn resolve_asset_project_image(project_dir: Option<&Path>, asset_path: &Path) -> (PathBuf, Option<String>, Option<String>) {
    let ext = asset_path.extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase();
    let stem = asset_path.file_stem().and_then(|n| n.to_str()).unwrap_or("asset");
    let previews_dir = project_dir.map(|d| d.join("assets/.previews")).unwrap_or_else(|| asset_path.parent().unwrap_or(Path::new(".")).join(".previews"));
    let preview_png = previews_dir.join(format!("{stem}.png"));

    match ext.as_str() {
        "pcraft" | "psd" => {
            if craft_util::media::extract_craft_preview(asset_path, &preview_png).unwrap_or(false) && preview_png.is_file() {
                return (preview_png, Some(asset_path.to_string_lossy().into_owned()), Some("photocraft".into()));
            }
            (asset_path.to_path_buf(), Some(asset_path.to_string_lossy().into_owned()), Some("photocraft".into()))
        }
        "vectorcraft" | "vcraft" | "drawcraft" | "svg" => {
            if craft_util::media::extract_craft_preview(asset_path, &preview_png).unwrap_or(false) && preview_png.is_file() {
                return (preview_png, Some(asset_path.to_string_lossy().into_owned()), Some("vectorcraft".into()));
            }
            (asset_path.to_path_buf(), Some(asset_path.to_string_lossy().into_owned()), Some("vectorcraft".into()))
        }
        "ecproj" => {
            let sibling_png = asset_path.with_extension("png");
            let img = if sibling_png.is_file() { sibling_png } else { asset_path.to_path_buf() };
            (img, Some(asset_path.to_string_lossy().into_owned()), Some("effectcraft".into()))
        }
        "fcproj" => {
            let sibling_png = asset_path.with_extension("png");
            let img = if sibling_png.is_file() { sibling_png } else { asset_path.to_path_buf() };
            (img, Some(asset_path.to_string_lossy().into_owned()), Some("filmcraft".into()))
        }
        _ => {
            for (cand_ext, app) in
                [("pcraft", "photocraft"), ("psd", "photocraft"), ("vectorcraft", "vectorcraft"), ("ecproj", "effectcraft"), ("fcproj", "filmcraft")]
            {
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
            }
            Ok(entry)
        })
        .collect::<Result<Vec<Value>>>()?;
    Ok(json!({"assets": assets, "active_asset": s.active_asset, "project": s.project_path}))
}

fn ensure_asset_project(s: &mut Session, params: &Value) -> Result<()> {
    if s.project_path.is_none()
        && (params.get("isolated").and_then(Value::as_bool) == Some(true) || params.get("use_default_project").and_then(Value::as_bool) == Some(true))
    {
        let _ = ensure_default_asset_project(s)?;
    }
    Ok(())
}

struct AssetRequest {
    path: Option<PathBuf>,
    kind: String,
    name: String,
    stem: String,
    load: bool,
    if_not_exists: bool,
    explicit_id: Option<String>,
}

fn file_kind(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    if IMAGE_EXTENSIONS.contains(&ext.as_str()) {
        Some("image")
    } else if MODEL_EXTENSIONS.contains(&ext.as_str()) {
        Some("model_3d")
    } else {
        None
    }
}

fn asset_kind<'a>(requested: &'a str, path: &Path) -> Result<&'a str> {
    let found = path.is_file().then(|| file_kind(path)).flatten();
    match (requested, found) {
        ("auto", Some(kind)) => Ok(kind),
        ("image", Some("image")) | ("model_3d", Some("model_3d")) => Ok(requested),
        ("image", _) => Err(EngineError::InvalidParams(format!("{} is not an image ({})", path.display(), IMAGE_EXTENSIONS.join(", ")))),
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
    let kind = asset_kind(requested, &path)?;
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("asset").to_owned();
    let name = params.get("name").and_then(Value::as_str).unwrap_or(&stem).trim().to_owned();
    if name.is_empty() {
        return Err(EngineError::InvalidParams("asset name cannot be empty".into()));
    }
    let explicit_id = params.get("id").and_then(Value::as_str).and_then(crate::workspace::try_sanitize_asset_id);
    Ok(AssetRequest {
        path: Some(path),
        kind: kind.to_owned(),
        name,
        stem,
        load: params.get("load").and_then(Value::as_bool).unwrap_or(false),
        if_not_exists: params.get("if_not_exists").and_then(Value::as_bool).unwrap_or(false),
        explicit_id,
    })
}

fn outside_project_image(dir: &Path, source: &Path) -> bool {
    !source.starts_with(dir) && file_kind(source) == Some("image")
}

fn asset_destination_folder(dir: &Path, params: &Value) -> Result<(PathBuf, PathBuf)> {
    let dest = |key: &str| {
        params
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|n| !n.is_empty() && !n.contains(['/', '\\']) && *n != "." && *n != "..")
            .map(str::to_owned)
    };
    match (dest("character"), dest("background"), dest("image")) {
        (Some(c), None, None) => {
            let folder = dir.join("assets/characters").join(&c);
            Ok((folder.clone(), folder.join("parts")))
        }
        (None, Some(b), None) => {
            let folder = dir.join("assets/backgrounds").join(&b);
            Ok((folder.clone(), folder))
        }
        (None, None, Some(name)) => {
            let folder = dir.join("assets/images").join(name);
            Ok((folder.clone(), folder))
        }
        _ => Err(EngineError::InvalidParams(
            "an image from outside the project needs one destination: \
             character=<name> (copied to assets/characters/<name>/parts/) or \
             background=<name> (copied to assets/backgrounds/<name>/) or image=<name> (copied to assets/images/<name>/)"
                .into(),
        )),
    }
}

fn copy_destination(s: &Session, params: &Value, path: &Option<PathBuf>) -> Result<Option<(PathBuf, PathBuf)>> {
    let Some(dir) = s.project_path.as_ref().and_then(|p| p.parent()) else {
        return Ok(None);
    };
    let Some(source) = path.as_ref() else {
        return Ok(None);
    };
    if !outside_project_image(dir, source) {
        return Ok(None);
    }
    asset_destination_folder(dir, params).map(Some)
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

fn record_provenance(folder: &Path, copy: &Path, source: &Path, meta: &std::fs::Metadata) -> Result<()> {
    let unix = |t: std::time::SystemTime| t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let provenance = folder.join("provenance.json");
    let mut doc: Value = std::fs::read_to_string(&provenance).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_else(|| json!({}));
    if !doc["files"].is_array() {
        doc["files"] = json!([]);
    }
    let assets_dir = folder.ancestors().nth(2).unwrap_or(folder);
    doc["files"].as_array_mut().ok_or_else(|| EngineError::InvalidParams("Invalid provenance files list".into()))?.push(json!({
        "file": copy.strip_prefix(assets_dir).unwrap_or(copy),
        "source": source,
        "bytes": meta.len(),
        "modified_unix": meta.modified().map(unix).unwrap_or(0),
        "copied_unix": unix(std::time::SystemTime::now()),
    }));
    craft_atomic::atomic_write(&provenance, format!("{doc:#}\n").as_bytes())?;
    Ok(())
}

fn copy_asset_file(folder: &Path, files: &Path, source: &Path) -> Result<PathBuf> {
    let copy = files.join(source.file_name().unwrap_or_default());
    if copy.exists() {
        return Err(EngineError::InvalidParams(format!("{} is already in the project; connect that file instead", copy.display())));
    }
    std::fs::create_dir_all(files)?;
    std::fs::copy(source, &copy)?;
    let meta = std::fs::metadata(source)?;
    record_provenance(folder, &copy, source, &meta)?;
    Ok(copy)
}

fn reuse_existing_asset(s: &mut Session, path: &Option<PathBuf>, load: bool, if_not_exists: bool) -> Result<Option<Value>> {
    if !if_not_exists {
        return Ok(None);
    }
    let Some(existing_id) = s.project_assets.iter().find(|a| path.is_some() && a.path == *path).map(|a| a.id.clone()) else {
        return Ok(None);
    };
    if load {
        cmd_project_load_asset(s, &json!({"id": existing_id}))?;
    }
    Ok(Some(json!({"ok": true, "asset_id": existing_id, "loaded": load, "assets": s.project_assets})))
}

/// Connect an asset by path to the open project, optionally running `PhotoCraft` (`remove_background: true`)
/// or `VectorCraft` (`vectorize: true`) subject isolation before registration.
pub fn cmd_project_add_asset(s: &mut Session, params: &Value) -> Result<Value> {
    ensure_asset_project(s, params)?;
    let AssetRequest { path, kind, name, stem, load, if_not_exists, explicit_id } = asset_request(s, params)?;
    if let Some(reused) = reuse_existing_asset(s, &path, load, if_not_exists)? {
        return Ok(reused);
    }
    let base_id = explicit_id.clone().or_else(|| crate::workspace::try_sanitize_asset_id(&name)).unwrap_or_else(|| crate::workspace::sanitize_asset_id(&stem));
    let copy_into = copy_destination(s, params, &path)?;
    let id = unique_asset_id(s, &base_id, explicit_id.is_some())?;
    let mut stored = path.clone();
    if let (Some((folder, files)), Some(source)) = (&copy_into, &path) {
        stored = Some(copy_asset_file(folder, files, source)?);
    }
    if params["remove_background"].as_bool().unwrap_or(false)
        && kind == "image"
        && let Some(ref img_path) = stored.clone()
    {
        let cutout_path = img_path.with_file_name(format!("{stem}-cutout.png"));
        CraftHub::remove_background_photo(img_path, &cutout_path, params["chroma"].as_str(), 0.8)?;
        stored = Some(cutout_path);
    }
    if params["vectorize"].as_bool().unwrap_or(false)
        && kind == "image"
        && let Some(ref img_path) = stored.clone()
    {
        let svg_path = img_path.with_file_name(format!("{stem}.svg"));
        CraftHub::trace_subject_vector(img_path, &svg_path, None, true, params["strokes"].as_bool().unwrap_or(false))?;
        stored = Some(svg_path);
    }
    s.project_assets.push(crate::ProjectAsset { id: id.clone(), name, preset: None, path: stored.clone(), kind, extra: Default::default() });
    if load {
        cmd_project_load_asset(s, &json!({"id": id}))?;
    }
    if let Ok(sb_pdf) = crate::cmd::storyboard::doc_path(s, &json!({}))
        && sb_pdf.is_file()
    {
        let mut sb_state = crate::cmd::storyboard::load_field_state(&sb_pdf);
        crate::cmd::storyboard::sync_assets_into_state(s, &mut sb_state);
        let _ = crate::cmd::storyboard::save_field_state(&sb_pdf, &sb_state);
    }
    if params["save"].as_bool() == Some(true) {
        cmd_project_save(s, &json!({}))?;
    }
    Ok(json!({"ok": true, "asset_id": id, "loaded": load, "assets": s.project_assets, "copied_to": copy_into.is_some().then_some(&stored)}))
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
    let name = params
        .get("name")
        .map(|v| v.as_str().map(str::trim).filter(|n| !n.is_empty()).ok_or_else(|| EngineError::InvalidParams("asset name cannot be empty".into())))
        .transpose()?;
    let target = params
        .get("index")
        .map(|v| {
            v.as_u64()
                .filter(|i| *i < s.project_assets.len() as u64)
                .map(|i| i as usize)
                .ok_or_else(|| EngineError::InvalidParams("asset index out of range".into()))
        })
        .transpose()?;
    if let Some(name) = name {
        s.project_assets[index].name = name.into();
    }
    if let Some(target) = target {
        let entry = s.project_assets.remove(index);
        s.project_assets.insert(target, entry);
    }
    cmd_project_list_assets(s, params)
}

fn loaded_asset(entry: &crate::ProjectAsset, path: &Path) -> Result<crate::Asset> {
    if !path.exists() {
        return Err(EngineError::InvalidParams(format!("project asset file is missing: {}", path.display())));
    }
    let mut asset = crate::Asset { id: entry.id.clone(), path: path.to_path_buf(), ..Default::default() };
    if let Some((w, h, has_alpha)) = craft_util::media::fast_dimensions(path) {
        asset.width = w;
        asset.height = h;
        asset.has_alpha = has_alpha;
    } else if let Ok(reader) = image::ImageReader::open(path).and_then(|r| r.with_guessed_format())
        && let Ok((w, h)) = reader.into_dimensions()
    {
        asset.width = w;
        asset.height = h;
    }
    Ok(asset)
}

pub fn cmd_project_load_asset(s: &mut Session, params: &Value) -> Result<Value> {
    let entry = s.project_assets[project_index(s, params)?].clone();
    let reload = params.get("reload").and_then(Value::as_bool).unwrap_or(false);
    if !s.assets.contains_key(&entry.id) || reload {
        let path = entry.path.as_ref().ok_or_else(|| EngineError::InvalidParams(format!("project asset '{}' has no file path", entry.id)))?;
        let asset = loaded_asset(&entry, path)?;
        s.assets.insert(entry.id.clone(), asset);
    }
    s.active_asset = Some(entry.id.clone());
    Ok(json!({"ok": true, "asset_id": entry.id}))
}
