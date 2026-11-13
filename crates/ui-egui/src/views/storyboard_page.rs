//! Interactive `612 × 792 pt` DesignCraft (`storyboard.designcraft`)
//! Storyboard page renderer with anti-slop word-wrapped card typography, click-in-place text
//! editing, drag-and-drop / Wayland paste / click artwork placement, and right-click handoff
//! to `PhotoCraft`, `VectorCraft`, `EffectCraft`, or `FilmCraft`.

use egui::{Align2, Color32, CornerRadius, Event, FontFamily, FontId, Key, Pos2, Rect, Sense, Stroke, StrokeKind, TextEdit, Ui, Vec2};
use serde_json::{Value, json};

use crate::controls::paint_local_image;
use crate::editorial::{WrappedTextSpec, paint_wrapped_editorial_text};
use crate::state::{PickerTarget, UiAction, ViewCtx};
use crate::theme::MagicPalette;

pub const DEFAULT_PAGE_W_PT: f32 = 612.0;
pub const DEFAULT_PAGE_H_PT: f32 = 792.0;

const SLOT_PICKER_EXTS: &[&str] = &[
    "png",
    "jpg",
    "jpeg",
    "webp",
    "pcraft",
    "psd",
    "psb",
    "ora",
    "xcf",
    "kra",
    "pdn",
    "vectorcraft",
    "svg",
    "svgz",
    "ai",
    "eps",
    "af",
    "afdesign",
    "afphoto",
    "ecproj",
];

pub struct PageSummary {
    pub index: usize,
    pub section: String,
    pub page_role: String,
    pub sub_code: String,
    pub title: String,
    pub page_size_pt: [f32; 2],
    pub light_green: bool,
    pub filled_slots: usize,
    pub total_slots: usize,
    pub preview_png: Option<String>,
    pub slots: Vec<Value>,
    pub layout_items: Vec<Value>,
}

impl PageSummary {
    pub fn aspect_h_over_w(&self) -> f32 {
        let w = self.page_size_pt[0].max(100.0);
        let h = self.page_size_pt[1].max(100.0);
        h / w
    }

    pub fn is_cover_or_section_title(&self) -> bool {
        matches!(self.page_role.as_str(), "cover" | "section_title")
    }
}

struct SlotRenderEnv<'a> {
    page_rect: Rect,
    scale: f32,
    has_raster_bg: bool,
    dropped_paths: &'a [String],
    pointer_pos: Option<Pos2>,
    pasted: Option<String>,
}

fn local_rect(page_rect: Rect, page_pt: [f32; 2], bounds: &[Value]) -> Option<Rect> {
    if bounds.len() < 4 {
        return None;
    }
    let sx = page_rect.width() / page_pt[0].max(1.0);
    let sy = page_rect.height() / page_pt[1].max(1.0);
    let x0 = page_rect.min.x + (bounds[0].as_f64()? as f32) * sx;
    let y0 = page_rect.min.y + (bounds[1].as_f64()? as f32) * sy;
    let x1 = page_rect.min.x + (bounds[2].as_f64()? as f32) * sx;
    let y1 = page_rect.min.y + (bounds[3].as_f64()? as f32) * sy;
    Some(Rect::from_min_max(
        Pos2::new(x0.clamp(page_rect.min.x, page_rect.max.x), y0.clamp(page_rect.min.y, page_rect.max.y)),
        Pos2::new(x1.clamp(page_rect.min.x, page_rect.max.x), y1.clamp(page_rect.min.y, page_rect.max.y)),
    ))
}

fn pasted_file_path(ui: &Ui) -> Option<String> {
    ui.input(|i| {
        i.events.iter().find_map(|ev| match ev {
            Event::Paste(raw) => {
                let first = raw.lines().map(str::trim).find(|l| !l.is_empty() && !l.starts_with('#'))?;
                let cleaned = first.strip_prefix("file://").unwrap_or(first);
                std::path::Path::new(cleaned).exists().then(|| cleaned.to_owned())
            }
            _ => None,
        })
    })
}

/// Draw a miniature Barnyard slide / page preview inside the hierarchical left scroller.
pub fn paint_mini_page(ui: &mut Ui, rect: Rect, page: &PageSummary, selected: bool, accent: Color32) {
    let paper = MagicPalette::editorial_swatch("Cream").unwrap_or(Color32::from_rgb(255, 253, 248));
    ui.painter().rect_filled(rect, CornerRadius::same(4), paper);
    if !paint_local_image(ui, rect.shrink(1.0), page.preview_png.as_deref(), paper, 3) {
        paint_mini_vector_chrome(ui, rect, page);
    }
    let border = if selected { Stroke::new(2.0, accent) } else { Stroke::new(1.0, Color32::from_rgb(210, 198, 182)) };
    ui.painter().rect_stroke(rect, CornerRadius::same(4), border, StrokeKind::Inside);
}

