//! `04 Animations` bottom Theatre, Property-Reveal Keys, & Value/Speed Cubic Bezier Graph Editor
//! dock (`Sheet 08` Zone B — `T19` & `T20`), with live `EffectCraft` ↔ `FilmCraft` track lane sync.

use egui::{
    Align, Align2, Color32, CornerRadius, FontId, Layout, Pos2, RichText, Sense, Stroke,
    StrokeKind, Ui, Vec2,
};
use serde_json::{Value, json};

use crate::controls::{ScalarSpec, fill_card, paint_cubic_bezier, ring_slider_h};
use crate::state::{UiAction, ViewCtx};
use crate::timeline::{CraftTimelineLane, CraftTimelineSpan, draw_craft_timeline};
use crate::widgets::{BtnSpec, action_button};

/// Bottom dock (`[Theatre] [Keys / Props] [Graph Editor]`) covering `T19` & `T20`.
pub fn draw_theatre_dock(ui: &mut Ui, ctx: &mut ViewCtx<'_>, cid: &str) {
    let dock = fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            for (tid, lbl, tip) in [
                ("theatre", "Theatre", "Action clips, ruler, channels & keyframe diamonds"),
                ("keys", "Keys / Props", "Property reveal filter A/P/S/R/T/M/E/U/EE/UU & Separate XY (T20)"),
                ("curves", "Graph Editor", "Full-width Value & Speed Cubic Bezier Curve Workspace (T19)"),
            ] {
                let id = format!("anim-dock-{tid}");
                let sel = ctx.state.animation.dock_tab == tid;
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, tip).selected(sel)).clicked() {
                    ctx.state.animation.dock_tab = tid.into();
                }
            }
            let dur = active_timeline_span(ctx);
            ui.label(
                RichText::new(format!("RANGE 0.0s–{dur:.1}s · {}", ctx.state.animation.track_source.to_ascii_uppercase()))
                    .monospace()
                    .size(9.5)
                    .color(ctx.pal.text_muted),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::new("anim-place-v2", "film", "Place on V2", "Place animated clip onto FilmCraft V2")
                        .primary()
                        .enabled(!cid.is_empty()),
                )
                .clicked()
                {
                    ctx.exec(
                        "character.shot",
                        json!({
                            "character": cid,
                            "sprite": ctx.state.animation.selected_action,
                            "from": ctx.state.animation.place_start_sec,
                            "duration": ctx.state.animation.place_duration_sec,
                            "studio": true,
                        }),
                        Some("Placed character clip onto FilmCraft V2"),
                    );
                }
            });
        });

        match ctx.state.animation.dock_tab.as_str() {
            "keys" => draw_property_reveal_dock(ui, ctx, cid),
            "curves" => draw_bezier_graph_editor_dock(ui, ctx, cid),
            _ => draw_theatre_clips_and_timeline(ui, ctx, cid),
        }
    });
    ctx.reg.record("anim-theatre-dock", "Animation Theatre Dock", &dock.response, None);
}

fn active_timeline_span(ctx: &ViewCtx<'_>) -> f64 {
    if ctx.state.animation.track_source == "effectcraft" {
        ctx.state.animation.action_duration.max(2.5)
    } else {
        ctx.state.audio_duration_sec.max(ctx.state.animation.action_duration).max(12.0)
    }
}

fn collect_rig_keyframes(ctx: &ViewCtx<'_>, dur: f64) -> (Vec<f64>, Vec<f64>) {
    let mut pose_keys: Vec<f64> = ctx
        .state
        .cache
        .character_keys
        .get("keyframes")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(|k| k["time"].as_f64()).collect())
        .unwrap_or_default();
    if pose_keys.is_empty() {
        pose_keys = vec![0.0, dur * 0.25, dur * 0.50, dur * 0.75, dur];
    }
    let motion_keys = vec![0.0, dur * 0.20, dur * 0.48, dur * 0.80, dur];
    (motion_keys, pose_keys)
}

