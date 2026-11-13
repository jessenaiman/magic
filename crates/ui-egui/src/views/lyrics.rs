//! `03 Lyrics` view (`#lyrics-workspace-root`): Interactive `EffectCraft` `V3` Kinetic Typography
//! stage hosted inside [`craft_ui::docking`]:
//! - `draw_stage_header`: section publish/save/reload + choreography & move strip.
//! - `draw_navigator_panel`: `lyrics_dock::draw_moment_film_reel` (`Lyric Moments`).
//! - `draw_canvas_panel`: interactive 16:9 kinetic lyrics stage + transport + word pills + captions.
//! - `draw_inspector_panel`: `lyrics_inspector::draw_lyrics_inspector` (`Typography & FX`).
//! - `draw_timeline_panel`: `lyrics_dock::draw_lyrics_timeline_panel` (`Lyrics Timeline`).

use std::time::Duration;

use egui::epaint::CubicBezierShape;
use egui::{Align, Align2, Color32, CornerRadius, FontId, Id, LayerId, Layout, Order, Painter, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Ui, Vec2};
use serde_json::{Value, json};

use crate::controls::{FilmbarOrientation, FilmbarStyle, fill_card, paint_craft_doc_card, paint_filmbar_frame, paint_local_image, pill_toggle};
use crate::editorial::{ScrubberSpec, draw_compact_scrubber};
use crate::state::{UiAction, ViewCtx};
use crate::views::lyrics_dock::{draw_lyrics_timeline_panel, draw_moment_film_reel};
use crate::views::lyrics_inspector::{draw_captions_and_template_strip, draw_lyrics_inspector};
use crate::widgets::{BtnSpec, action_button};

#[derive(Clone)]
pub struct StageWordToken {
    pub id: String,
    pub text: String,
    pub emphasis: bool,
    pub start_sec: f64,
    pub end_sec: f64,
}

#[derive(Default, Clone, Copy)]
struct LyricsTetherAnchors {
    stage_word_bottom: Option<Pos2>,
    token_pill_top: Option<Pos2>,
}

pub fn draw_stage_header(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let anims = ctx.state.cache.words_doc.pointer("/document/animations").and_then(Value::as_array).cloned().unwrap_or_default();
    let active =
        ctx.state.lyrics.selected_anim_id.as_deref().and_then(|id| anims.iter().find(|a| a["animation_id"].as_str() == Some(id))).or_else(|| anims.first());
    ctx.pal.panel_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        draw_stage_header_bar(ui, ctx, active);
    });
}

pub fn draw_navigator_panel(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let _ = draw_moment_film_reel(ui, ctx);
}

pub fn draw_canvas_panel(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let anims = ctx.state.cache.words_doc.pointer("/document/animations").and_then(Value::as_array).cloned().unwrap_or_default();
    let active =
        ctx.state.lyrics.selected_anim_id.as_deref().and_then(|id| anims.iter().find(|a| a["animation_id"].as_str() == Some(id))).or_else(|| anims.first());
    let tokens = collect_stage_word_tokens(ctx, active);
    let mut anchors = LyricsTetherAnchors::default();
    let clip_rect = ui.max_rect();
    draw_interactive_lyrics_stage_card(ui, ctx, active, &tokens, &mut anchors);
    if ctx.state.lyrics.show_tethers && ctx.state.active_modal.is_none() {
        paint_stage_to_pill_tether(ui, ctx, clip_rect, &anchors);
    }
}

pub fn draw_inspector_panel(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let _ = draw_lyrics_inspector(ui, ctx);
}

pub fn draw_timeline_panel(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    draw_lyrics_timeline_panel(ui, ctx);
}

fn cubic_point(p0: Pos2, c1: Pos2, c2: Pos2, p3: Pos2, t: f32) -> Pos2 {
    let u = 1.0 - t;
    let b0 = u * u * u;
    let b1 = 3.0 * u * u * t;
    let b2 = 3.0 * u * t * t;
    let b3 = t * t * t;
    Pos2::new(b0 * p0.x + b1 * c1.x + b2 * c2.x + b3 * p3.x, b0 * p0.y + b1 * c1.y + b2 * c2.y + b3 * p3.y)
}

pub(crate) struct MusicalTetherSpec {
    pub from: Pos2,
    pub to: Pos2,
    pub c1: Pos2,
    pub c2: Pos2,
    pub color_a: Color32,
    pub color_b: Color32,
    pub time_sec: f64,
    pub phase_offset: f64,
}

