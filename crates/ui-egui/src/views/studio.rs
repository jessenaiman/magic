//! `Studio` view (`#view-studio`): Craft-project-native `FilmCraft Studio` (`film/<song>.fcproj`)
//! matching `07-studio-filmcraft-assembly.jpg`, plus the 7-mode `Sheet 08` Zone E Finishing Deck
//! (`studio_finishing.rs` & `studio_audio_titles.rs`: `[Matrix] [Edit] [Color] [FX] [Audio] [Titles] [Export]`).

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
use crate::timeline::{CraftTimelineLane, CraftTimelineSpan, draw_craft_timeline};
use crate::views::studio_audio_titles::{draw_audio_tab, draw_export_tab, draw_titles_tab};
use crate::views::studio_finishing::{
    count_offline_clips, draw_color_tab, draw_edit_tab, draw_fx_tab, draw_graded_scopes_strip,
    draw_program_trim_and_three_point_bar, draw_studio_mode_tabs,
};
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
    selected: bool,
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
    if ctx.state.show_bottom_dock
        && let Some(vert) = split_rect(remaining, SplitAxis::Vertical, SplitSize::FixedSecond(148.0), 8.0, 220.0)
    {
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

fn gold_outline_button(ui: &mut Ui, ctx: &mut ViewCtx<'_>, id: &str, label: &str, tooltip: &str) -> bool {
    let font = FontId::proportional(12.0);
    let galley = ui.painter().layout_no_wrap(label.to_owned(), font.clone(), ctx.pal.accent_amber);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(galley.size().x + 22.0, 26.0), Sense::click());
    let fill = if resp.hovered() {
        Color32::from_rgba_unmultiplied(0x66, 0x4b, 0x1d, 190)
    } else {
        Color32::from_rgba_unmultiplied(0x4c, 0x38, 0x16, 165)
    };
    ui.painter().rect(rect, CornerRadius::same(6), fill, Stroke::new(1.1, ctx.pal.accent_amber), StrokeKind::Inside);
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, label, font, ctx.pal.accent_amber);
    let resp = resp.on_hover_text(tooltip);
    ctx.reg.record(id, label, &resp, None);
    resp.clicked()
}

fn draw_studio_header(ui: &mut Ui, ctx: &mut ViewCtx<'_>, outputs: &CraftOutputs) {
    let ready = ctx.state.cache.film_status["export_ready"].as_bool().unwrap_or(false);
    let fc_short = outputs.fcproj_path.rsplit('/').next().unwrap_or("Song.fcproj");
    let offline_n = count_offline_clips(&ctx.state.cache.timeline);
    let bar = ctx.pal.card_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("FilmCraft Studio — film/{fc_short}")).size(14.5).strong().color(ctx.pal.text_primary));
            status_pill(
                ui,
                ctx.pal,
                if ready { "Export Ready" } else if outputs.fcproj_exists { ".fcproj Live" } else { "Unassembled" },
                if ready { ctx.pal.accent_emerald } else { ctx.pal.accent_amber },
            );
            if offline_n > 0 {
                status_pill(ui, ctx.pal, &format!("{offline_n} Offline"), ctx.pal.danger);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
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
                if gold_outline_button(ui, ctx, "film-open", "Open in FilmCraft", "Open master .fcproj in FilmCraft") {
                    ctx.exec("video.open", json!({}), Some("Opened .fcproj in FilmCraft"));
                }
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("film-export", "upload", "Export Film", "Render final master film via FilmCraft")).clicked() {
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
                    ctx.exec("video.export", payload, Some("Exported master film"));
                }
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("film-frame", "Grab Frame", "Render program PNG frame from .fcproj at playhead")).clicked() {
                    let sec = ctx.state.studio.seek_seconds;
                    ctx.exec("video.frame", json!({"seconds": sec}), Some("Rendered program frame from .fcproj"));
                }
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("film-preview", "play", "Preview", "Render half-scale FilmCraft preview").primary()).clicked() {
                    ctx.exec("video.preview", json!({}), Some("Rendered FilmCraft preview"));
                }
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("film-assemble", "Assemble", "Assemble FilmCraft .fcproj from V1-V4 Craft outputs")).clicked() {
                    ctx.exec("video.assemble", json!({}), Some("Assembled FilmCraft .fcproj sequence"));
                }
            });
        });
    });
    ctx.reg.record("film-head", "Studio Header", &bar.response, None);
}

