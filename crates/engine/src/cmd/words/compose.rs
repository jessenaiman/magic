//! Words composition and EffectCraft/FilmCraft placement (`words.compose`, `words.apply`, `words.design`, `words.track`).
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub use super::validate::cmd_words_effect_install;
pub(crate) use super::validate::{kinetic_design, line_design, text_effect};
use super::{
    HOLD_TICK, aligned, cmd_words_read, cmd_words_save, cmd_words_studio_track, fill_untimed, invalid, printed_caption_words, studio_times, timing, word_tick,
    word_time_key,
};
use crate::time::{TICKS_PER_SECOND, Tick};
use crate::{Result, Session};

fn build_section_designs(doc: &Value, style_cfg: &Value, times: &HashMap<String, (Tick, Tick)>) -> Value {
    let top_band = style_cfg["band"].as_str() != Some("bottom");
    let mut out = serde_json::Map::new();
    for a in doc["animations"].as_array().into_iter().flatten() {
        let Some(anim_id) = a["animation_id"].as_str() else { continue };
        let mut by_line: HashMap<String, Vec<(String, String, Option<(Tick, Tick)>)>> = HashMap::new();
        let mut line_order: Vec<String> = Vec::new();
        for w in a["words"].as_array().into_iter().flatten() {
            let lid = w["line_id"].as_str().unwrap_or("l1").to_owned();
            let wid = w["word_id"].as_str().unwrap_or_default().to_owned();
            let txt = w["text"].as_str().unwrap_or_default().to_owned();
            let span =
                times.get(&word_time_key(&a["scope"], &wid)).copied().or_else(|| word_tick(w, "start_tick", "start").zip(word_tick(w, "end_tick", "end")));
            if !by_line.contains_key(&lid) {
                line_order.push(lid.clone());
            }
            by_line.entry(lid).or_default().push((wid, txt, span));
        }
        let mut section_map = serde_json::Map::new();
        for lid in line_order {
            if let Some(words) = by_line.get(&lid) {
                let items = super::lettering::default_line_items(words, style_cfg, top_band);
                section_map.insert(lid, json!({"items": items, "draws": []}));
            }
        }
        out.insert(anim_id.to_owned(), Value::Object(section_map));
    }
    Value::Object(out)
}

/// `words.design`: lay out non-overlapping word/phrase groups across sung lines using `craft_font_catalog`
/// glyph widths without requiring a `lyrics-design.json` file.
pub fn cmd_words_design(s: &mut Session, p: &Value) -> Result<Value> {
    let optional_recipe = p["recipe"]
        .as_str()
        .map(|raw| crate::workspace::resolve_workspace_path(s, raw))
        .filter(|rp| rp.is_file())
        .and_then(|rp| fs::read(rp).ok())
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok());
    let style_cfg = optional_recipe.unwrap_or_else(|| {
        json!({
            "band": p["band"].as_str().unwrap_or("top"),
            "font": p["font"].as_str().unwrap_or("sans"),
            "size": p["size"].as_f64().unwrap_or(68.0),
            "color": p["color"].as_str().unwrap_or("#FFF6E3"),
            "outline": p["outline"].as_str().unwrap_or("#143042"),
        })
    });
    let doc = cmd_words_read(s, &json!({}))?["document"].clone();
    let times = studio_times(s);
    let designs = build_section_designs(&doc, &style_cfg, &times);
    let style = style_cfg["font"].as_str().map(|f| json!({"font": f})).unwrap_or_else(|| json!({}));
    let only = p["section"].as_str();
    let mut sections = Vec::new();
    for a in doc["animations"].as_array().into_iter().flatten() {
        let id = a["animation_id"].as_str().unwrap_or_default();
        if only.is_some_and(|o| o != id) {
            continue;
        }
        let saved = cmd_words_save(
            s,
            &json!({
                "animation_id": id,
                "expected_source_revision": a["source_revision"],
                "origin": "words.design",
                "style": style,
                "design": designs[id],
            }),
        )?;
        let applied = if p["apply"] == false { Value::Null } else { cmd_words_apply(s, &json!({"animation_id": id}))?["in_video"].clone() };
        sections.push(json!({
            "animation_id": id,
            "lines": designs[id].as_object().map_or(0, |m| m.len()),
            "revision": saved["animation"]["source_revision"],
            "in_video": applied,
        }));
    }
    Ok(json!({
        "ok": true,
        "sections": sections,
        "lines": sections.iter().filter_map(|x| x["lines"].as_u64()).sum::<u64>(),
        "next": "run words.letter to build effects/lyrics.ecproj",
    }))
}

