<<<<<<< HEAD
//! `Storyboard` view (`#view-storyboard`): Anti-slop 16:9 Barnyard Production Bible with strict
//! **Sections → Section Cover / Title Pages → Sub-Pages** hierarchy:
//! 1. Left (`Book Hierarchy · Sections & Scenes`): scrollable hierarchical navigator grouping every
//!    Section (`01 Cover`, `02 Backgrounds`, `03 Characters`, `04 Scenes`, `05 Assets`, `06 Final`)
//!    into its dedicated **Section Cover / Title Card** followed by indented **Sub-Pages**
//!    (`1.1`, `2.1`, `3.1..3.N`, `SC-01..SC-NN`) with tree guide lines and 16:9 live thumbnails.
//! 2. Center (`16:9 Production Bible · Barnyard Spreads`): crisp widescreen (`960 × 540 pt`) slides
//!    containing 100% of `Storyboard.potx` fields (`PRODUCTION / VIDEO INSTRUCTION`, `CAST · PERFORMERS`,
//!    `LOCATIONS 1–5`, `SONG MAP`, `SEQUENCE MAP`, `MOVEMENT VOCABULARY`, `SCENE BEAT`, `PRODUCTION HANDOFF`)
//!    with click-in-place text editing and drag/drop/paste/click Craft artwork slots.
//! 3. Right (`Production Checklist · Barnyard Fields`): grouped field navigator synced 1:1 with
//!    `storyboard.check` to jump directly to any text or image slot.
=======
//! `Storyboard` view (`#view-storyboard`): Editable `DesignCraft` (`storyboard.designcraft`)
//! design document (`612 × 792 pt` US Letter) coordinated with host `DesignCraft` (`--control 7979` / `designcraft-cli`):
//! 1. Left (`Book Hierarchy · Sections & Spreads`): scrollable navigator driven directly by
//!    `doc["sections"]` in `storyboard.designcraft` (`Cover`, `Backgrounds`, `Characters`,
//!    `Scenes`, `Assets`, `Final`).
//! 2. Center (`DesignCraft Storyboard · Editable Spreads`): 1-page or 2-page spread scroll reading
//!    `document.json` frames directly without intermediate slide interpretations, with inline text
//!    editing and drag/drop/paste/click `PhotoCraft` / `VectorCraft` / `EffectCraft` artwork slots.
//! 3. Right (`Field Checklist`): Red/Green readiness navigator synced 1:1 with `storyboard.check`.
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)

use egui::{Align, Align2, Color32, CornerRadius, FontId, Layout, Pos2, Rect, RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui, Vec2};
use serde_json::{Value, json};

use super::storyboard_page::{DEFAULT_PAGE_H_PT, DEFAULT_PAGE_W_PT, PageSummary, draw_interactive_page, paint_mini_page};
use crate::controls::{RequirementRow, ScalarSpec, fill_card, paint_craft_doc_card, requirement_light_row, ring_slider_h};
<<<<<<< HEAD
use crate::editorial::{ThreePaneSpec, draw_pane_kicker_header, draw_three_pane_workspace};
use crate::state::ViewCtx;
use crate::widgets::{BtnSpec, action_button, status_pill};

pub fn show(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let pages = collect_pages(ctx);
    draw_toolbar(ui, ctx, &pages);
    ui.add_space(6.0);

=======
use crate::editorial::draw_pane_kicker_header;
use crate::fonts_ui::{FontPickerSpec, font_family_combo};
use crate::state::ViewCtx;
use crate::widgets::{BtnSpec, action_button, status_pill};

pub fn draw_stage_header(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let pages = collect_pages(ctx);
    ctx.pal.panel_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        draw_toolbar(ui, ctx, &pages);
    });
}

pub fn draw_navigator_panel(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let pages = collect_pages(ctx);
    draw_hierarchical_book_rail(ui, ctx, &pages);
}

pub fn draw_canvas_panel(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let pages = collect_pages(ctx);
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    if pages.is_empty() {
        draw_empty_storyboard_state(ui, ctx);
        return;
    }
<<<<<<< HEAD

    let dropped_paths: Vec<String> = ui.ctx().input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_string_lossy().into_owned()).collect());

    let wide = ui.available_width() >= 1220.0;
    draw_three_pane_workspace(
        ui,
        ThreePaneSpec {
            id_prefix: "sb_barnyard",
            left_width: if wide { 296.0 } else { 252.0 },
            right_width: if wide { 296.0 } else { 254.0 },
            gap: 8.0,
            min_center: 260.0,
            show_left: ctx.state.show_nav_rail,
            show_right: ctx.state.show_right_inspector,
        },
        ctx,
        |left, ctx| draw_hierarchical_book_rail(left, ctx, &pages),
        |center, ctx| draw_seamless_document_scroll(center, ctx, &pages, &dropped_paths),
        |right, ctx| draw_requirements_navigator(right, ctx),
    );
=======
    let dropped_paths: Vec<String> = ui.ctx().input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_string_lossy().into_owned()).collect());
    draw_seamless_document_scroll(ui, ctx, &pages, &dropped_paths);
}

pub fn draw_inspector_panel(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    draw_requirements_navigator(ui, ctx);
}

