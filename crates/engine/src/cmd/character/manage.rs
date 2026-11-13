//! Character listing, style classification, multi-action `.ecproj` clip discovery (`animations/`
//! and `animation-map.json`), frame/clip rendering, keyframing, and Craft app handoff.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::{
    View, bad, character_folder, check_ecproj_head, discover_character_folders, ecproj_first_comp_name, effectcraft, effectcraft_frame, effectcraft_render,
    find_dir_ecproj, find_dir_pcraft, find_dir_photo_doc, find_dir_psd, find_dir_vector_doc, find_initial_cutout, mesh_io, point, sprite, step,
};
use crate::{Result, Session};

fn rendered_pngs(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut frames: Vec<PathBuf> = std::fs::read_dir(dir)?.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "png")).collect();
    frames.sort();
    Ok(frames)
}

fn collect_view_frames(dir: &Path) -> Vec<PathBuf> {
    let frames_root = dir.join("frames");
    let mut out = rendered_pngs(&frames_root).unwrap_or_default();
    let mut subdirs: Vec<PathBuf> = std::fs::read_dir(&frames_root).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    subdirs.sort();
    for sub in subdirs {
        out.extend(rendered_pngs(&sub).unwrap_or_default());
    }
    out
}

fn read_animation_map(project_dir: &Path) -> Value {
    if let Some(sec) = crate::cmd::project::read_magic_section_in_dir(project_dir, "animation_map", "assets/animations/animation-map.json") {
        return sec;
    }
    for rel in ["animations/animation-map.json", "animation-map.json"] {
        let p = project_dir.join(rel);
        if let Ok(raw) = std::fs::read_to_string(&p)
            && let Ok(parsed) = serde_json::from_str::<Value>(&raw)
        {
            return parsed;
        }
    }
    Value::Null
}

fn collect_ecprojs_in_dir(dir: &Path, max_depth: usize, out: &mut Vec<PathBuf>) {
    if !dir.is_dir() {
        return;
    }
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).collect();
    entries.sort();
    for entry in entries {
        if entry.is_file() && entry.extension().is_some_and(|e| e.eq_ignore_ascii_case("ecproj")) {
            let stem = entry.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
            if !stem.contains("before-") && !out.contains(&entry) {
                out.push(entry);
            }
        } else if entry.is_dir() && max_depth > 0 {
            collect_ecprojs_in_dir(&entry, max_depth - 1, out);
        }
    }
}

/// Discover all `.ecproj` action variants (`miss-hayley-waving.ecproj`, `miss-hayley-running.ecproj`,
/// `oilcan-badger-rig.ecproj`, `layered_motion.ecproj`, etc.) belonging to a character across both
/// `assets/characters/**/<char>` and `animations/**/<char>`.
fn discover_character_action_ecprojs(project_dir: &Path, folder: &Path, raw_id: &str, canonical_id: &str) -> Vec<Value> {
    let mut ecprojs = Vec::new();
    collect_ecprojs_in_dir(folder, 2, &mut ecprojs);

    let proj_slug = project_dir.file_name().and_then(|n| n.to_str()).map(crate::workspace::sanitize_asset_id).unwrap_or_default();
    let mut anim_roots = vec![project_dir.join("animations"), project_dir.join("assets/animations")];
    if let Some(parent) = project_dir.parent() {
        anim_roots.push(parent.join("animations"));
    }
    for aroot in anim_roots {
        for sub in [
            aroot.join(raw_id),
            aroot.join(canonical_id),
            aroot.join("characters").join(raw_id),
            aroot.join("characters").join(canonical_id),
            aroot.join(proj_slug.replace('_', "-")).join(raw_id),
            aroot.join(&proj_slug).join(raw_id),
        ] {
            collect_ecprojs_in_dir(&sub, 2, &mut ecprojs);
        }
    }

    ecprojs
        .into_iter()
        .map(|path| {
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("rig");
            let action = stem
                .strip_prefix(&format!("{raw_id}-"))
                .or_else(|| stem.strip_prefix(&format!("{canonical_id}-")))
                .or_else(|| stem.strip_prefix(&format!("{raw_id}_")))
                .unwrap_or(stem);
            let comp = ecproj_first_comp_name(&path).unwrap_or_else(|| stem.to_owned());
            json!({
                "action": action,
                "name": stem,
                "comp": comp,
                "path": path,
            })
        })
        .collect()
}

