//! Seamless `612 × 792 pt` US Letter `storyboard.pdf` (`PDFCraft`) page renderer with
//! click-in-place text editing, drag-and-drop / Wayland paste / click image placement, and
//! right-click handoff to `PhotoCraft`, `VectorCraft`, or `EffectCraft` for linked Craft files.

use egui::{Align2, Color32, CornerRadius, Event, FontFamily, FontId, Key, Pos2, Rect, Sense, Stroke, StrokeKind, TextEdit, Ui, Vec2};
use serde_json::{Value, json};

use crate::controls::paint_local_image;
use crate::state::{PickerTarget, UiAction, ViewCtx};

pub const PAGE_W_PT: f32 = 612.0;
pub const PAGE_H_PT: f32 = 792.0;

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
    pub title: String,
    pub light_green: bool,
    pub preview_png: Option<String>,
    pub slots: Vec<Value>,
    pub layout_items: Vec<Value>,
}

fn swatch_color(swatch: &str) -> Option<Color32> {
    match swatch {
        "Cream" => Some(Color32::from_rgb(247, 243, 234)),
        "Kraft" => Some(Color32::from_rgb(241, 235, 223)),
        "[Paper]" => Some(Color32::from_rgb(252, 250, 245)),
        "Ink" | "[Black]" => Some(Color32::from_rgb(34, 33, 31)),
        "Rust" => Some(Color32::from_rgb(175, 79, 51)),
        "Pencil" => Some(Color32::from_rgb(116, 112, 106)),
        "Hair" => Some(Color32::from_rgb(204, 198, 187)),
        "Slot" => Some(Color32::from_rgb(234, 229, 218)),
        "Tape" => Some(Color32::from_rgb(231, 226, 215)),
        "Film" => Some(Color32::from_rgb(217, 213, 204)),
        "Leather" => Some(Color32::from_rgb(207, 201, 190)),
        "Navy" => Some(Color32::from_rgb(51, 70, 93)),
        "Moss" => Some(Color32::from_rgb(65, 97, 74)),
        "Sun" => Some(Color32::from_rgb(199, 148, 51)),
        _ => None,
    }
}

