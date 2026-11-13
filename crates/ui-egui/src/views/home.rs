//! `Home` view (`#view-home`): Song Catalog grid matching `01-home-song-catalog.jpg`
//! (`All Songs`, `In Production`, `Ready to Render`, `Character Rigs`), dashed-mint
//! `+ START HERE — New Song Project` hero card, true `16:9` horizontal 35mm **filmbar** preview
//! cards backed by real rendered Craft frames, and aspect-preserving character-rig cards.

use egui::{Align, Align2, Color32, CornerRadius, FontId, Layout, Pos2, RichText, ScrollArea, Sense, Stroke, Ui, Vec2};
use serde_json::{Value, json};

use crate::controls::{FilmbarOrientation, FilmbarStyle, paint_craft_badge, paint_craft_doc_card, paint_dashed_rect, paint_filmbar_frame, paint_local_image};
use crate::state::{PickerTarget, UiAction, ViewCtx, ViewMode};
use crate::widgets::{BtnSpec, action_button, status_pill, text_input};

const CHAR_CARD_W: f32 = 224.0;
const CHAR_CARD_H: f32 = 228.0;

enum GridSlot<'a> {
    StartHere,
    Project(usize, &'a Value),
}

pub fn show(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.vertical(|ui| {
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
                ui.label(RichText::new("PROJECTS & RECENT PRODUCTIONS").monospace().size(10.5).color(ctx.pal.text_muted));
                ui.add_space(6.0);
                draw_project_grid(ui, ctx);
                ui.add_space(16.0);
                draw_character_catalog(ui, ctx);
            }
        });
    });
}

fn catalog_projects(ctx: &ViewCtx<'_>) -> Vec<Value> {
    ctx.state.cache.catalog.get("projects").or_else(|| ctx.state.cache.catalog.get("recent")).and_then(Value::as_array).cloned().unwrap_or_default()
}

fn catalog_characters(ctx: &ViewCtx<'_>) -> Vec<Value> {
    ctx.state.cache.catalog.get("characters").and_then(Value::as_array).cloned().unwrap_or_default()
}

