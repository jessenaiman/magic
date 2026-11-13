//! Full `craft-timeline` integration (`time`, `rate`, `report`, `path`, `model`, `fx`, `build`,
//! `ale`, `edl`, `fcp7`, `fcpxml`, `otio`, `Format`, `detect_text`, `decode_text`) exposed as
//! first-class Magic Studio engine commands (`timeline.*` & `video.markers` `action="interchange"`)
//! so both `magic-cli` and the `magic` desktop app consume whatever slice they need.
//!
//! Upstream `craft-timeline` contract note: craft-timeline d23f5f3 API differs from the original magic-fixes-7 design:
//!   - `Project` has `new(name)` not `Default`; no `Serialize`/`Deserialize` impls
//!   - `otio::export`/`fcpxml::export`/`fcp7::export` take `(&Project, ItemId, &ExportOptions, &mut Report)`
//!   - `otio::import`/`fcpxml::import`/`fcp7::import` take `(&str, &ImportOptions, &mut Report) -> Result<Imported>`
//!   - `edl::parse` returns `Edl` directly (no `Result`), `edl::write` takes `(&OutHeader, &[OutEvent])`
//!   - `time::format_smpte`/`parse_smpte` do not exist; use `time::parse_timecode` / `time::tick_of`
//!   - `ExportOptions`/`ImportOptions` have no `Default` impl
//!   - These require upstream alignment or adapter wrappers; marked with TODO below.

use std::path::{Path, PathBuf};

use craft_timeline::ale::{self, AleDoc};
use craft_timeline::model::Project as TimelineProject;
use craft_timeline::time::{FrameRate, TICKS_PER_SECOND, Tick};
use craft_timeline::{FcpxmlVersion, Format, decode_text, detect_text, rate};
use serde_json::{Value, json};

use super::{Film, bad, fcproj_doc, film};
use crate::time::{parse_frame_rate, parse_raw_tick, parse_tick};
use crate::{Result, Session};

fn sequence_rate(doc: &Value) -> FrameRate {
    doc["sequence"].get("fps").and_then(parse_frame_rate).unwrap_or(FrameRate::FPS_24).sane()
}

fn parse_format_name(raw: &str) -> Result<Format> {
    let clean = raw.trim().trim_start_matches('.').to_ascii_lowercase();
    match clean.as_str() {
        "edl" | "cmx3600" => Ok(Format::Edl),
        "xml" | "fcp7" | "xmeml" => Ok(Format::Fcp7Xml),
        "fcpxml" | "fcpxmld" => Ok(Format::Fcpxml),
        "otio" => Ok(Format::Otio),
        "aaf" => Ok(Format::Aaf),
        "omf" => Ok(Format::Omf),
        other => Format::from_extension(other)
            .ok_or_else(|| bad(format!("unsupported timeline format `{raw}`; expected edl, xml/fcp7, fcpxml, otio, aaf, omf, or ale"))),
    }
}

#[allow(dead_code)]
fn parse_fcpxml_version(raw: Option<&str>) -> FcpxmlVersion {
    match raw.unwrap_or("1.10").trim() {
        "1.9" => FcpxmlVersion::V1_9,
        "1.11" => FcpxmlVersion::V1_11,
        _ => FcpxmlVersion::V1_10,
    }
}

/// Convert Magic Studio's `.fcproj` sequence (`V1`–`V4`, `A1`, markers, rate) into a neutral
/// [`craft_timeline::model::Project`] so `fcp7`, `fcpxml`, `otio`, and `edl` writers share one
/// exact source of truth.
///
/// Upstream `craft-timeline` contract note: `craft_timeline::model::Project` in d23f5f3 has no `Serialize`/`Deserialize` and
/// no `Default`. The original magic-fixes-7 serde round-trip (`to_value`/`from_value`) cannot
/// compile. This temporary build constructs `Project::new(name)` only; full sequence/marker/rate
/// population requires upstream `craft-timeline` serde impls or an adapter.
pub(crate) fn fcproj_to_timeline_model(fcproj_doc: &Value) -> TimelineProject {
    let name = fcproj_doc["name"].as_str().unwrap_or("Untitled");
    // model::Project gains serde or typed builder APIs. See craft-timeline README/model.rs.
    TimelineProject::new(name)
}

