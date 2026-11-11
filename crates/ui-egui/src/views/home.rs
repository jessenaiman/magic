//! `Home` view (`#view-home`): Song Catalog grid (`All Songs`, `In Production`, `Ready to Render`,
//! `Character Rigs`), `+ START HERE` new-song card, 35mm **filmbar** preview cards backed strictly
//! by real rendered Craft frames (`film/frames`, `storyboard/.preview`, `lettering/.preview`,
//! `motion/.preview`, `.pcraft`/`.ecproj` character renders), and Craft file badges.

use egui::{
    Align, Align2, Color32, CornerRadius, FontId, Layout, Pos2, Rect, RichText, ScrollArea, Sense,
    Stroke, StrokeKind, Ui, Vec2,
};
use serde_json::{Value, json};

use crate::controls::{
    FilmbarOrientation, FilmbarStyle, badge_accent_color, paint_craft_doc_card,
    paint_filmbar_frame, paint_local_image,
};
use crate::state::{PickerTarget, UiAction, ViewCtx, ViewMode};
use crate::widgets::{BtnSpec, action_button, status_pill, text_input};

pub fn show(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.add_space(2.0);
        draw_filter_and_action_bar(ui, ctx);
        if ctx.state.home.show_new_form {
            ui.add_space(8.0);
            draw_new_project_form(ui, ctx);
        }
        ui.add_space(10.0);
        if ctx.state.home.filter == "character" {
            draw_character_catalog(ui, ctx);
        } else {
            ui.label(
                RichText::new("PROJECTS & CRAFT PRODUCTIONS")
                    .monospace()
                    .size(10.5)
                    .color(ctx.pal.text_muted),
            );
            ui.add_space(6.0);
            draw_project_grid(ui, ctx);
            ui.add_space(14.0);
            draw_character_catalog(ui, ctx);
        }
    });
}

