//! Pure-Rust `eframe` 0.37 + `egui` 0.37 + `storytold/craft-libs` + `storytold/craft-ai-integrations`
//! desktop coordinator for Magic Studio (`magic --control 9875`, bridging `FilmCraft` `:9876` and
//! `EffectCraft` `:9877`).

#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable))]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod capture;
mod clipboard_image;
mod control;
mod diagnostics;
mod dialogs;
mod file_sync;
mod input;
mod keymap;
mod lifecycle;
mod rpc;
mod studio;

use std::collections::VecDeque;
use std::io::Write;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use clap::Parser;
use craft_dialogs::Answer;
#[cfg(target_os = "macos")]
use craft_keymap::CommandRef;
use craft_keymap::egui::Dispatcher;
use egui::{Event, ViewportCommand};
use magic::{AutomationRoots, Session};
use magic_ui_egui::{
    ActiveModal, AutomationRegistry, MagicPalette, PickerTarget, UiAction, UiShellState, ViewCtx,
    ViewMode,
};
use rpc::{ControlEnvelope, DeferredReply};
use serde_json::{Value, json};

const CONTROL_REPLY_TIMEOUT: Duration = Duration::from_secs(60);
const APP_ICON_PNG: &[u8] = include_bytes!("../../../assets/app-icon/icon.png");

#[derive(Parser, Debug)]
#[command(name = "magic", about = "Magic Studio pure-Rust ArtCraft desktop coordinator (`--control 9875`)")]
struct Cli {
    #[arg(long)]
    project: Option<String>,
    #[arg(
        long,
        default_value = "project",
        value_parser = ["home", "project", "storyboard", "animations", "animation", "design", "lyrics", "motion", "studio"]
    )]
    view: String,
    /// Open authenticated loopback `--control` JSON-lines socket (`9875` by default when flag is bare)
    #[arg(
        long,
        visible_alias = "controls",
        num_args = 0..=1,
        default_missing_value = "9875",
        env = "MAGIC_CONTROL_PORT"
    )]
    control: Option<String>,
    /// Restrict `--control` to `ui.*` inspection/input methods and disable physical keyboard/pointer input
    #[arg(long, env = "MAGIC_CONTROL_INPUT_ONLY", default_value_t = false)]
    control_input_only: bool,
    /// Override `FilmCraft` `--control` port (`127.0.0.1:9876` by default)
    #[arg(long)]
    filmcraft_control: Option<u16>,
    /// Override `EffectCraft` `--control` port (`127.0.0.1:9877` by default)
    #[arg(long)]
    effectcraft_control: Option<u16>,
    /// Print full JSON catalog of coordinator commands & `--control` methods, then exit
    #[arg(long)]
    list_controls: bool,
    #[arg(long, env = "MAGIC_IN_WINDOW_MENUS", default_value_t = false)]
    in_window_menus: bool,
    #[arg(long = "automation-read-root")]
    automation_read_roots: Vec<String>,
    #[arg(long = "automation-write-root")]
    automation_write_roots: Vec<String>,
}

pub(crate) fn only_keys(params: &Value, method: &str, allowed: &[&str]) -> Result<(), String> {
    let object = params.as_object().ok_or("params must be an object")?;
    match object.keys().find(|key| !allowed.contains(&key.as_str())) {
        Some(key) => Err(format!("{method}: unknown parameter `{key}`; accepted: {}", allowed.join(", "))),
        None => Ok(()),
    }
}

#[cfg(target_os = "macos")]
struct MacMenuHooks {
    raw_input: Box<dyn FnMut(&mut egui::RawInput)>,
    sync_frame: Box<dyn FnMut(&egui::Context, &UiShellState, bool, &dyn Fn(&str) -> bool) -> Vec<CommandRef>>,
}

#[cfg(target_os = "macos")]
fn install_macos_menu(ctx: &egui::Context, initial: &UiShellState) -> Option<MacMenuHooks> {
    use std::cell::RefCell;
    use std::rc::Rc;

    use craft_menu::{Gate, LayoutOptions, mac, mac_layout};
    use magic_ui_egui::menus::build_menu_bar;

    let bar = build_menu_bar(initial, keymap::catalog_keymap(), |_| true);
    let installed = mac::install(ctx, &mac_layout(&bar, &LayoutOptions::new("Magic Studio")).bar)?;
    let shared = Rc::new(RefCell::new(installed));
    let raw_ref = Rc::clone(&shared);
    let frame_ref = Rc::clone(&shared);
    Some(MacMenuHooks {
        raw_input: Box::new(move |raw| {
            raw_ref.borrow_mut().raw_input(raw);
        }),
        sync_frame: Box<dyn FnMut(&egui::Context, &UiShellState, bool, &dyn Fn(&str) -> bool) -> Vec<CommandRef>>::from(Box::new(
            move |egui_ctx, state, modal, is_enabled| {
                let mut menu = frame_ref.borrow_mut();
                menu.set_gate(Gate {
                    modal,
                    recording_shortcut: false,
                    text_editing: egui_ctx.egui_wants_keyboard_input(),
                });
                let clicks = menu.take_clicks(egui_ctx);
                let hash = egui_ctx.cumulative_frame_nr();
                let _ = menu.sync_with(hash, || {
                    let updated = build_menu_bar(state, keymap::catalog_keymap(), is_enabled);
                    mac_layout(&updated, &LayoutOptions::new("Magic Studio"))
                });
                clicks
            },
        )),
    })
}

