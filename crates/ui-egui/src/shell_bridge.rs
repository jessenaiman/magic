//! Shell-level `FilmCraft` × `EffectCraft` bridge chrome (`T07`, `T09`) plus workstation modals:
//! - `draw_footer_bridge_pills`: live `FC :9876` (`FILMCRAFT_CONTROL_TOKEN`) & `EC :9877`
//!   (`effectcraft-cli mcp --autosave`) TCP control bridge status pills in `#app-footer`.
//! - `draw_options_bridge_and_history_tab`: Options modal `Bridge (:9876 / :9877) & History` tab.
//! - `draw_history_recovery_window`: non-linear branching Undo History (`edit.history.list` / `goto`)
//!   and 1-second crash-journal recovery (`file.recover`) across `FilmCraft` and `EffectCraft`.

use egui::{
    Align, Align2, CornerRadius, Layout, RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui, Vec2,
};
use serde_json::{Value, json};

use crate::controls::pill_toggle;
use crate::options_modal::{draw_options_window, draw_report_problem_window};
use crate::state::{ActiveModal, UiAction, ViewCtx};
use crate::widgets::{BtnSpec, action_button, text_input};

/// Live `FC :9876` (`FILMCRAFT_CONTROL_TOKEN`) & `EC :9877` (`--autosave`) bridge indicators (`T07`).
pub fn draw_footer_bridge_pills(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let fc_port = ctx
        .state
        .cache
        .bridge_status
        .pointer("/filmcraft/port")
        .and_then(Value::as_u64)
        .unwrap_or(9876);
    let ec_port = ctx
        .state
        .cache
        .bridge_status
        .pointer("/effectcraft/port")
        .and_then(Value::as_u64)
        .unwrap_or(9877);
    let fc_live = ctx
        .state
        .cache
        .bridge_status
        .pointer("/filmcraft/tcp_listening")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let fc_tok = ctx
        .state
        .cache
        .bridge_status
        .pointer("/filmcraft/token_ready")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let ec_live = ctx
        .state
        .cache
        .bridge_status
        .pointer("/effectcraft/tcp_listening")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let fc_lbl = format!("FC :{fc_port}");
    let ec_lbl = format!("EC :{ec_port}");
    draw_bridge_chip(
        ui,
        ctx,
        "footer-bridge-fc",
        &fc_lbl,
        fc_live,
        fc_tok,
        "FilmCraft live --control (127.0.0.1:9876 + FILMCRAFT_CONTROL_TOKEN) — click to probe",
    );
    draw_bridge_chip(
        ui,
        ctx,
        "footer-bridge-ec",
        &ec_lbl,
        ec_live,
        true,
        "EffectCraft live --control (127.0.0.1:9877 + effectcraft-cli mcp --autosave) — click to probe",
    );
}

fn draw_bridge_chip(
    ui: &mut Ui,
    ctx: &mut ViewCtx<'_>,
    id: &str,
    label: &str,
    live: bool,
    token_ready: bool,
    tooltip: &str,
) {
    let dot_col = if live {
        ctx.pal.accent_emerald
    } else if token_ready {
        ctx.pal.accent_amber
    } else {
        ctx.pal.danger
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
        ctx.exec(
            "bridge.status",
            json!({"ensure_tokens": true}),
            Some("Probed FilmCraft (:9876) & EffectCraft (:9877) control bridges"),
        );
    }
    ctx.reg.record(id, label, &resp, Some(live.to_string()));
}

