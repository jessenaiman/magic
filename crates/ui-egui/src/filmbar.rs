//! 35mm cinema sprocket (`filmbar`) border frames, aspect-preserving local `file://` image
//! rendering, Craft document state cards, anti-aliased Bezier & SVG `d` path previews, dashed
//! borders, and 3-ring glowing readiness lights (`00-figma-component-system-sheet.jpg`).

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

use egui::{Align, Align2, Color32, CornerRadius, FontId, Image, Layout, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Ui, Vec2};

use crate::controls::badge_accent_color;
use crate::theme::MagicPalette;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilmbarOrientation {
    Vertical,
    Horizontal,
}

#[derive(Clone, Copy, Debug)]
pub struct FilmbarStyle<'a> {
    pub orientation: FilmbarOrientation,
    pub selected: bool,
    pub hovered: bool,
    pub timecode: Option<&'a str>,
}

/// Create a clipped top-down child `Ui` inside `rect` without advancing the parent cursor prematurely.
// TODO(MAGIC-REVIEW): CHANGE film previews to verified target-app output; egui API compatibility does not prove a render.
pub fn child_pane(ui: &mut Ui, rect: Rect, salt: &str) -> Ui {
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(Layout::top_down(Align::Min)).scope_id(ui.scope_id().with(salt)));
    child.set_clip_rect(rect);
    child
}

/// Render a panel card with guaranteed vertical interior layout across the available pane width.
pub fn fill_card<R>(ui: &mut Ui, pal: &MagicPalette, stretch_height: bool, add_contents: impl FnOnce(&mut Ui) -> R) -> egui::InnerResponse<R> {
    let avail = ui.available_size();
    pal.card_frame().show(ui, |ui| {
        let inner_w = (avail.x - 22.0).max(64.0);
        if stretch_height {
            let inner_h = (avail.y - 22.0).max(48.0);
            ui.set_min_size(Vec2::new(inner_w, inner_h));
        } else {
            ui.set_min_width(inner_w);
        }
        ui.set_max_width(inner_w);
        ui.vertical(add_contents).inner
    })
}

/// Paint a dashed rounded rectangle border (`+ START HERE` hero card in `01-home-song-catalog.jpg`).
pub fn paint_dashed_rect(ui: &mut Ui, rect: Rect, stroke: Stroke, dash_len: f32, gap_len: f32) {
    let step = (dash_len + gap_len).max(4.0);
    let mut x = rect.min.x + 6.0;
    while x < rect.max.x - 6.0 {
        let x2 = (x + dash_len).min(rect.max.x - 6.0);
        ui.painter().line_segment([Pos2::new(x, rect.min.y), Pos2::new(x2, rect.min.y)], stroke);
        ui.painter().line_segment([Pos2::new(x, rect.max.y), Pos2::new(x2, rect.max.y)], stroke);
        x += step;
    }
    let mut y = rect.min.y + 6.0;
    while y < rect.max.y - 6.0 {
        let y2 = (y + dash_len).min(rect.max.y - 6.0);
        ui.painter().line_segment([Pos2::new(rect.min.x, y), Pos2::new(rect.min.x, y2)], stroke);
        ui.painter().line_segment([Pos2::new(rect.max.x, y), Pos2::new(rect.max.x, y2)], stroke);
        y += step;
    }
}

/// Paint a 3-ring glowing readiness indicator (`5. Readiness Lights` in `00-figma-component-system-sheet.jpg`).
pub fn paint_readiness_light(ui: &mut Ui, center: Pos2, color: Color32) {
    ui.painter().circle_filled(center, 7.6, color.gamma_multiply(0.14));
    ui.painter().circle_filled(center, 5.4, color.gamma_multiply(0.32));
    ui.painter().circle_filled(center, 3.6, color);
}

