//! Headless Unicode line segmentation (UAX #14 + kinsoku), grapheme-cluster emergency breaks
//! (UAX #29), bidirectional paragraph ordering (UAX #9), dictionary hyphenation ([`craft_text`]),
//! and canonical codepoint range encoding for [`storytold/font-preview-service`].

use std::collections::BTreeSet;

use craft_text::bidi::{Direction, Paragraph};
use craft_text::hyphen::{Lang, Limits, hyphen_points_in};
use craft_text::segment::{Kinsoku, Options, emergency_breaks, line_breaks};

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
