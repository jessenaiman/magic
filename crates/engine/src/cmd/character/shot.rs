//! `character.shot` & `character.publish`: author a film-sized `EffectCraft` `.ecproj` character
//! performance shot, render it as ProRes 4444 RGBA (`renderQueue.add` + `renderQueue.render`),
//! and export/publish it onto `FilmCraft`'s dedicated `V2` `"Character Animation"` track.

use std::path::PathBuf;

use serde_json::{Value, json};

use super::{bad, effectcraft, point, sprite::sprite_dir, step};
use crate::{Result, Session};

const FRAME: [u32; 2] = [1920, 1080];
const FPS: f64 = 30.0;

struct Placement {
    from: [f64; 2],
    to: [f64; 2],
    scale: f64,
    mirror: bool,
    duration: f64,
    hop: Option<(f64, f64)>,
}

fn placement(p: &Value) -> Placement {
    let from = point(&p["from"]).unwrap_or([960.0, 780.0]);
    Placement {
        from,
        to: point(&p["to"]).unwrap_or(from),
        scale: p["scale"].as_f64().unwrap_or(100.0),
        mirror: p["mirror"] == true,
        duration: p["duration"].as_f64().unwrap_or(4.0).clamp(0.2, 600.0),
        hop: p["hop_period"].as_f64().filter(|t| *t > 0.1).map(|t| (t, p["height"].as_f64().unwrap_or(140.0))),
    }
}

fn shot_steps(sprite_project: &std::path::Path, sprite_comp: &str, name: &str, at: &Placement) -> Vec<Value> {
    let sx = if at.mirror { -at.scale } else { at.scale };
    let key = move |t: f64, v: [f64; 2]| step("prop.addKey", json!({"layer": 1, "path": "transform/position", "time": t, "timeBase": "comp", "value": v}));
    let mut steps: Vec<Value> = vec![
        step("file.importProject", json!({"path": sprite_project})),
        step("comp.new", json!({"name": name, "width": FRAME[0], "height": FRAME[1], "frameRate": FPS, "duration": at.duration, "open": true})),
        step("layer.addItem", json!({"item": sprite_comp, "position": at.from, "duration": at.duration})),
        step("layer.setTransform", json!({"layers": [1], "prop": "scale", "value": [sx, at.scale]})),
    ];
    steps.extend(position_keys(at).into_iter().map(|(t, v)| key(t, v)));
    steps.push(step("keys.selectAll", json!({})));
    steps.push(step("keys.easyEase", json!({})));
    steps
}

fn position_keys(at: &Placement) -> Vec<(f64, [f64; 2])> {
    let along = |t: f64| {
        let k = t / at.duration;
        [at.from[0] + (at.to[0] - at.from[0]) * k, at.from[1] + (at.to[1] - at.from[1]) * k]
    };
    let Some((period, height)) = at.hop else {
        return vec![(0.0, at.from), (at.duration, at.to)];
    };
    let mut keys = vec![(0.0, at.from)];
    let mut start = 0.0;
    while start + period * 0.75 <= at.duration {
        for (phase, lift) in [(0.15, 0.0), (0.5, height), (0.75, 0.0)] {
            let t = start + period * phase;
            let p = along(t);
            keys.push((t, [p[0], p[1] - lift]));
        }
        start += period;
    }
    keys.push((at.duration, at.to));
    keys
}

pub fn cmd_character_shot(s: &mut Session, p: &Value) -> Result<Value> {
    let id = p["character"].as_str().map(crate::workspace::sanitize_asset_id).ok_or_else(|| bad("character=<id> is required"))?;
    let sprite = p["sprite"].as_str().ok_or_else(|| bad("sprite=<name> is required"))?;
    let name = p["name"].as_str().map_or_else(|| format!("{id}-{sprite}-shot"), str::to_string);
    let sprite_comp = format!("{id}-{sprite}");
    let sprite_project = sprite_dir(s, &id, sprite)?.join(format!("{sprite_comp}.ecproj"));
    if !sprite_project.is_file() {
        return Err(bad(format!("{} is missing; run character.sprite first", sprite_project.display())));
    }
    let at = placement(p);
    let dir = crate::workshop::project_dir(s)?.join("assets/characters").join(&id).join("shots");
    std::fs::create_dir_all(&dir)?;
    let project = dir.join(format!("{name}.ecproj"));
    let mut steps = shot_steps(&sprite_project, &sprite_comp, &name, &at);
    steps.push(step("file.saveAs", json!({"path": project})));
    effectcraft(&dir, None, &steps, true)?;
    let clip = dir.join(format!("{name}.mov"));
    crate::cmd::character::effectcraft_render(
        &project,
        json!({
            "comp": name, "output": clip, "format": "prores", "proresProfile": "4444", "channels": "rgba"
        }),
    )?;
    let studio = if p["studio"].as_bool().unwrap_or(false) { Some(crate::cmd::film::publish_character(s, &clip)?) } else { None };
    Ok(json!({
        "ok": true,
        "clip": clip,
        "ecproj": project,
        "comp": name,
        "duration": at.duration,
        "from": at.from,
        "to": at.to,
        "scale": at.scale,
        "mirror": at.mirror,
        "hops": at.hop.is_some(),
        "studio": studio,
    }))
}

/// `character.publish`: publish a rendered `EffectCraft` character ProRes 4444 alpha `.mov`
/// onto `FilmCraft`'s dedicated `V2` `"Character Animation"` track.
pub fn cmd_character_publish(s: &mut Session, p: &Value) -> Result<Value> {
    let dir = crate::workshop::project_dir(s)?.canonicalize()?;
    let raw = p["path"].as_str().or_else(|| p["clip"].as_str()).ok_or_else(|| bad("character.publish requires path=<rendered .mov inside project>"))?;
    let path = PathBuf::from(raw);
    let full = if path.is_absolute() { path } else { dir.join(path) }.canonicalize()?;
    if !full.starts_with(&dir) || !full.extension().is_some_and(|e| e.eq_ignore_ascii_case("mov")) {
        return Err(bad("Choose a rendered .mov inside the Magic project"));
    }
    let studio = crate::cmd::film::publish_character(s, &full)?;
    Ok(json!({"ok": true, "render": full, "track": "Character Animation", "studio": studio}))
}