fn draw_theatre_clips_and_timeline(ui: &mut Ui, ctx: &mut ViewCtx<'_>, cid: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.label(RichText::new("CLIPS:").monospace().size(9.5).color(ctx.pal.text_muted));
        for clip in ["idle", "fly", "sing", "jump", "wave", "rest"] {
            let id = format!("theatre-clip-{clip}");
            let sel = ctx.state.animation.selected_action == clip;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, clip, "Trigger action clip").selected(sel)).clicked() {
                ctx.state.animation.selected_action = clip.into();
            }
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("theatre-save-action", "Save Action", "Persist custom action clip").enabled(!cid.is_empty())).clicked() {
            ctx.exec(
                "character.action",
                json!({"character": cid, "action": ctx.state.animation.selected_action, "duration": ctx.state.animation.action_duration}),
                Some("Saved character action cycle"),
            );
        }
    });

    let dur = active_timeline_span(ctx);
    let cyc_dur = ctx.state.animation.action_duration.max(2.0).min(dur);
    let (motion_keys, pose_keys) = collect_rig_keyframes(ctx, cyc_dur);
    let film_frames = ctx.state.active_film_frames();
    let has_audio = ctx.state.song_audio_path().is_some();
    let show_fc = ctx.state.animation.track_source != "effectcraft";

    let act_lbl = format!("{} (.ecproj)", ctx.state.animation.selected_action);
    let part_lbl = format!("{} · {:.0}°", ctx.state.animation.selected_part, ctx.state.animation.bone_rotation);

    let mut lanes = Vec::with_capacity(3);
    if show_fc {
        lanes.push(CraftTimelineLane {
            code: "V1",
            title: "FC V1 / AUDIO",
            color: ctx.pal.accent_emerald,
            filmbar: true,
            waveform: has_audio,
            dirty: false,
            spans: Vec::new(),
            keyframes: &[],
            frames: &film_frames,
            empty_hint: "FilmCraft V1 sequence + underlying song audio",
        });
    }
    lanes.push(CraftTimelineLane {
        code: "MOTION",
        title: "EC V2 MOTION",
        color: ctx.pal.accent_sky,
        filmbar: false,
        waveform: !show_fc && has_audio,
        dirty: ctx.state.dirty_tracks.contains("V2"),
        spans: vec![CraftTimelineSpan {
            start_sec: 0.0,
            end_sec: cyc_dur,
            label: act_lbl,
        }],
        keyframes: &motion_keys,
        frames: &[],
        empty_hint: "",
    });
    lanes.push(CraftTimelineLane {
        code: "POSE",
        title: "POSE / RIG",
        color: ctx.pal.accent_amber,
        filmbar: false,
        waveform: false,
        dirty: false,
        spans: vec![CraftTimelineSpan {
            start_sec: 0.0,
            end_sec: cyc_dur,
            label: part_lbl,
        }],
        keyframes: &pose_keys,
        frames: &[],
        empty_hint: "",
    });

    let mut scrub = ctx.state.animation.scrub_seconds;
    if draw_craft_timeline(ui, ctx, "anim-theatre", dur, &mut scrub, &lanes) {
        ctx.state.animation.scrub_seconds = scrub;
        ctx.push(UiAction::SeekAudio(scrub));
    }
}

