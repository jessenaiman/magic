//! `Lyrics` view (`#lyrics-workspace-root`): Interactive `EffectCraft` `V3` Kinetic Typography
//! editor matching `05-lyrics-v3-effectcraft.jpg` and the Argon kinetic lyric concept:
//! - **Interactive 16:9 Stage**: Selectable, pointer-draggable kinetic words over the scene backdrop
//!   with cyan/emerald selection frame, 4 corner scale handles, top rotation stem handle, and curved
//!   inter-word warp trajectories (never flattened into a dead PNG video player).
//! - **Animated Musical-Note Connections (`♪` / `♫` / `♩`)**: Flowing cubic-Bézier tethers linking
//!   the active Left Film Reel moment → Selected Stage Word → Word Token Pill & Active Craft Effect.
//! - **Compact Music Transport**: Single sleek 26px scrubber/transport bar under the stage so
//!   Choreography, Quick Craft Effects (`words.effects` `--control :9877` API), and the interactive
//!   Cubic-Bézier Easing Curve stay prominent in the center workspace.

use std::time::Duration;

use craft_ui::layout::{SplitAxis, SplitSize, split_rect};
use egui::epaint::CubicBezierShape;
use egui::{
    Align, Align2, Color32, CornerRadius, FontId, Id, LayerId, Layout, Order, Painter, Pos2, Rect, RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui, Vec2,
};
use serde_json::{Value, json};

use crate::controls::{
    FilmbarOrientation, FilmbarStyle, badge_accent_color, child_pane, fill_card, paint_craft_doc_card, paint_filmbar_frame, paint_local_image, pill_toggle,
};
use crate::editorial::{ScrubberSpec, ThreePaneSpec, draw_compact_scrubber, draw_three_pane_workspace};
use crate::state::{UiAction, ViewCtx};
use crate::timeline::{CraftTimelineLane, CraftTimelineSpan, draw_craft_timeline};
use crate::views::lyrics_inspector::{apply_effect_to_selected_word, draw_captions_and_template_strip, draw_lyrics_inspector, resolved_lyrics_effects};
use crate::widgets::{BtnSpec, action_button, status_pill, text_input};

struct LyricReelEntry {
    id: String,
    anchor_sec: f64,
    line_text: String,
    frame_path: Option<String>,
}

#[derive(Clone)]
struct StageWordToken {
    id: String,
    text: String,
    emphasis: bool,
    start_sec: f64,
    end_sec: f64,
}

#[derive(Default, Clone, Copy)]
struct LyricsTetherAnchors {
    reel_socket: Option<Pos2>,
    stage_word_left: Option<Pos2>,
    stage_word_bottom: Option<Pos2>,
    stage_word_right: Option<Pos2>,
    token_pill_top: Option<Pos2>,
    effect_anchor: Option<Pos2>,
}

pub fn show(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let wide = ui.available_width() >= 1180.0;
    let mut anchors = LyricsTetherAnchors::default();
    let show_left = ctx.state.show_nav_rail;
    let show_right = ctx.state.show_right_inspector;

    let body = draw_three_pane_workspace(
        ui,
        ThreePaneSpec {
            id_prefix: "lyrics_ws",
            left_width: if wide { 236.0 } else { 208.0 },
            right_width: if wide { 328.0 } else { 284.0 },
            gap: 8.0,
            min_center: 220.0,
            show_left,
            show_right,
        },
        &mut (&mut *ctx, &mut anchors),
        |rail, (ctx, anchors)| {
            anchors.reel_socket = draw_moment_film_reel(rail, ctx);
        },
        |center, (ctx, anchors)| {
            draw_center_kinetic_workspace(center, ctx, anchors);
        },
        |insp, (ctx, anchors)| {
            if let Some(insp_fx) = draw_lyrics_inspector(insp, ctx) {
                anchors.effect_anchor = Some(insp_fx);
            }
        },
    );

    if ctx.state.lyrics.show_tethers && ctx.state.active_modal.is_none() {
        paint_animated_musical_tethers(ui, ctx, body, &anchors);
    }
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
        let blend = if idx % 2 == 0 { color_a } else { color_b };
        painter.circle_filled(pt, 7.5, Color32::from_rgba_unmultiplied(8, 13, 22, 210));
        painter.circle_stroke(pt, 7.5, Stroke::new(1.0, blend.gamma_multiply(0.65)));
        painter.text(pt, Align2::CENTER_CENTER, *glyph, FontId::proportional(10.5), blend);
    }
}

