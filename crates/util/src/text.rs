//! Headless Unicode line segmentation (UAX #14 + kinsoku), grapheme-cluster emergency breaks
//! (UAX #29), bidirectional paragraph ordering (UAX #9), dictionary hyphenation ([`craft_text`]),
//! OpenType `hmtx`/`cmap`/`kern` glyph advance width measurement ([`craft_font_catalog`]), and
//! canonical codepoint range encoding for [`storytold/font-preview-service`].

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};

use craft_font_catalog::Catalog;
use craft_text::bidi::{Direction, Paragraph};
use craft_text::hyphen::{Lang, Limits, hyphen_points_in};
use craft_text::segment::{Kinsoku, Options, emergency_breaks, line_breaks};

/// Horizontal span metrics used to compute non-overlapping collision spacing between adjacent words.
#[derive(Clone, Copy, Debug)]
pub struct TextSpanMetrics {
    pub width: f64,
    pub size: f64,
    pub is_hero: bool,
    pub swell_ratio: f64,
}

#[derive(Clone)]
struct CachedFontFace {
    family_norm: String,
    weight: u16,
    italic: bool,
    face_index: u32,
    bytes: Arc<[u8]>,
}

#[derive(Default)]
struct FontDiskCache {
    scanned: bool,
    faces: Vec<CachedFontFace>,
    by_query: HashMap<String, Option<(Arc<[u8]>, u32, u16, bool)>>,
}

static FONT_DISK_CACHE: LazyLock<Mutex<FontDiskCache>> = LazyLock::new(|| Mutex::new(FontDiskCache::default()));

/// Return byte offsets where `word` may hyphenate for `language` (BCP 47 code or English name).
pub fn hyphenation_offsets(word: &str, language: &str) -> Vec<usize> {
    match Lang::for_language(language) {
        Some(lang) => hyphen_points_in(word, &Limits::default(), lang),
        None => Vec::new(),
    }
}

/// True when the first paragraph of `text` resolves to an odd (right-to-left) UAX #9 embedding level.
pub fn is_paragraph_rtl(text: &str, force_rtl: bool) -> bool {
    let dir = if force_rtl { Direction::Rtl } else { Direction::Auto };
    let para = Paragraph::new(text, dir);
    para.level() % 2 == 1
}

/// Encode up to 256 unique Unicode scalar values from `sample_text` into the canonical
/// `storytold/font-preview-service` `{codepoints}` format: sorted lowercase hex with contiguous
/// runs compacted as `start-end` and joined by `,` (for example `20,41-5a,e9`).
pub fn canonical_codepoint_spec(sample_text: &str) -> Option<String> {
    let mut cps = BTreeSet::new();
    for ch in sample_text.chars() {
        if ch.is_control() && ch != ' ' {
            continue;
        }
        cps.insert(u32::from(ch));
        if cps.len() >= 256 {
            break;
        }
    }
    if cps.is_empty() {
        return None;
    }
    let sorted: Vec<u32> = cps.into_iter().collect();
    let mut parts = Vec::new();
    let mut idx = 0usize;
    while idx < sorted.len() {
        let start = sorted[idx];
        let mut end = start;
        while idx + 1 < sorted.len() && sorted[idx + 1] == end + 1 {
            idx += 1;
            end = sorted[idx];
        }
        if start == end {
            parts.push(format!("{start:x}"));
        } else {
            parts.push(format!("{start:x}-{end:x}"));
        }
        idx += 1;
    }
    Some(parts.join(","))
}

/// Build an on-demand WOFF2 subset route (`/v1/faces/{sha256}/{inst}/u/{codepoints}.woff2`)
/// against a `storytold/font-preview-service` base URL (`http://127.0.0.1:8000` or HTTPS).
pub fn font_preview_subset_url(base_url: &str, sha256: &str, inst: &str, sample_text: &str) -> Option<String> {
    let base = base_url.trim().trim_end_matches('/');
    let hash = sha256.trim();
    let instance = if inst.trim().is_empty() { "list" } else { inst.trim() };
    if base.is_empty() || hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let codepoints = canonical_codepoint_spec(sample_text)?;
    Some(format!("{base}/v1/faces/{}/{}//u/{codepoints}.woff2", hash.to_ascii_lowercase(), instance).replace("//u/", "/u/"))
}

