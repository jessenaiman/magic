//! Self-contained project file trunk (`project.trunk`) and owned file import (`project.import_file`).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::Serialize;
use serde_json::{Value, json};

use crate::cmd::film::linked_comp::{LinkedCompRecord, inspect_linked_comps};
use crate::cmd::project::cmd_project_add_asset;
use crate::cmd::project::workshop::{IMAGE_EXTENSIONS, project_dir, song_seconds};
use crate::workspace::{AUDIO_EXTENSIONS, has_extension, resolve_workspace_path};
use crate::{EngineError, ProjectAsset, Result, Session};

struct FileClass {
    group: &'static str,
    group_label: &'static str,
    group_order: u8,
    craft_priority: u8,
    badge: &'static str,
    open_label: &'static str,
    is_craft_project: bool,
}

impl FileClass {
    const fn new(
        group: &'static str,
        group_label: &'static str,
        group_order: u8,
        craft_priority: u8,
        badge: &'static str,
        open_label: &'static str,
        is_craft_project: bool,
    ) -> Self {
        Self { group, group_label, group_order, craft_priority, badge, open_label, is_craft_project }
    }
}

#[derive(Serialize)]
struct ProjectFiles {
    ok: bool,
    project_dir: PathBuf,
    count: usize,
    registered_asset_count: usize,
    linked_comp_count: usize,
    files: Vec<ProjectFile>,
    external_assets: Vec<ProjectAsset>,
    missing_assets: Vec<ProjectAsset>,
}

#[derive(Serialize)]
struct ProjectFile {
    name: String,
    path: PathBuf,
    preview_path: Option<PathBuf>,
    rel_dir: String,
    group: &'static str,
    group_label: &'static str,
    group_order: u8,
    craft_priority: u8,
    badge: &'static str,
    open_label: &'static str,
    is_craft_project: bool,
    is_image: bool,
    visual_kind: &'static str,
    current_timeline: bool,
    can_animate: bool,
    can_link_filmcraft: bool,
    linked_comp: Option<LinkedCompRecord>,
    assets: Vec<ProjectAsset>,
    size_bytes: u64,
    modified_ms: u64,
    detail: String,
}

fn visual_kind(class: &FileClass) -> &'static str {
    match class.badge {
        "FC" | "MP4" | "MOV" | "WEBM" => "film",
        "EC" => "motion",
        "DK" | "DC" => "design",
        "VCRAFT" | "SVG" | "EMF" | "WMF" => "vector",
        "PCRAFT" | "PNG" | "JPG" | "WEBP" => "image",
        "MP3" | "WAV" | "FLAC" => "audio",
        "MAGIC" => "project",
        _ if class.group == "texture_3d" => "model",
        _ => "document",
    }
}

fn classify_craft_or_video(ext: &str) -> Option<FileClass> {
    match ext {
        "magic" => Some(FileClass::new("project_files", "Project & Supporting Files", 6, 0, "MAGIC", "Open", true)),
        "deckcraft" | "pptx" | "potx" => Some(FileClass::new("deckcraft", "DeckCraft Storyboard (.deckcraft / .pptx)", 1, 1, "DK", "Open in DeckCraft", true)),
        "designcraft" | "idml" => Some(FileClass::new("designcraft", "DesignCraft Layout (.designcraft)", 1, 2, "DC", "Open in DesignCraft", true)),
        "fcproj" => Some(FileClass::new("filmcraft", "FilmCraft (.fcproj)", 0, 1, "FC", "Open in FilmCraft", true)),
        "ecproj" => Some(FileClass::new("effect_film", "EffectCraft (.ecproj)", 2, 1, "EC", "Open in EffectCraft", true)),
        "mp4" | "mov" | "webm" | "mkv" => {
            let b = if ext == "mov" {
                "MOV"
            } else if ext == "webm" {
                "WEBM"
            } else {
                "MP4"
            };
            Some(FileClass::new("effect_film", "EffectCraft (.ecproj)", 2, 3, b, "Open in FilmCraft", false))
        }
        _ => None,
    }
}

