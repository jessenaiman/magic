//! `Storyboard` view (`#view-storyboard`): 3-pane `DesignCraft` (`storyboard.designcraft`) studio:
//! 1. Left (`Sections & 2-Up Spreads`, 270px): 2-page spread pairs (`A-Cover | B-World`, `C-Character | D-Motion`, `Scene 1 | Scene 2`).
//! 2. Center (`Single Page Canvas`): vertical scroll sheet showing the active page's editable slots and locked `*_cli` fineprint.
//! 3. Right (`Requirements & Readiness`, 310px): red/green traffic-light checklist for all required sections/slots + Slot Inspector.

use craft_ui::layout::{SplitAxis, SplitSize, split_rect};
use egui::{Align2, Color32, CornerRadius, FontId, Pos2, Rect, RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui, Vec2};
use serde_json::{Value, json};

use crate::controls::{FilmbarOrientation, FilmbarStyle, RequirementRow, ScalarSpec, paint_filmbar_frame, requirement_light_row, ring_slider_h};
use crate::state::{UiAction, ViewCtx, ViewMode};
use crate::widgets::{BtnSpec, action_button, status_pill, text_input};

struct SpreadSummary {
    index: usize,
    title: String,
    archetype: String,
    filled: bool,
    slots: Vec<Value>,
}

pub fn show(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let outer = ui.available_rect_before_wrap();
    let root_resp = ui.allocate_rect(outer, Sense::hover());
    ctx.reg.record("view-storyboard", "Storyboard", &root_resp, None);

    let spreads = collect_spreads(ctx);

    ui.vertical(|ui| {
        draw_toolbar(ui, ctx, &spreads);
        if ctx.state.storyboard.show_wizard {
            ui.add_space(6.0);
            draw_wizard_bar(ui, ctx);
        }
        ui.add_space(6.0);
        let body = ui.available_rect_before_wrap();
        if let Some(left_split) = split_rect(body, SplitAxis::Horizontal, SplitSize::FixedFirst(272.0), 8.0, 200.0) {
            let mut left = ui.new_child(egui::UiBuilder::new().max_rect(left_split.first).scope_id(ui.scope_id().with("sb_left_2up")));
            draw_two_up_rail(&mut left, ctx, &spreads);
            if let Some(right_split) = split_rect(left_split.second, SplitAxis::Horizontal, SplitSize::FixedSecond(312.0), 8.0, 220.0) {
                let mut center = ui.new_child(egui::UiBuilder::new().max_rect(right_split.first).scope_id(ui.scope_id().with("sb_center_scroll")));
                draw_single_page_scroll(&mut center, ctx, &spreads);
                let mut right = ui.new_child(egui::UiBuilder::new().max_rect(right_split.second).scope_id(ui.scope_id().with("sb_right_reqs")));
                draw_requirements_panel(&mut right, ctx, &spreads);
            } else {
                let mut center = ui.new_child(egui::UiBuilder::new().max_rect(left_split.second).scope_id(ui.scope_id().with("sb_center_scroll")));
                draw_single_page_scroll(&mut center, ctx, &spreads);
            }
        } else {
            draw_single_page_scroll(ui, ctx, &spreads);
        }
    });
}

