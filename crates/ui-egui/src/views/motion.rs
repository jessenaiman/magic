//! `Motion` view (`#view-motion`): Interactive `V4` `Action Motion` vector/spline workstation
//! matching `06-motion-v4-vectorcraft-effectcraft.jpg`:
//! - **Interactive 16:9 Vector Stage**: Draggable cue anchor `(pos_x, pos_y)`, interactive Bezier
//!   tangent handles (`tangent_in`, `tangent_out`), 4 corner scale handles (`width`, `height`),
//!   and arc-bend stem handle overlaid directly over the scene backdrop (never flattened into a
//!   dead PNG video player).
//! - **Animated Musical-Note Connections (`♪` / `♫` / `♩`)**: Flowing cubic-Bézier tethers linking
//!   the selected Left Film Reel cue card → Interactive Stage Spline Anchor → Active Craft Effect.
//! - **Compact Music Transport & Quick Craft Effects**: Single sleek 24px scrubber bar + quick
//!   `motion.effects` strip (`VectorCraft :7979` + `EffectCraft :9877`) + compact 118px timeline.

use std::time::Duration;

use craft_ui::layout::{SplitAxis, SplitSize, split_rect};
use egui::epaint::CubicBezierShape;
use egui::{Align2, Color32, CornerRadius, FontId, Id, LayerId, Order, Pos2, Rect, RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui, Vec2};
use serde_json::{Value, json};

use crate::controls::{
    FilmbarOrientation, FilmbarStyle, badge_accent_color, child_pane, fill_card, paint_craft_doc_card, paint_filmbar_frame, paint_local_image,
    paint_svg_path_preview, pill_toggle,
};
use crate::editorial::{ScrubberSpec, ThreePaneSpec, draw_compact_scrubber, draw_three_pane_workspace};
use crate::state::{PickerTarget, UiAction, ViewCtx};
use crate::timeline::{CraftTimelineLane, CraftTimelineSpan, draw_craft_timeline};
use crate::views::lyrics::{MusicalTetherSpec, paint_musical_bezier_tether};
use crate::views::motion_inspector::{
    apply_motion_effect_choice, draw_lottie_toolbar_buttons, draw_lottie_warnings_strip, draw_motion_inspector, resolved_motion_effects,
};
use crate::widgets::{BtnSpec, action_button, status_pill};

#[derive(Default, Clone, Copy)]
struct MotionTetherAnchors {
    reel_socket: Option<Pos2>,
    stage_left: Option<Pos2>,
    stage_right: Option<Pos2>,
    effect_anchor: Option<Pos2>,
}

pub fn show(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    draw_toolbar(ui, ctx);
    draw_lottie_warnings_strip(ui, ctx);
    ui.add_space(4.0);
    let wide = ui.available_width() >= 1180.0;
    let mut anchors = MotionTetherAnchors::default();
    let show_left = ctx.state.show_nav_rail;
    let show_right = ctx.state.show_right_inspector;

    let body = draw_three_pane_workspace(
        ui,
        ThreePaneSpec {
            id_prefix: "motion_ws",
            left_width: if wide { 238.0 } else { 210.0 },
            right_width: if wide { 328.0 } else { 282.0 },
            gap: 8.0,
            min_center: 220.0,
            show_left,
            show_right,
        },
        &mut (&mut *ctx, &mut anchors),
        |cues_rail, (ctx, anchors)| {
            anchors.reel_socket = draw_motion_film_reel(cues_rail, ctx);
        },
        |stage, (ctx, anchors)| {
            draw_motion_stage_and_timeline(stage, ctx, anchors);
        },
        |insp, (ctx, anchors)| {
            if let Some(fx_pt) = draw_motion_inspector(insp, ctx) {
                anchors.effect_anchor = Some(fx_pt);
            }
        },
    );

    if ctx.state.motion.show_tethers && ctx.state.active_modal.is_none() {
        paint_motion_musical_tethers(ui, ctx, body, &anchors);
    }
}

fn paint_motion_musical_tethers(ui: &Ui, ctx: &ViewCtx<'_>, clip_rect: Rect, anchors: &MotionTetherAnchors) {
    let time_sec = ui.ctx().input(|i| i.time);
    let layer = LayerId::new(Order::Foreground, Id::unique("motion_musical_tethers"));
    let painter = ui.ctx().layer_painter(layer).with_clip_rect(clip_rect);

    if let (Some(reel), Some(st_l)) = (anchors.reel_socket, anchors.stage_left)
        && clip_rect.contains(reel)
        && clip_rect.contains(st_l)
    {
        let dx = ((st_l.x - reel.x) * 0.45).clamp(30.0, 140.0);
        paint_musical_bezier_tether(
            &painter,
            MusicalTetherSpec {
                from: reel,
                to: st_l,
                c1: Pos2::new(reel.x + dx, reel.y),
                c2: Pos2::new(st_l.x - dx, st_l.y),
                color_a: ctx.pal.accent_emerald,
                color_b: ctx.pal.accent_amber,
                time_sec,
                phase_offset: 0.12,
            },
        );
    }

    if let (Some(st_r), Some(fx_pt)) = (anchors.stage_right, anchors.effect_anchor)
        && clip_rect.contains(fx_pt)
    {
        let dx = ((fx_pt.x - st_r.x).abs() * 0.42).clamp(28.0, 150.0);
        paint_musical_bezier_tether(
            &painter,
            MusicalTetherSpec {
                from: st_r,
                to: fx_pt,
                c1: Pos2::new(st_r.x + dx, st_r.y),
                c2: Pos2::new(fx_pt.x - dx, fx_pt.y),
                color_a: ctx.pal.accent_amber,
                color_b: ctx.pal.accent_sky,
                time_sec,
                phase_offset: 0.48,
            },
        );
    }

    ui.ctx().request_repaint_after(Duration::from_millis(48));
}