pub fn draw_timeline_panel(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let pages = collect_pages(ctx);
    fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Storyboard Spreads").size(11.5).strong());
            status_pill(ui, ctx.pal, &format!("{} pages", pages.len()), ctx.pal.accent_emerald);
        });
        ScrollArea::horizontal().id_salt("sb_timeline_strip").show(ui, |ui| {
            ui.horizontal(|ui| {
                for pg in &pages {
                    let sel = ctx.state.storyboard.current_page == pg.index;
                    let (r, resp) = ui.allocate_exact_size(Vec2::new(78.0, 98.0), Sense::click());
                    // TODO(agent): verify storyboard_page README for paint_mini_page arg order; d23f5f3 takes (ui, rect, page, selected, accent)
                    paint_mini_page(ui, r, pg, sel, ctx.pal.accent_sky);
                    if resp.on_hover_text(format!("{} · {}", pg.sub_code, pg.title)).clicked() {
                        ctx.state.storyboard.current_page = pg.index;
                        ctx.state.storyboard.scroll_to_page = Some(pg.index);
                    }
                }
            });
        });
    });
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
}

fn collect_pages(ctx: &ViewCtx<'_>) -> Vec<PageSummary> {
    let check_pages = ctx.state.cache.storyboard_check["pages"].as_array();
    ctx.state.cache.storyboard_pages["pages"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|pg| {
            let index = pg["page"].as_u64().unwrap_or(0) as usize;
            let chk = check_pages.and_then(|arr| arr.get(index));
            let slots = pg["slots"].as_array().cloned().unwrap_or_default();
            let total_slots = slots.len();
            let filled_slots = slots.iter().filter(|s| s["filled"] == true).count();
<<<<<<< HEAD
            let light_green =
                chk.and_then(|c| c["light"].as_str()).or_else(|| pg["light"].as_str()).map_or(total_slots > 0 && filled_slots == total_slots, |l| l == "green");
=======
            let light_green = chk
                .and_then(|c| c["light"].as_str())
                .or_else(|| pg["light"].as_str())
                .map_or(total_slots == 0 || filled_slots == total_slots, |l| l == "green");
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
            let bounds = pg["bounds"].as_array();
            let pw = bounds.and_then(|b| b.get(2)).and_then(Value::as_f64).map_or(DEFAULT_PAGE_W_PT, |v| v as f32);
            let ph = bounds.and_then(|b| b.get(3)).and_then(Value::as_f64).map_or(DEFAULT_PAGE_H_PT, |v| v as f32);
            let role = pg["page_role"].as_str().unwrap_or(if index == 0 { "cover" } else { "subpage" }).to_owned();
            let sub_code = pg["sub_code"].as_str().map(str::to_owned).unwrap_or_else(|| format!("P{:02}", index + 1));
            PageSummary {
                index,
                section: pg["section"].as_str().unwrap_or("Scenes").to_owned(),
                page_role: role,
                sub_code,
<<<<<<< HEAD
                title: pg["title"].as_str().unwrap_or("Slide").to_owned(),
=======
                title: pg["title"].as_str().unwrap_or("Page").to_owned(),
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
                page_size_pt: [pw.max(200.0), ph.max(200.0)],
                light_green,
                filled_slots,
                total_slots,
                preview_png: pg["preview_png"].as_str().map(str::to_owned),
                slots,
                layout_items: pg["layout_items"].as_array().cloned().unwrap_or_default(),
            }
        })
        .collect()
}

fn draw_empty_storyboard_state(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    fill_card(ui, ctx.pal, true, |ui| {
        let avail = ui.available_size();
        let box_h = (avail.y - 54.0).clamp(160.0, 320.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::new(avail.x.max(240.0), box_h), Sense::hover());
        paint_craft_doc_card(
            ui,
            rect,
            ctx.pal,
<<<<<<< HEAD
            "16:9",
            "16:9 Production Bible · Storyboard",
            "Initialize the hierarchical 16:9 Barnyard Storyboard (Sections → Cover Pages → Scene Beat Sub-Pages).",
=======
            "DC",
            "DesignCraft Storyboard (storyboard.designcraft)",
            "Initialize the editable storyboard.designcraft design document from the DesignCraft master.",
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
        );
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new(
                    "storyboard-empty-init",
                    "plus",
<<<<<<< HEAD
                    "Initialize Production Bible",
                    "Create 16:9 Barnyard Storyboard from project song, cast, and locations",
=======
                    "Initialize Storyboard (.designcraft)",
                    "Create storyboard.designcraft as a complete editable DesignCraft design document",
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
                )
                .primary(),
            )
            .clicked()
            {
<<<<<<< HEAD
                ctx.exec("storyboard.new", json!({}), Some("Initialized 16:9 Production Bible"));
=======
                ctx.exec("storyboard.new", json!({}), Some("Initialized storyboard.designcraft"));
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
                ctx.exec("storyboard.pages", json!({"full": true}), None);
                ctx.exec("storyboard.check", json!({}), None);
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("storyboard-empty-reload", "refresh", "Reload", "Reload Storyboard pages")).clicked() {
                ctx.exec("storyboard.pages", json!({"full": true}), None);
                ctx.exec("storyboard.check", json!({}), None);
            }
        });
    });
}

