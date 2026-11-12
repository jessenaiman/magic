<<<<<<< HEAD
//! Pure-Rust `egui` 0.37 + `storytold/craft-libs` workstation shell for Magic Studio.
//!
//! Built on `craft_ui::fonts::UiFonts`, `craft_ui::theme::WidgetPalette` (`docs/theme-infrastructure.md`),
//! `craft-menu`, `craft-window`, shared anti-slop editorial workstation primitives (`editorial.rs`),
//! shared Craft-document timelines (`timeline.rs`), MailCraft-inspired `Options` & quick diagnostics
//! (`options_modal.rs`), workstation modals (`shell_bridge.rs`), and custom GPU `egui::Painter`
//! controls (`controls.rs`, `filmbar.rs`).
=======
//! `egui` + `storytold/craft-libs` workstation shell for Magic Studio.
//!
//! Built on [`craft_ui::docking`] (`docs/craft-reference/craft-libs/docs/ui-docking.md`),
//! [`craft_ui::fonts::UiFonts`], [`craft_ui::theme::WidgetPalette`], [`craft_menu`],
//! [`craft_window`], [`craft_font_manager::FontsPanel`], shared Craft-document timelines
//! (`timeline.rs`), workstation `Options` & diagnostics (`options_modal.rs`, `modals.rs`), and
//! GPU `egui::Painter` controls (`controls.rs`, `filmbar.rs`).
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)

#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable))]

use craft_ui::fonts::{I18nScripts, UiFonts};

pub mod controls;
<<<<<<< HEAD
pub mod editorial;
pub mod filmbar;
pub mod icons;
pub mod menus;
pub mod options_modal;
pub mod shell;
pub mod shell_bridge;
=======
pub mod docking;
pub mod editorial;
pub mod filmbar;
pub mod fonts_ui;
pub mod icons;
pub mod menus;
pub mod modals;
pub mod options_modal;
pub mod shell;
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
pub mod state;
pub mod state_views;
pub mod theme;
pub mod timeline;
pub mod views;
pub mod widgets;

<<<<<<< HEAD
=======
pub use docking::{DockFrameOutcome, DockWorkspaceState, PRESET_NAMES as WORKSPACE_PRESETS, PanelId};
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
pub use state::{ActiveModal, PickerTarget, UiAction, UiShellState, ViewCtx, ViewMode};
pub use theme::{AccentChoice, InterfaceStyle, MagicPalette, ThemeMode};
pub use widgets::AutomationRegistry;

/// Install `egui_extras` image/SVG loaders and `craft_ui::fonts::UiFonts` (`system_fonts` +
/// `color_fonts` script font chain shared with `craft-font-catalog`).
pub fn install_loaders(ctx: &egui::Context) {
    egui_extras::install_image_loaders(ctx);
    UiFonts::new(&I18nScripts).ui_language("en").install(ctx);
}
