//! Home project navigation history (`file.recent` / `project.catalog`) backed by
//! [`crate::settings::SessionSettings`] (`general.recent_projects` in `config.toml`).

use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use serde_json::{Value, json};

/// Active configuration file storing recent projects (`config.toml` via `craft-settings`).
pub fn file_for(s: &crate::Session) -> Option<PathBuf> {
    s.settings.path()
}

pub fn file() -> Option<PathBuf> {
    crate::settings::prefs::open_store().path().map(Path::to_path_buf)
}

fn parse_recent_list(raw: &str) -> Vec<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    if let Ok(arr) = serde_json::from_str::<Vec<String>>(trimmed) {
        return arr.into_iter().filter(|s| !s.trim().is_empty()).collect();
    }
    trimmed.lines().map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned).collect()
}

pub fn list_for(s: &crate::Session) -> Vec<String> {
    parse_recent_list(&s.settings.prefs().general.recent_projects)
}

pub fn list() -> Vec<String> {
    let store = crate::settings::prefs::open_store();
    parse_recent_list(&store.get().general.recent_projects)
}

fn canonical(path: &str) -> String {
    fs::canonicalize(path).map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|_| path.to_string())
}

/// Never persist test/fixture or temp paths as the user's recent project when using a persistent user profile.
fn is_transient(path: &str) -> bool {
    path.contains("/.agent/") || path.contains("/tmp/") || path.contains("/target/") || Path::new(path).starts_with(std::env::temp_dir())
}

fn update_session(s: &crate::Session, path: &str, forget: bool) -> Result<(), String> {
    let path = canonical(path);
    if s.settings.is_persistent() && is_transient(&path) && !forget {
        return Ok(());
    }
    let mut items = list_for(s);
    items.retain(|p| canonical(p) != path);
    if !forget {
        items.insert(0, path);
        items.truncate(10);
    }
    let encoded = serde_json::to_string(&items).map_err(|e| e.to_string())?;
    let flush_now = s.control_address.is_none();
    s.settings
        .mutate(
            |prefs| {
                prefs.general.recent_projects = encoded;
            },
            flush_now,
        )
        .map_err(|e| e.to_string())?;
    if s.record_user_state {
        crate::change_marker::invalidate();
    }
    Ok(())
}

pub fn record_for(s: &crate::Session, path: &str) -> Result<(), String> {
    update_session(s, path, false)
}

pub fn record(path: &str) -> Result<(), String> {
    let s = crate::Session::user_session();
    update_session(&s, path, false)
}

pub fn forget_for(s: &crate::Session, path: &str) -> Result<(), String> {
    update_session(s, path, true)
}

pub fn forget(path: &str) -> Result<(), String> {
    let s = crate::Session::user_session();
    update_session(&s, path, true)
}

pub fn clear_for(s: &crate::Session) -> Result<(), String> {
    let flush_now = s.control_address.is_none();
    s.settings
        .mutate(
            |prefs| {
                prefs.general.recent_projects.clear();
            },
            flush_now,
        )
        .map_err(|e| e.to_string())?;
    if s.record_user_state {
        crate::change_marker::invalidate();
    }
    Ok(())
}

pub fn clear() -> Result<(), String> {
    let s = crate::Session::user_session();
    clear_for(&s)
}

fn is_image_file(path: &Path) -> bool {
    path.is_file() && crate::workspace::has_extension(path, &["png", "webp", "jpg", "jpeg", "svg"])
}

fn sorted_dir_entries(dir: &Path) -> Vec<PathBuf> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).collect();
    entries.sort();
    entries
}

fn first_image_in_tree(dir: &Path, max_depth: usize) -> Option<PathBuf> {
    if !dir.is_dir() {
        return None;
    }
    let entries = sorted_dir_entries(dir);
    for entry in &entries {
        if is_image_file(entry) {
            return Some(entry.clone());
        }
    }
    if max_depth == 0 {
        return None;
    }
    for entry in entries {
        if entry.is_dir()
            && let Some(found) = first_image_in_tree(&entry, max_depth - 1)
        {
            return Some(found);
        }
    }
    None
}

