//! Song & `FilmCraft` / `EffectCraft` audio transport powered by `craft-audio` (`ReadSeekSource`,
//! `Decoder::open`, sample-exact `Decoder::seek`, and 24-bit `FileEncoder`) across every format in
//! `craft_audio::DECODE_EXTENSIONS` (`WAV`, `BWF`, `RF64`, `AIFF`, `FLAC`, `AAC`/`M4A`, `ALAC`,
//! `MP1`/`MP2`/`MP3`, `Opus`, `Ogg`, `AC-3`/`E-AC-3`, `Matroska`/`WebM`).

use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use craft_audio::{DecodeOptions, Decoder, EncodeOptions, FileEncoder, ReadSeekSource};
use craft_dialogs::Answer;
use magic_ui_egui::{MagicPalette, PickerTarget, StageHost, UiShellState, ViewCtx, ViewMode};
use serde_json::json;

use crate::MagicDesktopApp;

#[derive(Default)]
pub(crate) struct AudioTransportPlayer {
    child: Option<Child>,
    seek_wav_path: Option<PathBuf>,
    last_spawn_at: Option<std::time::Instant>,
}

impl Drop for AudioTransportPlayer {
    fn drop(&mut self) {
        self.stop_os_process();
    }
}

impl AudioTransportPlayer {
    pub(crate) fn stop_os_process(&mut self) {
        if let Some(mut proc) = self.child.take() {
            let _ = proc.kill();
            let _ = proc.wait();
        }
        if let Some(tmp) = self.seek_wav_path.take() {
            let _ = std::fs::remove_file(tmp);
        }
    }

    pub(crate) fn toggle(&mut self, state: &mut UiShellState, stage_host: &mut StageHost) {
        if matches!(state.effective_view(), ViewMode::Home | ViewMode::Storyboard) {
            self.stop(state);
            return;
        }
        if state.audio_playing {
            self.stop(state);
        } else {
            let start = active_stage_playhead(state, stage_host);
            self.start_at(state, stage_host, start);
        }
    }

    pub(crate) fn seek(&mut self, state: &mut UiShellState, stage_host: &mut StageHost, seconds: f64) {
        let clamped = seconds.max(0.0);
        state.audio_time_sec = clamped;
        sync_stage_playheads(state, stage_host, clamped);
        if state.audio_playing && self.last_spawn_at.is_none_or(|t| t.elapsed() >= std::time::Duration::from_millis(180)) {
            self.start_at(state, stage_host, clamped);
        }
    }

    pub(crate) fn stop(&mut self, state: &mut UiShellState) {
        state.audio_playing = false;
        self.stop_os_process();
    }

    pub(crate) fn start_at(&mut self, state: &mut UiShellState, stage_host: &mut StageHost, start_sec: f64) {
        self.stop_os_process();
        let clamped = start_sec.max(0.0);
        state.audio_time_sec = clamped;
        sync_stage_playheads(state, stage_host, clamped);

        if let Some(audio_file) = resolve_active_audio_file(state, stage_host) {
            let play_target = match render_seek_wav_with_craft_audio(&audio_file, clamped) {
                Some(tmp) => {
                    self.seek_wav_path = Some(tmp.clone());
                    tmp
                }
                None => audio_file,
            };
            self.child = spawn_os_audio_player(&play_target, clamped);
            self.last_spawn_at = Some(std::time::Instant::now());
        }
        state.audio_playing = true;
    }

    pub(crate) fn tick_frame(&mut self, state: &mut UiShellState, stage_host: &mut StageHost, dt: f64) -> bool {
        if !state.audio_playing {
            return false;
        }
        if matches!(state.effective_view(), ViewMode::Home | ViewMode::Storyboard) {
            self.stop(state);
            return false;
        }
        if let Some(proc) = &mut self.child
            && let Ok(Some(_)) = proc.try_wait()
        {
            self.child = None;
        }
        state.audio_time_sec += dt;
        let max_dur = if state.audio_duration_sec > 0.0 { state.audio_duration_sec } else { state.timeline.range_end.max(30.0) };
        if state.audio_time_sec >= max_dur {
            state.audio_time_sec = 0.0;
            sync_stage_playheads(state, stage_host, 0.0);
            self.stop(state);
            return false;
        }
        sync_stage_playheads(state, stage_host, state.audio_time_sec);
        true
    }
}

