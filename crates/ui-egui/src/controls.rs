//! Signature GPU-painted Magic Studio controls (`egui::Painter`) & layout helpers:
//! - `child_pane` & `fill_card`: top-down clipped sub-pane builders preventing cursor overflow.
//! - `paint_local_image` & `paint_craft_doc_card`: renders real `.png`/`.jpg`/`.webp`/`.svg` Craft frames via `file://` URI, or an honest Craft document badge when unrendered (never dummy/procedural mockups).
//! - `paint_cubic_bezier` & `paint_svg_path_preview`: renders real EffectCraft keyframes and real SVG `d` path coordinates.
//! - `paint_filmbar_frame`: 35mm cinema sprocket border (`Vertical` left/right rails or `Horizontal` top/bottom rails).
//! - `ring_slider_h`, `ring_slider_v`, `xy_path_pad`, `rotary_dial`, `pill_toggle`: precision workstation widgets.
//! - `craft_swatch_button`, `effect_row_button`, & `requirement_light_row`: ArtCraft badges and traffic-light rows.

use std::path::Path;

use egui::{
    Align, Align2, Color32, CornerRadius, FontId, Image, Layout, Pos2, Rect, Response, Sense,
    Shape, Stroke, StrokeKind, Ui, Vec2,
};

use crate::theme::MagicPalette;
use crate::widgets::AutomationRegistry;

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

pub struct ScalarSpec<'a> {
    pub id: &'a str,
    pub label: &'a str,
    pub range: std::ops::RangeInclusive<f64>,
    pub suffix: &'a str,
    pub accent: Color32,
}

impl<'a> ScalarSpec<'a> {
    pub const fn new(
        id: &'a str,
        label: &'a str,
        range: std::ops::RangeInclusive<f64>,
        suffix: &'a str,
        accent: Color32,
    ) -> Self {
        Self {
            id,
            label,
            range,
            suffix,
            accent,
        }
    }
}

pub struct RequirementRow<'a> {
    pub id: &'a str,
    pub title: &'a str,
    pub detail: &'a str,
    pub filled: bool,
    pub selected: bool,
}

pub struct EffectRowSpec<'a> {
    pub id: &'a str,
    pub badge: &'a str,
    pub label: &'a str,
    pub effect_id: &'a str,
    pub selected: bool,
}

/// Create a clipped top-down child `Ui` inside `rect` without advancing the parent cursor prematurely.
pub fn child_pane(ui: &mut Ui, rect: Rect, salt: &str) -> Ui {
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(Layout::top_down(Align::Min))
            .scope_id(ui.scope_id().with(salt)),
    );
    child.set_clip_rect(rect);
    child
}

