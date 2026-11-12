//! Barnyard 16:9 Hierarchical Slide-Spread Storyboard generator (`960 × 540 pt` widescreen).
//!
//! Combines 100% of the fields and editorial card geometry from `references/storyboard/Storyboard.potx`
//! (`PRODUCTION / VIDEO INSTRUCTION`, `VIDEO PROMPT`, `CAST · PERFORMERS` Lead + Friends 1..3 + Ensemble,
//! `LOCATIONS 1..5`, `SONG MAP Phrase 1..4 / Action 1..4`, `MOVEMENT / CONTINUITY / EDITING`,
//! `SEQUENCE MAP Beat 1..5` + `MOVEMENT VOCABULARY: LEAD → ECHO → RETURN → REPEAT`, per-scene
//! `LYRIC CUE / TIMING / VISIBLE CAST / STORY BEAT / CONTINUITY / MOTION / CAPTION / ASSET`, and
//! `PRODUCTION HANDOFF: STORY SEQUENCE / IDENTITY / MOVEMENT / EDITING / NEXT REVIEW`) with a strict
//! **Sections → Section Cover / Title Pages → Sub-Pages** book hierarchy.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::{FieldState, PageInfo, Slot, field_key, load_field_state, slot_cli_command, slot_is_filled};
use crate::Session;

pub const BARNYARD_W: f64 = 960.0;
pub const BARNYARD_H: f64 = 540.0;

struct ProjectSeed {
    title: String,
    tagline: String,
    lyric_lines: Vec<String>,
    characters: Vec<(String, String, Option<String>)>,
    backgrounds: Vec<(String, String)>,
    assets: Vec<(String, String, Option<String>, Option<String>)>,
}

struct SectionCoverSpec<'a> {
    sec_num: &'a str,
    section: &'a str,
    kicker: &'a str,
    title_slot: (&'a str, &'a str),
    summary_slot: (&'a str, &'a str),
    image_slot_spec: (&'a str, Option<&'a str>, &'a str),
    accent_swatch: &'a str,
}

struct PageBuilder {
    index: usize,
    section: String,
    page_role: &'static str,
    sub_code: String,
    heading: String,
    slots: Vec<Slot>,
    layout_items: Vec<Value>,
    next_id: u64,
}

impl PageBuilder {
    fn new(index: usize, section: &str, page_role: &'static str, sub_code: &str, heading: &str) -> Self {
        Self {
            index,
            section: section.to_owned(),
            page_role,
            sub_code: sub_code.to_owned(),
            heading: heading.to_owned(),
            slots: Vec::new(),
            layout_items: Vec::new(),
            next_id: (index as u64 + 1) * 1000,
        }
    }

    fn shape(&mut self, bounds: [f64; 4], fill: &str, stroke: &str, stroke_weight: f64) {
        self.next_id += 1;
        self.layout_items.push(json!({
            "id": self.next_id,
            "kind": "shape",
            "locked": true,
            "bounds": bounds,
            "fill": fill,
            "stroke": stroke,
            "stroke_weight": stroke_weight,
        }));
    }

    fn label_chrome(&mut self, bounds: [f64; 4], text: &str, size: f64, color: &str) {
        self.next_id += 1;
        self.layout_items.push(json!({
            "id": self.next_id,
            "kind": "text",
            "locked": true,
            "bounds": bounds,
            "fill": "[None]",
            "stroke": "[None]",
            "stroke_weight": 0.0,
            "text": text,
            "font_size": size,
            "font_style": "Bold",
            "text_fill": color,
        }));
    }

    fn badge_pill(&mut self, bounds: [f64; 4], bg: &str, text: &str, text_col: &str) {
        self.shape(bounds, bg, "[None]", 0.0);
        self.label_chrome([bounds[0] + 5.0, bounds[1] + 2.0, bounds[2] - 5.0, bounds[3] - 1.0], text, 7.6, text_col);
    }

    fn text_slot(&mut self, state: &FieldState, label: &str, bounds: [f64; 4], default_text: &str) {
        self.next_id += 1;
        let fkey = field_key(self.index, label);
        let val = state.text.get(&fkey).or_else(|| state.text.get(label)).cloned().unwrap_or_else(|| default_text.to_owned());
        let filled = slot_is_filled("text", Some(&val), None);
        self.slots.push(Slot {
            label: label.to_owned(),
            form_name: fkey,
            id: self.next_id,
            kind: "text",
            text: Some(val),
            image: None,
            linked_file: None,
            asset_id: None,
            craft_app: Some("DeckCraft".into()),
            outline: None,
            time_sec: None,
            bounds,
            locked: false,
            filled,
            cli_command: slot_cli_command(self.index, label, "text"),
        });
    }

    fn image_slot(&mut self, state: &FieldState, base_dir: &Path, label: &str, bounds: [f64; 4], default_img: Option<&str>, craft_app: &str) {
        self.next_id += 1;
        let fkey = field_key(self.index, label);
        let stored = state.images.get(&fkey).or_else(|| state.images.get(label));
        let resolved_img = stored.map(|m| resolve_rel_path(base_dir, &m.image)).or_else(|| default_img.map(|d| resolve_rel_path(base_dir, d)));
        let linked = stored.and_then(|m| m.linked_file.as_deref().map(|l| resolve_rel_path(base_dir, l))).or_else(|| resolved_img.clone());
        let filled = slot_is_filled("image", None, resolved_img.as_deref());
        self.slots.push(Slot {
            label: label.to_owned(),
            form_name: fkey,
            id: self.next_id,
            kind: "image",
            text: None,
            image: resolved_img,
            linked_file: linked,
            asset_id: stored.and_then(|m| m.asset_id.clone()),
            craft_app: stored.and_then(|m| m.craft_app.clone()).or_else(|| Some(craft_app.to_owned())),
            outline: stored.and_then(|m| m.outline.clone()),
            time_sec: stored.and_then(|m| m.time_sec),
            bounds,
            locked: false,
            filled,
            cli_command: slot_cli_command(self.index, label, "image"),
        });
    }

    fn finish(self) -> PageInfo {
        PageInfo {
            index: self.index,
            spread_index: self.index,
            page_in_spread: 0,
            spread_page_count: 1,
            section: self.section,
            page_role: self.page_role,
            sub_code: self.sub_code,
            bounds: [0.0, 0.0, BARNYARD_W, BARNYARD_H],
            heading: Some(self.heading),
            slots: self.slots,
            layout_items: self.layout_items,
        }
    }
}