/// Paint glowing animated cubic-Bézier connection tethers with flowing musical notes (`♪` / `♫` / `♩`).
pub(crate) fn paint_musical_bezier_tether(painter: &Painter, spec: MusicalTetherSpec) {
    let MusicalTetherSpec { from, to, c1, c2, color_a, color_b, time_sec, phase_offset } = spec;
    painter.add(CubicBezierShape::from_points_stroke([from, c1, c2, to], false, Color32::TRANSPARENT, Stroke::new(4.2, color_a.gamma_multiply(0.16))));
    painter.add(CubicBezierShape::from_points_stroke([from, c1, c2, to], false, Color32::TRANSPARENT, Stroke::new(1.5, color_a.gamma_multiply(0.72))));

    painter.circle_filled(from, 3.6, color_a);
    painter.circle_stroke(from, 5.2, Stroke::new(1.0, color_a.gamma_multiply(0.45)));
    painter.circle_filled(to, 3.6, color_b);

    let notes = ["♪", "♫", "♩"];
    for (idx, glyph) in notes.iter().enumerate() {
        let t = ((time_sec * 0.34 + phase_offset + (idx as f64) * 0.33).fract()) as f32;
        let pt = cubic_point(from, c1, c2, to, t);
        let blend = if idx.is_multiple_of(2) { color_a } else { color_b };
        painter.circle_filled(pt, 7.5, Color32::from_rgba_unmultiplied(8, 13, 22, 210));
        painter.circle_stroke(pt, 7.5, Stroke::new(1.0, blend.gamma_multiply(0.65)));
        painter.text(pt, Align2::CENTER_CENTER, *glyph, FontId::proportional(10.5), blend);
    }
}

fn paint_stage_to_pill_tether(ui: &Ui, ctx: &ViewCtx<'_>, clip_rect: Rect, anchors: &LyricsTetherAnchors) {
    let Some((word_b, pill_t)) = anchors.stage_word_bottom.zip(anchors.token_pill_top) else {
        return;
    };
    if (pill_t.y - word_b.y).abs() <= 10.0 {
        return;
    }
    let time_sec = ui.ctx().input(|i| i.time);
    let layer = LayerId::new(Order::Foreground, Id::unique("lyrics_musical_tethers"));
    let painter = ui.ctx().layer_painter(layer).with_clip_rect(clip_rect);
    let dy = ((pill_t.y - word_b.y) * 0.48).clamp(16.0, 72.0);
    paint_musical_bezier_tether(
        &painter,
        MusicalTetherSpec {
            from: word_b,
            to: pill_t,
            c1: Pos2::new(word_b.x, word_b.y + dy),
            c2: Pos2::new(pill_t.x, pill_t.y - dy),
            color_a: ctx.pal.accent_sky,
            color_b: ctx.pal.accent_amber,
            time_sec,
            phase_offset: 0.25,
        },
    );
    ui.ctx().request_repaint_after(Duration::from_millis(48));
}

fn active_section_revision(ctx: &ViewCtx<'_>) -> Option<(String, u64)> {
    let anims = ctx.state.cache.words_doc.pointer("/document/animations").and_then(Value::as_array)?;
    let chosen =
        ctx.state.lyrics.selected_anim_id.as_deref().and_then(|id| anims.iter().find(|a| a["animation_id"].as_str() == Some(id))).or_else(|| anims.first())?;
    let id = chosen["animation_id"].as_str()?.to_owned();
    let rev = chosen["source_revision"].as_u64().unwrap_or(0);
    Some((id, rev))
}

pub fn collect_stage_word_tokens(ctx: &ViewCtx<'_>, active: Option<&Value>) -> Vec<StageWordToken> {
    if let Some(arr) = active.and_then(|a| a["words"].as_array()).filter(|a| !a.is_empty()) {
        let anchor = active.and_then(|a| a["song_anchor"].as_f64()).unwrap_or(0.0);
        return arr
            .iter()
            .take(12)
            .enumerate()
            .filter_map(|(i, w)| {
                let text = w["text"].as_str()?.trim();
                if text.is_empty() {
                    return None;
                }
                let id = w["word_id"].as_str().filter(|s| !s.is_empty()).map_or_else(|| format!("w-{i}"), str::to_owned);
                let st = w["start"].as_f64().unwrap_or(anchor + (i as f64) * 0.42);
                let en = w["end"].as_f64().unwrap_or(st + 0.38);
                let emp = w["emphasis"].as_bool() == Some(true) || text.chars().filter(|c| c.is_alphabetic()).count() >= 7;
                Some(StageWordToken { id, text: text.to_owned(), emphasis: emp, start_sec: st, end_sec: en })
            })
            .collect();
    }

    let line_str = ctx.state.cache.song_lyrics.get("lyrics").and_then(Value::as_str).and_then(|txt| {
        let lines: Vec<&str> = txt.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
        if lines.is_empty() {
            None
        } else {
            let total = ctx.state.audio_duration_sec.max(24.0);
            let idx = (((ctx.state.lyrics.scrub_seconds / total) * (lines.len() as f64)).floor() as usize).min(lines.len().saturating_sub(1));
            Some((idx, lines[idx].to_owned(), lines.len()))
        }
    });

    if let Some((line_idx, line, line_count)) = line_str {
        let total = ctx.state.audio_duration_sec.max(24.0);
        let line_span = total / (line_count.max(1) as f64);
        let line_start = (line_idx as f64) * line_span;
        let parts: Vec<&str> = line.split_whitespace().take(12).collect();
        let word_step = (line_span / (parts.len().max(1) as f64)).clamp(0.22, 0.85);
        return parts
            .into_iter()
            .enumerate()
            .map(|(i, raw)| {
                let st = line_start + (i as f64) * word_step;
                let alpha_len = raw.chars().filter(|c| c.is_alphabetic()).count();
                StageWordToken {
                    id: format!("line{}-w{}", line_idx + 1, i),
                    text: raw.to_owned(),
                    emphasis: alpha_len >= 7 || i == 4,
                    start_sec: st,
                    end_sec: st + word_step * 0.9,
                }
            })
            .collect();
    }

    ["Kinetic", "Lyrics", "Stage", "Ready"]
        .into_iter()
        .enumerate()
        .map(|(i, txt)| StageWordToken {
            id: format!("seed-w{i}"),
            text: txt.to_owned(),
            emphasis: i == 1,
            start_sec: (i as f64) * 0.5,
            end_sec: (i as f64) * 0.5 + 0.45,
        })
        .collect()
}

