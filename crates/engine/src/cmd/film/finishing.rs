//! Complete `FilmCraft` Studio NLE finishing command dispatcher (`video.finish` — `T32`–`T42`).
//!
//! Maps structured `Magic Studio` UI finishing actions onto `FilmCraft` commands (`source.*`,
//! `timeline.*`, `sequence.*`, `trim.*`, `clip.*`, `media.*`, `lumetri.*`, `lut.*`, `effects.*`,
//! `essentialSound.*`, `mixer.*`, `clipMixer.*`, `graphics.*`, `captions.*`, `export.queue.*`,
//! `jobs.*`), applying them immediately to `film/<song>.fcproj` via [`super::fcproj_doc`] so edits
//! happen live inside Magic Studio's `05 Studio` UI.

use serde_json::{Value, json};

use crate::{Result, Session};

use super::{bad, film};

fn map_edit_and_trim_op(op: &str, p: &Value) -> Option<(&'static str, Value, bool)> {
    match op {
        "workspace" => Some(("workspace.select", json!({"name": p["preset"].as_str().unwrap_or("Assembly")}), false)),
        "toggle_proxies" => Some(("media.toggleProxies", json!({"enabled": p["enabled"].as_bool().unwrap_or(true)}), true)),
        "mark_in" => Some(("source.markIn", json!({"seconds": p["seconds"].as_f64().unwrap_or(0.0)}), true)),
        "mark_out" => Some(("source.markOut", json!({"seconds": p["seconds"].as_f64().unwrap_or(5.0)}), true)),
        "insert" => Some((
            "timeline.insert",
            json!({
                "track": p["track"].as_str().unwrap_or("V1"),
                "audioTrack": p["audioTrack"].as_str().unwrap_or("A1"),
                "name": p["name"].as_str().unwrap_or("Inserted Clip"),
            }),
            true,
        )),
        "overwrite" => Some((
            "timeline.overwrite",
            json!({
                "track": p["track"].as_str().unwrap_or("V1"),
                "audioTrack": p["audioTrack"].as_str().unwrap_or("A1"),
                "name": p["name"].as_str().unwrap_or("Overwritten Clip"),
            }),
            true,
        )),
        "lift" => Some(("sequence.lift", json!({}), true)),
        "extract" => Some(("sequence.extract", json!({"ripple": true}), true)),
        "replace_from_bin" => Some(("clip.replaceFromBin", json!({"matchFrame": p["match_frame"].as_bool().unwrap_or(true)}), true)),
        "track_patch" => Some((
            "timeline.setTrack",
            json!({
                "track": p["track"].as_str().unwrap_or("V1"),
                "patched": p["patched"].as_bool().unwrap_or(true),
                "target": p["target"].as_bool().unwrap_or(true),
                "syncLock": p["sync_lock"].as_bool().unwrap_or(true),
                "muted": p["muted"].as_bool().unwrap_or(false),
                "solo": p["solo"].as_bool().unwrap_or(false),
            }),
            true,
        )),
        "add_edit" => Some((
            "sequence.addEdit",
            json!({
                "seconds": p["seconds"].as_f64().unwrap_or(0.0),
                "allTracks": p["all_tracks"].as_bool().unwrap_or(false),
            }),
            true,
        )),
        "razor" => Some((
            "timeline.razor",
            json!({
                "seconds": p["seconds"].as_f64().unwrap_or(0.0),
                "track": p.get("track").and_then(Value::as_str),
            }),
            true,
        )),
        "clip_trim" => Some((
            "clip.trim",
            json!({
                "clip": p["clip"],
                "start_sec": p["start_sec"],
                "end_sec": p["end_sec"],
                "name": p["name"],
            }),
            true,
        )),
        "clip_move" => Some((
            "clip.move",
            json!({
                "clip": p["clip"],
                "start_sec": p["start_sec"],
                "name": p["name"],
            }),
            true,
        )),
        "clip_delete" => Some((
            "clip.delete",
            json!({
                "clip": p["clip"],
                "ripple": p["ripple"].as_bool().unwrap_or(false),
            }),
            true,
        )),
        "clip_rename" => Some(("clip.rename", json!({"clip": p["clip"], "name": p["name"]}), true)),
        "nest" => Some(("sequence.nest", json!({"name": p["name"].as_str().unwrap_or("Nested Scene")}), true)),
        "trim_mode" => Some((
            "trim.selectEditPoint",
            json!({
                "kind": p["kind"].as_str().unwrap_or("ripple"),
                "linked": p["linked"].as_bool().unwrap_or(true),
            }),
            true,
        )),
        "trim_nudge" => Some(("trim.nudge", json!({"frames": p["frames"].as_i64().unwrap_or(1), "clip": p["clip"]}), true)),
        "ripple_to_playhead" => Some((
            "trim.extendToPlayhead",
            json!({
                "side": p["side"].as_str().unwrap_or("prev"),
                "ripple": true,
                "seconds": p["seconds"].as_f64().unwrap_or(0.0),
            }),
            true,
        )),
        "rate_stretch" => Some((
            "clip.rateStretch",
            json!({
                "clip": p["clip"],
                "speedPct": p["speed_pct"].as_f64().unwrap_or(100.0),
                "maintainPitch": p["maintain_pitch"].as_bool().unwrap_or(true),
                "algorithm": "wsola",
                "interpolation": p["interpolation"].as_str().unwrap_or("opticalFlow"),
            }),
            true,
        )),
        "trim_shuttle" => Some((
            "trim.shuttle",
            json!({
                "direction": p["direction"].as_i64().unwrap_or(1),
                "clock": p["clock"].as_f64().unwrap_or(0.0),
            }),
            false,
        )),
        "trim_stop" => Some(("trim.shuttleStop", json!({"clock": p["clock"].as_f64().unwrap_or(0.0)}), true)),
        "sequence_rate" => Some((
            "sequence.settings",
            json!({
                "fps": p.get("fps").cloned().unwrap_or_else(|| json!([24, 1])),
                "dropFrame": p["drop_frame"].as_bool().unwrap_or(false),
            }),
            true,
        )),
        _ => None,
    }
}

