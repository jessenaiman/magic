//! `Studio` view (`#view-studio`): unified `FilmCraft Studio` (`film/<song>.fcproj`) bringing
//! `V1` Storyboard Scenes, `V2` Character Animation, `V3` Kinetic Lyrics, `V4` Action Motion &
//! Titles, `C1` Captions, and `A1` Song Master together onto one live timeline & composite
//! Program Monitor so everything can be previewed and edited directly inside Magic Studio.

use egui::{Align, Align2, Color32, CornerRadius, FontId, Layout, Pos2, Rect, RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui, Vec2};
use serde_json::{Value, json};

use crate::controls::{
    FilmbarOrientation, FilmbarStyle, ScalarSpec, fill_card, paint_craft_doc_card, paint_filmbar_frame, paint_local_image, pill_toggle, ring_slider_h,
};
use crate::state::{UiAction, ViewCtx, ViewMode};
use crate::timeline::{CraftTimelineLane, CraftTimelineSpan, StandardTimelineSpec, TimelineFlavor, draw_standard_timeline, format_smpte_timecode};
use crate::views::studio_audio_titles::{draw_audio_tab, draw_export_tab, draw_titles_tab};
use crate::views::studio_finishing::{
    count_offline_clips, draw_color_tab, draw_edit_tab, draw_fx_tab, draw_graded_scopes_strip, draw_program_trim_and_three_point_bar, draw_studio_mode_tabs,
};
use crate::widgets::{BtnSpec, action_button, status_pill};

const TICKS_PER_SEC_F64: f64 = 254_016_000_000.0;

struct StudioSceneItem {
    title: String,
    start_sec: f64,
    end_sec: f64,
    image: Option<String>,
}

struct CraftOutputs {
    fcproj_path: String,
    fcproj_exists: bool,
    program_frames: Vec<String>,
    v1_frames: Vec<String>,
    v2_frames: Vec<String>,
    v3_frames: Vec<String>,
    v4_frames: Vec<String>,
}

pub fn draw_stage_header(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let outputs = collect_craft_outputs(ctx);
    draw_studio_header(ui, ctx, &outputs);
    if ctx.state.studio.show_export_settings {
        ui.add_space(4.0);
        draw_export_drawer(ui, ctx);
    }
}

pub fn draw_navigator_panel(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let outputs = collect_craft_outputs(ctx);
    let scenes = collect_studio_scenes(ctx, &outputs);
    draw_film_scenes_rail(ui, ctx, &scenes, &outputs);
}

pub fn draw_canvas_panel(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let outputs = collect_craft_outputs(ctx);
    let scenes = collect_studio_scenes(ctx, &outputs);
    draw_program_monitor_column(ui, ctx, &scenes, &outputs);
}

pub fn draw_inspector_panel(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let outputs = collect_craft_outputs(ctx);
    let scenes = collect_studio_scenes(ctx, &outputs);
    draw_right_finishing_deck(ui, ctx, &scenes, &outputs);
}

pub fn draw_timeline_panel(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let outputs = collect_craft_outputs(ctx);
    let scenes = collect_studio_scenes(ctx, &outputs);
    draw_filmcraft_sequence_dock(ui, ctx, &scenes, &outputs);
}

fn json_str_vec(val: &Value) -> Vec<String> {
    val.as_array().into_iter().flatten().filter_map(|v| v.as_str().map(str::to_owned)).collect()
}

fn collect_craft_outputs(ctx: &ViewCtx<'_>) -> CraftOutputs {
    let co = &ctx.state.cache.film_status["craft_outputs"];
    let fcproj_path = ctx.state.cache.film_status.pointer("/timeline/path").and_then(Value::as_str).unwrap_or("film/Song.fcproj").to_owned();
    let fcproj_exists = ctx.state.cache.film_status.pointer("/timeline/exists").and_then(Value::as_bool).unwrap_or(false);
    let mut program_frames = json_str_vec(&co["program_frames"]);
    if let Some(last) = &ctx.state.cache.last_video_frame
        && !program_frames.contains(last)
    {
        program_frames.insert(0, last.clone());
    }
    let v1_frames = json_str_vec(&co["v1_frames"]);
    let mut v2_frames = json_str_vec(&co["v2_frames"]);
    if let Some(last) = &ctx.state.cache.last_character_frame
        && !v2_frames.contains(last)
    {
        v2_frames.insert(0, last.clone());
    }
    let mut v3_frames = json_str_vec(&co["v3_frames"]);
    if let Some(last) = &ctx.state.cache.last_words_frame
        && !v3_frames.contains(last)
    {
        v3_frames.insert(0, last.clone());
    }
    let mut v4_frames = json_str_vec(&co["v4_frames"]);
    if let Some(last) = &ctx.state.cache.last_motion_frame
        && !v4_frames.contains(last)
    {
        v4_frames.insert(0, last.clone());
    }
    CraftOutputs { fcproj_path, fcproj_exists, program_frames, v1_frames, v2_frames, v3_frames, v4_frames }
}

fn sequence_duration_sec(ctx: &ViewCtx<'_>) -> f64 {
    let tl_sec = ctx.state.cache.timeline["duration_tick"].as_i64().unwrap_or(0) as f64 / TICKS_PER_SEC_F64;
    tl_sec.max(ctx.state.audio_duration_sec).max(ctx.state.studio.range_end).max(12.0)
}