fn draw_toolbar(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let has_ecproj = ctx.state.cache.motion_doc["ecproj"].is_string();
    let svg_cnt = ctx.state.cache.motion_doc["svg_files"].as_array().map_or(0, Vec::len);
    ctx.pal.panel_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            ui.spacing_mut().item_spacing.y = 4.0;
            let doc_lbl = if has_ecproj { format!("V4 · actions.ecproj · {svg_cnt} .svg") } else { format!("V4 · Action Motion ({svg_cnt} .svg)") };
            let badge_resp = ui.label(RichText::new(doc_lbl).monospace().size(10.0).color(ctx.pal.accent_amber));
            ctx.reg.record("motion-anicraft-cli-badge", "anicraft-cli headless badge", &badge_resp, None);

            let _ = pill_toggle(ui, ctx.reg, ctx.pal, "motion-tethers-toggle", "♪ Tethers", &mut ctx.state.motion.show_tethers, ctx.pal.accent_emerald);
            let _ = pill_toggle(ui, ctx.reg, ctx.pal, "motion-flat-preview-toggle", "Flat PNG", &mut ctx.state.motion.flat_preview_only, ctx.pal.accent_amber);

            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("motion-trace-image", "scissors", "Trace Sketch", "Trace raster sketch to SVG via VectorCraft"))
                .clicked()
            {
                ctx.push(UiAction::PickFile {
                    target: PickerTarget::MotionTraceImage,
                    title: "Select Sketch to Trace in VectorCraft".into(),
                    extensions: vec!["png".into(), "jpg".into(), "jpeg".into(), "webp".into()],
                    directory: false,
                    save: false,
                });
            }
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("motion-save-cue", "save", "Save Cue", "Write SVG + sync motion/actions.ecproj + live Craft save"),
            )
            .clicked()
            {
                save_active_motion_cue(ctx);
            }
            draw_lottie_toolbar_buttons(ui, ctx);

            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("motion-open-vectorcraft", "pen", "VectorCraft", "Open motion/svg/*.svg in VectorCraft"))
                .clicked()
            {
                ctx.exec("motion.open", json!({"app": "vectorcraft"}), Some("Opened VectorCraft"));
            }
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("motion-open-effectcraft", "external", "EffectCraft", "Open motion/actions.ecproj in EffectCraft"),
            )
            .clicked()
            {
                ctx.exec("motion.open", json!({"app": "effectcraft"}), Some("Opened motion/actions.ecproj in EffectCraft"));
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("motion-reload", "refresh", "Reload motion.read & motion.effects (--control API)"))
                .clicked()
            {
                ctx.exec("motion.read", json!({}), None);
                ctx.exec("motion.effects", json!({"refresh": true}), None);
            }
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("motion-publish-v4", "film", "Publish V4", "Render motion/actions.ecproj & publish V4 Action Motion").primary(),
            )
            .clicked()
            {
                ctx.exec("motion.publish", json!({"render": true}), Some("Published Action Motion onto V4"));
            }
        });
    });
}

fn save_active_motion_cue(ctx: &mut ViewCtx<'_>) {
    let wid = ctx.state.motion.selected_word_id.clone().unwrap_or_else(|| "t/w0".into());
    let payload = json!({
        "word_id": wid,
        "title": ctx.state.motion.cue_name,
        "preset": ctx.state.motion.selected_preset,
        "effect_id": ctx.state.motion.selected_effect_id,
        "start": ctx.state.motion.start_sec,
        "draw_duration": ctx.state.motion.draw_duration,
        "hold_duration": ctx.state.motion.hold_duration,
        "point_x": ctx.state.motion.pos_x,
        "point_y": ctx.state.motion.pos_y,
        "arc_bend": ctx.state.motion.arc_bend_deg,
        "taper": ctx.state.motion.taper_pct,
        "stroke_width": ctx.state.motion.stroke_width,
        "color": ctx.state.motion.stroke_color,
    });
    ctx.state.dirty_drafts.remove("motion");
    ctx.exec("motion.save", payload, Some("Saved V4 motion cue to motion/actions.ecproj"));
}

