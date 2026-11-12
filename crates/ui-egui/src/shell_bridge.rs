//! Workstation modals (`Options`, `Doctor`, `About`, `UnsavedChanges`, `ReportProblem`),
//! `#app-footer` session/journal chips, Undo & AutoSave recovery window, and toast notifications.

use egui::{
    Align, Align2, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Sense, Stroke,
    StrokeKind, Ui, Vec2,
};
use serde_json::json;

use crate::options_modal::{draw_options_window, draw_report_problem_window};
use crate::state::{ActiveModal, UiAction, ViewCtx};
use crate::widgets::{BtnSpec, action_button};

/// Compact `#app-footer` chips for Magic Studio's `.history/` journal and Suite Doctor.
pub fn draw_footer_session_pills(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let has_proj = ctx.state.project_path.is_some();
    let dirty = !ctx.state.dirty_drafts.is_empty() || !ctx.state.dirty_tracks.is_empty();
    draw_status_chip(
        ui,
        ctx,
        "footer-history-pill",
        "History",
        has_proj && !dirty,
        has_proj,
        "Open Session Undo & AutoSave Journal (.history/)",
        true,
    );
    draw_status_chip(
        ui,
        ctx,
        "footer-control-pill",
        "Magic :9875",
        has_proj,
        true,
        "Run Magic Studio workspace & Craft tool health check (app.doctor)",
        false,
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_status_chip(
    ui: &mut Ui,
    ctx: &mut ViewCtx<'_>,
    id: &str,
    label: &str,
    ok: bool,
    ready: bool,
    tooltip: &str,
    opens_history: bool,
) {
    let dot_col = if ok {
        ctx.pal.accent_emerald
    } else if ready {
        ctx.pal.accent_amber
    } else {
        ctx.pal.border
    };
    let font = egui::FontId::monospace(9.5);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font.clone(), ctx.pal.text_secondary);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(galley.size().x + 20.0, 18.0), Sense::click());
    let fill = if resp.hovered() { ctx.pal.bg_hover } else { ctx.pal.bg_input };
    ui.painter().rect(
        rect,
        CornerRadius::same(9),
        fill,
        Stroke::new(1.0, dot_col.gamma_multiply(0.55)),
        StrokeKind::Inside,
    );
    ui.painter()
        .circle_filled(egui::pos2(rect.min.x + 7.5, rect.center().y), 2.8, dot_col);
    ui.painter().text(
        egui::pos2(rect.min.x + 13.5, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        font,
        ctx.pal.text_secondary,
    );
    let resp = resp.on_hover_text(tooltip);
    if resp.clicked() {
        if opens_history {
            ctx.state.history_popup_open = !ctx.state.history_popup_open;
        } else {
            ctx.exec("app.doctor", json!({}), Some("Checked Magic Studio workspace health"));
        }
    }
    ctx.reg.record(id, label, &resp, Some(ok.to_string()));
}

/// Options dialog `History & Control` tab (restored without cross-app CraftHub polling).
pub fn draw_options_history_tab(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ui.label(
        RichText::new("Undo History, Atomic AutoSave & Live --control (:9875)")
            .size(15.0)
            .strong()
            .color(ctx.pal.accent_emerald),
    );
    ui.add_space(4.0);
    ui.label(
        RichText::new(
            "Magic Studio journals project changes atomically inside `.history/` via `craft-atomic` and exposes `127.0.0.1:9875` when launched with `magic --control 9875`.",
        )
        .size(11.0)
        .color(ctx.pal.text_secondary),
    );
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::new("opt-open-history", "wand", "Open Undo & AutoSave Journal…", "Open Session Undo & AutoSave window").primary(),
        )
        .clicked()
        {
            ctx.state.history_popup_open = true;
        }
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::new("opt-write-autosave", "save", "Write .autosave Snapshot", "Write crash-recovery .autosave journal now"),
        )
        .clicked()
        {
            ctx.exec("project.autosave", json!({}), Some("Wrote .history/*.autosave snapshot"));
        }
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::new("opt-run-doctor", "refresh", "Run Suite Doctor", "Inspect installed ArtCraft CLIs and project health"),
        )
        .clicked()
        {
            ctx.exec("app.doctor", json!({}), Some("Updated Suite Doctor report"));
        }
    });
}

/// Magic Studio Undo / Redo & `.history/*.autosave` window.
pub fn draw_history_recovery_window(egui_ctx: &egui::Context, ctx: &mut ViewCtx<'_>) {
    if !ctx.state.history_popup_open {
        return;
    }
    let mut keep_open = true;
    egui::Window::new("Session Undo & AutoSave Journal")
        .id(egui::Id::unique("magic_history_recovery_win"))
        .open(&mut keep_open)
        .collapsible(false)
        .resizable(true)
        .default_width(390.0)
        .anchor(Align2::LEFT_TOP, egui::vec2(18.0, 42.0))
        .show(egui_ctx, |ui| {
            ui.horizontal(|ui| {
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::new("history-undo", "undo", "Undo", "Undo last project command"),
                )
                .clicked()
                {
                    ctx.exec("edit.undo", json!({}), None);
                }
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::new("history-redo", "redo", "Redo", "Redo last undone project command"),
                )
                .clicked()
                {
                    ctx.exec("edit.redo", json!({}), None);
                }
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::new("history-autosave", "save", "Snapshot", "Write crash-recovery .autosave journal"),
                )
                .clicked()
                {
                    ctx.exec("project.autosave", json!({}), Some("Wrote .history/*.autosave snapshot"));
                }
            });
            ui.add_space(6.0);
            ui.label(
                RichText::new("Project changes are journaled atomically inside `.history/` via craft-atomic.")
                    .size(11.0)
                    .color(ctx.pal.text_muted),
            );
        });
    if !keep_open {
        ctx.state.history_popup_open = false;
    }
}

