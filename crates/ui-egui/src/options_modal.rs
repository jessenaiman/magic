//! MailCraft-inspired `Options` dialog (`General` Interface Style & Appearance + `AI & LLM`
//! powered strictly by `storytold/craft-ai-integrations` + `Project & AutoSave`), `Fidget &
//! Scribbles` prompt stager window, sanitized `Report a Problem` window, and live `Search commands`
//! palette popup.
//!
//! Uses custom themed popups — `Interface Style` (`Aurora`, `Modern`, `Classic`,
//! `Minimal`, `Ocean`, `Forest`, `Plum`, `Sand`) renders as a custom themed dropdown list.

use egui::{Align, Align2, Color32, CornerRadius, FontId, Layout, Order, Pos2, Rect, RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui, Vec2};
use serde_json::{Value, json};

use crate::controls::{ScalarSpec, pill_toggle, ring_slider_h};
use crate::icons::companion_image;
use crate::menus::translate_menu_command;
use crate::state::{UiAction, ViewCtx};
use crate::theme::{AccentChoice, INTERFACE_SIZE_PRESETS, InterfaceStyle, ThemeMode};
use crate::widgets::{BtnSpec, action_button, status_pill, text_input};

const SCRIBBLES_TEMPLATES: &[(&str, &str)] = &[
    ("new", "New Character"),
    ("parts_chroma", "Parts Sheet"),
    ("poses_2x2", "2x2 Poses"),
    ("actions", "Action Strip"),
    ("image", "Scene / Prop"),
    ("custom", "Custom Prompt"),
];

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
<<<<<<< HEAD
=======
                    ("fonts", "Fonts (Font Store)", "type"),
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
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
<<<<<<< HEAD
                    "autosave" => draw_autosave_tab(ui, ctx),
                    "history" => crate::shell_bridge::draw_options_history_tab(ui, ctx),
=======
                    "fonts" => {
                        crate::fonts_ui::draw_font_manager_body(ui, ctx);
                        ui.separator();
                        crate::fonts_ui::draw_font_sources_settings(ui);
                    }
                    "autosave" => draw_autosave_tab(ui, ctx),
                    "history" => crate::modals::draw_options_history_tab(ui, ctx),
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
                    _ => draw_ai_llm_tab(ui, ctx),
                }
            });
        });
    keep_open
}

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

fn role_status_summary(ctx: &ViewCtx<'_>, role_id: &str) -> (String, Color32) {
    if !ctx.state.ai.enabled {
        return ("Off (ai.enabled = false)".into(), ctx.pal.text_muted);
    }
    let Some(roles) = ctx.state.cache.ai_status.get("roles").and_then(Value::as_array) else {
        return ("Unconfigured".into(), ctx.pal.accent_amber);
    };
    let Some(entry) = roles.iter().find(|r| r["id"].as_str() == Some(role_id)) else {
        return ("Unconfigured".into(), ctx.pal.accent_amber);
    };
    let st = &entry["status"];
    if st["state"].as_str() == Some("ready") {
        let conn = st["connection_label"].as_str().or_else(|| st["connection_id"].as_str()).unwrap_or("conn");
        let model = st["model_label"].as_str().or_else(|| st["model"].as_str()).unwrap_or("model");
        (format!("Ready · {conn} / {model}"), ctx.pal.accent_emerald)
    } else {
        let why = st["reason"].as_str().unwrap_or("Bind connection & model");
        (why.to_owned(), ctx.pal.accent_amber)
    }
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
                    "Enable AI Integrations (storytold/craft-ai-integrations)",
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
                RichText::new(
                    "Shared store & OS keychain secrets managed via craft-ai-client / craft-ai-native. Raw API keys are never written to project files.",
                )
                .size(10.5)
                .color(ctx.pal.text_muted),
            );
        });
    });

    ui.add_space(8.0);
    draw_fidget_role_box(ui, ctx);
    ui.add_space(8.0);
    draw_scribbles_role_box(ui, ctx);
    ui.add_space(8.0);
    draw_register_connection_box(ui, ctx);
    ui.add_space(8.0);
    draw_consent_ledger_box(ui, ctx);
}