fn draw_toolbar(ui: &mut Ui, ctx: &mut ViewCtx<'_>, pages: &[PageSummary]) {
<<<<<<< HEAD
    let filled = ctx.state.cache.storyboard_check["filled_count"].as_u64().unwrap_or_else(|| pages.iter().map(|p| p.filled_slots as u64).sum());
    let total = ctx.state.cache.storyboard_check["total_required"].as_u64().unwrap_or_else(|| pages.iter().map(|p| p.total_slots as u64).sum());
    let scene_count = pages.iter().filter(|p| p.section == "Scenes" && !p.is_cover_or_section_title()).count();
=======
    if let Some(doc_font) = ctx.state.cache.storyboard_pages["font_family"].as_str().filter(|f| !f.is_empty())
        && ctx.state.storyboard.font_family.is_empty()
    {
        ctx.state.storyboard.font_family = doc_font.to_owned();
    }
    let filled = ctx.state.cache.storyboard_check["filled_count"].as_u64().unwrap_or_else(|| pages.iter().map(|p| p.filled_slots as u64).sum());
    let total = ctx.state.cache.storyboard_check["total_required"].as_u64().unwrap_or_else(|| pages.iter().map(|p| p.total_slots as u64).sum());
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    let pill_col = if total > 0 && filled == total { ctx.pal.accent_emerald } else { ctx.pal.accent_amber };

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 5.0;
<<<<<<< HEAD
        status_pill(ui, ctx.pal, &format!("16:9 Production Bible · {} Slides ({scene_count} Scenes) · {filled}/{total} Fields", pages.len()), pill_col);

        let cur = ctx.state.storyboard.current_page.min(pages.len().saturating_sub(1));
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("sb-prev-slide", "chevronLeft", "Previous Storyboard slide").enabled(cur > 0)).clicked() {
=======
        status_pill(ui, ctx.pal, &format!("DesignCraft · {} Pages · {filled}/{total} Fields", pages.len()), pill_col);

        let cur = ctx.state.storyboard.current_page.min(pages.len().saturating_sub(1));
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("sb-prev-slide", "chevronLeft", "Previous Storyboard page").enabled(cur > 0)).clicked() {
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
            let prev = cur.saturating_sub(1);
            ctx.state.storyboard.current_page = prev;
            ctx.state.storyboard.scroll_to_page = Some(prev);
        }
<<<<<<< HEAD
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("sb-next-slide", "chevronRight", "Next Storyboard slide").enabled(cur + 1 < pages.len()))
=======
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("sb-next-slide", "chevronRight", "Next Storyboard page").enabled(cur + 1 < pages.len()))
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
            .clicked()
        {
            let next = (cur + 1).min(pages.len().saturating_sub(1));
            ctx.state.storyboard.current_page = next;
            ctx.state.storyboard.scroll_to_page = Some(next);
        }

        let single_sel = !ctx.state.storyboard.double_page_mode;
<<<<<<< HEAD
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::label_only("sb-mode-single", "1-Slide Focus", "Large widescreen 16:9 slide scroll").selected(single_sel),
        )
        .clicked()
=======
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("sb-mode-single", "pageSingle", "Single-page DesignCraft view").selected(single_sel))
            .clicked()
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
        {
            ctx.state.storyboard.double_page_mode = false;
        }
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
<<<<<<< HEAD
            BtnSpec::label_only("sb-mode-double", "2-Slide Spreads", "Side-by-side 2-up slide spread view").selected(!single_sel),
=======
            BtnSpec::icon_only("sb-mode-double", "spreadTwoPage", "Side-by-side DesignCraft 2-page spread view").selected(!single_sel),
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
        )
        .clicked()
        {
            ctx.state.storyboard.double_page_mode = true;
        }

<<<<<<< HEAD
        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("sb-add-scene-beat", "plus", "+ Scene Beat", "Append a new Barnyard 16:9 Scene Beat sub-page"))
            .clicked()
        {
            ctx.exec("storyboard.add", json!({}), Some("Added new 16:9 Scene Beat slide"));
            ctx.exec("storyboard.pages", json!({"full": true}), None);
            ctx.exec("storyboard.check", json!({}), None);
        }

        ui.allocate_ui_with_layout(Vec2::new(142.0, 24.0), Layout::left_to_right(Align::Center), |ui| {
            ring_slider_h(ui, ctx.reg, ctx.pal, ScalarSpec::new("sb-zoom", "Zoom", 65.0..=145.0, "%", ctx.pal.accent_emerald), &mut ctx.state.storyboard.zoom);
        });

        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("storyboard-refresh", "refresh", "Sync", "Sync Storyboard pages & checklist")).clicked() {
=======
        let mut font_changed = false;
        let avail_fonts = ctx.state.available_fonts.clone();
        if font_family_combo(
            ui,
            ctx.reg,
            ctx.pal,
            FontPickerSpec { id_prefix: "sb-font", label: "Font" },
            &mut ctx.state.storyboard.font_family,
            &avail_fonts,
            &mut ctx.state.font_picker_filter,
            ctx.actions,
        ) {
            font_changed = true;
        }
        if font_changed {
            let chosen = ctx.state.storyboard.font_family.clone();
            ctx.exec("storyboard.set", json!({"font": chosen}), Some("Updated Storyboard typography family"));
        }

        ui.allocate_ui_with_layout(Vec2::new(136.0, 24.0), Layout::left_to_right(Align::Center), |ui| {
            ring_slider_h(ui, ctx.reg, ctx.pal, ScalarSpec::new("sb-zoom", "Zoom", 65.0..=145.0, "%", ctx.pal.accent_emerald), &mut ctx.state.storyboard.zoom);
        });

        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("storyboard-refresh", "refresh", "Sync storyboard.designcraft pages & checklist")).clicked() {
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
            ctx.exec("storyboard.pages", json!({"full": true}), None);
            ctx.exec("storyboard.check", json!({}), None);
        }

        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