fn extract_parts_rig_meta(parts_path: &Path) -> (Vec<Value>, Vec<Value>, Option<(f64, f64)>) {
    let Ok(text) = std::fs::read_to_string(parts_path) else {
        return (Vec::new(), Vec::new(), None);
    };
    let Ok(doc) = serde_json::from_str::<Value>(&text) else {
        return (Vec::new(), Vec::new(), None);
    };
    let canvas = doc["size"]
        .as_array()
        .filter(|a| a.len() == 2)
        .and_then(|a| Some((a[0].as_f64()?, a[1].as_f64()?)))
        .or_else(|| Some((doc["width"].as_f64()?, doc["height"].as_f64()?)));
    let mut joints = Vec::new();
    let mut pins = Vec::new();
    for part in doc["parts"].as_array().into_iter().flatten() {
        let id = part["id"].as_str().unwrap_or_default();
        if let Some(j) = point(&part["joint"]) {
            joints.push(json!({
                "id": id,
                "parent": part["parent"],
                "joint": j,
                "bounds": part["bounds"],
            }));
        }
        for pin_pt in part["pins"].as_array().into_iter().flatten() {
            if let Some(pt) = point(pin_pt) {
                pins.push(json!({ "part": id, "pin": pt }));
            }
        }
    }
    (joints, pins, canvas)
}

fn build_view_summary(dir: &Path, canonical_id: &str, raw_id: &str, view_name: &str, is_flat: bool) -> Option<Value> {
    let ecproj_path = find_dir_ecproj(dir, canonical_id, raw_id, view_name, is_flat);
    let pcraft_path = find_dir_pcraft(dir, canonical_id, raw_id, view_name, is_flat);
    let psd_path = find_dir_psd(dir, canonical_id, raw_id, view_name, is_flat);
    let photo_doc = find_dir_photo_doc(dir, canonical_id, raw_id, view_name, is_flat);
    let vector_doc = find_dir_vector_doc(dir, canonical_id, raw_id, view_name, is_flat);
    let parts_path = dir.join("parts.json");
    let has_parts = parts_path.is_file();
    let (joints, pins, canvas) = extract_parts_rig_meta(&parts_path);
    let drawing = std::fs::read_to_string(&parts_path)
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|p| p["image"].as_str().or_else(|| p["source"].as_str()).map(|i| dir.join(i)))
        .filter(|p| p.is_file())
        .or_else(|| {
            let staged = dir.join(format!("{canonical_id}-{view_name}-drawing.png"));
            staged.is_file().then_some(staged)
        })
        .or_else(|| {
            let staged = dir.join(format!("{raw_id}-{view_name}-drawing.png"));
            staged.is_file().then_some(staged)
        })
        .or_else(|| is_flat.then(|| find_initial_cutout(dir)).flatten());
    let rendered_frames = collect_view_frames(dir);
    let extracted_craft_png = if rendered_frames.is_empty() && drawing.is_none() {
        photo_doc.as_deref().or(vector_doc.as_deref()).and_then(|doc_path| {
            let out = dir.join(".preview").join(format!("{canonical_id}-{view_name}-craft.png"));
            if out.is_file() || magic_util::media::extract_craft_preview(doc_path, &out).unwrap_or(false) { out.is_file().then_some(out) } else { None }
        })
    } else {
        None
    };
    let latest_frame = rendered_frames.first().cloned().or_else(|| drawing.clone()).or(extracted_craft_png);
    let any_craft = ecproj_path.is_some() || photo_doc.is_some() || vector_doc.is_some() || has_parts || drawing.is_some();
    if is_flat && !any_craft {
        return None;
    }
    Some(json!({
        "view": view_name,
        "parts": has_parts,
        "parts_path": has_parts.then_some(parts_path),
        "drawing": drawing,
        "latest_frame": latest_frame,
        "rendered_frames": rendered_frames,
        "psd": photo_doc.is_some(),
        "pcraft": pcraft_path.is_some(),
        "pcraft_path": pcraft_path,
        "psd_path": psd_path,
        "photo_doc": photo_doc,
        "vector_doc": vector_doc,
        "ecproj": ecproj_path.is_some(),
        "ecproj_path": ecproj_path,
        "joints": joints,
        "pins": pins,
        "canvas_size": canvas.map(|(w, h)| json!([w, h])),
    }))
}

