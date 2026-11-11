//! Pure-Rust `eframe` 0.37 + `egui` 0.37 + `storytold/craft-libs` desktop coordinator for Magic Studio.
//!
//! Owns native windowing (`craft-window`), system/in-window menus (`craft-menu`), non-blocking native
//! file pickers (`craft-dialogs`), keyboard shortcut dispatch (`craft-keymap`), cross-platform GPU
//! screenshots (`craft-ui` `CaptureSlot`), quit guards (`craft-quit`), and the loopback JSON-lines
//! `--control` socket while delegating media, compositing, cutout, vector, and storyboard layout to
//! installed ArtCraft binaries (`FilmCraft`, `EffectCraft`, `DesignCraft`, `PhotoCraft`,
//! `VectorCraft`, and headless `anicraft-cli`).

#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable))]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod capture;
mod clipboard_image;
mod control;
mod dialogs;
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
use magic_ui_egui::{ActiveModal, AutomationRegistry, MagicPalette, PickerTarget, UiAction, UiShellState, ViewCtx, ViewMode};
use rpc::{ControlEnvelope, DeferredReply};
use serde_json::{Value, json};

const CONTROL_REPLY_TIMEOUT: Duration = Duration::from_secs(60);
const APP_ICON_PNG: &[u8] = include_bytes!("../../../assets/app-icon/icon.png");

#[derive(Parser, Debug)]
#[command(name = "magic", about = "Magic Studio pure-Rust ArtCraft desktop coordinator")]
struct Cli {
    #[arg(long)]
    project: Option<String>,
    #[arg(
        long,
        default_value = "project",
        value_parser = ["home", "project", "storyboard", "animations", "animation", "design", "lyrics", "motion", "studio"]
    )]
    view: String,
    /// Open loopback control socket (`127.0.0.1:9876` or port `9876`) so `magic-cli mcp --connect` watches live updates
    #[arg(long, env = "MAGIC_CONTROL_PORT")]
    control: Option<String>,
    /// Force in-window `craft-menu` bar even on macOS (`MAGIC_IN_WINDOW_MENUS=1`)
    #[arg(long, env = "MAGIC_IN_WINDOW_MENUS", default_value_t = false)]
    in_window_menus: bool,
    /// Confine control-channel reads to this directory (repeatable)
    #[arg(long = "automation-read-root")]
    automation_read_roots: Vec<String>,
    /// Confine control-channel writes to this directory (repeatable)
    #[arg(long = "automation-write-root")]
    automation_write_roots: Vec<String>,
}

/// Control parameters are hostile input: a key the method does not name is an error.
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
        sync_frame: Box::new(move |egui_ctx, state, modal, is_enabled| {
            let mut menu = frame_ref.borrow_mut();
            menu.set_gate(Gate { modal, recording_shortcut: false, text_editing: egui_ctx.egui_wants_keyboard_input() });
            let clicks = menu.take_clicks(egui_ctx);
            let hash = egui_ctx.cumulative_frame_nr();
            let _ = menu.sync_with(hash, || {
                let updated = build_menu_bar(state, keymap::catalog_keymap(), is_enabled);
                mac_layout(&updated, &LayoutOptions::new("Magic Studio"))
            });
            clicks
        }),
    })
}

