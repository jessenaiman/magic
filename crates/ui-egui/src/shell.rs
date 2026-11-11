//! Workstation shell chrome (`#app-bar`, `#stage-assets` collapsible bar, `#app-footer` pipeline
//! readiness rail, `#fidget-panel` assistant popover, toast overlay, and modals) using `egui` 0.37.

use craft_keymap::Keymap;
use egui::{
    Align, Align2, Color32, CornerRadius, FontId, Layout, Pos2, Rect, RichText, ScrollArea, Sense,
    Stroke, StrokeKind, Ui, Vec2,
};
use serde_json::{Value, json};

use crate::controls::{craft_swatch_button, paint_local_image};
use crate::icons::companion_image;
use crate::menus::{build_menu_bar, draw_in_window_bar, translate_menu_command};
use crate::state::{ActiveModal, PickerTarget, UiAction, ViewCtx, ViewMode};
use crate::theme::ThemeMode;
use crate::views;
use crate::widgets::{BtnSpec, action_button, companion_button, mode_tab, status_pill, text_input};

pub fn draw_workstation(
    ui: &mut Ui,
    ctx: &mut ViewCtx<'_>,
    keymap: &Keymap,
    show_in_window_menus: bool,
    is_enabled: impl Fn(&str) -> bool,
) {
    let modal_open = ctx.state.active_modal.is_some();

    if show_in_window_menus {
        let bar = build_menu_bar(ctx.state, keymap, &is_enabled);
        egui::Panel::top("magic_menu_bar")
            .frame(
                egui::Frame::new()
                    .fill(ctx.pal.bg_stage)
                    .inner_margin(egui::Margin::symmetric(8, 2))
                    .stroke(Stroke::new(1.0, ctx.pal.border_subtle)),
            )
            .show(ui, |ui| {
                let clicked = draw_in_window_bar(ui, &bar, modal_open, ctx.reg);
                for cmd in clicked {
                    if let Some(act) = translate_menu_command(&cmd, ctx.state) {
                        ctx.push(act);
                    }
                }
            });
    }

    let top_resp = egui::Panel::top("magic_app_bar")
        .frame(
            egui::Frame::new()
                .fill(ctx.pal.bg_stage)
                .inner_margin(egui::Margin::symmetric(10, 6))
                .stroke(Stroke::new(1.0, ctx.pal.border)),
        )
        .show(ui, |ui| {
            craft_window::title_bar::empty_area(ui, ui.max_rect());
            draw_app_bar(ui, ctx);
        });
    ctx.reg.record("app-bar", "App Bar", &top_resp.response, None);
    ctx.reg.record_panel("app-bar", top_resp.response.rect);

    let active_view = ctx.state.effective_view();
    if ctx.state.project_path.is_some() && active_view != ViewMode::Home {
        let assets_resp = egui::Panel::top("magic_stage_assets")
            .frame(
                egui::Frame::new()
                    .fill(ctx.pal.bg_panel)
                    .inner_margin(egui::Margin::symmetric(10, 4))
                    .stroke(Stroke::new(1.0, ctx.pal.border_subtle)),
            )
            .show(ui, |ui| {
                draw_stage_assets_drawer(ui, ctx);
            });
        ctx.reg.record("stage-assets", "Stage Assets", &assets_resp.response, None);
        ctx.reg.record_panel("stage-assets", assets_resp.response.rect);
    }

    let foot_resp = egui::Panel::bottom("magic_app_footer")
        .frame(
            egui::Frame::new()
                .fill(ctx.pal.bg_stage)
                .inner_margin(egui::Margin::symmetric(10, 4))
                .stroke(Stroke::new(1.0, ctx.pal.border)),
        )
        .show(ui, |ui| {
            draw_footer(ui, ctx);
        });
    ctx.reg.record("app-footer", "App Footer", &foot_resp.response, None);
    ctx.reg.record_panel("app-footer", foot_resp.response.rect);

    let central_resp = egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(ctx.pal.bg_canvas).inner_margin(egui::Margin::same(10)))
        .show(ui, |ui| match active_view {
            ViewMode::Home => views::home::show(ui, ctx),
            ViewMode::Project => views::project::show(ui, ctx),
            ViewMode::Storyboard => views::storyboard::show(ui, ctx),
            ViewMode::Animation => views::animation::show(ui, ctx),
            ViewMode::Design => views::lyrics::show(ui, ctx),
            ViewMode::Motion => views::motion::show(ui, ctx),
            ViewMode::Studio => views::studio::show(ui, ctx),
        });
    let (root_id, root_lbl) = active_view.root_automation_id();
    ctx.reg.record(root_id, root_lbl, &central_resp.response, None);
    ctx.reg.record_panel("central", central_resp.response.rect);

    if ctx.state.fidget_open {
        draw_fidget_popover(ui.ctx(), ctx);
    }
    if let Some(modal) = ctx.state.active_modal {
        draw_modal(ui.ctx(), ctx, modal);
    }
    draw_toast_if_active(ui.ctx(), ctx);
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
    let light = sb["light"]
        .as_str()
        .or_else(|| ctx.state.cache.storyboard_check["light"].as_str())
        .unwrap_or("red");
    if sb["production_ready"].as_bool() == Some(true)
        || ctx.state.cache.storyboard_check["production_ready"].as_bool() == Some(true)
        || light == "green"
    {
        ctx.pal.accent_emerald
    } else if light == "amber" || sb["title_sections_complete"].as_bool() == Some(true) {
        ctx.pal.accent_amber
    } else {
        ctx.pal.danger
    }
}

