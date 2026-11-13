//! Shared `craft-ui` workstation controls, icon/label buttons, stadium mint mode-tab capsules
//! with embedded stage SVG icons (`1. App-Bar & Navigation Tokens` in `00-figma-component-system-sheet.jpg`),
//! glowing project pill button, traffic-light pills, and per-frame widget automation-ID registry.

use std::collections::BTreeMap;

use craft_ui::buttons::IconButton;
use craft_ui::tabs::Tab;
use egui::{Align2, Color32, CornerRadius, FontId, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2};
use serde_json::{Value, json};

use crate::icons::{companion_image, icon_image};
use crate::theme::MagicPalette;

#[derive(Clone, Debug)]
pub struct AutomationElement {
    pub id: String,
    pub label: String,
    pub enabled: bool,
    pub egui_id: egui::Id,
    pub rect: Rect,
    pub value: Option<String>,
}

impl AutomationElement {
    pub fn to_json(&self) -> Value {
        let mut obj = json!({
            "id": self.id,
            "label": self.label,
            "enabled": self.enabled,
            "rect": [self.rect.min.x, self.rect.min.y, self.rect.width(), self.rect.height()],
        });
        if let Some(val) = &self.value {
            obj["value"] = json!(val);
        }
        obj
    }
}

#[derive(Clone, Debug, Default)]
pub struct AutomationRegistry {
    current: BTreeMap<String, AutomationElement>,
    panels: BTreeMap<String, Rect>,
    pending_set: BTreeMap<String, String>,
    pending_check: BTreeMap<String, bool>,
}

impl AutomationRegistry {
    pub fn begin_frame(&mut self) {
        self.current.clear();
        self.panels.clear();
    }

    pub fn record_panel(&mut self, name: &str, rect: Rect) {
        if rect.is_positive() {
            self.panels.insert(name.to_owned(), rect);
        }
    }

    pub fn panel_rect(&self, name: &str) -> Option<Rect> {
        self.panels.get(name).copied()
    }

    pub fn record(&mut self, id: impl Into<String>, label: impl Into<String>, resp: &Response, value: Option<String>) {
        let id = id.into();
        if id.is_empty() || !resp.rect.is_positive() {
            return;
        }
        self.current.insert(id.clone(), AutomationElement { id, label: label.into(), enabled: resp.enabled(), egui_id: resp.id, rect: resp.rect, value });
    }

    pub fn get(&self, id: &str) -> Option<&AutomationElement> {
        let normalized = id.strip_prefix('#').unwrap_or(id);
        self.current.get(normalized)
    }

    pub fn find_by_egui_id(&self, id: egui::Id) -> Option<&AutomationElement> {
        self.current.values().find(|e| e.egui_id == id)
    }

    pub fn list(&self, prefix: Option<&str>) -> Vec<Value> {
        self.current.values().filter(|el| prefix.is_none_or(|p| el.id.starts_with(p.strip_prefix('#').unwrap_or(p)))).map(AutomationElement::to_json).collect()
    }

    pub fn queue_set_value(&mut self, id: String, value: String) {
        let key = id.strip_prefix('#').unwrap_or(&id).to_owned();
        self.pending_set.insert(key, value);
    }

    pub fn take_pending_value(&mut self, id: &str) -> Option<String> {
        let key = id.strip_prefix('#').unwrap_or(id);
        self.pending_set.remove(key)
    }

    pub fn queue_set_checked(&mut self, id: String, checked: bool) {
        let key = id.strip_prefix('#').unwrap_or(&id).to_owned();
        self.pending_check.insert(key, checked);
    }

    pub fn take_pending_checked(&mut self, id: &str) -> Option<bool> {
        let key = id.strip_prefix('#').unwrap_or(id);
        self.pending_check.remove(key)
    }
}

pub struct BtnSpec<'a> {
    pub id: &'a str,
    pub icon: Option<&'a str>,
    pub label: &'a str,
    pub tooltip: &'a str,
    pub primary: bool,
    pub selected: bool,
    pub enabled: bool,
}

impl<'a> BtnSpec<'a> {
    pub const fn new(id: &'a str, icon: &'a str, label: &'a str, tooltip: &'a str) -> Self {
        Self { id, icon: Some(icon), label, tooltip, primary: false, selected: false, enabled: true }
    }

    pub const fn label_only(id: &'a str, label: &'a str, tooltip: &'a str) -> Self {
        Self { id, icon: None, label, tooltip, primary: false, selected: false, enabled: true }
    }

    pub const fn icon_only(id: &'a str, icon: &'a str, tooltip: &'a str) -> Self {
        Self { id, icon: Some(icon), label: "", tooltip, primary: false, selected: false, enabled: true }
    }

    pub const fn primary(mut self) -> Self {
        self.primary = true;
        self
    }