fn collect_studio_scenes(ctx: &ViewCtx<'_>, outputs: &CraftOutputs) -> Vec<StudioSceneItem> {
    let total_dur = sequence_duration_sec(ctx);
    if let Some(v1_clips) = ctx.state.cache.timeline["tracks"]
        .as_array()
        .and_then(|arr| arr.iter().find(|t| t["label"].as_str() == Some("V1")))
        .and_then(|t| t["clips"].as_array())
        .filter(|c| !c.is_empty())
    {
        return v1_clips
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let st = (c["start_tick"].as_i64().unwrap_or(0) as f64 / TICKS_PER_SEC_F64).max(0.0);
                let en = (c["end_tick"].as_i64().unwrap_or(0) as f64 / TICKS_PER_SEC_F64).max(st + 0.5);
                let title = c["name"].as_str().map(str::to_owned).unwrap_or_else(|| format!("Scene {}", i + 1));
                let image = c["source_path"].as_str().filter(|s| !s.is_empty()).map(str::to_owned).or_else(|| outputs.v1_frames.get(i).cloned());
                StudioSceneItem { title, start_sec: st, end_sec: en, image }
            })
            .collect();
    }

    let sb_pages = ctx.state.cache.storyboard_pages.get("pages").and_then(Value::as_array).cloned().unwrap_or_default();
    let scene_pages: Vec<&Value> =
        sb_pages.iter().filter(|p| p["section"].as_str() == Some("Scenes") && p["page_role"].as_str() != Some("section_title")).collect();
    let count = scene_pages.len().max(1);
    scene_pages
        .into_iter()
        .enumerate()
        .map(|(i, p)| {
            let st = (i as f64 / count as f64) * total_dur;
            let en = ((i + 1) as f64 / count as f64) * total_dur;
            let title = p["name"].as_str().map(str::to_owned).unwrap_or_else(|| format!("Scene {}", i + 1));
            let image = p["preview_png"]
                .as_str()
                .filter(|x| !x.is_empty())
                .map(str::to_owned)
                .or_else(|| p["slots"].as_array().and_then(|arr| arr.iter().find_map(|s| s["image"].as_str().filter(|x| !x.is_empty()).map(str::to_owned))))
                .or_else(|| outputs.v1_frames.get(i).cloned());
            StudioSceneItem { title, start_sec: st, end_sec: en.max(st + 1.0), image }
        })
        .collect()
}

fn gold_outline_button(ui: &mut Ui, ctx: &mut ViewCtx<'_>, id: &str, label: &str, tooltip: &str) -> bool {
    let font = FontId::proportional(11.8);
    let galley = ui.painter().layout_no_wrap(label.to_owned(), font.clone(), ctx.pal.accent_amber);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(galley.size().x + 20.0, 26.0), Sense::click());
    let fill = if resp.hovered() { Color32::from_rgba_unmultiplied(0x66, 0x4b, 0x1d, 190) } else { Color32::from_rgba_unmultiplied(0x4c, 0x38, 0x16, 165) };
    ui.painter().rect(rect, CornerRadius::same(6), fill, Stroke::new(1.1, ctx.pal.accent_amber), StrokeKind::Inside);
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, label, font, ctx.pal.accent_amber);
    let resp = resp.on_hover_text(tooltip);
    ctx.reg.record(id, label, &resp, None);
    resp.clicked()
}

#[allow(clippy::cognitive_complexity)]
fn draw_studio_header(ui: &mut Ui, ctx: &mut ViewCtx<'_>, outputs: &CraftOutputs) {
    let ready = ctx.state.cache.film_status["export_ready"].as_bool().unwrap_or(false);
    let fc_live = ctx.state.project_path.is_some();
    let fc_short = outputs.fcproj_path.rsplit('/').next().unwrap_or("Song.fcproj");
    let offline_n = count_offline_clips(&ctx.state.cache.timeline);

    let bar = ctx.pal.card_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            ui.spacing_mut().item_spacing.y = 4.0;
            ui.label(RichText::new(format!("FilmCraft Studio — film/{fc_short}")).size(14.0).strong().color(ctx.pal.text_primary));
            status_pill(
                ui,
                ctx.pal,
                if ready {
                    "Export Ready"
                } else if outputs.fcproj_exists {
                    ".fcproj Live"
                } else {
                    "Ready to Assemble"
                },
                if ready || outputs.fcproj_exists { ctx.pal.accent_emerald } else { ctx.pal.accent_amber },
            );
            status_pill(
                ui,
                ctx.pal,
                if fc_live { "All Tracks Synced" } else { "No Project" },
                if fc_live { ctx.pal.accent_emerald } else { ctx.pal.text_muted },
            );
            if offline_n > 0 {
                status_pill(ui, ctx.pal, &format!("{offline_n} Offline"), ctx.pal.danger);
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("film-assemble", "Assemble", "Rebuild FilmCraft .fcproj from V1–V4 + A1 + C1")).clicked()
            {
                ctx.exec("video.assemble", json!({}), Some("Assembled FilmCraft .fcproj sequence"));
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("film-preview", "play", "Preview", "Render half-scale FilmCraft preview").primary()).clicked() {
                ctx.exec("video.preview", json!({}), Some("Rendered FilmCraft preview"));
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("film-frame", "camera", "Render composite PNG frame from .fcproj at playhead")).clicked()
            {
                let sec = ctx.state.studio.seek_seconds;
                ctx.exec("video.frame", json!({"seconds": sec}), Some("Rendered composite frame at playhead"));
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("film-export", "upload", "Export Film", "Render final master film or craft-audio master"))
                .clicked()
            {
                let payload = json!({
                    "format": ctx.state.studio.export_format,
                    "container": ctx.state.studio.export_container,
                    "hdrTransfer": ctx.state.studio.export_hdr_transfer,
                    "bitrateMode": ctx.state.studio.bitrate_mode,
                    "crf": ctx.state.studio.crf_value as u64,
                    "scale": ctx.state.studio.export_scale,
                    "quality": ctx.state.studio.export_quality as u64,
                    "burnCaptions": ctx.state.studio.burn_captions,
                    "allowOffline": ctx.state.studio.allow_offline,
                    "gpuRendering": ctx.state.studio.gpu_rendering,
                    "hwEncoding": ctx.state.studio.hw_encoding,
                    "queue": ctx.state.studio.queue_export,
                });
                ctx.exec("video.export", payload, Some("Exported master output"));
            }
            if gold_outline_button(ui, ctx, "film-open", "Open in FilmCraft", "Optional: inspect .fcproj externally in FilmCraft") {
                ctx.exec("video.open", json!({}), Some("Opened .fcproj in FilmCraft"));
            }
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::icon_only("film-export-settings-toggle", "sliders", "Configure export settings").selected(ctx.state.studio.show_export_settings),
            )
            .clicked()
            {
                ctx.state.studio.show_export_settings = !ctx.state.studio.show_export_settings;
            }
        });
    });
    ctx.reg.record("film-head", "Studio Header", &bar.response, None);
}

