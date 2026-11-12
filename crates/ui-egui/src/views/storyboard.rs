//! `Storyboard` view (`#view-storyboard`): 3-pane `PDFCraft` (`storyboard.pdf`) viewer & inline
//! form filler backed by the strictly read-only `Magic Studio Storyboard.designcraft` master template:
//! 1. Left (`Storyboard Sections · 2-Up`): 6 sections (`Cover`, `Backgrounds`, `Characters`,
//!    `Scenes`, `Assets`, `Final`) with side-by-side 2-up miniature page pairs.
//! 2. Center (`Seamless Storyboard — storyboard.pdf`): single continuous vertical scroll across all
//!    17 pages (`Single Page` or `Double Page` side-by-side), where clicking directly on any unlocked
//!    text field edits inline on the page and dropping/clicking an image slot places a still image.
//! 3. Right (`Required Fields`): clickable Red/Green requirement links synced 1:1 with
//!    `magic-cli storyboard check` — zero duplicate form inputs.

use craft_ui::layout::{SplitAxis, SplitSize, split_rect};
use egui::{Align, Color32, CornerRadius, Layout, RichText, ScrollArea, Sense, Ui, Vec2};
use serde_json::{Value, json};

use super::storyboard_page::{PAGE_H_PT, PAGE_W_PT, PageSummary, draw_interactive_page, paint_mini_page};
use crate::controls::{RequirementRow, ScalarSpec, child_pane, fill_card, paint_craft_doc_card, requirement_light_row, ring_slider_h};
use crate::state::ViewCtx;
use crate::widgets::{BtnSpec, action_button, status_pill};

// TODO(MAGIC-REVIEW): CHANGE asset handoff to preserve the owning app document; dropped-file API correction alone does not verify it.
pub fn show(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let pages = collect_pages(ctx);
    draw_toolbar(ui, ctx, &pages);
    ui.add_space(6.0);

    if pages.is_empty() {
        draw_empty_storyboard_state(ui, ctx);
        return;
    }

    let dropped_paths: Vec<String> = ui.ctx().input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_string_lossy().into_owned()).collect());

    let body = ui.available_rect_before_wrap();
    let left_w = if body.width() >= 1220.0 { 286.0 } else { 244.0 };
    let right_w = if body.width() >= 1220.0 { 304.0 } else { 262.0 };

    let after_left = if ctx.state.show_nav_rail
        && let Some(left_split) = split_rect(body, SplitAxis::Horizontal, SplitSize::FixedFirst(left_w), 8.0, 220.0)
    {
        let mut left = child_pane(ui, left_split.first, "sb_left_sections_2up");
        draw_sections_two_up_rail(&mut left, ctx, &pages);
        left_split.second
    } else {
        body
    };

    if ctx.state.show_right_inspector
        && let Some(right_split) = split_rect(after_left, SplitAxis::Horizontal, SplitSize::FixedSecond(right_w), 8.0, 220.0)
    {
        let mut center = child_pane(ui, right_split.first, "sb_center_seamless");
        draw_seamless_document_scroll(&mut center, ctx, &pages, &dropped_paths);
        let mut right = child_pane(ui, right_split.second, "sb_right_requirements");
        draw_requirements_navigator(&mut right, ctx);
    } else {
        let mut center = child_pane(ui, after_left, "sb_center_seamless");
        draw_seamless_document_scroll(&mut center, ctx, &pages, &dropped_paths);
    }
    let _ = ui.allocate_rect(body, Sense::hover());
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
            let light_green = chk.and_then(|c| c["light"].as_str()).or_else(|| pg["light"].as_str()).is_some_and(|l| l == "green");
            PageSummary {
                index,
                section: pg["section"].as_str().unwrap_or("Scenes").to_owned(),
                title: pg["title"].as_str().unwrap_or("Page").to_owned(),
                light_green,
                preview_png: pg["preview_png"].as_str().map(str::to_owned),
                slots: pg["slots"].as_array().cloned().unwrap_or_default(),
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
            "PDF",
            "storyboard.pdf",
            "Click Initialize storyboard.pdf to create the project storyboard from the read-only Master Template.",
        );
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("storyboard-empty-init", "plus", "Initialize storyboard.pdf", "Create storyboard.pdf from read-only Master Template").primary(),
            )
            .clicked()
            {
                ctx.exec("storyboard.new", json!({}), Some("Initialized storyboard.pdf"));
                ctx.exec("storyboard.pages", json!({"full": true}), None);
                ctx.exec("storyboard.check", json!({}), None);
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("storyboard-empty-reload", "refresh", "Reload", "Inspect storyboard.pdf")).clicked() {
                ctx.exec("storyboard.pages", json!({"full": true}), None);
                ctx.exec("storyboard.check", json!({}), None);
            }
        });
    });
}