fn local_rect(page_rect: Rect, bounds: &[Value]) -> Option<Rect> {
    if bounds.len() < 4 {
        return None;
    }
    let sx = page_rect.width() / PAGE_W_PT;
    let sy = page_rect.height() / PAGE_H_PT;
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

/// Draw a compact miniature page preview (`2-up` left section rail).
pub fn paint_mini_page(ui: &mut Ui, rect: Rect, page: &PageSummary, selected: bool, accent: Color32) {
    let paper = Color32::from_rgb(247, 243, 234);
    ui.painter().rect_filled(rect, CornerRadius::same(3), paper);
    if !paint_local_image(ui, rect.shrink(1.0), page.preview_png.as_deref(), paper, 2) {
        for item in &page.layout_items {
            let Some(b) = item["bounds"].as_array().and_then(|arr| local_rect(rect, arr)) else {
                continue;
            };
            let kind = item["kind"].as_str().unwrap_or("shape");
            if let Some(img) = item["image"].as_str().filter(|p| !p.is_empty()) {
                let _ = paint_local_image(ui, b, Some(img), Color32::from_rgb(234, 229, 218), 1);
                continue;
            }
            if let Some(fill) = item["fill"].as_str().and_then(swatch_color) {
                ui.painter().rect_filled(b, CornerRadius::ZERO, fill);
            }
            if let Some(st) = item["stroke"].as_str().and_then(swatch_color) {
                if b.height() < 1.2 && b.width() > 2.0 {
                    ui.painter().line_segment([b.left_center(), b.right_center()], Stroke::new(0.8, st));
                } else if b.width() > 2.0 && b.height() > 2.0 {
                    ui.painter().rect_stroke(b, CornerRadius::ZERO, Stroke::new(0.5, st), StrokeKind::Inside);
                }
            }
            if kind == "text"
                && item["font_size"].as_f64().unwrap_or(9.0) >= 22.0
                && let Some(t) = item["text"].as_str().filter(|s| !s.is_empty())
            {
                let col = item["text_fill"].as_str().and_then(swatch_color).unwrap_or(Color32::from_rgb(34, 33, 31));
                ui.painter().text(
                    b.left_top() + Vec2::new(1.0, 0.5),
                    Align2::LEFT_TOP,
                    t.chars().take(12).collect::<String>(),
                    FontId::new((rect.width() * 0.085).clamp(6.0, 9.5), FontFamily::Proportional),
                    col,
                );
            }
        }
    }
    let border = if selected { Stroke::new(2.0, accent) } else { Stroke::new(1.0, Color32::from_rgb(185, 179, 168)) };
    ui.painter().rect_stroke(rect, CornerRadius::same(3), border, StrokeKind::Inside);
}

/// Draw a full interactive `storyboard.pdf` page inside the seamless center scroll:
/// - Locked chrome, editorial rules, and headings are strictly read-only.
/// - Unlocked text fields become live inline `TextEdit` widgets directly on the page when clicked.
/// - Unlocked image slots accept OS drag-and-drop, Wayland clipboard file paste (`Ctrl+V` / `⌘V`),
///   click-to-pick (`craft-dialogs`), and right-click to open linked `.pcraft`/`.vectorcraft`/`.ecproj`.
pub fn draw_interactive_page(ui: &mut Ui, ctx: &mut ViewCtx<'_>, page: &PageSummary, page_size: Vec2, dropped_paths: &[String]) -> Rect {
    let (page_rect, page_resp) = ui.allocate_exact_size(page_size, Sense::click());
    ctx.reg.record(format!("sb-page-{:02}", page.index + 1), format!("Storyboard Page {:02} ({})", page.index + 1, page.section), &page_resp, None);
    if page_resp.clicked() {
        ctx.state.storyboard.current_page = page.index;
    }

    let shadow = page_rect.translate(Vec2::new(0.0, 3.0));
    ui.painter().rect_filled(shadow, CornerRadius::same(4), Color32::from_black_alpha(70));
    let paper = Color32::from_rgb(247, 243, 234);
    ui.painter().rect_filled(page_rect, CornerRadius::same(3), paper);

    let has_raster_bg = paint_local_image(ui, page_rect, page.preview_png.as_deref(), paper, 3);
    let scale = page_rect.width() / PAGE_W_PT;

    if !has_raster_bg {
        paint_readonly_master_chrome(ui, page_rect, page, scale);
    }

    draw_interactive_slots_on_page(ui, ctx, page_rect, page, scale, has_raster_bg, dropped_paths);

    let is_active = ctx.state.storyboard.current_page == page.index;
    let frame_stroke = if is_active { Stroke::new(1.8, ctx.pal.accent_emerald) } else { Stroke::new(1.0, Color32::from_rgb(196, 190, 178)) };
    ui.painter().rect_stroke(page_rect, CornerRadius::same(3), frame_stroke, StrokeKind::Inside);
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
        let Some(rect) = item["bounds"].as_array().and_then(|arr| local_rect(page_rect, arr)) else {
            continue;
        };
        if let Some(fill) = item["fill"].as_str().and_then(swatch_color) {
            clip_painter.rect_filled(rect, CornerRadius::ZERO, fill);
        }
        let sw = (item["stroke_weight"].as_f64().unwrap_or(0.0) as f32 * scale).max(0.6);
        if item["stroke_weight"].as_f64().unwrap_or(0.0) > 0.0
            && let Some(st) = item["stroke"].as_str().and_then(swatch_color)
        {
            if rect.height() < 1.5 && rect.width() > 2.0 {
                clip_painter.line_segment([rect.left_center(), rect.right_center()], Stroke::new(sw, st));
            } else if rect.width() < 1.5 && rect.height() > 2.0 {
                clip_painter.line_segment([rect.center_top(), rect.center_bottom()], Stroke::new(sw, st));
            } else {
                clip_painter.rect_stroke(rect, CornerRadius::ZERO, Stroke::new(sw, st), StrokeKind::Inside);
            }
        }
        if kind == "text"
            && let Some(raw_text) = item["text"].as_str().filter(|t| !t.trim().is_empty())
        {
            let pt = (item["font_size"].as_f64().unwrap_or(9.0) as f32 * scale * 0.88).clamp(6.5, 84.0);
            let col = item["text_fill"].as_str().and_then(swatch_color).unwrap_or(Color32::from_rgb(34, 33, 31));
            let line_h = (pt * 1.15).max(8.0);
            let mut y = rect.min.y + 1.0;
            for line in raw_text.lines() {
                if y + pt > rect.max.y + 4.0 {
                    break;
                }
                clip_painter.text(Pos2::new(rect.min.x + 1.0, y), Align2::LEFT_TOP, line, FontId::new(pt, FontFamily::Proportional), col);
                y += line_h;
            }
        }
    }
}