#[allow(clippy::cognitive_complexity)]
/// Wrap `para` into lines of at most `max_chars` characters using `craft_text::segment::line_breaks`
/// (`Kinsoku::Soft`, preserving CJK kinsoku rules) with `emergency_breaks` enabled so emoji ZWJ
/// sequences and combining marks are never split mid-cluster.
pub fn wrap_paragraph_uax14(para: &str, max_chars: usize) -> Vec<String> {
    let trimmed = para.trim();
    if trimmed.is_empty() {
        return vec![String::new()];
    }
    let cap = max_chars.max(4);
    let options = Options { kinsoku: Kinsoku::Soft, ..Options::default() };
    let breaks = line_breaks(trimmed, &options);

    let mut lines = Vec::new();
    let mut line_start = 0usize;
    let mut last_good_end = 0usize;

    for brk in breaks {
        let off = brk.offset.min(trimmed.len());
        let candidate = trimmed[line_start..off].trim_end();
        let count = candidate.chars().count();

        if count <= cap {
            last_good_end = off;
            if brk.mandatory {
                lines.push(candidate.to_owned());
                line_start = off;
                while line_start < trimmed.len() && trimmed[line_start..].starts_with(char::is_whitespace) {
                    if let Some(ch) = trimmed[line_start..].chars().next() {
                        line_start += ch.len_utf8();
                    } else {
                        break;
                    }
                }
                last_good_end = line_start;
            }
            continue;
        }

        if last_good_end > line_start {
            lines.push(trimmed[line_start..last_good_end].trim_end().to_owned());
            line_start = last_good_end;
            while line_start < off && trimmed[line_start..].starts_with(char::is_whitespace) {
                if let Some(ch) = trimmed[line_start..].chars().next() {
                    line_start += ch.len_utf8();
                } else {
                    break;
                }
            }
        }

        while trimmed[line_start..off].trim_end().chars().count() > cap {
            let em_breaks = emergency_breaks(trimmed, line_start..off);
            let mut split_at = None;
            for eb in em_breaks {
                if eb > line_start && trimmed[line_start..eb].chars().count() <= cap {
                    split_at = Some(eb);
                } else if trimmed[line_start..eb].chars().count() > cap {
                    break;
                }
            }
            let cut = split_at.unwrap_or(off);
            if cut <= line_start {
                break;
            }
            lines.push(trimmed[line_start..cut].trim_end().to_owned());
            line_start = cut;
        }
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

    if line_start < trimmed.len() {
        let rem = trimmed[line_start..].trim();
        if !rem.is_empty() {
            lines.push(rem.to_owned());
        }
    }
    if lines.is_empty() {
        lines.push(trimmed.to_owned());
    }
    lines
}

fn u16_be(data: &[u8], off: usize) -> Option<u16> {
    let b = data.get(off..off.checked_add(2)?)?;
    Some(u16::from_be_bytes([b[0], b[1]]))
}

fn i16_be(data: &[u8], off: usize) -> Option<i16> {
    let b = data.get(off..off.checked_add(2)?)?;
    Some(i16::from_be_bytes([b[0], b[1]]))
}

fn u32_be(data: &[u8], off: usize) -> Option<u32> {
    let b = data.get(off..off.checked_add(4)?)?;
    Some(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

fn sfnt_offset_for_index(bytes: &[u8], face_index: u32) -> Option<usize> {
    let magic = bytes.get(0..4)?;
    if magic == b"ttcf" {
        let count = u32_be(bytes, 8)?;
        if face_index >= count {
            return None;
        }
        let table_pos = 12usize.checked_add((face_index as usize).checked_mul(4)?)?;
        Some(u32_be(bytes, table_pos)? as usize)
    } else if face_index == 0 && matches!(magic, [0x00, 0x01, 0x00, 0x00] | b"OTTO" | b"true" | b"typ1") {
        Some(0)
    } else {
        None
    }
}

fn find_sfnt_table<'a>(bytes: &'a [u8], sfnt_off: usize, tag: &[u8; 4]) -> Option<&'a [u8]> {
    let num_tables = (u16_be(bytes, sfnt_off.checked_add(4)?)? as usize).min(256);
    let records_base = sfnt_off.checked_add(12)?;
    for idx in 0..num_tables {
        let rec = records_base.checked_add(idx.checked_mul(16)?)?;
        if bytes.get(rec..rec.checked_add(4)?)? == tag {
            let off = u32_be(bytes, rec.checked_add(8)?)? as usize;
            let len = u32_be(bytes, rec.checked_add(12)?)? as usize;
            return bytes.get(off..off.checked_add(len)?);
        }
    }
    None
}

fn cmap_format12_lookup(sub: &[u8], cp: u32) -> Option<u16> {
    let num_groups = (u32_be(sub, 12)? as usize).min(65_536);
    let mut lo = 0usize;
    let mut hi = num_groups;
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        let off = 16usize.checked_add(mid.checked_mul(12)?)?;
        let start = u32_be(sub, off)?;
        let end = u32_be(sub, off + 4)?;
        if cp < start {
            hi = mid;
        } else if cp > end {
            lo = mid + 1;
        } else {
            let start_gid = u32_be(sub, off + 8)?;
            return u16::try_from(start_gid.saturating_add(cp - start)).ok();
        }
    }
    Some(0)
}

