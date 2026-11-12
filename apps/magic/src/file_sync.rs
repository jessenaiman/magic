//! Live Craft file synchronization (`craft-shared` Phases 3, 4 & 5):
//! - Polls `(size, mtime_ms)` of project `.ecproj` tracks (`V2` Character Animation, `V3` Kinetic
//!   Lyrics, `V4` Action Motion, plus `film/linked-compositions.json`) on focus regain and stage
//!   saves, populating `ui_state.dirty_tracks`.
//! - Routes dropped media onto the active stage's `.ecproj` via `project.import_file` when
//!   on `Animations`, `Lyrics`, or `Motion`.
//! - Runs ~1-second debounced crash-recovery journal auto-save (`project.autosave`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, UNIX_EPOCH};

use magic::Session;
use magic_ui_egui::{UiShellState, ViewMode};
use serde_json::{Value, json};

const AUTOSAVE_DEBOUNCE: Duration = Duration::from_millis(1000);

#[derive(Default)]
pub(crate) struct FileSyncMonitor {
    stamps: BTreeMap<PathBuf, (u64, u64)>,
    was_focused: bool,
    pending_autosave_since: Option<Instant>,
}

fn stamp_of(path: &Path) -> Option<(u64, u64)> {
    let meta = std::fs::metadata(path).ok()?;
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_millis() as u64);
    Some((meta.len(), mtime))
}

fn active_character_ecproj(ui_state: &UiShellState) -> Option<PathBuf> {
    let chars = ui_state.cache.characters["characters"].as_array()?;
    let chosen = match &ui_state.selected_character_id {
        Some(cid) => chars.iter().find(|c| c["id"].as_str() == Some(cid.as_str())),
        None => chars.first(),
    }?;
    chosen
        .pointer("/views/0/ecproj_path")
        .and_then(Value::as_str)
        .or_else(|| chosen.pointer("/action_ecprojs/0/path").and_then(Value::as_str))
        .map(PathBuf::from)
}

impl FileSyncMonitor {
    pub fn seed_project(&mut self, session: &mut Session, ui_state: &mut UiShellState) {
        self.stamps.clear();
        ui_state.dirty_tracks.clear();
        let Some(proj_dir) = ui_state.project_path.as_deref().and_then(Path::parent) else {
            return;
        };
        for rel in ["lettering/lyrics.ecproj", "motion/actions.ecproj"] {
            let p = proj_dir.join(rel);
            if let Some(st) = stamp_of(&p) {
                self.stamps.insert(p, st);
            }
        }
        if let Some(ec) = active_character_ecproj(ui_state)
            && let Some(st) = stamp_of(&ec)
        {
            self.stamps.insert(ec, st);
        }
        if let Ok(sync) = session.execute("video.sync_links", json!({"render": false}))
            && let Some(arr) = sync["dirty_tracks"].as_array()
        {
            for item in arr.iter().filter_map(Value::as_str) {
                ui_state.dirty_tracks.insert(item.to_owned());
            }
        }
    }

    pub fn note_mutation(&mut self, cmd_id: &str, ui_state: &mut UiShellState) {
        if ui_state.project_path.is_some() && !matches!(cmd_id, "project.save" | "file.save" | "file.saveAs" | "project.autosave") {
            self.pending_autosave_since = Some(Instant::now());
        } else if matches!(cmd_id, "project.save" | "file.save" | "file.saveAs") {
            self.pending_autosave_since = None;
        }
        if matches!(cmd_id, "words.save" | "words.motion.set" | "words.design") {
            ui_state.dirty_tracks.insert("V3".into());
        } else if cmd_id == "motion.save" {
            ui_state.dirty_tracks.insert("V4".into());
        } else if cmd_id.starts_with("character.") && !matches!(cmd_id, "character.publish" | "character.shot") {
            ui_state.dirty_tracks.insert("V2".into());
        } else if matches!(cmd_id, "words.letter" | "words.publish") {
            ui_state.dirty_tracks.remove("V3");
        } else if cmd_id == "motion.publish" {
            ui_state.dirty_tracks.remove("V4");
        } else if matches!(cmd_id, "character.publish" | "video.assemble" | "video.sync_links") {
            ui_state.dirty_tracks.clear();
        }
    }

    pub fn poll_frame(&mut self, ctx: &egui::Context, session: &mut Session, ui_state: &mut UiShellState) {
        let focused = ctx.input(|i| i.focused);
        let regained_focus = focused && !self.was_focused;
        self.was_focused = focused;

        if regained_focus {
            self.check_disk_ecprojs(session, ui_state);
        }

        if let Some(since) = self.pending_autosave_since {
            if since.elapsed() >= AUTOSAVE_DEBOUNCE {
                self.pending_autosave_since = None;
                let _ = session.execute("project.autosave", json!({}));
            } else {
                ctx.request_repaint_after(Duration::from_millis(250));
            }
        }
    }

    pub fn check_disk_ecprojs(&mut self, session: &mut Session, ui_state: &mut UiShellState) {
        let Some(proj_dir) = ui_state.project_path.as_deref().and_then(Path::parent).map(Path::to_path_buf) else {
            return;
        };
        let mut candidates = vec![
            ("V3", proj_dir.join("lettering/lyrics.ecproj")),
            ("V4", proj_dir.join("motion/actions.ecproj")),
        ];
        if let Some(ec) = active_character_ecproj(ui_state) {
            candidates.push(("V2", ec));
        }
        for (track, path) in candidates {
            let Some(now_stamp) = stamp_of(&path) else {
                continue;
            };
            match self.stamps.insert(path, now_stamp) {
                Some(prev) if prev != now_stamp => {
                    ui_state.dirty_tracks.insert(track.into());
                }
                _ => {}
            }
        }
        if let Ok(sync) = session.execute("video.sync_links", json!({"render": false}))
            && let Some(arr) = sync["dirty_tracks"].as_array()
        {
            for item in arr.iter().filter_map(Value::as_str) {
                ui_state.dirty_tracks.insert(item.to_owned());
            }
        }
    }
}

/// Target `.ecproj` when dropping media directly onto `Animations`, `Lyrics`, or `Motion`.
pub(crate) fn stage_ecproj_for_drop(ui_state: &UiShellState) -> Option<PathBuf> {
    let proj_dir = ui_state.project_path.as_deref()?.parent()?;
    match ui_state.effective_view() {
        ViewMode::Animation => active_character_ecproj(ui_state).filter(|p| p.is_file()),
        ViewMode::Design => {
            let p = proj_dir.join("lettering/lyrics.ecproj");
            p.is_file().then_some(p)
        }
        ViewMode::Motion => {
            let p = proj_dir.join("motion/actions.ecproj");
            p.is_file().then_some(p)
        }
        _ => None,
    }
}