/// Convert a local filesystem path into a `file://` URI for `egui_extras` image loader if it exists.
pub fn local_image_uri(path: &str) -> Option<String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return None;
    }
    let p = Path::new(trimmed);
    if !p.is_file() {
        return None;
    }
    let ext = p.extension()?.to_str()?.to_ascii_lowercase();
    if !matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp" | "svg") {
        return None;
    }
    let abs = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    let display = abs.to_string_lossy();
    if display.starts_with('/') { Some(format!("file://{display}")) } else { Some(format!("file:///{}", display.replace('\\', "/"))) }
}

fn parse_svg_aspect(path: &Path) -> Option<f32> {
    let raw = std::fs::read_to_string(path).ok()?;
    let head = &raw[..raw.len().min(4096)];
    if let Some(vb_pos) = head.find("viewBox=") {
        let rest = &head[vb_pos + 8..];
        let quote = rest.chars().next()?;
        if quote == '"' || quote == '\'' {
            let inner = rest[1..].split(quote).next()?;
            let nums: Vec<f32> = inner.split(|c: char| c.is_whitespace() || c == ',').filter(|s| !s.is_empty()).filter_map(|s| s.parse::<f32>().ok()).collect();
            if nums.len() == 4 && nums[2] > 0.0 && nums[3] > 0.0 {
                return Some(nums[2] / nums[3]);
            }
        }
    }
    None
}

/// Cached `(width / height)` native aspect ratio read from image header bytes without pixel decode.
pub fn cached_image_aspect(path: &str) -> Option<f32> {
    static CACHE: OnceLock<Mutex<HashMap<String, Option<f32>>>> = OnceLock::new();
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return None;
    }
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(guard) = cache.lock()
        && let Some(&cached) = guard.get(trimmed)
    {
        return cached;
    }
    let p = Path::new(trimmed);
    let computed = if p.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("svg")) {
        parse_svg_aspect(p)
    } else {
        image::image_dimensions(p).ok().filter(|&(w, h)| w > 0 && h > 0).map(|(w, h)| (w as f32) / (h as f32))
    };
    if let Ok(mut guard) = cache.lock() {
        if guard.len() > 2048 {
            guard.clear();
        }
        guard.insert(trimmed.to_owned(), computed);
    }
    computed
}

fn fit_contain_rect(bounds: Rect, aspect: f32) -> Rect {
    if !aspect.is_finite() || aspect <= 0.02 || bounds.width() <= 1.0 || bounds.height() <= 1.0 {
        return bounds;
    }
    let box_aspect = bounds.width() / bounds.height();
    let (w, h) = if aspect >= box_aspect {
        let w = bounds.width();
        (w, (w / aspect).min(bounds.height()))
    } else {
        let h = bounds.height();
        ((h * aspect).min(bounds.width()), h)
    };
    Rect::from_center_size(bounds.center(), Vec2::new(w.max(1.0), h.max(1.0)))
}

/// Paint a local image inside `rect` preserving native aspect ratio (`contain`), returning `true` when drawn.
pub fn paint_local_image(ui: &mut Ui, rect: Rect, path: Option<&str>, bg_fill: Color32, corner_radius: u8) -> bool {
    if bg_fill != Color32::TRANSPARENT {
        ui.painter().rect_filled(rect, CornerRadius::same(corner_radius), bg_fill);
    }
    let Some(raw_path) = path else {
        return false;
    };
    let Some(uri) = local_image_uri(raw_path) else {
        return false;
    };
    let inner = rect.shrink(1.0);
    if !inner.is_positive() {
        return false;
    }
    let img = Image::from_uri(uri);
    let aspect = cached_image_aspect(raw_path).or_else(|| match img.load_for_size(ui.ctx(), inner.size()) {
        Ok(egui::load::TexturePoll::Ready { texture }) if texture.size.y > 0.0 => Some((texture.size.x as f32) / (texture.size.y as f32)),
        _ => None,
    });
    let target = aspect.map_or(inner, |ar| fit_contain_rect(inner, ar));
    let mut clipped = ui.new_child(egui::UiBuilder::new().max_rect(inner));
    clipped.set_clip_rect(inner);
    img.corner_radius(CornerRadius::same(corner_radius.saturating_sub(1))).paint_at(&mut clipped, target);
    true
}

