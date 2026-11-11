//! `Studio` view (`#view-studio`): Craft-project-native `FilmCraft Studio` (`film/<song>.fcproj`)
//! displaying ONLY real rendered frames and clips from the project's Craft documents:
//! - `V1` Storyboard / Backgrounds (`storyboard.designcraft` -> `storyboard/renders/*.png`, `storyboard/.preview/*.png`)
//! - `V2` Character Animation (`assets/characters/*/*.pcraft` + `.ecproj` -> `frames/*.png`)
//! - `V3` Kinetic Lyrics (`lettering/lyrics.ecproj` -> `lettering/.preview/*.png`)
//! - `V4` Action Motion (`motion/actions.ecproj` + `motion/svg/*.svg` -> `motion/.preview/*.png`)
//! - Master Program (`film/<song>.fcproj` -> `film/frames/*.png`)

use craft_ui::layout::{SplitAxis, SplitSize, split_rect};
use egui::{
    Align, Align2, Color32, CornerRadius, FontId, Layout, Pos2, Rect, RichText, ScrollArea, Sense,
    Stroke, StrokeKind, Ui, Vec2,
};
use serde_json::{Value, json};

use crate::controls::{
    FilmbarOrientation, FilmbarStyle, ScalarSpec, child_pane, fill_card, paint_craft_doc_card,
    paint_filmbar_frame, paint_local_image, pill_toggle, ring_slider_h,
};
use crate::state::{UiAction, ViewCtx, ViewMode};
use crate::widgets::{BtnSpec, action_button, status_pill};

struct StudioSceneItem {
    title: String,
    start_sec: f64,
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

struct MomentCardSpec<'a> {
    card_w: f32,
    code: &'a str,
    title: &'a str,
    craft_badge: &'a str,
    craft_doc: &'a str,
    frame: Option<&'a str>,
    dot_col: Color32,
    target: ViewMode,
}

pub fn show(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let outputs = collect_craft_outputs(ctx);
    let scenes = collect_studio_scenes(ctx, &outputs);

    draw_studio_header(ui, ctx, &outputs);
    if ctx.state.studio.show_export_settings {
        ui.add_space(6.0);
        draw_export_drawer(ui, ctx);
    }
    ui.add_space(6.0);

    let remaining = ui.available_rect_before_wrap();
    if let Some(vert) = split_rect(remaining, SplitAxis::Vertical, SplitSize::FixedSecond(144.0), 8.0, 220.0) {
        let mut upper = child_pane(ui, vert.first, "studio_upper");
        draw_upper_three_columns(&mut upper, ctx, &scenes, &outputs);
        let mut lower = child_pane(ui, vert.second, "studio_review");
        draw_review_range_dock(&mut lower, ctx, &outputs);
        let _ = ui.allocate_rect(remaining, Sense::hover());
    } else {
        draw_upper_three_columns(ui, ctx, &scenes, &outputs);
    }
}

fn json_str_vec(val: &Value) -> Vec<String> {
    val.as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str().map(str::to_owned))
        .collect()
}

fn collect_craft_outputs(ctx: &ViewCtx<'_>) -> CraftOutputs {
    let co = &ctx.state.cache.film_status["craft_outputs"];
    let fcproj_path = ctx
        .state
        .cache
        .film_status
        .pointer("/timeline/path")
        .and_then(Value::as_str)
        .unwrap_or("film/Song.fcproj")
        .to_owned();
    let fcproj_exists = ctx
        .state
        .cache
        .film_status
        .pointer("/timeline/exists")
        .and_then(Value::as_bool)
        .unwrap_or(false);
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
    CraftOutputs {
        fcproj_path,
        fcproj_exists,
        program_frames,
        v1_frames,
        v2_frames,
        v3_frames,
        v4_frames,
    }
}

