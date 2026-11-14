//! MailCraft-inspired `Options` dialog (`General` Interface Style & Appearance + `AI & LLM`
//! powered by `craft-ai-core` + `Project & AutoSave`), sanitized `Report a Problem` window,
//! and live `Search commands` palette popup.
//!
//! Uses custom themed popups — `Interface Style` (`Aurora`, `Modern`, `Classic`,
//! `Minimal`, `Ocean`, `Forest`, `Plum`, `Sand`) renders as a custom themed dropdown list.

use egui::{Align, Align2, Color32, CornerRadius, FontId, Layout, Order, Pos2, Rect, RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui, Vec2};
use serde_json::json;

use crate::controls::{ScalarSpec, pill_toggle, ring_slider_h};
use crate::menus::translate_menu_command;
use crate::state::{UiAction, ViewCtx};
use crate::theme::{AccentChoice, INTERFACE_SIZE_PRESETS, InterfaceStyle, ThemeMode};
use crate::widgets::{BtnSpec, action_button, status_pill, text_input};

pub fn draw_options_window(egui_ctx: &egui::Context, ctx: &mut ViewCtx<'_>) -> bool {
    let mut keep_open = true;
    egui::Window::new("Options")
        .id(egui::Id::unique(("magic_modal", "Options")))
        .open(&mut keep_open)
        .collapsible(false)
        .resizable(true)
        .default_width(680.0)
        .default_height(520.0)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .show(egui_ctx, |ui| {
            ui.horizontal(|ui| {
                for (tab_id, label, icon) in [
                    ("general", "General & Style", "sliders"),
                    ("fonts", "Fonts (Font Store)", "type"),
                    ("ai", "AI & LLM (Craft AI)", "sparkles"),
                    ("autosave", "Project & AutoSave", "save"),
                    ("history", "History & Control", "wand"),
                ] {
                    let id = format!("options-tab-{tab_id}");
                    let sel = ctx.state.ai.options_tab == tab_id;
                    if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new(&id, icon, label, label).selected(sel)).clicked() {
                        ctx.state.ai.options_tab = tab_id.into();
                        ctx.state.ai.style_menu_open = false;
                    }
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("options-refresh-ai", "refresh", "Refresh AI & Preferences status")).clicked() {
                        ctx.exec("ai.status", json!({}), None);
                    }
                });
            });
            ui.separator();

            ScrollArea::vertical().id_salt("options_body_scroll").auto_shrink([false, false]).max_height(440.0).show(ui, |ui| {
                match ctx.state.ai.options_tab.as_str() {
                    "general" => draw_general_tab(ui, ctx),
                    "fonts" => {
                        crate::fonts_ui::draw_font_manager_body(ui, ctx);
                        ui.separator();
                        crate::fonts_ui::draw_font_sources_settings(ui);
                    }
                    "autosave" => draw_autosave_tab(ui, ctx),
                    "history" => crate::modals::draw_options_history_tab(ui, ctx),
                    _ => draw_ai_llm_tab(ui, ctx),
                }
            });
        });
    keep_open
}

