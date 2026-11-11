//! `Storyboard` view (`#view-storyboard`): 3-pane `DesignCraft` (`storyboard.designcraft`) studio
//! matching `03-storyboard-3pane-designcraft.jpg`:
//! 1. Left (`Page Sections (2-up)`, 284px): portrait 2-page spread pairs framed in vertical 35mm filmbars.
//! 2. Center (`Active Storyboard Spread — storyboard.designcraft`): scrollable DesignCraft page card
//!    with lyric lines, linked `.pcraft`/`.vectorcraft` image slot, and bottom action bar
//!    (`[Render Spread] [Sync References] [Open in DesignCraft] [Export PDF]`).
//! 3. Right (`Requirements & Readiness`, 316px): 3-ring glowing green/red requirement rows with Craft file badges.

use craft_ui::layout::{SplitAxis, SplitSize, split_rect};
use egui::{
    Align, Align2, CornerRadius, FontId, Layout, Pos2, Rect, RichText, ScrollArea, Sense, Stroke,
    StrokeKind, Ui, Vec2,
};
use serde_json::{Value, json};

use crate::controls::{
    FilmbarOrientation, FilmbarStyle, RequirementRow, ScalarSpec, badge_accent_color, child_pane,
    fill_card, paint_craft_badge, paint_craft_doc_card, paint_filmbar_frame, paint_local_image,
    requirement_light_row, ring_slider_h,
};
use crate::state::{UiAction, ViewCtx, ViewMode};
use crate::widgets::{BtnSpec, action_button, status_pill, text_input};

struct SpreadSummary {
    index: usize,
    title: String,
    archetype: String,
    filled: bool,
    preview_png: Option<String>,
    hero_image: Option<String>,
    slots: Vec<Value>,
}

pub fn show(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let spreads = collect_spreads(ctx);

    draw_toolbar(ui, ctx, &spreads);
    if ctx.state.storyboard.show_wizard {
        ui.add_space(6.0);
        draw_wizard_bar(ui, ctx);
    }
    ui.add_space(6.0);

    let body = ui.available_rect_before_wrap();
    if spreads.is_empty() {
        draw_missing_designcraft_state(ui, ctx);
        return;
    }

    let after_left = if ctx.state.show_nav_rail
        && let Some(left_split) = split_rect(body, SplitAxis::Horizontal, SplitSize::FixedFirst(284.0), 8.0, 220.0)
    {
        let mut left = child_pane(ui, left_split.first, "sb_left_2up");
        draw_two_up_rail(&mut left, ctx, &spreads);
        left_split.second
    } else {
        body
    };

    if ctx.state.show_right_inspector
        && let Some(right_split) = split_rect(after_left, SplitAxis::Horizontal, SplitSize::FixedSecond(316.0), 8.0, 220.0)
    {
        let mut center = child_pane(ui, right_split.first, "sb_center_scroll");
        draw_single_page_scroll(&mut center, ctx, &spreads);
        let mut right = child_pane(ui, right_split.second, "sb_right_reqs");
        draw_requirements_panel(&mut right, ctx, &spreads);
    } else {
        let mut center = child_pane(ui, after_left, "sb_center_scroll");
        draw_single_page_scroll(&mut center, ctx, &spreads);
    }
    let _ = ui.allocate_rect(body, Sense::hover());
}

fn draw_missing_designcraft_state(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    fill_card(ui, ctx.pal, true, |ui| {
        let avail = ui.available_size();
        let box_h = (avail.y - 54.0).clamp(160.0, 340.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::new(avail.x.max(240.0), box_h), Sense::hover());
        paint_craft_doc_card(
            ui,
            rect,
            ctx.pal,
            "DC",
            "storyboard.designcraft",
            "No DesignCraft storyboard document found in this project yet.",
        );
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("storyboard-empty-init", "plus", "Initialize storyboard.designcraft", "Create storyboard.designcraft from DesignCraft template")
                    .primary(),
            )
            .clicked()
            {
                ctx.exec("storyboard.new", json!({}), Some("Initialized storyboard.designcraft"));
            }
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("storyboard-empty-reload", "refresh", "Reload", "Inspect existing storyboard.designcraft"),
            )
            .clicked()
            {
                ctx.exec("storyboard.pages", json!({"full": true}), None);
                ctx.exec("storyboard.check", json!({}), None);
            }
        });
    });
}

