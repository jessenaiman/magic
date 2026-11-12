// TODO(MAGIC-REVIEW): CHANGE only as needed for the verified integration; this revision applies compiler/lint cleanup, not feature completion.
//! Base project conversion pipeline (`BaseConverter` + `LegacyConverter` trait) shared by all
//! three migration modes:
//! 1. [`hyperframes::HyperFramesConverter`] (`convert.kind = "hyperframes"`)
//! 2. [`remotion::RemotionConverter`] (`convert.kind = "remotion"`)
//! 3. [`custom::CustomConverter`] (`convert.kind = "custom"`)
//!
//! Guarantees for every converted project:
//! - Creates/maintains the enclosing `magic/` workspace (`magic/projects`, `magic/assets`, `magic/animations`).
//! - Creates a dedicated `<magic_root>/projects/<slug>/` folder with self-contained `<slug>/assets/`.
//! - Preserves and compresses the original project inside `<slug>/magic/originals/<slug>-legacy.tar.gz`
//!   tagged `"legacy"` in `legacy-manifest.json` and `<Title>.magic`.
//! - Copies discovered audio/assets into `<slug>/audio/` and `<slug>/assets/`, initializes a working
//!   FilmCraft `.fcproj` timeline + printable `storyboard.pdf`, records any missing custom assets in
//!   `.magic` (`missing_assets`), and outputs a structured terminal remediation playbook with verified
//!   official HyperFrames and Remotion documentation URLs whenever a hard wall is encountered.

pub mod archive;
pub mod custom;
pub mod hyperframes;
pub mod remotion;

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::workspace_layout::{MagicWorkspaceLayout, ensure_magic_workspace, promote_asset_to_workspace, resolve_magic_root};
use super::{copy_audio_into_project, project_slug, read_project_document, reset_for_project, validate_project_name, write_project_file};
use crate::time::TICKS_PER_SECOND;
use crate::{EngineError, ProjectAsset, Result, Session};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConvertKind {
    HyperFrames,
    Remotion,
    Custom,
}