/// Render a panel card that stretches across the available pane width and optional full pane height.
pub fn fill_card<R>(
    ui: &mut Ui,
    pal: &MagicPalette,
    stretch_height: bool,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> egui::InnerResponse<R> {
    let avail = ui.available_size();
    pal.card_frame().show(ui, |ui| {
        let inner_w = (avail.x - 22.0).max(64.0);
        if stretch_height {
            let inner_h = (avail.y - 22.0).max(48.0);
            ui.set_min_size(Vec2::new(inner_w, inner_h));
        } else {
            ui.set_min_width(inner_w);
        }
        add_contents(ui)
    })
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
    if display.starts_with('/') {
        Some(format!("file://{display}"))
    } else {
        Some(format!("file:///{}", display.replace('\\', "/")))
    }
}

/// Paint a local image inside `rect` if `path` exists on disk, returning `true` when drawn.
pub fn paint_local_image(
    ui: &mut Ui,
    rect: Rect,
    path: Option<&str>,
    bg_fill: Color32,
    corner_radius: u8,
) -> bool {
    ui.painter().rect_filled(rect, CornerRadius::same(corner_radius), bg_fill);
    let Some(uri) = path.and_then(local_image_uri) else {
        return false;
    };
    let mut clipped = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    clipped.set_clip_rect(rect.shrink(1.0));
    Image::from_uri(uri)
        .maintain_aspect_ratio(true)
        .max_size(rect.size())
        .paint_at(&mut clipped, rect.shrink(1.0));
    true
}

/// Honest Craft project document state inside `rect` when no rendered `.png`/`.svg` frame exists yet on disk.
/// Displays the real Craft app badge (`EC`, `FC`, `DC`, `PC`, `VC`) and exact Craft document path — never fake artwork.
pub fn paint_craft_doc_card(
    ui: &mut Ui,
    rect: Rect,
    pal: &MagicPalette,
    badge: &str,
    craft_file: &str,
    detail: &str,
) {
    ui.painter().rect(
        rect,
        CornerRadius::same(4),
        pal.bg_stage,
        Stroke::new(1.0, pal.border_subtle),
        StrokeKind::Inside,
    );
    let bcol = badge_accent_color(pal, badge);
    let chip_w = (badge.len() as f32 * 7.0 + 14.0).max(28.0);
    let cy = rect.center().y;
    let chip = Rect::from_center_size(Pos2::new(rect.center().x, cy - 12.0), Vec2::new(chip_w, 18.0));
    ui.painter().rect(
        chip,
        CornerRadius::same(4),
        bcol.gamma_multiply(0.18),
        Stroke::new(1.0, bcol.gamma_multiply(0.75)),
        StrokeKind::Inside,
    );
    ui.painter()
        .text(chip.center(), Align2::CENTER_CENTER, badge, FontId::monospace(9.5), bcol);
    ui.painter().text(
        Pos2::new(rect.center().x, cy + 4.0),
        Align2::CENTER_CENTER,
        craft_file,
        FontId::monospace(10.0),
        pal.text_secondary,
    );
    if !detail.is_empty() && rect.height() > 54.0 {
        ui.painter().text(
            Pos2::new(rect.center().x, cy + 19.0),
            Align2::CENTER_CENTER,
            detail,
            FontId::proportional(9.5),
            pal.text_muted,
        );
    }
}

/// Paint a smooth anti-aliased cubic Bezier curve via `egui::Shape::line` for real sampled keyframes.
pub fn paint_cubic_bezier(ui: &mut Ui, pts: [Pos2; 4], stroke: Stroke) {
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
    ui.painter().add(Shape::line(samples, stroke));
}

/// Paint real SVG `d` path coordinates read from an authored `motion/svg/*.svg` or `motion/actions.json` cue.
/// Never synthesizes procedural shapes when `svg_d` is empty or non-SVG.
pub fn paint_svg_path_preview(
    ui: &mut Ui,
    rect: Rect,
    svg_d: &str,
    stroke: Stroke,
    handle_accent: Option<Color32>,
) {
    let parsed = parse_svg_coords(svg_d);
    if parsed.len() < 2 {
        return;
    }
    let points = fit_points_into_rect(
        &parsed,
        rect.shrink2(Vec2::new(rect.width() * 0.10, rect.height() * 0.14)),
    );
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
    let nums: Vec<f32> = raw
        .split(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
        .filter(|tok| !tok.is_empty())
        .filter_map(|tok| tok.parse::<f32>().ok())
        .collect();
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
        .map(|&(x, y)| {
            Pos2::new(
                target.min.x + ((x - min_x) / span_x) * target.width(),
                target.min.y + ((y - min_y) / span_y) * target.height(),
            )
        })
        .collect()
}

/// Paint a 35mm film-sprocket (`filmbar`) border around `rect` and return the inner content `Rect`.
pub fn paint_filmbar_frame(ui: &mut Ui, rect: Rect, pal: &MagicPalette, style: FilmbarStyle<'_>) -> Rect {
    let rail_bg = Color32::from_rgb(0x0a, 0x0e, 0x15);
    let border_col = if style.selected {
        pal.accent_emerald
    } else if style.hovered {
        pal.accent_sky
    } else {
        pal.border
    };
    let hole_col = if style.selected {
        pal.accent_emerald.gamma_multiply(0.82)
    } else if style.hovered {
        pal.accent_sky.gamma_multiply(0.60)
    } else {
        Color32::from_rgb(0x36, 0x44, 0x5c)
    };
    let stroke_w = if style.selected { 1.8 } else { 1.0 };
    ui.painter()
        .rect(rect, CornerRadius::same(7), rail_bg, Stroke::new(stroke_w, border_col), StrokeKind::Inside);

    let inner = match style.orientation {
        FilmbarOrientation::Vertical => {
            let rail_w = 10.0;
            let mut y = rect.min.y + 6.0;
            while y + 5.0 < rect.max.y - 4.0 {
                let left_hole = Rect::from_min_size(Pos2::new(rect.min.x + 2.5, y), Vec2::new(5.0, 4.0));
                let right_hole = Rect::from_min_size(Pos2::new(rect.max.x - 7.5, y), Vec2::new(5.0, 4.0));
                ui.painter().rect_filled(left_hole, CornerRadius::same(1), hole_col);
                ui.painter().rect_filled(right_hole, CornerRadius::same(1), hole_col);
                y += 8.5;
            }
            Rect::from_min_max(
                Pos2::new(rect.min.x + rail_w, rect.min.y + 2.0),
                Pos2::new(rect.max.x - rail_w, rect.max.y - 2.0),
            )
        }
        FilmbarOrientation::Horizontal => {
            let rail_h = 9.0;
            let mut x = rect.min.x + 6.0;
            while x + 5.0 < rect.max.x - 4.0 {
                let top_hole = Rect::from_min_size(Pos2::new(x, rect.min.y + 2.0), Vec2::new(5.0, 4.0));
                let bot_hole = Rect::from_min_size(Pos2::new(x, rect.max.y - 6.0), Vec2::new(5.0, 4.0));
                ui.painter().rect_filled(top_hole, CornerRadius::same(1), hole_col);
                ui.painter().rect_filled(bot_hole, CornerRadius::same(1), hole_col);
                x += 9.0;
            }
            Rect::from_min_max(
                Pos2::new(rect.min.x + 2.0, rect.min.y + rail_h),
                Pos2::new(rect.max.x - 2.0, rect.max.y - rail_h),
            )
        }
    };

    ui.painter().rect_filled(inner, CornerRadius::same(4), pal.bg_stage);
    if let Some(tc) = style.timecode {
        paint_timecode_pill(ui, inner, pal, tc);
    }
    inner
}

/// Paint a translucent monospace timecode pill in the bottom-right corner of `inner`.
pub fn paint_timecode_pill(ui: &mut Ui, inner: Rect, pal: &MagicPalette, tc: &str) {
    let galley = ui
        .painter()
        .layout_no_wrap(tc.to_owned(), FontId::monospace(9.5), pal.text_primary);
    let badge = Rect::from_min_size(
        Pos2::new(inner.max.x - galley.size().x - 12.0, inner.max.y - 17.0),
        Vec2::new(galley.size().x + 8.0, 14.0),
    );
    ui.painter()
        .rect_filled(badge, CornerRadius::same(4), Color32::from_rgba_unmultiplied(10, 14, 21, 220));
    ui.painter()
        .text(badge.center(), Align2::CENTER_CENTER, tc, FontId::monospace(9.5), pal.text_primary);
}

fn format_inline_value(val: f64, suffix: &str) -> String {
    if (val - val.round()).abs() < 0.05 {
        format!("{:.0}{suffix}", val.round())
    } else {
        format!("{val:.2}{suffix}")
    }
}

/// Horizontal hollow-ring slider with small muted label on left and small inline monospace value on right.
pub fn ring_slider_h(
    ui: &mut Ui,
    reg: &mut AutomationRegistry,
    pal: &MagicPalette,
    spec: ScalarSpec<'_>,
    value: &mut f64,
) -> Response {
    let lo = *spec.range.start();
    let hi = (*spec.range.end()).max(lo + 1e-6);
    if let Some(raw) = reg.take_pending_value(spec.id)
        && let Ok(parsed) = raw.parse::<f64>()
    {
        *value = parsed.clamp(lo, hi);
    }
    let width = ui.available_width().max(96.0);
    let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(width, 28.0), Sense::click_and_drag());
    let track_left = rect.min.x + 7.0;
    let track_right = (rect.max.x - 7.0).max(track_left + 12.0);
    let track_y = rect.max.y - 7.5;

    if (resp.dragged() || resp.clicked())
        && let Some(pos) = resp.interact_pointer_pos()
    {
        let t = ((pos.x - track_left) / (track_right - track_left)).clamp(0.0, 1.0) as f64;
        let next = lo + t * (hi - lo);
        if (next - *value).abs() > 1e-6 {
            *value = next;
            resp.mark_changed();
        }
    }

    let val_text = format_inline_value(*value, spec.suffix);
    ui.painter().text(
        Pos2::new(rect.min.x + 1.0, rect.min.y + 6.5),
        Align2::LEFT_CENTER,
        spec.label,
        FontId::proportional(10.5),
        pal.text_muted,
    );
    ui.painter().text(
        Pos2::new(rect.max.x - 1.0, rect.min.y + 6.5),
        Align2::RIGHT_CENTER,
        &val_text,
        FontId::monospace(10.5),
        spec.accent,
    );

    let frac = ((*value - lo) / (hi - lo)).clamp(0.0, 1.0) as f32;
    let thumb_x = track_left + frac * (track_right - track_left);
    let full_track = Rect::from_min_max(Pos2::new(track_left, track_y - 2.0), Pos2::new(track_right, track_y + 2.0));
    ui.painter().rect_filled(full_track, CornerRadius::same(2), pal.border);
    if thumb_x > track_left + 1.0 {
        let active_track = Rect::from_min_max(Pos2::new(track_left, track_y - 2.0), Pos2::new(thumb_x, track_y + 2.0));
        ui.painter().rect_filled(active_track, CornerRadius::same(2), spec.accent);
    }
    let thumb_center = Pos2::new(thumb_x, track_y);
    let ring_r = if resp.hovered() || resp.dragged() { 6.6 } else { 5.8 };
    ui.painter().circle_filled(thumb_center, ring_r, pal.bg_input);
    ui.painter().circle_stroke(thumb_center, ring_r, Stroke::new(2.0, spec.accent));

    reg.record(spec.id, spec.label, &resp, Some(format!("{:.2}", *value)));
    resp
}

/// Vertical hollow-ring slider (`Point Y  310px`) with small inline label & monospace readout above.
pub fn ring_slider_v(
    ui: &mut Ui,
    reg: &mut AutomationRegistry,
    pal: &MagicPalette,
    spec: ScalarSpec<'_>,
    value: &mut f64,
    height: f32,
) -> Response {
    let lo = *spec.range.start();
    let hi = (*spec.range.end()).max(lo + 1e-6);
    if let Some(raw) = reg.take_pending_value(spec.id)
        && let Ok(parsed) = raw.parse::<f64>()
    {
        *value = parsed.clamp(lo, hi);
    }
    let width = ui.available_width().clamp(84.0, 136.0);
    let total_h = height.max(68.0);
    let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(width, total_h), Sense::click_and_drag());
    let track_top = rect.min.y + 19.0;
    let track_bot = (rect.max.y - 7.0).max(track_top + 18.0);
    let track_x = rect.center().x;

    if (resp.dragged() || resp.clicked())
        && let Some(pos) = resp.interact_pointer_pos()
    {
        let t = (1.0 - (pos.y - track_top) / (track_bot - track_top)).clamp(0.0, 1.0) as f64;
        let next = lo + t * (hi - lo);
        if (next - *value).abs() > 1e-6 {
            *value = next;
            resp.mark_changed();
        }
    }

    let val_text = format_inline_value(*value, spec.suffix);
    ui.painter().text(
        Pos2::new(rect.min.x + 2.0, rect.min.y + 6.5),
        Align2::LEFT_CENTER,
        spec.label,
        FontId::proportional(10.5),
        pal.text_muted,
    );
    ui.painter().text(
        Pos2::new(rect.max.x - 2.0, rect.min.y + 6.5),
        Align2::RIGHT_CENTER,
        &val_text,
        FontId::monospace(10.5),
        spec.accent,
    );

    let frac = ((*value - lo) / (hi - lo)).clamp(0.0, 1.0) as f32;
    let thumb_y = track_bot - frac * (track_bot - track_top);
    let full_track = Rect::from_min_max(Pos2::new(track_x - 2.0, track_top), Pos2::new(track_x + 2.0, track_bot));
    ui.painter().rect_filled(full_track, CornerRadius::same(2), pal.border);
    if track_bot > thumb_y + 1.0 {
        let active_track = Rect::from_min_max(Pos2::new(track_x - 2.0, thumb_y), Pos2::new(track_x + 2.0, track_bot));
        ui.painter().rect_filled(active_track, CornerRadius::same(2), spec.accent);
    }
    let thumb_center = Pos2::new(track_x, thumb_y);
    let ring_r = if resp.hovered() || resp.dragged() { 6.6 } else { 5.8 };
    ui.painter().circle_filled(thumb_center, ring_r, pal.bg_input);
    ui.painter().circle_stroke(thumb_center, ring_r, Stroke::new(2.0, spec.accent));

    reg.record(spec.id, spec.label, &resp, Some(format!("{:.2}", *value)));
    resp
}