/// List characters (including nested cast folders & `animations/` action `.ecproj` clips) and their Craft files.
pub fn cmd_character_list(s: &mut Session, _: &Value) -> Result<Value> {
    let project_dir = crate::workshop::project_dir(s)?;
    let animation_map = read_animation_map(&project_dir);
    let cast: Vec<String> = s
        .project_path
        .as_ref()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|d| d["animation_cast"].as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|v| v.as_str().map(str::to_owned))
        .collect();

    let mut discovered = discover_character_folders(&project_dir);
    for asset in s.project_assets.iter().filter(|a| a.kind == "model_3d") {
        let id = asset.id.replace('_', "-");
        if !discovered.iter().any(|(raw, _, _)| crate::workspace::sanitize_asset_id(raw) == crate::workspace::sanitize_asset_id(&id)) {
            discovered.push((id.clone(), None, project_dir.join("assets/characters").join(&id)));
        }
    }

    let mut out = Vec::new();
    for (raw_name, group, folder) in discovered {
        let canonical_id = crate::workspace::sanitize_asset_id(&raw_name);
        if !cast.is_empty() && !cast.contains(&raw_name) && !cast.contains(&canonical_id) {
            continue;
        }
        let mut views: Vec<Value> = ["front", "side", "back"]
            .iter()
            .filter(|v| folder.join(v).is_dir())
            .filter_map(|v| build_view_summary(&folder.join(v), &canonical_id, &raw_name, v, false))
            .collect();
        if views.is_empty()
            && let Some(flat_view) = build_view_summary(&folder, &canonical_id, &raw_name, "front", true)
        {
            views.push(flat_view);
        }
        let action_ecprojs = discover_character_action_ecprojs(&project_dir, &folder, &raw_name, &canonical_id);
        if views.is_empty()
            && let Some(first_ec) = action_ecprojs.first()
        {
            views.push(json!({
                "view": "front",
                "parts": false,
                "drawing": find_initial_cutout(&folder),
                "latest_frame": find_initial_cutout(&folder),
                "rendered_frames": [],
                "psd": false,
                "pcraft": false,
                "ecproj": true,
                "ecproj_path": first_ec["path"],
                "joints": [],
                "pins": [],
            }));
        }
        let char_magic = folder.join("character.magic");
        let info_3d = mesh_io::inspect_character_3d(s, &project_dir, &canonical_id);
        let m3d_view = mesh_io::view_model3d_summary(&info_3d);
        if !m3d_view.is_null() {
            if views.is_empty() {
                views.push(json!({"view": "front", "parts": false, "ecproj": false, "joints": [], "pins": [], "model3d": m3d_view}));
            } else if let Some(first) = views.first_mut() {
                first["model3d"] = m3d_view;
            }
        }
        let mut sprites = sprite_summaries(&folder);
        for act in &action_ecprojs {
            let act_name = act["action"].as_str().unwrap_or("rig");
            if !sprites.iter().any(|sp| sp["name"].as_str() == Some(act_name)) {
                sprites.push(json!({
                    "name": act_name,
                    "animation_style": "cutout_2_5d",
                    "ecproj": act["path"],
                    "comp": act["comp"],
                    "frames": [],
                    "fps": 24,
                    "duration": 2.0,
                }));
            }
        }
        if !views.is_empty() || !sprites.is_empty() || !action_ecprojs.is_empty() || info_3d["available"].as_bool() == Some(true) {
            let (animation_style, animation_styles) = classify_animation_style(&views, &sprites, &info_3d);
            out.push(json!({
                "id": canonical_id,
                "name": raw_name,
                "group": group,
                "folder": folder,
                "character_magic": char_magic.is_file().then_some(char_magic),
                "action_ecprojs": action_ecprojs,
                "animation_style": animation_style,
                "animation_styles": animation_styles,
                "views": views,
                "sprites": sprites,
                "model_3d": info_3d,
            }));
        }
    }
    Ok(json!({
        "ok": true,
        "characters": out,
        "animation_map": animation_map,
    }))
}