fn resolve_rel_path(base: &Path, raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() || Path::new(trimmed).is_absolute() {
        return trimmed.to_owned();
    }
    let cand = base.join(trimmed);
    if cand.exists() { cand.to_string_lossy().into_owned() } else { trimmed.to_owned() }
}

fn discover_characters(proj_dir: &Path) -> Vec<(String, String, Option<String>)> {
    let mut characters = Vec::new();
    let Ok(entries) = std::fs::read_dir(proj_dir.join("assets/characters")) else {
        return characters;
    };
    let mut dirs: Vec<PathBuf> = entries.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    dirs.sort();
    for cdir in dirs {
        let cid = cdir.file_name().and_then(|n| n.to_str()).unwrap_or("char").to_owned();
        let display = cid
            .split(['-', '_'])
            .filter(|w| !w.is_empty())
            .map(|w| {
                let mut ch = w.chars();
                ch.next().map_or_else(String::new, |f| format!("{}{}", f.to_ascii_uppercase(), ch.as_str()))
            })
            .collect::<Vec<_>>()
            .join(" ");
        let img = ["body.png", "cutout.png", "front.png", "preview.png"]
            .into_iter()
            .map(|f| cdir.join(f))
            .find(|p| p.is_file())
            .or_else(|| {
                std::fs::read_dir(&cdir)
                    .ok()?
                    .flatten()
                    .map(|e| e.path())
                    .find(|p| p.extension().and_then(|x| x.to_str()).is_some_and(|x| matches!(x, "png" | "jpg" | "webp")))
            })
            .map(|p| p.to_string_lossy().into_owned());
        characters.push((cid, display, img));
    }
    characters
}

fn discover_backgrounds(proj_dir: &Path) -> Vec<(String, String)> {
    let mut backgrounds = Vec::new();
    let Ok(entries) = std::fs::read_dir(proj_dir.join("assets/backgrounds")) else {
        return backgrounds;
    };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    paths.sort();
    for p in paths {
        let is_img = p.extension().and_then(|x| x.to_str()).is_some_and(|x| matches!(x, "png" | "jpg" | "jpeg" | "webp"));
        if p.is_file() && is_img {
            let name = p.file_stem().and_then(|n| n.to_str()).unwrap_or("Location").replace(['-', '_'], " ");
            backgrounds.push((name, p.to_string_lossy().into_owned()));
        } else if p.is_dir()
            && let Some(img) = std::fs::read_dir(&p)
                .ok()
                .into_iter()
                .flatten()
                .flatten()
                .map(|e| e.path())
                .find(|f| f.extension().and_then(|x| x.to_str()).is_some_and(|x| matches!(x, "png" | "jpg" | "webp")))
        {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("Location").replace(['-', '_'], " ");
            backgrounds.push((name, img.to_string_lossy().into_owned()));
        }
    }
    backgrounds
}

fn discover_lyric_lines(proj_dir: &Path) -> Vec<String> {
    let mut lyric_lines = Vec::new();
    let Ok(lyr) = crate::cmd::words::lyrics::song_lyrics_in(proj_dir) else {
        return lyric_lines;
    };
    if let Some(secs) = lyr["sections"].as_array() {
        for sec in secs {
            for line in sec["lines"].as_array().into_iter().flatten() {
                if let Some(t) = line["text"].as_str().map(str::trim).filter(|t| !t.is_empty()) {
                    lyric_lines.push(t.to_owned());
                }
            }
        }
    }
    if lyric_lines.is_empty()
        && let Some(raw) = lyr["lyrics"].as_str()
    {
        lyric_lines.extend(raw.lines().map(str::trim).filter(|l| !l.is_empty()).map(str::to_owned));
    }
    lyric_lines
}

fn discover_project_seed(s: Option<&Session>, doc_path: &Path, state: &FieldState) -> ProjectSeed {
    let proj_dir = s.and_then(|sess| sess.project_path.as_deref().and_then(Path::parent)).or_else(|| doc_path.parent()).unwrap_or_else(|| Path::new("."));

    let default_title = s
        .map(|sess| sess.song.title.trim())
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
        .or_else(|| state.text.get("p00_cover_title").cloned())
        .or_else(|| proj_dir.file_name().and_then(|n| n.to_str()).map(|slug| slug.replace('-', " ")))
        .unwrap_or_else(|| "Song Storyboard".into());

    let mut assets = Vec::new();
    if let Some(sess) = s {
        for a in sess.project_assets.iter().take(6) {
            if let Some(ref apath) = a.path {
                let (proj_img, linked, app) = crate::cmd::project::assets::resolve_asset_project_image(Some(proj_dir), apath);
                assets.push((a.name.clone(), proj_img.to_string_lossy().into_owned(), linked, app));
            }
        }
    }

    ProjectSeed {
        title: default_title,
        tagline: "16:9 Storybook Production Bible · Visual Direction, Cast, Locations & Timed Scene Beats".into(),
        lyric_lines: discover_lyric_lines(proj_dir),
        characters: discover_characters(proj_dir),
        backgrounds: discover_backgrounds(proj_dir),
        assets,
    }
}