pub(crate) struct MagicDesktopApp {
    pub(crate) session: Session,
    pub(crate) ui_state: UiShellState,
    pub(crate) registry: AutomationRegistry,
    pub(crate) dialogs: dialogs::DialogHost,
    pub(crate) file_sync: file_sync::FileSyncMonitor,
    pub(crate) key_dispatcher: Dispatcher,
    pub(crate) lifecycle: lifecycle::Lifecycle,
    pub(crate) control_rx: Option<mpsc::Receiver<ControlEnvelope>>,
    pub(crate) control_input_only: bool,
    pub(crate) synthetic_events: VecDeque<Event>,
    pub(crate) deferred_replies: Vec<DeferredReply>,
    pub(crate) pending_captures: Vec<capture::PendingCapture>,
    pub(crate) last_render_ms: f32,
    pub(crate) last_applied_scale_pct: u16,
    pub(crate) frame_started: Instant,
    pub(crate) last_tick: Instant,
    pub(crate) in_window_menus: bool,
    #[cfg(target_os = "macos")]
    native_menu: Option<MacMenuHooks>,
    #[cfg(target_os = "macos")]
    mac_movable_applied: Option<bool>,
}

impl MagicDesktopApp {
    fn new(
        cc: &eframe::CreationContext<'_>,
        mut session: Session,
        initial_view: ViewMode,
        in_window_menus: bool,
        control_input_only: bool,
        control_rx: Option<mpsc::Receiver<ControlEnvelope>>,
    ) -> Self {
        magic_ui_egui::install_loaders(&cc.egui_ctx);
        let mut ui_state = UiShellState {
            view: initial_view,
            ..UiShellState::default()
        };
        diagnostics::hydrate_prefs_and_ai(&mut session, &mut ui_state);
        MagicPalette::resolve(
            ui_state.theme_mode,
            ui_state.interface_style,
            ui_state.accent_choice,
            ui_state.high_contrast,
            None,
        )
        .apply(&cc.egui_ctx);

        #[cfg(target_os = "macos")]
        let native_menu = if in_window_menus {
            None
        } else {
            install_macos_menu(&cc.egui_ctx, &ui_state)
        };
        #[cfg(target_os = "macos")]
        let use_in_window = in_window_menus || native_menu.is_none();
        #[cfg(not(target_os = "macos"))]
        let use_in_window = {
            let _ = in_window_menus;
            true
        };

        diagnostics::refresh_cached_state(&mut session, &mut ui_state, true);
        let mut file_sync = file_sync::FileSyncMonitor::default();
        file_sync.seed_project(&mut session, &mut ui_state);
        let now = Instant::now();
        Self {
            session,
            ui_state,
            registry: AutomationRegistry::default(),
            dialogs: dialogs::DialogHost::new(&cc.egui_ctx),
            file_sync,
            key_dispatcher: Dispatcher::new(),
            lifecycle: lifecycle::Lifecycle::default(),
            control_rx,
            control_input_only,
            synthetic_events: VecDeque::new(),
            deferred_replies: Vec::new(),
            pending_captures: Vec::new(),
            last_render_ms: 0.0,
            last_applied_scale_pct: 100,
            frame_started: now,
            last_tick: now,
            in_window_menus: use_in_window,
            #[cfg(target_os = "macos")]
            native_menu,
            #[cfg(target_os = "macos")]
            mac_movable_applied: None,
        }
    }