/// Options window tab for `T07` (`:9876` / `:9877` live `--control` & `mcp --autosave`) and `T09` (History & Recovery).
pub fn draw_options_bridge_and_history_tab(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ui.label(
        RichText::new("FilmCraft (:9876) & EffectCraft (:9877) --control Bridge")
            .size(15.0)
            .strong()
            .color(ctx.pal.accent_emerald),
    );
    ui.add_space(4.0);
    ui.label(
        RichText::new(
            "Magic Studio (:9875) drives installed FilmCraft (127.0.0.1:9876 + FILMCRAFT_CONTROL_TOKEN) and EffectCraft (127.0.0.1:9877 + effectcraft-cli mcp --autosave) over loopback JSON-lines or headless CLI.",
        )
        .size(11.0)
        .color(ctx.pal.text_secondary),
    );
    ui.add_space(6.0);

    if pill_toggle(
        ui,
        ctx.reg,
        ctx.pal,
        "pref-bridge-live",
        "Route Engine Commands Through Live --control Window When Open",
        &mut ctx.state.ai.prefer_live_bridge,
        ctx.pal.accent_emerald,
    )
    .changed()
    {
        ctx.exec(
            "prefs.set",
            json!({"key": "craft.prefer_live_bridge", "value": ctx.state.ai.prefer_live_bridge}),
            None,
        );
    }
    if pill_toggle(
        ui,
        ctx.reg,
        ctx.pal,
        "pref-ec-mcp-autosave",
        "Enable Headless EffectCraft MCP Checkpoints (effectcraft-cli mcp --autosave)",
        &mut ctx.state.ai.ec_mcp_autosave,
        ctx.pal.accent_sky,
    )
    .changed()
    {
        ctx.exec(
            "prefs.set",
            json!({"key": "craft.effectcraft_mcp_autosave", "value": ctx.state.ai.ec_mcp_autosave}),
            None,
        );
    }

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("FilmCraft Port:").size(11.5));
        if text_input(
            ui,
            ctx.reg,
            "pref-fc-port",
            "FilmCraft --control port",
            &mut ctx.state.ai.fc_port_str,
            "9876",
            72.0,
        )
        .lost_focus()
            && let Ok(port) = ctx.state.ai.fc_port_str.trim().parse::<u16>()
        {
            ctx.exec("prefs.set", json!({"key": "craft.filmcraft_port", "value": port}), None);
        }
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::new("opt-launch-fc", "film", "Launch FilmCraft :9876", "Spawn filmcraft --control 9876"),
        )
        .clicked()
        {
            ctx.exec("bridge.control", json!({"app": "filmcraft", "action": "launch"}), Some("Launched FilmCraft --control"));
        }
    });
    ui.horizontal(|ui| {
        ui.label(RichText::new("EffectCraft Port:").size(11.5));
        if text_input(
            ui,
            ctx.reg,
            "pref-ec-port",
            "EffectCraft --control port",
            &mut ctx.state.ai.ec_port_str,
            "9877",
            72.0,
        )
        .lost_focus()
            && let Ok(port) = ctx.state.ai.ec_port_str.trim().parse::<u16>()
        {
            ctx.exec("prefs.set", json!({"key": "craft.effectcraft_port", "value": port}), None);
        }
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::new("opt-launch-ec", "sparkles", "Launch EffectCraft :9877", "Spawn effectcraft --control 9877"),
        )
        .clicked()
        {
            ctx.exec("bridge.control", json!({"app": "effectcraft", "action": "launch"}), Some("Launched EffectCraft --control"));
        }
    });

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::new("opt-probe-bridges", "refresh", "Probe :9876 & :9877 + Ensure Tokens", "Run bridge.status ensure_tokens=true").primary(),
        )
        .clicked()
        {
            ctx.exec("bridge.status", json!({"ensure_tokens": true}), Some("Verified :9876 & :9877 control tokens"));
        }
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::new("opt-open-history", "wand", "Open Branching Undo & Crash Recovery…", "Open T09 Undo History & file.recover window"),
        )
        .clicked()
        {
            ctx.state.history_popup_open = true;
            ctx.exec("bridge.history", json!({"op": "list"}), None);
        }
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::new("opt-close-siblings", "close", "Close Opened Craft Apps", "Gracefully close non-dirty Craft windows"),
        )
        .clicked()
        {
            ctx.exec("workspace.close_opened", json!({}), Some("Closed clean sibling Craft apps"));
        }
    });
}