fn collect_studio_scenes(ctx: &ViewCtx<'_>, outputs: &CraftOutputs) -> Vec<StudioSceneItem> {
    let sb_pages = ctx
        .state
        .cache
        .storyboard_pages
        .get("pages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let scene_pages: Vec<StudioSceneItem> = sb_pages
        .iter()
        .filter(|p| p["archetype"].as_str() == Some("scene"))
        .enumerate()
        .map(|(i, p)| {
            let title = p["name"]
                .as_str()
                .or_else(|| p["title"].as_str())
                .map(str::to_owned)
                .unwrap_or_else(|| format!("Scene {}", i + 1));
            let img = p["preview_png"]
                .as_str()
                .filter(|x| !x.is_empty())
                .map(str::to_owned)
                .or_else(|| {
                    p["slots"].as_array().and_then(|arr| {
                        arr.iter().find_map(|s| s["image"].as_str().filter(|x| !x.is_empty()).map(str::to_owned))
                    })
                })
                .or_else(|| outputs.program_frames.get(i).cloned())
                .or_else(|| outputs.v1_frames.get(i).cloned());
            StudioSceneItem {
                title,
                start_sec: (i as f64) * 5.0,
                image: img,
            }
        })
        .collect();

    if !scene_pages.is_empty() {
        return scene_pages;
    }
    outputs
        .program_frames
        .iter()
        .enumerate()
        .map(|(idx, frame)| StudioSceneItem {
            title: frame.rsplit('/').next().unwrap_or("Program Frame").to_owned(),
            start_sec: (idx as f64) * 3.0,
            image: Some(frame.clone()),
        })
        .collect()
}

fn frame_at_index(frames: &[String], idx: usize) -> Option<&str> {
    if frames.is_empty() {
        None
    } else {
        frames.get(idx % frames.len()).map(String::as_str)
    }
}

fn draw_studio_header(ui: &mut Ui, ctx: &mut ViewCtx<'_>, outputs: &CraftOutputs) {
    let ready = ctx.state.cache.film_status["export_ready"].as_bool().unwrap_or(false);
    let fc_short = outputs.fcproj_path.rsplit('/').next().unwrap_or("Song.fcproj");
    let bar = ctx.pal.card_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 1.0;
                ui.label(
                    RichText::new(format!("FilmCraft Studio — film/{fc_short}"))
                        .size(14.5)
                        .strong()
                        .color(ctx.pal.text_primary),
                );
                let sub = if outputs.fcproj_exists {
                    format!(
                        "{} program · {} V2 anim · {} V3 lyrics · {} V4 motion frames on disk",
                        outputs.program_frames.len(),
                        outputs.v2_frames.len(),
                        outputs.v3_frames.len(),
                        outputs.v4_frames.len()
                    )
                } else {
                    "No FilmCraft .fcproj sequence assembled yet — click Assemble to build film timeline".to_owned()
                };
                ui.label(RichText::new(sub).monospace().size(10.0).color(ctx.pal.text_muted));
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("film-export", "upload", "Render final master film via FilmCraft")).clicked() {
                    let payload = json!({
                        "format": ctx.state.studio.export_format,
                        "scale": ctx.state.studio.export_scale,
                        "quality": ctx.state.studio.export_quality as u64,
                        "burnCaptions": ctx.state.studio.burn_captions,
                        "allowOffline": ctx.state.studio.allow_offline,
                        "gpuRendering": ctx.state.studio.gpu_rendering,
                        "queue": ctx.state.studio.queue_export,
                    });
                    ctx.exec("video.export", payload, Some("Exported master film"));
                }
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::icon_only("film-export-settings-toggle", "sliders", "Configure FilmCraft export settings")
                        .selected(ctx.state.studio.show_export_settings),
                )
                .clicked()
                {
                    ctx.state.studio.show_export_settings = !ctx.state.studio.show_export_settings;
                }
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("film-open", "Open in FilmCraft", "Open master .fcproj in FilmCraft")).clicked() {
                    ctx.exec("video.open", json!({}), Some("Opened .fcproj in FilmCraft"));
                }
                let frame_lbl = format!("Grab Frame ({:.1}s)", ctx.state.studio.seek_seconds);
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("film-frame", &frame_lbl, "Render program PNG frame from .fcproj at playhead")).clicked() {
                    let sec = ctx.state.studio.seek_seconds;
                    ctx.exec("video.frame", json!({"seconds": sec}), Some("Rendered program frame from .fcproj"));
                }
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("film-preview", "Preview", "Render half-scale FilmCraft preview").primary()).clicked() {
                    ctx.exec("video.preview", json!({}), Some("Rendered FilmCraft preview"));
                }
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("film-assemble", "Assemble", "Assemble FilmCraft .fcproj from V1-V4 Craft outputs")).clicked() {
                    ctx.exec("video.assemble", json!({}), Some("Assembled FilmCraft .fcproj sequence"));
                }
                status_pill(
                    ui,
                    ctx.pal,
                    if ready { "Export Ready" } else if outputs.fcproj_exists { ".fcproj Live" } else { "Unassembled" },
                    if ready { ctx.pal.accent_emerald } else { ctx.pal.accent_amber },
                );
            });
        });
    });
    ctx.reg.record("film-head", "Studio Header", &bar.response, None);
}

