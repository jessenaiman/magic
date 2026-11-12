//! `04 Motion` dockable `Navigator` (`Motion Cues` 35mm spline reel) and `Timeline`
//! (`EffectCraft` `V4` + `TRIM PATHS` sequence dock) panels rendered inside [`craft_ui::docking`].

use egui::{Align2, Color32, CornerRadius, FontId, Pos2, Rect, RichText, ScrollArea, Sense, Stroke, Ui, Vec2};
use serde_json::Value;

use crate::controls::{FilmbarOrientation, FilmbarStyle, badge_accent_color, fill_card, paint_filmbar_frame, paint_local_image, paint_svg_path_preview};
use crate::state::{UiAction, ViewCtx};
use crate::state_views::MotionSegmentEdit;
use crate::timeline::{CraftTimelineLane, CraftTimelineSpan, StandardTimelineSpec, TimelineFlavor, draw_standard_timeline};
use crate::views::motion::{activate_segment_by_id, ensure_cues_hydrated_into_segments, preset_svg_d, resolved_motion_cues};
use crate::widgets::{BtnSpec, action_button, status_pill};

pub fn draw_motion_film_reel(ui: &mut Ui, ctx: &mut ViewCtx<'_>) -> Option<Pos2> {
    ensure_cues_hydrated_into_segments(ctx);
    let lines = ctx.state.cache.motion_doc.get("lines").and_then(Value::as_array).cloned().unwrap_or_default();
    let cues = resolved_motion_cues(ctx);
    let film_frames = ctx.state.active_film_frames();
    let mut selected_socket = None;

    let panel = fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Film Reel · Cues").size(12.4).strong());
            status_pill(ui, ctx.pal, &format!("{} segment(s)", cues.len().max(4)), ctx.pal.accent_emerald);
        });
        ui.label(RichText::new("Select any scene cue to edit its spline segment").size(9.5).color(ctx.pal.text_muted));
        ui.separator();

        ScrollArea::vertical().id_salt("motion_cues_list").auto_shrink([false, false]).show(ui, |ui| {
            if cues.is_empty() {
                selected_socket = draw_starter_reel_cues(ui, ctx, &film_frames);
            } else {
                for (idx, cue) in cues.iter().enumerate() {
                    let scene_bg = cue["backdrop_path"]
                        .as_str()
                        .filter(|s| !s.is_empty())
                        .or_else(|| if film_frames.is_empty() { None } else { film_frames.get(idx % film_frames.len()).map(String::as_str) });
                    if let Some(sock) = draw_single_cue_reel_frame(ui, ctx, idx, cue, scene_bg) {
                        selected_socket = Some(sock);
                    }
                    ui.add_space(5.0);
                }
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
    ui.collapsing(RichText::new("Anchor Segment to Sung Word").size(10.5).color(ctx.pal.text_secondary), |ui| {
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
                    if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&btn_id, txt, "Bind motion segment to sung word").selected(sel)).clicked() {
                        // TODO(agent): verify motion_dock README for borrow pattern; d23f5f3 needs values extracted before mutable borrow
                        let new_id = format!("word-cue-{}", wid.replace('/', "-"));
                        let d_str = preset_svg_d(ctx, &ctx.state.motion.selected_preset).unwrap_or_default();
                        let st = w["start"].as_f64().unwrap_or(ctx.state.motion.start_sec);
                        let preset = ctx.state.motion.selected_preset.clone();
                        let effect_id = ctx.state.motion.selected_effect_id.clone();
                        let backdrop_path = ctx.state.film_frame_at_time(st);
                        ctx.state.motion.segments.entry(new_id.clone()).or_insert_with(|| MotionSegmentEdit {
                            cue_id: new_id.clone(),
                            word_id: wid.clone(),
                            title: txt.to_owned(),
                            preset,
                            d: d_str,
                            pos_x: 960.0,
                            pos_y: 520.0,
                            width: 520.0,
                            height: 240.0,
                            arc_bend_deg: 18.0,
                            tangent_in: [-125.0, -60.0],
                            tangent_out: [125.0, -60.0],
                            start_sec: st,
                            end_sec: st + 2.0,
                            effect_id,
                            backdrop_path,
                        });
                        activate_segment_by_id(ctx, &new_id);
                        ctx.push(UiAction::SeekAudio(st));
                    }
                }
            });
        }
    });
}

