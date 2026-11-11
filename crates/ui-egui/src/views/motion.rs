//! `Motion` view (`#view-motion`): Craft-project-native `V4` `Action Motion` workstation
//! backed by `motion/actions.ecproj` (`EffectCraft`), `motion/svg/*.svg` & `motion/*.vectorcraft`
//! (`VectorCraft`), `motion/.preview/motion-*.png` rendered frames, and `motion.effects`.
//! Eliminates procedural fallback curves in favor of real rendered Craft frames & SVG files.

use craft_ui::layout::{SplitAxis, SplitSize, split_rect};
use egui::{
    Align, Align2, CornerRadius, FontId, Layout, Pos2, Rect, RichText, ScrollArea, Sense, Stroke,
    StrokeKind, Ui, Vec2,
};
use serde_json::{Value, json};

use crate::controls::{
    EffectRowSpec, FilmbarOrientation, FilmbarStyle, ScalarSpec, badge_accent_color, child_pane,
    effect_row_button, fill_card, paint_craft_doc_card, paint_filmbar_frame, paint_local_image,
    paint_svg_path_preview, ring_slider_h, rotary_dial, xy_path_pad,
};
use crate::state::{PickerTarget, UiAction, ViewCtx};
use crate::widgets::{BtnSpec, action_button, status_pill, text_input};

const FALLBACK_MOTION_FX: &[(&str, &str, &str)] = &[
    ("vc.trace.line_art", "VC", "Ink Line Trace"),
    ("vc.trace.silhouette", "VC", "Cutout Silhouette Trace"),
    ("vc.brush.chalk_charcoal", "VC", "Chalk Art Brush"),
    ("vc.stroke.tapered_quill", "VC", "Tapered Quill Stroke"),
    ("vc.effect.roughen", "VC", "Roughen Sketch"),
    ("vc.effect.zig_zag", "VC", "Zig-Zag Wave"),
    ("ec.shape.trim_paths", "EC", "Trim Paths"),
    ("ec.generate.writeon", "EC", "Write-On Reveal"),
    ("ec.generate.scribble", "EC", "Hand-Drawn Scribble Fill"),
    ("ec.distort.turbulentdisplace", "EC", "Turbulent Line Boil"),
    ("pc.filter.motion_trail", "PC", "Directional Motion Trail"),
];

pub fn show(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    draw_toolbar(ui, ctx);
    ui.add_space(6.0);
    let body = ui.available_rect_before_wrap();
    if let Some(left_split) = split_rect(body, SplitAxis::Horizontal, SplitSize::FixedFirst(252.0), 8.0, 180.0) {
        let mut cues_rail = child_pane(ui, left_split.first, "motion_cues_rail");
        draw_song_action_cues_rail(&mut cues_rail, ctx);
        if let Some(right_split) = split_rect(left_split.second, SplitAxis::Horizontal, SplitSize::FixedSecond(316.0), 8.0, 220.0) {
            let mut stage = child_pane(ui, right_split.first, "motion_stage_pane");
            draw_motion_stage(&mut stage, ctx);
            let mut insp = child_pane(ui, right_split.second, "motion_insp_pane");
            draw_cue_inspector(&mut insp, ctx);
        } else {
            let mut stage = child_pane(ui, left_split.second, "motion_stage_pane");
            draw_motion_stage(&mut stage, ctx);
        }
        let _ = ui.allocate_rect(body, Sense::hover());
    } else {
        draw_motion_stage(ui, ctx);
    }
}