/// Honest Craft project document state inside `rect` when no rendered `.png`/`.svg` frame exists yet on disk.
pub fn paint_craft_doc_card(ui: &mut Ui, rect: Rect, pal: &MagicPalette, badge: &str, craft_file: &str, detail: &str) {
    ui.painter().rect(rect, CornerRadius::same(4), pal.bg_stage, Stroke::new(1.0, pal.border_subtle), StrokeKind::Inside);
    let clipped = ui.painter().with_clip_rect(rect.shrink(2.0));
    let bcol = badge_accent_color(pal, badge);
    let chip_w = (badge.len() as f32 * 7.0 + 14.0).clamp(28.0, (rect.width() - 8.0).max(28.0));
    let cy = rect.center().y;
    let y_shift = if detail.is_empty() || rect.height() <= 54.0 { 8.0 } else { 13.0 };
    let chip = Rect::from_center_size(Pos2::new(rect.center().x, cy - y_shift), Vec2::new(chip_w, 17.0));
    clipped.rect(chip, CornerRadius::same(5), bcol.gamma_multiply(0.18), Stroke::new(1.0, bcol.gamma_multiply(0.75)), StrokeKind::Inside);
    clipped.text(chip.center(), Align2::CENTER_CENTER, badge, FontId::monospace(9.0), bcol);
    let max_chars = ((rect.width() / 6.2) as usize).max(6);
    let file_lbl = if craft_file.chars().count() > max_chars {
        let mut s: String = craft_file.chars().take(max_chars.saturating_sub(1)).collect();
        s.push('…');
        s
    } else {
        craft_file.to_owned()
    };
    clipped.text(
        Pos2::new(rect.center().x, cy + (if detail.is_empty() || rect.height() <= 54.0 { 7.0 } else { 3.0 })),
        Align2::CENTER_CENTER,
        &file_lbl,
        FontId::monospace(9.5),
        pal.text_secondary,
    );
    if !detail.is_empty() && rect.height() > 54.0 {
        let max_det = ((rect.width() / 5.6) as usize).max(10);
        let det_lbl = if detail.chars().count() > max_det {
            let mut s: String = detail.chars().take(max_det.saturating_sub(1)).collect();
            s.push('…');
            s
        } else {
            detail.to_owned()
        };
        clipped.text(Pos2::new(rect.center().x, cy + 18.0), Align2::CENTER_CENTER, &det_lbl, FontId::proportional(9.5), pal.text_muted);
    }
}

/// Paint a smooth anti-aliased cubic Bezier curve via `egui::Shape::line` for real sampled keyframes.
pub fn paint_cubic_bezier(painter: &egui::Painter, pts: [Pos2; 4], stroke: Stroke) {
    let steps = 28usize;
    let mut samples = Vec::with_capacity(steps + 1);
    for i in 0..=steps {
        let t = (i as f32) / (steps as f32);
        let u = 1.0 - t;
        let b0 = u * u * u;
        let b1 = 3.0 * u * u * t;
        let b2 = 3.0 * u * t * t;
        let b3 = t * t * t;
        let x = b0 * pts[0].x + b1 * pts[1].x + b2 * pts[2].x + b3 * pts[3].x;
        let y = b0 * pts[0].y + b1 * pts[1].y + b2 * pts[2].y + b3 * pts[3].y;
        samples.push(Pos2::new(x, y));
    }
    painter.add(Shape::line(samples, stroke));
}

