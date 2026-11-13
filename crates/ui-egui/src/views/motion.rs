//! `04 Motion` view (`#view-motion`): Interactive `V4` `Action Motion` vector/spline stage hosted
//! inside [`craft_ui::docking`]:
//! - `draw_stage_header`: `actions.ecproj` status + Trace Sketch + Lottie + VectorCraft / EffectCraft + Publish V4.
//! - `draw_navigator_panel`: `motion_dock::draw_motion_film_reel` (`Motion Cues`).
//! - `draw_canvas_panel`: interactive 16:9 Bézier spline gizmo stage + `bezierSpline` SVG presets.
//! - `draw_inspector_panel`: `motion_inspector::draw_motion_inspector` (`Spline & Lottie`).
//! - `draw_timeline_panel`: `motion_dock::draw_real_motion_timeline` (`Motion Timeline`).

use egui::epaint::CubicBezierShape;
use egui::{Align2, Color32, CornerRadius, FontId, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Ui, Vec2};
use serde_json::{Value, json};

use crate::controls::{fill_card, paint_craft_doc_card, paint_local_image, paint_svg_path_preview, pill_toggle};
use crate::editorial::{ScrubberSpec, draw_compact_scrubber};
use crate::state::{PickerTarget, UiAction, ViewCtx};
use crate::state_views::MotionSegmentEdit;
use crate::views::motion_dock::{draw_motion_film_reel, draw_real_motion_timeline};
use crate::views::motion_inspector::{draw_lottie_toolbar_buttons, draw_lottie_warnings_strip, draw_motion_inspector};
use crate::widgets::{BtnSpec, action_button};

pub fn draw_stage_header(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ensure_cues_hydrated_into_segments(ctx);
    draw_toolbar(ui, ctx);
    draw_lottie_warnings_strip(ui, ctx);
}

pub fn draw_navigator_panel(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let _ = draw_motion_film_reel(ui, ctx);
    sync_active_fields_back_to_segment(ctx);
}

pub fn draw_canvas_panel(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ensure_cues_hydrated_into_segments(ctx);
    draw_motion_stage(ui, ctx);
    sync_active_fields_back_to_segment(ctx);
}

pub fn draw_inspector_panel(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ensure_cues_hydrated_into_segments(ctx);
    let _ = draw_motion_inspector(ui, ctx);
    sync_active_fields_back_to_segment(ctx);
}

pub fn draw_timeline_panel(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    draw_real_motion_timeline(ui, ctx);
    sync_active_fields_back_to_segment(ctx);
}

pub(crate) fn resolved_motion_cues(ctx: &ViewCtx<'_>) -> Vec<Value> {
    if let Some(arr) = ctx.state.cache.motion_doc.get("cues").and_then(Value::as_array).filter(|a| !a.is_empty()) {
        return arr.clone();
    }
    let film_frames = ctx.state.active_film_frames();
    if let Some(anims) = ctx.state.cache.words_doc.pointer("/document/animations").and_then(Value::as_array).filter(|a| !a.is_empty()) {
        let presets = ["moon", "swoop_arc", "burst", "wing_glide", "wave", "spiral"];
        return anims
            .iter()
            .enumerate()
            .map(|(idx, anim)| {
                let words_str: String = anim["words"].as_array().into_iter().flatten().filter_map(|w| w["text"].as_str()).take(6).collect::<Vec<_>>().join(" ");
                let st = anim["song_anchor"].as_f64().unwrap_or((idx as f64) * 4.8);
                let pid = presets[idx % presets.len()];
                let d_str = preset_svg_d(ctx, pid).unwrap_or_default();
                let bg =
                    anim["background_path"].as_str().filter(|s| !s.is_empty()).or_else(|| film_frames.get(idx % film_frames.len().max(1)).map(String::as_str));
                json!({
                    "id": format!("anim-cue-{}", idx + 1),
                    "word_id": format!("line-{}/w0", idx + 1),
                    "word": if words_str.is_empty() { format!("Scene {}", idx + 1) } else { words_str.clone() },
                    "title": if words_str.is_empty() { format!("Scene {}", idx + 1) } else { words_str },
                    "preset": pid,
                    "d": d_str,
                    "start": st,
                    "end": st + 2.6,
                    "clock": format!("0:{st:04.1}"),
                    "point_x": 860.0 + ((idx % 3) as f64) * 100.0,
                    "point_y": 480.0 + ((idx % 2) as f64) * 90.0,
                    "width": 520.0 + ((idx % 3) as f64) * 52.0,
                    "height": 236.0 + ((idx % 2) as f64) * 44.0,
                    "arc_bend": if idx.is_multiple_of(2) { 18.0 } else { -20.0 },
                    "effect_id": if idx.is_multiple_of(2) { "ec.shape.trim_paths" } else { "ec.generate.writeon" },
                    "backdrop_path": bg,
                })
            })
            .collect();
    }
    Vec::new()
}

