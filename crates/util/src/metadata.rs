//! Embedded & sidecar media metadata backed by `craft-metadata`, `craft-psd`, `craft-heif`, and `craft-raw`.
//!
//! Handles EXIF, IPTC-IIM, XMP (read/write/merge), GPX tracklog geotagging, orientation-aware
//! pixel dimensions, and container-specific metadata blocks (HEIF/HEIC/AVIF, PSD/PSB, Minolta MRW).

use std::path::{Path, PathBuf};

pub use craft_metadata::{
    DatePrecision, DateTime, Embedded, Flash, FocusPoint, Gps, Match, MergeRules, Metadata, Orientation, Point, Rect, Region, RegionKind, TagRow, TrackPoint,
    Tracklog,
};
use serde_json::{Value, json};

use crate::{Result, UtilError};

fn is_heif_or_avif(bytes: &[u8], ext_hint: Option<&str>) -> bool {
    if matches!(ext_hint, Some("heic" | "heif" | "avif" | "hif")) {
        return true;
    }
    if bytes.len() >= 12 && bytes.get(4..8) == Some(b"ftyp") {
        let brand = bytes.get(8..12);
        return matches!(brand, Some(b"heic" | b"heix" | b"hevc" | b"hevx" | b"heim" | b"heis" | b"mif1" | b"msf1" | b"avif" | b"avis"));
    }
    false
}

fn extract_psd_metadata(bytes: &[u8]) -> Option<Metadata> {
    let head = craft_psd::HeadView::from_bytes(bytes).ok()?;
    let mut exif: Option<Vec<u8>> = None;
    let mut xmp: Option<String> = None;
    let mut iptc: Option<Vec<u8>> = None;
    for res in head.resources() {
        let Ok(r) = res else { break };
        match r.id {
            craft_psd::resources::ids::EXIF => {
                exif = Some(craft_metadata::strip_exif_header(r.data).to_vec());
            }
            craft_psd::resources::ids::XMP => {
                xmp = std::str::from_utf8(r.data).ok().map(str::to_owned);
            }
            1028 => {
                iptc = Some(r.data.to_vec());
            }
            _ => {}
        }
    }
    let mut meta = craft_metadata::from_embedded(&Embedded { exif, xmp, iptc, ..Default::default() });
    if meta.width.is_none() {
        meta.width = Some(head.header.width);
    }
    if meta.height.is_none() {
        meta.height = Some(head.header.height);
    }
    Some(meta)
}

fn extract_mrw_metadata(bytes: &[u8]) -> Option<Metadata> {
    let mrw = craft_raw::mrw::parse(bytes)?;
    let exif = mrw.block(b"TTW").map(|b| b.to_vec());
    Some(craft_metadata::from_embedded(&Embedded { exif, ..Default::default() }))
}

fn extract_heif_metadata(bytes: &[u8]) -> Option<Metadata> {
    let (exif, xmp) = craft_heif::read_metadata(bytes).ok()?;
    Some(craft_metadata::from_embedded(&Embedded { exif, xmp, ..Default::default() }))
}

/// Extract unified photo/media metadata from raw file bytes across standard and specialty containers.
pub fn extract_image_metadata_bytes(bytes: &[u8], ext_hint: Option<&str>) -> Metadata {
    let ext_lc = ext_hint.map(str::to_ascii_lowercase);
    let ext = ext_lc.as_deref();
    if (bytes.starts_with(b"8BPS") || matches!(ext, Some("psd" | "psb")))
        && let Some(meta) = extract_psd_metadata(bytes)
    {
        return meta;
    }
    if (bytes.starts_with(b"\0MRM") || ext == Some("mrw"))
        && let Some(meta) = extract_mrw_metadata(bytes)
    {
        return meta;
    }
    if is_heif_or_avif(bytes, ext)
        && let Some(meta) = extract_heif_metadata(bytes)
    {
        return meta;
    }
    craft_metadata::extract(bytes)
}

/// Return existing `.xmp` sidecar paths associated with `path` in precedence order.
pub fn find_xmp_sidecar(path: &Path) -> Option<PathBuf> {
    let stem_sidecar = path.with_extension("xmp");
    if stem_sidecar.is_file() {
        return Some(stem_sidecar);
    }
    let appended_sidecar = PathBuf::from(format!("{}.xmp", path.display()));
    if appended_sidecar.is_file() {
        return Some(appended_sidecar);
    }
    None
}

/// Read metadata from `path`, overlaying any sibling `.xmp` sidecar packet when present.
pub fn extract_image_metadata(path: &Path) -> Result<Metadata> {
    let bytes = std::fs::read(path)?;
    let ext = path.extension().and_then(|e| e.to_str());
    let mut meta = extract_image_metadata_bytes(&bytes, ext);
    if let Some(sidecar) = find_xmp_sidecar(path)
        && let Ok(text) = std::fs::read_to_string(&sidecar)
        && let Ok(xmp) = craft_metadata::parse_xmp(&text)
    {
        meta.overlay_user_fields(&xmp.metadata);
        meta.fill_missing(&xmp.metadata);
    }
    Ok(meta)
}

/// Apply EXIF axis-swapping orientations (`5..=8`) to stored pixel dimensions.
pub fn oriented_dimensions(raw_w: u32, raw_h: u32, orientation: Option<Orientation>) -> (u32, u32) {
    if orientation.is_some_and(Orientation::swaps_axes) { (raw_h, raw_w) } else { (raw_w, raw_h) }
}

/// Extract detailed `(group, tag, name, value)` EXIF/TIFF/GPS rows from `bytes`.
pub fn inspect_tag_rows(bytes: &[u8]) -> Vec<Value> {
    craft_metadata::file_tag_rows(bytes)
        .into_iter()
        .map(|row| {
            json!({
                "group": row.group,
                "tag": row.tag,
                "name": row.name,
                "value": row.value,
            })
        })
        .collect()
}

/// Match `meta.capture_time` against a GPX XML tracklog and populate `meta.gps`.
pub fn geotag_metadata_from_gpx(meta: &mut Metadata, gpx_xml: &str, max_gap_secs: f64) -> Result<Option<Match>> {
    let tracklog = craft_metadata::parse_gpx(gpx_xml).map_err(|e| UtilError::Invalid(format!("invalid GPX tracklog: {e}")))?;
    let Some(dt) = meta.capture_time else {
        return Ok(None);
    };
    let capture_secs = dt.unix_seconds() as f64 + f64::from(dt.millis) / 1000.0;
    if let Some((gps, kind)) = tracklog.locate(capture_secs, max_gap_secs) {
        meta.gps = Some(gps);
        Ok(Some(kind))
    } else {
        Ok(None)
    }
}

/// Serialize `meta` to an XMP packet, merging cleanly into `existing_xmp` when provided.
pub fn merge_or_write_xmp(existing_xmp: Option<&str>, meta: &Metadata, rules: &MergeRules<'_>) -> Result<String> {
    let fresh = craft_metadata::write_xmp(meta);
    match existing_xmp {
        Some(prev) => craft_metadata::merge_xmp(prev, &fresh, *rules).map_err(|e| UtilError::Invalid(format!("cannot merge XMP packet: {e}"))),
        None => Ok(fresh),
    }
}

/// Encode `meta` into a standalone TIFF-structured EXIF byte block.
pub fn encode_exif_tiff(meta: &Metadata) -> Vec<u8> {
    craft_metadata::write_exif(meta)
}