<<<<<<< HEAD
            BtnSpec::new("storyboard-open-pdfcraft", "external", "Open in Craft", "Open Storyboard in PDFCraft / DeckCraft (--control)").primary(),
        )
        .clicked()
        {
            ctx.exec("storyboard.open", json!({"page": cur}), Some("Opened Storyboard in Craft"));
=======
            BtnSpec::new("storyboard-render-spreads", "eye", "Render Spreads", "Render storyboard.designcraft spreads via host DesignCraft"),
        )
        .clicked()
        {
            ctx.exec("storyboard.thumbnail", json!({"page": cur, "rail": true}), Some("Rendered DesignCraft storyboard previews"));
        }

        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::new("storyboard-export", "download", "Export", "Export storyboard.designcraft spreads via host DesignCraft"),
        )
        .clicked()
        {
            ctx.exec("storyboard.export", json!({}), Some("Exported storyboard.designcraft spreads"));
        }

        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::new("storyboard-open-designcraft", "external", "DesignCraft", "Open storyboard.designcraft in host DesignCraft").primary(),
        )
        .clicked()
        {
            ctx.exec("storyboard.open", json!({"app": "designcraft", "page": cur}), Some("Opened storyboard.designcraft in DesignCraft"));
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
        }
    });
}

fn ordered_sections(ctx: &ViewCtx<'_>, pages: &[PageSummary]) -> Vec<String> {
    let mut uniq = Vec::new();
    for pg in pages {
        if !uniq.contains(&pg.section) {
            uniq.push(pg.section.clone());
        }
    }
    if uniq.is_empty()
        && let Some(arr) = ctx.state.cache.storyboard_pages["sections"].as_array()
    {
        uniq.extend(arr.iter().filter_map(|v| v.as_str().map(str::to_owned)));
    }
    uniq
}

fn draw_hierarchical_book_rail(ui: &mut Ui, ctx: &mut ViewCtx<'_>, pages: &[PageSummary]) {
    fill_card(ui, ctx.pal, true, |ui| {
        let cur_sec = pages.get(ctx.state.storyboard.current_page).map_or("Cover", |p| p.section.as_str());
        let hdr_accent = ctx.pal.section_accent(cur_sec);
<<<<<<< HEAD
        draw_pane_kicker_header(ui, ctx.pal, "BOOK HIERARCHY", "Sections & Scene Beats", hdr_accent, Some(("16:9", hdr_accent)));
=======
        draw_pane_kicker_header(ui, ctx.pal, "DESIGNCRAFT DOCUMENT", "Sections & Pages", hdr_accent, Some((".designcraft", hdr_accent)));
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
        ui.add_space(5.0);

        let sections = ordered_sections(ctx, pages);
        draw_section_jump_chips(ui, ctx, &sections, pages);
        ui.add_space(6.0);

        ScrollArea::vertical().id_salt("sb_hierarchy_tree_scroll").auto_shrink([false, false]).show(ui, |ui| {
            for (sec_idx, sec_name) in sections.iter().enumerate() {
                let sec_pages: Vec<&PageSummary> = pages.iter().filter(|p| &p.section == sec_name).collect();
                if sec_pages.is_empty() {
                    continue;
                }
                draw_section_hierarchy_group(ui, ctx, sec_idx + 1, sec_name, &sec_pages);
                ui.add_space(8.0);
            }
        });
    });
}

fn draw_section_jump_chips(ui: &mut Ui, ctx: &mut ViewCtx<'_>, sections: &[String], pages: &[PageSummary]) {
    let cur_sec = pages.get(ctx.state.storyboard.current_page).map(|p| p.section.as_str());
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(3.5, 3.5);
        for (i, sec) in sections.iter().enumerate() {
            let short = match sec.as_str() {
                "Backgrounds" => "02 World",
                "Characters" => "03 Cast",
                "Scenes" => "04 Scenes",
                "Assets" => "05 Assets",
<<<<<<< HEAD
                "Final" => "06 Handoff",
=======
                "Final" => "06 Final",
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
                _ => "01 Cover",
            };
            let sel = cur_sec == Some(sec.as_str());
            let id = format!("sb-sec-jump-{}", i + 1);
<<<<<<< HEAD
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, short, &format!("Jump to {sec} section cover")).selected(sel)).clicked()
=======
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&id, short, &format!("Jump to {sec} section")).selected(sel)).clicked()
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
                && let Some(first_pg) = pages.iter().find(|p| &p.section == sec)
            {
                ctx.state.storyboard.current_page = first_pg.index;
                ctx.state.storyboard.scroll_to_page = Some(first_pg.index);
            }
        }
    });
}