fn draw_export_drawer(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let drawer = ctx.pal.card_frame().show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            for fmt in ["h264", "hevc", "prores", "dnxhr", "av1", "vp9", "exr", "webm", "gif", "wav", "flac"] {
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

fn draw_upper_three_columns(ui: &mut Ui, ctx: &mut ViewCtx<'_>, scenes: &[StudioSceneItem], outputs: &CraftOutputs) {
    let rect = ui.available_rect_before_wrap();
    let after_left = if ctx.state.show_nav_rail
        && let Some(left_split) = split_rect(rect, SplitAxis::Horizontal, SplitSize::FixedFirst(236.0), 8.0, 180.0)
    {
        let mut scenes_col = child_pane(ui, left_split.first, "studio_scenes_col");
        draw_film_scenes_rail(&mut scenes_col, ctx, scenes, outputs);
        left_split.second
    } else {
        rect
    };

    if ctx.state.show_right_inspector
        && let Some(right_split) = split_rect(after_left, SplitAxis::Horizontal, SplitSize::FixedSecond(364.0), 8.0, 240.0)
    {
        let mut center_col = child_pane(ui, right_split.first, "studio_center_col");
        draw_center_monitor_and_moment(&mut center_col, ctx, scenes, outputs);
        let mut elements_col = child_pane(ui, right_split.second, "studio_elements_col");
        draw_right_finishing_deck(&mut elements_col, ctx, scenes, outputs);
    } else {
        let mut center_col = child_pane(ui, after_left, "studio_center_col");
        draw_center_monitor_and_moment(&mut center_col, ctx, scenes, outputs);
    }
    let _ = ui.allocate_rect(rect, Sense::hover());
}

fn draw_film_scenes_rail(ui: &mut Ui, ctx: &mut ViewCtx<'_>, scenes: &[StudioSceneItem], outputs: &CraftOutputs) {
    let panel = fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Film / Scenes").size(13.0).strong());
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("studio-rail-csv", "CSV", "Export sequence markers CSV (T35)")).clicked() {
                    ctx.exec("video.markers", json!({"action": "export_csv"}), Some("Exported sequence markers CSV"));
                }
                let prox = ctx.state.studio.proxies_enabled;
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("studio-rail-proxy", "Proxy", "Toggle proxies (T35)").selected(prox)).clicked() {
                    ctx.state.studio.proxies_enabled = !prox;
                    ctx.exec("video.finish", json!({"op": "toggle_proxies", "enabled": !prox}), Some("Toggled FilmCraft proxy mode"));
                }
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("studio-rail-relink", "link", "Relink offline clips (T08)")).clicked() {
                    ctx.exec("bridge.preflight", json!({"action": "relink"}), Some("Checked & relinked FilmCraft media"));
                }
            });
        });
        ui.add_space(4.0);
        ScrollArea::vertical()
            .id_salt("studio_film_scenes_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if scenes.is_empty() {
                    let w = ui.available_width().max(150.0);
                    let (erect, _) = ui.allocate_exact_size(Vec2::new(w, 118.0), Sense::hover());
                    paint_craft_doc_card(ui, erect, ctx.pal, "FC", outputs.fcproj_path.rsplit('/').next().unwrap_or("Song.fcproj"), "Assemble FilmCraft timeline or add DesignCraft scenes");
                    return;
                }
                for (idx, sc) in scenes.iter().enumerate() {
                    let selected = ctx.state.studio.selected_scene == idx;
                    let w = ui.available_width().max(150.0);
                    let (card_rect, resp) = ui.allocate_exact_size(Vec2::new(w, 110.0), Sense::click());
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
        let mon_h = (avail_w * 9.0 / 16.0).clamp(128.0, 208.0);
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(avail_w, mon_h), Sense::click());
        if !paint_local_image(ui, rect, program_img, Color32::BLACK, 8) {
            paint_craft_doc_card(
                ui,
                rect,
                ctx.pal,
                "FC",
                outputs.fcproj_path.rsplit('/').next().unwrap_or("Song.fcproj"),
                "Click Assemble then Grab Frame to render FilmCraft program output",
            );
        }
        ui.painter().rect_stroke(rect, CornerRadius::same(8), Stroke::new(1.0, ctx.pal.border), StrokeKind::Inside);
        ctx.reg.record("film-player", "Program Monitor", &resp, None);

        ui.horizontal(|ui| {
            let play_icon = if ctx.state.audio_playing { "pause" } else { "play" };
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("film-play", play_icon, "Toggle transport").primary()).clicked() {
                ctx.push(UiAction::ToggleAudio);
            }
            let dur = ctx.state.audio_duration_sec.max(30.0);
            let scrub_w = (ui.available_width() - 90.0).max(100.0);
            ui.allocate_ui_with_layout(Vec2::new(scrub_w, 30.0), Layout::top_down(Align::Min), |ui| {
                ring_slider_h(ui, ctx.reg, ctx.pal, ScalarSpec::new("film-seek", "Playhead", 0.0..=dur, "s", ctx.pal.accent_emerald), &mut ctx.state.studio.seek_seconds);
            });
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("film-add-marker", "pin", "Marker", "Add timeline marker")).clicked() {
                ctx.exec("video.markers", json!({"action": "seed"}), Some("Synced timeline markers"));
            }
        });
        draw_program_trim_and_three_point_bar(ui, ctx);
        draw_graded_scopes_strip(ui, ctx);
    });
    ctx.reg.record("film-status", "Program Card", &mon_card.response, None);

    ui.add_space(6.0);
    fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("At this moment").size(13.0).strong());
            ui.label(RichText::new("(Craft Project Outputs)").size(12.0).color(ctx.pal.text_muted));
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let card_w = ((ui.available_width() - 12.0) / 3.0).max(90.0);
            draw_moment_layer_card(ui, ctx, MomentCardSpec {
                card_w,
                code: "V2",
                title: "V1/V2 Animation · .pcraft/.ecproj",
                craft_badge: "EC",
                craft_doc: ".pcraft / .ecproj",
                frame: frame_at_index(&outputs.v2_frames, step_idx),
                selected: true,
                target: ViewMode::Animation,
            });
            draw_moment_layer_card(ui, ctx, MomentCardSpec {
                card_w,
                code: "V3",
                title: "V3 Lyrics · lettering/lyrics.ecproj",
                craft_badge: "EC",
                craft_doc: "lettering/lyrics.ecproj",
                frame: frame_at_index(&outputs.v3_frames, step_idx),
                selected: false,
                target: ViewMode::Design,
            });
            draw_moment_layer_card(ui, ctx, MomentCardSpec {
                card_w,
                code: "V4",
                title: "V4 Motion · motion/actions.ecproj",
                craft_badge: "VCRAFT",
                craft_doc: "motion/actions.ecproj",
                frame: frame_at_index(&outputs.v4_frames, step_idx),
                selected: false,
                target: ViewMode::Motion,
            });
        });
    });
}