fn collect_spreads(ctx: &ViewCtx<'_>) -> Vec<SpreadSummary> {
    let raw_pages = ctx
        .state
        .cache
        .storyboard_pages
        .get("pages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let check_pages = ctx
        .state
        .cache
        .storyboard_check
        .get("pages")
        .or_else(|| ctx.state.cache.snapshot.pointer("/data/storyboard/pages"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    raw_pages
        .into_iter()
        .enumerate()
        .map(|(idx, p)| {
            let arch = p["archetype"].as_str().unwrap_or("scene").to_owned();
            let title = p["name"]
                .as_str()
                .or_else(|| p["title"].as_str())
                .filter(|s| !s.trim().is_empty())
                .map(str::to_owned)
                .unwrap_or_else(|| default_archetype_title(idx, &arch));
            let slots = p["slots"].as_array().cloned().unwrap_or_default();
            let check_light = check_pages
                .iter()
                .find(|cp| cp["page"].as_u64() == Some(idx as u64))
                .and_then(|cp| cp["light"].as_str());
            let filled = check_light == Some("green") || (check_light.is_none() && page_slots_filled(&slots));
            let preview_png = p["preview_png"]
                .as_str()
                .filter(|img| !img.trim().is_empty())
                .map(str::to_owned);
            let slot_image = slots.iter().find_map(|s| {
                s["image"]
                    .as_str()
                    .filter(|img| !img.trim().is_empty())
                    .map(str::to_owned)
            });
            let hero_image = preview_png.clone().or(slot_image);
            SpreadSummary {
                index: idx,
                title,
                archetype: arch,
                filled,
                preview_png,
                hero_image,
                slots,
            }
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
        if label.ends_with("_cli")
            || s["locked"].as_bool() == Some(true)
            || label.starts_with("ref_")
            || label.starts_with("cover_reg_")
            || label.starts_with("cover_songmap_")
        {
            continue;
        }
        checked += 1;
        let val = s["text"]
            .as_str()
            .or_else(|| s["image"].as_str())
            .or_else(|| s["value"].as_str())
            .unwrap_or("");
        let trimmed = val.trim();
        if trimmed.is_empty() || (trimmed.contains('[') && trimmed.contains(']')) {
            return false;
        }
    }
    checked > 0
}

fn draw_toolbar(ui: &mut Ui, ctx: &mut ViewCtx<'_>, spreads: &[SpreadSummary]) {
    let main_ready = spreads.iter().take(4).filter(|s| s.filled).count();
    let complete = !spreads.is_empty() && main_ready >= 4.min(spreads.len()) && spreads.iter().all(|s| s.filled);
    ctx.pal.panel_frame().show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            let pill_color = if complete {
                ctx.pal.accent_emerald
            } else if main_ready > 0 {
                ctx.pal.accent_amber
            } else {
                ctx.pal.danger
            };
            status_pill(ui, ctx.pal, &format!("{main_ready}/4 Main Pages · storyboard.designcraft"), pill_color);

            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("storyboard-new", "plus", "Init", "Initialize storyboard.designcraft")).clicked() {
                ctx.exec("storyboard.new", json!({}), Some("Initialized storyboard.designcraft"));
            }
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("storyboard-wizard-toggle", "wand", "Wizard", "Fill Main Section Title Pages")
                    .selected(ctx.state.storyboard.show_wizard),
            )
            .clicked()
            {
                ctx.state.storyboard.show_wizard = !ctx.state.storyboard.show_wizard;
            }
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("storyboard-bulk-generate", "sparkles", "Bulk Generate", "Generate scene spreads from title pages").primary(),
            )
            .clicked()
            {
                ctx.exec("storyboard.bulk_generate", json!({}), Some("Generated storyboard scenes"));
            }
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("storyboard-render-rail", "image", "Render Spreads", "Render DesignCraft page PNGs via designcraft-cli"),
            )
            .clicked()
            {
                ctx.exec("storyboard.thumbnail", json!({"rail": true, "scale": 0.75}), Some("Rendered DesignCraft page previews"));
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("storyboard-refresh", "check", "Check Gate", "Verify readiness gate & refresh slots")).clicked() {
                ctx.exec("storyboard.check", json!({}), None);
                ctx.exec("storyboard.pages", json!({"full": true}), None);
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("storyboard-export", "download", "Export PDF", "Export storyboard.pdf via DesignCraft")).clicked() {
                ctx.exec("storyboard.export", json!({}), Some("Exported storyboard.pdf"));
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::new("storyboard-go-studio", "film", "Studio →", "Proceed to FilmCraft Studio").enabled(complete),
                )
                .clicked()
                {
                    ctx.push(UiAction::SwitchView(ViewMode::Studio));
                }
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::new("storyboard-open-editor", "external", "Open in DesignCraft", "Open storyboard.designcraft in DesignCraft"),
                )
                .clicked()
                {
                    ctx.exec("storyboard.open", json!({}), Some("Opened DesignCraft"));
                }
            });
        });
    });
}