fn draw_section_hierarchy_group(ui: &mut Ui, ctx: &mut ViewCtx<'_>, sec_num: usize, sec_name: &str, sec_pages: &[&PageSummary]) {
    let accent = ctx.pal.section_accent(sec_name);
    let filled_sum: usize = sec_pages.iter().map(|p| p.filled_slots).sum();
    let total_sum: usize = sec_pages.iter().map(|p| p.total_slots).sum();

<<<<<<< HEAD
    // Level 1: Section Banner
=======
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    ui.horizontal(|ui| {
        let (dot_r, _) = ui.allocate_exact_size(Vec2::new(8.0, 14.0), Sense::hover());
        ui.painter().circle_filled(dot_r.center(), 3.6, accent);
        ui.label(RichText::new(format!("{sec_num:02} · {}", sec_name.to_ascii_uppercase())).monospace().strong().size(10.0).color(accent));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new(format!("{filled_sum}/{total_sum}")).monospace().size(9.0).color(if total_sum > 0 && filled_sum == total_sum {
                ctx.pal.accent_emerald
            } else {
                ctx.pal.text_muted
            }));
        });
    });
    ui.add_space(3.0);

<<<<<<< HEAD
    // Level 2 (Cover / Section Title Card) & Level 3 (Indented Sub-Pages)
=======
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    let sub_total = sec_pages.iter().filter(|p| !p.is_cover_or_section_title()).count();
    let mut sub_seen = 0usize;
    for pg in sec_pages {
        if pg.is_cover_or_section_title() {
            draw_cover_title_rail_card(ui, ctx, pg, accent);
            ui.add_space(4.0);
        } else {
            sub_seen += 1;
            let is_last = sub_seen == sub_total;
            draw_indented_subpage_row(ui, ctx, pg, accent, is_last);
            ui.add_space(3.0);
        }
    }
}

fn draw_cover_title_rail_card(ui: &mut Ui, ctx: &mut ViewCtx<'_>, pg: &PageSummary, accent: Color32) {
    let selected = ctx.state.storyboard.current_page == pg.index;
    let card_w = (ui.available_width() - 4.0).max(168.0);
    let card_h = 66.0;
    let (crect, resp) = ui.allocate_exact_size(Vec2::new(card_w, card_h), Sense::click());
    ctx.reg.record(format!("sb-rail-p{:02}", pg.index + 1), format!("{} Cover ({})", pg.section, pg.title), &resp, None);

    let bg = if selected {
        accent.gamma_multiply(0.18)
    } else if resp.hovered() {
        ctx.pal.bg_hover
    } else {
        ctx.pal.bg_input
    };
    let border = if selected { Stroke::new(1.6, accent) } else { Stroke::new(1.0, ctx.pal.border) };
    ui.painter().rect(crect, CornerRadius::same(6), bg, border, StrokeKind::Inside);

    let thumb_h = 54.0;
<<<<<<< HEAD
    let thumb_w = (thumb_h / pg.aspect_h_over_w()).clamp(64.0, 98.0);
=======
    let thumb_w = (thumb_h / pg.aspect_h_over_w()).clamp(40.0, 98.0);
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    let thumb_r = Rect::from_min_size(Pos2::new(crect.min.x + 6.0, crect.min.y + 6.0), Vec2::new(thumb_w, thumb_h));
    paint_mini_page(ui, thumb_r, pg, selected, accent);

    let tx = thumb_r.max.x + 8.0;
<<<<<<< HEAD
    let role_tag = if pg.page_role == "cover" { format!("MASTER COVER · {}", pg.sub_code) } else { format!("SECTION COVER · {}", pg.sub_code) };
=======
    let role_tag = if pg.page_role == "cover" { format!("MASTER COVER · {}", pg.sub_code) } else { format!("SECTION START · {}", pg.sub_code) };
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    ui.painter().text(Pos2::new(tx, crect.min.y + 15.0), Align2::LEFT_CENTER, role_tag, FontId::monospace(8.5), accent);
    ui.painter().text(Pos2::new(tx, crect.min.y + 33.0), Align2::LEFT_CENTER, clamp_label(&pg.title, 22), FontId::proportional(11.0), ctx.pal.text_primary);
    ui.painter().text(
        Pos2::new(tx, crect.min.y + 50.0),
        Align2::LEFT_CENTER,
        format!("{}/{} fields ready", pg.filled_slots, pg.total_slots),
        FontId::monospace(8.5),
        if pg.light_green { ctx.pal.accent_emerald } else { ctx.pal.text_muted },
    );

    if resp.clicked() {
        ctx.state.storyboard.current_page = pg.index;
        ctx.state.storyboard.scroll_to_page = Some(pg.index);
    }
}