fn draw_export_drawer(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let drawer = ctx.pal.card_frame().show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            for fmt in ["h264", "prores", "webm", "gif", "wav"] {
                let id = format!("film-export-{fmt}");
                let sel = ctx.state.studio.export_format == fmt;
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, fmt, "Select export codec").selected(sel)).clicked() {
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

fn draw_upper_three_columns(ui: &mut Ui, ctx: &mut ViewCtx<'_>, scenes: &[StudioSceneItem], outputs: &CraftOutputs) {
    let rect = ui.available_rect_before_wrap();
    if let Some(left_split) = split_rect(rect, SplitAxis::Horizontal, SplitSize::FixedFirst(236.0), 8.0, 180.0) {
        let mut scenes_col = child_pane(ui, left_split.first, "studio_scenes_col");
        draw_film_scenes_rail(&mut scenes_col, ctx, scenes, outputs);
        if let Some(right_split) = split_rect(left_split.second, SplitAxis::Horizontal, SplitSize::FixedSecond(356.0), 8.0, 240.0) {
            let mut center_col = child_pane(ui, right_split.first, "studio_center_col");
            draw_center_monitor_and_moment(&mut center_col, ctx, scenes, outputs);
            let mut elements_col = child_pane(ui, right_split.second, "studio_elements_col");
            draw_elements_matrix(&mut elements_col, ctx, scenes, outputs);
        } else {
            let mut center_col = child_pane(ui, left_split.second, "studio_center_col");
            draw_center_monitor_and_moment(&mut center_col, ctx, scenes, outputs);
        }
        let _ = ui.allocate_rect(rect, Sense::hover());
    } else {
        draw_center_monitor_and_moment(ui, ctx, scenes, outputs);
    }
}

/// Left column: `Film / Scenes` cards with 35mm vertical filmbar borders & real project frames.
fn draw_film_scenes_rail(ui: &mut Ui, ctx: &mut ViewCtx<'_>, scenes: &[StudioSceneItem], outputs: &CraftOutputs) {
    let panel = fill_card(ui, ctx.pal, true, |ui| {
        ui.label(RichText::new("Film / Scenes").size(13.0).strong());
        ui.add_space(4.0);
        ScrollArea::vertical()
            .id_salt("studio_film_scenes_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if scenes.is_empty() {
                    let w = ui.available_width().max(150.0);
                    let (erect, _) = ui.allocate_exact_size(Vec2::new(w, 118.0), Sense::hover());
                    paint_craft_doc_card(
                        ui,
                        erect,
                        ctx.pal,
                        "FC",
                        outputs.fcproj_path.rsplit('/').next().unwrap_or("Song.fcproj"),
                        "Assemble FilmCraft timeline or add DesignCraft scenes",
                    );
                    return;
                }
                for (idx, sc) in scenes.iter().enumerate() {
                    let tc = format!("0:{:04.1}", sc.start_sec);
                    let selected = ctx.state.studio.selected_scene == idx;
                    let w = ui.available_width().max(150.0);
                    let (card_rect, resp) = ui.allocate_exact_size(Vec2::new(w, 108.0), Sense::click());
                    if resp.clicked() {
                        ctx.state.studio.selected_scene = idx;
                        ctx.state.studio.seek_seconds = sc.start_sec;
                    }
                    let inner = paint_filmbar_frame(
                        ui,
                        card_rect,
                        ctx.pal,
                        FilmbarStyle {
                            orientation: FilmbarOrientation::Vertical,
                            selected,
                            hovered: resp.hovered(),
                            timecode: None,
                        },
                    );
                    if !paint_local_image(ui, inner, sc.image.as_deref(), ctx.pal.bg_stage, 3) {
                        paint_craft_doc_card(ui, inner, ctx.pal, "DC", &sc.title, "Unrendered scene");
                    }
                    crate::controls::paint_timecode_pill(ui, inner, ctx.pal, &tc);
                    ui.label(
                        RichText::new(format!("Scene {} · {}", idx + 1, sc.title))
                            .size(10.5)
                            .color(if selected { ctx.pal.accent_emerald } else { ctx.pal.text_secondary }),
                    );
                    ctx.reg.record(format!("studio-scene-{}", idx + 1), &sc.title, &resp, None);
                    ui.add_space(6.0);
                }
            });
    });
    ctx.reg.record("studio-scenes-rail", "Film / Scenes", &panel.response, None);
}