/// Merge an imported [`craft_timeline::model::Project`] back into a Magic `.fcproj` JSON document.
///
/// Upstream `craft-timeline` contract note: `craft_timeline::model::Project` in d23f5f3 has no `Serialize`. The original
/// `serde_json::to_value(&model)` cannot compile. This temporary build returns only the clip count
/// from the fcproj_doc's existing video tracks; full model merge requires upstream serde impls.
#[allow(dead_code)]
pub(crate) fn merge_timeline_model_into_fcproj(model: &TimelineProject, fcproj_doc: &mut Value) -> Value {
    let _ = model; // TODO: use model fields once craft-timeline::model::Project gains serde or accessor APIs
    let mut imported_clips = 0usize;
    if let Some(seqs) = fcproj_doc.get("sequence").and_then(Value::as_object)
        && let Some(v_tracks) = seqs.get("video").and_then(Value::as_array)
    {
        for trk in v_tracks.iter() {
            if let Some(items) = trk.get("items").and_then(Value::as_array) {
                imported_clips += items.len();
            }
        }
    }
    fcproj_doc::recompute_sequence_duration(fcproj_doc);
    json!({
        "imported_clips": imported_clips,
        "model": json!({"note": "craft-timeline Project merge pending upstream serde"}),
    })
}

/// Build an Avid Log Exchange ([`AleDoc`]) from the active `.fcproj` sequence clips and markers.
pub(crate) fn fcproj_to_ale_doc(fcproj_doc: &Value) -> AleDoc {
    let rate = sequence_rate(fcproj_doc);
    let heading = ale::standard_heading(rate, 1080, 48_000);
    let columns = vec![
        "Name".to_owned(),
        "Tracks".to_owned(),
        "Start".to_owned(),
        "End".to_owned(),
        "Duration".to_owned(),
        "Source File".to_owned(),
        "Comments".to_owned(),
    ];
    let mut rows = Vec::new();
    for (kind, arr_key, is_video) in [("V", "video", true), ("A", "audio", false)] {
        for (idx, trk) in fcproj_doc["sequence"][arr_key].as_array().into_iter().flatten().enumerate() {
            let trk_code = trk["code"].as_str().map(str::to_owned).unwrap_or_else(|| format!("{kind}{}", idx + 1));
            for clip in trk["items"].as_array().into_iter().flatten() {
                let st = Tick(clip["start"].as_i64().unwrap_or(0).max(0));
                let en = Tick(clip["end"].as_i64().unwrap_or(st.0).max(st.0));
                let dur = Tick((en.0 - st.0).max(0));
                let name = ale::clean(clip["name"].as_str().unwrap_or(&trk_code));
                let src_file = ale::clean(clip["source_path"].as_str().unwrap_or(""));
                let tracks = if is_video { ale::tracks_text(true, 0) } else { ale::tracks_text(false, 2) };
                rows.push(vec![
                    name,
                    tracks,
                    craft_timeline::time::format_timecode_frames(rate.frame_at(st), rate, false),
                    craft_timeline::time::format_timecode_frames(rate.frame_at(en), rate, false),
                    craft_timeline::time::format_timecode_frames(rate.frame_at(dur), rate, false),
                    src_file,
                    trk_code.clone(),
                ]);
            }
        }
    }
    for m in fcproj_doc["sequence"]["markers"].as_array().into_iter().flatten() {
        let st = parse_raw_tick(&m["start"]).unwrap_or(Tick::ZERO);
        let dur = parse_raw_tick(&m["duration"]).unwrap_or(Tick::ZERO);
        rows.push(vec![
            ale::clean(m["name"].as_str().unwrap_or("Marker")),
            "V".to_owned(),
            craft_timeline::time::format_timecode_frames(rate.frame_at(st), rate, false),
            craft_timeline::time::format_timecode_frames(rate.frame_at(st + dur), rate, false),
            craft_timeline::time::format_timecode_frames(rate.frame_at(dur), rate, false),
            String::new(),
            ale::clean(m["comment"].as_str().unwrap_or("")),
        ]);
    }
    AleDoc { heading, columns, rows }
}