/// Paint real SVG `d` path coordinates read from an authored `motion/svg/*.svg` or `motion/actions.json` cue.
pub fn paint_svg_path_preview(ui: &mut Ui, rect: Rect, svg_d: &str, stroke: Stroke, handle_accent: Option<Color32>) {
    let parsed = parse_svg_coords(svg_d);
    if parsed.len() < 2 {
        return;
    }
    let points = fit_points_into_rect(&parsed, rect.shrink2(Vec2::new(rect.width() * 0.10, rect.height() * 0.14)));
    if points.len() < 2 {
        return;
    }
    ui.painter().add(Shape::line(points.clone(), stroke));
    if let Some(accent) = handle_accent {
        let first = points[0];
        let mid = points[points.len() / 2];
        let last = points[points.len() - 1];
        for anchor in [first, mid, last] {
            ui.painter().circle_filled(anchor, 4.2, Color32::from_rgb(0x0e, 0x14, 0x1e));
            ui.painter().circle_stroke(anchor, 4.2, Stroke::new(1.6, accent));
        }
    }
}

fn parse_svg_coords(raw: &str) -> Vec<(f32, f32)> {
    if !raw.contains('M') && !raw.contains('L') && !raw.contains('C') && !raw.contains('Q') {
        return Vec::new();
    }
    let nums: Vec<f32> =
        raw.split(|c: char| !c.is_ascii_digit() && c != '.' && c != '-').filter(|tok| !tok.is_empty()).filter_map(|tok| tok.parse::<f32>().ok()).collect();
    nums.chunks_exact(2).map(|pair| (pair[0], pair[1])).collect()
}