#[allow(clippy::cognitive_complexity)]
fn draw_stage_header_bar(ui: &mut Ui, ctx: &mut ViewCtx<'_>, active: Option<&Value>) {
    let sec_heading = active.and_then(|a| a["scope"]["section"].as_str().or_else(|| a["animation_id"].as_str())).unwrap_or("Lyrics Stage");
    let ecproj_meta = &ctx.state.cache.words_doc["ecproj"];
    let ecproj_summary = if ecproj_meta.is_object() {
        let layers = ecproj_meta["layer_count"].as_u64().unwrap_or(0);
        format!("lyrics.ecproj · {layers} layers")
    } else {
        "effects/lyrics.ecproj".to_owned()
    };

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.label(RichText::new(sec_heading).size(12.2).strong());
        ui.label(RichText::new(ecproj_summary).monospace().size(9.2).color(ctx.pal.text_muted));
        let _ = pill_toggle(ui, ctx.reg, ctx.pal, "words-tethers-toggle", "Tethers", &mut ctx.state.lyrics.show_tethers, ctx.pal.accent_emerald);
        let _ = pill_toggle(ui, ctx.reg, ctx.pal, "words-warp-toggle", "Warp Arcs", &mut ctx.state.lyrics.show_warp_paths, ctx.pal.accent_sky);
        let _ = pill_toggle(ui, ctx.reg, ctx.pal, "words-flat-preview-toggle", "Flat PNG", &mut ctx.state.lyrics.flat_preview_only, ctx.pal.accent_amber);

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("words-render-studio", "film", "Publish V3", "Build effects/lyrics.ecproj & publish V3 Kinetic Lyrics").primary(),
            )
            .clicked()
            {
                ctx.exec("words.letter", json!({"render": true, "studio": true}), Some("Published Kinetic Lyrics onto V3"));
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("words-open-effectcraft", "external", "Open effects/lyrics.ecproj in EffectCraft"))
                .clicked()
            {
                ctx.exec("workspace.open", json!({"app": "effectcraft", "domain": "lyrics"}), Some("Opened effects/lyrics.ecproj"));
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("words-save", "save", "Save lyric section & sync EffectCraft")).clicked() {
                if let Some((anim_id, rev)) = active_section_revision(ctx) {
                    let payload = json!({
                        "animation_id": anim_id,
                        "expected_source_revision": rev,
                        "teaching_mode": ctx.state.lyrics.teaching_mode,
                        "style": {
                            "size": ctx.state.lyrics.font_size.clamp(40.0, 320.0),
                            "color": ctx.state.lyrics.color_hex,
                            "outline_color": ctx.state.lyrics.outline_hex,
                        }
                    });
                    ctx.state.dirty_drafts.remove("lyrics");
                    ctx.exec("words.save", payload, Some("Saved lyric section"));
                } else {
                    ctx.exec("project.autosave", json!({}), Some("Saved lyric draft"));
                }
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("words-reload", "refresh", "Reload words.read & words.effects")).clicked() {
                ctx.exec("words.read", json!({}), None);
                ctx.exec("words.effects", json!({"refresh": true}), None);
            }
        });
    });

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 3.5;
        ui.label(RichText::new("Choreo:").monospace().size(9.0).color(ctx.pal.text_muted));
        for (cid, lbl, tip) in [
            ("poster", "Poster", "Storybook hero word center-stage"),
            ("cascade", "Cascade", "Diagonal kinetic staircase"),
            ("scatter", "Scatter", "Tilted expressive spread"),
            ("stack", "Stack", "Stepped two-column stack"),
            ("plain", "Sing-Along", "Clean lyric follow line"),
        ] {
            let id = format!("words-choreo-{cid}");
            let sel = ctx.state.lyrics.choreography == cid;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, tip).selected(sel)).clicked() {
                ctx.state.lyrics.choreography = cid.into();
                ctx.state.dirty_drafts.insert("lyrics".into());
            }
        }
        ui.label(RichText::new("·").color(ctx.pal.border));
        for (mid, lbl) in [("pop", "Pop"), ("rise", "Rise"), ("loop", "Wave"), ("fall", "Fall"), ("sleep", "Sleep"), ("settle", "Settle")] {
            let id = format!("words-move-{mid}");
            let sel = ctx.state.lyrics.move_preset == mid;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, "Per-word kinetic act").selected(sel)).clicked() {
                ctx.state.lyrics.move_preset = mid.into();
                ctx.state.dirty_drafts.insert("lyrics".into());
            }
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("words-reset-offsets", "undo", "Reset manual stage word offsets")).clicked() {
            ctx.state.lyrics.word_offsets.clear();
            ctx.state.lyrics.word_scales.clear();
            ctx.state.lyrics.word_rotations.clear();
        }
    });
}

