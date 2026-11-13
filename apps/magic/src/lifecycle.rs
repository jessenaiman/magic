//! Pure-Rust document & editor draft lifecycle check before closing, restarting, or switching
//! `.magic` projects, coordinated with [`craft_quit::on_close_request`].

use std::sync::atomic::{AtomicBool, Ordering};

use magic_engine::Session;
use magic_ui_egui::UiShellState;

#[derive(Default)]
pub struct Lifecycle {
    exit_approved: AtomicBool,
    restart_requested: AtomicBool,
}

impl Lifecycle {
    pub fn approve_exit(&self, restart: bool) {
        self.restart_requested.store(restart, Ordering::Release);
        self.exit_approved.store(true, Ordering::Release);
    }

    pub fn is_approved(&self) -> bool {
        self.exit_approved.load(Ordering::Acquire)
    }

    pub fn restart_requested(&self) -> bool {
        self.restart_requested.load(Ordering::Acquire)
    }
}

pub fn verify_clean(session: &Session, ui_state: &UiShellState) -> Result<(), String> {
    if ui_state.saving {
        return Err("Wait for the editor save to finish before closing, restarting, or switching projects".into());
    }
    if !ui_state.dirty_drafts.is_empty() {
        let names: Vec<&str> = ui_state.dirty_drafts.iter().map(String::as_str).collect();
        return Err(format!("Save the editor changes before closing, restarting, or switching projects: {}", names.join(", ")));
    }
    if session.docs.iter().any(|doc| doc.dirty) {
        return Err("Save the edited project documents before closing, restarting, or switching projects".into());
    }
    if let Some(path) = &session.project_path {
        let bytes = std::fs::read(path).map_err(|e| format!("Cannot verify saved project {}: {e}; save it before continuing", path.display()))?;
        let saved: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| format!("Cannot verify saved project: {e}; save it before continuing"))?;
        if !saved.is_object() || saved["format"] != "magic.project" {
            return Err("Cannot verify saved project format; save the project before continuing".into());
        }
        if magic_engine::project::project_is_dirty(session, &saved) {
            return Err("Save project changes before closing, restarting, or switching projects".into());
        }
    }
    Ok(())
}
