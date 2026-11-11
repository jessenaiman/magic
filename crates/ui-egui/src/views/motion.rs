//! `Motion` view (`#view-motion`): Craft-project-native `V4` `Action Motion` workstation
//! matching `06-motion-v4-vectorcraft-effectcraft.jpg`, backed by `motion/actions.ecproj`
//! (`EffectCraft`), `motion/svg/*.svg` & `motion/*.vectorcraft` (`VectorCraft`), and
//! `Sheet 08` Zone D (`motion_inspector.rs`: `T04`, `T27`–`T31`).

use craft_ui::layout::{SplitAxis, SplitSize, split_rect};
use egui::{Align, CornerRadius, Layout, RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui, Vec2};
use serde_json::{Value, json};

use crate::controls::{
    FilmbarOrientation, FilmbarStyle, badge_accent_color, child_pane, fill_card,
    paint_craft_doc_card, paint_filmbar_frame, paint_local_image, paint_svg_path_preview,
};
use crate::state::{PickerTarget, UiAction, ViewCtx};
use crate::timeline::{CraftTimelineLane, CraftTimelineSpan, draw_craft_timeline};
use crate::views::motion_inspector::{
    draw_layer_compositing_switches, draw_lottie_toolbar_buttons, draw_lottie_warnings_strip,
    draw_motion_inspector,
};
use crate::widgets::{BtnSpec, action_button, status_pill};

pub fn show(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    draw_toolbar(ui, ctx);
    draw_lottie_warnings_strip(ui, ctx);
    ui.add_space(6.0);
    let body = ui.available_rect_before_wrap();

    let after_left = if ctx.state.show_nav_rail
        && let Some(left_split) = split_rect(body, SplitAxis::Horizontal, SplitSize::FixedFirst(252.0), 8.0, 180.0)
    {
        let mut cues_rail = child_pane(ui, left_split.first, "motion_cues_rail");
        draw_song_action_cues_rail(&mut cues_rail, ctx);
        left_split.second
    } else {
        body
    };

    if ctx.state.show_right_inspector
        && let Some(right_split) = split_rect(after_left, SplitAxis::Horizontal, SplitSize::FixedSecond(316.0), 8.0, 220.0)
    {
        let mut stage = child_pane(ui, right_split.first, "motion_stage_pane");
        draw_motion_stage(&mut stage, ctx);
        let mut insp = child_pane(ui, right_split.second, "motion_insp_pane");
        draw_motion_inspector(&mut insp, ctx);
    } else {
        let mut stage = child_pane(ui, after_left, "motion_stage_pane");
        draw_motion_stage(&mut stage, ctx);
    }
    let _ = ui.allocate_rect(body, Sense::hover());
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

            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("motion-trace-image", "scissors", "Trace Sketch (.svg)", "Trace raster sketch to SVG via VectorCraft")).clicked() {
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
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("motion-render-frame", "image", "Render Frame", "Render motion/actions.ecproj preview PNG at cue start")).clicked() {
                ctx.exec("motion.preview_frame", json!({"time": ctx.state.motion.start_sec}), Some("Rendered EffectCraft motion preview frame"));
            }
            draw_lottie_toolbar_buttons(ui, ctx);

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("motion-reload", "refresh", "Reload motion.read & motion.effects")).clicked() {
                    ctx.exec("motion.read", json!({}), None);
                    ctx.exec("motion.effects", json!({}), None);
                }
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("motion-open-effectcraft", "external", "EffectCraft", "Open motion/actions.ecproj in EffectCraft")).clicked() {
                    ctx.exec("motion.open", json!({"app": "effectcraft"}), Some("Opened motion/actions.ecproj in EffectCraft"));
                }
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("motion-open-vectorcraft", "pen", "VectorCraft", "Open motion/svg/*.svg in VectorCraft")).clicked() {
                    ctx.exec("motion.open", json!({"app": "vectorcraft"}), Some("Opened VectorCraft"));
                }
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("motion-publish-v4", "film", "Publish V4", "Render motion/actions.ecproj & publish V4 Action Motion").primary()).clicked() {
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
    let lines = ctx.state.cache.motion_doc.get("lines").and_then(Value::as_array).cloned().unwrap_or_default();
    let cues = ctx.state.cache.motion_doc.get("cues").and_then(Value::as_array).cloned().unwrap_or_default();

    let panel = fill_card(ui, ctx.pal, true, |ui| {
        ui.label(RichText::new("Motion Cues").size(13.0).strong());
        ui.label(RichText::new("(motion/actions.ecproj & motion/svg/*.svg)").monospace().size(9.5).color(ctx.pal.text_muted));
        draw_layer_compositing_switches(ui, ctx);
        ui.separator();

        ScrollArea::vertical()
            .id_salt("motion_cues_list")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if cues.is_empty() {
                    ui.label(RichText::new("No motion cues authored yet — pick a sung word below or trace an SVG sketch.").size(10.5).color(ctx.pal.text_muted));
                }
                for cue in &cues {
                    let cid = cue["id"].as_str().unwrap_or("cue");
                    let word = cue["word"].as_str().or_else(|| cue["title"].as_str()).unwrap_or(cid);
                    let clock = cue["clock"].as_str().unwrap_or("0:00.0");
                    let fx = cue["effect_id"].as_str().unwrap_or("ec.shape.trim_paths");
                    let preview_img = cue["preview_frame"].as_str().or_else(|| cue["svg_file"].as_str());
                    let svg_d = cue["d"].as_str().unwrap_or("");
                    let svg_label = cue["svg_file"].as_str().and_then(|s| s.rsplit('/').next()).unwrap_or("actions.ecproj");

                    ctx.pal.inset_frame().show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(format!("▸ {word} [{clock}]")).size(11.5).strong());
                                status_pill(ui, ctx.pal, svg_label, badge_accent_color(ctx.pal, "SVG"));
                            });
                            let (mini_r, mresp) = ui.allocate_exact_size(Vec2::new(ui.available_width().max(140.0), 38.0), Sense::click());
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
                                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&btn_id, txt, "Anchor action drawing to sung word").selected(sel)).clicked() {
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
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("35mm Stage").size(13.0).strong().color(ctx.pal.accent_amber));
            ui.label(RichText::new("SVG Seed:").size(10.5).color(ctx.pal.text_muted));
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
        let dock_reserve = if ctx.state.show_bottom_dock { 126.0 } else { 10.0 };
        let w = avail.x.max(220.0);
        let h = (w * 9.0 / 16.0).min((avail.y - dock_reserve).max(150.0));
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
                        "VCRAFT",
                        "motion/actions.ecproj · motion/svg/*.svg",
                        "Trace a sketch in VectorCraft or click Save Cue (.ecproj) to author V4 motion",
                    );
                }
            }
        }
        ui.painter()
            .rect_stroke(rect, CornerRadius::same(4), Stroke::new(1.0, ctx.pal.border_subtle), StrokeKind::Inside);

        if ctx.state.show_bottom_dock {
            draw_real_motion_timeline(ui, ctx);
        }
    });
}