/// Branching Undo History (`edit.history.list` / `goto`) + Crash-Journal Recovery (`T09`).
pub fn draw_history_recovery_window(egui_ctx: &egui::Context, ctx: &mut ViewCtx<'_>) {
    if !ctx.state.history_popup_open {
        return;
    }
    let mut keep_open = true;
    egui::Window::new("Undo History & Crash-Journal Recovery")
        .id(egui::Id::unique("magic_history_recovery_win"))
        .open(&mut keep_open)
        .collapsible(false)
        .resizable(true)
        .default_width(440.0)
        .anchor(Align2::LEFT_TOP, egui::vec2(18.0, 42.0))
        .show(egui_ctx, |ui| {
            ui.horizontal(|ui| {
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::new("history-refresh", "refresh", "Refresh", "Query edit.history.list & crash journals"),
                )
                .clicked()
                {
                    ctx.exec("bridge.history", json!({"op": "list"}), None);
                }
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::new("history-recover-fc", "wand", "Recover FilmCraft", "Replay 1s crash journal via file.recover"),
                )
                .clicked()
                {
                    ctx.exec(
                        "bridge.history",
                        json!({"op": "recover_filmcraft", "app": "filmcraft"}),
                        Some("Recovered FilmCraft crash journal"),
                    );
                }
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::new("history-recover-ec", "sparkles", "Recover EffectCraft", "Replay EffectCraft crash journal via file.recover"),
                )
                .clicked()
                {
                    ctx.exec(
                        "bridge.history",
                        json!({"op": "recover_effectcraft", "app": "effectcraft"}),
                        Some("Recovered EffectCraft crash journal"),
                    );
                }
            });

            let journals = ctx
                .state
                .cache
                .bridge_history
                .get("recovery_journals")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if !journals.is_empty() {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(format!("Rolling backups / crash journals in .history/: {}", journals.len()))
                        .size(11.0)
                        .color(ctx.pal.accent_amber),
                );
            }

            ui.separator();
            ui.label(RichText::new("Non-Linear Branching Undo States (edit.history.goto)").size(11.5).strong());
            let fc_states = ctx
                .state
                .cache
                .bridge_history
                .pointer("/filmcraft_history/entries")
                .or_else(|| ctx.state.cache.bridge_history.pointer("/filmcraft_history/states"))
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let ec_states = ctx
                .state
                .cache
                .bridge_history
                .get("states")
                .or_else(|| ctx.state.cache.bridge_history.pointer("/effectcraft_history/states"))
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();

            ScrollArea::vertical().max_height(210.0).show(ui, |ui| {
                if fc_states.is_empty() && ec_states.is_empty() {
                    ui.label(
                        RichText::new("Click Refresh with an open project to inspect FilmCraft & EffectCraft undo branches.")
                            .size(10.5)
                            .color(ctx.pal.text_muted),
                    );
                }
                for (idx, item) in fc_states.iter().enumerate() {
                    let lbl = item["label"].as_str().unwrap_or("FilmCraft edit");
                    let btn_id = format!("history-goto-fc-{idx}");
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("FC #{idx}")).monospace().size(10.0).color(ctx.pal.accent_emerald));
                        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&btn_id, lbl, "Jump FilmCraft history")).clicked() {
                            ctx.exec(
                                "bridge.history",
                                json!({"op": "goto", "app": "filmcraft", "index": idx}),
                                Some("Jumped FilmCraft undo state"),
                            );
                        }
                    });
                }
                for (idx, item) in ec_states.iter().enumerate() {
                    let lbl = item["label"].as_str().unwrap_or("EffectCraft edit");
                    let depth = item["branch"].as_u64().or_else(|| item["depth"].as_u64()).unwrap_or(0);
                    let btn_id = format!("history-goto-ec-{idx}");
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("EC #{idx} b{depth}"))
                                .monospace()
                                .size(10.0)
                                .color(ctx.pal.accent_sky),
                        );
                        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&btn_id, lbl, "Jump EffectCraft branching history")).clicked() {
                            ctx.exec(
                                "bridge.history",
                                json!({"op": "goto", "app": "effectcraft", "index": idx}),
                                Some("Jumped EffectCraft undo branch"),
                            );
                        }
                    });
                }
            });
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
                    ui.label("Installed ArtCraft CLI, `--control`, & `craft-ai-integrations` diagnostics:");
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
                ui.label("Pure-Rust eframe + egui 0.37 + storytold/craft-libs + storytold/craft-ai-integrations coordinator (:9875).");
                ui.label("Delegates timeline, compositing, cutout, vector, and storyboard layout to FilmCraft (:9876), EffectCraft (:9877), DesignCraft (:7979), PhotoCraft (:9879), VectorCraft (:9880), and headless anicraft-cli.");
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
            let resp = egui::Frame::new()
                .fill(ctx.pal.bg_elevated)
                .corner_radius(CornerRadius::same(8))
                .inner_margin(egui::Margin::symmetric(12, 8))
                .stroke(Stroke::new(1.0, ctx.pal.accent_emerald))
                .show(ui, |ui| {
                    ui.label(RichText::new(&ctx.state.toast_message).color(ctx.pal.text_primary));
                });
            ctx.reg.record("app-toast", &ctx.state.toast_message, &resp.response, None);
        });
}
