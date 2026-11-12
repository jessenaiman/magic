//! `Motion` view (`#view-motion`): Craft-project-native `V4` `Action Motion` workstation
//! matching `06-motion-v4-vectorcraft-effectcraft.jpg`, featuring a dedicated **Left 35mm Film Reel**,
//! underlying audio playback, and `EffectCraft` (`V4`) ↔ `FilmCraft` (`V1`) track switching.

use craft_ui::layout::{SplitAxis, SplitSize, split_rect};
use egui::{Align, Align2, Color32, CornerRadius, FontId, Layout, Pos2, Rect, RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui, Vec2};
use serde_json::{Value, json};

use crate::controls::{
    FilmbarOrientation, FilmbarStyle, badge_accent_color, child_pane, fill_card, paint_craft_doc_card, paint_filmbar_frame, paint_local_image,
    paint_svg_path_preview, pill_toggle,
};
use crate::state::{PickerTarget, UiAction, ViewCtx};
use crate::timeline::{CraftTimelineLane, CraftTimelineSpan, draw_craft_timeline};
use crate::views::motion_inspector::{draw_lottie_toolbar_buttons, draw_lottie_warnings_strip, draw_motion_inspector};
use crate::widgets::{BtnSpec, action_button, status_pill};

pub fn show(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    draw_toolbar(ui, ctx);
    draw_lottie_warnings_strip(ui, ctx);
    ui.add_space(6.0);
    let body = ui.available_rect_before_wrap();
    let left_w = if body.width() >= 1180.0 { 248.0 } else { 220.0 };
    let right_w = if body.width() >= 1180.0 { 324.0 } else { 278.0 };

    let after_left = if ctx.state.show_nav_rail
        && let Some(left_split) = split_rect(body, SplitAxis::Horizontal, SplitSize::FixedFirst(left_w), 8.0, 180.0)
    {
        let mut cues_rail = child_pane(ui, left_split.first, "motion_cues_rail");
        draw_motion_film_reel(&mut cues_rail, ctx);
        left_split.second
    } else {
        body
    };

    if ctx.state.show_right_inspector
        && let Some(right_split) = split_rect(after_left, SplitAxis::Horizontal, SplitSize::FixedSecond(right_w), 8.0, 220.0)
    {
        let mut stage = child_pane(ui, right_split.first, "motion_stage_pane");
        draw_motion_stage_and_timeline(&mut stage, ctx);
        let mut insp = child_pane(ui, right_split.second, "motion_insp_pane");
        draw_motion_inspector(&mut insp, ctx);
    } else {
        let mut stage = child_pane(ui, after_left, "motion_stage_pane");
        draw_motion_stage_and_timeline(&mut stage, ctx);
    }
    let _ = ui.allocate_rect(body, Sense::hover());
}