fn map_color_and_fx_op(op: &str, p: &Value) -> Option<(&'static str, Value, bool)> {
    match op {
        "color_space" => Some((
            "sequence.colorSettings",
            json!({
                "workingSpace": p["working_space"].as_str().unwrap_or("rec709"),
                "wideGamut": p["wide_gamut"].as_bool().unwrap_or(false),
                "autoToneMap": p["auto_tone_map"].as_bool().unwrap_or(true),
                "hdrWhiteNits": p["hdr_white_nits"].as_u64().unwrap_or(203),
            }),
            true,
        )),
        "interpret_log" => Some(("clip.interpretFootage", json!({"colorSpace": p["log_curve"].as_str().unwrap_or("Sony S-Log3")}), true)),
        "lumetri_section" => Some((
            "lumetri.setSection",
            json!({
                "section": p["section"].as_str().unwrap_or("basic"),
                "on": p["on"].as_bool().unwrap_or(true),
            }),
            true,
        )),
        "lumetri_look" => Some((
            "lumetri.setLook",
            json!({
                "lut": p["look"].as_str().unwrap_or("builtin:warm_storybook"),
                "intensity": p["intensity"].as_f64().unwrap_or(80.0),
            }),
            true,
        )),
        "lumetri_wheel" => Some((
            "effects.setParam",
            json!({
                "effect": "lumetri",
                "section": "colorWheels",
                "wheel": p["wheel"].as_str().unwrap_or("midtones"),
                "x": p["x"].as_f64().unwrap_or(0.0),
                "y": p["y"].as_f64().unwrap_or(0.0),
                "luma": p["luma"].as_f64().unwrap_or(0.0),
            }),
            true,
        )),
        "lumetri_curve" => Some((
            "effects.setParam",
            json!({
                "effect": "lumetri",
                "section": "curves",
                "channel": p["channel"].as_str().unwrap_or("rgb"),
                "interpolation": "monotoneCubic",
                "points": p.get("points").cloned().unwrap_or_else(|| json!([[0.0, 0.0], [0.5, 0.5], [1.0, 1.0]])),
            }),
            true,
        )),
        "lumetri_param" | "fx_param" => Some((
            "effects.setParam",
            json!({
                "effect": p["effect"].as_str().unwrap_or("lumetri"),
                "param": p["param"].as_str().unwrap_or("exposure"),
                "value": p.get("value").cloned().unwrap_or_else(|| json!(0.0)),
            }),
            true,
        )),
        "lumetri_match" => Some((
            "lumetri.applyMatch",
            json!({
                "referenceSeconds": p["reference_seconds"].as_f64().unwrap_or(0.0),
                "faceDetection": p["face_detection"].as_bool().unwrap_or(true),
            }),
            true,
        )),
        "fx_apply" | "transition_apply" => Some(("effects.apply", json!({"effect": p["effect"].as_str().unwrap_or("Cross Dissolve")}), true)),
        "clip_motion" => Some((
            "clip.motion",
            json!({
                "clip": p["clip"],
                "position": p.get("position").cloned().unwrap_or_else(|| json!([960.0, 540.0])),
                "scale": p["scale"].as_f64().unwrap_or(100.0),
                "rotation": p["rotation"].as_f64().unwrap_or(0.0),
                "anchor": p.get("anchor").cloned().unwrap_or_else(|| json!([960.0, 540.0])),
                "antiFlicker": p["anti_flicker"].as_f64().unwrap_or(0.0),
            }),
            true,
        )),
        "blend_mode" => Some(("clip.setBlendMode", json!({"clip": p["clip"], "mode": p["mode"].as_str().unwrap_or("Normal")}), true)),
        "keyframe_toggle" => Some((
            "effects.addKeyframe",
            json!({
                "effect": p["effect"].as_str().unwrap_or("motion"),
                "param": p["param"].as_str().unwrap_or("scale"),
                "seconds": p["seconds"].as_f64().unwrap_or(0.0),
                "interpolation": p["interpolation"].as_str().unwrap_or("bezier"),
            }),
            true,
        )),
        _ => None,
    }
}