fn draw_export_drawer(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let drawer = ctx.pal.card_frame().show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.spacing_mut().item_spacing.y = 4.0;
            for fmt in ["h264", "hevc", "prores", "dnxhr", "av1", "vp9", "exr", "webm", "gif", "wav", "aiff", "flac", "mp3", "aac"] {
                let id = format!("film-export-{fmt}");
                let sel = ctx.state.studio.export_format == fmt;
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, fmt, "Select pure-Rust export codec").selected(sel)).clicked() {
                    ctx.state.studio.export_format = fmt.into();
                }
            }
            pill_toggle(ui, ctx.reg, ctx.pal, "film-burn-captions", "Burn Captions", &mut ctx.state.studio.burn_captions, ctx.pal.accent_emerald);
            pill_toggle(ui, ctx.reg, ctx.pal, "film-export-gpu", "GPU Render", &mut ctx.state.studio.gpu_rendering, ctx.pal.accent_sky);
            pill_toggle(ui, ctx.reg, ctx.pal, "film-export-queue", "Queue", &mut ctx.state.studio.queue_export, ctx.pal.accent_amber);
        });
    });
    ctx.reg.record("film-export-settings", "Export Settings", &drawer.response, None);
}

fn draw_film_scenes_rail(ui: &mut Ui, ctx: &mut ViewCtx<'_>, scenes: &[StudioSceneItem], outputs: &CraftOutputs) {
    let panel = fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.label(RichText::new("Film / Scenes").size(12.5).strong());
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("studio-rail-relink", "link", "Relink offline clips (T08)")).clicked() {
                ctx.exec("video.sync_links", json!({"render": false}), Some("Checked linked track assets"));
            }
            let prox = ctx.state.studio.proxies_enabled;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("studio-rail-proxy", "Proxy", "Toggle proxies (T35)").selected(prox)).clicked() {
                ctx.state.studio.proxies_enabled = !prox;
                ctx.exec("video.finish", json!({"op": "toggle_proxies", "enabled": !prox}), Some("Toggled FilmCraft proxy mode"));
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("studio-rail-csv", "CSV", "Export sequence markers CSV (T35)")).clicked() {
                ctx.exec("video.markers", json!({"action": "export_csv"}), Some("Exported sequence markers CSV"));
            }
        });
        ui.add_space(4.0);
        ScrollArea::vertical().id_salt("studio_film_scenes_scroll").auto_shrink([false, false]).show(ui, |ui| {
            if scenes.is_empty() {
                let w = (ui.available_width() - 4.0).max(140.0);
                let (erect, _) = ui.allocate_exact_size(Vec2::new(w, 114.0), Sense::hover());
                paint_craft_doc_card(
                    ui,
                    erect,
                    ctx.pal,
                    "FC",
                    outputs.fcproj_path.rsplit('/').next().unwrap_or("Song.fcproj"),
                    "Add DesignCraft scenes or click Assemble",
                );
                return;
            }
            for (idx, sc) in scenes.iter().enumerate() {
                let selected = ctx.state.studio.selected_scene == idx;
                let w = (ui.available_width() - 4.0).max(140.0);
                let (card_rect, resp) = ui.allocate_exact_size(Vec2::new(w, 112.0), Sense::click());
                if resp.clicked() {
                    ctx.state.studio.selected_scene = idx;
                    ctx.state.studio.seek_seconds = sc.start_sec;
                    ctx.push(UiAction::SeekAudio(sc.start_sec));
                }
                let inner = paint_filmbar_frame(
                    ui,
                    card_rect,
                    ctx.pal,
                    FilmbarStyle { orientation: FilmbarOrientation::Vertical, selected, hovered: resp.hovered(), timecode: None },
                );
                if !paint_local_image(ui, inner, sc.image.as_deref(), ctx.pal.bg_stage, 3) {
                    paint_craft_doc_card(ui, inner, ctx.pal, "DC", &sc.title, "Scene");
                }
                let band = Rect::from_min_max(Pos2::new(inner.min.x, (inner.max.y - 24.0).max(inner.min.y)), inner.max);
                ui.painter().rect_filled(band, CornerRadius::same(2), Color32::from_rgba_unmultiplied(8, 12, 18, 220));
                ui.painter().text(
                    Pos2::new(band.min.x + 6.0, band.center().y),
                    Align2::LEFT_CENTER,
                    format!("0:{:04.1} · {}", sc.start_sec, sc.title),
                    FontId::proportional(9.8),
                    if selected { ctx.pal.accent_emerald } else { ctx.pal.text_primary },
                );
                ctx.reg.record(format!("studio-scene-{}", idx + 1), &sc.title, &resp, None);
                ui.add_space(6.0);
            }
        });
    });
    ctx.reg.record("studio-scenes-rail", "Film / Scenes", &panel.response, None);
}

