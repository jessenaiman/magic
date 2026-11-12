<<<<<<< HEAD
//! Workstation shell chrome (`#menu-bar` with MailCraft quick Undo/Redo + History/Recovery +
//! `Search commands` + `Copy Diagnostics` + `Report a Problem` + `Options`, `#app-bar` stadium
//! capsule matching `00`–`07`, `#stage-assets` contextual shelf, non-modal info banner, and
//! `#app-footer` readiness rail + 3 layout-pane toggles).
=======
//! Workstation shell chrome (`#menu-bar`, `#app-bar` stadium capsule, `#app-footer` readiness
//! rail + `craft_ui::docking` panel toggles, and `CentralPanel` hosting `DockArea`).
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)

use craft_keymap::Keymap;
use egui::{Align, Align2, Color32, CornerRadius, FontId, Layout, Pos2, Rect, RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui, Vec2};
use serde_json::{Value, json};

<<<<<<< HEAD
use crate::controls::{craft_swatch_button, paint_local_image};
use crate::editorial::draw_persisted_style_controls;
use crate::menus::{build_menu_bar, draw_in_window_bar, translate_menu_command};
use crate::options_modal::{draw_command_palette_popup, draw_fidget_stager_window};
use crate::shell_bridge::{draw_footer_session_pills, draw_history_recovery_window, draw_modal, draw_toast_if_active};
use crate::state::{ActiveModal, PickerTarget, UiAction, ViewCtx, ViewMode};
use crate::theme::paint_aurora_wave;
use crate::views;
use crate::widgets::{BtnSpec, action_button, companion_button, mode_tab, project_pill_button, status_pill, text_input};

const CAPSULE_TABS: [ViewMode; 5] = [ViewMode::Storyboard, ViewMode::Animation, ViewMode::Design, ViewMode::Motion, ViewMode::Studio];