fn segment_from_cue_json(ctx: &ViewCtx<'_>, idx: usize, cue: &Value) -> MotionSegmentEdit {
    let cid = cue["id"].as_str().map(str::to_owned).unwrap_or_else(|| format!("cue-{}", idx + 1));
    let wid = cue["word_id"].as_str().map(str::to_owned).unwrap_or_else(|| format!("cue/w{idx}"));
    let title = cue["word"].as_str().or_else(|| cue["title"].as_str()).unwrap_or(&cid).to_owned();
    let preset = cue["preset"].as_str().unwrap_or("wing_glide").to_owned();
    let d = cue["d"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .or_else(|| preset_svg_d(ctx, &preset))
        .unwrap_or_else(|| "M 380 620 C 560 390 790 360 960 510 C 1130 360 1360 390 1540 620".into());
    let tin = cue["tangent_in"].as_array().filter(|a| a.len() == 2).map_or([-130.0 + (idx as f32 * 14.0), -58.0 + ((idx % 3) as f32 * 24.0)], |a| {
        [a[0].as_f64().unwrap_or(-120.0) as f32, a[1].as_f64().unwrap_or(-54.0) as f32]
    });
    let tout = cue["tangent_out"].as_array().filter(|a| a.len() == 2).map_or([130.0 - (idx as f32 * 10.0), -48.0 + ((idx % 2) as f32 * 36.0)], |a| {
        [a[0].as_f64().unwrap_or(120.0) as f32, a[1].as_f64().unwrap_or(-54.0) as f32]
    });
    let st = cue["start"].as_f64().unwrap_or((idx as f64) * 4.5);
    let en = cue["end"].as_f64().unwrap_or(st + 2.2);
    MotionSegmentEdit {
        cue_id: cid,
        word_id: wid,
        title,
        preset,
        d,
        pos_x: cue["point_x"].as_f64().unwrap_or(960.0),
        pos_y: cue["point_y"].as_f64().unwrap_or(540.0),
        width: cue["width"].as_f64().unwrap_or(520.0),
        height: cue["height"].as_f64().unwrap_or(240.0),
        arc_bend_deg: cue["arc_bend"].as_f64().unwrap_or(16.0),
        tangent_in: tin,
        tangent_out: tout,
        start_sec: st,
        end_sec: en,
        effect_id: cue["effect_id"].as_str().unwrap_or("ec.shape.trim_paths").to_owned(),
        backdrop_path: cue["backdrop_path"].as_str().or_else(|| cue["preview_frame"].as_str()).map(str::to_owned),
    }
}

pub(crate) fn ensure_cues_hydrated_into_segments(ctx: &mut ViewCtx<'_>) {
    let cues = resolved_motion_cues(ctx);
    for (idx, cue) in cues.iter().enumerate() {
        let seg = segment_from_cue_json(ctx, idx, cue);
        ctx.state.motion.segments.entry(seg.cue_id.clone()).or_insert(seg);
    }
    if ctx.state.motion.selected_cue_id.is_none()
        && let Some(first_id) = cues.first().and_then(|c| c["id"].as_str()).map(str::to_owned)
    {
        activate_segment_by_id(ctx, &first_id);
    }
}

pub(crate) fn activate_segment_by_id(ctx: &mut ViewCtx<'_>, cue_id: &str) {
    sync_active_fields_back_to_segment(ctx);
    if let Some(seg) = ctx.state.motion.segments.get(cue_id).cloned() {
        ctx.state.motion.selected_cue_id = Some(seg.cue_id);
        ctx.state.motion.selected_word_id = Some(seg.word_id);
        ctx.state.motion.cue_name = seg.title;
        ctx.state.motion.selected_preset = seg.preset;
        ctx.state.motion.active_d = seg.d;
        ctx.state.motion.pos_x = seg.pos_x;
        ctx.state.motion.pos_y = seg.pos_y;
        ctx.state.motion.width = seg.width;
        ctx.state.motion.height = seg.height;
        ctx.state.motion.arc_bend_deg = seg.arc_bend_deg;
        ctx.state.motion.tangent_in = seg.tangent_in;
        ctx.state.motion.tangent_out = seg.tangent_out;
        ctx.state.motion.start_sec = seg.start_sec;
        ctx.state.motion.end_sec = seg.end_sec;
        ctx.state.motion.selected_effect_id = seg.effect_id;
        ctx.state.motion.active_backdrop = seg.backdrop_path;
    }
}

pub(crate) fn sync_active_fields_back_to_segment(ctx: &mut ViewCtx<'_>) {
    let Some(cid) = ctx.state.motion.selected_cue_id.clone() else {
        return;
    };
    if let Some(seg) = ctx.state.motion.segments.get_mut(&cid) {
        seg.title = ctx.state.motion.cue_name.clone();
        seg.preset = ctx.state.motion.selected_preset.clone();
        if !ctx.state.motion.active_d.is_empty() {
            seg.d = ctx.state.motion.active_d.clone();
        }
        seg.pos_x = ctx.state.motion.pos_x;
        seg.pos_y = ctx.state.motion.pos_y;
        seg.width = ctx.state.motion.width;
        seg.height = ctx.state.motion.height;
        seg.arc_bend_deg = ctx.state.motion.arc_bend_deg;
        seg.tangent_in = ctx.state.motion.tangent_in;
        seg.tangent_out = ctx.state.motion.tangent_out;
        seg.start_sec = ctx.state.motion.start_sec;
        seg.end_sec = ctx.state.motion.end_sec;
        seg.effect_id = ctx.state.motion.selected_effect_id.clone();
    }
}

fn rebuild_dynamic_segment_svg(ctx: &mut ViewCtx<'_>) {
    let cx = ctx.state.motion.pos_x.clamp(200.0, 1720.0);
    let cy = ctx.state.motion.pos_y.clamp(160.0, 920.0);
    let hw = (ctx.state.motion.width * 0.5).clamp(90.0, 720.0);
    let c1x = cx + f64::from(ctx.state.motion.tangent_in[0]) * 1.4;
    let c1y = cy + f64::from(ctx.state.motion.tangent_in[1]) * 1.6;
    let c2x = cx + f64::from(ctx.state.motion.tangent_out[0]) * 1.4;
    let c2y = cy + f64::from(ctx.state.motion.tangent_out[1]) * 1.6;
    let x0 = (cx - hw).clamp(80.0, 1840.0);
    let x3 = (cx + hw).clamp(80.0, 1840.0);
    ctx.state.motion.active_d = format!(
        "M {x0:.0} {cy:.0} C {c1x:.0} {c1y:.0} {c2x:.0} {c2y:.0} {x3:.0} {cy:.0} M {:.0} {:.0} L {x3:.0} {cy:.0} L {:.0} {:.0}",
        x3 - 34.0,
        cy - 24.0,
        x3 - 34.0,
        cy + 24.0
    );
}

fn draw_toolbar(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let has_ecproj = ctx.state.cache.motion_doc["ecproj"].is_string();
    let svg_cnt = ctx.state.cache.motion_doc["svg_files"].as_array().map_or(0, Vec::len);
    let layer_cnt = ctx.state.cache.motion_doc["ecproj_layers"].as_array().map_or(0, Vec::len);
    ctx.pal.panel_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            ui.spacing_mut().item_spacing.y = 4.0;
            let doc_lbl = if has_ecproj || layer_cnt > 0 {
                format!("V4 · actions.ecproj · {layer_cnt} layers · {svg_cnt} .svg")
            } else {
                format!("V4 · Action Motion ({svg_cnt} .svg)")
            };
            let badge_resp = ui.label(RichText::new(doc_lbl).monospace().size(10.0).color(ctx.pal.accent_amber));
            ctx.reg.record("motion-ecproj-layer-badge", "V4 Motion layer status", &badge_resp, None);

            let _ = pill_toggle(ui, ctx.reg, ctx.pal, "motion-flat-preview-toggle", "Flat PNG", &mut ctx.state.motion.flat_preview_only, ctx.pal.accent_amber);

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
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("motion-save-cue", "save", "Write SVG + sync motion/actions.ecproj")).clicked() {
                save_active_motion_cue(ctx);
            }
            draw_lottie_toolbar_buttons(ui, ctx);

            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("motion-open-vectorcraft", "bezierSpline", "Open motion/svg/*.svg in VectorCraft"))
                .clicked()
            {
                ctx.exec("motion.open", json!({"app": "vectorcraft"}), Some("Opened VectorCraft"));
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("motion-open-effectcraft", "external", "Open motion/actions.ecproj in EffectCraft"))
                .clicked()
            {
                ctx.exec("motion.open", json!({"app": "effectcraft"}), Some("Opened motion/actions.ecproj in EffectCraft"));
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("motion-reload", "refresh", "Reload motion.read & live Craft effects")).clicked() {
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
    sync_active_fields_back_to_segment(ctx);
    let cid = ctx.state.motion.selected_cue_id.clone().unwrap_or_else(|| "cue-1".into());
    let wid = ctx.state.motion.selected_word_id.clone().unwrap_or_else(|| "cue/w0".into());
    let payload = json!({
        "id": cid,
        "word_id": wid,
        "title": ctx.state.motion.cue_name,
        "preset": ctx.state.motion.selected_preset,
        "d": ctx.state.motion.active_d,
        "effect_id": ctx.state.motion.selected_effect_id,
        "start": ctx.state.motion.start_sec,
        "draw_duration": ctx.state.motion.draw_duration,
        "hold_duration": ctx.state.motion.hold_duration,
        "point_x": ctx.state.motion.pos_x,
        "point_y": ctx.state.motion.pos_y,
        "width": ctx.state.motion.width,
        "height": ctx.state.motion.height,
        "arc_bend": ctx.state.motion.arc_bend_deg,
        "taper": ctx.state.motion.taper_pct,
        "tangent_in": ctx.state.motion.tangent_in,
        "tangent_out": ctx.state.motion.tangent_out,
        "stroke_width": ctx.state.motion.stroke_width,
        "color": ctx.state.motion.stroke_color,
    });
    ctx.state.dirty_drafts.remove("motion");
    ctx.exec("motion.save", payload, Some("Saved V4 motion segment to motion/actions.ecproj"));
}