pub(crate) struct MagicDesktopApp {
    pub(crate) session: Session,
    pub(crate) ui_state: UiShellState,
    pub(crate) registry: AutomationRegistry,
    pub(crate) dialogs: dialogs::DialogHost,
    pub(crate) key_dispatcher: Dispatcher,
    pub(crate) lifecycle: lifecycle::Lifecycle,
    pub(crate) control_rx: Option<mpsc::Receiver<ControlEnvelope>>,
    pub(crate) synthetic_events: VecDeque<Event>,
    pub(crate) deferred_replies: Vec<DeferredReply>,
    pub(crate) pending_captures: Vec<capture::PendingCapture>,
    pub(crate) last_render_ms: f32,
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
        control_rx: Option<mpsc::Receiver<ControlEnvelope>>,
    ) -> Self {
        magic_ui_egui::install_loaders(&cc.egui_ctx);
        let mut ui_state = UiShellState { view: initial_view, ..UiShellState::default() };
        MagicPalette::dark().apply(&cc.egui_ctx);

        #[cfg(target_os = "macos")]
        let native_menu = if in_window_menus { None } else { install_macos_menu(&cc.egui_ctx, &ui_state) };
        #[cfg(target_os = "macos")]
        let use_in_window = in_window_menus || native_menu.is_none();
        #[cfg(not(target_os = "macos"))]
        let use_in_window = {
            let _ = in_window_menus;
            true
        };

        refresh_cached_state(&mut session, &mut ui_state, true);
        let now = Instant::now();
        Self {
            session,
            ui_state,
            registry: AutomationRegistry::default(),
            dialogs: dialogs::DialogHost::new(&cc.egui_ctx),
            key_dispatcher: Dispatcher::new(),
            lifecycle: lifecycle::Lifecycle::default(),
            control_rx,
            synthetic_events: VecDeque::new(),
            deferred_replies: Vec::new(),
            pending_captures: Vec::new(),
            last_render_ms: 0.0,
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
        if matches!(id, "project.open" | "project.new" | "project.close" | "file.open" | "file.new" | "file.close") && self.session.project_path.is_some() {
            lifecycle::verify_clean(&self.session, &self.ui_state)?;
        }
        let result = if id == "clipboard.import_image" {
            clipboard_image::import_image(&mut self.session, &params)?
        } else {
            self.session.execute(id, params).map_err(|e| e.to_string())?
        };
        self.capture_query_cache(id, &result);
        if changed {
            refresh_cached_state(&mut self.session, &mut self.ui_state, false);
            ctx.request_repaint();
        }
        Ok(result)
    }

    fn capture_query_cache(&mut self, id: &str, result: &Value) {
        match id {
            "project.catalog" => self.ui_state.cache.catalog = result.clone(),
            "storyboard.pages" => self.ui_state.cache.storyboard_pages = result.clone(),
            "storyboard.check" => self.ui_state.cache.storyboard_check = result.clone(),
            "words.read" => self.ui_state.cache.words_doc = result.clone(),
            "words.effects" => self.ui_state.cache.words_effects = result.clone(),
            "motion.read" => self.ui_state.cache.motion_doc = result.clone(),
            "motion.effects" => self.ui_state.cache.motion_effects = result.clone(),
            "video.timeline" => self.ui_state.cache.timeline = result.clone(),
            "video.film" => self.ui_state.cache.film_status = result.clone(),
            "app.doctor" => self.ui_state.cache.doctor = result.clone(),
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
                    _ => {}
                }
                self.refresh_view_data(view);
            }
            UiAction::SelectCharacter { id, jump_animation } => {
                self.ui_state.selected_character_id = Some(id);
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
                self.ui_state.set_toast(now, if self.ui_state.window_snapped_right { "Snapped window right" } else { "Snapped window left" });
            }
            UiAction::OpenPreferences => self.ui_state.active_modal = Some(ActiveModal::Preferences),
            UiAction::OpenDoctor => {
                let _ = self.execute_engine(ctx, "app.doctor", json!({}));
                self.ui_state.active_modal = Some(ActiveModal::Doctor);
            }
            UiAction::OpenAbout => self.ui_state.active_modal = Some(ActiveModal::About),
            UiAction::CloseProject => {
                self.ui_state.audio_playing = false;
                match self.execute_engine(ctx, "file.close", json!({})) {
                    Ok(_) => self.ui_state.view = ViewMode::Home,
                    Err(err) => self.ui_state.set_toast(now, err),
                }
            }
            UiAction::RequestQuit => {
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
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
            ViewMode::Storyboard => {
                if let Ok(pages) = self.session.execute("storyboard.pages", json!({})) {
                    self.ui_state.cache.storyboard_pages = pages;
                }
                if let Ok(chk) = self.session.execute("storyboard.check", json!({})) {
                    self.ui_state.cache.storyboard_check = chk;
                }
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
                if let Ok(film) = self.session.execute("video.film", json!({})) {
                    self.ui_state.cache.film_status = film;
                }
                if let Ok(tl) = self.session.execute("video.timeline", json!({})) {
                    self.ui_state.cache.timeline = tl;
                }
            }
            ViewMode::Project | ViewMode::Animation => {}
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
                        let _ = self.execute_engine(ctx, "project.add_asset", json!({"path": path_str, "kind": "image"}));
                    }
                    PickerTarget::ImportTrunkFile => {
                        let _ = self.execute_engine(ctx, "project.import_file", json!({"path": path_str}));
                    }
                    PickerTarget::MotionTraceImage => {
                        let _ = self.execute_engine(ctx, "vector.trace", json!({"path": path_str, "ignoreWhite": true}));
                    }
                }
            }
        }
    }
}