fn draw_toolbar(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let has_ecproj = ctx.state.cache.motion_doc["ecproj"].is_string();
    let svg_cnt = ctx.state.cache.motion_doc["svg_files"].as_array().map_or(0, Vec::len);
    ctx.pal.panel_frame().show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            let doc_lbl = if has_ecproj {
                format!("V4 · motion/actions.ecproj · {svg_cnt} .svg")
            } else {
                format!("V4 · Action Motion ({svg_cnt} .svg) · anicraft-cli headless")
            };
            let badge_resp = ui.label(RichText::new(doc_lbl).monospace().size(10.5).color(ctx.pal.accent_amber));
            ctx.reg.record("motion-anicraft-cli-badge", "anicraft-cli headless badge", &badge_resp, None);

            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("motion-trace-image", "scissors", "Trace Sketch (.svg)", "Trace raster sketch to SVG via VectorCraft"),
            )
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
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("motion-save-cue", "save", "Save Cue (.ecproj)", "Write SVG + sync motion/actions.ecproj")).clicked() {
                save_active_motion_cue(ctx);
            }
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("motion-render-frame", "image", "Render Frame", "Render motion/actions.ecproj preview PNG at cue start"),
            )
            .clicked()
            {
                ctx.exec(
                    "motion.preview_frame",
                    json!({"time": ctx.state.motion.start_sec}),
                    Some("Rendered EffectCraft motion preview frame"),
                );
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("motion-reload", "refresh", "Reload motion.read & motion.effects")).clicked() {
                    ctx.exec("motion.read", json!({}), None);
                    ctx.exec("motion.effects", json!({}), None);
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
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::new("motion-open-vectorcraft", "pen", "VectorCraft", "Open motion/svg/*.svg in VectorCraft"),
                )
                .clicked()
                {
                    ctx.exec("motion.open", json!({"app": "vectorcraft"}), Some("Opened VectorCraft"));
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

fn draw_song_action_cues_rail(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let lines = ctx
        .state
        .cache
        .motion_doc
        .get("lines")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let cues = ctx
        .state
        .cache
        .motion_doc
        .get("cues")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let panel = fill_card(ui, ctx.pal, true, |ui| {
        ui.label(RichText::new("Motion Cues (.ecproj & .svg)").size(12.5).strong());
        ui.label(
            RichText::new("motion/actions.ecproj · motion/svg/*.svg")
                .monospace()
                .size(9.5)
                .color(ctx.pal.text_muted),
        );
        ui.separator();

        ScrollArea::vertical()
            .id_salt("motion_cues_list")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if cues.is_empty() {
                    ui.label(
                        RichText::new("No motion cues authored yet — pick a sung word below or trace an SVG sketch.")
                            .size(10.5)
                            .color(ctx.pal.text_muted),
                    );
                }
                for cue in &cues {
                    let cid = cue["id"].as_str().unwrap_or("cue");
                    let word = cue["word"].as_str().or_else(|| cue["title"].as_str()).unwrap_or(cid);
                    let clock = cue["clock"].as_str().unwrap_or("0:00.0");
                    let fx = cue["effect_id"].as_str().unwrap_or("ec.shape.trim_paths");
                    let preview_img = cue["preview_frame"].as_str().or_else(|| cue["svg_file"].as_str());
                    let svg_d = cue["d"].as_str().unwrap_or("");
                    let svg_label = cue["svg_file"]
                        .as_str()
                        .and_then(|s| s.rsplit('/').next())
                        .unwrap_or("actions.ecproj");

                    ctx.pal.inset_frame().show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("[{clock}] {word}")).size(11.0).strong());
                            status_pill(ui, ctx.pal, svg_label, badge_accent_color(ctx.pal, "SVG"));
                        });
                        let (mini_r, mresp) = ui.allocate_exact_size(Vec2::new(ui.available_width().max(140.0), 34.0), Sense::click());
                        if !paint_local_image(ui, mini_r, preview_img, ctx.pal.bg_stage, 4) {
                            paint_svg_path_preview(ui, mini_r, svg_d, Stroke::new(2.0, ctx.pal.accent_emerald), None);
                        }
                        if mresp.clicked() {
                            ctx.state.motion.cue_name = word.to_owned();
                            ctx.state.motion.selected_effect_id = fx.to_owned();
                            if let Some(st) = cue["start"].as_f64() {
                                ctx.state.motion.start_sec = st;
                            }
                        }
                    });
                    ui.add_space(4.0);
                }

                if !lines.is_empty() {
                    ui.add_space(4.0);
                    ui.label(RichText::new("ANCHOR TO SUNG WORD").monospace().size(9.5).color(ctx.pal.text_muted));
                    ui.add_space(2.0);
                }
                for line in lines.iter().take(12) {
                    if let Some(words) = line["words"].as_array() {
                        ui.horizontal_wrapped(|ui| {
                            for w in words {
                                let wid = w["word_id"].as_str().unwrap_or_default().to_owned();
                                let txt = w["text"].as_str().unwrap_or("");
                                if txt.is_empty() {
                                    continue;
                                }
                                let sel = ctx.state.motion.selected_word_id.as_deref() == Some(wid.as_str());
                                let btn_id = format!("motion-word-{wid}");
                                if action_button(
                                    ui,
                                    ctx.reg,
                                    ctx.pal,
                                    BtnSpec::label_only(&btn_id, txt, "Anchor action drawing to sung word").selected(sel),
                                )
                                .clicked()
                                {
                                    ctx.state.motion.selected_word_id = Some(wid);
                                    ctx.state.motion.cue_name = txt.to_owned();
                                    if let Some(st) = w["start"].as_f64() {
                                        ctx.state.motion.start_sec = st;
                                    }
                                }
                            }
                        });
                    }
                }
            });
    });
    ctx.reg.record("motion-cues-rail", "Song Action Cues", &panel.response, None);
}