/// 3-zone `#app-bar`: Left (`✦` + full project pill + `⇱`), True Horizontal Center (5-tab capsule),
/// Right (`▶ 0:18 / 2:45` + `PC VC DC EC FC` swatches + Fidget & Scribbles).
fn draw_app_bar(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let has_proj = ctx.state.project_path.is_some();
    let bar_w = ui.available_width().max(320.0);
    let (bar_rect, _) = ui.allocate_exact_size(Vec2::new(bar_w, 30.0), Sense::hover());

    // Left zone (`✦` brand + active project pill + window detach)
    let left_rect = Rect::from_min_max(bar_rect.min, Pos2::new((bar_rect.center().x - 196.0).max(bar_rect.min.x + 220.0), bar_rect.max.y));
    let mut left_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(left_rect)
            .layout(Layout::left_to_right(Align::Center))
            .scope_id(ui.scope_id().with("app_bar_left")),
    );
    let brand_label = if has_proj { "" } else { "Magic Studio" };
    let brand_tip = if has_proj { "Close project and return to Home" } else { "Magic Studio" };
    if action_button(&mut left_ui, ctx.reg, ctx.pal, BtnSpec::new("brand", "brand", brand_label, brand_tip)).clicked()
        && has_proj
    {
        ctx.push(UiAction::CloseProject);
    }

    let pname = ctx.state.project_name();
    if has_proj {
        let display_name = clamp_title(&pname, 26);
        let selected = ctx.state.effective_view() == ViewMode::Project;
        if action_button(
            &mut left_ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::new("tab-project", "book", &display_name, &format!("Project · {pname}")).selected(true),
        )
        .clicked()
            && !selected
        {
            ctx.push(UiAction::SwitchView(ViewMode::Project));
        }
    } else {
        let lbl = left_ui.label(RichText::new("No project").size(12.0).color(ctx.pal.text_muted));
        ctx.reg.record("project-name", "No project", &lbl, None);
    }
    if action_button(
        &mut left_ui,
        ctx.reg,
        ctx.pal,
        BtnSpec::icon_only("window-snap", "external", "Snap / detach window"),
    )
    .clicked()
    {
        ctx.push(UiAction::SnapWindow);
    }

    // Center zone: horizontally centered 5-stage workstation mode capsule (`Gallery` & `Mayhem` removed)
    if has_proj {
        let capsule_w = 392.0_f32.min((bar_w - 420.0).max(310.0));
        let center_x = bar_rect
            .center()
            .x
            .max(left_ui.min_rect().max.x + capsule_w * 0.5 + 8.0);
        let cap_rect = Rect::from_center_size(Pos2::new(center_x, bar_rect.center().y), Vec2::new(capsule_w, 30.0));
        let mut center_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(cap_rect)
                .layout(Layout::centered_and_justified(egui::Direction::LeftToRight))
                .scope_id(ui.scope_id().with("app_bar_center")),
        );
        egui::Frame::new()
            .fill(ctx.pal.bg_input)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(egui::Margin::symmetric(5, 2))
            .stroke(Stroke::new(1.0, ctx.pal.border))
            .show(&mut center_ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 3.0;
                    for tab in ViewMode::STAGE_TABS {
                        let id = format!("tab-{}", tab.as_id());
                        let dot = (tab == ViewMode::Storyboard).then(|| storyboard_dot_color(ctx));
                        let selected = ctx.state.effective_view() == tab;
                        if mode_tab(ui, ctx.reg, ctx.pal, &id, tab.tab_label(), selected, dot).clicked() {
                            ctx.push(UiAction::SwitchView(tab));
                        }
                    }
                });
            });
    }

    // Right zone: rendered right-to-left so left-to-right visual order is `[▶ 0:00 / 2:45] [PC][VC][DC][EC][FC] [Fidget][Scribbles]`
    let right_rect = Rect::from_min_max(Pos2::new((bar_rect.center().x + 198.0).min(bar_rect.max.x - 210.0), bar_rect.min.y), bar_rect.max);
    let mut right_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(right_rect)
            .layout(Layout::right_to_left(Align::Center))
            .scope_id(ui.scope_id().with("app_bar_right")),
    );
    right_ui.spacing_mut().item_spacing.x = 4.0;

    if companion_button(
        &mut right_ui,
        ctx.reg,
        ctx.pal,
        "scribbles-toggle",
        "scribbles_work",
        "Open Scribbles character studio",
        false,
    )
    .clicked()
    {
        let cid = ctx.state.selected_character_id.clone();
        ctx.exec("scribbles.open", json!({"character": cid}), Some("Opened Scribbles"));
    }
    if companion_button(
        &mut right_ui,
        ctx.reg,
        ctx.pal,
        "fidget-toggle",
        "fidget_work",
        "Toggle Fidget workspace assistant",
        ctx.state.fidget_open,
    )
    .clicked()
    {
        ctx.state.fidget_open = !ctx.state.fidget_open;
    }

    // Reverse slice in right-to-left layout yields visual left-to-right order: PC, VC, DC, EC, FC
    for (id, code, fill, app, tip) in [
        ("dock-filmcraft", "FC", Color32::from_rgb(0x10, 0xb9, 0x81), "filmcraft", "Launch FilmCraft"),
        ("dock-effectcraft", "EC", Color32::from_rgb(0xa8, 0x55, 0xf7), "effectcraft", "Launch EffectCraft"),
        ("dock-designcraft", "DC", Color32::from_rgb(0xec, 0x48, 0x99), "designcraft", "Launch DesignCraft"),
        ("dock-vectorcraft", "VC", Color32::from_rgb(0xf5, 0x9e, 0x0b), "vectorcraft", "Launch VectorCraft"),
        ("dock-photocraft", "PC", Color32::from_rgb(0x3b, 0x82, 0xf6), "photocraft", "Launch PhotoCraft"),
    ] {
        if craft_swatch_button(&mut right_ui, ctx.reg, id, code, fill, tip).clicked() {
            ctx.exec("workspace.open", json!({"app": app}), Some(tip));
        }
    }

    if has_proj {
        right_ui.add_space(4.0);
        let elapsed = format_clock(ctx.state.audio_time_sec);
        let dur = format_clock(ctx.state.audio_duration_sec);
        let transport_txt = format!("{elapsed} / {dur}");
        let icon = if ctx.state.audio_playing { "pause" } else { "play" };
        if action_button(
            &mut right_ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::new("app-audio-toggle", icon, &transport_txt, "Play or pause song audio")
                .selected(ctx.state.audio_playing),
        )
        .clicked()
        {
            ctx.push(UiAction::ToggleAudio);
        }
    }
}