fn draw_indented_subpage_row(ui: &mut Ui, ctx: &mut ViewCtx<'_>, pg: &PageSummary, accent: Color32, is_last: bool) {
    let selected = ctx.state.storyboard.current_page == pg.index;
    let row_w = (ui.available_width() - 4.0).max(168.0);
    let row_h = 48.0;
    let (full_r, resp) = ui.allocate_exact_size(Vec2::new(row_w, row_h), Sense::click());
<<<<<<< HEAD
    ctx.reg.record(format!("sb-rail-p{:02}", pg.index + 1), format!("Sub-Page {} ({})", pg.sub_code, pg.title), &resp, None);

    // Tree guide connector (`├─` or `└─`)
=======
    ctx.reg.record(format!("sb-rail-p{:02}", pg.index + 1), format!("Page {} ({})", pg.sub_code, pg.title), &resp, None);

>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    let guide_x = full_r.min.x + 11.0;
    let guide_bottom = if is_last { full_r.center().y } else { full_r.max.y + 3.0 };
    let guide_col = if selected { accent } else { ctx.pal.border };
    ui.painter().line_segment([Pos2::new(guide_x, full_r.min.y - 2.0), Pos2::new(guide_x, guide_bottom)], Stroke::new(1.2, guide_col));
    ui.painter().line_segment([Pos2::new(guide_x, full_r.center().y), Pos2::new(guide_x + 8.0, full_r.center().y)], Stroke::new(1.2, guide_col));

    let card_r = Rect::from_min_max(Pos2::new(full_r.min.x + 21.0, full_r.min.y), full_r.max);
    let bg = if selected {
        accent.gamma_multiply(0.15)
    } else if resp.hovered() {
        ctx.pal.bg_hover
    } else {
        ctx.pal.bg_panel
    };
    let st = if selected { Stroke::new(1.4, accent) } else { Stroke::new(1.0, ctx.pal.border_subtle) };
    ui.painter().rect(card_r, CornerRadius::same(5), bg, st, StrokeKind::Inside);

    let th = 38.0;
<<<<<<< HEAD
    let tw = (th / pg.aspect_h_over_w()).clamp(50.0, 68.0);
=======
    let tw = (th / pg.aspect_h_over_w()).clamp(30.0, 68.0);
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    let thumb_r = Rect::from_min_size(Pos2::new(card_r.min.x + 5.0, card_r.min.y + 5.0), Vec2::new(tw, th));
    paint_mini_page(ui, thumb_r, pg, selected, accent);

    let tx = thumb_r.max.x + 7.0;
    ui.painter().text(
        Pos2::new(tx, card_r.min.y + 14.0),
        Align2::LEFT_CENTER,
        &pg.sub_code,
        FontId::monospace(9.0),
        if selected { accent } else { ctx.pal.text_secondary },
    );
    ui.painter().text(Pos2::new(tx, card_r.min.y + 31.0), Align2::LEFT_CENTER, clamp_label(&pg.title, 20), FontId::proportional(10.3), ctx.pal.text_primary);

    let dot_col = if pg.light_green { ctx.pal.accent_emerald } else { ctx.pal.accent_amber };
    ui.painter().circle_filled(Pos2::new(card_r.max.x - 9.0, card_r.min.y + 13.0), 3.0, dot_col);

    if resp.clicked() {
        ctx.state.storyboard.current_page = pg.index;
        ctx.state.storyboard.scroll_to_page = Some(pg.index);
    }
}

fn clamp_label(raw: &str, max_chars: usize) -> String {
    if raw.chars().count() <= max_chars {
        raw.to_owned()
    } else {
        let mut s: String = raw.chars().take(max_chars.saturating_sub(1)).collect();
        s.push('…');
        s
    }
}

fn draw_seamless_document_scroll(ui: &mut Ui, ctx: &mut ViewCtx<'_>, pages: &[PageSummary], dropped_paths: &[String]) {
    fill_card(ui, ctx.pal, true, |ui| {
        if let Some(cur_pg) = pages.get(ctx.state.storyboard.current_page.min(pages.len().saturating_sub(1))) {
            let sec_col = ctx.pal.section_accent(&cur_pg.section);
            let role_word = if cur_pg.page_role == "cover" {
                "Master Cover"
            } else if cur_pg.page_role == "section_title" {
<<<<<<< HEAD
                "Section Cover"
            } else {
                "Sub-Page"
            };
            let kicker = format!("{} › {} · {}", cur_pg.section.to_ascii_uppercase(), role_word.to_ascii_uppercase(), cur_pg.sub_code);
            let badge_txt = format!("Slide {}/{}", cur_pg.index + 1, pages.len());
=======
                "Section Start"
            } else {
                "Page"
            };
            let kicker = format!("{} › {} · {}", cur_pg.section.to_ascii_uppercase(), role_word.to_ascii_uppercase(), cur_pg.sub_code);
            let badge_txt = format!("Page {}/{}", cur_pg.index + 1, pages.len());
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
            draw_pane_kicker_header(ui, ctx.pal, &kicker, &cur_pg.title, sec_col, Some((&badge_txt, sec_col)));
        }
        ui.label(
            RichText::new(
<<<<<<< HEAD
                "Click any text box on the slide to edit inline · Drop, paste, or click any artwork frame to link PhotoCraft / VectorCraft / EffectCraft files",
=======
                "Click any unlocked text frame to edit inline · Drop, paste, or click any unlocked image frame to link PhotoCraft / VectorCraft / EffectCraft files",
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
            )
            .size(10.0)
            .color(ctx.pal.text_muted),
        );
        ui.add_space(5.0);

        let zoom = (ctx.state.storyboard.zoom as f32 / 100.0).clamp(0.65, 1.45);
        let target_scroll = ctx.state.storyboard.scroll_to_page.take();

        ScrollArea::vertical().id_salt("sb_seamless_doc_scroll").auto_shrink([false, false]).show(ui, |ui| {
            let avail_w = (ui.available_width() - 16.0).max(260.0);
            if ctx.state.storyboard.double_page_mode {
                draw_two_up_spread_scroll(ui, ctx, pages, dropped_paths, avail_w, zoom, target_scroll);
            } else {
<<<<<<< HEAD
                draw_single_slide_scroll(ui, ctx, pages, dropped_paths, avail_w, zoom, target_scroll);
=======
                draw_single_page_scroll(ui, ctx, pages, dropped_paths, avail_w, zoom, target_scroll);
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
            }
        });
    });
}