fn collect_spreads(ctx: &ViewCtx<'_>) -> Vec<SpreadSummary> {
    let raw_pages = ctx.state.cache.storyboard_pages.get("pages").and_then(Value::as_array).cloned().unwrap_or_default();
    if !raw_pages.is_empty() {
        return raw_pages
            .into_iter()
            .enumerate()
            .map(|(idx, p)| {
                let arch = p["archetype"].as_str().unwrap_or("scene").to_owned();
                let title = p["name"].as_str().or_else(|| p["title"].as_str()).map(str::to_owned).unwrap_or_else(|| default_archetype_title(idx, &arch));
                let slots = p["slots"].as_array().cloned().unwrap_or_default();
                let filled = page_slots_filled(&slots);
                SpreadSummary { index: idx, title, archetype: arch, filled, slots }
            })
            .collect();
    }
    [
        (0, "A-Cover", "cover", "cover_title"),
        (1, "B-World", "world", "world_stage_image"),
        (2, "C-Character", "char", "char_original_image"),
        (3, "D-Motion", "motion", "motion_stage_image"),
        (4, "Scene 1", "scene", "hero_image"),
        (5, "Scene 2", "scene", "hero_image"),
    ]
    .into_iter()
    .map(|(idx, title, arch, primary_slot)| SpreadSummary {
        index: idx,
        title: title.to_owned(),
        archetype: arch.to_owned(),
        filled: false,
        slots: vec![json!({"label": primary_slot, "kind": "image", "text": ""})],
    })
    .collect()
}

fn default_archetype_title(idx: usize, arch: &str) -> String {
    match arch {
        "cover" => "A-Cover".into(),
        "world" => "B-World".into(),
        "char" => "C-Character".into(),
        "motion" => "D-Motion".into(),
        _ => format!("Scene {}", idx.saturating_sub(3).max(1)),
    }
}

fn page_slots_filled(slots: &[Value]) -> bool {
    let mut checked = 0usize;
    for s in slots {
        let label = s["label"].as_str().unwrap_or_default();
        if label.ends_with("_cli") || s["locked"].as_bool() == Some(true) {
            continue;
        }
        checked += 1;
        let val = s["text"].as_str().or_else(|| s["image"].as_str()).or_else(|| s["value"].as_str()).unwrap_or("");
        let trimmed = val.trim();
        if trimmed.is_empty() || (trimmed.contains('[') && trimmed.contains(']')) {
            return false;
        }
    }
    checked > 0
}

fn draw_toolbar(ui: &mut Ui, ctx: &mut ViewCtx<'_>, spreads: &[SpreadSummary]) {
    let main_ready = spreads.iter().take(4).filter(|s| s.filled).count();
    let complete = main_ready >= 4 && spreads.iter().all(|s| s.filled);
    ctx.pal.panel_frame().show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            let pill_color = if complete {
                ctx.pal.accent_emerald
            } else if main_ready > 0 {
                ctx.pal.accent_amber
            } else {
                ctx.pal.danger
            };
            status_pill(ui, ctx.pal, &format!("{main_ready}/4 Main Pages Ready"), pill_color);

            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("storyboard-new", "plus", "Init", "Initialize storyboard.designcraft")).clicked() {
                ctx.exec("storyboard.new", json!({}), Some("Initialized storyboard.designcraft"));
            }
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("storyboard-wizard-toggle", "wand", "Wizard", "Fill Main Section Title Pages").selected(ctx.state.storyboard.show_wizard),
            )
            .clicked()
            {
                ctx.state.storyboard.show_wizard = !ctx.state.storyboard.show_wizard;
            }
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("storyboard-bulk-generate", "sparkles", "Bulk", "Generate scene spreads from title pages").primary(),
            )
            .clicked()
            {
                ctx.exec("storyboard.bulk_generate", json!({}), Some("Generated storyboard scenes"));
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("storyboard-refresh", "check", "Check", "Verify readiness gate")).clicked() {
                ctx.exec("storyboard.check", json!({}), None);
                ctx.exec("storyboard.pages", json!({}), None);
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("storyboard-export", "download", "Export", "Export storyboard PDF via DesignCraft")).clicked() {
                ctx.exec("storyboard.export", json!({}), Some("Exported storyboard PDF"));
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("storyboard-open-editor", "book", "Design", "Open storyboard in DesignCraft")).clicked() {
                ctx.exec("storyboard.open", json!({}), Some("Opened DesignCraft"));
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("storyboard-go-studio", "film", "Studio", "Proceed to Studio timeline").enabled(complete))
                .clicked()
            {
                ctx.push(UiAction::SwitchView(ViewMode::Studio));
            }
        });
    });
}