pub fn draw_workstation(ui: &mut Ui, ctx: &mut ViewCtx<'_>, keymap: &Keymap, show_in_window_menus: bool, is_enabled: impl Fn(&str) -> bool) {
    let modal_open = ctx.state.active_modal.is_some();
=======
use crate::controls::{craft_swatch_button, fill_card, paint_local_image};
use crate::docking::{DockFrameOutcome, PanelId, show_dock_area};
use crate::editorial::draw_persisted_style_controls;
use crate::menus::{build_menu_bar, draw_in_window_bar, translate_menu_command};
use crate::modals::{draw_footer_session_pills, draw_modal, draw_toast_if_active};
use crate::options_modal::draw_command_palette_popup;
use crate::state::{ActiveModal, PickerTarget, UiAction, ViewCtx, ViewMode};
use crate::theme::paint_aurora_wave;
use crate::views;
use crate::widgets::{BtnSpec, ModeTabSpec, action_button, companion_button, mode_tab, project_pill_button, status_pill, text_input};

const CAPSULE_TABS: [ViewMode; 5] = [ViewMode::Storyboard, ViewMode::Animation, ViewMode::Design, ViewMode::Motion, ViewMode::Studio];

pub fn draw_workstation(
    ui: &mut Ui,
    ctx: &mut ViewCtx<'_>,
    keymap: &Keymap,
    show_in_window_menus: bool,
    modal_open: bool,
    is_enabled: impl Fn(&str) -> bool,
) -> DockFrameOutcome {
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    let mut cmd_search_rect = Rect::NOTHING;

    if show_in_window_menus {
        let bar = build_menu_bar(ctx.state, keymap, &is_enabled);
        egui::Panel::top("magic_menu_bar")
            .frame(egui::Frame::new().fill(ctx.pal.bg_stage).inner_margin(egui::Margin::symmetric(8, 2)).stroke(Stroke::new(1.0, ctx.pal.border_subtle)))
            .show(ui, |ui| {
                cmd_search_rect = draw_top_menu_and_utilities(ui, ctx, &bar, modal_open);
            });
    }

    let top_resp = egui::Panel::top("magic_app_bar")
        .frame(egui::Frame::new().fill(ctx.pal.bg_stage).inner_margin(egui::Margin::symmetric(10, 5)).stroke(Stroke::new(1.0, ctx.pal.border)))
        .show(ui, |ui| {
            craft_window::title_bar::empty_area(ui, ui.max_rect());
            draw_app_bar(ui, ctx);
        });
    ctx.reg.record("app-bar", "App Bar", &top_resp.response, None);
    ctx.reg.record_panel("app-bar", top_resp.response.rect);

<<<<<<< HEAD
    let active_view = ctx.state.effective_view();
    let stage_uses_asset_bar = matches!(active_view, ViewMode::Animation);
    if ctx.state.project_path.is_some()
        && active_view != ViewMode::Home
        && active_view != ViewMode::Project
        && ((stage_uses_asset_bar && ctx.state.show_nav_rail) || ctx.state.assets_drawer_open || ctx.state.assets_drawer_pinned)
    {
        let assets_resp = egui::Panel::top("magic_stage_assets")
            .frame(egui::Frame::new().fill(ctx.pal.bg_panel).inner_margin(egui::Margin::symmetric(10, 4)).stroke(Stroke::new(1.0, ctx.pal.border_subtle)))
            .show(ui, |ui| {
                draw_stage_assets_drawer(ui, ctx);
            });
        ctx.reg.record("stage-assets", "Stage Assets", &assets_resp.response, None);
        ctx.reg.record_panel("stage-assets", assets_resp.response.rect);
    }

=======
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    let foot_resp = egui::Panel::bottom("magic_app_footer")
        .frame(egui::Frame::new().fill(ctx.pal.bg_stage).inner_margin(egui::Margin::symmetric(10, 4)).stroke(Stroke::new(1.0, ctx.pal.border)))
        .show(ui, |ui| {
            draw_footer(ui, ctx);
        });
    ctx.reg.record("app-footer", "App Footer", &foot_resp.response, None);
    ctx.reg.record_panel("app-footer", foot_resp.response.rect);

    if !ctx.state.dirty_tracks.is_empty() && !ctx.state.update_banner_dismissed {
        egui::Panel::bottom("magic_sync_banner")
            .frame(egui::Frame::new().fill(ctx.pal.bg_panel).inner_margin(egui::Margin::symmetric(12, 4)).stroke(Stroke::new(1.0, ctx.pal.border)))
            .show(ui, |ui| {
                draw_sync_info_banner(ui, ctx);
            });
    }

<<<<<<< HEAD
    let central_resp = egui::CentralPanel::default().frame(egui::Frame::new().fill(ctx.pal.bg_canvas).inner_margin(egui::Margin::same(10))).show(ui, |ui| {
        paint_aurora_wave(ui, ui.max_rect(), ctx.pal);
        match active_view {
            ViewMode::Home => views::home::show(ui, ctx),
            ViewMode::Project => views::project::show(ui, ctx),
            ViewMode::Storyboard => views::storyboard::show(ui, ctx),
            ViewMode::Animation => views::animation::show(ui, ctx),
            ViewMode::Design => views::lyrics::show(ui, ctx),
            ViewMode::Motion => views::motion::show(ui, ctx),
            ViewMode::Studio => views::studio::show(ui, ctx),
=======
    let active_view = ctx.state.effective_view();
    let mut dock_outcome = DockFrameOutcome::default();

    let central_resp = egui::CentralPanel::default().frame(egui::Frame::new().fill(ctx.pal.bg_canvas).inner_margin(egui::Margin::same(8))).show(ui, |ui| {
        paint_aurora_wave(ui, ui.max_rect(), ctx.pal);
        if active_view == ViewMode::Home {
            views::home::show(ui, ctx);
        } else {
            if !ctx.state.ribbon_collapsed {
                draw_active_stage_header(ui, ctx, active_view);
            }
            dock_outcome = show_dock_area(ui, ctx, modal_open);
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
        }
    });
    let (root_id, root_lbl) = active_view.root_automation_id();
    ctx.reg.record(root_id, root_lbl, &central_resp.response, None);
    ctx.reg.record_panel("central", central_resp.response.rect);

    if !ctx.state.command_search.trim().is_empty() {
        draw_command_palette_popup(ui.ctx(), ctx, cmd_search_rect);
    }
<<<<<<< HEAD
    draw_history_recovery_window(ui.ctx(), ctx);
    if ctx.state.fidget_open {
        draw_fidget_stager_window(ui.ctx(), ctx);
    }
=======
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    if let Some(modal) = ctx.state.active_modal {
        draw_modal(ui.ctx(), ctx, modal);
    }
    draw_toast_if_active(ui.ctx(), ctx);
<<<<<<< HEAD
=======

    dock_outcome
}

fn draw_active_stage_header(ui: &mut Ui, ctx: &mut ViewCtx<'_>, active_view: ViewMode) {
    match active_view {
        ViewMode::Storyboard => {
            views::storyboard::draw_stage_header(ui, ctx);
            ui.add_space(5.0);
        }
        ViewMode::Animation => {
            views::animation::draw_stage_header(ui, ctx);
            ui.add_space(5.0);
        }
        ViewMode::Design => {
            views::lyrics::draw_stage_header(ui, ctx);
            ui.add_space(5.0);
        }
        ViewMode::Motion => {
            views::motion::draw_stage_header(ui, ctx);
            ui.add_space(5.0);
        }
        ViewMode::Studio => {
            views::studio::draw_stage_header(ui, ctx);
            ui.add_space(5.0);
        }
        ViewMode::Project | ViewMode::Home => {}
    }
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
}

fn draw_top_menu_and_utilities(ui: &mut Ui, ctx: &mut ViewCtx<'_>, bar: &craft_menu::MenuBar, modal_open: bool) -> Rect {
    let mut search_rect = Rect::NOTHING;
    let wide = ui.available_width() >= 1120.0;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("quick-undo", "undo", "Undo (Ctrl+Z)")).clicked() {
            ctx.exec("edit.undo", json!({}), None);
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("quick-redo", "redo", "Redo (Ctrl+Shift+Z)")).clicked() {
            ctx.exec("edit.redo", json!({}), None);
        }
<<<<<<< HEAD
        let hist_spec = if wide {
            BtnSpec::new("quick-history", "refresh", "History", "Branching Undo History & 1s Crash-Journal Recovery (T09)")
        } else {
            BtnSpec::icon_only("quick-history", "refresh", "Branching Undo History & 1s Crash-Journal Recovery (T09)")
        };
        if action_button(ui, ctx.reg, ctx.pal, hist_spec.selected(ctx.state.history_popup_open)).clicked() {
            ctx.state.history_popup_open = !ctx.state.history_popup_open;
            if ctx.state.history_popup_open {}
=======
        let hist_open = ctx.state.docking.is_panel_visible(PanelId::History);
        let hist_spec = BtnSpec::icon_only("quick-history", "refresh", "Toggle Undo & AutoSave Journal panel");
        if action_button(ui, ctx.reg, ctx.pal, hist_spec.selected(hist_open)).clicked() {
            ctx.state.docking.toggle_panel(PanelId::History);
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
        }
        let clicked = draw_in_window_bar(ui, bar, modal_open, ctx.reg);
        for cmd in clicked {
            if let Some(act) = translate_menu_command(&cmd, ctx.state) {
                ctx.push(act);
            }
        }

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("menu-help-btn", "help", "About & Suite Doctor")).clicked() {
                ctx.push(UiAction::OpenDoctor);
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("menu-options-btn", "settings", "Options (Interface Style & Craft AI)")).clicked() {
                ctx.push(UiAction::OpenPreferences);
            }
            draw_persisted_style_controls(ui, ctx, wide);
<<<<<<< HEAD
            let diag_spec = if wide {
                BtnSpec::new("menu-copy-diagnostics", "copy", "Copy Diagnostics", "Copy sanitized system & Craft diagnostics to clipboard")
            } else {
                BtnSpec::icon_only("menu-copy-diagnostics", "copy", "Copy Diagnostics")
            };
            if action_button(ui, ctx.reg, ctx.pal, diag_spec).clicked() {
                ctx.push(UiAction::CopyDiagnostics);
            }
            let rep_spec = if wide {
                BtnSpec::new("menu-report-problem", "chat", "Report a Problem", "Open sanitized bug report dialog")
            } else {
                BtnSpec::icon_only("menu-report-problem", "chat", "Report a Problem")
            };
=======
            let diag_spec = BtnSpec::icon_only("menu-copy-diagnostics", "copy", "Copy sanitized system & Craft diagnostics to clipboard");
            if action_button(ui, ctx.reg, ctx.pal, diag_spec).clicked() {
                ctx.push(UiAction::CopyDiagnostics);
            }
            let rep_spec = BtnSpec::icon_only("menu-report-problem", "chat", "Report a Problem (sanitized diagnostics)");
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
            if action_button(ui, ctx.reg, ctx.pal, rep_spec).clicked() {
                ctx.push(UiAction::OpenReportProblem);
            }
            let search_w = if wide { 168.0 } else { 116.0 };
            let sresp = text_input(ui, ctx.reg, "command-search-input", "Search commands", &mut ctx.state.command_search, "Search commands…", search_w);
            search_rect = sresp.rect;
        });
    });
    search_rect
}