/// Export the active `.fcproj` timeline to text/XML/JSON (`otio`, `fcpxml`, `xml`/`fcp7`, `edl`, `ale`) via `craft-timeline`.
pub(crate) fn export_timeline_document(s: &Session, f: &Film, format_str: &str, out_path: &Path, p: &Value) -> Result<Value> {
    let proj_path = f.ensure_on_disk(s)?.to_path_buf();
    let mut fc_doc = fcproj_doc::load_fcproj(&proj_path);
    fcproj_doc::hydrate_live_stage_tracks(s, &f.dir, &mut fc_doc);

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let fmt_lower = format_str.trim().to_ascii_lowercase();
    if fmt_lower == "ale" {
        let ale_doc = fcproj_to_ale_doc(&fc_doc);
        let text = ale::write(&ale_doc);
        craft_atomic::atomic_write(out_path, text.as_bytes())?;
        return Ok(json!({
            "ok": true,
            "engine": "craft-timeline",
            "format": "ale",
            "path": out_path,
            "rows": ale_doc.rows.len(),
            "fps": ale::fps_text(sequence_rate(&fc_doc)),
            "video_format": ale::video_format(1080),
            "warnings": [],
        }));
    }

    let fmt = parse_format_name(&fmt_lower)?;
    if matches!(fmt, Format::Aaf | Format::Omf) {
        let live = crate::craft_api::execute_in_live_app("filmcraft", "file.exportInterchange", json!({"path": out_path, "format": fmt.extension()})).ok();
        if live.is_some() && out_path.is_file() {
            return Ok(json!({
                "ok": true,
                "engine": "filmcraft-live",
                "format": fmt.extension(),
                "path": out_path,
                "live": live,
            }));
        }
        return Err(bad(format!(
            "{} binary container export requires a connected FilmCraft session (`filmcraft --control 9876`) or text interchange (`otio`, `fcpxml`, `xml`, `edl`, `ale`)",
            fmt.name()
        )));
    }

    // Project::new() creates an empty project with no sequence; seq_id resolution and ExportOptions
    // construction need a Catalog impl. Temporary stub returns unsupported error for text formats.
    let _ = f;
    let _ = p;
    let _ = fmt;
    Err(bad("timeline export requires upstream craft-timeline alignment (d23f5f3 API migrated)"))
}

/// Import an external timeline document (`otio`, `fcpxml`, `xml`/`fcp7`, `edl`, `ale`) into `.fcproj` via `craft-timeline`.
pub(crate) fn import_timeline_document(_s: &Session, _f: &Film, src_path: &Path, _format_hint: Option<&str>, _save_to_fcproj: bool) -> Result<Value> {
    if !src_path.is_file() {
        return Err(bad(format!("timeline interchange file not found: {}", src_path.display())));
    }
    let bytes = std::fs::read(src_path)?;
    let ext = _format_hint.or_else(|| src_path.extension().and_then(|e| e.to_str()));
    if ext.is_some_and(|e| e.eq_ignore_ascii_case("ale")) {
        let text = decode_text(&bytes);
        let doc = ale::parse(&text).map_err(bad)?;
        return Ok(json!({
            "ok": true,
            "engine": "craft-timeline",
            "format": "ale",
            "path": src_path,
            "heading": doc.heading,
            "columns": doc.columns,
            "row_count": doc.rows.len(),
            "rows": doc.rows,
        }));
    }

    let detected = detect_text(&bytes, ext)
        .or_else(|| ext.and_then(|e| parse_format_name(e).ok()))
        .ok_or_else(|| bad(format!("could not detect timeline format for {}", src_path.display())))?;
    // with a Catalog impl. ImportOptions has no Default. Temporary stub returns unsupported error.
    let _ = detected;
    Err(bad("timeline import requires upstream craft-timeline ImportOptions/Catalog alignment (d23f5f3 API migrated)".to_string()))
}

/// `timeline.detect`: Sniff a timeline file or inline text via `craft_timeline::detect_text` + `Format::ALL`.
pub fn cmd_timeline_detect(s: &mut Session, p: &Value) -> Result<Value> {
    let formats: Vec<Value> = Format::ALL.iter().map(|f| json!({"name": f.name(), "extension": f.extension()})).collect();
    if let Some(raw_path) = p["path"].as_str().filter(|v| !v.trim().is_empty()) {
        let resolved = if let Ok(dir) = crate::workshop::project_dir(s) {
            crate::workspace::resolve_path_in(s, Some(&dir), raw_path)
        } else {
            crate::workspace::resolve_workspace_path(s, raw_path)
        };
        let bytes = std::fs::read(&resolved)?;
        let ext = resolved.extension().and_then(|e| e.to_str());
        let detected = detect_text(&bytes, ext);
        return Ok(json!({
            "ok": true,
            "engine": "craft-timeline",
            "path": resolved,
            "detected": detected.map(|d| json!({"name": d.name(), "extension": d.extension()})),
            "supported_formats": formats,
            "max_rate_fps": rate::MAX_RATE,
            "ticks_per_second": TICKS_PER_SECOND,
        }));
    }
    if let Some(inline) = p["text"].as_str() {
        let ext = p["extension"].as_str();
        let detected = detect_text(inline.as_bytes(), ext);
        return Ok(json!({
            "ok": true,
            "engine": "craft-timeline",
            "detected": detected.map(|d| json!({"name": d.name(), "extension": d.extension()})),
            "supported_formats": formats,
            "max_rate_fps": rate::MAX_RATE,
            "ticks_per_second": TICKS_PER_SECOND,
        }));
    }
    Ok(json!({
        "ok": true,
        "engine": "craft-timeline",
        "supported_formats": formats,
        "extra_log_formats": ["ale"],
        "max_rate_fps": rate::MAX_RATE,
        "ticks_per_second": TICKS_PER_SECOND,
    }))
}