pub fn classify_animation_style(views: &[Value], sprites: &[Value], info_3d: &Value) -> (&'static str, Vec<&'static str>) {
    let has_2_5d_rig = views.iter().any(|v| {
        v["rigged"].as_bool() == Some(true)
            || v["ecproj"].as_bool() == Some(true)
            || v["ecproj"].is_string()
            || v["cut"].as_bool() == Some(true)
            || v["psd"].as_bool() == Some(true)
            || v["pcraft"].as_bool() == Some(true)
            || v["marked"].as_bool() == Some(true)
            || v["parts"].as_bool() == Some(true)
            || v["parts"].is_array()
    });
    let has_2d_sprites = sprites.iter().any(|s| s["animation_style"].as_str() == Some("sprite_2d"));
    let has_any_sprites = !sprites.is_empty();
    let has_3d = info_3d["available"].as_bool() == Some(true);
    let is_2_5d_3d = info_3d["animation_style"].as_str() == Some("cutout_2_5d");

    let mut styles = Vec::new();
    if has_2_5d_rig || (has_3d && is_2_5d_3d) {
        styles.push("cutout_2_5d");
    }
    if has_2d_sprites || (!has_2_5d_rig && has_any_sprites) {
        styles.push("sprite_2d");
    }
    if has_3d {
        styles.push("rig_3d");
    }
    if styles.is_empty() {
        styles.push("cutout");
    }

    let primary = if has_3d && !is_2_5d_3d && !has_2_5d_rig {
        "rig_3d"
    } else if has_2_5d_rig {
        "cutout_2_5d"
    } else if has_2d_sprites || has_any_sprites {
        "sprite_2d"
    } else if has_3d {
        "rig_3d"
    } else {
        "cutout"
    };
    (primary, styles)
}

/// Render one frame of the rigged character with EffectCraft.
pub fn cmd_character_frame(s: &mut Session, p: &Value) -> Result<Value> {
    let view = View::from_params(s, p)?;
    let project = if let Some(raw_ec) = p["ecproj"].as_str().filter(|v| !v.is_empty()) {
        crate::workspace::resolve_workspace_path(s, raw_ec)
    } else {
        view.require("ecproj", "character.rig")?
    };
    let comp = ecproj_first_comp_name(&project).unwrap_or_else(|| view.comp());
    let time = p["time"].as_f64().unwrap_or(0.0).max(0.0);
    let side = p["max_side"].as_u64().unwrap_or(900).clamp(64, 4096);
    let frames = view.dir.join("frames");
    std::fs::create_dir_all(&frames)?;
    let out = frames.join(format!("{}-{}-{:07.3}.png", view.id, view.view, time));
    effectcraft_frame(&project, &comp, time, side as u32, &out)?;
    Ok(json!({"ok": true, "frame": out, "time": time, "project": project}))
}

fn render_target(s: &Session, p: &Value) -> Result<(PathBuf, PathBuf, String, String)> {
    if let Some(name) = p["sprite"].as_str() {
        let id = p["character"].as_str().map(crate::workspace::sanitize_asset_id).ok_or_else(|| bad("character=<id> is required"))?;
        let dir = sprite::sprite_dir(s, &id, name)?;
        let comp = format!("{id}-{name}");
        let project = dir.join(format!("{comp}.ecproj"));
        if !project.is_file() {
            return Err(bad(format!("{} is missing; run character.sprite first", project.display())));
        }
        return Ok((dir, project, comp.clone(), comp));
    }
    let view = View::from_params(s, p)?;
    let project = view.require("ecproj", "character.rig")?;
    let safe_stem = format!("{}-{}", view.id, view.view);
    Ok((view.dir.clone(), project, safe_stem, view.comp()))
}