fn draw_toolbar(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let has_ecproj = ctx.state.cache.motion_doc["ecproj"].is_string();
    let svg_cnt = ctx.state.cache.motion_doc["svg_files"].as_array().map_or(0, Vec::len);
    ctx.pal.panel_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            ui.spacing_mut().item_spacing.y = 4.0;
            let doc_lbl = if has_ecproj { format!("V4 · motion/actions.ecproj · {svg_cnt} .svg") } else { format!("V4 · Action Motion ({svg_cnt} .svg)") };
            let badge_resp = ui.label(RichText::new(doc_lbl).monospace().size(10.0).color(ctx.pal.accent_amber));
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
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("motion-save-cue", "save", "Save Cue", "Write SVG + sync motion/actions.ecproj")).clicked() {
                save_active_motion_cue(ctx);
            }
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("motion-render-frame", "image", "Render Frame", "Render active EffectCraft or FilmCraft frame at cue time"),
            )
            .clicked()
            {
                if ctx.state.motion.track_source == "filmcraft" {
                    ctx.exec("video.frame", json!({"seconds": ctx.state.motion.start_sec}), Some("Rendered FilmCraft frame"));
                } else {
                    ctx.exec("motion.preview_frame", json!({"time": ctx.state.motion.start_sec}), Some("Rendered EffectCraft motion preview frame"));
                }
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
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("motion-reload", "refresh", "Reload motion.read & motion.effects")).clicked() {
                ctx.exec("motion.read", json!({}), None);
                ctx.exec("motion.effects", json!({}), None);
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

/// Dedicated Left 35mm Film Reel (`FilmbarOrientation::Vertical`) for Motion Cues & FilmCraft scene beats.
fn draw_motion_film_reel(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let lines = ctx.state.cache.motion_doc.get("lines").and_then(Value::as_array).cloned().unwrap_or_default();
    let cues = ctx.state.cache.motion_doc.get("cues").and_then(Value::as_array).cloned().unwrap_or_default();
    let film_frames = ctx.state.active_film_frames();

    let panel = fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Film Reel · Cues").size(13.0).strong());
            status_pill(ui, ctx.pal, &format!("{} cue(s)", cues.len()), ctx.pal.accent_emerald);
        });
        ui.label(RichText::new("motion/actions.ecproj & motion/svg/*.svg").monospace().size(9.5).color(ctx.pal.text_muted));
        ui.separator();

        ScrollArea::vertical().id_salt("motion_cues_list").auto_shrink([false, false]).show(ui, |ui| {
            if cues.is_empty() {
                draw_starter_reel_cues(ui, ctx, &film_frames);
            }
            for (idx, cue) in cues.iter().enumerate() {
                let scene_bg = if film_frames.is_empty() { None } else { film_frames.get(idx % film_frames.len()).map(String::as_str) };
                draw_single_cue_reel_frame(ui, ctx, idx, cue, scene_bg);
                ui.add_space(6.0);
            }

            if !lines.is_empty() {
                ui.add_space(6.0);
                ui.collapsing(RichText::new("Anchor Cue to Sung Word").size(10.5).color(ctx.pal.text_secondary), |ui| {
                    for line in lines.iter().take(12) {
                        if let Some(words) = line["words"].as_array() {
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
                                            ctx.push(UiAction::SeekAudio(st));
                                        }
                                    }
                                }
                            });
                        }
                    }
                });
            }
        });
    });
    ctx.reg.record("motion-cues-rail", "Motion Film Reel", &panel.response, None);
}

fn draw_starter_reel_cues(ui: &mut Ui, ctx: &mut ViewCtx<'_>, film_frames: &[String]) {
    let total = ctx.state.audio_duration_sec.max(20.0);
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
        let card_w = (ui.available_width() - 4.0).max(140.0);
        let (crect, resp) = ui.allocate_exact_size(Vec2::new(card_w, 106.0), Sense::click());
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
        let band = Rect::from_min_max(Pos2::new(inner.min.x, (inner.max.y - 32.0).max(inner.min.y)), inner.max);
        ui.painter().rect_filled(band, CornerRadius::same(2), Color32::from_rgba_unmultiplied(8, 12, 18, 222));
        ui.painter().text(
            Pos2::new(band.min.x + 6.0, band.min.y + 8.5),
            Align2::LEFT_CENTER,
            format!("0:{reel_sec:04.1} · ▸ {title}"),
            FontId::proportional(10.8),
            ctx.pal.text_primary,
        );
        ui.painter().text(
            Pos2::new(band.min.x + 6.0, band.max.y - 8.0),
            Align2::LEFT_CENTER,
            sub,
            FontId::monospace(8.8),
            if sel { ctx.pal.accent_emerald } else { ctx.pal.accent_amber },
        );
        if resp.clicked() {
            ctx.state.motion.selected_preset = pid.into();
            ctx.state.motion.cue_name = title.into();
            ctx.state.motion.start_sec = reel_sec;
            ctx.state.dirty_drafts.insert("motion".into());
            ctx.push(UiAction::SeekAudio(reel_sec));
        }
        ctx.reg.record(format!("motion-seed-card-{pid}"), title, &resp, None);
        ui.add_space(6.0);
    }
}

