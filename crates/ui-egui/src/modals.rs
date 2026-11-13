//! Workstation modals (`Options`, `Doctor`, `About`, `UnsavedChanges`, `ReportProblem`),
//! `#app-footer` session/journal chips, dockable `Undo & AutoSave` panel (`PanelId::History`),
//! and toast notifications.

use egui::{Align, Align2, Color32, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui, Vec2};
use serde_json::json;

use crate::controls::fill_card;
use crate::docking::PanelId;
use crate::options_modal::{draw_options_window, draw_report_problem_window};
use crate::state::{ActiveModal, UiAction, ViewCtx};
use crate::widgets::{BtnSpec, action_button};

struct FooterChipSpec<'a> {
    id: &'a str,
    label: &'a str,
    ok: bool,
    ready: bool,
    tooltip: &'a str,
    opens_history: bool,
}

/// Compact `#app-footer` chips for Magic Studio's `.history/` journal and live `--control` status.
pub fn draw_footer_session_pills(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let has_proj = ctx.state.project_path.is_some();
    let dirty = !ctx.state.dirty_drafts.is_empty() || !ctx.state.dirty_tracks.is_empty();
    draw_status_chip(
        ui,
        ctx,
        FooterChipSpec {
            id: "footer-history-pill",
            label: "History",
            ok: has_proj && !dirty,
            ready: has_proj,
            tooltip: "Toggle Undo & AutoSave Journal panel (.history/)",
            opens_history: true,
        },
    );
    let ctrl_lbl = match ctx.state.control_port {
        Some(port) => format!("Magic :{port}"),
        None => "Magic Local".into(),
    };
    draw_status_chip(
        ui,
        ctx,
        FooterChipSpec {
            id: "footer-control-pill",
            label: &ctrl_lbl,
            ok: ctx.state.control_port.is_some(),
            ready: has_proj,
            tooltip: "Run Magic Studio workspace & Craft tool health check (app.doctor)",
            opens_history: false,
        },
    );
}

fn draw_status_chip(ui: &mut Ui, ctx: &mut ViewCtx<'_>, spec: FooterChipSpec<'_>) {
    let dot_col = if spec.ok {
        ctx.pal.accent_emerald
    } else if spec.ready {
        ctx.pal.accent_amber
    } else {
        ctx.pal.border
    };
    let font = egui::FontId::monospace(9.5);
    let galley = ui.painter().layout_no_wrap(spec.label.to_owned(), font.clone(), ctx.pal.text_secondary);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(galley.size().x + 20.0, 18.0), Sense::click());
    let fill = if resp.hovered() { ctx.pal.bg_hover } else { ctx.pal.bg_input };
    ui.painter().rect(rect, CornerRadius::same(9), fill, Stroke::new(1.0, dot_col.gamma_multiply(0.55)), StrokeKind::Inside);
    ui.painter().circle_filled(egui::pos2(rect.min.x + 7.5, rect.center().y), 2.8, dot_col);
    ui.painter().text(egui::pos2(rect.min.x + 13.5, rect.center().y), Align2::LEFT_CENTER, spec.label, font, ctx.pal.text_secondary);
    let resp = resp.on_hover_text(spec.tooltip);
    if resp.clicked() {
        if spec.opens_history {
            ctx.state.docking.toggle_panel(PanelId::History);
        } else {
            ctx.exec("app.doctor", json!({}), Some("Checked Magic Studio workspace health"));
        }
    }
    ctx.reg.record(spec.id, spec.label, &resp, Some(spec.ok.to_string()));
}

/// Options dialog `History & Control` tab.
pub fn draw_options_history_tab(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ui.label(RichText::new("Undo History, Atomic AutoSave & Live --control (:9875)").size(15.0).strong().color(ctx.pal.accent_emerald));
    ui.add_space(4.0);
    ui.label(
        RichText::new(
            "Magic Studio journals project changes atomically inside `.history/` via `craft-atomic` and listens on `127.0.0.1:9875` so `magic-cli` and `magic-mcp` can attach directly.",
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
            BtnSpec::new("opt-open-history", "refresh", "Open Undo & AutoSave Panel", "Show Undo & AutoSave Journal in craft_ui::docking").primary(),
        )
        .clicked()
        {
            ctx.state.docking.set_panel_visible(PanelId::History, true);
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
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("opt-run-doctor", "wand", "Run Suite Doctor", "Inspect installed ArtCraft CLIs and project health"))
            .clicked()
        {
            ctx.exec("app.doctor", json!({}), Some("Updated Suite Doctor report"));
        }
    });
}