fn draw_compact_music_transport_bar(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let has_audio = ctx.state.song_audio_path().is_some();
    let play_icon = if ctx.state.audio_playing { "pause" } else { "play" };
    let dur = ctx.state.audio_duration_sec.max(30.0);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("words-audio-play", play_icon, "Play / pause song audio").primary().enabled(has_audio))
            .clicked()
        {
            ctx.push(UiAction::ToggleAudio);
        }
        ui.label(RichText::new(format!("0:{:04.1}/{:04.1}", ctx.state.lyrics.scrub_seconds, dur)).monospace().size(9.3).color(if ctx.state.audio_playing {
            ctx.pal.accent_emerald
        } else {
            ctx.pal.text_secondary
        }));

        let scrub_w = (ui.available_width() - 284.0).clamp(80.0, 420.0);
        if let Some(sec) = draw_compact_scrubber(
            ui,
            ctx.reg,
            ctx.pal,
            ScrubberSpec { id: "words-scrub", label: "Lyrics Scrubber", width: scrub_w, duration_sec: dur, accent: ctx.pal.accent_emerald },
            &mut ctx.state.lyrics.scrub_seconds,
        ) {
            ctx.push(UiAction::SeekAudio(sec));
        }

        for (src, lbl, tip) in [
            ("overlay", "FC+V3", "Composite EffectCraft V3 lyrics over FilmCraft V1"),
            ("effectcraft", "EC V3", "Isolate EffectCraft effects/lyrics.ecproj"),
            ("filmcraft", "FC V1", "Preview FilmCraft V1 sequence"),
        ] {
            let id = format!("words-track-src-{src}");
            let sel = ctx.state.lyrics.track_source == src;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, tip).selected(sel)).clicked() {
                ctx.state.lyrics.track_source = src.into();
            }
        }
        let aud_icon = if ctx.state.lyrics.audio_source == "filmcraft" { "waveform" } else { "music" };
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("words-audio-src-toggle", aud_icon, "Toggle Song Stem vs FilmCraft Audio Mix")).clicked() {
            ctx.state.lyrics.audio_source = if ctx.state.lyrics.audio_source == "filmcraft" { "song".into() } else { "filmcraft".into() };
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("words-align-audio", "sparkles", "Run speech alignment on song audio")).clicked() {
            ctx.exec("audio.analyze", json!({}), Some("Aligned song audio"));
        }
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::icon_only("words-captions-drawer-toggle", "captionsCc", "Toggle C1 Broadcast Captions (craft-captions) & .ectemplate")
                .selected(ctx.state.lyrics.draw_enabled),
        )
        .clicked()
        {
            ctx.state.lyrics.draw_enabled = !ctx.state.lyrics.draw_enabled;
        }
    });
}

