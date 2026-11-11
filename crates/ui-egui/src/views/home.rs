//! `Home` view (`#view-home`): Song Catalog grid (`All Songs`, `In Production`, `Ready to Render`,
//! `Character Rigs`), `+ Start Here` new-song card, 35mm **filmbar** preview cards, and new-project wizard.

use egui::{Align2, CornerRadius, FontId, RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui, Vec2};
use serde_json::{Value, json};

use crate::controls::{FilmbarOrientation, FilmbarStyle, paint_filmbar_frame};
use crate::state::{PickerTarget, UiAction, ViewCtx, ViewMode};
use crate::widgets::{BtnSpec, action_button, status_pill, text_input};

pub fn show(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let resp = ui.scope_builder(egui::UiBuilder::new(), |ui| {
        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.add_space(4.0);
            draw_filter_and_action_bar(ui, ctx);
            if ctx.state.home.show_new_form {
                ui.add_space(8.0);
                draw_new_project_form(ui, ctx);
            }
            ui.add_space(10.0);
            ui.label(RichText::new("PROJECTS & RECENT PRODUCTIONS").monospace().size(10.5).color(ctx.pal.text_muted));
            ui.add_space(4.0);
            draw_project_grid(ui, ctx);
            ui.add_space(12.0);
            draw_character_catalog(ui, ctx);
        });
    });
    ctx.reg.record("view-home", "Home", &resp.response, None);
}

fn draw_filter_and_action_bar(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ui.horizontal_wrapped(|ui| {
        for (id, key, label, tip) in [
            ("home-filter-all", "", "All", "Show all song projects"),
            ("home-filter-production", "production", "Production", "Filter active song films"),
            ("home-filter-render", "render", "Ready", "Filter projects with rendered exports"),
            ("home-filter-character", "character", "Rigs", "Filter character rigs"),
        ] {
            let active = ctx.state.home.filter == key;
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new(id, "layers", label, tip).selected(active)).clicked() {
                ctx.state.home.filter = key.to_owned();
            }
        }
        text_input(ui, ctx.reg, "home-search-input", "Search projects", &mut ctx.state.home.search, "Filter songs or characters…", 280.0);

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("home-refresh", "refresh", "Refresh project catalog")).clicked() {
                ctx.exec("project.catalog", json!({}), None);
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("home-open", "folder", "Open", "Open an existing .magic project")).clicked() {
                ctx.push(UiAction::PickFile {
                    target: PickerTarget::OpenProject,
                    title: "Open Magic Studio Project".into(),
                    extensions: vec!["magic".into()],
                    directory: false,
                    save: false,
                });
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("home-new", "plus", "New", "Create a new song project").primary()).clicked() {
                ctx.state.home.show_new_form = !ctx.state.home.show_new_form;
            }
        });
    });
}

fn draw_new_project_form(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let form_resp = ctx.pal.card_frame().show(ui, |ui| {
        ui.label(RichText::new("Create Song Project").size(14.5).strong().color(ctx.pal.text_primary));
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label("Title");
            text_input(ui, ctx.reg, "new-name", "Project title", &mut ctx.state.home.new_name, "My Song", 240.0);
            ui.label("Folder");
            text_input(ui, ctx.reg, "new-root", "Projects folder", &mut ctx.state.home.new_root, "~/Magic", 260.0);
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("new-root-pick", "folder", "Choose projects folder")).clicked() {
                ctx.push(UiAction::PickFile {
                    target: PickerTarget::NewProjectRoot,
                    title: "Choose Projects Folder".into(),
                    extensions: Vec::new(),
                    directory: true,
                    save: false,
                });
            }
        });
        ui.horizontal(|ui| {
            ui.label("Song");
            text_input(ui, ctx.reg, "new-audio", "Song audio file", &mut ctx.state.home.new_audio, "/path/to/song.wav", 420.0);
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("new-audio-pick", "music", "Audio", "Pick song audio file")).clicked() {
                ctx.push(UiAction::PickFile {
                    target: PickerTarget::NewProjectAudio,
                    title: "Select Song Audio".into(),
                    extensions: vec!["wav".into(), "flac".into(), "mp3".into(), "m4a".into(), "ogg".into()],
                    directory: false,
                    save: false,
                });
            }
        });
        ui.collapsing("Storyboard Title-Page Prompts (Optional)", |ui| {
            ui.horizontal(|ui| {
                ui.label("D01 Style");
                text_input(ui, ctx.reg, "new-wizard-style", "Visual style", &mut ctx.state.home.wizard_style, "Paper cutout storybook", 220.0);
                ui.label("L01 World");
                text_input(ui, ctx.reg, "new-wizard-world", "Master world", &mut ctx.state.home.wizard_world, "Lantern harbor at dusk", 220.0);
            });
            ui.horizontal(|ui| {
                ui.label("C01 Cast");
                text_input(ui, ctx.reg, "new-wizard-character", "Lead character", &mut ctx.state.home.wizard_character, "Curious fox conductor", 220.0);
                ui.label("M01 Motion");
                text_input(ui, ctx.reg, "new-wizard-motion", "Song-action motion", &mut ctx.state.home.wizard_motion, "Chalk star arcs on chorus", 220.0);
            });
        });
        ui.horizontal(|ui| {
            let can_create = !ctx.state.home.new_name.trim().is_empty() && !ctx.state.home.new_audio.trim().is_empty();
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("new-create", "sparkles", "Create", "Transcribe song, sync FilmCraft, and build storyboard").primary().enabled(can_create),
            )
            .clicked()
            {
                let params = build_new_project_payload(&ctx.state.home);
                ctx.state.home.show_new_form = false;
                ctx.exec("project.new", params, Some("Created song project"));
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("new-cancel", "close", "Cancel", "Close new-project form")).clicked() {
                ctx.state.home.show_new_form = false;
            }
        });
    });
    ctx.reg.record("new-form", "New Project Form", &form_resp.response, None);
}

