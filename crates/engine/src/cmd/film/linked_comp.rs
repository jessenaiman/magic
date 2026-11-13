//! Open animation & EffectCraft relinking (`video.link_effectcraft`, `video.list_ec_comps`,
//! `video.sync_links`, and `video.bake_link`).
//!
//! Eliminates custom `film/linked-compositions.json` sidecars: derives dirty/offline state directly
//! from filesystem `(exists, mtime)` checks across `<project>/effects/`, `<project>/assets/`, and
//! `film/<song>.fcproj`, relinking rendered footage in-place via `FilmCraft` `media.replaceFootage`
//! and optional `.otio` (`file.exportTimeline`) interchange.

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::cmd::character::{check_ecproj_head, ecproj_first_comp_name};
use crate::time::TICKS_PER_SECOND;
use crate::{EngineError, Result, Session};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LinkedCompRecord {
    pub link_id: String,
    pub ecproj_path: PathBuf,
    pub comp: String,
    pub track: String,
    pub from_seconds: f64,
    pub clip_id: Option<u64>,
    pub item_id: Option<u64>,
    pub cache_mov: PathBuf,
    pub preview_png: PathBuf,
    pub cache_format: String,
    pub fps_num: u64,
    pub fps_den: u64,
    pub last_size: u64,
    pub last_mtime_ms: u64,
    pub dirty: bool,
    pub status: String,
    pub reason: Option<String>,
}

fn bad(msg: impl Into<String>) -> EngineError {
    EngineError::InvalidParams(msg.into())
}

fn file_size_and_mtime(path: &Path) -> Option<(u64, u64)> {
    let meta = std::fs::metadata(path).ok()?;
    let mtime = meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_millis() as u64);
    Some((meta.len(), mtime))
}

fn canonical_or_self(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn is_ecproj_file(path: &Path) -> bool {
    path.is_file() && path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("ecproj"))
}

fn track_display_name(raw: &str) -> &'static str {
    match raw.trim() {
        "V3" | "lyrics" | "Kinetic Lyrics" => super::TRACK_KINETIC_LYRICS,
        "V4" | "motion" | "Action Motion" => super::TRACK_ACTION_MOTION,
        _ => super::TRACK_CHARACTER_ANIMATION,
    }
}

fn default_track_for_stem(stem: &str) -> &'static str {
    match stem {
        "lyrics" => "V3",
        "motion" | "actions" => "V4",
        _ => "V2",
    }
}

fn build_link_id(ecproj: &Path, comp: &str) -> String {
    let stem = ecproj.file_stem().and_then(|s| s.to_str()).unwrap_or("comp");
    let clean_stem = crate::workspace::sanitize_asset_id(stem);
    let clean_comp = crate::workspace::sanitize_asset_id(comp);
    if clean_comp.is_empty() || clean_comp == clean_stem { clean_stem } else { format!("{clean_stem}-{clean_comp}") }
}

fn normalize_cache_format(raw: Option<&str>) -> (&'static str, &'static str) {
    match raw.unwrap_or("prores").trim().to_ascii_lowercase().as_str() {
        "webm" | "vp9" | "vp9-alpha" => ("webm", "webm"),
        "exr" | "openexr" | "exr32" => ("exr", "exr"),
        _ => ("prores", "mov"),
    }
}

fn parse_rational_fps(p: &Value) -> (u64, u64) {
    if let (Some(n), Some(d)) = (p["fps_num"].as_u64(), p["fps_den"].as_u64())
        && n > 0
        && d > 0
    {
        return (n, d);
    }
    if let Some(s) = p["fps_preset"].as_str().or_else(|| p["fps"].as_str()) {
        return match s.trim().to_ascii_lowercase().as_str() {
            "23.976" | "23.98" => (24_000, 1001),
            "25" => (25, 1),
            "29.97" | "29.97 df" | "29.97 ndf" => (30_000, 1001),
            "30" => (30, 1),
            "50" => (50, 1),
            "59.94" => (60_000, 1001),
            "60" => (60, 1),
            _ => (24, 1),
        };
    }
    if let Some(f) = p["fps"].as_f64() {
        return ((f.round().clamp(1.0, 120.0)) as u64, 1);
    }
    (24, 1)
}

pub fn list_ecproj_comps(ecproj: &Path) -> Vec<String> {
    let mut comps: Vec<String> = Vec::new();
    if let Ok(bytes) = std::fs::read(ecproj) {
        if let Ok(doc) = serde_json::from_slice::<Value>(&bytes) {
            for arr_key in ["items", "compositions"] {
                for item in doc[arr_key].as_array().into_iter().flatten() {
                    if let Some(name) = item["name"].as_str().map(str::trim).filter(|s| !s.is_empty())
                        && !comps.iter().any(|c| c == name)
                    {
                        comps.push(name.to_owned());
                    }
                }
            }
        }
        if comps.is_empty()
            && let Ok(outline) = craft_doc::outline(&bytes, &craft_doc::Limits::default())
        {
            for page in outline.pages {
                if !page.name.trim().is_empty() && !comps.contains(&page.name) {
                    comps.push(page.name);
                }
            }
        }
    }
    if comps.is_empty()
        && let Some(first) = ecproj_first_comp_name(ecproj)
    {
        comps.push(first);
    }
    comps
}