fn paint_animated_musical_tethers(ui: &Ui, ctx: &ViewCtx<'_>, clip_rect: Rect, anchors: &LyricsTetherAnchors) {
    let time_sec = ui.ctx().input(|i| i.time);
    let layer = LayerId::new(Order::Foreground, Id::unique("lyrics_musical_tethers"));
    let painter = ui.ctx().layer_painter(layer).with_clip_rect(clip_rect);

    if let (Some(reel), Some(word_l)) = (anchors.reel_socket, anchors.stage_word_left)
        && clip_rect.contains(reel)
        && clip_rect.contains(word_l)
    {
        let span = ((word_l.x - reel.x) * 0.45).clamp(32.0, 140.0);
        paint_musical_bezier_tether(
            &painter,
            MusicalTetherSpec {
                from: reel,
                to: word_l,
                c1: Pos2::new(reel.x + span, reel.y),
                c2: Pos2::new(word_l.x - span, word_l.y),
                color_a: ctx.pal.accent_emerald,
                color_b: ctx.pal.accent_sky,
                time_sec,
                phase_offset: 0.0,
            },
        );
    }

    if let (Some(word_b), Some(pill_t)) = (anchors.stage_word_bottom, anchors.token_pill_top)
        && (pill_t.y - word_b.y).abs() > 10.0
    {
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
    }

    if let (Some(word_r), Some(fx_pt)) = (anchors.stage_word_right, anchors.effect_anchor)
        && clip_rect.contains(fx_pt)
    {
        let dx = ((fx_pt.x - word_r.x).abs() * 0.42).clamp(28.0, 150.0);
        paint_musical_bezier_tether(
            &painter,
            MusicalTetherSpec {
                from: word_r,
                to: fx_pt,
                c1: Pos2::new(word_r.x + dx, word_r.y),
                c2: Pos2::new(fx_pt.x - dx, fx_pt.y),
                color_a: ctx.pal.accent_amber,
                color_b: ctx.pal.accent_emerald,
                time_sec,
                phase_offset: 0.52,
            },
        );
    }

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

fn collect_lyric_reel_entries(ctx: &ViewCtx<'_>, anims: &[Value]) -> Vec<LyricReelEntry> {
    let film_frames = ctx.state.active_film_frames();
    if !anims.is_empty() {
        return anims
            .iter()
            .enumerate()
            .map(|(idx, anim)| {
                let id = anim["animation_id"].as_str().unwrap_or("sec").to_owned();
                let sec_name = anim["scope"]["section"].as_str().unwrap_or(&id);
                let anchor_sec = anim["song_anchor"].as_f64().unwrap_or((idx as f64) * 4.0);
                let words_str: String = anim["words"].as_array().into_iter().flatten().filter_map(|w| w["text"].as_str()).take(8).collect::<Vec<_>>().join(" ");
                let line_text = if words_str.is_empty() { sec_name.to_owned() } else { words_str };
                let frame_path = anim["preview_frame"]
                    .as_str()
                    .or_else(|| anim["lyrics_frame"].as_str())
                    .or_else(|| anim["background_path"].as_str())
                    .filter(|p| !p.is_empty())
                    .map(str::to_owned)
                    .or_else(|| if film_frames.is_empty() { None } else { film_frames.get(idx % film_frames.len()).cloned() });
                LyricReelEntry { id, anchor_sec, line_text, frame_path }
            })
            .collect();
    }

    let raw_lines: Vec<String> = ctx
        .state
        .cache
        .song_lyrics
        .get("lyrics")
        .and_then(Value::as_str)
        .map(|txt| txt.lines().map(str::trim).filter(|l| !l.is_empty()).take(8).map(str::to_owned).collect())
        .unwrap_or_default();

    let count = raw_lines.len().max(film_frames.len()).clamp(4, 8);
    let total = ctx.state.audio_duration_sec.max(24.0);
    (0..count)
        .map(|idx| {
            let anchor_sec = (idx as f64) * (total / (count as f64));
            let line_text = raw_lines.get(idx).cloned().unwrap_or_else(|| format!("Reel Beat {} · Align Audio", idx + 1));
            let frame_path = if film_frames.is_empty() { None } else { film_frames.get(idx % film_frames.len()).cloned() };
            LyricReelEntry { id: format!("reel-{}", idx + 1), anchor_sec, line_text, frame_path }
        })
        .collect()
}

/// Resolve interactive stage words for the active section — even when `words_doc` has not been aligned yet!
fn collect_stage_word_tokens(ctx: &ViewCtx<'_>, active: Option<&Value>) -> Vec<StageWordToken> {
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

    // Starter kinetic phrase when no song lyric sheet is loaded yet
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

/// Dedicated Left 35mm Film Reel (`FilmbarOrientation::Vertical`), returning right-edge socket `Pos2` of selected card.
fn draw_moment_film_reel(ui: &mut Ui, ctx: &mut ViewCtx<'_>) -> Option<Pos2> {
    let anims = ctx.state.cache.words_doc.pointer("/document/animations").and_then(Value::as_array).cloned().unwrap_or_default();
    let entries = collect_lyric_reel_entries(ctx, &anims);
    let mut selected_socket = None;

    let panel = fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Film Reel · Lyrics").size(12.2).strong());
            status_pill(
                ui,
                ctx.pal,
                if anims.is_empty() { "Scene Reel" } else { "Timed V3" },
                if anims.is_empty() { ctx.pal.accent_amber } else { ctx.pal.accent_emerald },
            );
        });
        text_input(
            ui,
            ctx.reg,
            "words-lyrics-search",
            "Search lyrics",
            &mut ctx.state.lyrics.search,
            "Filter lyric moments…",
            (ui.available_width() - 4.0).max(120.0),
        );
        ui.add_space(4.0);

        let query = ctx.state.lyrics.search.trim().to_ascii_lowercase();
        ScrollArea::vertical().id_salt("words_moments_scroll").auto_shrink([false, false]).show(ui, |ui| {
            for (idx, item) in entries.iter().enumerate() {
                if !query.is_empty() && !item.line_text.to_ascii_lowercase().contains(&query) && !item.id.to_ascii_lowercase().contains(&query) {
                    continue;
                }
                let selected = ctx.state.lyrics.selected_anim_id.as_deref().map_or(idx == 0, |sel| sel == item.id)
                    || (anims.is_empty() && (ctx.state.lyrics.scrub_seconds - item.anchor_sec).abs() < 2.2);

                let reel_w = (ui.available_width() - 4.0).max(132.0);
                let (crect, resp) = ui.allocate_exact_size(Vec2::new(reel_w, 96.0), Sense::click());
                let inner = paint_filmbar_frame(
                    ui,
                    crect,
                    ctx.pal,
                    FilmbarStyle { orientation: FilmbarOrientation::Vertical, selected, hovered: resp.hovered(), timecode: None },
                );
                if !paint_local_image(ui, inner, item.frame_path.as_deref(), ctx.pal.bg_stage, 3) {
                    paint_craft_doc_card(ui, inner, ctx.pal, "EC", "lyrics.ecproj", "");
                }
                let band = Rect::from_min_max(Pos2::new(inner.min.x, (inner.max.y - 30.0).max(inner.min.y)), inner.max);
                ui.painter().rect_filled(band, CornerRadius::same(2), Color32::from_rgba_unmultiplied(8, 12, 18, 224));
                ui.painter().text(
                    Pos2::new(band.min.x + 6.0, band.min.y + 8.0),
                    Align2::LEFT_CENTER,
                    format!("♪ 0:{:04.1}", item.anchor_sec),
                    FontId::monospace(8.6),
                    if selected { ctx.pal.accent_emerald } else { ctx.pal.accent_amber },
                );
                ui.painter().text(
                    Pos2::new(band.min.x + 6.0, band.max.y - 8.0),
                    Align2::LEFT_CENTER,
                    truncate_words(&item.line_text, 22),
                    FontId::proportional(10.2),
                    ctx.pal.text_primary,
                );
                if selected {
                    selected_socket = Some(Pos2::new(crect.max.x - 2.0, crect.center().y));
                }
                if resp.clicked() {
                    if !anims.is_empty() {
                        ctx.state.lyrics.selected_anim_id = Some(item.id.clone());
                    }
                    ctx.state.lyrics.selected_word_id = None;
                    ctx.state.lyrics.scrub_seconds = item.anchor_sec;
                    ctx.push(UiAction::SeekAudio(item.anchor_sec));
                }
                ctx.reg.record(format!("words-moment-{}-0", item.id), &item.line_text, &resp, None);
                ui.add_space(5.0);
            }
        });
    });
    ctx.reg.record("words-lyrics", "Lyric Film Reel", &panel.response, None);
    selected_socket
}