fn classify_visual_or_3d(ext: &str) -> Option<FileClass> {
    match ext {
        "pcraft" | "psd" => Some(FileClass::new("photo_vector", "PhotoCraft & VectorCraft (.pcraft / .svg)", 3, 1, "PCRAFT", "Open in PhotoCraft", true)),
        "vectorcraft" | "vcraft" | "drawcraft" => {
            Some(FileClass::new("photo_vector", "PhotoCraft & VectorCraft (.pcraft / .svg)", 3, 2, "VCRAFT", "Open in VectorCraft", true))
        }
        "svg" | "emf" | "wmf" | "dxf" => {
            let b = if ext == "emf" {
                "EMF"
            } else if ext == "wmf" {
                "WMF"
            } else {
                "SVG"
            };
            Some(FileClass::new("photo_vector", "PhotoCraft & VectorCraft (.pcraft / .svg)", 3, 3, b, "Open in VectorCraft", true))
        }
        "png" | "jpg" | "jpeg" | "webp" => {
            let b = if ext == "png" {
                "PNG"
            } else if ext == "webp" {
                "WEBP"
            } else {
                "JPG"
            };
            Some(FileClass::new("photo_vector", "PhotoCraft & VectorCraft (.pcraft / .svg)", 3, 4, b, "Open in PhotoCraft", false))
        }
        "tcraft" | "mcraft" => Some(FileClass::new("texture_3d", "3D & Texture (.tcraft / .glb)", 4, 1, "TC", "Open in TextureCraft", true)),
        "glb" | "gltf" | "blend" | "cadcraft" | "solidcraft" | "step" | "stp" => {
            let b = if matches!(ext, "glb" | "gltf") {
                "GLB"
            } else if ext == "blend" {
                "3D"
            } else {
                "CAD"
            };
            Some(FileClass::new("texture_3d", "3D & Texture (.tcraft / .glb)", 4, 2, b, "Open", matches!(ext, "cadcraft" | "solidcraft")))
        }
        _ => None,
    }
}

fn classify_file(path: &Path) -> FileClass {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase();
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_ascii_lowercase();
    if let Some(cls) = classify_craft_or_video(&ext).or_else(|| classify_visual_or_3d(&ext)) {
        return cls;
    }
    match ext.as_str() {
        "mp3" | "wav" | "flac" | "m4a" | "aac" | "ogg" => {
            let b = if ext == "wav" {
                "WAV"
            } else if ext == "flac" {
                "FLAC"
            } else {
                "MP3"
            };
            FileClass::new("audio_words", "Audio & Timed Lyrics", 5, 1, b, "Open", false)
        }
        "srt" | "lrc" | "txt" | "md" => {
            let b = if matches!(ext.as_str(), "srt" | "lrc") { "SRT" } else { "TXT" };
            FileClass::new("audio_words", "Audio & Timed Lyrics", 5, 2, b, "Open", false)
        }
        "json" if matches!(name.as_str(), "words.json" | "transcript.json" | "official_lyrics.json" | "timing-reference.json") => {
            FileClass::new("audio_words", "Audio & Timed Lyrics", 5, 3, "JSON", "Open", false)
        }
        "zip" if name == "original-project.zip" => FileClass::new("project_files", "Project & Supporting Files", 6, 1, "ZIP", "Open", false),
        _ => {
            let b = if matches!(ext.as_str(), "json" | "jsonl") { "JSON" } else { "FILE" };
            FileClass::new("project_files", "Project & Supporting Files", 6, 4, b, "Open", false)
        }
    }
}

fn first_png_in_dir(dir: &Path) -> Option<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|x| x.eq_ignore_ascii_case("png")))
        .collect();
    files.sort();
    files.into_iter().next()
}