pub fn cmd_character_render(s: &mut Session, p: &Value) -> Result<Value> {
    let external = p["effectcraft_project"].as_str();
    let (base, project, name, comp) = if let Some(path) = external {
        let dir = crate::workshop::project_dir(s)?;
        let project = dir.join(path).canonicalize()?;
        if !project.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("ecproj")) {
            return Err(bad("Choose an EffectCraft .ecproj file"));
        }
        let name = project.file_stem().and_then(|stem| stem.to_str()).unwrap_or("animation");
        let name = crate::workspace::sanitize_asset_id(name);
        (dir.join("film"), project, name, p["comp"].clone())
    } else {
        let (base, project, safe_stem, comp) = render_target(s, p)?;
        (base, project, safe_stem, json!(comp))
    };
    let start = p["start"].as_f64().unwrap_or(0.0).max(0.0);
    let end = p["end"].as_f64().filter(|e| *e > start).unwrap_or(start + 1.0);
    let fps = p["fps"].as_u64().unwrap_or(24).clamp(1, 60);
    let fmt_req = p["format"].as_str().unwrap_or("png");
    let clip = fmt_req == "clip" || fmt_req == "prores";
    let gif = fmt_req == "gif";
    let name = if external.is_some() {
        let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|error| bad(error.to_string()))?.as_nanos();
        format!("{name}-{timestamp}")
    } else {
        format!("{name}-{start:.2}-{end:.2}")
    };
    let (dir, out) = if clip {
        let dir = base.join("clips");
        (dir.clone(), dir.join(format!("{name}.mov")))
    } else if gif {
        let dir = base.join("clips");
        (dir.clone(), dir.join(format!("{name}.gif")))
    } else {
        let dir = base.join("frames").join(&name);
        (dir.clone(), dir.join("frame.png"))
    };
    std::fs::create_dir_all(&dir)?;
    effectcraft_render(
        &project,
        json!({
            "comp": comp,
            "output": out,
            "start": start,
            "end": end,
            "frameRate": fps,
            "channels": "rgba",
            "format": if clip { "prores" } else if gif { "gif" } else { "png" },
            "proresProfile": "4444",
            "resolution": if clip { "full" } else { "half" }
        }),
    )?;
    let frames = if clip || gif { Vec::new() } else { rendered_pngs(&dir)? };
    Ok(json!({"ok": true, "start": start, "end": end, "fps": fps, "clip": (clip || gif).then_some(&out), "frames": frames}))
}