fn paint_mini_vector_chrome(ui: &mut Ui, rect: Rect, page: &PageSummary) {
    for item in &page.layout_items {
        let Some(b) = item["bounds"].as_array().and_then(|arr| local_rect(rect, page.page_size_pt, arr)) else {
            continue;
        };
        if let Some(fill) = item["fill"].as_str().and_then(MagicPalette::editorial_swatch) {
            ui.painter().rect_filled(b, CornerRadius::same(1), fill);
        }
        if let Some(st) = item["stroke"].as_str().and_then(MagicPalette::editorial_swatch)
            && b.width() > 2.0
            && b.height() > 2.0
        {
            ui.painter().rect_stroke(b, CornerRadius::same(1), Stroke::new(0.5, st), StrokeKind::Inside);
        }
    }
    for slot in &page.slots {
        if slot["kind"].as_str() == Some("image")
            && let Some(b) = slot["bounds"].as_array().and_then(|arr| local_rect(rect, page.page_size_pt, arr))
        {
            let img = slot["image"].as_str().filter(|p| !p.trim().is_empty());
            let slot_bg = MagicPalette::editorial_swatch("Slot").unwrap_or(Color32::from_rgb(238, 232, 220));
            if !paint_local_image(ui, b, img, slot_bg, 1) {
                ui.painter().rect_filled(b, CornerRadius::same(1), slot_bg);
            }
        }
    }
}

/// Draw a full interactive Barnyard 16:9 Storyboard slide inside the center scroll.
pub fn draw_interactive_page(ui: &mut Ui, ctx: &mut ViewCtx<'_>, page: &PageSummary, page_size: Vec2, dropped_paths: &[String]) -> Rect {
    let (page_rect, page_resp) = ui.allocate_exact_size(page_size, Sense::click());
    ctx.reg.record(format!("sb-page-{:02}", page.index + 1), format!("Storyboard {} ({})", page.sub_code, page.title), &page_resp, None);
    if page_resp.clicked() {
        ctx.state.storyboard.current_page = page.index;
    }

    let shadow = page_rect.translate(Vec2::new(0.0, 4.0));
    ui.painter().rect_filled(shadow, CornerRadius::same(6), Color32::from_black_alpha(64));
    let paper = MagicPalette::editorial_swatch("Cream").unwrap_or(Color32::from_rgb(255, 253, 248));
    ui.painter().rect_filled(page_rect, CornerRadius::same(5), paper);

    let has_raster_bg = paint_local_image(ui, page_rect, page.preview_png.as_deref(), paper, 4);
    let scale = page_rect.width() / page.page_size_pt[0].max(1.0);

    if !has_raster_bg {
        paint_readonly_master_chrome(ui, page_rect, page, scale);
    }

    let env = SlotRenderEnv {
        page_rect,
        scale,
        has_raster_bg,
        dropped_paths,
        pointer_pos: ui.ctx().input(|i| i.pointer.hover_pos().or_else(|| i.pointer.latest_pos())),
        pasted: pasted_file_path(ui),
    };
    for slot in &page.slots {
        draw_single_slot(ui, ctx, page, slot, &env);
    }

    let is_active = ctx.state.storyboard.current_page == page.index;
    let sec_col = ctx.pal.section_accent(&page.section);
    let frame_stroke = if is_active { Stroke::new(2.0, sec_col) } else { Stroke::new(1.0, Color32::from_rgb(212, 200, 184)) };
    ui.painter().rect_stroke(page_rect, CornerRadius::same(5), frame_stroke, StrokeKind::Inside);
    page_rect
}