fn find_active_clip_at<'a>(timeline: &'a Value, code: &str, sec: f64) -> Option<&'a Value> {
    let tick = (sec.max(0.0) * TICKS_PER_SEC_F64).round() as i64;
    let track = timeline["tracks"].as_array()?.iter().find(|t| t["label"].as_str() == Some(code))?;
    if track["muted"].as_bool() == Some(true) || track["enabled"].as_bool() == Some(false) {
        return None;
    }
    track["clips"].as_array()?.iter().find(|c| c["start_tick"].as_i64().unwrap_or(0) <= tick && c["end_tick"].as_i64().unwrap_or(0) >= tick)
}

fn frame_for_playhead(frames: &[String], playhead_sec: f64, dur: f64) -> Option<&str> {
    if frames.is_empty() {
        return None;
    }
    let idx = (((playhead_sec / dur.max(0.1)).clamp(0.0, 0.9999)) * (frames.len() as f64)).floor() as usize;
    frames.get(idx).map(String::as_str)
}

#[allow(clippy::cognitive_complexity)]
/// Live playhead-synced 16:9 Program Monitor compositing active `V1`–`V4` + `C1` tracks in real time.
fn draw_program_monitor_column(ui: &mut Ui, ctx: &mut ViewCtx<'_>, scenes: &[StudioSceneItem], outputs: &CraftOutputs) {
    let dur = sequence_duration_sec(ctx);
    let playhead = ctx.state.studio.seek_seconds;
    let solo = ctx.state.studio.solo_track.clone();

    let active_scene = scenes
        .iter()
        .find(|s| playhead >= s.start_sec && playhead <= s.end_sec)
        .or_else(|| scenes.get(ctx.state.studio.selected_scene))
        .or_else(|| scenes.first());

    let v1_clip = find_active_clip_at(&ctx.state.cache.timeline, "V1", playhead).cloned();
    let v2_clip = find_active_clip_at(&ctx.state.cache.timeline, "V2", playhead).cloned();
    let v3_clip = find_active_clip_at(&ctx.state.cache.timeline, "V3", playhead).cloned();
    let v4_clip = find_active_clip_at(&ctx.state.cache.timeline, "V4", playhead).cloned();
    let c1_clip = find_active_clip_at(&ctx.state.cache.timeline, "C1", playhead).cloned();

    let base_img = match solo.as_deref() {
        Some("V2") => v2_clip.as_ref().and_then(|c| c["source_path"].as_str()).or_else(|| frame_for_playhead(&outputs.v2_frames, playhead, dur)),
        Some("V3") => frame_for_playhead(&outputs.v3_frames, playhead, dur),
        Some("V4") => frame_for_playhead(&outputs.v4_frames, playhead, dur),
        _ => frame_for_playhead(&outputs.program_frames, playhead, dur)
            .or_else(|| v1_clip.as_ref().and_then(|c| c["source_path"].as_str()))
            .or_else(|| active_scene.and_then(|s| s.image.as_deref())),
    };

    let v2_overlay_img = if solo.is_none() && !ctx.state.studio.muted_tracks[1] {
        v2_clip.as_ref().and_then(|c| c["source_path"].as_str()).or_else(|| frame_for_playhead(&outputs.v2_frames, playhead, dur))
    } else {
        None
    };
    let active_lyric = if (solo.is_none() || solo.as_deref() == Some("V3")) && !ctx.state.studio.muted_tracks[2] {
        v3_clip.as_ref().and_then(|c| c["name"].as_str()).map(str::to_owned)
    } else {
        None
    };
    let active_motion_label = if (solo.is_none() || solo.as_deref() == Some("V4")) && !ctx.state.studio.muted_tracks[3] {
        v4_clip.as_ref().and_then(|c| c["name"].as_str()).map(str::to_owned)
    } else {
        None
    };
    let active_caption = c1_clip.as_ref().and_then(|c| c["name"].as_str()).map(str::to_owned);

    let mon_card = fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            ui.label(RichText::new("Program Monitor").size(12.2).strong());
            if let Some(ref solo_trk) = solo {
                status_pill(ui, ctx.pal, &format!("Solo {solo_trk}"), ctx.pal.accent_amber);
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("film-clear-solo", "Full Mix", "Return monitor to full V1–V4 composite")).clicked()
                {
                    ctx.state.studio.solo_track = None;
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                let fit_lbl = if ctx.state.studio.monitor_fit { "Fit" } else { "100%" };
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("film-mon-fit", fit_lbl, "Toggle Fit vs 100% monitor zoom")).clicked() {
                    ctx.state.studio.monitor_fit = !ctx.state.studio.monitor_fit;
                }
                let _ = pill_toggle(ui, ctx.reg, ctx.pal, "film-safe-margins", "Safe Zone", &mut ctx.state.studio.safe_margins, ctx.pal.accent_sky);
                let _ = pill_toggle(ui, ctx.reg, ctx.pal, "film-snap-toggle", "Snap", &mut ctx.state.studio.snap_enabled, ctx.pal.accent_emerald);
            });
        });

        let scopes_reserve = if ctx.state.studio.scopes_visible { 92.0 } else { 0.0 };
        let controls_reserve = 74.0 + scopes_reserve;
        let avail_w = (ui.available_width() - 2.0).max(210.0);
        let max_mon_h = (ui.available_height() - controls_reserve).max(164.0);
        let mon_h = (avail_w * 9.0 / 16.0).min(max_mon_h).max(150.0);
        let mon_w = (mon_h * 16.0 / 9.0).min(avail_w);

        let (outer, resp) = ui.allocate_exact_size(Vec2::new(avail_w, mon_h), Sense::click());
        let rect = Rect::from_center_size(outer.center(), Vec2::new(mon_w, mon_h));
        ui.painter().rect_filled(outer, CornerRadius::same(8), Color32::from_rgb(7, 10, 15));

        if !paint_local_image(ui, rect, base_img, Color32::from_rgb(13, 18, 28), 8) {
            let scene_title = active_scene.map_or("Storyboard Scene", |s| s.title.as_str());
            paint_craft_doc_card(ui, rect, ctx.pal, "V1", scene_title, "Active sequence composite");
        }

        // Composite V2 Character preview overlay
        if let Some(v2_img) = v2_overlay_img
            && Some(v2_img) != base_img
        {
            let char_rect = Rect::from_center_size(
                Pos2::new(rect.center().x, rect.center().y + rect.height() * 0.04),
                Vec2::new(rect.height() * 0.62, rect.height() * 0.62),
            );
            let _ = paint_local_image(ui, char_rect, Some(v2_img), Color32::TRANSPARENT, 4);
        }

        // Composite V4 Motion / Graphic Title overlay
        if let Some(mot_txt) = &active_motion_label {
            let pill = Rect::from_center_size(Pos2::new(rect.center().x, rect.min.y + 26.0), Vec2::new((rect.width() * 0.56).min(340.0), 26.0));
            ui.painter().rect(
                pill,
                CornerRadius::same(6),
                Color32::from_rgba_unmultiplied(8, 22, 26, 210),
                Stroke::new(1.1, ctx.pal.accent_emerald),
                StrokeKind::Inside,
            );
            ui.painter().text(pill.center(), Align2::CENTER_CENTER, format!("V4 · {mot_txt}"), FontId::proportional(11.5), ctx.pal.accent_emerald);
        }

        // Composite V3 Kinetic Lyrics overlay
        if let Some(lyric_txt) = &active_lyric {
            let lyr_rect = Rect::from_center_size(Pos2::new(rect.center().x, rect.max.y - 42.0), Vec2::new((rect.width() - 32.0).max(120.0), 34.0));
            ui.painter().rect_filled(lyr_rect, CornerRadius::same(6), Color32::from_rgba_unmultiplied(8, 12, 20, 196));
            ui.painter().text(lyr_rect.center(), Align2::CENTER_CENTER, lyric_txt, FontId::proportional(15.0), ctx.pal.accent_amber);
        } else if let Some(cap_txt) = &active_caption {
            let cap_rect = Rect::from_center_size(Pos2::new(rect.center().x, rect.max.y - 26.0), Vec2::new((rect.width() * 0.72).max(120.0), 24.0));
            ui.painter().rect_filled(cap_rect, CornerRadius::same(4), Color32::from_rgba_unmultiplied(0, 0, 0, 215));
            ui.painter().text(cap_rect.center(), Align2::CENTER_CENTER, cap_txt, FontId::proportional(11.5), Color32::WHITE);
        }

        // Live exposure / vignette preview
        if ctx.state.studio.color_exposure.abs() > 0.05 {
            let alpha = (ctx.state.studio.color_exposure.abs() * 28.0).clamp(0.0, 110.0) as u8;
            let tint = if ctx.state.studio.color_exposure > 0.0 {
                Color32::from_rgba_unmultiplied(255, 250, 240, alpha)
            } else {
                Color32::from_rgba_unmultiplied(0, 0, 0, alpha)
            };
            ui.painter().rect_filled(rect, CornerRadius::same(8), tint);
        }

        if ctx.state.studio.safe_margins {
            for (frac, col) in [(0.90_f32, ctx.pal.accent_sky.gamma_multiply(0.55)), (0.80_f32, ctx.pal.accent_amber.gamma_multiply(0.55))] {
                let sr = Rect::from_center_size(rect.center(), rect.size() * frac);
                ui.painter().rect_stroke(sr, CornerRadius::same(2), Stroke::new(1.0, col), StrokeKind::Inside);
            }
        }
        ui.painter().rect_stroke(outer, CornerRadius::same(8), Stroke::new(1.0, ctx.pal.border), StrokeKind::Inside);
        ctx.reg.record("film-player", "Program Monitor", &resp, None);

        ui.add_space(3.0);
        draw_filmcraft_transport_row(ui, ctx, dur);
        draw_program_trim_and_three_point_bar(ui, ctx);
        draw_graded_scopes_strip(ui, ctx);
    });
    ctx.reg.record("film-status", "Program Card", &mon_card.response, None);
}