/// Paired inset `Point X` (horizontal) + `Point Y` (vertical) coordinate slider box (`Path & depth`).
pub fn xy_path_pad(
    ui: &mut Ui,
    reg: &mut AutomationRegistry,
    pal: &MagicPalette,
    spec_x: ScalarSpec<'_>,
    x: &mut f64,
    spec_y: ScalarSpec<'_>,
    y: &mut f64,
) -> bool {
    let mut changed = false;
    pal.inset_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            let left_w = (ui.available_width() * 0.55).max(92.0);
            ui.allocate_ui(Vec2::new(left_w, 72.0), |ui| {
                ui.add_space(20.0);
                if ring_slider_h(ui, reg, pal, spec_x, x).changed() {
                    changed = true;
                }
            });
            ui.add_space(4.0);
            if ring_slider_v(ui, reg, pal, spec_y, y, 72.0).changed() {
                changed = true;
            }
        });
    });
    changed
}

/// Circular 360° rotary dial with radial tick mark and compact inline value readout.
pub fn rotary_dial(
    ui: &mut Ui,
    reg: &mut AutomationRegistry,
    pal: &MagicPalette,
    spec: ScalarSpec<'_>,
    value: &mut f64,
) -> Response {
    let lo = *spec.range.start();
    let hi = (*spec.range.end()).max(lo + 1e-6);
    if let Some(raw) = reg.take_pending_value(spec.id)
        && let Ok(parsed) = raw.parse::<f64>()
    {
        *value = parsed.clamp(lo, hi);
    }
    let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(76.0, 72.0), Sense::click_and_drag());
    if resp.double_clicked() {
        *value = 0.0_f64.clamp(lo, hi);
        resp.mark_changed();
    } else if resp.dragged() {
        let delta = (-resp.drag_delta().y + resp.drag_delta().x) as f64;
        let step = (hi - lo) / 220.0;
        *value = (*value + delta * step).clamp(lo, hi);
        resp.mark_changed();
    }

    ui.painter().text(
        Pos2::new(rect.center().x, rect.min.y + 7.0),
        Align2::CENTER_CENTER,
        spec.label,
        FontId::proportional(10.0),
        pal.text_muted,
    );
    let center = Pos2::new(rect.center().x, rect.min.y + 35.0);
    let r = 17.0;
    ui.painter().circle_filled(center, r, pal.bg_input);
    let rim = if resp.hovered() || resp.dragged() { spec.accent } else { pal.border };
    ui.painter().circle_stroke(center, r, Stroke::new(1.5, rim));

    for tick_idx in 0..12 {
        let a = (tick_idx as f32) * std::f32::consts::TAU / 12.0;
        let p0 = Pos2::new(center.x + a.sin() * (r + 2.0), center.y - a.cos() * (r + 2.0));
        let p1 = Pos2::new(center.x + a.sin() * (r + 4.2), center.y - a.cos() * (r + 4.2));
        ui.painter().line_segment([p0, p1], Stroke::new(1.0, pal.border_subtle));
    }

    let angle_rad = if spec.suffix == "°" {
        (*value as f32).to_radians()
    } else {
        let norm = ((*value - lo) / (hi - lo)).clamp(0.0, 1.0) as f32;
        (-135.0_f32 + norm * 270.0).to_radians()
    };
    let tip = Pos2::new(center.x + angle_rad.sin() * (r - 4.0), center.y - angle_rad.cos() * (r - 4.0));
    ui.painter().line_segment([center, tip], Stroke::new(2.0, spec.accent));
    ui.painter().circle_filled(center, 2.2, spec.accent);

    let val_str = format!("{:.0}{}", value.round(), spec.suffix);
    ui.painter().text(
        Pos2::new(rect.center().x, rect.max.y - 6.0),
        Align2::CENTER_CENTER,
        &val_str,
        FontId::monospace(10.0),
        spec.accent,
    );
    reg.record(spec.id, spec.label, &resp, Some(format!("{:.1}", *value)));
    resp
}

