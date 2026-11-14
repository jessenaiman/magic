//! In-memory coordinator [`Session`] holding the active `.magic` v1 [`Project`], revision
//! counters for dirty tracking, and snapshot [`History`] for undo/redo.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::cmd::project::format::Project;
use crate::cmd::song::song_metadata::SongMetadata;
use crate::{EngineError, Result};

mod edit;
mod execute;
pub mod undo;

/// The engine session. All project changes go through [`Session::execute`].
#[derive(Debug)]
pub struct Session {
    pub cwd: PathBuf,
    pub settings: crate::settings::SessionSettings,
    pub assets: HashMap<String, Asset>,
    pub project_path: Option<PathBuf>,
    pub project: Option<Project>,
    pub lyrics_workspace: Option<PathBuf>,
    pub standalone_audio: Option<PathBuf>,
    pub record_user_state: bool,
    pub control_address: Option<std::net::SocketAddr>,
    pub automation_roots: Option<crate::confinement::AutomationRoots>,
    pub project_stamp: std::cell::Cell<Option<crate::link_watch::FileStamp>>,
    pub active_asset: Option<String>,
    pub last_result: Option<Value>,
    pub project_saves: std::sync::atomic::AtomicU64,
    pub project_revision: u64,
    pub next_revision: u64,
    pub saved_revision: u64,
    pub history: crate::history::History,
    pub recovery_run: Option<crate::recovery::RecoveryRun>,
    pub last_autosave_at: Option<std::time::Instant>,
    pub last_autosave_revision: Option<u64>,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            settings: crate::settings::SessionSettings::default(),
            assets: HashMap::new(),
            project_path: None,
            project: None,
            lyrics_workspace: None,
            standalone_audio: None,
            record_user_state: false,
            control_address: None,
            automation_roots: None,
            project_stamp: std::cell::Cell::new(None),
            active_asset: None,
            last_result: None,
            project_saves: std::sync::atomic::AtomicU64::new(0),
            project_revision: 0,
            next_revision: 0,
            saved_revision: 0,
            history: crate::history::History::default(),
            recovery_run: None,
            last_autosave_at: None,
            last_autosave_revision: None,
        }
    }
}

impl Clone for Session {
    fn clone(&self) -> Self {
        Self {
            cwd: self.cwd.clone(),
            settings: self.settings.clone(),
            assets: self.assets.clone(),
            project_path: self.project_path.clone(),
            project: self.project.clone(),
            lyrics_workspace: self.lyrics_workspace.clone(),
            standalone_audio: self.standalone_audio.clone(),
            record_user_state: self.record_user_state,
            control_address: self.control_address,
            automation_roots: self.automation_roots.clone(),
            project_stamp: std::cell::Cell::new(self.project_stamp.get()),
            active_asset: self.active_asset.clone(),
            last_result: self.last_result.clone(),
            project_saves: std::sync::atomic::AtomicU64::new(self.project_saves.load(std::sync::atomic::Ordering::Relaxed)),
            project_revision: self.project_revision,
            next_revision: self.next_revision,
            saved_revision: self.saved_revision,
            history: self.history.clone(),
            recovery_run: None,
            last_autosave_at: None,
            last_autosave_revision: None,
        }
    }
}

impl Session {
    pub fn user_session() -> Self {
        Self { settings: crate::settings::SessionSettings::auto(), record_user_state: true, ..Self::default() }
    }

    pub fn with_automation_roots(mut self, roots: Option<crate::confinement::AutomationRoots>) -> Self {
        self.automation_roots = roots;
        self
    }

    pub fn is_dirty(&self) -> bool {
        self.project_path.is_some() && self.project.is_some() && self.project_revision != self.saved_revision
    }

    pub fn project(&self) -> Option<&Project> {
        self.project.as_ref()
    }

    pub fn project_mut(&mut self) -> Option<&mut Project> {
        self.project.as_mut()
    }

    pub fn require_project(&self) -> Result<&Project> {
        self.project.as_ref().ok_or_else(|| EngineError::InvalidParams("no project is open; open or create a .magic project first".into()))
    }

    pub fn require_project_mut(&mut self) -> Result<&mut Project> {
        self.project.as_mut().ok_or_else(|| EngineError::InvalidParams("no project is open; open or create a .magic project first".into()))
    }

    pub fn project_assets(&self) -> &[ProjectAsset] {
        self.project.as_ref().map_or(&[], |p| p.assets.as_slice())
    }

    pub fn song_audio(&self) -> Option<&Path> {
        self.project.as_ref().and_then(|p| p.workshop.audio.as_deref()).or(self.standalone_audio.as_deref())
    }

    pub fn song_metadata(&self) -> Option<&SongMetadata> {
        self.project.as_ref().map(|p| &p.song)
    }

    pub fn video_target(&self) -> &str {
        self.project.as_ref().map_or("filmcraft", |p| p.workshop.target.as_str())
    }

    pub fn video_project_path(&self) -> Option<&Path> {
        self.project.as_ref().and_then(|p| p.workshop.video_project.as_deref())
    }

    pub fn draft_approved(&self) -> bool {
        self.project.as_ref().is_some_and(|p| p.workshop.draft_approved)
    }

    pub fn project_mode(&self) -> Option<&str> {
        self.project.as_ref().and_then(|p| p.mode.as_deref())
    }

    pub fn clear_active_recovery(&mut self) {
        crate::recovery::clear_active(self);
    }

    pub fn reset_for_opened_project(&mut self, path: PathBuf, project: Project) {
        self.clear_active_recovery();
        self.assets.clear();
        self.project_stamp.set(path.is_file().then(|| crate::link_watch::FileStamp::of(&path)).flatten());
        self.project_path = Some(path);
        self.project = Some(project);
        self.lyrics_workspace = None;
        self.standalone_audio = None;
        self.active_asset = None;
        self.last_result = None;
        self.project_revision = 0;
        self.next_revision = 0;
        self.saved_revision = 0;
        self.last_autosave_at = None;
        self.last_autosave_revision = None;
        self.history.clear();
    }

    pub fn close_project(&mut self) {
        self.clear_active_recovery();
        self.assets.clear();
        self.project_path = None;
        self.project = None;
        self.lyrics_workspace = None;
        self.standalone_audio = None;
        self.project_stamp.set(None);
        self.active_asset = None;
        self.last_result = None;
        self.project_revision = 0;
        self.next_revision = 0;
        self.saved_revision = 0;
        self.last_autosave_at = None;
        self.last_autosave_revision = None;
        self.history.clear();
    }

    pub fn set_standalone_lyrics_workspace(&mut self, folder: Option<PathBuf>, audio: Option<PathBuf>) {
        self.lyrics_workspace = folder;
        self.standalone_audio = audio;
    }

    pub fn record(&mut self, value: Value) {
        self.last_result = Some(value);
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectAsset {
    pub id: String,
    pub name: String,
    pub preset: Option<String>,
    pub path: Option<PathBuf>,
    pub kind: String,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Asset {
    pub id: String,
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub has_alpha: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<craft_metadata::Metadata>,
}