fn draw_interactive_slots_on_page(
    ui: &mut Ui,
    ctx: &mut ViewCtx<'_>,
    page_rect: Rect,
    page: &PageSummary,
    scale: f32,
    has_raster_bg: bool,
    dropped_paths: &[String],
) {
    let pointer_pos = ui.ctx().input(|i| i.pointer.hover_pos().or_else(|| i.pointer.latest_pos()));
    let pasted = pasted_file_path(ui);

    for slot in &page.slots {
        let Some(label) = slot["label"].as_str() else { continue };
        let kind = slot["kind"].as_str().unwrap_or("text");
        let filled = slot["filled"].as_bool().unwrap_or(false);
        let required = slot["required"].as_bool().unwrap_or(false);
        let Some(rect) = slot["bounds"].as_array().and_then(|arr| local_rect(page_rect, arr)) else {
            continue;
        };
        if rect.width() < 10.0 || rect.height() < 8.0 {
            continue;
        }

        let is_editing = ctx.state.storyboard.editing_page == Some(page.index) && ctx.state.storyboard.editing_slot.as_deref() == Some(label);

        if kind == "text" {
            if is_editing {
                draw_inline_text_editor(ui, ctx, rect, page.index, label, scale);
            } else {
                let resp = ui.interact(rect, ui.make_persistent_id(("sb_txt_slot", page.index, label)), Sense::click());
                ctx.reg.record(format!("sb-field-p{:02}-{label}", page.index), format!("Edit {} (Page {})", label, page.index + 1), &resp, None);
                let val_str = slot["text"].as_str().unwrap_or("");
                if !has_raster_bg {
                    paint_text_slot_value(ui, rect, val_str, filled, required, resp.hovered(), scale);
                } else if resp.hovered() || (required && !filled) {
                    let outline = if resp.hovered() {
                        Stroke::new(1.5, ctx.pal.accent_emerald)
                    } else {
                        Stroke::new(1.0, Color32::from_rgba_unmultiplied(210, 72, 62, 140))
                    };
                    ui.painter().rect_stroke(rect, CornerRadius::same(2), outline, StrokeKind::Inside);
                }
                if resp.clicked() {
                    let clean_initial = if val_str.contains('[') && val_str.contains(']') { String::new() } else { val_str.to_owned() };
                    ctx.state.storyboard.current_page = page.index;
                    ctx.state.storyboard.editing_page = Some(page.index);
                    ctx.state.storyboard.editing_slot = Some(label.to_owned());
                    ctx.state.storyboard.draft_text = clean_initial;
                    ctx.state.storyboard.focus_requested = true;
                }
                resp.on_hover_text(format!("Click to edit `{label}` in storyboard.pdf"));
            }
        } else {
            let resp = ui.interact(rect, ui.make_persistent_id(("sb_img_slot", page.index, label)), Sense::click());
            ctx.reg.record(format!("sb-img-p{:02}-{label}", page.index), format!("Image Slot {} (Page {})", label, page.index + 1), &resp, None);
            let img_path = slot["image"].as_str().filter(|p| !p.trim().is_empty());
            if !has_raster_bg || img_path.is_some() {
                let slot_bg = Color32::from_rgb(234, 229, 218);
                if !paint_local_image(ui, rect, img_path, slot_bg, 2) && !has_raster_bg {
                    ui.painter().rect_filled(rect, CornerRadius::same(2), slot_bg);
                    ui.painter().text(
                        rect.center(),
                        Align2::CENTER_CENTER,
                        "Drop image or click",
                        FontId::new((9.0 * scale).clamp(7.5, 11.5), FontFamily::Proportional),
                        Color32::from_rgb(116, 112, 106),
                    );
                }
            }
            let hovered = resp.hovered() || pointer_pos.is_some_and(|pos| rect.contains(pos));
            if hovered || is_editing || (required && !filled) {
                let stroke = if hovered || is_editing {
                    Stroke::new(1.8, ctx.pal.accent_emerald)
                } else {
                    Stroke::new(1.0, Color32::from_rgba_unmultiplied(210, 72, 62, 150))
                };
                ui.painter().rect_stroke(rect, CornerRadius::same(2), stroke, StrokeKind::Inside);
            }
            let linked_meta = slot["linked_file"].as_str().filter(|l| !l.is_empty());
            let app_meta = slot["craft_app"].as_str().filter(|a| !a.is_empty());

            if hovered && let Some(dropped) = dropped_paths.first().or(pasted.as_ref()) {
                ctx.state.storyboard.current_page = page.index;
                ctx.exec("storyboard.set", json!({"page": page.index, "label": label, "image": dropped}), Some("Placed artwork onto storyboard.pdf"));
            } else if resp.secondary_clicked()
                && let Some(linked_path) = linked_meta
            {
                let mut open_req = json!({"path": linked_path});
                if let Some(app) = app_meta {
                    open_req["app"] = json!(app);
                }
                ctx.exec("workspace.open", open_req, Some("Opened linked Craft project file"));
            } else if resp.clicked() {
                ctx.state.storyboard.current_page = page.index;
                ctx.state.storyboard.editing_page = Some(page.index);
                ctx.state.storyboard.editing_slot = Some(label.to_owned());
                ctx.push(UiAction::PickFile {
                    target: PickerTarget::StoryboardSlotImage { page: page.index, slot: label.to_owned() },
                    title: format!("Choose Artwork for {label} (Page {:02})", page.index + 1),
                    extensions: SLOT_PICKER_EXTS.iter().map(|s| (*s).to_owned()).collect(),
                    directory: false,
                    save: false,
                });
            }

            if let Some(app) = app_meta.or_else(|| linked_meta.map(|_| "linked")) {
                let badge_txt = match slot["time_sec"].as_f64() {
                    Some(sec) => format!("{} {:.2}S", app.to_ascii_uppercase(), sec),
                    None => app.to_ascii_uppercase(),
                };
                let badge_rect = Rect::from_min_size(
                    Pos2::new(rect.min.x + 3.0, rect.min.y + 3.0),
                    Vec2::new((badge_txt.len() as f32 * 5.6 + 8.0).min(rect.width() - 6.0), 13.0),
                );
                if badge_rect.width() > 16.0 && rect.height() > 20.0 {
                    ui.painter().rect_filled(badge_rect, CornerRadius::same(2), Color32::from_black_alpha(165));
                    ui.painter().text(
                        badge_rect.center(),
                        Align2::CENTER_CENTER,
                        badge_txt,
                        FontId::new(8.0, FontFamily::Monospace),
                        Color32::from_rgb(245, 242, 235),
                    );
                }
            }
            let outline_info = slot["outline"].as_str().filter(|o| !o.is_empty());
            let hover_msg = match (linked_meta, outline_info, app_meta) {
                (Some(lf), Some(out), Some(app)) => {
                    format!("`{label}` · {app} ({out})\nLinked: {lf}\nClick/drop/paste to replace · Right-click to open in {app}")
                }
                (Some(lf), _, Some(app)) => {
                    format!("`{label}` · Linked: {lf}\nClick/drop/paste to replace · Right-click to open in {app}")
                }
                (Some(lf), _, None) => {
                    format!("`{label}` · Linked: {lf}\nClick/drop/paste to replace · Right-click to open")
                }
                _ => format!("Drop, paste, or click to place PhotoCraft / VectorCraft / EffectCraft artwork in `{label}`"),
            };
            resp.on_hover_text(hover_msg);
        }
    }
}