    pub const fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

#[allow(clippy::cognitive_complexity)]
/// Draw a uniform workstation button via `craft_ui::buttons::IconButton` and register its automation ID.
pub fn action_button(ui: &mut Ui, reg: &mut AutomationRegistry, pal: &MagicPalette, spec: BtnSpec<'_>) -> Response {
    let font = FontId::proportional(12.0);
    let text_width = if spec.label.is_empty() { 0.0 } else { ui.painter().layout_no_wrap(spec.label.to_owned(), font.clone(), pal.text_primary).size().x };
    let has_icon = spec.icon.is_some();
    let width = match (has_icon, spec.label.is_empty()) {
        (true, true) => 28.0,
        (true, false) => (26.0 + text_width + 12.0).max(50.0),
        (false, _) => (text_width + 18.0).max(42.0),
    };
    let style = if spec.label.is_empty() && !spec.primary { pal.icon_button_style() } else { pal.solid_button_style(spec.primary) };
    let accessible = if spec.label.is_empty() { spec.tooltip } else { spec.label };
    let resp = ui
        .push_id(spec.id, |ui| {
            ui.add_enabled_ui(spec.enabled, |ui| {
                IconButton::new(accessible, Vec2::new(width, 26.0), &style).selected(spec.selected).show(ui, |ui, rect, tint| {
                    let mut x = rect.min.x + 8.0;
                    if let Some(icon_name) = spec.icon {
                        let icon_rect = if spec.label.is_empty() {
                            Rect::from_center_size(rect.center(), Vec2::splat(15.0))
                        } else {
                            Rect::from_min_size(Pos2::new(x, rect.center().y - 7.0), Vec2::splat(14.0))
                        };
                        let icon_tint = if icon_name == "brand" && !spec.primary { pal.accent_amber } else { tint };
                        icon_image(icon_name, icon_rect.width(), icon_tint).paint_at(ui, icon_rect);
                        x = icon_rect.max.x + 5.0;
                    }
                    if !spec.label.is_empty() {
                        if has_icon {
                            ui.painter().text(Pos2::new(x, rect.center().y), Align2::LEFT_CENTER, spec.label, font, tint);
                        } else {
                            ui.painter().text(rect.center(), Align2::CENTER_CENTER, spec.label, font, tint);
                        }
                    }
                })
            })
            .inner
        })
        .inner;
    let resp = if spec.tooltip.is_empty() { resp } else { resp.on_hover_text(spec.tooltip) };
    reg.record(spec.id, accessible, &resp, None);
    resp
}

/// Stadium project pill button (`[📖 Pterodactyl Song]` in `02-project-craft-files.jpg`).
pub fn project_pill_button(ui: &mut Ui, reg: &mut AutomationRegistry, pal: &MagicPalette, id: &str, label: &str, selected: bool, tooltip: &str) -> Response {
    let font = FontId::proportional(12.5);
    let galley = ui.painter().layout_no_wrap(label.to_owned(), font.clone(), pal.text_primary);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(galley.size().x + 38.0, 28.0), Sense::click());
    let fill = if selected {
        pal.accent_emerald_bg
    } else if resp.hovered() {
        pal.bg_hover
    } else {
        pal.bg_elevated
    };
    let stroke = if selected { Stroke::new(1.4, pal.accent_emerald) } else { Stroke::new(1.0, pal.border) };
    ui.painter().rect(rect, CornerRadius::same(14), fill, stroke, StrokeKind::Inside);
    let icon_rect = Rect::from_min_size(Pos2::new(rect.min.x + 10.0, rect.center().y - 7.0), Vec2::splat(14.0));
    let fg = if selected { pal.text_primary } else { pal.text_secondary };
    icon_image("book", 14.0, if selected { pal.accent_emerald } else { fg }).paint_at(ui, icon_rect);
    ui.painter().text(Pos2::new(icon_rect.max.x + 6.0, rect.center().y), Align2::LEFT_CENTER, label, font, pal.text_primary);
    let resp = resp.on_hover_text(tooltip);
    reg.record(id, label, &resp, None);
    resp
}

pub fn companion_button(ui: &mut Ui, reg: &mut AutomationRegistry, pal: &MagicPalette, id: &str, pose: &str, tooltip: &str, selected: bool) -> Response {
    let style = pal.icon_button_style();
    let resp = ui
        .push_id(id, |ui| {
            IconButton::new(tooltip, Vec2::splat(28.0), &style).selected(selected).show(ui, |ui, rect, _tint| {
                let img_rect = Rect::from_center_size(rect.center(), Vec2::splat(20.0));
                companion_image(pose, 20.0).paint_at(ui, img_rect);
            })
        })
        .inner
        .on_hover_text(tooltip);
    reg.record(id, tooltip, &resp, None);
    resp
}