/// Build a dedicated Editorial Section Cover / Title Page (`Sections → Title Pages → Sub-Pages`).
fn build_section_cover_page(index: usize, spec: SectionCoverSpec<'_>, state: &FieldState, base_dir: &Path) -> PageBuilder {
    let mut b = PageBuilder::new(index, spec.section, "section_title", spec.sec_num, spec.title_slot.1);
    b.shape([0.0, 0.0, BARNYARD_W, 8.0], spec.accent_swatch, "[None]", 0.0);
    b.shape([24.0, 26.0, 468.0, 514.0], "WarmCard", "Rule", 1.0);
    b.badge_pill([42.0, 44.0, 232.0, 64.0], spec.accent_swatch, spec.kicker, "[Paper]");
    b.label_chrome([42.0, 72.0, 148.0, 142.0], spec.sec_num, 44.0, spec.accent_swatch);
    b.label_chrome([42.0, 152.0, 220.0, 168.0], "SECTION TITLE", 8.0, "Pencil");
    b.text_slot(state, spec.title_slot.0, [42.0, 170.0, 448.0, 226.0], spec.title_slot.1);
    b.label_chrome([42.0, 238.0, 260.0, 254.0], "PRODUCTION SCOPE & VISUAL DIRECTION", 8.0, "Pencil");
    b.text_slot(state, spec.summary_slot.0, [42.0, 258.0, 448.0, 434.0], spec.summary_slot.1);
    b.shape([42.0, 448.0, 448.0, 496.0], "Petrol", "[None]", 0.0);
    b.label_chrome([54.0, 456.0, 210.0, 470.0], "SECTION CONTINUITY", 7.5, "Sky");
    b.text_slot(
        state,
        &format!("{}_edit_line", spec.section.to_ascii_lowercase()),
        [54.0, 472.0, 438.0, 492.0],
        "Locked master palette & proportions carry into every sub-page below.",
    );

    b.shape([486.0, 26.0, 936.0, 514.0], "[Paper]", "Rule", 1.0);
    b.label_chrome([502.0, 38.0, 760.0, 54.0], "HERO REFERENCE PLATE", 8.0, spec.accent_swatch);
    b.image_slot(state, base_dir, spec.image_slot_spec.0, [502.0, 58.0, 920.0, 498.0], spec.image_slot_spec.1, spec.image_slot_spec.2);
    b
}

/// Slide 1A (`Cover` — Barnyard `PRODUCTION / VIDEO INSTRUCTION`, `VIDEO PROMPT`, `CAST · PERFORMERS`, `MOVEMENT / CONTINUITY / EDITING`).
fn build_barnyard_master_cover(index: usize, seed: &ProjectSeed, state: &FieldState, base_dir: &Path) -> PageInfo {
    let mut b = PageBuilder::new(index, "Cover", "cover", "01", &format!("{} · Production Bible", seed.title));
    b.shape([0.0, 0.0, BARNYARD_W, 8.0], "Rust", "[None]", 0.0);
    b.label_chrome([24.0, 18.0, 560.0, 34.0], "01 · PRODUCTION / VIDEO INSTRUCTION", 8.5, "Rust");
    b.badge_pill([780.0, 20.0, 936.0, 42.0], "Petrol", "16:9 MASTER BIBLE", "[Paper]");
    b.text_slot(state, "cover_title", [24.0, 36.0, 760.0, 72.0], &seed.title);
    b.text_slot(state, "cover_tagline", [24.0, 74.0, 760.0, 96.0], &seed.tagline);

    // Left Card: VIDEO PROMPT + Key Art
    b.shape([24.0, 106.0, 356.0, 438.0], "WarmCard", "Rule", 1.0);
    b.badge_pill([36.0, 116.0, 168.0, 134.0], "Rust", "VIDEO PROMPT", "[Paper]");
    b.text_slot(
        state,
        "cover_overall_design",
        [36.0, 140.0, 344.0, 256.0],
        "Hand-crafted storybook illustration with warm paper grain, expressive cutout silhouettes, crisp ink linework, and natural theatrical lighting.",
    );
    b.label_chrome([36.0, 264.0, 220.0, 278.0], "MASTER KEY ART", 7.8, "Petrol");
    let hero_bg = seed.backgrounds.first().map(|(_, p)| p.as_str());
    b.image_slot(state, base_dir, "cover_key_art", [36.0, 282.0, 344.0, 426.0], hero_bg, "PhotoCraft");

    // Right Card: CAST · PERFORMERS (Lead + Friends 1..3 + Ensemble 5..9 — 100% of Barnyard Slide 1 Cast!)
    b.shape([370.0, 106.0, 936.0, 438.0], "WarmCard", "Rule", 1.0);
    b.badge_pill([382.0, 116.0, 520.0, 134.0], "Petrol", "CAST · PERFORMERS", "[Paper]");
    let cast_slots = [
        ("lead", "lead_role", "lead_portrait", "Lead", "Lead Vocal / Hero"),
        ("friend_1", "friend_role_1", "friend_portrait_1", "Friend 1", "Supporting Cast"),
        ("friend_2", "friend_role_2", "friend_portrait_2", "Friend 2", "Chorus Companion"),
        ("friend_3", "friend_role_3", "friend_portrait_3", "Friend 3", "Ensemble Feature"),
    ];
    for (ci, (n_lbl, r_lbl, p_lbl, def_n, def_r)) in cast_slots.into_iter().enumerate() {
        let x0 = 382.0 + (ci as f64) * 136.0;
        let x1 = x0 + 128.0;
        b.shape([x0, 142.0, x1, 326.0], "[Paper]", "Rule", 1.0);
        let seed_char = seed.characters.get(ci);
        let c_img = seed_char.and_then(|c| c.2.as_deref());
        let c_name = seed_char.map_or(def_n, |c| c.1.as_str());
        b.image_slot(state, base_dir, p_lbl, [x0 + 8.0, 150.0, x1 - 8.0, 268.0], c_img, "PhotoCraft");
        b.text_slot(state, n_lbl, [x0 + 6.0, 272.0, x1 - 6.0, 296.0], c_name);
        b.text_slot(state, r_lbl, [x0 + 6.0, 298.0, x1 - 6.0, 320.0], def_r);
    }

    // Barnyard Ensemble Roster Strip (Slots 5..9)
    b.label_chrome([382.0, 336.0, 620.0, 350.0], "ENSEMBLE & SUPPORTING ROSTER (SLOTS 5–9)", 7.6, "Petrol");
    for ei in 0..5usize {
        let slot_num = ei + 5;
        let x0 = 382.0 + (ei as f64) * 109.0;
        let x1 = x0 + 103.0;
        b.shape([x0, 354.0, x1, 426.0], "[Paper]", "Rule", 1.0);
        let def_ename = seed.characters.get(slot_num - 1).map_or_else(|| format!("Cast {slot_num}"), |c| c.1.clone());
        b.text_slot(state, &format!("cast_{slot_num}_name"), [x0 + 5.0, 360.0, x1 - 5.0, 390.0], &def_ename);
        b.text_slot(state, &format!("cast_{slot_num}_role"), [x0 + 5.0, 392.0, x1 - 5.0, 420.0], "Ensemble Beat");
    }

    // Footer Petrol Directives Bar: MOVEMENT · CONTINUITY · EDITING
    b.shape([24.0, 448.0, 936.0, 522.0], "Petrol", "[None]", 0.0);
    b.badge_pill([36.0, 456.0, 128.0, 472.0], "Sky", "MOVEMENT", "Ink");
    b.text_slot(state, "movement", [36.0, 476.0, 318.0, 514.0], "Playful beat-locked bounce, natural arcs, gentle camera drift.");
    b.badge_pill([332.0, 456.0, 432.0, 472.0], "Sun", "CONTINUITY", "Ink");
    b.text_slot(state, "identity", [332.0, 476.0, 622.0, 514.0], "Preserve exact character silhouettes, proportions, and palette across shots.");
    b.badge_pill([636.0, 456.0, 726.0, 472.0], "Rose", "EDITING", "Ink");
    b.text_slot(state, "editing", [636.0, 476.0, 924.0, 514.0], "Cut on musical phrase downbeats; keep C1 captions clear of hero action.");

    b.finish()
}