fn humanize_slug(raw: &str) -> String {
    if raw.contains(' ') {
        return raw.to_owned();
    }
    raw.split(['-', '_'])
        .filter(|s| !s.is_empty())
        .map(|part| {
            let mut ch = part.chars();
            match ch.next() {
                Some(f) => format!("{}{}", f.to_uppercase(), ch.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn clean_character_name(ch: &Value) -> String {
    let raw = ch["name"].as_str().or_else(|| ch["id"].as_str()).unwrap_or("Character");
    if !matches!(raw.to_ascii_lowercase().as_str(), "parts" | "front" | "side" | "back" | "sprites" | "cutout") {
        return humanize_slug(raw);
    }
    if let Some(grp) = ch["group"].as_str().filter(|g| !g.is_empty() && !g.eq_ignore_ascii_case("animations")) {
        return humanize_slug(grp);
    }
    for key in ["folder", "preview", "path"] {
        if let Some(p) = ch[key].as_str() {
            let segs: Vec<&str> = p
                .split(['/', '\\'])
                .filter(|s| {
                    !s.is_empty()
                        && !s.contains('.')
                        && !matches!(s.to_ascii_lowercase().as_str(), "assets" | "characters" | "parts" | "sprites" | "frames" | "front" | "source")
                })
                .collect();
            if let Some(last) = segs.last() {
                return humanize_slug(last);
            }
        }
    }
    humanize_slug(raw)
}

fn elide_middle(raw: &str, max_chars: usize) -> String {
    let total = raw.chars().count();
    if total <= max_chars || max_chars < 12 {
        return raw.to_owned();
    }
    let keep = (max_chars - 1) / 2;
    let head: String = raw.chars().take(keep).collect();
    let tail: String = raw.chars().skip(total.saturating_sub(keep)).collect();
    format!("{head}…{tail}")
}

fn draw_filter_and_action_bar(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let projects_len = catalog_projects(ctx).len();
    let chars_len = catalog_characters(ctx).len();
    let all_lbl = format!("All Songs ({projects_len})");
    let rigs_lbl = if chars_len > 0 { format!("Character Rigs ({chars_len})") } else { "Character Rigs".into() };

    ui.horizontal_wrapped(|ui| {
        for (id, key, label, tip) in [
            ("home-filter-all", "", all_lbl.as_str(), "Show all song projects"),
            ("home-filter-production", "production", "In Production", "Filter active song films"),
            ("home-filter-render", "render", "Ready to Render", "Filter projects with FilmCraft timelines"),
            ("home-filter-character", "character", rigs_lbl.as_str(), "Browse PhotoCraft & EffectCraft character rigs"),
        ] {
            let active = ctx.state.home.filter == key;
            let spec = if active { BtnSpec::label_only(id, label, tip).primary() } else { BtnSpec::label_only(id, label, tip) };
            if action_button(ui, ctx.reg, ctx.pal, spec).clicked() {
                ctx.state.home.filter = key.to_owned();
            }
        }

        let search_w = (ui.available_width() - 292.0).clamp(140.0, 560.0);
        text_input(ui, ctx.reg, "home-search-input", "Search projects", &mut ctx.state.home.search, "Filter songs or Craft files…", search_w);

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("home-refresh", "refresh", "Refresh project catalog")).clicked() {
                ctx.exec("project.catalog", json!({}), None);
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("home-open", "folder", "Open", "Open a .magic song or animation project")).clicked() {
                ctx.push(UiAction::PickFile {
                    target: PickerTarget::OpenProject,
                    title: "Open Magic Studio Project (.magic)".into(),
                    extensions: vec!["magic".into()],
                    directory: false,
                    save: false,
                });
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("home-new", "plus", "New Song Project", "Create a new song project").primary()).clicked() {
                ctx.state.home.show_new_form = !ctx.state.home.show_new_form;
            }
        });
    });
}

fn draw_new_project_form(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let full_w = ui.available_width();
    let form_resp = ctx.pal.card_frame().show(ui, |ui| {
        ui.set_min_width((full_w - 24.0).max(320.0));
        ui.vertical(|ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Create Song Project").size(14.5).strong().color(ctx.pal.text_primary));
                ui.label(RichText::new("Initializes .designcraft, .ecproj, and .fcproj Craft project files").size(11.0).color(ctx.pal.text_muted));
            });
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label("Title");
                text_input(ui, ctx.reg, "new-name", "Project title", &mut ctx.state.home.new_name, "My Song", 240.0);
                ui.label("Folder");
                text_input(ui, ctx.reg, "new-root", "Projects folder", &mut ctx.state.home.new_root, "~/Magic", 280.0);
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
            ui.horizontal_wrapped(|ui| {
                ui.label("Song");
                text_input(ui, ctx.reg, "new-audio", "Song audio file", &mut ctx.state.home.new_audio, "/path/to/song.wav", 420.0);
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("new-audio-pick", "music", "Choose Audio…", "Pick song audio file")).clicked() {
                    ctx.push(UiAction::PickFile {
                        target: PickerTarget::NewProjectAudio,
                        title: "Select Song Audio".into(),
                        extensions: magic_util::media::decode_audio_extensions().iter().map(|s| (*s).to_owned()).collect(),
                        directory: false,
                        save: false,
                    });
                }
            });
            ui.collapsing("Storyboard Title-Page Prompts (Optional)", |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label("D01 Style");
                    text_input(ui, ctx.reg, "new-wizard-style", "Visual style", &mut ctx.state.home.wizard_style, "Paper cutout storybook", 220.0);
                    ui.label("L01 World");
                    text_input(ui, ctx.reg, "new-wizard-world", "Master world", &mut ctx.state.home.wizard_world, "Lantern harbor at dusk", 220.0);
                });
                ui.horizontal_wrapped(|ui| {
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
                    ctx.push(UiAction::SwitchView(ViewMode::Project));
                }
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("new-cancel", "close", "Cancel", "Close new-project form")).clicked() {
                    ctx.state.home.show_new_form = false;
                }
            });
        });
    });
    ctx.reg.record("new-form", "New Project Form", &form_resp.response, None);
}