pub fn sprite_summaries(folder: &Path) -> Vec<Value> {
    let newest = |dir: &Path, ext: &str| {
        std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|found| found == ext))
            .max_by_key(|path| std::fs::metadata(path).and_then(|m| m.modified()).ok())
    };
    let mut dirs: Vec<PathBuf> =
        std::fs::read_dir(folder.join("sprites")).into_iter().flatten().flatten().map(|entry| entry.path()).filter(|path| path.is_dir()).collect();
    dirs.sort();
    let mut out: Vec<Value> = dirs
        .iter()
        .filter_map(|dir| {
            let ecproj = newest(dir, "ecproj")?;
            let frames = rendered_pngs(&dir.join("frames")).unwrap_or_default();
            let meta: Value = std::fs::read_to_string(dir.join("sprite.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(Value::Null);
            let fps = meta["fps"].as_u64().unwrap_or(24);
            let duration = meta["cycle_seconds"].as_f64().unwrap_or_else(|| frames.len() as f64 / fps.max(1) as f64);
            let style = meta["animation_style"].as_str().unwrap_or("sprite_2d");
            Some(json!({
                "name": dir.file_name().map(|n| n.to_string_lossy().into_owned()),
                "animation_style": style,
                "ecproj": ecproj,
                "preview": newest(&dir.join("clips"), "gif"),
                "frames": frames,
                "fps": fps,
                "duration": duration,
            }))
        })
        .collect();

    let folder_name = folder.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    let id = crate::workspace::sanitize_asset_id(folder_name);
    let mut view_dirs: Vec<(PathBuf, bool, &str)> =
        ["front", "side", "back"].into_iter().filter(|v| folder.join(v).is_dir()).map(|v| (folder.join(v), false, v)).collect();
    if view_dirs.is_empty() && folder.join("frames").is_dir() {
        view_dirs.push((folder.to_path_buf(), true, "front"));
    }
    for (vdir, is_flat, vname) in view_dirs {
        let Some(ecproj) = find_dir_ecproj(&vdir, &id, folder_name, vname, is_flat) else { continue };
        let actions_map = std::fs::read_to_string(vdir.join("parts.json"))
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .and_then(|p| p.get("actions").and_then(Value::as_object).cloned())
            .unwrap_or_default();
        let mut seq_dirs: Vec<PathBuf> =
            std::fs::read_dir(vdir.join("frames")).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
        seq_dirs.sort();
        for seq_dir in seq_dirs {
            let frames = rendered_pngs(&seq_dir).unwrap_or_default();
            if frames.is_empty() {
                continue;
            }
            let raw_dir_name = seq_dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let mut cycle_name = raw_dir_name.clone();
            let mut duration = frames.len() as f64 / 24.0;
            for (act_name, spec) in &actions_map {
                if let (Some(st), Some(en)) = (spec.get("start").and_then(Value::as_f64), spec.get("end").and_then(Value::as_f64)) {
                    let suffix = format!("-{st:.2}-{en:.2}");
                    if raw_dir_name.ends_with(&suffix) {
                        cycle_name = act_name.clone();
                        duration = (en - st).max(0.1);
                        break;
                    }
                }
            }
            if out.iter().any(|existing| existing["name"].as_str() == Some(cycle_name.as_str())) {
                continue;
            }
            out.push(json!({
                "name": cycle_name,
                "animation_style": "cutout_2_5d",
                "ecproj": ecproj,
                "preview": newest(&vdir.join("clips"), "gif"),
                "frames": frames,
                "fps": 24,
                "duration": duration,
            }));
        }
    }
    out
}

fn animation_documents(s: &Session) -> Result<Vec<Value>> {
    let root = crate::workshop::project_dir(s)?;
    let mut paths = Vec::new();
    for folder in ["effects", "motion", "lettering", "animations", "assets/animations"] {
        collect_ecprojs_in_dir(&root.join(folder), 3, &mut paths);
    }
    Ok(paths.iter().map(|path| json!({"name": path.file_stem().map(|n| n.to_string_lossy().into_owned()), "path": path})).collect())
}

pub fn cmd_animation_view(s: &mut Session, p: &Value) -> Result<Value> {
    let list = cmd_character_list(s, &json!({}))?;
    let all = list["characters"].as_array().cloned().unwrap_or_default();
    let views_of = |c: &Value| c["views"].as_array().into_iter().flatten().filter_map(|v| v["view"].as_str().map(str::to_owned)).collect::<Vec<_>>();
    let characters: Vec<Value> = all
        .iter()
        .map(|c| {
            json!({
                "id": c["id"],
                "group": c["group"],
                "animation_style": c["animation_style"],
                "animation_styles": c["animation_styles"],
                "action_ecprojs": c["action_ecprojs"],
                "views": views_of(c),
            })
        })
        .collect();
    let documents = animation_documents(s)?;
    let chosen = p["character"]
        .as_str()
        .and_then(|id| all.iter().find(|c| c["id"] == id))
        .or_else(|| all.iter().find(|c| !views_of(c).is_empty() || c["sprites"].as_array().is_some_and(|s| !s.is_empty())));
    let Some(character) = chosen else {
        return Ok(json!({"ok": true, "characters": characters, "documents": documents, "selected": null}));
    };
    let id = character["id"].as_str().unwrap_or_default();
    let view = p["view"].as_str().map(str::to_owned).or_else(|| views_of(character).first().cloned()).unwrap_or_else(|| "front".into());
    let time = p["time"].as_f64().unwrap_or(0.0).max(0.0);
    let folder = character_folder(&crate::workshop::project_dir(s)?, id);
    let frame = cmd_character_frame(s, &json!({"character": id, "view": view, "time": time}))
        .ok()
        .map(|f| f["frame"].clone())
        .or_else(|| find_initial_cutout(&folder).map(|p| json!(p)));
    let clip = [folder.join(&view).join("clips"), folder.join("clips")]
        .iter()
        .flat_map(|dir| std::fs::read_dir(dir).into_iter().flatten().flatten())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "mov" || ext == "mp4"))
        .max_by_key(|path| std::fs::metadata(path).and_then(|m| m.modified()).ok());
    Ok(json!({
        "ok": true,
        "characters": characters,
        "documents": documents,
        "easings": crate::animation_model::easing_catalog(),
        "properties": crate::animation_model::property_catalog(),
        "symbol_loop_modes": ["loop", "play_once", "single_frame"],
        "selected": {
            "character": id,
            "view": view,
            "animation_style": character["animation_style"],
            "animation_styles": character["animation_styles"],
            "action_ecprojs": character["action_ecprojs"],
            "time": time,
            "frame": frame,
            "clip": clip,
            "sprites": character["sprites"],
            "symbol": {
                "kind": "graphic",
                "loop_mode": p.get("loop_mode").and_then(Value::as_str).unwrap_or("loop"),
                "first_frame": p.get("first_frame").and_then(Value::as_u64).unwrap_or(0),
            }
        }
    }))
}

