//! Pure-Rust `egui` 0.37 + `storytold/craft-libs` workstation shell for Magic Studio.
//!
//! Built on `craft_ui::theme::WidgetPalette` (`docs/theme-infrastructure.md`), `craft-menu`,
//! `craft-window`, and custom GPU `egui::Painter` controls (`controls.rs`: 35mm filmbar sprocket
//! borders, horizontal & vertical hollow-ring sliders with compact inline monospace values, 360°
//! rotary angle dials, compact pill toggles, and Storyboard red/amber/green requirement rows).

#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable))]

pub mod controls;
pub mod icons;
pub mod menus;
pub mod shell;
pub mod state;
pub mod theme;
pub mod views;
pub mod widgets;

pub use state::{ActiveModal, PickerTarget, UiAction, UiShellState, ViewCtx, ViewMode};
pub use theme::{MagicPalette, ThemeMode};
pub use widgets::AutomationRegistry;

pub fn install_loaders(ctx: &egui::Context) {
    egui_extras::install_image_loaders(ctx);
}