fn draw_toolbar(ui: &mut Ui, ctx: &mut ViewCtx<'_>, pages: &[PageSummary]) {
    let filled = ctx.state.cache.storyboard_check["filled_count"].as_u64().unwrap_or(0);
    let total = ctx.state.cache.storyboard_check["total_required"].as_u64().unwrap_or(0);
    let all_green = total > 0 && filled == total;
    let pill_col = if all_green { ctx.pal.accent_emerald } else { ctx.pal.danger };

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        status_pill(ui, ctx.pal, &format!("{filled}/{total} Ready · storyboard.pdf (Master Read-Only)"), pill_col);

        let single_sel = !ctx.state.storyboard.double_page_mode;
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::label_only("sb-mode-single", "1-Page Scroll", "Continuous single-page vertical scroll").selected(single_sel),
        )
        .clicked()
        {
            ctx.state.storyboard.double_page_mode = false;
        }
        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::label_only("sb-mode-double", "2-Page Spreads", "Continuous side-by-side double-page scroll").selected(!single_sel),
        )
        .clicked()
        {
            ctx.state.storyboard.double_page_mode = true;
        }

        ui.allocate_ui_with_layout(Vec2::new(150.0, 24.0), Layout::left_to_right(Align::Center), |ui| {
            ring_slider_h(ui, ctx.reg, ctx.pal, ScalarSpec::new("sb-zoom", "Zoom", 60.0..=150.0, "%", ctx.pal.accent_emerald), &mut ctx.state.storyboard.zoom);
        });

        if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("storyboard-refresh", "refresh", "Sync", "Refresh storyboard.pdf pages and Red/Green check"))
            .clicked()
        {
            ctx.exec("storyboard.pages", json!({"full": true}), None);
            ctx.exec("storyboard.check", json!({}), None);
        }

        if action_button(
            ui,
            ctx.reg,
            ctx.pal,
            BtnSpec::new("storyboard-open-pdfcraft", "external", "Open in PDFCraft", "Open storyboard.pdf in PDFCraft (--control)").primary(),
        )
        .clicked()
        {
            let pg = ctx.state.storyboard.current_page.min(pages.len().saturating_sub(1));
            ctx.exec("storyboard.open", json!({"page": pg}), Some("Opened storyboard.pdf in PDFCraft"));
        }
    });
}

fn draw_sections_two_up_rail(ui: &mut Ui, ctx: &mut ViewCtx<'_>, pages: &[PageSummary]) {
    fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Storyboard Sections").strong().size(12.0).color(ctx.pal.text_primary));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new("2-UP").monospace().size(9.5).color(ctx.pal.text_muted));
            });
        });
        ui.add_space(4.0);

        let sec_checks = ctx.state.cache.storyboard_check["sections"].as_array();
        ScrollArea::vertical().id_salt("sb_sections_2up_scroll").auto_shrink([false, false]).show(ui, |ui| {
            let avail_w = (ui.available_width() - 6.0).max(160.0);
            let gap = 6.0;
            let thumb_w = ((avail_w - gap) * 0.5).floor().max(72.0);
            let thumb_h = (thumb_w * (PAGE_H_PT / PAGE_W_PT)).round();

            let dynamic_sections: Vec<String> = ctx.state.cache.storyboard_pages["sections"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect())
                .filter(|v: &Vec<String>| !v.is_empty())
                .unwrap_or_else(|| {
                    let mut uniq = Vec::new();
                    for pg in pages {
                        if !uniq.contains(&pg.section) {
                            uniq.push(pg.section.clone());
                        }
                    }
                    uniq
                });
            for sec_name_owned in &dynamic_sections {
                let sec_name = sec_name_owned.as_str();
                let sec_pages: Vec<&PageSummary> = pages.iter().filter(|p| p.section == sec_name).collect();
                if sec_pages.is_empty() {
                    continue;
                }
                let sec_green = sec_checks
                    .and_then(|arr| arr.iter().find(|s| s["section"].as_str() == Some(sec_name)))
                    .is_some_and(|s| s["light"].as_str() == Some("green"));
                let dot_col = if sec_green { ctx.pal.accent_emerald } else { ctx.pal.danger };

                ui.horizontal(|ui| {
                    let (dot_r, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                    ui.painter().circle_filled(dot_r.center(), 3.6, dot_col);
                    ui.label(RichText::new(sec_name.to_ascii_uppercase()).strong().size(10.0).color(ctx.pal.text_secondary));
                });
                ui.add_space(2.0);

                for pair in sec_pages.chunks(2) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = gap;
                        for pg in pair {
                            ui.vertical(|ui| {
                                let (r, resp) = ui.allocate_exact_size(Vec2::new(thumb_w, thumb_h), Sense::click());
                                ctx.reg.record(&format!("sb-rail-p{:02}", pg.index + 1), &format!("{sec_name} P{:02}", pg.index + 1), &resp, None);
                                let selected = ctx.state.storyboard.current_page == pg.index;
                                paint_mini_page(ui, r, pg, selected, ctx.pal.accent_emerald);
                                if resp.clicked() {
                                    ctx.state.storyboard.current_page = pg.index;
                                    ctx.state.storyboard.scroll_to_page = Some(pg.index);
                                }
                                ui.horizontal(|ui| {
                                    let p_col = if pg.light_green { ctx.pal.accent_emerald } else { ctx.pal.danger };
                                    let (dr, _) = ui.allocate_exact_size(Vec2::splat(6.0), Sense::hover());
                                    ui.painter().circle_filled(dr.center(), 2.5, p_col);
                                    ui.label(RichText::new(format!("P{:02}", pg.index + 1)).monospace().size(9.0).color(if selected {
                                        ctx.pal.text_primary
                                    } else {
                                        ctx.pal.text_muted
                                    }));
                                });
                            });
                        }
                    });
                    ui.add_space(6.0);
                }
                ui.add_space(4.0);
            }
        });
    });
}