fn cmap_format4_lookup(sub: &[u8], cp: u32) -> Option<u16> {
    let cp16 = u16::try_from(cp).ok()?;
    let seg_count_x2 = u16_be(sub, 6)? as usize;
    if seg_count_x2 < 2 || !seg_count_x2.is_multiple_of(2) {
        return None;
    }
    let seg_count = seg_count_x2 / 2;
    let end_off = 14usize;
    let start_off = end_off.checked_add(seg_count_x2)?.checked_add(2)?;
    let delta_off = start_off.checked_add(seg_count_x2)?;
    let range_off = delta_off.checked_add(seg_count_x2)?;

    let mut lo = 0usize;
    let mut hi = seg_count;
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        let end_code = u16_be(sub, end_off + mid * 2)?;
        if cp16 > end_code {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    if lo >= seg_count {
        return Some(0);
    }
    let start_code = u16_be(sub, start_off + lo * 2)?;
    if cp16 < start_code {
        return Some(0);
    }
    let id_delta = i32::from(i16_be(sub, delta_off + lo * 2)?);
    let id_range_offset = u16_be(sub, range_off + lo * 2)? as usize;
    if id_range_offset == 0 {
        return Some(((i32::from(cp16) + id_delta) & 0xFFFF) as u16);
    }
    let glyph_slot = (range_off + lo * 2).checked_add(id_range_offset)?.checked_add(usize::from(cp16 - start_code) * 2)?;
    let raw_gid = u16_be(sub, glyph_slot)?;
    if raw_gid == 0 { Some(0) } else { Some(((i32::from(raw_gid) + id_delta) & 0xFFFF) as u16) }
}

fn cmap_glyph_id(cmap: &[u8], ch: char) -> Option<u16> {
    let num_subtables = (u16_be(cmap, 2)? as usize).min(64);
    let cp = u32::from(ch);
    let mut fmt12_off: Option<usize> = None;
    let mut fmt4_off: Option<usize> = None;

    for idx in 0..num_subtables {
        let rec = 4 + idx * 8;
        let platform = u16_be(cmap, rec)?;
        let encoding = u16_be(cmap, rec + 2)?;
        let sub_off = u32_be(cmap, rec + 4)? as usize;
        let Some(fmt) = u16_be(cmap, sub_off) else { continue };
        let is_unicode = platform == 0 || (platform == 3 && matches!(encoding, 1 | 10));
        if is_unicode && fmt == 12 && fmt12_off.is_none() {
            fmt12_off = Some(sub_off);
        } else if is_unicode && fmt == 4 && fmt4_off.is_none() {
            fmt4_off = Some(sub_off);
        }
    }

    if let Some(off) = fmt12_off
        && let Some(sub) = cmap.get(off..)
        && let Some(gid) = cmap_format12_lookup(sub, cp)
        && gid != 0
    {
        return Some(gid);
    }
    if let Some(off) = fmt4_off
        && let Some(sub) = cmap.get(off..)
    {
        return cmap_format4_lookup(sub, cp);
    }
    fmt12_off.and_then(|off| cmap.get(off..)).and_then(|sub| cmap_format12_lookup(sub, cp))
}

fn hmtx_advance_units(hmtx: &[u8], num_hmetrics: u16, gid: u16) -> Option<u16> {
    if num_hmetrics == 0 {
        return None;
    }
    let idx = usize::from(gid.min(num_hmetrics - 1));
    u16_be(hmtx, idx.checked_mul(4)?)
}

fn kern_pair_units(kern: Option<&[u8]>, left_gid: u16, right_gid: u16) -> i16 {
    let Some(table) = kern else { return 0 };
    if u16_be(table, 0) != Some(0) {
        return 0;
    }
    let n_tables = (u16_be(table, 2).unwrap_or(0) as usize).min(16);
    let target = (u32::from(left_gid) << 16) | u32::from(right_gid);
    let mut sub_off = 4usize;

    for _ in 0..n_tables {
        let Some(len) = u16_be(table, sub_off + 2).map(usize::from) else { break };
        let Some(coverage) = u16_be(table, sub_off + 4) else { break };
        let is_horiz_fmt0 = (coverage & 0x0001) != 0 && (coverage & 0x0004) == 0 && (coverage >> 8) == 0;
        if is_horiz_fmt0 && let Some(n_pairs) = u16_be(table, sub_off + 6).map(usize::from) {
            let pairs_base = sub_off + 14;
            let mut lo = 0usize;
            let mut hi = n_pairs;
            while lo < hi {
                let mid = lo + (hi - lo) / 2;
                let off = pairs_base + mid * 6;
                let Some(pair_key) = u32_be(table, off) else { break };
                if target < pair_key {
                    hi = mid;
                } else if target > pair_key {
                    lo = mid + 1;
                } else {
                    return i16_be(table, off + 4).unwrap_or(0);
                }
            }
        }
        if len < 6 {
            break;
        }
        sub_off = sub_off.saturating_add(len);
    }
    0
}

/// Measure horizontal advance width in pixels from raw OpenType/TrueType (`hmtx` + `cmap` + `kern`)
/// font bytes for `face_index` at `size_px`. Returns `None` if tables are missing or no glyphs map.
#[allow(clippy::cognitive_complexity)]
pub fn measure_opentype_advance(text: &str, font_bytes: &[u8], face_index: u32, size_px: f64) -> Option<f64> {
    if text.is_empty() {
        return Some(0.0);
    }
    if !size_px.is_finite() || size_px <= 0.0 {
        return Some(0.0);
    }
    let sfnt_off = sfnt_offset_for_index(font_bytes, face_index)?;
    let head = find_sfnt_table(font_bytes, sfnt_off, b"head")?;
    let upem = u16_be(head, 18).filter(|&u| (16..=16_384).contains(&u))?;
    let hhea = find_sfnt_table(font_bytes, sfnt_off, b"hhea")?;
    let num_hm = u16_be(hhea, 34).filter(|&n| n >= 1)?;
    let hmtx = find_sfnt_table(font_bytes, sfnt_off, b"hmtx")?;
    let cmap = find_sfnt_table(font_bytes, sfnt_off, b"cmap")?;
    let kern = find_sfnt_table(font_bytes, sfnt_off, b"kern");

    let scale = size_px / f64::from(upem);
    let mut total_px = 0.0_f64;
    let mut prev_gid: Option<u16> = None;
    let mut mapped_any = false;

    for ch in text.chars() {
        if ch.is_control() && ch != ' ' && ch != '\t' {
            continue;
        }
        let gid = cmap_glyph_id(cmap, ch).unwrap_or(0);
        if gid != 0 {
            mapped_any = true;
            if let Some(adv) = hmtx_advance_units(hmtx, num_hm, gid) {
                let kern_u = prev_gid.map_or(0, |l| kern_pair_units(kern, l, gid));
                total_px += (f64::from(adv) + f64::from(kern_u)).max(0.0) * scale;
                prev_gid = Some(gid);
                continue;
            }
        } else if ch == ' '
            && let Some(space_gid) = cmap_glyph_id(cmap, ' ').filter(|&g| g != 0)
            && let Some(adv) = hmtx_advance_units(hmtx, num_hm, space_gid)
        {
            mapped_any = true;
            total_px += f64::from(adv) * scale;
            prev_gid = Some(space_gid);
            continue;
        }
        // Fallback for an unmapped glyph within an otherwise valid face
        total_px += proportional_char_em(ch) * size_px;
        prev_gid = None;
    }

    mapped_any.then_some(total_px)
}

fn normalize_family_key(name: &str) -> String {
    name.chars().filter(|c| c.is_ascii_alphanumeric()).map(|c| c.to_ascii_lowercase()).collect()
}

fn style_weight_and_italic(style: &str) -> (u16, bool) {
    let lower = style.to_ascii_lowercase();
    let italic = lower.contains("italic") || lower.contains("oblique");
    let weight = if lower.contains("black") || lower.contains("heavy") {
        900
    } else if lower.contains("extrabold") || lower.contains("ultrabold") {
        800
    } else if lower.contains("semibold") || lower.contains("demibold") {
        600
    } else if lower.contains("bold") {
        700
    } else if lower.contains("medium") {
        500
    } else if lower.contains("light") {
        300
    } else if lower.contains("thin") || lower.contains("hairline") {
        100
    } else {
        400
    };
    (weight, italic)
}

fn decode_sfnt_name_string(bytes: &[u8], platform: u16) -> Option<String> {
    if platform == 0 || platform == 3 {
        if !bytes.len().is_multiple_of(2) {
            return None;
        }
        let units: Vec<u16> = bytes.chunks_exact(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
        String::from_utf16(&units).ok()
    } else {
        Some(bytes.iter().map(|&b| char::from(b)).collect())
    }
}

#[allow(clippy::cognitive_complexity)]
fn parse_sfnt_face_header(bytes: &Arc<[u8]>, face_index: u32) -> Option<CachedFontFace> {
    let sfnt_off = sfnt_offset_for_index(bytes, face_index)?;
    let name_tbl = find_sfnt_table(bytes, sfnt_off, b"name")?;
    let _ = find_sfnt_table(bytes, sfnt_off, b"hmtx")?;
    let _ = find_sfnt_table(bytes, sfnt_off, b"cmap")?;

    let count = (u16_be(name_tbl, 2)? as usize).min(512);
    let storage_off = u16_be(name_tbl, 4)? as usize;
    let mut fam_pref: Option<String> = None;
    let mut fam_base: Option<String> = None;
    let mut sub_str: Option<String> = None;

    for idx in 0..count {
        let rec = 6 + idx * 12;
        let platform = u16_be(name_tbl, rec)?;
        let name_id = u16_be(name_tbl, rec + 6)?;
        if !matches!(name_id, 1 | 2 | 16 | 17) {
            continue;
        }
        let len = u16_be(name_tbl, rec + 8)? as usize;
        let str_off = storage_off.checked_add(u16_be(name_tbl, rec + 10)? as usize)?;
        let Some(slice) = name_tbl.get(str_off..str_off.checked_add(len)?) else { continue };
        let Some(decoded) = decode_sfnt_name_string(slice, platform).map(|s| s.trim().to_owned()).filter(|s| !s.is_empty()) else {
            continue;
        };
        match name_id {
            16 if fam_pref.is_none() => fam_pref = Some(decoded),
            1 if fam_base.is_none() => fam_base = Some(decoded),
            17 | 2 if sub_str.is_none() => sub_str = Some(decoded),
            _ => {}
        }
    }

    let family = fam_pref.or(fam_base)?;
    let family_norm = normalize_family_key(&family);
    if family_norm.is_empty() {
        return None;
    }
    let (mut weight, mut italic) = style_weight_and_italic(sub_str.as_deref().unwrap_or("Regular"));
    if let Some(os2) = find_sfnt_table(bytes, sfnt_off, b"OS/2") {
        if let Some(w) = u16_be(os2, 4).filter(|&w| (100..=1000).contains(&w)) {
            weight = w;
        }
        if let Some(sel) = u16_be(os2, 62) {
            italic = italic || (sel & 0x0001) != 0;
        }
    }
    Some(CachedFontFace { family_norm, weight, italic, face_index, bytes: Arc::clone(bytes) })
}

fn candidate_font_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    for env_key in ["EFFECTCRAFT_FONT_DIRS", "CRAFT_FONT_STORE"] {
        if let Some(val) = std::env::var_os(env_key) {
            for part in std::env::split_paths(&val) {
                if part.is_dir() {
                    dirs.push(part);
                }
            }
        }
    }
    if let Some(xdg) = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from) {
        dirs.push(xdg.join("craft-fonts"));
        dirs.push(xdg.join("fonts"));
    }
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        dirs.push(home.join(".local/share/craft-fonts"));
        dirs.push(home.join(".local/share/fonts"));
        dirs.push(home.join(".fonts"));
        dirs.push(home.join("Library/Application Support/Craft Fonts"));
        dirs.push(home.join("Library/Fonts"));
    }
    for sys in ["/usr/share/fonts", "/usr/local/share/fonts", "/Library/Fonts", "/System/Library/Fonts"] {
        let pb = PathBuf::from(sys);
        if pb.is_dir() {
            dirs.push(pb);
        }
    }
    dirs
}

