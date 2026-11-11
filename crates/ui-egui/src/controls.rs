//! Signature GPU-painted interactive Magic Studio controls (`egui::Painter`) matching
//! `00-figma-component-system-sheet.jpg` (`e142e6c2.jpeg`):
//! - `ring_slider_h` & `ring_slider_v`: 4px rounded track, hollow accent ring thumb, and warm gold
//!   `#EABB62` monospace value readouts (`2.0s`, `56px`, `310px`).
//! - `xy_path_pad`: true 2D gridded coordinate pad with fine left/bottom ruler ticks, intersecting
//!   crosshairs, and paired `Point X` (horizontal) + `Point Y` (vertical) hollow-ring sliders.
//! - `rotary_dial`: 37-tick radial crown, tapered pointer needle, and warm `#EABB62` readout.
//! - `pill_toggle`: label-left, stadium switch-right (`36x18px`) with solid mint fill when active.
//! - `craft_swatch_button`, `effect_row_button`, & `requirement_light_row`.

use egui::{
    Align2, Color32, CornerRadius, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind,
    Ui, Vec2,
};

pub use crate::filmbar::{
    FilmbarOrientation, FilmbarStyle, child_pane, fill_card, local_image_uri, paint_craft_badge,
    paint_craft_doc_card, paint_cubic_bezier, paint_dashed_rect, paint_filmbar_frame,
    paint_local_image, paint_readiness_light, paint_svg_path_preview, paint_timecode_pill,
};
use crate::theme::MagicPalette;
use crate::widgets::AutomationRegistry;

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

fn format_inline_value(val: f64, suffix: &str) -> String {
    if suffix == "s" && (val - val.round()).abs() < 0.05 {
        format!("{val:.1}{suffix}")
    } else if (val - val.round()).abs() < 0.05 {
        format!("{:.0}{suffix}", val.round())
    } else {
        format!("{val:.2}{suffix}")
    }
}

/// Horizontal hollow-ring slider (`3. Precision Sliders` in `00-figma-component-system-sheet.jpg`).
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
    let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(width, 30.0), Sense::click_and_drag());
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
        Pos2::new(rect.min.x + 1.0, rect.min.y + 7.0),
        Align2::LEFT_CENTER,
        spec.label,
        FontId::proportional(11.0),
        pal.text_secondary,
    );
    ui.painter().text(
        Pos2::new(rect.max.x - 1.0, rect.min.y + 7.0),
        Align2::RIGHT_CENTER,
        &val_text,
        FontId::monospace(11.0),
        pal.accent_amber,
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
    let ring_r = if resp.hovered() || resp.dragged() { 6.8 } else { 6.0 };
    ui.painter().circle_filled(thumb_center, ring_r, pal.bg_input);
    ui.painter().circle_stroke(thumb_center, ring_r, Stroke::new(2.1, spec.accent));

    reg.record(spec.id, spec.label, &resp, Some(format!("{:.2}", *value)));
    resp
}

/// Vertical hollow-ring slider (`Point Y` top label + warm gold `310px` bottom value readout).
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
    let width = ui.available_width().clamp(58.0, 108.0);
    let total_h = height.max(84.0);
    let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(width, total_h), Sense::click_and_drag());
    let track_top = rect.min.y + 18.0;
    let track_bot = (rect.max.y - 18.0).max(track_top + 20.0);
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

    ui.painter().text(
        Pos2::new(track_x, rect.min.y + 6.5),
        Align2::CENTER_CENTER,
        spec.label,
        FontId::proportional(10.5),
        pal.text_secondary,
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
    let ring_r = if resp.hovered() || resp.dragged() { 6.8 } else { 6.0 };
    ui.painter().circle_filled(thumb_center, ring_r, pal.bg_input);
    ui.painter().circle_stroke(thumb_center, ring_r, Stroke::new(2.1, spec.accent));

    let val_text = format_inline_value(*value, spec.suffix);
    ui.painter().text(
        Pos2::new(track_x, rect.max.y - 6.5),
        Align2::CENTER_CENTER,
        &val_text,
        FontId::monospace(10.5),
        pal.accent_amber,
    );
    reg.record(spec.id, spec.label, &resp, Some(format!("{:.2}", *value)));
    resp
}