fn draw_wizard_bar(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ctx.pal.card_frame().show(ui, |ui| {
        ui.label(RichText::new("Main Section Title Pages (`A-Cover`, `B-World`, `C-Character`, `D-Motion`)").strong());
        ui.horizontal_wrapped(|ui| {
            ui.label("Style");
            text_input(ui, ctx.reg, "sb-wiz-style", "D01 Style", &mut ctx.state.storyboard.style_d01, "Visual style…", 160.0);
            ui.label("World");
            text_input(ui, ctx.reg, "sb-wiz-world", "L01 World", &mut ctx.state.storyboard.world_l01, "Master location…", 160.0);
            ui.label("Character");
            text_input(ui, ctx.reg, "sb-wiz-char", "C01 Character", &mut ctx.state.storyboard.character_c01, "Lead character…", 160.0);
            ui.label("Motion");
            text_input(ui, ctx.reg, "sb-wiz-motion", "M01 Motion", &mut ctx.state.storyboard.motion_m01, "Action motion…", 160.0);
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("storyboard-wizard-apply", "check", "Apply", "Write title-page prompts").primary()).clicked() {
                let payload = json!({
                    "style": ctx.state.storyboard.style_d01,
                    "world": ctx.state.storyboard.world_l01,
                    "character": ctx.state.storyboard.character_c01,
                    "motion": ctx.state.storyboard.motion_m01,
                });
                ctx.exec("storyboard.wizard", payload, Some("Updated Storyboard Title Pages"));
            }
        });
    });
}

/// Left Pane: 2-page spread pairs (`[01 A-Cover | 02 B-World]`, `[03 C-Character | 04 D-Motion]`, `[05 Scene 1 | 06 Scene 2]`).
fn draw_two_up_rail(ui: &mut Ui, ctx: &mut ViewCtx<'_>, spreads: &[SpreadSummary]) {
    let panel = ctx.pal.card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Sections & 2-Up Spreads").size(12.5).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("storyboard-remove-page", "trash", "Remove current page")).clicked() {
                    let page = ctx.state.storyboard.current_page;
                    ctx.exec("storyboard.remove", json!({"page": page}), Some("Removed spread"));
                }
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("storyboard-add-page", "plus", "Add scene spread")).clicked() {
                    let arch = ctx.state.storyboard.archetype.clone();
                    ctx.exec("storyboard.add", json!({"archetype": arch}), Some("Added spread"));
                }
            });
        });
        ui.separator();

        ScrollArea::vertical().id_salt("sb_2up_scroll").show(ui, |ui| {
            let mut row_idx = 0usize;
            while row_idx < spreads.len() {
                if row_idx == 0 {
                    ui.label(RichText::new("MAIN TITLE PAGES").size(9.5).color(ctx.pal.text_muted));
                } else if row_idx == 4 {
                    ui.add_space(4.0);
                    ui.label(RichText::new("SONG SCENE SPREADS").size(9.5).color(ctx.pal.text_muted));
                }
                ui.horizontal(|ui| {
                    let pair_w = ((ui.available_width() - 6.0) * 0.5).max(96.0);
                    draw_mini_spread_card(ui, ctx, &spreads[row_idx], pair_w);
                    if let Some(second) = spreads.get(row_idx + 1) {
                        draw_mini_spread_card(ui, ctx, second, pair_w);
                    } else {
                        draw_add_spread_tile(ui, ctx, pair_w);
                    }
                });
                ui.add_space(4.0);
                row_idx += 2;
            }
            if spreads.len().is_multiple_of(2) {
                let pair_w = ((ui.available_width() - 6.0) * 0.5).max(96.0);
                draw_add_spread_tile(ui, ctx, pair_w);
            }
        });
    });
    ctx.reg.record("sb-2up-rail", "2-Up Spreads", &panel.response, None);
}

