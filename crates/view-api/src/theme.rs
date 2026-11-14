//! Workstation theme engine wired through `craft_ui::theme::WidgetPalette`
//! (`storytold/craft-libs/docs/theme-infrastructure.md`) with `InterfaceStyle` presets
//! (`Aurora`, `Editorial`, `Modern`, `Classic`, `Minimal`, `Ocean`, `Forest`, `Plum`, `Sand`),
//! `AccentChoice` swatches (`Teal`, `Blue`, `Violet`, `Amber`, `Graphite`), `High Contrast`,
//! Barnyard/Editorial paper tokens, and `Interface Size` scaling (`80%`–`200%`).

use craft_ui::buttons::{ButtonVisuals, IconButtonStyle};
use craft_ui::theme::WidgetPalette;
use egui::{Color32, CornerRadius, Pos2, Rect, Stroke, Style, Ui, Visuals};
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InterfaceStyle {
    #[default]
    Aurora,
    Editorial,
    Modern,
    Classic,
    Minimal,
    Ocean,
    Forest,
    Plum,
    Sand,
}

impl InterfaceStyle {
    pub const ALL: [Self; 9] = [Self::Aurora, Self::Editorial, Self::Modern, Self::Classic, Self::Minimal, Self::Ocean, Self::Forest, Self::Plum, Self::Sand];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Aurora => "aurora",
            Self::Editorial => "editorial",
            Self::Modern => "modern",
            Self::Classic => "classic",
            Self::Minimal => "minimal",
            Self::Ocean => "ocean",
            Self::Forest => "forest",
            Self::Plum => "plum",
            Self::Sand => "sand",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Aurora => "Aurora",
            Self::Editorial => "Editorial",
            Self::Modern => "Modern",
            Self::Classic => "Classic",
            Self::Minimal => "Minimal",
            Self::Ocean => "Ocean",
            Self::Forest => "Forest",
            Self::Plum => "Plum",
            Self::Sand => "Sand",
        }
    }

    pub fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|&s| s == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "aurora" => Some(Self::Aurora),
            "editorial" | "barnyard" | "archival" => Some(Self::Editorial),
            "modern" => Some(Self::Modern),
            "classic" => Some(Self::Classic),
            "minimal" => Some(Self::Minimal),
            "ocean" => Some(Self::Ocean),
            "forest" => Some(Self::Forest),
            "plum" => Some(Self::Plum),
            "sand" => Some(Self::Sand),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccentChoice {
    #[default]
    Teal,
    Blue,
    Violet,
    Amber,
    Graphite,
}

impl AccentChoice {
    pub const ALL: [Self; 5] = [Self::Teal, Self::Blue, Self::Violet, Self::Amber, Self::Graphite];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Teal => "teal",
            Self::Blue => "blue",
            Self::Violet => "violet",
            Self::Amber => "amber",
            Self::Graphite => "graphite",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Teal => "Teal",
            Self::Blue => "Blue",
            Self::Violet => "Violet",
            Self::Amber => "Amber",
            Self::Graphite => "Graphite",
        }
    }

    pub fn swatch(self, dark: bool) -> Color32 {
        match (self, dark) {
            (Self::Teal, true) => Color32::from_rgb(0x34, 0xd8, 0xb8),
            (Self::Teal, false) => Color32::from_rgb(0x16, 0x7b, 0x67),
            (Self::Blue, true) => Color32::from_rgb(0x5a, 0xa2, 0xff),
            (Self::Blue, false) => Color32::from_rgb(0x1f, 0x5f, 0xd0),
            (Self::Violet, true) => Color32::from_rgb(0xa3, 0x71, 0xf7),
            (Self::Violet, false) => Color32::from_rgb(0x6d, 0x38, 0xd9),
            (Self::Amber, true) => Color32::from_rgb(0xe3, 0xb3, 0x59),
            (Self::Amber, false) => Color32::from_rgb(0xb4, 0x69, 0x14),
            (Self::Graphite, true) => Color32::from_rgb(0x98, 0xa6, 0xba),
            (Self::Graphite, false) => Color32::from_rgb(0x47, 0x55, 0x69),
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "teal" | "mint" => Some(Self::Teal),
            "blue" => Some(Self::Blue),
            "violet" | "purple" => Some(Self::Violet),
            "amber" | "gold" => Some(Self::Amber),
            "graphite" | "slate" => Some(Self::Graphite),
            _ => None,
        }
    }
}