/// True 2D intersecting coordinate pad (`3. Precision Sliders & XY Pad` in `00-figma-component-system-sheet.jpg`).
pub fn xy_path_pad(
    ui: &mut Ui,
    reg: &mut AutomationRegistry,
    pal: &MagicPalette,
    spec_x: ScalarSpec<'_>,
    x: &mut f64,
    spec_y: ScalarSpec<'_>,
    y: &mut f64,
) -> bool {
    let lo_x = *spec_x.range.start();
    let hi_x = (*spec_x.range.end()).max(lo_x + 1e-6);
    let lo_y = *spec_y.range.start();
    let hi_y = (*spec_y.range.end()).max(lo_y + 1e-6);
    let mut changed = false;

    if let Some(raw_x) = reg.take_pending_value(spec_x.id)
        && let Ok(px) = raw_x.parse::<f64>()
    {
        *x = px.clamp(lo_x, hi_x);
        changed = true;
    }
    if let Some(raw_y) = reg.take_pending_value(spec_y.id)
        && let Ok(py) = raw_y.parse::<f64>()
    {
        *y = py.clamp(lo_y, hi_y);
        changed = true;
    }

    let w = ui.available_width().max(180.0);
    let (pad, resp) = ui.allocate_exact_size(Vec2::new(w, 128.0), Sense::click_and_drag());
    ui.painter().rect(
        pad,
        CornerRadius::same(8),
        Color32::from_rgb(0x0b, 0x0f, 0x16),
        Stroke::new(1.0, pal.border),
        StrokeKind::Inside,
    );

    let plot = Rect::from_min_max(
        Pos2::new(pad.min.x + 12.0, pad.min.y + 24.0),
        Pos2::new(pad.max.x - 12.0, pad.max.y - 12.0),
    );
    let grid_stroke = Stroke::new(1.0, Color32::from_rgba_unmultiplied(44, 56, 76, 85));
    for g in 1..4 {
        let gf = (g as f32) * 0.25;
        let gx = plot.min.x + gf * plot.width();
        let gy = plot.min.y + gf * plot.height();
        ui.painter().line_segment([Pos2::new(gx, plot.min.y), Pos2::new(gx, plot.max.y)], grid_stroke);
        ui.painter().line_segment([Pos2::new(plot.min.x, gy), Pos2::new(plot.max.x, gy)], grid_stroke);
    }
    let tick_stroke = Stroke::new(1.0, pal.border);
    let mut tx = plot.min.x;
    while tx <= plot.max.x {
        ui.painter().line_segment([Pos2::new(tx, pad.max.y - 6.0), Pos2::new(tx, pad.max.y - 2.0)], tick_stroke);
        tx += 6.0;
    }
    let mut ty = plot.min.y;
    while ty <= plot.max.y {
        ui.painter().line_segment([Pos2::new(pad.min.x + 2.0, ty), Pos2::new(pad.min.x + 6.0, ty)], tick_stroke);
        ty += 6.0;
    }

    if (resp.dragged() || resp.clicked())
        && let Some(pos) = resp.interact_pointer_pos()
    {
        let fx = ((pos.x - plot.min.x) / plot.width().max(1.0)).clamp(0.0, 1.0) as f64;
        let fy = (1.0 - (pos.y - plot.min.y) / plot.height().max(1.0)).clamp(0.0, 1.0) as f64;
        *x = lo_x + fx * (hi_x - lo_x);
        *y = lo_y + fy * (hi_y - lo_y);
        changed = true;
    }

    let fx = ((*x - lo_x) / (hi_x - lo_x)).clamp(0.0, 1.0) as f32;
    let fy = ((*y - lo_y) / (hi_y - lo_y)).clamp(0.0, 1.0) as f32;
    let y_bar_x = (plot.max.x - 24.0).max(plot.min.x + plot.width() * 0.72);
    let x_span_max = (y_bar_x - 14.0).max(plot.min.x + 36.0);
    let thumb_x = plot.min.x + fx * (x_span_max - plot.min.x);
    let thumb_y = plot.max.y - fy * plot.height();
    let h_line_y = plot.center().y;

    ui.painter().text(
        Pos2::new(y_bar_x - 36.0, pad.min.y + 11.0),
        Align2::RIGHT_CENTER,
        spec_y.label,
        FontId::proportional(10.5),
        pal.text_secondary,
    );
    ui.painter().text(
        Pos2::new(pad.max.x - 10.0, pad.min.y + 11.0),
        Align2::RIGHT_CENTER,
        format_inline_value(*y, spec_y.suffix),
        FontId::monospace(10.5),
        pal.accent_amber,
    );

    ui.painter().text(
        Pos2::new(plot.min.x, h_line_y - 13.0),
        Align2::LEFT_CENTER,
        spec_x.label,
        FontId::proportional(10.5),
        pal.text_secondary,
    );
    ui.painter().text(
        Pos2::new(x_span_max, h_line_y - 13.0),
        Align2::RIGHT_CENTER,
        format_inline_value(*x, spec_x.suffix),
        FontId::monospace(10.5),
        pal.accent_amber,
    );
    ui.painter().line_segment([Pos2::new(plot.min.x, h_line_y), Pos2::new(plot.max.x, h_line_y)], Stroke::new(1.5, pal.border));
    ui.painter().line_segment([Pos2::new(plot.min.x, h_line_y), Pos2::new(thumb_x, h_line_y)], Stroke::new(3.2, spec_x.accent));
    ui.painter().circle_filled(Pos2::new(thumb_x, h_line_y), 5.8, Color32::from_rgb(0x0b, 0x0f, 0x16));
    ui.painter().circle_stroke(Pos2::new(thumb_x, h_line_y), 5.8, Stroke::new(2.0, spec_x.accent));

    ui.painter().line_segment([Pos2::new(y_bar_x, plot.min.y), Pos2::new(y_bar_x, plot.max.y)], Stroke::new(2.5, pal.border));
    ui.painter().line_segment([Pos2::new(y_bar_x, thumb_y), Pos2::new(y_bar_x, plot.max.y)], Stroke::new(3.2, spec_y.accent));
    ui.painter().circle_filled(Pos2::new(y_bar_x, thumb_y), 5.8, Color32::from_rgb(0x0b, 0x0f, 0x16));
    ui.painter().circle_stroke(Pos2::new(y_bar_x, thumb_y), 5.8, Stroke::new(2.0, spec_y.accent));

    reg.record(spec_x.id, spec_x.label, &resp, Some(format!("{:.2}", *x)));
    reg.record(spec_y.id, spec_y.label, &resp, Some(format!("{:.2}", *y)));
    changed
}