fn truncate_words(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_owned()
    } else {
        let mut out: String = s.chars().take(max_chars.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

fn draw_center_kinetic_workspace(ui: &mut Ui, ctx: &mut ViewCtx<'_>, anchors: &mut LyricsTetherAnchors) {
    let anims = ctx.state.cache.words_doc.pointer("/document/animations").and_then(Value::as_array).cloned().unwrap_or_default();
    let active =
        ctx.state.lyrics.selected_anim_id.as_deref().and_then(|id| anims.iter().find(|a| a["animation_id"].as_str() == Some(id))).or_else(|| anims.first());
    let tokens = collect_stage_word_tokens(ctx, active);

    let center_rect = ui.available_rect_before_wrap();
    // Compact lower studio dock (Choreography + Quick Craft FX + Bezier Easing + Slim Beat Sync)
    let dock_h = (center_rect.height() * 0.43).clamp(186.0, 264.0);
    if ctx.state.show_bottom_dock
        && let Some(vert) = split_rect(center_rect, SplitAxis::Vertical, SplitSize::FixedSecond(dock_h), 6.0, 170.0)
    {
        let mut stage_ui = child_pane(ui, vert.first, "lyrics_upper_stage");
        draw_interactive_lyrics_stage_card(&mut stage_ui, ctx, active, &tokens, anchors);
        let mut dock_ui = child_pane(ui, vert.second, "lyrics_lower_dock");
        draw_choreography_fx_and_beat_dock(&mut dock_ui, ctx, &anims, active, &tokens, anchors);
        let _ = ui.allocate_rect(center_rect, Sense::hover());
    } else {
        draw_interactive_lyrics_stage_card(ui, ctx, active, &tokens, anchors);
    }
}

fn draw_stage_header_bar(ui: &mut Ui, ctx: &mut ViewCtx<'_>, active: Option<&Value>) {
    let sec_heading = active.and_then(|a| a["scope"]["section"].as_str().or_else(|| a["animation_id"].as_str())).unwrap_or("Lyrics Stage");
    let ecproj_meta = &ctx.state.cache.words_doc["ecproj"];
    let ecproj_summary = if ecproj_meta.is_object() {
        let layers = ecproj_meta["layer_count"].as_u64().unwrap_or(0);
        format!("lyrics.ecproj · {layers} layers")
    } else {
        "lettering/lyrics.ecproj".to_owned()
    };

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.label(RichText::new(sec_heading).size(12.2).strong());
        ui.label(RichText::new(ecproj_summary).monospace().size(9.2).color(ctx.pal.text_muted));
        let _ = pill_toggle(ui, ctx.reg, ctx.pal, "words-tethers-toggle", "♪ Tethers", &mut ctx.state.lyrics.show_tethers, ctx.pal.accent_emerald);
        let _ = pill_toggle(ui, ctx.reg, ctx.pal, "words-warp-toggle", "Warp Arcs", &mut ctx.state.lyrics.show_warp_paths, ctx.pal.accent_sky);
        let _ = pill_toggle(ui, ctx.reg, ctx.pal, "words-flat-preview-toggle", "Flat PNG", &mut ctx.state.lyrics.flat_preview_only, ctx.pal.accent_amber);

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("words-render-studio", "film", "Publish V3", "Build lettering/lyrics.ecproj & publish V3 Kinetic Lyrics").primary(),
            )
            .clicked()
            {
                ctx.exec("words.letter", json!({"render": true, "studio": true}), Some("Published Kinetic Lyrics onto V3"));
            }
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("words-open-effectcraft", "external", "EffectCraft", "Open lettering/lyrics.ecproj in EffectCraft"),
            )
            .clicked()
            {
                ctx.exec("workspace.open", json!({"app": "effectcraft", "domain": "lyrics"}), Some("Opened lettering/lyrics.ecproj"));
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("words-save", "save", "Save", "Save lyric section & sync open EffectCraft app")).clicked() {
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
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("words-reload", "refresh", "Reload words.read & words.effects (--control API)")).clicked()
            {
                ctx.exec("words.read", json!({}), None);
                ctx.exec("words.effects", json!({"refresh": true}), None);
            }
        });
    });
}