fn companion_media_for_ecproj(project_dir: &Path, ecproj: &Path) -> PathBuf {
    let stem = ecproj.file_stem().and_then(|s| s.to_str()).unwrap_or("comp");
    let mut dirs = vec![project_dir.join("effects"), project_dir.join("film/.linked-cache"), project_dir.join("assets/animations")];
    if let Some(parent) = ecproj.parent() {
        dirs.insert(0, parent.to_path_buf());
    }
    for d in &dirs {
        for ext in ["mov", "webm", "mp4", "exr", "png"] {
            let cand = d.join(format!("{stem}.{ext}"));
            if cand.is_file() {
                return cand;
            }
        }
    }
    project_dir.join("effects").join(format!("{stem}.mov"))
}

fn find_fcproj_item_for_media(project_dir: &Path, media_path: &Path, track_code: &str) -> (Option<u64>, Option<u64>) {
    let Ok(rd) = std::fs::read_dir(project_dir.join("film")) else {
        return (None, None);
    };
    let Some(fcproj) = rd.flatten().map(|e| e.path()).find(|p| p.extension().and_then(|e| e.to_str()) == Some("fcproj")) else {
        return (None, None);
    };
    let doc = super::fcproj_doc::load_fcproj(&fcproj);
    let canon_media = canonical_or_self(media_path);
    let item_id = doc["media"].as_array().into_iter().flatten().find_map(|m| {
        let p = PathBuf::from(m["path"].as_str()?);
        (canonical_or_self(&p) == canon_media).then(|| m["item"].as_u64()).flatten()
    });
    let t_name = track_display_name(track_code);
    let clip_id = doc["sequence"]["video"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|t| t["code"].as_str() == Some(track_code) || t["name"].as_str() == Some(t_name))
        .and_then(|t| t["items"].as_array()?.first())
        .and_then(|c| c["clip"].as_u64().or_else(|| c["id"].as_u64()));
    (clip_id, item_id)
}

/// Derive linked `.ecproj` status directly from `<project>/effects/`, `<project>/assets/`, and rendered `(mtime, exists)` checks.
pub fn inspect_linked_comps(project_dir: &Path) -> Vec<LinkedCompRecord> {
    let mut ec_files: Vec<PathBuf> = Vec::new();
    for sub in ["effects", "assets/animations", "assets/characters", "lettering", "motion"] {
        let folder = project_dir.join(sub);
        for entry in std::fs::read_dir(folder).into_iter().flatten().flatten() {
            let p = entry.path();
            if is_ecproj_file(&p) && !ec_files.contains(&p) {
                ec_files.push(p);
            }
        }
    }
    ec_files.sort();

    let mut out = Vec::with_capacity(ec_files.len());
    for ecproj in ec_files {
        let stem = ecproj.file_stem().and_then(|s| s.to_str()).unwrap_or("comp");
        let comp = list_ecproj_comps(&ecproj).into_iter().next().unwrap_or_else(|| stem.to_owned());
        let link_id = build_link_id(&ecproj, &comp);
        let track = default_track_for_stem(stem).to_owned();
        let cache_mov = companion_media_for_ecproj(project_dir, &ecproj);
        let preview_png = project_dir.join("effects/.preview").join(format!("{link_id}.png"));
        let (ec_sz, ec_mtime) = file_size_and_mtime(&ecproj).unwrap_or((0, 0));
        let render_mtime = file_size_and_mtime(&cache_mov).map(|(_, mt)| mt);
        let dirty = render_mtime.is_none_or(|rm| ec_mtime > rm);
        let (clip_id, item_id) = find_fcproj_item_for_media(project_dir, &cache_mov, &track);
        let status = if cache_mov.is_file() { "linked" } else { "unrendered" };
        out.push(LinkedCompRecord {
            link_id,
            ecproj_path: ecproj,
            comp,
            track,
            from_seconds: 0.0,
            clip_id,
            item_id,
            cache_mov,
            preview_png,
            cache_format: "prores".into(),
            fps_num: 24,
            fps_den: 1,
            last_size: ec_sz,
            last_mtime_ms: ec_mtime,
            dirty,
            status: status.into(),
            reason: None,
        });
    }
    out
}