fn draw_single_cue_reel_frame(ui: &mut Ui, ctx: &mut ViewCtx<'_>, idx: usize, cue: &Value, scene_bg: Option<&str>) {
    let cid = cue["id"].as_str().unwrap_or("cue");
    let word = cue["word"].as_str().or_else(|| cue["title"].as_str()).unwrap_or(cid);
    let clock = cue["clock"].as_str().unwrap_or("0:00.0");
    let fx = cue["effect_id"].as_str().unwrap_or("ec.shape.trim_paths");
    let preview_img = cue["preview_frame"].as_str().or_else(|| cue["svg_file"].as_str()).or(scene_bg);
    let svg_d = cue["d"].as_str().unwrap_or("");
    let svg_label = cue["svg_file"].as_str().and_then(|s| s.rsplit('/').next()).unwrap_or("actions.ecproj");
    let selected = ctx.state.motion.cue_name == word;

    let card_w = (ui.available_width() - 4.0).max(140.0);
    let (crect, resp) = ui.allocate_exact_size(Vec2::new(card_w, 108.0), Sense::click());
    let inner =
        paint_filmbar_frame(ui, crect, ctx.pal, FilmbarStyle { orientation: FilmbarOrientation::Vertical, selected, hovered: resp.hovered(), timecode: None });
    if !paint_local_image(ui, inner, preview_img, ctx.pal.bg_stage, 3) {
        ui.painter().rect_filled(inner, CornerRadius::same(3), ctx.pal.bg_stage);
    }
    if !svg_d.is_empty() {
        paint_svg_path_preview(ui, inner.shrink(10.0), svg_d, Stroke::new(2.0, ctx.pal.accent_emerald), None);
    }

    let band = Rect::from_min_max(Pos2::new(inner.min.x, (inner.max.y - 34.0).max(inner.min.y)), inner.max);
    ui.painter().rect_filled(band, CornerRadius::same(2), Color32::from_rgba_unmultiplied(8, 12, 18, 224));
    ui.painter().text(
        Pos2::new(band.min.x + 6.0, band.min.y + 9.0),
        Align2::LEFT_CENTER,
        format!("{clock} · ▸ {word}"),
        FontId::proportional(11.0),
        ctx.pal.text_primary,
    );
    ui.painter().text(
        Pos2::new(band.min.x + 6.0, band.max.y - 8.5),
        Align2::LEFT_CENTER,
        svg_label,
        FontId::monospace(8.8),
        badge_accent_color(ctx.pal, "SVG"),
    );

    let mut enabled_cue = true;
    let tog_rect = Rect::from_min_size(Pos2::new(band.max.x - 42.0, band.max.y - 19.0), Vec2::new(38.0, 17.0));
    let mut tog_ui = ui.new_child(egui::UiBuilder::new().max_rect(tog_rect).scope_id(ui.scope_id().with(("cue_tog", idx))));
    let _ = pill_toggle(&mut tog_ui, ctx.reg, ctx.pal, &format!("motion-cue-enable-{idx}"), "", &mut enabled_cue, ctx.pal.accent_emerald);

    if resp.clicked() {
        ctx.state.motion.cue_name = word.to_owned();
        ctx.state.motion.selected_effect_id = fx.to_owned();
        if let Some(st) = cue["start"].as_f64() {
            ctx.state.motion.start_sec = st;
            ctx.push(UiAction::SeekAudio(st));
        }
    }
    ctx.reg.record(format!("motion-cue-card-{idx}"), word, &resp, None);
}