/// Compact capsule toggle switch with smaller inline text label.
pub fn pill_toggle(
    ui: &mut Ui,
    reg: &mut AutomationRegistry,
    pal: &MagicPalette,
    id: &str,
    label: &str,
    checked: &mut bool,
    accent: Color32,
) -> Response {
    if let Some(next) = reg.take_pending_checked(id) {
        *checked = next;
    }
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), FontId::proportional(11.0), pal.text_secondary);
    let width = (36.0 + galley.size().x + 6.0).min(ui.available_width().max(80.0));
    let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(width, 22.0), Sense::click());
    if resp.clicked() {
        *checked = !*checked;
        resp.mark_changed();
    }
    let pill = Rect::from_min_size(Pos2::new(rect.min.x, rect.center().y - 7.5), Vec2::new(28.0, 15.0));
    let fill = if *checked { accent.gamma_multiply(0.28) } else { pal.bg_input };
    let border = if *checked { accent } else { pal.border };
    ui.painter()
        .rect(pill, CornerRadius::same(8), fill, Stroke::new(1.0, border), StrokeKind::Inside);
    let knob_x = if *checked { pill.max.x - 7.5 } else { pill.min.x + 7.5 };
    let knob_col = if *checked { accent } else { pal.text_muted };
    ui.painter().circle_filled(Pos2::new(knob_x, pill.center().y), 5.0, knob_col);
    ui.painter().text(
        Pos2::new(pill.max.x + 7.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(11.0),
        if *checked { pal.text_primary } else { pal.text_secondary },
    );
    reg.record(id, label, &resp, Some(checked.to_string()));
    resp
}

