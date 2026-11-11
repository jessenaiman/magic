//! Dynamic-Link `EffectCraft` -> `FilmCraft` adapter (`video.link_effectcraft`,
//! `video.list_ec_comps`, `video.sync_links`, and `video.bake_link`).
//!
//! Implements `T01`, `T02`, and `T10` over the Craft Dynamic Link contract
//! (`storytold/filmcraft#388`, `storytold/filmcraft#3201`, `storytold/filmcraft#1024`).

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::cmd::character::{check_ecproj_head, ecproj_first_comp_name};
use crate::craft_hub::discovery::resolve_executable_path;
use crate::time::TICKS_PER_SECOND;
use crate::{CraftHub, EngineError, Result, Session};

const LINK_MANIFEST_REL: &str = "film/linked-compositions.json";

fn default_cache_format() -> String {
    "prores".into()
}

fn default_fps_num() -> u64 {
    24
}

fn default_fps_den() -> u64 {
    1
}

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
    #[serde(default = "default_cache_format")]
    pub cache_format: String,
    #[serde(default = "default_fps_num")]
    pub fps_num: u64,
    #[serde(default = "default_fps_den")]
    pub fps_den: u64,
    pub last_size: u64,
    pub last_mtime_ms: u64,
    pub dirty: bool,
    pub status: String,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct LinkManifest {
    pub version: u32,
    pub links: Vec<LinkedCompRecord>,
}

fn bad(msg: impl Into<String>) -> EngineError {
    EngineError::InvalidParams(msg.into())
}

fn file_size_and_mtime(path: &Path) -> Option<(u64, u64)> {
    let meta = std::fs::metadata(path).ok()?;
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_millis() as u64);
    Some((meta.len(), mtime))
}

fn manifest_path(project_dir: &Path) -> PathBuf {
    project_dir.join(LINK_MANIFEST_REL)
}

fn load_manifest(project_dir: &Path) -> LinkManifest {
    let path = manifest_path(project_dir);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str::<LinkManifest>(&raw).ok())
        .unwrap_or(LinkManifest {
            version: 2,
            links: Vec::new(),
        })
}

fn save_manifest(project_dir: &Path, manifest: &LinkManifest) -> Result<()> {
    let path = manifest_path(project_dir);
    let _ = crate::doc_state::write_json(&path, manifest)?;
    Ok(())
}