fn draw_native_word_token_bar(ui: &mut Ui, ctx: &mut ViewCtx<'_>, tokens: &[StageWordToken], anchors: &mut LyricsTetherAnchors) {
    let active_wid = ctx
        .state
        .lyrics
        .selected_word_id
        .clone()
        .or_else(|| tokens.iter().find(|t| t.emphasis).map(|t| t.id.clone()))
        .or_else(|| tokens.first().map(|t| t.id.clone()));

    ctx.pal.inset_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.spacing_mut().item_spacing.y = 3.0;
            ui.label(RichText::new("WORDS:").monospace().size(9.0).color(ctx.pal.text_muted));
            for (idx, tok) in tokens.iter().enumerate() {
                let sel = active_wid.as_deref() == Some(tok.id.as_str());
                let has_fx = ctx.state.lyrics.word_effects.contains_key(&tok.id);
                let btn_id = format!("words-token-pill-{idx}");
                let spec = if has_fx {
                    BtnSpec::new(&btn_id, "sparkles", &tok.text, "Select word on stage & scrub to beat").selected(sel)
                } else if tok.emphasis {
                    BtnSpec::new(&btn_id, "music", &tok.text, "Select emphasized word on stage").selected(sel)
                } else {
                    BtnSpec::label_only(&btn_id, &tok.text, "Select word on stage & scrub to beat").selected(sel)
                };
                let resp = action_button(ui, ctx.reg, ctx.pal, spec);
                if sel {
                    anchors.token_pill_top = Some(Pos2::new(resp.rect.center().x, resp.rect.min.y));
                }
                if resp.clicked() {
                    ctx.state.lyrics.selected_word_id = Some(tok.id.clone());
                    ctx.state.lyrics.scrub_seconds = tok.start_sec;
                    ctx.push(UiAction::SeekAudio(tok.start_sec));
                }
            }
        });
    });
}

fn choreography_base_pos(choreo: &str, idx: usize, total: usize, emphasis: bool) -> (f32, f32, f32) {
    let n = total.max(1) as f32;
    let t = if total <= 1 { 0.5 } else { (idx as f32) / ((total - 1) as f32) };
    match choreo {
        "cascade" => (0.18 + t * 0.64, 0.24 + t * 0.52, if emphasis { -4.0 } else { 0.0 }),
        "scatter" => {
            let row = (idx % 3) as f32;
            let rot = if idx.is_multiple_of(2) { -6.0 } else { 5.5 };
            (0.17 + t * 0.66, 0.26 + row * 0.22, rot)
        }
        "stack" => {
            let col = (idx % 2) as f32;
            let row = (idx / 2) as f32 / ((total.div_ceil(2)).max(1) as f32);
            (0.36 + col * 0.28, 0.22 + row * 0.58, 0.0)
        }
        "plain" => {
            let row = idx / 5;
            let col = idx % 5;
            let cols_in_row = (total.saturating_sub(row * 5)).clamp(1, 5) as f32;
            let nx = 0.18 + ((col as f32 + 0.5) / cols_in_row) * 0.64;
            let ny = if total > 5 { 0.42 + (row as f32) * 0.22 } else { 0.54 };
            (nx, ny, 0.0)
        }
        _ => {
            if total >= 5 {
                let hero_idx = total / 2;
                if idx < hero_idx {
                    let frac = (idx as f32 + 0.5) / (hero_idx.max(1) as f32);
                    (0.22 + frac * 0.56, 0.27, 0.0)
                } else if idx == hero_idx {
                    (0.50, 0.52, -2.0)
                } else {
                    let rem = (total - hero_idx - 1).max(1) as f32;
                    let frac = ((idx - hero_idx - 1) as f32 + 0.5) / rem;
                    (0.32 + frac * 0.42, 0.76, 0.0)
                }
            } else {
                let frac = (idx as f32 + 0.5) / n;
                (0.20 + frac * 0.60, if emphasis { 0.50 } else { 0.36 }, 0.0)
            }
        }
    }
}