/// Dedicated Left 35mm Film Reel (`FilmbarOrientation::Vertical`), returning right-edge socket `Pos2` of selected cue.
fn draw_motion_film_reel(ui: &mut Ui, ctx: &mut ViewCtx<'_>) -> Option<Pos2> {
    let lines = ctx.state.cache.motion_doc.get("lines").and_then(Value::as_array).cloned().unwrap_or_default();
    let cues = ctx.state.cache.motion_doc.get("cues").and_then(Value::as_array).cloned().unwrap_or_default();
    let film_frames = ctx.state.active_film_frames();
    let mut selected_socket = None;

    let panel = fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Film Reel · Cues").size(12.4).strong());
            status_pill(ui, ctx.pal, &format!("{} cue(s)", cues.len()), ctx.pal.accent_emerald);
        });
        ui.label(RichText::new("motion/actions.ecproj & motion/svg/*.svg").monospace().size(9.2).color(ctx.pal.text_muted));
        ui.separator();

        ScrollArea::vertical().id_salt("motion_cues_list").auto_shrink([false, false]).show(ui, |ui| {
            if cues.is_empty() {
                selected_socket = draw_starter_reel_cues(ui, ctx, &film_frames);
            }
            for (idx, cue) in cues.iter().enumerate() {
                let scene_bg = if film_frames.is_empty() { None } else { film_frames.get(idx % film_frames.len()).map(String::as_str) };
                if let Some(sock) = draw_single_cue_reel_frame(ui, ctx, idx, cue, scene_bg) {
                    selected_socket = Some(sock);
                }
                ui.add_space(5.0);
            }

            if !lines.is_empty() {
                draw_word_anchor_drawer(ui, ctx, &lines);
            }
        });
    });
    ctx.reg.record("motion-cues-rail", "Motion Film Reel", &panel.response, None);
    selected_socket
}

fn draw_word_anchor_drawer(ui: &mut Ui, ctx: &mut ViewCtx<'_>, lines: &[Value]) {
    ui.add_space(4.0);
    ui.collapsing(RichText::new("Anchor Cue to Sung Word").size(10.5).color(ctx.pal.text_secondary), |ui| {
        for line in lines.iter().take(12) {
            let Some(words) = line["words"].as_array() else { continue };
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 3.0;
                for w in words {
                    let wid = w["word_id"].as_str().unwrap_or_default().to_owned();
                    let txt = w["text"].as_str().unwrap_or("");
                    if txt.is_empty() {
                        continue;
                    }
                    let sel = ctx.state.motion.selected_word_id.as_deref() == Some(wid.as_str());
                    let btn_id = format!("motion-word-{wid}");
                    if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&btn_id, txt, "Anchor action drawing to sung word").selected(sel)).clicked() {
                        ctx.state.motion.selected_word_id = Some(wid);
                        ctx.state.motion.cue_name = txt.to_owned();
                        if let Some(st) = w["start"].as_f64() {
                            ctx.state.motion.start_sec = st;
                            ctx.push(UiAction::SeekAudio(st));
                        }
                    }
                }
            });
        }
    });
}

fn draw_starter_reel_cues(ui: &mut Ui, ctx: &mut ViewCtx<'_>, film_frames: &[String]) -> Option<Pos2> {
    let total = ctx.state.audio_duration_sec.max(20.0);
    let mut selected_socket = None;
    for (idx, (pid, title, sub)) in [
        ("wave", "Beat", "beat-arc.svg"),
        ("star", "Flourish", "flourish.vectorcraft"),
        ("jump", "Swoop", "swoop-arc.ecproj"),
        ("whale", "Trace", "trace-arc.svg"),
    ]
    .into_iter()
    .enumerate()
    {
        let sel = ctx.state.motion.selected_preset == pid;
        let reel_sec = (idx as f64) * (total / 4.0);
        let card_w = (ui.available_width() - 4.0).max(132.0);
        let (crect, resp) = ui.allocate_exact_size(Vec2::new(card_w, 98.0), Sense::click());
        let inner = paint_filmbar_frame(
            ui,
            crect,
            ctx.pal,
            FilmbarStyle { orientation: FilmbarOrientation::Vertical, selected: sel, hovered: resp.hovered(), timecode: None },
        );
        let bg_frame = if film_frames.is_empty() { None } else { film_frames.get(idx % film_frames.len()).map(String::as_str) };
        let _ = paint_local_image(ui, inner, bg_frame, ctx.pal.bg_stage, 3);
        if let Some(d_str) = preset_svg_d(ctx, pid) {
            paint_svg_path_preview(ui, inner.shrink(10.0), &d_str, Stroke::new(2.2, ctx.pal.accent_emerald), None);
        }
        let band = Rect::from_min_max(Pos2::new(inner.min.x, (inner.max.y - 30.0).max(inner.min.y)), inner.max);
        ui.painter().rect_filled(band, CornerRadius::same(2), Color32::from_rgba_unmultiplied(8, 12, 18, 224));
        ui.painter().text(
            Pos2::new(band.min.x + 6.0, band.min.y + 8.0),
            Align2::LEFT_CENTER,
            format!("♪ 0:{reel_sec:04.1} · {title}"),
            FontId::proportional(10.4),
            ctx.pal.text_primary,
        );
        ui.painter().text(
            Pos2::new(band.min.x + 6.0, band.max.y - 7.5),
            Align2::LEFT_CENTER,
            sub,
            FontId::monospace(8.6),
            if sel { ctx.pal.accent_emerald } else { ctx.pal.accent_amber },
        );
        if sel {
            selected_socket = Some(Pos2::new(crect.max.x - 2.0, crect.center().y));
        }
        if resp.clicked() {
            ctx.state.motion.selected_preset = pid.into();
            ctx.state.motion.cue_name = title.into();
            ctx.state.motion.start_sec = reel_sec;
            ctx.state.dirty_drafts.insert("motion".into());
            ctx.push(UiAction::SeekAudio(reel_sec));
        }
        ctx.reg.record(format!("motion-seed-card-{pid}"), title, &resp, None);
        ui.add_space(5.0);
    }
    selected_socket
}

