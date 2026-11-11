//! Dark Creative Workstation & Light Paper palettes wired through `craft_ui::theme::WidgetPalette`
//! (`storytold/craft-libs/docs/theme-infrastructure.md`):
//! - Background canvas: `#141C28` (deep stage `#0D121B`)
//! - Surface / Panels: `#1C2433` (elevated `#222C3E`, input `#0E141E`)
//! - Borders / Seams: `#2B364A` (`1px`)
//! - Primary text: `#F0F4FC`, Muted labels: `#8897B5`
//! - Accent / Highlight: `#35E1C0` (mint), `#EABB62` (amber), `#58A6FF` (sky), `#A371F7` (violet)

use craft_ui::buttons::{ButtonVisuals, IconButtonStyle};
use craft_ui::theme::WidgetPalette;
use egui::{Color32, CornerRadius, Stroke, Style, Visuals};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
    System,
}

impl ThemeMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
            Self::System => "system",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "dark" => Some(Self::Dark),
            "light" => Some(Self::Light),
            "system" | "auto" => Some(Self::System),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MagicPalette {
    pub dark: bool,
    pub bg_canvas: Color32,
    pub bg_stage: Color32,
    pub bg_panel: Color32,
    pub bg_elevated: Color32,
    pub bg_input: Color32,
    pub bg_hover: Color32,
    pub bg_active: Color32,
    pub border: Color32,
    pub border_subtle: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_muted: Color32,
    pub accent_emerald: Color32,
    pub accent_emerald_bg: Color32,
    pub accent_mint: Color32,
    pub accent_amber: Color32,
    pub accent_sky: Color32,
    pub accent_violet: Color32,
    pub danger: Color32,
    pub stage_paper: Color32,
}

impl MagicPalette {
    pub const fn dark() -> Self {
        Self {
            dark: true,
            bg_canvas: Color32::from_rgb(0x14, 0x1c, 0x28),
            bg_stage: Color32::from_rgb(0x0d, 0x12, 0x1b),
            bg_panel: Color32::from_rgb(0x1c, 0x24, 0x33),
            bg_elevated: Color32::from_rgb(0x22, 0x2c, 0x3e),
            bg_input: Color32::from_rgb(0x0e, 0x14, 0x1e),
            bg_hover: Color32::from_rgb(0x27, 0x33, 0x47),
            bg_active: Color32::from_rgb(0x14, 0x32, 0x2e),
            border: Color32::from_rgb(0x2b, 0x36, 0x4a),
            border_subtle: Color32::from_rgb(0x1f, 0x28, 0x38),
            text_primary: Color32::from_rgb(0xf0, 0xf4, 0xfc),
            text_secondary: Color32::from_rgb(0xc6, 0xd2, 0xe6),
            text_muted: Color32::from_rgb(0x88, 0x97, 0xb5),
            accent_emerald: Color32::from_rgb(0x35, 0xe1, 0xc0),
            accent_emerald_bg: Color32::from_rgb(0x14, 0x32, 0x2e),
            accent_mint: Color32::from_rgb(0x63, 0xed, 0xd2),
            accent_amber: Color32::from_rgb(0xea, 0xbb, 0x62),
            accent_sky: Color32::from_rgb(0x58, 0xa6, 0xff),
            accent_violet: Color32::from_rgb(0xa3, 0x71, 0xf7),
            danger: Color32::from_rgb(0xf8, 0x51, 0x49),
            stage_paper: Color32::from_rgb(0xf5, 0xef, 0xe2),
        }
    }

    pub const fn light() -> Self {
        Self {
            dark: false,
            bg_canvas: Color32::from_rgb(0xf3, 0xf5, 0xf9),
            bg_stage: Color32::from_rgb(0xe6, 0xea, 0xf2),
            bg_panel: Color32::from_rgb(0xff, 0xff, 0xff),
            bg_elevated: Color32::from_rgb(0xeb, 0xef, 0xf6),
            bg_input: Color32::from_rgb(0xf7, 0xf9, 0xfc),
            bg_hover: Color32::from_rgb(0xdf, 0xe6, 0xf2),
            bg_active: Color32::from_rgb(0xd4, 0xf5, 0xec),
            border: Color32::from_rgb(0xc9, 0xd3, 0xe3),
            border_subtle: Color32::from_rgb(0xe1, 0xe7, 0xf2),
            text_primary: Color32::from_rgb(0x16, 0x1e, 0x2b),
            text_secondary: Color32::from_rgb(0x38, 0x45, 0x5c),
            text_muted: Color32::from_rgb(0x62, 0x71, 0x8a),
            accent_emerald: Color32::from_rgb(0x0f, 0x8a, 0x72),
            accent_emerald_bg: Color32::from_rgb(0xd4, 0xf5, 0xec),
            accent_mint: Color32::from_rgb(0x0d, 0x94, 0x7a),
            accent_amber: Color32::from_rgb(0xb4, 0x69, 0x0e),
            accent_sky: Color32::from_rgb(0x09, 0x69, 0xda),
            accent_violet: Color32::from_rgb(0x7c, 0x3a, 0xed),
            danger: Color32::from_rgb(0xcf, 0x22, 0x2e),
            stage_paper: Color32::from_rgb(0xf5, 0xef, 0xe2),
        }
    }

