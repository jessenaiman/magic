//! `Motion` view (`#view-motion`): Song-action vector & shape trim-path authoring (`V4`
//! `Action Motion`) pulling a distinct CLI-mapped catalog of `VectorCraft` (`imageTrace.make`,
//! `brush.apply`, `roughen`, `zigZag`), `EffectCraft` (`trim`, `ec.generate.writeon`,
//! `ec.generate.scribble`, `ec.distort.turbulentdisplace`), and `PhotoCraft` (`motionBlur`)
//! effects (`motion.effects`), paired with `Point X / Point Y` hollow-ring sliders and rotary dials.

use craft_ui::layout::{SplitAxis, SplitSize, split_rect};
use egui::{Align2, CornerRadius, FontId, Pos2, Rect, RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui, Vec2};
use serde_json::{Value, json};

use crate::controls::{FilmbarOrientation, FilmbarStyle, ScalarSpec, paint_filmbar_frame, ring_slider_h, rotary_dial, xy_path_pad};
use crate::state::{PickerTarget, UiAction, ViewCtx};
use crate::widgets::{BtnSpec, action_button, status_pill, text_input};

const FALLBACK_MOTION_FX: &[(&str, &str, &str)] = &[
    ("vc.trace.line_art", "VC", "Ink Line Trace"),
    ("vc.brush.chalk_charcoal", "VC", "Chalk Art Brush"),
    ("vc.effect.roughen", "VC", "Roughen Sketch"),
    ("vc.effect.zig_zag", "VC", "Zig-Zag Wave"),
    ("ec.shape.trim_paths", "EC", "Trim Paths"),
    ("ec.generate.writeon", "EC", "Write-On Reveal"),
    ("ec.distort.turbulentdisplace", "EC", "Turbulent Boil"),
    ("pc.filter.motion_trail", "PC", "Motion Trail"),
];

pub fn show(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let outer = ui.available_rect_before_wrap();
    let root_resp = ui.allocate_rect(outer, Sense::hover());
    ctx.reg.record("view-motion", "Motion", &root_resp, None);

    ui.vertical(|ui| {
        draw_toolbar(ui, ctx);
        ui.add_space(6.0);
        let body = ui.available_rect_before_wrap();
        if let Some(left_split) = split_rect(body, SplitAxis::Horizontal, SplitSize::FixedFirst(235.0), 8.0, 180.0) {
            let mut cues_rail = ui.new_child(egui::UiBuilder::new().max_rect(left_split.first).scope_id(ui.scope_id().with("motion_cues_rail")));
            draw_song_action_cues_rail(&mut cues_rail, ctx);
            if let Some(right_split) = split_rect(left_split.second, SplitAxis::Horizontal, SplitSize::FixedSecond(310.0), 8.0, 220.0) {
                let mut stage = ui.new_child(egui::UiBuilder::new().max_rect(right_split.first).scope_id(ui.scope_id().with("motion_stage_pane")));
                draw_motion_stage(&mut stage, ctx);
                let mut insp = ui.new_child(egui::UiBuilder::new().max_rect(right_split.second).scope_id(ui.scope_id().with("motion_insp_pane")));
                draw_cue_inspector(&mut insp, ctx);
            } else {
                let mut stage = ui.new_child(egui::UiBuilder::new().max_rect(left_split.second).scope_id(ui.scope_id().with("motion_stage_pane")));
                draw_motion_stage(&mut stage, ctx);
            }
        } else {
            draw_motion_stage(ui, ctx);
        }
    });
}