    pub(crate) fn execute_engine(&mut self, ctx: &egui::Context, id: &str, params: Value) -> Result<Value, String> {
        if let Some(expected) = params.get("_expected_project").and_then(Value::as_str)
            && self.session.project_path.as_ref().and_then(|p| p.to_str()) != Some(expected)
        {
            return Err("The active project changed; the previous workspace command was not run".into());
        }
        let changed = !magic::commands::is_read_only_command(id, &params);
        let is_project_switch = matches!(id, "project.open" | "project.new" | "project.close" | "file.open" | "file.new" | "file.close");
        if is_project_switch && self.session.project_path.is_some() {
            lifecycle::verify_clean(&self.session, &self.ui_state)?;
        }
        let result = if id == "clipboard.import_image" {
            clipboard_image::import_image(&mut self.session, &params)?
        } else {
            self.session.execute(id, params).map_err(|e| e.to_string())?
        };
        self.capture_query_cache(id, &result);
        if changed {
            diagnostics::refresh_cached_state(&mut self.session, &mut self.ui_state, false);
            if is_project_switch {
                self.file_sync.seed_project(&mut self.session, &mut self.ui_state);
            } else {
                self.file_sync.note_mutation(id, &mut self.ui_state);
            }
            ctx.request_repaint();
        }
        Ok(result)
    }

    fn capture_query_cache(&mut self, id: &str, result: &Value) {
        match id {
            "project.catalog" => self.ui_state.cache.catalog = result.clone(),
            "song.lyrics" => self.ui_state.cache.song_lyrics = result.clone(),
            "storyboard.pages" => self.ui_state.cache.storyboard_pages = result.clone(),
            "storyboard.check" => self.ui_state.cache.storyboard_check = result.clone(),
            "storyboard.thumbnail" => {
                if let Some(out) = result["out"].as_str() {
                    self.ui_state.cache.last_storyboard_thumb = Some(out.to_owned());
                }
                if let Ok(pages) = self.session.execute("storyboard.pages", json!({"full": true})) {
                    self.ui_state.cache.storyboard_pages = pages;
                }
            }
            "character.list" => self.ui_state.cache.characters = result.clone(),
            "character.keys" => self.ui_state.cache.character_keys = result.clone(),
            "character.path" => self.ui_state.cache.character_path = result.clone(),
            "character.frame" => {
                if let Some(f) = result["frame"].as_str() {
                    self.ui_state.cache.last_character_frame = Some(f.to_owned());
                }
            }
            "words.read" => self.ui_state.cache.words_doc = result.clone(),
            "words.effects" => self.ui_state.cache.words_effects = result.clone(),
            "words.preview_frame" => {
                if let Some(f) = result["frame"].as_str() {
                    self.ui_state.cache.last_words_frame = Some(f.to_owned());
                }
            }
            "motion.read" => self.ui_state.cache.motion_doc = result.clone(),
            "motion.effects" => self.ui_state.cache.motion_effects = result.clone(),
            "motion.preview_frame" => {
                if let Some(f) = result["frame"].as_str() {
                    self.ui_state.cache.last_motion_frame = Some(f.to_owned());
                }
            }
            "video.timeline" => self.ui_state.cache.timeline = result.clone(),
            "video.film" => self.ui_state.cache.film_status = result.clone(),
            "video.frame" => {
                if let Some(f) = result["frame"].as_str() {
                    self.ui_state.cache.last_video_frame = Some(f.to_owned());
                }
            }
            "video.list_ec_comps" => self.ui_state.cache.ec_comps = result.clone(),
            "bridge.status" => self.ui_state.cache.bridge_status = result.clone(),
            "bridge.preflight" => self.ui_state.cache.bridge_preflight = result.clone(),
            "bridge.history" => self.ui_state.cache.bridge_history = result.clone(),
            "effect.lottie" => self.ui_state.cache.lottie_last = result.clone(),
            "effect.import_aep" => self.ui_state.cache.aep_report = result.clone(),
            "app.doctor" => self.ui_state.cache.doctor = result.clone(),
            "ai.status" | "ai.configure" | "scribbles.connect" => {
                diagnostics::sync_ai_ui_from_status(&mut self.ui_state, result);
                self.ui_state.cache.ai_status = result.clone();
            }
            "fidget.prepare" | "fidget.stage" => {
                if let Some(p) = result.get("prompt").or_else(|| result.pointer("/fidget/prompt")).and_then(Value::as_str) {
                    self.ui_state.ai.staged_prompt = p.to_owned();
                }
                if let Some(md) = result.get("markdown").or_else(|| result.pointer("/fidget/markdown")).and_then(Value::as_str) {
                    self.ui_state.ai.staged_markdown = md.to_owned();
                }
            }
            _ => {}
        }
    }