fn draw_real_motion_timeline(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let total_dur = ctx.state.cache.motion_doc["duration"].as_f64().unwrap_or(ctx.state.audio_duration_sec).max(15.0);
    let cues = ctx.state.cache.motion_doc["cues"].as_array().cloned().unwrap_or_default();
    let has_audio = ctx.state.song_audio_path().is_some();

    let cue_spans: Vec<CraftTimelineSpan> = cues
        .iter()
        .map(|c| {
            let st = c["start"].as_f64().unwrap_or(0.0);
            CraftTimelineSpan {
                start_sec: st,
                end_sec: c["end"].as_f64().unwrap_or(st + 1.5),
                label: c["preset"].as_str().unwrap_or("Cue").to_owned(),
            }
        })
        .collect();
    let trim_spans: Vec<CraftTimelineSpan> = cues
        .iter()
        .map(|c| {
            let st = c["start"].as_f64().unwrap_or(0.0);
            let dd = c["draw_duration"].as_f64().unwrap_or(0.85);
            CraftTimelineSpan {
                start_sec: st,
                end_sec: st + dd,
                label: String::new(),
            }
        })
        .collect();
    let audio_spans = if has_audio {
        vec![CraftTimelineSpan {
            start_sec: 0.0,
            end_sec: total_dur,
            label: "Song Audio".into(),
        }]
    } else {
        Vec::new()
    };
    let dirty_v4 = ctx.state.dirty_tracks.contains("V4");
    let lanes = [
        CraftTimelineLane { code: "BEAT", title: "Beat", color: ctx.pal.accent_amber, filmbar: false, dirty: false, spans: audio_spans, frames: &[], empty_hint: "Beat grid" },
        CraftTimelineLane { code: "CUE", title: "Cue", color: ctx.pal.accent_amber, filmbar: false, dirty: false, spans: cue_spans.clone(), frames: &[], empty_hint: "" },
        CraftTimelineLane { code: "MOTION", title: "Motion", color: ctx.pal.accent_emerald, filmbar: false, dirty: dirty_v4, spans: cue_spans, frames: &[], empty_hint: "motion/actions.ecproj" },
        CraftTimelineLane { code: "TRIM", title: "Trim", color: ctx.pal.accent_sky, filmbar: false, dirty: false, spans: trim_spans, frames: &[], empty_hint: "" },
    ];
    let mut playhead = ctx.state.motion.start_sec;
    if draw_craft_timeline(ui, ctx, "motion", total_dur, &mut playhead, &lanes) {
        ctx.state.motion.start_sec = playhead;
    }
}