fn draw_center_monitor_and_moment(ui: &mut Ui, ctx: &mut ViewCtx<'_>, scenes: &[StudioSceneItem], outputs: &CraftOutputs) {
    let active_scene_img = scenes
        .get(ctx.state.studio.selected_scene)
        .or_else(|| scenes.first())
        .and_then(|s| s.image.as_deref());
    let program_img = outputs.program_frames.first().map(String::as_str).or(active_scene_img);
    let step_idx = ((ctx.state.studio.seek_seconds / 2.5).floor().max(0.0)) as usize;

    let mon_card = fill_card(ui, ctx.pal, false, |ui| {
        let avail_w = ui.available_width().max(220.0);
        let mon_h = (avail_w * 9.0 / 16.0).clamp(136.0, 224.0);
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(avail_w, mon_h), Sense::click());
        if !paint_local_image(ui, rect, program_img, ctx.pal.bg_stage, 8) {
            paint_craft_doc_card(
                ui,
                rect,
                ctx.pal,
                "FC",
                outputs.fcproj_path.rsplit('/').next().unwrap_or("Song.fcproj"),
                "Click Assemble then Grab Frame to render FilmCraft program output",
            );
        }
        ui.painter()
            .rect_stroke(rect, CornerRadius::same(8), Stroke::new(1.0, ctx.pal.border), StrokeKind::Inside);
        ctx.reg.record("film-player", "Program Monitor", &resp, None);

        ui.horizontal(|ui| {
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("film-step-prev", "chevronLeft", "Step back")).clicked() {
                ctx.state.studio.seek_seconds = (ctx.state.studio.seek_seconds - 1.0).max(0.0);
            }
            let play_icon = if ctx.state.audio_playing { "pause" } else { "play" };
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("film-play", play_icon, "Toggle transport").primary()).clicked() {
                ctx.push(UiAction::ToggleAudio);
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("film-step-next", "chevronRight", "Step forward")).clicked() {
                ctx.state.studio.seek_seconds += 1.0;
            }
            let dur = ctx.state.audio_duration_sec.max(30.0);
            ring_slider_h(
                ui,
                ctx.reg,
                ctx.pal,
                ScalarSpec::new("film-seek", "Playhead", 0.0..=dur, "s", ctx.pal.accent_emerald),
                &mut ctx.state.studio.seek_seconds,
            );
        });
    });
    ctx.reg.record("film-status", "Program Card", &mon_card.response, None);

    ui.add_space(6.0);
    fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("At this moment (Craft Project Outputs)").size(12.5).strong());
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("0:{:04.1}", ctx.state.studio.seek_seconds))
                        .monospace()
                        .size(10.5)
                        .color(ctx.pal.text_muted),
                );
            });
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let card_w = ((ui.available_width() - 12.0) / 3.0).max(90.0);
            draw_moment_layer_card(
                ui,
                ctx,
                MomentCardSpec {
                    card_w,
                    code: "V2",
                    title: "V2 Animation",
                    craft_badge: "EC",
                    craft_doc: ".pcraft / .ecproj",
                    frame: frame_at_index(&outputs.v2_frames, step_idx),
                    dot_col: ctx.pal.accent_violet,
                    target: ViewMode::Animation,
                },
            );
            draw_moment_layer_card(
                ui,
                ctx,
                MomentCardSpec {
                    card_w,
                    code: "V3",
                    title: "V3 Lyrics",
                    craft_badge: "EC",
                    craft_doc: "lettering/lyrics.ecproj",
                    frame: frame_at_index(&outputs.v3_frames, step_idx),
                    dot_col: ctx.pal.accent_amber,
                    target: ViewMode::Design,
                },
            );
            draw_moment_layer_card(
                ui,
                ctx,
                MomentCardSpec {
                    card_w,
                    code: "V4",
                    title: "V4 Motion",
                    craft_badge: "VC",
                    craft_doc: "motion/actions.ecproj",
                    frame: frame_at_index(&outputs.v4_frames, step_idx),
                    dot_col: ctx.pal.accent_sky,
                    target: ViewMode::Motion,
                },
            );
        });
    });
}