#[allow(clippy::cognitive_complexity)]
fn draw_interactive_lyrics_stage_card(
    ui: &mut Ui,
    ctx: &mut ViewCtx<'_>,
    active: Option<&Value>,
    tokens: &[StageWordToken],
    anchors: &mut LyricsTetherAnchors,
) {
    let panel = fill_card(ui, ctx.pal, true, |ui| {
        let captions_extra = if ctx.state.lyrics.draw_enabled { 46.0 } else { 0.0 };
        let avail = ui.available_size();
        let reserved_below = 62.0 + captions_extra;
        let max_h = (avail.y - reserved_below).max(164.0);
        let max_w = (avail.x - 4.0).max(220.0);
        let mut stage_w = max_w;
        let mut stage_h = stage_w * 9.0 / 16.0;
        if stage_h > max_h {
            stage_h = max_h;
            stage_w = (stage_h * 16.0 / 9.0).min(max_w);
        }

        ui.vertical_centered(|ui| {
            let (outer_stage, stage_resp) = ui.allocate_exact_size(Vec2::new(stage_w, stage_h), Sense::click());
            let tc_badge = match ctx.state.lyrics.track_source.as_str() {
                "filmcraft" => "V1 · FilmCraft .fcproj",
                "effectcraft" => "V3 · Interactive lyrics.ecproj",
                _ => "FC V1 + Interactive EC V3 Stage",
            };
            let stage_rect = paint_filmbar_frame(
                ui,
                outer_stage,
                ctx.pal,
                FilmbarStyle { orientation: FilmbarOrientation::Horizontal, selected: false, hovered: stage_resp.hovered(), timecode: Some(tc_badge) },
            );
            ctx.reg.record("words-stage", "Lyrics Interactive Stage", &stage_resp, None);

            let film_bg = ctx.state.film_frame_at_time(ctx.state.lyrics.scrub_seconds);
            let scene_bg = active.and_then(|a| a["background_path"].as_str()).filter(|p| !p.is_empty()).or(film_bg.as_deref());
            let rendered_frame =
                ctx.state.cache.last_words_frame.as_deref().or_else(|| active.and_then(|a| a["preview_frame"].as_str().or_else(|| a["lyrics_frame"].as_str())));
            let bg = if ctx.state.lyrics.backdrop_mode == "paper" { ctx.pal.stage_paper } else { ctx.pal.bg_stage };

            if ctx.state.lyrics.track_source == "filmcraft" {
                if !paint_local_image(ui, stage_rect, film_bg.as_deref(), ctx.pal.bg_stage, 4) {
                    paint_craft_doc_card(
                        ui,
                        stage_rect.shrink(14.0),
                        ctx.pal,
                        "FC",
                        "film/Song.fcproj",
                        "Switch to FC+V3 or EC V3 to select and edit kinetic lyrics directly on stage",
                    );
                }
            } else if ctx.state.lyrics.flat_preview_only {
                if !paint_local_image(ui, stage_rect, rendered_frame.or(scene_bg), bg, 4) {
                    ui.painter().rect_filled(stage_rect, CornerRadius::same(4), bg);
                }
            } else {
                if ctx.state.lyrics.backdrop_mode == "scene" {
                    if !paint_local_image(ui, stage_rect, scene_bg, bg, 4) {
                        ui.painter().rect_filled(stage_rect, CornerRadius::same(4), bg);
                    }
                    ui.painter().rect_filled(stage_rect, CornerRadius::same(4), Color32::from_rgba_unmultiplied(7, 11, 18, 92));
                } else {
                    ui.painter().rect_filled(stage_rect, CornerRadius::same(4), bg);
                }
                paint_interactive_stage_words(ui, ctx, stage_rect, tokens, anchors);
            }
        });

        ui.add_space(2.0);
        draw_compact_music_transport_bar(ui, ctx);
        draw_native_word_token_bar(ui, ctx, tokens, anchors);

        if ctx.state.lyrics.draw_enabled {
            draw_captions_and_template_strip(ui, ctx);
        }
    });
    ctx.reg.record("words-stage-panel", "Stage Panel", &panel.response, None);
}