pub const INTERFACE_SIZE_PRESETS: &[u16] = &[80, 90, 100, 110, 125, 150, 175, 200];

#[derive(Clone, Copy, Debug)]
pub struct MagicPalette {
    pub dark: bool,
    pub style: InterfaceStyle,
    pub corner: u8,
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
            style: InterfaceStyle::Aurora,
            corner: 6,
            bg_canvas: Color32::from_rgb(0x11, 0x15, 0x1c),
            bg_stage: Color32::from_rgb(0x0c, 0x10, 0x16),
            bg_panel: Color32::from_rgb(0x16, 0x1b, 0x24),
            bg_elevated: Color32::from_rgb(0x1c, 0x22, 0x2d),
            bg_input: Color32::from_rgb(0x0e, 0x12, 0x19),
            bg_hover: Color32::from_rgb(0x25, 0x2d, 0x3b),
            bg_active: Color32::from_rgb(0x14, 0x31, 0x2d),
            border: Color32::from_rgb(0x2b, 0x34, 0x43),
            border_subtle: Color32::from_rgb(0x1f, 0x26, 0x32),
            text_primary: Color32::from_rgb(0xf2, 0xf4, 0xf8),
            text_secondary: Color32::from_rgb(0xc8, 0xd1, 0xdf),
            text_muted: Color32::from_rgb(0x8a, 0x96, 0xaa),
            accent_emerald: Color32::from_rgb(0x34, 0xd8, 0xb8),
            accent_emerald_bg: Color32::from_rgb(0x14, 0x31, 0x2d),
            accent_mint: Color32::from_rgb(0x63, 0xe8, 0xcd),
            accent_amber: Color32::from_rgb(0xe3, 0xb3, 0x59),
            accent_sky: Color32::from_rgb(0x5e, 0xa8, 0xff),
            accent_violet: Color32::from_rgb(0xa3, 0x71, 0xf7),
            danger: Color32::from_rgb(0xe8, 0x58, 0x4a),
            stage_paper: Color32::from_rgb(0xff, 0xfd, 0xf8),
        }
    }

    pub const fn light() -> Self {
        Self {
            dark: false,
            style: InterfaceStyle::Aurora,
            corner: 6,
            bg_canvas: Color32::from_rgb(0xf4, 0xf2, 0xed),
            bg_stage: Color32::from_rgb(0xea, 0xe6, 0xde),
            bg_panel: Color32::from_rgb(0xfb, 0xfa, 0xf7),
            bg_elevated: Color32::from_rgb(0xff, 0xfe, 0xfb),
            bg_input: Color32::from_rgb(0xff, 0xff, 0xff),
            bg_hover: Color32::from_rgb(0xe9, 0xe4, 0xda),
            bg_active: Color32::from_rgb(0xd8, 0xf0, 0xe9),
            border: Color32::from_rgb(0xd8, 0xd1, 0xc5),
            border_subtle: Color32::from_rgb(0xe5, 0xdf, 0xd4),
            text_primary: Color32::from_rgb(0x22, 0x21, 0x1e),
            text_secondary: Color32::from_rgb(0x45, 0x42, 0x3c),
            text_muted: Color32::from_rgb(0x70, 0x6b, 0x61),
            accent_emerald: Color32::from_rgb(0x16, 0x7b, 0x67),
            accent_emerald_bg: Color32::from_rgb(0xd8, 0xf0, 0xe9),
            accent_mint: Color32::from_rgb(0x12, 0x8b, 0x73),
            accent_amber: Color32::from_rgb(0xb4, 0x69, 0x14),
            accent_sky: Color32::from_rgb(0x28, 0x62, 0x88),
            accent_violet: Color32::from_rgb(0x6d, 0x38, 0xd9),
            danger: Color32::from_rgb(0xb6, 0x4a, 0x37),
            stage_paper: Color32::from_rgb(0xff, 0xfd, 0xf8),
        }
    }

    pub fn for_mode(mode: ThemeMode, system_dark: Option<bool>) -> Self {
        Self::resolve(mode, InterfaceStyle::Aurora, AccentChoice::Teal, false, system_dark)
    }

    pub fn resolve(mode: ThemeMode, style: InterfaceStyle, accent: AccentChoice, high_contrast: bool, system_dark: Option<bool>) -> Self {
        let is_dark = match mode {
            ThemeMode::Dark => true,
            ThemeMode::Light => false,
            ThemeMode::System => system_dark.unwrap_or(true),
        };
        let mut pal = if is_dark { Self::dark() } else { Self::light() };
        pal.style = style;
        pal.corner = match style {
            InterfaceStyle::Classic => 4,
            InterfaceStyle::Editorial | InterfaceStyle::Minimal => 5,
            _ => 6,
        };
        apply_style_tints(&mut pal, style, is_dark);

        let chosen = accent.swatch(is_dark);
        pal.accent_emerald = chosen;
        pal.accent_mint = chosen;
        pal.accent_emerald_bg = chosen.gamma_multiply(if is_dark { 0.20 } else { 0.16 });
        pal.bg_active = pal.accent_emerald_bg;

        if high_contrast {
            pal.border = if is_dark { Color32::from_rgb(0xa8, 0xb5, 0xcc) } else { Color32::from_rgb(0x26, 0x30, 0x42) };
            pal.border_subtle = pal.border.gamma_multiply(0.75);
            pal.text_primary = if is_dark { Color32::WHITE } else { Color32::BLACK };
            pal.text_secondary = pal.text_primary;
        }
        pal
    }

    /// Canonical Barnyard (`Storyboard.potx`) & DesignCraft editorial swatch lookup.
    pub fn editorial_swatch(swatch: &str) -> Option<Color32> {
        match swatch {
            "Cream" => Some(Color32::from_rgb(0xff, 0xfd, 0xf8)),
            "WarmCard" | "Kraft" => Some(Color32::from_rgb(0xff, 0xf9, 0xf0)),
            "[Paper]" => Some(Color32::from_rgb(0xfc, 0xf9, 0xf2)),
            "Rule" | "Hair" => Some(Color32::from_rgb(0xd9, 0xc9, 0xb8)),
            "Ink" | "[Black]" => Some(Color32::from_rgb(0x29, 0x28, 0x21)),
            "Pencil" => Some(Color32::from_rgb(0x66, 0x5f, 0x55)),
            "Rust" => Some(Color32::from_rgb(0xb6, 0x4a, 0x37)),
            "Petrol" | "Navy" => Some(Color32::from_rgb(0x31, 0x5e, 0x72)),
            "Sun" => Some(Color32::from_rgb(0xd2, 0xa1, 0x3a)),
            "Moss" | "Sage" => Some(Color32::from_rgb(0x6f, 0x8e, 0x4d)),
            "Sky" => Some(Color32::from_rgb(0x70, 0xbb, 0xdd)),
            "Rose" => Some(Color32::from_rgb(0xd9, 0x7c, 0x90)),
            "Slot" | "Tape" => Some(Color32::from_rgb(0xf0, 0xe8, 0xdc)),
            "Film" | "Leather" => Some(Color32::from_rgb(0xe3, 0xd9, 0xc9)),
            _ => None,
        }
    }

    /// Section-specific studio accent color used across Storyboard tree rails and stage headers.
    pub fn section_accent(&self, section: &str) -> Color32 {
        match section {
            "Cover" => Color32::from_rgb(0xc8, 0x5a, 0x44),
            "Backgrounds" => self.accent_sky,
            "Characters" => self.accent_amber,
            "Scenes" => self.accent_emerald,
            "Assets" => self.accent_violet,
            "Final" => Color32::from_rgb(0x6f, 0xa8, 0x62),
            _ => self.accent_emerald,
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
            corner_radius: CornerRadius::same(self.corner),
            focus_stroke: Stroke::new(1.5, self.accent_sky),
        }
    }

    pub fn solid_button_style(&self, primary: bool) -> IconButtonStyle {
        if primary {
            let fg = if self.dark { Color32::from_rgb(0x06, 0x13, 0x18) } else { Color32::WHITE };
            IconButtonStyle {
                normal: ButtonVisuals { fill: self.accent_emerald, stroke: Stroke::NONE, icon: fg },
                hovered: ButtonVisuals { fill: self.accent_mint, stroke: Stroke::NONE, icon: fg },
                pressed: None,
                selected: ButtonVisuals { fill: self.accent_emerald, stroke: Stroke::new(1.0, self.text_primary), icon: fg },
                corner_radius: CornerRadius::same(self.corner),
                focus_stroke: Stroke::new(1.5, self.accent_sky),
            }
        } else {
            IconButtonStyle {
                normal: ButtonVisuals { fill: self.bg_elevated, stroke: Stroke::new(1.0, self.border), icon: self.text_primary },
                hovered: ButtonVisuals { fill: self.bg_hover, stroke: Stroke::new(1.0, self.border), icon: self.text_primary },
                pressed: None,
                selected: ButtonVisuals { fill: self.accent_emerald_bg, stroke: Stroke::new(1.0, self.accent_emerald), icon: self.accent_emerald },
                corner_radius: CornerRadius::same(self.corner),
                focus_stroke: Stroke::new(1.5, self.accent_sky),
            }
        }
    }

    pub fn panel_frame(&self) -> egui::Frame {
        egui::Frame::new().fill(self.bg_panel).inner_margin(egui::Margin::symmetric(10, 6)).stroke(Stroke::new(1.0, self.border))
    }

    pub fn card_frame(&self) -> egui::Frame {
        egui::Frame::new()
            .fill(self.bg_elevated)
            .corner_radius(CornerRadius::same(self.corner + 2))
            .inner_margin(egui::Margin::same(10))
            .stroke(Stroke::new(1.0, self.border))
    }

    pub fn inset_frame(&self) -> egui::Frame {
        egui::Frame::new()
            .fill(self.bg_input)
            .corner_radius(CornerRadius::same(self.corner))
            .inner_margin(egui::Margin::same(8))
            .stroke(Stroke::new(1.0, self.border_subtle))
    }

    pub fn apply(&self, ctx: &egui::Context) {
        let mut style: Style = (*ctx.global_style()).clone();
        let mut visuals = if self.dark { Visuals::dark() } else { Visuals::light() };
        self.widget_palette().apply_to(&mut visuals.widgets, CornerRadius::same(self.corner), Stroke::new(1.0, self.text_primary));
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
        visuals.window_corner_radius = CornerRadius::same(self.corner + 3);
        visuals.window_stroke = Stroke::new(1.0, self.border);
        style.visuals = visuals;
        style.spacing.item_spacing = egui::vec2(6.0, 6.0);
        style.spacing.button_padding = egui::vec2(8.0, 4.0);
        style.spacing.interact_size = egui::vec2(28.0, 26.0);
        ctx.set_global_style(style);
    }
}