fn resolve_craft_preview(project_dir: &Path, path: &Path, badge: &str, modified_ms: u64, linked_comp: Option<&LinkedCompRecord>) -> Option<PathBuf> {
    if let Some(lc) = linked_comp
        && lc.preview_png.is_file()
    {
        return Some(lc.preview_png.clone());
    }
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase();
    if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp" | "svg") {
        return Some(path.to_path_buf());
    }
    if matches!(ext.as_str(), "pcraft" | "vectorcraft" | "vcraft" | "drawcraft" | "deckcraft" | "designcraft") {
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("craft");
        let cache_png = project_dir.join(".preview/trunk").join(format!("{stem}-{modified_ms}.png"));
        if cache_png.is_file() || (magic_util::media::extract_craft_preview(path, &cache_png).unwrap_or(false) && cache_png.is_file()) {
            return Some(cache_png);
        }
    }
    match badge {
        "FC" => first_png_in_dir(&project_dir.join("film/frames")),
        "DK" | "DC" => first_png_in_dir(&project_dir.join("storyboard/slides")),
        "EC" => path.parent().and_then(|p| first_png_in_dir(&p.join(".preview")).or_else(|| first_png_in_dir(p))),
        _ => None,
    }
}

fn human_size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{} KB", (bytes + 512) / 1024)
    } else {
        format!("{bytes} B")
    }
}

fn format_clock(secs: f64) -> String {
    let total = secs.max(0.0).round() as u64;
    format!("{:02}:{:02}", total / 60, total % 60)
}

fn craft_envelope_detail(path: &Path, size: u64) -> String {
    let mut parts = Vec::new();
    if size <= 64 * 1024 * 1024
        && let Ok(bytes) = std::fs::read(path)
        && let Ok(info) = craft_doc::info(&bytes, &craft_doc::Limits::default())
    {
        if !info.supported {
            parts.push("newer schema".to_string());
        } else if let Ok(outline) = craft_doc::outline(&bytes, &craft_doc::Limits::default())
            && !outline.layers.is_empty()
        {
            parts.push(format!("v{} · {} layers", info.version, outline.layers.len()));
        } else {
            parts.push(format!("v{}", info.version));
        }
    }
    parts.push(human_size(size));
    parts.join(" · ")
}

fn inspect_detail(path: &Path, badge: &str, size: u64, duration_sec: Option<f64>) -> String {
    match badge {
        "PCRAFT" | "VCRAFT" | "EC" => craft_envelope_detail(path, size),
        "FC" => format!("Master cut · {}", duration_sec.map_or_else(|| human_size(size), format_clock)),
        "PNG" | "JPG" | "WEBP" => match image::image_dimensions(path) {
            Ok((w, h)) => format!("{w}×{h} · {}", human_size(size)),
            Err(_) => human_size(size),
        },
        "MP3" | "WAV" | "FLAC" => match duration_sec {
            Some(secs) => format!("{} · {}", format_clock(secs), human_size(size)),
            None => human_size(size),
        },
        _ => human_size(size),
    }
}

fn walk_project_files(root: &Path, current: &Path, visited: &mut BTreeSet<PathBuf>, out: &mut Vec<PathBuf>) {
    let Ok(canonical) = current.canonicalize() else { return };
    if !canonical.starts_with(root) || !visited.insert(canonical) {
        return;
    }
    let mut entries: Vec<PathBuf> = std::fs::read_dir(current).into_iter().flatten().flatten().map(|e| e.path()).collect();
    entries.sort();
    for entry in entries {
        if entry.file_name().and_then(|n| n.to_str()).is_some_and(|n| n == ".git" || n == ".history" || (n == ".preview" && current == root)) {
            continue;
        }
        let Ok(canon_entry) = entry.canonicalize() else { continue };
        if !canon_entry.starts_with(root) {
            continue;
        }
        if entry.is_dir() {
            walk_project_files(root, &entry, visited, out);
        } else if entry.is_file() {
            out.push(entry);
        }
    }
}