#[allow(clippy::cognitive_complexity)]
fn paint_interactive_stage_words(ui: &mut Ui, ctx: &mut ViewCtx<'_>, stage_rect: Rect, tokens: &[StageWordToken], anchors: &mut LyricsTetherAnchors) {
    if tokens.is_empty() {
        paint_craft_doc_card(ui, stage_rect.shrink(14.0), ctx.pal, "EC", "effects/lyrics.ecproj", "Add lyrics or align audio");
        return;
    }

    let guide_col = Color32::from_rgba_unmultiplied(140, 175, 210, 18);
    for frac in [1.0_f32 / 3.0, 2.0_f32 / 3.0] {
        let gx = stage_rect.min.x + stage_rect.width() * frac;
        let gy = stage_rect.min.y + stage_rect.height() * frac;
        ui.painter().line_segment([Pos2::new(gx, stage_rect.min.y), Pos2::new(gx, stage_rect.max.y)], Stroke::new(1.0, guide_col));
        ui.painter().line_segment([Pos2::new(stage_rect.min.x, gy), Pos2::new(stage_rect.max.x, gy)], Stroke::new(1.0, guide_col));
    }

    let default_sel = tokens.iter().find(|t| t.emphasis).or_else(|| tokens.first()).map(|t| t.id.clone());
    if ctx.state.lyrics.selected_word_id.is_none() {
        ctx.state.lyrics.selected_word_id = default_sel;
    }

    let choreo = ctx.state.lyrics.choreography.clone();
    let base_font_scale = (ctx.state.lyrics.font_size as f32 / 56.0).clamp(0.55, 1.95);
    let mut centers: Vec<Pos2> = Vec::with_capacity(tokens.len());
    let mut boxes: Vec<Rect> = Vec::with_capacity(tokens.len());

    for (idx, tok) in tokens.iter().enumerate() {
        let (bx, by, _) = choreography_base_pos(&choreo, idx, tokens.len(), tok.emphasis);
        let [dx, dy] = ctx.state.lyrics.word_offsets.get(&tok.id).copied().unwrap_or([0.0, 0.0]);
        let w_scale = ctx.state.lyrics.word_scales.get(&tok.id).copied().unwrap_or(1.0);
        let cx = stage_rect.min.x + ((bx + dx).clamp(0.08, 0.92)) * stage_rect.width();
        let cy = stage_rect.min.y + ((by + dy).clamp(0.12, 0.88)) * stage_rect.height();

        let hero_mult = if tok.emphasis || (choreo == "poster" && idx == tokens.len() / 2) { 1.72 } else { 0.98 };
        let px = (20.0 * hero_mult * base_font_scale * w_scale * (stage_rect.height() / 230.0).clamp(0.80, 1.65)).clamp(13.0, 72.0);
        let galley = ui.painter().layout_no_wrap(tok.text.clone(), FontId::proportional(px), Color32::WHITE);
        let pad = Vec2::new(14.0, 7.0);
        let wrect = Rect::from_center_size(Pos2::new(cx, cy), galley.size() + pad);
        centers.push(Pos2::new(cx, cy));
        boxes.push(wrect);
    }

    if ctx.state.lyrics.show_warp_paths && centers.len() >= 2 {
        for i in 0..(centers.len() - 1) {
            let p0 = centers[i];
            let p3 = centers[i + 1];
            let mid_x = (p0.x + p3.x) * 0.5;
            let arc_lift = if i.is_multiple_of(2) { -24.0 } else { 20.0 };
            let c1 = Pos2::new(mid_x, p0.y.min(p3.y) + arc_lift);
            let c2 = Pos2::new(mid_x, p0.y.max(p3.y) + arc_lift * 0.5);
            ui.painter().add(CubicBezierShape::from_points_stroke(
                [p0, c1, c2, p3],
                false,
                Color32::TRANSPARENT,
                Stroke::new(1.3, ctx.pal.accent_sky.gamma_multiply(0.42)),
            ));
            let mid_pt = cubic_point(p0, c1, c2, p3, 0.5);
            ui.painter().circle_filled(mid_pt, 2.3, ctx.pal.accent_amber.gamma_multiply(0.72));
        }
    }

    let scrub_t = ctx.state.lyrics.scrub_seconds;
    for (idx, tok) in tokens.iter().enumerate() {
        let wrect = boxes[idx];
        let is_sel = ctx.state.lyrics.selected_word_id.as_deref() == Some(tok.id.as_str());
        let is_sung_now = scrub_t >= tok.start_sec && scrub_t <= tok.end_sec + 0.15;
        let (_, _, base_rot) = choreography_base_pos(&choreo, idx, tokens.len(), tok.emphasis);
        let rot_deg = ctx.state.lyrics.word_rotations.get(&tok.id).copied().unwrap_or(base_rot);
        let fx_opt = ctx.state.lyrics.word_effects.get(&tok.id).cloned();

        let hit = ui.interact(wrect.expand(3.0), ui.scope_id().with(("stage_word_drag", &tok.id)), Sense::click_and_drag());
        if hit.clicked() || hit.drag_started() {
            ctx.state.lyrics.selected_word_id = Some(tok.id.clone());
            ctx.state.lyrics.rotate_deg = f64::from(rot_deg);
            if let Some(ref fx_id) = fx_opt {
                ctx.state.lyrics.selected_effect_id = fx_id.clone();
            }
        }
        if hit.dragged() {
            let delta = hit.drag_delta();
            if delta.length_sq() > 0.0 {
                let entry = ctx.state.lyrics.word_offsets.entry(tok.id.clone()).or_insert([0.0, 0.0]);
                entry[0] = (entry[0] + delta.x / stage_rect.width().max(1.0)).clamp(-0.46, 0.46);
                entry[1] = (entry[1] + delta.y / stage_rect.height().max(1.0)).clamp(-0.44, 0.44);
                ctx.state.dirty_drafts.insert("lyrics".into());
            }
        }

        let font_px = (wrect.height() - 7.0).clamp(13.0, 72.0);
        let fg = if is_sung_now {
            ctx.pal.accent_emerald
        } else if tok.emphasis || idx == tokens.len() / 2 {
            ctx.pal.accent_amber
        } else if ctx.state.lyrics.backdrop_mode == "paper" {
            Color32::from_rgb(0x17, 0x3f, 0x49)
        } else {
            ctx.pal.text_primary
        };

        if has_glow_or_emphasis(fx_opt.as_deref(), is_sel, is_sung_now) {
            ui.painter().rect_filled(wrect.expand(2.0), CornerRadius::same(6), fg.gamma_multiply(if is_sel { 0.14 } else { 0.08 }));
        }
        ui.painter().text(
            wrect.center() + Vec2::new(2.0, 2.2),
            Align2::CENTER_CENTER,
            &tok.text,
            FontId::proportional(font_px),
            Color32::from_rgba_unmultiplied(6, 10, 16, 225),
        );
        ui.painter().text(wrect.center(), Align2::CENTER_CENTER, &tok.text, FontId::proportional(font_px), fg);

        if let Some(ref fx_id) = fx_opt {
            let short_fx = fx_id.rsplit('.').next().unwrap_or(fx_id);
            let tag_pos = Pos2::new(wrect.max.x, wrect.min.y - 5.0);
            ui.painter().text(tag_pos, Align2::RIGHT_BOTTOM, short_fx, FontId::monospace(8.4), ctx.pal.accent_emerald);
        }

        if is_sel {
            draw_selected_word_gizmo(ui, ctx, [stage_rect, wrect], tok, base_rot, anchors);
        }

        ctx.reg.record(format!("word-box-{idx}"), &tok.text, &hit, None);
    }
}