fn draw_moment_layer_card(ui: &mut Ui, ctx: &mut ViewCtx<'_>, spec: MomentCardSpec<'_>) {
    ui.allocate_ui(Vec2::new(spec.card_w, 106.0), |ui| {
        let (r, _) = ui.allocate_exact_size(Vec2::new(spec.card_w, 74.0), Sense::hover());
        if !paint_local_image(ui, r, spec.frame, ctx.pal.bg_stage, 6) {
            paint_craft_doc_card(ui, r, ctx.pal, spec.craft_badge, spec.craft_doc, "Not rendered yet");
        }
        ui.painter()
            .rect_stroke(r, CornerRadius::same(6), Stroke::new(1.0, ctx.pal.border), StrokeKind::Inside);
        ui.horizontal(|ui| {
            let (drect, _) = ui.allocate_exact_size(Vec2::new(8.0, 16.0), Sense::hover());
            ui.painter().circle_filled(drect.center(), 3.2, spec.dot_col);
            ui.label(RichText::new(spec.title).size(10.5).color(ctx.pal.text_secondary));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let btn_id = format!("studio-moment-edit-{}", spec.code);
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&btn_id, "Edit in…", "Open Craft domain view at playhead")).clicked() {
                    ctx.push(UiAction::NavigateAuthoring {
                        view: spec.target,
                        seconds: ctx.state.studio.seek_seconds,
                    });
                }
            });
        });
    });
}