fn draw_mini_spread_card(ui: &mut Ui, ctx: &mut ViewCtx<'_>, sp: &SpreadSummary, width: f32) {
    let selected = ctx.state.storyboard.current_page == sp.index;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, 78.0), Sense::click());
    if resp.clicked() {
        ctx.state.storyboard.current_page = sp.index;
    }
    let thumb_rect = Rect::from_min_size(rect.min, Vec2::new(width, 58.0));
    if sp.archetype == "scene" {
        let inner = paint_filmbar_frame(
            ui,
            thumb_rect,
            ctx.pal,
            FilmbarStyle { orientation: FilmbarOrientation::Vertical, selected, hovered: resp.hovered(), timecode: None },
        );
        ui.painter().text(inner.center(), Align2::CENTER_CENTER, &sp.title, FontId::proportional(10.5), ctx.pal.text_secondary);
    } else {
        let border = if selected {
            Stroke::new(1.8, ctx.pal.accent_emerald)
        } else if resp.hovered() {
            Stroke::new(1.0, ctx.pal.accent_sky)
        } else {
            Stroke::new(1.0, ctx.pal.border)
        };
        ui.painter().rect(thumb_rect, CornerRadius::same(6), ctx.pal.stage_paper, border, StrokeKind::Inside);
        ui.painter().text(thumb_rect.center(), Align2::CENTER_CENTER, &sp.title, FontId::proportional(11.0), Color32::from_rgb(0x22, 0x28, 0x31));
    }

    let dot_col = if sp.filled { ctx.pal.accent_emerald } else { ctx.pal.danger };
    let label_y = rect.max.y - 9.0;
    ui.painter().text(
        Pos2::new(rect.min.x + 2.0, label_y),
        Align2::LEFT_CENTER,
        format!("{:02} {}", sp.index + 1, sp.title),
        FontId::proportional(10.0),
        if selected { ctx.pal.accent_emerald } else { ctx.pal.text_secondary },
    );
    ui.painter().circle_filled(Pos2::new(rect.max.x - 5.0, label_y), 3.5, dot_col);
    ctx.reg.record(format!("sb-nav-{}", sp.index + 1), &sp.title, &resp, None);
}

fn draw_add_spread_tile(ui: &mut Ui, ctx: &mut ViewCtx<'_>, width: f32) {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, 58.0), Sense::click());
    let stroke_col = if resp.hovered() { ctx.pal.accent_emerald } else { ctx.pal.border };
    ui.painter().rect(rect, CornerRadius::same(6), ctx.pal.bg_input, Stroke::new(1.0, stroke_col), StrokeKind::Inside);
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, "+ Add Page", FontId::proportional(11.0), ctx.pal.text_muted);
    if resp.clicked() {
        let arch = ctx.state.storyboard.archetype.clone();
        ctx.exec("storyboard.add", json!({"archetype": arch}), Some("Added spread"));
    }
    ctx.reg.record("sb-add-tile", "Add Page", &resp, None);
}