    pub(crate) fn handle_ui_action(&mut self, ctx: &egui::Context, action: UiAction) {
        let now = ctx.input(|i| i.time);
        match action {
            UiAction::SwitchView(next) => {
                self.ui_state.view = next;
                self.refresh_view_data(next);
            }
            UiAction::NavigateAuthoring { view, seconds } => {
                self.ui_state.view = view;
                match view {
                    ViewMode::Design => self.ui_state.lyrics.scrub_seconds = seconds,
                    ViewMode::Motion => self.ui_state.motion.start_sec = seconds,
                    ViewMode::Animation => self.ui_state.animation.scrub_seconds = seconds,
                    _ => {}
                }
                self.refresh_view_data(view);
            }
            UiAction::SelectCharacter { id, jump_animation } => {
                self.ui_state.selected_character_id = Some(id);
                diagnostics::hydrate_selected_character(&mut self.session, &mut self.ui_state);
                if jump_animation {
                    self.ui_state.view = ViewMode::Animation;
                }
            }
            UiAction::Execute { id, params, toast } => match self.execute_engine(ctx, &id, params) {
                Ok(_) => {
                    if let Some(msg) = toast {
                        self.ui_state.set_toast(now, msg);
                    }
                }
                Err(err) => self.ui_state.set_toast(now, format!("{id}: {err}")),
            },
            UiAction::PickFile { target, title, extensions, directory, save } => {
                self.dialogs.open_picker(target, &title, &extensions, directory, save);
            }
            UiAction::ToggleAudio => {
                if self.ui_state.song_audio_path().is_some() {
                    self.ui_state.audio_playing = !self.ui_state.audio_playing;
                }
            }
            UiAction::SnapWindow => {
                self.ui_state.window_snapped_right = !self.ui_state.window_snapped_right;
                let x = if self.ui_state.window_snapped_right { 640.0 } else { 0.0 };
                ctx.send_viewport_cmd(ViewportCommand::OuterPosition(egui::pos2(x, 0.0)));
                self.ui_state.set_toast(
                    now,
                    if self.ui_state.window_snapped_right { "Snapped window right" } else { "Snapped window left" },
                );
            }
            UiAction::OpenPreferences => {
                let _ = self.execute_engine(ctx, "ai.status", json!({}));
                let _ = self.execute_engine(ctx, "bridge.status", json!({}));
                self.ui_state.active_modal = Some(ActiveModal::Preferences);
            }
            UiAction::OpenDoctor => {
                let _ = self.execute_engine(ctx, "app.doctor", json!({}));
                self.ui_state.active_modal = Some(ActiveModal::Doctor);
            }
            UiAction::OpenReportProblem => self.ui_state.active_modal = Some(ActiveModal::ReportProblem),
            UiAction::CopyDiagnostics => {
                let report = diagnostics::build_sanitized_diagnostics(&mut self.session, &mut self.ui_state, self.last_render_ms);
                ctx.copy_text(report.clone());
                if let Ok(mut cb) = arboard::Clipboard::new() {
                    let _ = cb.set_text(report);
                }
                self.ui_state.set_toast(now, "Copied sanitized diagnostics to clipboard");
            }
            UiAction::OpenUrl(url) => diagnostics::open_https_url(ctx, &url),
            UiAction::OpenAbout => self.ui_state.active_modal = Some(ActiveModal::About),
            UiAction::CloseProject => {
                self.ui_state.audio_playing = false;
                match self.execute_engine(ctx, "file.close", json!({})) {
                    Ok(_) => self.ui_state.view = ViewMode::Home,
                    Err(err) => self.ui_state.set_toast(now, err),
                }
            }
            UiAction::RequestQuit => ctx.send_viewport_cmd(ViewportCommand::Close),
            UiAction::ShowToast(msg) => self.ui_state.set_toast(now, msg),
        }
    }