pub fn cmd_animation_open(s: &mut Session, p: &Value) -> Result<Value> {
    let path = p["path"]
        .as_str()
        .map(|raw| crate::workspace::resolve_workspace_path(s, raw))
        .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext == "ecproj"))
        .ok_or_else(|| bad("path=<existing .ecproj> is required"))?;
    let head = check_ecproj_head(&path)?;
    crate::workspace::open_editor("effectcraft", &path)?;
    Ok(json!({
        "ok": true,
        "app": "effectcraft",
        "file": path,
        "saved_by": head.saved_by,
        "schema": head.schema,
        "document": {"app": "effectcraft", "file": path},
    }))
}

pub fn cmd_character_import(s: &mut Session, p: &Value) -> Result<Value> {
    let raw_path = p["path"].as_str().ok_or_else(|| bad("path=<file> is required"))?;
    let source = crate::workspace::resolve_workspace_path(s, raw_path);
    let ext = source.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).unwrap_or_default();
    if matches!(
        ext.as_str(),
        "ecproj" | "pcraft" | "psd" | "psb" | "ora" | "xcf" | "kra" | "pdn" | "vectorcraft" | "svg" | "ai" | "af" | "afdesign" | "afphoto"
    ) {
        if !magic_util::media::is_file_or_pcraft_bundle(&source) {
            return Err(bad(format!("{} does not exist", source.display())));
        }
        let head = (ext == "ecproj").then(|| check_ecproj_head(&source)).transpose()?;
        let stem = source.file_stem().and_then(|n| n.to_str()).unwrap_or("character");
        let clean_stem = stem
            .trim_end_matches("-rig")
            .trim_end_matches("-front")
            .trim_end_matches("-parts")
            .trim_end_matches("-editable")
            .trim_end_matches("-waving")
            .trim_end_matches("-running");
        let char_id = p["character"]
            .as_str()
            .map(crate::workspace::sanitize_asset_id)
            .filter(|id| !id.is_empty())
            .unwrap_or_else(|| crate::workspace::sanitize_asset_id(clean_stem));
        let project_dir = crate::workshop::project_dir(s)?;
        let char_dir = character_folder(&project_dir, &char_id);
        std::fs::create_dir_all(&char_dir)?;
        let file_name = source.file_name().ok_or_else(|| bad("invalid Craft document filename"))?;
        let dest = if source.starts_with(project_dir.join("assets/characters")) || source.starts_with(project_dir.join("animations")) || source.is_dir() {
            source.clone()
        } else {
            let d = char_dir.join(file_name);
            if source != d {
                let bytes = std::fs::read(&source)?;
                craft_atomic::atomic_write(&d, &bytes)?;
            }
            d
        };
        return Ok(json!({
            "ok": true,
            "character": char_id,
            "craft_kind": ext,
            "file": dest,
            "ecproj": (ext == "ecproj").then_some(&dest),
            "pcraft": (ext == "pcraft").then_some(&dest),
            "saved_by": head.as_ref().map(|h| &h.saved_by),
            "schema": head.as_ref().map(|h| h.schema),
        }));
    }
    let name = Path::new(raw_path).file_stem().and_then(|n| n.to_str()).unwrap_or("character");
    crate::project::cmd_project_add_asset(s, &json!({"path": raw_path, "character": crate::workspace::sanitize_asset_id(name), "name": name, "kind": "image"}))
}

