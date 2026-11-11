//! Session state for Magic Studio.
//!
//! Holds the current project and its registered asset files, and provides
//! [`Session::execute`] for command dispatch. Pixel, vector, layout, and motion work belongs to
//! the installed ArtCraft apps; the session only records which files a project uses.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

mod documents;
mod edit;
mod execute;
pub mod undo;

/// The engine session. All project changes go through [`Session::execute`].
#[derive(Debug)]
pub struct Session {
    pub cwd: PathBuf,
    pub assets: HashMap<String, Asset>,
    pub project_assets: Vec<ProjectAsset>,
    pub project_path: Option<PathBuf>,
    pub lyrics_workspace: Option<PathBuf>,
    pub video_project_path: Option<PathBuf>,
    pub project_mode: Option<String>,
    pub record_user_state: bool,
    pub control_address: Option<std::net::SocketAddr>,
    pub automation_roots: Option<crate::confinement::AutomationRoots>,
    pub agent_selection: Option<Value>,
    pub project_stamp: std::cell::Cell<Option<crate::link_watch::FileStamp>>,
    pub song_audio: Option<PathBuf>,
    pub song: crate::cmd::song::song_metadata::SongMetadata,
    pub draft_approved: bool,
    pub video_target: String,
    pub active_asset: Option<String>,
    pub last_result: Option<Value>,
    pub project_saves: std::sync::atomic::AtomicU64,
    pub edits_since_save: u64,
    pub docs: Vec<crate::doc_state::DocState>,
    pub history: crate::history::History,
    pub step_base: Vec<crate::doc_state::DocState>,
}

impl Default for Session {
    fn default() -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            cwd,
            assets: HashMap::new(),
            project_assets: Vec::new(),
            project_path: None,
            lyrics_workspace: None,
            video_project_path: None,
            project_mode: None,
            record_user_state: false,
            control_address: None,
            automation_roots: None,
            agent_selection: None,
            project_stamp: std::cell::Cell::new(None),
            song_audio: None,
            song: crate::cmd::song::song_metadata::SongMetadata::default(),
            draft_approved: false,
            video_target: "filmcraft".into(),
            active_asset: None,
            last_result: None,
            project_saves: std::sync::atomic::AtomicU64::new(0),
            edits_since_save: 0,
            docs: Vec::new(),
            history: crate::history::History::default(),
            step_base: Vec::new(),
        }
    }
}

impl Clone for Session {
    fn clone(&self) -> Self {
        Self {
            cwd: self.cwd.clone(),
            assets: self.assets.clone(),
            project_assets: self.project_assets.clone(),
            project_path: self.project_path.clone(),
            lyrics_workspace: self.lyrics_workspace.clone(),
            video_project_path: self.video_project_path.clone(),
            project_mode: self.project_mode.clone(),
            record_user_state: self.record_user_state,
            control_address: self.control_address,
            automation_roots: self.automation_roots.clone(),
            agent_selection: self.agent_selection.clone(),
            project_stamp: std::cell::Cell::new(self.project_stamp.get()),
            song_audio: self.song_audio.clone(),
            song: self.song.clone(),
            draft_approved: self.draft_approved,
            video_target: self.video_target.clone(),
            active_asset: self.active_asset.clone(),
            last_result: self.last_result.clone(),
            project_saves: std::sync::atomic::AtomicU64::new(self.project_saves.load(std::sync::atomic::Ordering::Relaxed)),
            edits_since_save: self.edits_since_save,
            docs: self.docs.clone(),
            history: self.history.clone(),
            step_base: Vec::new(),
        }
    }
}

impl Session {
    pub fn user_session() -> Self {
        Self { record_user_state: true, ..Self::default() }
    }

    pub fn with_automation_roots(mut self, roots: Option<crate::confinement::AutomationRoots>) -> Self {
        self.automation_roots = roots;
        self
    }

    pub fn record(&mut self, value: Value) {
        self.last_result = Some(value);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectAsset {
    pub id: String,
    pub name: String,
    pub preset: Option<String>,
    pub path: Option<PathBuf>,
    pub kind: String,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub id: String,
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub has_alpha: bool,
}