fn map_audio_and_titles_op(op: &str, p: &Value) -> Option<(&'static str, Value, bool)> {
    match op {
        "audio_gain" => Some((
            "clip.audioGain",
            json!({
                "mode": p["mode"].as_str().unwrap_or("normalizeAll"),
                "db": p["db"].as_f64().unwrap_or(-14.0),
                "targetLufs": p["target_lufs"].as_f64().unwrap_or(-14.0),
                "truePeakLimitDbtp": p["true_peak_dbtp"].as_f64().unwrap_or(-1.0),
            }),
            true,
        )),
        "audio_crossfade" => Some((
            "effects.apply",
            json!({
                "effect": p["transition"].as_str().unwrap_or("Constant Power"),
                "durationSeconds": p["duration"].as_f64().unwrap_or(0.5),
            }),
            true,
        )),
        "audio_ducking" => Some((
            "essentialSound.generateDucking",
            json!({
                "against": p["against"].as_str().unwrap_or("dialogue"),
                "reduceDb": p["reduce_db"].as_f64().unwrap_or(-9.5),
                "fadeS": p["fade_s"].as_f64().unwrap_or(0.35),
            }),
            true,
        )),
        "essential_sound" => {
            Some(("essentialSound.set", json!({"values": p.get("values").cloned().unwrap_or_else(|| json!({"repair.noise.on": true}))}), true))
        }
        "audio_sync" => Some(("sequence.syncAudio", json!({"method": "gcc_phat", "subsample": true}), true)),
        "mixer_strip" => Some((
            "mixer.setStrip",
            json!({
                "strip": p["strip"].as_str().unwrap_or("A1"),
                "volume": p["volume_db"].as_f64().unwrap_or(0.0),
                "pan": p["pan"].as_f64().unwrap_or(0.0),
                "mute": p["mute"].as_bool().unwrap_or(false),
                "solo": p["solo"].as_bool().unwrap_or(false),
                "mode": p["automation"].as_str().unwrap_or("Read"),
            }),
            true,
        )),
        "mixer_insert" => Some((
            "mixer.addInsert",
            json!({
                "strip": p["strip"].as_str().unwrap_or("A1"),
                "slot": p["slot"].as_u64().unwrap_or(0),
                "preFader": p["pre_fader"].as_bool().unwrap_or(true),
                "effect": p["effect"].as_str().unwrap_or("parametric_eq_full"),
            }),
            true,
        )),
        "mixer_automation" => Some((
            "mixer.writeAutomation",
            json!({
                "strip": p["strip"].as_str().unwrap_or("A1"),
                "mode": p["mode"].as_str().unwrap_or("Latch"),
            }),
            true,
        )),
        "clip_mixer" => Some(("clipMixer.set", json!({"gainDb": p["gain_db"].as_f64().unwrap_or(0.0), "pan": p["pan"].as_f64().unwrap_or(0.0)}), true)),
        "graphic_text" => Some((
            if p["vertical"].as_bool() == Some(true) { "graphics.newVerticalText" } else { "graphics.newText" },
            json!({
                "text": p["text"].as_str().unwrap_or("Song Title"),
                "size": p["size"].as_f64().unwrap_or(72.0),
                "font": p["font"].as_str().unwrap_or("Inter"),
                "fontStyle": p["font_style"].as_str().unwrap_or("SemiBold"),
                "seconds": p["duration"].as_f64().unwrap_or(5.0),
                "at_seconds": p["at_seconds"].as_f64().unwrap_or(0.0),
            }),
            true,
        )),
        "graphic_shape" => Some((
            "graphics.newShape",
            json!({
                "shape": p["shape"].as_str().unwrap_or("rectangle"),
                "size": p.get("size").cloned().unwrap_or_else(|| json!([640, 140])),
                "seconds": p["duration"].as_f64().unwrap_or(5.0),
                "at_seconds": p["at_seconds"].as_f64().unwrap_or(0.0),
            }),
            true,
        )),
        "graphic_style" => Some(("graphics.set", json!({"props": p.get("props").cloned().unwrap_or_else(|| json!({}))}), true)),
        "graphic_pin" => Some((
            "graphics.setPinning",
            json!({
                "pinTo": p["pin_to"].as_str().unwrap_or("videoFrame"),
                "edges": p.get("edges").cloned().unwrap_or_else(|| json!(["bottom", "left"])),
            }),
            true,
        )),
        "responsive_time" => Some((
            "graphics.setResponsiveTime",
            json!({
                "introFrames": p["intro_frames"].as_u64().unwrap_or(18),
                "outroFrames": p["outro_frames"].as_u64().unwrap_or(18),
            }),
            true,
        )),
        "roll_crawl" => Some((
            "graphics.setRoll",
            json!({
                "mode": p["mode"].as_str().unwrap_or("roll"),
                "startOffScreen": p["start_off"].as_bool().unwrap_or(true),
                "endOffScreen": p["end_off"].as_bool().unwrap_or(true),
                "easeInFrames": p["ease_in"].as_u64().unwrap_or(12),
                "easeOutFrames": p["ease_out"].as_u64().unwrap_or(12),
            }),
            true,
        )),
        "jobs_list" => Some(("jobs.list", json!({}), false)),
        _ => None,
    }
}

