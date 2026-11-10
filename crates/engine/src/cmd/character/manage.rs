//! Character listing, style classification, frame/clip rendering, keyframing, and Craft app handoff.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::{
    View, bad, character_folder, check_ecproj_head, effectcraft, effectcraft_frame, effectcraft_render, find_dir_ecproj, find_initial_cutout, model3d, sprite,
    step,
};
use crate::{Result, Session};

fn rendered_pngs(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut frames: Vec<PathBuf> = std::fs::read_dir(dir)?.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "png")).collect();
    frames.sort();
    Ok(frames)
}

/// List characters and which Craft app files each view has.
pub fn cmd_character_list(s: &mut Session, _: &Value) -> Result<Value> {
    let project_dir = crate::workshop::project_dir(s)?;
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
    let root = project_dir.join("assets/characters");
    let skip_names = ["approved-crops", "lineup", "expressions", "contact-parts-v1", "cast"];
    let mut ids: Vec<String> = std::fs::read_dir(&root)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|name| !skip_names.iter().any(|skip| name.eq_ignore_ascii_case(skip)))
        .collect();
    for asset in s.project_assets.iter().filter(|a| a.kind == "model_3d") {
        let id = asset.id.replace('_', "-");
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    let mut out = Vec::new();
    for id in ids {
        if !cast.is_empty() && !cast.contains(&id) {
            continue;
        }
        let folder = root.join(&id);
        let canonical_id = crate::workspace::sanitize_asset_id(&id);
        let mut views: Vec<Value> = ["front", "side", "back"]
            .iter()
            .filter(|v| folder.join(v).is_dir())
            .map(|v| {
                let dir = folder.join(v);
                let has = |ext: &str| dir.join(format!("{canonical_id}-{v}.{ext}")).is_file() || dir.join(format!("{id}-{v}.{ext}")).is_file();
                let drawing = std::fs::read_to_string(dir.join("parts.json"))
                    .ok()
                    .and_then(|t| serde_json::from_str::<Value>(&t).ok())
                    .and_then(|p| p["image"].as_str().or_else(|| p["source"].as_str()).map(|i| dir.join(i)))
                    .filter(|p| p.is_file())
                    .or_else(|| {
                        let staged = dir.join(format!("{canonical_id}-{v}-drawing.png"));
                        staged.is_file().then_some(staged)
                    })
                    .or_else(|| {
                        let staged = dir.join(format!("{id}-{v}-drawing.png"));
                        staged.is_file().then_some(staged)
                    });
                json!({
                    "view": v,
                    "parts": dir.join("parts.json").is_file(),
                    "drawing": drawing,
                    "psd": has("psd"),
                    "ecproj": find_dir_ecproj(&dir, &canonical_id, &id, v, false).is_some(),
                })
            })
            .collect();
        if views.is_empty() {
            let flat_ecproj = find_dir_ecproj(&folder, &canonical_id, &id, "front", true);
            let flat_parts = folder.join("parts.json").is_file();
            let flat_psd = folder.join(format!("{id}-parts.psd")).is_file() || folder.join(format!("{id}.psd")).is_file();
            let cutout = std::fs::read_to_string(folder.join("parts.json"))
                .ok()
                .and_then(|t| serde_json::from_str::<Value>(&t).ok())
                .and_then(|p| p["image"].as_str().or_else(|| p["source"].as_str()).map(|i| folder.join(i)))
                .filter(|p| p.is_file())
                .or_else(|| find_initial_cutout(&folder));
            if flat_ecproj.is_some() || flat_parts || cutout.is_some() {
                views.push(json!({"view": "front", "parts": flat_parts, "drawing": cutout, "psd": flat_psd, "ecproj": flat_ecproj.is_some()}));
            }
        }
        let info_3d = model3d::inspect_character_3d(s, &project_dir, &canonical_id);
        let sprites = sprite_summaries(&folder);
        if !views.is_empty() || !sprites.is_empty() || info_3d["available"].as_bool() == Some(true) {
            let (animation_style, animation_styles) = classify_animation_style(&views, &sprites, &info_3d);
            out.push(json!({
                "id": canonical_id,
                "name": id,
                "animation_style": animation_style,
                "animation_styles": animation_styles,
                "views": views,
                "sprites": sprites,
                "model_3d": info_3d,
            }));
        }
    }
    Ok(json!({"ok": true, "characters": out}))
}