fn draw_sync_info_banner(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let tracks: Vec<&str> = ctx.state.dirty_tracks.iter().map(String::as_str).collect();
    let msg = format!("Craft tracks ({}) were updated on disk. Re-assemble your FilmCraft Studio timeline now?", tracks.join(", "));
    ui.horizontal(|ui| {
        let _ = action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("banner-info-icon", "info", "Timeline sync notice"));
        ui.label(RichText::new(msg).size(11.5).color(ctx.pal.text_secondary));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("banner-dismiss", "No", "Dismiss notice")).clicked() {
                ctx.state.update_banner_dismissed = true;
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("banner-sync-yes", "Yes", "Assemble FilmCraft sequence now").primary()).clicked() {
                ctx.state.dirty_tracks.clear();
                ctx.exec("video.assemble", json!({}), Some("Synchronized FilmCraft sequence"));
            }
        });
    });
}

fn format_clock(sec: f64) -> String {
    if !sec.is_finite() || sec < 0.0 {
        return "0:00".into();
    }
    let total = sec.floor() as u64;
    format!("{}:{:02}", total / 60, total % 60)
}

fn clamp_title(raw: &str, max_chars: usize) -> String {
    let count = raw.chars().count();
    if count <= max_chars {
        raw.to_owned()
    } else {
        let mut s: String = raw.chars().take(max_chars.saturating_sub(1)).collect();
        s.push('…');
        s
    }
}

fn storyboard_dot_color(ctx: &ViewCtx<'_>) -> Color32 {
    let sb = &ctx.state.cache.snapshot["data"]["storyboard"];
    let light = sb["light"].as_str().or_else(|| ctx.state.cache.storyboard_check["light"].as_str()).unwrap_or("red");
    if sb["production_ready"].as_bool() == Some(true) || ctx.state.cache.storyboard_check["production_ready"].as_bool() == Some(true) || light == "green" {
        ctx.pal.accent_emerald
    } else if light == "amber" || sb["title_sections_complete"].as_bool() == Some(true) {
        ctx.pal.accent_amber
    } else {
        ctx.pal.danger
    }
}

