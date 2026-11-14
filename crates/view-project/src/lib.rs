//! `Home` catalog and `Project` Craft file browser isolated in `magic-view-project`.

#![forbid(unsafe_code)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable))]

mod home;
mod project;
mod state;

use egui::Ui;
use magic_view_api::{PanelId, StageView, ViewCtx, ViewMode};

pub use home::MagicHomeView;
use state::ProjectViewState;

#[derive(Default)]
pub struct ProjectView {
    state: ProjectViewState,
}

impl ProjectView {
    pub fn new() -> Self {
        Self::default()
    }
}

impl StageView for ProjectView {
    fn mode(&self) -> ViewMode {
        ViewMode::Project
    }

    fn has_header(&self) -> bool {
        false
    }

    fn draw_panel(&mut self, ui: &mut Ui, ctx: &mut ViewCtx<'_>, panel: PanelId) {
        match panel {
            PanelId::Navigator => project::draw_navigator_panel(ui, ctx),
            PanelId::Canvas => project::draw_canvas_panel(&mut self.state, ui, ctx),
            PanelId::Inspector => project::draw_inspector_panel(ui, ctx),
            PanelId::Timeline => project::draw_timeline_panel(ui, ctx),
            _ => {}
        }
    }

    fn reset_for_project(&mut self) {
        self.state = ProjectViewState::default();
    }
}
