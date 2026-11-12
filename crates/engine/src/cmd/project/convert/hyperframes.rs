//! `HyperFramesConverter`: inherits [`super::BaseConverter`] via [`super::LegacyConverter`] to
//! migrate HeyGen HyperFrames HTML/GSAP compositions (`hyperframes.json`, `index.html`,
//! `data-composition-id`, `.clip`, `.lyric-word-clip`, `.part-pivot`, `film-data.js`) into `.magic`.

use std::path::Path;

use serde_json::{Value, json};

use super::{
    BaseConverter, ConversionPlan, ConvertKind, DiscoveredAsset, HardWallIssue, LegacyConverter,
    MissingAssetNotice, collect_files_filtered, extract_attr, extract_quoted_paths,
};
use crate::Result;

const HF_DOCS: &[(&str, &str)] = &[
    ("HyperFrames Repository & README", "https://github.com/heygen-com/hyperframes/blob/main/README.md"),
    ("HyperFrames HTML Schema Reference", "https://hyperframes.heygen.com/reference/html-schema"),
    ("HyperFrames Compositions & Sub-Comps", "https://hyperframes.heygen.com/concepts/compositions"),
    ("HyperFrames Data Attributes Timing", "https://hyperframes.heygen.com/concepts/data-attributes"),
    ("HyperFrames GSAP Animation Guide", "https://hyperframes.heygen.com/guides/gsap-animation"),
    ("HyperFrames Frame Sources (`window.__hf`)", "https://hyperframes.heygen.com/guides/frame-sources"),
    ("HyperFrames Voice, Audio & Captions", "https://hyperframes.heygen.com/guides/voice-and-audio"),
    ("HyperFrames Captions & Recuts", "https://hyperframes.heygen.com/guides/captions-and-recuts"),
];

pub struct HyperFramesConverter;

impl LegacyConverter for HyperFramesConverter {
    fn kind(&self) -> ConvertKind {
        ConvertKind::HyperFrames
    }

    fn detect(&self, source_dir: &Path) -> bool {
        if source_dir.join("hyperframes.json").is_file() || source_dir.join("project/hyperframes.json").is_file() {
            return true;
        }
        for cand in [source_dir.join("index.html"), source_dir.join("project/index.html")] {
            if let Ok(html) = std::fs::read_to_string(cand)
                && (html.contains("data-composition-id") || html.contains("window.__hf"))
            {
                return true;
            }
        }
        false
    }

    fn extract_plan(&self, base: &BaseConverter, source_dir: &Path) -> Result<ConversionPlan> {
        let mut plan = base.new_empty_plan(source_dir, ConvertKind::HyperFrames);
        let hf_root = if source_dir.join("project/index.html").is_file() && !source_dir.join("index.html").is_file() {
            source_dir.join("project")
        } else {
            source_dir.to_path_buf()
        };

        if let Ok(cfg_str) = std::fs::read_to_string(hf_root.join("hyperframes.json"))
            && let Ok(cfg) = serde_json::from_str::<Value>(&cfg_str)
        {
            if let Some(fps) = cfg["fps"].as_f64().filter(|&v| v > 0.0) {
                plan.fps = fps;
            }
            if let Some(name) = cfg["name"].as_str().filter(|s| !s.trim().is_empty())
                && base.explicit_name.is_none()
            {
                plan.title = name.trim().to_owned();
            }
        }

        let html_path = hf_root.join("index.html");
        if let Ok(html) = std::fs::read_to_string(&html_path) {
            parse_hyperframes_html(&hf_root, &html_path, &html, &mut plan);
        }

        scan_hyperframes_scripts(&hf_root, &mut plan);
        discover_local_media(source_dir, &mut plan);
        Ok(plan)
    }
}

