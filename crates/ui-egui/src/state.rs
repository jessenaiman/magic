//! Shared UI state, view routing (with `Gallery` and `Mayhem` removed), draft tracking, and shell
//! command queue for `magic-ui-egui`.

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::theme::{MagicPalette, ThemeMode};
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
    /// Focused 5-stage production tabs in `#app-bar` (`Gallery` & `Mayhem` intentionally removed).
    pub const STAGE_TABS: [Self; 5] = [Self::Storyboard, Self::Animation, Self::Design, Self::Motion, Self::Studio];

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
}

#[derive(Clone, Debug)]
pub enum UiAction {
    SwitchView(ViewMode),
    NavigateAuthoring { view: ViewMode, seconds: f64 },
    SelectCharacter { id: String, jump_animation: bool },
    Execute { id: String, params: Value, toast: Option<String> },
    PickFile { target: PickerTarget, title: String, extensions: Vec<String>, directory: bool, save: bool },
    ToggleAudio,
    SnapWindow,
    OpenPreferences,
    OpenDoctor,
    OpenAbout,
    CloseProject,
    RequestQuit,
    ShowToast(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveModal {
    Preferences,
    Doctor,
    About,
    UnsavedClose,
}

#[derive(Clone, Debug, Default)]
pub struct HomeFormState {
    pub search: String,
    pub filter: String,
    pub show_new_form: bool,
    pub new_name: String,
    pub new_root: String,
    pub new_audio: String,
    pub wizard_style: String,
    pub wizard_world: String,
    pub wizard_character: String,
    pub wizard_motion: String,
}

#[derive(Clone, Debug)]
pub struct ProjectViewState {
    pub search: String,
    pub type_filter: String,
    pub group_by_folder: bool,
    pub selected_asset_id: Option<String>,
}

impl Default for ProjectViewState {
    fn default() -> Self {
        Self { search: String::new(), type_filter: String::new(), group_by_folder: true, selected_asset_id: None }
    }
}

#[derive(Clone, Debug)]
pub struct StoryboardViewState {
    pub current_page: usize,
    pub zoom: f64,
    pub show_wizard: bool,
    pub style_d01: String,
    pub world_l01: String,
    pub character_c01: String,
    pub motion_m01: String,
    pub selected_slot: String,
    pub slot_value: String,
    pub archetype: String,
}

impl Default for StoryboardViewState {
    fn default() -> Self {
        Self {
            current_page: 0,
            zoom: 100.0,
            show_wizard: false,
            style_d01: String::new(),
            world_l01: String::new(),
            character_c01: String::new(),
            motion_m01: String::new(),
            selected_slot: "cover_title".into(),
            slot_value: String::new(),
            archetype: "scene".into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct LyricsViewState {
    pub sub_mode: String,
    pub search: String,
    pub selected_anim_id: Option<String>,
    pub selected_word_id: Option<String>,
    pub scrub_seconds: f64,
    pub font_size: f64,
    pub tracking: f64,
    pub rotate_deg: f64,
    pub skew_slant_deg: f64,
    pub skew_axis_deg: f64,
    pub scale_x: f64,
    pub scale_y: f64,
    pub stroke_width: f64,
    pub glow_radius: f64,
    pub per_char_3d: bool,
    pub selected_effect_id: String,
    pub color_hex: String,
    pub outline_hex: String,
    pub teaching_mode: String,
    pub choreography: String,
    pub move_preset: String,
    pub draw_enabled: bool,
    pub backdrop_mode: String,
}

impl Default for LyricsViewState {
    fn default() -> Self {
        Self {
            sub_mode: "design".into(),
            search: String::new(),
            selected_anim_id: None,
            selected_word_id: None,
            scrub_seconds: 0.0,
            font_size: 56.0,
            tracking: 0.0,
            rotate_deg: 0.0,
            skew_slant_deg: 0.0,
            skew_axis_deg: 0.0,
            scale_x: 100.0,
            scale_y: 100.0,
            stroke_width: 6.0,
            glow_radius: 14.0,
            per_char_3d: false,
            selected_effect_id: "ec.stylize.glow".into(),
            color_hex: "#f0f4fc".into(),
            outline_hex: "#0e141e".into(),
            teaching_mode: "sing_along".into(),
            choreography: "cascade".into(),
            move_preset: "pop".into(),
            draw_enabled: false,
            backdrop_mode: "scene".into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct MotionViewState {
    pub selected_word_id: Option<String>,
    pub selected_preset: String,
    pub selected_effect_id: String,
    pub cue_name: String,
    pub start_sec: f64,
    pub end_sec: f64,
    pub draw_duration: f64,
    pub hold_duration: f64,
    pub pos_x: f64,
    pub pos_y: f64,
    pub arc_bend_deg: f64,
    pub taper_pct: f64,
    pub width: f64,
    pub height: f64,
    pub stroke_width: f64,
    pub stroke_color: String,
    pub trace_source: String,
}

impl Default for MotionViewState {
    fn default() -> Self {
        Self {
            selected_word_id: None,
            selected_preset: "wave".into(),
            selected_effect_id: "ec.shape.trim_paths".into(),
            cue_name: "Flight Arc".into(),
            start_sec: 0.0,
            end_sec: 2.05,
            draw_duration: 0.85,
            hold_duration: 1.2,
            pos_x: 420.0,
            pos_y: 280.0,
            arc_bend_deg: 22.0,
            taper_pct: 65.0,
            width: 320.0,
            height: 180.0,
            stroke_width: 8.0,
            stroke_color: "#35e1c0".into(),
            trace_source: String::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct AnimationViewState {
    pub style_tab: String,
    pub dock_tab: String,
    pub selected_action: String,
    pub action_duration: f64,
    pub loop_clip: bool,
    pub scrub_seconds: f64,
    pub show_grid: bool,
    pub show_marks: bool,
    pub show_path_hud: bool,
    pub bone_rotation: f64,
    pub jaw_open_deg: f64,
    pub wing_bank_deg: f64,
    pub lip_intensity: f64,
    pub follow_delay: f64,
    pub bone_scale: f64,
    pub bone_opacity: f64,
    pub path_x: f64,
    pub path_y: f64,
    pub travel_duration: f64,
    pub start_size_pct: f64,
    pub end_size_pct: f64,
    pub pivot_x: f64,
    pub pivot_y: f64,
    pub place_start_sec: f64,
    pub place_duration_sec: f64,
    pub selected_part: String,
    pub expression: String,
    pub viseme: String,
}

impl Default for AnimationViewState {
    fn default() -> Self {
        Self {
            style_tab: "cutout-2-5d".into(),
            dock_tab: "theatre".into(),
            selected_action: "idle".into(),
            action_duration: 2.0,
            loop_clip: true,
            scrub_seconds: 0.0,
            show_grid: true,
            show_marks: true,
            show_path_hud: true,
            bone_rotation: -6.0,
            jaw_open_deg: 18.0,
            wing_bank_deg: 12.0,
            lip_intensity: 100.0,
            follow_delay: 0.07,
            bone_scale: 100.0,
            bone_opacity: 100.0,
            path_x: 260.0,
            path_y: 310.0,
            travel_duration: 2.5,
            start_size_pct: 100.0,
            end_size_pct: 82.0,
            pivot_x: 0.5,
            pivot_y: 0.5,
            place_start_sec: 0.0,
            place_duration_sec: 3.0,
            selected_part: "head".into(),
            expression: "happy".into(),
            viseme: "OH".into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct StudioViewState {
    pub selected_scene: usize,
    pub seek_seconds: f64,
    pub zoom_pct: f64,
    pub volume: f64,
    pub show_export_settings: bool,
    pub export_format: String,
    pub export_scale: f64,
    pub export_quality: f64,
    pub burn_captions: bool,
    pub allow_offline: bool,
    pub gpu_rendering: bool,
    pub queue_export: bool,
    pub range_start: f64,
    pub range_end: f64,
}

impl Default for StudioViewState {
    fn default() -> Self {
        Self {
            selected_scene: 0,
            seek_seconds: 0.0,
            zoom_pct: 100.0,
            volume: 100.0,
            show_export_settings: false,
            export_format: "h264".into(),
            export_scale: 1.0,
            export_quality: 85.0,
            burn_captions: false,
            allow_offline: false,
            gpu_rendering: true,
            queue_export: false,
            range_start: 0.0,
            range_end: 10.0,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct CachedPayloads {
    pub snapshot: Value,
    pub catalog: Value,
    pub storyboard_pages: Value,
    pub storyboard_check: Value,
    pub words_doc: Value,
    pub words_effects: Value,
    pub motion_doc: Value,
    pub motion_effects: Value,
    pub timeline: Value,
    pub film_status: Value,
    pub doctor: Value,
}

#[derive(Clone, Debug, Default)]
pub struct UiShellState {
    pub view: ViewMode,
    pub theme_mode: ThemeMode,
    pub focused_panel: String,
    pub project_path: Option<PathBuf>,
    pub selected_character_id: Option<String>,
    pub assets_drawer_open: bool,
    pub fidget_open: bool,
    pub fidget_agent: String,
    pub fidget_prompt: String,
    pub active_modal: Option<ActiveModal>,
    pub busy: bool,
    pub saving: bool,
    pub dirty_drafts: BTreeSet<String>,
    pub toast_message: String,
    pub toast_until: f64,
    pub audio_playing: bool,
    pub audio_time_sec: f64,
    pub audio_duration_sec: f64,
    pub window_snapped_right: bool,
    pub home: HomeFormState,
    pub project: ProjectViewState,
    pub storyboard: StoryboardViewState,
    pub lyrics: LyricsViewState,
    pub motion: MotionViewState,
    pub animation: AnimationViewState,
    pub studio: StudioViewState,
    pub cache: CachedPayloads,
}

impl UiShellState {
    pub fn effective_view(&self) -> ViewMode {
        if self.project_path.is_some() { self.view } else { ViewMode::Home }
    }

    pub fn project_name(&self) -> String {
        if let Some(name) = self.cache.snapshot.pointer("/data/name").and_then(Value::as_str).filter(|s| !s.is_empty()) {
            return name.to_owned();
        }
        self.project_path.as_ref().and_then(|p| p.file_stem()).map_or_else(|| "No project".into(), |s| s.to_string_lossy().into_owned())
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

    pub fn drafts_report(&self) -> Value {
        let drafts: Vec<&str> = self.dirty_drafts.iter().map(String::as_str).collect();
        json!({
            "dirty": !drafts.is_empty(),
            "saving": self.saving,
            "drafts": drafts,
        })
    }

    pub fn set_toast(&mut self, now: f64, msg: impl Into<String>) {
        self.toast_message = msg.into();
        self.toast_until = now + 5.5;
    }
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
        self.actions.push(UiAction::Execute { id: id.to_owned(), params, toast: toast.map(str::to_owned) });
    }
}