fn draw_stage_assets_drawer(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let drawer = &ctx.state.cache.snapshot["data"]["drawer"];
    let char_cards = drawer["characters"].as_array().cloned().unwrap_or_default();
    let bg_cards = drawer["backgrounds"].as_array().cloned().unwrap_or_default();
    let total = drawer["total"]
        .as_u64()
        .map_or(char_cards.len() + bg_cards.len(), |n| n as usize);

    if ctx.state.assets_drawer_tab.is_empty() {
        ctx.state.assets_drawer_tab = "characters".into();
    }

    ui.horizontal(|ui| {
        let icon = if ctx.state.assets_drawer_open { "chevronUp" } else { "chevronDown" };
        let assets_lbl = format!("Assets ({total})");
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::new("stage-assets-toggle", icon, &assets_lbl, "Toggle project asset shelf")
                .selected(ctx.state.assets_drawer_open),
        )
        .clicked()
        {
            ctx.state.assets_drawer_open = !ctx.state.assets_drawer_open;
        }
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::icon_only("stage-assets-pin", "pin", "Pin asset shelf open")
                .selected(ctx.state.assets_drawer_pinned),
        )
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
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("stage-assets-add", "plus", "Add Asset", "Add project artwork").primary(),
            )
            .clicked()
            {
                ctx.push(UiAction::PickFile {
                    target: PickerTarget::AddProjectAsset,
                    title: "Add Project Artwork".into(),
                    extensions: vec![
                        "png".into(),
                        "jpg".into(),
                        "jpeg".into(),
                        "webp".into(),
                        "pcraft".into(),
                        "vectorcraft".into(),
                    ],
                    directory: false,
                    save: false,
                });
            }
        });
    });

    let active_cards = if ctx.state.assets_drawer_tab == "backgrounds" {
        &bg_cards
    } else {
        &char_cards
    };
    if (ctx.state.assets_drawer_open || ctx.state.assets_drawer_pinned) && !active_cards.is_empty() {
        ui.add_space(4.0);
        ScrollArea::horizontal().id_salt("stage_assets_strip").show(ui, |ui| {
            ui.horizontal(|ui| {
                for card in active_cards {
                    let cid = card["char_id"]
                        .as_str()
                        .or_else(|| card["id"].as_str())
                        .unwrap_or("asset");
                    let name = card["name"].as_str().unwrap_or(cid);
                    let badge = card["badge"].as_str().unwrap_or("Asset");
                    let thumb = card["thumb"].as_str();
                    let sel = ctx.state.selected_character_id.as_deref() == Some(cid);
                    let (crect, resp) = ui.allocate_exact_size(Vec2::new(152.0, 44.0), Sense::click());
                    let stroke = if sel {
                        Stroke::new(1.5, ctx.pal.accent_emerald)
                    } else {
                        Stroke::new(1.0, ctx.pal.border)
                    };
                    ui.painter().rect(
                        crect,
                        CornerRadius::same(7),
                        if sel { ctx.pal.accent_emerald_bg } else { ctx.pal.bg_input },
                        stroke,
                        StrokeKind::Inside,
                    );
                    let img_rect = Rect::from_min_size(Pos2::new(crect.min.x + 4.0, crect.min.y + 4.0), Vec2::new(46.0, 36.0));
                    if !paint_local_image(ui, img_rect, thumb, ctx.pal.bg_stage, 5) {
                        ui.painter().text(img_rect.center(), Align2::CENTER_CENTER, "IMG", FontId::monospace(9.0), ctx.pal.text_muted);
                    }
                    ui.painter().text(
                        Pos2::new(img_rect.max.x + 6.0, crect.min.y + 14.0),
                        Align2::LEFT_CENTER,
                        clamp_title(name, 13),
                        FontId::proportional(11.0),
                        ctx.pal.text_primary,
                    );
                    ui.painter().text(
                        Pos2::new(img_rect.max.x + 6.0, crect.min.y + 29.0),
                        Align2::LEFT_CENTER,
                        badge,
                        FontId::monospace(9.5),
                        ctx.pal.accent_emerald,
                    );
                    if resp.clicked() {
                        ctx.push(UiAction::SelectCharacter {
                            id: cid.to_owned(),
                            jump_animation: false,
                        });
                    }
                    ctx.reg.record(format!("asset-card-{cid}"), name, &resp, None);
                }
            });
        });
    }
}