fn draw_selected_word_gizmo(
    ui: &mut Ui,
    ctx: &mut ViewCtx<'_>,
    [stage_rect, wrect]: [Rect; 2],
    tok: &StageWordToken,
    base_rot: f32,
    anchors: &mut LyricsTetherAnchors,
) {
    let sel_col = ctx.pal.accent_sky;
    let rot_deg = ctx.state.lyrics.word_rotations.get(&tok.id).copied().unwrap_or(base_rot);
    ui.painter().rect_stroke(wrect, CornerRadius::same(4), Stroke::new(1.6, sel_col), StrokeKind::Outside);

    for (ci, cpos) in [wrect.left_top(), wrect.right_top(), wrect.left_bottom(), wrect.right_bottom()].iter().enumerate() {
        let hrect = Rect::from_center_size(*cpos, Vec2::splat(8.0));
        ui.painter().rect_filled(hrect, CornerRadius::same(1), Color32::WHITE);
        ui.painter().rect_stroke(hrect, CornerRadius::same(1), Stroke::new(1.2, sel_col), StrokeKind::Outside);
        let c_hit = ui.interact(hrect.expand(4.0), ui.scope_id().with(("word_scale_corner", &tok.id, ci)), Sense::drag());
        if c_hit.dragged() {
            let sign_y = if ci < 2 { -1.0 } else { 1.0 };
            let cur = ctx.state.lyrics.word_scales.entry(tok.id.clone()).or_insert(1.0);
            *cur = (*cur + c_hit.drag_delta().y * sign_y * 0.012).clamp(0.45, 2.4);
            ctx.state.lyrics.scale_x = f64::from(*cur * 100.0);
            ctx.state.lyrics.scale_y = f64::from(*cur * 100.0);
            ctx.state.dirty_drafts.insert("lyrics".into());
        }
    }

    let top_mid = Pos2::new(wrect.center().x, wrect.min.y);
    let rot_knob = Pos2::new(wrect.center().x, (wrect.min.y - 16.0).max(stage_rect.min.y + 6.0));
    ui.painter().line_segment([top_mid, rot_knob], Stroke::new(1.3, sel_col));
    ui.painter().circle_filled(rot_knob, 4.5, Color32::WHITE);
    ui.painter().circle_stroke(rot_knob, 4.5, Stroke::new(1.3, ctx.pal.accent_emerald));
    let r_hit = ui.interact(Rect::from_center_size(rot_knob, Vec2::splat(13.0)), ui.scope_id().with(("word_rot_knob", &tok.id)), Sense::drag());
    if r_hit.dragged() {
        let cur_r = ctx.state.lyrics.word_rotations.entry(tok.id.clone()).or_insert(base_rot);
        *cur_r = (*cur_r + r_hit.drag_delta().x * 0.8).clamp(-180.0, 180.0);
        ctx.state.lyrics.rotate_deg = f64::from(*cur_r);
        ctx.state.dirty_drafts.insert("lyrics".into());
    }

    let hud_txt = if rot_deg.abs() > 0.4 { format!("{:.2}s · {:.0}°", tok.start_sec, rot_deg) } else { format!("{:.2}s · drag to place", tok.start_sec) };
    ui.painter().text(
        Pos2::new(wrect.center().x, (wrect.max.y + 8.5).min(stage_rect.max.y - 5.0)),
        Align2::CENTER_CENTER,
        hud_txt,
        FontId::monospace(8.2),
        sel_col,
    );

    anchors.stage_word_bottom = Some(Pos2::new(wrect.center().x, wrect.max.y));
}

fn has_glow_or_emphasis(fx_id: Option<&str>, is_sel: bool, is_sung_now: bool) -> bool {
    is_sel || is_sung_now || fx_id.is_some_and(|id| id.contains("glow") || id.contains("sweep") || id.contains("star"))
}