fn draw_fidget_role_box(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let (status_txt, status_col) = role_status_summary(ctx, "fidget");
    ctx.pal.inset_frame().show(ui, |ui| {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.add(companion_image("fidget_work", 20.0));
                ui.label(RichText::new("Fidget · Structured Prompt Stager (`text.complete`)").size(12.5).strong());
                status_pill(ui, ctx.pal, &status_txt, status_col);
            });
            ui.horizontal_wrapped(|ui| {
                if pill_toggle(ui, ctx.reg, ctx.pal, "ai-fidget-enabled", "Enable Fidget rephrasing", &mut ctx.state.ai.fidget_enabled, ctx.pal.accent_emerald)
                    .changed()
                {
                    ctx.exec("ai.configure", json!({"fidget_enabled": ctx.state.ai.fidget_enabled}), None);
                }
                if pill_toggle(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    "ai-fidget-seamless",
                    "Seamless handoff straight to Scribbles",
                    &mut ctx.state.ai.fidget_seamless,
                    ctx.pal.accent_sky,
                )
                .changed()
                {
                    ctx.exec(
                        "ai.configure",
                        json!({"fidget_seamless": ctx.state.ai.fidget_seamless}),
                        Some(if ctx.state.ai.fidget_seamless {
                            "Fidget set to seamless Scribbles handoff"
                        } else {
                            "Fidget set to review-before-Scribbles staging"
                        }),
                    );
                }
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Connection").size(11.0).color(ctx.pal.text_muted));
                text_input(ui, ctx.reg, "ai-fidget-conn", "Fidget connection ID", &mut ctx.state.ai.fidget_connection, "local-sub", 150.0);
                ui.label(RichText::new("Model").size(11.0).color(ctx.pal.text_muted));
                text_input(ui, ctx.reg, "ai-fidget-model", "Fidget model ID", &mut ctx.state.ai.fidget_model, "my-text-model", 180.0);
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("ai-fidget-bind", "check", "Bind Fidget", "Bind Fidget in ToolDesk").primary()).clicked() {
                    let fc = ctx.state.ai.fidget_connection.trim().to_owned();
                    let fm = ctx.state.ai.fidget_model.trim().to_owned();
                    ctx.exec("ai.configure", json!({"fidget_connection": fc, "fidget_model": fm}), Some("Bound Fidget model"));
                }
            });
        });
    });
}

fn draw_scribbles_role_box(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let (status_txt, status_col) = role_status_summary(ctx, "scribbles");
    ctx.pal.inset_frame().show(ui, |ui| {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.add(companion_image("scribbles_work", 20.0));
                ui.label(RichText::new("Scribbles · Character & Asset Studio (`image.generate`)").size(12.5).strong());
                status_pill(ui, ctx.pal, &status_txt, status_col);
            });
            if pill_toggle(
                ui,
                ctx.reg,
                ctx.pal,
                "ai-scribbles-enabled",
                "Enable Scribbles image generation",
                &mut ctx.state.ai.scribbles_enabled,
                ctx.pal.accent_emerald,
            )
            .changed()
            {
                ctx.exec("ai.configure", json!({"scribbles_enabled": ctx.state.ai.scribbles_enabled}), None);
            }
            ui.horizontal(|ui| {
                ui.label(RichText::new("Connection").size(11.0).color(ctx.pal.text_muted));
                text_input(ui, ctx.reg, "ai-scribbles-conn", "Scribbles connection ID", &mut ctx.state.ai.scribbles_connection, "local-sub", 150.0);
                ui.label(RichText::new("Model").size(11.0).color(ctx.pal.text_muted));
                text_input(ui, ctx.reg, "ai-scribbles-model", "Scribbles model ID", &mut ctx.state.ai.scribbles_model, "my-image-model", 180.0);
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("ai-scribbles-bind", "check", "Bind Scribbles", "Bind Scribbles in ToolDesk").primary())
                    .clicked()
                {
                    let sc = ctx.state.ai.scribbles_connection.trim().to_owned();
                    let sm = ctx.state.ai.scribbles_model.trim().to_owned();
                    ctx.exec("ai.configure", json!({"scribbles_connection": sc, "scribbles_model": sm}), Some("Bound Scribbles model"));
                }
            });
            draw_discovered_connections(ui, ctx);
        });
    });
}

