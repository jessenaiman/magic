//! `CustomConverter`: inherits [`super::BaseConverter`] via [`super::LegacyConverter`] to migrate
//! earlier FilmCraft storytelling projects, loose media folders, or hybrid drafts into the canonical
//! `magic/projects/<slug>/` workspace with a working FilmCraft project and empty/seeded `storyboard.pdf`.

use std::path::Path;

use serde_json::{Value, json};

use super::{BaseConverter, ConversionPlan, ConvertKind, DiscoveredAsset, HardWallIssue, LegacyConverter, MissingAssetNotice, collect_files_filtered};
use crate::Result;

pub struct CustomConverter;

impl LegacyConverter for CustomConverter {
    fn kind(&self) -> ConvertKind {
        ConvertKind::Custom
    }

    fn detect(&self, source_dir: &Path) -> bool {
        source_dir.is_dir()
    }

    fn extract_plan(&self, base: &BaseConverter, source_dir: &Path) -> Result<ConversionPlan> {
        let mut plan = base.new_empty_plan(source_dir, ConvertKind::Custom);

        let existing_magic = collect_files_filtered(source_dir, &["magic"]);
        if let Some(mpath) = existing_magic.first()
            && let Ok(txt) = std::fs::read_to_string(mpath)
            && let Ok(doc) = serde_json::from_str::<Value>(&txt)
        {
            if base.explicit_name.is_none()
                && let Some(n) = doc["name"].as_str().filter(|s| !s.trim().is_empty())
            {
                plan.title = n.trim().to_owned();
            }
            for asset in doc["assets"].as_array().into_iter().flatten() {
                if let Some(rel) = asset["path"].as_str() {
                    let cand = mpath.parent().unwrap_or(source_dir).join(rel);
                    if !cand.exists() {
                        plan.missing_assets.push(MissingAssetNotice {
                            referenced_path: rel.to_owned(),
                            referenced_in: mpath.display().to_string(),
                            kind_hint: asset["kind"].as_str().unwrap_or("asset").to_owned(),
                            suggested_project_dest: format!("assets/images/{}", Path::new(rel).file_name().and_then(|n| n.to_str()).unwrap_or("asset.png")),
                            // TODO(MAGIC-REVIEW): CHANGE missing-asset recovery to verified imports; this message is not a completed import.
                            resolution_step: format!(
                                concat!(
                                    "Legacy `.magic` referenced missing custom asset `{rel}`; ",
                                    "save via `magic-cli exec project.save_asset path=<file>`"
                                ),
                                rel = rel
                            ),
                        });
                    }
                }
            }
        }

        let fcprojs = collect_files_filtered(source_dir, &["fcproj"]);
        if let Some(fc) = fcprojs.first() {
            plan.existing_fcproj = Some(fc.clone());
            plan.notes
                .push(format!("Linked existing FilmCraft timeline `{}` into `film/`", fc.file_name().and_then(|n| n.to_str()).unwrap_or("project.fcproj")));
        }

        for audio in collect_files_filtered(source_dir, &["wav", "mp3", "flac", "m4a", "ogg"]) {
            plan.audio_candidates.push(audio);
        }
        if plan.audio_candidates.is_empty() {
            plan.missing_assets.push(MissingAssetNotice {
                referenced_path: "audio/<song_or_narration>.wav".into(),
                referenced_in: source_dir.display().to_string(),
                kind_hint: "audio".into(),
                suggested_project_dest: "audio/master.wav".into(),
                resolution_step: concat!(
                    "No master audio file was found inside this custom folder; ",
                    "attach via `magic-cli exec workshop.set_audio path=<audio.wav>`."
                )
                .into(),
            });
        }

        for retired in collect_files_filtered(source_dir, &["blend"]) {
            let ext = retired.extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase();
            plan.hard_walls.push(HardWallIssue {
                code: "CUSTOM_RETIRED_PIPELINE_ARTIFACT",
                summary: format!(
                    "Retired `{ext}` artifact (`{}`) archived to `magic/originals/`",
                    retired.file_name().and_then(|n| n.to_str()).unwrap_or("legacy")
                ),
                source_file: retired.display().to_string(),
                agent_steps: vec![if ext == "blend" {
                    concat!(
                        "Export 3D geometry to `.glb` and import into `<project>/assets/3d/` ",
                        "(`project.save_asset kind=model_3d`) instead of Blender scripts."
                    )
                    .into()
                } else {
                    concat!(
                        "Populate printable `storyboard.pdf` via `magic-cli exec storyboard.wizard` ",
                        "and `storyboard.set` instead of legacy slide decks."
                    )
                    .into()
                }],
                doc_links: vec![
                    ("FilmCraft Control Protocol", "https://github.com/storytold/filmcraft/blob/main/docs/control-protocol.md"),
                    ("EffectCraft Control Protocol", "https://github.com/storytold/effectcraft/blob/main/docs/control-protocol.md"),
                ],
            });
        }

        for ec in collect_files_filtered(source_dir, &["ecproj", "ectemplate"]) {
            plan.discovered_assets.push(DiscoveredAsset { source_path: ec, bucket: "animations".into(), character_id: None, promote_shared: false });
        }

        for rig_json in collect_files_filtered(source_dir, &["json"]) {
            let fname = rig_json.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            if fname == "rig.json" || fname == "parts.json" {
                let cid = rig_json.parent().and_then(|p| p.file_name()).and_then(|n| n.to_str()).unwrap_or("character").to_owned();
                plan.character_rigs.push(json!({
                    "character": cid,
                    "source_manifest": rig_json,
                    "promote_to_magic_animations": true,
                }));
            }
        }

        let asset_exts = ["png", "jpg", "jpeg", "webp", "svg", "glb", "gltf", "pcraft", "psd", "vectorcraft", "emf", "wmf", "dxf"];
        for asset in collect_files_filtered(source_dir, &asset_exts) {
            let lower = asset.to_string_lossy().to_ascii_lowercase();
            if lower.contains("frames/") || lower.contains(".previews/") {
                continue;
            }
            let is_char = ["character", "moppet", "crt", "vcr", "parts/"].iter().any(|k| lower.contains(k));
            let (bucket, cid, promote) = if is_char {
                let inferred = asset
                    .parent()
                    .and_then(|p| if p.ends_with("parts") { p.parent() } else { Some(p) })
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .filter(|&n| n != "characters" && n != "assets")
                    .map(str::to_owned);
                ("characters", inferred, true)
            } else if lower.contains("bg") || lower.contains("background") || lower.contains("world") || lower.contains("scene") {
                ("backgrounds", None, false)
            } else if asset.extension().is_some_and(|e| e.eq_ignore_ascii_case("glb") || e.eq_ignore_ascii_case("gltf")) {
                ("3d", None, false)
            } else if asset.extension().is_some_and(|e| matches!(e.to_str().unwrap_or_default(), "svg" | "vectorcraft" | "emf" | "wmf" | "dxf")) {
                ("vectors", None, false)
            } else {
                ("images", None, false)
            };
            plan.discovered_assets.push(DiscoveredAsset { source_path: asset, bucket: bucket.into(), character_id: cid, promote_shared: promote });
        }
        Ok(plan)
    }
}