fn build_new_project_payload(form: &crate::state::HomeFormState) -> Value {
    let root = if form.new_root.trim().is_empty() {
        magic_engine::project_settings::root()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| std::env::var("HOME").map_or_else(|_| ".".into(), |h| format!("{h}/Magic")))
    } else {
        magic_engine::workspace::expand_tilde(form.new_root.trim()).to_string_lossy().into_owned()
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

fn project_card_dimensions(col_w: f32) -> (f32, f32, f32) {
    let inner_w = (col_w - 26.0).max(180.0);
    let hero_h = ((inner_w - 5.0) * 9.0 / 16.0 + 22.0).clamp(186.0, 286.0);
    let card_h = hero_h + 132.0;
    (inner_w, hero_h, card_h)
}

fn draw_start_here_card(ui: &mut Ui, ctx: &mut ViewCtx<'_>, col_w: f32) {
    let (inner_w, hero_h, card_h) = project_card_dimensions(col_w);
    ui.allocate_ui_with_layout(Vec2::new(col_w, card_h), Layout::top_down(Align::Min), |ui| {
        egui::Frame::new()
            .fill(ctx.pal.bg_panel)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(egui::Margin::same(12))
            .stroke(Stroke::new(1.0, ctx.pal.border))
            .show(ui, |ui| {
                ui.set_width(inner_w);
                ui.vertical(|ui| {
                    let (hero, hresp) = ui.allocate_exact_size(Vec2::new(inner_w, hero_h - 18.0), Sense::click());
                    ui.painter().rect_filled(hero, CornerRadius::same(8), Color32::from_rgb(0x0b, 0x16, 0x18));
                    paint_dashed_rect(ui, hero, Stroke::new(1.4, ctx.pal.accent_emerald.gamma_multiply(0.82)), 8.0, 5.0);
                    ui.painter().circle_filled(hero.center(), 54.0, ctx.pal.accent_emerald.gamma_multiply(0.08));
                    let plus_c = Pos2::new(hero.center().x, hero.center().y - 9.0);
                    ui.painter().circle_stroke(plus_c, 22.0, Stroke::new(1.6, ctx.pal.accent_emerald));
                    ui.painter().text(plus_c, Align2::CENTER_CENTER, "+", FontId::proportional(21.0), ctx.pal.accent_emerald);
                    ui.painter().text(
                        Pos2::new(hero.center().x, plus_c.y + 34.0),
                        Align2::CENTER_CENTER,
                        "START HERE",
                        FontId::monospace(10.5),
                        ctx.pal.accent_emerald,
                    );
                    if hresp.clicked() {
                        ctx.state.home.show_new_form = true;
                    }
                    ui.add_space(7.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("+ START HERE").size(13.0).strong().color(ctx.pal.accent_emerald));
                        ui.label(RichText::new("— New Song Project").size(13.5).strong().color(ctx.pal.text_primary));
                    });
                    ui.label(RichText::new("Drop an .mp3/.wav track or start a blank song file").size(11.0).color(ctx.pal.text_muted));
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        for step in ["Audio", "Lyrics", "Storyboard", "Film"] {
                            status_pill(ui, ctx.pal, step, ctx.pal.accent_emerald);
                        }
                    });
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        let half = ((inner_w - 6.0) * 0.5).max(80.0);
                        ui.allocate_ui_with_layout(Vec2::new(half, 28.0), Layout::top_down_justified(Align::Min), |ui| {
                            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only("home-choose-audio", "Choose Audio…", "Pick song audio")).clicked() {
                                ctx.state.home.show_new_form = true;
                                ctx.push(UiAction::PickFile {
                                    target: PickerTarget::NewProjectAudio,
                                    title: "Select Song Audio".into(),
                                    extensions: vec!["wav".into(), "flac".into(), "mp3".into(), "m4a".into()],
                                    directory: false,
                                    save: false,
                                });
                            }
                        });
                        ui.allocate_ui_with_layout(Vec2::new(half, 28.0), Layout::top_down_justified(Align::Min), |ui| {
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
            });
    });
}