    fn refresh_view_data(&mut self, view: ViewMode) {
        match view {
            ViewMode::Home => {
                if let Ok(cat) = self.session.execute("project.catalog", json!({})) {
                    self.ui_state.cache.catalog = cat;
                }
            }
            ViewMode::Project => {
                if let Ok(lyr) = self.session.execute("song.lyrics", json!({})) {
                    self.ui_state.cache.song_lyrics = lyr;
                }
                if let Ok(snap) = self.session.execute("project.snapshot", json!({})) {
                    self.ui_state.cache.snapshot = snap;
                }
                if let Ok(pre) = self.session.execute("bridge.preflight", json!({"action": "check"})) {
                    self.ui_state.cache.bridge_preflight = pre;
                }
            }
            ViewMode::Storyboard => {
                if let Ok(pages) = self.session.execute("storyboard.pages", json!({"full": true})) {
                    self.ui_state.cache.storyboard_pages = pages;
                }
                if let Ok(chk) = self.session.execute("storyboard.check", json!({})) {
                    self.ui_state.cache.storyboard_check = chk;
                }
            }
            ViewMode::Animation => {
                if let Ok(chars) = self.session.execute("character.list", json!({})) {
                    self.ui_state.cache.characters = chars;
                }
                diagnostics::hydrate_selected_character(&mut self.session, &mut self.ui_state);
            }
            ViewMode::Design => {
                if let Ok(words) = self.session.execute("words.read", json!({})) {
                    self.ui_state.cache.words_doc = words;
                }
                if let Ok(fx) = self.session.execute("words.effects", json!({})) {
                    self.ui_state.cache.words_effects = fx;
                }
            }
            ViewMode::Motion => {
                if let Ok(mot) = self.session.execute("motion.read", json!({})) {
                    self.ui_state.cache.motion_doc = mot;
                }
                if let Ok(fx) = self.session.execute("motion.effects", json!({})) {
                    self.ui_state.cache.motion_effects = fx;
                }
            }
            ViewMode::Studio => {
                self.file_sync.check_disk_ecprojs(&mut self.session, &mut self.ui_state);
                if let Ok(film) = self.session.execute("video.film", json!({})) {
                    self.ui_state.cache.film_status = film;
                }
                if let Ok(tl) = self.session.execute("video.timeline", json!({})) {
                    self.ui_state.cache.timeline = tl;
                }
            }
        }
    }

    fn handle_picker_answer(&mut self, ctx: &egui::Context, target: PickerTarget, answer: Answer) {
        let now = ctx.input(|i| i.time);
        match answer {
            Answer::Cancelled | Answer::Message(_) => {}
            Answer::Unavailable(why) => {
                self.ui_state.set_toast(now, format!("File dialog unavailable: {why}"));
            }
            Answer::Paths(paths) => {
                let Some(first) = paths.first() else { return };
                let path_str = first.to_string_lossy().into_owned();
                let ext = first.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).unwrap_or_default();
                match target {
                    PickerTarget::OpenProject => {
                        let _ = self.execute_engine(ctx, "file.open", json!({"path": path_str}));
                        self.ui_state.view = ViewMode::Project;
                    }
                    PickerTarget::SaveProjectAs => {
                        let _ = self.execute_engine(ctx, "file.saveAs", json!({"path": path_str}));
                    }
                    PickerTarget::NewProjectRoot => self.ui_state.home.new_root = path_str,
                    PickerTarget::NewProjectAudio => self.ui_state.home.new_audio = path_str,
                    PickerTarget::SetSongAudio => {
                        let _ = self.execute_engine(ctx, "workshop.set_audio", json!({"path": path_str}));
                    }
                    PickerTarget::ImportTranscript => {
                        let _ = self.execute_engine(ctx, "workshop.import_transcript", json!({"path": path_str, "replace": true}));
                    }
                    PickerTarget::AddProjectAsset => {
                        if matches!(ext.as_str(), "ecproj" | "pcraft" | "psd" | "vectorcraft" | "anicraft") {
                            let _ = self.execute_engine(ctx, "character.import", json!({"path": path_str}));
                        } else {
                            let _ = self.execute_engine(ctx, "project.add_asset", json!({"path": path_str, "kind": "image"}));
                        }
                    }
                    PickerTarget::ImportTrunkFile => {
                        if matches!(ext.as_str(), "ecproj" | "pcraft" | "psd" | "vectorcraft" | "anicraft") {
                            let _ = self.execute_engine(ctx, "character.import", json!({"path": path_str}));
                        } else {
                            let _ = self.execute_engine(ctx, "project.import_file", json!({"path": path_str}));
                        }
                    }
                    PickerTarget::MotionTraceImage => {
                        let _ = self.execute_engine(ctx, "vector.trace", json!({"path": path_str, "ignore_white": true}));
                    }
                    PickerTarget::ImportLottie => {
                        self.ui_state.motion.lottie_show_warnings = true;
                        let _ = self.execute_engine(ctx, "effect.lottie", json!({"mode": "import", "path": path_str, "ecproj": "motion/actions.ecproj"}));
                    }
                    PickerTarget::ImportAep => {
                        let _ = self.execute_engine(ctx, "effect.import_aep", json!({"path": path_str, "dry_run": true}));
                    }
                    PickerTarget::ImportLut => {
                        let _ = self.execute_engine(ctx, "video.finish", json!({"op": "lut_import", "path": path_str}));
                        self.ui_state.set_toast(now, "Loaded 3D LUT (.cube / .3dl)");
                    }
                    PickerTarget::ImportFcgt => {
                        let _ = self.execute_engine(ctx, "bridge.templates", json!({"action": "install", "path": path_str}));
                        self.ui_state.set_toast(now, "Installed native graphic template");
                    }
                    PickerTarget::ImportFontDir => {
                        self.ui_state.lyrics.font_dir_extra = path_str.clone();
                        diagnostics::apply_app_font_dirs(&path_str);
                        let _ = self.execute_engine(ctx, "prefs.set", json!({"key": "lyrics.font_dirs", "value": path_str}));
                        self.ui_state.set_toast(now, "Saved EFFECTCRAFT_FONT_DIRS folder");
                    }
                    PickerTarget::ImportWasmPlugin => {
                        let _ = self.execute_engine(
                            ctx,
                            "effect.exec",
                            json!({"path": "motion/actions.ecproj", "id": "effect.plugins.load", "params": {"path": path_str}, "save": true}),
                        );
                        self.ui_state.set_toast(now, "Loaded WebAssembly (.wasm) plug-in into EffectCraft");
                    }
                    PickerTarget::ImportModel3d => {
                        let cid = self.ui_state.selected_character_id.clone().unwrap_or_else(|| "lead".into());
                        let _ = self.execute_engine(ctx, "character.layer.new3dModel", json!({"character": cid, "path": path_str}));
                        self.ui_state.set_toast(now, "Imported 3D model into EffectCraft comp");
                    }
                }
            }
        }
    }

    fn handle_dropped_files(&mut self, ctx: &egui::Context, dropped: Vec<PathBuf>) {
        let now = ctx.input(|i| i.time);
        if self.ui_state.project_path.is_none() {
            self.ui_state.set_toast(now, "Open a project before dropping Craft files");
            return;
        }
        let stage_ecproj = file_sync::stage_ecproj_for_drop(&self.ui_state);
        let mut imported = 0usize;
        for path in dropped {
            let ext = path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).unwrap_or_default();
            let res = if matches!(ext.as_str(), "ecproj" | "pcraft" | "psd" | "vectorcraft" | "anicraft") {
                self.execute_engine(ctx, "character.import", json!({"path": path}))
            } else {
                self.execute_engine(ctx, "project.add_asset", json!({"path": path, "kind": "image"}))
            };
            if let Ok(val) = res {
                imported += 1;
                if let Some(ref ec_doc) = stage_ecproj
                    && !matches!(ext.as_str(), "ecproj" | "pcraft" | "psd" | "vectorcraft" | "anicraft")
                {
                    let asset_path = val["path"].as_str().map(PathBuf::from).unwrap_or(path);
                    let _ = self.execute_engine(
                        ctx,
                        "effect.exec",
                        json!({"path": ec_doc, "id": "file.import", "params": {"paths": [asset_path]}, "save": true}),
                    );
                }
            }
        }
        if imported > 0 {
            self.ui_state.set_toast(now, format!("Imported {imported} Craft file(s)"));
        }
    }
}