/// Sleek single-line 24px music transport + scrubber + compact source selector (replaces bulky player).
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

        let scrub_w = (ui.available_width() - 312.0).clamp(80.0, 380.0);
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
            ("effectcraft", "EC V3", "Isolate EffectCraft lettering/lyrics.ecproj"),
            ("filmcraft", "FC V1", "Preview FilmCraft V1 sequence"),
        ] {
            let id = format!("words-track-src-{src}");
            let sel = ctx.state.lyrics.track_source == src;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, tip).selected(sel)).clicked() {
                ctx.state.lyrics.track_source = src.into();
            }
        }
        let aud_lbl = if ctx.state.lyrics.audio_source == "filmcraft" { "FC Mix" } else { "♪ Song" };
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("words-audio-src-toggle", aud_lbl, "Toggle Song Stem vs FilmCraft Audio Mix")).clicked() {
            ctx.state.lyrics.audio_source = if ctx.state.lyrics.audio_source == "filmcraft" { "song".into() } else { "filmcraft".into() };
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("words-align-audio", "Align", "Run speech alignment on song audio")).clicked() {
            ctx.exec("audio.analyze", json!({}), Some("Aligned song audio"));
        }
        let cap_lbl = if ctx.state.lyrics.draw_enabled { "C1 ▴" } else { "C1 ▾" };
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::label_only("words-captions-drawer-toggle", cap_lbl, "Toggle C1 Broadcast Captions & .ectemplate").selected(ctx.state.lyrics.draw_enabled),
        )
        .clicked()
        {
            ctx.state.lyrics.draw_enabled = !ctx.state.lyrics.draw_enabled;
        }
    });
}

/// Interactive bracketed word token strip (`[ All · the · pterodactyls · were · sleeping ✨ · very · still. ]`).
fn draw_bracketed_word_token_bar(ui: &mut Ui, ctx: &mut ViewCtx<'_>, tokens: &[StageWordToken], anchors: &mut LyricsTetherAnchors) {
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
            ui.spacing_mut().item_spacing.x = 3.5;
            ui.spacing_mut().item_spacing.y = 3.0;
            ui.label(RichText::new("[").monospace().size(12.0).strong().color(ctx.pal.accent_emerald));
            for (idx, tok) in tokens.iter().enumerate() {
                let sel = active_wid.as_deref() == Some(tok.id.as_str());
                let has_fx = ctx.state.lyrics.word_effects.contains_key(&tok.id);
                let label = if has_fx {
                    format!("{} ✨", tok.text)
                } else if tok.emphasis {
                    format!("♪ {}", tok.text)
                } else {
                    tok.text.clone()
                };
                let btn_id = format!("words-token-pill-{idx}");
                let resp = action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::label_only(&btn_id, &label, "Click to select word on stage & scrub to its beat").selected(sel),
                );
                if sel {
                    anchors.token_pill_top = Some(Pos2::new(resp.rect.center().x, resp.rect.min.y));
                }
                if resp.clicked() {
                    ctx.state.lyrics.selected_word_id = Some(tok.id.clone());
                    ctx.state.lyrics.scrub_seconds = tok.start_sec;
                    ctx.push(UiAction::SeekAudio(tok.start_sec));
                }
                if idx + 1 < tokens.len() {
                    ui.label(RichText::new("·").monospace().color(ctx.pal.text_muted));
                }
            }
            ui.label(RichText::new("]").monospace().size(12.0).strong().color(ctx.pal.accent_emerald));
        });
    });
}

