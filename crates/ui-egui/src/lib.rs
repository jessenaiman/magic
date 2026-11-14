//! `egui` + `storytold/craft-libs` workstation shell hosting isolated per-stage view crates
//! through [`StageView`] and [`HomeView`] contracts in [`magic_view_api`].

#![forbid(unsafe_code)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable))]

use craft_ui::fonts::{CraftFace, I18nScripts, UiFonts};
use magic_view_animation::AnimationView;
use magic_view_lyrics::LyricsView;
use magic_view_motion::MotionView;
use magic_view_project::{MagicHomeView, ProjectView};
use magic_view_storyboard::StoryboardView;
use magic_view_studio::StudioView;

pub use magic_view_api as view_api;
pub use magic_view_api::{
    AccentChoice, ActiveModal, AutomationRegistry, BtnSpec, CachedPayloads, HomeView, InterfaceStyle, MagicPalette, OptionsUiState, PickerTarget,
    SharedTimelineState, StageView, ThemeMode, UiAction, UiShellState, ViewCtx, ViewMode, action_button, controls, disabled_todo_button, editorial, filmbar,
    fonts_ui, icons, state, theme, timeline, widgets,
};

pub mod docking;
pub mod menus;
pub mod modals;
pub mod options_modal;
pub mod shell;

pub use docking::{DockFrameOutcome, DockWorkspaceState, PRESET_NAMES as WORKSPACE_PRESETS, PanelId};

/// Owns the isolated view crates (`Project`, `01 Storyboard`, `02 Animations`, `03 Lyrics`,
/// `04 Motion`, `05 Studio`).
pub struct StageHost {
    home: MagicHomeView,
    project: ProjectView,
    storyboard: StoryboardView,
    animation: AnimationView,
    lyrics: LyricsView,
    motion: MotionView,
    studio: StudioView,
}

impl Default for StageHost {
    fn default() -> Self {
        Self::new()
    }
}

impl StageHost {
    pub fn new() -> Self {
        Self {
            home: MagicHomeView::new(),
            project: ProjectView::new(),
            storyboard: StoryboardView::new(),
            animation: AnimationView::new(),
            lyrics: LyricsView::new(),
            motion: MotionView::new(),
            studio: StudioView::new(),
        }
    }

    pub fn home(&self) -> &dyn HomeView {
        &self.home
    }

    pub fn home_mut(&mut self) -> &mut dyn HomeView {
        &mut self.home
    }

    pub fn stage(&self, mode: ViewMode) -> &dyn StageView {
        match mode {
            ViewMode::Home | ViewMode::Project => &self.project,
            ViewMode::Storyboard => &self.storyboard,
            ViewMode::Animation => &self.animation,
            ViewMode::Design => &self.lyrics,
            ViewMode::Motion => &self.motion,
            ViewMode::Studio => &self.studio,
        }
    }

    pub fn stage_mut(&mut self, mode: ViewMode) -> &mut dyn StageView {
        match mode {
            ViewMode::Home | ViewMode::Project => &mut self.project,
            ViewMode::Storyboard => &mut self.storyboard,
            ViewMode::Animation => &mut self.animation,
            ViewMode::Design => &mut self.lyrics,
            ViewMode::Motion => &mut self.motion,
            ViewMode::Studio => &mut self.studio,
        }
    }

    pub fn reset_for_project_switch(&mut self) {
        self.project.reset_for_project();
        self.storyboard.reset_for_project();
        self.animation.reset_for_project();
        self.lyrics.reset_for_project();
        self.motion.reset_for_project();
        self.studio.reset_for_project();
    }
}

/// Install `egui_extras` image/SVG loaders and `craft_ui::fonts::UiFonts` (`system_fonts` +
/// `color_fonts` script font chain shared with `craft-font-catalog`).
pub fn install_loaders(ctx: &egui::Context) {
    egui_extras::install_image_loaders(ctx);
    let lang = magic_util::active_language_code();
    let faces: Vec<CraftFace> = magic_util::CRAFT_FONTS.iter().copied().map(CraftFace::from).collect();
    UiFonts::new(&I18nScripts).ui_language(lang).craft_faces(&faces).install(ctx);
    craft_ui::direction::apply(ctx, craft_ui::direction::resolve(lang, craft_ui::direction::Setting::Language));
}