fn draw_app_bar(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let has_proj = ctx.state.project_path.is_some();
    let active_view = ctx.state.effective_view();
    let bar_w = ui.available_width().max(320.0);
    let compact = bar_w < 1160.0;
    let (bar_rect, _) = ui.allocate_exact_size(Vec2::new(bar_w, 32.0), Sense::hover());

<<<<<<< HEAD
    let numbered_tabs = bar_w >= 1320.0;
    let capsule_w = if has_proj {
        if compact {
            378.0
        } else if numbered_tabs {
            476.0
        } else {
            414.0
=======
    let icon_only_tabs = bar_w < 980.0;
    let numbered_tabs = bar_w >= 1380.0;
    let capsule_w = if has_proj {
        if icon_only_tabs {
            218.0
        } else if compact {
            446.0
        } else if numbered_tabs {
            548.0
        } else {
            478.0
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
        }
    } else {
        0.0
    };
<<<<<<< HEAD
    let right_reserve = if has_proj && !compact { 338.0 } else { 256.0 };
=======
    let right_reserve = if has_proj && !compact { 366.0 } else { 282.0 };
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    let left_limit =
        if has_proj { (bar_rect.max.x - right_reserve - capsule_w - 16.0).max(bar_rect.min.x + 170.0) } else { bar_rect.max.x - right_reserve - 8.0 };
    let left_rect = Rect::from_min_max(bar_rect.min, Pos2::new(left_limit, bar_rect.max.y));
    let mut left_ui =
        ui.new_child(egui::UiBuilder::new().max_rect(left_rect).layout(Layout::left_to_right(Align::Center)).scope_id(ui.scope_id().with("app_bar_left")));
    left_ui.spacing_mut().item_spacing.x = 4.0;

    if has_proj {
        if action_button(
            &mut left_ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::icon_only("tab-home", "brand", "Return to Magic Studio Home Catalog").selected(active_view == ViewMode::Home),
        )
        .clicked()
        {
            ctx.push(UiAction::SwitchView(ViewMode::Home));
        }

        let pname = ctx.state.project_name();
        let display_name = clamp_title(&pname, if compact { 14 } else { 24 });
        if project_pill_button(
            &mut left_ui,
            ctx.reg,
            ctx.pal,
            "project-pill",
            &display_name,
            active_view == ViewMode::Project,
            &format!("Project Craft Files & Rendered Assets · {pname}"),
        )
        .clicked()
        {
            ctx.push(UiAction::SwitchView(ViewMode::Project));
        }

        if action_button(&mut left_ui, ctx.reg, ctx.pal, BtnSpec::icon_only("project-close", "close", "Close active project")).clicked() {
            ctx.push(UiAction::CloseProject);
        }
    } else {
        let _ = action_button(&mut left_ui, ctx.reg, ctx.pal, BtnSpec::new("brand", "brand", "Magic Studio", "Magic Studio Home Catalog").selected(true));
        let lbl = left_ui.label(RichText::new("No project open").size(12.0).color(ctx.pal.text_muted));
        ctx.reg.record("project-name", "No project open", &lbl, None);
    }

    if !compact && action_button(&mut left_ui, ctx.reg, ctx.pal, BtnSpec::icon_only("window-snap", "external", "Snap / detach window")).clicked() {
        ctx.push(UiAction::SnapWindow);
    }

    let left_used_max = left_ui.min_rect().max.x;
    let mut cap_max_x = left_used_max;

    if has_proj {
        let min_cx = left_used_max + capsule_w * 0.5 + 8.0;
        let max_cx = (bar_rect.max.x - right_reserve - capsule_w * 0.5 - 8.0).max(min_cx);
        let center_x = bar_rect.center().x.clamp(min_cx, max_cx);
        let cap_rect = Rect::from_center_size(Pos2::new(center_x, bar_rect.center().y), Vec2::new(capsule_w, 32.0));
        cap_max_x = cap_rect.max.x;
        let mut center_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(cap_rect)
                .layout(Layout::centered_and_justified(egui::Direction::LeftToRight))
                .scope_id(ui.scope_id().with("app_bar_center")),
        );
        egui::Frame::new()
            .fill(ctx.pal.bg_input)
            .corner_radius(CornerRadius::same(16))
            .inner_margin(egui::Margin::symmetric(4, 3))
            .stroke(Stroke::new(1.0, ctx.pal.border))
            .show(&mut center_ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = if compact { 1.5 } else { 3.0 };
                    for (idx, tab) in CAPSULE_TABS.into_iter().enumerate() {
                        let id = format!("tab-{}", tab.as_id());
                        let dot = if tab == ViewMode::Storyboard {
                            Some(storyboard_dot_color(ctx))
                        } else if tab == ViewMode::Studio && !ctx.state.dirty_tracks.is_empty() {
                            Some(ctx.pal.accent_amber)
                        } else {
                            None
                        };
                        let selected = active_view == tab;
                        let lbl = if numbered_tabs { format!("{:02} {}", idx + 1, tab.tab_label()) } else { tab.tab_label().to_owned() };
<<<<<<< HEAD
                        if mode_tab(ui, ctx.reg, ctx.pal, &id, &lbl, selected, dot).clicked() {
=======
                        let tip = format!("Switch to {:02} {}", idx + 1, tab.tab_label());
                        if mode_tab(
                            ui,
                            ctx.reg,
                            ctx.pal,
                            ModeTabSpec { id: &id, icon: tab.rail_icon(), label: &lbl, tooltip: &tip, icon_only: icon_only_tabs, selected, dot_color: dot },
                        )
                        .clicked()
                        {
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
                            ctx.push(UiAction::SwitchView(tab));
                        }
                    }
                });
            });
    }

    let right_min_x = (cap_max_x + 6.0).min(bar_rect.max.x - 160.0);
    let right_rect = Rect::from_min_max(Pos2::new(right_min_x, bar_rect.min.y), bar_rect.max);
    let mut right_ui =
        ui.new_child(egui::UiBuilder::new().max_rect(right_rect).layout(Layout::right_to_left(Align::Center)).scope_id(ui.scope_id().with("app_bar_right")));
    right_ui.spacing_mut().item_spacing.x = 4.0;

    if action_button(
        &mut right_ui,
        ctx.reg,
        ctx.pal,
        BtnSpec::icon_only("app-options-toggle", "settings", "Options (Interface Style & Craft AI)")
            .selected(ctx.state.active_modal == Some(ActiveModal::Preferences)),
    )
    .clicked()
    {
        ctx.push(UiAction::OpenPreferences);
    }

