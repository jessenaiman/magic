//! `Studio` view (`#view-studio`): Preserves and polishes `FilmCraft Studio`:
//! - Header bar (`Assemble`, mint `Preview`, `Frame`, `FilmCraft`, `Options`, `Export`).
//! - Left `Film / Scenes` rail with 35mm vertical **filmbar** sprocket frames.
//! - Center 16:9 program monitor + hollow-ring scrub/zoom sliders + `At this moment` 3-up cards.
//! - Right `Elements` 3-column (`● Animation`, `● Lyrics`, `● Motion`) **filmbar** matrix with mint playhead.
//! - Bottom `Review range` multi-track filmstrip (`Film`, `Animation`, `Lyrics`, `Motion`).

use craft_ui::layout::{SplitAxis, SplitSize, split_rect};
use egui::{Align2, Color32, CornerRadius, FontId, Pos2, RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui, Vec2};
use serde_json::json;

use crate::controls::{FilmbarOrientation, FilmbarStyle, ScalarSpec, paint_filmbar_frame, pill_toggle, ring_slider_h};
use crate::state::{UiAction, ViewCtx, ViewMode};
use crate::widgets::{BtnSpec, action_button, status_pill};

pub fn show(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let outer = ui.available_rect_before_wrap();
    let root_resp = ui.allocate_rect(outer, Sense::hover());
    ctx.reg.record("view-studio", "Studio", &root_resp, None);

    ui.vertical(|ui| {
        draw_studio_header(ui, ctx);
        if ctx.state.studio.show_export_settings {
            ui.add_space(6.0);
            draw_export_drawer(ui, ctx);
        }
        ui.add_space(6.0);
        let remaining = ui.available_rect_before_wrap();
        if let Some(vert) = split_rect(remaining, SplitAxis::Vertical, SplitSize::FixedSecond(136.0), 8.0, 220.0) {
            let mut upper = ui.new_child(egui::UiBuilder::new().max_rect(vert.first).scope_id(ui.scope_id().with("studio_upper")));
            draw_upper_three_columns(&mut upper, ctx);
            let mut lower = ui.new_child(egui::UiBuilder::new().max_rect(vert.second).scope_id(ui.scope_id().with("studio_review")));
            draw_review_range_dock(&mut lower, ctx);
        } else {
            draw_upper_three_columns(ui, ctx);
        }
    });
}