fn draw_footer(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let has_proj = ctx.state.project_path.is_some();
    let has_song = ctx.state.song_audio_path().is_some();
    let sb = &ctx.state.cache.snapshot["data"]["storyboard"];
    let sb_ok = sb["production_ready"].as_bool() == Some(true);
    let status = &ctx.state.cache.snapshot["data"]["status"];
    let lyrics_ok = status["lyrics_ready"].as_bool().unwrap_or(has_proj);
    let chars_ok = ctx
        .state
        .cache
        .snapshot
        .pointer("/data/characters")
        .and_then(Value::as_array)
        .is_some_and(|c| !c.is_empty());
    let studio_ok = status["video_project_exists"].as_bool().unwrap_or(has_proj);

    let bar_w = ui.available_width().max(320.0);
    let (frect, _) = ui.allocate_exact_size(Vec2::new(bar_w, 20.0), Sense::hover());

    let mut left_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(Rect::from_min_max(frect.min, Pos2::new(frect.min.x + 360.0, frect.max.y)))
            .layout(Layout::left_to_right(Align::Center))
            .scope_id(ui.scope_id().with("footer_left")),
    );
    for (label, ready) in [
        ("Song", has_song),
        ("Lyrics", lyrics_ok),
        ("Transcript", has_song),
        ("Storyboard", sb_ok),
        ("Animate", chars_ok),
        ("Studio", studio_ok),
    ] {
        let dot_col = if ready { ctx.pal.accent_emerald } else { ctx.pal.border };
        let (drect, _) = left_ui.allocate_exact_size(egui::vec2(8.0, 14.0), Sense::hover());
        left_ui.painter().circle_filled(drect.center(), 3.2, dot_col);
        left_ui.label(
            RichText::new(label)
                .size(10.5)
                .color(if ready { ctx.pal.text_secondary } else { ctx.pal.text_muted }),
        );
        left_ui.add_space(3.0);
    }

    let center_msg = if has_proj {
        let summary = ctx
            .state
            .cache
            .snapshot
            .pointer("/data/footer/summary")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty());
        match summary {
            Some(extra) => format!("{} · {} · {extra}", ctx.state.project_name(), ctx.state.effective_view().tab_label()),
            None => format!("{} · {}", ctx.state.project_name(), ctx.state.effective_view().tab_label()),
        }
    } else {
        "Choose a project or start a new song".into()
    };
    ui.painter().text(
        frect.center(),
        Align2::CENTER_CENTER,
        &center_msg,
        FontId::proportional(10.5),
        ctx.pal.text_muted,
    );

    let mut right_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(Rect::from_min_max(Pos2::new(frect.max.x - 180.0, frect.min.y), frect.max))
            .layout(Layout::right_to_left(Align::Center))
            .scope_id(ui.scope_id().with("footer_right")),
    );
    let dirty = !ctx.state.dirty_drafts.is_empty();
    let save_label = if ctx.state.busy {
        "Working…"
    } else if dirty {
        "Unsaved drafts"
    } else if has_proj {
        "Project saved"
    } else {
        "Not saved"
    };
    let save_color = if ctx.state.busy || dirty {
        ctx.pal.accent_amber
    } else if has_proj {
        ctx.pal.accent_emerald
    } else {
        ctx.pal.text_muted
    };
    status_pill(&mut right_ui, ctx.pal, save_label, save_color);
}

