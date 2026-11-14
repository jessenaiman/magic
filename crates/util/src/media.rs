//! Craft document outline/preview inspection (`craft-doc`), font-catalog discovery
//! (`craft-font-catalog`), media container demux probing (`craft-media`: `isobmff`, `matroska`,
//! `ogg`), and lossless audio decode/encode/peaks (`craft-audio`).
//!
//! Rendering, editing, and lossy broadcast codecs stay in the host Craft applications
//! (`DesignCraft`, `DeckCraft`, `PhotoCraft`, `VectorCraft`, `EffectCraft`, `FilmCraft`), driven via
//! headless `<app>-cli` binaries.

use std::path::Path;

use craft_audio::{BitDepth, DecodeOptions, Decoder, EncodeFormat, EncodeOptions, ReadSeekSource};
use craft_doc::{Kind, Limits, RenditionKind};
use craft_ecproj_format::{Head as EcprojHead, ReadError as EcprojReadError};
use craft_file_formats::Format;
use craft_font_catalog::Catalog;
use craft_media::isobmff::{TrackKind as IsoTrackKind, open as open_isobmff};
use craft_media::matroska::{TrackKind as MkvTrackKind, open as open_matroska};
use craft_pcraft_bundle::{DirectoryBundle, Limits as PcraftBundleLimits};
use craft_pcraft_format::{
    COMPOSITE as PCRAFT_COMPOSITE, FORMAT_VERSION as PCRAFT_FORMAT_VERSION, FULL_COMPOSITE as PCRAFT_FULL_COMPOSITE, Limits as PcraftManifestLimits,
    MANIFEST as PCRAFT_MANIFEST, SAVE_LOCK as PCRAFT_SAVE_LOCK, THUMB as PCRAFT_THUMB, read_manifest as read_pcraft_manifest,
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::time::Tick;
use crate::{Result, UtilError};

const PCRAFT_PREVIEWS: &[&str] = &[PCRAFT_FULL_COMPOSITE, PCRAFT_COMPOSITE, PCRAFT_THUMB];

/// True when `path` has one of Magic Studio's Craft project extensions
/// (`DesignCraft`, `PhotoCraft`, `VectorCraft`, `EffectCraft`, `FilmCraft`, `AniCraft`) or `.magic`.
pub fn has_craft_doc_extension(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };
    if let Some(fmt) = craft_file_formats::from_extension(ext)
        && matches!(
            fmt,
            Format::DesignCraft | Format::Pcraft | Format::Psd | Format::Ora | Format::Kra | Format::Xcf | Format::Pdn | Format::Idml | Format::Indd
        )
    {
        return true;
    }
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "magic"
            | "fcproj"
            | "fcgt"
            | "ecproj"
            | "ectemplate"
            | "anicraft"
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

/// Metadata read from an `.ecproj` header via `craft_doc::info` and `craft_ecproj_format::Head::read`.
#[derive(Clone, Debug)]
pub struct EcprojHeaderInfo {
    pub schema: Option<u32>,
    pub saved_by: Option<String>,
    pub newer_than_lib: bool,
}

pub fn inspect_ecproj_head(path: &Path) -> Result<EcprojHeaderInfo> {
    let bytes = std::fs::read(path).map_err(|e| UtilError::Invalid(format!("{}: {e}", path.display())))?;
    match craft_doc::info(&bytes, &Limits::default()) {
        Ok(info) if info.kind == Kind::EffectCraft => {
            Ok(EcprojHeaderInfo { schema: Some(info.version), saved_by: info.generator, newer_than_lib: !info.supported })
        }
        Ok(other) => Err(UtilError::Invalid(format!("{}: expected EffectCraft .ecproj, found {}", path.display(), other.kind.extension()))),
        Err(doc_err) => {
            let text = std::str::from_utf8(&bytes).map_err(|_| UtilError::Invalid(format!("{}: {doc_err}", path.display())))?;
            match EcprojHead::read(text) {
                Ok(head) => Ok(EcprojHeaderInfo { schema: Some(head.schema), saved_by: head.saved_by, newer_than_lib: false }),
                Err(EcprojReadError::TooNew { too_new, saved_by, .. }) => Ok(EcprojHeaderInfo { schema: Some(too_new.schema), saved_by, newer_than_lib: true }),
                Err(err) => Err(UtilError::Invalid(format!("{}: {err}", path.display()))),
            }
        }
    }
}

fn inspect_pcraft_dir_bundle(dir: &Path) -> Option<Value> {
    if dir.join(PCRAFT_SAVE_LOCK).exists() {
        return Some(json!({
            "kind": "pcraft",
            "bundle": "directory",
            "locked": true,
            "supported": true,
        }));
    }
    let bundle = DirectoryBundle::open(dir, PcraftBundleLimits::default()).ok()?;
    let raw = bundle.manifest().ok()?;
    let manifest: craft_pcraft_format::Manifest<Value> =
        read_pcraft_manifest::<Value, craft_pcraft_format::Error>(&raw, &PcraftManifestLimits::default(), &[], PCRAFT_FORMAT_VERSION).ok()?;
    Some(json!({
        "kind": "pcraft",
        "bundle": "directory",
        "version": manifest.format_version,
        "generator": manifest.generator,
        "supported": manifest.format_version <= PCRAFT_FORMAT_VERSION,
    }))
}

struct AnicraftValueDecoder;
impl craft_anicraft_format::PayloadDecoder<Value> for AnicraftValueDecoder {
    type Document = Value;
    fn decode<'de, D>(&mut self, deserializer: D, _paths: Option<Vec<Value>>) -> std::result::Result<Self::Document, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Value::deserialize(deserializer)
    }
}