fn draw_studio_header(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let ready = ctx.state.cache.film_status["export_ready"].as_bool().unwrap_or(false);
    let bar = ctx.pal.card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new("FilmCraft Studio").size(15.0).strong().color(ctx.pal.text_primary));
                ui.label(RichText::new("Master A1 + V1–V4 timeline · click Preview to refresh composite").size(10.5).color(ctx.pal.text_muted));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("film-export", "download", "Export", "Render final master film via FilmCraft")).clicked() {
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
                    BtnSpec::new("film-export-settings-toggle", "sliders", "Options", "Configure FilmCraft export settings")
                        .selected(ctx.state.studio.show_export_settings),
                )
                .clicked()
                {
                    ctx.state.studio.show_export_settings = !ctx.state.studio.show_export_settings;
                }
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("film-open", "film", "FilmCraft", "Open master .fcproj in FilmCraft")).clicked() {
                    ctx.exec("video.open", json!({}), Some("Opened FilmCraft"));
                }
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("film-frame", "camera", "Frame", "Grab program PNG frame at playhead")).clicked() {
                    let sec = ctx.state.studio.seek_seconds;
                    ctx.exec("video.frame", json!({"seconds": sec}), Some("Exported program frame"));
                }
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("film-preview", "play", "Preview", "Render Studio preview").primary()).clicked() {
                    ctx.exec("video.preview", json!({}), Some("Rendered Studio preview"));
                }
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("film-assemble", "layers", "Assemble", "Assemble FilmCraft .fcproj")).clicked() {
                    ctx.exec("video.assemble", json!({}), Some("Assembled FilmCraft sequence"));
                }
                status_pill(ui, ctx.pal, if ready { "Ready" } else { "Draft" }, if ready { ctx.pal.accent_emerald } else { ctx.pal.accent_amber });
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
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new(&id, "film", fmt, "Select export codec").selected(sel)).clicked() {
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

fn draw_upper_three_columns(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let rect = ui.available_rect_before_wrap();
    if let Some(left_split) = split_rect(rect, SplitAxis::Horizontal, SplitSize::FixedFirst(220.0), 8.0, 180.0) {
        let mut scenes_col = ui.new_child(egui::UiBuilder::new().max_rect(left_split.first).scope_id(ui.scope_id().with("studio_scenes_col")));
        draw_film_scenes_rail(&mut scenes_col, ctx);
        if let Some(right_split) = split_rect(left_split.second, SplitAxis::Horizontal, SplitSize::FixedSecond(330.0), 8.0, 240.0) {
            let mut center_col = ui.new_child(egui::UiBuilder::new().max_rect(right_split.first).scope_id(ui.scope_id().with("studio_center_col")));
            draw_center_monitor_and_moment(&mut center_col, ctx);
            let mut elements_col = ui.new_child(egui::UiBuilder::new().max_rect(right_split.second).scope_id(ui.scope_id().with("studio_elements_col")));
            draw_elements_matrix(&mut elements_col, ctx);
        } else {
            let mut center_col = ui.new_child(egui::UiBuilder::new().max_rect(left_split.second).scope_id(ui.scope_id().with("studio_center_col")));
            draw_center_monitor_and_moment(&mut center_col, ctx);
        }
    } else {
        draw_center_monitor_and_moment(ui, ctx);
    }
}

/// Left column: `Film / Scenes` cards with 35mm vertical filmbar borders.
fn draw_film_scenes_rail(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let panel = ctx.pal.card_frame().show(ui, |ui| {
        ui.label(RichText::new("Film / Scenes").size(12.5).strong());
        ui.add_space(4.0);
        ScrollArea::vertical().id_salt("studio_film_scenes_scroll").show(ui, |ui| {
            for idx in 0..4usize {
                let tc = format!("0:{:02}.0", idx * 5);
                let selected = ctx.state.studio.selected_scene == idx;
                let w = ui.available_width().max(150.0);
                let (card_rect, resp) = ui.allocate_exact_size(Vec2::new(w, 88.0), Sense::click());
                if resp.clicked() {
                    ctx.state.studio.selected_scene = idx;
                    ctx.state.studio.seek_seconds = (idx as f64) * 5.0;
                }
                let inner = paint_filmbar_frame(
                    ui,
                    card_rect,
                    ctx.pal,
                    FilmbarStyle { orientation: FilmbarOrientation::Vertical, selected, hovered: resp.hovered(), timecode: Some(&tc) },
                );
                ui.painter().text(
                    inner.center(),
                    Align2::CENTER_CENTER,
                    format!("Scene {} · Spread {:02}", idx + 1, idx + 5),
                    FontId::proportional(11.0),
                    if selected { ctx.pal.accent_emerald } else { ctx.pal.text_secondary },
                );
                ctx.reg.record(format!("studio-scene-{}", idx + 1), format!("Scene {}", idx + 1), &resp, None);
                ui.add_space(4.0);
            }
        });
    });
    ctx.reg.record("studio-scenes-rail", "Film / Scenes", &panel.response, None);
}

fn draw_center_monitor_and_moment(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ui.vertical(|ui| {
        let mon_card = ctx.pal.card_frame().show(ui, |ui| {
            let avail_w = ui.available_width().max(220.0);
            let mon_h = (avail_w * 9.0 / 16.0).clamp(130.0, 210.0);
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(avail_w, mon_h), Sense::click());
            ui.painter().rect(rect, CornerRadius::same(8), ctx.pal.bg_stage, Stroke::new(1.0, ctx.pal.border), StrokeKind::Inside);
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                format!("{} · {:04.1}s", ctx.state.project_name(), ctx.state.studio.seek_seconds),
                FontId::proportional(15.0),
                ctx.pal.text_primary,
            );
            ctx.reg.record("film-player", "Program Monitor", &resp, None);

            ui.horizontal(|ui| {
                let play_icon = if ctx.state.audio_playing { "pause" } else { "play" };
                let play_word = if ctx.state.audio_playing { "Pause" } else { "Play" };
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("film-play", play_icon, play_word, "Toggle transport").primary()).clicked() {
                    ctx.push(UiAction::ToggleAudio);
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
        ctx.pal.card_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("At this moment").size(12.0).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(format!("{:.1}s", ctx.state.studio.seek_seconds)).monospace().size(10.5).color(ctx.pal.text_muted));
                });
            });
            ui.horizontal(|ui| {
                let card_w = ((ui.available_width() - 12.0) / 3.0).max(90.0);
                for (code, title, dot_col, bg_col, target) in [
                    ("V2", "Animation", ctx.pal.accent_violet, ctx.pal.bg_stage, ViewMode::Animation),
                    ("V3", "Lyrics", ctx.pal.accent_amber, ctx.pal.stage_paper, ViewMode::Design),
                    ("V4", "Motion", ctx.pal.accent_sky, ctx.pal.stage_paper, ViewMode::Motion),
                ] {
                    ui.allocate_ui(Vec2::new(card_w, 86.0), |ui| {
                        let (r, _) = ui.allocate_exact_size(Vec2::new(card_w, 56.0), Sense::hover());
                        ui.painter().rect(r, CornerRadius::same(6), bg_col, Stroke::new(1.0, ctx.pal.border), StrokeKind::Inside);
                        let txt_col = if bg_col == ctx.pal.stage_paper { Color32::from_rgb(0x20, 0x38, 0x42) } else { ctx.pal.text_secondary };
                        ui.painter().text(r.center(), Align2::CENTER_CENTER, format!("{code} · {title}"), FontId::proportional(11.0), txt_col);
                        ui.horizontal(|ui| {
                            let (drect, _) = ui.allocate_exact_size(Vec2::new(8.0, 16.0), Sense::hover());
                            ui.painter().circle_filled(drect.center(), 3.2, dot_col);
                            ui.label(RichText::new(title).size(10.5).color(ctx.pal.text_secondary));
                            let btn_id = format!("studio-moment-edit-{code}");
                            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new(&btn_id, "pen", "Edit", "Edit layer at playhead")).clicked() {
                                ctx.push(UiAction::NavigateAuthoring { view: target, seconds: ctx.state.studio.seek_seconds });
                            }
                        });
                    });
                }
            });
        });
    });
}