pub fn classify_animation_style(views: &[Value], sprites: &[Value], info_3d: &Value) -> (&'static str, Vec<&'static str>) {
    let has_2_5d_rig = views.iter().any(|v| {
        v["rigged"].as_bool() == Some(true)
            || v["ecproj"].as_bool() == Some(true)
            || v["ecproj"].is_string()
            || v["cut"].as_bool() == Some(true)
            || v["psd"].as_bool() == Some(true)
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
    let project = view.require("ecproj", "character.rig")?;
    let time = p["time"].as_f64().unwrap_or(0.0).max(0.0);
    let side = p["max_side"].as_u64().unwrap_or(900).clamp(64, 4096);
    let frames = view.dir.join("frames");
    std::fs::create_dir_all(&frames)?;
    let out = frames.join(format!("{}-{}-{:07.3}.png", view.id, view.view, time));
    effectcraft_frame(&project, &view.comp(), time, side as u32, &out)?;
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
    let mut paths: Vec<PathBuf> = ["motion", "lettering"]
        .iter()
        .flat_map(|folder| std::fs::read_dir(root.join(folder)).into_iter().flatten().flatten())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "ecproj"))
        .collect();
    paths.sort();
    Ok(paths.iter().map(|path| json!({"name": path.file_stem().map(|n| n.to_string_lossy().into_owned()), "path": path})).collect())
}

pub fn cmd_animation_view(s: &mut Session, p: &Value) -> Result<Value> {
    let list = cmd_character_list(s, &json!({}))?;
    let all = list["characters"].as_array().cloned().unwrap_or_default();
    let views_of = |c: &Value| c["views"].as_array().into_iter().flatten().filter_map(|v| v["view"].as_str().map(str::to_owned)).collect::<Vec<_>>();
    let characters: Vec<Value> = all
        .iter()
        .map(|c| json!({"id": c["id"], "animation_style": c["animation_style"], "animation_styles": c["animation_styles"], "views": views_of(c)}))
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
    crate::CraftHub::open_live(crate::CraftTarget::Effect, &path)?;
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
    if source.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("ecproj")) {
        if !source.is_file() {
            return Err(bad(format!("{} does not exist", source.display())));
        }
        let head = check_ecproj_head(&source)?;
        let stem = source.file_stem().and_then(|n| n.to_str()).unwrap_or("character");
        let clean_stem = stem.trim_end_matches("-rig").trim_end_matches("-front");
        let char_id = p["character"]
            .as_str()
            .map(crate::workspace::sanitize_asset_id)
            .filter(|id| !id.is_empty())
            .unwrap_or_else(|| crate::workspace::sanitize_asset_id(clean_stem));
        let project_dir = crate::workshop::project_dir(s)?;
        let char_dir = project_dir.join("assets/characters").join(&char_id);
        std::fs::create_dir_all(&char_dir)?;
        let file_name = source.file_name().ok_or_else(|| bad("invalid .ecproj filename"))?;
        let dest = if source.starts_with(project_dir.join("assets/characters")) {
            source.clone()
        } else {
            let d = char_dir.join(file_name);
            if source != d {
                std::fs::copy(&source, &d)?;
            }
            d
        };
        return Ok(json!({"ok": true, "character": char_id, "ecproj": dest, "saved_by": head.saved_by, "schema": head.schema}));
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

/// Open a character's live document in `EffectCraft` (2.5D/3D `.ecproj`) or `PhotoCraft` (`.psd`/`.pcraft`).
/// `AniCraft` GUI is intentionally disabled for safety while in early alpha (`anicraft-cli` runs headlessly).
pub fn cmd_character_open(s: &mut Session, p: &Value) -> Result<Value> {
    let view = View::from_params(s, p)?;
    let (app, target, file) = match p["app"].as_str().unwrap_or("effectcraft") {
        "effectcraft" => {
            let ec = view.file("ecproj");
            let chosen = if ec.is_file() {
                ec
            } else {
                let project_dir = crate::workshop::project_dir(s)?;
                let info_3d = model3d::inspect_character_3d(s, &project_dir, &view.id);
                info_3d["clips"]
                    .as_array()
                    .and_then(|clips| clips.iter().find_map(|c| c["ecproj_3d"].as_str().map(PathBuf::from)))
                    .filter(|p| p.is_file())
                    .ok_or_else(|| bad("no EffectCraft .ecproj found; run character.rig or character.model3d.animate first"))?
            };
            ("effectcraft", crate::CraftTarget::Effect, chosen)
        }
        "photocraft" => ("photocraft", crate::CraftTarget::Photo, view.require("psd", "character.cut")?),
        "anicraft" => {
            return Err(bad(
                "AniCraft desktop UI is disabled for safety while in early alpha; run headless `anicraft.exec` or `character.sprite` and inspect inside EffectCraft (`app=\"effectcraft\"`).",
            ));
        }
        other => return Err(bad(format!("app must be effectcraft or photocraft, not {other}"))),
    };
    if app == "effectcraft" {
        let _ = check_ecproj_head(&file)?;
    }
    crate::CraftHub::open_live(target, &file)?;
    Ok(json!({"ok": true, "app": app, "file": file, "document": {"app": app, "file": file}}))
}