/// Slide 1B (`Cover` Sub-Page `1.1` — Barnyard `LOCATIONS 1..5` & `SONG MAP Phrase 1..4 / Action 1..4`).
fn build_barnyard_locations_and_song_map(index: usize, seed: &ProjectSeed, state: &FieldState, base_dir: &Path) -> PageInfo {
    let mut b = PageBuilder::new(index, "Cover", "subpage", "1.1", "Locations & Song Map");
    b.shape([0.0, 0.0, BARNYARD_W, 8.0], "Petrol", "[None]", 0.0);
    b.label_chrome([24.0, 18.0, 480.0, 34.0], "1.1 · MASTER LOCATIONS & SONG PHRASE MAP", 8.5, "Petrol");

    b.badge_pill([24.0, 40.0, 148.0, 58.0], "Petrol", "LOCATIONS (1–5)", "[Paper]");
    let loc_badges = [("Rust", "START + END"), ("Sun", "LOC 02"), ("Sky", "LOC 03"), ("Rose", "LOC 04"), ("Moss", "FINALE")];
    for (li, loc_badge) in loc_badges.iter().enumerate() {
        let n = li + 1;
        let x0 = 24.0 + (li as f64) * 184.0;
        let x1 = x0 + 176.0;
        b.shape([x0, 64.0, x1, 254.0], "WarmCard", "Rule", 1.0);
        b.badge_pill([x0 + 8.0, 70.0, x0 + 96.0, 86.0], loc_badge.0, loc_badge.1, "[Paper]");
        let bg_entry = seed.backgrounds.get(li).or_else(|| seed.backgrounds.first());
        let def_loc_name = bg_entry.map_or_else(|| format!("Stage Location {n}"), |(nm, _)| nm.clone());
        let def_loc_img = bg_entry.map(|(_, p)| p.as_str());
        b.image_slot(state, base_dir, &format!("place_art_{n}"), [x0 + 8.0, 90.0, x1 - 8.0, 212.0], def_loc_img, "PhotoCraft");
        b.text_slot(state, &format!("location_{n}"), [x0 + 8.0, 216.0, x1 - 8.0, 248.0], &def_loc_name);
    }

    b.badge_pill([24.0, 266.0, 156.0, 284.0], "Petrol", "SONG MAP (1–4)", "[Paper]");
    let verse_accents = ["Sun", "Sky", "Rose", "Rust"];
    for (vi, verse_accent) in verse_accents.iter().enumerate() {
        let n = vi + 1;
        let x0 = 24.0 + (vi as f64) * 230.0;
        let x1 = x0 + 222.0;
        b.shape([x0, 290.0, x1, 518.0], "WarmCard", "Rule", 1.0);
        b.badge_pill([x0 + 10.0, 298.0, x0 + 86.0, 316.0], verse_accent, &format!("PHRASE {n}"), "[Paper]");
        let def_phrase = seed.lyric_lines.get(vi).cloned().unwrap_or_else(|| format!("Sung lyric phrase {n} establishes the beat."));
        b.label_chrome([x0 + 10.0, 322.0, x1 - 10.0, 336.0], "LYRIC PHRASE", 7.4, "Pencil");
        b.text_slot(state, &format!("phrase_{n}"), [x0 + 10.0, 338.0, x1 - 10.0, 418.0], &def_phrase);
        b.label_chrome([x0 + 10.0, 424.0, x1 - 10.0, 438.0], "SONG ACTION & STAGING", 7.4, "Petrol");
        b.text_slot(state, &format!("action_{n}"), [x0 + 10.0, 440.0, x1 - 10.0, 510.0], &format!("Cast performs beat {n} gesture synced to vocal rhythm."));
    }

    b.finish()
}

/// Section 2 Sub-Page (`2.1` — Master Location Staging & Prompt Sheet).
fn build_world_location_subpage(index: usize, seed: &ProjectSeed, state: &FieldState, base_dir: &Path) -> PageInfo {
    let mut b = PageBuilder::new(index, "Backgrounds", "subpage", "2.1", "Master Location Plates & Lighting");
    b.shape([0.0, 0.0, BARNYARD_W, 8.0], "Petrol", "[None]", 0.0);
    b.label_chrome([24.0, 18.0, 520.0, 34.0], "2.1 · LOCATION PLATES, DEPTH LAYERS & LIGHTING KEY", 8.5, "Petrol");

    let bg1 = seed.backgrounds.first().map(|(_, p)| p.as_str());
    let bg2 = seed.backgrounds.get(1).or(seed.backgrounds.first()).map(|(_, p)| p.as_str());
    b.shape([24.0, 44.0, 588.0, 344.0], "[Paper]", "Rule", 1.0);
    b.image_slot(state, base_dir, "world_primary_plate", [34.0, 54.0, 578.0, 334.0], bg1, "PhotoCraft");

    b.shape([24.0, 354.0, 588.0, 518.0], "WarmCard", "Rule", 1.0);
    b.badge_pill([36.0, 364.0, 184.0, 382.0], "Petrol", "WORLD PROMPT", "[Paper]");
    b.text_slot(
        state,
        "world_location_prompt",
        [36.0, 388.0, 576.0, 508.0],
        "Layered foreground, midground stage plane, and painted sky backdrop separated cleanly for 2.5D parallax.",
    );

    b.shape([602.0, 44.0, 936.0, 260.0], "[Paper]", "Rule", 1.0);
    b.label_chrome([614.0, 52.0, 860.0, 66.0], "SECONDARY ENVIRONMENT PLATE", 7.6, "Petrol");
    b.image_slot(state, base_dir, "world_secondary_plate", [614.0, 70.0, 924.0, 248.0], bg2, "PhotoCraft");

    b.shape([602.0, 270.0, 936.0, 518.0], "WarmCard", "Rule", 1.0);
    b.label_chrome([614.0, 282.0, 880.0, 296.0], "LIGHTING & HORIZON CONTINUITY", 7.8, "Rust");
    b.text_slot(
        state,
        "world_lighting_notes",
        [614.0, 300.0, 924.0, 404.0],
        "Warm key light from upper-left (120°); ground shadow softness matched across character cutouts.",
    );
    b.label_chrome([614.0, 414.0, 880.0, 428.0], "CAMERA & PARALLAX PLANES", 7.8, "Petrol");
    b.text_slot(state, "world_camera_staging", [614.0, 432.0, 924.0, 508.0], "Keep center 60% clear for hero cast performance and V3 kinetic lyrics.");
    b.finish()
}