fn draw_starter_reel_cues(ui: &mut Ui, ctx: &mut ViewCtx<'_>, film_frames: &[String]) -> Option<Pos2> {
    let total = ctx.state.audio_duration_sec.max(24.0);
    let mut selected_socket = None;
    for (idx, (pid, title, sub)) in [
        ("moon", "Sleeping Still", "cave-rest.svg"),
        ("swoop_arc", "Bottom of Hill", "cave-swoop.svg"),
        ("burst", "Sudden Cry", "wake-burst.svg"),
        ("wing_glide", "Fly East & West", "wing-glide.svg"),
        ("spiral", "Fly All Around", "nest-spiral.svg"),
    ]
    .into_iter()
    .enumerate()
    {
        let cid = format!("starter-{pid}");
        let sel = ctx.state.motion.selected_cue_id.as_deref() == Some(cid.as_str()) || (ctx.state.motion.selected_cue_id.is_none() && idx == 0);
        let reel_sec = (idx as f64) * (total / 5.0);
        let card_w = (ui.available_width() - 4.0).max(132.0);
        let (crect, resp) = ui.allocate_exact_size(Vec2::new(card_w, 96.0), Sense::click());
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
            format!("0:{reel_sec:04.1} · {title}"),
            FontId::proportional(10.2),
            ctx.pal.text_primary,
        );
        ui.painter().text(
            Pos2::new(band.min.x + 6.0, band.max.y - 7.5),
            Align2::LEFT_CENTER,
            sub,
            FontId::monospace(8.5),
            if sel { ctx.pal.accent_emerald } else { ctx.pal.accent_amber },
        );
        if sel {
            selected_socket = Some(Pos2::new(crect.max.x - 2.0, crect.center().y));
        }
        if resp.clicked() {
            let d_str = preset_svg_d(ctx, pid).unwrap_or_default();
            ctx.state.motion.segments.entry(cid.clone()).or_insert_with(|| MotionSegmentEdit {
                cue_id: cid.clone(),
                word_id: format!("seed/w{idx}"),
                title: title.into(),
                preset: pid.into(),
                d: d_str,
                pos_x: 840.0 + (idx as f64) * 70.0,
                pos_y: 470.0 + ((idx % 2) as f64) * 110.0,
                width: 520.0,
                height: 240.0,
                arc_bend_deg: if idx % 2 == 0 { 20.0 } else { -18.0 },
                tangent_in: [-135.0, -52.0 + (idx as f32) * 18.0],
                tangent_out: [135.0, -52.0 - (idx as f32) * 12.0],
                start_sec: reel_sec,
                end_sec: reel_sec + 2.4,
                effect_id: "ec.shape.trim_paths".into(),
                backdrop_path: bg_frame.map(str::to_owned),
            });
            activate_segment_by_id(ctx, &cid);
            ctx.state.dirty_drafts.insert("motion".into());
            ctx.push(UiAction::SeekAudio(reel_sec));
        }
        ctx.reg.record(format!("motion-seed-card-{pid}"), title, &resp, None);
        ui.add_space(5.0);
    }
    selected_socket
}

fn draw_single_cue_reel_frame(ui: &mut Ui, ctx: &mut ViewCtx<'_>, idx: usize, cue: &Value, scene_bg: Option<&str>) -> Option<Pos2> {
    let cid = cue["id"].as_str().map(str::to_owned).unwrap_or_else(|| format!("cue-{}", idx + 1));
    let title = cue["title"].as_str().or_else(|| cue["word"].as_str()).unwrap_or(&cid);
    let clock = cue["clock"].as_str().unwrap_or("0:00.0");
    let preview_img = cue["preview_frame"].as_str().or(scene_bg);
    let seg_d = ctx.state.motion.segments.get(&cid).map(|s| s.d.as_str()).or_else(|| cue["d"].as_str()).unwrap_or("");
    let svg_label = cue["svg_file"].as_str().and_then(|s| s.rsplit('/').next()).or_else(|| cue["preset"].as_str()).unwrap_or("actions.ecproj");
    let selected = ctx.state.motion.selected_cue_id.as_deref().map_or(idx == 0, |sel| sel == cid);

    let card_w = (ui.available_width() - 4.0).max(132.0);
    let (crect, resp) = ui.allocate_exact_size(Vec2::new(card_w, 98.0), Sense::click());
    let inner =
        paint_filmbar_frame(ui, crect, ctx.pal, FilmbarStyle { orientation: FilmbarOrientation::Vertical, selected, hovered: resp.hovered(), timecode: None });
    if !paint_local_image(ui, inner, preview_img, ctx.pal.bg_stage, 3) {
        ui.painter().rect_filled(inner, CornerRadius::same(3), ctx.pal.bg_stage);
    }
    if !seg_d.is_empty() {
        paint_svg_path_preview(ui, inner.shrink(10.0), seg_d, Stroke::new(2.2, ctx.pal.accent_emerald), None);
    }

    let band = Rect::from_min_max(Pos2::new(inner.min.x, (inner.max.y - 32.0).max(inner.min.y)), inner.max);
    ui.painter().rect_filled(band, CornerRadius::same(2), Color32::from_rgba_unmultiplied(8, 12, 18, 224));
    let short_title: String = if title.chars().count() > 20 {
        let mut s: String = title.chars().take(19).collect();
        s.push('…');
        s
    } else {
        title.to_owned()
    };
    ui.painter().text(
        Pos2::new(band.min.x + 6.0, band.min.y + 8.5),
        Align2::LEFT_CENTER,
        format!("{clock} · {short_title}"),
        FontId::proportional(10.2),
        ctx.pal.text_primary,
    );
    ui.painter().text(
        Pos2::new(band.min.x + 6.0, band.max.y - 8.0),
        Align2::LEFT_CENTER,
        svg_label,
        FontId::monospace(8.5),
        badge_accent_color(ctx.pal, "SVG"),
    );

    if resp.clicked() {
        activate_segment_by_id(ctx, &cid);
        ctx.push(UiAction::SeekAudio(ctx.state.motion.start_sec));
    }
    ctx.reg.record(format!("motion-cue-card-{idx}"), title, &resp, None);
    selected.then_some(Pos2::new(crect.max.x - 2.0, crect.center().y))
}

