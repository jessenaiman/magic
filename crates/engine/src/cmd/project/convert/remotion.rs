//! `RemotionConverter`: inherits [`super::BaseConverter`] via [`super::LegacyConverter`] to
//! migrate Remotion React/TSX projects (`package.json`, `Root.tsx`, `<Composition>`, `staticFile()`,
//! `RIG_CONFIG`, `<LyricMotionText>`, `<Interactive.Svg>`) into self-contained `.magic` projects.

use std::path::Path;

use serde_json::{Value, json};

use super::{
    BaseConverter, ConversionPlan, ConvertKind, DiscoveredAsset, HardWallIssue, LegacyConverter, MissingAssetNotice, collect_files_filtered, extract_attr,
    extract_static_file_calls,
};
use crate::Result;

const REMOTION_DOCS: &[(&str, &str)] = &[
    ("Remotion Repository & README", "https://github.com/remotion-dev/remotion"),
    ("Remotion Documentation Home", "https://www.remotion.dev/docs"),
    ("Remotion <Composition> API", "https://www.remotion.dev/docs/composition"),
    ("Remotion <Sequence> & <Series>", "https://www.remotion.dev/docs/sequence"),
    ("Remotion staticFile() Asset Resolution", "https://www.remotion.dev/docs/staticfile"),
    ("Remotion useCurrentFrame() & interpolate()", "https://www.remotion.dev/docs/interpolate"),
    ("Remotion spring() Physics", "https://www.remotion.dev/docs/spring"),
    ("Remotion Interactive SVG & evolvePath()", "https://www.remotion.dev/docs/paths/evolve-path"),
];

pub struct RemotionConverter;

impl LegacyConverter for RemotionConverter {
    fn kind(&self) -> ConvertKind {
        ConvertKind::Remotion
    }