#[allow(clippy::cognitive_complexity)]
fn draw_general_tab(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ui.label(RichText::new("General").size(15.0).strong().color(ctx.pal.accent_violet));
    ui.add_space(6.0);

    // Interface Style row + custom dropdown menu (matching MailCraft Options screenshot)
    let mut style_btn_rect = Rect::NOTHING;
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(Vec2::new(170.0, 26.0), Layout::left_to_right(Align::Center), |ui| {
            ui.label(RichText::new("Interface Style").size(12.5).color(ctx.pal.text_primary));
        });
        let lbl = format!("{}  ▾", ctx.state.interface_style.label());
        let resp = action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::label_only("pref-interface-style-btn", &lbl, "Select MailCraft/Magic interface style preset").selected(ctx.state.ai.style_menu_open),
        );
        style_btn_rect = resp.rect;
        if resp.clicked() {
            ctx.state.ai.style_menu_open = !ctx.state.ai.style_menu_open;
        }
    });

    if ctx.state.ai.style_menu_open && style_btn_rect.is_positive() {
        draw_interface_style_popup(ui.ctx(), ctx, style_btn_rect.left_bottom() + egui::vec2(0.0, 3.0));
    }

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(Vec2::new(170.0, 26.0), Layout::left_to_right(Align::Center), |ui| {
            ui.label(RichText::new("Colour mode").size(12.5).color(ctx.pal.text_primary));
        });
        for (mode, word) in [(ThemeMode::Dark, "Dark"), (ThemeMode::Light, "Light"), (ThemeMode::System, "System")] {
            let id = format!("pref-theme-{}", mode.as_str());
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new(&id, "eye", word, "Set colour mode").selected(ctx.state.theme_mode == mode)).clicked() {
                ctx.state.theme_mode = mode;
                ctx.exec("prefs.set", json!({"key": "appearance.theme", "value": mode.as_str()}), None);
            }
        }
    });

    ui.add_space(4.0);
    if pill_toggle(ui, ctx.reg, ctx.pal, "pref-high-contrast", "High contrast borders & text", &mut ctx.state.high_contrast, ctx.pal.accent_emerald).changed() {
        let hc = ctx.state.high_contrast;
        ctx.exec("prefs.set", json!({"key": "appearance.accessible_contrast", "value": hc}), None);
    }

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(Vec2::new(170.0, 26.0), Layout::left_to_right(Align::Center), |ui| {
            ui.label(RichText::new("Accent").size(12.5).color(ctx.pal.text_primary));
        });
        for choice in AccentChoice::ALL {
            let sw = choice.swatch(ctx.pal.dark);
            let sel = ctx.state.accent_choice == choice;
            let (r, resp) = ui.allocate_exact_size(Vec2::new(72.0, 26.0), Sense::click());
            ui.painter().rect(
                r,
                CornerRadius::same(6),
                if sel { sw.gamma_multiply(0.22) } else { ctx.pal.bg_input },
                Stroke::new(if sel { 1.6 } else { 1.0 }, if sel { sw } else { ctx.pal.border }),
                StrokeKind::Inside,
            );
            ui.painter().circle_filled(Pos2::new(r.min.x + 11.0, r.center().y), 5.0, sw);
            ui.painter().text(Pos2::new(r.min.x + 21.0, r.center().y), Align2::LEFT_CENTER, choice.label(), FontId::proportional(11.0), ctx.pal.text_primary);
            if resp.clicked() {
                ctx.state.accent_choice = choice;
                ctx.exec("prefs.set", json!({"key": "appearance.accent", "value": choice.as_str()}), None);
            }
            ctx.reg.record(format!("pref-accent-{}", choice.as_str()), choice.label(), &resp, None);
        }
    });

    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        ui.allocate_ui_with_layout(Vec2::new(170.0, 26.0), Layout::left_to_right(Align::Center), |ui| {
            ui.label(RichText::new("Interface size").size(12.5).color(ctx.pal.text_primary));
        });
        for &pct in INTERFACE_SIZE_PRESETS {
            let id = format!("pref-scale-{pct}");
            let lbl = format!("{pct}%");
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, &lbl, "Set UI zoom percentage").selected(ctx.state.ui_scale_pct == pct)).clicked() {
                ctx.state.ui_scale_pct = pct;
                ctx.exec("prefs.set", json!({"key": "appearance.ui_scale", "value": pct}), None);
            }
        }
    });
}

fn draw_interface_style_popup(egui_ctx: &egui::Context, ctx: &mut ViewCtx<'_>, pos: Pos2) {
    egui::Area::new(egui::Id::unique("interface_style_popup")).fixed_pos(pos).order(Order::Foreground).show(egui_ctx, |ui| {
        egui::Frame::new()
            .fill(ctx.pal.bg_elevated)
            .corner_radius(CornerRadius::same(8))
            .inner_margin(egui::Margin::same(6))
            .stroke(Stroke::new(1.0, ctx.pal.border))
            .show(ui, |ui| {
                ui.set_width(156.0);
                ui.vertical(|ui| {
                    for st in InterfaceStyle::ALL {
                        let active = ctx.state.interface_style == st;
                        let (r, resp) = ui.allocate_exact_size(Vec2::new(144.0, 24.0), Sense::click());
                        let fill = if active {
                            ctx.pal.accent_violet.gamma_multiply(0.38)
                        } else if resp.hovered() {
                            ctx.pal.bg_hover
                        } else {
                            Color32::TRANSPARENT
                        };
                        ui.painter().rect_filled(r, CornerRadius::same(5), fill);
                        ui.painter().text(
                            Pos2::new(r.min.x + 10.0, r.center().y),
                            Align2::LEFT_CENTER,
                            st.label(),
                            FontId::proportional(12.0),
                            if active { Color32::WHITE } else { ctx.pal.text_primary },
                        );
                        if resp.clicked() {
                            ctx.state.interface_style = st;
                            ctx.state.ai.style_menu_open = false;
                            ctx.exec("prefs.set", json!({"key": "appearance.interface_style", "value": st.as_str()}), Some("Updated Interface Style"));
                        }
                        ctx.reg.record(format!("pref-style-{}", st.as_str()), st.label(), &resp, None);
                    }
                });
            });
    });
}