fn draw_discovered_connections(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let conns = ctx.state.cache.ai_status.get("connections").and_then(Value::as_array).cloned().unwrap_or_default();
    if conns.is_empty() {
        return;
    }
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("SharedStore:").monospace().size(10.0).color(ctx.pal.text_muted));
        for c in &conns {
            let cid = c["id"].as_str().unwrap_or_default();
            let prov = c["provider"].as_str().unwrap_or("openai");
            let first_model = c.pointer("/models/0/id").and_then(Value::as_str).unwrap_or_default().to_owned();
            let lbl = format!("{cid} ({prov})");
            let btn_id = format!("ai-pick-conn-{cid}");
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&btn_id, &lbl, "Fill connection & first model")).clicked() {
                ctx.state.ai.fidget_connection = cid.to_owned();
                ctx.state.ai.scribbles_connection = cid.to_owned();
                if !first_model.is_empty() {
                    if ctx.state.ai.fidget_model.is_empty() {
                        ctx.state.ai.fidget_model = first_model.clone();
                    }
                    if ctx.state.ai.scribbles_model.is_empty() {
                        ctx.state.ai.scribbles_model = first_model;
                    }
                }
            }
        }
    });
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
<<<<<<< HEAD
                for &(pid, plabel) in magic::craft_ai::STANDARD_PROVIDERS {
=======
                for &(pid, plabel) in magic_engine::craft_ai::STANDARD_PROVIDERS {
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
                    let btn_id = format!("ai-prov-{pid}");
                    let sel = ctx.state.ai.new_conn_provider == pid;
                    if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&btn_id, pid, plabel).selected(sel)).clicked() {
                        ctx.state.ai.new_conn_provider = pid.to_owned();
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
                text_input(ui, ctx.reg, "ai-new-text-model", "Fidget text model", &mut ctx.state.ai.new_conn_text_model, "local-chat", 110.0);
                ui.label(RichText::new("Image Model").size(10.5).color(ctx.pal.text_muted));
                text_input(ui, ctx.reg, "ai-new-img-model", "Scribbles image model", &mut ctx.state.ai.new_conn_image_model, "local-image-tool", 110.0);
            });
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("ai-save-conn", "save", "Register & Bind in SharedStore", "Write connection to craft-ai-native SharedStore").primary(),
            )
            .clicked()
            {
                let cid = ctx.state.ai.new_conn_id.trim().to_owned();
                let tm = ctx.state.ai.new_conn_text_model.trim().to_owned();
                let im = ctx.state.ai.new_conn_image_model.trim().to_owned();
                let mut payload = json!({
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
                if !tm.is_empty() {
                    payload["fidget_connection"] = json!(cid);
                    payload["fidget_model"] = json!(tm);
                    ctx.state.ai.fidget_connection = cid.clone();
                    ctx.state.ai.fidget_model = tm;
                }
                if !im.is_empty() {
                    payload["scribbles_connection"] = json!(cid);
                    payload["scribbles_model"] = json!(im);
                    ctx.state.ai.scribbles_connection = cid;
                    ctx.state.ai.scribbles_model = im;
                }
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
}

<<<<<<< HEAD
/// Fidget & Scribbles Prompt Stager (`#fidget-panel`) powered strictly by `fidget.prepare`, `fidget.stage`, and `scribbles.generate`.
pub fn draw_fidget_stager_window(egui_ctx: &egui::Context, ctx: &mut ViewCtx<'_>) {
    let mut open = ctx.state.fidget_open;
    egui::Window::new("Fidget & Scribbles · Prompt Stager")
        .id(egui::Id::unique("fidget-panel"))
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .default_width(430.0)
        .show(egui_ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add(companion_image("fidget_work", 26.0));
                ui.add(companion_image("scribbles_work", 26.0));
                let (st_lbl, st_col) = role_status_summary(ctx, "fidget");
                status_pill(ui, ctx.pal, &st_lbl, st_col);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if action_button(
                        ui,
                        ctx.reg,
                        ctx.pal,
                        BtnSpec::new("fidget-open-ai-prefs", "settings", "AI Options", "Configure Fidget & Scribbles models"),
                    )
                    .clicked()
                    {
                        ctx.state.ai.options_tab = "ai".into();
                        ctx.push(UiAction::OpenPreferences);
                    }
                });
            });

            ui.horizontal_wrapped(|ui| {
                for &(tpl, lbl) in SCRIBBLES_TEMPLATES {
                    let id = format!("fidget-tpl-{tpl}");
                    let sel = ctx.state.ai.scribbles_template == tpl;
                    if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, "Select Scribbles template").selected(sel)).clicked() {
                        ctx.state.ai.scribbles_template = tpl.into();
                    }
                }
            });

            text_input(
                ui,
                ctx.reg,
                "fidget-prompt",
                "Rough idea for Fidget",
                &mut ctx.state.fidget_prompt,
                "Describe character, puppet parts sheet, or scene…",
                ui.available_width().max(260.0),
            );

            if pill_toggle(
                ui,
                ctx.reg,
                ctx.pal,
                "fidget-seamless-inline",
                "Seamless handoff straight to Scribbles",
                &mut ctx.state.ai.fidget_seamless,
                ctx.pal.accent_sky,
            )
            .changed()
            {
                ctx.exec("ai.configure", json!({"fidget_seamless": ctx.state.ai.fidget_seamless}), None);
            }

            ui.horizontal(|ui| {
                let has_text = !ctx.state.fidget_prompt.trim().is_empty();
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::new("fidget-prepare", "eye", "Template Preview", "Compose deterministic template prompt offline").enabled(has_text),
                )
                .clicked()
                {
                    let tpl = ctx.state.ai.scribbles_template.clone();
                    let prompt = ctx.state.fidget_prompt.trim().to_owned();
                    let cid = ctx.state.selected_character_id.clone();
                    ctx.exec("fidget.prepare", json!({"kind": tpl, "prompt": prompt, "character": cid}), Some("Prepared template brief"));
                }
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::new("fidget-stage-btn", "sparkles", "Stage with Fidget", "Rephrase into structured prompt via Fidget (text.complete)")
                        .primary()
                        .enabled(has_text),
                )
                .clicked()
                {
                    let tpl = ctx.state.ai.scribbles_template.clone();
                    let prompt = ctx.state.fidget_prompt.trim().to_owned();
                    let seamless = ctx.state.ai.fidget_seamless;
                    let cid = ctx.state.selected_character_id.clone();
                    ctx.exec(
                        "fidget.stage",
                        json!({"kind": tpl, "prompt": prompt, "seamless": seamless, "character": cid}),
                        Some(if seamless { "Fidget staged & handed off to Scribbles" } else { "Staged structured prompt via Fidget" }),
                    );
                }
            });

            if !ctx.state.ai.staged_prompt.is_empty() {
                ui.separator();
                ui.label(RichText::new("Staged Scribbles Prompt (Review / Edit before Generate):").size(11.0).color(ctx.pal.accent_amber));
                let edit_resp = ui.add_sized([ui.available_width().max(260.0), 86.0], egui::TextEdit::multiline(&mut ctx.state.ai.staged_prompt));
                ctx.reg.record("fidget-staged-prompt", "Staged Prompt", &edit_resp, Some(ctx.state.ai.staged_prompt.clone()));
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::new("scribbles-run-staged", "wand", "Generate with Scribbles", "Run image.generate via configured Scribbles model").primary(),
                )
                .clicked()
                {
                    let tpl = ctx.state.ai.scribbles_template.clone();
                    let final_p = ctx.state.ai.staged_prompt.trim().to_owned();
                    let cid = ctx.state.selected_character_id.clone();
                    ctx.exec("scribbles.generate", json!({"kind": tpl, "prompt": final_p, "character": cid}), Some("Generated asset via Scribbles"));
                }
            }
        });
    if !open {
        ctx.state.fidget_open = false;
    }