fn paint_readonly_master_chrome(ui: &mut Ui, page_rect: Rect, page: &PageSummary, scale: f32) {
    let clip_painter = ui.painter().with_clip_rect(page_rect);
    for item in &page.layout_items {
        let locked = item["locked"].as_bool().unwrap_or(false);
        let kind = item["kind"].as_str().unwrap_or("shape");
        if !locked && kind != "shape" {
            continue;
        }
        let Some(rect) = item["bounds"].as_array().and_then(|arr| local_rect(page_rect, page.page_size_pt, arr)) else {
            continue;
        };
        let radius = if rect.height() <= 24.0 * scale { 3 } else { 4 };
        if let Some(fill) = item["fill"].as_str().and_then(MagicPalette::editorial_swatch) {
            let r = if rect.height() <= 10.0 * scale && rect.width() >= page_rect.width() * 0.9 { 0 } else { radius };
            clip_painter.rect_filled(rect, CornerRadius::same(r), fill);
        }
        let sw = (item["stroke_weight"].as_f64().unwrap_or(0.0) as f32 * scale).max(0.75);
        if item["stroke_weight"].as_f64().unwrap_or(0.0) > 0.0
            && let Some(st) = item["stroke"].as_str().and_then(MagicPalette::editorial_swatch)
        {
            if rect.height() < 1.5 && rect.width() > 2.0 {
                clip_painter.line_segment([rect.left_center(), rect.right_center()], Stroke::new(sw, st));
            } else if rect.width() < 1.5 && rect.height() > 2.0 {
                clip_painter.line_segment([rect.center_top(), rect.center_bottom()], Stroke::new(sw, st));
            } else {
                clip_painter.rect_stroke(rect, CornerRadius::same(radius), Stroke::new(sw, st), StrokeKind::Inside);
            }
        }
        if kind == "text"
            && let Some(raw_text) = item["text"].as_str().filter(|t| !t.trim().is_empty())
        {
            let raw_sz = item["font_size"].as_f64().unwrap_or(8.5) as f32;
            let pt = (raw_sz * scale * 0.94).clamp(6.8, 72.0);
            let col = item["text_fill"].as_str().and_then(MagicPalette::editorial_swatch).unwrap_or(Color32::from_rgb(41, 40, 33));
            let is_kicker = raw_sz <= 9.2;
            paint_wrapped_editorial_text(&clip_painter, WrappedTextSpec { rect, text: raw_text, font_pt: pt, color: col, monospace: is_kicker });
        }
    }
}

fn draw_single_slot(ui: &mut Ui, ctx: &mut ViewCtx<'_>, page: &PageSummary, slot: &Value, env: &SlotRenderEnv<'_>) {
    let Some(label) = slot["label"].as_str() else { return };
    let Some(rect) = slot["bounds"].as_array().and_then(|arr| local_rect(env.page_rect, page.page_size_pt, arr)) else {
        return;
    };
    if rect.width() < 10.0 || rect.height() < 8.0 {
        return;
    }
    if slot["kind"].as_str().unwrap_or("text") == "text" {
        draw_text_slot(ui, ctx, page, slot, label, rect, env);
    } else {
        draw_image_slot(ui, ctx, page, slot, label, rect, env);
    }
}

fn is_dark_banner_slot(label: &str) -> bool {
    matches!(
        label,
        "status"
            | "scene_id"
            | "movement"
            | "identity"
            | "editing"
            | "motion"
            | "caption"
            | "asset"
            | "review"
            | "final_deliverables"
            | "beat_1"
            | "beat_2"
            | "beat_3"
            | "beat_4"
            | "beat_5"
    ) || label.ends_with("_edit_line")
}

fn draw_text_slot(ui: &mut Ui, ctx: &mut ViewCtx<'_>, page: &PageSummary, slot: &Value, label: &str, rect: Rect, env: &SlotRenderEnv<'_>) {
    let is_editing = ctx.state.storyboard.editing_page == Some(page.index) && ctx.state.storyboard.editing_slot.as_deref() == Some(label);
    if is_editing {
        draw_inline_text_editor(ui, ctx, rect, page.index, label, env.scale);
        return;
    }

    let filled = slot["filled"].as_bool().unwrap_or(false);
    let required = slot["required"].as_bool().unwrap_or(false);
    let resp = ui.interact(rect, ui.make_persistent_id(("sb_txt_slot", page.index, label)), Sense::click());
    ctx.reg.record(format!("sb-field-p{:02}-{label}", page.index), format!("Edit {label} ({})", page.sub_code), &resp, None);

    let val_str = slot["text"].as_str().unwrap_or("");
    let on_dark = is_dark_banner_slot(label);
    if !env.has_raster_bg {
        paint_slot_text_box(ui, rect, val_str, filled, required && !filled, resp.hovered(), on_dark);
    } else if resp.hovered() || (required && !filled) {
        let st = if resp.hovered() { Stroke::new(1.5, ctx.pal.accent_emerald) } else { Stroke::new(1.0, Color32::from_rgba_unmultiplied(210, 72, 62, 140)) };
        ui.painter().rect_stroke(rect, CornerRadius::same(2), st, StrokeKind::Inside);
    }

    if resp.clicked() {
        let clean_initial = if val_str.contains('[') && val_str.contains(']') { String::new() } else { val_str.to_owned() };
        ctx.state.storyboard.current_page = page.index;
        ctx.state.storyboard.editing_page = Some(page.index);
        ctx.state.storyboard.editing_slot = Some(label.to_owned());
        ctx.state.storyboard.draft_text = clean_initial;
        ctx.state.storyboard.focus_requested = true;
    }
    resp.on_hover_text(format!("Click to edit `{label}` ({})", page.sub_code));
}

