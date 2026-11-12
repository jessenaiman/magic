//! Underlying song & `FilmCraft` / `EffectCraft` audio transport for `Animations`, `Lyrics`,
//! `Motion`, `Project`, and `Studio` (`Storyboard` remains silent editorial print layout).
//!
//! Pure-Rust RIFF `.wav` seek-slicer + non-blocking OS audio child process (`afplay`, `pw-play`,
//! `paplay`, `aplay`, `ffplay`, or `Media.SoundPlayer`) with local transport sync

use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use craft_dialogs::Answer;
use magic_ui_egui::{PickerTarget, UiShellState, ViewMode};
use serde_json::{Value, json};

use crate::MagicDesktopApp;
use crate::diagnostics;

#[derive(Default)]
pub(crate) struct AudioTransportPlayer {
    child: Option<Child>,
    sliced_wav_path: Option<PathBuf>,
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
        if let Some(tmp) = self.sliced_wav_path.take() {
            let _ = std::fs::remove_file(tmp);
        }
    }

    pub(crate) fn toggle(&mut self, state: &mut UiShellState) {
        if matches!(state.effective_view(), ViewMode::Home | ViewMode::Storyboard) {
            self.stop(state);
            return;
        }
        if state.audio_playing {
            self.stop(state);
        } else {
            let start = active_stage_playhead(state);
            self.start_at(state, start);
        }
    }

    pub(crate) fn seek(&mut self, state: &mut UiShellState, seconds: f64) {
        let clamped = seconds.max(0.0);
        state.audio_time_sec = clamped;
        sync_stage_playheads(state, clamped);
        sync_live_craft_seek(state, clamped);
        if state.audio_playing {
            self.start_at(state, clamped);
        }
    }

    pub(crate) fn stop(&mut self, state: &mut UiShellState) {
        state.audio_playing = false;
        self.stop_os_process();
        sync_live_craft_play(state, false, state.audio_time_sec);
    }

    pub(crate) fn start_at(&mut self, state: &mut UiShellState, start_sec: f64) {
        self.stop_os_process();
        let Some(audio_file) = resolve_active_audio_file(state) else {
            state.audio_playing = false;
            return;
        };
        let clamped = start_sec.max(0.0);
        state.audio_time_sec = clamped;
        sync_stage_playheads(state, clamped);

        let play_target = if clamped > 0.05 && audio_file.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("wav")) {
            match slice_riff_wav_from(&audio_file, clamped) {
                Some(tmp) => {
                    self.sliced_wav_path = Some(tmp.clone());
                    tmp
                }
                None => audio_file,
            }
        } else {
            audio_file
        };

        self.child = spawn_os_audio_player(&play_target, clamped);
        state.audio_playing = true;
        sync_live_craft_play(state, true, clamped);
    }

    pub(crate) fn tick_frame(&mut self, state: &mut UiShellState, dt: f64) -> bool {
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
        let max_dur = state.audio_duration_sec;
        if max_dur > 0.0 && state.audio_time_sec >= max_dur {
            state.audio_time_sec = 0.0;
            sync_stage_playheads(state, 0.0);
            self.stop(state);
            return false;
        }
        sync_stage_playheads(state, state.audio_time_sec);
        true
    }
}

fn active_stage_playhead(state: &UiShellState) -> f64 {
    match state.effective_view() {
        ViewMode::Animation => state.animation.scrub_seconds,
        ViewMode::Design => state.lyrics.scrub_seconds,
        ViewMode::Motion => state.motion.start_sec,
        ViewMode::Studio => state.studio.seek_seconds,
        _ => state.audio_time_sec,
    }
}

