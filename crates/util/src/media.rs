<<<<<<< HEAD
//! Fast zero-subprocess document, `.ecproj`, `.pcraft` (both ZIP and unpacked directory bundles),
//! `.vectorcraft` (v1–v4 + `VCBLOBS`), image, container, and audio inspection built on
//! `storytold/craft-libs` (`craft-doc`, `craft-ecproj-format`, `craft-pcraft-format`,
//! `craft-vectorcraft-format`, `craft-audio`, `craft-media`, `craft-atomic`).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use craft_doc::{Limits, RenditionKind};
use craft_media::isobmff::{TrackKind as IsoTrackKind, open as open_isobmff};
use craft_pcraft_format::{
    COMPOSITE, FORMAT_VERSION as PCRAFT_VERSION, FULL_COMPOSITE, Limits as PcraftLimits, MANIFEST, SAVE_LOCK, Step as PcraftStep, THUMB, read_manifest,
};
use craft_vectorcraft_format::{Contents as VectorContents, is_compressed as vc_is_compressed, sniff as vc_sniff, split_tail};
use serde::de::IgnoredAny;
=======
//! Craft document outline/preview inspection (`craft-doc`), font-catalog discovery
//! (`craft-font-catalog`), media container demux probing (`craft-media`: `isobmff`, `matroska`,
//! `ogg`), and lossless audio decode/encode/peaks (`craft-audio`).
//!
//! Rendering, editing, and lossy broadcast codecs stay in the host Craft applications
//! (`DesignCraft`, `PhotoCraft`, `VectorCraft`, `EffectCraft`, `FilmCraft`), accessed via their
//! `--control` JSON-lines API and `<app>-cli` binaries.

use std::path::Path;

use craft_audio::{BitDepth, DecodeOptions, Decoder, EncodeFormat, EncodeOptions, ReadSeekSource};
use craft_doc::{Kind, Limits, RenditionKind};
use craft_font_catalog::Catalog;
use craft_media::isobmff::{TrackKind as IsoTrackKind, open as open_isobmff};
use craft_media::matroska::{TrackKind as MkvTrackKind, open as open_matroska};
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
use serde_json::{Value, json};

use crate::time::Tick;
use crate::{Result, UtilError};

<<<<<<< HEAD
const PCRAFT_STEPS: &[PcraftStep] = &[];

/// True when `path` is an existing Craft file or an unpacked PhotoCraft `.pcraft` directory bundle
/// containing `manifest.json` (`craft-pcraft-format`).
pub fn is_craft_doc_path(path: &Path) -> bool {
    if path.is_file() {
        return true;
    }
    path.is_dir() && path.extension().is_some_and(|e| e.eq_ignore_ascii_case("pcraft")) && path.join(MANIFEST).is_file()
}

/// Metadata read from an `.ecproj` header, tolerating `ReadError::TooNew` so files saved by a
/// newer ArtCraft Launcher `effectcraft` release continue to work via `effectcraft-cli`.
=======
const PCRAFT_MANIFEST: &str = "manifest.json";
const PCRAFT_SAVE_LOCK: &str = ".save.lock";
const PCRAFT_PREVIEWS: &[&str] = &["full-composite.png", "composite.png", "thumb.png"];

/// True when `path` has one of Magic Studio's Craft project extensions
/// (`DesignCraft`, `PhotoCraft`, `VectorCraft`, `EffectCraft`, `FilmCraft`) or `.magic`.
pub fn has_craft_doc_extension(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|ext| {
        matches!(
            ext.to_ascii_lowercase().as_str(),
            "magic"
                | "fcproj"
                | "fcgt"
                | "ecproj"
                | "ectemplate"
                | "pcraft"
                | "psd"
                | "psb"
                | "ora"
                | "xcf"
                | "kra"
                | "pdn"
                | "vectorcraft"
                | "vcraft"
                | "drawcraft"
                | "designcraft"
                | "idml"
                | "indd"
                | "indt"
        )
    })
}

/// True when `path` is an unpacked PhotoCraft `.pcraft` directory bundle containing `manifest.json`.
pub fn is_unpacked_pcraft_dir(path: &Path) -> bool {
    path.is_dir() && path.extension().is_some_and(|e| e.eq_ignore_ascii_case("pcraft")) && path.join(PCRAFT_MANIFEST).is_file()
}

