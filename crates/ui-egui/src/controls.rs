//! Signature GPU-painted Magic Studio controls (`egui::Painter`):
//! - `paint_filmbar_frame`: 35mm cinema sprocket border (`Vertical` left/right rails or `Horizontal` top/bottom rails).
//! - `ring_slider_h` & `ring_slider_v`: horizontal and vertical hollow-ring sliders with smaller inline monospace values.
//! - `xy_path_pad`: paired inset `Point X` (horizontal) + `Point Y` (vertical) coordinate pad (`Path & depth`).
//! - `rotary_dial`: 360° circular angle dial with radial pointer tick and compact inline degree readout.
//! - `pill_toggle`: compact capsule toggle switch with smaller inline text label.
//! - `craft_swatch_button` & `requirement_light_row`: ArtCraft quick-launch swatches and Storyboard traffic-light rows.

use egui::{Align2, Color32, CornerRadius, FontId, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2};

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
    pub const fn new(id: &'a str, label: &'a str, range: std::ops::RangeInclusive<f64>, suffix: &'a str, accent: Color32) -> Self {
        Self { id, label, range, suffix, accent }
    }
}

pub struct RequirementRow<'a> {
    pub id: &'a str,
    pub title: &'a str,
    pub detail: &'a str,
    pub filled: bool,
    pub selected: bool,
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
        pal.accent_emerald.gamma_multiply(0.78)
    } else if style.hovered {
        pal.accent_sky.gamma_multiply(0.55)
    } else {
        pal.border
    };
    let stroke_w = if style.selected { 1.6 } else { 1.0 };
    ui.painter().rect(rect, CornerRadius::same(7), rail_bg, Stroke::new(stroke_w, border_col), StrokeKind::Inside);

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
            Rect::from_min_max(Pos2::new(rect.min.x + rail_w, rect.min.y + 2.0), Pos2::new(rect.max.x - rail_w, rect.max.y - 2.0))
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
            Rect::from_min_max(Pos2::new(rect.min.x + 2.0, rect.min.y + rail_h), Pos2::new(rect.max.x - 2.0, rect.max.y - rail_h))
        }
    };

    ui.painter().rect_filled(inner, CornerRadius::same(4), pal.bg_stage);
    if let Some(tc) = style.timecode {
        let galley = ui.painter().layout_no_wrap(tc.to_owned(), FontId::monospace(9.5), pal.text_primary);
        let badge = Rect::from_min_size(Pos2::new(inner.max.x - galley.size().x - 12.0, inner.max.y - 17.0), Vec2::new(galley.size().x + 8.0, 14.0));
        ui.painter().rect_filled(badge, CornerRadius::same(4), Color32::from_rgba_unmultiplied(10, 14, 21, 215));
        ui.painter().text(badge.center(), Align2::CENTER_CENTER, tc, FontId::monospace(9.5), pal.text_primary);
    }
    inner
}

fn format_inline_value(val: f64, suffix: &str) -> String {
    if (val - val.round()).abs() < 0.05 { format!("{:.0}{suffix}", val.round()) } else { format!("{val:.2}{suffix}") }
}

/// Horizontal hollow-ring slider with small muted label on left and small inline monospace value on right.
pub fn ring_slider_h(ui: &mut Ui, reg: &mut AutomationRegistry, pal: &MagicPalette, spec: ScalarSpec<'_>, value: &mut f64) -> Response {
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
    let track_y = rect.max.y - 8.0;

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
    ui.painter().text(Pos2::new(rect.min.x + 1.0, rect.min.y + 7.0), Align2::LEFT_CENTER, spec.label, FontId::proportional(10.5), pal.text_muted);
    ui.painter().text(Pos2::new(rect.max.x - 1.0, rect.min.y + 7.0), Align2::RIGHT_CENTER, &val_text, FontId::monospace(10.5), spec.accent);

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
    ui.painter().circle_stroke(thumb_center, ring_r, Stroke::new(2.0, spec.accent));

    reg.record(spec.id, spec.label, &resp, Some(format!("{:.2}", *value)));
    resp
}

/// Vertical hollow-ring slider (`Point Y  310px`) with small inline label & monospace readout above.
pub fn ring_slider_v(ui: &mut Ui, reg: &mut AutomationRegistry, pal: &MagicPalette, spec: ScalarSpec<'_>, value: &mut f64, height: f32) -> Response {
    let lo = *spec.range.start();
    let hi = (*spec.range.end()).max(lo + 1e-6);
    if let Some(raw) = reg.take_pending_value(spec.id)
        && let Ok(parsed) = raw.parse::<f64>()
    {
        *value = parsed.clamp(lo, hi);
    }
    let width = ui.available_width().clamp(84.0, 132.0);
    let total_h = height.max(68.0);
    let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(width, total_h), Sense::click_and_drag());
    let track_top = rect.min.y + 20.0;
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
    ui.painter().text(Pos2::new(rect.min.x + 2.0, rect.min.y + 7.0), Align2::LEFT_CENTER, spec.label, FontId::proportional(10.5), pal.text_muted);
    ui.painter().text(Pos2::new(rect.max.x - 2.0, rect.min.y + 7.0), Align2::RIGHT_CENTER, &val_text, FontId::monospace(10.5), spec.accent);

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
            let left_w = (ui.available_width() * 0.56).max(96.0);
            ui.allocate_ui(Vec2::new(left_w, 74.0), |ui| {
                ui.add_space(20.0);
                if ring_slider_h(ui, reg, pal, spec_x, x).changed() {
                    changed = true;
                }
            });
            ui.add_space(4.0);
            if ring_slider_v(ui, reg, pal, spec_y, y, 74.0).changed() {
                changed = true;
            }
        });
    });
    changed
}