    pub fn for_mode(mode: ThemeMode, system_dark: Option<bool>) -> Self {
        match mode {
            ThemeMode::Dark => Self::dark(),
            ThemeMode::Light => Self::light(),
            ThemeMode::System => {
                if system_dark.unwrap_or(true) {
                    Self::dark()
                } else {
                    Self::light()
                }
            }
        }
    }

    pub fn widget_palette(&self) -> WidgetPalette {
        WidgetPalette { noninteractive: self.bg_panel, inactive: self.bg_elevated, hovered: self.bg_hover, active: self.bg_active, open: self.bg_elevated }
    }

    pub fn icon_button_style(&self) -> IconButtonStyle {
        IconButtonStyle {
            normal: ButtonVisuals { fill: Color32::TRANSPARENT, stroke: Stroke::NONE, icon: self.text_secondary },
            hovered: ButtonVisuals { fill: self.bg_hover, stroke: Stroke::new(1.0, self.border), icon: self.text_primary },
            pressed: Some(ButtonVisuals { fill: self.bg_active, stroke: Stroke::new(1.0, self.accent_emerald), icon: self.accent_emerald }),
            selected: ButtonVisuals { fill: self.accent_emerald_bg, stroke: Stroke::new(1.0, self.accent_emerald), icon: self.accent_emerald },
            corner_radius: CornerRadius::same(8),
            focus_stroke: Stroke::new(1.5, self.accent_sky),
        }
    }

    pub fn solid_button_style(&self, primary: bool) -> IconButtonStyle {
        if primary {
            let fg = if self.dark { Color32::from_rgb(0x06, 0x16, 0x12) } else { Color32::WHITE };
            IconButtonStyle {
                normal: ButtonVisuals { fill: self.accent_emerald, stroke: Stroke::NONE, icon: fg },
                hovered: ButtonVisuals { fill: self.accent_mint, stroke: Stroke::NONE, icon: fg },
                pressed: None,
                selected: ButtonVisuals { fill: self.accent_emerald, stroke: Stroke::new(1.0, self.text_primary), icon: fg },
                corner_radius: CornerRadius::same(8),
                focus_stroke: Stroke::new(1.5, self.accent_sky),
            }
        } else {
            IconButtonStyle {
                normal: ButtonVisuals { fill: self.bg_elevated, stroke: Stroke::new(1.0, self.border), icon: self.text_primary },
                hovered: ButtonVisuals { fill: self.bg_hover, stroke: Stroke::new(1.0, self.border), icon: self.text_primary },
                pressed: None,
                selected: ButtonVisuals { fill: self.accent_emerald_bg, stroke: Stroke::new(1.0, self.accent_emerald), icon: self.accent_emerald },
                corner_radius: CornerRadius::same(8),
                focus_stroke: Stroke::new(1.5, self.accent_sky),
            }
        }
    }

    pub fn panel_frame(&self) -> egui::Frame {
        egui::Frame::new().fill(self.bg_panel).inner_margin(egui::Margin::symmetric(10, 6)).stroke(Stroke::new(1.0, self.border))
    }

    pub fn card_frame(&self) -> egui::Frame {
        egui::Frame::new().fill(self.bg_panel).corner_radius(CornerRadius::same(10)).inner_margin(egui::Margin::same(10)).stroke(Stroke::new(1.0, self.border))
    }

    pub fn inset_frame(&self) -> egui::Frame {
        egui::Frame::new()
            .fill(self.bg_input)
            .corner_radius(CornerRadius::same(8))
            .inner_margin(egui::Margin::same(8))
            .stroke(Stroke::new(1.0, self.border_subtle))
    }

    pub fn apply(&self, ctx: &egui::Context) {
        let mut style: Style = (*ctx.global_style()).clone();
        let mut visuals = if self.dark { Visuals::dark() } else { Visuals::light() };
        // Shared 5-state operation (`craft_ui::theme::WidgetPalette::apply_to`), followed by local borders/radii.
        self.widget_palette().apply_to(&mut visuals.widgets, CornerRadius::same(8), Stroke::new(1.0, self.text_primary));
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, self.border_subtle);
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, self.text_secondary);
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, self.border);
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, self.accent_emerald);
        visuals.widgets.active.bg_stroke = Stroke::new(1.0, self.accent_emerald);
        visuals.panel_fill = self.bg_canvas;
        visuals.window_fill = self.bg_panel;
        visuals.extreme_bg_color = self.bg_input;
        visuals.faint_bg_color = self.bg_elevated;
        visuals.selection.bg_fill = self.accent_emerald_bg;
        visuals.selection.stroke = Stroke::new(1.0, self.accent_emerald);
        visuals.window_corner_radius = CornerRadius::same(10);
        visuals.window_stroke = Stroke::new(1.0, self.border);
        style.visuals = visuals;
        style.spacing.item_spacing = egui::vec2(6.0, 6.0);
        style.spacing.button_padding = egui::vec2(8.0, 4.0);
        style.spacing.interact_size = egui::vec2(28.0, 26.0);
        ctx.set_global_style(style);
    }
}