fn render_or_resolve_animation(project_dir: &Path, ecproj: &Path, comp: &str, cache_format: &str, ext: &str, force_render: bool) -> Result<PathBuf> {
    if !ecproj.exists() {
        return Err(bad(format!("Animation source not found: {}", ecproj.display())));
    }
    if !is_ecproj_file(ecproj) {
        return super::map_tracks::resolve_importable_animation_path(ecproj).ok_or_else(|| bad(format!("Unsupported animation format: {}", ecproj.display())));
    }
    check_ecproj_head(ecproj)?;
    let companion = companion_media_for_ecproj(project_dir, ecproj);
    let can_render = crate::craft_api::probe_live_app("effectcraft")["connected"] == true || crate::craft_api::effectcraft_cli_available();
    if (force_render || !companion.is_file()) && can_render {
        let stem = ecproj.file_stem().and_then(|s| s.to_str()).unwrap_or("comp");
        let out_media = project_dir.join("effects").join(format!("{stem}.{ext}"));
        if let Some(parent) = out_media.parent() {
            std::fs::create_dir_all(parent)?;
        }
        crate::craft_api::render_effectcraft_comp(
            ecproj,
            &json!({"comp": comp, "output": out_media, "format": cache_format, "proresProfile": "4444", "channels": "rgba"}),
        )
        .map_err(|e| EngineError::ExternalTool("effectcraft".into(), e))?;
        return Ok(out_media);
    }
    if companion.is_file() {
        return Ok(companion);
    }
    if force_render {
        return Err(EngineError::ExternalTool(
            "effectcraft".into(),
            format!("Cannot render {}: EffectCraft is not running on --control 9877 and `effectcraft-cli` is not on PATH", ecproj.display()),
        ));
    }
    Ok(ecproj.to_path_buf())
}

pub fn cmd_video_list_ec_comps(s: &mut Session, p: &Value) -> Result<Value> {
    let raw_proj = p["project"]
        .as_str()
        .or_else(|| p["path"].as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| bad("project=<path to .ecproj> is required"))?;
    let ecproj = canonical_or_self(&crate::workspace::resolve_workspace_path(s, raw_proj));
    if !ecproj.is_file() {
        return Err(bad(format!("EffectCraft file not found: {}", ecproj.display())));
    }
    let comps = list_ecproj_comps(&ecproj);
    let used_in: Vec<Value> = crate::workshop::project_dir(s)
        .ok()
        .map(|dir| {
            inspect_linked_comps(&dir)
                .into_iter()
                .filter(|r| canonical_or_self(&r.ecproj_path) == ecproj)
                .map(|r| json!({"link_id": r.link_id, "comp": r.comp, "track": r.track, "status": r.status, "dirty": r.dirty}))
                .collect()
        })
        .unwrap_or_default();
    Ok(json!({
        "ok": true,
        "project": ecproj,
        "compositions": comps,
        "used_in_count": used_in.len(),
        "used_in": used_in,
        "cache_formats": ["prores", "webm", "exr"],
        "ticks_per_second": TICKS_PER_SECOND,
    }))
}

pub fn cmd_video_link_effectcraft(s: &mut Session, p: &Value) -> Result<Value> {
    let dir = crate::workshop::project_dir(s)?;
    let raw_proj = p["project"]
        .as_str()
        .or_else(|| p["path"].as_str())
        .or_else(|| p["clip"].as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| bad("project=<path to .ecproj or saved animation> is required"))?;
    let ecproj = canonical_or_self(&crate::workspace::resolve_workspace_path(s, raw_proj));
    let available_comps = if is_ecproj_file(&ecproj) { list_ecproj_comps(&ecproj) } else { Vec::new() };
    let comp = p["comp"]
        .as_str()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
        .or_else(|| available_comps.first().cloned())
        .unwrap_or_else(|| ecproj.file_stem().and_then(|st| st.to_str()).unwrap_or("Main").to_owned());
    let track = p["track"].as_str().unwrap_or("V2").to_owned();
    let from_seconds = p["from"].as_f64().unwrap_or(0.0).max(0.0);
    let (fps_num, fps_den) = parse_rational_fps(p);
    let (cache_format, ext) = normalize_cache_format(p["format"].as_str().or_else(|| p["cache_format"].as_str()));
    let force_render = p["render"].as_bool().unwrap_or(false);
    let media_to_place = render_or_resolve_animation(&dir, &ecproj, &comp, cache_format, ext, force_render)?;

    let mut otio_exported: Option<PathBuf> = None;
    if p["otio"].as_bool() == Some(true) && is_ecproj_file(&ecproj) {
        let otio_path = dir.join("film").join(format!("{}.otio", build_link_id(&ecproj, &comp)));
        crate::craft_api::execute_effectcraft_command(
            Some(&ecproj),
            false,
            Some(false),
            "file.exportTimeline",
            json!({"comp": comp, "path": otio_path, "prerender": "unsupported"}),
        )
        .map_err(|e| EngineError::ExternalTool("effectcraft".into(), e))?;
        otio_exported = Some(otio_path);
    }

    let (_, existing_item) = find_fcproj_item_for_media(&dir, &media_to_place, &track);
    let placed = if let Some(item_id) = existing_item {
        let fcproj = super::film_path(s)?;
        super::replace_item_footage(&fcproj, item_id, &media_to_place)?
    } else if from_seconds <= 0.001 && matches!(track.as_str(), "V2" | "V3" | "V4" | "Character Animation" | "Kinetic Lyrics" | "Action Motion") {
        super::publish_effect_track(s, track_display_name(&track), &media_to_place)?
    } else {
        super::cmd_video_place_clip(
            s,
            &json!({"clip": media_to_place, "track": track, "from": from_seconds, "fps": (fps_num as f64) / (fps_den.max(1) as f64)}),
        )?
    };

    Ok(json!({
        "ok": true,
        "link_id": build_link_id(&ecproj, &comp),
        "project": ecproj,
        "tool": super::map_tracks::detect_animation_tool(&ecproj),
        "comp": comp,
        "available_comps": available_comps,
        "track": track,
        "cache_format": cache_format,
        "fps_rational": [fps_num, fps_den],
        "cache_mov": media_to_place,
        "otio": otio_exported,
        "placement": placed,
    }))
}