/// Middle Pane: Single page in the middle scrolling smoothly from top to bottom.
fn draw_single_page_scroll(ui: &mut Ui, ctx: &mut ViewCtx<'_>, spreads: &[SpreadSummary]) {
    let active_idx = ctx.state.storyboard.current_page.min(spreads.len().saturating_sub(1));
    let active = spreads.get(active_idx);
    let panel = ctx.pal.card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            let heading = active.map_or_else(|| "Single Page Canvas".to_owned(), |sp| format!("Single Page Canvas — {:02} · {}", sp.index + 1, sp.title));
            ui.label(RichText::new(heading).size(13.5).strong().color(ctx.pal.accent_amber));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("sb-next", "chevronRight", "Next page")).clicked()
                    && ctx.state.storyboard.current_page + 1 < spreads.len()
                {
                    ctx.state.storyboard.current_page += 1;
                }
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("sb-prev", "chevronLeft", "Previous page")).clicked()
                    && ctx.state.storyboard.current_page > 0
                {
                    ctx.state.storyboard.current_page -= 1;
                }
            });
        });
        ui.separator();

        ScrollArea::vertical().id_salt("sb_single_page_scroll").auto_shrink([false, false]).show(ui, |ui| {
            if let Some(sp) = active {
                draw_paper_page_sheet(ui, ctx, sp);
                if let Some(next_sp) = spreads.get(active_idx + 1) {
                    ui.add_space(14.0);
                    ui.label(RichText::new(format!("↓ Continues to {:02} · {}", next_sp.index + 1, next_sp.title)).size(11.0).color(ctx.pal.text_muted));
                    draw_paper_page_sheet(ui, ctx, next_sp);
                }
            }
        });
    });
    ctx.reg.record("sb-single-page", "Single Page Canvas", &panel.response, None);
}

fn draw_paper_page_sheet(ui: &mut Ui, ctx: &mut ViewCtx<'_>, sp: &SpreadSummary) {
    let zoom_scale = (ctx.state.storyboard.zoom / 100.0).clamp(0.6, 1.5) as f32;
    let paper_ink = Color32::from_rgb(0x1f, 0x26, 0x30);
    let paper_muted = Color32::from_rgb(0x57, 0x60, 0x6a);

    egui::Frame::new()
        .fill(ctx.pal.stage_paper)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(egui::Margin::same((16.0 * zoom_scale).round() as i8))
        .stroke(Stroke::new(1.5, if sp.filled { ctx.pal.accent_emerald } else { ctx.pal.border }))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{:02} · {}", sp.index + 1, sp.title)).size(16.0 * zoom_scale).strong().color(paper_ink));
                status_pill(ui, ctx.pal, if sp.filled { "Ready" } else { "Needs Content" }, if sp.filled { ctx.pal.accent_emerald } else { ctx.pal.danger });
            });
            ui.add_space(6.0);

            for slot in &sp.slots {
                let key = slot["label"].as_str().or_else(|| slot["id"].as_str()).unwrap_or("slot");
                if key.ends_with("_cli") {
                    continue;
                }
                let kind = slot["kind"].as_str().unwrap_or("text");
                let val = slot["text"].as_str().or_else(|| slot["image"].as_str()).or_else(|| slot["value"].as_str()).unwrap_or("");
                let is_empty = val.trim().is_empty() || (val.contains('[') && val.contains(']'));

                egui::Frame::new()
                    .fill(Color32::from_rgb(0xed, 0xe5, 0xd4))
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(egui::Margin::same(10))
                    .stroke(Stroke::new(1.0, if is_empty { ctx.pal.danger } else { Color32::from_rgb(0xc8, 0xbe, 0xaa) }))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("{key} ({kind})")).size(11.5).strong().color(paper_ink));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let pick_id = format!("sb-slot-pick-{}-{key}", sp.index);
                                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new(&pick_id, "pen", "Edit", "Load slot into inspector")).clicked() {
                                    ctx.state.storyboard.current_page = sp.index;
                                    ctx.state.storyboard.selected_slot = key.to_owned();
                                    ctx.state.storyboard.slot_value = val.to_owned();
                                }
                            });
                        });
                        ui.label(
                            RichText::new(if is_empty { "Unfilled requirement — select Edit or run Wizard / Bulk" } else { val })
                                .size(12.0)
                                .color(if is_empty { ctx.pal.danger } else { paper_ink }),
                        );
                        let cli_hint = format!("magic-cli storyboard set page={} label={key}", sp.index);
                        ui.label(RichText::new(cli_hint).monospace().size(9.5).color(paper_muted));
                    });
                ui.add_space(6.0);
            }
        });
}