fn draw_single_cue_reel_frame(ui: &mut Ui, ctx: &mut ViewCtx<'_>, idx: usize, cue: &Value, scene_bg: Option<&str>) -> Option<Pos2> {
    let cid = cue["id"].as_str().unwrap_or("cue");
    let word = cue["word"].as_str().or_else(|| cue["title"].as_str()).unwrap_or(cid);
    let clock = cue["clock"].as_str().unwrap_or("0:00.0");
    let fx = cue["effect_id"].as_str().unwrap_or("ec.shape.trim_paths");
    let preview_img = cue["preview_frame"].as_str().or_else(|| cue["svg_file"].as_str()).or(scene_bg);
    let svg_d = cue["d"].as_str().unwrap_or("");
    let svg_label = cue["svg_file"].as_str().and_then(|s| s.rsplit('/').next()).unwrap_or("actions.ecproj");
    let selected = ctx.state.motion.cue_name == word || (idx == 0 && ctx.state.motion.cue_name == "Flight Arc");

    let card_w = (ui.available_width() - 4.0).max(132.0);
    let (crect, resp) = ui.allocate_exact_size(Vec2::new(card_w, 100.0), Sense::click());
    let inner =
        paint_filmbar_frame(ui, crect, ctx.pal, FilmbarStyle { orientation: FilmbarOrientation::Vertical, selected, hovered: resp.hovered(), timecode: None });
    if !paint_local_image(ui, inner, preview_img, ctx.pal.bg_stage, 3) {
        ui.painter().rect_filled(inner, CornerRadius::same(3), ctx.pal.bg_stage);
    }
    if !svg_d.is_empty() {
        paint_svg_path_preview(ui, inner.shrink(10.0), svg_d, Stroke::new(2.0, ctx.pal.accent_emerald), None);
    }

    let band = Rect::from_min_max(Pos2::new(inner.min.x, (inner.max.y - 32.0).max(inner.min.y)), inner.max);
    ui.painter().rect_filled(band, CornerRadius::same(2), Color32::from_rgba_unmultiplied(8, 12, 18, 224));
    ui.painter().text(
        Pos2::new(band.min.x + 6.0, band.min.y + 8.5),
        Align2::LEFT_CENTER,
        format!("♪ {clock} · {word}"),
        FontId::proportional(10.6),
        ctx.pal.text_primary,
    );
    ui.painter().text(
        Pos2::new(band.min.x + 6.0, band.max.y - 8.0),
        Align2::LEFT_CENTER,
        svg_label,
        FontId::monospace(8.6),
        badge_accent_color(ctx.pal, "SVG"),
    );

    let mut enabled_cue = true;
    let tog_rect = Rect::from_min_size(Pos2::new(band.max.x - 42.0, band.max.y - 18.0), Vec2::new(38.0, 16.0));
    let mut tog_ui = ui.new_child(egui::UiBuilder::new().max_rect(tog_rect).scope_id(ui.scope_id().with(("cue_tog", idx))));
    let _ = pill_toggle(&mut tog_ui, ctx.reg, ctx.pal, &format!("motion-cue-enable-{idx}"), "", &mut enabled_cue, ctx.pal.accent_emerald);

    if resp.clicked() {
        ctx.state.motion.cue_name = word.to_owned();
        ctx.state.motion.selected_effect_id = fx.to_owned();
        if let Some(px) = cue["point_x"].as_f64() {
            ctx.state.motion.pos_x = px;
        }
        if let Some(py) = cue["point_y"].as_f64() {
            ctx.state.motion.pos_y = py;
        }
        if let Some(st) = cue["start"].as_f64() {
            ctx.state.motion.start_sec = st;
            ctx.push(UiAction::SeekAudio(st));
        }
    }
    ctx.reg.record(format!("motion-cue-card-{idx}"), word, &resp, None);
    selected.then_some(Pos2::new(crect.max.x - 2.0, crect.center().y))
}

fn active_cue(ctx: &ViewCtx<'_>) -> Option<Value> {
    let cues = ctx.state.cache.motion_doc["cues"].as_array()?;
    cues.iter()
        .find(|c| c["word"].as_str() == Some(ctx.state.motion.cue_name.as_str()) || c["title"].as_str() == Some(ctx.state.motion.cue_name.as_str()))
        .or_else(|| cues.first())
        .cloned()
}

fn preset_svg_d(ctx: &ViewCtx<'_>, preset_id: &str) -> Option<String> {
    if let Some(found) = ctx.state.cache.motion_doc["presets"]
        .as_array()
        .and_then(|arr| arr.iter().find(|pr| pr["id"].as_str() == Some(preset_id)))
        .and_then(|pr| pr["d"].as_str())
    {
        return Some(found.to_owned());
    }
    magic::cmd::motion::action_sketch_presets().into_iter().find(|pr| pr["id"].as_str() == Some(preset_id)).and_then(|pr| pr["d"].as_str().map(str::to_owned))
}