/// Right column: `Elements` 3-column (`Animation`, `Lyrics`, `Motion`) filmbar matrix + mint playhead.
fn draw_elements_matrix(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let panel = ctx.pal.card_frame().show(ui, |ui| {
        ui.label(RichText::new("Elements").size(12.5).strong());
        ui.horizontal(|ui| {
            let col_w = ((ui.available_width() - 12.0) / 3.0).max(72.0);
            for (title, dot) in [("Animation", ctx.pal.accent_violet), ("Lyrics", ctx.pal.accent_amber), ("Motion", ctx.pal.accent_sky)] {
                ui.allocate_ui(Vec2::new(col_w, 18.0), |ui| {
                    ui.horizontal(|ui| {
                        let (dr, _) = ui.allocate_exact_size(Vec2::new(8.0, 14.0), Sense::hover());
                        ui.painter().circle_filled(dr.center(), 3.0, dot);
                        ui.label(RichText::new(title).size(10.5).color(ctx.pal.text_secondary));
                    });
                });
            }
        });

        ScrollArea::vertical().id_salt("studio_elements_matrix_scroll").show(ui, |ui| {
            for row in 0..4usize {
                let row_sec = (row as f64) * 2.5;
                let active_row = (ctx.state.studio.seek_seconds - row_sec).abs() < 1.3;
                let row_resp = ui.horizontal(|ui| {
                    let col_w = ((ui.available_width() - 12.0) / 3.0).max(72.0);
                    for (col_idx, label, target) in
                        [(0usize, "V2 Rig", ViewMode::Animation), (1usize, "V3 Words", ViewMode::Design), (2usize, "V4 Action", ViewMode::Motion)]
                    {
                        let (cell_rect, cresp) = ui.allocate_exact_size(Vec2::new(col_w, 54.0), Sense::click());
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
                        if col_idx > 0 {
                            ui.painter().rect_filled(inner, CornerRadius::same(3), ctx.pal.stage_paper);
                            ui.painter().text(inner.center(), Align2::CENTER_CENTER, label, FontId::proportional(10.0), Color32::from_rgb(0x22, 0x35, 0x3c));
                        } else {
                            ui.painter().text(inner.center(), Align2::CENTER_CENTER, label, FontId::proportional(10.0), ctx.pal.text_secondary);
                        }
                    }
                });
                if active_row {
                    let r = row_resp.response.rect;
                    let y = r.center().y;
                    ui.painter().line_segment([Pos2::new(r.min.x, y), Pos2::new(r.max.x, y)], Stroke::new(1.5, ctx.pal.accent_emerald));
                }
                ui.add_space(3.0);
            }
        });
    });
    ctx.reg.record("studio-elements-matrix", "Elements Matrix", &panel.response, None);
}

/// Bottom dock: `Review range` multi-track lanes (`Film` horizontal filmbar + `Animation`, `Lyrics`, `Motion`).
fn draw_review_range_dock(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let dock = ctx.pal.card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Review range").size(12.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("{:.1}s – {:.1}s", ctx.state.studio.range_start, ctx.state.studio.range_end.max(10.0)))
                        .monospace()
                        .size(10.0)
                        .color(ctx.pal.text_muted),
                );
            });
        });

        for (code, name, color, is_filmbar) in [
            ("V1", "Film", ctx.pal.accent_emerald, true),
            ("V2", "Animation", ctx.pal.accent_violet, false),
            ("V3", "Lyrics", ctx.pal.accent_amber, false),
            ("V4", "Motion", ctx.pal.accent_sky, false),
        ] {
            ui.horizontal(|ui| {
                ui.allocate_ui(Vec2::new(78.0, 20.0), |ui| {
                    ui.horizontal(|ui| {
                        let (dr, _) = ui.allocate_exact_size(Vec2::new(8.0, 16.0), Sense::hover());
                        ui.painter().circle_filled(dr.center(), 3.0, color);
                        ui.label(RichText::new(name).size(10.5).color(ctx.pal.text_secondary));
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
                if is_filmbar {
                    let _ = paint_filmbar_frame(
                        ui,
                        lane_rect,
                        ctx.pal,
                        FilmbarStyle { orientation: FilmbarOrientation::Horizontal, selected: true, hovered: lresp.hovered(), timecode: None },
                    );
                } else {
                    ui.painter().rect(
                        lane_rect,
                        CornerRadius::same(4),
                        color.gamma_multiply(0.26),
                        Stroke::new(1.0, color.gamma_multiply(0.75)),
                        StrokeKind::Inside,
                    );
                }
                ctx.reg.record(format!("studio-lane-{code}"), name, &lresp, None);
            });
        }
    });
    ctx.reg.record("studio-review-range", "Review Range", &dock.response, None);
}