fn draw_wizard_bar(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    ctx.pal.card_frame().show(ui, |ui| {
        ui.vertical(|ui| {
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
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::new("storyboard-wizard-apply", "check", "Apply Wizard", "Write title-page prompts").primary(),
                )
                .clicked()
                {
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
    });
}

/// Left Pane: portrait 2-up spread pairs matching `03-storyboard-3pane-designcraft.jpg`.
fn draw_two_up_rail(ui: &mut Ui, ctx: &mut ViewCtx<'_>, spreads: &[SpreadSummary]) {
    let panel = fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Page Sections (2-up)").size(13.0).strong());
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
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

        ScrollArea::vertical().id_salt("sb_2up_scroll").auto_shrink([false, false]).show(ui, |ui| {
            let mut row_idx = 0usize;
            while row_idx < spreads.len() {
                ui.horizontal(|ui| {
                    let pair_w = ((ui.available_width() - 6.0) * 0.5).max(104.0);
                    draw_mini_spread_card(ui, ctx, &spreads[row_idx], pair_w);
                    if let Some(second) = spreads.get(row_idx + 1) {
                        draw_mini_spread_card(ui, ctx, second, pair_w);
                    } else {
                        draw_add_spread_tile(ui, ctx, pair_w);
                    }
                });
                ui.add_space(6.0);
                row_idx += 2;
            }
            if spreads.len().is_multiple_of(2) {
                let pair_w = ((ui.available_width() - 6.0) * 0.5).max(104.0);
                draw_add_spread_tile(ui, ctx, pair_w);
            }
        });
    });
    ctx.reg.record("sb-2up-rail", "2-Up Spreads", &panel.response, None);
}