fn walk_font_dir(dir: &Path, depth: usize, files: &mut Vec<PathBuf>) {
    if depth > 4 || files.len() >= 512 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        if files.len() >= 512 {
            break;
        }
        let p = entry.path();
        if p.is_dir() {
            walk_font_dir(&p, depth + 1, files);
        } else if p.extension().and_then(|e| e.to_str()).is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "ttf" | "otf" | "ttc" | "otc"))
            && std::fs::metadata(&p).is_ok_and(|m| m.len() <= 32 * 1024 * 1024)
        {
            files.push(p);
        }
    }
}

fn family_candidates_for_alias(family: &str) -> Vec<String> {
    let trimmed = family.trim();
    let lower = trimmed.to_ascii_lowercase();
    let mut out = Vec::new();
    if !trimmed.is_empty() {
        out.push(trimmed.to_owned());
    }
    match lower.as_str() {
        "sans" | "sans-bold" | "nimbus sans" => {
            for cand in ["Nimbus Sans", "Inter", "Liberation Sans", "DejaVu Sans", "Noto Sans", "Arial"] {
                out.push(cand.to_owned());
            }
        }
        "serif" | "song" | "song-bold" | "serif-bold" | "serif-italic" | "c059" => {
            for cand in ["C059", "Liberation Serif", "DejaVu Serif", "Noto Serif", "Times New Roman", "Georgia"] {
                out.push(cand.to_owned());
            }
        }
        _ => {}
    }
    out
}