#[allow(clippy::cognitive_complexity)]
fn draw_single_project_card(ui: &mut Ui, ctx: &mut ViewCtx<'_>, col_w: f32, idx: usize, item: &Value) {
    let path = item["path"].as_str().unwrap_or_default();
    let dir_str = item["directory"].as_str().unwrap_or(path);
    let name = item["name"].as_str().unwrap_or_else(|| path.rsplit('/').next().unwrap_or("Untitled"));
    let cover = item["cover"].as_str();
    let badges: Vec<String> = item["craft_badges"].as_array().into_iter().flatten().filter_map(|b| b.as_str().map(str::to_owned)).collect();

    let (inner_w, hero_h, card_h) = project_card_dimensions(col_w);
    ui.allocate_ui_with_layout(Vec2::new(col_w, card_h), Layout::top_down(Align::Min), |ui| {
        let card_resp = ctx.pal.card_frame().show(ui, |ui| {
            ui.set_width(inner_w);
            ui.vertical(|ui| {
                let (thumb, tresp) = ui.allocate_exact_size(Vec2::new(inner_w, hero_h), Sense::click());
                let inner = paint_filmbar_frame(
                    ui,
                    thumb,
                    ctx.pal,
                    FilmbarStyle { orientation: FilmbarOrientation::Horizontal, selected: false, hovered: tresp.hovered(), timecode: None },
                );
                if !paint_local_image(ui, inner, cover, ctx.pal.bg_stage, 3) {
                    let primary_badge = badges.first().map_or("MAGIC", String::as_str);
                    let file_name = path.rsplit('/').next().unwrap_or(name);
                    paint_craft_doc_card(ui, inner, ctx.pal, primary_badge, file_name, "No rendered frame on disk yet — open project to render");
                }
                if tresp.clicked() {
                    ctx.exec("file.open", json!({"path": path}), Some("Opened project"));
                    ctx.push(UiAction::SwitchView(ViewMode::Project));
                }

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(elide_middle(name, 26)).size(14.0).strong().color(ctx.pal.text_primary));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        if badges.is_empty() {
                            paint_craft_badge(ui, ctx.pal, ".magic", false);
                        } else {
                            for b in badges.iter().take(2) {
                                paint_craft_badge(ui, ctx.pal, b, false);
                            }
                        }
                    });
                });

                let max_dir_chars = ((inner_w / 6.2) as usize).max(28);
                ui.label(RichText::new(elide_middle(dir_str, max_dir_chars)).monospace().size(10.5).color(ctx.pal.text_muted));
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    let third = ((inner_w - 12.0) / 3.0).max(62.0);
                    ui.allocate_ui_with_layout(Vec2::new(third, 28.0), Layout::top_down_justified(Align::Min), |ui| {
                        let anim_id = format!("home-animate-card-{idx}");
                        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&anim_id, "Animate", "Open in Animations")).clicked() {
                            ctx.exec("file.open", json!({"path": path}), None);
                            ctx.push(UiAction::SwitchView(ViewMode::Animation));
                        }
                    });
                    ui.allocate_ui_with_layout(Vec2::new(third, 28.0), Layout::top_down_justified(Align::Min), |ui| {
                        let stud_id = format!("home-studio-card-{idx}");
                        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&stud_id, "Studio", "Open in FilmCraft Studio")).clicked() {
                            ctx.exec("file.open", json!({"path": path}), None);
                            ctx.push(UiAction::SwitchView(ViewMode::Studio));
                        }
                    });
                    ui.allocate_ui_with_layout(Vec2::new(third, 28.0), Layout::top_down_justified(Align::Min), |ui| {
                        let open_id = format!("home-open-card-{idx}");
                        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&open_id, "Open Project", "Open project Craft files").primary()).clicked() {
                            ctx.exec("file.open", json!({"path": path}), Some("Opened project"));
                            ctx.push(UiAction::SwitchView(ViewMode::Project));
                        }
                    });
                });
            });
        });
        ctx.reg.record(format!("home-card-{idx}"), name, &card_resp.response, Some(path.to_owned()));
    });
}

fn draw_project_grid(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let projects = catalog_projects(ctx);
    let query = ctx.state.home.search.trim().to_ascii_lowercase();
    let filter = ctx.state.home.filter.clone();

    let mut slots: Vec<GridSlot<'_>> = vec![GridSlot::StartHere];
    for (idx, item) in projects.iter().enumerate() {
        let path = item["path"].as_str().unwrap_or_default();
        let dir_str = item["directory"].as_str().unwrap_or(path);
        let name = item["name"].as_str().unwrap_or_else(|| path.rsplit('/').next().unwrap_or("Untitled"));
        let has_video = item["video_project"].as_str().is_some();
        if filter == "render" && !has_video {
            continue;
        }
        if !query.is_empty() && !name.to_ascii_lowercase().contains(&query) && !dir_str.to_ascii_lowercase().contains(&query) {
            continue;
        }
        slots.push(GridSlot::Project(idx, item));
    }

    let avail = ui.available_width().max(300.0);
    let cols: usize = if avail >= 1040.0 {
        3
    } else if avail >= 660.0 {
        2
    } else {
        1
    };
    let gap = 14.0_f32;
    let col_w = ((avail - ((cols.saturating_sub(1)) as f32) * gap) / (cols as f32)).clamp(260.0, 620.0);

    let grid_resp = ui.vertical(|ui| {
        for (row_idx, row_slots) in slots.chunks(cols).enumerate() {
            if row_idx > 0 {
                ui.add_space(gap);
            }
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for slot in row_slots {
                    match slot {
                        GridSlot::StartHere => draw_start_here_card(ui, ctx, col_w),
                        GridSlot::Project(idx, item) => draw_single_project_card(ui, ctx, col_w, *idx, item),
                    }
                }
            });
        }
    });
    ctx.reg.record("recent", "Recent Projects", &grid_resp.response, None);
}