fn draw_mini_spread_card(ui: &mut Ui, ctx: &mut ViewCtx<'_>, sp: &SpreadSummary, width: f32) {
    let selected = ctx.state.storyboard.current_page == sp.index;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, 168.0), Sense::click());
    if resp.clicked() {
        ctx.state.storyboard.current_page = sp.index;
    }
    let border = if selected {
        Stroke::new(1.4, ctx.pal.accent_emerald)
    } else {
        Stroke::new(1.0, ctx.pal.border_subtle)
    };
    ui.painter().rect(rect, CornerRadius::same(7), ctx.pal.bg_input, border, StrokeKind::Inside);

    ui.painter().text(
        Pos2::new(rect.min.x + 6.0, rect.min.y + 11.0),
        Align2::LEFT_CENTER,
        format!("P{:02} · {}", sp.index + 1, sp.title),
        FontId::proportional(10.0),
        if selected { ctx.pal.text_primary } else { ctx.pal.text_secondary },
    );

    let thumb_rect = Rect::from_min_max(
        Pos2::new(rect.min.x + 4.0, rect.min.y + 21.0),
        Pos2::new(rect.max.x - 4.0, rect.max.y - 18.0),
    );
    let inner = paint_filmbar_frame(
        ui,
        thumb_rect,
        ctx.pal,
        FilmbarStyle {
            orientation: FilmbarOrientation::Vertical,
            selected,
            hovered: resp.hovered(),
            timecode: None,
        },
    );
    if !paint_local_image(ui, inner, sp.hero_image.as_deref(), ctx.pal.bg_stage, 3) {
        paint_craft_doc_card(ui, inner, ctx.pal, "DC", &sp.title, "");
    }

    ui.painter().text(
        Pos2::new(rect.min.x + 5.0, rect.max.y - 8.5),
        Align2::LEFT_CENTER,
        "storyboard.designcraft",
        FontId::monospace(8.2),
        ctx.pal.text_muted,
    );
    ctx.reg.record(format!("sb-nav-{}", sp.index + 1), &sp.title, &resp, None);
}

fn draw_add_spread_tile(ui: &mut Ui, ctx: &mut ViewCtx<'_>, width: f32) {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, 168.0), Sense::click());
    let stroke_col = if resp.hovered() { ctx.pal.accent_emerald } else { ctx.pal.border };
    ui.painter()
        .rect(rect, CornerRadius::same(7), ctx.pal.bg_input, Stroke::new(1.0, stroke_col), StrokeKind::Inside);
    ui.painter()
        .text(rect.center(), Align2::CENTER_CENTER, "+ Add Spread", FontId::proportional(11.0), ctx.pal.text_muted);
    if resp.clicked() {
        let arch = ctx.state.storyboard.archetype.clone();
        ctx.exec("storyboard.add", json!({"archetype": arch}), Some("Added spread"));
    }
    ctx.reg.record("sb-add-tile", "Add Page", &resp, None);
}

fn draw_single_page_scroll(ui: &mut Ui, ctx: &mut ViewCtx<'_>, spreads: &[SpreadSummary]) {
    let active_idx = ctx.state.storyboard.current_page.min(spreads.len().saturating_sub(1));
    let active = spreads.get(active_idx);
    let panel = fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal(|ui| {
            let heading = active.map_or_else(
                || "Active Storyboard Spread — storyboard.designcraft".to_owned(),
                |sp| format!("Active Storyboard Spread — storyboard.designcraft (P{:02} · {})", sp.index + 1, sp.title),
            );
            ui.label(RichText::new(heading).size(13.0).strong().color(ctx.pal.text_primary));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
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
                ui.label(
                    RichText::new(format!("Page {} of {}", active_idx + 1, spreads.len().max(1)))
                        .monospace()
                        .size(10.5)
                        .color(ctx.pal.text_muted),
                );
            });
        });
        ui.separator();

        let body_h = (ui.available_height() - 38.0).max(160.0);
        ScrollArea::vertical()
            .id_salt("sb_single_page_scroll")
            .max_height(body_h)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (offset, sp) in spreads.iter().skip(active_idx).take(3).enumerate() {
                    if offset > 0 {
                        ui.add_space(14.0);
                        ui.label(
                            RichText::new(format!("↓ Continues to P{:02} · {}", sp.index + 1, sp.title))
                                .monospace()
                                .size(10.5)
                                .color(ctx.pal.text_muted),
                        );
                        ui.add_space(4.0);
                    }
                    draw_designcraft_spread_sheet(ui, ctx, sp);
                }
            });

        // Bottom spread action bar (`[Render Spread] [Sync References] [Open in DesignCraft] [Export PDF]`, `03-storyboard-3pane-designcraft.jpg`)
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("sb-bottom-render", "sparkles", "Render Spread", "Render DesignCraft spread PNG"),
            )
            .clicked()
            {
                ctx.exec("storyboard.thumbnail", json!({"page": active_idx, "scale": 1.0}), Some("Rendered spread PNG"));
            }
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("sb-bottom-sync", "refresh", "Sync References", "Re-check & refresh DesignCraft references"),
            )
            .clicked()
            {
                ctx.exec("storyboard.check", json!({}), None);
                ctx.exec("storyboard.pages", json!({"full": true}), Some("Synchronized storyboard references"));
            }
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("sb-bottom-open-dc", "external", "Open in DesignCraft", "Open storyboard.designcraft in DesignCraft"),
            )
            .clicked()
            {
                ctx.exec("storyboard.open", json!({}), Some("Opened DesignCraft"));
            }
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("sb-bottom-export-pdf", "download", "Export PDF", "Export storyboard.pdf via DesignCraft"),
            )
            .clicked()
            {
                ctx.exec("storyboard.export", json!({}), Some("Exported storyboard.pdf"));
            }
        });
    });
    ctx.reg.record("sb-single-page", "Single Page Canvas", &panel.response, None);
}