fn style_distance(face_weight: u16, face_italic: bool, target_weight: u16, target_italic: bool) -> u32 {
    let italic_pen = if face_italic == target_italic { 0 } else { 1_000 };
    italic_pen + u32::from(face_weight.abs_diff(target_weight))
}

// shortcut: reads hmtx/cmap/kern without GPOS pair/contextual shaping; upgrade to full harfrust shaping if complex scripts need ligature widths
#[allow(clippy::cognitive_complexity, clippy::type_complexity)]
fn resolve_font_face(family: &str, weight: u16, italic: bool) -> Option<(Arc<[u8]>, u32, u16, bool)> {
    let candidates = family_candidates_for_alias(family);
    let norm_candidates: Vec<String> = candidates.iter().map(|c| normalize_family_key(c)).filter(|s| !s.is_empty()).collect();

    // 1. Check embedded CRAFT_FONTS (`craft-fonts-build`) first with zero disk I/O.
    let mut best_embedded: Option<(u32, &'static [u8], u32, u16, bool)> = None;
    for face in crate::CRAFT_FONTS {
        let fnorm = normalize_family_key(face.family);
        if norm_candidates.iter().any(|c| c == &fnorm) {
            let (_, f_italic) = style_weight_and_italic(face.style);
            let f_weight = face.weight();
            let dist = style_distance(f_weight, f_italic, weight, italic);
            if best_embedded.is_none_or(|(b, ..)| dist < b) {
                best_embedded = Some((dist, face.bytes, face.index, f_weight, f_italic));
            }
        }
    }
    if let Some((_, bytes, idx, fw, fi)) = best_embedded {
        return Some((Arc::from(bytes), idx, fw, fi));
    }

    // 2. Check `craft-font-catalog` (`Catalog::global()`) + cached SFNT table scan.
    let catalog = Catalog::global();
    let matched_catalog_family = candidates.iter().find(|cand| catalog.family(cand).is_some()).cloned();
    let has_env_dirs = std::env::var_os("EFFECTCRAFT_FONT_DIRS").is_some() || std::env::var_os("CRAFT_FONT_STORE").is_some();
    if matched_catalog_family.is_none() && !has_env_dirs {
        return None;
    }

    let query_key = format!("{}|{}|{}", norm_candidates.join(","), weight, italic);
    let mut guard = FONT_DISK_CACHE.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(cached) = guard.by_query.get(&query_key) {
        return cached.clone();
    }
    if !guard.scanned {
        guard.scanned = true;
        let mut font_files = Vec::new();
        for dir in candidate_font_dirs() {
            walk_font_dir(&dir, 0, &mut font_files);
        }
        for file in font_files {
            let Ok(raw) = std::fs::read(&file) else { continue };
            let shared: Arc<[u8]> = Arc::from(raw);
            let max_idx = if shared.get(0..4) == Some(b"ttcf") { u32_be(&shared, 8).unwrap_or(1).min(16) } else { 1 };
            for idx in 0..max_idx {
                if let Some(parsed) = parse_sfnt_face_header(&shared, idx) {
                    guard.faces.push(parsed);
                }
            }
        }
    }

    let mut best: Option<(usize, u32, &CachedFontFace)> = None;
    for (rank, cand_norm) in norm_candidates.iter().enumerate() {
        for face in &guard.faces {
            if &face.family_norm == cand_norm {
                let dist = style_distance(face.weight, face.italic, weight, italic);
                if best.is_none_or(|(b_rank, b_dist, _)| (rank, dist) < (b_rank, b_dist)) {
                    best = Some((rank, dist, face));
                }
            }
        }
    }
    let resolved = best.map(|(_, _, f)| (Arc::clone(&f.bytes), f.face_index, f.weight, f.italic));
    guard.by_query.insert(query_key, resolved.clone());
    resolved
}

fn proportional_char_em(ch: char) -> f64 {
    match ch {
        '\u{0300}'..='\u{036F}' | '\u{200B}'..='\u{200D}' | '\u{FE00}'..='\u{FE0F}' => 0.0,
        ' ' => 0.27,
        '\t' => 1.08,
        'i' | 'l' | '!' | '|' | '.' | ',' | ':' | ';' | '\'' | '`' | '¡' => 0.26,
        'I' | 'j' | '1' | '(' | ')' | '[' | ']' | '{' | '}' | '"' => 0.34,
        'f' | 't' | 'r' | '-' | '/' | '\\' => 0.38,
        's' | 'c' | 'z' | 'J' | '?' => 0.49,
        'a' | 'e' | 'o' | 'v' | 'x' | 'y' | 'k' => 0.53,
        'b' | 'd' | 'g' | 'h' | 'n' | 'p' | 'q' | 'u' | '0' | '2'..='9' => 0.56,
        'F' | 'L' | 'E' | 'P' | 'S' | 'T' | 'Z' => 0.59,
        'A' | 'B' | 'C' | 'K' | 'R' | 'V' | 'X' | 'Y' => 0.65,
        'D' | 'G' | 'H' | 'N' | 'O' | 'Q' | 'U' | '@' | '&' | '%' => 0.71,
        'm' | 'w' => 0.82,
        'M' | 'W' | '—' => 0.89,
        '\u{2E80}'..='\u{9FFF}' | '\u{AC00}'..='\u{D7AF}' | '\u{F900}'..='\u{FAFF}' | '\u{FF01}'..='\u{FF60}' => 1.0,
        c if u32::from(c) >= 0x1F300 => 1.05,
        c if c.is_uppercase() => 0.66,
        _ => 0.55,
    }
}

fn fallback_kern_em(left: char, right: char) -> f64 {
    match (left, right) {
        ('A', 'V' | 'W' | 'Y' | 'T') | ('V' | 'W' | 'Y', 'A') => -0.07,
        ('T', 'a' | 'e' | 'o' | 'r' | 'u' | 'w' | 'y')
        | ('V' | 'W' | 'Y', 'a' | 'e' | 'o' | 'u')
        | ('P' | 'F', 'a' | 'o' | 'e' | '.' | ',')
        | ('L', 'T' | 'V' | 'W' | 'Y') => -0.05,
        ('r', '.' | ',') | ('f', '\'' | '"') => -0.03,
        ('r', 'n') => 0.015,
        _ => 0.0,
    }
}

/// Measure horizontal advance width in pixels for `text` using OpenType `hmtx`/`cmap`/`kern`
/// metrics resolved via `CRAFT_FONTS` and [`craft_font_catalog::Catalog`], with a calibrated
/// proportional width + kerning fallback when running headlessly without system fonts.
pub fn measure_text_advance(text: &str, family: &str, weight: u16, italic: bool, size_px: f64) -> f64 {
    if text.is_empty() || !size_px.is_finite() || size_px <= 0.0 {
        return 0.0;
    }
    if let Some((font_bytes, face_index, face_weight, face_italic)) = resolve_font_face(family, weight, italic)
        && let Some(mut measured) = measure_opentype_advance(text, &font_bytes, face_index, size_px)
    {
        // Apply synthetic emboldening / oblique overhang only when the matched face lacked the style
        if weight >= 700 && face_weight < 600 {
            measured *= 1.05;
        }
        if italic && !face_italic {
            measured += size_px * 0.07;
        }
        return measured;
    }

    // Headless proportional advance + kerning fallback (`B10-01`)
    let fam_lower = family.to_ascii_lowercase();
    let family_factor = if fam_lower.contains("mono") || fam_lower.contains("code") {
        1.04
    } else if fam_lower.contains("serif") || fam_lower == "song" || fam_lower == "c059" {
        0.98
    } else {
        1.0
    };
    let weight_factor = if weight >= 800 {
        1.09
    } else if weight >= 700 {
        1.06
    } else if weight >= 600 {
        1.03
    } else {
        1.0
    };
    let mut em_sum = 0.0_f64;
    let mut prev: Option<char> = None;
    for ch in text.chars() {
        if let Some(p) = prev {
            em_sum += fallback_kern_em(p, ch);
        }
        em_sum += proportional_char_em(ch);
        prev = Some(ch);
    }
    let italic_pad = if italic { size_px * 0.08 } else { 0.0 };
    (em_sum.max(0.0) * size_px * family_factor * weight_factor) + italic_pad
}

/// Minimum horizontal non-overlap gap (in pixels) between two adjacent word spans, reserving
/// collision clearance for hero/emphasized words and peak motion swell (such as `pop` 1.35x).
pub fn minimum_non_overlap_gap(left: TextSpanMetrics, right: TextSpanMetrics) -> f64 {
    let max_size = left.size.max(right.size).max(12.0);
    let base_space = 0.28 * max_size;
    let left_swell = left.width.max(0.0) * left.swell_ratio.max(0.0);
    let right_swell = right.width.max(0.0) * right.swell_ratio.max(0.0);
    let hero_clearance = match (left.is_hero, right.is_hero) {
        (true, true) => (0.22 * max_size).max(16.0),
        (true, false) | (false, true) => (0.14 * max_size).max(10.0),
        (false, false) => 0.0,
    };
    base_space + left_swell + right_swell + hero_clearance
}

#[cfg(test)]
mod tests {
    use super::{TextSpanMetrics, measure_opentype_advance, measure_text_advance, minimum_non_overlap_gap};

    fn push_u16(buf: &mut Vec<u8>, v: u16) {
        buf.extend_from_slice(&v.to_be_bytes());
    }

    fn push_i16(buf: &mut Vec<u8>, v: i16) {
        buf.extend_from_slice(&v.to_be_bytes());
    }

    fn push_u32(buf: &mut Vec<u8>, v: u32) {
        buf.extend_from_slice(&v.to_be_bytes());
    }

    /// Build a minimal valid OpenType/TrueType SFNT binary with `head`, `hhea`, `cmap` (Format 4),
    /// `hmtx`, and `kern` (Format 0) tables for deterministic glyph advance width testing.
    fn build_synthetic_sfnt() -> Vec<u8> {
        // Glyphs: 0 = .notdef (500), 1 = 'A' (650), 2 = 'V' (650), 3 = 'i' (250), 4 = 'W' (920)
        let mut head = vec![0u8; 54];
        head[18..20].copy_from_slice(&1000_u16.to_be_bytes()); // unitsPerEm = 1000

        let mut hhea = vec![0u8; 36];
        hhea[34..36].copy_from_slice(&5_u16.to_be_bytes()); // numberOfHMetrics = 5

        let mut hmtx = Vec::new();
        for adv in [500_u16, 650, 650, 250, 920] {
            push_u16(&mut hmtx, adv);
            push_i16(&mut hmtx, 0);
        }

        // cmap Format 4 with 5 segments: 'A'(65)->1, 'V'(86)->2, 'W'(87)->4, 'i'(105)->3, 0xFFFF->0
        let segs: [(u16, u16, i16); 5] = [(65, 65, -64), (86, 86, -84), (87, 87, -83), (105, 105, -102), (0xFFFF, 0xFFFF, 1)];
        let mut sub4 = Vec::new();
        push_u16(&mut sub4, 4); // format
        push_u16(&mut sub4, 14 + 5 * 2 * 4 + 2); // length
        push_u16(&mut sub4, 0); // language
        push_u16(&mut sub4, 10); // segCountX2 = 5 * 2
        push_u16(&mut sub4, 8);
        push_u16(&mut sub4, 2);
        push_u16(&mut sub4, 2);
        for (_, end_c, _) in segs {
            push_u16(&mut sub4, end_c);
        }
        push_u16(&mut sub4, 0); // reservedPad
        for (start_c, _, _) in segs {
            push_u16(&mut sub4, start_c);
        }
        for (_, _, delta) in segs {
            push_i16(&mut sub4, delta);
        }
        for _ in segs {
            push_u16(&mut sub4, 0); // idRangeOffset = 0
        }

        let mut cmap = Vec::new();
        push_u16(&mut cmap, 0);
        push_u16(&mut cmap, 1);
        push_u16(&mut cmap, 3); // Windows
        push_u16(&mut cmap, 1); // Unicode BMP
        push_u32(&mut cmap, 12);
        cmap.extend_from_slice(&sub4);

        // kern Format 0: pair ('A'=1, 'V'=2) -> -80 FUnits
        let mut kern = Vec::new();
        push_u16(&mut kern, 0); // version
        push_u16(&mut kern, 1); // nTables
        push_u16(&mut kern, 0); // subtable version
        push_u16(&mut kern, 20); // length = 14 + 6
        push_u16(&mut kern, 0x0001); // horizontal format 0
        push_u16(&mut kern, 1); // nPairs
        push_u16(&mut kern, 6);
        push_u16(&mut kern, 0);
        push_u16(&mut kern, 0);
        push_u16(&mut kern, 1); // left = 'A'
        push_u16(&mut kern, 2); // right = 'V'
        push_i16(&mut kern, -80);

        let tables: [(&[u8; 4], &[u8]); 5] = [(b"cmap", &cmap), (b"head", &head), (b"hhea", &hhea), (b"hmtx", &hmtx), (b"kern", &kern)];
        let mut sfnt = Vec::new();
        push_u32(&mut sfnt, 0x0001_0000);
        push_u16(&mut sfnt, tables.len() as u16);
        push_u16(&mut sfnt, 64);
        push_u16(&mut sfnt, 2);
        push_u16(&mut sfnt, 16);

        let mut offset = 12 + tables.len() * 16;
        let mut payloads = Vec::new();
        for (tag, data) in tables {
            sfnt.extend_from_slice(tag);
            push_u32(&mut sfnt, 0);
            push_u32(&mut sfnt, offset as u32);
            push_u32(&mut sfnt, data.len() as u32);
            payloads.extend_from_slice(data);
            while payloads.len() % 4 != 0 {
                payloads.push(0);
            }
            offset = 12 + tables.len() * 16 + payloads.len();
        }
        sfnt.extend_from_slice(&payloads);
        sfnt
    }

    #[test]
    fn opentype_hmtx_cmap_and_kern_advance_widths_are_exact() {
        let sfnt = build_synthetic_sfnt();
        let w_narrow = measure_opentype_advance("iiii", &sfnt, 0, 100.0).unwrap();
        let w_wide = measure_opentype_advance("WWWW", &sfnt, 0, 100.0).unwrap();
        assert!((w_narrow - 100.0).abs() < 1e-6, "4 * 250/1000 * 100px = 100px, got {w_narrow}");
        assert!((w_wide - 368.0).abs() < 1e-6, "4 * 920/1000 * 100px = 368px, got {w_wide}");

        let w_aa = measure_opentype_advance("AA", &sfnt, 0, 100.0).unwrap();
        let w_av = measure_opentype_advance("AV", &sfnt, 0, 100.0).unwrap();
        assert!((w_aa - 130.0).abs() < 1e-6);
        assert!((w_av - 122.0).abs() < 1e-6, "650 + (650 - 80) = 1220/1000 * 100 = 122px, got {w_av}");
    }

    #[test]
    fn proportional_fallback_and_hero_collision_spacing_behave_accurately() {
        let narrow = measure_text_advance("iiii", "uninstalled-test-sans", 400, false, 80.0);
        let wide = measure_text_advance("WWWW", "uninstalled-test-sans", 400, false, 80.0);
        assert!(wide > narrow * 2.5, "expected wide glyphs ({wide}) >> narrow glyphs ({narrow})");

        let gap_plain = minimum_non_overlap_gap(
            TextSpanMetrics { width: 160.0, size: 60.0, is_hero: false, swell_ratio: 0.0 },
            TextSpanMetrics { width: 180.0, size: 60.0, is_hero: false, swell_ratio: 0.0 },
        );
        let gap_heroes = minimum_non_overlap_gap(
            TextSpanMetrics { width: 160.0, size: 96.0, is_hero: true, swell_ratio: 0.18 },
            TextSpanMetrics { width: 180.0, size: 96.0, is_hero: true, swell_ratio: 0.18 },
        );
        assert!(gap_heroes > gap_plain + 50.0, "hero collision gap ({gap_heroes}) must exceed plain gap ({gap_plain})");
    }
}