/// True when `path` exists as a regular file on disk OR an unpacked `.pcraft` directory bundle.
pub fn is_file_or_pcraft_bundle(path: &Path) -> bool {
    path.is_file() || is_unpacked_pcraft_dir(path)
}

/// True when `path` is an existing Craft document file or an unpacked `.pcraft` directory bundle.
pub fn is_craft_doc_path(path: &Path) -> bool {
    (path.is_file() && has_craft_doc_extension(path)) || is_unpacked_pcraft_dir(path)
}

/// Metadata read from an `.ecproj` header via `craft_doc::info`, tolerating newer schema versions
/// saved by a host `EffectCraft` installation.
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
#[derive(Clone, Debug)]
pub struct EcprojHeaderInfo {
    pub schema: Option<u32>,
    pub saved_by: Option<String>,
    pub newer_than_lib: bool,
}

<<<<<<< HEAD
pub type EcprojHeadSummary = EcprojHeaderInfo;

pub fn inspect_ecproj_head(path: &Path) -> Result<EcprojHeaderInfo> {
    let file = std::fs::File::open(path).map_err(|e| UtilError::Invalid(format!("{}: {e}", path.display())))?;
    match craft_ecproj_format::Head::read_from(std::io::BufReader::new(file)) {
        Ok(head) => {
            Ok(EcprojHeaderInfo { schema: Some(head.schema), saved_by: head.saved_by, newer_than_lib: craft_ecproj_format::check_schema(head.schema).is_err() })
        }
        Err(craft_ecproj_format::ReadError::TooNew { saved_by, .. }) => Ok(EcprojHeaderInfo { schema: None, saved_by, newer_than_lib: true }),
        Err(other) => Err(UtilError::Invalid(format!("{}: {other}", path.display()))),
    }
}