fn draw_designcraft_spread_sheet(ui: &mut Ui, ctx: &mut ViewCtx<'_>, sp: &SpreadSummary) {
    let zoom_scale = (ctx.state.storyboard.zoom / 100.0).clamp(0.6, 1.5) as f32;
    egui::Frame::new()
        .fill(ctx.pal.bg_input)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(egui::Margin::same((14.0 * zoom_scale).round() as i8))
        .stroke(Stroke::new(1.4, if sp.filled { ctx.pal.accent_emerald.gamma_multiply(0.75) } else { ctx.pal.border }))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("DesignCraft · P{:02} {}", sp.index + 1, sp.title))
                            .size(14.0 * zoom_scale)
                            .strong()
                            .color(ctx.pal.text_primary),
                    );
                    status_pill(
                        ui,
                        ctx.pal,
                        if sp.filled { "Ready" } else { "Needs Content" },
                        if sp.filled { ctx.pal.accent_emerald } else { ctx.pal.danger },
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        paint_craft_badge(ui, ctx.pal, ".designcraft", true);
                    });
                });

                if let Some(prev_png) = sp.preview_png.as_deref() {
                    ui.add_space(6.0);
                    let pw = ui.available_width().max(200.0);
                    let ph = (pw * 9.0 / 16.0).clamp(130.0, 250.0);
                    let (prect, _) = ui.allocate_exact_size(Vec2::new(pw, ph), Sense::hover());
                    let _ = paint_local_image(ui, prect, Some(prev_png), ctx.pal.bg_stage, 6);
                }
                ui.add_space(6.0);

                for slot in &sp.slots {
                    draw_spread_slot_card(ui, ctx, sp.index, slot);
                }
            });
        });
}