fn choreography_base_pos(choreo: &str, idx: usize, total: usize, emphasis: bool) -> (f32, f32, f32) {
    let n = total.max(1) as f32;
    let t = if total <= 1 { 0.5 } else { (idx as f32) / ((total - 1) as f32) };
    match choreo {
        "cascade" => {
            let nx = 0.18 + t * 0.64;
            let ny = 0.24 + t * 0.52;
            (nx, ny, if emphasis { -4.0 } else { 0.0 })
        }
        "scatter" => {
            let row = (idx % 3) as f32;
            let nx = 0.17 + t * 0.66;
            let ny = 0.26 + row * 0.22;
            let rot = if idx.is_multiple_of(2) { -6.0 } else { 5.5 };
            (nx, ny, rot)
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
            // "poster" — Storybook poster layout matching Pterodactyl Song (`All the pterodactyls were / sleeping / very still.`)
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

fn draw_interactive_lyrics_stage_card(
    ui: &mut Ui,
    ctx: &mut ViewCtx<'_>,
    active: Option<&Value>,
    tokens: &[StageWordToken],
    anchors: &mut LyricsTetherAnchors,
) {
    let panel = fill_card(ui, ctx.pal, true, |ui| {
        draw_stage_header_bar(ui, ctx, active);

        let captions_extra = if ctx.state.lyrics.draw_enabled { 56.0 } else { 0.0 };
        let avail = ui.available_size();
        // Keep stage crisp and proportional so music transport + token strip + effects never get crushed!
        let reserved_below = 64.0 + captions_extra;
        let max_stage_h = (avail.y - reserved_below).clamp(132.0, 252.0);
        let stage_h = ((avail.x - 8.0) * 9.0 / 16.0).min(max_stage_h).max(126.0);
        let stage_w = (stage_h * 16.0 / 9.0).min((avail.x - 4.0).max(180.0));

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
            let scene_bg = active.and_then(|a| a["background_path"].as_str()).or(film_bg.as_deref());
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
                // Interactive Stage Mode (default): draw clean scene backdrop (NOT baked text PNG) + live editable words!
                if ctx.state.lyrics.backdrop_mode == "scene" {
                    if !paint_local_image(ui, stage_rect, scene_bg, bg, 4) {
                        ui.painter().rect_filled(stage_rect, CornerRadius::same(4), bg);
                    }
                    // Subtle cinematic scrim so interactive typography & handles pop clearly over bright scenes
                    ui.painter().rect_filled(stage_rect, CornerRadius::same(4), Color32::from_rgba_unmultiplied(7, 11, 18, 92));
                } else {
                    ui.painter().rect_filled(stage_rect, CornerRadius::same(4), bg);
                }
                paint_interactive_stage_words(ui, ctx, stage_rect, tokens, anchors);
            }
        });

        ui.add_space(2.0);
        draw_compact_music_transport_bar(ui, ctx);
        draw_bracketed_word_token_bar(ui, ctx, tokens, anchors);

        if ctx.state.lyrics.draw_enabled {
            draw_captions_and_template_strip(ui, ctx);
        }
    });
    ctx.reg.record("words-stage-panel", "Stage Panel", &panel.response, None);
}

