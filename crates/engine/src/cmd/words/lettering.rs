//! `words.letter`: build `effects/lyrics.ecproj` and render alpha `effects/lyrics.mov` via `EffectCraft`.
//! Uses `craft_font_catalog::Catalog::global()` so any installed/suite font works directly without
//! requiring a static `lyrics-design.json` recipe, while measuring real SFNT glyph advance widths (`B10-01`).

use std::collections::HashMap;
use std::path::Path;

use magic_util::text::measure_text_advance;
use serde_json::{Value, json};

use crate::cmd::words::word_tick;
use crate::time::{TICKS_PER_SECOND, Tick};
use crate::{EngineError, Result, Session};

const COMP: &str = "Lyrics";
const FRAME: (f64, f64) = (1920.0, 1080.0);
const LEAD_TICK: Tick = Tick(TICKS_PER_SECOND / 4);
const HOLD_TICK: Tick = Tick(TICKS_PER_SECOND);

fn invalid(message: impl Into<String>) -> EngineError {
    EngineError::InvalidParams(message.into())
}

pub(crate) struct Line {
    pub(crate) id: String,
    pub(crate) layer: String,
    pub(crate) words: Vec<(String, String, Option<(Tick, Tick)>)>,
    pub(crate) start: Tick,
    pub(crate) end: Tick,
}

impl Line {
    fn text(&self) -> String {
        self.words.iter().map(|w| w.1.as_str()).collect::<Vec<_>>().join(" ")
    }
}

pub(crate) fn lines(aligned: &Value) -> Vec<Line> {
    let mut out: Vec<Line> = Vec::new();
    for (idx, w) in aligned["words"].as_array().into_iter().flatten().enumerate() {
        let id = w["sung_id"].as_str().or(w["line_id"].as_str()).unwrap_or("l1");
        let wid = w["word_id"].as_str().map(str::to_owned).unwrap_or_else(|| format!("{id}/w{idx}"));
        let time = word_tick(w, "start_tick", "start").zip(word_tick(w, "end_tick", "end"));
        let entry = (wid, w["text"].as_str().unwrap_or_default().to_owned(), time);
        match out.last_mut() {
            Some(line) if line.id == id => line.words.push(entry),
            _ => out.push(Line { id: id.to_owned(), layer: id.to_owned(), words: vec![entry], start: Tick(i64::MAX), end: Tick(i64::MIN) }),
        }
    }
    for line in &mut out {
        for (_, _, span) in &line.words {
            if let Some((s, e)) = *span {
                (line.start, line.end) = (line.start.min(s), line.end.max(e));
            }
        }
    }
    out.retain(|l| l.start <= l.end);
    let mut seen: HashMap<String, usize> = HashMap::new();
    for line in &mut out {
        let n = seen.entry(line.id.clone()).or_insert(0);
        *n += 1;
        if *n > 1 {
            line.layer = format!("{}-{n}", line.id);
        }
    }
    out
}

pub(crate) fn resolve_catalog_font(raw: Option<&str>) -> String {
    let requested = raw.map(str::trim).filter(|s| !s.is_empty()).unwrap_or("sans");
    if !matches!(requested, "song" | "sans" | "sans-bold" | "serif" | "serif-bold" | "serif-italic") {
        if magic_util::media::is_font_family_installed(requested) {
            return requested.to_owned();
        }
        return requested.to_owned();
    }
    let catalog = craft_font_catalog::Catalog::global();
    let candidates: &[&str] = if requested.starts_with("serif") {
        &["Fraunces", "Georgia", "DejaVu Serif", "Liberation Serif", "C059"]
    } else {
        &["Ubuntu", "Inter", "DejaVu Sans", "Liberation Sans", "Nimbus Sans"]
    };
    for cand in candidates {
        if catalog.family(cand).is_some() {
            return (*cand).to_owned();
        }
    }
    catalog.families().first().map(ToString::to_string).unwrap_or_else(|| if requested.starts_with("serif") { "C059".into() } else { "Nimbus Sans".into() })
}