<<<<<<< HEAD
fn draw_single_slide_scroll(
=======
fn draw_single_page_scroll(
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    ui: &mut Ui,
    ctx: &mut ViewCtx<'_>,
    pages: &[PageSummary],
    dropped_paths: &[String],
    avail_w: f32,
    zoom: f32,
    target_scroll: Option<usize>,
) {
    let mut prev_sec = "";
    for pg in pages {
        if pg.section != prev_sec {
            prev_sec = pg.section.as_str();
            draw_inline_section_divider(ui, ctx, prev_sec);
        }
<<<<<<< HEAD
        let base_w = (avail_w * 0.94).clamp(280.0, 860.0) * zoom;
=======
        let max_natural_w = if pg.aspect_h_over_w() > 1.0 { 640.0 } else { 860.0 };
        let base_w = (avail_w * 0.88).clamp(260.0, max_natural_w) * zoom;
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
        let page_w = base_w.min(avail_w);
        let page_h = page_w * pg.aspect_h_over_w();
        let pad_x = ((avail_w - page_w) * 0.5).max(0.0);

        ui.horizontal(|ui| {
            if pad_x > 1.0 {
                ui.add_space(pad_x);
            }
            ui.vertical(|ui| {
<<<<<<< HEAD
                draw_slide_caption_strip(ui, ctx, pg, page_w);
=======
                draw_page_caption_strip(ui, ctx, pg, page_w);
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
                let r = draw_interactive_page(ui, ctx, pg, Vec2::new(page_w, page_h), dropped_paths);
                if target_scroll == Some(pg.index) {
                    ui.scroll_to_rect(r, Some(Align::TOP));
                }
            });
        });
        ui.add_space(16.0);
    }
}

fn draw_two_up_spread_scroll(
    ui: &mut Ui,
    ctx: &mut ViewCtx<'_>,
    pages: &[PageSummary],
    dropped_paths: &[String],
    avail_w: f32,
    zoom: f32,
    target_scroll: Option<usize>,
) {
    let gap = 12.0;
    let mut idx = 0usize;
    let mut prev_sec = "";
    while idx < pages.len() {
        if pages[idx].section != prev_sec {
            prev_sec = pages[idx].section.as_str();
            draw_inline_section_divider(ui, ctx, prev_sec);
        }
        let pair_end = if idx + 1 < pages.len() && pages[idx].section == pages[idx + 1].section { idx + 2 } else { idx + 1 };
        let slice = &pages[idx..pair_end];
<<<<<<< HEAD
        let base_w = ((avail_w - gap) * 0.5).clamp(220.0, 520.0) * zoom;
=======
        let base_w = ((avail_w - gap) * 0.5).clamp(200.0, 480.0) * zoom;
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
        let page_w = base_w.min((avail_w - gap) * 0.5);
        let row_w = slice.len() as f32 * page_w + (slice.len().saturating_sub(1) as f32) * gap;
        let pad_x = ((avail_w - row_w) * 0.5).max(0.0);

        ui.horizontal(|ui| {
            if pad_x > 1.0 {
                ui.add_space(pad_x);
            }
            ui.spacing_mut().item_spacing.x = gap;
            for pg in slice {
                let page_h = page_w * pg.aspect_h_over_w();
                ui.vertical(|ui| {
<<<<<<< HEAD
                    draw_slide_caption_strip(ui, ctx, pg, page_w);
=======
                    draw_page_caption_strip(ui, ctx, pg, page_w);
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
                    let r = draw_interactive_page(ui, ctx, pg, Vec2::new(page_w, page_h), dropped_paths);
                    if target_scroll == Some(pg.index) {
                        ui.scroll_to_rect(r, Some(Align::TOP));
                    }
                });
            }
        });
        ui.add_space(14.0);
        idx = pair_end;
    }
}

fn draw_inline_section_divider(ui: &mut Ui, ctx: &ViewCtx<'_>, sec_name: &str) {
    let col = ctx.pal.section_accent(sec_name);
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        let (pill_r, _) = ui.allocate_exact_size(Vec2::new(10.0, 14.0), Sense::hover());
        ui.painter().rect_filled(pill_r, CornerRadius::same(2), col);
        ui.label(RichText::new(format!("SECTION · {}", sec_name.to_ascii_uppercase())).monospace().strong().size(10.0).color(col));
    });
    ui.add_space(4.0);
}