impl eframe::App for MagicDesktopApp {
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        if self.control_input_only {
            raw_input.events.clear();
            raw_input.dropped_files.clear();
        }
        #[cfg(target_os = "macos")]
        if !self.control_input_only
            && let Some(menu) = &mut self.native_menu
        {
            (menu.raw_input)(raw_input);
        }
        while let Some(ev) = self.synthetic_events.pop_front() {
            raw_input.events.push(ev);
        }
    }

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frame_started = Instant::now();
        let dt = self.frame_started.duration_since(self.last_tick).as_secs_f64();
        self.last_tick = self.frame_started;

        if self.ui_state.ui_scale_pct != self.last_applied_scale_pct {
            self.last_applied_scale_pct = self.ui_state.ui_scale_pct;
            ctx.set_zoom_factor((f32::from(self.ui_state.ui_scale_pct) / 100.0).clamp(0.8, 2.0));
        }

        let mut remaining_deferred = Vec::new();
        for mut item in self.deferred_replies.drain(..) {
            if item.frames_left == 0 {
                let _ = item.reply.send(Ok(item.result));
            } else {
                item.frames_left -= 1;
                remaining_deferred.push(item);
                ctx.request_repaint();
            }
        }
        self.deferred_replies = remaining_deferred;

        self.drain_control(ctx);

        let ppp = ctx.pixels_per_point();
        let reg_ref = &self.registry;
        self.pending_captures.retain_mut(|cap| !cap.poll(ppp, reg_ref));

        if self.ui_state.audio_playing {
            self.ui_state.audio_time_sec += dt;
            if self.ui_state.audio_duration_sec > 0.0 && self.ui_state.audio_time_sec >= self.ui_state.audio_duration_sec {
                self.ui_state.audio_time_sec = 0.0;
                self.ui_state.audio_playing = false;
            } else {
                ctx.request_repaint_after(Duration::from_millis(50));
            }
        }

        if !self.control_input_only {
            let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).collect());
            if !dropped.is_empty() {
                self.handle_dropped_files(ctx, dropped);
            }
        }

        self.file_sync.poll_frame(ctx, &mut self.session, &mut self.ui_state);
        if ctx.input(|i| i.focused) {
            craft_font_catalog::Catalog::global().rescan_if_changed();
        }

        if ctx.input(|i| i.viewport().close_requested()) && !self.lifecycle.is_approved() {
            if lifecycle::verify_clean(&self.session, &self.ui_state).is_err() {
                ctx.send_viewport_cmd(ViewportCommand::CancelClose);
                self.ui_state.active_modal = Some(ActiveModal::UnsavedClose);
            } else {
                self.lifecycle.approve_exit(false);
            }
        }
        craft_quit::on_close_request(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let egui_ctx = ui.ctx().clone();
        let system_dark = egui_ctx.input(|i| i.raw.system_theme.map(|t| matches!(t, egui::Theme::Dark)));
        let palette = MagicPalette::resolve(
            self.ui_state.theme_mode,
            self.ui_state.interface_style,
            self.ui_state.accent_choice,
            self.ui_state.high_contrast,
            system_dark,
        );
        palette.apply(&egui_ctx);

        let modal_open = self.ui_state.active_modal.is_some() || self.dialogs.is_open();

        #[cfg(target_os = "macos")]
        if !self.control_input_only
            && let Some(menu) = &mut self.native_menu
        {
            let session_ref = &self.session;
            let clicks = (menu.sync_frame)(&egui_ctx, &self.ui_state, modal_open, &|cid| {
                magic::find_command(cid).is_some_and(|spec| (spec.enabled)(session_ref).is_ok())
            });
            for cmd in clicks {
                if let Some(act) = magic_ui_egui::menus::translate_menu_command(&cmd, &mut self.ui_state) {
                    self.handle_ui_action(&egui_ctx, act);
                }
            }
        }

        for cmd in keymap::dispatch_frame_keys(&egui_ctx, &mut self.key_dispatcher, modal_open) {
            if let Some(act) = magic_ui_egui::menus::translate_menu_command(&cmd, &mut self.ui_state) {
                self.handle_ui_action(&egui_ctx, act);
            }
        }

        for (target, answer) in self.dialogs.drain_answers() {
            self.handle_picker_answer(&egui_ctx, target, answer);
        }

        self.registry.begin_frame();
        let mut emitted = Vec::new();
        {
            let session_ref = &self.session;
            let mut view_ctx = ViewCtx {
                state: &mut self.ui_state,
                reg: &mut self.registry,
                pal: &palette,
                actions: &mut emitted,
            };
            magic_ui_egui::shell::draw_workstation(
                ui,
                &mut view_ctx,
                keymap::catalog_keymap(),
                self.in_window_menus,
                |cid| magic::find_command(cid).is_some_and(|spec| (spec.enabled)(session_ref).is_ok()),
            );
        }
        for action in emitted {
            self.handle_ui_action(&egui_ctx, action);
        }

        self.dialogs.show_pending(frame);

        #[cfg(target_os = "macos")]
        craft_window::title_bar::sync_movable(&egui_ctx, &mut self.mac_movable_applied);

        self.last_render_ms = self.frame_started.elapsed().as_secs_f32() * 1000.0;
    }

    fn on_exit(&mut self) {
        if self.lifecycle.restart_requested()
            && let Ok(exe) = std::env::current_exe()
        {
            let _ = std::process::Command::new(exe).args(std::env::args_os().skip(1)).spawn();
        }
    }
}