pub fn inspect_craft_envelope(path: &Path) -> Option<Value> {
    if is_unpacked_pcraft_dir(path) {
        return inspect_pcraft_dir_bundle(path);
    }
    let bytes = std::fs::read(path).ok()?;
    if let Ok(info) = craft_doc::info(&bytes, &Limits::default()) {
        return Some(json!({
            "kind": info.kind.extension(),
            "version": info.version,
            "generator": info.generator,
            "supported": info.supported,
        }));
    }
    if (path.extension().is_some_and(|e| e.eq_ignore_ascii_case("vectorcraft") || e.eq_ignore_ascii_case("drawcraft"))
        || craft_vectorcraft_format::sniff(&bytes))
        && let Ok(vf) = craft_vectorcraft_format::File::<serde::de::IgnoredAny>::read(&bytes)
    {
        return Some(json!({
            "kind": vf.format,
            "version": vf.version,
            "generator": vf.generator,
            "supported": vf.version <= craft_vectorcraft_format::VERSION,
        }));
    }
    if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("anicraft"))
        && let Ok(text) = std::str::from_utf8(&bytes)
    {
        let mut dec = AnicraftValueDecoder;
        if let Ok(env) = craft_anicraft_format::read(text, &mut dec) {
            return Some(json!({
                "kind": env.format,
                "version": env.version,
                "generator": "AniCraft",
                "supported": env.version <= craft_anicraft_format::FORMAT_VERSION,
            }));
        }
    }
    None
}

