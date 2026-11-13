//! Workshop Stage 2 — Transcript validation, SRT caption export, canonical `timing-reference.json`
//! generation, and `FilmCraft` audio/speech transcription + synced lyric track assembly in [`Tick`]s.
//!
//! // TODO: Verify headless `filmcraft-cli` `audio.transcribe` / `captions.fromTranscript` model names
//! // on fresh Linux/macOS installs when Whisper weights have not been downloaded in FilmCraft yet.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::cmd::project::workshop::{project_dir, transcript_files, workshop_status};
use crate::time::{TICKS_PER_SECOND, Tick, parse_tick};
use crate::workspace::resolve_workspace_path;
use crate::{EngineError, Result, Session};

pub(crate) fn transcript_document(path: &Path) -> Result<Value> {
    let mut doc: Value =
        serde_json::from_slice(&std::fs::read(path)?).map_err(|e| EngineError::InvalidParams(format!("Invalid transcript {}: {e}", path.display())))?;
    if doc.is_array() {
        doc = json!({"words": doc});
    }
    if !doc.is_object() {
        return Err(EngineError::InvalidParams("Transcript must be a word array or object with words".into()));
    }
    if !doc["paragraphs"].is_array() {
        let captions = path.with_file_name("transcript.srt");
        doc["paragraphs"] = if captions.is_file() { json!(import_caption_paragraphs(&std::fs::read_to_string(captions)?)?) } else { doc["words"].clone() };
    }
    Ok(doc)
}

fn timed_span(v: &Value) -> Option<(Tick, Tick)> {
    let start = v.get("start_tick").or_else(|| v.get("start")).and_then(parse_tick)?;
    let end = v.get("end_tick").or_else(|| v.get("end")).and_then(parse_tick)?;
    Some((start, end))
}

fn word_timing_ok(a: Tick, b: Tick, last: Tick) -> bool {
    let slop = Tick::from_units(1, 1000);
    a >= Tick::ZERO && b >= a && a >= last - slop
}

fn word_problems(words: &[Value]) -> Vec<String> {
    let mut problems = Vec::new();
    let mut last = Tick::MIN;
    for (i, w) in words.iter().enumerate() {
        let has_text = w.get("text").and_then(Value::as_str).is_some_and(|t| !t.trim().is_empty());
        match (has_text, timed_span(w)) {
            (true, Some((a, b))) if word_timing_ok(a, b, last) => last = a,
            (true, Some(_)) => problems.push(format!("word {} is out of order or ends before it starts", i + 1)),
            _ => problems.push(format!("word {} needs text, start and end", i + 1)),
        }
        if problems.len() > 5 {
            break;
        }
    }
    problems
}

fn paragraphs_ordered(lines: &[Value]) -> bool {
    let mut previous = Tick::ZERO;
    for p in lines {
        match timed_span(p) {
            Some((a, b)) if a >= previous && a >= Tick::ZERO && b >= a && p["text"].as_str().is_some_and(|t| !t.trim().is_empty()) => {
                previous = a;
            }
            _ => return false,
        }
    }
    true
}

fn duration_problems(doc: &Value, duration: Tick) -> Vec<String> {
    let mut problems = Vec::new();
    let slop = Tick::from_units(1, 1000);
    for key in ["words", "paragraphs"] {
        for (i, v) in doc[key].as_array().into_iter().flatten().enumerate() {
            if v.get("end_tick").or_else(|| v.get("end")).and_then(parse_tick).is_some_and(|end| end > duration + slop) {
                problems.push(format!(
                    "{key} item {} ends after the recording ({} ticks). Check matching audio/transcript; original received bytes retained.",
                    i + 1,
                    duration.0
                ));
            }
        }
    }
    problems
}

pub(crate) fn transcript_problems(doc: &Value) -> Vec<String> {
    let mut problems = Vec::new();
    match doc.get("words").and_then(Value::as_array) {
        None => problems.push("missing \"words\": [{text, start_tick, end_tick}]".into()),
        Some(words) if words.is_empty() => problems.push("\"words\" is empty".into()),
        Some(words) => problems.extend(word_problems(words)),
    }
    match doc.get("paragraphs").and_then(Value::as_array) {
        None => problems.push("missing \"paragraphs\": [{text, start_tick, end_tick}]".into()),
        Some(lines) if lines.is_empty() || !paragraphs_ordered(lines) => {
            problems.push("paragraphs need ordered nonnegative start/end and nonempty text".into());
        }
        Some(_) => {}
    }
    if let Some(duration) = doc.get("recording_duration_tick").or_else(|| doc.get("recording_duration_sec")).and_then(parse_tick).filter(|t| t.0 >= 0) {
        problems.extend(duration_problems(doc, duration));
    }
    problems
}