fn paint_interactive_stage_words(ui: &mut Ui, ctx: &mut ViewCtx<'_>, stage_rect: Rect, tokens: &[StageWordToken], anchors: &mut LyricsTetherAnchors) {
    if tokens.is_empty() {
        paint_craft_doc_card(ui, stage_rect.shrink(14.0), ctx.pal, "EC", "lettering/lyrics.ecproj", "Add lyrics or align audio");
        return;
    }

    // Subtle rule-of-thirds stage guide
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
    let base_font_scale = (ctx.state.lyrics.font_size as f32 / 56.0).clamp(0.55, 1.85);
    let mut centers: Vec<Pos2> = Vec::with_capacity(tokens.len());
    let mut boxes: Vec<Rect> = Vec::with_capacity(tokens.len());

    // First compute word centers & bounding rects so we can draw inter-word warp arcs underneath glyphs
    for (idx, tok) in tokens.iter().enumerate() {
        let (bx, by, _) = choreography_base_pos(&choreo, idx, tokens.len(), tok.emphasis);
        let [dx, dy] = ctx.state.lyrics.word_offsets.get(&tok.id).copied().unwrap_or([0.0, 0.0]);
        let w_scale = ctx.state.lyrics.word_scales.get(&tok.id).copied().unwrap_or(1.0);
        let cx = stage_rect.min.x + ((bx + dx).clamp(0.08, 0.92)) * stage_rect.width();
        let cy = stage_rect.min.y + ((by + dy).clamp(0.12, 0.88)) * stage_rect.height();

        let hero_mult = if tok.emphasis || (choreo == "poster" && idx == tokens.len() / 2) { 1.68 } else { 0.96 };
        let px = (19.0 * hero_mult * base_font_scale * w_scale * (stage_rect.height() / 210.0).clamp(0.75, 1.35)).clamp(12.0, 56.0);
        let galley = ui.painter().layout_no_wrap(tok.text.clone(), FontId::proportional(px), Color32::WHITE);
        let pad = Vec2::new(12.0, 6.0);
        let wrect = Rect::from_center_size(Pos2::new(cx, cy), galley.size() + pad);
        centers.push(Pos2::new(cx, cy));
        boxes.push(wrect);
    }

    // Draw curved inter-word kinetic warp paths (`Warp Arcs`) across the stage
    if ctx.state.lyrics.show_warp_paths && centers.len() >= 2 {
        for i in 0..(centers.len() - 1) {
            let p0 = centers[i];
            let p3 = centers[i + 1];
            let mid_x = (p0.x + p3.x) * 0.5;
            let arc_lift = if i % 2 == 0 { -22.0 } else { 18.0 };
            let c1 = Pos2::new(mid_x, p0.y.min(p3.y) + arc_lift);
            let c2 = Pos2::new(mid_x, p0.y.max(p3.y) + arc_lift * 0.5);
            ui.painter().add(CubicBezierShape::from_points_stroke(
                [p0, c1, c2, p3],
                false,
                Color32::TRANSPARENT,
                Stroke::new(1.25, ctx.pal.accent_sky.gamma_multiply(0.42)),
            ));
            let mid_pt = cubic_point(p0, c1, c2, p3, 0.5);
            ui.painter().circle_filled(mid_pt, 2.2, ctx.pal.accent_amber.gamma_multiply(0.7));
        }
    }

    // Now draw & interact with every stage word + transform handles on the selected word!
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

        let font_px = (wrect.height() - 6.0).clamp(12.0, 56.0);
        let fg = if is_sung_now {
            ctx.pal.accent_emerald
        } else if tok.emphasis || idx == tokens.len() / 2 {
            ctx.pal.accent_amber
        } else if ctx.state.lyrics.backdrop_mode == "paper" {
            Color32::from_rgb(0x17, 0x3f, 0x49)
        } else {
            ctx.pal.text_primary
        };

        // Soft glow / shadow behind stage lettering
        if has_glow_or_emphasis(fx_opt.as_deref(), is_sel, is_sung_now) {
            ui.painter().rect_filled(wrect.expand(2.0), CornerRadius::same(6), fg.gamma_multiply(if is_sel { 0.14 } else { 0.08 }));
        }
        ui.painter().text(
            wrect.center() + Vec2::new(1.8, 2.0),
            Align2::CENTER_CENTER,
            &tok.text,
            FontId::proportional(font_px),
            Color32::from_rgba_unmultiplied(6, 10, 16, 225),
        );
        ui.painter().text(wrect.center(), Align2::CENTER_CENTER, &tok.text, FontId::proportional(font_px), fg);

        if let Some(ref fx_id) = fx_opt {
            let short_fx = fx_id.rsplit('.').next().unwrap_or(fx_id);
            let tag_pos = Pos2::new(wrect.max.x, wrect.min.y - 5.0);
            ui.painter().text(tag_pos, Align2::RIGHT_BOTTOM, format!("✨ {short_fx}"), FontId::monospace(8.2), ctx.pal.accent_emerald);
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

    let hud_txt =
        if rot_deg.abs() > 0.4 { format!("♪ {:.2}s · {:.0}°", tok.start_sec, rot_deg) } else { format!("♪ {:.2}s · drag to place", tok.start_sec) };
    ui.painter().text(
        Pos2::new(wrect.center().x, (wrect.max.y + 8.5).min(stage_rect.max.y - 5.0)),
        Align2::CENTER_CENTER,
        hud_txt,
        FontId::monospace(8.2),
        sel_col,
    );

    anchors.stage_word_left = Some(Pos2::new(wrect.min.x, wrect.center().y));
    anchors.stage_word_bottom = Some(Pos2::new(wrect.center().x, wrect.max.y));
    anchors.stage_word_right = Some(Pos2::new(wrect.max.x, wrect.center().y));
}

fn has_glow_or_emphasis(fx_id: Option<&str>, is_sel: bool, is_sung_now: bool) -> bool {
    is_sel || is_sung_now || fx_id.is_some_and(|id| id.contains("glow") || id.contains("sweep") || id.contains("star"))
}

/// Lower center dock: Choreography & Moves + Quick Craft Effects (`words.effects` API) + Interactive Bezier Easing + Compact Beat Strip.
fn draw_choreography_fx_and_beat_dock(
    ui: &mut Ui,
    ctx: &mut ViewCtx<'_>,
    anims: &[Value],
    active: Option<&Value>,
    tokens: &[StageWordToken],
    anchors: &mut LyricsTetherAnchors,
) {
    fill_card(ui, ctx.pal, true, |ui| {
        let full_r = ui.available_rect_before_wrap();
        let beat_h = 74.0_f32.min((full_r.height() * 0.38).max(58.0));
        if let Some(vsplit) = split_rect(full_r, SplitAxis::Vertical, SplitSize::FixedSecond(beat_h), 5.0, 88.0) {
            let mut upper = child_pane(ui, vsplit.first, "lyrics_dock_cards");
            draw_three_column_kinetic_studio(&mut upper, ctx, anchors);
            let mut lower = child_pane(ui, vsplit.second, "lyrics_dock_beats");
            draw_compact_beat_and_words_strip(&mut lower, ctx, anims, active, tokens);
            let _ = ui.allocate_rect(full_r, Sense::hover());
        } else {
            draw_three_column_kinetic_studio(ui, ctx, anchors);
        }
    });
}

fn draw_three_column_kinetic_studio(ui: &mut Ui, ctx: &mut ViewCtx<'_>, anchors: &mut LyricsTetherAnchors) {
    let row_r = ui.available_rect_before_wrap();
    let col1_w = (row_r.width() * 0.31).clamp(150.0, 260.0);
    let col3_w = (row_r.width() * 0.28).clamp(144.0, 236.0);

    if let Some(s1) = split_rect(row_r, SplitAxis::Horizontal, SplitSize::FixedFirst(col1_w), 6.0, 220.0)
        && let Some(s2) = split_rect(s1.second, SplitAxis::Horizontal, SplitSize::FixedSecond(col3_w), 6.0, 140.0)
    {
        let mut c1 = child_pane(ui, s1.first, "lyrics_choreo_col");
        draw_choreography_and_move_card(&mut c1, ctx);

        let mut c2 = child_pane(ui, s2.first, "lyrics_quick_fx_col");
        if let Some(fx_pt) = draw_center_quick_craft_effects(&mut c2, ctx)
            && anchors.effect_anchor.is_none()
        {
            anchors.effect_anchor = Some(fx_pt);
        }

        let mut c3 = child_pane(ui, s2.second, "lyrics_bezier_col");
        draw_interactive_bezier_easing_card(&mut c3, ctx);

        let _ = ui.allocate_rect(row_r, Sense::hover());
    } else {
        draw_choreography_and_move_card(ui, ctx);
    }
}

fn draw_choreography_and_move_card(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ctx.pal.inset_frame().show(ui, |ui| {
        ui.set_min_size(ui.available_size());
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Choreography & Moves").size(10.8).strong().color(ctx.pal.accent_sky));
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::label_only("words-reset-offsets", "Reset Pos", "Reset manual stage word offsets to choreography layout"),
            )
            .clicked()
            {
                ctx.state.lyrics.word_offsets.clear();
                ctx.state.lyrics.word_scales.clear();
                ctx.state.lyrics.word_rotations.clear();
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 3.5;
            ui.spacing_mut().item_spacing.y = 3.0;
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
        });
        ui.add_space(2.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 3.5;
            ui.spacing_mut().item_spacing.y = 3.0;
            for (mid, lbl) in [("pop", "Pop"), ("rise", "Rise"), ("loop", "Wave"), ("fall", "Fall"), ("sleep", "Sleep"), ("settle", "Settle")] {
                let id = format!("words-move-{mid}");
                let sel = ctx.state.lyrics.move_preset == mid;
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, "Per-word kinetic act").selected(sel)).clicked() {
                    ctx.state.lyrics.move_preset = mid.into();
                    ctx.state.dirty_drafts.insert("lyrics".into());
                }
            }
        });
    });
}