fn draw_filmcraft_transport_row(ui: &mut Ui, ctx: &mut ViewCtx<'_>, dur: f64) {
    let frame_dt = 1.0 / 24.0;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.label(
            RichText::new(format!("{} / {}", format_smpte_timecode(ctx.state.studio.seek_seconds, 24.0), format_smpte_timecode(dur, 24.0)))
                .monospace()
                .size(10.0)
                .color(ctx.pal.accent_emerald),
        );
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("film-go-in", "skipStart", "Go to In / Start")).clicked() {
            ctx.state.studio.seek_seconds = ctx.state.studio.range_start.max(0.0);
            ctx.push(UiAction::SeekAudio(ctx.state.studio.seek_seconds));
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("film-step-prev", "stepBack", "Step Back 1 Frame (Left)")).clicked() {
            ctx.state.studio.seek_seconds = (ctx.state.studio.seek_seconds - frame_dt).max(0.0);
            ctx.push(UiAction::SeekAudio(ctx.state.studio.seek_seconds));
        }
        let play_icon = if ctx.state.audio_playing { "pause" } else { "play" };
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("film-play", play_icon, "Play / Stop Program Monitor (Space)").primary()).clicked() {
            ctx.push(UiAction::ToggleAudio);
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("film-step-next", "stepForward", "Step Forward 1 Frame (Right)")).clicked() {
            ctx.state.studio.seek_seconds = (ctx.state.studio.seek_seconds + frame_dt).min(dur);
            ctx.push(UiAction::SeekAudio(ctx.state.studio.seek_seconds));
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("film-go-out", "skipEnd", "Go to Out / End")).clicked() {
            ctx.state.studio.seek_seconds = dur;
            ctx.push(UiAction::SeekAudio(dur));
        }

        let scrub_w = (ui.available_width() - 156.0).clamp(84.0, 290.0);
        ui.allocate_ui_with_layout(Vec2::new(scrub_w, 26.0), Layout::top_down(Align::Min), |ui| {
            if ring_slider_h(
                ui,
                ctx.reg,
                ctx.pal,
                ScalarSpec::new("film-seek", "Playhead", 0.0..=dur, "s", ctx.pal.accent_emerald),
                &mut ctx.state.studio.seek_seconds,
            )
            .changed()
            {
                let s = ctx.state.studio.seek_seconds;
                ctx.push(UiAction::SeekAudio(s));
            }
        });

        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("film-razor-split", "razor", "Split clip at playhead (⌘K)")).clicked() {
            ctx.exec("video.finish", json!({"op": "razor", "seconds": ctx.state.studio.seek_seconds}), Some("Split clip at playhead"));
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("film-add-marker", "pin", "Add timeline marker (M)")).clicked() {
            ctx.exec(
                "video.markers",
                json!({"action": "add", "seconds": ctx.state.studio.seek_seconds, "name": "Studio Cue"}),
                Some("Added sequence marker at playhead"),
            );
        }
    });
}

