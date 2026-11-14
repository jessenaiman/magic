//! Headless Unicode line segmentation (UAX #14 + kinsoku), grapheme-cluster emergency breaks
//! (UAX #29), bidirectional paragraph ordering (UAX #9) via [`craft_text`], and OpenType
//! glyph advance measurement via [`craft_font_catalog::Catalog`] + [`skrifa`].

use craft_font_catalog::Catalog;
use craft_fonts_build::EmbeddedFace;
use craft_text::bidi::{Direction, Paragraph};
use craft_text::segment::{Kinsoku, Options, emergency_breaks, line_breaks};
use skrifa::instance::{LocationRef, Size};
use skrifa::{FontRef, MetadataProvider as _};

/// True when the first paragraph of `text` resolves to an odd (right-to-left) UAX #9 embedding level.
pub fn is_paragraph_rtl(text: &str, force_rtl: bool) -> bool {
    let dir = if force_rtl { Direction::Rtl } else { Direction::Auto };
    let para = Paragraph::new(text, dir);
    para.level() % 2 == 1
}

fn trim_leading_ws(text: &str, mut pos: usize, limit: usize) -> usize {
    while pos < limit && text[pos..].starts_with(char::is_whitespace) {
        if let Some(ch) = text[pos..].chars().next() {
            pos += ch.len_utf8();
        } else {
            break;
        }
    }
    pos
}

fn push_emergency_chunks(trimmed: &str, mut start: usize, end: usize, cap: usize, lines: &mut Vec<String>) -> usize {
    while trimmed[start..end].trim_end().chars().count() > cap {
        let mut split_at = None;
        for eb in emergency_breaks(trimmed, start..end) {
            if eb > start && trimmed[start..eb].chars().count() <= cap {
                split_at = Some(eb);
            } else if trimmed[start..eb].chars().count() > cap {
                break;
            }
        }
        let cut = split_at.unwrap_or(end);
        if cut <= start {
            break;
        }
        lines.push(trimmed[start..cut].trim_end().to_owned());
        start = cut;
    }
    start
}

/// Wrap `para` into lines of at most `max_chars` characters using `craft_text::segment::line_breaks`
/// (`Kinsoku::Soft`) and `emergency_breaks` so emoji ZWJ sequences and combining marks stay intact.
pub fn wrap_paragraph_uax14(para: &str, max_chars: usize) -> Vec<String> {
    let trimmed = para.trim();
    if trimmed.is_empty() {
        return vec![String::new()];
    }
    let cap = max_chars.max(4);
    let options = Options { kinsoku: Kinsoku::Soft, ..Options::default() };

    let mut lines = Vec::new();
    let mut line_start = 0usize;
    let mut last_good_end = 0usize;

    for brk in line_breaks(trimmed, &options) {
        let off = brk.offset.min(trimmed.len());
        let candidate = trimmed[line_start..off].trim_end();
        if candidate.chars().count() <= cap {
            last_good_end = off;
            if brk.mandatory {
                lines.push(candidate.to_owned());
                line_start = trim_leading_ws(trimmed, off, trimmed.len());
                last_good_end = line_start;
            }
            continue;
        }
        if last_good_end > line_start {
            lines.push(trimmed[line_start..last_good_end].trim_end().to_owned());
            line_start = trim_leading_ws(trimmed, last_good_end, off);
        }
        line_start = push_emergency_chunks(trimmed, line_start, off, cap, &mut lines);
        last_good_end = off;
        if brk.mandatory && line_start < trimmed.len() {
            let tail = trimmed[line_start..off].trim();
            if !tail.is_empty() {
                lines.push(tail.to_owned());
            }
            line_start = off;
            last_good_end = off;
        }
    }

    let rem = trimmed[line_start..].trim();
    if !rem.is_empty() {
        lines.push(rem.to_owned());
    }
    if lines.is_empty() {
        lines.push(trimmed.to_owned());
    }
    lines
}