impl eframe::App for MagicDesktopApp {
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        #[cfg(target_os = "macos")]
        if let Some(menu) = &mut self.native_menu {
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

        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).filter(|p| !p.as_os_str().is_empty()).collect());
        if !dropped.is_empty() {
            let now = ctx.input(|i| i.time);
            if self.ui_state.project_path.is_none() {
                self.ui_state.set_toast(now, "Open a project before dropping artwork");
            } else {
                let mut imported = 0usize;
                for path in dropped {
                    if self.execute_engine(ctx, "project.add_asset", json!({"path": path, "kind": "image"})).is_ok() {
                        imported += 1;
                    }
                }
                if imported > 0 {
                    self.ui_state.set_toast(now, format!("Imported {imported} dropped file(s)"));
                }
            }
        }

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
        let palette = MagicPalette::for_mode(self.ui_state.theme_mode, system_dark);
        palette.apply(&egui_ctx);

        let modal_open = self.ui_state.active_modal.is_some() || self.dialogs.is_open();

        #[cfg(target_os = "macos")]
        if let Some(menu) = &mut self.native_menu {
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
            let mut view_ctx = ViewCtx { state: &mut self.ui_state, reg: &mut self.registry, pal: &palette, actions: &mut emitted };
            magic_ui_egui::shell::draw_workstation(ui, &mut view_ctx, keymap::catalog_keymap(), self.in_window_menus, |cid| {
                magic::find_command(cid).is_some_and(|spec| (spec.enabled)(session_ref).is_ok())
            });
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

fn refresh_cached_state(session: &mut Session, ui_state: &mut UiShellState, initial: bool) {
    if let Ok(snap) = session.execute("project.snapshot", json!({})) {
        ui_state.project_path = snap["project"].as_str().map(PathBuf::from);
        ui_state.cache.snapshot = snap;
    } else {
        ui_state.project_path = session.project_path.clone();
    }
    if initial
        && ui_state.project_path.is_none()
        && let Ok(cat) = session.execute("project.catalog", json!({}))
    {
        ui_state.cache.catalog = cat;
    }
    if let Some(song) = ui_state.song_audio_path()
        && let Ok(probe) = craft_util::media::probe_audio_in_process(std::path::Path::new(&song))
        && let Some(ticks) = probe["duration"].as_i64()
    {
        ui_state.audio_duration_sec = craft_util::time::Tick(ticks).seconds();
    }
}

fn load_app_icon() -> Option<Arc<egui::IconData>> {
    let decoded = image::load_from_memory(APP_ICON_PNG).ok()?.into_rgba8();
    let (width, height) = decoded.dimensions();
    Some(Arc::new(egui::IconData { rgba: decoded.into_raw(), width, height }))
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
            tx.send(ControlEnvelope { method: method.to_owned(), params, reply: reply_tx }).map_err(|_| "Magic desktop event loop has exited".to_string())?;
            if let Ok(guard) = wake_slot.lock()
                && let Some(ctx) = guard.as_ref()
            {
                ctx.request_repaint();
            }
            reply_rx.recv_timeout(CONTROL_REPLY_TIMEOUT).map_err(|_| "UI did not respond within 60 seconds".to_string())?
        }),
    )?;
    Ok((addr, rx))
}

fn main() {
    let cli = Cli::parse();
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
    let token = if cli.control.is_some() { magic::workspace::ensure_control_token().unwrap_or_default() } else { String::new() };
    if cli.control.is_some() && (token.len() != 64 || !token.bytes().all(|b| b.is_ascii_hexdigit())) {
        diag(format_args!("MAGIC_CONTROL_TOKEN must contain 64 hexadecimal characters"));
        std::process::exit(2);
    }

    craft_font_catalog::Catalog::global().scan_in_background();

    let mut session = Session::user_session().with_automation_roots(automation_roots);
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

    let options = eframe::NativeOptions { viewport, renderer: eframe::Renderer::Wgpu, ..Default::default() };

    let in_window_menus = cli.in_window_menus;
    let run_res = eframe::run_native(
        "Magic Studio",
        options,
        Box::new(move |cc| {
            if let Ok(mut slot) = wake_ctx.lock() {
                *slot = Some(cc.egui_ctx.clone());
            }
            Ok(Box::new(MagicDesktopApp::new(cc, session, initial_view, in_window_menus, control_rx)))
        }),
    );
    if let Err(error) = run_res {
        diag(format_args!("magic: error while running Magic Studio: {error}"));
        std::process::exit(1);
    }
}