fn draw_ai_llm_tab(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ctx.pal.inset_frame().show(ui, |ui| {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                if pill_toggle(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    "ai-master-enabled",
                    "Enable AI Integrations (craft-ai-core)",
                    &mut ctx.state.ai.enabled,
                    ctx.pal.accent_emerald,
                )
                .changed()
                {
                    let en = ctx.state.ai.enabled;
                    ctx.exec("ai.configure", json!({"enabled": en}), Some(if en { "Enabled Craft AI" } else { "Disabled Craft AI" }));
                }
                let (lbl, col) = if ctx.state.ai.enabled {
                    ("Master AI Active", ctx.pal.accent_emerald)
                } else {
                    ("Disabled — Zero LLM/Key Activity", ctx.pal.text_muted)
                };
                status_pill(ui, ctx.pal, lbl, col);
            });
            ui.label(
                RichText::new("Shared store & secret references managed via `craft-ai-core`. Raw API keys are never written to project files.")
                    .size(10.5)
                    .color(ctx.pal.text_muted),
            );
        });
    });

    ui.add_space(8.0);
    draw_register_connection_box(ui, ctx);
    ui.add_space(8.0);
    draw_consent_ledger_box(ui, ctx);
}

fn draw_register_connection_box(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ctx.pal.inset_frame().show(ui, |ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new("Add / Update SharedStore Connection (Local Subscription Bridge or Cloud Provider)")
                    .size(12.0)
                    .strong()
                    .color(ctx.pal.accent_amber),
            );
            ui.horizontal_wrapped(|ui| {
                let sel_local = ctx.state.ai.new_conn_provider == "local";
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::label_only("ai-prov-local", "Local Loopback", "Local loopback proxy (http://127.0.0.1:8000/v1)").selected(sel_local),
                )
                .clicked()
                {
                    ctx.state.ai.new_conn_provider = "local".to_owned();
                    ctx.state.ai.new_conn_endpoint = "http://127.0.0.1:8000/v1".to_owned();
                    ctx.state.ai.new_conn_label = "Local Loopback Bridge".to_owned();
                }
                for preset in craft_ai_core::PRESETS {
                    let btn_id = format!("ai-prov-{}", preset.key);
                    let sel = ctx.state.ai.new_conn_provider == preset.provider;
                    if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&btn_id, preset.name, preset.note).selected(sel)).clicked() {
                        ctx.state.ai.new_conn_provider = preset.provider.to_owned();
                        ctx.state.ai.new_conn_endpoint = preset.endpoint.to_owned();
                        ctx.state.ai.new_conn_label = preset.name.to_owned();
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Conn ID").size(10.5).color(ctx.pal.text_muted));
                text_input(ui, ctx.reg, "ai-new-conn-id", "Connection ID", &mut ctx.state.ai.new_conn_id, "local-sub", 120.0);
                ui.label(RichText::new("Label").size(10.5).color(ctx.pal.text_muted));
                text_input(ui, ctx.reg, "ai-new-conn-label", "Connection label", &mut ctx.state.ai.new_conn_label, "Local Subscription Bridge", 190.0);
                ui.label(RichText::new("Keychain Ref").size(10.5).color(ctx.pal.text_muted));
                text_input(ui, ctx.reg, "ai-new-conn-secret", "OS Keychain secret name", &mut ctx.state.ai.new_conn_secret, "(none for local)", 130.0);
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Endpoint").size(10.5).color(ctx.pal.text_muted));
                text_input(
                    ui,
                    ctx.reg,
                    "ai-new-conn-endpoint",
                    "Loopback http://127.0.0.1:... or https://...",
                    &mut ctx.state.ai.new_conn_endpoint,
                    "http://127.0.0.1:8000/v1",
                    250.0,
                );
                ui.label(RichText::new("Text Model").size(10.5).color(ctx.pal.text_muted));
                text_input(ui, ctx.reg, "ai-new-text-model", "Text model", &mut ctx.state.ai.new_conn_text_model, "local-chat", 110.0);
                ui.label(RichText::new("Image Model").size(10.5).color(ctx.pal.text_muted));
                text_input(ui, ctx.reg, "ai-new-img-model", "Image model", &mut ctx.state.ai.new_conn_image_model, "local-image-tool", 110.0);
            });
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("ai-save-conn", "save", "Register Connection in SharedStore", "Write connection to craft-ai-core SharedStore").primary(),
            )
            .clicked()
            {
                let cid = ctx.state.ai.new_conn_id.trim().to_owned();
                let tm = ctx.state.ai.new_conn_text_model.trim().to_owned();
                let im = ctx.state.ai.new_conn_image_model.trim().to_owned();
                let payload = json!({
                    "enabled": ctx.state.ai.enabled,
                    "register_connection": {
                        "id": cid,
                        "label": ctx.state.ai.new_conn_label.trim(),
                        "provider": ctx.state.ai.new_conn_provider.trim(),
                        "endpoint": ctx.state.ai.new_conn_endpoint.trim(),
                        "secret_name": ctx.state.ai.new_conn_secret.trim(),
                        "text_model": tm,
                        "image_model": im,
                    }
                });
                ctx.exec("ai.configure", payload, Some("Saved connection in craft-ai SharedStore"));
            }
        });
    });
}