=======
/// Dockable `Fidget & Scribbles` prompt stager body (`PanelId::Fidget`) powered by `fidget.prepare`, `fidget.stage`, and `scribbles.generate`.
pub fn draw_fidget_panel_body(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    crate::controls::fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal(|ui| {
            ui.add(companion_image("fidget_work", 26.0));
            ui.add(companion_image("scribbles_work", 26.0));
            let (st_lbl, st_col) = role_status_summary(ctx, "fidget");
            status_pill(ui, ctx.pal, &st_lbl, st_col);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("fidget-open-ai-prefs", "settings", "Configure Fidget & Scribbles AI models"))
                    .clicked()
                {
                    ctx.state.ai.options_tab = "ai".into();
                    ctx.push(UiAction::OpenPreferences);
                }
            });
        });

        ui.horizontal_wrapped(|ui| {
            for &(tpl, lbl) in SCRIBBLES_TEMPLATES {
                let id = format!("fidget-tpl-{tpl}");
                let sel = ctx.state.ai.scribbles_template == tpl;
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, lbl, "Select Scribbles template").selected(sel)).clicked() {
                    ctx.state.ai.scribbles_template = tpl.into();
                }
            }
        });

        text_input(
            ui,
            ctx.reg,
            "fidget-prompt",
            "Rough idea for Fidget",
            &mut ctx.state.fidget_prompt,
            "Describe character, puppet parts sheet, or scene…",
            ui.available_width().max(220.0),
        );

        if pill_toggle(
            ui,
            ctx.reg,
            ctx.pal,
            "fidget-seamless-inline",
            "Seamless handoff straight to Scribbles",
            &mut ctx.state.ai.fidget_seamless,
            ctx.pal.accent_sky,
        )
        .changed()
        {
            ctx.exec("ai.configure", json!({"fidget_seamless": ctx.state.ai.fidget_seamless}), None);
        }

        ui.horizontal_wrapped(|ui| {
            let has_text = !ctx.state.fidget_prompt.trim().is_empty();
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("fidget-prepare", "eye", "Preview", "Compose deterministic template prompt offline").enabled(has_text),
            )
            .clicked()
            {
                let tpl = ctx.state.ai.scribbles_template.clone();
                let prompt = ctx.state.fidget_prompt.trim().to_owned();
                let cid = ctx.state.selected_character_id.clone();
                ctx.exec("fidget.prepare", json!({"kind": tpl, "prompt": prompt, "character": cid}), Some("Prepared template brief"));
            }
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("fidget-stage-btn", "sparkles", "Stage", "Rephrase into structured prompt via Fidget (text.complete)").primary().enabled(has_text),
            )
            .clicked()
            {
                let tpl = ctx.state.ai.scribbles_template.clone();
                let prompt = ctx.state.fidget_prompt.trim().to_owned();
                let seamless = ctx.state.ai.fidget_seamless;
                let cid = ctx.state.selected_character_id.clone();
                ctx.exec(
                    "fidget.stage",
                    json!({"kind": tpl, "prompt": prompt, "seamless": seamless, "character": cid}),
                    Some(if seamless { "Fidget staged & handed off to Scribbles" } else { "Staged structured prompt via Fidget" }),
                );
            }
        });

        if !ctx.state.ai.staged_prompt.is_empty() {
            ui.separator();
            ui.label(RichText::new("Staged Scribbles Prompt:").size(11.0).color(ctx.pal.accent_amber));
            let edit_resp = ui.add_sized([ui.available_width().max(220.0), 82.0], egui::TextEdit::multiline(&mut ctx.state.ai.staged_prompt));
            ctx.reg.record("fidget-staged-prompt", "Staged Prompt", &edit_resp, Some(ctx.state.ai.staged_prompt.clone()));
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("scribbles-run-staged", "wand", "Generate with Scribbles", "Run image.generate via configured Scribbles model").primary(),
            )
            .clicked()
            {
                let tpl = ctx.state.ai.scribbles_template.clone();
                let final_p = ctx.state.ai.staged_prompt.trim().to_owned();
                let cid = ctx.state.selected_character_id.clone();
                ctx.exec("scribbles.generate", json!({"kind": tpl, "prompt": final_p, "character": cid}), Some("Generated asset via Scribbles"));
            }
        }
    });
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
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

/// Live floating command-palette popup beneath `🔍 Search commands` (`image_222897.png`).
pub fn draw_command_palette_popup(egui_ctx: &egui::Context, ctx: &mut ViewCtx<'_>, anchor: Rect) {
    let q = ctx.state.command_search.trim().to_ascii_lowercase();
    if q.is_empty() || !anchor.is_positive() {
        return;
    }
<<<<<<< HEAD
    let matches: Vec<&magic::CommandSpec> = magic::command_specs()
=======
    let matches: Vec<&magic_engine::CommandSpec> = magic_engine::command_specs()
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
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
