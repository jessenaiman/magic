//! Shared contracts, docking tokens, palette, widgets, and timeline primitives
//! isolating every Magic Studio stage view into its own crate.

#![forbid(unsafe_code)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable))]

pub mod controls;
pub mod docking;
pub mod editorial;
pub mod filmbar;
pub mod fonts_ui;
pub mod icons;
pub mod state;
pub mod theme;
pub mod timeline;
pub mod widgets;

use std::path::PathBuf;

use egui::{Response, Ui};

pub use docking::{DockFrameOutcome, DockWorkspaceState, PRESET_NAMES, PanelId};
pub use state::{ActiveModal, AiUiState, CachedPayloads, PickerTarget, SharedTimelineState, UiAction, UiShellState, ViewCtx, ViewMode};
pub use theme::{AccentChoice, InterfaceStyle, MagicPalette, ThemeMode};
pub use widgets::{AutomationRegistry, BtnSpec, action_button};

/// Contract implemented by every isolated Magic Studio stage view crate (`01 Storyboard`,
/// `02 Animations`, `03 Lyrics`, `04 Motion`, `05 Studio`, and `Project`).
pub trait StageView {
    /// Mode tab handled by this view.
    fn mode(&self) -> ViewMode;

    /// Whether the workstation renders a compact stage ribbon above the dock area.
    fn has_header(&self) -> bool {
        true
    }

    /// Whether this stage has a playable timeline panel (false for standalone stages such as `01 Storyboard`).
    fn has_timeline(&self) -> bool {
        true
    }

    /// Dock tab title override for `panel` when this stage is active.
    fn panel_title(&self, panel: PanelId) -> &'static str {
        panel.title(self.mode())
    }

    /// Draw the stage header strip above the docking area when `has_header()` is true.
    fn draw_header(&mut self, ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
        let _ = (ui, ctx);
    }

    /// Draw the requested docking panel body (`Canvas`, `Navigator`, `Inspector`, `Timeline`, or stage-owned panels).
    fn draw_panel(&mut self, ui: &mut Ui, ctx: &mut ViewCtx<'_>, panel: PanelId);

    /// Clear per-project draft buffers and transient UI selection when opening or closing a project.
    fn reset_for_project(&mut self);

    /// Current stage playhead position in seconds (used when syncing transport across views).
    fn playhead_seconds(&self) -> f64 {
        0.0
    }

    /// Update the stage playhead position in seconds.
    fn set_playhead_seconds(&mut self, sec: f64) {
        let _ = sec;
    }

    /// Follow master song transport playback when `audio_playing` is active.
    fn sync_from_song_time(&mut self, song_sec: f64, cache: &CachedPayloads) {
        let _ = (song_sec, cache);
    }

    /// Select an authored action / clip at a local offset in seconds.
    fn select_action_at(&mut self, action: &str, local_sec: f64) {
        let _ = (action, local_sec);
    }

    /// Whether transport should play the master FilmCraft sequence audio rather than standalone preview audio.
    fn uses_filmcraft_audio(&self) -> bool {
        true
    }

    /// Active inspector sub-tab ID for UI state inspection (`ui.state`).
    fn inspector_tab(&self) -> &str {
        ""
    }

    /// Set the active inspector sub-tab ID from control requests.
    fn set_inspector_tab(&mut self, tab: &str) {
        let _ = tab;
    }

    /// Active bottom/dock sub-tab ID for UI state inspection (`ui.state`).
    fn dock_tab(&self) -> &str {
        ""
    }

    /// Set the active bottom/dock sub-tab ID from control requests.
    fn set_dock_tab(&mut self, tab: &str) {
        let _ = tab;
    }

    /// Handle a completed file dialog pick targeted at this stage; return `true` if handled.
    fn on_picked_file(&mut self, target: &PickerTarget, path: &str, ctx: &mut ViewCtx<'_>) -> bool {
        let _ = (target, path, ctx);
        false
    }

    /// Handle OS drag-and-drop file drops while this stage is active.
    fn on_dropped_files(&mut self, paths: &[PathBuf], ctx: &mut ViewCtx<'_>) {
        let _ = (paths, ctx);
    }
}

/// Contract for the no-project Home / Catalog view (`crates/view-project`).
pub trait HomeView {
    /// Draw the full Home Catalog & New Project workspace.
    fn show(&mut self, ui: &mut Ui, ctx: &mut ViewCtx<'_>);

    /// Expand and focus the New Project creation form.
    fn open_new_form(&mut self);

    /// Route `NewProjectRoot` / `NewProjectAudio` file dialog results into the form.
    fn on_picked_file(&mut self, target: &PickerTarget, path: &str) -> bool;
}

/// Render an unwired control visibly disabled (`enabled(false)`) instead of wiring fake toast handlers.
/// Callers place `// TODO: Read and find the docs` directly above each call site.
pub fn disabled_todo_button(ui: &mut Ui, ctx: &mut ViewCtx<'_>, id: &str, icon: &str, label: &str, tooltip: &str) -> Response {
    action_button(ui, ctx.reg, ctx.pal, BtnSpec::new(id, icon, label, tooltip).enabled(false))
}