fn paint_text_slot_value(ui: &mut Ui, rect: Rect, val_str: &str, filled: bool, required: bool, hovered: bool, scale: f32) {
    let bg = if hovered {
        Color32::from_rgb(255, 252, 242)
    } else if required && !filled {
        Color32::from_rgba_unmultiplied(210, 72, 62, 16)
    } else {
        Color32::TRANSPARENT
    };
    if bg != Color32::TRANSPARENT {
        ui.painter().rect_filled(rect, CornerRadius::same(2), bg);
    }
    let border = if hovered {
        Some(Stroke::new(1.4, Color32::from_rgb(175, 79, 51)))
    } else if required && !filled {
        Some(Stroke::new(0.8, Color32::from_rgba_unmultiplied(210, 72, 62, 130)))
    } else {
        None
    };
    if let Some(st) = border {
        ui.painter().rect_stroke(rect, CornerRadius::same(2), st, StrokeKind::Inside);
    }
    if val_str.trim().is_empty() {
        return;
    }
    let is_title = rect.height() >= 34.0 * scale && val_str.len() <= 36;
    let pt = if is_title { (22.0 * scale).clamp(11.0, 32.0) } else { (9.5 * scale).clamp(7.5, 13.5) };
    let col = if filled { Color32::from_rgb(30, 29, 27) } else { Color32::from_rgb(132, 124, 114) };
    let clip = ui.painter().with_clip_rect(rect.shrink(1.0));
    let mut y = rect.min.y + 1.5;
    let line_h = (pt * 1.2).max(9.0);
    for line in val_str.lines() {
        if y + pt > rect.max.y + 4.0 {
            break;
        }
        clip.text(Pos2::new(rect.min.x + 2.0, y), Align2::LEFT_TOP, line, FontId::new(pt, FontFamily::Proportional), col);
        y += line_h;
    }
}

