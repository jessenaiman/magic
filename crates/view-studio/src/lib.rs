//! `05 Studio` isolated stage view crate implementing [`StageView`].

#![forbid(unsafe_code)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable))]

mod stage;
mod state;

use egui::Ui;
use magic_view_api::{CachedPayloads, PanelId, StageView, ViewCtx, ViewMode};
use state::StudioViewState;

#[derive(Default)]
pub struct StudioView {
    state: StudioViewState,
}

impl StudioView {
    pub fn new() -> Self {
        Self::default()
    }
}

impl StageView for StudioView {
    fn mode(&self) -> ViewMode {
        ViewMode::Studio
    }

    fn draw_header(&mut self, ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
        stage::draw_stage_header(&mut self.state, ui, ctx);
    }

    fn draw_panel(&mut self, ui: &mut Ui, ctx: &mut ViewCtx<'_>, panel: PanelId) {
        match panel {
            PanelId::Navigator => stage::draw_navigator_panel(&mut self.state, ui, ctx),
            PanelId::Canvas => stage::draw_canvas_panel(&mut self.state, ui, ctx),
            PanelId::Inspector => stage::draw_inspector_panel(&mut self.state, ui, ctx),
            PanelId::Timeline => stage::draw_timeline_panel(&mut self.state, ui, ctx),
            PanelId::Scopes => stage::draw_scopes_summary(&self.state, ui, ctx),
            _ => {}
        }
    }

    fn reset_for_project(&mut self) {
        self.state = StudioViewState::default();
    }

    fn playhead_seconds(&self) -> f64 {
        self.state.seek_seconds
    }

    fn set_playhead_seconds(&mut self, sec: f64) {
        self.state.seek_seconds = sec;
    }

    fn sync_from_song_time(&mut self, song_sec: f64, _cache: &CachedPayloads) {
        self.state.seek_seconds = song_sec;
    }

    fn inspector_tab(&self) -> &str {
        &self.state.inspector_tab
    }

    fn set_inspector_tab(&mut self, tab: &str) {
        self.state.inspector_tab = tab.to_owned();
    }
}