fn draw_moment_layer_card(ui: &mut Ui, ctx: &mut ViewCtx<'_>, spec: MomentCardSpec<'_>) {
    ui.allocate_ui_with_layout(Vec2::new(spec.card_w, 110.0), Layout::top_down(Align::Min), |ui| {
        let (card_rect, resp) = ui.allocate_exact_size(Vec2::new(spec.card_w, 106.0), Sense::click());
        let border = if spec.selected { Stroke::new(1.5, ctx.pal.accent_emerald) } else { Stroke::new(1.0, ctx.pal.border) };
        ui.painter().rect(card_rect, CornerRadius::same(7), ctx.pal.bg_input, border, StrokeKind::Inside);
        let img_rect = Rect::from_min_max(Pos2::new(card_rect.min.x + 3.0, card_rect.min.y + 3.0), Pos2::new(card_rect.max.x - 3.0, card_rect.max.y - 24.0));
        if !paint_local_image(ui, img_rect, spec.frame, ctx.pal.bg_stage, 5) {
            paint_craft_doc_card(ui, img_rect, ctx.pal, spec.craft_badge, spec.craft_doc, "");
        }
        let tag_r = Rect::from_min_size(Pos2::new(img_rect.min.x + 5.0, img_rect.min.y + 5.0), Vec2::new(30.0, 15.0));
        ui.painter().rect_filled(tag_r, CornerRadius::same(4), Color32::from_rgba_unmultiplied(10, 14, 21, 210));
        ui.painter().text(tag_r.center(), Align2::CENTER_CENTER, "16:9", FontId::monospace(8.5), ctx.pal.text_primary);
        ui.painter().text(Pos2::new(card_rect.min.x + 7.0, card_rect.max.y - 11.5), Align2::LEFT_CENTER, spec.title, FontId::proportional(10.0), ctx.pal.text_secondary);
        if resp.clicked() {
            ctx.push(UiAction::NavigateAuthoring { view: spec.target, seconds: ctx.state.studio.seek_seconds });
        }
        ctx.reg.record(format!("studio-moment-edit-{}", spec.code), spec.title, &resp, None);
    });
}