fn load_app_icon() -> Option<Arc<egui::IconData>> {
    let decoded = image::load_from_memory(APP_ICON_PNG).ok()?.into_rgba8();
    let (width, height) = decoded.dimensions();
    Some(Arc::new(egui::IconData {
        rgba: decoded.into_raw(),
        width,
        height,
    }))
}

fn diag(message: std::fmt::Arguments) {
    let _ = writeln!(std::io::stderr(), "{message}");
}

fn start_control_bridge(
    spec: &str,
    token: &str,
    wake_slot: Arc<std::sync::Mutex<Option<egui::Context>>>,
) -> Result<(SocketAddr, mpsc::Receiver<ControlEnvelope>), String> {
    let (tx, rx) = mpsc::channel::<ControlEnvelope>();
    let addr = control::start(
        spec,
        token,
        Arc::new(move |method, params| {
            let (reply_tx, reply_rx) = mpsc::channel();
            tx.send(ControlEnvelope {
                method: method.to_owned(),
                params,
                reply: reply_tx,
            })
            .map_err(|_| "Magic desktop event loop has exited".to_string())?;
            if let Ok(guard) = wake_slot.lock()
                && let Some(ctx) = guard.as_ref()
            {
                ctx.request_repaint();
            }
            reply_rx
                .recv_timeout(CONTROL_REPLY_TIMEOUT)
                .map_err(|_| "UI did not respond within 60 seconds".to_string())?
        }),
    )?;
    Ok((addr, rx))
}

