//! Shared editorial typography, header kickers, playhead scrubber, and persisted style controls
//! reused across `Storyboard`, `Lyrics`, `Motion`, and the top workstation shell (`#menu-bar`).
//!
//! Panel layout and docking are owned exclusively by [`crate::docking`] (`craft_ui::docking`).

use egui::{Align, Align2, Color32, CornerRadius, FontFamily, FontId, Layout, Painter, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2};
use serde_json::json;

use crate::state::ViewCtx;
use crate::theme::{MagicPalette, ThemeMode};
use crate::widgets::AutomationRegistry;
use crate::widgets::{BtnSpec, action_button, status_pill};

/// Crisp editorial header strip with a vertical accent bar, tracked monospace kicker, and right pill.
pub fn draw_pane_kicker_header(ui: &mut Ui, pal: &MagicPalette, kicker: &str, title: &str, accent: Color32, badge: Option<(&str, Color32)>) {
    ui.horizontal(|ui| {
        let (bar_r, _) = ui.allocate_exact_size(Vec2::new(3.0, 24.0), Sense::hover());
        ui.painter().rect_filled(bar_r, CornerRadius::same(2), accent);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            ui.label(RichText::new(kicker).monospace().size(8.8).color(accent));
            ui.label(RichText::new(title).strong().size(12.0).color(pal.text_primary));
        });
        if let Some((txt, col)) = badge {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                status_pill(ui, pal, txt, col);
            });
        }
    });
}

pub struct ScrubberSpec<'a> {
    pub id: &'a str,
    pub label: &'a str,
    pub width: f32,
    pub duration_sec: f64,
    pub accent: Color32,
}

/// Compact 16px playhead scrubber bar shared by `Lyrics` and `Motion` transport rows.
pub fn draw_compact_scrubber(ui: &mut Ui, reg: &mut AutomationRegistry, pal: &MagicPalette, spec: ScrubberSpec<'_>, current_sec: &mut f64) -> Option<f64> {
    let dur = spec.duration_sec.max(0.5);
    let (srect, sresp) = ui.allocate_exact_size(Vec2::new(spec.width.max(72.0), 16.0), Sense::click_and_drag());
    let track_r = Rect::from_center_size(srect.center(), Vec2::new(srect.width(), 4.0));
    ui.painter().rect_filled(track_r, CornerRadius::same(2), pal.bg_input);
    let frac = (*current_sec / dur).clamp(0.0, 1.0) as f32;
    let knob_x = track_r.min.x + track_r.width() * frac;
    let fill_r = Rect::from_min_max(track_r.min, Pos2::new(knob_x, track_r.max.y));
    ui.painter().rect_filled(fill_r, CornerRadius::same(2), spec.accent);
    ui.painter().circle_filled(Pos2::new(knob_x, track_r.center().y), 5.2, Color32::WHITE);
    ui.painter().circle_stroke(Pos2::new(knob_x, track_r.center().y), 5.2, Stroke::new(1.2, spec.accent));

    let mut sought = None;
    if (sresp.clicked() || sresp.dragged())
        && let Some(ptr) = sresp.interact_pointer_pos()
    {
        let next_f = ((ptr.x - track_r.min.x) / track_r.width().max(1.0)).clamp(0.0, 1.0) as f64;
        let sec = next_f * dur;
        *current_sec = sec;
        sought = Some(sec);
    }
    reg.record(spec.id, spec.label, &sresp, Some(format!("{current_sec:.2}")));
    sought
}

pub struct WrappedTextSpec<'a> {
    pub rect: Rect,
    pub text: &'a str,
    pub font_pt: f32,
    pub color: Color32,
    pub monospace: bool,
}

/// Word-wrapped text renderer inside bounded slide cards so multi-word sentences wrap cleanly
/// onto subsequent lines instead of overflowing or clipping horizontally.
pub fn paint_wrapped_editorial_text(painter: &Painter, spec: WrappedTextSpec<'_>) {
    let inner = spec.rect.shrink2(Vec2::new(2.0, 1.5));
    if inner.width() < 8.0 || inner.height() < 6.0 || spec.text.trim().is_empty() {
        return;
    }
    let family = if spec.monospace { FontFamily::Monospace } else { FontFamily::Proportional };
    let avg_char_w = (spec.font_pt * if spec.monospace { 0.60 } else { 0.51 }).max(3.6);
    let max_chars = ((inner.width() / avg_char_w).floor() as usize).max(4);
    let line_h = (spec.font_pt * 1.22).max(8.5);
    let clip = painter.with_clip_rect(spec.rect);

    let mut y = inner.min.y;
    for para in spec.text.lines() {
        let lines = wrap_paragraph(para, max_chars);
        for line in lines {
            if y + spec.font_pt > inner.max.y + 3.0 {
                return;
            }
            clip.text(Pos2::new(inner.min.x, y), Align2::LEFT_TOP, &line, FontId::new(spec.font_pt, family.clone()), spec.color);
            y += line_h;
        }
    }
}

fn wrap_paragraph(para: &str, max_chars: usize) -> Vec<String> {
    let _ = magic_util::text::is_paragraph_rtl(para, false);
    magic_util::text::wrap_paragraph_uax14(para, max_chars)
}

/// Top-bar style preset & colour-mode pills that persist immediately to disk via `prefs.set`.
pub fn draw_persisted_style_controls(ui: &mut Ui, ctx: &mut ViewCtx<'_>, wide: bool) {
    let mode_icon = match ctx.state.theme_mode {
        ThemeMode::Dark => "moon",
        ThemeMode::Light => "sun",
        ThemeMode::System => "eye",
    };
    if action_button(ui, ctx.reg, ctx.pal, BtnSpec::icon_only("menu-theme-mode", mode_icon, "Toggle Dark / Light studio mode (persisted)")).clicked() {
        let next_mode = if ctx.pal.dark { ThemeMode::Light } else { ThemeMode::Dark };
        ctx.state.theme_mode = next_mode;
        ctx.exec("prefs.set", json!({"key": "appearance.theme", "value": next_mode.as_str()}), Some("Saved colour mode"));
    }

    let style_lbl = ctx.state.interface_style.label();
    let style_spec = if wide {
        BtnSpec::new("menu-style-cycle", "colorWheel", style_lbl, "Cycle studio interface style preset (Aurora · Editorial · Modern … — persisted)")
    } else {
        BtnSpec::icon_only("menu-style-cycle", "colorWheel", "Cycle studio interface style preset")
    };
    if action_button(ui, ctx.reg, ctx.pal, style_spec).clicked() {
        let next_style = ctx.state.interface_style.next();
        ctx.state.interface_style = next_style;
        ctx.exec(
            "prefs.set",
            json!({"key": "appearance.interface_style", "value": next_style.as_str()}),
            Some(&format!("Saved {} interface style", next_style.label())),
        );
    }
}