fn catalog_projects(ctx: &ViewCtx<'_>) -> Vec<Value> {
    ctx.state
        .cache
        .catalog
        .get("projects")
        .or_else(|| ctx.state.cache.catalog.get("recent"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn catalog_characters(ctx: &ViewCtx<'_>) -> Vec<Value> {
    ctx.state
        .cache
        .catalog
        .get("characters")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn draw_filter_and_action_bar(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let projects_len = catalog_projects(ctx).len();
    let chars_len = catalog_characters(ctx).len();
    let all_lbl = format!("All Songs ({projects_len})");
    let rigs_lbl = format!("Character Rigs ({chars_len})");

    ui.horizontal_wrapped(|ui| {
        for (id, key, label, tip) in [
            ("home-filter-all", "", all_lbl.as_str(), "Show all song projects"),
            ("home-filter-production", "production", "In Production", "Filter active song films"),
            ("home-filter-render", "render", "Ready to Render", "Filter projects with FilmCraft timelines"),
            ("home-filter-character", "character", rigs_lbl.as_str(), "Browse PhotoCraft & EffectCraft character rigs"),
        ] {
            let active = ctx.state.home.filter == key;
            let spec = if active {
                BtnSpec::label_only(id, label, tip).primary()
            } else {
                BtnSpec::label_only(id, label, tip)
            };
            if action_button(ui, ctx.reg, ctx.pal, spec).clicked() {
                ctx.state.home.filter = key.to_owned();
            }
        }

        let search_w = (ui.available_width() - 250.0).clamp(180.0, 480.0);
        text_input(
            ui,
            ctx.reg,
            "home-search-input",
            "Search projects",
            &mut ctx.state.home.search,
            "Filter songs or Craft files…",
            search_w,
        );

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::icon_only("home-refresh", "refresh", "Refresh project catalog"),
            )
            .clicked()
            {
                ctx.exec("project.catalog", json!({}), None);
            }
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::label_only("home-open", "Open .magic", "Open an existing .magic project"),
            )
            .clicked()
            {
                ctx.push(UiAction::PickFile {
                    target: PickerTarget::OpenProject,
                    title: "Open Magic Studio Project".into(),
                    extensions: vec!["magic".into()],
                    directory: false,
                    save: false,
                });
            }
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("home-new", "plus", "New Song Project", "Create a new song project").primary(),
            )
            .clicked()
            {
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
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("new-audio-pick", "music", "Choose Audio…", "Pick song audio file")).clicked() {
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
                BtnSpec::new("new-create", "sparkles", "Create Project", "Transcribe song, sync FilmCraft, and build storyboard")
                    .primary()
                    .enabled(can_create),
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

fn draw_start_here_card(ui: &mut Ui, ctx: &mut ViewCtx<'_>, col_w: f32) {
    ui.allocate_ui(Vec2::new(col_w, 298.0), |ui| {
        egui::Frame::new()
            .fill(ctx.pal.bg_panel)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(egui::Margin::same(12))
            .stroke(Stroke::new(1.2, ctx.pal.accent_emerald.gamma_multiply(0.55)))
            .show(ui, |ui| {
                let thumb_w = (col_w - 26.0).max(180.0);
                let (hero, hresp) = ui.allocate_exact_size(Vec2::new(thumb_w, 152.0), Sense::click());
                ui.painter().rect(
                    hero,
                    CornerRadius::same(8),
                    Color32::from_rgb(0x0b, 0x16, 0x18),
                    Stroke::new(1.0, ctx.pal.border),
                    StrokeKind::Inside,
                );
                ui.painter()
                    .circle_filled(hero.center(), 46.0, ctx.pal.accent_emerald.gamma_multiply(0.08));
                let plus_c = Pos2::new(hero.center().x, hero.center().y - 8.0);
                ui.painter()
                    .circle_stroke(plus_c, 19.0, Stroke::new(1.5, ctx.pal.accent_emerald));
                ui.painter()
                    .text(plus_c, Align2::CENTER_CENTER, "+", FontId::proportional(20.0), ctx.pal.accent_emerald);
                ui.painter().text(
                    Pos2::new(hero.center().x, plus_c.y + 32.0),
                    Align2::CENTER_CENTER,
                    "START HERE",
                    FontId::monospace(10.0),
                    ctx.pal.accent_emerald,
                );
                if hresp.clicked() {
                    ctx.state.home.show_new_form = true;
                }
                ui.add_space(4.0);
                ui.label(RichText::new("New Song Project").size(14.0).strong().color(ctx.pal.text_primary));
                ui.label(
                    RichText::new("Creates .designcraft, .ecproj, and .fcproj Craft project files")
                        .size(11.0)
                        .color(ctx.pal.text_muted),
                );
                status_pill(ui, ctx.pal, ".designcraft · .pcraft · .ecproj · .fcproj", ctx.pal.accent_emerald);
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if action_button(
                        ui,
                        ctx.reg,
                        ctx.pal,
                        BtnSpec::label_only("home-choose-audio", "Choose Audio…", "Pick song audio"),
                    )
                    .clicked()
                    {
                        ctx.state.home.show_new_form = true;
                        ctx.push(UiAction::PickFile {
                            target: PickerTarget::NewProjectAudio,
                            title: "Select Song Audio".into(),
                            extensions: vec!["wav".into(), "flac".into(), "mp3".into(), "m4a".into()],
                            directory: false,
                            save: false,
                        });
                    }
                    if action_button(
                        ui,
                        ctx.reg,
                        ctx.pal,
                        BtnSpec::new("home-new-slot-hero", "plus", "New Project", "Start a new song project").primary(),
                    )
                    .clicked()
                    {
                        ctx.state.home.show_new_form = true;
                    }
                });
            });
    });
}

fn draw_project_grid(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let projects = catalog_projects(ctx);
    let query = ctx.state.home.search.trim().to_ascii_lowercase();
    let filter = ctx.state.home.filter.clone();

    let recent_resp = ui.horizontal_wrapped(|ui| {
        let avail = ui.available_width();
        let cols = if avail >= 1060.0 { 3.0 } else if avail >= 680.0 { 2.0 } else { 1.0 };
        let col_w = ((avail - (cols - 1.0) * 10.0) / cols).clamp(260.0, 520.0);

        draw_start_here_card(ui, ctx, col_w);

        for (idx, item) in projects.iter().enumerate() {
            let path = item["path"].as_str().unwrap_or_default();
            let dir_str = item["directory"].as_str().unwrap_or(path);
            let name = item["name"]
                .as_str()
                .unwrap_or_else(|| path.rsplit('/').next().unwrap_or("Untitled"));
            let has_video = item["video_project"].as_str().is_some();
            if filter == "render" && !has_video {
                continue;
            }
            if !query.is_empty()
                && !name.to_ascii_lowercase().contains(&query)
                && !dir_str.to_ascii_lowercase().contains(&query)
            {
                continue;
            }
            let cover = item["cover"].as_str();
            let char_count = item["characters"].as_array().map_or(0, Vec::len);
            let badges: Vec<String> = item["craft_badges"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|b| b.as_str().map(str::to_owned))
                .collect();

            ui.allocate_ui(Vec2::new(col_w, 298.0), |ui| {
                let card_resp = ctx.pal.card_frame().show(ui, |ui| {
                    let thumb_w = (col_w - 24.0).max(180.0);
                    let (thumb, tresp) = ui.allocate_exact_size(Vec2::new(thumb_w, 152.0), Sense::click());
                    let inner = paint_filmbar_frame(
                        ui,
                        thumb,
                        ctx.pal,
                        FilmbarStyle {
                            orientation: FilmbarOrientation::Horizontal,
                            selected: false,
                            hovered: tresp.hovered(),
                            timecode: cover.map(|_| "16:9"),
                        },
                    );
                    if !paint_local_image(ui, inner, cover, ctx.pal.bg_stage, 3) {
                        let primary_badge = badges.first().map_or("MAGIC", String::as_str);
                        let file_name = path.rsplit('/').next().unwrap_or(name);
                        paint_craft_doc_card(
                            ui,
                            inner,
                            ctx.pal,
                            primary_badge,
                            file_name,
                            "No rendered frame on disk yet — open project to render",
                        );
                    }
                    if tresp.clicked() {
                        ctx.exec("file.open", json!({"path": path}), Some("Opened project"));
                    }
                    ui.add_space(4.0);
                    ui.label(RichText::new(name).size(14.0).strong().color(ctx.pal.text_primary));
                    ui.label(RichText::new(dir_str).size(10.0).color(ctx.pal.text_muted));
                    ui.horizontal_wrapped(|ui| {
                        for b in &badges {
                            status_pill(ui, ctx.pal, b, badge_accent_color(ctx.pal, b));
                        }
                        if badges.is_empty() {
                            status_pill(ui, ctx.pal, ".magic", ctx.pal.text_muted);
                        }
                        if char_count > 0 {
                            status_pill(ui, ctx.pal, &format!("{char_count} rigs"), ctx.pal.accent_violet);
                        }
                    });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        let anim_id = format!("home-animate-card-{idx}");
                        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&anim_id, "Animate", "Open in Animations")).clicked() {
                            ctx.exec("file.open", json!({"path": path}), None);
                            ctx.push(UiAction::SwitchView(ViewMode::Animation));
                        }
                        let stud_id = format!("home-studio-card-{idx}");
                        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&stud_id, "Studio", "Open in FilmCraft Studio")).clicked() {
                            ctx.exec("file.open", json!({"path": path}), None);
                            ctx.push(UiAction::SwitchView(ViewMode::Studio));
                        }
                        let open_id = format!("home-open-card-{idx}");
                        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&open_id, "Open Project", "Open project").primary()).clicked() {
                            ctx.exec("file.open", json!({"path": path}), Some("Opened project"));
                        }
                    });
                });
                ctx.reg
                    .record(format!("home-card-{idx}"), name, &card_resp.response, Some(path.to_owned()));
            });
        }
    });
    ctx.reg.record("recent", "Recent Projects", &recent_resp.response, None);
}