fn draw_right_finishing_deck(ui: &mut Ui, ctx: &mut ViewCtx<'_>, scenes: &[StudioSceneItem], outputs: &CraftOutputs) {
    let panel = fill_card(ui, ctx.pal, true, |ui| {
        draw_studio_mode_tabs(ui, ctx);
        ui.separator();

        if ctx.state.studio.inspector_tab == "matrix" {
            draw_elements_matrix_body(ui, ctx, scenes, outputs);
        } else {
            ScrollArea::vertical()
                .id_salt("studio_finishing_deck_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| match ctx.state.studio.inspector_tab.as_str() {
                    "edit" => draw_edit_tab(ui, ctx),
                    "color" => draw_color_tab(ui, ctx),
                    "fx" => draw_fx_tab(ui, ctx),
                    "audio" => draw_audio_tab(ui, ctx),
                    "titles" => draw_titles_tab(ui, ctx),
                    "export" => draw_export_tab(ui, ctx),
                    _ => {}
                });
        }
    });
    ctx.reg.record("studio-elements-matrix", "Elements Matrix", &panel.response, None);
}

fn draw_elements_matrix_body(ui: &mut Ui, ctx: &mut ViewCtx<'_>, scenes: &[StudioSceneItem], outputs: &CraftOutputs) {
    let row_count = scenes
        .len()
        .max(outputs.v2_frames.len())
        .max(outputs.v3_frames.len())
        .max(outputs.v4_frames.len())
        .clamp(1, 6);

    ui.horizontal(|ui| {
        ui.label(RichText::new("Elements Matrix").size(13.0).strong());
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("studio-matrix-bake", "Bake V3", "Bake linked lettering/lyrics.ecproj to ProRes 4444 .mov (T01)")).clicked() {
                ctx.exec("video.bake_link", json!({"project": "lettering/lyrics.ecproj", "track": "V3"}), Some("Baked linked V3 .ecproj to ProRes 4444"));
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("studio-matrix-sync", "refresh", "Sync Comps", "Re-render dirty linked .ecproj comps (T01)")).clicked() {
                ctx.exec("video.sync_links", json!({}), Some("Synced linked EffectCraft compositions"));
            }
        });
    });
    ui.horizontal(|ui| {
        ui.add_space(38.0);
        let col_w = ((ui.available_width() - 12.0) / 3.0).max(68.0);
        for title in ["Animation", "Lyrics", "Motion"] {
            ui.allocate_ui_with_layout(Vec2::new(col_w, 18.0), Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                ui.label(RichText::new(title).size(10.5).color(ctx.pal.text_secondary));
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

                ui.horizontal(|ui| {
                    let (tc_rect, _) = ui.allocate_exact_size(Vec2::new(34.0, 62.0), Sense::hover());
                    ui.painter().text(
                        tc_rect.center(),
                        Align2::CENTER_CENTER,
                        format!("0:{row_sec:04.1}"),
                        FontId::monospace(8.5),
                        if active_row { ctx.pal.accent_emerald } else { ctx.pal.text_muted },
                    );

                    let col_w = ((ui.available_width() - 12.0) / 3.0).max(68.0);
                    for (col_idx, target, badge, doc, frame) in [
                        (0usize, ViewMode::Animation, "EC", ".ecproj", outputs.v2_frames.get(row).map(String::as_str)),
                        (1usize, ViewMode::Design, "EC", "lyrics.ecproj", outputs.v3_frames.get(row).map(String::as_str)),
                        (2usize, ViewMode::Motion, "VCRAFT", "actions.ecproj", outputs.v4_frames.get(row).map(String::as_str)),
                    ] {
                        let (cell_rect, cresp) = ui.allocate_exact_size(Vec2::new(col_w, 62.0), Sense::click());
                        if cresp.clicked() {
                            ctx.state.studio.seek_seconds = row_sec;
                            ctx.push(UiAction::NavigateAuthoring { view: target, seconds: row_sec });
                        }
                        let inner = paint_filmbar_frame(
                            ui,
                            cell_rect,
                            ctx.pal,
                            FilmbarStyle {
                                orientation: FilmbarOrientation::Horizontal,
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
                ui.add_space(4.0);
            }
        });
}

fn draw_review_range_dock(ui: &mut Ui, ctx: &mut ViewCtx<'_>, outputs: &CraftOutputs) {
    let tracks = ctx.state.cache.timeline["tracks"].as_array().cloned().unwrap_or_default();
    let total_ticks = ctx.state.cache.timeline["duration_tick"].as_i64().unwrap_or(0).max(1) as f64;
    let total_dur = ctx.state.studio.range_end.max(ctx.state.audio_duration_sec).max(10.0);

    let spans_for = |role_match: &str, code: &str| -> Vec<CraftTimelineSpan> {
        tracks
            .iter()
            .filter(|t| t["role"].as_str() == Some(role_match) || t["label"].as_str() == Some(code))
            .flat_map(|t| t["clips"].as_array().cloned().unwrap_or_default())
            .map(|c| {
                let st_frac = (c["start_tick"].as_i64().unwrap_or(0).max(0) as f64) / total_ticks;
                let en_frac = (c["end_tick"].as_i64().unwrap_or(0).max(1) as f64) / total_ticks;
                CraftTimelineSpan {
                    start_sec: st_frac * total_dur,
                    end_sec: (en_frac * total_dur).max(st_frac * total_dur + 0.25),
                    label: c["name"].as_str().unwrap_or(code).to_owned(),
                }
            })
            .collect()
    };

    let v1_src = if outputs.program_frames.is_empty() { outputs.v1_frames.as_slice() } else { outputs.program_frames.as_slice() };
    let dirty_v2 = ctx.state.dirty_tracks.contains("V2");
    let dirty_v3 = ctx.state.dirty_tracks.contains("V3");
    let dirty_v4 = ctx.state.dirty_tracks.contains("V4");

    let lanes = [
        CraftTimelineLane { code: "V1", title: "Film", color: ctx.pal.text_muted, filmbar: true, dirty: false, spans: Vec::new(), frames: v1_src, empty_hint: "V1 scenes" },
        CraftTimelineLane { code: "V2", title: "V1/V2 Animation", color: ctx.pal.accent_sky, filmbar: false, dirty: dirty_v2, spans: spans_for("character", "V2"), frames: &outputs.v2_frames, empty_hint: "V1/V2 Animation — click Link V2 or Assemble" },
        CraftTimelineLane { code: "V3", title: "V3 Lyrics", color: ctx.pal.accent_amber, filmbar: false, dirty: dirty_v3, spans: spans_for("lyrics", "V3"), frames: &outputs.v3_frames, empty_hint: "V3 Lyrics — click Publish V3 in Lyrics" },
        CraftTimelineLane { code: "V4", title: "V4 Motion", color: ctx.pal.accent_emerald, filmbar: false, dirty: dirty_v4, spans: spans_for("motion", "V4"), frames: &outputs.v4_frames, empty_hint: "V4 Motion — click Publish V4 in Motion" },
    ];

    let dock = fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Review range").size(12.5).strong());
            ui.label(RichText::new("· 254,016,000,000 ticks/s exact frame math").monospace().size(9.5).color(ctx.pal.text_muted));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("0:{:04.1}–0:{total_dur:04.1}", ctx.state.studio.range_start))
                        .monospace()
                        .size(10.0)
                        .color(ctx.pal.text_muted),
                );
            });
        });
        let mut seek = ctx.state.studio.seek_seconds;
        if draw_craft_timeline(ui, ctx, "studio", total_dur, &mut seek, &lanes) {
            ctx.state.studio.seek_seconds = seek;
        }
    });
    ctx.reg.record("studio-review-range", "Review Range", &dock.response, None);
}