fn draw_inline_text_editor(ui: &mut Ui, ctx: &mut ViewCtx<'_>, rect: Rect, page_idx: usize, label: &str, scale: f32) {
    let edit_rect = Rect::from_min_size(rect.min, Vec2::new(rect.width().max(110.0), rect.height().max(22.0)));
    ui.painter().rect_filled(edit_rect, CornerRadius::same(3), Color32::from_rgb(255, 253, 247));
    ui.painter().rect_stroke(edit_rect, CornerRadius::same(3), Stroke::new(1.8, ctx.pal.accent_emerald), StrokeKind::Inside);

    let multiline = rect.height() >= 28.0 * scale;
    let font_pt = (10.5 * scale).clamp(9.0, 15.0);
    let mut commit = false;
    let mut cancel = false;

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
        cancel = true;
    } else if enter_commit || resp.lost_focus() {
        commit = true;
    }

    if cancel {
        ctx.state.storyboard.editing_page = None;
        ctx.state.storyboard.editing_slot = None;
    } else if commit {
        let new_val = ctx.state.storyboard.draft_text.trim().to_owned();
        ctx.state.storyboard.editing_page = None;
        ctx.state.storyboard.editing_slot = None;
        ctx.exec("storyboard.set", json!({"page": page_idx, "label": label, "text": new_val}), Some("Updated storyboard.pdf field"));
    }
}