fn build_new_project_payload(form: &crate::state::HomeFormState) -> Value {
    let root = if form.new_root.trim().is_empty() {
        std::env::var("HOME").map_or_else(|_| ".".into(), |h| format!("{h}/Magic"))
    } else {
        form.new_root.trim().to_owned()
    };
    let mut wiz = json!({});
    if !form.wizard_style.trim().is_empty() {
        wiz["style"] = json!(form.wizard_style.trim());
    }
    if !form.wizard_world.trim().is_empty() {
        wiz["world"] = json!(form.wizard_world.trim());
    }
    if !form.wizard_character.trim().is_empty() {
        wiz["character"] = json!(form.wizard_character.trim());
    }
    if !form.wizard_motion.trim().is_empty() {
        wiz["motion"] = json!(form.wizard_motion.trim());
    }
    let mut payload = json!({
        "name": form.new_name.trim(),
        "root": root,
        "audio": form.new_audio.trim(),
    });
    if wiz.as_object().is_some_and(|m| !m.is_empty()) {
        payload["wizard"] = wiz;
    }
    payload
}

fn draw_project_grid(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let projects =
        ctx.state.cache.catalog.get("projects").or_else(|| ctx.state.cache.catalog.get("recent")).and_then(Value::as_array).cloned().unwrap_or_default();
    let query = ctx.state.home.search.trim().to_ascii_lowercase();

    let recent_resp = ui.horizontal_wrapped(|ui| {
        let col_w = ((ui.available_width() - 20.0) / 3.0).clamp(260.0, 420.0);

        // Card 0: "+ START HERE · New Song Project"
        ui.allocate_ui(Vec2::new(col_w, 236.0), |ui| {
            egui::Frame::new()
                .fill(ctx.pal.bg_panel)
                .corner_radius(CornerRadius::same(10))
                .inner_margin(egui::Margin::same(12))
                .stroke(Stroke::new(1.2, ctx.pal.accent_emerald.gamma_multiply(0.55)))
                .show(ui, |ui| {
                    let (hero, hresp) = ui.allocate_exact_size(Vec2::new(col_w - 26.0, 118.0), Sense::click());
                    ui.painter().rect(hero, CornerRadius::same(8), ctx.pal.accent_emerald_bg, Stroke::new(1.0, ctx.pal.border), StrokeKind::Inside);
                    ui.painter().circle_stroke(hero.center(), 18.0, Stroke::new(1.5, ctx.pal.accent_emerald));
                    ui.painter().text(hero.center(), Align2::CENTER_CENTER, "+", FontId::proportional(18.0), ctx.pal.accent_emerald);
                    if hresp.clicked() {
                        ctx.state.home.show_new_form = true;
                    }
                    ui.label(RichText::new("New Song Project").size(13.5).strong().color(ctx.pal.text_primary));
                    status_pill(ui, ctx.pal, "Audio · Lyrics · Storyboard · Film", ctx.pal.accent_emerald);
                    ui.horizontal(|ui| {
                        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("home-choose-audio", "music", "Audio", "Pick song audio")).clicked() {
                            ctx.state.home.show_new_form = true;
                            ctx.push(UiAction::PickFile {
                                target: PickerTarget::NewProjectAudio,
                                title: "Select Song Audio".into(),
                                extensions: vec!["wav".into(), "flac".into(), "mp3".into(), "m4a".into()],
                                directory: false,
                                save: false,
                            });
                        }
                        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("home-new-slot-hero", "plus", "Create", "Start a new song project").primary())
                            .clicked()
                        {
                            ctx.state.home.show_new_form = true;
                        }
                    });
                });
        });

        for (idx, item) in projects.iter().enumerate() {
            let path = item["path"].as_str().unwrap_or_default();
            let name = item["name"].as_str().unwrap_or_else(|| path.rsplit('/').next().unwrap_or("Untitled"));
            if !query.is_empty() && !name.to_ascii_lowercase().contains(&query) && !path.to_ascii_lowercase().contains(&query) {
                continue;
            }
            ui.allocate_ui(Vec2::new(col_w, 236.0), |ui| {
                let card_resp = ctx.pal.card_frame().show(ui, |ui| {
                    let (thumb, tresp) = ui.allocate_exact_size(Vec2::new(col_w - 24.0, 118.0), Sense::click());
                    let inner = paint_filmbar_frame(
                        ui,
                        thumb,
                        ctx.pal,
                        FilmbarStyle { orientation: FilmbarOrientation::Horizontal, selected: false, hovered: tresp.hovered(), timecode: Some("16:9") },
                    );
                    ui.painter().text(inner.center(), Align2::CENTER_CENTER, name, FontId::proportional(14.0), ctx.pal.text_primary);
                    if tresp.clicked() {
                        ctx.exec("file.open", json!({"path": path}), Some("Opened project"));
                    }
                    ui.label(RichText::new(name).size(13.5).strong().color(ctx.pal.text_primary));
                    ui.label(RichText::new(path).size(10.0).color(ctx.pal.text_muted));
                    ui.horizontal(|ui| {
                        status_pill(ui, ctx.pal, "Song · audio", ctx.pal.accent_emerald);
                        status_pill(ui, ctx.pal, "Studio · editor", ctx.pal.accent_amber);
                    });
                    ui.horizontal(|ui| {
                        let anim_id = format!("home-animate-card-{idx}");
                        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new(&anim_id, "user", "Animate", "Open in Animations")).clicked() {
                            ctx.exec("file.open", json!({"path": path}), None);
                            ctx.push(UiAction::SwitchView(ViewMode::Animation));
                        }
                        let stud_id = format!("home-studio-card-{idx}");
                        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new(&stud_id, "film", "Studio", "Open in FilmCraft Studio")).clicked() {
                            ctx.exec("file.open", json!({"path": path}), None);
                            ctx.push(UiAction::SwitchView(ViewMode::Studio));
                        }
                        let open_id = format!("home-open-card-{idx}");
                        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new(&open_id, "folder", "Open", "Open project").primary()).clicked() {
                            ctx.exec("file.open", json!({"path": path}), Some("Opened project"));
                        }
                    });
                });
                ctx.reg.record(format!("home-card-{idx}"), name, &card_resp.response, Some(path.to_owned()));
            });
        }
    });
    ctx.reg.record("recent", "Recent Projects", &recent_resp.response, None);
}