fn draw_fidget_popover(egui_ctx: &egui::Context, ctx: &mut ViewCtx<'_>) {
    let mut open = ctx.state.fidget_open;
    egui::Window::new("Fidget · Studio Assistant")
        .id(egui::Id::unique("fidget-panel"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(340.0)
        .show(egui_ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add(companion_image("fidget_work", 32.0));
                ui.label(RichText::new("Prepare CLI or MCP task prompts").color(ctx.pal.text_secondary));
            });
            text_input(
                ui,
                ctx.reg,
                "fidget-prompt",
                "Assistant instruction",
                &mut ctx.state.fidget_prompt,
                "Describe storyboard, puppet, or lyric step…",
                310.0,
            );
            ui.horizontal(|ui| {
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::new("fidget-prepare", "send", "Run", "Dispatch instruction via agent.send").primary(),
                )
                .clicked()
                    && !ctx.state.fidget_prompt.trim().is_empty()
                {
                    let prompt = ctx.state.fidget_prompt.trim().to_owned();
                    ctx.exec("agent.send", json!({"message": prompt, "new_chat": false}), Some("Sent task to Fidget"));
                }
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("fidget-panel-close", "close", "Close", "Close Fidget panel")).clicked() {
                    ctx.state.fidget_open = false;
                }
            });
        });
    if !open {
        ctx.state.fidget_open = false;
    }
}