/// Right column: `Elements Matrix` 3-column (`Animation`, `Lyrics`, `Motion`) filmbar grid of real rendered Craft frames.
fn draw_elements_matrix(ui: &mut Ui, ctx: &mut ViewCtx<'_>, scenes: &[StudioSceneItem], outputs: &CraftOutputs) {
    let row_count = scenes
        .len()
        .max(outputs.v2_frames.len())
        .max(outputs.v3_frames.len())
        .max(outputs.v4_frames.len())
        .clamp(1, 6);

    let panel = fill_card(ui, ctx.pal, true, |ui| {
        ui.label(RichText::new("Elements Matrix (Craft Renders)").size(13.0).strong());
        ui.horizontal(|ui| {
            ui.add_space(38.0);
            let col_w = ((ui.available_width() - 12.0) / 3.0).max(68.0);
            for (title, dot) in [
                ("Animation", ctx.pal.accent_violet),
                ("Lyrics", ctx.pal.accent_amber),
                ("Motion", ctx.pal.accent_sky),
            ] {
                ui.allocate_ui(Vec2::new(col_w, 18.0), |ui| {
                    ui.horizontal(|ui| {
                        let (dr, _) = ui.allocate_exact_size(Vec2::new(8.0, 14.0), Sense::hover());
                        ui.painter().circle_filled(dr.center(), 3.0, dot);
                        ui.label(RichText::new(title).size(10.5).color(ctx.pal.text_secondary));
                    });
                });
            }
        });

        ScrollArea::vertical()
            .id_salt("studio_elements_matrix_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for row in 0..row_count {
                    let row_sec = scenes.get(row).map_or((row as f64) * 2.7, |s| s.start_sec);
                    let active_row = (ctx.state.studio.seek_seconds - row_sec).abs() < 1.4;

                    let row_resp = ui.horizontal(|ui| {
                        let (tc_rect, _) = ui.allocate_exact_size(Vec2::new(34.0, 58.0), Sense::hover());
                        if active_row {
                            let pill = Rect::from_center_size(tc_rect.center(), Vec2::new(34.0, 16.0));
                            ui.painter().rect_filled(pill, CornerRadius::same(4), ctx.pal.accent_emerald);
                            ui.painter().text(
                                pill.center(),
                                Align2::CENTER_CENTER,
                                format!("0:{row_sec:04.1}"),
                                FontId::monospace(8.5),
                                Color32::from_rgb(0x06, 0x16, 0x12),
                            );
                        } else {
                            ui.painter().text(
                                tc_rect.center(),
                                Align2::CENTER_CENTER,
                                format!("0:{row_sec:04.1}"),
                                FontId::monospace(8.5),
                                ctx.pal.text_muted,
                            );
                        }

                        let col_w = ((ui.available_width() - 12.0) / 3.0).max(68.0);
                        for (col_idx, target, badge, doc, frame) in [
                            (0usize, ViewMode::Animation, "EC", ".ecproj", outputs.v2_frames.get(row).map(String::as_str)),
                            (1usize, ViewMode::Design, "EC", "lyrics.ecproj", outputs.v3_frames.get(row).map(String::as_str)),
                            (2usize, ViewMode::Motion, "VC", "actions.ecproj", outputs.v4_frames.get(row).map(String::as_str)),
                        ] {
                            let (cell_rect, cresp) = ui.allocate_exact_size(Vec2::new(col_w, 58.0), Sense::click());
                            if cresp.clicked() {
                                ctx.state.studio.seek_seconds = row_sec;
                                ctx.push(UiAction::NavigateAuthoring { view: target, seconds: row_sec });
                            }
                            let inner = paint_filmbar_frame(
                                ui,
                                cell_rect,
                                ctx.pal,
                                FilmbarStyle {
                                    orientation: FilmbarOrientation::Vertical,
                                    selected: active_row && col_idx == 0,
                                    hovered: cresp.hovered(),
                                    timecode: None,
                                },
                            );
                            if !paint_local_image(ui, inner, frame, ctx.pal.bg_stage, 2) {
                                paint_craft_doc_card(ui, inner, ctx.pal, badge, doc, "");
                            }
                        }
                    });
                    if active_row {
                        let r = row_resp.response.rect;
                        let y = r.center().y;
                        ui.painter().line_segment(
                            [Pos2::new(r.min.x + 36.0, y), Pos2::new(r.max.x, y)],
                            Stroke::new(1.6, ctx.pal.accent_emerald),
                        );
                    }
                    ui.add_space(4.0);
                }
            });
    });
    ctx.reg.record("studio-elements-matrix", "Elements Matrix", &panel.response, None);
}