/// Section 3 Per-Character Sub-Pages (`3.1..3.N`).
fn build_character_subpage(
    index: usize,
    sub_idx: usize,
    char_info: Option<&(String, String, Option<String>)>,
    state: &FieldState,
    base_dir: &Path,
) -> PageInfo {
    let (_, cname, cimg) = match char_info {
        Some(tup) => (tup.0.as_str(), tup.1.as_str(), tup.2.as_deref()),
        None => ("lead", "Lead Performer", None),
    };
    let code = format!("3.{sub_idx}");
    let mut b = PageBuilder::new(index, "Characters", "subpage", &code, &format!("{cname} · Character Sheet"));
    b.shape([0.0, 0.0, BARNYARD_W, 8.0], "Sun", "[None]", 0.0);
    b.label_chrome([24.0, 18.0, 560.0, 34.0], &format!("{code} · CHARACTER MODEL, RIG & IDENTITY SHEET"), 8.5, "Rust");
    b.text_slot(state, "char_heading", [24.0, 36.0, 620.0, 68.0], cname);

    b.shape([24.0, 78.0, 312.0, 516.0], "[Paper]", "Rule", 1.0);
    b.badge_pill([36.0, 88.0, 206.0, 106.0], "Petrol", "PHOTOCRAFT CUTOUT", "[Paper]");
    b.image_slot(state, base_dir, "char_original_image", [36.0, 112.0, 300.0, 502.0], cimg, "PhotoCraft");

    b.shape([326.0, 78.0, 614.0, 516.0], "[Paper]", "Rule", 1.0);
    b.badge_pill([338.0, 88.0, 508.0, 106.0], "Moss", "EFFECTCRAFT POSE RIG", "[Paper]");
    b.image_slot(state, base_dir, "char_pose_image", [338.0, 112.0, 602.0, 502.0], cimg, "EffectCraft");

    b.shape([628.0, 78.0, 936.0, 516.0], "WarmCard", "Rule", 1.0);
    b.badge_pill([640.0, 88.0, 796.0, 106.0], "Rust", "IDENTITY PROMPT", "[Paper]");
    b.text_slot(
        state,
        "char_exact_prompt",
        [640.0, 114.0, 924.0, 268.0],
        &format!("{cname}: consistent silhouette, clean articulated limbs for 2.5D cutout animation, expressive eyes and mouth visemes."),
    );
    b.label_chrome([640.0, 280.0, 900.0, 294.0], "PERFORMER ROLE & EXPRESSIONS", 7.8, "Petrol");
    b.text_slot(state, "char_subheading", [640.0, 298.0, 924.0, 392.0], "Idle breathe · Sing visemes · Wave · Jump / Fly cycle.");
    b.label_chrome([640.0, 404.0, 900.0, 418.0], "RIG & LAYER CONTINUITY", 7.8, "Pencil");
    b.text_slot(state, "char_rig_notes", [640.0, 422.0, 924.0, 504.0], "Pivot pins locked at shoulder, hip, head, and jaw; transparent RGBA8 bounds.");

    b.finish()
}