<<<<<<< HEAD
    if companion_button(&mut right_ui, ctx.reg, ctx.pal, "scribbles-toggle", "scribbles_work", "Open Scribbles character & asset studio", ctx.state.fidget_open)
        .clicked()
    {
        ctx.state.fidget_open = true;
    }
    if companion_button(&mut right_ui, ctx.reg, ctx.pal, "fidget-toggle", "fidget_work", "Toggle Fidget & Scribbles prompt stager", ctx.state.fidget_open)
        .clicked()
    {
        ctx.state.fidget_open = !ctx.state.fidget_open;
=======
    let fidget_docked = ctx.state.docking.is_panel_visible(PanelId::Fidget);
    if companion_button(&mut right_ui, ctx.reg, ctx.pal, "scribbles-toggle", "scribbles_work", "Open Scribbles character & asset studio", fidget_docked)
        .clicked()
    {
        if has_proj {
            ctx.state.docking.set_panel_visible(PanelId::Fidget, true);
        } else {
            ctx.state.ai.options_tab = "ai".into();
            ctx.push(UiAction::OpenPreferences);
        }
    }
    if companion_button(&mut right_ui, ctx.reg, ctx.pal, "fidget-toggle", "fidget_work", "Toggle Fidget & Scribbles prompt stager", fidget_docked).clicked() {
        if has_proj {
            ctx.state.docking.toggle_panel(PanelId::Fidget);
        } else {
            ctx.state.ai.options_tab = "ai".into();
            ctx.push(UiAction::OpenPreferences);
        }
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    }

    for (id, code, fill, app, tip) in [
        ("dock-filmcraft", "FC", Color32::from_rgb(0x10, 0xb9, 0x81), "filmcraft", "Launch FilmCraft (.fcproj)"),
        ("dock-effectcraft", "EC", Color32::from_rgb(0xa8, 0x55, 0xf7), "effectcraft", "Launch EffectCraft (.ecproj)"),
        ("dock-designcraft", "DC", Color32::from_rgb(0xec, 0x48, 0x99), "designcraft", "Launch DesignCraft (.designcraft)"),
        ("dock-vectorcraft", "VC", Color32::from_rgb(0xf5, 0x9e, 0x0b), "vectorcraft", "Launch VectorCraft (.vectorcraft / .svg)"),
        ("dock-photocraft", "PC", Color32::from_rgb(0x3b, 0x82, 0xf6), "photocraft", "Launch PhotoCraft (.pcraft)"),
    ] {
        if craft_swatch_button(&mut right_ui, ctx.reg, id, code, fill, tip).clicked() {
            ctx.exec("workspace.open", json!({"app": app}), Some(tip));
        }
    }

    if has_proj {
        right_ui.add_space(3.0);
        let elapsed = format_clock(ctx.state.audio_time_sec);
        let dur = format_clock(ctx.state.audio_duration_sec);
        let transport_txt = format!("{elapsed} / {dur}");
        let icon = if ctx.state.audio_playing { "pause" } else { "play" };
        let audio_spec = if compact {
            BtnSpec::icon_only("app-audio-toggle", icon, &transport_txt)
        } else {
            BtnSpec::new("app-audio-toggle", icon, &transport_txt, "Play or pause song audio")
        };
        if action_button(&mut right_ui, ctx.reg, ctx.pal, audio_spec.selected(ctx.state.audio_playing)).clicked() {
            ctx.push(UiAction::ToggleAudio);
        }
    }
}