/// Right Pane: Navigation panel for all Storyboard requirements showing a red or green light.
fn draw_requirements_panel(ui: &mut Ui, ctx: &mut ViewCtx<'_>, spreads: &[SpreadSummary]) {
    let filled_total = spreads.iter().filter(|s| s.filled).count();
    let total = spreads.len().max(1);
    let panel = ctx.pal.card_frame().show(ui, |ui| {
        ui.label(RichText::new("Requirements & Readiness").size(13.0).strong());
        ui.label(RichText::new(format!("{filled_total} / {total} spreads filled · click any item to jump")).size(10.5).color(ctx.pal.text_muted));

        let bar_w = ui.available_width().max(120.0);
        let (bar_rect, _) = ui.allocate_exact_size(Vec2::new(bar_w, 6.0), Sense::hover());
        ui.painter().rect_filled(bar_rect, CornerRadius::same(3), ctx.pal.border);
        let ratio = (filled_total as f32) / (total as f32);
        if ratio > 0.0 {
            let fill_rect = Rect::from_min_size(bar_rect.min, Vec2::new(bar_w * ratio, 6.0));
            ui.painter().rect_filled(fill_rect, CornerRadius::same(3), ctx.pal.accent_amber);
        }
        ui.add_space(4.0);

        ScrollArea::vertical().id_salt("sb_reqs_list").max_height(260.0).show(ui, |ui| {
            for sp in spreads {
                let req_id = format!("sb-req-page-{}", sp.index);
                let detail = if sp.filled { "Filled" } else { "Missing" };
                let selected = ctx.state.storyboard.current_page == sp.index;
                if requirement_light_row(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    RequirementRow { id: &req_id, title: &format!("{:02} · {}", sp.index + 1, sp.title), detail, filled: sp.filled, selected },
                )
                .clicked()
                {
                    ctx.state.storyboard.current_page = sp.index;
                    if let Some(first_slot) = sp.slots.iter().find_map(|s| s["label"].as_str().filter(|l| !l.ends_with("_cli"))) {
                        ctx.state.storyboard.selected_slot = first_slot.to_owned();
                    }
                }
            }
        });

        ui.separator();
        ui.label(RichText::new("Slot Inspector").size(12.0).strong().color(ctx.pal.accent_amber));
        ring_slider_h(ui, ctx.reg, ctx.pal, ScalarSpec::new("sb-zoom", "Page Zoom", 60.0..=150.0, "%", ctx.pal.accent_sky), &mut ctx.state.storyboard.zoom);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Slot").size(11.0).color(ctx.pal.text_muted));
            text_input(ui, ctx.reg, "sb-slot-key", "Slot label", &mut ctx.state.storyboard.selected_slot, "cover_title", 180.0);
        });
        let field_id = format!("sb-canvas-field-{}", ctx.state.storyboard.selected_slot);
        text_input(ui, ctx.reg, &field_id, "Slot value", &mut ctx.state.storyboard.slot_value, "Text or image/Craft path…", 268.0);
        let save_id = format!("sb-canvas-save-{}", ctx.state.storyboard.selected_slot);
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new(&save_id, "save", "Save", "Write slot to storyboard.designcraft").primary()).clicked() {
            let page_idx = ctx.state.storyboard.current_page;
            let slot_key = ctx.state.storyboard.selected_slot.clone();
            let val = ctx.state.storyboard.slot_value.clone();
            let is_img = val.ends_with(".png") || val.ends_with(".jpg") || val.ends_with(".ecproj") || val.ends_with(".pcraft") || val.ends_with(".svg");
            let payload =
                if is_img { json!({"page": page_idx, "label": slot_key, "image": val}) } else { json!({"page": page_idx, "label": slot_key, "text": val}) };
            ctx.exec("storyboard.set", payload, Some("Saved storyboard slot"));
        }
    });
    ctx.reg.record("sb-requirements-panel", "Storyboard Requirements", &panel.response, None);
}