fn draw_right_finishing_deck(ui: &mut Ui, ctx: &mut ViewCtx<'_>, scenes: &[StudioSceneItem], outputs: &CraftOutputs) {
    let panel = fill_card(ui, ctx.pal, true, |ui| {
        draw_studio_mode_tabs(ui, ctx);
        ui.separator();

        if ctx.state.studio.inspector_tab == "matrix" {
            draw_elements_matrix_body(ui, ctx, scenes, outputs);
        } else {
            ScrollArea::vertical().id_salt("studio_finishing_deck_scroll").auto_shrink([false, false]).show(ui, |ui| {
                match ctx.state.studio.inspector_tab.as_str() {
                    "edit" => draw_edit_tab(ui, ctx),
                    "color" => draw_color_tab(ui, ctx),
                    "fx" => draw_fx_tab(ui, ctx),
                    "audio" => draw_audio_tab(ui, ctx),
                    "titles" => draw_titles_tab(ui, ctx),
                    "export" => draw_export_tab(ui, ctx),
                    _ => {}
                }
            });
        }
    });
    ctx.reg.record("studio-elements-matrix", "Elements Matrix", &panel.response, None);
}

#[allow(clippy::cognitive_complexity)]
fn draw_elements_matrix_body(ui: &mut Ui, ctx: &mut ViewCtx<'_>, scenes: &[StudioSceneItem], outputs: &CraftOutputs) {
    let row_count = scenes.len().max(outputs.v2_frames.len()).max(outputs.v3_frames.len()).max(outputs.v4_frames.len()).clamp(4, 8);

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 5.0;
        ui.label(RichText::new("Elements Matrix").size(12.5).strong());
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("studio-matrix-sync", "refresh", "Sync Comps", "Re-render dirty linked .ecproj comps (T01)"))
            .clicked()
        {
            ctx.exec("video.sync_links", json!({}), Some("Synced linked EffectCraft compositions"));
        }
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::label_only("studio-matrix-bake", "Bake V3", "Bake linked lettering/lyrics.ecproj to ProRes 4444 .mov (T01)"),
        )
        .clicked()
        {
            ctx.exec("video.bake_link", json!({"project": "lettering/lyrics.ecproj", "track": "V3"}), Some("Baked linked V3 .ecproj to ProRes 4444"));
        }
    });
    ui.add_space(3.0);

    let avail_total = (ui.available_width() - 6.0).max(220.0);
    let tc_w = 38.0_f32;
    let gap = 4.0_f32;
    let col_w = ((avail_total - tc_w - gap * 3.0) / 3.0).floor().max(60.0);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        ui.allocate_exact_size(Vec2::new(tc_w, 24.0), Sense::hover());
        for (trk, play_id, title, tip) in [
            ("V2", "studio-play-track-v2", "▶ Animation", "Play / Solo V2 Character Animation track"),
            ("V3", "studio-play-track-v3", "▶ Lyrics", "Play / Solo V3 Kinetic Lyrics track"),
            ("V4", "studio-play-track-v4", "▶ Motion", "Play / Solo V4 Action Motion track"),
        ] {
            ui.allocate_ui_with_layout(Vec2::new(col_w, 24.0), Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                let is_solo = ctx.state.studio.solo_track.as_deref() == Some(trk);
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(play_id, title, tip).selected(is_solo)).clicked() {
                    ctx.state.studio.solo_track = if is_solo { None } else { Some(trk.into()) };
                    if !ctx.state.audio_playing {
                        ctx.push(UiAction::ToggleAudio);
                    }
                    ctx.exec("video.finish", json!({"op": "track_patch", "track": trk, "solo": !is_solo, "patched": true, "target": true}), None);
                }
            });
        }
    });
    ui.add_space(2.0);

    ScrollArea::vertical().id_salt("studio_elements_matrix_scroll").auto_shrink([false, false]).show(ui, |ui| {
        for row in 0..row_count {
            let row_sec = scenes.get(row).map_or((row as f64) * 4.5, |s| s.start_sec);
            let scene_img = scenes.get(row).and_then(|s| s.image.as_deref()).or_else(|| scenes.first().and_then(|s| s.image.as_deref()));
            let active_row = (ctx.state.studio.seek_seconds - row_sec).abs() < 2.2;

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                let (tc_rect, tc_resp) = ui.allocate_exact_size(Vec2::new(tc_w, 62.0), Sense::click());
                if active_row {
                    let pill_r = Rect::from_center_size(tc_rect.center(), Vec2::new(tc_w - 2.0, 18.0));
                    ui.painter().rect_filled(pill_r, CornerRadius::same(4), ctx.pal.accent_emerald);
                    ui.painter().text(
                        pill_r.center(),
                        Align2::CENTER_CENTER,
                        format!("0:{row_sec:04.1}"),
                        FontId::monospace(8.2),
                        Color32::from_rgb(6, 20, 16),
                    );
                } else {
                    ui.painter().text(tc_rect.center(), Align2::CENTER_CENTER, format!("0:{row_sec:04.1}"), FontId::monospace(8.4), ctx.pal.text_muted);
                }
                if tc_resp.clicked() {
                    ctx.state.studio.seek_seconds = row_sec;
                    ctx.push(UiAction::SeekAudio(row_sec));
                }

                let v2_img = outputs.v2_frames.get(row).map(String::as_str).or(scene_img);
                let v3_img = outputs.v3_frames.get(row).map(String::as_str).or(scene_img);
                let v4_img = outputs.v4_frames.get(row).map(String::as_str).or(scene_img);

                for (trk_code, target, badge, doc, frame) in [
                    ("V2", ViewMode::Animation, "EC", "anim.ecproj", v2_img),
                    ("V3", ViewMode::Design, "EC", "lyrics.ecproj", v3_img),
                    ("V4", ViewMode::Motion, "VC", "actions.ecproj", v4_img),
                ] {
                    draw_matrix_play_cell(ui, ctx, col_w, row, row_sec, active_row, (trk_code, target, badge, doc, frame));
                }
            });
            ui.add_space(4.0);
        }
    });
}