pub(crate) fn preset_svg_d(ctx: &ViewCtx<'_>, preset_id: &str) -> Option<String> {
    if let Some(found) = ctx.state.cache.motion_doc["presets"]
        .as_array()
        .and_then(|arr| arr.iter().find(|pr| pr["id"].as_str() == Some(preset_id)))
        .and_then(|pr| pr["d"].as_str())
    {
        return Some(found.to_owned());
    }
    magic_engine::cmd::motion::action_sketch_presets()
        .into_iter()
        .find(|pr| pr["id"].as_str() == Some(preset_id))
        .and_then(|pr| pr["d"].as_str().map(str::to_owned))
}

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

        let scrub_w = (ui.available_width() - 224.0).clamp(80.0, 420.0);
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
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("motion-render-frame", "camera", "Render motion preview frame")).clicked() {
            if ctx.state.motion.track_source == "filmcraft" {
                ctx.exec("video.frame", json!({"seconds": ctx.state.motion.start_sec}), Some("Rendered FilmCraft frame"));
            } else {
                ctx.exec("motion.preview_frame", json!({"time": ctx.state.motion.start_sec}), Some("Rendered EffectCraft motion frame"));
            }
        }
    });
}

#[allow(clippy::cognitive_complexity)]
fn draw_motion_stage(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let film_bg = ctx.state.motion.active_backdrop.clone().or_else(|| ctx.state.film_frame_at_time(ctx.state.motion.start_sec));
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

    fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.label(RichText::new("Spline:").size(10.2).color(ctx.pal.text_muted));
            for (pid, word, tip) in [
                ("wing_glide", "Glide", "Apply Wing Glide spline to active segment"),
                ("swoop_arc", "Swoop", "Apply Dive Swoop spline to active segment"),
                ("burst", "Burst", "Apply Wake Cry Burst to active segment"),
                ("spiral", "Spiral", "Apply All-Around Spiral to active segment"),
                ("wave", "Wave", "Apply Wave spline to active segment"),
                ("star", "Star", "Apply Star spline to active segment"),
                ("moon", "Moon", "Apply Sleeping Moon spline to active segment"),
                ("jump", "Jump", "Apply Jump Arc spline to active segment"),
            ] {
                let id = format!("motion-preset-{pid}");
                let sel = ctx.state.motion.selected_preset == pid;
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new(&id, "bezierSpline", word, tip).selected(sel)).clicked() {
                    ctx.state.motion.selected_preset = pid.into();
                    if let Some(d_str) = preset_svg_d(ctx, pid) {
                        ctx.state.motion.active_d = d_str;
                    }
                    sync_active_fields_back_to_segment(ctx);
                    ctx.state.dirty_drafts.insert("motion".into());
                }
            }
        });

        let avail = ui.available_size();
        let max_stage_h = (avail.y - 34.0).max(156.0);
        let h = ((avail.x - 6.0) * 9.0 / 16.0).min(max_stage_h).max(148.0);
        let w = (h * 16.0 / 9.0).min((avail.x - 2.0).max(200.0));

        ui.vertical_centered(|ui| {
            let (outer, oresp) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click());
            let rect = outer.shrink(2.0);
            ui.painter().rect_filled(outer, CornerRadius::same(8), Color32::from_rgb(7, 10, 15));
            ctx.reg.record("motion-canvas", "Motion Interactive Stage", &oresp, None);

            let rendered_motion = ctx.state.cache.last_motion_frame.as_deref().or_else(|| preview_frames.first().and_then(Value::as_str));

            if ctx.state.motion.track_source == "filmcraft" {
                if !paint_local_image(ui, rect, backdrop.as_deref(), ctx.pal.bg_stage, 6) {
                    paint_craft_doc_card(
                        ui,
                        rect.shrink(16.0),
                        ctx.pal,
                        "FC",
                        "film/Song.fcproj",
                        "Switch to FC+V4 or EC V4 to drag and edit vector motion segments",
                    );
                }
            } else if ctx.state.motion.flat_preview_only {
                if !paint_local_image(ui, rect, rendered_motion.or(backdrop.as_deref()), ctx.pal.bg_stage, 6) {
                    ui.painter().rect_filled(rect, CornerRadius::same(6), ctx.pal.bg_stage);
                }
            } else {
                if ctx.state.motion.track_source == "overlay" {
                    if !paint_local_image(ui, rect, backdrop.as_deref(), ctx.pal.bg_stage, 6) {
                        ui.painter().rect_filled(rect, CornerRadius::same(6), ctx.pal.bg_stage);
                    }
                    ui.painter().rect_filled(rect, CornerRadius::same(6), Color32::from_rgba_unmultiplied(7, 11, 18, 72));
                } else {
                    ui.painter().rect_filled(rect, CornerRadius::same(6), ctx.pal.bg_stage);
                }
                paint_interactive_motion_gizmo(ui, ctx, rect);
            }
            ui.painter().rect_stroke(outer, CornerRadius::same(8), Stroke::new(1.0, ctx.pal.border), StrokeKind::Inside);
        });

        ui.add_space(2.0);
        draw_compact_motion_transport_bar(ui, ctx);
    });
}