/// Colorful rounded-square ArtCraft sibling launcher swatch (`PC`, `VC`, `DC`, `EC`, `FC`).
pub fn craft_swatch_button(
    ui: &mut Ui,
    reg: &mut AutomationRegistry,
    id: &str,
    code: &str,
    fill: Color32,
    tooltip: &str,
) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::click());
    let bg = if resp.hovered() { fill.gamma_multiply(1.12) } else { fill };
    ui.painter().rect(
        rect,
        CornerRadius::same(6),
        bg,
        Stroke::new(
            1.0,
            Color32::from_rgba_unmultiplied(255, 255, 255, if resp.hovered() { 110 } else { 42 }),
        ),
        StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        code,
        FontId::proportional(10.5),
        Color32::WHITE,
    );
    let resp = resp.on_hover_text(tooltip);
    reg.record(id, tooltip, &resp, None);
    resp
}

/// Color-coded ArtCraft badge color (`EC` violet, `VC` amber, `PC` sky, `FC` emerald, `DC` pink).
pub fn badge_accent_color(pal: &MagicPalette, badge: &str) -> Color32 {
    match badge.to_ascii_uppercase().trim_start_matches('.') {
        "EC" | "ECPROJ" | "ANICRAFT" => pal.accent_violet,
        "VC" | "VECTORCRAFT" | "SVG" => pal.accent_amber,
        "PC" | "PCRAFT" | "PSD" | "PNG" | "JPG" | "WEBP" => pal.accent_sky,
        "FC" | "FCPROJ" | "MP4" | "MOV" => pal.accent_emerald,
        "DC" | "DESIGNCRAFT" | "PDF" => Color32::from_rgb(0xec, 0x48, 0x99),
        _ => pal.text_muted,
    }
}