fn draw_consent_ledger_box(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ctx.pal.inset_frame().show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Outbound Disclosure Consent (`ConsentLedger`):").size(11.5).strong());
            for (term, lbl) in [("session", "Allow This Session"), ("thirty_days", "Allow 30 Days"), ("until_revoked", "Allow Until Revoked")] {
                let id = format!("ai-consent-{term}");
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, "Record disclosure consent")).clicked() {
                    ctx.exec("ai.consent", json!({"agree": true, "term": term}), Some("Recorded outbound AI consent"));
                }
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("ai-consent-revoke", "trash", "Revoke", "Clear ConsentLedger")).clicked() {
                ctx.exec("ai.consent", json!({"revoke": true}), Some("Revoked outbound AI consent"));
            }
        });
    });
}

fn draw_autosave_tab(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ui.label(RichText::new("Project & AutoSave").size(15.0).strong().color(ctx.pal.accent_emerald));
    ui.add_space(6.0);
    if pill_toggle(ui, ctx.reg, ctx.pal, "pref-autosave-enabled", "Automatically save project drafts", &mut ctx.state.autosave_enabled, ctx.pal.accent_emerald)
        .changed()
    {
        let en = ctx.state.autosave_enabled;
        ctx.exec("prefs.set", json!({"key": "autosave.enabled", "value": en}), None);
    }
    if ring_slider_h(
        ui,
        ctx.reg,
        ctx.pal,
        ScalarSpec::new("pref-autosave-mins", "AutoSave interval", 1.0..=30.0, "m", ctx.pal.accent_emerald),
        &mut ctx.state.autosave_minutes,
    )
    .changed()
    {
        let mins = ctx.state.autosave_minutes.round() as u32;
        ctx.exec("prefs.set", json!({"key": "autosave.interval_minutes", "value": mins}), None);
    }
    ui.add_space(8.0);
    ctx.pal.inset_frame().show(ui, |ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new("App Default Folders & Storyboard System Template").size(12.0).strong().color(ctx.pal.accent_sky));
            let note = "Projects default to ~/Magic; optional starter document lives in ~/.local/share/magic/templates/storyboard.designcraft";
            ui.label(RichText::new(note).size(10.5).color(ctx.pal.text_muted));
            ui.horizontal_wrapped(|ui| {
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::new(
                        "pref-open-system-storyboard-tpl",
                        "external",
                        "Inspect System Template",
                        "Open ~/.local/share/magic/templates/storyboard.designcraft in DesignCraft",
                    ),
                )
                .clicked()
                    && let Some(dir) = magic_engine::workspace::ensure_default_app_folders()
                {
                    let tpl = dir.join("templates/storyboard.designcraft");
                    ctx.exec("workspace.open", json!({"app": "designcraft", "path": tpl}), None);
                }
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::new(
                        "pref-reset-storyboard-tpl",
                        "refresh",
                        "Use Built-in Magic Template",
                        "Reset project.storyboard_template to default system template",
                    ),
                )
                .clicked()
                {
                    ctx.exec("prefs.set", json!({"key": "project.storyboard_template", "value": "magic"}), Some("Reset storyboard template to system default"));
                }
            });
        });
    });
}