pub fn draw_modal(egui_ctx: &egui::Context, ctx: &mut ViewCtx<'_>, modal: ActiveModal) {
    match modal {
        ActiveModal::Preferences => {
            if !draw_options_window(egui_ctx, ctx) {
                ctx.state.active_modal = None;
                ctx.state.ai.style_menu_open = false;
            }
            return;
        }
        ActiveModal::ReportProblem => {
            if !draw_report_problem_window(egui_ctx, ctx) {
                ctx.state.active_modal = None;
            }
            return;
        }
        _ => {}
    }

    let title = match modal {
        ActiveModal::Doctor => "ArtCraft Suite Doctor",
        ActiveModal::About => "About Magic Studio",
        ActiveModal::UnsavedClose => "Unsaved Changes",
        ActiveModal::Preferences | ActiveModal::ReportProblem => return,
    };
    let mut keep_open = true;
    egui::Window::new(title)
        .id(egui::Id::unique(("magic_modal", title)))
        .open(&mut keep_open)
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(egui_ctx, |ui| match modal {
            ActiveModal::Doctor => {
                ui.horizontal(|ui| {
                    ui.label("Installed ArtCraft CLI & `craft-ai-integrations` diagnostics:");
                    if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("doctor-refresh", "refresh", "Check", "Run app.doctor")).clicked() {
                        ctx.exec("app.doctor", json!({}), None);
                    }
                    if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("doctor-copy", "copy", "Copy Diagnostics", "Copy scrubbed diagnostics")).clicked() {
                        ctx.push(UiAction::CopyDiagnostics);
                    }
                });
                let text = serde_json::to_string_pretty(&ctx.state.cache.doctor).unwrap_or_else(|_| "{}".into());
                ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                    ui.code(text);
                });
            }
            ActiveModal::About => {
                ui.label(RichText::new(concat!("Magic Studio ", env!("CARGO_PKG_VERSION"))).size(16.0).strong());
                ui.label("Pure-Rust eframe + egui 0.37 + storytold/craft-libs + storytold/craft-ai-integrations coordinator.");
                ui.label("Exchanges native project files on disk with DeckCraft, DesignCraft, PhotoCraft, VectorCraft, EffectCraft, FilmCraft, and headless anicraft-cli.");
            }
            ActiveModal::UnsavedClose => {
                ui.label("You have unsaved project or stage changes. Save before closing?");
                ui.horizontal(|ui| {
                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                        if action_button(
                            ui,
                            ctx.reg,
                            ctx.pal,
                            BtnSpec::new("dialog.unsaved.save", "save", "Save", "Save changes and close").primary(),
                        )
                        .clicked()
                        {
                            ctx.state.dirty_drafts.clear();
                            ctx.exec("file.save", json!({}), None);
                            ctx.state.active_modal = None;
                            ctx.push(UiAction::RequestQuit);
                        }
                        if action_button(
                            ui,
                            ctx.reg,
                            ctx.pal,
                            BtnSpec::new("dialog.unsaved.dontSave", "trash", "Discard", "Discard unsaved drafts and quit"),
                        )
                        .clicked()
                        {
                            ctx.state.dirty_drafts.clear();
                            ctx.state.active_modal = None;
                            ctx.push(UiAction::Execute {
                                id: "app.quit".into(),
                                params: json!({"force": true}),
                                toast: None,
                            });
                        }
                        if action_button(
                            ui,
                            ctx.reg,
                            ctx.pal,
                            BtnSpec::new("dialog.unsaved.cancel", "close", "Cancel", "Keep working"),
                        )
                        .clicked()
                        {
                            ctx.state.active_modal = None;
                        }
                    });
                });
            }
            ActiveModal::Preferences | ActiveModal::ReportProblem => {}
        });
    if !keep_open {
        ctx.state.active_modal = None;
    }
}

pub fn draw_toast_if_active(egui_ctx: &egui::Context, ctx: &mut ViewCtx<'_>) {
    let now = egui_ctx.input(|i| i.time);
    if ctx.state.toast_message.is_empty() || now > ctx.state.toast_until {
        return;
    }
    egui::Area::new(egui::Id::unique("app-toast"))
        .anchor(Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -48.0))
        .order(egui::Order::Foreground)
        .show(egui_ctx, |ui| {
            let resp = Frame::new()
                .fill(ctx.pal.bg_elevated)
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::symmetric(12, 8))
                .stroke(Stroke::new(1.0, ctx.pal.accent_emerald))
                .show(ui, |ui| {
                    ui.label(RichText::new(&ctx.state.toast_message).color(ctx.pal.text_primary));
                });
            ctx.reg.record("app-toast", &ctx.state.toast_message, &resp.response, None);
        });
}