fn paint_interactive_motion_gizmo(ui: &mut Ui, ctx: &mut ViewCtx<'_>, stage_rect: Rect) {
    let nx = (ctx.state.motion.pos_x / 1920.0).clamp(0.10, 0.90) as f32;
    let ny = (ctx.state.motion.pos_y / 1080.0).clamp(0.14, 0.86) as f32;
    let nw = (ctx.state.motion.width / 1920.0).clamp(0.14, 0.72) as f32;
    let nh = (ctx.state.motion.height / 1080.0).clamp(0.14, 0.68) as f32;

    let center = Pos2::new(stage_rect.min.x + nx * stage_rect.width(), stage_rect.min.y + ny * stage_rect.height());
    let box_size = Vec2::new(nw * stage_rect.width(), nh * stage_rect.height());
    let cue_box = Rect::from_center_size(center, box_size).intersect(stage_rect.shrink(6.0));

    let d_str =
        if !ctx.state.motion.active_d.is_empty() { Some(ctx.state.motion.active_d.clone()) } else { preset_svg_d(ctx, &ctx.state.motion.selected_preset) };

    let stroke_w = (ctx.state.motion.stroke_width as f32 * 0.44).clamp(2.0, 10.0);
    if let Some(ref d) = d_str {
        paint_svg_path_preview(ui, cue_box, d, Stroke::new(stroke_w, ctx.pal.accent_emerald), Some(ctx.pal.accent_amber));
    }

    let scale_px = stage_rect.width() / 960.0;
    let p_start = Pos2::new(cue_box.min.x + 8.0, cue_box.center().y + 12.0);
    let p_end = Pos2::new(cue_box.max.x - 8.0, cue_box.center().y - 12.0);
    let c1 = center + Vec2::new(ctx.state.motion.tangent_in[0] * scale_px, ctx.state.motion.tangent_in[1] * scale_px);
    let c2 = center + Vec2::new(ctx.state.motion.tangent_out[0] * scale_px, ctx.state.motion.tangent_out[1] * scale_px);

    ui.painter().add(CubicBezierShape::from_points_stroke(
        [p_start, c1, c2, p_end],
        false,
        Color32::TRANSPARENT,
        Stroke::new(1.8, ctx.pal.accent_amber.gamma_multiply(0.85)),
    ));
    ui.painter().line_segment([p_start, c1], Stroke::new(1.0, ctx.pal.accent_sky.gamma_multiply(0.65)));
    ui.painter().line_segment([p_end, c2], Stroke::new(1.0, ctx.pal.accent_sky.gamma_multiply(0.65)));

    let body_hit = ui.interact(cue_box, ui.scope_id().with("motion_cue_body_drag"), Sense::click_and_drag());
    if body_hit.dragged() {
        let d = body_hit.drag_delta();
        ctx.state.motion.pos_x = (ctx.state.motion.pos_x + f64::from(d.x / stage_rect.width().max(1.0)) * 1920.0).clamp(120.0, 1800.0);
        ctx.state.motion.pos_y = (ctx.state.motion.pos_y + f64::from(d.y / stage_rect.height().max(1.0)) * 1080.0).clamp(100.0, 980.0);
        sync_active_fields_back_to_segment(ctx);
        ctx.state.dirty_drafts.insert("motion".into());
    }
    ctx.reg.record("motion-stage-cue-box", &ctx.state.motion.cue_name, &body_hit, None);

    ui.painter().rect_stroke(cue_box, CornerRadius::same(4), Stroke::new(1.4, ctx.pal.accent_sky), StrokeKind::Outside);
    for (ci, cpos) in [cue_box.left_top(), cue_box.right_top(), cue_box.left_bottom(), cue_box.right_bottom()].iter().enumerate() {
        let hrect = Rect::from_center_size(*cpos, Vec2::splat(8.0));
        ui.painter().rect_filled(hrect, CornerRadius::same(1), Color32::WHITE);
        ui.painter().rect_stroke(hrect, CornerRadius::same(1), Stroke::new(1.2, ctx.pal.accent_sky), StrokeKind::Outside);
        let ch = ui.interact(hrect.expand(4.0), ui.scope_id().with(("motion_corner", ci)), Sense::drag());
        if ch.dragged() {
            let sign_x = if ci.is_multiple_of(2) { -1.0 } else { 1.0 };
            let sign_y = if ci < 2 { -1.0 } else { 1.0 };
            let d = ch.drag_delta();
            ctx.state.motion.width = (ctx.state.motion.width + f64::from(d.x * sign_x * 2.0 / stage_rect.width().max(1.0)) * 1920.0).clamp(180.0, 1500.0);
            ctx.state.motion.height = (ctx.state.motion.height + f64::from(d.y * sign_y * 2.0 / stage_rect.height().max(1.0)) * 1080.0).clamp(120.0, 860.0);
            sync_active_fields_back_to_segment(ctx);
            ctx.state.dirty_drafts.insert("motion".into());
        }
    }

    for (ti, tpos) in [(0usize, c1), (1usize, c2)] {
        let clamped = Pos2::new(tpos.x.clamp(stage_rect.min.x + 8.0, stage_rect.max.x - 8.0), tpos.y.clamp(stage_rect.min.y + 8.0, stage_rect.max.y - 8.0));
        ui.painter().circle_filled(clamped, 5.0, ctx.pal.accent_amber);
        ui.painter().circle_stroke(clamped, 5.0, Stroke::new(1.2, Color32::WHITE));
        let th = ui.interact(Rect::from_center_size(clamped, Vec2::splat(14.0)), ui.scope_id().with(("motion_tangent", ti)), Sense::drag());
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
            rebuild_dynamic_segment_svg(ctx);
            sync_active_fields_back_to_segment(ctx);
            ctx.state.dirty_drafts.insert("motion".into());
        }
    }

    let fx_short = ctx.state.motion.selected_effect_id.rsplit('.').next().unwrap_or("trim_paths");
    let seg_tag = ctx.state.motion.selected_cue_id.as_deref().unwrap_or("seg-1");
    ui.painter().text(
        Pos2::new(cue_box.center().x, (cue_box.min.y - 7.0).max(stage_rect.min.y + 10.0)),
        Align2::CENTER_BOTTOM,
        format!("{} [{seg_tag}] · {fx_short} ({:.0}, {:.0})", ctx.state.motion.cue_name, ctx.state.motion.pos_x, ctx.state.motion.pos_y),
        FontId::monospace(8.8),
        ctx.pal.accent_emerald,
    );
}