pub fn draw_report_problem_window(egui_ctx: &egui::Context, ctx: &mut ViewCtx<'_>) -> bool {
    let mut keep_open = true;
    egui::Window::new("Report a Problem")
        .id(egui::Id::unique(("magic_modal", "ReportProblem")))
        .open(&mut keep_open)
        .collapsible(false)
        .resizable(false)
        .default_width(480.0)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .show(egui_ctx, |ui| {
            ui.label(
                RichText::new("Describe what happened (diagnostics scrub home paths to `~` and omit all tokens/keys):")
                    .size(11.5)
                    .color(ctx.pal.text_secondary),
            );
            text_input(ui, ctx.reg, "problem-summary", "Summary", &mut ctx.state.ai.problem_summary, "Short summary of the issue…", 450.0);
            let steps_resp = ui.add_sized([450.0, 80.0], egui::TextEdit::multiline(&mut ctx.state.ai.problem_steps).hint_text("Steps to reproduce…"));
            ctx.reg.record("problem-steps", "Steps to reproduce", &steps_resp, None);
            ui.horizontal(|ui| {
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::new("problem-copy-diag", "copy", "Copy Sanitized Report", "Copy scrubbed report & diagnostics to clipboard").primary(),
                )
                .clicked()
                {
                    ctx.push(UiAction::CopyDiagnostics);
                }
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::new("problem-open-issues", "external", "Open Issue Tracker ↗", "Open GitHub issue tracker in browser"),
                )
                .clicked()
                {
                    ctx.push(UiAction::OpenUrl("https://github.com/storytold/craft-ai-integrations/issues".into()));
                }
            });
        });
    keep_open
}

#[allow(clippy::cognitive_complexity)]
/// Live floating command-palette popup beneath `🔍 Search commands` (`image_222897.png`).
pub fn draw_command_palette_popup(egui_ctx: &egui::Context, ctx: &mut ViewCtx<'_>, anchor: Rect) {
    let q = ctx.state.command_search.trim().to_ascii_lowercase();
    if q.is_empty() || !anchor.is_positive() {
        return;
    }
    let matches: Vec<&magic_engine::CommandSpec> = magic_engine::command_specs()
        .iter()
        .filter(|spec| {
            let id_lc = spec.id.to_ascii_lowercase();
            let lbl_lc = spec.label.to_ascii_lowercase();
            !id_lc.contains("moppet")
                && !lbl_lc.contains("moppet")
                && (id_lc.contains(&q) || lbl_lc.contains(&q) || spec.menu.iter().any(|m| m.to_ascii_lowercase().contains(&q)))
        })
        .take(10)
        .collect();

    egui::Area::new(egui::Id::unique("magic_command_palette_popup"))
        .fixed_pos(Pos2::new((anchor.max.x - 320.0).max(8.0), anchor.max.y + 4.0))
        .order(Order::Foreground)
        .show(egui_ctx, |ui| {
            egui::Frame::new()
                .fill(ctx.pal.bg_elevated)
                .corner_radius(CornerRadius::same(8))
                .inner_margin(egui::Margin::same(6))
                .stroke(Stroke::new(1.0, ctx.pal.accent_emerald))
                .show(ui, |ui| {
                    ui.set_width(316.0);
                    ui.vertical(|ui| {
                        if matches.is_empty() {
                            ui.label(RichText::new("No matching commands").size(11.0).color(ctx.pal.text_muted));
                            return;
                        }
                        for spec in matches {
                            let (r, resp) = ui.allocate_exact_size(Vec2::new(304.0, 26.0), Sense::click());
                            if resp.hovered() {
                                ui.painter().rect_filled(r, CornerRadius::same(5), ctx.pal.bg_hover);
                            }
                            ui.painter().text(
                                Pos2::new(r.min.x + 8.0, r.center().y),
                                Align2::LEFT_CENTER,
                                spec.label,
                                FontId::proportional(11.5),
                                ctx.pal.text_primary,
                            );
                            let right_txt = spec.shortcut.unwrap_or(spec.id);
                            ui.painter().text(
                                Pos2::new(r.max.x - 8.0, r.center().y),
                                Align2::RIGHT_CENTER,
                                right_txt,
                                FontId::monospace(9.5),
                                ctx.pal.text_muted,
                            );
                            if resp.clicked() {
                                let cref = craft_keymap::CommandRef::from(spec.id);
                                if let Some(act) = translate_menu_command(&cref, ctx.state) {
                                    ctx.push(act);
                                }
                                ctx.state.command_search.clear();
                            }
                            ctx.reg.record(format!("cmd-hit-{}", spec.id), spec.label, &resp, None);
                        }
                    });
                });
        });
}