fn draw_character_catalog(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let chars = ctx.state.cache.catalog.get("characters").and_then(Value::as_array).cloned().unwrap_or_default();
    if chars.is_empty() {
        return;
    }
    ui.label(RichText::new("CHARACTER RIGS").monospace().size(10.5).color(ctx.pal.text_muted));
    let cat_resp = ui.horizontal_wrapped(|ui| {
        for (idx, ch) in chars.iter().enumerate() {
            let cname = ch["name"].as_str().or_else(|| ch["id"].as_str()).unwrap_or("Character");
            let cid = ch["id"].as_str().unwrap_or(cname).to_owned();
            let proj = ch["project"].as_str().unwrap_or_default().to_owned();
            ctx.pal.card_frame().show(ui, |ui| {
                ui.label(RichText::new(cname).strong().color(ctx.pal.text_primary));
                let btn_id = format!("home-char-animate-{idx}");
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new(&btn_id, "wand", "Rig", "Open character in Animations")).clicked() {
                    if !proj.is_empty() {
                        ctx.exec("file.open", json!({"path": proj}), None);
                    }
                    ctx.push(UiAction::SelectCharacter { id: cid, jump_animation: true });
                }
            });
        }
    });
    ctx.reg.record("character-catalog", "Character Catalog", &cat_resp.response, None);
}