/// Slide 2 (`Scenes` Section Title Page `04` — Barnyard `SEQUENCE MAP` Beat 1..5 + `MOVEMENT VOCABULARY: LEAD → ECHO → RETURN → REPEAT`).
fn build_barnyard_sequence_map_cover(index: usize, seed: &ProjectSeed, state: &FieldState, base_dir: &Path) -> PageInfo {
    let mut b = PageBuilder::new(index, "Scenes", "section_title", "04", "04 · Sequence Map & Movement Vocabulary");
    b.shape([0.0, 0.0, BARNYARD_W, 8.0], "Petrol", "[None]", 0.0);
    b.badge_pill([24.0, 18.0, 178.0, 36.0], "Petrol", "04 · SEQUENCE MAP", "[Paper]");
    b.text_slot(state, "sequence_title", [24.0, 40.0, 720.0, 72.0], &format!("{} — Story & Beat Sequence", seed.title));
    b.text_slot(
        state,
        "sequence_order",
        [24.0, 74.0, 880.0, 96.0],
        "Opening Establish → Call & Introduce → Rising Action → Chorus Climax → Gentle Return Home",
    );

    let beat_colors = ["Rust", "Sun", "Sky", "Rose", "Moss"];
    let beat_defaults = ["Beat 1 · Intro", "Beat 2 · Verse", "Beat 3 · Turn", "Beat 4 · Climax", "Beat 5 · Finale"];
    for bi in 0..5usize {
        let n = bi + 1;
        let x0 = 24.0 + (bi as f64) * 184.0;
        let x1 = x0 + 170.0;
        b.shape([x0, 106.0, x1, 336.0], "WarmCard", "Rule", 1.0);
        let bg_img = seed.backgrounds.get(bi % seed.backgrounds.len().max(1)).map(|(_, p)| p.as_str());
        b.image_slot(state, base_dir, &format!("art_{n}"), [x0 + 7.0, 114.0, x1 - 7.0, 212.0], bg_img, "FilmCraft");
        b.shape([x0 + 7.0, 218.0, x1 - 7.0, 244.0], beat_colors[bi], "[None]", 0.0);
        b.text_slot(state, &format!("beat_{n}"), [x0 + 10.0, 221.0, x1 - 10.0, 242.0], beat_defaults[bi]);
        let def_act = seed.lyric_lines.get(bi).cloned().unwrap_or_else(|| format!("Visual action & staging for beat {n}."));
        b.text_slot(state, &format!("seq_action_{n}"), [x0 + 8.0, 250.0, x1 - 8.0, 328.0], &def_act);

        if bi < 4 {
            b.label_chrome([x1 + 2.0, 206.0, x1 + 14.0, 226.0], "→", 13.0, "Petrol");
        }
    }

    b.shape([24.0, 348.0, 936.0, 520.0], "Petrol", "[None]", 0.0);
    b.badge_pill([38.0, 358.0, 238.0, 378.0], "Sky", "MOVEMENT VOCABULARY", "Ink");
    let vocab = [
        ("vocab_lead", "LEAD", "Hero initiates gesture on beat 1"),
        ("vocab_echo", "ECHO", "Friends mirror motion on beat 3"),
        ("vocab_return", "RETURN", "Settle cleanly to neutral pose"),
        ("vocab_repeat", "REPEAT", "Chorus callback with bigger arc"),
    ];
    for (vi, (vkey, vhead, vdef)) in vocab.into_iter().enumerate() {
        let x0 = 38.0 + (vi as f64) * 224.0;
        let x1 = x0 + 206.0;
        b.shape([x0, 388.0, x1, 506.0], "[Paper]", "Rule", 1.0);
        b.badge_pill([x0 + 8.0, 396.0, x0 + 92.0, 414.0], "Petrol", vhead, "[Paper]");
        b.text_slot(state, vkey, [x0 + 8.0, 420.0, x1 - 8.0, 498.0], vdef);
        if vi < 3 {
            b.label_chrome([x1 + 3.0, 436.0, x1 + 17.0, 456.0], "→", 13.0, "Sky");
        }
    }

    b.finish()
}

/// Slide 3 (`Scenes` Sub-Pages `SC-01..SC-NN` — Barnyard Scene Beat Slide with ALL Barnyard fields!).
fn build_barnyard_scene_beat_subpage(
    index: usize,
    scene_num: usize,
    custom_slide: Option<&Value>,
    seed: &ProjectSeed,
    state: &FieldState,
    base_dir: &Path,
) -> PageInfo {
    let sc_code = custom_slide.and_then(|s| s["shot_id"].as_str()).map(str::to_owned).unwrap_or_else(|| format!("SC-{scene_num:02}"));
    let def_lyric = custom_slide
        .and_then(|s| s["lyric_cue"].as_str())
        .map(str::to_owned)
        .or_else(|| seed.lyric_lines.get(scene_num - 1).cloned())
        .unwrap_or_else(|| format!("Sung lyric line for scene {scene_num}"));
    let def_title = custom_slide.and_then(|s| s["title"].as_str()).map(str::to_owned).unwrap_or_else(|| {
        let snippet: String = def_lyric.chars().take(38).collect();
        format!("{sc_code} · {snippet}")
    });
    let start_s = ((scene_num - 1) as f64) * 6.5;
    let end_s = start_s + 6.5;
    let def_timing = format!("0:{start_s:04.1} – 0:{end_s:04.1} (6.5s hold)");
    let def_cast = custom_slide.and_then(|s| s["visible_cast"].as_str()).map(str::to_owned).unwrap_or_else(|| {
        if seed.characters.is_empty() { "Lead + Supporting Cast".into() } else { seed.characters.iter().map(|c| c.1.as_str()).collect::<Vec<_>>().join(", ") }
    });
    let def_img = custom_slide
        .and_then(|s| s["main_image"].as_str())
        .or_else(|| seed.backgrounds.get((scene_num - 1) % seed.backgrounds.len().max(1)).map(|(_, p)| p.as_str()));

    let mut b = PageBuilder::new(index, "Scenes", "subpage", &sc_code, &def_title);
    b.shape([0.0, 0.0, BARNYARD_W, 8.0], "Rust", "[None]", 0.0);
    b.label_chrome([24.0, 16.0, 360.0, 30.0], &format!("SCENE BEAT · {sc_code}"), 8.2, "Rust");
    b.text_slot(state, "title", [24.0, 32.0, 760.0, 62.0], &def_title);
    b.shape([776.0, 18.0, 936.0, 54.0], "Rust", "[None]", 0.0);
    b.text_slot(state, "scene_id", [784.0, 24.0, 928.0, 48.0], &sc_code);

    b.shape([24.0, 68.0, 616.0, 424.0], "[Paper]", "Rule", 1.0);
    b.image_slot(state, base_dir, "scene_image", [32.0, 76.0, 608.0, 332.0], def_img, "FilmCraft");

    let c1_img = seed.characters.first().and_then(|c| c.2.as_deref());
    let c2_img = seed.characters.get(1).and_then(|c| c.2.as_deref());
    b.image_slot(state, base_dir, "character_model_1", [32.0, 340.0, 116.0, 416.0], c1_img, "EffectCraft");
    b.image_slot(state, base_dir, "character_model_2", [122.0, 340.0, 206.0, 416.0], c2_img, "EffectCraft");
    b.label_chrome([216.0, 340.0, 420.0, 354.0], "LOCATION & VISUAL PROMPT", 7.5, "Petrol");
    let def_loc = custom_slide
        .and_then(|s| s["scene_location"].as_str())
        .or_else(|| seed.backgrounds.first().map(|(n, _)| n.as_str()))
        .unwrap_or("Master Stage Location");
    b.text_slot(state, "scene_location", [216.0, 356.0, 606.0, 378.0], def_loc);
    b.text_slot(
        state,
        "prompt",
        [216.0, 380.0, 606.0, 418.0],
        custom_slide.and_then(|s| s["prompt"].as_str()).unwrap_or("Wide 16:9 storybook framing with clear center silhouette staging."),
    );

    b.shape([628.0, 68.0, 936.0, 424.0], "WarmCard", "Rule", 1.0);
    b.shape([640.0, 78.0, 924.0, 104.0], "Rust", "[None]", 0.0);
    b.text_slot(state, "status", [648.0, 81.0, 916.0, 101.0], custom_slide.and_then(|s| s["status"].as_str()).unwrap_or("READY · IDENTITY LOCKED"));

    b.label_chrome([640.0, 112.0, 880.0, 126.0], "LYRIC CUE", 7.6, "Rust");
    b.text_slot(state, "lyric_cue", [640.0, 128.0, 924.0, 178.0], &def_lyric);

    b.label_chrome([640.0, 184.0, 880.0, 198.0], "TIMING", 7.6, "Rust");
    b.text_slot(state, "timing", [640.0, 200.0, 924.0, 224.0], &def_timing);

    b.label_chrome([640.0, 230.0, 880.0, 244.0], "VISIBLE CAST", 7.6, "Rust");
    b.text_slot(state, "visible_cast", [640.0, 246.0, 924.0, 276.0], &def_cast);

    b.label_chrome([640.0, 282.0, 880.0, 296.0], "STORY BEAT", 7.6, "Rust");
    b.text_slot(state, "story_beat", [640.0, 298.0, 924.0, 352.0], custom_slide.and_then(|s| s["story_beat"].as_str()).unwrap_or(&def_lyric));

    b.label_chrome([640.0, 358.0, 880.0, 372.0], "CONTINUITY", 7.6, "Rust");
    b.text_slot(
        state,
        "continuity",
        [640.0, 374.0, 924.0, 418.0],
        custom_slide.and_then(|s| s["continuity_lock"].as_str()).unwrap_or("Match cast scale, lighting direction, and prop placement with adjacent scenes."),
    );

    b.shape([24.0, 434.0, 936.0, 524.0], "Petrol", "[None]", 0.0);
    b.badge_pill([36.0, 442.0, 122.0, 458.0], "Sky", "MOTION", "Ink");
    b.text_slot(
        state,
        "motion",
        [36.0, 462.0, 456.0, 494.0],
        custom_slide.and_then(|s| s["motion"].as_str()).unwrap_or("Gentle camera push-in + beat-synced character bounce (V2/V4)."),
    );
    b.badge_pill([474.0, 442.0, 566.0, 458.0], "Sky", "CAPTION", "Ink");
    b.text_slot(
        state,
        "caption",
        [474.0, 462.0, 924.0, 494.0],
        custom_slide.and_then(|s| s["editable_caption"].as_str()).filter(|c| !c.is_empty()).unwrap_or(&def_lyric),
    );
    b.text_slot(
        state,
        "asset",
        [36.0, 498.0, 924.0, 518.0],
        "Linked Craft Files: assets/backgrounds (.pcraft) · assets/characters (.ecproj) · lettering/lyrics.ecproj (V3)",
    );

    b.finish()
}