/// Circular 360° rotary dial (`4. 360° Rotary Dials` in `00-figma-component-system-sheet.jpg`).
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
    let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(82.0, 82.0), Sense::click_and_drag());
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
        Pos2::new(rect.center().x, rect.min.y + 7.5),
        Align2::CENTER_CENTER,
        spec.label,
        FontId::proportional(10.5),
        pal.text_secondary,
    );
    let center = Pos2::new(rect.center().x, rect.min.y + 41.0);
    let r = 19.5;
    ui.painter().circle_filled(center, r, Color32::from_rgb(0x16, 0x1e, 0x2b));
    let rim = if resp.hovered() || resp.dragged() { spec.accent } else { pal.border };
    ui.painter().circle_stroke(center, r, Stroke::new(1.4, rim));

    let norm = ((*value - lo) / (hi - lo)).clamp(0.0, 1.0) as f32;
    for tick_idx in 0..=36 {
        let tf = (tick_idx as f32) / 36.0;
        let a = (-135.0_f32 + tf * 270.0).to_radians();
        let outer = if tick_idx % 3 == 0 { r + 6.4 } else { r + 4.8 };
        let p0 = Pos2::new(center.x + a.sin() * (r + 2.2), center.y - a.cos() * (r + 2.2));
        let p1 = Pos2::new(center.x + a.sin() * outer, center.y - a.cos() * outer);
        let lit = if spec.suffix == "°" && lo < 0.0 {
            let zero_f = ((0.0 - lo) / (hi - lo)) as f32;
            (tf >= norm.min(zero_f)) && (tf <= norm.max(zero_f))
        } else {
            tf <= norm
        };
        let tcol = if lit { pal.accent_amber } else { pal.border };
        ui.painter().line_segment([p0, p1], Stroke::new(1.0, tcol));
    }

    let angle_rad = if spec.suffix == "°" {
        (*value as f32).to_radians()
    } else {
        (-135.0_f32 + norm * 270.0).to_radians()
    };
    let dir = Vec2::new(angle_rad.sin(), -angle_rad.cos());
    let perp = Vec2::new(-dir.y, dir.x);
    let base = center + dir * 2.5;
    let tip = center + dir * (r - 2.8);
    ui.painter().add(Shape::convex_polygon(
        vec![base + perp * 1.8, tip, base - perp * 1.8],
        Color32::from_rgb(0xf0, 0xc9, 0x78),
        Stroke::NONE,
    ));
    ui.painter().circle_filled(center, 2.2, Color32::from_rgb(0xf0, 0xc9, 0x78));

    let val_str = format!("{:.0}{}", value.round(), spec.suffix);
    ui.painter().text(
        Pos2::new(rect.center().x, rect.max.y - 6.0),
        Align2::CENTER_CENTER,
        &val_str,
        FontId::monospace(10.5),
        pal.accent_amber,
    );
    reg.record(spec.id, spec.label, &resp, Some(format!("{:.1}", *value)));
    resp
}