fn main() {
    let cli = Cli::parse();
    if cli.list_controls {
        let cmds: Vec<Value> = magic::command_specs().iter().map(magic::commands::CommandSpec::describe).collect();
        let payload = json!({
            "ok": true,
            "ports": {"magic": 9875, "filmcraft": 9876, "effectcraft": 9877, "designcraft": 7979},
            "commands": cmds,
        });
        let _ = writeln!(std::io::stdout(), "{}", serde_json::to_string_pretty(&payload).unwrap_or_default());
        return;
    }
    if cli.control_input_only && cli.control.is_none() {
        diag(format_args!("magic: --control-input-only requires --control [PORT]"));
        std::process::exit(2);
    }
    if let Some(spec) = &cli.control
        && let Err(err) = control::parse_control_addr(spec)
    {
        diag(format_args!("magic: {err}"));
        std::process::exit(2);
    }
    let automation_roots = match AutomationRoots::from_args(&cli.automation_read_roots, &cli.automation_write_roots) {
        Ok(roots) => roots,
        Err(err) => {
            diag(format_args!("magic: {err}"));
            std::process::exit(2);
        }
    };
    let token = if cli.control.is_some() {
        magic::workspace::ensure_control_token().unwrap_or_default()
    } else {
        String::new()
    };
    if cli.control.is_some() && (token.len() != 64 || !token.bytes().all(|b| b.is_ascii_hexdigit())) {
        diag(format_args!("MAGIC_CONTROL_TOKEN must contain 64 hexadecimal characters"));
        std::process::exit(2);
    }

    craft_font_catalog::Catalog::global().scan_in_background();

    let mut session = Session::user_session().with_automation_roots(automation_roots);
    if let Some(fp) = cli.filmcraft_control {
        let _ = session.execute("prefs.set", json!({"key": "craft.filmcraft_port", "value": fp}));
    }
    if let Some(ep) = cli.effectcraft_control {
        let _ = session.execute("prefs.set", json!({"key": "craft.effectcraft_port", "value": ep}));
    }
    if let Some(path) = &cli.project
        && let Err(err) = session.execute("file.open", json!({"path": path}))
    {
        diag(format_args!("magic: could not open {path}: {err}"));
        std::process::exit(1);
    }

    let wake_ctx: Arc<std::sync::Mutex<Option<egui::Context>>> = Arc::new(std::sync::Mutex::new(None));
    let control_rx = match &cli.control {
        Some(spec) => match start_control_bridge(spec, &token, Arc::clone(&wake_ctx)) {
            Ok((address, rx)) => {
                session.control_address = Some(address);
                diag(format_args!("magic: control listening on {address}"));
                Some(rx)
            }
            Err(err) => {
                diag(format_args!("magic: {err}"));
                std::process::exit(1);
            }
        },
        None => None,
    };

    let initial_view = ViewMode::parse(&cli.view).unwrap_or(ViewMode::Project);
    let win_choice = craft_window::Choice::probe(craft_window::Prefix::new("MAGIC"));
    let mut base_vp = egui::ViewportBuilder::default()
        .with_title("Magic Studio")
        .with_app_id("io.github.jessenaiman.magic")
        .with_inner_size([1280.0, 800.0])
        .with_min_inner_size([960.0, 600.0])
        .with_active(cli.control.is_none());
    if let Some(icon) = load_app_icon() {
        base_vp = base_vp.with_icon(icon);
    }
    let viewport = craft_window::decorate(base_vp, win_choice.frame());

    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Wgpu,
        wgpu_options: eframe::egui_wgpu::WgpuConfiguration::default(),
        ..Default::default()
    };

    let in_window_menus = cli.in_window_menus;
    let control_input_only = cli.control_input_only;
    let run_res = eframe::run_native(
        "Magic Studio",
        options,
        Box::new(move |cc| {
            if let Ok(mut slot) = wake_ctx.lock() {
                *slot = Some(cc.egui_ctx.clone());
            }
            Ok(Box::new(MagicDesktopApp::new(
                cc,
                session,
                initial_view,
                in_window_menus,
                control_input_only,
                control_rx,
            )))
        }),
    );
    if let Err(error) = run_res {
        diag(format_args!("magic: error while running Magic Studio: {error}"));
        std::process::exit(1);
    }
}