/// `words.track`: write the song's lyric timing track (`effects/words-track.json`), one clip per printed word in `Tick`s.
pub fn cmd_words_track(s: &mut Session, _: &Value) -> Result<Value> {
    let dir = crate::cmd::words::workspace_dir(s)?;
    let (words, _) = printed_caption_words(&aligned(s)?);
    if words.is_empty() {
        return Err(invalid("No printed words with timing; check words.align"));
    }
    let mut line_end: HashMap<String, Tick> = HashMap::new();
    for w in &words {
        let end = word_tick(w, "end_tick", "end").unwrap_or(Tick::ZERO);
        let e = line_end.entry(word_time_key(w, w["line"].as_str().unwrap_or_default())).or_insert(Tick::ZERO);
        if end > *e {
            *e = end;
        }
    }
    let min_dur = Tick(TICKS_PER_SECOND / 10);
    let mut clips = Vec::with_capacity(words.len());
    for (i, w) in words.iter().enumerate() {
        let id = w["word_id"].as_str().unwrap_or_default();
        let text = w["text"].as_str().unwrap_or_default();
        let start = word_tick(w, "start_tick", "start").unwrap_or(Tick::ZERO);
        let end = line_end.get(&word_time_key(w, w["line"].as_str().unwrap_or_default())).copied().unwrap_or(start) + HOLD_TICK;
        let dur = Tick((end.0 - start.0).max(min_dur.0));
        clips.push(json!({
            "clip_id": format!("kinetic-r{}-{}", w["occurrence"].as_u64().unwrap_or(1), id.replace("/w", "-")),
            "word_id": id,
            "line": w["line"],
            "section": w["section"],
            "occurrence": w["occurrence"],
            "text": text,
            "track_index": 200 + i,
            "start_tick": start.0,
            "end_tick": (start + dur).0,
            "duration_tick": dur.0,
            "start": start.seconds(),
            "end": (start + dur).seconds(),
            "duration": dur.seconds()
        }));
    }
    let path: PathBuf = dir.join("effects/words-track.json");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let payload = serde_json::to_vec_pretty(&json!({"format": "magic.words.track", "version": 1, "clips": clips})).map_err(|e| invalid(e.to_string()))?;
    crate::doc_state::write_file(&path, payload)?;
    Ok(json!({
        "ok": true,
        "clips": words.len(),
        "first": words.first(),
        "last": words.last(),
        "path": path,
        "next": "words.design builds the lyric visuals on these anchors"
    }))
}

