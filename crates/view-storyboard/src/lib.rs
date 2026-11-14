//! Read-only standalone `01 Storyboard` view backed exclusively by `DesignCraft`
//! (`storyboard.designcraft` + `designcraft-cli` PNG renders). Zero coupling to `05 Studio`.

#![forbid(unsafe_code)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable))]

use egui::{CornerRadius, FontId, Layout, Pos2, Rect, RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui, Vec2};
use magic_view_api::controls::{cached_image_aspect, fill_card, paint_local_image};
use magic_view_api::editorial::draw_pane_kicker_header;
use magic_view_api::widgets::{BtnSpec, action_button, status_pill};
use magic_view_api::{PanelId, StageView, ViewCtx, ViewMode, disabled_todo_button};
use serde_json::{Value, json};

const DEFAULT_PAGE_W_PT: f32 = 612.0;
const DEFAULT_PAGE_H_PT: f32 = 792.0;

#[derive(Default)]
pub struct StoryboardView {
    selected_page: usize,
}

impl StoryboardView {
    pub fn new() -> Self {
        Self::default()
    }

    fn pages<'a>(&self, ctx: &'a ViewCtx<'_>) -> &'a [Value] {
        ctx.state.cache.storyboard_pages["pages"].as_array().map_or(&[][..], Vec::as_slice)
    }

    fn current_page<'a>(&mut self, ctx: &'a ViewCtx<'_>) -> Option<&'a Value> {
        let pages = self.pages(ctx);
        if pages.is_empty() {
            self.selected_page = 0;
            return None;
        }
        if self.selected_page >= pages.len() {
            self.selected_page = pages.len() - 1;
        }
        pages.get(self.selected_page)
    }