struct Look {
    font: String,
    style: &'static str,
    weight: u16,
    italic: bool,
    size: f64,
    fill: String,
    outline: Option<String>,
    stroke_width: Option<f64>,
    tracking: Option<f64>,
}

fn look_from_item(base_spec: &Value, item: &Value) -> Look {
    let raw_font = item["font"].as_str().or_else(|| base_spec["font"].as_str()).or_else(|| base_spec["lettering"].as_str());
    let font = resolve_catalog_font(raw_font);
    let weight = item["weight"].as_u64().or_else(|| base_spec["weight"].as_u64()).unwrap_or(700).clamp(100, 900) as u16;
    let italic = item["italic"].as_bool().unwrap_or(false);
    let style = match (weight >= 700, italic) {
        (true, true) => "Bold Italic",
        (true, false) => "Bold",
        (false, true) => "Italic",
        (false, false) => "Regular",
    };
    let fill = item["color"].as_str().or_else(|| base_spec["color"].as_str()).unwrap_or("#FFF6E3").to_owned();
    let outline = item["outline"].as_str().or_else(|| base_spec["outline"].as_str()).map(str::to_owned);
    Look {
        font,
        style,
        weight,
        italic,
        size: item["size"].as_f64().or_else(|| base_spec["size"].as_f64()).unwrap_or(64.0),
        fill,
        outline,
        stroke_width: item["stroke_width"].as_f64(),
        tracking: item["tracking"].as_f64(),
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn measure_word_box_width(
    text_str: &str,
    font: &str,
    weight: u16,
    italic: bool,
    size: f64,
    has_outline: bool,
    stroke_width: Option<f64>,
    tracking: Option<f64>,
    explicit_w: Option<f64>,
    x: f64,
    next_same_row_x: Option<f64>,
) -> f64 {
    let measured_glyphs = measure_text_advance(text_str, font, weight, italic, size);
    let stroke_pad = if has_outline { 2.0 * stroke_width.unwrap_or_else(|| (size * 0.08).max(3.0)) } else { 0.0 };
    let tracking_pad = tracking.map(|tr| (text_str.chars().count().saturating_sub(1) as f64) * (tr / 1000.0) * size).unwrap_or(0.0);
    let natural = explicit_w
        .filter(|&w| w.is_finite() && w > 0.0)
        .unwrap_or_else(|| (measured_glyphs + stroke_pad + tracking_pad + size * 0.08).ceil())
        .clamp((size * 0.45).max(20.0), (FRAME.0 - x - 24.0).max(40.0));
    match next_same_row_x {
        Some(next_x) if next_x > x + 16.0 => {
            let min_gap = (size * 0.14).max(8.0);
            natural.min((next_x - x - min_gap).max(16.0))
        }
        _ => natural,
    }
}

/// Lay out non-overlapping per-word items along rows using SFNT glyph advance widths (`B10-01`).
pub(crate) fn default_line_items(words: &[(String, String, Option<(Tick, Tick)>)], style: &Value, top_band: bool) -> Vec<Value> {
    let font = style["font"].as_str().unwrap_or("sans");
    let resolved_font = resolve_catalog_font(Some(font));
    let weight = style["weight"].as_u64().unwrap_or(700).clamp(100, 900) as u16;
    let size = style["size"].as_f64().unwrap_or(68.0).clamp(24.0, 240.0);
    let color = style["color"].as_str().unwrap_or("#FFF6E3");
    let outline = style["outline"].as_str().unwrap_or("#143042");
    let margin = 120.0;
    let max_right = FRAME.0 - margin;
    let row_step = (size * 1.48).max(88.0);
    let mut x = margin;
    let mut y = if top_band { 96.0 } else { FRAME.1 - 280.0 };
    let space_gap = (size * 0.34).max(18.0);

    let mut out = Vec::with_capacity(words.len());
    for (wid, text, _) in words {
        let w = measure_word_box_width(text, &resolved_font, weight, false, size, true, None, None, None, 0.0, None);
        if x > margin && x + w > max_right {
            x = margin;
            y += row_step;
        }
        out.push(json!({
            "word_ids": [wid],
            "text": text,
            "x": x.round(),
            "y": y.round(),
            "w": w,
            "size": size,
            "weight": weight,
            "italic": false,
            "color": color,
            "font": font,
            "outline": outline,
            "rotate": 0.0,
            "motion": "pop",
            "accent": wid,
        }));
        x += w + space_gap;
    }
    out
}

fn step(command: &str, params: Value) -> Value {
    json!({"command": command, "params": params})
}

fn text_params(look: &Look) -> Value {
    let mut p = json!({"font": look.font, "style": look.style, "size": look.size, "fill": look.fill});
    if let Some(tracking) = look.tracking {
        p["tracking"] = json!(tracking);
    }
    if let Some(outline) = &look.outline {
        let sw = look.stroke_width.unwrap_or_else(|| (look.size * 0.08).max(3.0));
        if sw > 0.0 {
            p["stroke"] = json!(outline);
            p["applyStroke"] = json!(true);
            p["strokeWidth"] = json!(sw);
            p["strokeOverFill"] = json!(false);
        }
    }
    p
}

fn trim_steps(layer: &str, shown: Tick, gone: Tick) -> Vec<Value> {
    vec![
        step("layer.select", json!({"layers": [layer]})),
        step("time.set", json!({"time": shown.seconds()})),
        step("layer.timing", json!({"op": "trimInToTime"})),
        step("time.set", json!({"time": gone.seconds()})),
        step("layer.timing", json!({"op": "trimOutToTime"})),
    ]
}

fn animate_word_pop(layer_id: &str, text_len: usize, motion_name: &str, start: Tick, end: Tick, steps: &mut Vec<Value>) {
    if motion_name == "quiet" || motion_name == "none" || text_len == 0 {
        return;
    }
    let t_60ms = Tick(TICKS_PER_SECOND * 3 / 50);
    let t_100ms = Tick(TICKS_PER_SECOND / 10);
    let t_240ms = Tick(TICKS_PER_SECOND * 6 / 25);
    let (prop, keys) = match motion_name {
        "rise" => ("position", vec![(start - t_60ms, json!([0, 42, 0])), (end.max(start + t_240ms), json!([0, 0, 0]))]),
        "fall" => ("position", vec![(start - t_60ms, json!([0, -48, 0])), (end.max(start + t_240ms), json!([0, 0, 0]))]),
        _ => ("scale", vec![(start - t_100ms, json!([100, 100, 100])), (start + t_60ms, json!([128, 128, 100])), (start + t_240ms, json!([100, 100, 100]))]),
    };
    steps.push(step("layer.addTextAnimator", json!({"layer": layer_id, "properties": [prop]})));
    let selector = "text/animators/animator/selectors/rangeSelector";
    for (sub, val) in [("advanced/units", json!(1)), ("start", json!(0)), ("end", json!(text_len))] {
        steps.push(step("prop.set", json!({"layer": layer_id, "path": format!("{selector}/{sub}"), "value": val})));
    }
    for (tm, val) in keys {
        steps.push(step(
            "prop.addKey",
            json!({"layer": layer_id, "path": format!("text/animators/animator/properties/{prop}"), "value": val, "time": tm.max(Tick::ZERO).seconds(), "timeBase": "comp"}),
        ));
    }
}

fn next_row_neighbor_x(items: &[Value], idx: usize, x: f64, y: f64, bh: f64) -> Option<f64> {
    items
        .iter()
        .skip(idx + 1)
        .filter_map(|other| {
            let ox = other["x"].as_f64()?;
            let oy = other["y"].as_f64()?;
            ((oy - y).abs() < bh * 0.55 && ox > x).then_some(ox)
        })
        .reduce(f64::min)
}

fn item_word_time(line: &Line, item: &Value) -> (Tick, Tick) {
    let target_id = item["accent"].as_str().or_else(|| item["word_ids"].get(0).and_then(Value::as_str)).unwrap_or_default();
    if let Some((_, _, Some(span))) = line.words.iter().find(|(wid, _, _)| wid == target_id) {
        return *span;
    }
    if let Some((_, suffix)) = target_id.rsplit_once("/w")
        && let Ok(idx) = suffix.parse::<usize>()
        && let Some((_, _, Some(span))) = line.words.get(idx)
    {
        return *span;
    }
    (line.start, line.end)
}

fn item_text(line: &Line, item: &Value) -> String {
    if let Some(s) = item["text"].as_str().filter(|s| !s.is_empty()) {
        return s.to_owned();
    }
    item["word_ids"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter_map(|wid| line.words.iter().find(|(id, _, _)| id == wid).map(|(_, txt, _)| txt.as_str()))
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn build_steps_with_designs(base_cfg: &Value, designs: &Value, all: &[Line], duration: Tick) -> (Vec<Value>, Vec<Value>) {
    let (w, h) = FRAME;
    let top_band = base_cfg["band"].as_str() != Some("bottom");
    let base_spec = if base_cfg["base"].is_object() { &base_cfg["base"] } else { base_cfg };
    let fade_tick = Tick(TICKS_PER_SECOND / 5);
    let mut steps = vec![step("comp.new", json!({"name": COMP, "width": w, "height": h, "frameRate": 30, "duration": duration.seconds()}))];
    let mut summary = Vec::new();
    let mut previous_out = Tick::ZERO;

    for (i, line) in all.iter().enumerate() {
        let next = all.get(i + 1).map_or(duration, |l| l.start);
        let shown = (line.start - LEAD_TICK).max(previous_out).max(Tick::ZERO);
        let gone = if next - line.end < HOLD_TICK { next } else { (line.end + HOLD_TICK).min(duration) };
        previous_out = gone;

        let fallback_items = default_line_items(&line.words, base_spec, top_band);
        let items: &[Value] = designs[&line.layer]["items"].as_array().filter(|a| !a.is_empty()).map_or(fallback_items.as_slice(), Vec::as_slice);

        for (idx, item) in items.iter().enumerate() {
            let layer_id = format!("{}_item{idx}", line.layer);
            let text_owned = item_text(line, item);
            let look = look_from_item(base_spec, item);
            let x = item["x"].as_f64().unwrap_or(120.0);
            let y = item["y"].as_f64().unwrap_or(if top_band { 96.0 } else { h - 280.0 });
            let bh = (look.size * 1.5).max(96.0);
            let next_x = next_row_neighbor_x(items, idx, x, y, bh);
            let bw = measure_word_box_width(
                &text_owned,
                &look.font,
                look.weight,
                look.italic,
                look.size,
                look.outline.is_some(),
                look.stroke_width,
                look.tracking,
                item["w"].as_f64(),
                x,
                next_x,
            );
            let mut text = text_params(&look);
            text["text"] = json!(text_owned);
            text["name"] = json!(layer_id);
            text["box"] = json!([x, y, bw, bh]);
            text["justify"] = json!("left");
            steps.push(step("layer.newText", text));

            let (ws, we) = item_word_time(line, item);
            if let Some(keys) = item["motion_keys"].as_array().filter(|k| !k.is_empty()) {
                let (origin_x, origin_y) = (x + bw / 2.0, y + bh / 2.0);
                let rotate = item["rotate"].as_f64().unwrap_or(0.0);
                for key in keys {
                    let num = |k_idx: usize| key[k_idx].as_f64().unwrap_or_default();
                    let key_tick = ws + Tick::from_seconds_f64(num(0));
                    for (prop, val) in [
                        ("position", json!([origin_x + num(1), origin_y + num(2)])),
                        ("scale", json!([num(3), num(4)])),
                        ("rotation", json!(rotate + num(5))),
                        ("opacity", json!(num(6))),
                    ] {
                        steps.push(step(
                            "prop.addKey",
                            json!({"layer": layer_id, "path": format!("transform/{prop}"), "time": key_tick.seconds(), "timeBase": "comp", "value": val}),
                        ));
                    }
                }
            } else {
                if let Some(preset) = item["preset"].as_str().filter(|s| !s.is_empty()) {
                    steps.push(step("layer.select", json!({"layers": [layer_id]})));
                    steps.push(step("time.set", json!({"time": ws.max(Tick::ZERO).seconds()})));
                    steps.push(step("layer.applyTextPreset", json!({"layer": layer_id, "preset": preset})));
                } else {
                    let motion_name = item["motion"].as_str().unwrap_or("pop");
                    animate_word_pop(&layer_id, text_owned.chars().count(), motion_name, ws, we, &mut steps);
                }
                for (tm, val) in [(shown, 0.0), (shown + fade_tick, item["opacity"].as_f64().unwrap_or(100.0))] {
                    steps.push(step(
                        "prop.addKey",
                        json!({"layer": layer_id, "path": "transform/opacity", "value": val, "time": tm.seconds(), "timeBase": "comp"}),
                    ));
                }
            }
            steps.extend(trim_steps(&layer_id, shown, gone));
        }

        steps.push(step(
            "markers.set",
            json!({
                "new": true,
                "time": shown.seconds(),
                "duration": (gone - shown).seconds(),
                "comment": line.text(),
                "label": "Blue",
                "cuePoint": {"name": line.layer, "navigation": true, "params": [["id", line.id]]},
            }),
        ));
        summary.push(json!({
            "id": line.id,
            "text": line.text(),
            "in_tick": shown.0,
            "out_tick": gone.0,
            "in": shown.seconds(),
            "out": gone.seconds(),
        }));
    }
    (steps, summary)
}

fn collect_saved_designs(s: &mut Session, dir: &Path, all: &[Line]) -> (Value, Value) {
    let Ok(Some(doc)) = crate::cmd::words::load(s, dir) else {
        return (json!({}), json!({}));
    };
    let mut map = serde_json::Map::new();
    let mut base_style = json!({});
    for anim in doc["animations"].as_array().into_iter().flatten() {
        if base_style.as_object().is_some_and(|m| m.is_empty()) && anim["style"].is_object() {
            base_style = anim["style"].clone();
        }
        for line in all {
            if let Some(d) = anim["design"].get(&line.id) {
                map.insert(line.layer.clone(), d.clone());
            }
        }
    }
    (Value::Object(map), base_style)
}

/// `words.letter`: build `effects/lyrics.ecproj` directly from aligned lyrics and `craft_font_catalog`
/// fonts (no `lyrics-design.json` file required), and optionally render `effects/lyrics.mov` + publish `V3`.
pub fn cmd_words_letter(s: &mut Session, p: &Value) -> Result<Value> {
    let dir = crate::cmd::words::workspace_dir(s)?;
    let all = lines(&crate::cmd::words::aligned_in(&dir)?);
    if all.is_empty() {
        return Err(invalid("No timed lyric lines: import the transcript and the artist's lyric sheet first"));
    }
    let (designs, saved_style) = collect_saved_designs(s, &dir, &all);
    let explicit_recipe = p["recipe"]
        .as_str()
        .map(|raw| crate::workspace::resolve_workspace_path(s, raw))
        .filter(|rp| rp.is_file())
        .and_then(|rp| std::fs::read(rp).ok())
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok());
    let base_cfg = explicit_recipe.unwrap_or_else(|| {
        json!({
            "band": p["band"].as_str().unwrap_or("top"),
            "font": p["font"].as_str().or_else(|| saved_style["font"].as_str()).unwrap_or("sans"),
            "size": p["size"].as_f64().or_else(|| saved_style["size"].as_f64()).unwrap_or(68.0),
            "color": p["color"].as_str().or_else(|| saved_style["color"].as_str()).unwrap_or("#FFF6E3"),
            "outline": p["outline"].as_str().or_else(|| saved_style["outline_color"].as_str()).unwrap_or("#143042"),
        })
    });

    let last = all.last().map_or(Tick::ZERO, |l| l.end);
    let duration = crate::workshop::song_ticks(&dir).unwrap_or(last + Tick(2 * TICKS_PER_SECOND)).max(last);
    let (mut steps, summary) = build_steps_with_designs(&base_cfg, &designs, &all, duration);
    let project = crate::cmd::words::lyrics_ecproj_write_path(&dir);
    if let Some(parent) = project.parent() {
        std::fs::create_dir_all(parent)?;
    }
    steps.push(json!({"command": "file.saveAs", "params": {"path": project}}));
    crate::craft_api::run_effectcraft_steps(None, true, &steps).map_err(|e| EngineError::ExternalTool("effectcraft".into(), e))?;

    let head = crate::cmd::character::check_ecproj_head(&project).ok();
    let should_render = p["render"].as_bool().unwrap_or(false) || p["studio"].as_bool().unwrap_or(false);
    let render = if should_render {
        let movie = dir.join("effects/lyrics.mov");
        crate::craft_api::render_effectcraft_comp(
            &project,
            &json!({"comp": COMP, "output": movie, "format": "prores", "proresProfile": "4444", "channels": "rgba"}),
        )
        .map_err(|e| EngineError::ExternalTool("effectcraft".into(), e))?;
        Some(movie)
    } else {
        None
    };
    let studio = if p["studio"] == true {
        let movie = render.as_deref().ok_or_else(|| invalid("Render `effects/lyrics.mov` before publishing V3 to Studio"))?;
        Some(crate::cmd::film::publish_lyrics(s, movie)?)
    } else {
        None
    };
    Ok(json!({
        "ok": true,
        "project": project,
        "saved_by": head.as_ref().map(|h| &h.saved_by),
        "schema": head.as_ref().map(|h| h.schema),
        "render": render,
        "duration_tick": duration.0,
        "duration": duration.seconds(),
        "lines": summary,
        "studio": studio,
    }))
}

/// Publish rendered alpha lettering (`effects/lyrics.mov`) onto `FilmCraft` `V3`.
pub fn cmd_words_publish(s: &mut Session, p: &Value) -> Result<Value> {
    let dir = crate::cmd::words::workspace_dir(s)?.canonicalize()?;
    let candidate = p["path"].as_str().map(|raw| crate::workspace::resolve_workspace_path(s, raw)).unwrap_or_else(|| crate::cmd::words::lyrics_mov_path(&dir));
    let path = candidate.canonicalize().map_err(|_| invalid(format!("Rendered lyrics movie not found at {}", candidate.display())))?;
    if !path.starts_with(&dir) || !path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("mov")) {
        return Err(invalid("Choose a rendered .mov inside the Magic project"));
    }
    let studio = crate::cmd::film::publish_lyrics(s, &path)?;
    Ok(json!({"ok": true, "render": path, "studio": studio}))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{build_steps_with_designs, lines};
    use crate::time::{TICKS_PER_SECOND, Tick};

    #[test]
    fn spatial_steps_for_turn_and_round_hero_boxes_never_overlap() {
        let aligned = json!({
            "words": [
                {"word_id": "l1/w0", "line_id": "l1", "text": "turn", "start": 1.0, "end": 1.5},
                {"word_id": "l1/w1", "line_id": "l1", "text": "round", "start": 1.55, "end": 2.1}
            ]
        });
        let base_cfg = json!({"band": "top", "font": "Fraunces", "size": 104.0, "color": "#B8442A", "outline": "#FFF8E9"});
        let parsed_lines = lines(&aligned);
        let (steps, _) = build_steps_with_designs(&base_cfg, &json!({}), &parsed_lines, Tick(5 * TICKS_PER_SECOND));

        let text_boxes: Vec<(String, f64, f64)> = steps
            .iter()
            .filter(|s| s["command"] == "layer.newText")
            .map(|s| {
                let b = &s["params"]["box"];
                (s["params"]["text"].as_str().unwrap().to_owned(), b[0].as_f64().unwrap(), b[2].as_f64().unwrap())
            })
            .collect();

        assert_eq!(text_boxes.len(), 2);
        assert_eq!(text_boxes[0].0, "turn");
        assert_eq!(text_boxes[1].0, "round");
        assert!(
            text_boxes[0].1 + text_boxes[0].2 < text_boxes[1].1,
            "EffectCraft text boxes for `turn` ({} + {}) and `round` ({}) must not overlap horizontally",
            text_boxes[0].1,
            text_boxes[0].2,
            text_boxes[1].1
        );
    }
}