fn draw_matrix_play_cell(
    ui: &mut Ui,
    ctx: &mut ViewCtx<'_>,
    col_w: f32,
    row: usize,
    row_sec: f64,
    active_row: bool,
    cell: (&str, ViewMode, &str, &str, Option<&str>),
) {
    let (trk_code, target, badge, doc, frame) = cell;
    let (cell_rect, cresp) = ui.allocate_exact_size(Vec2::new(col_w, 62.0), Sense::click());
    let inner = paint_filmbar_frame(
        ui,
        cell_rect,
        ctx.pal,
        FilmbarStyle { orientation: FilmbarOrientation::Horizontal, selected: active_row, hovered: cresp.hovered(), timecode: None },
    );
    if !paint_local_image(ui, inner, frame, ctx.pal.bg_stage, 2) {
        paint_craft_doc_card(ui, inner, ctx.pal, badge, doc, "");
    }
    if active_row {
        ui.painter().line_segment(
            [Pos2::new(inner.min.x, inner.center().y), Pos2::new(inner.max.x, inner.center().y)],
            Stroke::new(1.3, ctx.pal.accent_emerald.gamma_multiply(0.8)),
        );
    }

    let play_rect = Rect::from_min_size(Pos2::new(inner.max.x - 22.0, inner.max.y - 18.0), Vec2::new(20.0, 16.0));
    let play_hit = ui.interact(play_rect, ui.scope_id().with(("mat_play", trk_code, row)), Sense::click());
    let play_fill = if play_hit.hovered() { ctx.pal.accent_emerald } else { Color32::from_rgba_unmultiplied(8, 16, 22, 224) };
    let play_fg = if play_hit.hovered() { Color32::from_rgb(6, 20, 16) } else { ctx.pal.accent_emerald };
    ui.painter().rect(play_rect, CornerRadius::same(4), play_fill, Stroke::new(1.0, ctx.pal.accent_emerald), StrokeKind::Inside);
    ui.painter().text(play_rect.center(), Align2::CENTER_CENTER, "▶", FontId::proportional(9.5), play_fg);

    let edit_rect = Rect::from_min_size(Pos2::new(inner.min.x + 2.0, inner.min.y + 2.0), Vec2::new(24.0, 14.0));
    ui.painter().rect_filled(edit_rect, CornerRadius::same(3), Color32::from_rgba_unmultiplied(8, 12, 18, 205));
    ui.painter().text(edit_rect.center(), Align2::CENTER_CENTER, trk_code, FontId::monospace(7.8), ctx.pal.text_secondary);

    if play_hit.clicked() {
        ctx.state.studio.seek_seconds = row_sec;
        ctx.state.studio.solo_track = Some(trk_code.into());
        ctx.push(UiAction::SeekAudio(row_sec));
        if !ctx.state.audio_playing {
            ctx.push(UiAction::ToggleAudio);
        }
    } else if cresp.clicked() {
        ctx.state.studio.seek_seconds = row_sec;
        ctx.push(UiAction::NavigateAuthoring { view: target, seconds: row_sec });
    }
    ctx.reg.record(format!("studio-play-{}-{row}", trk_code.to_ascii_lowercase()), format!("Play {trk_code} row {}", row + 1), &play_hit, None);
    ctx.reg.record(format!("studio-cell-{}-{row}", trk_code.to_ascii_lowercase()), format!("Open {trk_code} row {}", row + 1), &cresp, None);
}

