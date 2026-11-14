//! `03 Lyrics` isolated stage view crate implementing [`StageView`].

#![forbid(unsafe_code)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable))]

mod dock;
mod inspector;
mod stage;
mod state;

use egui::Ui;
use magic_view_api::{CachedPayloads, PanelId, StageView, ViewCtx, ViewMode};
use serde_json::Value;
use state::LyricsViewState;

#[derive(Default)]
pub struct LyricsView {
    state: LyricsViewState,
}

impl LyricsView {
    pub fn new() -> Self {
        Self::default()
    }
}

impl StageView for LyricsView {
    fn mode(&self) -> ViewMode {
        ViewMode::Design
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
        self.state = LyricsViewState::default();
    }

    fn playhead_seconds(&self) -> f64 {
        self.state.scrub_seconds
    }

    fn set_playhead_seconds(&mut self, sec: f64) {
        self.state.scrub_seconds = sec;
    }

    fn sync_from_song_time(&mut self, song_sec: f64, cache: &CachedPayloads) {
        self.state.scrub_seconds = song_sec;
        if let Some(anims) = cache.words_doc.pointer("/document/animations").and_then(Value::as_array) {
            for anim in anims {
                let st = anim["song_anchor"].as_f64().unwrap_or(0.0);
                let dur = anim["authored_duration"].as_f64().unwrap_or(3.0).max(0.25);
                if song_sec >= st
                    && song_sec <= st + dur
                    && let Some(aid) = anim["animation_id"].as_str()
                {
                    self.state.selected_anim_id = Some(aid.to_owned());
                    break;
                }
            }
        }
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
}