/// Inspect an unpacked directory `.pcraft` bundle via `craft-pcraft-format`, honoring `save.lock`.
fn inspect_pcraft_dir_bundle(dir: &Path) -> Option<Value> {
    if dir.join(SAVE_LOCK).exists() {
=======
pub fn inspect_ecproj_head(path: &Path) -> Result<EcprojHeaderInfo> {
    let bytes = std::fs::read(path).map_err(|e| UtilError::Invalid(format!("{}: {e}", path.display())))?;
    match craft_doc::info(&bytes, &Limits::default()) {
        Ok(info) if info.kind == Kind::EffectCraft => {
            Ok(EcprojHeaderInfo { schema: Some(info.version), saved_by: info.generator, newer_than_lib: !info.supported })
        }
        Ok(other) => Err(UtilError::Invalid(format!("{}: expected EffectCraft .ecproj, found {}", path.display(), other.kind.extension()))),
        Err(err) => {
            let parsed: Value = serde_json::from_slice(&bytes).map_err(|_| UtilError::Invalid(format!("{}: {err}", path.display())))?;
            Ok(EcprojHeaderInfo {
                schema: parsed["schema"].as_u64().and_then(|v| u32::try_from(v).ok()),
                saved_by: parsed["savedBy"].as_str().map(str::to_owned),
                newer_than_lib: true,
            })
        }
    }
}

fn inspect_pcraft_dir_bundle(dir: &Path) -> Option<Value> {
    if dir.join(PCRAFT_SAVE_LOCK).exists() {
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
        return Some(json!({
            "kind": "pcraft",
            "bundle": "directory",
            "locked": true,
            "supported": true,
        }));
    }
<<<<<<< HEAD
    // TODO(MAGIC-REVIEW): KEEP byte-based format parsing for explicit offline metadata; live edits belong to the owning app.
    let manifest_bytes = std::fs::read(dir.join(MANIFEST)).ok()?;
    match read_manifest::<IgnoredAny, craft_pcraft_format::Error>(&manifest_bytes, &PcraftLimits::default(), PCRAFT_STEPS, PCRAFT_VERSION) {
        Ok(m) => Some(json!({
            "kind": "pcraft",
            "bundle": "directory",
            "version": m.format_version,
            "generator": m.generator,
            "has_thumbnail": m.thumbnail.is_some(),
            "has_composite": m.composite.is_some() || m.full_composite.is_some(),
            "supported": true,
        })),
        Err(craft_pcraft_format::Error::TooNew { found, generator, .. }) => Some(json!({
            "kind": "pcraft",
            "bundle": "directory",
            "version": found,
            "generator": generator,
            "supported": false,
        })),
        Err(_) => None,
    }
}

/// Inspect plain `.vectorcraft` / `drawcraft` bytes via `craft-vectorcraft-format` (`Contents::read_with_pdf_preference`).
fn inspect_vectorcraft_plain(bytes: &[u8]) -> Option<Value> {
    if vc_is_compressed(bytes) || !vc_sniff(bytes) {
        return None;
    }
    let (json_part, _after) = split_tail(bytes);
    let (contents, pdf_compatible) = VectorContents::<IgnoredAny>::read_with_pdf_preference(json_part).ok()?;
    Some(json!({
        "kind": "vectorcraft",
        "format": contents.format,
        "version": contents.version,
        "pdf_compatible": pdf_compatible,
        "embedded_images": contents.images.len(),
        "icc_profiles": contents.profiles.len(),
=======
    let manifest: Value = serde_json::from_slice(&std::fs::read(dir.join(PCRAFT_MANIFEST)).ok()?).ok()?;
    Some(json!({
        "kind": "pcraft",
        "bundle": "directory",
        "version": manifest["format_version"].as_u64().unwrap_or(1),
        "generator": manifest["generator"],
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
        "supported": true,
    }))
}

<<<<<<< HEAD
/// Inspect any Craft document envelope (`.ecproj`, `.pcraft` ZIP or directory bundle, `.vectorcraft`,
/// or `craft-doc` outline) in-process without launching a desktop or CLI subprocess.
pub fn inspect_craft_envelope(path: &Path) -> Option<Value> {
    if path.is_dir() && path.extension().is_some_and(|e| e.eq_ignore_ascii_case("pcraft")) {
        return inspect_pcraft_dir_bundle(path);
    }
    if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("ecproj"))
        && let Ok(head) = inspect_ecproj_head(path)
    {
        return Some(json!({
            "kind": "ecproj",
            "schema": head.schema,
            "saved_by": head.saved_by,
            "newer_than_lib": head.newer_than_lib,
        }));
    }
    let bytes = std::fs::read(path).ok()?;
    if let Some(vc) = inspect_vectorcraft_plain(&bytes) {
        return Some(vc);
    }
=======
pub fn inspect_craft_envelope(path: &Path) -> Option<Value> {
    if is_unpacked_pcraft_dir(path) {
        return inspect_pcraft_dir_bundle(path);
    }
    let bytes = std::fs::read(path).ok()?;
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    let info = craft_doc::info(&bytes, &Limits::default()).ok()?;
    Some(json!({
        "kind": info.kind.extension(),
        "version": info.version,
        "generator": info.generator,
        "supported": info.supported,
    }))
}

<<<<<<< HEAD
/// Build a concise human-readable outline tag (`layers`, `pages`/`artboards`/`comps`, canvas size)
/// directly via `craft-doc` / `craft-pcraft-format` without spawning any subprocess.
pub fn craft_outline_summary(path: &Path) -> Option<String> {
    if path.is_dir() && path.extension().is_some_and(|e| e.eq_ignore_ascii_case("pcraft")) {
=======
pub fn craft_outline_summary(path: &Path) -> Option<String> {
    if is_unpacked_pcraft_dir(path) {
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
        let info = inspect_pcraft_dir_bundle(path)?;
        let ver = info["version"].as_u64().unwrap_or(1);
        return Some(format!("pcraft dir v{ver}"));
    }
    let bytes = std::fs::read(path).ok()?;
    craft_doc::sniff(&bytes)?;
    let outline = craft_doc::outline(&bytes, &Limits::default()).ok()?;
    let layers = outline.layers.len();
    let pages = outline.pages.len();
    if let Some(c) = outline.canvas {
        return Some(format!("{}x{} · {} layers", c.width, c.height, layers));
    }
    if let Some(first_pg) = outline.pages.first() {
        if pages > 1 {
            return Some(format!("{pages} pages/comps · {layers} layers"));
        }
        return Some(format!("{} · {layers} layers", first_pg.name));
    }
    (layers > 0).then(|| format!("{layers} layers"))
}

<<<<<<< HEAD
/// Extract the embedded composite or thumbnail PNG rendition from a `.pcraft` (directory bundle or
/// ZIP), `.vectorcraft`, or `.ecproj` document directly via `craft-pcraft-format` / `craft-doc`
/// and write it atomically via `craft-atomic` without spawning any CLI subprocess.
pub fn extract_craft_preview(path: &Path, out_png: &Path) -> Result<bool> {
    if path.is_dir() && path.extension().is_some_and(|e| e.eq_ignore_ascii_case("pcraft")) {
        if path.join(SAVE_LOCK).exists() {
            return Ok(false);
        }
        let manifest_ok = std::fs::read(path.join(MANIFEST))
            .ok()
            .and_then(|raw| read_manifest::<IgnoredAny, craft_pcraft_format::Error>(&raw, &PcraftLimits::default(), PCRAFT_STEPS, PCRAFT_VERSION).ok());
        let candidates: Vec<&str> = match manifest_ok.as_ref() {
            Some(m) => [m.full_composite.as_deref(), m.composite.as_deref(), m.thumbnail.as_deref()].into_iter().flatten().collect(),
            None => vec![FULL_COMPOSITE, COMPOSITE, THUMB],
        };
        for rel in candidates {
=======
pub fn extract_craft_preview(path: &Path, out_png: &Path) -> Result<bool> {
    if is_unpacked_pcraft_dir(path) {
        if path.join(PCRAFT_SAVE_LOCK).exists() {
            return Ok(false);
        }
        for rel in PCRAFT_PREVIEWS {
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
            let candidate = path.join(rel);
            if candidate.is_file() {
                let png = std::fs::read(&candidate).map_err(|e| UtilError::Invalid(format!("{}: {e}", candidate.display())))?;
                if let Some(parent) = out_png.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| UtilError::Invalid(format!("{}: {e}", parent.display())))?;
                }
                craft_atomic::atomic_write(out_png, &png).map_err(|e| UtilError::Invalid(format!("{}: {e}", out_png.display())))?;
                return Ok(true);
            }
        }
        return Ok(false);
    }

    let bytes = std::fs::read(path).map_err(|e| UtilError::Invalid(format!("{}: {e}", path.display())))?;
    if craft_doc::sniff(&bytes).is_none() {
        return Ok(false);
    }
    let limits = Limits::default();
    let rendition = match craft_doc::rendition(&bytes, RenditionKind::Composite, &limits) {
        Ok(Some(found)) => Some(found),
        _ => craft_doc::rendition(&bytes, RenditionKind::Thumbnail, &limits).ok().flatten(),
    };
    let Some(rendition) = rendition else {
        return Ok(false);
    };
    if let Some(parent) = out_png.parent() {
        std::fs::create_dir_all(parent).map_err(|e| UtilError::Invalid(format!("{}: {e}", parent.display())))?;
    }
    craft_atomic::atomic_write(out_png, &rendition.bytes).map_err(|e| UtilError::Invalid(format!("{}: {e}", out_png.display())))?;
    Ok(true)
}

<<<<<<< HEAD
/// Extract an embedded PDF rendition from a `.vectorcraft` document via `craft-doc` + `craft-atomic`.
pub fn extract_craft_pdf(path: &Path, out_pdf: &Path) -> Result<bool> {
    let bytes = std::fs::read(path).map_err(|e| UtilError::Invalid(format!("{}: {e}", path.display())))?;
    if craft_doc::sniff(&bytes).is_none() {
        return Ok(false);
    }
    let Some(pdf) = craft_doc::rendition(&bytes, RenditionKind::Pdf, &Limits::default()).map_err(|e| UtilError::Invalid(format!("{}: {e}", path.display())))?
    else {
        return Ok(false);
    };
    if let Some(parent) = out_pdf.parent() {
        std::fs::create_dir_all(parent).map_err(|e| UtilError::Invalid(format!("{}: {e}", parent.display())))?;
    }
    craft_atomic::atomic_write(out_pdf, &pdf.bytes).map_err(|e| UtilError::Invalid(format!("{}: {e}", out_pdf.display())))?;
    Ok(true)
}

/// Read `(width, height, has_alpha)` from an image header, `.pcraft` directory bundle, or
/// `craft_doc::outline` without decoding full pixel buffers into RAM.
pub fn fast_dimensions(path: &Path) -> Option<(u32, u32, bool)> {
    if path.is_dir() && path.extension().is_some_and(|e| e.eq_ignore_ascii_case("pcraft")) {
        for rel in [FULL_COMPOSITE, COMPOSITE, THUMB] {
=======
pub fn fast_dimensions(path: &Path) -> Option<(u32, u32, bool)> {
    if is_unpacked_pcraft_dir(path) {
        for rel in PCRAFT_PREVIEWS {
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
            if let Ok((w, h)) = image::image_dimensions(path.join(rel)) {
                return Some((w, h, true));
            }
        }
        return None;
    }
    if let Ok((w, h)) = image::image_dimensions(path) {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase();
        let has_alpha = matches!(ext.as_str(), "png" | "webp" | "gif" | "svg" | "tif" | "tiff");
        return Some((w, h, has_alpha));
    }
    let bytes = std::fs::read(path).ok()?;
    if craft_doc::sniff(&bytes).is_some()
        && let Ok(outline) = craft_doc::outline(&bytes, &Limits::default())
    {
        if let Some(c) = outline.canvas {
            return Some((c.width, c.height, true));
        }
        if let Some(page) = outline.pages.first() {
            return Some((page.width.round() as u32, page.height.round() as u32, false));
        }
    }
    None
}

<<<<<<< HEAD
/// Probe an ISO BMFF (`.mp4`, `.mov`, `.m4v`) container directly in-process via `craft_media::isobmff`
/// without spawning `filmcraft-cli` or `ffprobe`.
pub fn probe_container_in_process(path: &Path) -> Result<Value> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase();
    if !matches!(ext.as_str(), "mp4" | "mov" | "m4v") {
        return Err(UtilError::Invalid(format!("unsupported ISO BMFF extension `{ext}`")));
    }
    let file = std::fs::File::open(path).map_err(|e| UtilError::Invalid(format!("cannot read {}: {e}", path.display())))?;
    let mp4 = open_isobmff(&file).map_err(|e| UtilError::Invalid(format!("cannot demux {}: {e}", path.display())))?;
    let movie_secs = if mp4.timescale > 0 { (mp4.duration as f64) / (mp4.timescale as f64) } else { 0.0 };
    let mut max_secs = movie_secs;
    let mut video_val = Value::Null;
    let mut audio_streams = Vec::new();
    for track in &mp4.tracks {
        let t_secs = if track.timescale > 0 { (track.duration as f64) / (track.timescale as f64) } else { 0.0 };
        if t_secs > max_secs {
            max_secs = t_secs;
        }
        let cp = serde_json::to_value(track.codec_params()).unwrap_or(Value::Null);
        match track.kind {
            IsoTrackKind::Video if video_val.is_null() => {
                let fps = if t_secs > 0.0 && !track.samples.is_empty() { Some((track.samples.len() as f64) / t_secs) } else { None };
                video_val = json!({
                    "codec": cp["codec"],
                    "width": cp["width"],
                    "height": cp["height"],
                    "fps": fps,
                    "samples": track.samples.len(),
                });
            }
            IsoTrackKind::Audio => {
                audio_streams.push(json!({
                    "codec": cp["codec"],
                    "channels": cp["channels"],
                    "sample_rate": cp["sample_rate"],
                }));
            }
            _ => {}
        }
    }
    if max_secs <= 0.0 {
        return Err(UtilError::Invalid(format!("{}: container reports zero duration", path.display())));
    }
    let ticks = Tick::from_seconds_f64(max_secs).0;
    Ok(json!({
        "kind": if video_val.is_null() { "Audio" } else { "Movie" },
        "duration": ticks,
        "video": video_val,
        "audio_streams": audio_streams,
    }))
}

/// Probe an audio file directly in-process via `craft_audio::Decoder` (zero subprocess spawns).
pub fn probe_audio_in_process(path: &Path) -> Result<Value> {
    let ext = path.extension().and_then(|e| e.to_str());
    let file = std::fs::File::open(path).map_err(|e| UtilError::Invalid(format!("cannot read {}: {e}", path.display())))?;
    let dec = craft_audio::Decoder::open(file, ext, &craft_audio::DecodeOptions::default())
        .map_err(|e| UtilError::Invalid(format!("cannot inspect {}: {e}", path.display())))?;
=======
/// Probe any `craft-media` container (`isobmff` MP4/MOV/M4V or `matroska` MKV/WebM).
pub fn probe_container_in_process(path: &Path) -> Result<Value> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase();
    let file = std::fs::File::open(path).map_err(|e| UtilError::Invalid(format!("cannot read {}: {e}", path.display())))?;

    match ext.as_str() {
        "mp4" | "mov" | "m4v" => {
            let mp4 = open_isobmff(&file).map_err(|e| UtilError::Invalid(format!("cannot demux {}: {e}", path.display())))?;
            let movie_secs = if mp4.timescale > 0 { (mp4.duration as f64) / (mp4.timescale as f64) } else { 0.0 };
            let mut max_secs = movie_secs;
            let mut video_val = Value::Null;
            let mut audio_streams = Vec::new();
            for track in &mp4.tracks {
                let t_secs = if track.timescale > 0 { (track.duration as f64) / (track.timescale as f64) } else { 0.0 };
                if t_secs > max_secs {
                    max_secs = t_secs;
                }
                let cp = serde_json::to_value(track.codec_params()).unwrap_or(Value::Null);
                match track.kind {
                    IsoTrackKind::Video if video_val.is_null() => {
                        let fps = if t_secs > 0.0 && !track.samples.is_empty() { Some((track.samples.len() as f64) / t_secs) } else { None };
                        video_val = json!({"codec": cp["codec"], "width": cp["width"], "height": cp["height"], "fps": fps, "samples": track.samples.len()});
                    }
                    IsoTrackKind::Audio => {
                        audio_streams.push(json!({"codec": cp["codec"], "channels": cp["channels"], "sample_rate": cp["sample_rate"]}));
                    }
                    _ => {}
                }
            }
            if max_secs <= 0.0 {
                return Err(UtilError::Invalid(format!("{}: container reports zero duration", path.display())));
            }
            let chapters = mp4.read_chapters(&file).map_err(|e| UtilError::Invalid(format!("read chapters: {e}")))?.len();
            Ok(json!({
                "kind": if video_val.is_null() { "Audio" } else { "Movie" },
                "container": if mp4.is_quicktime { "QuickTime" } else { "MP4" },
                "duration": Tick::from_seconds_f64(max_secs).0,
                "chapters": chapters,
                "video": video_val,
                "audio_streams": audio_streams,
            }))
        }
        "mkv" | "webm" => {
            let mkv = open_matroska(&file).map_err(|e| UtilError::Invalid(format!("cannot demux {}: {e}", path.display())))?;
            let mut video_val = Value::Null;
            let mut audio_streams = Vec::new();
            let mut max_ns: i64 = 0;
            for track in &mkv.tracks {
                let cp = serde_json::to_value(track.codec_params()).unwrap_or(Value::Null);
                if let Some(last) = track.samples.last() {
                    let end_ns = last.pts.saturating_add(last.duration as i64);
                    if end_ns > max_ns {
                        max_ns = end_ns;
                    }
                }
                match track.kind {
                    MkvTrackKind::Video if video_val.is_null() => {
                        video_val = json!({"codec": cp["codec"], "width": cp["width"], "height": cp["height"], "samples": track.samples.len()});
                    }
                    MkvTrackKind::Audio => {
                        audio_streams.push(json!({"codec": cp["codec"], "channels": cp["channels"], "sample_rate": cp["sample_rate"]}));
                    }
                    _ => {}
                }
            }
            let dur_ticks = probe_audio_in_process(path)
                .ok()
                .and_then(|v| v["duration"].as_i64())
                .unwrap_or_else(|| Tick::from_seconds_f64((max_ns.max(1_000_000) as f64) / 1e9).0);
            Ok(json!({
                "kind": if video_val.is_null() { "Audio" } else { "Movie" },
                "container": if ext == "webm" { "WebM" } else { "Matroska" },
                "duration": dur_ticks,
                "video": video_val,
                "audio_streams": audio_streams,
            }))
        }
        _ => Err(UtilError::Invalid(format!("container `{ext}` is handled by FilmCraft on the host machine"))),
    }
}

/// Probe an audio file directly via `craft_audio::Decoder` + `ReadSeekSource`.
pub fn probe_audio_in_process(path: &Path) -> Result<Value> {
    let ext = path.extension().and_then(|e| e.to_str());
    let file = std::fs::File::open(path).map_err(|e| UtilError::Invalid(format!("cannot read {}: {e}", path.display())))?;
    let src = ReadSeekSource::new(file).map_err(|e| UtilError::Invalid(format!("cannot size {}: {e}", path.display())))?;
    let dec = Decoder::open(src, ext, &DecodeOptions::default()).map_err(|e| UtilError::Invalid(format!("cannot inspect {}: {e}", path.display())))?;
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    let info = if dec.length_known() {
        dec.info().clone()
    } else {
        let f2 = std::fs::File::open(path).map_err(|e| UtilError::Invalid(format!("cannot read {}: {e}", path.display())))?;
<<<<<<< HEAD
        craft_audio::Decoder::open(f2, ext, &craft_audio::DecodeOptions::default())
            .and_then(craft_audio::Decoder::probe)
=======
        let src2 = ReadSeekSource::new(f2).map_err(|e| UtilError::Invalid(format!("cannot size {}: {e}", path.display())))?;
        Decoder::open(src2, ext, &DecodeOptions::default())
            .and_then(Decoder::probe)
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
            .map_err(|e| UtilError::Invalid(format!("cannot inspect {}: {e}", path.display())))?
    };
    let ticks = Tick::from_seconds_f64(info.duration_secs()).0;
    Ok(json!({
        "kind": "Audio",
<<<<<<< HEAD
        "duration": ticks,
        "audio_streams": [{
            "channels": info.channels,
=======
        "container": info.container.name(),
        "codec": info.codec.name(),
        "duration": ticks,
        "damaged": info.damaged,
        "audio_streams": [{
            "codec": info.codec.name(),
            "channels": info.layout.channels(),
            "layout": info.layout.to_string(),
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
            "sample_rate": info.sample_rate,
        }],
    }))
}

<<<<<<< HEAD
/// True when `path` has a pure audio extension that `craft-audio` can probe directly in-process.
pub fn is_pure_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "wav" | "flac" | "mp3" | "ogg" | "aac" | "m4a" | "aiff"))
}

/// True when `path` has an ISO BMFF video container extension (`mp4`, `mov`, `m4v`).
=======
/// Encode a lossless audio master (`wav`, `aiff`, `flac`) via `craft-audio` (`lossless-encode`).
/// Compressed broadcast formats (`mp3`, `aac`, `mp4`, `mov`) are exported by `FilmCraft` on the host.
pub fn encode_audio_master_in_process(src_audio: &Path, dst_path: &Path, requested_format: &str) -> Result<Value> {
    let raw = std::fs::read(src_audio).map_err(|e| UtilError::Invalid(format!("cannot read {}: {e}", src_audio.display())))?;
    let src_ext = src_audio.extension().and_then(|e| e.to_str());
    let (info, buf) = craft_audio::decode(&raw, src_ext).map_err(|e| UtilError::Invalid(format!("cannot decode {}: {e}", src_audio.display())))?;
    let fmt_lower = requested_format.trim().to_ascii_lowercase();
    let channels = buf.channels.len() as u16;
    if channels == 0 || buf.sample_rate == 0 {
        return Err(UtilError::Invalid("decoded audio buffer is empty".into()));
    }

    let encode_fmt = match fmt_lower.as_str() {
        "wav" => EncodeFormat::Wav,
        "aiff" | "aif" => EncodeFormat::Aiff,
        "flac" => EncodeFormat::Flac,
        other => craft_audio::encode_format_for(other).ok_or_else(|| {
            UtilError::Invalid(format!("format `{other}` is exported by host FilmCraft (`filmcraft-cli`); craft-audio writes wav, aiff, and flac"))
        })?,
    };
    let encoded_bytes = craft_audio::encode(&buf, &EncodeOptions::new(encode_fmt).with_bit_depth(BitDepth::Int24).with_dither(true))
        .map_err(|e| UtilError::Invalid(format!("{fmt_lower} encode failed: {e}")))?;

    if let Some(parent) = dst_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| UtilError::Invalid(format!("{}: {e}", parent.display())))?;
    }
    craft_atomic::atomic_write(dst_path, &encoded_bytes).map_err(|e| UtilError::Invalid(format!("{}: {e}", dst_path.display())))?;
    Ok(json!({
        "ok": true,
        "path": dst_path,
        "bytes": encoded_bytes.len(),
        "sample_rate": info.sample_rate,
        "channels": channels,
        "format": fmt_lower,
        "engine": "craft-audio",
    }))
}

pub fn decode_audio_extensions() -> Vec<String> {
    craft_audio::DECODE_EXTENSIONS.iter().map(|s| (*s).to_string()).collect()
}

pub fn encode_audio_extensions() -> Vec<String> {
    craft_audio::ENCODE_EXTENSIONS.iter().map(|s| (*s).to_string()).collect()
}

pub fn is_pure_audio_file(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };
    if matches!(ext.to_ascii_lowercase().as_str(), "mp4" | "mov" | "m4v" | "mkv" | "webm") {
        return false;
    }
    craft_audio::DECODE_EXTENSIONS.iter().any(|allowed| allowed.eq_ignore_ascii_case(ext))
}

>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
pub fn is_isobmff_file(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "mp4" | "mov" | "m4v"))
}

<<<<<<< HEAD
/// Sweep orphaned `.{stem}.{pid}-{n}.craft_tmp` staging files left behind by abnormal process exits
/// (`craft-atomic` recovery contract) inside `root` and its immediate workspace subdirectories.
=======
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
pub fn sweep_orphan_craft_tmp(root: &Path) -> usize {
    let mut dirs = vec![root.to_path_buf()];
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let sub = entry.path();
            if sub.is_dir() {
                dirs.push(sub);
            }
        }
    }
    let mut removed = 0;
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let candidate = entry.path();
            let is_orphan = candidate.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with('.') && n.ends_with(".craft_tmp"));
            if is_orphan && candidate.is_file() && std::fs::remove_file(&candidate).is_ok() {
                removed += 1;
            }
        }
    }
    removed
}