fn active_stage_playhead(state: &UiShellState, stage_host: &StageHost) -> f64 {
    let view = state.effective_view();
    let stage = stage_host.stage(view);
    if stage.has_timeline() { stage.playhead_seconds() } else { state.audio_time_sec }
}

fn sync_stage_playheads(state: &mut UiShellState, stage_host: &mut StageHost, song_sec: f64) {
    stage_host.stage_mut(state.effective_view()).sync_from_song_time(song_sec, &state.cache);
}

fn resolve_active_audio_file(state: &UiShellState, stage_host: &StageHost) -> Option<PathBuf> {
    let proj_dir = state.project_path.as_deref().and_then(Path::parent);
    let want_fc_mix = stage_host.stage(state.effective_view()).uses_filmcraft_audio();
    if want_fc_mix && let Some(dir) = proj_dir {
        for rel in ["film/audio.wav", "film/preview.wav", "film/mix.wav", "film/preview.mp4", "film/master.mp4"] {
            let candidate = dir.join(rel);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    let raw = state.song_audio_path()?;
    let p = PathBuf::from(&raw);
    if p.is_file() {
        return Some(p);
    }
    proj_dir.map(|dir| dir.join(raw)).filter(|c| c.is_file())
}

/// Sample-exact seek & transcode to 24-bit WAV via `craft_audio::Decoder` (`ReadSeekSource`) +
/// `craft_audio::FileEncoder`, enabling every `DECODE_EXTENSIONS` codec (`m4a`, `aac`, `flac`,
/// `mp3`, `opus`, `ogg`, `aiff`, `ac3`, `wav`) to scrub and play cleanly on all desktop platforms.
fn render_seek_wav_with_craft_audio(src: &Path, start_sec: f64) -> Option<PathBuf> {
    let ext = src.extension().and_then(|e| e.to_str());
    let is_wav = ext.is_some_and(|e| e.eq_ignore_ascii_case("wav"));
    if start_sec <= 0.05 && is_wav {
        return None;
    }
    let file = File::open(src).ok()?;
    let source = ReadSeekSource::new(file).ok()?;
    let mut dec = Decoder::open(source, ext, &DecodeOptions::default()).ok()?;
    let rate = dec.info().sample_rate;
    let channels = dec.info().layout.channels();
    if rate == 0 || channels == 0 {
        return None;
    }
    let start_frame = (start_sec.max(0.0) * f64::from(rate)).floor() as u64;
    if start_frame > 0 {
        dec.seek(start_frame).ok()?;
    }
    let out_path = std::env::temp_dir().join(format!("magic-scrub-{}.wav", std::process::id()));
    let out_file = File::create(&out_path).ok()?;
    let mut enc = FileEncoder::new(out_file, &EncodeOptions::default(), rate, channels).ok()?;
    let mut written_frames = 0_usize;
    while let Some(block) = dec.next_block().ok()? {
        let n = block.frames();
        if n > 0 {
            enc.write(&block.channels, n).ok()?;
            written_frames += n;
        }
    }
    enc.finish().ok()?;
    (written_frames > 0).then_some(out_path)
}

fn spawn_os_audio_player(path: &Path, start_sec: f64) -> Option<Child> {
    let p_str = path.to_str()?;
    #[cfg(target_os = "macos")]
    {
        let _ = start_sec;
        return Command::new("afplay").arg(p_str).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().ok();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = start_sec;
        let escaped = p_str.replace('\'', "''");
        return Command::new("powershell")
            .args(["-NoProfile", "-Command", &format!("(New-Object Media.SoundPlayer '{escaped}').PlaySync()")])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok();
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        for tool in ["pw-play", "paplay", "aplay"] {
            if let Ok(child) = Command::new(tool).arg(p_str).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn() {
                return Some(child);
            }
        }
        Command::new("ffplay")
            .args(["-nodisp", "-autoexit", "-loglevel", "quiet", "-ss", &format!("{start_sec:.2}"), p_str])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok()
    }
}

/// Extracted file-picker answer handler so `apps/magic/src/main.rs` stays well below 900 lines.
pub(crate) fn dispatch_picker_answer(app: &mut MagicDesktopApp, ctx: &egui::Context, target: PickerTarget, answer: Answer) {
    let now = ctx.input(|i| i.time);
    match answer {
        Answer::Cancelled | Answer::Message(_) => {}
        Answer::Unavailable(why) => {
            app.ui_state.set_toast(now, format!("File dialog unavailable: {why}"));
        }
        Answer::Paths(paths) => {
            let Some(first) = paths.first() else { return };
            let path_str = first.to_string_lossy().into_owned();
            if app.stage_host.home_mut().on_picked_file(&target, &path_str) {
                return;
            }
            let target_view = match &target {
                PickerTarget::ImportLottie => ViewMode::Motion,
                _ => app.ui_state.effective_view(),
            };
            let mut emitted = Vec::new();
            let pal =
                MagicPalette::resolve(app.ui_state.theme_mode, app.ui_state.interface_style, app.ui_state.accent_choice, app.ui_state.high_contrast, None);
            let handled = {
                let mut view_ctx = ViewCtx { state: &mut app.ui_state, reg: &mut app.registry, pal: &pal, actions: &mut emitted };
                app.stage_host.stage_mut(target_view).on_picked_file(&target, &path_str, &mut view_ctx)
            };
            for act in emitted {
                app.handle_ui_action(ctx, act);
            }
            if handled {
                return;
            }

            let ext = first.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).unwrap_or_default();
            match target {
                PickerTarget::OpenProject => match app.execute_engine(ctx, "file.open", json!({"path": path_str})) {
                    Ok(_) => app.ui_state.view = ViewMode::Project,
                    Err(err) => app.ui_state.set_toast(now, format!("Open project: {err}")),
                },
                PickerTarget::SaveProjectAs => {
                    let _ = app.execute_engine(ctx, "file.saveAs", json!({"path": path_str}));
                }
                PickerTarget::SetSongAudio => {
                    let _ = app.execute_engine(ctx, "workshop.set_audio", json!({"path": path_str}));
                }
                PickerTarget::ImportTranscript => {
                    let _ = app.execute_engine(ctx, "workshop.import_transcript", json!({"path": path_str, "replace": true}));
                }
                PickerTarget::AddProjectAsset => {
                    if matches!(ext.as_str(), "ecproj" | "pcraft" | "psd" | "vectorcraft") {
                        let _ = app.execute_engine(ctx, "character.import", json!({"path": path_str}));
                    } else if matches!(ext.as_str(), "glb" | "gltf" | "obj" | "fbx" | "stl" | "ply" | "3mf") {
                        let cid = app.ui_state.selected_character_id.clone().unwrap_or_else(|| "lead".into());
                        let _ = app.execute_engine(ctx, "character.layer.new3dModel", json!({"character": cid, "path": path_str}));
                    } else {
                        let _ = app.execute_engine(ctx, "project.add_asset", json!({"path": path_str, "kind": "image"}));
                    }
                }
                PickerTarget::ImportCraftFile => {
                    if matches!(ext.as_str(), "ecproj" | "pcraft" | "psd" | "vectorcraft") {
                        let _ = app.execute_engine(ctx, "character.import", json!({"path": path_str}));
                    } else {
                        let _ = app.execute_engine(ctx, "project.import_file", json!({"path": path_str}));
                    }
                }
                PickerTarget::MotionTraceImage => {
                    let _ = app.execute_engine(ctx, "project.add_asset", json!({"path": path_str, "kind": "image", "vectorize": true}));
                }
                PickerTarget::ImportAep => {
                    let _ = app.execute_engine(ctx, "project.import_file", json!({"path": path_str}));
                }
                PickerTarget::ImportFcgt => {
                    let _ = app.execute_engine(ctx, "project.import_file", json!({"path": path_str}));
                    app.ui_state.set_toast(now, "Installed native graphic template");
                }
                PickerTarget::ImportWasmPlugin => {
                    let _ = app.execute_engine(ctx, "project.import_file", json!({"path": path_str}));
                    app.ui_state.set_toast(now, "Loaded WebAssembly (.wasm) plug-in into EffectCraft");
                }
                PickerTarget::ImportModel3d => {
                    let cid = app.ui_state.selected_character_id.clone().unwrap_or_else(|| "lead".into());
                    let _ = app.execute_engine(ctx, "character.layer.new3dModel", json!({"character": cid, "path": path_str}));
                    app.ui_state.set_toast(now, "Imported 3D model into EffectCraft comp");
                }
                PickerTarget::NewProjectRoot | PickerTarget::NewProjectAudio | PickerTarget::ImportLottie => {}
            }
        }
    }
}