fn partition_project_assets(s: &Session, canon_dir: &Path) -> (BTreeMap<PathBuf, Vec<ProjectAsset>>, Vec<ProjectAsset>, Vec<ProjectAsset>) {
    let mut by_path = BTreeMap::<PathBuf, Vec<ProjectAsset>>::new();
    let mut external = Vec::new();
    let mut missing = Vec::new();
    for asset in &s.project_assets {
        let Some(path) = asset.path.as_ref() else {
            missing.push(asset.clone());
            continue;
        };
        let raw_str = path.to_string_lossy();
        let absolute = craft_docpath::resolve_absolute(&raw_str, s.project_path.as_deref())
            .unwrap_or_else(|_| if path.is_absolute() { path.clone() } else { canon_dir.join(path) });
        match absolute.canonicalize() {
            Ok(canonical) if canonical.starts_with(canon_dir) => by_path.entry(canonical).or_default().push(asset.clone()),
            Ok(_) => external.push(asset.clone()),
            Err(_) => missing.push(asset.clone()),
        }
    }
    (by_path, external, missing)
}

fn file_can_animate(cls: &FileClass, assets: &[ProjectAsset]) -> bool {
    matches!(cls.badge, "PCRAFT" | "VCRAFT" | "EC")
        || assets.first().is_some_and(|a| matches!(a.kind.as_str(), "image" | "model_3d" | "character" | "cutout_rig" | "poses" | "turnaround" | "anim_3d"))
}

pub fn cmd_project_trunk(s: &mut Session, _params: &Value) -> Result<Value> {
    let dir = project_dir(s)?;
    let canon_dir = dir.canonicalize().unwrap_or_else(|_| dir.clone());
    let duration_sec = song_seconds(&dir);
    let timeline = crate::cmd::film::film_path(s).ok().and_then(|p| p.canonicalize().ok());
    let linked_comps = inspect_linked_comps(&canon_dir);
    let linked_by_path: BTreeMap<PathBuf, LinkedCompRecord> =
        linked_comps.iter().map(|lc| (lc.ecproj_path.canonicalize().unwrap_or_else(|_| lc.ecproj_path.clone()), lc.clone())).collect();

    let (assets_by_path, external_assets, missing_assets) = partition_project_assets(s, &canon_dir);

    let mut discovered = Vec::new();
    walk_project_files(&canon_dir, &canon_dir, &mut BTreeSet::new(), &mut discovered);

    let mut rows = Vec::with_capacity(discovered.len());
    for path in discovered {
        let cls = classify_file(&path);
        let meta = std::fs::metadata(&path).ok();
        let size = meta.as_ref().map_or(0, std::fs::Metadata::len);
        let modified_ms = meta.and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_millis() as u64);
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
        let rel_dir = path
            .parent()
            .and_then(|p| p.strip_prefix(&canon_dir).ok())
            .map(|r| {
                let s = r.to_string_lossy().replace('\\', "/");
                if s.is_empty() { "./".to_string() } else { format!("{s}/") }
            })
            .unwrap_or_else(|| "./".to_string());
        let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
        let assets = assets_by_path.get(&canonical).cloned().unwrap_or_default();
        let linked_comp = linked_by_path.get(&canonical).cloned();
        let current_timeline = timeline.as_ref() == Some(&canonical);
        let can_animate = file_can_animate(&cls, &assets);
        let can_link_filmcraft = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("ecproj"));
        let preview_path = resolve_craft_preview(&canon_dir, &path, cls.badge, modified_ms, linked_comp.as_ref());
        let mut detail = inspect_detail(&path, cls.badge, size, duration_sec);
        if current_timeline {
            detail = format!("Current timeline · {detail}");
        }
        if let Some(ref lc) = linked_comp {
            let tag = if lc.status == "offline" { format!("Offline ({})", lc.reason.as_deref().unwrap_or("missing")) } else { format!("Linked {}", lc.track) };
            detail = format!("{tag} · {detail}");
        }
        rows.push(ProjectFile {
            name,
            is_image: has_extension(&path, IMAGE_EXTENSIONS),
            visual_kind: visual_kind(&cls),
            path,
            preview_path,
            rel_dir,
            group: cls.group,
            group_label: cls.group_label,
            group_order: cls.group_order,
            craft_priority: cls.craft_priority,
            badge: cls.badge,
            open_label: cls.open_label,
            is_craft_project: cls.is_craft_project,
            current_timeline,
            can_animate,
            can_link_filmcraft,
            linked_comp,
            assets,
            size_bytes: size,
            modified_ms,
            detail,
        });
    }

    rows.sort_by(|a, b| {
        b.current_timeline
            .cmp(&a.current_timeline)
            .then_with(|| a.group_order.cmp(&b.group_order))
            .then_with(|| a.craft_priority.cmp(&b.craft_priority))
            .then_with(|| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()))
    });

    serde_json::to_value(ProjectFiles {
        ok: true,
        project_dir: canon_dir,
        count: rows.len(),
        registered_asset_count: s.project_assets.len(),
        linked_comp_count: linked_comps.len(),
        files: rows,
        external_assets,
        missing_assets,
    })
    .map_err(|e| EngineError::InvalidParams(e.to_string()))
}