<<<<<<< HEAD
fn draw_slide_caption_strip(ui: &mut Ui, ctx: &ViewCtx<'_>, pg: &PageSummary, width: f32) {
    let accent = ctx.pal.section_accent(&pg.section);
    let kind_lbl = if pg.is_cover_or_section_title() { "COVER / TITLE" } else { "SUB-PAGE" };
=======
fn draw_page_caption_strip(ui: &mut Ui, ctx: &ViewCtx<'_>, pg: &PageSummary, width: f32) {
    let accent = ctx.pal.section_accent(&pg.section);
    let kind_lbl = if pg.is_cover_or_section_title() { "COVER / SECTION" } else { "PAGE" };
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    ui.allocate_ui_with_layout(Vec2::new(width, 18.0), Layout::left_to_right(Align::Center), |ui| {
        ui.label(RichText::new(format!("{} · {kind_lbl}", pg.sub_code)).monospace().size(9.0).color(accent));
        ui.label(RichText::new("·").color(ctx.pal.text_muted));
        ui.label(RichText::new(clamp_label(&pg.title, 48)).size(10.2).color(ctx.pal.text_secondary));
    });
}

fn draw_requirements_navigator(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    fill_card(ui, ctx.pal, true, |ui| {
<<<<<<< HEAD
        draw_pane_kicker_header(ui, ctx.pal, "FIELD CHECKLIST", "Barnyard Fields", ctx.pal.accent_emerald, None);
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            for (fid, lbl, tip) in
                [("all", "All", "Show all Barnyard Storyboard fields"), ("red", "Missing", "Show unfilled fields"), ("green", "Ready", "Show completed fields")]
=======
        draw_pane_kicker_header(ui, ctx.pal, "FIELD CHECKLIST", "Storyboard Fields", ctx.pal.accent_emerald, None);
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            for (fid, lbl, tip) in
                [("all", "All", "Show all unlocked Storyboard fields"), ("red", "Missing", "Show unfilled fields"), ("green", "Ready", "Show completed fields")]
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
            {
                let sel = ctx.state.storyboard.req_filter == fid;
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&format!("sb-req-filter-{fid}"), lbl, tip).selected(sel)).clicked() {
                    ctx.state.storyboard.req_filter = fid.into();
                }
            }
        });
        ui.add_space(4.0);

        let sections = ctx.state.cache.storyboard_check["sections"].as_array().cloned().unwrap_or_default();
        ScrollArea::vertical().id_salt("sb_reqs_nav_scroll").auto_shrink([false, false]).show(ui, |ui| {
            for sec in &sections {
                draw_section_requirement_block(ui, ctx, sec);
            }
        });
    });
}

fn draw_section_requirement_block(ui: &mut Ui, ctx: &mut ViewCtx<'_>, sec: &Value) {
    let sec_name = sec["section"].as_str().unwrap_or("Section");
    let filled_n = sec["filled_count"].as_u64().unwrap_or(0);
    let total_n = sec["total_required"].as_u64().unwrap_or(0);
    let reqs = sec["requirements"].as_array().cloned().unwrap_or_default();
    let filtered: Vec<&Value> = reqs
        .iter()
        .filter(|r| match ctx.state.storyboard.req_filter.as_str() {
            "red" => r["filled"] != true,
            "green" => r["filled"] == true,
            _ => true,
        })
        .collect();
    if filtered.is_empty() {
        return;
    }

    let accent = ctx.pal.section_accent(sec_name);
    ui.horizontal(|ui| {
        ui.label(RichText::new(sec_name.to_ascii_uppercase()).monospace().strong().size(9.6).color(accent));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new(format!("{filled_n}/{total_n}")).monospace().size(9.2).color(if filled_n == total_n {
                ctx.pal.accent_emerald
            } else {
                ctx.pal.text_muted
            }));
        });
    });
    ui.add_space(2.0);

    for req in filtered {
        let page_idx = req["page"].as_u64().unwrap_or(0) as usize;
        let label = req["label"].as_str().unwrap_or("field");
        let title = req["title"].as_str().unwrap_or(label);
        let kind = req["kind"].as_str().unwrap_or("text");
        let filled = req["filled"].as_bool().unwrap_or(false);
        let selected = ctx.state.storyboard.current_page == page_idx && ctx.state.storyboard.editing_slot.as_deref() == Some(label);
        let req_id = format!("sb-req-p{page_idx:02}-{label}");
        let row_title = format!("P{:02} · {title}", page_idx + 1);
        let detail = if kind == "image" { "IMG" } else { "TXT" };

        let resp = requirement_light_row(ui, ctx.reg, ctx.pal, RequirementRow { id: &req_id, title: &row_title, detail, filled, selected });
        if resp.clicked() {
            ctx.state.storyboard.current_page = page_idx;
            ctx.state.storyboard.scroll_to_page = Some(page_idx);
            ctx.state.storyboard.editing_page = Some(page_idx);
            ctx.state.storyboard.editing_slot = Some(label.to_owned());
            if kind == "text" {
                let raw = req["text"].as_str().unwrap_or("");
                ctx.state.storyboard.draft_text = if raw.contains('[') && raw.contains(']') { String::new() } else { raw.to_owned() };
                ctx.state.storyboard.focus_requested = true;
            }
        }
        if let Some(cli) = req["cli_command"].as_str() {
            resp.on_hover_ui(|ui| {
                ui.painter().rect_filled(ui.max_rect(), CornerRadius::same(3), ctx.pal.bg_panel);
                ui.label(RichText::new(cli).monospace().size(10.0).color(ctx.pal.text_primary));
            });
        }
        ui.add_space(2.0);
    }
    ui.add_space(6.0);
}