fn paint_slot_text_box(ui: &mut Ui, rect: Rect, val_str: &str, filled: bool, missing_req: bool, hovered: bool, on_dark: bool) {
    if hovered {
        let hover_fill = if on_dark { Color32::from_white_alpha(24) } else { Color32::from_rgb(255, 250, 238) };
        ui.painter().rect_filled(rect, CornerRadius::same(3), hover_fill);
        ui.painter().rect_stroke(rect, CornerRadius::same(3), Stroke::new(1.3, Color32::from_rgb(182, 74, 55)), StrokeKind::Inside);
    } else if missing_req {
        ui.painter().rect_stroke(rect, CornerRadius::same(3), Stroke::new(0.9, Color32::from_rgba_unmultiplied(210, 72, 62, 145)), StrokeKind::Inside);
    }
    if val_str.trim().is_empty() {
        return;
    }
    let is_hero_title = rect.height() >= 26.0 && val_str.chars().count() <= 46 && !on_dark;
    let pt = if is_hero_title { (rect.height() * 0.50).clamp(11.0, 22.0) } else { (rect.height() * 0.34).clamp(8.2, 11.2) };
    let col = if on_dark {
        Color32::from_rgb(250, 247, 240)
    } else if filled {
        Color32::from_rgb(38, 36, 31)
    } else {
        Color32::from_rgb(122, 114, 102)
    };
    paint_wrapped_editorial_text(ui.painter(), WrappedTextSpec { rect, text: val_str, font_pt: pt, color: col, monospace: false });
}

fn draw_image_slot(ui: &mut Ui, ctx: &mut ViewCtx<'_>, page: &PageSummary, slot: &Value, label: &str, rect: Rect, env: &SlotRenderEnv<'_>) {
    let filled = slot["filled"].as_bool().unwrap_or(false);
    let required = slot["required"].as_bool().unwrap_or(false);
    let is_editing = ctx.state.storyboard.editing_page == Some(page.index) && ctx.state.storyboard.editing_slot.as_deref() == Some(label);

    let resp = ui.interact(rect, ui.make_persistent_id(("sb_img_slot", page.index, label)), Sense::click());
    ctx.reg.record(format!("sb-img-p{:02}-{label}", page.index), format!("Artwork Slot {label} ({})", page.sub_code), &resp, None);

    let img_path = slot["image"].as_str().filter(|p| !p.trim().is_empty());
    let slot_bg = MagicPalette::editorial_swatch("Slot").unwrap_or(Color32::from_rgb(240, 232, 220));
    if (!env.has_raster_bg || img_path.is_some()) && !paint_local_image(ui, rect, img_path, slot_bg, 3) && !env.has_raster_bg {
        ui.painter().rect_filled(rect, CornerRadius::same(3), slot_bg);
        ui.painter().rect_stroke(rect.shrink(2.0), CornerRadius::same(2), Stroke::new(0.8, Color32::from_rgb(206, 194, 178)), StrokeKind::Inside);
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            format!("+ {label}"),
            FontId::new((8.8 * env.scale).clamp(7.5, 11.0), FontFamily::Monospace),
            Color32::from_rgb(112, 104, 94),
        );
    }

    let hovered = resp.hovered() || env.pointer_pos.is_some_and(|pos| rect.contains(pos));
    if hovered || is_editing || (required && !filled) {
        let stroke =
            if hovered || is_editing { Stroke::new(1.8, ctx.pal.accent_emerald) } else { Stroke::new(1.0, Color32::from_rgba_unmultiplied(182, 74, 55, 155)) };
        ui.painter().rect_stroke(rect, CornerRadius::same(3), stroke, StrokeKind::Inside);
    }

    let linked_meta = slot["linked_file"].as_str().filter(|l| !l.is_empty());
    let app_meta = slot["craft_app"].as_str().filter(|a| !a.is_empty());

    if hovered && let Some(dropped) = env.dropped_paths.first().or(env.pasted.as_ref()) {
        ctx.state.storyboard.current_page = page.index;
        ctx.exec("storyboard.set", json!({"page": page.index, "label": label, "image": dropped}), Some("Placed artwork onto Storyboard slide"));
    } else if resp.secondary_clicked()
        && let Some(linked_path) = linked_meta
    {
        let mut open_req = json!({"path": linked_path});
        if let Some(app) = app_meta {
            open_req["app"] = json!(app);
        }
        ctx.exec("workspace.open", open_req, Some("Opened linked Craft file"));
    } else if resp.clicked() {
        ctx.state.storyboard.current_page = page.index;
        ctx.state.storyboard.editing_page = Some(page.index);
        ctx.state.storyboard.editing_slot = Some(label.to_owned());
        ctx.push(UiAction::PickFile {
            target: PickerTarget::StoryboardSlotImage { page: page.index, slot: label.to_owned() },
            title: format!("Choose Artwork for {label} ({})", page.sub_code),
            extensions: SLOT_PICKER_EXTS.iter().map(|s| (*s).to_owned()).collect(),
            directory: false,
            save: false,
        });
    }

    paint_craft_slot_badge(ui, rect, app_meta, linked_meta, slot["time_sec"].as_f64());
    resp.on_hover_text(format!("`{label}` · Click/drop/paste to place artwork · Right-click to open linked Craft file"));
}