fn style_name(weight: u16, italic: bool) -> &'static str {
    match (weight >= 650, italic) {
        (true, true) => "Bold Italic",
        (true, false) => "Bold",
        (false, true) => "Italic",
        (false, false) => "Regular",
    }
}

fn find_embedded_face(family: &str, weight: u16, italic: bool) -> Option<&'static EmbeddedFace> {
    let want_style = style_name(weight, italic);
    crate::CRAFT_FONTS
        .iter()
        .find(|f| f.family.eq_ignore_ascii_case(family) && f.style.eq_ignore_ascii_case(want_style))
        .or_else(|| crate::CRAFT_FONTS.iter().find(|f| f.family.eq_ignore_ascii_case(family)))
}

fn measure_skrifa_advance(text: &str, font_bytes: &[u8], face_index: u32, size_px: f64, variations: &[([u8; 4], f32)]) -> Option<f64> {
    let font = FontRef::from_index(font_bytes, face_index).ok()?;
    let charmap = font.charmap();
    let loc = (!variations.is_empty()).then(|| font.axes().location(variations.iter().map(|(tag, val)| (skrifa::Tag::new(tag), *val))));
    let loc_ref = loc.as_ref().map_or_else(LocationRef::default, LocationRef::from);
    let metrics = font.glyph_metrics(Size::new(size_px as f32), loc_ref);
    let mut sum = 0.0_f64;
    for ch in text.chars() {
        let gid = charmap.map(ch).unwrap_or_default();
        sum += metrics.advance_width(gid).map_or(size_px * 0.5, f64::from);
    }
    Some(sum)
}

fn proportional_fallback_advance(text: &str, family: &str, weight: u16, italic: bool, size_px: f64) -> f64 {
    let fam_lower = family.to_ascii_lowercase();
    let family_factor = if fam_lower.contains("mono") || fam_lower.contains("code") {
        1.04
    } else if fam_lower.contains("serif") || fam_lower == "song" {
        0.98
    } else {
        1.0
    };
    let weight_factor = if weight >= 700 { 1.06 } else { 1.0 };
    let em_sum: f64 = text
        .chars()
        .map(|ch| match ch {
            ' ' => 0.28,
            'i' | 'l' | 'I' | '|' | '!' | '.' | ',' | ':' | ';' | '\'' => 0.28,
            'j' | 't' | 'f' | 'r' => 0.38,
            'm' | 'w' | 'M' | 'W' => 0.86,
            ch if !ch.is_ascii() => 0.88,
            ch if ch.is_ascii_uppercase() => 0.68,
            _ => 0.54,
        })
        .sum();
    let italic_pad = if italic { size_px * 0.08 } else { 0.0 };
    (em_sum * size_px * family_factor * weight_factor) + italic_pad
}

/// Measure horizontal advance width in pixels for `text` using [`craft_font_catalog::Catalog`]
/// and [`skrifa`], falling back to proportional estimation only when no matching font is installed.
pub fn measure_text_advance(text: &str, family: &str, weight: u16, italic: bool, size_px: f64) -> f64 {
    if text.is_empty() || !size_px.is_finite() || size_px <= 0.0 {
        return 0.0;
    }
    if let Some(embedded) = find_embedded_face(family, weight, italic)
        && let Some(measured) = measure_skrifa_advance(text, embedded.bytes, embedded.index, size_px, &[])
    {
        return measured;
    }

    let catalog = Catalog::global();
    if let Some((face, _)) = catalog.resolve(family, style_name(weight, italic))
        && let Ok(data) = catalog.load(&face)
        && let Some(mut measured) = measure_skrifa_advance(text, &data.bytes, data.index, size_px, &face.variations)
    {
        if weight >= 700 && face.weight < 600.0 {
            measured *= 1.05;
        }
        if italic && !face.italic {
            measured += size_px * 0.07;
        }
        return measured;
    }

    proportional_fallback_advance(text, family, weight, italic, size_px)
}