pub fn cmd_video_sync_links(s: &mut Session, p: &Value) -> Result<Value> {
    let dir = crate::workshop::project_dir(s)?;
    let do_render = p["render"].as_bool().unwrap_or(false);
    let mut links = inspect_linked_comps(&dir);
    let fcproj = super::film_path(s).ok().filter(|fp| fp.is_file());
    let mut refreshed = Vec::new();
    let mut dirty_tracks = Vec::new();

    for rec in &mut links {
        if rec.dirty && !dirty_tracks.contains(&rec.track) {
            dirty_tracks.push(rec.track.clone());
        }
        if do_render && rec.dirty {
            let media_path = render_or_resolve_animation(&dir, &rec.ecproj_path, &rec.comp, "prores", "mov", true)?;
            if let (Some(proj), Some(item_id)) = (fcproj.as_deref(), rec.item_id) {
                super::replace_item_footage(proj, item_id, &media_path)?;
            } else {
                super::publish_effect_track(s, track_display_name(&rec.track), &media_path)?;
            }
            rec.cache_mov = media_path;
            rec.dirty = false;
            rec.status = "linked".into();
            refreshed.push(rec.link_id.clone());
        }
    }
    if do_render && !refreshed.is_empty() {
        dirty_tracks.retain(|t| links.iter().any(|r| &r.track == t && r.dirty));
    }
    Ok(json!({"ok": true, "links": links, "dirty_tracks": dirty_tracks, "refreshed": refreshed}))
}

pub fn cmd_video_bake_link(s: &mut Session, p: &Value) -> Result<Value> {
    if p["unbake"].as_bool() == Some(true) {
        return cmd_video_link_effectcraft(s, p);
    }
    let dir = crate::workshop::project_dir(s)?;
    let target_key = p["link_id"]
        .as_str()
        .or_else(|| p["project"].as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| bad("link_id=<id> or project=<path> is required"))?;
    let resolved_target = canonical_or_self(&crate::workspace::resolve_workspace_path(s, target_key));
    let links = inspect_linked_comps(&dir);
    let rec = links
        .into_iter()
        .find(|r| r.link_id == target_key || canonical_or_self(&r.ecproj_path) == resolved_target)
        .ok_or_else(|| bad(format!("No EffectCraft composition matching `{target_key}`")))?;

    let source_mov =
        if rec.cache_mov.is_file() { rec.cache_mov.clone() } else { render_or_resolve_animation(&dir, &rec.ecproj_path, &rec.comp, "prores", "mov", true)? };
    let baked_dir = dir.join("film/baked");
    std::fs::create_dir_all(&baked_dir)?;
    let ext = source_mov.extension().and_then(|e| e.to_str()).unwrap_or("mov");
    let baked_mov = baked_dir.join(format!("{}.{ext}", rec.link_id));
    let bytes = std::fs::read(&source_mov)?;
    craft_atomic::atomic_write(&baked_mov, &bytes)?;

    if let (Ok(fcproj), Some(item_id)) = (super::film_path(s), rec.item_id)
        && fcproj.is_file()
    {
        super::replace_item_footage(&fcproj, item_id, &baked_mov)?;
    } else {
        super::publish_effect_track(s, track_display_name(&rec.track), &baked_mov)?;
    }
    Ok(json!({"ok": true, "baked": true, "link_id": rec.link_id, "media_path": baked_mov, "track": rec.track}))
}