fn has_ext_in_tree(dir: &Path, exts: &[&str], max_depth: usize) -> bool {
    if !dir.is_dir() {
        return false;
    }
    for entry in sorted_dir_entries(dir) {
        if entry.is_file() && crate::workspace::has_extension(&entry, exts) {
            return true;
        }
        if max_depth > 0 && entry.is_dir() && has_ext_in_tree(&entry, exts, max_depth - 1) {
            return true;
        }
    }
    false
}

fn discover_project_craft_badges(dir: &Path, doc: &Value) -> Vec<&'static str> {
    let mut badges = Vec::new();
    let has_fcproj =
        doc.pointer("/workshop/video_project").and_then(Value::as_str).is_some_and(|s| !s.is_empty()) || has_ext_in_tree(&dir.join("film"), &["fcproj"], 1);
    if has_fcproj {
        badges.push(".fcproj");
    }
    if dir.join("storyboard.designcraft").is_file() {
        badges.push(".designcraft");
    }
    let has_ecproj = dir.join("effects/lyrics.ecproj").is_file()
        || dir.join("effects/motion.ecproj").is_file()
        || dir.join("effects/animation.ecproj").is_file()
        || dir.join("lettering/lyrics.ecproj").is_file()
        || dir.join("motion/actions.ecproj").is_file()
        || has_ext_in_tree(&dir.join("animations"), &["ecproj"], 3)
        || has_ext_in_tree(&dir.join("assets/characters"), &["ecproj"], 3);
    if has_ecproj {
        badges.push(".ecproj");
    }
    if has_ext_in_tree(&dir.join("assets"), &["pcraft"], 4) {
        badges.push(".pcraft");
    } else if has_ext_in_tree(&dir.join("assets"), &["psd"], 4) {
        badges.push(".psd");
    }
    if has_ext_in_tree(&dir.join("motion"), &["vectorcraft", "svg"], 2) || has_ext_in_tree(&dir.join("assets"), &["vectorcraft", "svg", "emf", "wmf"], 3) {
        badges.push(".svg");
    }
    badges
}

fn find_character_preview(project_dir: &Path, char_dir: &Path, raw_name: &str, canonical_id: &str) -> Option<PathBuf> {
    let direct_candidates = [
        char_dir.join("front/frames").join(format!("{canonical_id}-front-000.000.png")),
        project_dir.join("assets/characters/approved-crops").join(format!("{raw_name}_from-lineup-v2.png")),
        project_dir.join("assets/characters/approved-crops").join(format!("{canonical_id}_from-lineup-v2.png")),
        char_dir.join("front").join(format!("{canonical_id}-front-drawing.png")),
        char_dir.join("front").join(format!("{raw_name}-front-drawing.png")),
        char_dir.join("sprites/sing/frames/frame_0001.png"),
        char_dir.join("sprites/jump/frames/frame_0001.png"),
        char_dir.join("sprites/walk/frames/frame_0001.png"),
        char_dir.join("sprites/run/frames/frame_0001.png"),
        char_dir.join(format!("{raw_name}-assembled.png")),
        char_dir.join(format!("{canonical_id}-assembled.png")),
        char_dir.join("rig/assembled.png"),
        char_dir.join("cutout.png"),
        char_dir.join("front.png"),
        char_dir.join("3d/breakouts").join(format!("{canonical_id}-cutout.png")),
        char_dir.join("angles/front.png"),
    ];
    for candidate in direct_candidates {
        if is_image_file(&candidate) {
            return Some(candidate);
        }
    }
    first_image_in_tree(char_dir, 2)
}

fn find_project_cover(project_dir: &Path, doc: &Value) -> Option<PathBuf> {
    for rendered_dir in [
        project_dir.join("film/frames"),
        project_dir.join("storyboard/renders"),
        project_dir.join("storyboard/.preview"),
        project_dir.join("lettering/.preview"),
        project_dir.join("motion/.preview"),
    ] {
        if let Some(found) = first_image_in_tree(&rendered_dir, 1) {
            return Some(found);
        }
    }
    for rel in ["storyboard/rail/page_0000.png", "storyboard/spreads/page_0000.png", "assets/backgrounds/cover.png"] {
        let p = project_dir.join(rel);
        if is_image_file(&p) {
            return Some(p);
        }
    }
    for asset in doc["assets"].as_array().into_iter().flatten() {
        if let Some(rel) = asset["path"].as_str() {
            let p = if Path::new(rel).is_absolute() { PathBuf::from(rel) } else { project_dir.join(rel) };
            if is_image_file(&p) {
                return Some(p);
            }
        }
    }
    first_image_in_tree(&project_dir.join("assets"), 3)
}