fn fit_points_into_rect(pts: &[(f32, f32)], target: Rect) -> Vec<Pos2> {
    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for &(x, y) in pts {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    let span_x = (max_x - min_x).max(1.0);
    let span_y = (max_y - min_y).max(1.0);
    pts.iter()
        .map(|&(x, y)| Pos2::new(target.min.x + ((x - min_x) / span_x) * target.width(), target.min.y + ((y - min_y) / span_y) * target.height()))
        .collect()
}

/// Paint a 35mm film-sprocket (`filmbar`) border matching `2. 35mm Filmbar Border Frames` in `00-figma-component-system-sheet.jpg`.
pub fn paint_filmbar_frame(ui: &mut Ui, rect: Rect, pal: &MagicPalette, style: FilmbarStyle<'_>) -> Rect {
    let (rail_fill, border_col, hole_col, stroke_w) = if style.selected {
        (pal.accent_emerald, pal.accent_mint, Color32::from_rgb(0x0a, 0x11, 0x16), 1.6)
    } else if style.hovered {
        (
            if pal.dark { Color32::from_rgb(0x2d, 0x39, 0x4d) } else { pal.bg_hover },
            pal.accent_sky.gamma_multiply(0.75),
            if pal.dark { Color32::from_rgb(0x0b, 0x10, 0x17) } else { pal.bg_stage },
            1.2,
        )
    } else {
        (
            if pal.dark { Color32::from_rgb(0x1f, 0x27, 0x35) } else { pal.border },
            pal.border,
            if pal.dark { Color32::from_rgb(0x0b, 0x0f, 0x16) } else { pal.bg_stage },
            1.0,
        )
    };

    if style.selected {
        ui.painter().rect_stroke(rect.expand(1.5), CornerRadius::same(7), Stroke::new(2.0, pal.accent_emerald.gamma_multiply(0.26)), StrokeKind::Outside);
    }
    ui.painter().rect(rect, CornerRadius::same(6), rail_fill, Stroke::new(stroke_w, border_col), StrokeKind::Inside);

    let inner = match style.orientation {
        FilmbarOrientation::Vertical => {
            let rail_w = 12.0;
            let mut y = rect.min.y + 6.0;
            while y + 6.0 < rect.max.y - 4.0 {
                let left_hole = Rect::from_min_size(Pos2::new(rect.min.x + 3.0, y), Vec2::new(6.0, 4.8));
                let right_hole = Rect::from_min_size(Pos2::new(rect.max.x - 9.0, y), Vec2::new(6.0, 4.8));
                ui.painter().rect_filled(left_hole, CornerRadius::same(1), hole_col);
                ui.painter().rect_filled(right_hole, CornerRadius::same(1), hole_col);
                y += 9.5;
            }
            Rect::from_min_max(Pos2::new(rect.min.x + rail_w, rect.min.y + 2.5), Pos2::new(rect.max.x - rail_w, rect.max.y - 2.5))
        }
        FilmbarOrientation::Horizontal => {
            let rail_h = 11.0;
            let mut x = rect.min.x + 6.0;
            while x + 6.0 < rect.max.x - 4.0 {
                let top_hole = Rect::from_min_size(Pos2::new(x, rect.min.y + 2.6), Vec2::new(5.8, 5.2));
                let bot_hole = Rect::from_min_size(Pos2::new(x, rect.max.y - 7.8), Vec2::new(5.8, 5.2));
                ui.painter().rect_filled(top_hole, CornerRadius::same(1), hole_col);
                ui.painter().rect_filled(bot_hole, CornerRadius::same(1), hole_col);
                x += 10.0;
            }
            Rect::from_min_max(Pos2::new(rect.min.x + 2.5, rect.min.y + rail_h), Pos2::new(rect.max.x - 2.5, rect.max.y - rail_h))
        }
    };

    ui.painter().rect(
        inner,
        CornerRadius::same(3),
        pal.bg_stage,
        Stroke::new(1.0, if style.selected { pal.accent_emerald } else { pal.border_subtle }),
        StrokeKind::Inside,
    );
    if let Some(tc) = style.timecode {
        paint_timecode_pill(ui, inner, pal, tc);
    }
    inner
}

/// Paint a translucent monospace timecode pill in the bottom-right corner of `inner`.
pub fn paint_timecode_pill(ui: &mut Ui, inner: Rect, pal: &MagicPalette, tc: &str) {
    let galley = ui.painter().layout_no_wrap(tc.to_owned(), FontId::monospace(9.5), pal.text_primary);
    let badge = Rect::from_min_size(Pos2::new(inner.max.x - galley.size().x - 12.0, inner.max.y - 17.0), Vec2::new(galley.size().x + 8.0, 14.0));
    ui.painter().rect_filled(badge, CornerRadius::same(4), Color32::from_rgba_unmultiplied(10, 14, 21, 220));
    ui.painter().text(badge.center(), Align2::CENTER_CENTER, tc, FontId::monospace(9.5), pal.text_primary);
}

/// Paint a Craft domain / extension badge (`5. Craft File Badges` in `00` and middle column of `02`).
pub fn paint_craft_badge(ui: &mut Ui, pal: &MagicPalette, label: &str, stadium: bool) {
    let norm = label.trim();
    let upper = norm.to_ascii_uppercase();
    let clean = upper.trim_start_matches('.');
    let (fill, border, fg) = match clean {
        "FC" | "DC" | "EC" | "PCRAFT" | "VCRAFT" | "PC" | "VC" | "VECTORCRAFT" => {
            (Color32::from_rgba_unmultiplied(0x52, 0x3f, 0x1c, 165), Color32::from_rgb(0xc6, 0x9b, 0x4b), pal.accent_amber)
        }
        "FCPROJ" | "PCPROJ" => (pal.accent_emerald_bg, pal.accent_emerald, pal.accent_emerald),
        _ => (Color32::from_rgba_unmultiplied(0x28, 0x33, 0x45, 200), Color32::from_rgb(0x4b, 0x5c, 0x78), pal.text_secondary),
    };
    let font = FontId::monospace(10.0);
    let galley = ui.painter().layout_no_wrap(norm.to_owned(), font.clone(), fg);
    let pad_x = if stadium { 16.0 } else { 12.0 };
    let (rect, _) = ui.allocate_exact_size(Vec2::new((galley.size().x + pad_x).max(32.0), 20.0), Sense::hover());
    let cr = if stadium { CornerRadius::same(10) } else { CornerRadius::same(5) };
    ui.painter().rect(rect, cr, fill, Stroke::new(1.0, border), StrokeKind::Inside);
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, norm, font, fg);
}