fn paint_craft_slot_badge(ui: &mut Ui, rect: Rect, app_meta: Option<&str>, linked_meta: Option<&str>, time_sec: Option<f64>) {
    let Some(app) = app_meta.or_else(|| linked_meta.map(|_| "linked")) else {
        return;
    };
    let badge_txt = match time_sec {
        Some(sec) => format!("{} {:.1}S", app.to_ascii_uppercase(), sec),
        None => app.to_ascii_uppercase(),
    };
    let badge_w = (badge_txt.len() as f32 * 5.4 + 8.0).min(rect.width() - 6.0);
    let badge_rect = Rect::from_min_size(Pos2::new(rect.min.x + 3.0, rect.min.y + 3.0), Vec2::new(badge_w, 13.0));
    if badge_rect.width() > 18.0 && rect.height() > 24.0 {
        ui.painter().rect_filled(badge_rect, CornerRadius::same(2), Color32::from_black_alpha(168));
        ui.painter().text(badge_rect.center(), Align2::CENTER_CENTER, badge_txt, FontId::new(7.8, FontFamily::Monospace), Color32::from_rgb(248, 245, 238));
    }
}

fn draw_inline_text_editor(ui: &mut Ui, ctx: &mut ViewCtx<'_>, rect: Rect, page_idx: usize, label: &str, scale: f32) {
    let edit_rect = Rect::from_min_size(rect.min, Vec2::new(rect.width().max(130.0), rect.height().max(24.0)));
    ui.painter().rect_filled(edit_rect, CornerRadius::same(3), Color32::from_rgb(255, 253, 247));
    ui.painter().rect_stroke(edit_rect, CornerRadius::same(3), Stroke::new(1.8, ctx.pal.accent_emerald), StrokeKind::Inside);

    let multiline = rect.height() >= 28.0 * scale;
    let font_pt = (10.2 * scale).clamp(9.0, 14.5);

    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(edit_rect.shrink(2.0)).id_salt(("sb_inline_editor", page_idx, label)));
    child.style_mut().visuals.extreme_bg_color = Color32::from_rgb(255, 253, 247);
    child.style_mut().visuals.override_text_color = Some(Color32::from_rgb(24, 24, 22));

    let resp = if multiline {
        child.add_sized(
            edit_rect.shrink(2.0).size(),
            TextEdit::multiline(&mut ctx.state.storyboard.draft_text).font(FontId::new(font_pt, FontFamily::Proportional)).frame(egui::Frame::NONE),
        )
    } else {
        child.add_sized(
            edit_rect.shrink(2.0).size(),
            TextEdit::singleline(&mut ctx.state.storyboard.draft_text).font(FontId::new(font_pt, FontFamily::Proportional)).frame(egui::Frame::NONE),
        )
    };

    if ctx.state.storyboard.focus_requested {
        resp.request_focus();
        ctx.state.storyboard.focus_requested = false;
    }

    let enter_commit = ui.input(|i| i.key_pressed(Key::Enter) && (!multiline || i.modifiers.command || i.modifiers.ctrl));
    let esc_pressed = ui.input(|i| i.key_pressed(Key::Escape));

    if esc_pressed {
        ctx.state.storyboard.editing_page = None;
        ctx.state.storyboard.editing_slot = None;
    } else if enter_commit || resp.lost_focus() {
        let new_val = ctx.state.storyboard.draft_text.trim().to_owned();
        ctx.state.storyboard.editing_page = None;
        ctx.state.storyboard.editing_slot = None;
        ctx.exec("storyboard.set", json!({"page": page_idx, "label": label, "text": new_val}), Some("Saved Storyboard field"));
    }
}
