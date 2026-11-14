//! Session state for Magic Studio.
//!
//! Holds the current project in memory, its registered asset files, and the `craft-settings` store
//! (`Location::Memory` for isolated/test sessions, `Location::Auto` for user sessions), providing
//! [`Session::execute`] for single-point command dispatch.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

mod edit;
mod execute;
pub mod undo;

/// The engine session. All project changes go through [`Session::execute`].
#[derive(Debug)]
pub struct Session {
    pub cwd: PathBuf,
    pub settings: crate::settings::SessionSettings,
    pub assets: HashMap<String, Asset>,
    pub project_assets: Vec<ProjectAsset>,
    pub project_path: Option<PathBuf>,
    pub lyrics_workspace: Option<PathBuf>,
    pub video_project_path: Option<PathBuf>,
    pub project_mode: Option<String>,
    pub record_user_state: bool,
    pub control_address: Option<std::net::SocketAddr>,
    pub automation_roots: Option<crate::confinement::AutomationRoots>,
    pub project_stamp: std::cell::Cell<Option<crate::link_watch::FileStamp>>,
    pub song_audio: Option<PathBuf>,
    pub song: crate::cmd::song::song_metadata::SongMetadata,
    pub draft_approved: bool,
    pub video_target: String,
    pub active_asset: Option<String>,
    pub last_result: Option<Value>,
    pub project_saves: std::sync::atomic::AtomicU64,
    pub project_revision: u64,
    pub saved_revision: u64,
    pub history: crate::history::History,
}

impl Default for Session {
    fn default() -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            cwd,
            settings: crate::settings::SessionSettings::memory(),
            assets: HashMap::new(),
            project_assets: Vec::new(),
            project_path: None,
            lyrics_workspace: None,
            video_project_path: None,
            project_mode: None,
            record_user_state: false,
            control_address: None,
            automation_roots: None,
            project_stamp: std::cell::Cell::new(None),
            song_audio: None,
            song: crate::cmd::song::song_metadata::SongMetadata::default(),
            draft_approved: false,
            video_target: "filmcraft".into(),
            active_asset: None,
            last_result: None,
            project_saves: std::sync::atomic::AtomicU64::new(0),
            project_revision: 0,
            saved_revision: 0,
            history: crate::history::History::default(),
        }
    }
}

impl Clone for Session {
    fn clone(&self) -> Self {
        Self {
            cwd: self.cwd.clone(),
            settings: self.settings.clone(),
            assets: self.assets.clone(),
            project_assets: self.project_assets.clone(),
            project_path: self.project_path.clone(),
            lyrics_workspace: self.lyrics_workspace.clone(),
            video_project_path: self.video_project_path.clone(),
            project_mode: self.project_mode.clone(),
            record_user_state: self.record_user_state,
            control_address: self.control_address,
            automation_roots: self.automation_roots.clone(),
            project_stamp: std::cell::Cell::new(self.project_stamp.get()),
            song_audio: self.song_audio.clone(),
            song: self.song.clone(),
            draft_approved: self.draft_approved,
            video_target: self.video_target.clone(),
            active_asset: self.active_asset.clone(),
            last_result: self.last_result.clone(),
            project_saves: std::sync::atomic::AtomicU64::new(self.project_saves.load(std::sync::atomic::Ordering::Relaxed)),
            project_revision: self.project_revision,
            saved_revision: self.saved_revision,
            history: self.history.clone(),
        }
    }
}

impl Session {
    pub fn user_session() -> Self {
        Self { record_user_state: true, settings: crate::settings::SessionSettings::auto(), ..Self::default() }
    }

    pub fn with_automation_roots(mut self, roots: Option<crate::confinement::AutomationRoots>) -> Self {
        self.automation_roots = roots;
        self
    }

    pub fn is_dirty(&self) -> bool {
        self.project_path.is_some() && self.project_revision != self.saved_revision
    }

    pub fn reset_for_project(&mut self, path: Option<PathBuf>, assets: Vec<ProjectAsset>) {
        let stamp = path.as_deref().and_then(crate::link_watch::FileStamp::of);
        self.assets.clear();
        self.project_assets = assets;
        self.project_path = path;
        self.lyrics_workspace = None;
        self.video_project_path = None;
        self.project_mode = None;
        self.project_stamp.set(stamp);
        self.song_audio = None;
        self.song = crate::cmd::song::song_metadata::SongMetadata::default();
        self.draft_approved = false;
        self.video_target = "filmcraft".into();
        self.active_asset = None;
        self.last_result = None;
        self.project_saves.store(0, std::sync::atomic::Ordering::Relaxed);
        self.project_revision = 0;
        self.saved_revision = 0;
        self.history.clear();
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