fn draw_filmcraft_sequence_dock(ui: &mut Ui, ctx: &mut ViewCtx<'_>, scenes: &[StudioSceneItem], outputs: &CraftOutputs) {
    let tracks = ctx.state.cache.timeline["tracks"].as_array().cloned().unwrap_or_default();
    let total_dur = sequence_duration_sec(ctx);
    let has_audio = ctx.state.song_audio_path().is_some();

    let spans_for = |role_match: &str, code: &str| -> Vec<CraftTimelineSpan> {
        tracks
            .iter()
            .filter(|t| t["role"].as_str() == Some(role_match) || t["label"].as_str() == Some(code))
            .flat_map(|t| t["clips"].as_array().cloned().unwrap_or_default())
            .map(|c| {
                let st_sec = (c["start_tick"].as_i64().unwrap_or(0).max(0) as f64) / TICKS_PER_SEC_F64;
                let en_sec = ((c["end_tick"].as_i64().unwrap_or(0).max(1) as f64) / TICKS_PER_SEC_F64).max(st_sec + 0.25);
                let cid = c["clip"].as_u64().map(|n| n.to_string()).unwrap_or_default();
                CraftTimelineSpan::new(st_sec, en_sec, c["name"].as_str().unwrap_or(code)).with_id(cid)
            })
            .collect()
    };

    let mut film_strip_frames: Vec<String> = if !outputs.program_frames.is_empty() { outputs.program_frames.clone() } else { outputs.v1_frames.clone() };
    if film_strip_frames.is_empty() {
        film_strip_frames = scenes.iter().filter_map(|s| s.image.clone()).collect();
    }
    let marker_keys: Vec<f64> = ctx.state.cache.timeline["markers"].as_array().into_iter().flatten().filter_map(|m| m["time_sec"].as_f64()).collect();

    let dirty_v2 = ctx.state.dirty_tracks.contains("V2");
    let dirty_v3 = ctx.state.dirty_tracks.contains("V3");
    let dirty_v4 = ctx.state.dirty_tracks.contains("V4");

    let lanes = [
        CraftTimelineLane {
            code: "V4",
            title: "Action Motion",
            color: ctx.pal.accent_emerald,
            filmbar: false,
            waveform: false,
            dirty: dirty_v4,
            spans: spans_for("motion", "V4"),
            keyframes: &marker_keys,
            frames: &outputs.v4_frames,
            empty_hint: "V4 Motion — add cues in Motion or Titles",
        },
        CraftTimelineLane {
            code: "V3",
            title: "Kinetic Lyrics",
            color: ctx.pal.accent_amber,
            filmbar: false,
            waveform: false,
            dirty: dirty_v3,
            spans: spans_for("lyrics", "V3"),
            keyframes: &[],
            frames: &outputs.v3_frames,
            empty_hint: "V3 Lyrics — author timed phrases in Lyrics",
        },
        CraftTimelineLane {
            code: "V2",
            title: "Animation",
            color: ctx.pal.accent_sky,
            filmbar: false,
            waveform: false,
            dirty: dirty_v2,
            spans: spans_for("character", "V2"),
            keyframes: &[],
            frames: &outputs.v2_frames,
            empty_hint: "V2 Animation — rig or import character",
        },
        CraftTimelineLane {
            code: "V1",
            title: "Scenes",
            color: ctx.pal.accent_emerald,
            filmbar: true,
            waveform: false,
            dirty: false,
            spans: spans_for("background", "V1"),
            keyframes: &[],
            frames: &film_strip_frames,
            empty_hint: "V1 Storyboard Scenes",
        },
        CraftTimelineLane {
            code: "C1",
            title: "Captions",
            color: ctx.pal.accent_sky,
            filmbar: false,
            waveform: false,
            dirty: false,
            spans: spans_for("caption", "C1"),
            keyframes: &[],
            frames: &[],
            empty_hint: "C1 Captions — Sync V3→C1 in Titles",
        },
        CraftTimelineLane {
            code: "A1",
            title: "Song Master",
            color: ctx.pal.accent_emerald,
            filmbar: false,
            waveform: has_audio,
            dirty: false,
            spans: spans_for("song", "A1"),
            keyframes: &[],
            frames: &[],
            empty_hint: "Attach song audio",
        },
    ];

    let lane_codes = ["V4", "V3", "V2", "V1", "C1", "A1"];

    let panel = fill_card(ui, ctx.pal, true, |ui| {
        let mut seek = ctx.state.studio.seek_seconds;
        let out = draw_standard_timeline(
            ui,
            ctx,
            StandardTimelineSpec {
                id_prefix: "studio",
                flavor: TimelineFlavor::FilmCraft,
                total_dur_sec: total_dur,
                fps: 24.0,
                in_out: Some((ctx.state.studio.range_start, ctx.state.studio.range_end)),
            },
            &mut seek,
            &lanes,
        );
        if out.toggled_play {
            ctx.push(UiAction::ToggleAudio);
        }
        if let Some(split_sec) = out.razor_at {
            ctx.exec("video.finish", json!({"op": "razor", "seconds": split_sec}), Some("Split sequence clip at playhead"));
        }
        if let Some((lane_idx, clip_id_str, start_sec)) = out.clicked_span {
            let code = lane_codes.get(lane_idx).copied().unwrap_or("V1");
            if let Ok(cid) = clip_id_str.parse::<u64>() {
                ctx.state.studio.selected_clip_id = Some(cid);
                ctx.state.studio.selected_clip_track = code.into();
                if let Some(found) = lanes.get(lane_idx).and_then(|l| l.spans.iter().find(|sp| sp.id == clip_id_str)) {
                    ctx.state.studio.selected_clip_name = found.label.clone();
                    ctx.state.studio.selected_clip_start = found.start_sec;
                    ctx.state.studio.selected_clip_end = found.end_sec;
                }
                ctx.state.studio.inspector_tab = "edit".into();
            }
            ctx.state.studio.seek_seconds = start_sec;
            ctx.push(UiAction::SeekAudio(start_sec));
        } else if out.scrubbed {
            ctx.state.studio.seek_seconds = seek;
            ctx.push(UiAction::SeekAudio(seek));
        }
    });
    ctx.reg.record("studio-sequence-dock", "FilmCraft Sequence Timeline", &panel.response, None);
}