/// Circular 360° rotary dial with radial tick mark and compact inline value readout.
pub fn rotary_dial(ui: &mut Ui, reg: &mut AutomationRegistry, pal: &MagicPalette, spec: ScalarSpec<'_>, value: &mut f64) -> Response {
    let lo = *spec.range.start();
    let hi = (*spec.range.end()).max(lo + 1e-6);
    if let Some(raw) = reg.take_pending_value(spec.id)
        && let Ok(parsed) = raw.parse::<f64>()
    {
        *value = parsed.clamp(lo, hi);
    }
    let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(76.0, 74.0), Sense::click_and_drag());
    if resp.double_clicked() {
        *value = 0.0_f64.clamp(lo, hi);
        resp.mark_changed();
    } else if resp.dragged() {
        let delta = (-resp.drag_delta().y + resp.drag_delta().x) as f64;
        let step = (hi - lo) / 220.0;
        *value = (*value + delta * step).clamp(lo, hi);
        resp.mark_changed();
    }

    ui.painter().text(Pos2::new(rect.center().x, rect.min.y + 7.0), Align2::CENTER_CENTER, spec.label, FontId::proportional(10.0), pal.text_muted);
    let center = Pos2::new(rect.center().x, rect.min.y + 36.0);
    let r = 17.5;
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
    ui.painter().text(Pos2::new(rect.center().x, rect.max.y - 6.0), Align2::CENTER_CENTER, &val_str, FontId::monospace(10.0), spec.accent);
    reg.record(spec.id, spec.label, &resp, Some(format!("{:.1}", *value)));
    resp
}

/// Compact capsule toggle switch with smaller inline text label.
pub fn pill_toggle(ui: &mut Ui, reg: &mut AutomationRegistry, pal: &MagicPalette, id: &str, label: &str, checked: &mut bool, accent: Color32) -> Response {
    if let Some(next) = reg.take_pending_checked(id) {
        *checked = next;
    }
    let galley = ui.painter().layout_no_wrap(label.to_owned(), FontId::proportional(11.0), pal.text_secondary);
    let width = (36.0 + galley.size().x + 6.0).min(ui.available_width().max(80.0));
    let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(width, 22.0), Sense::click());
    if resp.clicked() {
        *checked = !*checked;
        resp.mark_changed();
    }
    let pill = Rect::from_min_size(Pos2::new(rect.min.x, rect.center().y - 7.5), Vec2::new(28.0, 15.0));
    let fill = if *checked { accent.gamma_multiply(0.28) } else { pal.bg_input };
    let border = if *checked { accent } else { pal.border };
    ui.painter().rect(pill, CornerRadius::same(8), fill, Stroke::new(1.0, border), StrokeKind::Inside);
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
pub fn craft_swatch_button(ui: &mut Ui, reg: &mut AutomationRegistry, id: &str, code: &str, fill: Color32, tooltip: &str) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::click());
    let bg = if resp.hovered() { fill.gamma_multiply(1.12) } else { fill };
    ui.painter().rect(
        rect,
        CornerRadius::same(6),
        bg,
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, if resp.hovered() { 95 } else { 35 })),
        StrokeKind::Inside,
    );
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, code, FontId::proportional(10.5), Color32::WHITE);
    let resp = resp.on_hover_text(tooltip);
    reg.record(id, tooltip, &resp, None);
    resp
}

/// Clickable Storyboard requirement row showing a glowing Green (`filled`) or Red (`missing`) light.
pub fn requirement_light_row(ui: &mut Ui, reg: &mut AutomationRegistry, pal: &MagicPalette, row: RequirementRow<'_>) -> Response {
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
    ui.painter().rect(rect, CornerRadius::same(6), bg, Stroke::new(1.0, border), StrokeKind::Inside);

    let light_col = if row.filled { pal.accent_emerald } else { pal.danger };
    let dot_center = Pos2::new(rect.min.x + 12.0, rect.center().y);
    ui.painter().circle_filled(dot_center, 6.0, light_col.gamma_multiply(0.22));
    if row.filled {
        ui.painter().circle_filled(dot_center, 4.0, light_col);
    } else {
        ui.painter().circle_stroke(dot_center, 4.0, Stroke::new(1.6, light_col));
    }
    ui.painter().text(Pos2::new(rect.min.x + 24.0, rect.center().y), Align2::LEFT_CENTER, row.title, FontId::proportional(11.5), pal.text_primary);
    ui.painter().text(Pos2::new(rect.max.x - 8.0, rect.center().y), Align2::RIGHT_CENTER, row.detail, FontId::monospace(9.5), light_col);
    reg.record(row.id, row.title, &resp, Some(if row.filled { "filled" } else { "missing" }.into()));
    resp
}
