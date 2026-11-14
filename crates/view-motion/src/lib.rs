//! `04 Motion` isolated stage view crate implementing [`StageView`].

#![forbid(unsafe_code)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable))]

mod dock;
mod inspector;
mod stage;
mod state;

use egui::Ui;
use magic_view_api::{CachedPayloads, PanelId, PickerTarget, StageView, ViewCtx, ViewMode};
use serde_json::json;
use state::MotionViewState;

#[derive(Default)]
pub struct MotionView {
    state: MotionViewState,
}

impl MotionView {
    pub fn new() -> Self {
        Self::default()
    }
}

impl StageView for MotionView {
    fn mode(&self) -> ViewMode {
        ViewMode::Motion
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
            _ => {}
        }
    }

    fn reset_for_project(&mut self) {
        self.state = MotionViewState::default();
    }

    fn playhead_seconds(&self) -> f64 {
        self.state.start_sec
    }

    fn set_playhead_seconds(&mut self, sec: f64) {
        self.state.start_sec = sec;
    }

    fn sync_from_song_time(&mut self, song_sec: f64, _cache: &CachedPayloads) {
        self.state.start_sec = song_sec;
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

    fn on_picked_file(&mut self, target: &PickerTarget, path: &str, ctx: &mut ViewCtx<'_>) -> bool {
        if matches!(target, PickerTarget::ImportLottie) {
            self.state.lottie_show_warnings = true;
            ctx.exec("project.import_file", json!({"path": path}), None);
            return true;
        }
        false
    }
}