fn project_card(raw_path: &str) -> Value {
    let path = Path::new(raw_path);
    let exists = path.is_file();
    let doc = exists.then(|| fs::read_to_string(path).ok()).flatten().and_then(|s| serde_json::from_str::<Value>(&s).ok()).unwrap_or_else(|| json!({}));
    let dir = path.parent().unwrap_or(Path::new("."));
    let name = doc["song"]["title"]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| doc["name"].as_str().filter(|s| !s.trim().is_empty()))
        .or_else(|| path.file_stem().and_then(|s| s.to_str()))
        .unwrap_or("Untitled");
    let cover = exists.then(|| find_project_cover(dir, &doc)).flatten();
    let craft_badges = if exists { discover_project_craft_badges(dir, &doc) } else { Vec::new() };
    let mut character_previews = Vec::new();
    for (raw_name, group, entry) in crate::cmd::character::discover_character_folders(dir) {
        let cid = crate::workspace::sanitize_asset_id(&raw_name);
        let has_pcraft = has_ext_in_tree(&entry, &["pcraft"], 2);
        let has_ecproj = has_ext_in_tree(&entry, &["ecproj"], 2);
        if let Some(img) = find_character_preview(dir, &entry, &raw_name, &cid) {
            character_previews.push(json!({
                "id": cid,
                "name": raw_name,
                "group": group,
                "preview": img,
                "pcraft": has_pcraft,
                "ecproj": has_ecproj,
                "project": raw_path,
                "project_name": name,
            }));
        }
    }
    let modified = fs::metadata(path).and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs());
    json!({
        "path": raw_path,
        "name": name,
        "exists": exists,
        "directory": dir,
        "mode": doc.get("mode").cloned().unwrap_or(Value::Null),
        "audio": doc.pointer("/workshop/audio").cloned().unwrap_or(Value::Null),
        "video_project": doc.pointer("/workshop/video_project").cloned().unwrap_or(Value::Null),
        "craft_badges": craft_badges,
        "cover": cover,
        "characters": character_previews,
        "modified": modified,
    })
}

fn discover_root_magic_files(s: &crate::Session) -> Vec<String> {
    let Ok(root) = crate::project_settings::root_for(s) else {
        return Vec::new();
    };
    if !root.is_dir() {
        return Vec::new();
    }
    let mut found = Vec::new();
    for entry in sorted_dir_entries(&root) {
        if !entry.is_dir() {
            continue;
        }
        for child in sorted_dir_entries(&entry) {
            if child.is_file() && crate::workspace::has_extension(&child, &["magic"]) {
                let s_path = child.to_string_lossy().into_owned();
                if !is_transient(&s_path) {
                    found.push(s_path);
                }
            }
        }
    }
    found
}

pub fn catalog(s: &mut crate::Session, params: &Value) -> crate::Result<Value> {
    if params["clear"] == true {
        clear_for(s).map_err(crate::EngineError::InvalidParams)?;
    } else if let Some(forget_path) = params["forget"].as_str().filter(|p| !p.trim().is_empty()) {
        forget_for(s, forget_path).map_err(crate::EngineError::InvalidParams)?;
    }
    let mut seen_paths = HashSet::new();
    let mut combined_paths = Vec::new();
    for p in list_for(s).into_iter().chain(discover_root_magic_files(s)) {
        if seen_paths.insert(canonical(&p)) {
            combined_paths.push(p);
        }
    }
    let recent: Vec<Value> = combined_paths.iter().map(|p| project_card(p)).collect();
    let mut seen_chars = HashSet::new();
    let mut characters = Vec::new();
    for card in &recent {
        for ch in card["characters"].as_array().into_iter().flatten() {
            let key = format!("{}:{}", ch["project"].as_str().unwrap_or_default(), ch["id"].as_str().unwrap_or_default());
            if seen_chars.insert(key) {
                characters.push(ch.clone());
            }
        }
    }
    Ok(json!({
        "ok": true,
        "config": file_for(s),
        "count": recent.len(),
        "projects": recent,
        "recent": recent,
        "characters": characters,
    }))
}