pub fn draw_real_motion_timeline(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ensure_cues_hydrated_into_segments(ctx);
    let total_dur = ctx.state.cache.motion_doc["duration"].as_f64().unwrap_or(ctx.state.audio_duration_sec).max(15.0);
    let cues = resolved_motion_cues(ctx);
    let has_audio = ctx.state.song_audio_path().is_some();

    let mut cue_spans: Vec<CraftTimelineSpan> = cues
        .iter()
        .enumerate()
        .map(|(idx, c)| {
            let cid = c["id"].as_str().map(str::to_owned).unwrap_or_else(|| format!("cue-{}", idx + 1));
            let st = c["start"].as_f64().unwrap_or((idx as f64) * 4.5);
            let en = c["end"].as_f64().unwrap_or(st + 2.2);
            let lbl = c["word"].as_str().or_else(|| c["title"].as_str()).unwrap_or("Segment");
            CraftTimelineSpan::new(st, en, lbl).with_id(cid)
        })
        .collect();

    if cue_spans.is_empty() {
        let st = ctx.state.motion.start_sec.clamp(0.0, (total_dur - 1.5).max(0.0));
        let en = (st + ctx.state.motion.draw_duration + ctx.state.motion.hold_duration).min(total_dur);
        cue_spans.push(CraftTimelineSpan::new(st, en.max(st + 1.2), format!("{} (draft)", ctx.state.motion.cue_name)));
    }

    let trim_spans: Vec<CraftTimelineSpan> = cue_spans
        .iter()
        .map(|sp| {
            CraftTimelineSpan::new(sp.start_sec, (sp.start_sec + ctx.state.motion.draw_duration.max(0.4)).min(sp.end_sec), "Trim Paths").with_id(sp.id.clone())
        })
        .collect();

    let beat_keys: Vec<f64> = (0..8).map(|i| (i as f64) * (total_dur / 8.0)).collect();
    let motion_keys: Vec<f64> = cue_spans.iter().flat_map(|s| [s.start_sec, s.end_sec]).collect();
    let dirty_v4 = ctx.state.dirty_tracks.contains("V4") || ctx.state.dirty_drafts.contains("motion");

    fill_card(ui, ctx.pal, true, |ui| {
        let lanes = [
            CraftTimelineLane {
                code: "AUDIO",
                title: "BEAT / AUDIO",
                color: ctx.pal.accent_amber,
                filmbar: false,
                waveform: has_audio,
                dirty: false,
                spans: Vec::new(),
                keyframes: &beat_keys,
                frames: &[],
                empty_hint: "Attach song audio",
            },
            CraftTimelineLane {
                code: "V4",
                title: "EC V4 SEGMENTS",
                color: ctx.pal.accent_emerald,
                filmbar: false,
                waveform: false,
                dirty: dirty_v4,
                spans: cue_spans,
                keyframes: &motion_keys,
                frames: &[],
                empty_hint: "Select or save a motion segment",
            },
            CraftTimelineLane {
                code: "TRIM",
                title: "TRIM PATHS",
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
        let out = draw_standard_timeline(
            ui,
            ctx,
            StandardTimelineSpec { id_prefix: "motion", flavor: TimelineFlavor::EffectCraft, total_dur_sec: total_dur, fps: 24.0, in_out: None },
            &mut playhead,
            &lanes,
        );
        if out.toggled_play {
            ctx.push(UiAction::ToggleAudio);
        }
        if let Some((_, span_id, span_sec)) = out.clicked_span {
            if ctx.state.motion.segments.contains_key(&span_id) {
                activate_segment_by_id(ctx, &span_id);
            } else {
                ctx.state.motion.start_sec = span_sec;
            }
            ctx.push(UiAction::SeekAudio(span_sec));
        } else if out.scrubbed {
            ctx.state.motion.start_sec = playhead;
            ctx.push(UiAction::SeekAudio(playhead));
        }
    });
}