/// Stadium capsule toggle switch (`4. Pill Toggles` in `00-figma-component-system-sheet.jpg`).
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
        .layout_no_wrap(label.to_owned(), FontId::proportional(11.5), pal.text_secondary);
    let width = (galley.size().x + 48.0).min(ui.available_width().max(84.0));
    let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(width, 22.0), Sense::click());
    if resp.clicked() {
        *checked = !*checked;
        resp.mark_changed();
    }
    ui.painter().text(
        Pos2::new(rect.min.x + 1.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(11.5),
        if *checked { pal.text_primary } else { pal.text_secondary },
    );

    let pill = Rect::from_min_size(Pos2::new(rect.max.x - 38.0, rect.center().y - 9.0), Vec2::new(36.0, 18.0));
    let fill = if *checked { accent } else { pal.bg_input };
    let border = if *checked { accent } else { pal.border };
    ui.painter()
        .rect(pill, CornerRadius::same(9), fill, Stroke::new(1.0, border), StrokeKind::Inside);
    let knob_x = if *checked { pill.max.x - 9.0 } else { pill.min.x + 9.0 };
    let knob_col = if *checked {
        Color32::from_rgb(0x0c, 0x16, 0x1a)
    } else {
        Color32::from_rgb(0x3a, 0x46, 0x59)
    };
    ui.painter().circle_filled(Pos2::new(knob_x, pill.center().y), 6.2, knob_col);
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
        "VC" | "VCRAFT" | "VECTORCRAFT" | "SVG" => pal.accent_amber,
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

/// Clickable Storyboard requirement row showing a 3-ring glowing Green (`filled`) or Red (`missing`) light + Craft badge pill.
pub fn requirement_light_row(
    ui: &mut Ui,
    reg: &mut AutomationRegistry,
    pal: &MagicPalette,
    row: RequirementRow<'_>,
) -> Response {
    let w = ui.available_width().max(180.0);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 30.0), Sense::click());
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
    paint_readiness_light(ui, Pos2::new(rect.min.x + 13.0, rect.center().y), light_col);

    ui.painter().text(
        Pos2::new(rect.min.x + 26.0, rect.center().y),
        Align2::LEFT_CENTER,
        row.title,
        FontId::proportional(11.5),
        pal.text_primary,
    );

    let pill_w = (row.detail.len() as f32 * 6.2 + 12.0).max(46.0);
    let pill_r = Rect::from_min_size(
        Pos2::new(rect.max.x - pill_w - 6.0, rect.center().y - 8.5),
        Vec2::new(pill_w, 17.0),
    );
    ui.painter().rect(
        pill_r,
        CornerRadius::same(5),
        light_col.gamma_multiply(0.16),
        Stroke::new(1.0, light_col.gamma_multiply(0.58)),
        StrokeKind::Inside,
    );
    ui.painter()
        .text(pill_r.center(), Align2::CENTER_CENTER, row.detail, FontId::monospace(9.2), light_col);
    reg.record(row.id, row.title, &resp, Some(if row.filled { "filled" } else { "missing" }.into()));
    resp
}
