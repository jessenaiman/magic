//! Fast zero-subprocess document, `.ecproj`, `.pcraft`, `.vectorcraft`, image, and audio
//! inspection built on `storytold/craft-libs` (`craft-doc`, `craft-ecproj-format`,
//! `craft-pcraft-format`, `craft-vectorcraft-format`, `craft-audio`, `craft-media`) and header-only
//! `image::image_dimensions`.

use std::path::Path;

use serde_json::{Value, json};

use crate::time::Tick;
use crate::{Result, UtilError};

/// Metadata read from an `.ecproj` header, tolerating `ReadError::TooNew` so files saved by a
/// newer ArtCraft Launcher `effectcraft` release continue to work via `effectcraft-cli`.
#[derive(Clone, Debug)]
pub struct EcprojHeaderInfo {
    pub schema: Option<u32>,
    pub saved_by: Option<String>,
    pub newer_than_lib: bool,
}

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

/// Inspect any Craft document envelope (`.ecproj`, `.pcraft`, `.vectorcraft`, or `craft-doc` outline)
/// in-process without launching a desktop or CLI subprocess.
pub fn inspect_craft_envelope(path: &Path) -> Option<Value> {
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
    let info = craft_doc::info(&bytes, &craft_doc::Limits::default()).ok()?;
    Some(json!({
        "kind": info.kind.extension(),
        "version": info.version,
        "generator": info.generator,
        "supported": info.supported,
    }))
}

/// Read `(width, height, has_alpha)` from an image header or `craft_doc::outline` without
/// decoding full pixel buffers into RAM.
pub fn fast_dimensions(path: &Path) -> Option<(u32, u32, bool)> {
    if let Ok((w, h)) = image::image_dimensions(path) {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase();
        let has_alpha = matches!(ext.as_str(), "png" | "webp" | "gif" | "svg" | "tif" | "tiff");
        return Some((w, h, has_alpha));
    }
    let bytes = std::fs::read(path).ok()?;
    if craft_doc::sniff(&bytes).is_some()
        && let Ok(outline) = craft_doc::outline(&bytes, &craft_doc::Limits::default())
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

/// Probe an audio file directly in-process via `craft_audio::Decoder` (zero subprocess spawns).
pub fn probe_audio_in_process(path: &Path) -> Result<Value> {
    let ext = path.extension().and_then(|e| e.to_str());
    let file = std::fs::File::open(path).map_err(|e| UtilError::Invalid(format!("cannot read {}: {e}", path.display())))?;
    let dec = craft_audio::Decoder::open(file, ext, &craft_audio::DecodeOptions::default())
        .map_err(|e| UtilError::Invalid(format!("cannot inspect {}: {e}", path.display())))?;
    let info = if dec.length_known() {
        dec.info().clone()
    } else {
        let f2 = std::fs::File::open(path).map_err(|e| UtilError::Invalid(format!("cannot read {}: {e}", path.display())))?;
        craft_audio::Decoder::open(f2, ext, &craft_audio::DecodeOptions::default())
            .and_then(craft_audio::Decoder::probe)
            .map_err(|e| UtilError::Invalid(format!("cannot inspect {}: {e}", path.display())))?
    };
    let ticks = Tick::from_seconds_f64(info.duration_secs()).0;
    Ok(json!({
        "kind": "Audio",
        "duration": ticks,
        "audio_streams": [{
            "channels": info.channels,
            "sample_rate": info.sample_rate,
        }],
    }))
}

/// True when `path` has a pure audio extension that `craft-audio` can probe directly in-process.
pub fn is_pure_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "wav" | "flac" | "mp3" | "ogg" | "aac" | "m4a" | "aiff"))
}