    fn draw_navigator(&mut self, ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
        draw_pane_kicker_header(ui, ctx.pal, "DESIGNCRAFT", "Pages (Read-Only)", ctx.pal.accent_emerald, None);
        ui.add_space(6.0);

        let pages: Vec<Value> = self.pages(ctx).to_vec();
        if pages.is_empty() {
            fill_card(ui, ctx.pal, false, |ui| {
                ui.label(RichText::new("No storyboard.designcraft in this project yet.").size(11.0).color(ctx.pal.text_secondary));
                ui.add_space(4.0);
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("sb-nav-new", "plus", "New Storyboard", "Create storyboard.designcraft").primary())
                    .clicked()
                {
                    ctx.exec("storyboard.new", json!({"overwrite": false}), Some("Created storyboard.designcraft"));
                }
            });
            return;
        }

        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            for (idx, pg) in pages.iter().enumerate() {
                self.draw_page_list_item(ui, ctx, idx, pg);
                ui.add_space(4.0);
            }
        });
    }

    fn draw_page_list_item(&mut self, ui: &mut Ui, ctx: &mut ViewCtx<'_>, idx: usize, pg: &Value) {
        let sel = idx == self.selected_page;
        let title = pg["title"].as_str().unwrap_or("Page");
        let png = pg["render_png"].as_str();
        let avail_w = ui.available_width().max(120.0);
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(avail_w, 50.0), Sense::click());
        let fill = if sel { ctx.pal.accent_emerald_bg } else { ctx.pal.bg_input };
        let stroke = Stroke::new(if sel { 1.5 } else { 1.0 }, if sel { ctx.pal.accent_emerald } else { ctx.pal.border });
        ui.painter().rect(rect, CornerRadius::same(7), fill, stroke, StrokeKind::Inside);

        let thumb_r = Rect::from_min_size(Pos2::new(rect.min.x + 6.0, rect.min.y + 5.0), Vec2::new(31.0, 40.0));
        if !paint_local_image(ui, thumb_r, png, ctx.pal.bg_stage, 4) {
            ui.painter().text(thumb_r.center(), egui::Align2::CENTER_CENTER, "PNG", FontId::monospace(8.5), ctx.pal.text_muted);
        }

        let tx = thumb_r.max.x + 8.0;
        ui.painter().text(Pos2::new(tx, rect.min.y + 17.0), egui::Align2::LEFT_CENTER, title, FontId::proportional(11.5), ctx.pal.text_primary);
        let sub = if png.is_some() { "PNG rendered" } else { "Not rendered" };
        ui.painter().text(
            Pos2::new(tx, rect.min.y + 34.0),
            egui::Align2::LEFT_CENTER,
            sub,
            FontId::monospace(9.3),
            if sel { ctx.pal.accent_emerald } else { ctx.pal.text_muted },
        );
        if resp.clicked() {
            self.selected_page = idx;
        }
        ctx.reg.record(format!("sb-page-{idx}"), title, &resp, Some(sel.to_string()));
    }

    fn draw_canvas(&mut self, ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
        let exists = ctx.state.cache.storyboard_pages["exists"].as_bool() == Some(true);
        if !exists {
            fill_card(ui, ctx.pal, true, |ui| {
                ui.add_space(16.0);
                ui.heading(RichText::new("Standalone DesignCraft Storyboard (Read-Only)").color(ctx.pal.text_primary));
                ui.label(
                    RichText::new("Independent .designcraft layout viewed via designcraft-cli renders. Edit in DesignCraft; not wired into 05 Studio.")
                        .size(11.5)
                        .color(ctx.pal.text_secondary),
                );
                ui.add_space(10.0);
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("sb-canvas-new", "plus", "New Storyboard", "Create storyboard.designcraft").primary())
                    .clicked()
                {
                    ctx.exec("storyboard.new", json!({"overwrite": false}), Some("Created storyboard.designcraft"));
                }
            });
            return;
        }

        let Some(pg) = self.current_page(ctx).cloned() else {
            return;
        };
        let page_idx = pg["page"].as_u64().unwrap_or(0) as usize;
        let pg_w = (pg["width"].as_f64().unwrap_or(f64::from(DEFAULT_PAGE_W_PT)) as f32).max(1.0);
        let pg_h = (pg["height"].as_f64().unwrap_or(f64::from(DEFAULT_PAGE_H_PT)) as f32).max(1.0);
        let render_png = pg["render_png"].as_str().map(str::to_owned);

        fill_card(ui, ctx.pal, true, |ui| {
            let avail = ui.available_size();
            let rect = ui.allocate_exact_size(Vec2::new(avail.x.max(120.0), avail.y.max(120.0)), Sense::hover()).0;
            let aspect = render_png.as_deref().and_then(cached_image_aspect).unwrap_or(pg_w / pg_h);
            let fit = fit_rect_aspect(rect.shrink(6.0), aspect);

            if let Some(png_path) = render_png.as_deref() {
                let _ = paint_local_image(ui, fit, Some(png_path), ctx.pal.bg_stage, 6);
            } else {
                ui.painter().rect(fit, CornerRadius::same(8), ctx.pal.bg_input, Stroke::new(1.0, ctx.pal.border), StrokeKind::Inside);
                let btn_r = Rect::from_center_size(fit.center(), Vec2::new(170.0, 32.0));
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(btn_r).layout(Layout::centered_and_justified(egui::Direction::LeftToRight)));
                if action_button(&mut child, ctx.reg, ctx.pal, BtnSpec::new("sb-canvas-render", "image", "Render", "Render page PNG").primary()).clicked() {
                    ctx.exec("storyboard.render", json!({"page": page_idx}), Some(&format!("Rendered Page {}", page_idx + 1)));
                }
            }
        });
    }

    fn draw_inspector(&mut self, ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
        draw_pane_kicker_header(ui, ctx.pal, "DESIGNCRAFT", "Read-Only Inspector", ctx.pal.accent_sky, None);
        ui.add_space(6.0);
        fill_card(ui, ctx.pal, false, |ui| {
            ui.label(
                RichText::new("01 Storyboard is read-only inside Magic Studio. Edit storyboard.designcraft directly in DesignCraft or via designcraft-cli.")
                    .size(11.0)
                    .color(ctx.pal.text_secondary),
            );
            ui.add_space(6.0);
            if let Some(path) = ctx.state.cache.storyboard_pages["path"].as_str() {
                ui.label(RichText::new(path).monospace().size(9.5).color(ctx.pal.text_muted));
            }
        });
        ui.add_space(8.0);
        // TODO: Read and find the docs
        let _ = disabled_todo_button(ui, ctx, "sb-todo-edit", "edit", "Inline Field Edit", "Edit storyboard.designcraft directly in DesignCraft");
    }
}