/// `timeline.timecode`: Convert between `Tick`s, seconds, frames, and SMPTE timecode (`:` NDF / `;` DF) via `craft_timeline::time`.
pub fn cmd_timeline_timecode(_: &mut Session, p: &Value) -> Result<Value> {
    let fps = p.get("fps").and_then(parse_frame_rate).unwrap_or(FrameRate::FPS_24).sane();
    let tick = if let Some(tc_str) = p["timecode"].as_str().filter(|s| !s.trim().is_empty()) {
        let frame = craft_timeline::time::parse_timecode(tc_str.trim(), fps, false, 0).map_err(|e| bad(format!("invalid SMPTE timecode `{tc_str}`: {e}")))?;
        Tick::from_units(frame, TICKS_PER_SECOND)
    } else if let Some(frame) = p["frame"].as_i64() {
        fps.tick_of(frame)
    } else if let Some(t) = p.get("tick").or_else(|| p.get("ticks")).or_else(|| p.get("seconds")).and_then(parse_tick) {
        t
    } else {
        Tick::ZERO
    };
    let smpte = craft_timeline::time::format_timecode_frames(fps.frame_at(tick), fps, false);
    Ok(json!({
        "ok": true,
        "engine": "craft-timeline",
        "ticks": tick.0,
        "ticks_per_second": TICKS_PER_SECOND,
        "seconds": tick.seconds(),
        "frame": fps.frame_at(tick),
        "fps": {"num": fps.num, "den": fps.den, "value": fps.as_f64()},
        "smpte": smpte,
    }))
}

/// `timeline.inspect`: Inspect an external interchange file (`.otio`, `.fcpxml`, `.xml`, `.edl`, `.ale`) or the active `.fcproj` as a `craft-timeline` model.
pub fn cmd_timeline_inspect(s: &mut Session, p: &Value) -> Result<Value> {
    if let Some(raw_path) = p["path"].as_str().filter(|v| !v.trim().is_empty()) {
        let f = film(s)?;
        let src = crate::workspace::resolve_path_in(s, Some(&f.dir), raw_path);
        return import_timeline_document(s, &f, &src, p["format"].as_str(), false);
    }
    let f = film(s)?;
    let proj_path = f.ensure_on_disk(s)?.to_path_buf();
    let mut fc_doc = fcproj_doc::load_fcproj(&proj_path);
    fcproj_doc::hydrate_live_stage_tracks(s, &f.dir, &mut fc_doc);
    let _model = fcproj_to_timeline_model(&fc_doc);
    let ale_preview = fcproj_to_ale_doc(&fc_doc);
    Ok(json!({
        "ok": true,
        "engine": "craft-timeline",
        "timeline": proj_path,
        "fps": ale::fps_text(sequence_rate(&fc_doc)),
        "ale_event_count": ale_preview.rows.len(),
        "model": json!({"note": "TimelineProject model pending upstream serde"}),
    }))
}

/// `timeline.export`: Export active `.fcproj` to `otio`, `fcpxml`, `xml` (`fcp7`), `edl` (`CMX 3600`), or `ale`.
pub fn cmd_timeline_export(s: &mut Session, p: &Value) -> Result<Value> {
    let f = film(s)?;
    let fmt = p["format"].as_str().or_else(|| p["path"].as_str().and_then(|r| Path::new(r).extension()?.to_str())).unwrap_or("otio");
    let ext = match fmt.to_ascii_lowercase().as_str() {
        "fcp7" | "xmeml" | "xml" => "xml",
        "cmx3600" | "edl" => "edl",
        "fcpxml" => "fcpxml",
        "ale" => "ale",
        "aaf" => "aaf",
        "omf" => "omf",
        _ => "otio",
    };
    let out = p["path"]
        .as_str()
        .filter(|v| !v.trim().is_empty())
        .map(|raw| crate::workspace::resolve_path_in(s, Some(&f.dir), raw))
        .unwrap_or_else(|| f.dir.join("film/interchange").join(format!("{}.{ext}", f.name)));
    if let Some(roots) = &s.automation_roots {
        roots.check_write(&out)?;
    }
    export_timeline_document(s, &f, fmt, &out, p)
}