fn draw_property_reveal_dock(ui: &mut Ui, ctx: &mut ViewCtx<'_>, cid: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.label(RichText::new("Reveal (T20):").monospace().size(9.5).color(ctx.pal.text_muted));
        for prop in ["A", "P", "S", "R", "T", "M", "E", "U", "EE", "UU"] {
            let id = format!("anim-prop-{prop}");
            let sel = ctx.state.animation.prop_filter == prop;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, prop, "Reveal timeline property").selected(sel)).clicked() {
                ctx.state.animation.prop_filter = prop.into();
                let cmd = format!("timeline.reveal.{}", prop.to_ascii_lowercase());
                ctx.exec("effect.exec", json!({"domain": "character", "character": cid, "command": cmd, "params": {}}), None);
            }
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-sep-xyz", "Separate XY", "prop.separateDimensions")).clicked() {
            ctx.exec(
                "effect.exec",
                json!({"domain": "character", "character": cid, "command": "prop.separateDimensions", "params": {"property": "transform.position", "separate": true}}),
                Some("Separated X / Y / Z position dimensions"),
            );
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-prev-key", "◀ Prev", "time.previousKey")).clicked() {
            ctx.state.animation.scrub_seconds = (ctx.state.animation.scrub_seconds - 0.5).max(0.0);
            ctx.exec("effect.exec", json!({"domain": "character", "character": cid, "command": "time.previousKey", "params": {}}), None);
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-add-key", "◆ Key", "prop.addKey").primary()).clicked() {
            ctx.exec("effect.exec", json!({"domain": "character", "character": cid, "command": "prop.addKey", "params": {"time": ctx.state.animation.scrub_seconds}}), Some("Inserted keyframe"));
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-next-key", "Next ▶", "time.nextKey")).clicked() {
            let max_d = active_timeline_span(ctx);
            ctx.state.animation.scrub_seconds = (ctx.state.animation.scrub_seconds + 0.5).min(max_d);
            ctx.exec("effect.exec", json!({"domain": "character", "character": cid, "command": "time.nextKey", "params": {}}), None);
        }
    });

    let dur = active_timeline_span(ctx);
    let (motion_keys, pose_keys) = collect_rig_keyframes(ctx, dur);
    let prop_title = format!("PROP · {}", ctx.state.animation.prop_filter);
    let lanes = [
        CraftTimelineLane {
            code: "PROPX",
            title: &prop_title,
            color: ctx.pal.accent_emerald,
            filmbar: false,
            waveform: false,
            dirty: false,
            spans: Vec::new(),
            keyframes: &motion_keys,
            frames: &[],
            empty_hint: "",
        },
        CraftTimelineLane {
            code: "PROPY",
            title: "ROTATION",
            color: ctx.pal.accent_sky,
            filmbar: false,
            waveform: false,
            dirty: false,
            spans: Vec::new(),
            keyframes: &pose_keys,
            frames: &[],
            empty_hint: "",
        },
    ];
    let mut scrub = ctx.state.animation.scrub_seconds;
    if draw_craft_timeline(ui, ctx, "anim-keys", dur, &mut scrub, &lanes) {
        ctx.state.animation.scrub_seconds = scrub;
        ctx.push(UiAction::SeekAudio(scrub));
    }
}