fn target_subfolder_for_ext(ext: &str) -> &'static str {
    match ext {
        "deckcraft" | "pptx" | "potx" | "designcraft" | "idml" => ".",
        "ecproj" => "effects",
        "fcproj" | "mp4" | "mov" | "webm" | "mkv" | "srt" | "lrc" => "film",
        _ if AUDIO_EXTENSIONS.contains(&ext) => "audio",
        _ => "assets",
    }
}

pub fn cmd_project_import_file(s: &mut Session, params: &Value) -> Result<Value> {
    let dir = project_dir(s)?;
    let mut raw_paths: Vec<String> = params.get("paths").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str).map(str::to_owned).collect();
    if let Some(single) = params.get("path").and_then(Value::as_str).filter(|p| !p.trim().is_empty()) {
        raw_paths.push(single.to_owned());
    }
    if raw_paths.is_empty() {
        return Err(EngineError::InvalidParams("missing 'paths' or 'path' to import".into()));
    }
    let overwrite = params.get("overwrite").and_then(Value::as_bool).unwrap_or(false);
    let mut imported = Vec::new();

    for raw in &raw_paths {
        let src = resolve_workspace_path(s, raw);
        if !src.is_file() {
            return Err(EngineError::InvalidParams(format!("file not found: {}", src.display())));
        }
        if has_extension(&src, IMAGE_EXTENSIONS) {
            let stem = src.file_stem().and_then(|st| st.to_str()).unwrap_or("artwork");
            let res = cmd_project_add_asset(s, &json!({"path": src, "kind": "image", "name": stem, "if_not_exists": true, "load": false}))?;
            imported.push(res.get("path").cloned().unwrap_or_else(|| json!(src)));
            continue;
        }
        let ext = src.extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase();
        let subfolder = target_subfolder_for_ext(&ext);
        let dest_dir = if subfolder == "." { dir.clone() } else { dir.join(subfolder) };
        std::fs::create_dir_all(&dest_dir)?;
        let dest = dest_dir.join(src.file_name().ok_or_else(|| EngineError::InvalidParams("invalid file name".into()))?);
        if src.canonicalize().ok() != dest.canonicalize().ok() {
            crate::doc_state::atomic_copy_file(&src, &dest, overwrite)?;
        }
        if matches!(ext.as_str(), "glb" | "gltf") {
            let stem = dest.file_stem().and_then(|st| st.to_str()).unwrap_or("model");
            let _ = cmd_project_add_asset(s, &json!({"path": dest, "kind": "model_3d", "name": stem, "if_not_exists": true, "load": false}));
        }
        imported.push(json!(dest));
    }

    Ok(json!({"ok": true, "imported": imported, "trunk": cmd_project_trunk(s, &json!({}))?}))
}