/// Dockable `Undo & AutoSave Journal` panel body (`PanelId::History`).
pub fn draw_history_panel_body(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal_wrapped(|ui| {
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("history-undo", "undo", "Undo", "Undo last project command (Ctrl+Z)")).clicked() {
                ctx.exec("edit.undo", json!({}), None);
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("history-redo", "redo", "Redo", "Redo last undone project command (Ctrl+Shift+Z)")).clicked() {
                ctx.exec("edit.redo", json!({}), None);
            }
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("history-autosave", "save", "Snapshot", "Write atomic crash-recovery .autosave journal").primary(),
            )
            .clicked()
            {
                ctx.exec("project.autosave", json!({}), Some("Wrote .history/*.autosave snapshot"));
            }
        });
        ui.add_space(6.0);
        ui.label(
            RichText::new("Project edits are journaled atomically inside `.history/` via `craft-atomic`, and converted source folders are archived under `magic/originals/<slug>.zip` via `craft-zip`.")
                .size(11.0)
                .color(ctx.pal.text_muted),
        );
    });
}

#[allow(clippy::cognitive_complexity)]
pub fn draw_modal(egui_ctx: &egui::Context, ctx: &mut ViewCtx<'_>, modal: ActiveModal) {
    match modal {
        ActiveModal::Preferences => {
            if !draw_options_window(egui_ctx, ctx) {
                ctx.state.active_modal = None;
                ctx.state.ai.style_menu_open = false;
            }
            return;
        }
        ActiveModal::FontManager => {
            if !crate::fonts_ui::draw_font_manager_window(egui_ctx, ctx) {
                ctx.state.active_modal = None;
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
        ActiveModal::Preferences | ActiveModal::FontManager | ActiveModal::ReportProblem => return,
    };
    let mut keep_open = true;
    egui::Window::new(title)
        .id(egui::Id::unique(title))
        .open(&mut keep_open)
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(egui_ctx, |ui| match modal {
            ActiveModal::Doctor => {
                ui.horizontal(|ui| {
                    ui.label("Installed ArtCraft CLI & `craft-ai-integrations` diagnostics:");
                    if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("doctor-refresh", "refresh", "Refresh Suite Doctor (app.doctor)")).clicked() {
                        ctx.exec("app.doctor", json!({}), None);
                    }
                    if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("doctor-copy", "copy", "Copy scrubbed diagnostics to clipboard")).clicked() {
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
                ui.label("eframe + egui + storytold/craft-libs + storytold/craft-ai-integrations coordinator.");
                ui.label("Coordinates host DesignCraft, PhotoCraft, VectorCraft, EffectCraft, and FilmCraft over --control and <app>-cli.");
            }
            ActiveModal::UnsavedClose => {
                ui.label("You have unsaved project or stage changes. Save before closing?");
                ui.horizontal(|ui| {
                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("dialog.unsaved.save", "save", "Save", "Save changes and close").primary())
                            .clicked()
                        {
                            ctx.state.dirty_drafts.clear();
                            ctx.exec("file.save", json!({}), None);
                            ctx.state.active_modal = None;
                            ctx.push(UiAction::RequestQuit);
                        }
                        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("dialog.unsaved.dontSave", "trash", "Discard", "Discard unsaved drafts and quit"))
                            .clicked()
                        {
                            ctx.state.dirty_drafts.clear();
                            ctx.state.active_modal = None;
                            ctx.push(UiAction::Execute { id: "app.quit".into(), params: json!({"force": true}), toast: None });
                        }
                        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("dialog.unsaved.cancel", "close", "Cancel", "Keep working")).clicked() {
                            ctx.state.active_modal = None;
                        }
                    });
                });
            }
            ActiveModal::Preferences | ActiveModal::FontManager | ActiveModal::ReportProblem => {}
        });
    if !keep_open {
        ctx.state.active_modal = None;
    }
}

pub fn draw_toast_if_active(egui_ctx: &egui::Context, ctx: &mut ViewCtx<'_>) {
    let now = egui_ctx.input(|i| i.time);
    if ctx.state.toast_message.is_empty() || now >= ctx.state.toast_until {
        return;
    }
    egui::Area::new(egui::Id::unique("magic_toast_area")).anchor(Align2::RIGHT_BOTTOM, Vec2::new(-16.0, -34.0)).order(egui::Order::Tooltip).show(
        egui_ctx,
        |ui| {
            Frame::new()
                .fill(Color32::from_rgba_unmultiplied(12, 18, 28, 238))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::symmetric(12, 7))
                .stroke(Stroke::new(1.0, ctx.pal.accent_emerald))
                .show(ui, |ui| {
                    ui.label(RichText::new(&ctx.state.toast_message).size(11.5).color(ctx.pal.text_primary));
                });
        },
    );
}