fn active_cue(ctx: &ViewCtx<'_>) -> Option<Value> {
    let cues = ctx.state.cache.motion_doc["cues"].as_array()?;
    cues.iter()
        .find(|c| c["word"].as_str() == Some(ctx.state.motion.cue_name.as_str()) || c["title"].as_str() == Some(ctx.state.motion.cue_name.as_str()))
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

fn draw_motion_stage_and_timeline(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let center_rect = ui.available_rect_before_wrap();
    let timeline_h = 178.0;
    if ctx.state.show_bottom_dock
        && let Some(vert) = split_rect(center_rect, SplitAxis::Vertical, SplitSize::FixedSecond(timeline_h), 8.0, 180.0)
    {
        let mut stage_ui = child_pane(ui, vert.first, "motion_upper_stage");
        draw_motion_stage(&mut stage_ui, ctx);
        let mut tl_ui = child_pane(ui, vert.second, "motion_lower_timeline");
        draw_real_motion_timeline(&mut tl_ui, ctx);
        let _ = ui.allocate_rect(center_rect, Sense::hover());
    } else {
        draw_motion_stage(ui, ctx);
    }
}

fn draw_motion_audio_and_track_bar(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let has_audio = ctx.state.song_audio_path().is_some();
    let play_icon = if ctx.state.audio_playing { "pause" } else { "play" };
    let play_lbl = if ctx.state.audio_playing { "Pause" } else { "Play Audio" };
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.spacing_mut().item_spacing.y = 4.0;
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::new("motion-audio-play", play_icon, play_lbl, "Play / pause underlying song or FilmCraft audio track").primary().enabled(has_audio),
        )
        .clicked()
        {
            ctx.push(UiAction::ToggleAudio);
        }
        ui.label(RichText::new(format!("0:{:04.1}", ctx.state.motion.start_sec)).monospace().size(9.8).color(if ctx.state.audio_playing {
            ctx.pal.accent_emerald
        } else {
            ctx.pal.text_muted
        }));
        for (src, lbl, tip) in [
            ("effectcraft", "EC · V4 Motion", "Isolate EffectCraft motion/actions.ecproj track"),
            ("filmcraft", "FC · Film Track", "Preview underlying FilmCraft V1 / program sequence track"),
            ("overlay", "FC + V4", "Composite EffectCraft V4 vector motion over FilmCraft V1 sequence"),
        ] {
            let id = format!("motion-track-src-{src}");
            let sel = ctx.state.motion.track_source == src;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, tip).selected(sel)).clicked() {
                ctx.state.motion.track_source = src.into();
                ctx.exec("video.frame", json!({"seconds": ctx.state.motion.start_sec}), None);
            }
        }
        for (aud, lbl, tip) in
            [("song", "♪ Song", "Monitor underlying song audio stem"), ("filmcraft", "FC Mix", "Monitor rendered FilmCraft sequence audio mix")]
        {
            let id = format!("motion-audio-src-{aud}");
            let sel = ctx.state.motion.audio_source == aud;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, tip).selected(sel)).clicked() {
                ctx.state.motion.audio_source = aud.into();
                if ctx.state.audio_playing {
                    let t = ctx.state.motion.start_sec;
                    ctx.push(UiAction::SeekAudio(t));
                }
            }
        }
    });
}