fn draw_spread_slot_card(ui: &mut Ui, ctx: &mut ViewCtx<'_>, page_idx: usize, slot: &Value) {
    let key = slot["label"].as_str().or_else(|| slot["id"].as_str()).unwrap_or("slot");
    if key.ends_with("_cli") || slot["hidden"].as_bool() == Some(true) {
        return;
    }
    let kind = slot["kind"].as_str().unwrap_or("text");
    let img_path = slot["image"].as_str().filter(|s| !s.trim().is_empty());
    let src_proj = slot["source_project"].as_str().filter(|s| !s.trim().is_empty());
    let craft_app = slot["craft_app"].as_str().filter(|s| !s.trim().is_empty());
    let val = slot["text"]
        .as_str()
        .or(img_path)
        .or_else(|| slot["value"].as_str())
        .unwrap_or("");
    let is_empty = val.trim().is_empty() || (val.contains('[') && val.contains(']'));

    egui::Frame::new()
        .fill(ctx.pal.bg_panel)
        .corner_radius(CornerRadius::same(6))
        .inner_margin(egui::Margin::same(10))
        .stroke(Stroke::new(1.0, if is_empty { ctx.pal.danger } else { ctx.pal.border_subtle }))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.vertical(|ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(format!("{key} · {kind}")).size(11.5).strong().color(ctx.pal.text_primary));
                    if let Some(proj) = src_proj {
                        let fname = proj.rsplit('/').next().unwrap_or(proj);
                        let ext = fname.rsplit('.').next().unwrap_or("CRAFT");
                        status_pill(ui, ctx.pal, fname, badge_accent_color(ctx.pal, ext));
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let pick_id = format!("sb-slot-pick-{page_idx}-{key}");
                        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new(&pick_id, "pen", "Edit", "Load slot into inspector")).clicked() {
                            ctx.state.storyboard.current_page = page_idx;
                            ctx.state.storyboard.selected_slot = key.to_owned();
                            ctx.state.storyboard.slot_value = src_proj.unwrap_or(val).to_owned();
                        }
                        if let (Some(proj), Some(app)) = (src_proj, craft_app) {
                            let open_id = format!("sb-slot-open-{page_idx}-{key}");
                            let lbl = format!("Open in {app} ↗");
                            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&open_id, &lbl, "Open source Craft project")).clicked() {
                                ctx.exec("workspace.open", json!({"app": app, "path": proj}), Some("Opened linked Craft project"));
                            }
                        }
                    });
                });
                if kind == "image" {
                    let img_w = ui.available_width().max(180.0);
                    let img_h = (img_w * 9.0 / 16.0).clamp(110.0, 210.0);
                    let (irect, _) = ui.allocate_exact_size(Vec2::new(img_w, img_h), Sense::hover());
                    if !paint_local_image(ui, irect, img_path, ctx.pal.bg_stage, 6) {
                        let hint_file = src_proj.or(img_path).unwrap_or("Assign .pcraft / .ecproj / .vectorcraft / .svg / .fcproj");
                        paint_craft_doc_card(ui, irect, ctx.pal, "DC", key, hint_file);
                    }
                } else {
                    ui.label(
                        RichText::new(if is_empty {
                            "Unfilled DesignCraft story slot — click Edit or run Wizard / Bulk Generate"
                        } else {
                            val
                        })
                        .size(12.5)
                        .color(if is_empty { ctx.pal.danger } else { ctx.pal.text_primary }),
                    );
                }
            });
        });
    ui.add_space(6.0);
}

fn craft_ext_for_archetype(arch: &str) -> &'static str {
    match arch {
        "cover" => ".designcraft",
        "world" => ".svg",
        "char" => ".pcraft",
        "motion" => ".ecproj",
        _ => ".fcproj",
    }
}