/// Full-width Value & Speed Cubic Bezier Graph Editor workspace (`Sheet 08` Section `9`).
fn draw_bezier_graph_editor_dock(ui: &mut Ui, ctx: &mut ViewCtx<'_>, cid: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (interp, lbl) in [
            ("linear", "Linear"),
            ("bezier", "Bezier"),
            ("continuousBezier", "Continuous Bezier"),
            ("hold", "Hold"),
            ("roving", "Roving"),
        ] {
            let id = format!("anim-interp-{interp}");
            let sel = ctx.state.animation.key_interp == interp;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, "keys.setInterpolation").selected(sel)).clicked() {
                ctx.state.animation.key_interp = interp.into();
                if interp == "linear" {
                    ctx.state.animation.curve_handle_out = [0.25, 0.25];
                    ctx.state.animation.curve_handle_in = [0.75, 0.75];
                }
                ctx.exec(
                    "effect.exec",
                    json!({"domain": "character", "character": cid, "command": "keys.setInterpolation", "params": {"inType": interp, "outType": interp}}),
                    Some("Set keyframe interpolation"),
                );
            }
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-easy-ease-f9", "Easy Ease F9", "keys.easyEase").primary()).clicked() {
            ctx.state.animation.key_interp = "bezier".into();
            ctx.state.animation.curve_handle_out = [0.36, 0.04];
            ctx.state.animation.curve_handle_in = [0.64, 0.96];
            ctx.exec("effect.exec", json!({"domain": "character", "character": cid, "command": "keys.easyEase", "params": {"mode": "both"}}), Some("Applied Easy Ease (F9)"));
        }

        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-graph-prev", "◀", "Previous keyframe")).clicked() {
            ctx.state.animation.scrub_seconds = (ctx.state.animation.scrub_seconds - 0.5).max(0.0);
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-graph-key", "◆", "Add keyframe at playhead")).clicked() {
            ctx.exec("effect.exec", json!({"domain": "character", "character": cid, "command": "prop.addKey", "params": {"time": ctx.state.animation.scrub_seconds}}), None);
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("anim-graph-next", "▶", "Next keyframe")).clicked() {
            let max_d = active_timeline_span(ctx);
            ctx.state.animation.scrub_seconds = (ctx.state.animation.scrub_seconds + 0.5).min(max_d);
        }

        let inf_w = (ui.available_width() - 8.0).clamp(120.0, 196.0);
        ui.allocate_ui_with_layout(Vec2::new(inf_w, 28.0), Layout::top_down(Align::Min), |ui| {
            if ring_slider_h(
                ui,
                ctx.reg,
                ctx.pal,
                ScalarSpec::new("anim-key-vel", "Influence", 1.0..=100.0, "%", ctx.pal.accent_amber),
                &mut ctx.state.animation.key_velocity_pct,
            )
            .changed()
            {
                let f = (ctx.state.animation.key_velocity_pct / 100.0).clamp(0.05, 0.95) as f32;
                ctx.state.animation.curve_handle_out[0] = f * 0.5;
                ctx.state.animation.curve_handle_in[0] = 1.0 - f * 0.5;
            }
        });
    });

    let graph_w = (ui.available_width() - 4.0).max(220.0);
    let graph_h = (ui.available_height() - 2.0).clamp(96.0, 134.0);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(graph_w, graph_h), Sense::click_and_drag());
    let painter = ui.painter().with_clip_rect(rect);
    painter.rect(
        rect,
        CornerRadius::same(7),
        Color32::from_rgb(0x0a, 0x0f, 0x16),
        Stroke::new(1.0, ctx.pal.border),
        StrokeKind::Inside,
    );
    let pad = egui::Rect::from_min_max(
        Pos2::new(rect.min.x + 38.0, rect.min.y + 18.0),
        Pos2::new(rect.max.x - 16.0, rect.max.y - 16.0),
    );

    let gstroke = Stroke::new(1.0, Color32::from_rgba_unmultiplied(44, 58, 80, 75));
    for (gy, lbl) in [(0.0_f32, "100%"), (0.5, "50%"), (1.0, "0%")] {
        let y = pad.min.y + gy * pad.height();
        painter.line_segment([Pos2::new(pad.min.x, y), Pos2::new(pad.max.x, y)], gstroke);
        painter.text(Pos2::new(pad.min.x - 5.0, y), Align2::RIGHT_CENTER, lbl, FontId::monospace(8.5), ctx.pal.text_muted);
    }
    let dur = ctx.state.animation.action_duration.max(2.0);
    for step in 0..=4 {
        let fx = (step as f32) * 0.25;
        let x = pad.min.x + fx * pad.width();
        painter.line_segment([Pos2::new(x, pad.min.y), Pos2::new(x, pad.max.y)], gstroke);
        painter.text(
            Pos2::new(x, rect.max.y - 7.0),
            Align2::CENTER_CENTER,
            format!("{:.1}s", (fx as f64) * dur),
            FontId::monospace(8.5),
            ctx.pal.text_muted,
        );
    }

    let p0 = Pos2::new(pad.min.x, pad.max.y);
    let p3 = Pos2::new(pad.max.x, pad.min.y);
    let mut h_out = ctx.state.animation.curve_handle_out;
    let mut h_in = ctx.state.animation.curve_handle_in;
    let p1 = Pos2::new(pad.min.x + h_out[0] * pad.width(), pad.max.y - h_out[1] * pad.height());
    let p2 = Pos2::new(pad.min.x + h_in[0] * pad.width(), pad.max.y - h_in[1] * pad.height());

    if resp.dragged()
        && let Some(pos) = resp.interact_pointer_pos()
    {
        let nx = ((pos.x - pad.min.x) / pad.width().max(1.0)).clamp(0.05, 0.95);
        let ny = (1.0 - (pos.y - pad.min.y) / pad.height().max(1.0)).clamp(0.0, 1.0);
        if pos.distance(p1) <= pos.distance(p2) {
            h_out = [nx, ny];
            ctx.state.animation.curve_handle_out = h_out;
        } else {
            h_in = [nx, ny];
            ctx.state.animation.curve_handle_in = h_in;
        }
    }
    if resp.drag_stopped() {
        ctx.exec(
            "effect.exec",
            json!({
                "domain": "character",
                "character": cid,
                "command": "keys.velocity",
                "params": {
                    "outInfluence": (h_out[0] * 100.0).clamp(1.0, 100.0),
                    "inInfluence": ((1.0 - h_in[0]) * 100.0).clamp(1.0, 100.0),
                    "timeBase": "comp",
                }
            }),
            Some("Updated Cubic Bezier keyframe velocity handles"),
        );
    }

    let speed_stroke = Stroke::new(1.5, ctx.pal.accent_sky.gamma_multiply(0.85));
    let segs = 32usize;
    for s in (0..segs).step_by(2) {
        let eval_speed = |t: f32| -> Pos2 {
            let u = 1.0 - t;
            let dy = 3.0 * u * u * h_out[1] + 6.0 * u * t * (h_in[1] - h_out[1]) + 3.0 * t * t * (1.0 - h_in[1]);
            let norm_v = (dy / 2.4).clamp(0.02, 0.98);
            Pos2::new(pad.min.x + t * pad.width(), pad.max.y - norm_v * pad.height())
        };
        let t0 = (s as f32) / (segs as f32);
        let t1 = ((s + 1) as f32) / (segs as f32);
        painter.line_segment([eval_speed(t0), eval_speed(t1)], speed_stroke);
    }

    painter.line_segment([p0, p1], Stroke::new(1.3, ctx.pal.accent_amber));
    painter.line_segment([p3, p2], Stroke::new(1.3, ctx.pal.accent_amber));
    paint_cubic_bezier(&painter, [p0, p1, p2, p3], Stroke::new(2.3, ctx.pal.accent_emerald));
    painter.rect_filled(egui::Rect::from_center_size(p0, Vec2::splat(6.0)), CornerRadius::same(1), ctx.pal.accent_emerald);
    painter.rect_filled(egui::Rect::from_center_size(p3, Vec2::splat(6.0)), CornerRadius::same(1), ctx.pal.accent_emerald);
    painter.circle_filled(p1, 5.0, ctx.pal.accent_amber);
    painter.circle_filled(p2, 5.0, ctx.pal.accent_amber);

    let out_pct = (h_out[0] * 100.0).round();
    let in_pct = ((1.0 - h_in[0]) * 100.0).round();
    painter.text(
        Pos2::new(pad.min.x + 4.0, rect.min.y + 8.5),
        Align2::LEFT_CENTER,
        format!("Value Graph (Emerald) + Speed Graph (Dashed Sky) · Out {out_pct:.0}% / In {in_pct:.0}%"),
        FontId::monospace(9.0),
        ctx.pal.text_secondary,
    );
    ctx.reg.record("anim-bezier-graph", "Graph Editor Canvas", &resp, None);
}