fn canonical_or_self(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// Discover every composition inside an `.ecproj` document (`T01`).
pub fn list_ecproj_comps(ecproj: &Path) -> Vec<String> {
    let mut comps: Vec<String> = Vec::new();
    if let Ok(bytes) = std::fs::read(ecproj) {
        if let Ok(doc) = serde_json::from_slice::<Value>(&bytes) {
            for item in doc["items"].as_array().into_iter().flatten() {
                let is_comp = item["kind"]
                    .as_str()
                    .is_some_and(|k| k.eq_ignore_ascii_case("comp") || k.eq_ignore_ascii_case("composition"))
                    || item.get("comp").is_some();
                if is_comp
                    && let Some(name) = item["name"].as_str().map(str::trim).filter(|s| !s.is_empty())
                    && !comps.iter().any(|c| c == name)
                {
                    comps.push(name.to_owned());
                }
            }
            for c in doc["compositions"].as_array().into_iter().flatten() {
                if let Some(name) = c["name"].as_str().map(str::trim).filter(|s| !s.is_empty())
                    && !comps.iter().any(|existing| existing == name)
                {
                    comps.push(name.to_owned());
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

fn comp_exists_in_ecproj(ecproj: &Path, comp: &str) -> bool {
    if !ecproj.is_file() {
        return false;
    }
    let comps = list_ecproj_comps(ecproj);
    comps.is_empty() || comps.iter().any(|c| c == comp)
}

fn parse_rational_fps(p: &Value) -> (u64, u64) {
    if let (Some(n), Some(d)) = (p["fps_num"].as_u64(), p["fps_den"].as_u64())
        && n > 0
        && d > 0
    {
        return (n, d);
    }
    if let Some(arr) = p["fps_rational"].as_array().or_else(|| p["fps"].as_array())
        && arr.len() == 2
        && let (Some(n), Some(d)) = (arr[0].as_u64(), arr[1].as_u64())
        && n > 0
        && d > 0
    {
        return (n, d);
    }
    if let Some(s) = p["fps_preset"].as_str().or_else(|| p["fps"].as_str()) {
        return match s.trim().to_ascii_lowercase().as_str() {
            "23.976" | "23.98" => (24_000, 1001),
            "24" => (24, 1),
            "25" => (25, 1),
            "29.97" | "29.97 df" | "29.97df" | "29.97 ndf" => (30_000, 1001),
            "30" => (30, 1),
            "50" => (50, 1),
            "59.94" | "59.94 df" | "59.94df" => (60_000, 1001),
            "60" => (60, 1),
            _ => (24, 1),
        };
    }
    if let Some(f) = p["fps"].as_f64() {
        if (f - 23.976).abs() < 0.05 {
            return (24_000, 1001);
        }
        if (f - 29.97).abs() < 0.05 {
            return (30_000, 1001);
        }
        if (f - 59.94).abs() < 0.05 {
            return (60_000, 1001);
        }
        let rounded = f.round().clamp(1.0, 120.0) as u64;
        return (rounded, 1);
    }
    (24, 1)
}

fn normalize_cache_format(raw: Option<&str>) -> (&'static str, &'static str) {
    match raw.unwrap_or("prores").trim().to_ascii_lowercase().as_str() {
        "webm" | "vp9" | "vp9-alpha" => ("webm", "webm"),
        "exr" | "openexr" | "exr32" => ("exr", "exr"),
        _ => ("prores", "mov"),
    }
}

fn track_display_name(raw: &str) -> &'static str {
    match raw.trim() {
        "V3" | "lyrics" | "Kinetic Lyrics" => super::TRACK_KINETIC_LYRICS,
        "V4" | "motion" | "Action Motion" => super::TRACK_ACTION_MOTION,
        _ => super::TRACK_CHARACTER_ANIMATION,
    }
}

fn build_link_id(ecproj: &Path, comp: &str) -> String {
    let stem = ecproj.file_stem().and_then(|s| s.to_str()).unwrap_or("comp");
    let clean_stem = crate::workspace::sanitize_asset_id(stem);
    let clean_comp = crate::workspace::sanitize_asset_id(comp);
    if clean_comp.is_empty() || clean_comp == clean_stem {
        clean_stem
    } else {
        format!("{clean_stem}-{clean_comp}")
    }
}

fn upsert_record(manifest: &mut LinkManifest, record: LinkedCompRecord) {
    if let Some(slot) = manifest.links.iter_mut().find(|r| r.link_id == record.link_id) {
        *slot = record;
    } else {
        manifest.links.push(record);
    }
}

/// Public snapshot of linked compositions with live `(size, mtime)` dirty check for `video.film` and `project.trunk`.
pub fn inspect_linked_comps(project_dir: &Path) -> Vec<LinkedCompRecord> {
    let mut manifest = load_manifest(project_dir);
    for rec in &mut manifest.links {
        match file_size_and_mtime(&rec.ecproj_path) {
            Some((sz, mt)) => {
                if sz != rec.last_size || mt != rec.last_mtime_ms {
                    rec.dirty = true;
                }
            }
            None => {
                rec.status = "offline".into();
                rec.reason = Some(format!("Missing EffectCraft project: {}", rec.ecproj_path.display()));
            }
        }
    }
    manifest.links
}

fn render_linked_cache(record: &mut LinkedCompRecord, duration_sec: f64) -> std::result::Result<(), String> {
    if resolve_executable_path("effectcraft-cli").is_none() && resolve_executable_path("effectcraft").is_none() {
        return Err("EffectCraft (`effectcraft-cli`) is not installed on PATH or ArtCraft Launcher".into());
    }
    if !record.ecproj_path.is_file() {
        return Err(format!("EffectCraft file not found: {}", record.ecproj_path.display()));
    }
    if let Err(err) = check_ecproj_head(&record.ecproj_path) {
        return Err(err.to_string());
    }
    if !record.comp.is_empty() && !comp_exists_in_ecproj(&record.ecproj_path, &record.comp) {
        return Err(format!("Composition `{}` not found inside {}", record.comp, record.ecproj_path.display()));
    }
    if let Some(parent) = record.cache_mov.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let fps_num = record.fps_num.max(1);
    let fps_den = record.fps_den.max(1);
    let fps_float = (fps_num as f64) / (fps_den as f64);
    let render_params = match record.cache_format.as_str() {
        "webm" => json!({
            "comp": record.comp,
            "output": record.cache_mov,
            "start": 0.0,
            "end": duration_sec.max(1.0),
            "frameRate": fps_float,
            "channels": "rgba",
            "format": "webm",
            "videoCodec": "vp9",
            "audioCodec": "opus",
            "resolution": "full",
        }),
        "exr" => json!({
            "comp": record.comp,
            "output": record.cache_mov,
            "start": 0.0,
            "end": duration_sec.max(1.0),
            "frameRate": fps_float,
            "channels": "rgba",
            "format": "exr",
            "depth": "32f",
            "sequence": true,
            "resolution": "full",
        }),
        _ => json!({
            "comp": record.comp,
            "output": record.cache_mov,
            "start": 0.0,
            "end": duration_sec.max(1.0),
            "frameRate": fps_float,
            "channels": "rgba",
            "format": "prores",
            "proresProfile": "4444",
            "resolution": "full",
        }),
    };
    CraftHub::render_effect_clip(&record.ecproj_path, &render_params).map_err(|e| e.to_string())?;
    let comp_opt = (!record.comp.is_empty()).then_some(record.comp.as_str());
    let _ = CraftHub::render_effect_frame(&record.ecproj_path, comp_opt, 0.0, 960, &record.preview_png);
    Ok(())
}

/// `video.list_ec_comps`: inspect all compositions and usage info in an `.ecproj` (`T01`).
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
                .map(|r| json!({"link_id": r.link_id, "comp": r.comp, "track": r.track, "status": r.status}))
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

/// `video.link_effectcraft` / `filmcraft.import_effectcraft`: dynamically link an `.ecproj` composition onto FilmCraft.
pub fn cmd_video_link_effectcraft(s: &mut Session, p: &Value) -> Result<Value> {
    let dir = crate::workshop::project_dir(s)?;
    let raw_proj = p["project"]
        .as_str()
        .or_else(|| p["path"].as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| bad("project=<path to .ecproj> is required"))?;
    let ecproj = canonical_or_self(&crate::workspace::resolve_workspace_path(s, raw_proj));
    let available_comps = list_ecproj_comps(&ecproj);
    let comp = p["comp"]
        .as_str()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
        .or_else(|| available_comps.first().cloned())
        .unwrap_or_else(|| {
            ecproj
                .file_stem()
                .and_then(|st| st.to_str())
                .unwrap_or("Main")
                .to_owned()
        });
    let track = p["track"].as_str().unwrap_or("V2").to_owned();
    let from_seconds = p["from"].as_f64().unwrap_or(0.0).max(0.0);
    let (fps_num, fps_den) = parse_rational_fps(p);
    let (cache_format, ext) = normalize_cache_format(p["format"].as_str().or_else(|| p["cache_format"].as_str()));
    let duration_sec = p["duration"]
        .as_f64()
        .or_else(|| crate::workshop::song_seconds(&dir))
        .unwrap_or(8.0);

    let sync_rate = p["sync_comp_rate"]
        .as_bool()
        .or_else(|| p["sync_fps"].as_bool())
        .unwrap_or(true);
    if sync_rate && ecproj.is_file() {
        let fps_f = (fps_num as f64) / (fps_den.max(1) as f64);
        let _ = CraftHub::run_effect_steps(
            Some(&ecproj),
            true,
            &[("comp.settings".into(), json!({"comp": comp, "frameRate": fps_f}))],
        );
    }

    let link_id = build_link_id(&ecproj, &comp);
    let cache_dir = dir.join("film/.linked-cache");
    let cache_mov = cache_dir.join(format!("{link_id}.{ext}"));
    let preview_png = cache_dir.join(format!("{link_id}.png"));
    let (sz, mt) = file_size_and_mtime(&ecproj).unwrap_or((0, 0));

    let mut manifest = load_manifest(&dir);
    let prev = manifest.links.iter().find(|r| r.link_id == link_id).cloned();
    let mut rec = LinkedCompRecord {
        link_id: link_id.clone(),
        ecproj_path: ecproj.clone(),
        comp: comp.clone(),
        track: track.clone(),
        from_seconds,
        clip_id: prev.as_ref().and_then(|r| r.clip_id),
        item_id: prev.as_ref().and_then(|r| r.item_id),
        cache_mov: cache_mov.clone(),
        preview_png: preview_png.clone(),
        cache_format: cache_format.to_owned(),
        fps_num,
        fps_den,
        last_size: sz,
        last_mtime_ms: mt,
        dirty: false,
        status: "linked".into(),
        reason: None,
    };

    if let Err(why) = render_linked_cache(&mut rec, duration_sec) {
        rec.status = "offline".into();
        rec.reason = Some(why.clone());
        upsert_record(&mut manifest, rec.clone());
        save_manifest(&dir, &manifest)?;
        return Ok(json!({
            "ok": false,
            "link_id": link_id,
            "project": ecproj,
            "comp": comp,
            "available_comps": available_comps,
            "track": track,
            "cache_format": cache_format,
            "fps_rational": [fps_num, fps_den],
            "status": "offline",
            "reason": why,
        }));
    }

    let placed = if from_seconds <= 0.001
        && matches!(
            track.as_str(),
            "V2" | "V3" | "V4" | "Character Animation" | "Kinetic Lyrics" | "Action Motion"
        ) {
        let published = super::publish_effect_track(s, track_display_name(&track), &cache_mov)?;
        rec.clip_id = published["clip"].as_u64();
        rec.item_id = published["item"].as_u64();
        published
    } else if let Some(existing_item) = rec.item_id {
        let fcproj = super::film_path(s)?;
        super::replace_item_footage(&fcproj, existing_item, &cache_mov)?
    } else {
        let placed_res = super::cmd_video_place_clip(
            s,
            &json!({
                "clip": cache_mov,
                "track": track,
                "from": from_seconds,
                "fps": (fps_num as f64) / (fps_den.max(1) as f64),
            }),
        )?;
        rec.clip_id = placed_res["clips"].as_array().and_then(|a| a.first()).and_then(Value::as_u64);
        placed_res
    };

    let (new_sz, new_mt) = file_size_and_mtime(&ecproj).unwrap_or((sz, mt));
    rec.last_size = new_sz;
    rec.last_mtime_ms = new_mt;
    upsert_record(&mut manifest, rec.clone());
    save_manifest(&dir, &manifest)?;

    Ok(json!({
        "ok": true,
        "link_id": rec.link_id,
        "project": rec.ecproj_path,
        "comp": rec.comp,
        "available_comps": available_comps,
        "track": rec.track,
        "cache_format": rec.cache_format,
        "fps_rational": [rec.fps_num, rec.fps_den],
        "clip": rec.clip_id,
        "item": rec.item_id,
        "cache_mov": rec.cache_mov,
        "preview_png": rec.preview_png,
        "status": rec.status,
        "placement": placed,
    }))
}

/// `video.sync_links`: inspect all linked `.ecproj` compositions and when `render == true`
/// re-render + swap cache footage in-place via `media.replaceFootage` preserving sequence trims.
pub fn cmd_video_sync_links(s: &mut Session, p: &Value) -> Result<Value> {
    let dir = crate::workshop::project_dir(s)?;
    let do_render = p["render"].as_bool().unwrap_or(false);
    let duration_sec = crate::workshop::song_seconds(&dir).unwrap_or(8.0);
    let mut manifest = load_manifest(&dir);
    let fcproj = super::film_path(s).ok().filter(|fp| fp.is_file());

    let mut refreshed = Vec::new();
    let mut dirty_tracks = Vec::new();

    for rec in &mut manifest.links {
        if rec.fps_num == 0 {
            rec.fps_num = 24;
            rec.fps_den = 1;
        }
        if rec.cache_format.is_empty() {
            rec.cache_format = "prores".into();
        }
        let Some((sz, mt)) = file_size_and_mtime(&rec.ecproj_path) else {
            rec.status = "offline".into();
            rec.reason = Some(format!("Missing EffectCraft project: {}", rec.ecproj_path.display()));
            dirty_tracks.push(rec.track.clone());
            continue;
        };
        let changed = sz != rec.last_size || mt != rec.last_mtime_ms || !rec.cache_mov.is_file();
        if changed {
            rec.dirty = true;
            if !dirty_tracks.contains(&rec.track) {
                dirty_tracks.push(rec.track.clone());
            }
        }
        if do_render && rec.dirty {
            match render_linked_cache(rec, duration_sec) {
                Ok(()) => {
                    if let (Some(proj), Some(item_id)) = (fcproj.as_deref(), rec.item_id) {
                        let _ = super::replace_item_footage(proj, item_id, &rec.cache_mov);
                    } else {
                        let _ = super::publish_effect_track(s, track_display_name(&rec.track), &rec.cache_mov);
                    }
                    rec.last_size = sz;
                    rec.last_mtime_ms = mt;
                    rec.dirty = false;
                    rec.status = "linked".into();
                    rec.reason = None;
                    refreshed.push(rec.link_id.clone());
                }
                Err(why) => {
                    rec.status = "offline".into();
                    rec.reason = Some(why);
                }
            }
        }
    }

    if do_render && !refreshed.is_empty() {
        dirty_tracks.retain(|t| manifest.links.iter().any(|r| &r.track == t && r.dirty));
    }
    save_manifest(&dir, &manifest)?;

    Ok(json!({
        "ok": true,
        "links": manifest.links,
        "dirty_tracks": dirty_tracks,
        "refreshed": refreshed,
    }))
}

/// `video.bake_link`: replace a linked EffectCraft composition with a permanent standalone media
/// clip (or relink back to live `.ecproj` when `unbake == true`).
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

    let mut manifest = load_manifest(&dir);
    let idx = manifest
        .links
        .iter()
        .position(|r| r.link_id == target_key || r.ecproj_path.to_string_lossy() == target_key)
        .ok_or_else(|| bad(format!("No linked EffectCraft composition matching `{target_key}`")))?;
    let mut rec = manifest.links[idx].clone();

    if !rec.cache_mov.is_file() {
        let dur = crate::workshop::song_seconds(&dir).unwrap_or(8.0);
        render_linked_cache(&mut rec, dur).map_err(bad)?;
    }

    let baked_dir = dir.join("film/baked");
    std::fs::create_dir_all(&baked_dir)?;
    let ext = rec.cache_mov.extension().and_then(|e| e.to_str()).unwrap_or("mov");
    let baked_mov = baked_dir.join(format!("{}.{ext}", rec.link_id));
    let bytes = std::fs::read(&rec.cache_mov)?;
    craft_atomic::atomic_write(&baked_mov, &bytes)?;

    if let (Ok(fcproj), Some(item_id)) = (super::film_path(s), rec.item_id)
        && fcproj.is_file()
    {
        super::replace_item_footage(&fcproj, item_id, &baked_mov)?;
    } else {
        super::publish_effect_track(s, track_display_name(&rec.track), &baked_mov)?;
    }

    manifest.links.remove(idx);
    save_manifest(&dir, &manifest)?;

    Ok(json!({
        "ok": true,
        "baked": true,
        "link_id": rec.link_id,
        "media_path": baked_mov,
        "track": rec.track,
    }))
}