fn draw_modal(egui_ctx: &egui::Context, ctx: &mut ViewCtx<'_>, modal: ActiveModal) {
    let title = match modal {
        ActiveModal::Preferences => "Preferences",
        ActiveModal::Doctor => "ArtCraft Suite Doctor",
        ActiveModal::About => "About Magic Studio",
        ActiveModal::UnsavedClose => "Unsaved Changes",
    };
    let mut keep_open = true;
    egui::Window::new(title)
        .id(egui::Id::unique(("magic_modal", title)))
        .open(&mut keep_open)
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(egui_ctx, |ui| match modal {
            ActiveModal::Preferences => {
                ui.label(RichText::new("Appearance").strong());
                ui.horizontal(|ui| {
                    for (mode, word) in [
                        (ThemeMode::Dark, "Dark"),
                        (ThemeMode::Light, "Light"),
                        (ThemeMode::System, "System"),
                    ] {
                        let id = format!("pref-theme-{}", mode.as_str());
                        if action_button(
                            ui,
                            ctx.reg,
                            ctx.pal,
                            BtnSpec::new(&id, "eye", word, "Set theme").selected(ctx.state.theme_mode == mode),
                        )
                        .clicked()
                        {
                            ctx.state.theme_mode = mode;
                        }
                    }
                });
            }
            ActiveModal::Doctor => {
                ui.horizontal(|ui| {
                    ui.label("Installed ArtCraft CLI & `--control` diagnostics:");
                    if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("doctor-refresh", "refresh", "Check", "Run app.doctor")).clicked() {
                        ctx.exec("app.doctor", json!({}), None);
                    }
                });
                let text = serde_json::to_string_pretty(&ctx.state.cache.doctor).unwrap_or_else(|_| "{}".into());
                ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                    ui.code(text);
                });
            }
            ActiveModal::About => {
                ui.label(RichText::new(concat!("Magic Studio ", env!("CARGO_PKG_VERSION"))).size(16.0).strong());
                ui.label("Pure-Rust eframe + egui 0.37 + storytold/craft-libs coordinator for the ArtCraft suite.");
                ui.label("Delegates timeline, compositing, cutout, vector, and storyboard layout to FilmCraft, EffectCraft, DesignCraft, PhotoCraft, VectorCraft, and headless anicraft-cli.");
            }
            ActiveModal::UnsavedClose => {
                ui.label("You have unsaved project or stage changes. Save before closing?");
                ui.horizontal(|ui| {
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
            }
        });
    if !keep_open {
        ctx.state.active_modal = None;
    }
}

fn draw_toast_if_active(egui_ctx: &egui::Context, ctx: &mut ViewCtx<'_>) {
    let now = egui_ctx.input(|i| i.time);
    if ctx.state.toast_message.is_empty() || now > ctx.state.toast_until {
        return;
    }
    egui::Area::new(egui::Id::unique("app-toast"))
        .anchor(Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -44.0))
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