fn sync_stage_playheads(state: &mut UiShellState, song_sec: f64) {
    match state.effective_view() {
        ViewMode::Animation => {
            if state.animation.track_source == "effectcraft" && state.animation.loop_clip {
                let cyc = state.animation.action_duration.max(0.4);
                state.animation.scrub_seconds = song_sec % cyc;
            } else {
                state.animation.scrub_seconds = song_sec;
            }
        }
        ViewMode::Design => {
            state.lyrics.scrub_seconds = song_sec;
            if let Some(anims) = state.cache.words_doc.pointer("/document/animations").and_then(Value::as_array) {
                for anim in anims {
                    let st = anim["song_anchor"].as_f64().unwrap_or(0.0);
                    let dur = anim["authored_duration"].as_f64().unwrap_or(3.0).max(0.25);
                    if song_sec >= st
                        && song_sec <= st + dur
                        && let Some(aid) = anim["animation_id"].as_str()
                    {
                        state.lyrics.selected_anim_id = Some(aid.to_owned());
                        break;
                    }
                }
            }
        }
        ViewMode::Motion => {
            state.motion.start_sec = song_sec;
        }
        ViewMode::Studio => {
            state.studio.seek_seconds = song_sec;
        }
        _ => {}
    }
}

fn resolve_active_audio_file(state: &UiShellState) -> Option<PathBuf> {
    let want_fc_mix = match state.effective_view() {
        ViewMode::Animation => state.animation.audio_source == "filmcraft",
        ViewMode::Design => state.lyrics.audio_source == "filmcraft",
        ViewMode::Motion => state.motion.audio_source == "filmcraft",
        ViewMode::Studio => true,
        _ => false,
    };
    if want_fc_mix && let Some(proj_dir) = &state.project_path {
        for rel in ["film/audio.wav", "film/preview.wav", "film/mix.wav", "film/preview.mp4", "film/master.mp4"] {
            let candidate = proj_dir.join(rel);
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
    state.project_path.as_ref().map(|root| root.join(raw)).filter(|c| c.is_file())
}

fn slice_riff_wav_from(src: &Path, start_sec: f64) -> Option<PathBuf> {
    let mut file = File::open(src).ok()?;
    let mut header = [0_u8; 12];
    file.read_exact(&mut header).ok()?;
    if &header[0..4] != b"RIFF" || &header[8..12] != b"WAVE" {
        return None;
    }
    let mut fmt_bytes: Option<Vec<u8>> = None;
    let mut sample_rate = 0_u32;
    let mut block_align = 0_u16;
    let mut data_offset = 0_u64;
    let mut data_len = 0_u32;

    loop {
        let mut ch_hdr = [0_u8; 8];
        if file.read_exact(&mut ch_hdr).is_err() {
            break;
        }
        let cid = &ch_hdr[0..4];
        let clen = u32::from_le_bytes([ch_hdr[4], ch_hdr[5], ch_hdr[6], ch_hdr[7]]);
        let pos = file.stream_position().ok()?;
        if cid == b"fmt " && clen >= 16 && clen <= 256 {
            let mut buf = vec![0_u8; clen as usize];
            file.read_exact(&mut buf).ok()?;
            sample_rate = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
            block_align = u16::from_le_bytes([buf[12], buf[13]]);
            fmt_bytes = Some(buf);
        } else if cid == b"data" {
            data_offset = pos;
            data_len = clen;
            break;
        }
        let padded = u64::from(clen) + u64::from(clen % 2);
        file.seek(SeekFrom::Start(pos + padded)).ok()?;
    }

    let fmt = fmt_bytes?;
    if sample_rate == 0 || block_align == 0 || data_len == 0 {
        return None;
    }
    let frame_offset = (start_sec * f64::from(sample_rate)).floor().max(0.0) as u64;
    let byte_skip = (frame_offset * u64::from(block_align)).min(u64::from(data_len));
    let rem_len = u64::from(data_len).saturating_sub(byte_skip) as u32;
    if rem_len < u32::from(block_align) {
        return None;
    }

    file.seek(SeekFrom::Start(data_offset + byte_skip)).ok()?;
    let mut pcm = vec![0_u8; rem_len as usize];
    file.read_exact(&mut pcm).ok()?;

    let out_path = std::env::temp_dir().join(format!("magic-scrub-{}.wav", std::process::id()));
    let mut out = File::create(&out_path).ok()?;
    let riff_size = 4_u32.saturating_add(8 + fmt.len() as u32).saturating_add(8 + rem_len);
    out.write_all(b"RIFF").ok()?;
    out.write_all(&riff_size.to_le_bytes()).ok()?;
    out.write_all(b"WAVEfmt ").ok()?;
    out.write_all(&(fmt.len() as u32).to_le_bytes()).ok()?;
    out.write_all(&fmt).ok()?;
    out.write_all(b"data").ok()?;
    out.write_all(&rem_len.to_le_bytes()).ok()?;
    out.write_all(&pcm).ok()?;
    Some(out_path)
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

fn sync_live_craft_play(_state: &UiShellState, _playing: bool, _sec: f64) {}

fn sync_live_craft_seek(_state: &UiShellState, _sec: f64) {}

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
            let ext = first.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).unwrap_or_default();
            match target {
                PickerTarget::OpenProject => {
                    let _ = app.execute_engine(ctx, "file.open", json!({"path": path_str}));
                    app.ui_state.view = ViewMode::Project;
                }
                PickerTarget::SaveProjectAs => {
                    let _ = app.execute_engine(ctx, "file.saveAs", json!({"path": path_str}));
                }
                PickerTarget::NewProjectRoot => app.ui_state.home.new_root = path_str,
                PickerTarget::NewProjectAudio => app.ui_state.home.new_audio = path_str,
                PickerTarget::SetSongAudio => {
                    let _ = app.execute_engine(ctx, "workshop.set_audio", json!({"path": path_str}));
                }
                PickerTarget::ImportTranscript => {
                    let _ = app.execute_engine(ctx, "workshop.import_transcript", json!({"path": path_str, "replace": true}));
                }
                PickerTarget::AddProjectAsset => {
                    if matches!(ext.as_str(), "ecproj" | "pcraft" | "psd" | "vectorcraft" | "anicraft") {
                        let _ = app.execute_engine(ctx, "character.import", json!({"path": path_str}));
                    } else {
                        let _ = app.execute_engine(ctx, "project.add_asset", json!({"path": path_str, "kind": "image"}));
                    }
                }
                PickerTarget::ImportTrunkFile => {
                    if matches!(ext.as_str(), "ecproj" | "pcraft" | "psd" | "vectorcraft" | "anicraft") {
                        let _ = app.execute_engine(ctx, "character.import", json!({"path": path_str}));
                    } else {
                        let _ = app.execute_engine(ctx, "project.import_file", json!({"path": path_str}));
                    }
                }
                PickerTarget::MotionTraceImage => {
                    let _ = app.execute_engine(ctx, "project.add_asset", json!({"path": path_str, "kind": "image", "vectorize": true}));
                }
                PickerTarget::ImportLottie => {
                    app.ui_state.motion.lottie_show_warnings = true;
                    let _ = app.execute_engine(ctx, "project.import_file", json!({"path": path_str}));
                }
                PickerTarget::ImportAep => {
                    let _ = app.execute_engine(ctx, "project.import_file", json!({"path": path_str}));
                }
                PickerTarget::ImportLut => {
                    let _ = app.execute_engine(ctx, "video.finish", json!({"op": "lut_import", "path": path_str}));
                    app.ui_state.set_toast(now, "Loaded 3D LUT (.cube / .3dl)");
                }
                PickerTarget::ImportFcgt => {
                    let _ = app.execute_engine(ctx, "project.import_file", json!({"path": path_str}));
                    app.ui_state.set_toast(now, "Installed native graphic template");
                }
                PickerTarget::ImportFontDir => {
                    app.ui_state.lyrics.font_dir_extra = path_str.clone();
                    diagnostics::apply_app_font_dirs(&path_str);
                    let _ = app.execute_engine(ctx, "prefs.set", json!({"key": "lyrics.font_dirs", "value": path_str}));
                    app.ui_state.set_toast(now, "Saved EFFECTCRAFT_FONT_DIRS folder");
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
                PickerTarget::StoryboardSlotImage { page, slot } => {
                    let _ = app.execute_engine(ctx, "storyboard.set", json!({"page": page, "label": slot, "image": path_str}));
                    app.ui_state.set_toast(now, "Placed image onto storyboard.pdf");
                }
            }
        }
    }
}