fn draw_center_quick_craft_effects(ui: &mut Ui, ctx: &mut ViewCtx<'_>) -> Option<Pos2> {
    let effects = resolved_lyrics_effects(ctx);
    let api_live = ctx.state.cache.words_effects["api_connected"].as_bool() == Some(true);
    let mut selected_pt = None;

    ctx.pal.inset_frame().show(ui, |ui| {
        ui.set_min_size(ui.available_size());
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.label(RichText::new("Craft Effects (`words.effects`)").size(10.8).strong().color(ctx.pal.accent_emerald));
            status_pill(
                ui,
                ctx.pal,
                &if api_live { ":9877 Live".to_owned() } else { format!("{} Ready", effects.len()) },
                if api_live { ctx.pal.accent_emerald } else { ctx.pal.accent_amber },
            );
        });
        ScrollArea::vertical().id_salt("words_center_quick_fx").auto_shrink([false, false]).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.spacing_mut().item_spacing.y = 4.0;
                for fx in effects.iter().take(10) {
                    let fx_id = fx["id"].as_str().unwrap_or_default();
                    let label = fx["label"].as_str().unwrap_or(fx_id);
                    let badge = fx["badge"].as_str().unwrap_or("EC");
                    let sel = ctx.state.lyrics.selected_effect_id == fx_id;
                    let chip_lbl = format!("{badge} · {label}");
                    let btn_id = format!("words-quick-fx-{fx_id}");
                    let resp =
                        action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&btn_id, &chip_lbl, fx["summary"].as_str().unwrap_or(fx_id)).selected(sel));
                    if sel {
                        selected_pt = Some(Pos2::new(resp.rect.center().x, resp.rect.min.y));
                    }
                    if resp.clicked() {
                        let _ = badge_accent_color(ctx.pal, badge);
                        apply_effect_to_selected_word(ctx, fx_id, label);
                    }
                }
            });
        });
    });
    selected_pt
}