fn parse_hyperframes_html(hf_root: &Path, html_path: &Path, html: &str, plan: &mut ConversionPlan) {
    if let Some(w) = extract_attr(html, "data-width").and_then(|v| v.parse::<u32>().ok()) {
        plan.width = w;
    }
    if let Some(h) = extract_attr(html, "data-height").and_then(|v| v.parse::<u32>().ok()) {
        plan.height = h;
    }
    if let Some(fps) = extract_attr(html, "data-fps").and_then(|v| v.parse::<f64>().ok()).filter(|&f| f > 0.0) {
        plan.fps = fps;
    }
    if let Some(dur) = extract_attr(html, "data-duration").and_then(|v| v.parse::<f64>().ok()).filter(|&d| d > 0.0) {
        plan.duration_sec = Some(dur);
    }

    for rel in extract_quoted_paths(html, &["wav", "mp3", "m4a", "flac", "ogg", "png", "jpg", "jpeg", "webp", "svg", "mp4", "webm"]) {
        if rel.starts_with("http://") || rel.starts_with("https://") {
            plan.hard_walls.push(HardWallIssue {
                code: "HF_EXTERNAL_CDN_ASSET",
                summary: format!("Remote URL asset `{rel}` violates HyperFrames/Magic offline determinism"),
                source_file: html_path.display().to_string(),
                agent_steps: vec![
                    format!("Download or generate a local replacement for `{rel}` and save it via `magic-cli exec project.save_asset path=<local_file>`."),
                    "Re-link the local asset onto the Storyboard or EffectCraft composition.".into(),
                ],
                doc_links: vec![
                    ("HyperFrames Determinism & Lint Rules", "https://hyperframes.heygen.com/reference/lint-rules"),
                    ("HyperFrames HTML Schema", "https://hyperframes.heygen.com/reference/html-schema"),
                ],
            });
            continue;
        }
        let resolved = hf_root.join(rel.trim_start_matches("./"));
        if !resolved.is_file() {
            let is_audio = matches!(
                Path::new(&rel).extension().and_then(|e| e.to_str()).unwrap_or_default(),
                "wav" | "mp3" | "m4a" | "flac" | "ogg"
            );
            plan.missing_assets.push(MissingAssetNotice {
                referenced_path: rel.clone(),
                referenced_in: html_path.display().to_string(),
                kind_hint: if is_audio { "audio".into() } else { "image_or_media".into() },
                suggested_project_dest: if is_audio { format!("audio/{rel}") } else { format!("assets/images/{rel}") },
                resolution_step: if is_audio {
                    format!("Locate song audio `{rel}` and attach via `magic-cli exec workshop.set_audio path=<audio_file>`")
                } else {
                    format!("Locate or recreate `{rel}` and save into `<project>/assets/` via `magic-cli exec project.save_asset path=<file> save=true`")
                },
            });
        }
    }

    extract_hyperframes_lyric_words(html, plan);

    if html.contains("data-file=") {
        plan.hard_walls.push(HardWallIssue {
            code: "HF_SUB_COMPOSITION_DATA_FILE",
            summary: "HyperFrames nested sub-composition (`data-file` clip) requires separate EffectCraft pre-comp conversion".into(),
            source_file: html_path.display().to_string(),
            agent_steps: vec![
                "Inspect each `data-file=\"...\"` sub-composition archived inside `magic/originals/`.".into(),
                concat!(
                    "Build a matching EffectCraft composition (`effectcraft-cli mcp`) ",
                    "and link it onto FilmCraft (`magic-cli exec video.link_effectcraft project= <comp.ecproj>`)."
                ).into(),
            ],
            doc_links: vec![
                ("HyperFrames Compositions & Sub-Compositions", "https://hyperframes.heygen.com/concepts/compositions"),
                ("HyperFrames Data Attributes", "https://hyperframes.heygen.com/concepts/data-attributes"),
            ],
        });
    }
    if html.contains("stroke-dashoffset") {
        plan.motion_overlays.push(json!({
            "source": "hyperframes_svg_stroke_dashoffset",
            "target_track": "V4",
            "effectcraft_equivalent": "shape.trimPaths",
        }));
    }
}

fn extract_hyperframes_lyric_words(html: &str, plan: &mut ConversionPlan) {
    for line in html.lines() {
        if !line.contains("lyric-word") && !line.contains("data-word") {
            continue;
        }
        let start = extract_attr(line, "data-start").and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
        let dur = extract_attr(line, "data-duration").and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.35);
        let word = extract_attr(line, "data-word").or_else(|| {
            let gt = line.find('>')?;
            let lt = line[gt + 1..].find('<')?;
            let text = line[gt + 1..gt + 1 + lt].trim();
            (!text.is_empty()).then(|| text.to_owned())
        });
        if let Some(w) = word {
            plan.timed_words.push(json!({
                "word": w,
                "start": start,
                "end": start + dur,
            }));
        }
    }
}