<<<<<<< HEAD
fn draw_stage_assets_drawer(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
=======
/// Dockable `Stage Assets` panel body (`PanelId::Assets`).
pub fn draw_stage_assets_panel(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    let drawer = &ctx.state.cache.snapshot["data"]["drawer"];
    let char_cards = drawer["characters"].as_array().cloned().unwrap_or_default();
    let bg_cards = drawer["backgrounds"].as_array().cloned().unwrap_or_default();
    let total = drawer["total"].as_u64().map_or(char_cards.len() + bg_cards.len(), |n| n as usize);

    if ctx.state.assets_drawer_tab.is_empty() {
        ctx.state.assets_drawer_tab = "characters".into();
    }

<<<<<<< HEAD
    ui.horizontal(|ui| {
        let icon = if ctx.state.assets_drawer_open { "chevronUp" } else { "chevronDown" };
        let assets_lbl = format!("Assets ({total})");
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::new("stage-assets-toggle", icon, &assets_lbl, "Toggle project asset shelf").selected(ctx.state.assets_drawer_open),
        )
        .clicked()
        {
            ctx.state.assets_drawer_open = !ctx.state.assets_drawer_open;
        }
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("stage-assets-pin", "pin", "Pin asset shelf open").selected(ctx.state.assets_drawer_pinned))
            .clicked()
        {
            ctx.state.assets_drawer_pinned = !ctx.state.assets_drawer_pinned;
            if ctx.state.assets_drawer_pinned {
                ctx.state.assets_drawer_open = true;
            }
        }

        let chars_active = ctx.state.assets_drawer_tab != "backgrounds";
        let chars_lbl = format!("Characters / Rig ({})", char_cards.len());
        let chars_btn = if chars_active {
            BtnSpec::label_only("stage-assets-chars", &chars_lbl, "Show character & rig assets").primary()
        } else {
            BtnSpec::label_only("stage-assets-chars", &chars_lbl, "Show character & rig assets")
        };
        if action_button(ui, ctx.reg, ctx.pal, chars_btn).clicked() {
            ctx.state.assets_drawer_tab = "characters".into();
            ctx.state.assets_drawer_open = true;
        }

        let bgs_lbl = format!("Backgrounds ({})", bg_cards.len());
        let bgs_btn = if !chars_active {
            BtnSpec::label_only("stage-assets-bgs", &bgs_lbl, "Show background plates").primary()
        } else {
            BtnSpec::label_only("stage-assets-bgs", &bgs_lbl, "Show background plates")
        };
        if action_button(ui, ctx.reg, ctx.pal, bgs_btn).clicked() {
            ctx.state.assets_drawer_tab = "backgrounds".into();
            ctx.state.assets_drawer_open = true;
        }

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
=======
    let panel = fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal_wrapped(|ui| {
            status_pill(ui, ctx.pal, &format!("Assets ({total})"), ctx.pal.accent_emerald);

            let chars_active = ctx.state.assets_drawer_tab != "backgrounds";
            let chars_lbl = format!("Characters / Rig ({})", char_cards.len());
            let chars_btn = if chars_active {
                BtnSpec::label_only("stage-assets-chars", &chars_lbl, "Show character & rig assets").primary()
            } else {
                BtnSpec::label_only("stage-assets-chars", &chars_lbl, "Show character & rig assets")
            };
            if action_button(ui, ctx.reg, ctx.pal, chars_btn).clicked() {
                ctx.state.assets_drawer_tab = "characters".into();
            }

            let bgs_lbl = format!("Backgrounds ({})", bg_cards.len());
            let bgs_btn = if !chars_active {
                BtnSpec::label_only("stage-assets-bgs", &bgs_lbl, "Show background plates").primary()
            } else {
                BtnSpec::label_only("stage-assets-bgs", &bgs_lbl, "Show background plates")
            };
            if action_button(ui, ctx.reg, ctx.pal, bgs_btn).clicked() {
                ctx.state.assets_drawer_tab = "backgrounds".into();
            }

>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("stage-assets-add", "plus", "Add Asset", "Add project artwork").primary()).clicked() {
                ctx.push(UiAction::PickFile {
                    target: PickerTarget::AddProjectAsset,
                    title: "Add Project Artwork".into(),
<<<<<<< HEAD
                    extensions: vec!["png".into(), "jpg".into(), "jpeg".into(), "webp".into(), "pcraft".into(), "ecproj".into(), "vectorcraft".into()],
=======
                    extensions: vec![
                        "png".into(),
                        "jpg".into(),
                        "jpeg".into(),
                        "webp".into(),
                        "pcraft".into(),
                        "ecproj".into(),
                        "vectorcraft".into(),
                        "glb".into(),
                        "obj".into(),
                    ],
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
                    directory: false,
                    save: false,
                });
            }
        });
<<<<<<< HEAD
    });

    let active_cards = if ctx.state.assets_drawer_tab == "backgrounds" { &bg_cards } else { &char_cards };
    if (ctx.state.assets_drawer_open || ctx.state.assets_drawer_pinned) && !active_cards.is_empty() {
        ui.add_space(4.0);
        ScrollArea::horizontal().id_salt("stage_assets_strip").show(ui, |ui| {
            ui.horizontal(|ui| {
                for card in active_cards {
                    draw_asset_shelf_card(ui, ctx, card);
                }
            });
        });
    }