pub fn cmd_words_apply(s: &mut Session, p: &Value) -> Result<Value> {
    let dir = crate::cmd::words::workspace_dir(s)?;
    let id = p["animation_id"]
        .as_str()
        .filter(|i| i.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'))
        .ok_or_else(|| invalid("animation_id is required (letters, digits, '-' or '_')"))?;
    let use_it = p["use"].as_bool().unwrap_or(true);
    let mut receipt = json!({"ok":true,"animation_id":id,"in_video":use_it,"changed":true});
    if use_it {
        let composed = cmd_words_compose(s, &json!({"animation_id":id}))?;
        receipt["src"] = composed["src"].clone();
        receipt["start_tick"] = composed["song_anchor_tick"].clone();
        receipt["start"] = composed["song_anchor"].clone();
        receipt["drawings"] = composed["drawings"].clone();
    }
    if let Ok(Some(mut doc)) = crate::cmd::words::load(s, &dir)
        && let Some(anim) = doc["animations"].as_array_mut().and_then(|a| a.iter_mut().find(|x| x["animation_id"] == id))
    {
        anim["in_video"] = json!(use_it);
        crate::cmd::words::save(s, &dir, &doc)?;
    }
    Ok(receipt)
}

fn sync_studio_word_times(s: &mut Session, synced: &mut Value) {
    let track = cmd_words_studio_track(s, &json!({})).unwrap_or(Value::Null);
    let times = studio_times(s);
    let scope = synced["scope"].clone();
    for w in synced["words"].as_array_mut().into_iter().flatten() {
        if let Some((a, b)) = w["word_id"].as_str().and_then(|wid| times.get(&word_time_key(&scope, wid))) {
            w["start_tick"] = json!(a.0);
            w["end_tick"] = json!(b.0);
            w["start"] = json!(a.seconds());
            w["end"] = json!(b.seconds());
            w["timing_source"] = json!("studio");
        }
    }
    fill_untimed(synced);
    timing(synced);
    let next_start = track["sections"].as_array().and_then(|secs| {
        let pos = secs.iter().position(|sec| sec["section"] == scope["section"] && sec["occurrence"] == scope["occurrence"])?;
        secs.get(pos + 1)?["start_tick"].as_i64().map(Tick)
    });
    if let (Some(anchor), Some(ns)) = (synced["song_anchor_tick"].as_i64().map(Tick), next_start) {
        let cur_dur = synced["authored_duration_tick"].as_i64().map(Tick).unwrap_or(Tick::ZERO);
        let gap = Tick(TICKS_PER_SECOND / 25);
        let min_dur = Tick(TICKS_PER_SECOND / 5);
        let clamped = cur_dur.min(Tick((ns.0 - gap.0 - anchor.0).max(min_dur.0)));
        synced["authored_duration_tick"] = json!(clamped.0);
        synced["authored_duration"] = json!(clamped.seconds());
    }
}

fn build_local_words(anim: &Value, anchor: Tick) -> Result<Vec<Value>> {
    anim["words"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|w| {
            let start = word_tick(w, "start_tick", "start").ok_or_else(|| invalid("Word has no numeric start"))?;
            let end = word_tick(w, "end_tick", "end").ok_or_else(|| invalid("Word has no numeric end"))?;
            let local_start = start - anchor;
            let local_end = end - anchor;
            Ok(json!({
                "word_id": w["word_id"],
                "line_id": w["line_id"],
                "text": w["text"],
                "start_tick": local_start.0,
                "end_tick": local_end.0,
                "start": local_start.seconds(),
                "end": local_end.seconds(),
                "emphasis": w["emphasis"],
                "action": w["action"],
                "entrance": w["entrance"],
                "exit": w["exit"]
            }))
        })
        .collect()
}

pub fn cmd_words_compose(s: &mut Session, p: &Value) -> Result<Value> {
    let dir = crate::cmd::words::workspace_dir(s)?;
    let id = p["animation_id"].as_str().ok_or_else(|| invalid("animation_id is required"))?.to_owned();
    let instance = match p["instance_id"].as_str() {
        Some(i) if !i.is_empty() && i.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_') => Some(i.to_owned()),
        Some(_) => return Err(invalid("instance_id must be letters, digits, '-' or '_'")),
        None => None,
    };
    let composition_id = instance.as_deref().map_or_else(|| id.clone(), |i| format!("{id}--{i}"));
    let read = cmd_words_read(s, &json!({"animation_id": &id}))?;
    let mut synced = read["document"]["animations"][0].clone();
    sync_studio_word_times(s, &mut synced);
    let anim = &synced;
    let untimed: Vec<&str> = anim["words"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|w| word_tick(w, "start_tick", "start").is_none() || word_tick(w, "end_tick", "end").is_none())
        .filter_map(|w| w["word_id"].as_str())
        .collect();
    if !untimed.is_empty() {
        return Err(invalid(format!("Words without timing cannot be placed: {}; resolve them with words.align", untimed.join(", "))));
    }
    let anchor = anim["song_anchor_tick"]
        .as_i64()
        .map(Tick)
        .or_else(|| anim["song_anchor"].as_f64().map(Tick::from_seconds_f64))
        .ok_or_else(|| invalid("Animation has no song_anchor"))?;
    let duration = anim["authored_duration_tick"]
        .as_i64()
        .map(Tick)
        .or_else(|| anim["authored_duration"].as_f64().map(Tick::from_seconds_f64))
        .ok_or_else(|| invalid("Animation has no authored_duration"))?;
    let words = build_local_words(anim, anchor)?;
    let data = json!({
        "composition_id": composition_id,
        "animation_id": id,
        "source_revision": anim["source_revision"],
        "duration_tick": duration.0,
        "duration": duration.seconds(),
        "hold_tick": anim["hold_tick"],
        "hold": anim["hold"],
        "style": anim["style"],
        "kinetic": anim["kinetic"],
        "design": anim["design"],
        "words": words
    });
    let rel_src = "effects/lyrics.ecproj".to_owned();
    let file = dir.join(&rel_src);
    let bytes = serde_json::to_vec_pretty(&data).map_err(|e| invalid(e.to_string()))?;
    Ok(json!({
        "ok": true,
        "animation_id": id,
        "instance_id": instance,
        "composition_id": composition_id,
        "file": file,
        "drawings": [],
        "src": rel_src,
        "sha256": format!("{:x}", Sha256::digest(&bytes)),
        "source_revision": anim["source_revision"],
        "authored_duration_tick": duration.0,
        "song_anchor_tick": anchor.0,
        "authored_duration": duration.seconds(),
        "song_anchor": anchor.seconds(),
        "lead_in_tick": anim["lead_in_tick"],
        "hold_tick": anim["hold_tick"],
        "lead_in": anim["lead_in"],
        "hold": anim["hold"],
        "width": 1920u64,
        "height": 1080u64,
        "saved_source": read["saved"]
    }))
}