impl ConvertKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::HyperFrames => "hyperframes",
            Self::Remotion => "remotion",
            Self::Custom => "custom",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HardWallIssue {
    pub code: &'static str,
    pub summary: String,
    pub source_file: String,
    pub agent_steps: Vec<String>,
    pub doc_links: Vec<(&'static str, &'static str)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MissingAssetNotice {
    pub referenced_path: String,
    pub referenced_in: String,
    pub kind_hint: String,
    pub suggested_project_dest: String,
    pub resolution_step: String,
}

#[derive(Clone, Debug)]
pub struct DiscoveredAsset {
    pub source_path: PathBuf,
    pub bucket: String,
    pub character_id: Option<String>,
    pub promote_shared: bool,
}

#[derive(Clone, Debug)]
pub struct ConversionPlan {
    pub kind: ConvertKind,
    pub title: String,
    pub fps: f64,
    pub width: u32,
    pub height: u32,
    pub duration_sec: Option<f64>,
    pub audio_candidates: Vec<PathBuf>,
    pub existing_fcproj: Option<PathBuf>,
    pub discovered_assets: Vec<DiscoveredAsset>,
    pub character_rigs: Vec<Value>,
    pub timed_words: Vec<Value>,
    pub motion_overlays: Vec<Value>,
    pub missing_assets: Vec<MissingAssetNotice>,
    pub hard_walls: Vec<HardWallIssue>,
    pub notes: Vec<String>,
}

/// Trait implemented by all 3 concrete project converters (`HyperFrames`, `Remotion`, `Custom`),
/// delegating workspace creation, legacy archiving, asset copying, FilmCraft/Storyboard seeding,
/// and terminal hard-wall reporting to [`BaseConverter`].
pub trait LegacyConverter {
    fn kind(&self) -> ConvertKind;
    fn detect(&self, source_dir: &Path) -> bool;
    fn extract_plan(&self, base: &BaseConverter, source_dir: &Path) -> Result<ConversionPlan>;
}

pub struct BaseConverter {
    pub magic_root: PathBuf,
    pub explicit_name: Option<String>,
    pub overwrite: bool,
    pub emit_terminal: bool,
}

impl BaseConverter {
    pub fn new(magic_root: PathBuf, explicit_name: Option<String>, overwrite: bool, emit_terminal: bool) -> Self {
        Self { magic_root, explicit_name, overwrite, emit_terminal }
    }

    pub fn new_empty_plan(&self, source_dir: &Path, kind: ConvertKind) -> ConversionPlan {
        let default_title = self
            .explicit_name
            .clone()
            .or_else(|| source_dir.file_name().and_then(|n| n.to_str()).map(|s| s.replace(['-', '_'], " ")))
            .unwrap_or_else(|| "Converted Project".into());
        ConversionPlan {
            kind,
            title: default_title,
            fps: 24.0,
            width: 1920,
            height: 1080,
            duration_sec: None,
            audio_candidates: Vec::new(),
            existing_fcproj: None,
            discovered_assets: Vec::new(),
            character_rigs: Vec::new(),
            timed_words: Vec::new(),
            motion_overlays: Vec::new(),
            missing_assets: Vec::new(),
            hard_walls: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn run(&self, s: &mut Session, converter: &dyn LegacyConverter, source_dir: &Path) -> Result<Value> {
        if !source_dir.is_dir() {
            return Err(EngineError::InvalidParams(format!("conversion source folder not found: {}", source_dir.display())));
        }
        let ws = ensure_magic_workspace(&self.magic_root)?;
        let plan = converter.extract_plan(self, source_dir)?;
        let slug = validate_project_name(&plan.title).unwrap_or_else(|_| project_slug(&plan.title));
        let slug = if slug.is_empty() { "converted-project".to_owned() } else { slug };
        let project_dir = ws.projects_dir.join(&slug);
        let magic_file = project_dir.join(format!("{}.magic", plan.title));
        if magic_file.exists() && !self.overwrite {
            return Err(EngineError::InvalidParams(format!("converted project already exists: {} (pass overwrite=true to replace)", magic_file.display())));
        }

        scaffold_converted_project(&project_dir)?;
        let archive_receipt = archive::archive_legacy_project(source_dir, &project_dir, &slug)?;
        let local_audio = stage_converted_audio(&project_dir, &plan, self.overwrite)?;
        let staged_assets = stage_converted_assets(&ws, &project_dir, &plan, self.overwrite)?;
        stage_rigs_and_overlays(&ws, &project_dir, &plan)?;
        let fcproj_path = seed_filmcraft_project(s, &project_dir, &plan, local_audio.as_deref())?;

        reset_for_project(s, magic_file.clone(), staged_assets);
        s.song.title = plan.title.clone();
        s.song_audio = local_audio;
        s.video_project_path = Some(fcproj_path.clone());
        write_project_file(s)?;

        let sb_pdf = project_dir.join("storyboard.pdf");
        let _ = crate::cmd::storyboard::cmd_storyboard_new(s, &json!({"path": sb_pdf, "overwrite": self.overwrite, "title": plan.title}));

        let playbook = format_terminal_playbook(&plan, &magic_file, &archive_receipt);
        stamp_conversion_metadata(&magic_file, &plan, &archive_receipt, &playbook)?;

        if self.emit_terminal && (!plan.hard_walls.is_empty() || !plan.missing_assets.is_empty()) {
            let _ = std::io::Write::write_all(&mut std::io::stderr(), playbook.as_bytes());
        }

        Ok(json!({
            "ok": true,
            "converter": plan.kind.as_str(),
            "magic_root": ws.magic_root,
            "project": magic_file,
            "project_dir": project_dir,
            "filmcraft_project": fcproj_path,
            "storyboard_pdf": sb_pdf,
            "legacy_archive": {
                "tag": "legacy",
                "archive": archive_receipt.archive_rel,
                "manifest": archive_receipt.manifest_rel,
                "file_count": archive_receipt.file_count,
                "raw_bytes": archive_receipt.raw_bytes,
                "compressed_bytes": archive_receipt.compressed_bytes,
                "sha256": archive_receipt.sha256,
            },
            "assets_imported": s.project_assets.len(),
            "missing_assets": plan.missing_assets,
            "hard_walls": plan.hard_walls,
            "notes": plan.notes,
            "terminal_playbook": playbook,
        }))
    }
}

fn scaffold_converted_project(project_dir: &Path) -> Result<()> {
    for rel in [
        "audio",
        "assets/characters",
        "assets/backgrounds",
        "assets/images",
        "assets/vectors",
        "assets/3d",
        "assets/.previews",
        "storyboard",
        "film",
        "animations",
        "lettering",
        "motion",
        "liner-notes",
        "exports",
        "magic/originals",
    ] {
        std::fs::create_dir_all(project_dir.join(rel))?;
    }
    Ok(())
}

fn stage_converted_audio(project_dir: &Path, plan: &ConversionPlan, overwrite: bool) -> Result<Option<PathBuf>> {
    let Some(first_audio) = plan.audio_candidates.first() else {
        return Ok(None);
    };
    let (copied, _) = copy_audio_into_project(project_dir, first_audio, overwrite)?;
    Ok(Some(copied))
}

fn stage_converted_assets(ws: &MagicWorkspaceLayout, project_dir: &Path, plan: &ConversionPlan, overwrite: bool) -> Result<Vec<ProjectAsset>> {
    let mut out = Vec::new();
    for (idx, item) in plan.discovered_assets.iter().enumerate() {
        let Some(fname) = item.source_path.file_name() else {
            continue;
        };
        let dest_dir = match (item.bucket.as_str(), item.character_id.as_deref()) {
            ("characters", Some(cid)) => project_dir.join("assets/characters").join(crate::workspace::sanitize_asset_id(cid)).join("parts"),
            ("characters", None) => project_dir.join("assets/characters/cast/parts"),
            ("animations", _) => project_dir.join("animations"),
            (bucket, _) => project_dir.join("assets").join(bucket),
        };
        std::fs::create_dir_all(&dest_dir)?;
        let dest = dest_dir.join(fname);
        if item.source_path.is_dir() {
            let _ = crate::doc_state::atomic_copy_dir(&item.source_path, &dest, overwrite);
        } else {
            let _ = crate::doc_state::atomic_copy_file(&item.source_path, &dest, overwrite);
        }
        let stem = item.source_path.file_stem().and_then(|s| s.to_str()).unwrap_or("asset");
        let id = format!("{}_{}", crate::workspace::sanitize_asset_id(stem), idx + 1);
        let kind = if item.bucket == "3d" {
            "model_3d"
        } else if item.bucket == "characters" {
            "cutout_rig"
        } else {
            "image"
        };
        if item.promote_shared && dest.exists() {
            let _ = promote_asset_to_workspace(&ws.magic_root, &id, stem, kind, &item.bucket, &dest, true);
        }
        out.push(ProjectAsset { id, name: stem.to_owned(), preset: None, path: Some(dest), kind: kind.to_owned(), extra: Default::default() });
    }
    Ok(out)
}

fn stage_rigs_and_overlays(ws: &MagicWorkspaceLayout, project_dir: &Path, plan: &ConversionPlan) -> Result<()> {
    if !plan.timed_words.is_empty() {
        let words_doc = json!({"fps": plan.fps, "words": plan.timed_words});
        let _ = crate::doc_state::write_json(&project_dir.join("lettering/words.json"), &words_doc)?;
        let timing_ref = json!({
            "version": 1,
            "source": plan.kind.as_str(),
            "fps": plan.fps,
            "words": plan.timed_words,
        });
        let _ = crate::doc_state::write_json(&project_dir.join("liner-notes/timing-reference.json"), &timing_ref)?;
    }
    if !plan.motion_overlays.is_empty() {
        let _ = crate::doc_state::write_json(&project_dir.join("motion/converted-overlays.json"), &json!({"overlays": plan.motion_overlays}))?;
    }
    for (idx, rig) in plan.character_rigs.iter().enumerate() {
        let raw_cid = rig["character"].as_str().unwrap_or("character");
        let cid = format!("{}_{}", crate::workspace::sanitize_asset_id(raw_cid), idx + 1);
        let char_dir = project_dir.join("assets/characters").join(&cid);
        std::fs::create_dir_all(&char_dir)?;
        let _ = crate::doc_state::write_json(&char_dir.join("rig.json"), rig)?;
        if rig["promote_to_magic_animations"].as_bool() == Some(true) {
            let shared_anim_dir = ws.animations_dir.join(&cid);
            std::fs::create_dir_all(&shared_anim_dir)?;
            let _ = crate::doc_state::write_json(&shared_anim_dir.join("rig.json"), rig)?;
        }
    }
    Ok(())
}

fn seed_filmcraft_project(s: &mut Session, project_dir: &Path, plan: &ConversionPlan, local_audio: Option<&Path>) -> Result<PathBuf> {
    let safe_stem = plan.title.replace(['/', '\\'], "-");
    let fcproj_dest = project_dir.join("film").join(format!("{safe_stem}.fcproj"));
    if let Some(existing) = &plan.existing_fcproj
        && existing.is_file()
    {
        crate::doc_state::atomic_copy_file(existing, &fcproj_dest, true)?;
        return Ok(fcproj_dest);
    }
    if let Some(audio) = local_audio
        && let Ok(created) = crate::cmd::film::assembly::ensure_song_film(s, audio)
        && created.is_file()
    {
        return Ok(created);
    }
    let dur_sec = plan.duration_sec.unwrap_or(120.0);
    let duration_ticks = (dur_sec * (TICKS_PER_SECOND as f64)).round() as i64;
    let starter_fcproj = json!({
        "format": "filmcraft.project",
        "version": 1,
        "name": plan.title,
        "timebase_ticks_per_second": TICKS_PER_SECOND,
        "sequence": {
            "width": plan.width,
            "height": plan.height,
            "fps": plan.fps,
            "duration_ticks": duration_ticks,
            "video_tracks": [
                {"id": "V1", "name": "Storyboard Reference"},
                {"id": "V2", "name": "Character Animation"},
                {"id": "V3", "name": "Kinetic Lyrics"},
                {"id": "V4", "name": "Action Motion"},
            ],
            "audio_tracks": [
                {"id": "A1", "name": "Song Master", "source": local_audio.and_then(|p| p.strip_prefix(project_dir).ok())},
            ],
        },
    });
    let _ = crate::doc_state::write_json(&fcproj_dest, &starter_fcproj)?;
    Ok(fcproj_dest)
}

fn stamp_conversion_metadata(magic_file: &Path, plan: &ConversionPlan, receipt: &archive::LegacyArchiveReceipt, playbook: &str) -> Result<()> {
    let mut doc = read_project_document(magic_file)?;
    doc["legacy"] = json!({
        "tag": "legacy",
        "converter": plan.kind.as_str(),
        "archived_to": receipt.archive_rel,
        "manifest": receipt.manifest_rel,
        "sha256": receipt.sha256,
        "raw_bytes": receipt.raw_bytes,
        "compressed_bytes": receipt.compressed_bytes,
    });
    doc["integration_status"] = json!(if !plan.missing_assets.is_empty() || !plan.hard_walls.is_empty() { "needs_agent_remediation" } else { "ready" });
    doc["missing_assets"] = json!(plan.missing_assets);
    doc["hard_walls"] = json!(plan.hard_walls);
    doc["agent_next_steps_summary"] = json!(playbook);
    let _ = crate::doc_state::write_json(magic_file, &doc)?;
    Ok(())
}

fn format_terminal_playbook(plan: &ConversionPlan, magic_file: &Path, receipt: &archive::LegacyArchiveReceipt) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "=== MAGIC CONVERTER REPORT ({}) ===", plan.kind.as_str().to_ascii_uppercase());
    let _ = writeln!(out, "Project: {}", magic_file.display());
    let _ = writeln!(out, "Legacy Archive (\"legacy\"): {} ({} bytes -> {} bytes gzip)", receipt.archive_rel, receipt.raw_bytes, receipt.compressed_bytes);

    if !plan.missing_assets.is_empty() {
        let _ = writeln!(out, "\n[MISSING CUSTOM ASSETS — {} item(s) flagged in .magic]", plan.missing_assets.len());
        for (idx, miss) in plan.missing_assets.iter().enumerate() {
            let _ = writeln!(
                out,
                "  {}. `{}` (referenced in `{}`) -> target `{}`\n     Next step: {}",
                idx + 1,
                miss.referenced_path,
                miss.referenced_in,
                miss.suggested_project_dest,
                miss.resolution_step
            );
        }
    }

    if !plan.hard_walls.is_empty() {
        let _ = writeln!(out, "\n[CONVERSION HARD WALLS — ESTABLISHED AGENT REMEDIATION STEPS]");
        for (idx, wall) in plan.hard_walls.iter().enumerate() {
            let _ = writeln!(out, "  {}. [{}] {}", idx + 1, wall.code, wall.summary);
            let _ = writeln!(out, "     Source: {}", wall.source_file);
            for (sidx, step) in wall.agent_steps.iter().enumerate() {
                let _ = writeln!(out, "     Step {}.{}: {}", idx + 1, sidx + 1, step);
            }
            for (label, url) in &wall.doc_links {
                let _ = writeln!(out, "     Docs: {label} -> {url}");
            }
        }
    }
    if plan.missing_assets.is_empty() && plan.hard_walls.is_empty() {
        let _ = writeln!(out, "Status: READY — zero missing assets or hard walls detected.");
    }
    out
}

pub(crate) fn collect_files_filtered(root: &Path, exts: &[&str]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect_files_rec(root, exts, 0, &mut out);
    out
}

fn collect_files_rec(dir: &Path, exts: &[&str], depth: usize, out: &mut Vec<PathBuf>) {
    if depth > 6 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
    entries.sort();
    for entry in entries {
        let Some(name) = entry.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if matches!(name, "node_modules" | ".git" | "target" | "dist" | ".next" | "originals") || name.starts_with('.') {
            continue;
        }
        if entry.is_dir() {
            collect_files_rec(&entry, exts, depth + 1, out);
        } else if entry.extension().and_then(|e| e.to_str()).is_some_and(|e| exts.iter().any(|a| a.eq_ignore_ascii_case(e))) {
            out.push(entry);
        }
    }
}

pub(crate) fn extract_attr(text: &str, attr: &str) -> Option<String> {
    for quote in ['"', '\''] {
        let pat = format!("{attr}={quote}");
        if let Some(idx) = text.find(&pat) {
            let rest = &text[idx + pat.len()..];
            if let Some(end) = rest.find(quote) {
                return Some(rest[..end].trim().to_owned());
            }
        }
    }
    None
}

pub(crate) fn extract_quoted_paths(text: &str, exts: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    for attr in ["src", "href", "data-src", "poster"] {
        let mut slice = text;
        while let Some(pos) = slice.find(attr) {
            let after = &slice[pos + attr.len()..];
            if let Some(val) = extract_attr(&slice[pos..], attr)
                && Path::new(&val).extension().and_then(|e| e.to_str()).is_some_and(|e| exts.iter().any(|a| a.eq_ignore_ascii_case(e)))
                && !out.contains(&val)
            {
                out.push(val);
            }
            slice = after;
        }
    }
    out
}

pub(crate) fn extract_static_file_calls(code: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = code;
    while let Some(idx) = rest.find("staticFile(") {
        let after = &rest[idx + "staticFile(".len()..];
        let trimmed = after.trim_start();
        if let Some(quote) = trimmed.chars().next().filter(|&c| c == '"' || c == '\'' || c == '`') {
            let inner = &trimmed[1..];
            if let Some(end) = inner.find(quote) {
                let path = inner[..end].trim().to_owned();
                if !path.is_empty() && !path.contains("${") && !out.contains(&path) {
                    out.push(path);
                }
            }
        }
        rest = after;
    }
    out
}

/// `project.convert`: convert a `hyperframes`, `remotion`, or `custom` folder into `<magic_root>/projects/<slug>/`.
pub fn cmd_project_convert(s: &mut Session, params: &Value) -> Result<Value> {
    let raw_src = params
        .get("source")
        .or_else(|| params.get("path"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| EngineError::InvalidParams("project.convert requires `source` (<legacy project directory>)".into()))?;
    let source_dir = crate::workspace::resolve_workspace_path(s, raw_src);

    let magic_root = match params.get("magic_root").or_else(|| params.get("root")).and_then(Value::as_str).map(str::trim).filter(|v| !v.is_empty()) {
        Some(raw_root) => {
            let resolved = crate::workspace::resolve_workspace_path(s, raw_root);
            resolve_magic_root(&resolved, false)
        }
        None => {
            let default_projects = crate::project_settings::root_for(s)?;
            resolve_magic_root(&default_projects, false)
        }
    };

    let explicit_name = params.get("name").or_else(|| params.get("title")).and_then(Value::as_str).map(str::trim).filter(|v| !v.is_empty()).map(str::to_owned);
    let overwrite = params["overwrite"].as_bool().unwrap_or(false);
    let emit_terminal = params["emit_terminal"].as_bool().unwrap_or(true);

    let hf = hyperframes::HyperFramesConverter;
    let rem = remotion::RemotionConverter;
    let cust = custom::CustomConverter;

    let kind_str = params["kind"].as_str().unwrap_or("auto").trim().to_ascii_lowercase();
    let converter: &dyn LegacyConverter = match kind_str.as_str() {
        "hyperframes" | "hf" => &hf,
        "remotion" => &rem,
        "custom" | "filmcraft" => &cust,
        "auto" => {
            if hf.detect(&source_dir) {
                &hf
            } else if rem.detect(&source_dir) {
                &rem
            } else {
                &cust
            }
        }
        other => {
            return Err(EngineError::InvalidParams(format!("unknown converter kind `{other}` (expected hyperframes | remotion | custom | auto)")));
        }
    };

    let base = BaseConverter::new(magic_root, explicit_name, overwrite, emit_terminal);
    base.run(s, converter, &source_dir)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::json;

    use super::cmd_project_convert;
    use crate::Session;
    use crate::cmd::project::read_project_document;

    fn temp_dir(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let d = std::env::temp_dir().join(format!("magic_convert_{tag}_{}_{nanos}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn converts_hyperframes_remotion_and_custom_via_base_converter() {
        let sandbox = temp_dir("all_three");
        let magic_root = sandbox.join("magic");

        // 1. HyperFrames source with missing audio + renderAt hard wall
        let hf_src = sandbox.join("legacy_hf");
        std::fs::create_dir_all(&hf_src).unwrap();
        crate::doc_state::write_file(
            hf_src.join("index.html"),
            br#"<div data-composition-id="main" data-width="1920" data-height="1080" data-fps="30"><audio src="missing.wav"></audio></div>"#,
        )
        .unwrap();
        crate::doc_state::write_file(hf_src.join("film.js"), b"window.__hf = { renderAt(t) {} };").unwrap();

        let mut s = Session::default();
        let hf_res =
            cmd_project_convert(&mut s, &json!({"kind": "auto", "source": hf_src, "magic_root": magic_root, "name": "HF Song", "emit_terminal": false}))
                .unwrap();
        assert_eq!(hf_res["converter"], "hyperframes");
        let hf_magic = PathBuf::from(hf_res["project"].as_str().unwrap());
        let hf_doc = read_project_document(&hf_magic).unwrap();
        assert_eq!(hf_doc["legacy"]["tag"], "legacy");
        assert!(!hf_doc["missing_assets"].as_array().unwrap().is_empty());
        assert!(!hf_doc["hard_walls"].as_array().unwrap().is_empty());
        assert!(hf_magic.parent().unwrap().join("magic/originals/hf-song-legacy.tar.gz").is_file());

        // 2. Remotion source with staticFile missing asset + poseAt hard wall
        let rem_src = sandbox.join("legacy_rem");
        std::fs::create_dir_all(rem_src.join("src")).unwrap();
        crate::doc_state::write_file(rem_src.join("package.json"), br#"{"name":"pterodactyl","dependencies":{"remotion":"4.0.0"}}"#).unwrap();
        crate::doc_state::write_file(
            rem_src.join("src/Root.tsx"),
            concat!(
                "import {Composition, staticFile, useCurrentFrame} from 'remotion'; ",
                "const a = staticFile('dino.png'); function poseAt(t:number){return Math.sin(t);} ",
                "export const R = () => <Composition id=\"Main\" width={1920} height={1080} fps={24} durationInFrames={240} />;"
            )
            .as_bytes(),
        )
        .unwrap();
        let rem_res =
            cmd_project_convert(&mut s, &json!({"kind": "auto", "source": rem_src, "magic_root": magic_root, "name": "Dino Remotion", "emit_terminal": false}))
                .unwrap();
        assert_eq!(rem_res["converter"], "remotion");
        assert!(magic_root.join("projects/dino-remotion/film/Dino Remotion.fcproj").is_file());
        assert!(magic_root.join("projects/dino-remotion/storyboard.pdf").is_file());

        // 3. Custom project creating working FilmCraft + empty storyboard + magic/{projects,assets,animations}
        let cust_src = sandbox.join("legacy_custom");
        std::fs::create_dir_all(&cust_src).unwrap();
        crate::doc_state::write_file(cust_src.join("notes.txt"), b"Storytelling narrative draft").unwrap();
        let cust_res = cmd_project_convert(
            &mut s,
            &json!({"kind": "custom", "source": cust_src, "magic_root": magic_root, "name": "Story Custom", "emit_terminal": false}),
        )
        .unwrap();
        assert_eq!(cust_res["converter"], "custom");
        assert!(magic_root.join("projects/story-custom/storyboard.pdf").is_file());
        assert!(magic_root.join("projects/story-custom/film/Story Custom.fcproj").is_file());
        assert!(magic_root.join("assets/catalog.json").is_file());
        assert!(magic_root.join("animations").is_dir());

        let _ = std::fs::remove_dir_all(&sandbox);
    }
}