pub struct ModeTabSpec<'a> {
    pub id: &'a str,
    pub icon: &'a str,
    pub label: &'a str,
    pub tooltip: &'a str,
    pub icon_only: bool,
    pub selected: bool,
    pub dot_color: Option<Color32>,
}

/// Primary `#app-bar` mode tab (`1. App-Bar & Navigation Tokens` in `00-figma-component-system-sheet.jpg`)
/// pairing a custom stage SVG icon with its label (or collapsing to an icon pill on narrow viewports).
pub fn mode_tab(ui: &mut Ui, reg: &mut AutomationRegistry, pal: &MagicPalette, spec: ModeTabSpec<'_>) -> Response {
    let font = FontId::proportional(12.2);
    let text_w = if spec.icon_only { 0.0 } else { ui.painter().layout_no_wrap(spec.label.to_owned(), font.clone(), pal.text_primary).size().x };
    let dot_w = if spec.dot_color.is_some() { 9.5 } else { 0.0 };
    let total_w = if spec.icon_only { 34.0 + dot_w } else { 20.0 + 16.0 + text_w + dot_w };
    let (rect, _) = ui.allocate_exact_size(Vec2::new(total_w, 26.0), Sense::hover());
    let tab_id = ui.scope_id().with(spec.id);
    let dark_ink = Color32::from_rgb(0x06, 0x16, 0x12);
    let resp = Tab::new(tab_id, spec.label, spec.selected).sense(Sense::click()).show_at(ui, rect, |ui, response| {
        let (fill, stroke, fg) = mode_tab_colors(pal, dark_ink, spec.selected, response.hovered());
        ui.painter().rect(response.rect, CornerRadius::same(13), fill, stroke, StrokeKind::Inside);

        let mut x = response.rect.min.x + 10.0;
        if let Some(dot) = spec.dot_color {
            let center = Pos2::new(x + 2.5, response.rect.center().y);
            if spec.selected {
                ui.painter().circle_filled(center, 4.2, dark_ink);
            }
            ui.painter().circle_filled(center, 3.0, dot);
            x += 9.0;
        }
        let icon_rect = Rect::from_min_size(Pos2::new(x, response.rect.center().y - 6.5), Vec2::splat(13.0));
        icon_image(spec.icon, 13.0, fg).paint_at(ui, icon_rect);
        if !spec.icon_only {
            ui.painter().text(Pos2::new(icon_rect.max.x + 5.0, response.rect.center().y), Align2::LEFT_CENTER, spec.label, font, fg);
        }
    });
    let resp = resp.on_hover_text(spec.tooltip);
    reg.record(spec.id, spec.label, &resp, None);
    resp
}

fn mode_tab_colors(pal: &MagicPalette, dark_ink: Color32, selected: bool, hovered: bool) -> (Color32, Stroke, Color32) {
    let fill = if selected { pal.accent_emerald } else if hovered { pal.bg_hover } else { Color32::TRANSPARENT };
    let stroke = if selected || !hovered { Stroke::NONE } else { Stroke::new(1.0, pal.border) };
    let fg = if selected {
        if pal.dark { dark_ink } else { Color32::WHITE }
    } else if hovered {
        pal.text_primary
    } else {
        pal.text_secondary
    };
    (fill, stroke, fg)
}

pub fn text_input(ui: &mut Ui, reg: &mut AutomationRegistry, id: &str, label: &str, value: &mut String, hint: &str, width: f32) -> Response {
    if let Some(next) = reg.take_pending_value(id) {
        *value = next;
    }
    let resp = ui.push_id(id, |ui| ui.add_sized([width, 26.0], egui::TextEdit::singleline(value).hint_text(hint))).inner;
    reg.record(id, label, &resp, Some(value.clone()));
    resp
}

pub fn toggle_switch(ui: &mut Ui, reg: &mut AutomationRegistry, id: &str, label: &str, checked: &mut bool) -> Response {
    if let Some(next) = reg.take_pending_checked(id) {
        *checked = next;
    }
    let resp = ui.push_id(id, |ui| ui.checkbox(checked, label)).inner;
    reg.record(id, label, &resp, Some(checked.to_string()));
    resp
}

pub fn status_pill(ui: &mut Ui, pal: &MagicPalette, label: &str, color: Color32) {
    let galley = ui.painter().layout_no_wrap(label.to_owned(), FontId::proportional(11.0), color);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(galley.size().x + 18.0, 20.0), Sense::hover());
    ui.painter().rect(rect, CornerRadius::same(10), color.gamma_multiply(0.14), Stroke::new(1.0, color.gamma_multiply(0.55)), StrokeKind::Inside);
    ui.painter().circle_filled(Pos2::new(rect.min.x + 7.0, rect.center().y), 2.8, color);
    ui.painter().text(Pos2::new(rect.min.x + 13.0, rect.center().y), Align2::LEFT_CENTER, label, FontId::proportional(11.0), color);
    let _ = pal;
}