/// Full-width selectable Craft effect row (`EC` / `VC` / `PC` chip + effect title + subtle CLI effect ID).
pub fn effect_row_button(
    ui: &mut Ui,
    reg: &mut AutomationRegistry,
    pal: &MagicPalette,
    spec: EffectRowSpec<'_>,
) -> Response {
    let w = ui.available_width().max(180.0);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 26.0), Sense::click());
    let fill = if spec.selected {
        pal.accent_emerald_bg
    } else if resp.hovered() {
        pal.bg_hover
    } else {
        pal.bg_input
    };
    let border = if spec.selected {
        pal.accent_emerald
    } else if resp.hovered() {
        pal.border
    } else {
        pal.border_subtle
    };
    ui.painter()
        .rect(rect, CornerRadius::same(6), fill, Stroke::new(1.0, border), StrokeKind::Inside);

    let bcol = badge_accent_color(pal, spec.badge);
    let chip = Rect::from_min_size(Pos2::new(rect.min.x + 5.0, rect.center().y - 8.0), Vec2::new(24.0, 16.0));
    ui.painter().rect(
        chip,
        CornerRadius::same(4),
        bcol.gamma_multiply(0.22),
        Stroke::new(1.0, bcol.gamma_multiply(0.75)),
        StrokeKind::Inside,
    );
    ui.painter()
        .text(chip.center(), Align2::CENTER_CENTER, spec.badge, FontId::monospace(9.5), bcol);

    ui.painter().text(
        Pos2::new(chip.max.x + 7.0, rect.center().y),
        Align2::LEFT_CENTER,
        spec.label,
        FontId::proportional(11.5),
        if spec.selected { pal.accent_emerald } else { pal.text_primary },
    );
    let short_cli = spec.effect_id.rsplit('.').next().unwrap_or(spec.effect_id);
    ui.painter().text(
        Pos2::new(rect.max.x - 6.0, rect.center().y),
        Align2::RIGHT_CENTER,
        short_cli,
        FontId::monospace(9.0),
        pal.text_muted,
    );
    let resp = resp.on_hover_text(format!("{} ({})", spec.label, spec.effect_id));
    reg.record(spec.id, spec.label, &resp, Some(spec.effect_id.to_owned()));
    resp
}

