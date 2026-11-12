//! Shared UI state, view routing (`Home`, `Project`, `Storyboard`, `Animations`, `Lyrics`,
//! `Motion`, `Studio`), `InterfaceStyle` & `AccentChoice` appearance state, `CachedPayloads`,
//! dirty-track invalidation, and shell action queue.

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub use crate::state_views::{
    AiUiState, AnimationViewState, HomeFormState, LyricsViewState, MotionViewState,
    ProjectViewState, StoryboardViewState, StudioViewState,
};
use crate::theme::{AccentChoice, InterfaceStyle, MagicPalette, ThemeMode};
use crate::widgets::AutomationRegistry;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ViewMode {
    #[default]
    Home,
    Project,
    Storyboard,
    #[serde(alias = "animations")]
    Animation,
    #[serde(alias = "lyrics")]
    Design,
    Motion,
    Studio,
}

impl ViewMode {
    pub const STAGE_TABS: [Self; 6] = [
        Self::Project,
        Self::Storyboard,
        Self::Animation,
        Self::Design,
        Self::Motion,
        Self::Studio,
    ];

    pub fn as_id(self) -> &'static str {
        match self {
            Self::Home => "home",
            Self::Project => "project",
            Self::Storyboard => "storyboard",
            Self::Animation => "animation",
            Self::Design => "design",
            Self::Motion => "motion",
            Self::Studio => "studio",
        }
    }

    pub fn root_automation_id(self) -> (&'static str, &'static str) {
        match self {
            Self::Home => ("view-home", "Home"),
            Self::Project => ("view-project", "Project"),
            Self::Storyboard => ("view-storyboard", "Storyboard"),
            Self::Animation => ("view-animation", "Animation"),
            Self::Design => ("lyrics-workspace-root", "Lyrics Workspace"),
            Self::Motion => ("view-motion", "Motion"),
            Self::Studio => ("view-studio", "Studio"),
        }
    }

    pub fn tab_label(self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Project => "Project",
            Self::Storyboard => "Storyboard",
            Self::Animation => "Animations",
            Self::Design => "Lyrics",
            Self::Motion => "Motion",
            Self::Studio => "Studio",
        }
    }

    pub fn rail_icon(self) -> &'static str {
        match self {
            Self::Home => "home",
            Self::Project => "folder",
            Self::Storyboard => "book",
            Self::Animation => "user",
            Self::Design => "type",
            Self::Motion => "sparkles",
            Self::Studio => "film",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "home" => Some(Self::Home),
            "project" | "projects" => Some(Self::Project),
            "storyboard" => Some(Self::Storyboard),
            "animation" | "animations" => Some(Self::Animation),
            "design" | "lyrics" => Some(Self::Design),
            "motion" => Some(Self::Motion),
            "studio" | "film" => Some(Self::Studio),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub enum PickerTarget {
    OpenProject,
    SaveProjectAs,
    NewProjectRoot,
    NewProjectAudio,
    SetSongAudio,
    ImportTranscript,
    AddProjectAsset,
    ImportTrunkFile,
    MotionTraceImage,
    ImportLottie,
    ImportAep,
    ImportLut,
    ImportFcgt,
    ImportFontDir,
    ImportWasmPlugin,
    ImportModel3d,
    StoryboardSlotImage { page: usize, slot: String },
}

#[derive(Clone, Debug)]
pub enum UiAction {
    SwitchView(ViewMode),
    NavigateAuthoring { view: ViewMode, seconds: f64 },
    SelectCharacter { id: String, jump_animation: bool },
    Execute { id: String, params: Value, toast: Option<String> },
    PickFile {
        target: PickerTarget,
        title: String,
        extensions: Vec<String>,
        directory: bool,
        save: bool,
    },
    ToggleAudio,
    SeekAudio(f64),
    SnapWindow,
    OpenPreferences,
    OpenDoctor,
    OpenReportProblem,
    CopyDiagnostics,
    OpenUrl(String),
    OpenAbout,
    CloseProject,
    RequestQuit,
    ShowToast(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveModal {
    Preferences,
    Doctor,
    ReportProblem,
    About,
    UnsavedClose,
}

#[derive(Clone, Debug, Default)]
pub struct CachedPayloads {
    pub snapshot: Value,
    pub catalog: Value,
    pub song_lyrics: Value,
    pub storyboard_pages: Value,
    pub storyboard_check: Value,
    pub characters: Value,
    pub character_keys: Value,
    pub character_path: Value,
    pub words_doc: Value,
    pub words_effects: Value,
    pub motion_doc: Value,
    pub motion_effects: Value,
    pub timeline: Value,
    pub film_status: Value,
    pub doctor: Value,
    pub ai_status: Value,
    pub ec_comps: Value,
    pub lottie_last: Value,
    pub aep_report: Value,
    pub last_words_frame: Option<String>,
    pub last_motion_frame: Option<String>,
    pub last_character_frame: Option<String>,
    pub last_video_frame: Option<String>,
    pub last_storyboard_thumb: Option<String>,
}

#[derive(Clone, Debug)]
pub struct UiShellState {
    pub view: ViewMode,
    pub theme_mode: ThemeMode,
    pub interface_style: InterfaceStyle,
    pub accent_choice: AccentChoice,
    pub high_contrast: bool,
    pub ui_scale_pct: u16,
    pub autosave_enabled: bool,
    pub autosave_minutes: f64,
    pub focused_panel: String,
    pub project_path: Option<PathBuf>,
    pub selected_character_id: Option<String>,
    pub ribbon_collapsed: bool,
    pub show_nav_rail: bool,
    pub show_bottom_dock: bool,
    pub show_right_inspector: bool,
    pub assets_drawer_open: bool,
    pub assets_drawer_pinned: bool,
    pub assets_drawer_tab: String,
    pub global_search: String,
    pub command_search: String,
    pub update_banner_dismissed: bool,
    pub fidget_open: bool,
    pub fidget_prompt: String,
    pub history_popup_open: bool,
    pub active_modal: Option<ActiveModal>,
    pub busy: bool,
    pub saving: bool,
    pub dirty_drafts: BTreeSet<String>,
    pub dirty_tracks: BTreeSet<String>,
    pub toast_message: String,
    pub toast_until: f64,
    pub audio_playing: bool,
    pub audio_time_sec: f64,
    pub audio_duration_sec: f64,
    pub window_snapped_right: bool,
    pub ai: AiUiState,
    pub home: HomeFormState,
    pub project: ProjectViewState,
    pub storyboard: StoryboardViewState,
    pub lyrics: LyricsViewState,
    pub motion: MotionViewState,
    pub animation: AnimationViewState,
    pub studio: StudioViewState,
    pub cache: CachedPayloads,
}

impl Default for UiShellState {
    fn default() -> Self {
        Self {
            view: ViewMode::Home,
            theme_mode: ThemeMode::Dark,
            interface_style: InterfaceStyle::Aurora,
            accent_choice: AccentChoice::Teal,
            high_contrast: false,
            ui_scale_pct: 100,
            autosave_enabled: true,
            autosave_minutes: 5.0,
            focused_panel: String::new(),
            project_path: None,
            selected_character_id: None,
            ribbon_collapsed: false,
            show_nav_rail: true,
            show_bottom_dock: true,
            show_right_inspector: true,
            assets_drawer_open: false,
            assets_drawer_pinned: false,
            assets_drawer_tab: "characters".into(),
            global_search: String::new(),
            command_search: String::new(),
            update_banner_dismissed: false,
            fidget_open: false,
            fidget_prompt: String::new(),
            history_popup_open: false,
            active_modal: None,
            busy: false,
            saving: false,
            dirty_drafts: BTreeSet::new(),
            dirty_tracks: BTreeSet::new(),
            toast_message: String::new(),
            toast_until: 0.0,
            audio_playing: false,
            audio_time_sec: 0.0,
            audio_duration_sec: 0.0,
            window_snapped_right: false,
            ai: AiUiState::default(),
            home: HomeFormState::default(),
            project: ProjectViewState::default(),
            storyboard: StoryboardViewState::default(),
            lyrics: LyricsViewState::default(),
            motion: MotionViewState::default(),
            animation: AnimationViewState::default(),
            studio: StudioViewState::default(),
            cache: CachedPayloads::default(),
        }
    }
}

impl UiShellState {
    pub fn effective_view(&self) -> ViewMode {
        if self.project_path.is_some() {
            self.view
        } else {
            ViewMode::Home
        }
    }

    pub fn project_name(&self) -> String {
        for ptr in ["/data/status/name", "/data/name", "/title"] {
            if let Some(name) = self
                .cache
                .snapshot
                .pointer(ptr)
                .or_else(|| self.cache.song_lyrics.pointer(ptr))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                return name.to_owned();
            }
        }
        self.project_path
            .as_ref()
            .and_then(|p| p.file_stem())
            .map_or_else(|| "No project".into(), |s| prettify_slug(&s.to_string_lossy()))
    }

    pub fn song_audio_path(&self) -> Option<String> {
        self.cache
            .snapshot
            .pointer("/data/status/stages")
            .and_then(Value::as_array)?
            .iter()
            .find(|s| s["id"] == "song")?
            .pointer("/inputs/0/path")
            .and_then(Value::as_str)
            .map(str::to_owned)
    }

    /// Ordered `FilmCraft` (`V1` / program) & Storyboard scene frames for stage track switching and Film Reels.
    pub fn active_film_frames(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(last) = &self.cache.last_video_frame {
            out.push(last.clone());
        }
        let co = &self.cache.film_status["craft_outputs"];
        for key in ["program_frames", "v1_frames"] {
            if let Some(arr) = co[key].as_array() {
                for item in arr {
                    if let Some(s) = item.as_str().filter(|p| !p.is_empty())
                        && !out.iter().any(|x| x == s)
                    {
                        out.push(s.to_owned());
                    }
                }
            }
        }
        if let Some(pages) = self.cache.storyboard_pages["pages"].as_array() {
            for page in pages {
                if page["archetype"].as_str() == Some("cover") {
                    continue;
                }
                let img = page["preview_png"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .or_else(|| {
                        page["slots"].as_array().and_then(|slots| {
                            slots.iter().find_map(|sl| sl["image"].as_str().filter(|s| !s.is_empty()))
                        })
                    });
                if let Some(s) = img
                    && !out.iter().any(|x| x == s)
                {
                    out.push(s.to_owned());
                }
            }
        }
        if out.is_empty()
            && let Some(bg) = self
                .cache
                .snapshot
                .pointer("/data/drawer/backgrounds/0/thumb")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
        {
            out.push(bg.to_owned());
        }
        out
    }

    /// Select the `FilmCraft` (`V1` / program) frame corresponding to `sec` seconds along the song timeline.
    pub fn film_frame_at_time(&self, sec: f64) -> Option<String> {
        let frames = self.active_film_frames();
        if frames.is_empty() {
            return None;
        }
        let total = self.audio_duration_sec.max(15.0);
        let frac = (sec.max(0.0) / total).clamp(0.0, 0.9999);
        let idx = ((frac * (frames.len() as f64)).floor() as usize).min(frames.len().saturating_sub(1));
        frames.get(idx).cloned()
    }

    pub fn drafts_report(&self) -> Value {
        let drafts: Vec<&str> = self.dirty_drafts.iter().map(String::as_str).collect();
        let dirty_tracks: Vec<&str> = self.dirty_tracks.iter().map(String::as_str).collect();
        json!({
            "dirty": !drafts.is_empty(),
            "saving": self.saving,
            "drafts": drafts,
            "dirty_tracks": dirty_tracks,
        })
    }

    pub fn set_toast(&mut self, now: f64, msg: impl Into<String>) {
        self.toast_message = msg.into();
        self.toast_until = now + 5.5;
    }
}

fn prettify_slug(raw: &str) -> String {
    if raw.contains(' ') {
        return raw.to_owned();
    }
    raw.split(['-', '_'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let mut out = first.to_uppercase().collect::<String>();
                    out.push_str(chars.as_str());
                    out
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub struct ViewCtx<'a> {
    pub state: &'a mut UiShellState,
    pub reg: &'a mut AutomationRegistry,
    pub pal: &'a MagicPalette,
    pub actions: &'a mut Vec<UiAction>,
}

impl<'a> ViewCtx<'a> {
    pub fn push(&mut self, action: UiAction) {
        self.actions.push(action);
    }

    pub fn exec(&mut self, id: &str, params: Value, toast: Option<&str>) {
        self.actions.push(UiAction::Execute {
            id: id.to_owned(),
            params,
            toast: toast.map(str::to_owned),
        });
    }
}