fn draw_motion_stage_and_timeline(ui: &mut Ui, ctx: &mut ViewCtx<'_>, anchors: &mut MotionTetherAnchors) {
    let center_rect = ui.available_rect_before_wrap();
    let timeline_h = 122.0_f32.min((center_rect.height() * 0.28).max(98.0));
    if ctx.state.show_bottom_dock
        && let Some(vert) = split_rect(center_rect, SplitAxis::Vertical, SplitSize::FixedSecond(timeline_h), 6.0, 190.0)
    {
        let mut stage_ui = child_pane(ui, vert.first, "motion_upper_stage");
        draw_motion_stage(&mut stage_ui, ctx, anchors);
        let mut tl_ui = child_pane(ui, vert.second, "motion_lower_timeline");
        draw_real_motion_timeline(&mut tl_ui, ctx);
        let _ = ui.allocate_rect(center_rect, Sense::hover());
    } else {
        draw_motion_stage(ui, ctx, anchors);
    }
}

/// Compact 24px single-line transport + scrubber + source switcher for Motion.
fn draw_compact_motion_transport_bar(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let has_audio = ctx.state.song_audio_path().is_some();
    let play_icon = if ctx.state.audio_playing { "pause" } else { "play" };
    let dur = ctx.state.audio_duration_sec.max(30.0);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::icon_only("motion-audio-play", play_icon, "Play / pause underlying song audio").primary().enabled(has_audio),
        )
        .clicked()
        {
            ctx.push(UiAction::ToggleAudio);
        }
        ui.label(RichText::new(format!("0:{:04.1}/{:04.1}", ctx.state.motion.start_sec, dur)).monospace().size(9.3).color(if ctx.state.audio_playing {
            ctx.pal.accent_emerald
        } else {
            ctx.pal.text_secondary
        }));

        let scrub_w = (ui.available_width() - 252.0).clamp(80.0, 380.0);
        if let Some(sec) = draw_compact_scrubber(
            ui,
            ctx.reg,
            ctx.pal,
            ScrubberSpec { id: "motion-scrub", label: "Motion Scrubber", width: scrub_w, duration_sec: dur, accent: ctx.pal.accent_emerald },
            &mut ctx.state.motion.start_sec,
        ) {
            ctx.push(UiAction::SeekAudio(sec));
        }

        for (src, lbl, tip) in [
            ("overlay", "FC+V4", "Composite EffectCraft V4 vector motion over FilmCraft V1"),
            ("effectcraft", "EC V4", "Isolate EffectCraft motion/actions.ecproj"),
            ("filmcraft", "FC V1", "Preview FilmCraft V1 sequence"),
        ] {
            let id = format!("motion-track-src-{src}");
            let sel = ctx.state.motion.track_source == src;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, tip).selected(sel)).clicked() {
                ctx.state.motion.track_source = src.into();
            }
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("motion-render-frame", "Render", "Render motion preview frame")).clicked() {
            if ctx.state.motion.track_source == "filmcraft" {
                ctx.exec("video.frame", json!({"seconds": ctx.state.motion.start_sec}), Some("Rendered FilmCraft frame"));
            } else {
                ctx.exec("motion.preview_frame", json!({"time": ctx.state.motion.start_sec}), Some("Rendered EffectCraft motion frame"));
            }
        }
    });
}