pub fn cmd_character_set(s: &mut Session, p: &Value) -> Result<Value> {
    let view = View::from_params(s, p)?;
    let project = view.require("ecproj", "character.rig")?;
    let part = p["part"].as_str().ok_or_else(|| bad("part=<layer> is required"))?;
    let time = p["time"].as_f64().unwrap_or(0.0);
    let mut steps = vec![step("comp.open", json!({"comp": view.comp()}))];

    if let Some(props_obj) = p.get("props").and_then(Value::as_object) {
        for (prop, value) in props_obj {
            if !matches!(prop.as_str(), "position" | "rotation" | "scale" | "anchor" | "opacity") {
                return Err(bad("props keys must be position, rotation, scale, anchor or opacity"));
            }
            steps.push(step(
                "prop.addKey",
                json!({
                    "layer": part,
                    "path": format!("transform/{prop}"),
                    "time": time,
                    "timeBase": "comp",
                    "value": value,
                }),
            ));
        }
        effectcraft(&view.dir, Some(&project), &steps, true)?;
        return Ok(json!({"ok": true, "project": project, "part": part, "props": props_obj.keys().collect::<Vec<_>>(), "time": time}));
    }

    let prop = p["prop"].as_str().unwrap_or("rotation");
    let value = p.get("value").cloned().ok_or_else(|| bad("value is required"))?;
    match prop {
        "visible" => {
            let vis = value.as_bool().unwrap_or(true);
            steps.push(step("layer.setSwitch", json!({"layer": part, "switch": "video", "value": vis})));
        }
        "arrange" => {
            let to = value.as_str().unwrap_or("forward");
            if !matches!(to, "front" | "forward" | "backward" | "back") {
                return Err(bad("arrange value must be front, forward, backward or back"));
            }
            steps.push(step("layer.select", json!({"layer": part})));
            steps.push(step("layer.arrange", json!({"layers": [part], "to": to})));
        }
        "position" | "rotation" | "scale" | "anchor" | "opacity" => {
            steps.push(step(
                "prop.addKey",
                json!({
                    "layer": part,
                    "path": format!("transform/{prop}"),
                    "time": time,
                    "timeBase": "comp",
                    "value": value,
                }),
            ));
        }
        _ => return Err(bad("prop must be position, rotation, scale, anchor, opacity, visible or arrange")),
    }
    effectcraft(&view.dir, Some(&project), &steps, true)?;
    Ok(json!({"ok": true, "project": project, "part": part, "prop": prop, "time": time}))
}

pub fn cmd_character_open(s: &mut Session, p: &Value) -> Result<Value> {
    let view = View::from_params(s, p)?;
    let (app, file) = match p["app"].as_str().unwrap_or("effectcraft") {
        "effectcraft" => {
            let ec = view.file("ecproj");
            let chosen = if ec.is_file() {
                ec
            } else {
                let project_dir = crate::workshop::project_dir(s)?;
                let action_ecs = discover_character_action_ecprojs(&project_dir, &view.dir, &view.folder_name, &view.id);
                if let Some(first_ec) = action_ecs.first().and_then(|v| v["path"].as_str()).map(PathBuf::from).filter(|p| p.is_file()) {
                    first_ec
                } else {
                    let info_3d = mesh_io::inspect_character_3d(s, &project_dir, &view.id);
                    info_3d["clips"]
                        .as_array()
                        .and_then(|clips| clips.iter().find_map(|c| c["ecproj_3d"].as_str().map(PathBuf::from)))
                        .filter(|p| p.is_file())
                        .ok_or_else(|| bad("no EffectCraft .ecproj found; run character.rig or animation.promote first"))?
                }
            };
            ("effectcraft", chosen)
        }
        "photocraft" => ("photocraft", view.require_photo_doc("character.cut")?),
        "vectorcraft" => ("vectorcraft", view.vector_doc().ok_or_else(|| bad("no .vectorcraft or .svg found for this character"))?),
        "texturecraft" => {
            let project_dir = crate::workshop::project_dir(s)?;
            let info_3d = mesh_io::inspect_character_3d(s, &project_dir, &view.id);
            let tcraft = info_3d["tcraft_files"]
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(Value::as_str)
                .map(PathBuf::from)
                .filter(|p| p.is_file())
                .ok_or_else(|| bad("no .tcraft document found for this character"))?;
            ("texturecraft", tcraft)
        }
        other => return Err(bad(format!("app must be effectcraft, photocraft, vectorcraft, or texturecraft, not {other}"))),
    };
    if app == "effectcraft" {
        let _ = check_ecproj_head(&file)?;
    }
    crate::workspace::open_editor(app, &file)?;
    Ok(json!({"ok": true, "app": app, "file": file, "document": {"app": app, "file": file}}))
}