fn draw_motion_stage(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
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
    let svg_files = ctx.state.cache.motion_doc["svg_files"].as_array().cloned().unwrap_or_default();
    let cue = active_cue(ctx);

    fill_card(ui, ctx.pal, true, |ui| {
        draw_motion_audio_and_track_bar(ui, ctx);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
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
        let w = (avail.x - 2.0).max(180.0);
        let h = (w * 9.0 / 16.0).min((avail.y - 6.0).max(128.0));
        let (outer, oresp) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click());
        let tc_badge = match ctx.state.motion.track_source.as_str() {
            "filmcraft" => "V1 · FilmCraft .fcproj",
            "effectcraft" => "V4 · actions.ecproj",
            _ => "FC V1 + EC V4 · Composite",
        };
        let rect = paint_filmbar_frame(
            ui,
            outer,
            ctx.pal,
            FilmbarStyle { orientation: FilmbarOrientation::Horizontal, selected: false, hovered: oresp.hovered(), timecode: Some(tc_badge) },
        );

        let rendered_motion = ctx
            .state
            .cache
            .last_motion_frame
            .as_deref()
            .or_else(|| cue.as_ref().and_then(|c| c["preview_frame"].as_str()))
            .or_else(|| preview_frames.first().and_then(Value::as_str));
        let cue_svg = cue.as_ref().and_then(|c| c["svg_file"].as_str()).or_else(|| svg_files.first().and_then(Value::as_str));

        if ctx.state.motion.track_source == "filmcraft" {
            if !paint_local_image(ui, rect, backdrop.as_deref(), ctx.pal.bg_stage, 4) {
                paint_craft_doc_card(
                    ui,
                    rect.shrink(16.0),
                    ctx.pal,
                    "FC",
                    "film/Song.fcproj",
                    "Click Assemble in Studio or Render Frame to preview FilmCraft V1 track",
                );
            }
        } else if !paint_local_image(ui, rect, rendered_motion, ctx.pal.bg_stage, 4) {
            if ctx.state.motion.track_source == "overlay" {
                let _ = paint_local_image(ui, rect, backdrop.as_deref(), ctx.pal.bg_stage, 4);
            } else {
                ui.painter().rect_filled(rect, CornerRadius::same(4), ctx.pal.bg_stage);
            }
            if !paint_local_image(ui, rect.shrink(12.0), cue_svg, Color32::TRANSPARENT, 4) {
                if let Some(d_str) = cue
                    .as_ref()
                    .and_then(|c| c["d"].as_str())
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
                    .or_else(|| preset_svg_d(ctx, &ctx.state.motion.selected_preset))
                {
                    let stroke_w = (ctx.state.motion.stroke_width as f32 * 0.45).clamp(2.0, 9.0);
                    paint_svg_path_preview(ui, rect, &d_str, Stroke::new(stroke_w, ctx.pal.accent_emerald), Some(ctx.pal.accent_amber));
                } else {
                    paint_craft_doc_card(
                        ui,
                        rect.shrink(16.0),
                        ctx.pal,
                        "VCRAFT",
                        "motion/actions.ecproj · motion/svg/*.svg",
                        "Pick a starter SVG cue or trace a sketch, then click Save Cue",
                    );
                }
            }
        }
        ui.painter().rect_stroke(rect, CornerRadius::same(4), Stroke::new(1.0, ctx.pal.border_subtle), StrokeKind::Inside);
    });
}

fn draw_real_motion_timeline(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let total_dur = ctx.state.cache.motion_doc["duration"].as_f64().unwrap_or(ctx.state.audio_duration_sec).max(15.0);
    let cues = ctx.state.cache.motion_doc["cues"].as_array().cloned().unwrap_or_default();
    let has_audio = ctx.state.song_audio_path().is_some();
    let film_frames = ctx.state.active_film_frames();
    let show_fc = ctx.state.motion.track_source != "effectcraft";

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
            label: if ctx.state.motion.audio_source == "filmcraft" { "FC Audio Mix".into() } else { "Song Audio & Beats".into() },
        }]
    } else {
        Vec::new()
    };

    fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Timeline").size(12.5).strong());
            ui.label(RichText::new("· FC V1 + EC V4 Vector Keyframes").monospace().size(9.5).color(ctx.pal.text_muted));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("Playhead {:.1}s / {:.1}s", ctx.state.motion.start_sec, total_dur))
                        .monospace()
                        .size(9.5)
                        .color(ctx.pal.accent_emerald),
                );
            });
        });

        let dirty_v4 = ctx.state.dirty_tracks.contains("V4");
        let top_title = if show_fc { "FC V1 / BEAT" } else { "▸ Beat / Audio" };
        let lanes = [
            CraftTimelineLane {
                code: "BEAT",
                title: top_title,
                color: ctx.pal.accent_amber,
                filmbar: show_fc,
                waveform: has_audio,
                dirty: false,
                spans: beat_spans,
                keyframes: &beat_keys,
                frames: if show_fc { &film_frames } else { &[] },
                empty_hint: "Beat markers",
            },
            CraftTimelineLane {
                code: "CUE",
                title: "T Cue",
                color: ctx.pal.accent_amber,
                filmbar: false,
                waveform: false,
                dirty: false,
                spans: cue_spans.clone(),
                keyframes: &[],
                frames: &[],
                empty_hint: "",
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
                title: "⟳ Trim",
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