fn draw_toolbar(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ctx.pal.panel_frame().show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            let badge_resp = ui.label(RichText::new("V4 · Action Motion · anicraft-cli headless only").size(11.0).color(ctx.pal.accent_amber));
            ctx.reg.record("motion-anicraft-cli-badge", "anicraft-cli headless badge", &badge_resp, None);

            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("motion-trace-image", "scissors", "Trace", "Trace raster sketch to SVG via VectorCraft"))
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
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("motion-save-cue", "save", "Save", "Save song-action motion cue")).clicked() {
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
                ctx.exec("motion.save", payload, Some("Saved V4 motion cue"));
            }
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("motion-publish-v4", "film", "Publish", "Render actions.ecproj & publish V4 Action Motion").primary(),
            )
            .clicked()
            {
                ctx.exec("motion.publish", json!({"render": true}), Some("Published Action Motion onto V4"));
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("motion-open-vectorcraft", "pen", "Vector", "Open in VectorCraft")).clicked() {
                ctx.exec("motion.open", json!({"app": "vectorcraft"}), Some("Opened VectorCraft"));
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("motion-open-effectcraft", "sparkles", "Effect", "Open actions.ecproj in EffectCraft"))
                .clicked()
            {
                ctx.exec("motion.open", json!({"app": "effectcraft"}), Some("Opened EffectCraft"));
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("motion-reload", "refresh", "Reload motion.read & motion.effects")).clicked() {
                ctx.exec("motion.read", json!({}), None);
                ctx.exec("motion.effects", json!({}), None);
            }
        });
    });
}

fn draw_song_action_cues_rail(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let lines = ctx.state.cache.motion_doc.get("lines").and_then(Value::as_array).cloned().unwrap_or_default();
    let cues = ctx.state.cache.motion_doc.get("cues").and_then(Value::as_array).cloned().unwrap_or_default();
    let panel = ctx.pal.card_frame().show(ui, |ui| {
        ui.label(RichText::new("Song Action Cues · V4").size(12.5).strong());
        ui.label(RichText::new("timing-reference.json").monospace().size(10.0).color(ctx.pal.text_muted));
        ui.separator();
        ScrollArea::vertical().id_salt("motion_cues_list").show(ui, |ui| {
            for cue in &cues {
                let cid = cue["id"].as_str().unwrap_or("cue");
                let word = cue["word"].as_str().unwrap_or(cid);
                let clock = cue["clock"].as_str().unwrap_or("0:00.0");
                let fx = cue["effect_id"].as_str().unwrap_or("ec.shape.trim_paths");
                ctx.pal.inset_frame().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("[{clock}] {word}")).size(11.0).strong());
                        status_pill(ui, ctx.pal, fx.split('.').next_back().unwrap_or("trim"), ctx.pal.accent_emerald);
                    });
                });
                ui.add_space(3.0);
            }
            for line in lines.iter().take(10) {
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
                            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new(&btn_id, "pen", txt, "Anchor action drawing to sung word").selected(sel))
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