    fn detect(&self, source_dir: &Path) -> bool {
        for pkg in [source_dir.join("package.json"), source_dir.join("remotion/package.json")] {
            if let Ok(raw) = std::fs::read_to_string(pkg)
                && raw.contains("remotion")
            {
                return true;
            }
        }
        !collect_files_filtered(source_dir, &["tsx", "ts"])
            .into_iter()
            .filter(|p| {
                std::fs::read_to_string(p)
                    .map(|t| t.contains("@remotion/") || t.contains("from 'remotion'") || t.contains("from \"remotion\""))
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>()
            .is_empty()
    }

    fn extract_plan(&self, base: &BaseConverter, source_dir: &Path) -> Result<ConversionPlan> {
        let mut plan = base.new_empty_plan(source_dir, ConvertKind::Remotion);
        let rem_root = if source_dir.join("remotion/package.json").is_file() && !source_dir.join("package.json").is_file() {
            source_dir.join("remotion")
        } else {
            source_dir.to_path_buf()
        };

        if let Ok(pkg_str) = std::fs::read_to_string(rem_root.join("package.json"))
            && let Ok(pkg) = serde_json::from_str::<Value>(&pkg_str)
            && base.explicit_name.is_none()
            && let Some(name) = pkg["name"].as_str().filter(|s| !s.trim().is_empty())
        {
            plan.title = name.replace('-', " ");
        }

        let tsx_files = collect_files_filtered(&rem_root, &["tsx", "ts", "jsx", "js"]);
        for tsx in &tsx_files {
            if let Ok(code) = std::fs::read_to_string(tsx) {
                inspect_remotion_tsx(&rem_root, tsx, &code, &mut plan);
            }
        }

        collect_remotion_media(source_dir, &rem_root, &mut plan);
        Ok(plan)
    }
}

fn parse_numeric_prop(code: &str, prop: &str) -> Option<f64> {
    let needle = format!("{prop}={{");
    let idx = code.find(&needle)?;
    let rest = &code[idx + needle.len()..];
    let end = rest.find('}')?;
    rest[..end].trim().replace('_', "").parse::<f64>().ok()
}

fn inspect_remotion_tsx(rem_root: &Path, tsx_path: &Path, code: &str, plan: &mut ConversionPlan) {
    let fname = tsx_path.file_name().and_then(|n| n.to_str()).unwrap_or("Component.tsx");
    if code.contains("<Composition") {
        if let Some(w) = parse_numeric_prop(code, "width") {
            plan.width = w as u32;
        }
        if let Some(h) = parse_numeric_prop(code, "height") {
            plan.height = h as u32;
        }
        if let Some(fps) = parse_numeric_prop(code, "fps").filter(|&f| f > 0.0) {
            plan.fps = fps;
        }
        if let Some(frames) = parse_numeric_prop(code, "durationInFrames").filter(|&d| d > 0.0) {
            plan.duration_sec = Some(frames / plan.fps.max(1.0));
        }
        if let Some(cid) = extract_attr(code, "id") {
            plan.notes.push(format!("Detected Remotion <Composition id=\"{cid}\"> in {fname}"));
        }
    }

    for rel in extract_static_file_calls(code) {
        let public_cand = rem_root.join("public").join(rel.trim_start_matches('/'));
        let direct_cand = rem_root.join(rel.trim_start_matches('/'));
        if !public_cand.is_file() && !direct_cand.is_file() {
            let is_audio = matches!(Path::new(&rel).extension().and_then(|e| e.to_str()).unwrap_or_default(), "wav" | "mp3" | "m4a" | "flac" | "ogg");
            plan.missing_assets.push(MissingAssetNotice {
                referenced_path: format!("public/{rel}"),
                referenced_in: tsx_path.display().to_string(),
                kind_hint: if is_audio { "audio".into() } else { "staticFile_asset".into() },
                suggested_project_dest: if is_audio { format!("audio/{rel}") } else { format!("assets/images/{rel}") },
                resolution_step: format!(
                    concat!(
                        "Missing `staticFile(\"{rel}\")` in `{fname}`; ",
                        "save via `magic-cli exec project.save_asset path=<file>` (https://www.remotion.dev/docs/staticfile)"
                    ),
                    rel = rel,
                    fname = fname,
                ),
            });
        }
    }

    if code.contains("RIG_CONFIG") || fname.ends_with("_react_motion.tsx") {
        let char_id = fname.trim_end_matches("_react_motion.tsx").trim_end_matches(".tsx").to_owned();
        plan.character_rigs.push(json!({
            "character": char_id,
            "source_tsx": fname,
            "skeleton": "plush-9",
            "promote_to_magic_animations": true,
        }));
    }

    if code.contains("LyricMotionText") || code.contains("LYRIC_WORDS") {
        plan.notes.push(format!("Detected Remotion kinetic lyric component in `{fname}` -> target `lettering/lyrics.ecproj` (`V3`)"));
    }

    if code.contains("Interactive.Svg") || code.contains("evolvePath") {
        plan.motion_overlays.push(json!({
            "source_tsx": fname,
            "target_track": "V4",
            "effectcraft_equivalent": "shape.trimPaths",
        }));
    }

    if code.contains("poseAt(") || (code.contains("useCurrentFrame") && (code.contains("Math.sin") || code.contains("spring("))) {
        plan.hard_walls.push(HardWallIssue {
            code: "REMOTION_PROCEDURAL_FRAME_HOOK",
            summary: format!("Procedural React `useCurrentFrame()` / `spring()` / `poseAt()` math in `{fname}` requires EffectCraft keyframe translation"),
            source_file: tsx_path.display().to_string(),
            agent_steps: vec![
                format!("Read `{fname}` archived inside `<project>/magic/originals/` to inspect its `interpolate()` / `spring()` / `poseAt()` curves."),
                "Translate spring beats onto EffectCraft (`character.animate` on `V2` or `motion.add` on `V4`).".into(),
                "Verify sync on the FilmCraft timeline via `magic-cli exec app.doctor`.".into(),
            ],
            doc_links: vec![REMOTION_DOCS[5], REMOTION_DOCS[6], REMOTION_DOCS[1]],
        });
    }

    if code.contains("@remotion/three") || code.contains("ThreeCanvas") {
        plan.hard_walls.push(HardWallIssue {
            code: "REMOTION_THREE_CANVAS",
            summary: format!("WebGL `<ThreeCanvas>` (`@remotion/three`) in `{fname}` requires native `.glb` / TextureCraft or EffectCraft 3D layer handoff"),
            source_file: tsx_path.display().to_string(),
            agent_steps: vec![
                "Import `.glb`/`.gltf` models into `<project>/assets/3d/` via `magic-cli exec project.save_asset path=<model.glb> kind=model_3d`.".into(),
                "Render turntable/camera passes via TextureCraft or EffectCraft 3D layers and link onto FilmCraft.".into(),
            ],
            doc_links: vec![REMOTION_DOCS[0], REMOTION_DOCS[2]],
        });
    }
}

fn collect_remotion_media(source_dir: &Path, rem_root: &Path, plan: &mut ConversionPlan) {
    for audio in collect_files_filtered(source_dir, &["wav", "mp3", "flac", "m4a", "ogg"]) {
        if !plan.audio_candidates.contains(&audio) {
            plan.audio_candidates.push(audio);
        }
    }
    let public_dir = rem_root.join("public");
    let scan_root = if public_dir.is_dir() { public_dir.as_path() } else { source_dir };
    let exts = ["png", "jpg", "jpeg", "webp", "svg", "glb", "gltf", "pcraft", "psd", "vectorcraft"];
    for asset in collect_files_filtered(scan_root, &exts) {
        let lower = asset.to_string_lossy().to_ascii_lowercase();
        let is_rig = ["character", "part", "duke", "rig"].iter().any(|k| lower.contains(k));
        let (bucket, cid, promote) = if is_rig {
            let inferred_char = asset
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .filter(|&n| n != "parts" && n != "public" && n != "images")
                .map(str::to_owned);
            ("characters", inferred_char, true)
        } else if lower.contains("bg") || lower.contains("background") || lower.contains("stage") || lower.contains("world") {
            ("backgrounds", None, false)
        } else if asset.extension().is_some_and(|e| e.eq_ignore_ascii_case("glb") || e.eq_ignore_ascii_case("gltf")) {
            ("3d", None, false)
        } else if asset.extension().is_some_and(|e| e.eq_ignore_ascii_case("svg") || e.eq_ignore_ascii_case("vectorcraft")) {
            ("vectors", None, false)
        } else {
            ("images", None, false)
        };
        plan.discovered_assets.push(DiscoveredAsset { source_path: asset, bucket: bucket.into(), character_id: cid, promote_shared: promote });
    }
}