fn apply_style_tints(pal: &mut MagicPalette, style: InterfaceStyle, is_dark: bool) {
    if is_dark {
        match style {
            InterfaceStyle::Aurora => {
                pal.bg_stage = Color32::from_rgb(0x0c, 0x10, 0x16);
                pal.bg_input = Color32::from_rgb(0x0e, 0x12, 0x19);
                pal.bg_canvas = Color32::from_rgb(0x11, 0x15, 0x1c);
                pal.bg_panel = Color32::from_rgb(0x16, 0x1b, 0x24);
                pal.bg_elevated = Color32::from_rgb(0x1c, 0x22, 0x2d);
                pal.border = Color32::from_rgb(0x2b, 0x34, 0x43);
            }
            InterfaceStyle::Editorial => {
                pal.bg_stage = Color32::from_rgb(0x14, 0x14, 0x13);
                pal.bg_canvas = Color32::from_rgb(0x19, 0x18, 0x16);
                pal.bg_panel = Color32::from_rgb(0x20, 0x1f, 0x1c);
                pal.bg_elevated = Color32::from_rgb(0x28, 0x26, 0x22);
                pal.bg_input = Color32::from_rgb(0x13, 0x12, 0x11);
                pal.border = Color32::from_rgb(0x3b, 0x36, 0x2f);
                pal.border_subtle = Color32::from_rgb(0x2d, 0x2a, 0x24);
                pal.text_primary = Color32::from_rgb(0xf7, 0xf3, 0xeb);
                pal.text_secondary = Color32::from_rgb(0xd7, 0xcf, 0xc2);
            }
            InterfaceStyle::Modern => {}
            InterfaceStyle::Classic => {
                pal.bg_canvas = Color32::from_rgb(0x16, 0x19, 0x1f);
                pal.bg_panel = Color32::from_rgb(0x1d, 0x21, 0x28);
                pal.bg_elevated = Color32::from_rgb(0x24, 0x29, 0x32);
            }
            InterfaceStyle::Minimal => {
                pal.bg_canvas = Color32::from_rgb(0x10, 0x13, 0x18);
                pal.bg_panel = Color32::from_rgb(0x13, 0x17, 0x1d);
                pal.border = Color32::from_rgb(0x23, 0x2a, 0x36);
            }
            InterfaceStyle::Ocean => {
                pal.bg_canvas = Color32::from_rgb(0x0d, 0x18, 0x24);
                pal.bg_panel = Color32::from_rgb(0x12, 0x20, 0x30);
                pal.bg_elevated = Color32::from_rgb(0x17, 0x28, 0x3b);
            }
            InterfaceStyle::Forest => {
                pal.bg_canvas = Color32::from_rgb(0x0f, 0x19, 0x15);
                pal.bg_panel = Color32::from_rgb(0x14, 0x21, 0x1c);
                pal.bg_elevated = Color32::from_rgb(0x1a, 0x2a, 0x24);
            }
            InterfaceStyle::Plum => {
                pal.bg_canvas = Color32::from_rgb(0x17, 0x12, 0x21);
                pal.bg_panel = Color32::from_rgb(0x1f, 0x18, 0x2c);
                pal.bg_elevated = Color32::from_rgb(0x27, 0x1f, 0x37);
            }
            InterfaceStyle::Sand => {
                pal.bg_canvas = Color32::from_rgb(0x1b, 0x18, 0x14);
                pal.bg_panel = Color32::from_rgb(0x23, 0x1f, 0x1a);
                pal.bg_elevated = Color32::from_rgb(0x2c, 0x27, 0x21);
            }
        }
    } else if matches!(style, InterfaceStyle::Editorial | InterfaceStyle::Sand) {
        pal.bg_stage = Color32::from_rgb(0xec, 0xe6, 0xdb);
        pal.bg_canvas = Color32::from_rgb(0xf6, 0xf2, 0xe9);
        pal.bg_panel = Color32::from_rgb(0xfd, 0xfb, 0xf6);
        pal.bg_elevated = Color32::from_rgb(0xff, 0xfd, 0xf9);
        pal.border = Color32::from_rgb(0xd9, 0xc9, 0xb8);
    }
}

/// Paints the subtle Aurora horizon wave line along the bottom of the workstation canvas when
/// `InterfaceStyle::Aurora` is active.
pub fn paint_aurora_wave(ui: &Ui, rect: Rect, pal: &MagicPalette) {
    if pal.style != InterfaceStyle::Aurora || rect.width() < 120.0 || rect.height() < 80.0 {
        return;
    }
    let steps = 36usize;
    let base_y = rect.max.y - 16.0;
    let mut pts = Vec::with_capacity(steps + 1);
    for i in 0..=steps {
        let t = (i as f32) / (steps as f32);
        let x = rect.min.x + t * rect.width();
        let wave = (t * std::f32::consts::TAU * 1.15).sin() * 6.0;
        pts.push(Pos2::new(x, base_y - wave));
    }
    let glow = pal.accent_violet.gamma_multiply(if pal.dark { 0.18 } else { 0.10 });
    let line = pal.accent_emerald.gamma_multiply(if pal.dark { 0.24 } else { 0.14 });
    ui.painter().add(egui::Shape::line(pts.clone(), Stroke::new(2.4, glow)));
    ui.painter().add(egui::Shape::line(pts, Stroke::new(1.0, line)));
}