fn draw_character_catalog(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let chars = catalog_characters(ctx);
    if chars.is_empty() {
        return;
    }
    ui.label(RichText::new("CHARACTER CRAFT RIGS (.PCRAFT / .ECPROJ)").monospace().size(10.5).color(ctx.pal.text_muted));
    ui.add_space(4.0);
    let cat_resp = ui.horizontal_wrapped(|ui| {
        for (idx, ch) in chars.iter().enumerate() {
            let cname = ch["name"].as_str().or_else(|| ch["id"].as_str()).unwrap_or("Character");
            let cid = ch["id"].as_str().unwrap_or(cname).to_owned();
            let proj = ch["project"].as_str().unwrap_or_default().to_owned();
            let preview = ch["preview"].as_str();
            let has_pcraft = ch["pcraft"].as_bool() == Some(true);
            let has_ecproj = ch["ecproj"].as_bool() == Some(true);
            ui.allocate_ui(Vec2::new(204.0, 158.0), |ui| {
                ctx.pal.card_frame().show(ui, |ui| {
                    let (img_rect, _) = ui.allocate_exact_size(Vec2::new(180.0, 82.0), Sense::hover());
                    if !paint_local_image(ui, img_rect, preview, ctx.pal.bg_stage, 6) {
                        paint_craft_doc_card(ui, img_rect, ctx.pal, "PC", cname, "Unrendered character asset");
                    }
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(cname).size(12.0).strong().color(ctx.pal.text_primary));
                        if has_pcraft {
                            status_pill(ui, ctx.pal, ".pcraft", ctx.pal.accent_sky);
                        }
                        if has_ecproj {
                            status_pill(ui, ctx.pal, ".ecproj", ctx.pal.accent_violet);
                        }
                    });
                    let btn_id = format!("home-char-animate-{idx}");
                    if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new(&btn_id, "wand", "Open Rig", "Open character in Animations")).clicked() {
                        if !proj.is_empty() {
                            ctx.exec("file.open", json!({"path": proj}), None);
                        }
                        ctx.push(UiAction::SelectCharacter {
                            id: cid,
                            jump_animation: true,
                        });
                    }
                });
            });
        }
    });
    ctx.reg.record("character-catalog", "Character Catalog", &cat_resp.response, None);
}