=======

        ui.add_space(4.0);
        let active_cards = if ctx.state.assets_drawer_tab == "backgrounds" { &bg_cards } else { &char_cards };
        if active_cards.is_empty() {
            ui.label(
                RichText::new("No project assets in this category yet — click + Add Asset or generate via Scribbles.").size(11.0).color(ctx.pal.text_muted),
            );
        } else {
            ScrollArea::vertical().id_salt("stage_assets_panel_scroll").auto_shrink([false, false]).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for card in active_cards {
                        draw_asset_shelf_card(ui, ctx, card);
                    }
                });
            });
        }
    });
    ctx.reg.record("stage-assets", "Stage Assets", &panel.response, None);
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
}

fn draw_asset_shelf_card(ui: &mut Ui, ctx: &mut ViewCtx<'_>, card: &Value) {
    let cid = card["char_id"].as_str().or_else(|| card["id"].as_str()).unwrap_or("asset");
    let raw_name = card["name"].as_str().unwrap_or(cid);
    let name = if raw_name.eq_ignore_ascii_case("parts") { cid } else { raw_name };
    let badge = card["badge"].as_str().unwrap_or("Asset");
    let thumb = card["thumb"].as_str();
    let sel = ctx.state.selected_character_id.as_deref() == Some(cid);
    let (crect, resp) = ui.allocate_exact_size(Vec2::new(156.0, 44.0), Sense::click());
    let stroke = if sel { Stroke::new(1.5, ctx.pal.accent_emerald) } else { Stroke::new(1.0, ctx.pal.border) };
    ui.painter().rect(crect, CornerRadius::same(7), if sel { ctx.pal.accent_emerald_bg } else { ctx.pal.bg_input }, stroke, StrokeKind::Inside);
    let img_rect = Rect::from_min_size(Pos2::new(crect.min.x + 4.0, crect.min.y + 4.0), Vec2::new(44.0, 36.0));
    if !paint_local_image(ui, img_rect, thumb, ctx.pal.bg_stage, 5) {
        ui.painter().text(img_rect.center(), Align2::CENTER_CENTER, "IMG", FontId::monospace(9.0), ctx.pal.text_muted);
    }
    ui.painter().text(
        Pos2::new(img_rect.max.x + 6.0, crect.min.y + 14.0),
        Align2::LEFT_CENTER,
        clamp_title(name, 14),
        FontId::proportional(11.0),
        ctx.pal.text_primary,
    );
    ui.painter().text(Pos2::new(img_rect.max.x + 6.0, crect.min.y + 29.0), Align2::LEFT_CENTER, badge, FontId::monospace(9.5), ctx.pal.accent_emerald);
    if resp.clicked() {
        ctx.push(UiAction::SelectCharacter { id: cid.to_owned(), jump_animation: false });
    }
    ctx.reg.record(format!("asset-card-{cid}"), name, &resp, None);
}