fn active_cue(ctx: &ViewCtx<'_>) -> Option<Value> {
    let cues = ctx.state.cache.motion_doc["cues"].as_array()?;
    cues.iter()
        .find(|c| {
            c["word"].as_str() == Some(ctx.state.motion.cue_name.as_str())
                || c["title"].as_str() == Some(ctx.state.motion.cue_name.as_str())
        })
        .or_else(|| cues.first())
        .cloned()
}

fn preset_svg_d(ctx: &ViewCtx<'_>, preset_id: &str) -> Option<String> {
    ctx.state.cache.motion_doc["presets"]
        .as_array()?
        .iter()
        .find(|pr| pr["id"].as_str() == Some(preset_id))?
        .get("d")
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn draw_motion_stage(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let backdrop = ctx
        .state
        .cache
        .motion_doc
        .pointer("/backdrop/path")
        .or_else(|| ctx.state.cache.snapshot.pointer("/data/drawer/backgrounds/0/thumb"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    let preview_frames = ctx.state.cache.motion_doc["preview_frames"].as_array().cloned().unwrap_or_default();
    let svg_files = ctx.state.cache.motion_doc["svg_files"].as_array().cloned().unwrap_or_default();
    let cue = active_cue(ctx);

    fill_card(ui, ctx.pal, true, |ui| {
        // Horizontal 35mm filmbar strip showing real motion/.preview/*.png and motion/svg/*.svg files
        let strip_w = ui.available_width().max(200.0);
        let (strip_rect, sresp) = ui.allocate_exact_size(Vec2::new(strip_w, 38.0), Sense::click());
        let strip_inner = paint_filmbar_frame(
            ui,
            strip_rect,
            ctx.pal,
            FilmbarStyle {
                orientation: FilmbarOrientation::Horizontal,
                selected: false,
                hovered: sresp.hovered(),
                timecode: None,
            },
        );
        let mut strip_assets: Vec<String> = preview_frames.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect();
        for sv in &svg_files {
            if let Some(s) = sv.as_str() {
                strip_assets.push(s.to_owned());
            }
        }
        if strip_assets.is_empty() {
            ui.painter().text(
                strip_inner.center(),
                Align2::CENTER_CENTER,
                "No rendered motion/.preview/*.png or motion/svg/*.svg files yet",
                FontId::monospace(9.5),
                ctx.pal.text_muted,
            );
        } else {
            let n = strip_assets.len().clamp(1, 6);
            let cell_w = (strip_inner.width() / (n as f32)).max(36.0);
            for (i, asset_path) in strip_assets.iter().take(n).enumerate() {
                let cell = Rect::from_min_size(
                    Pos2::new(strip_inner.min.x + (i as f32) * cell_w + 2.0, strip_inner.min.y + 1.0),
                    Vec2::new(cell_w - 4.0, strip_inner.height() - 2.0),
                );
                let _ = paint_local_image(ui, cell, Some(asset_path.as_str()), ctx.pal.bg_stage, 2);
            }
        }

        // SVG template seed buttons (writes real motion/svg/<id>.svg + motion/actions.ecproj on Save Cue)
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("SVG Seed:").size(10.0).color(ctx.pal.text_muted));
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

        // Vertical 35mm filmbar stage showing real rendered motion/actions.ecproj frame or real motion/svg/*.svg
        let avail = ui.available_size();
        let w = avail.x.max(220.0);
        let h = (w * 9.0 / 16.0).min((avail.y - 104.0).max(142.0));
        let (outer, oresp) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click());
        let rect = paint_filmbar_frame(
            ui,
            outer,
            ctx.pal,
            FilmbarStyle {
                orientation: FilmbarOrientation::Vertical,
                selected: false,
                hovered: oresp.hovered(),
                timecode: Some("V4 · actions.ecproj"),
            },
        );

        let rendered_motion = ctx
            .state
            .cache
            .last_motion_frame
            .as_deref()
            .or_else(|| cue.as_ref().and_then(|c| c["preview_frame"].as_str()))
            .or_else(|| preview_frames.first().and_then(Value::as_str));
        let cue_svg = cue.as_ref().and_then(|c| c["svg_file"].as_str()).or_else(|| svg_files.first().and_then(Value::as_str));

        if !paint_local_image(ui, rect, rendered_motion, ctx.pal.bg_stage, 4) {
            let _ = paint_local_image(ui, rect, backdrop.as_deref(), ctx.pal.bg_stage, 4);
            if !paint_local_image(ui, rect.shrink(12.0), cue_svg, egui::Color32::TRANSPARENT, 4) {
                if let Some(d_str) = cue
                    .as_ref()
                    .and_then(|c| c["d"].as_str())
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
                    .or_else(|| ctx.state.dirty_drafts.contains("motion").then(|| preset_svg_d(ctx, &ctx.state.motion.selected_preset)).flatten())
                {
                    let stroke_w = (ctx.state.motion.stroke_width as f32 * 0.45).clamp(2.0, 9.0);
                    paint_svg_path_preview(ui, rect, &d_str, Stroke::new(stroke_w, ctx.pal.accent_emerald), Some(ctx.pal.accent_amber));
                } else {
                    paint_craft_doc_card(
                        ui,
                        rect.shrink(18.0),
                        ctx.pal,
                        "VC",
                        "motion/actions.ecproj · motion/svg/*.svg",
                        "Trace a sketch in VectorCraft or click Save Cue (.ecproj) to author V4 motion",
                    );
                }
            }
        }
        ui.painter()
            .rect_stroke(rect, CornerRadius::same(4), Stroke::new(1.0, ctx.pal.border_subtle), StrokeKind::Inside);

        draw_real_motion_timeline(ui, ctx);
    });
}

fn draw_real_motion_timeline(ui: &mut Ui, ctx: &ViewCtx<'_>) {
    let total_dur = ctx
        .state
        .cache
        .motion_doc["duration"]
        .as_f64()
        .unwrap_or(ctx.state.audio_duration_sec)
        .max(15.0);
    let cues = ctx.state.cache.motion_doc["cues"].as_array().cloned().unwrap_or_default();

    for (lane, col) in [
        ("CUES", ctx.pal.accent_amber),
        ("AUDIO", ctx.pal.text_muted),
        ("MOTION", ctx.pal.accent_emerald),
        ("SVG", ctx.pal.accent_violet),
        ("TRIM", ctx.pal.accent_sky),
    ] {
        ui.horizontal(|ui| {
            ui.allocate_ui(Vec2::new(50.0, 13.0), |ui| {
                ui.label(RichText::new(lane).monospace().size(9.0).color(ctx.pal.text_muted));
            });
            let lw = ui.available_width().max(80.0);
            let (lrect, _) = ui.allocate_exact_size(Vec2::new(lw, 12.0), Sense::hover());
            ui.painter().rect_filled(lrect, CornerRadius::same(3), ctx.pal.bg_input);
            if lane == "AUDIO" {
                if ctx.state.song_audio_path().is_some() {
                    ui.painter().rect_filled(lrect.shrink2(Vec2::new(2.0, 3.0)), CornerRadius::same(2), col.gamma_multiply(0.35));
                }
            } else {
                for c in &cues {
                let st = c["start"].as_f64().unwrap_or(0.0);
                let span = if lane == "TRIM" {
                    c["draw_duration"].as_f64().unwrap_or(0.85)
                } else {
                    (c["end"].as_f64().unwrap_or(st + 1.5) - st).max(0.2)
                };
                let x0 = lrect.min.x + ((st / total_dur).clamp(0.0, 1.0) as f32) * lw;
                let bw = ((span / total_dur) as f32 * lw).max(6.0);
                let bar = Rect::from_min_size(Pos2::new(x0, lrect.min.y + 2.0), Vec2::new(bw.min(lrect.max.x - x0), 8.0));
                ui.painter().rect_filled(bar, CornerRadius::same(2), col.gamma_multiply(0.6));
                }
            }
        });
    }
}

fn draw_cue_inspector(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let panel = fill_card(ui, ctx.pal, true, |ui| {
        ScrollArea::vertical()
            .id_salt("motion_cue_insp")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.label(RichText::new("CLI Craft Motion Effects (motion.effects)").size(12.0).strong());
                draw_mapped_motion_effects(ui, ctx);

                ui.separator();
                ui.label(RichText::new("Path & Stroke Timing").size(12.0).strong().color(ctx.pal.accent_amber));
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Cue").size(10.5).color(ctx.pal.text_muted));
                    text_input(ui, ctx.reg, "motion-drawing-name", "Cue name", &mut ctx.state.motion.cue_name, "Flight Arc", 210.0);
                });

                let amb = ctx.pal.accent_amber;
                xy_path_pad(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    ScalarSpec::new("motion-drawing-x", "Point X", 0.0..=1920.0, "px", amb),
                    &mut ctx.state.motion.pos_x,
                    ScalarSpec::new("motion-drawing-y", "Point Y", 0.0..=1080.0, "px", amb),
                    &mut ctx.state.motion.pos_y,
                );

                ring_slider_h(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    ScalarSpec::new("motion-drawing-stroke-width", "Stroke Width", 1.0..=48.0, "px", ctx.pal.accent_emerald),
                    &mut ctx.state.motion.stroke_width,
                );
                ring_slider_h(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    ScalarSpec::new("motion-drawing-start", "Draw Duration", 0.1..=6.0, "s", amb),
                    &mut ctx.state.motion.draw_duration,
                );
                ring_slider_h(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    ScalarSpec::new("motion-drawing-end", "Hold Duration", 0.1..=10.0, "s", ctx.pal.accent_sky),
                    &mut ctx.state.motion.hold_duration,
                );

                ui.horizontal(|ui| {
                    rotary_dial(
                        ui,
                        ctx.reg,
                        ctx.pal,
                        ScalarSpec::new("motion-arc-bend", "Arc Bend", -180.0..=180.0, "°", amb),
                        &mut ctx.state.motion.arc_bend_deg,
                    );
                    rotary_dial(
                        ui,
                        ctx.reg,
                        ctx.pal,
                        ScalarSpec::new("motion-taper", "Taper", 0.0..=100.0, "%", ctx.pal.accent_emerald),
                        &mut ctx.state.motion.taper_pct,
                    );
                });

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Ink").size(10.5).color(ctx.pal.text_muted));
                    text_input(ui, ctx.reg, "motion-drawing-color", "Stroke color", &mut ctx.state.motion.stroke_color, "#35e1c0", 92.0);
                    if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("motion-clear-canvas", "close", "Reset", "Reset cue transform")).clicked() {
                        ctx.state.motion = crate::state::MotionViewState::default();
                    }
                    if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("motion-delete-cue", "trash", "Delete", "Delete selected cue")).clicked() {
                        let name = ctx.state.motion.cue_name.clone();
                        ctx.exec("motion.save", json!({"remove_id": name}), Some("Removed motion cue"));
                    }
                });
            });
    });
    ctx.reg.record("existing-drawing-controls", "Drawing Controls", &panel.response, None);
}