/// Clickable Storyboard requirement row showing a glowing Green (`filled`) or Red (`missing`) light.
pub fn requirement_light_row(
    ui: &mut Ui,
    reg: &mut AutomationRegistry,
    pal: &MagicPalette,
    row: RequirementRow<'_>,
) -> Response {
    let w = ui.available_width().max(180.0);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 28.0), Sense::click());
    let bg = if row.selected {
        pal.accent_emerald_bg
    } else if resp.hovered() {
        pal.bg_hover
    } else {
        pal.bg_input
    };
    let border = if row.selected { pal.accent_emerald } else { pal.border_subtle };
    ui.painter()
        .rect(rect, CornerRadius::same(6), bg, Stroke::new(1.0, border), StrokeKind::Inside);

    let light_col = if row.filled { pal.accent_emerald } else { pal.danger };
    let dot_center = Pos2::new(rect.min.x + 12.0, rect.center().y);
    ui.painter().circle_filled(dot_center, 6.0, light_col.gamma_multiply(0.22));
    if row.filled {
        ui.painter().circle_filled(dot_center, 4.0, light_col);
    } else {
        ui.painter().circle_stroke(dot_center, 4.0, Stroke::new(1.6, light_col));
    }
    ui.painter().text(
        Pos2::new(rect.min.x + 24.0, rect.center().y),
        Align2::LEFT_CENTER,
        row.title,
        FontId::proportional(11.5),
        pal.text_primary,
    );
    ui.painter().text(
        Pos2::new(rect.max.x - 8.0, rect.center().y),
        Align2::RIGHT_CENTER,
        row.detail,
        FontId::monospace(9.5),
        light_col,
    );
    reg.record(row.id, row.title, &resp, Some(if row.filled { "filled" } else { "missing" }.into()));
    resp
}