/// Section 5 (`Assets` Section Title & 6-Card Linked Craft Asset Matrix).
fn build_assets_section_page(index: usize, seed: &ProjectSeed, state: &FieldState, base_dir: &Path) -> PageInfo {
    let mut b = PageBuilder::new(index, "Assets", "section_title", "05", "05 · Craft Assets & Prop Library");
    b.shape([0.0, 0.0, BARNYARD_W, 8.0], "Sun", "[None]", 0.0);
    b.badge_pill([24.0, 18.0, 212.0, 36.0], "Petrol", "05 · CRAFT ASSET MATRIX", "[Paper]");
    b.text_slot(state, "assets_heading", [24.0, 40.0, 620.0, 70.0], "Linked PhotoCraft, VectorCraft & EffectCraft Assets");
    b.text_slot(state, "assets_group", [630.0, 44.0, 936.0, 68.0], "Right-click any slot to open its native Craft file");

    for ai in 0..6usize {
        let n = ai + 1;
        let col = ai % 3;
        let row = ai / 3;
        let x0 = 24.0 + (col as f64) * 308.0;
        let y0 = 82.0 + (row as f64) * 220.0;
        let x1 = x0 + 296.0;
        let y1 = y0 + 208.0;
        b.shape([x0, y0, x1, y1], "WarmCard", "Rule", 1.0);
        let item = seed.assets.get(ai);
        let def_img = item.map(|x| x.1.as_str()).or_else(|| seed.backgrounds.first().map(|(_, p)| p.as_str()));
        let def_name = item.map_or_else(|| format!("Project Asset {n}"), |x| x.0.clone());
        let def_app = item.and_then(|x| x.3.as_deref()).unwrap_or("PhotoCraft");
        b.image_slot(state, base_dir, &format!("assets_thumb_{n}"), [x0 + 10.0, y0 + 10.0, x1 - 10.0, y0 + 148.0], def_img, def_app);
        b.text_slot(state, &format!("assets_name_{n}"), [x0 + 10.0, y0 + 152.0, x1 - 10.0, y0 + 176.0], &def_name);
        b.text_slot(state, &format!("assets_details_{n}"), [x0 + 10.0, y0 + 178.0, x1 - 10.0, y0 + 202.0], &format!("{def_app} · Linked project asset"));
    }

    b.finish()
}