/// `timeline.import`: Import `otio`, `fcpxml`, `xml` (`fcp7`), `edl` (`CMX 3600`), or `ale` into `film/<song>.fcproj`.
pub fn cmd_timeline_import(s: &mut Session, p: &Value) -> Result<Value> {
    let f = film(s)?;
    let raw = p["path"].as_str().filter(|v| !v.trim().is_empty()).ok_or_else(|| bad("path=<interchange file> is required"))?;
    let src = crate::workspace::resolve_path_in(s, Some(&f.dir), raw);
    if let Some(roots) = &s.automation_roots {
        roots.check_read(&src)?;
    }
    let save = p["save"].as_bool().unwrap_or(true);
    import_timeline_document(s, &f, &src, p["format"].as_str(), save)
}

/// `timeline.ale`: Parse or write an Avid Log Exchange (`.ale`) document directly via `craft_timeline::ale`.
pub fn cmd_timeline_ale(s: &mut Session, p: &Value) -> Result<Value> {
    let action = p["action"].as_str().unwrap_or(if p.get("text").is_some() { "parse" } else { "export" });
    if action == "parse" {
        if let Some(text) = p["text"].as_str() {
            let doc = ale::parse(text).map_err(bad)?;
            return Ok(json!({"ok": true, "engine": "craft-timeline", "heading": doc.heading, "columns": doc.columns, "rows": doc.rows}));
        }
        let mut p2 = p.clone();
        p2["format"] = json!("ale");
        return cmd_timeline_import(s, &p2);
    }
    let mut p2 = p.clone();
    p2["format"] = json!("ale");
    cmd_timeline_export(s, &p2)
}

/// `timeline.edl`: Parse or write a CMX 3600 Edit Decision List (`.edl`) directly via `craft_timeline::edl`.
pub fn cmd_timeline_edl(s: &mut Session, p: &Value) -> Result<Value> {
    let action = p["action"].as_str().unwrap_or(if p.get("text").is_some() { "parse" } else { "export" });
    // `Edl` may not implement Serialize. Temporary stub returns unsupported error.
    if action == "parse" {
        return Err(bad("EDL parse requires upstream craft-timeline Edl/Report alignment (d23f5f3 API migrated)"));
    }
    let mut p2 = p.clone();
    p2["format"] = json!("edl");
    cmd_timeline_export(s, &p2)
}

/// Shared handler for `video.markers` `action = "interchange"`, replacing the old JSON spec stub.
pub(crate) fn run_markers_interchange(s: &mut Session, f: &Film, default_ecproj: &Path, p: &Value) -> Result<Value> {
    let format = p["format"].as_str().unwrap_or("otio");
    let raw_dir = p["direction"].as_str().unwrap_or("film_to_effect");
    let direction = match raw_dir {
        "import" | "effect_to_film" => "effect_to_film",
        _ => "film_to_effect",
    };
    let ext = match format {
        "fcpxml" => "fcpxml",
        "otio" => "otio",
        "edl" => "edl",
        "aaf" => "aaf",
        "omf" => "omf",
        "ale" => "ale",
        _ => "xml",
    };
    let ecproj =
        p["ecproj"].as_str().filter(|v| !v.trim().is_empty()).map(|raw| crate::workspace::resolve_path_in(s, Some(&f.dir), raw)).unwrap_or_else(|| default_ecproj.to_path_buf());
    let interchange_path = p["path"]
        .as_str()
        .filter(|v| !v.trim().is_empty())
        .map(|raw| crate::workspace::resolve_path_in(s, Some(&f.dir), raw))
        .unwrap_or_else(|| f.dir.join("film/interchange").join(format!("{}-interchange.{ext}", f.name)));

    if direction == "effect_to_film" {
        if !interchange_path.is_file() && ecproj.is_file() {
            let _ = crate::craft_api::execute_in_live_app("effectcraft", "file.exportTimeline", json!({"path": interchange_path, "format": format}));
        }
        if interchange_path.is_file() {
            let imported = import_timeline_document(s, f, &interchange_path, Some(format), true)?;
            return Ok(json!({
                "ok": true,
                "engine": "craft-timeline",
                "direction": direction,
                "format": format,
                "ecproj": ecproj,
                "interchange_path": interchange_path,
                "imported": imported,
            }));
        }
    }

    let exported = export_timeline_document(s, f, format, &interchange_path, p)?;
    Ok(json!({
        "ok": true,
        "engine": "craft-timeline",
        "direction": direction,
        "format": format,
        "ecproj": ecproj,
        "interchange_path": interchange_path,
        "exported": exported,
    }))
}