fn scan_hyperframes_scripts(hf_root: &Path, plan: &mut ConversionPlan) {
    let js_files = collect_files_filtered(hf_root, &["js", "mjs", "html"]);
    for js_path in js_files {
        let Ok(code) = std::fs::read_to_string(&js_path) else {
            continue;
        };
        if code.contains("window.__hf") && code.contains("renderAt") {
            plan.hard_walls.push(HardWallIssue {
                code: "HF_CUSTOM_RENDER_AT_CALLBACK",
                summary: format!(
                    "Procedural `window.__hf.renderAt(t)` JS callback in `{}` requires EffectCraft keyframe sampling",
                    js_path.file_name().and_then(|n| n.to_str()).unwrap_or("script.js")
                ),
                source_file: js_path.display().to_string(),
                agent_steps: vec![
                    format!(
                        "Inspect `{}` inside `<project>/magic/originals/` for `renderAt(t)` transforms/pivots.",
                        js_path.file_name().and_then(|n| n.to_str()).unwrap_or("script.js")
                    ),
                    "Translate joint poses onto EffectCraft (`V2` `character.*` rig or `V4` `motion.*` keyframes) at integer `Tick` times.".into(),
                    "Link the resulting `.ecproj` onto FilmCraft via `magic-cli exec video.link_effectcraft project= animations/<rig>.ecproj --track V2`.".into(),
                ],
                doc_links: vec![
                    HF_DOCS[5],
                    HF_DOCS[4],
                    HF_DOCS[0],
                ],
            });
        }
        if code.contains("MorphSVGPlugin") || code.contains("SplitText") || code.contains("CustomBounce") {
            plan.hard_walls.push(HardWallIssue {
                code: "HF_GSAP_CLUB_PLUGIN",
                summary: "GSAP plugin (`MorphSVG` / `SplitText` / `CustomBounce`) requires native EffectCraft text animator or shape morph setup".into(),
                source_file: js_path.display().to_string(),
                agent_steps: vec![
                    "Use `words.*` (`lettering/lyrics.ecproj`) range selectors for per-character/per-word kinetic lyrics instead of GSAP `SplitText`.".into(),
                    "Use VectorCraft/EffectCraft path keyframes for SVG shape morphs.".into(),
                ],
                doc_links: vec![HF_DOCS[4], HF_DOCS[7]],
            });
        }
        if code.contains("part-pivot") || code.contains("PIVOTS") || code.contains("cutout") {
            plan.character_rigs.push(json!({
                "source_script": js_path.file_name().and_then(|n| n.to_str()).unwrap_or("cutout.js"),
                "style": "cutout_2_5d",
                "note": "Extracted HyperFrames DOM pivot rig candidate; verify joint pivots in `assets/characters/<id>/rig.json`.",
            }));
        }
    }
}

fn discover_local_media(source_dir: &Path, plan: &mut ConversionPlan) {
    for audio in collect_files_filtered(source_dir, &["wav", "mp3", "flac", "m4a", "ogg"]) {
        if !plan.audio_candidates.contains(&audio) {
            plan.audio_candidates.push(audio);
        }
    }
    for img in collect_files_filtered(source_dir, &["png", "jpg", "jpeg", "webp", "svg", "pcraft", "psd", "vectorcraft"]) {
        let lower = img.to_string_lossy().to_ascii_lowercase();
        let bucket = if lower.contains("char") || lower.contains("part") || lower.contains("puppet") {
            "characters"
        } else if lower.contains("bg") || lower.contains("background") || lower.contains("scene") || lower.contains("stage") {
            "backgrounds"
        } else if img.extension().is_some_and(|e| e.eq_ignore_ascii_case("svg") || e.eq_ignore_ascii_case("vectorcraft")) {
            "vectors"
        } else {
            "images"
        };
        plan.discovered_assets.push(DiscoveredAsset {
            source_path: img,
            bucket: bucket.into(),
            character_id: None,
            promote_shared: false,
        });
    }
}