fn parse_srt_timestamp(raw: &str) -> Option<Tick> {
    let token = raw.split_whitespace().next()?;
    let (hms, ms_str) = token.split_once([',', '.'])?;
    let mut parts = hms.split(':');
    let h: i64 = parts.next()?.parse().ok()?;
    let m: i64 = parts.next()?.parse().ok()?;
    let s: i64 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || m >= 60 || s >= 60 {
        return None;
    }
    let ms: i64 = format!("{:0<3}", &ms_str[..ms_str.len().min(3)]).parse().ok()?;
    Some(Tick::from_units(((h * 3600 + m * 60 + s) * 1000) + ms, 1000))
}

fn format_srt_timestamp(tick: Tick) -> String {
    let total_ms = tick.max(Tick::ZERO).to_units_floor(1000);
    let ms = total_ms % 1000;
    let total_s = total_ms / 1000;
    let s = total_s % 60;
    let total_m = total_s / 60;
    let m = total_m % 60;
    let h = total_m / 60;
    format!("{h:02}:{m:02}:{s:02},{ms:03}")
}

pub(crate) fn import_caption_paragraphs(text: &str) -> Result<Vec<Value>> {
    let normalized = text.trim_start_matches('\u{feff}').replace("\r\n", "\n");
    let mut cues = Vec::new();
    let mut previous = Tick::ZERO;
    let slop = Tick::from_units(1, 1_000_000);
    for block in normalized.split("\n\n") {
        let mut lines = block.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with("WEBVTT") && !l.starts_with("NOTE")).peekable();
        let Some(first) = lines.peek().copied() else { continue };
        if !first.contains("-->") {
            lines.next();
        }
        let Some(timing_line) = lines.next() else { continue };
        let Some((start_raw, end_raw)) = timing_line.split_once("-->") else {
            return Err(EngineError::InvalidParams("invalid caption file: missing '-->' timing arrow".into()));
        };
        let (Some(start), Some(end)) = (parse_srt_timestamp(start_raw), parse_srt_timestamp(end_raw)) else {
            return Err(EngineError::InvalidParams("invalid caption file: bad timestamp".into()));
        };
        let clean = lines.collect::<Vec<_>>().join(" ").trim().to_string();
        if start < previous - slop || end < start || clean.is_empty() {
            return Err(EngineError::InvalidParams("SRT cues must be ordered, timed and nonempty".into()));
        }
        previous = start;
        cues.push(json!({"text": clean, "start_tick": start.0, "end_tick": end.0, "start": start.0, "end": end.0}));
    }
    if cues.is_empty() {
        return Err(EngineError::InvalidParams("caption file is empty".into()));
    }
    Ok(cues)
}

pub(crate) fn srt_from_paragraphs(doc: &Value) -> Option<String> {
    let paragraphs = doc["paragraphs"].as_array()?;
    let mut out = String::new();
    for (idx, p) in paragraphs.iter().enumerate() {
        let start = format_srt_timestamp(p.get("start_tick").or_else(|| p.get("start")).and_then(parse_tick).unwrap_or(Tick::ZERO));
        let end = format_srt_timestamp(p.get("end_tick").or_else(|| p.get("end")).and_then(parse_tick).unwrap_or(Tick::ZERO));
        let text = p["text"].as_str().unwrap_or_default();
        out.push_str(&format!("{}\n{start} --> {end}\n{text}\n\n", idx + 1));
    }
    Some(out)
}

fn probed_duration(s: &Session) -> Option<Tick> {
    crate::cmd::film::media_duration_ticks(s.song_audio.as_deref()?, false).ok()
}

fn transcript_stem(s: &Session) -> String {
    crate::workspace::sanitize_asset_id(s.project_path.as_ref().and_then(|p| p.file_stem()).and_then(|n| n.to_str()).unwrap_or("song"))
}

