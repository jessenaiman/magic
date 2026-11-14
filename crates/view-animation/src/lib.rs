//! `02 Animations` isolated stage view crate implementing [`StageView`].

#![forbid(unsafe_code)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable))]

mod dock;
mod inspector;
mod stage;
mod state;

use egui::Ui;
use magic_view_api::{CachedPayloads, PanelId, StageView, ViewCtx, ViewMode};
use state::AnimationViewState;

#[derive(Default)]
pub struct AnimationView {
    state: AnimationViewState,
}

impl AnimationView {
    pub fn new() -> Self {
        Self::default()
    }
}

impl StageView for AnimationView {
    fn mode(&self) -> ViewMode {
        ViewMode::Animation
    }

    fn draw_header(&mut self, ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
        stage::draw_stage_header(ui, ctx);
    }

    fn draw_panel(&mut self, ui: &mut Ui, ctx: &mut ViewCtx<'_>, panel: PanelId) {
        match panel {
            PanelId::Navigator => stage::draw_navigator_panel(&mut self.state, ui, ctx),
            PanelId::Canvas => stage::draw_canvas_panel(&mut self.state, ui, ctx),
            PanelId::Inspector => stage::draw_inspector_panel(&mut self.state, ui, ctx),
            PanelId::Timeline => stage::draw_timeline_panel(&mut self.state, ui, ctx),
            _ => {}
        }
    }

    fn reset_for_project(&mut self) {
        self.state = AnimationViewState::default();
    }

    fn playhead_seconds(&self) -> f64 {
        self.state.scrub_seconds
    }

    fn set_playhead_seconds(&mut self, sec: f64) {
        self.state.scrub_seconds = sec;
    }

    fn sync_from_song_time(&mut self, song_sec: f64, _cache: &CachedPayloads) {
        if self.state.track_source == "effectcraft" && self.state.loop_clip {
            let cyc = self.state.action_duration.max(0.4);
            self.state.scrub_seconds = song_sec % cyc;
        } else {
            self.state.scrub_seconds = song_sec;
        }
    }

    fn select_action_at(&mut self, action: &str, local_sec: f64) {
        if !action.is_empty() {
            self.state.selected_action = action.to_owned();
        }
        self.state.scrub_seconds = local_sec;
    }

    fn uses_filmcraft_audio(&self) -> bool {
        self.state.audio_source == "filmcraft"
    }

    fn inspector_tab(&self) -> &str {
        &self.state.inspector_tab
    }

    fn set_inspector_tab(&mut self, tab: &str) {
        self.state.inspector_tab = tab.to_owned();
    }

    fn dock_tab(&self) -> &str {
        &self.state.dock_tab
    }

    fn set_dock_tab(&mut self, tab: &str) {
        self.state.dock_tab = tab.to_owned();
    }
}