/// Bottom dock: `Review range` multi-track lanes (`Film` horizontal filmbar + `V2`, `V3`, `V4` real timeline clips & frames).
fn draw_review_range_dock(ui: &mut Ui, ctx: &mut ViewCtx<'_>, outputs: &CraftOutputs) {
    let tracks = ctx.state.cache.timeline["tracks"].as_array().cloned().unwrap_or_default();
    let total_ticks = ctx.state.cache.timeline["duration_tick"].as_i64().unwrap_or(0).max(1) as f64;

    let dock = fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Review range (FilmCraft Sequence Tracks)").size(12.0).strong());
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(format!(
                        "0:{:04.1}–0:{:04.1}",
                        ctx.state.studio.range_start,
                        ctx.state.studio.range_end.max(10.0)
                    ))
                    .monospace()
                    .size(10.0)
                    .color(ctx.pal.text_muted),
                );
            });
        });

        for (code, role_match, name, color, frames) in [
            ("V1", "storyboard", "Film / V1", ctx.pal.text_muted, &outputs.program_frames),
            ("V2", "character", "V2 Animation", ctx.pal.accent_violet, &outputs.v2_frames),
            ("V3", "lyrics", "V3 Lyrics", ctx.pal.accent_amber, &outputs.v3_frames),
            ("V4", "motion", "V4 Motion", ctx.pal.accent_sky, &outputs.v4_frames),
        ] {
            ui.horizontal(|ui| {
                ui.allocate_ui(Vec2::new(86.0, 20.0), |ui| {
                    ui.horizontal(|ui| {
                        let (dr, _) = ui.allocate_exact_size(Vec2::new(8.0, 16.0), Sense::hover());
                        ui.painter().circle_filled(dr.center(), 3.0, color);
                        ui.label(RichText::new(name).size(10.0).color(ctx.pal.text_secondary));
                    });
                });
                let bar_w = ui.available_width().max(120.0);
                let (lane_rect, lresp) = ui.allocate_exact_size(Vec2::new(bar_w, 20.0), Sense::click());
                if lresp.clicked()
                    && let Some(pos) = lresp.interact_pointer_pos()
                {
                    let t = ((pos.x - lane_rect.min.x) / lane_rect.width()).clamp(0.0, 1.0) as f64;
                    ctx.state.studio.seek_seconds = t * ctx.state.studio.range_end.max(10.0);
                }
                if code == "V1" {
                    let inner = paint_filmbar_frame(
                        ui,
                        lane_rect,
                        ctx.pal,
                        FilmbarStyle {
                            orientation: FilmbarOrientation::Horizontal,
                            selected: false,
                            hovered: lresp.hovered(),
                            timecode: None,
                        },
                    );
                    let src_frames = if frames.is_empty() { &outputs.v1_frames } else { frames };
                    if !src_frames.is_empty() {
                        let n = src_frames.len().clamp(1, 8);
                        let tile_w = (inner.width() / (n as f32)).max(24.0);
                        for (t_idx, fp) in src_frames.iter().take(n).enumerate() {
                            let tr = Rect::from_min_size(
                                Pos2::new(inner.min.x + (t_idx as f32) * tile_w, inner.min.y),
                                Vec2::new(tile_w - 1.0, inner.height()),
                            );
                            let _ = paint_local_image(ui, tr, Some(fp.as_str()), ctx.pal.bg_stage, 1);
                        }
                    }
                } else {
                    ui.painter().rect(
                        lane_rect,
                        CornerRadius::same(4),
                        ctx.pal.bg_input,
                        Stroke::new(1.0, color.gamma_multiply(0.55)),
                        StrokeKind::Inside,
                    );
                    let matched_clips: Vec<Value> = tracks
                        .iter()
                        .filter(|t| t["role"].as_str() == Some(role_match) || t["label"].as_str() == Some(code))
                        .flat_map(|t| t["clips"].as_array().cloned().unwrap_or_default())
                        .collect();
                    if !matched_clips.is_empty() {
                        for c in &matched_clips {
                            let st = c["start_tick"].as_i64().unwrap_or(0).max(0) as f64;
                            let en = c["end_tick"].as_i64().unwrap_or(0).max(0) as f64;
                            let x0 = lane_rect.min.x + ((st / total_ticks).clamp(0.0, 1.0) as f32) * bar_w;
                            let cw = (((en - st).max(1.0) / total_ticks) as f32 * bar_w).max(18.0);
                            let cr = Rect::from_min_size(Pos2::new(x0, lane_rect.min.y + 2.0), Vec2::new(cw.min(lane_rect.max.x - x0), 16.0));
                            ui.painter().rect_filled(cr, CornerRadius::same(3), color.gamma_multiply(0.32));
                            let cname = c["name"].as_str().unwrap_or(code);
                            ui.painter().text(cr.center(), Align2::CENTER_CENTER, cname, FontId::monospace(8.5), ctx.pal.text_primary);
                        }
                    } else if !frames.is_empty() {
                        let n = frames.len().clamp(1, 5);
                        for (k, fp) in frames.iter().take(n).enumerate() {
                            let kr = Rect::from_center_size(
                                Pos2::new(lane_rect.min.x + (k as f32 + 0.5) * (bar_w / (n as f32)), lane_rect.center().y),
                                Vec2::new(28.0, 15.0),
                            );
                            let _ = paint_local_image(ui, kr, Some(fp.as_str()), ctx.pal.bg_stage, 2);
                        }
                    } else {
                        ui.painter().text(
                            lane_rect.center(),
                            Align2::CENTER_CENTER,
                            format!("{code} track empty in .fcproj"),
                            FontId::monospace(8.5),
                            ctx.pal.text_muted,
                        );
                    }
                }
                ctx.reg.record(format!("studio-lane-{code}"), name, &lresp, None);
            });
        }
    });
    ctx.reg.record("studio-review-range", "Review Range", &dock.response, None);
}