fn draw_mapped_motion_effects(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let dynamic_fx = ctx
        .state
        .cache
        .motion_doc
        .get("craft_effects")
        .or_else(|| ctx.state.cache.motion_effects.get("effects"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let items: Vec<(String, String, String)> = if dynamic_fx.is_empty() {
        FALLBACK_MOTION_FX
            .iter()
            .map(|(id, badge, label)| ((*id).to_owned(), (*badge).to_owned(), (*label).to_owned()))
            .collect()
    } else {
        dynamic_fx
            .iter()
            .filter_map(|fx| {
                Some((
                    fx["id"].as_str()?.to_owned(),
                    fx["badge"].as_str().unwrap_or("VC").to_owned(),
                    fx["label"].as_str()?.to_owned(),
                ))
            })
            .collect()
    };

    for (fx_id, badge, label) in items {
        let btn_id = format!("motion-fx-{fx_id}");
        let sel = ctx.state.motion.selected_effect_id == fx_id;
        if effect_row_button(
            ui,
            ctx.reg,
            ctx.pal,
            EffectRowSpec {
                id: &btn_id,
                badge: &badge,
                label: &label,
                effect_id: &fx_id,
                selected: sel,
            },
        )
        .clicked()
        {
            ctx.state.motion.selected_effect_id = fx_id;
            ctx.state.dirty_drafts.insert("motion".into());
        }
        ui.add_space(2.0);
    }
}