/// Slide 4 (`Final` Section Cover `06` — Barnyard `PRODUCTION HANDOFF`: `STORY SEQUENCE`, `IDENTITY`, `MOVEMENT`, `EDITING`, `NEXT REVIEW`).
fn build_barnyard_production_handoff(index: usize, seed: &ProjectSeed, state: &FieldState, base_dir: &Path) -> PageInfo {
    let mut b = PageBuilder::new(index, "Final", "section_title", "06", "06 · Production Handoff & Sign-Off");
    b.shape([0.0, 0.0, BARNYARD_W, 8.0], "Moss", "[None]", 0.0);
    b.badge_pill([24.0, 18.0, 214.0, 36.0], "Moss", "06 · PRODUCTION HANDOFF", "[Paper]");
    b.text_slot(state, "final_heading", [24.0, 40.0, 740.0, 72.0], &format!("{} — Studio Finishing & Handoff", seed.title));

    let handoff_cards = [
        ("sequence", "01 · STORY SEQUENCE", "All sections, cover sheets, and timed scene beats verified against canonical lyrics."),
        ("cast", "02 · IDENTITY", "Character cutouts and EffectCraft rigs consistent across every scene beat."),
        ("handoff_motion", "03 · MOVEMENT", "V2 Character Animation, V3 Kinetic Lyrics, and V4 Action Motion aligned to Tick clock."),
        ("edit", "04 · EDITING", "FilmCraft master sequence assembled with broadcast-safe C1 captions and -14 LUFS mix."),
    ];
    for (hi, (hkey, hlabel, hdef)) in handoff_cards.into_iter().enumerate() {
        let y0 = 84.0 + (hi as f64) * 100.0;
        let y1 = y0 + 92.0;
        b.shape([24.0, y0, 496.0, y1], "WarmCard", "Rule", 1.0);
        b.badge_pill([36.0, y0 + 8.0, 212.0, y0 + 26.0], "Moss", hlabel, "[Paper]");
        b.text_slot(state, hkey, [36.0, y0 + 30.0, 484.0, y1 - 8.0], hdef);
    }

    let hero_art = seed.backgrounds.first().map(|(_, p)| p.as_str());
    b.shape([512.0, 84.0, 936.0, 318.0], "[Paper]", "Rule", 1.0);
    b.image_slot(state, base_dir, "final_still_1", [522.0, 94.0, 926.0, 308.0], hero_art, "FilmCraft");

    b.shape([512.0, 328.0, 936.0, 476.0], "Petrol", "[None]", 0.0);
    b.badge_pill([526.0, 338.0, 662.0, 356.0], "Sky", "NEXT REVIEW", "Ink");
    b.text_slot(
        state,
        "review",
        [526.0, 362.0, 922.0, 424.0],
        "Review assembled V1–V4 tracks in FilmCraft Studio, verify lip-sync & lyric legibility, then export MP4 / ProRes master.",
    );
    b.text_slot(
        state,
        "final_deliverables",
        [526.0, 428.0, 922.0, 468.0],
        "Deliverables: film/*.fcproj · lettering/lyrics.ecproj · motion/actions.ecproj · exports/*.mp4",
    );

    b.shape([24.0, 488.0, 936.0, 524.0], "WarmCard", "Rule", 1.0);
    b.label_chrome([36.0, 498.0, 148.0, 514.0], "HANDOFF STATUS", 7.8, "Moss");
    b.text_slot(state, "final_credits", [156.0, 494.0, 924.0, 518.0], "Production Bible Ready · Linked to Live ArtCraft Suite");

    b.finish()
}

/// Assemble the complete Barnyard Hierarchical Storyboard (`Sections → Cover / Title Pages → Sub-Pages`).
pub(crate) fn build_barnyard_hierarchical_pages(s: Option<&Session>, doc_path: &Path) -> Vec<PageInfo> {
    let base_dir = doc_path.parent().unwrap_or_else(|| Path::new("."));
    let state = load_field_state(doc_path);
    let seed = discover_project_seed(s, doc_path, &state);

    let sj_path = super::slides::slides_json_path(doc_path);
    let custom_slides: Vec<Value> = std::fs::read_to_string(&sj_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|v| v["slides"].as_array().cloned())
        .unwrap_or_default();

    let mut pages = Vec::with_capacity(18);

    // Section 1: Cover (Master Cover Page `01` + Locations & Song Map Sub-Page `1.1`)
    pages.push(build_barnyard_master_cover(pages.len(), &seed, &state, base_dir));
    pages.push(build_barnyard_locations_and_song_map(pages.len(), &seed, &state, base_dir));

    // Section 2: Backgrounds (Section Cover `02` + Master Location Sub-Page `2.1`)
    let bg_hero = seed.backgrounds.first().map(|(_, p)| p.as_str());
    pages.push(
        build_section_cover_page(
            pages.len(),
            SectionCoverSpec {
                sec_num: "02",
                section: "Backgrounds",
                kicker: "SECTION COVER · WORLD",
                title_slot: ("world_heading", "World, Stage & Master Locations"),
                summary_slot: (
                    "world_subheading",
                    "Establishes the storybook horizon, color temperature, painted textures, and layered parallax planes shared across every scene.",
                ),
                image_slot_spec: ("world_stage_image", bg_hero, "PhotoCraft"),
                accent_swatch: "Petrol",
            },
            &state,
            base_dir,
        )
        .finish(),
    );
    pages.push(build_world_location_subpage(pages.len(), &seed, &state, base_dir));

    // Section 3: Characters (Section Cover `03` + Per-Character Sub-Pages `3.1..3.N`)
    let char_hero = seed.characters.first().and_then(|c| c.2.as_deref()).or(bg_hero);
    pages.push(
        build_section_cover_page(
            pages.len(),
            SectionCoverSpec {
                sec_num: "03",
                section: "Characters",
                kicker: "SECTION COVER · CAST",
                title_slot: ("character_cover_heading", "Cast Roster & Rigged Performers"),
                summary_slot: (
                    "character_cover_note",
                    "Every lead and supporting character maintains locked cutout proportions, viseme mouth shapes, and EffectCraft puppet pins.",
                ),
                image_slot_spec: ("character_thumb_1", char_hero, "PhotoCraft"),
                accent_swatch: "Sun",
            },
            &state,
            base_dir,
        )
        .finish(),
    );
    let char_count = seed.characters.len().clamp(1, 6);
    for ci in 0..char_count {
        pages.push(build_character_subpage(pages.len(), ci + 1, seed.characters.get(ci), &state, base_dir));
    }

    // Section 4: Scenes (Barnyard Slide 2 `SEQUENCE MAP` Cover `04` + Barnyard Slide 3 `Scene Beat` Sub-Pages `SC-01..SC-NN`)
    pages.push(build_barnyard_sequence_map_cover(pages.len(), &seed, &state, base_dir));
    let scene_count = if !custom_slides.is_empty() { custom_slides.len() } else { seed.lyric_lines.len().clamp(4, 12) };
    for si in 0..scene_count {
        pages.push(build_barnyard_scene_beat_subpage(pages.len(), si + 1, custom_slides.get(si), &seed, &state, base_dir));
    }

    // Section 5: Assets (Section Cover + 6-Card Linked Craft Asset Matrix `05`)
    pages.push(build_assets_section_page(pages.len(), &seed, &state, base_dir));

    // Section 6: Final (Barnyard Slide 4 `PRODUCTION HANDOFF` Cover `06`)
    pages.push(build_barnyard_production_handoff(pages.len(), &seed, &state, base_dir));

    pages
}