fn draw_motion_stage(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ctx.pal.card_frame().show(ui, |ui| {
        let strip_w = ui.available_width().max(200.0);
        let (strip_rect, sresp) = ui.allocate_exact_size(Vec2::new(strip_w, 40.0), Sense::click());
        let strip_inner = paint_filmbar_frame(
            ui,
            strip_rect,
            ctx.pal,
            FilmbarStyle { orientation: FilmbarOrientation::Horizontal, selected: false, hovered: sresp.hovered(), timecode: None },
        );
        ui.painter().text(strip_inner.center(), Align2::CENTER_CENTER, "Scene 1..6 · Vector Trim-Path Stage", FontId::proportional(10.5), ctx.pal.text_muted);

        ui.horizontal_wrapped(|ui| {
            for (pid, word, tip) in [
                ("whale", "Whale", "Storybook whale contour"),
                ("wave", "Wave", "Sinusoidal water wave"),
                ("star", "Star", "Chorus starburst"),
                ("boat", "Boat", "Sailing boat outline"),
                ("moon", "Moon", "Crescent moon arc"),
                ("jump", "Jump", "Parabolic leap arrow"),
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
        let w = avail.x.max(220.0);
        let h = (w * 9.0 / 16.0).min((avail.y - 105.0).max(140.0));
        let (rect, _) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click());
        ui.painter().rect(rect, CornerRadius::same(8), ctx.pal.bg_stage, Stroke::new(1.0, ctx.pal.border), StrokeKind::Inside);

        let p0 = Pos2::new(rect.min.x + rect.width() * 0.22, rect.center().y + 24.0);
        let p1 = Pos2::new(rect.center().x, rect.center().y - 32.0);
        let p2 = Pos2::new(rect.min.x + rect.width() * 0.78, rect.center().y + 10.0);
        ui.painter().line_segment([p0, p1], Stroke::new(3.0, ctx.pal.accent_emerald));
        ui.painter().line_segment([p1, p2], Stroke::new(3.0, ctx.pal.accent_amber));
        for pt in [p0, p1, p2] {
            ui.painter().circle_filled(pt, 4.5, ctx.pal.bg_input);
            ui.painter().circle_stroke(pt, 4.5, Stroke::new(2.0, ctx.pal.accent_emerald));
        }
        ui.painter().text(
            Pos2::new(rect.center().x, rect.max.y - 18.0),
            Align2::CENTER_CENTER,
            format!("{} · {}", ctx.state.motion.cue_name, ctx.state.motion.selected_effect_id),
            FontId::monospace(11.0),
            ctx.pal.text_secondary,
        );

        for (lane, col) in [("CUES", ctx.pal.accent_amber), ("MOTION", ctx.pal.accent_emerald), ("TRIM", ctx.pal.accent_sky)] {
            ui.horizontal(|ui| {
                ui.allocate_ui(Vec2::new(54.0, 14.0), |ui| {
                    ui.label(RichText::new(lane).monospace().size(9.5).color(ctx.pal.text_muted));
                });
                let lw = ui.available_width().max(80.0);
                let (lrect, _) = ui.allocate_exact_size(Vec2::new(lw, 13.0), Sense::hover());
                ui.painter().rect_filled(lrect, CornerRadius::same(3), ctx.pal.bg_input);
                let bar = Rect::from_min_size(Pos2::new(lrect.min.x + 18.0, lrect.min.y + 2.0), Vec2::new((lw * 0.36).max(24.0), 9.0));
                ui.painter().rect_filled(bar, CornerRadius::same(2), col.gamma_multiply(0.55));
            });
        }
    });
}

fn draw_cue_inspector(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let panel = ctx.pal.card_frame().show(ui, |ui| {
        ScrollArea::vertical().id_salt("motion_cue_insp").show(ui, |ui| {
            ui.label(RichText::new("Craft Vector & Motion Effects (CLI Mapped)").size(12.0).strong());
            draw_mapped_motion_effects(ui, ctx);

            ui.separator();
            ui.label(RichText::new("Path & Stroke Timing").size(12.0).strong().color(ctx.pal.accent_amber));
            ui.horizontal(|ui| {
                ui.label(RichText::new("Cue").size(10.5).color(ctx.pal.text_muted));
                text_input(ui, ctx.reg, "motion-drawing-name", "Cue name", &mut ctx.state.motion.cue_name, "Flight Arc", 190.0);
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
                rotary_dial(ui, ctx.reg, ctx.pal, ScalarSpec::new("motion-arc-bend", "Arc Bend", -180.0..=180.0, "°", amb), &mut ctx.state.motion.arc_bend_deg);
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
        FALLBACK_MOTION_FX.iter().map(|(id, badge, label)| ((*id).to_owned(), (*badge).to_owned(), (*label).to_owned())).collect()
    } else {
        dynamic_fx
            .iter()
            .filter_map(|fx| Some((fx["id"].as_str()?.to_owned(), fx["badge"].as_str().unwrap_or("VC").to_owned(), fx["label"].as_str()?.to_owned())))
            .collect()
    };

    ui.horizontal_wrapped(|ui| {
        for (fx_id, badge, label) in items {
            let btn_id = format!("motion-fx-{fx_id}");
            let sel = ctx.state.motion.selected_effect_id == fx_id;
            let short = format!("{badge}·{}", label.split_whitespace().next().unwrap_or("Motion"));
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new(&btn_id, "wand", &short, &format!("{label} ({fx_id})")).selected(sel)).clicked() {
                ctx.state.motion.selected_effect_id = fx_id;
                ctx.state.dirty_drafts.insert("motion".into());
            }
        }
    });
}