fn draw_motion_stage(ui: &mut Ui, ctx: &mut ViewCtx<'_>, anchors: &mut MotionTetherAnchors) {
    let film_bg = ctx.state.film_frame_at_time(ctx.state.motion.start_sec);
    let backdrop = film_bg.or_else(|| {
        ctx.state
            .cache
            .motion_doc
            .pointer("/backdrop/path")
            .or_else(|| ctx.state.cache.snapshot.pointer("/data/drawer/backgrounds/0/thumb"))
            .and_then(Value::as_str)
            .map(str::to_owned)
    });
    let preview_frames = ctx.state.cache.motion_doc["preview_frames"].as_array().cloned().unwrap_or_default();
    let cue = active_cue(ctx);

    fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.label(RichText::new("SVG Seed:").size(10.2).color(ctx.pal.text_muted));
            for (pid, word, tip) in [
                ("whale", "Whale", "Seed Whale SVG path into motion/svg/"),
                ("wave", "Wave", "Seed Wave SVG path into motion/svg/"),
                ("star", "Star", "Seed Star SVG path into motion/svg/"),
                ("boat", "Boat", "Seed Boat SVG path into motion/svg/"),
                ("moon", "Moon", "Seed Moon SVG path into motion/svg/"),
                ("jump", "Jump", "Seed Jump SVG path into motion/svg/"),
            ] {
                let id = format!("motion-preset-{pid}");
                let sel = ctx.state.motion.selected_preset == pid;
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new(&id, "pen", word, tip).selected(sel)).clicked() {
                    ctx.state.motion.selected_preset = pid.into();
                    ctx.state.dirty_drafts.insert("motion".into());
                }
            }
        });

        let avail = ui.available_size();
        // Reserve space below stage for compact transport + quick Craft Motion Effects strip
        let max_stage_h = (avail.y - 66.0).clamp(136.0, 268.0);
        let h = ((avail.x - 8.0) * 9.0 / 16.0).min(max_stage_h).max(128.0);
        let w = (h * 16.0 / 9.0).min((avail.x - 4.0).max(180.0));

        ui.vertical_centered(|ui| {
            let (outer, oresp) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click());
            let tc_badge = match ctx.state.motion.track_source.as_str() {
                "filmcraft" => "V1 · FilmCraft .fcproj",
                "effectcraft" => "V4 · Interactive actions.ecproj",
                _ => "FC V1 + Interactive EC V4 Stage",
            };
            let rect = paint_filmbar_frame(
                ui,
                outer,
                ctx.pal,
                FilmbarStyle { orientation: FilmbarOrientation::Horizontal, selected: false, hovered: oresp.hovered(), timecode: Some(tc_badge) },
            );
            ctx.reg.record("motion-canvas", "Motion Interactive Stage", &oresp, None);

            let rendered_motion = ctx
                .state
                .cache
                .last_motion_frame
                .as_deref()
                .or_else(|| cue.as_ref().and_then(|c| c["preview_frame"].as_str()))
                .or_else(|| preview_frames.first().and_then(Value::as_str));

            if ctx.state.motion.track_source == "filmcraft" {
                if !paint_local_image(ui, rect, backdrop.as_deref(), ctx.pal.bg_stage, 4) {
                    paint_craft_doc_card(
                        ui,
                        rect.shrink(16.0),
                        ctx.pal,
                        "FC",
                        "film/Song.fcproj",
                        "Switch to FC+V4 or EC V4 to drag and edit vector motion paths",
                    );
                }
            } else if ctx.state.motion.flat_preview_only {
                if !paint_local_image(ui, rect, rendered_motion.or(backdrop.as_deref()), ctx.pal.bg_stage, 4) {
                    ui.painter().rect_filled(rect, CornerRadius::same(4), ctx.pal.bg_stage);
                }
            } else {
                // Interactive Vector Stage Mode (default): draw clean scene backdrop + interactive spline & handles!
                if ctx.state.motion.track_source == "overlay" {
                    if !paint_local_image(ui, rect, backdrop.as_deref(), ctx.pal.bg_stage, 4) {
                        ui.painter().rect_filled(rect, CornerRadius::same(4), ctx.pal.bg_stage);
                    }
                    ui.painter().rect_filled(rect, CornerRadius::same(4), Color32::from_rgba_unmultiplied(7, 11, 18, 78));
                } else {
                    ui.painter().rect_filled(rect, CornerRadius::same(4), ctx.pal.bg_stage);
                }
                paint_interactive_motion_gizmo(ui, ctx, rect, cue.as_ref(), anchors);
            }
            ui.painter().rect_stroke(rect, CornerRadius::same(4), Stroke::new(1.0, ctx.pal.border_subtle), StrokeKind::Inside);
        });

        ui.add_space(2.0);
        draw_compact_motion_transport_bar(ui, ctx);
        if let Some(fx_pt) = draw_quick_motion_effects_strip(ui, ctx)
            && anchors.effect_anchor.is_none()
        {
            anchors.effect_anchor = Some(fx_pt);
        }
    });
}