fn draw_footer(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let has_proj = ctx.state.project_path.is_some();
    let has_song = ctx.state.song_audio_path().is_some();
    let sb = &ctx.state.cache.snapshot["data"]["storyboard"];
    let sb_ok = sb["production_ready"].as_bool() == Some(true);
    let status = &ctx.state.cache.snapshot["data"]["status"];
    let lyrics_ok = status["lyrics_ready"].as_bool().unwrap_or(has_proj);
    let chars_ok = ctx.state.cache.snapshot.pointer("/data/characters").and_then(Value::as_array).is_some_and(|c| !c.is_empty());
    let studio_ok = status["video_project_exists"].as_bool().unwrap_or(has_proj);

    let bar_w = ui.available_width().max(320.0);
    let (frect, _) = ui.allocate_exact_size(Vec2::new(bar_w, 24.0), Sense::hover());

    let left_w = 340.0_f32.min(bar_w * 0.42);
    let mut left_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(Rect::from_min_max(frect.min, Pos2::new(frect.min.x + left_w, frect.max.y)))
            .layout(Layout::left_to_right(Align::Center))
            .scope_id(ui.scope_id().with("footer_left")),
    );
    for (label, ready) in
        [("Song", has_song), ("Lyrics", lyrics_ok), ("Transcript", has_song), ("Storyboard", sb_ok), ("Animate", chars_ok), ("Studio", studio_ok)]
    {
        let dot_col = if ready { ctx.pal.accent_emerald } else { ctx.pal.border };
        let (drect, _) = left_ui.allocate_exact_size(egui::vec2(8.0, 14.0), Sense::hover());
        left_ui.painter().circle_filled(drect.center(), 3.2, dot_col);
        left_ui.label(RichText::new(label).size(10.5).color(if ready { ctx.pal.text_secondary } else { ctx.pal.text_muted }));
        left_ui.add_space(3.0);
    }

    if bar_w >= 1040.0 {
        let center_msg = if has_proj {
<<<<<<< HEAD
            let summary = ctx.state.cache.snapshot.pointer("/data/footer/summary").and_then(Value::as_str).filter(|s| !s.is_empty());
            match summary {
                Some(extra) => format!("{} · {} · {extra}", ctx.state.project_name(), ctx.state.effective_view().tab_label()),
                None => format!("{} · {}", ctx.state.project_name(), ctx.state.effective_view().tab_label()),
=======
            let ws_name = &ctx.state.docking.active_name;
            let summary = ctx.state.cache.snapshot.pointer("/data/footer/summary").and_then(Value::as_str).filter(|s| !s.is_empty());
            match summary {
                Some(extra) => format!("{} · {} ({ws_name}) · {extra}", ctx.state.project_name(), ctx.state.effective_view().tab_label()),
                None => format!("{} · {} ({ws_name})", ctx.state.project_name(), ctx.state.effective_view().tab_label()),
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
            }
        } else {
            "Choose a project or start a new song".into()
        };
        ui.painter().text(frect.center(), Align2::CENTER_CENTER, &center_msg, FontId::proportional(10.5), ctx.pal.text_muted);
    }

<<<<<<< HEAD
    let right_w = 400.0_f32.min(bar_w - left_w - 8.0).max(200.0);
=======
    let right_w = 420.0_f32.min(bar_w - left_w - 8.0).max(200.0);
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    let mut right_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(Rect::from_min_max(Pos2::new(frect.max.x - right_w, frect.min.y), frect.max))
            .layout(Layout::right_to_left(Align::Center))
            .scope_id(ui.scope_id().with("footer_right")),
    );
    right_ui.spacing_mut().item_spacing.x = 3.0;

<<<<<<< HEAD
    if action_button(
        &mut right_ui,
        ctx.reg,
        ctx.pal,
        BtnSpec::icon_only("footer-toggle-inspector", "inspector", "Toggle right inspector rail").selected(ctx.state.show_right_inspector),
    )
    .clicked()
    {
        ctx.state.show_right_inspector = !ctx.state.show_right_inspector;
    }
    if action_button(
        &mut right_ui,
        ctx.reg,
        ctx.pal,
        BtnSpec::icon_only("footer-toggle-dock", "dockBottom", "Toggle bottom timeline / theatre dock").selected(ctx.state.show_bottom_dock),
    )
    .clicked()
    {
        ctx.state.show_bottom_dock = !ctx.state.show_bottom_dock;
    }
    if action_button(
        &mut right_ui,
        ctx.reg,
        ctx.pal,
        BtnSpec::icon_only("footer-toggle-sidebar", "sidebar", "Toggle left rail & stage assets").selected(ctx.state.show_nav_rail),
    )
    .clicked()
    {
        ctx.state.show_nav_rail = !ctx.state.show_nav_rail;
=======
    let insp_on = ctx.state.docking.is_panel_visible(PanelId::Inspector);
    if action_button(&mut right_ui, ctx.reg, ctx.pal, BtnSpec::icon_only("footer-toggle-inspector", "inspector", "Toggle Inspector dock").selected(insp_on))
        .clicked()
    {
        ctx.state.docking.toggle_panel(PanelId::Inspector);
    }
    let tl_on = ctx.state.docking.is_panel_visible(PanelId::Timeline);
    if action_button(&mut right_ui, ctx.reg, ctx.pal, BtnSpec::icon_only("footer-toggle-dock", "dockBottom", "Toggle Timeline panel").selected(tl_on)).clicked()
    {
        ctx.state.docking.toggle_panel(PanelId::Timeline);
    }
    let nav_on = ctx.state.docking.is_panel_visible(PanelId::Navigator);
    if action_button(&mut right_ui, ctx.reg, ctx.pal, BtnSpec::icon_only("footer-toggle-sidebar", "sidebar", "Toggle Navigator dock").selected(nav_on))
        .clicked()
    {
        ctx.state.docking.toggle_panel(PanelId::Navigator);
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    }

    right_ui.add_space(4.0);
    draw_footer_session_pills(&mut right_ui, ctx);
    right_ui.add_space(4.0);
    let dirty = !ctx.state.dirty_drafts.is_empty();
    let tracks_dirty = !ctx.state.dirty_tracks.is_empty();
    let save_label = if ctx.state.busy {
        "Working…"
    } else if dirty {
        "Unsaved drafts"
    } else if tracks_dirty {
        "Tracks modified"
    } else if has_proj {
        "Project saved"
    } else {
        "Not saved"
    };
    let save_color = if ctx.state.busy || dirty || tracks_dirty {
        ctx.pal.accent_amber
    } else if has_proj {
        ctx.pal.accent_emerald
    } else {
        ctx.pal.text_muted
    };
    status_pill(&mut right_ui, ctx.pal, save_label, save_color);
}