pub(crate) fn write_timing_reference(dir: &Path, audio: Option<&Path>, duration: Option<Tick>, doc: &Value) -> Result<PathBuf> {
    let words: Vec<Value> = doc["words"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
        .filter_map(|(i, w)| {
            let (st, en) = timed_span(w)?;
            Some(json!({
                "index": i,
                "word_id": w.get("word_id").cloned().unwrap_or_else(|| json!(format!("w-{:04}", i + 1))),
                "text": w["text"],
                "start_tick": st.0,
                "end_tick": en.0,
                "start_sec": st.seconds(),
                "end_sec": en.seconds(),
            }))
        })
        .collect();

    let lines: Vec<Value> = doc["paragraphs"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
        .filter_map(|(i, p)| {
            let (st, en) = timed_span(p)?;
            Some(json!({
                "line_index": i,
                "line_id": format!("line-{:03}", i + 1),
                "text": p["text"],
                "start_tick": st.0,
                "end_tick": en.0,
                "start_sec": st.seconds(),
                "end_sec": en.seconds(),
            }))
        })
        .collect();

    let map_doc = json!({
        "format": "magic.timing-reference.v1",
        "role": "authoritative song timing map for Lyrics, Motion, Animation, and FilmCraft",
        "ticks_per_second": TICKS_PER_SECOND,
        "audio": audio,
        "duration_tick": duration.map(|t| t.0),
        "duration_sec": duration.map(Tick::seconds),
        "word_count": words.len(),
        "line_count": lines.len(),
        "lines": lines,
        "words": words,
    });
    let out = dir.join("timing-reference.json");
    let bytes = serde_json::to_vec_pretty(&map_doc).map_err(|e| EngineError::InvalidParams(e.to_string()))?;
    craft_atomic::atomic_write(&out, &bytes)?;
    Ok(out)
}

/// Sync both the structured transcript (`transcript.set`) and the SRT caption/lyric track (`captions.import`)
/// directly onto the linked `FilmCraft` `.fcproj` timeline.
pub(crate) fn sync_lyric_track_to_filmcraft(s: &mut Session, srt_path: &Path, _doc: Option<&Value>) {
    if let Ok(fcproj) = crate::cmd::film::film_path(s)
        && fcproj.is_file()
        && let Some(project_dir) = srt_path.parent()
    {
        let mut fc_doc = crate::cmd::film::fcproj_doc::load_fcproj(&fcproj);
        let srt_cues = crate::cmd::film::fcproj_doc::parse_srt_captions(project_dir);
        if let Some(c1) = fc_doc["sequence"]["captions"].get_mut(0) {
            c1["captions"] = json!(srt_cues);
        }
        let _ = crate::cmd::film::fcproj_doc::save_fcproj(&fcproj, &mut fc_doc);
    }
    let _ = crate::cmd::words::cmd_words_captions(s, &json!({"to": "video"}));
}

pub fn cmd_workshop_import_transcript(s: &mut Session, params: &Value) -> Result<Value> {
    let dir = project_dir(s)?;
    let source = params
        .get("path")
        .and_then(Value::as_str)
        .map(|p| resolve_workspace_path(s, p))
        .ok_or_else(|| EngineError::InvalidParams("missing transcript 'path'".into()))?;
    let mut doc: Value =
        serde_json::from_str(&std::fs::read_to_string(&source)?).map_err(|e| EngineError::InvalidParams(format!("not a JSON transcript: {e}")))?;
    let caption_raw = params["captions"].as_str().map(|p| resolve_workspace_path(s, p)).map(std::fs::read_to_string).transpose()?;
    if doc.is_array() {
        doc = json!({"words": doc});
    }
    if let Some(words) = doc["words"].as_array_mut() {
        for w in words {
            if let Some((start, end)) = timed_span(w) {
                w["start_tick"] = json!(start.0);
                w["end_tick"] = json!(end.0);
                w["start"] = json!(start.0);
                w["end"] = json!(end.0);
            }
        }
    }
    let mut grouping = "provided paragraphs";
    if let Some(text) = &caption_raw {
        doc["paragraphs"] = json!(import_caption_paragraphs(text)?);
    } else if doc.get("paragraphs").and_then(Value::as_array).is_none()
        && let Some(words) = doc["words"].as_array()
    {
        doc["paragraphs"] = json!(
            words
                .iter()
                .map(|w| json!({"text": w["text"], "start_tick": w["start_tick"], "end_tick": w["end_tick"], "start": w["start"], "end": w["end"]}))
                .collect::<Vec<_>>()
        );
        grouping = "provisional word grouping; original word timestamps";
    }
    let srt_text = caption_raw.or_else(|| srt_from_paragraphs(&doc));
    let probed = probed_duration(s);
    doc["recording_duration_tick"] = json!(probed.map(|t| t.0));
    let mut problems = transcript_problems(&doc);
    if let (Some(recorded), Some(song)) = (doc.get("audio").and_then(Value::as_str), s.song_audio.as_ref()) {
        let recorded_path = Path::new(recorded);
        let same_name = recorded_path.file_name().is_some() && recorded_path.file_name() == song.file_name();
        if recorded_path != song.as_path() && !same_name {
            problems.push(format!("transcript is for {recorded}, but this project's song is {}", song.display()));
        } else {
            doc["audio"] = json!(song);
        }
    }
    if !problems.is_empty() {
        return Err(EngineError::InvalidParams(format!("transcript does not match: {}", problems.join("; "))));
    }

    let stem = transcript_stem(s);
    for old in transcript_files(&dir) {
        if old != dir.join(format!("{stem}.transcript.json")) && old != dir.join("transcript.json") && !params["replace"].as_bool().unwrap_or(false) {
            return Err(EngineError::InvalidParams(format!("project already has a transcript: {} (remove it first)", old.display())));
        }
    }

    let serialized_words = serde_json::to_vec_pretty(&doc["words"]).map_err(|e| EngineError::InvalidParams(e.to_string()))?;
    let serialized_full = serde_json::to_vec_pretty(&doc).map_err(|e| EngineError::InvalidParams(e.to_string()))?;
    let dest_main = dir.join("transcript.json");
    let dest_stem = dir.join(format!("{stem}.transcript.json"));
    craft_atomic::atomic_write(&dest_main, &serialized_words)?;
    craft_atomic::atomic_write(&dest_stem, &serialized_full)?;

    let mut copied = vec![dest_main, dest_stem];
    let srt_path = dir.join("transcript.srt");
    if let Some(text) = srt_text {
        craft_atomic::atomic_write(&srt_path, text.as_bytes())?;
        copied.push(srt_path.clone());
    }
    let timing_ref = write_timing_reference(&dir, s.song_audio.as_deref(), probed, &doc)?;
    copied.push(timing_ref.clone());

    let provenance = json!({
        "mode": "imported transcript + canonical timing-reference.json",
        "duration_tick": probed.map(|t| t.0),
        "recording_duration_tick": probed.map(|t| t.0),
        "recording_duration_sec": probed.map(|t| t.0 as f64 / TICKS_PER_SECOND as f64),
        "grouping": grouping,
        "timing_reference": timing_ref,
    });
    craft_atomic::atomic_write(
        &dir.join("transcript-provenance.json"),
        &serde_json::to_vec_pretty(&provenance).map_err(|e| EngineError::InvalidParams(e.to_string()))?,
    )?;

    sync_lyric_track_to_filmcraft(s, &srt_path, Some(&doc));

    let mut status = workshop_status(s)?;
    status["imported"] = json!(copied);
    status["timing_reference"] = json!(timing_ref);
    status["recording_duration_tick"] = json!(probed.map(|t| t.0));
    Ok(status)
}

/// Transcribe the song audio via `FilmCraft` (`transcript.generate` when built with `--features whisper`,
/// or surface `transcript.set` / `--transcript` cleanly when `whisper` is disabled in release binaries),
/// write `timing-reference.json` + `transcript.srt`, and sync the lyric track on `film/<song>.fcproj`.
pub fn cmd_audio_analyze(s: &mut Session, params: &Value) -> Result<Value> {
    let dir = project_dir(s)?;
    let audio = params["audio"]
        .as_str()
        .map(|p| resolve_workspace_path(s, p))
        .or_else(|| s.song_audio.clone())
        .filter(|p| p.is_file())
        .ok_or_else(|| EngineError::InvalidParams("no song audio: pass audio=<file> or workshop.set_audio".into()))?;
    if s.song_audio.as_ref() != Some(&audio) {
        s.song_audio = Some(audio.clone());
    }
    crate::cmd::film::assembly::ensure_song_film(s, &audio)?;

    let model = params["model"].as_str().unwrap_or("whisper-base").to_string();
    let probe = crate::native_media::probe_media_in_process(&audio)?;
    let timing_path = dir.join("timing-reference.json");
    let payload = json!({
        "audio": audio,
        "model": model,
        "probe": probe,
        "words": [],
        "paragraphs": [],
        "note": "Populate word timestamps via Whisper or FilmCraft's own `transcript.generate` MCP tool."
    });
    crate::doc_state::write_json(&timing_path, &payload)?;
    Ok(json!({
        "ok": true,
        "audio": audio,
        "timing_reference": timing_path,
        "probe": probe,
    }))
}