fn paint_interactive_motion_gizmo(ui: &mut Ui, ctx: &mut ViewCtx<'_>, stage_rect: Rect, cue: Option<&Value>, anchors: &mut MotionTetherAnchors) {
    let nx = (ctx.state.motion.pos_x / 1920.0).clamp(0.10, 0.90) as f32;
    let ny = (ctx.state.motion.pos_y / 1080.0).clamp(0.14, 0.86) as f32;
    let nw = (ctx.state.motion.width / 1920.0).clamp(0.14, 0.72) as f32;
    let nh = (ctx.state.motion.height / 1080.0).clamp(0.14, 0.68) as f32;

    let center = Pos2::new(stage_rect.min.x + nx * stage_rect.width(), stage_rect.min.y + ny * stage_rect.height());
    let box_size = Vec2::new(nw * stage_rect.width(), nh * stage_rect.height());
    let cue_box = Rect::from_center_size(center, box_size).intersect(stage_rect.shrink(6.0));

    let d_str = cue.and_then(|c| c["d"].as_str()).filter(|s| !s.is_empty()).map(str::to_owned).or_else(|| preset_svg_d(ctx, &ctx.state.motion.selected_preset));

    let stroke_w = (ctx.state.motion.stroke_width as f32 * 0.42).clamp(2.0, 9.0);
    if let Some(ref d) = d_str {
        paint_svg_path_preview(ui, cue_box, d, Stroke::new(stroke_w, ctx.pal.accent_emerald), Some(ctx.pal.accent_amber));
    }

    // Interactive Bezier motion trajectory spline through `center` with draggable `tangent_in` & `tangent_out`
    let scale_px = stage_rect.width() / 960.0;
    let p_start = Pos2::new(cue_box.min.x + 8.0, cue_box.center().y + 12.0);
    let p_end = Pos2::new(cue_box.max.x - 8.0, cue_box.center().y - 12.0);
    let c1 = center + Vec2::new(ctx.state.motion.tangent_in[0] * scale_px, ctx.state.motion.tangent_in[1] * scale_px);
    let c2 = center + Vec2::new(ctx.state.motion.tangent_out[0] * scale_px, ctx.state.motion.tangent_out[1] * scale_px);

    ui.painter().add(CubicBezierShape::from_points_stroke(
        [p_start, c1, c2, p_end],
        false,
        Color32::TRANSPARENT,
        Stroke::new(1.8, ctx.pal.accent_amber.gamma_multiply(0.82)),
    ));
    ui.painter().line_segment([p_start, c1], Stroke::new(1.0, ctx.pal.accent_sky.gamma_multiply(0.65)));
    ui.painter().line_segment([p_end, c2], Stroke::new(1.0, ctx.pal.accent_sky.gamma_multiply(0.65)));

    // Drag body to move `(pos_x, pos_y)`
    let body_hit = ui.interact(cue_box, ui.scope_id().with("motion_cue_body_drag"), Sense::click_and_drag());
    if body_hit.dragged() {
        let d = body_hit.drag_delta();
        ctx.state.motion.pos_x = (ctx.state.motion.pos_x + f64::from(d.x / stage_rect.width().max(1.0)) * 1920.0).clamp(120.0, 1800.0);
        ctx.state.motion.pos_y = (ctx.state.motion.pos_y + f64::from(d.y / stage_rect.height().max(1.0)) * 1080.0).clamp(100.0, 980.0);
        ctx.state.dirty_drafts.insert("motion".into());
    }
    ctx.reg.record("motion-stage-cue-box", &ctx.state.motion.cue_name, &body_hit, None);

    // Selection bounding box + 4 corner resize handles
    ui.painter().rect_stroke(cue_box, CornerRadius::same(4), Stroke::new(1.4, ctx.pal.accent_sky), StrokeKind::Outside);
    for (ci, cpos) in [cue_box.left_top(), cue_box.right_top(), cue_box.left_bottom(), cue_box.right_bottom()].iter().enumerate() {
        let hrect = Rect::from_center_size(*cpos, Vec2::splat(8.0));
        ui.painter().rect_filled(hrect, CornerRadius::same(1), Color32::WHITE);
        ui.painter().rect_stroke(hrect, CornerRadius::same(1), Stroke::new(1.2, ctx.pal.accent_sky), StrokeKind::Outside);
        let ch = ui.interact(hrect.expand(4.0), ui.scope_id().with(("motion_corner", ci)), Sense::drag());
        if ch.dragged() {
            let sign_x = if ci % 2 == 0 { -1.0 } else { 1.0 };
            let sign_y = if ci < 2 { -1.0 } else { 1.0 };
            let d = ch.drag_delta();
            ctx.state.motion.width = (ctx.state.motion.width + f64::from(d.x * sign_x * 2.0 / stage_rect.width().max(1.0)) * 1920.0).clamp(180.0, 1500.0);
            ctx.state.motion.height = (ctx.state.motion.height + f64::from(d.y * sign_y * 2.0 / stage_rect.height().max(1.0)) * 1080.0).clamp(120.0, 860.0);
            ctx.state.dirty_drafts.insert("motion".into());
        }
    }

    // Interactive Bezier tangent handles (`c1`, `c2`)
    for (ti, tpos) in [(0usize, c1), (1usize, c2)] {
        let clamped = Pos2::new(tpos.x.clamp(stage_rect.min.x + 8.0, stage_rect.max.x - 8.0), tpos.y.clamp(stage_rect.min.y + 8.0, stage_rect.max.y - 8.0));
        ui.painter().circle_filled(clamped, 4.8, ctx.pal.accent_amber);
        ui.painter().circle_stroke(clamped, 4.8, Stroke::new(1.2, Color32::WHITE));
        let th = ui.interact(Rect::from_center_size(clamped, Vec2::splat(13.0)), ui.scope_id().with(("motion_tangent", ti)), Sense::drag());
        if th.dragged() {
            let d = th.drag_delta() / scale_px.max(0.25);
            if ti == 0 {
                ctx.state.motion.tangent_in[0] = (ctx.state.motion.tangent_in[0] + d.x).clamp(-320.0, 320.0);
                ctx.state.motion.tangent_in[1] = (ctx.state.motion.tangent_in[1] + d.y).clamp(-220.0, 220.0);
            } else {
                ctx.state.motion.tangent_out[0] = (ctx.state.motion.tangent_out[0] + d.x).clamp(-320.0, 320.0);
                ctx.state.motion.tangent_out[1] = (ctx.state.motion.tangent_out[1] + d.y).clamp(-220.0, 220.0);
            }
            ctx.state.motion.arc_bend_deg = f64::from((-ctx.state.motion.tangent_in[1] * 0.45).clamp(-90.0, 90.0));
            ctx.state.dirty_drafts.insert("motion".into());
        }
    }

    //Cue label & active effect tag
    let fx_short = ctx.state.motion.selected_effect_id.rsplit('.').next().unwrap_or("trim_paths");
    ui.painter().text(
        Pos2::new(cue_box.center().x, (cue_box.min.y - 7.0).max(stage_rect.min.y + 8.0)),
        Align2::CENTER_BOTTOM,
        format!("♪ {} · ✨ {fx_short} ({:.0}, {:.0})", ctx.state.motion.cue_name, ctx.state.motion.pos_x, ctx.state.motion.pos_y),
        FontId::monospace(8.6),
        ctx.pal.accent_emerald,
    );

    anchors.stage_left = Some(Pos2::new(cue_box.min.x, cue_box.center().y));
    anchors.stage_right = Some(Pos2::new(cue_box.max.x, cue_box.center().y));
}