fn fit_rect_aspect(bounds: Rect, aspect: f32) -> Rect {
    let safe_aspect = aspect.max(0.1);
    let avail_aspect = (bounds.width() / bounds.height().max(1.0)).max(0.1);
    let size = if avail_aspect > safe_aspect {
        Vec2::new(bounds.height() * safe_aspect, bounds.height())
    } else {
        Vec2::new(bounds.width(), bounds.width() / safe_aspect)
    };
    Rect::from_center_size(bounds.center(), size)
}

impl StageView for StoryboardView {
    fn mode(&self) -> ViewMode {
        ViewMode::Storyboard
    }

    fn has_timeline(&self) -> bool {
        false
    }

    fn uses_filmcraft_audio(&self) -> bool {
        false
    }

    fn draw_header(&mut self, ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
        let doc_exists = ctx.state.cache.storyboard_pages["exists"].as_bool() == Some(true);
        let pg_count = self.pages(ctx).len();
        let cur_png = self.pages(ctx).get(self.selected_page).and_then(|pg| pg["render_png"].as_str()).map(str::to_owned);

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            let status_txt = if doc_exists { format!("DesignCraft (Read-Only) · {pg_count} pg") } else { "Standalone · Optional".into() };
            let status_col = if doc_exists { ctx.pal.accent_emerald } else { ctx.pal.text_muted };
            status_pill(ui, ctx.pal, &status_txt, status_col);

            if !doc_exists {
                if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("sb-hdr-new", "plus", "New Storyboard", "Create storyboard.designcraft").primary())
                    .clicked()
                {
                    ctx.exec("storyboard.new", json!({"overwrite": false}), Some("Created storyboard.designcraft"));
                }
                return;
            }

            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("sb-hdr-open-dc", "external", "Open in DesignCraft", "Open storyboard.designcraft in DesignCraft"),
            )
            .clicked()
            {
                ctx.exec("storyboard.open", json!({}), Some("Opening storyboard.designcraft in DesignCraft"));
            }
            if action_button(ui, ctx.reg, ctx.pal, BtnSpec::new("sb-hdr-render", "image", "Render", "Render selected page via designcraft-cli").primary())
                .clicked()
            {
                ctx.exec("storyboard.render", json!({"page": self.selected_page}), Some(&format!("Rendered Page {}", self.selected_page + 1)));
            }
            if action_button(
                ui,
                ctx.reg,
                ctx.pal,
                BtnSpec::new("sb-hdr-copy-png", "copy", "Copy PNG Path", "Copy rendered page PNG path for handoff").enabled(cur_png.is_some()),
            )
            .clicked()
                && let Some(ref png) = cur_png
            {
                ui.ctx().copy_text(png.clone());
                let now = ui.input(|i| i.time);
                ctx.state.set_toast(now, format!("Copied PNG path: {png}"));
            }
        });
    }

    fn draw_panel(&mut self, ui: &mut Ui, ctx: &mut ViewCtx<'_>, panel: PanelId) {
        match panel {
            PanelId::Navigator => self.draw_navigator(ui, ctx),
            PanelId::Canvas => self.draw_canvas(ui, ctx),
            PanelId::Inspector => self.draw_inspector(ui, ctx),
            PanelId::Timeline => {
                ui.label(RichText::new("01 Storyboard is a standalone read-only DesignCraft viewer.").size(11.0).color(ctx.pal.text_muted));
            }
            _ => {}
        }
    }

    fn reset_for_project(&mut self) {
        self.selected_page = 0;
    }
}