pub fn craft_outline_summary(path: &Path) -> Option<String> {
    if is_unpacked_pcraft_dir(path) {
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

pub fn extract_craft_preview(path: &Path, out_png: &Path) -> Result<bool> {
    if is_unpacked_pcraft_dir(path) {
        if path.join(PCRAFT_SAVE_LOCK).exists() {
            return Ok(false);
        }
        let Ok(bundle) = DirectoryBundle::open(path, PcraftBundleLimits::default()) else {
            return Ok(false);
        };
        for rel in PCRAFT_PREVIEWS {
            if let Ok(Some(png)) = bundle.preview(rel) {
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
        if let Ok(vf) = craft_vectorcraft_format::File::<serde::de::IgnoredAny>::read(&bytes)
            && let Some(preview) = vf.preview
            && let Some(decoded) = craft_vectorcraft_format::base64::decode(&preview.data)
        {
            if let Some(parent) = out_png.parent() {
                std::fs::create_dir_all(parent).map_err(|e| UtilError::Invalid(format!("{}: {e}", parent.display())))?;
            }
            craft_atomic::atomic_write(out_png, &decoded).map_err(|e| UtilError::Invalid(format!("{}: {e}", out_png.display())))?;
            return Ok(true);
        }
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

pub fn fast_dimensions(path: &Path) -> Option<(u32, u32, bool)> {
    if is_unpacked_pcraft_dir(path) {
        for rel in PCRAFT_PREVIEWS {
            if let Ok((w, h)) = image::image_dimensions(path.join(rel)) {
                return Some((w, h, true));
            }
        }
        return None;
    }
    let bytes_opt = std::fs::read(path).ok();
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase();
    if let Some(bytes) = bytes_opt.as_deref()
        && let Ok(Some(insp)) = craft_image_codecs::inspect_raster(bytes)
    {
        let has_alpha = matches!(
            insp.color_type,
            craft_image_codecs::RasterColorType::Rgba | craft_image_codecs::RasterColorType::CmykAlpha | craft_image_codecs::RasterColorType::GrayAlpha
        );
        let orientation = crate::metadata::extract_image_metadata_bytes(bytes, (!ext.is_empty()).then_some(ext.as_str())).orientation;
        let (w, h) = crate::metadata::oriented_dimensions(insp.width, insp.height, orientation);
        return Some((w, h, has_alpha));
    }
    if let Ok((w, h)) = image::image_dimensions(path) {
        let has_alpha = matches!(ext.as_str(), "png" | "webp" | "gif" | "svg" | "tif" | "tiff");
        let orientation =
            bytes_opt.as_deref().and_then(|bytes| crate::metadata::extract_image_metadata_bytes(bytes, (!ext.is_empty()).then_some(ext.as_str())).orientation);
        let (ow, oh) = crate::metadata::oriented_dimensions(w, h, orientation);
        return Some((ow, oh, has_alpha));
    }
    let bytes = bytes_opt?;
    let meta = crate::metadata::extract_image_metadata_bytes(&bytes, (!ext.is_empty()).then_some(ext.as_str()));
    if let (Some(w), Some(h)) = (meta.width, meta.height) {
        let has_alpha = matches!(ext.as_str(), "psd" | "psb" | "png" | "webp" | "gif" | "svg" | "tif" | "tiff");
        let (ow, oh) = crate::metadata::oriented_dimensions(w, h, meta.orientation);
        return Some((ow, oh, has_alpha));
    }
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

#[allow(clippy::cognitive_complexity)]
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
    let info = if dec.length_known() {
        dec.info().clone()
    } else {
        let f2 = std::fs::File::open(path).map_err(|e| UtilError::Invalid(format!("cannot read {}: {e}", path.display())))?;
        let src2 = ReadSeekSource::new(f2).map_err(|e| UtilError::Invalid(format!("cannot size {}: {e}", path.display())))?;
        Decoder::open(src2, ext, &DecodeOptions::default())
            .and_then(Decoder::probe)
            .map_err(|e| UtilError::Invalid(format!("cannot inspect {}: {e}", path.display())))?
    };
    let ticks = Tick::from_seconds_f64(info.duration_secs()).0;
    Ok(json!({
        "kind": "Audio",
        "container": info.container.name(),
        "codec": info.codec.name(),
        "duration": ticks,
        "damaged": info.damaged,
        "audio_streams": [{
            "codec": info.codec.name(),
            "channels": info.layout.channels(),
            "layout": info.layout.to_string(),
            "sample_rate": info.sample_rate,
        }],
    }))
}

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

pub fn is_isobmff_file(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "mp4" | "mov" | "m4v"))
}

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

pub fn discover_installed_font_families() -> Vec<String> {
    Catalog::global().families().iter().map(|s| s.to_string()).collect()
}

pub fn font_family_styles(family: &str) -> Vec<String> {
    Catalog::global().styles(family).into_iter().map(|face| face.style.clone()).collect()
}

pub fn is_font_family_installed(family: &str) -> bool {
    let catalog = Catalog::global();
    catalog.family(family).is_some() || catalog.resolve(family, "Regular").is_some()
}