fn draw_quick_motion_effects_strip(ui: &mut Ui, ctx: &mut ViewCtx<'_>) -> Option<Pos2> {
    let effects = resolved_motion_effects(ctx);
    let mut selected_pt = None;
    ctx.pal.inset_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 3.5;
            ui.spacing_mut().item_spacing.y = 3.0;
            ui.label(RichText::new("FX API:").monospace().size(9.2).color(ctx.pal.accent_emerald));
            for fx in effects.iter().take(7) {
                let fx_id = fx["id"].as_str().unwrap_or_default();
                let label = fx["label"].as_str().unwrap_or(fx_id);
                let badge = fx["badge"].as_str().unwrap_or("EC");
                let sel = ctx.state.motion.selected_effect_id == fx_id;
                let btn_id = format!("motion-quick-fx-{fx_id}");
                let resp = action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::label_only(&btn_id, &format!("{badge} · {label}"), fx["summary"].as_str().unwrap_or(fx_id)).selected(sel),
                );
                if sel {
                    selected_pt = Some(Pos2::new(resp.rect.center().x, resp.rect.min.y));
                }
                if resp.clicked() {
                    apply_motion_effect_choice(ctx, fx_id, label);
                }
            }
        });
    });
    selected_pt
}

fn draw_real_motion_timeline(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let total_dur = ctx.state.cache.motion_doc["duration"].as_f64().unwrap_or(ctx.state.audio_duration_sec).max(15.0);
    let cues = ctx.state.cache.motion_doc["cues"].as_array().cloned().unwrap_or_default();
    let has_audio = ctx.state.song_audio_path().is_some();

    let mut cue_spans: Vec<CraftTimelineSpan> = cues
        .iter()
        .map(|c| {
            let st = c["start"].as_f64().unwrap_or(0.0);
            CraftTimelineSpan {
                start_sec: st,
                end_sec: c["end"].as_f64().unwrap_or(st + 1.8),
                label: c["word"].as_str().or_else(|| c["title"].as_str()).or_else(|| c["preset"].as_str()).unwrap_or("Cue").to_owned(),
            }
        })
        .collect();

    if cue_spans.is_empty() {
        let st = ctx.state.motion.start_sec.clamp(0.0, (total_dur - 1.5).max(0.0));
        let en = (st + ctx.state.motion.draw_duration + ctx.state.motion.hold_duration).min(total_dur);
        cue_spans.push(CraftTimelineSpan { start_sec: st, end_sec: en.max(st + 1.2), label: format!("{} (draft)", ctx.state.motion.cue_name) });
    }

    let trim_spans: Vec<CraftTimelineSpan> = cue_spans
        .iter()
        .map(|sp| CraftTimelineSpan {
            start_sec: sp.start_sec,
            end_sec: (sp.start_sec + ctx.state.motion.draw_duration.max(0.4)).min(sp.end_sec),
            label: "Trim Paths".into(),
        })
        .collect();

    let beat_keys: Vec<f64> = (0..8).map(|i| (i as f64) * (total_dur / 8.0)).collect();
    let motion_keys: Vec<f64> = cue_spans.iter().flat_map(|s| [s.start_sec, s.end_sec]).collect();

    let beat_spans = if has_audio {
        vec![CraftTimelineSpan {
            start_sec: 0.0,
            end_sec: total_dur,
            label: if ctx.state.motion.audio_source == "filmcraft" { "♪ FC Audio Mix".into() } else { "♪ Song Audio & Beats".into() },
        }]
    } else {
        Vec::new()
    };

    fill_card(ui, ctx.pal, true, |ui| {
        let dirty_v4 = ctx.state.dirty_tracks.contains("V4") || ctx.state.dirty_drafts.contains("motion");
        let lanes = [
            CraftTimelineLane {
                code: "BEAT",
                title: "♪ BEAT / AUDIO",
                color: ctx.pal.accent_amber,
                filmbar: false,
                waveform: has_audio,
                dirty: false,
                spans: beat_spans,
                keyframes: &beat_keys,
                frames: &[],
                empty_hint: "Beat markers",
            },
            CraftTimelineLane {
                code: "MOTION",
                title: "EC V4 MOTION",
                color: ctx.pal.accent_emerald,
                filmbar: false,
                waveform: false,
                dirty: dirty_v4,
                spans: cue_spans,
                keyframes: &motion_keys,
                frames: &[],
                empty_hint: "motion/actions.ecproj",
            },
            CraftTimelineLane {
                code: "TRIM",
                title: "⟳ TRIM PATHS",
                color: ctx.pal.accent_sky,
                filmbar: false,
                waveform: false,
                dirty: false,
                spans: trim_spans,
                keyframes: &motion_keys,
                frames: &[],
                empty_hint: "",
            },
        ];
        let mut playhead = ctx.state.motion.start_sec;
        if draw_craft_timeline(ui, ctx, "motion", total_dur, &mut playhead, &lanes) {
            ctx.state.motion.start_sec = playhead;
            ctx.push(UiAction::SeekAudio(playhead));
        }
    });
}