fn draw_requirements_panel(ui: &mut Ui, ctx: &mut ViewCtx<'_>, spreads: &[SpreadSummary]) {
    let filled_total = spreads.iter().filter(|s| s.filled).count();
    let total = spreads.len().max(1);
    let panel = fill_card(ui, ctx.pal, true, |ui| {
        ui.label(RichText::new("Requirements & Readiness").size(13.0).strong());
        ui.label(
            RichText::new(format!("{filled_total} / {total} scene & asset requirements ready"))
                .size(10.5)
                .color(ctx.pal.text_muted),
        );

        let bar_w = ui.available_width().max(120.0);
        let (bar_rect, _) = ui.allocate_exact_size(Vec2::new(bar_w, 6.0), Sense::hover());
        ui.painter().rect_filled(bar_rect, CornerRadius::same(3), ctx.pal.border);
        let ratio = (filled_total as f32) / (total as f32);
        if ratio > 0.0 {
            let fill_col = if filled_total == total {
                ctx.pal.accent_emerald
            } else {
                ctx.pal.accent_amber
            };
            let fill_rect = Rect::from_min_size(bar_rect.min, Vec2::new(bar_w * ratio, 6.0));
            ui.painter().rect_filled(fill_rect, CornerRadius::same(3), fill_col);
        }
        ui.add_space(6.0);

        let list_h = (ui.available_height() - 188.0).max(150.0);
        ScrollArea::vertical().id_salt("sb_reqs_list").max_height(list_h).show(ui, |ui| {
            for sp in spreads {
                let req_id = format!("sb-req-page-{}", sp.index);
                let detail = craft_ext_for_archetype(&sp.archetype);
                let selected = ctx.state.storyboard.current_page == sp.index;
                if requirement_light_row(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    RequirementRow {
                        id: &req_id,
                        title: &format!("P{:02} · {}", sp.index + 1, sp.title),
                        detail,
                        filled: sp.filled,
                        selected,
                    },
                )
                .clicked()
                {
                    ctx.state.storyboard.current_page = sp.index;
                    if let Some(first_slot) = sp
                        .slots
                        .iter()
                        .find_map(|s| s["label"].as_str().filter(|l| !l.ends_with("_cli")))
                    {
                        ctx.state.storyboard.selected_slot = first_slot.to_owned();
                    }
                }
                ui.add_space(2.0);
            }
        });

        ui.separator();
        ui.label(RichText::new("Slot & Craft Reference Inspector").size(12.0).strong().color(ctx.pal.accent_amber));
        ring_slider_h(
            ui,
            ctx.reg,
            ctx.pal,
            ScalarSpec::new("sb-zoom", "Page Zoom", 60.0..=150.0, "%", ctx.pal.accent_emerald),
            &mut ctx.state.storyboard.zoom,
        );
        ui.horizontal(|ui| {
            ui.label(RichText::new("Slot").size(11.0).color(ctx.pal.text_muted));
            text_input(ui, ctx.reg, "sb-slot-key", "Slot label", &mut ctx.state.storyboard.selected_slot, "cover_title", 210.0);
        });
        let field_id = format!("sb-canvas-field-{}", ctx.state.storyboard.selected_slot);
        text_input(
            ui,
            ctx.reg,
            &field_id,
            "Slot value or Craft file",
            &mut ctx.state.storyboard.slot_value,
            "Text or .pcraft / .ecproj / .svg / .fcproj…",
            272.0,
        );
        let save_id = format!("sb-canvas-save-{}", ctx.state.storyboard.selected_slot);
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::new(&save_id, "save", "Save to DesignCraft", "Flatten Craft project & write slot to storyboard.designcraft").primary(),
        )
        .clicked()
        {
            let page_idx = ctx.state.storyboard.current_page;
            let slot_key = ctx.state.storyboard.selected_slot.clone();
            let val = ctx.state.storyboard.slot_value.clone();
            let lower = val.to_ascii_lowercase();
            let is_img_or_craft = [
                ".png",
                ".jpg",
                ".jpeg",
                ".webp",
                ".svg",
                ".ecproj",
                ".pcraft",
                ".psd",
                ".vectorcraft",
                ".anicraft",
                ".fcproj",
            ]
            .iter()
            .any(|ext| lower.ends_with(ext));
            let payload = if is_img_or_craft {
                json!({"page": page_idx, "label": slot_key, "image": val})
            } else {
                json!({"page": page_idx, "label": slot_key, "text": val})
            };
            ctx.exec("storyboard.set", payload, Some("Saved storyboard.designcraft slot"));
        }
    });
    ctx.reg.record("sb-requirements-panel", "Storyboard Requirements", &panel.response, None);
}