fn draw_seamless_document_scroll(ui: &mut Ui, ctx: &mut ViewCtx<'_>, pages: &[PageSummary], dropped_paths: &[String]) {
    fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(
                    "Click any text field to edit · Drop, paste, or click any slot to place artwork · Right-click a filled slot to open its linked Craft file",
                )
                .size(10.5)
                .color(ctx.pal.text_muted),
            );
        });
        ui.add_space(4.0);

        let zoom = (ctx.state.storyboard.zoom as f32 / 100.0).clamp(0.6, 1.5);
        let target_scroll = ctx.state.storyboard.scroll_to_page.take();

        ScrollArea::vertical().id_salt("sb_seamless_doc_scroll").auto_shrink([false, false]).show(ui, |ui| {
            let avail_w = (ui.available_width() - 18.0).max(260.0);
            if ctx.state.storyboard.double_page_mode {
                let gap = 12.0;
                let base_w = ((avail_w - gap) * 0.5).clamp(210.0, 520.0) * zoom;
                let page_w = base_w.min((avail_w - gap) * 0.5);
                let page_h = page_w * (PAGE_H_PT / PAGE_W_PT);
                let page_size = Vec2::new(page_w, page_h);

                let mut idx = 0usize;
                while idx < pages.len() {
                    let pair_end = if pages[idx].section == "Cover" || idx + 1 < pages.len() && pages[idx].section == pages[idx + 1].section {
                        (idx + 2).min(pages.len())
                    } else {
                        idx + 1
                    };
                    let slice = &pages[idx..pair_end];
                    let row_w = slice.len() as f32 * page_w + (slice.len().saturating_sub(1) as f32) * gap;
                    let pad_x = ((avail_w - row_w) * 0.5).max(0.0);
                    ui.horizontal(|ui| {
                        if pad_x > 1.0 {
                            ui.add_space(pad_x);
                        }
                        ui.spacing_mut().item_spacing.x = gap;
                        for pg in slice {
                            let r = draw_interactive_page(ui, ctx, pg, page_size, dropped_paths);
                            if target_scroll == Some(pg.index) {
                                ui.scroll_to_rect(r, Some(Align::TOP));
                            }
                        }
                    });
                    ui.add_space(16.0);
                    idx = pair_end;
                }
            } else {
                let page_w = (avail_w * 0.86).clamp(260.0, 640.0) * zoom;
                let page_w = page_w.min(avail_w);
                let page_h = page_w * (PAGE_H_PT / PAGE_W_PT);
                let page_size = Vec2::new(page_w, page_h);
                let pad_x = ((avail_w - page_w) * 0.5).max(0.0);

                for pg in pages {
                    ui.horizontal(|ui| {
                        if pad_x > 1.0 {
                            ui.add_space(pad_x);
                        }
                        let r = draw_interactive_page(ui, ctx, pg, page_size, dropped_paths);
                        if target_scroll == Some(pg.index) {
                            ui.scroll_to_rect(r, Some(Align::TOP));
                        }
                    });
                    ui.add_space(16.0);
                }
            }
        });
    });
}

fn draw_requirements_navigator(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Required Fields").strong().size(12.0).color(ctx.pal.text_primary));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                for (fid, lbl, tip) in
                    [("green", "Green", "Show completed fields"), ("red", "Missing", "Show missing red fields"), ("all", "All", "Show all required fields")]
                {
                    let sel = ctx.state.storyboard.req_filter == fid;
                    if action_button(ui, ctx.reg, ctx.pal, BtnSpec::label_only(&format!("sb-req-filter-{fid}"), lbl, tip).selected(sel)).clicked() {
                        ctx.state.storyboard.req_filter = fid.into();
                    }
                }
            });
        });
        ui.add_space(4.0);

        let sections = ctx.state.cache.storyboard_check["sections"].as_array().cloned().unwrap_or_default();
        ScrollArea::vertical().id_salt("sb_reqs_nav_scroll").auto_shrink([false, false]).show(ui, |ui| {
            for sec in &sections {
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
                    continue;
                }

                ui.horizontal(|ui| {
                    ui.label(RichText::new(sec_name.to_ascii_uppercase()).strong().size(10.0).color(ctx.pal.text_secondary));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new(format!("{filled_n}/{total_n}")).monospace().size(9.5).color(if filled_n == total_n {
                            Color32::from_rgb(72, 199, 142)
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
        });
    });
}