/// Interactive draggable Cubic-Bézier Easing Curve card (integrated from Argon kinetic lyric editor).
fn draw_interactive_bezier_easing_card(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ctx.pal.inset_frame().show(ui, |ui| {
        ui.set_min_size(ui.available_size());
        let p1 = ctx.state.lyrics.curve_p1;
        let p2 = ctx.state.lyrics.curve_p2;
        ui.horizontal(|ui| {
            ui.label(RichText::new("Easing Curve").size(10.8).strong().color(ctx.pal.accent_amber));
            ui.label(RichText::new(format!("({:.2},{:.2},{:.2},{:.2})", p1[0], p1[1], p2[0], p2[1])).monospace().size(8.4).color(ctx.pal.text_muted));
        });

        let avail = ui.available_size();
        let graph_sz = Vec2::new((avail.x - 2.0).max(96.0), (avail.y - 4.0).max(52.0));
        let (grect, _) = ui.allocate_exact_size(graph_sz, Sense::hover());
        let inner = grect.shrink2(Vec2::new(8.0, 6.0));
        ui.painter().rect_filled(grect, CornerRadius::same(4), ctx.pal.bg_stage);
        ui.painter().rect_stroke(grect, CornerRadius::same(4), Stroke::new(1.0, ctx.pal.border_subtle), StrokeKind::Inside);

        let to_screen =
            |nx: f32, ny: f32| -> Pos2 { Pos2::new(inner.min.x + nx.clamp(0.0, 1.0) * inner.width(), inner.max.y - ny.clamp(0.0, 1.0) * inner.height()) };
        let start_pt = to_screen(0.0, 0.0);
        let end_pt = to_screen(1.0, 1.0);
        let c1_pt = to_screen(p1[0], p1[1]);
        let c2_pt = to_screen(p2[0], p2[1]);

        ui.painter().line_segment([start_pt, end_pt], Stroke::new(1.0, ctx.pal.border_subtle));
        ui.painter().line_segment([start_pt, c1_pt], Stroke::new(1.1, ctx.pal.accent_sky.gamma_multiply(0.65)));
        ui.painter().line_segment([end_pt, c2_pt], Stroke::new(1.1, ctx.pal.accent_amber.gamma_multiply(0.65)));
        ui.painter().add(CubicBezierShape::from_points_stroke(
            [start_pt, c1_pt, c2_pt, end_pt],
            false,
            Color32::TRANSPARENT,
            Stroke::new(2.0, ctx.pal.accent_emerald),
        ));

        for (idx, (pt, col)) in [(c1_pt, ctx.pal.accent_sky), (c2_pt, ctx.pal.accent_amber)].into_iter().enumerate() {
            ui.painter().circle_filled(pt, 4.8, col);
            ui.painter().circle_stroke(pt, 4.8, Stroke::new(1.2, Color32::WHITE));
            let h_hit = ui.interact(Rect::from_center_size(pt, Vec2::splat(14.0)), ui.scope_id().with(("lyrics_bezier_handle", idx)), Sense::drag());
            if h_hit.dragged()
                && let Some(ptr) = h_hit.interact_pointer_pos()
            {
                let nx = ((ptr.x - inner.min.x) / inner.width().max(1.0)).clamp(0.0, 1.0);
                let ny = ((inner.max.y - ptr.y) / inner.height().max(1.0)).clamp(0.0, 1.0);
                if idx == 0 {
                    ctx.state.lyrics.curve_p1 = [nx, ny];
                    ctx.state.lyrics.ease_high_pct = f64::from(ny * 100.0);
                } else {
                    ctx.state.lyrics.curve_p2 = [nx, ny];
                    ctx.state.lyrics.ease_low_pct = f64::from((1.0 - nx) * 100.0);
                }
            }
            ctx.reg.record(format!("words-bezier-p{}", idx + 1), "Easing Handle", &h_hit, None);
        }
    });
}

fn draw_compact_beat_and_words_strip(ui: &mut Ui, ctx: &mut ViewCtx<'_>, anims: &[Value], active: Option<&Value>, tokens: &[StageWordToken]) {
    let total_dur = ctx.state.audio_duration_sec.max(30.0);
    let has_audio = ctx.state.song_audio_path().is_some();
    let active_words = active.and_then(|a| a["words"].as_array()).cloned().unwrap_or_default();

    let word_spans: Vec<CraftTimelineSpan> = if !active_words.is_empty() {
        active_words
            .iter()
            .take(36)
            .map(|w| {
                let st = w["start"].as_f64().unwrap_or(0.0);
                CraftTimelineSpan { start_sec: st, end_sec: w["end"].as_f64().unwrap_or(st + 0.28), label: w["text"].as_str().unwrap_or("").to_owned() }
            })
            .collect()
    } else {
        tokens.iter().map(|t| CraftTimelineSpan { start_sec: t.start_sec, end_sec: t.end_sec, label: t.text.clone() }).collect()
    };

    let cue_keys: Vec<f64> =
        if !anims.is_empty() { anims.iter().filter_map(|a| a["song_anchor"].as_f64()).collect() } else { tokens.iter().map(|t| t.start_sec).collect() };

    let audio_spans = if has_audio {
        vec![CraftTimelineSpan {
            start_sec: 0.0,
            end_sec: total_dur,
            label: if ctx.state.lyrics.audio_source == "filmcraft" { "♪ FilmCraft Audio Mix".into() } else { "♪ Song Audio".into() },
        }]
    } else {
        Vec::new()
    };

    let dirty_v3 = ctx.state.dirty_tracks.contains("V3") || ctx.state.dirty_drafts.contains("lyrics");
    let lanes = [
        CraftTimelineLane {
            code: "WORDS",
            title: "EC V3 WORDS",
            color: ctx.pal.accent_sky,
            filmbar: false,
            waveform: false,
            dirty: dirty_v3,
            spans: word_spans,
            keyframes: &cue_keys,
            frames: &[],
            empty_hint: "Interactive word beats",
        },
        CraftTimelineLane {
            code: "AUDIO",
            title: "♪ BEAT",
            color: ctx.pal.accent_emerald,
            filmbar: false,
            waveform: has_audio,
            dirty: false,
            spans: audio_spans,
            keyframes: &cue_keys,
            frames: &[],
            empty_hint: "No audio linked",
        },
    ];
    let mut scrub = ctx.state.lyrics.scrub_seconds;
    if draw_craft_timeline(ui, ctx, "words", total_dur, &mut scrub, &lanes) {
        ctx.state.lyrics.scrub_seconds = scrub;
        ctx.push(UiAction::SeekAudio(scrub));
    }
}