<<<<<<< HEAD
fn candidate_font_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(store) = std::env::var("CRAFT_FONT_STORE")
        && !store.trim().is_empty()
    {
        dirs.push(PathBuf::from(store));
    }
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        let data_home = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".local/share"));
        dirs.push(data_home.join("craft-fonts"));
        dirs.push(data_home.join("fonts"));
        dirs.push(home.join("Library/Application Support/Craft Fonts"));
        dirs.push(home.join("Library/Fonts"));
    }
    if let Some(local) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) {
        dirs.push(local.join("Craft Fonts"));
        dirs.push(local.join("Microsoft/Windows/Fonts"));
    }
    dirs.push(PathBuf::from("/usr/local/share/fonts"));
    dirs.push(PathBuf::from("/usr/share/fonts"));
    dirs.push(PathBuf::from("/Library/Fonts"));
    dirs
}

fn collect_font_stems(dir: &Path, depth: u8, out: &mut BTreeSet<String>) {
    if depth == 0 || out.len() >= 256 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if out.len() >= 256 {
            break;
        }
        let path = entry.path();
        if path.is_dir() {
            collect_font_stems(&path, depth - 1, out);
            continue;
        }
        let is_font = path.extension().and_then(|e| e.to_str()).is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "ttf" | "otf" | "ttc" | "otc"));
        if !is_font {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            let family = stem.split('-').next().unwrap_or(stem).trim();
            if !family.is_empty() {
                out.insert(family.to_owned());
            }
        }
    }
}

/// Enumerate installed Craft Font Store and system font family stems (capped for fast JSON payloads).
pub fn discover_installed_font_families() -> Vec<String> {
    let mut set = BTreeSet::new();
    for dir in candidate_font_dirs() {
        collect_font_stems(&dir, 3, &mut set);
    }
    set.into_iter().collect()
=======
pub fn discover_installed_font_families() -> Vec<String> {
    Catalog::global().families().iter().map(|s| s.to_string()).collect()
}

pub fn font_family_styles(family: &str) -> Vec<String> {
    Catalog::global().styles(family).into_iter().map(|face| face.style.clone()).collect()
}

pub fn is_font_family_installed(family: &str) -> bool {
    Catalog::global().family(family).is_some()
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
}