/// `video.finish` (`T32`–`T42`): execute structured FilmCraft Studio finishing operations directly on `.fcproj`.
pub fn cmd_video_finish(s: &mut Session, p: &Value) -> Result<Value> {
    let op = p["op"].as_str().ok_or_else(|| bad("op=<finishing operation> is required"))?;
    if op == "scopes" {
        let scopes = p.get("shown").cloned().unwrap_or_else(|| json!(["waveform", "vectorscopeYuv"]));
        let ui_payload = json!({
            "panels": {
                "scopes": {
                    "shown": scopes,
                    "colorSpace": p["color_space"].as_str().unwrap_or("rec709"),
                }
            }
        });
        return Ok(json!({"ok": true, "op": "scopes", "config": ui_payload}));
    }
    let f = film(s)?;
    let project = f.ensure_on_disk(s)?.to_path_buf();
    if matches!(op, "lut_import" | "load_lut") {
        let raw = p["path"].as_str().ok_or_else(|| bad("path=<.cube or .3dl> is required"))?;
        let lut_path = crate::workspace::resolve_workspace_path(s, raw);
        let res = super::execute(&project, "lut.import", json!({"path": lut_path}))?;
        return Ok(json!({"ok": true, "op": "lut_import", "lut": lut_path, "result": res}));
    }
    let (cmd_id, cmd_params, _save) = map_edit_and_trim_op(op, p)
        .or_else(|| map_color_and_fx_op(op, p))
        .or_else(|| map_audio_and_titles_op(op, p))
        .ok_or_else(|| bad(format!("unknown video.finish op `{op}`")))?;
    let result = super::execute(&project, cmd_id, cmd_params)?;
    Ok(json!({
        "ok": true,
        "op": op,
        "command": cmd_id,
        "timeline": project,
        "result": result,
    }))
}