fn draw_character_catalog(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let chars = catalog_characters(ctx);
    if chars.is_empty() {
        return;
    }
    let query = ctx.state.home.search.trim().to_ascii_lowercase();
    let visible: Vec<(usize, &Value)> = chars
        .iter()
        .enumerate()
        .filter(|(_, ch)| {
            if query.is_empty() {
                return true;
            }
            let cname = clean_character_name(ch).to_ascii_lowercase();
            let grp = ch["group"].as_str().unwrap_or_default().to_ascii_lowercase();
            cname.contains(&query) || grp.contains(&query)
        })
        .collect();
    if visible.is_empty() {
        return;
    }

    ui.label(RichText::new("CHARACTER CRAFT RIGS (.PCRAFT / .ECPROJ)").monospace().size(10.5).color(ctx.pal.text_muted));
    ui.add_space(6.0);

    let avail = ui.available_width().max(240.0);
    let gap = 12.0_f32;
    let cols = (((avail + gap) / (CHAR_CARD_W + gap)).floor() as usize).max(1);
    let card_w = ((avail - ((cols.saturating_sub(1)) as f32) * gap) / (cols as f32)).clamp(192.0, 272.0);

    let cat_resp = ui.vertical(|ui| {
        for (r_idx, row) in visible.chunks(cols).enumerate() {
            if r_idx > 0 {
                ui.add_space(gap);
            }
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for &(idx, ch) in row {
                    draw_single_character_card(ui, ctx, card_w, idx, ch);
                }
            });
        }
    });
    ctx.reg.record("character-catalog", "Character Catalog", &cat_resp.response, None);
}

fn draw_single_character_card(ui: &mut Ui, ctx: &mut ViewCtx<'_>, card_w: f32, idx: usize, ch: &Value) {
    let cname = clean_character_name(ch);
    let cid = ch["id"].as_str().unwrap_or(&cname).to_owned();
    let proj = ch["project"].as_str().unwrap_or_default().to_owned();
    let preview = ch["preview"].as_str();
    let has_pcraft = ch["pcraft"].as_bool() == Some(true);
    let has_ecproj = ch["ecproj"].as_bool() == Some(true);

    ui.allocate_ui_with_layout(Vec2::new(card_w, CHAR_CARD_H), Layout::top_down(Align::Min), |ui| {
        ctx.pal.card_frame().show(ui, |ui| {
            let inner_w = (card_w - 24.0).max(140.0);
            ui.set_width(inner_w);
            ui.vertical(|ui| {
                let (img_rect, iresp) = ui.allocate_exact_size(Vec2::new(inner_w, 138.0), Sense::click());
                if !paint_local_image(ui, img_rect, preview, ctx.pal.bg_stage, 6) {
                    paint_craft_doc_card(ui, img_rect, ctx.pal, "PCRAFT", &cname, "Unrendered character asset");
                }
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(elide_middle(&cname, 17)).size(12.5).strong().color(ctx.pal.text_primary));
                    if has_pcraft {
                        paint_craft_badge(ui, ctx.pal, ".pcraft", true);
                    }
                    if has_ecproj {
                        paint_craft_badge(ui, ctx.pal, ".ecproj", true);
                    }
                });
                ui.add_space(4.0);
                let btn_id = format!("home-char-animate-{idx}");
                let open_clicked =
                    action_button(ui, ctx.reg, ctx.pal, BtnSpec::new(&btn_id, "wand", "Open Rig", "Open character in Animations")).clicked() || iresp.clicked();
                if open_clicked {
                    if !proj.is_empty() {
                        ctx.exec("file.open", json!({"path": proj}), None);
                    }
                    ctx.push(UiAction::SelectCharacter { id: cid, jump_animation: true });
                }
            });
        });
    });
}
