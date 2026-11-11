//! Characters as Craft app documents: Scribbles marks parts, `PhotoCraft` isolates subjects and cuts
//! layers (`.pcraft` / `.psd`), `VectorCraft` traces silhouettes (`.vectorcraft` / `.svg`), headless
//! `anicraft-cli` renders optional cel sequences (`.anicraft`), and `EffectCraft` rigs, puppet-pins,
//! renders, and publishes `V2` `"Character Animation"` tracks into `FilmCraft` via [`crate::CraftHub`].

mod action;
pub(crate) mod cut;
pub mod effect;
pub mod keys;
pub mod manage;
mod mark;
mod meta;
pub mod model3d;
pub mod path;
pub mod puppet;
pub mod review;
mod rig;
mod shot;
mod sprite;

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::{EngineError, Result, Session};

pub use action::{cmd_character_action, cmd_character_actions};
pub use cut::cmd_character_cut;
pub use effect::{
    cmd_expression_set, cmd_keyframe_easy_ease, cmd_keyframe_velocity, cmd_layer_new_3d_model, cmd_layer_set_blend_mode, cmd_layer_set_parent, cmd_mask_add,
    cmd_puppet_add_pin, cmd_puppet_move_pin, cmd_puppet_null_rig, cmd_puppet_remove_pin, cmd_puppet_select_pins, cmd_puppet_set_pin, cmd_time_remap,
};
pub use manage::{
    classify_animation_style, cmd_animation_open, cmd_animation_view, cmd_character_frame, cmd_character_import, cmd_character_list, cmd_character_open,
    cmd_character_render, cmd_character_set, sprite_summaries,
};
pub use mark::cmd_character_mark;
pub use meta::{cmd_character_meta, cmd_character_save_marks};
pub use model3d::{cmd_model3d_animate, cmd_model3d_breakout, cmd_model3d_catalog, cmd_model3d_rig};
pub use puppet::{cmd_puppet_expression, cmd_puppet_follow, cmd_puppet_lipsync, cmd_puppet_record, cmd_puppet_rig_face};
pub use rig::cmd_character_rig;
pub use shot::{cmd_character_publish, cmd_character_shot};
pub use sprite::cmd_character_sprite;

pub(crate) const POSES: &str = include_str!("poses.json");
pub(crate) const SKIP_CHARACTER_GROUPS: &[&str] = &[
    "_incoming",
    "approved-crops",
    "lineup",
    "expressions",
    "contact-parts-v1",
    "cast",
    "tripo-input",
    "parts",
    "sprites",
    "frames",
    "layers",
    "source",
    "renders",
    "preview",
    "rig",
];

pub(crate) fn bad(message: impl Into<String>) -> EngineError {
    EngineError::InvalidParams(message.into())
}

fn matches_character_name(folder_name: &str, id: &str) -> bool {
    let sanitized = crate::workspace::sanitize_asset_id(folder_name);
    folder_name.eq_ignore_ascii_case(id) || sanitized == id || id.starts_with(&format!("{sanitized}_"))
}

/// Discover all character directories under `assets/characters` (both flat `<char>` and nested
/// cast/family folders `<group>/<char>`) plus `animations/characters/<char>`.
pub(crate) fn discover_character_folders(project_dir: &Path) -> Vec<(String, Option<String>, PathBuf)> {
    let mut out = Vec::new();
    let root = project_dir.join("assets/characters");
    let mut top_entries: Vec<PathBuf> = std::fs::read_dir(&root).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    top_entries.sort();

    for entry in top_entries {
        let Some(name) = entry.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if SKIP_CHARACTER_GROUPS.iter().any(|s| name.eq_ignore_ascii_case(s)) || name.starts_with('.') {
            continue;
        }
        let direct_has_craft = entry.join("front").is_dir()
            || entry.join("side").is_dir()
            || entry.join("back").is_dir()
            || entry.join("parts").is_dir()
            || entry.join("sprites").is_dir()
            || entry.join("rig").is_dir()
            || entry.join("parts.json").is_file()
            || entry.join("character.magic").is_file()
            || std::fs::read_dir(&entry).into_iter().flatten().flatten().any(|f| {
                f.path().is_file()
                    && crate::workspace::has_extension(&f.path(), &["pcraft", "psd", "ecproj", "vectorcraft", "svg", "anicraft", "png", "webp", "jpg", "jpeg"])
            });
        if direct_has_craft {
            out.push((name.to_owned(), None, entry));
            continue;
        }
        let mut sub_entries: Vec<PathBuf> = std::fs::read_dir(&entry).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
        sub_entries.sort();
        for sub in sub_entries {
            if let Some(sub_name) = sub.file_name().and_then(|n| n.to_str())
                && !sub_name.starts_with('.')
                && !SKIP_CHARACTER_GROUPS.iter().any(|s| sub_name.eq_ignore_ascii_case(s))
            {
                out.push((sub_name.to_owned(), Some(name.to_owned()), sub));
            }
        }
    }

    for anim_chars_root in [project_dir.join("animations/characters"), project_dir.join("assets/animations/characters")] {
        let mut anim_entries: Vec<PathBuf> =
            std::fs::read_dir(&anim_chars_root).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
        anim_entries.sort();
        for entry in anim_entries {
            if let Some(name) = entry.file_name().and_then(|n| n.to_str())
                && !out.iter().any(|(existing, _, _)| crate::workspace::sanitize_asset_id(existing) == crate::workspace::sanitize_asset_id(name))
            {
                out.push((name.to_owned(), Some("animations".into()), entry));
            }
        }
    }
    out
}

pub(crate) fn character_folder(project_dir: &Path, id: &str) -> PathBuf {
    for (raw_name, _, path) in discover_character_folders(project_dir) {
        if matches_character_name(&raw_name, id) {
            return path;
        }
    }
    project_dir.join("assets/characters").join(id)
}

pub(crate) fn find_initial_cutout(char_folder: &Path) -> Option<PathBuf> {
    let folder_name = char_folder.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    for named in [
        char_folder.join("assembled.png"),
        char_folder.join("cutout.png"),
        char_folder.join("front.png"),
        char_folder.join(format!("{folder_name}-clean.png")),
        char_folder.join(format!("{folder_name}-drawing.png")),
        char_folder.join(format!("{folder_name}-front.png")),
        char_folder.join("flow-original.png"),
        char_folder.join("parts/source/body-plate_v1.png"),
    ] {
        if named.is_file() {
            return Some(named);
        }
    }
    let parts_dir = char_folder.join("parts");
    if let Ok(rd) = std::fs::read_dir(&parts_dir) {
        let mut files: Vec<PathBuf> =
            rd.flatten().map(|e| e.path()).filter(|p| p.is_file() && crate::workspace::has_extension(p, &["png", "webp", "jpg", "jpeg"])).collect();
        files.sort();
        if let Some(first) = files.into_iter().next() {
            return Some(first);
        }
    }
    let sing_frame = char_folder.join("sprites/sing/frames/frame_0001.png");
    if sing_frame.is_file() {
        return Some(sing_frame);
    }
    if let Some(sp) =
        std::fs::read_dir(char_folder.join("sprites")).ok().and_then(|rd| rd.flatten().map(|e| e.path().join("frames/frame_0001.png")).find(|p| p.is_file()))
    {
        return Some(sp);
    }
    if let Ok(rd) = std::fs::read_dir(char_folder) {
        let mut direct: Vec<PathBuf> =
            rd.flatten().map(|e| e.path()).filter(|p| p.is_file() && crate::workspace::has_extension(p, &["png", "webp", "jpg", "jpeg"])).collect();
        direct.sort();
        if let Some(first) = direct.into_iter().next() {
            return Some(first);
        }
    }
    None
}

fn find_by_ext(dir: &Path, id: &str, folder_name: &str, view: &str, is_flat: bool, ext: &str) -> Option<PathBuf> {
    let mut candidates = vec![dir.join(format!("{id}-{view}.{ext}")), dir.join(format!("{folder_name}-{view}.{ext}"))];
    if is_flat {
        candidates.extend([
            dir.join(format!("cutout.{ext}")),
            dir.join(format!("cut-contours.{ext}")),
            dir.join(format!("layered_motion.{ext}")),
            dir.join(format!("{folder_name}-parts.{ext}")),
            dir.join(format!("{id}-parts.{ext}")),
            dir.join(format!("{folder_name}-atlas-editable.{ext}")),
            dir.join(format!("{folder_name}-identity-atlas.{ext}")),
            dir.join(format!("{folder_name}-matte.{ext}")),
            dir.join(format!("{folder_name}-rig.{ext}")),
            dir.join(format!("{id}-rig.{ext}")),
            dir.join(format!("{folder_name}.{ext}")),
            dir.join(format!("{id}.{ext}")),
        ]);
    }
    for c in candidates {
        if c.is_file() {
            return Some(c);
        }
    }
    let mut all: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case(ext)))
        .collect();
    all.sort();
    all.iter().find(|p| p.file_stem().and_then(|s| s.to_str()).is_none_or(|s| !s.contains("before-"))).cloned().or_else(|| all.into_iter().next())
}

pub(crate) fn find_dir_ecproj(dir: &Path, id: &str, folder_name: &str, view: &str, is_flat: bool) -> Option<PathBuf> {
    find_by_ext(dir, id, folder_name, view, is_flat, "ecproj")
}

pub(crate) fn find_dir_pcraft(dir: &Path, id: &str, folder_name: &str, view: &str, is_flat: bool) -> Option<PathBuf> {
    find_by_ext(dir, id, folder_name, view, is_flat, "pcraft")
}

pub(crate) fn find_dir_psd(dir: &Path, id: &str, folder_name: &str, view: &str, is_flat: bool) -> Option<PathBuf> {
    find_by_ext(dir, id, folder_name, view, is_flat, "psd")
}

/// Canonical `PhotoCraft` document discovery: prefers native `.pcraft`, falling back to `.psd`.
pub(crate) fn find_dir_photo_doc(dir: &Path, id: &str, folder_name: &str, view: &str, is_flat: bool) -> Option<PathBuf> {
    find_dir_pcraft(dir, id, folder_name, view, is_flat).or_else(|| find_dir_psd(dir, id, folder_name, view, is_flat))
}

/// Canonical `VectorCraft` document discovery: prefers `.vectorcraft`, falling back to `.svg`.
pub(crate) fn find_dir_vector_doc(dir: &Path, id: &str, folder_name: &str, view: &str, is_flat: bool) -> Option<PathBuf> {
    find_by_ext(dir, id, folder_name, view, is_flat, "vectorcraft").or_else(|| find_by_ext(dir, id, folder_name, view, is_flat, "svg"))
}

pub(crate) fn find_dir_anicraft_doc(dir: &Path) -> Option<PathBuf> {
    if let Some(direct) = find_by_ext(dir, "sprite", "sprite", "front", true, "anicraft") {
        return Some(direct);
    }
    std::fs::read_dir(dir.join("sprites"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .find_map(|sub| find_by_ext(&sub, "sprite", "sprite", "front", true, "anicraft"))
}

pub(crate) fn check_ecproj_head(ecproj_path: &Path) -> Result<craft_util::media::EcprojHeadSummary> {
    craft_util::media::inspect_ecproj_head(ecproj_path).map_err(|e| bad(format!("{}: {e}", ecproj_path.display())))
}

pub(crate) fn ecproj_first_comp_name(ecproj_path: &Path) -> Option<String> {
    let _ = check_ecproj_head(ecproj_path).ok()?;
    let bytes = std::fs::read(ecproj_path).ok()?;
    let outline = craft_doc::outline(&bytes, &craft_doc::Limits::default()).ok()?;
    outline.pages.first().map(|p| p.name.clone()).filter(|s| !s.is_empty())
}

pub(crate) struct View {
    pub id: String,
    pub folder_name: String,
    pub view: String,
    pub dir: PathBuf,
    pub is_flat: bool,
}

impl View {
    pub fn from_params(s: &Session, p: &Value) -> Result<Self> {
        let raw_id = p["character"].as_str().map(str::trim).filter(|v| !v.is_empty()).ok_or_else(|| bad("character=<id> is required"))?;
        let id = crate::workspace::sanitize_asset_id(raw_id);
        let view = p["view"].as_str().map(str::trim).filter(|v| !v.is_empty()).unwrap_or("front").to_string();
        if !matches!(view.as_str(), "front" | "back" | "side") {
            return Err(bad("view must be front, back, or side"));
        }
        let project_dir = crate::workshop::project_dir(s)?;
        let char_folder = character_folder(&project_dir, &id);
        let folder_name = char_folder.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| id.clone());
        let sub = char_folder.join(&view);
        let has_flat_craft = char_folder.join("parts.json").is_file()
            || find_dir_ecproj(&char_folder, &id, &folder_name, &view, true).is_some()
            || find_dir_photo_doc(&char_folder, &id, &folder_name, &view, true).is_some()
            || find_dir_vector_doc(&char_folder, &id, &folder_name, &view, true).is_some();
        let (dir, is_flat) = if sub.is_dir() || !char_folder.is_dir() {
            (sub, false)
        } else if has_flat_craft {
            (char_folder, true)
        } else {
            (sub, false)
        };
        Ok(Self { id, folder_name, view, dir, is_flat })
    }

    pub fn comp(&self) -> String {
        if let Some(ecproj) = find_dir_ecproj(&self.dir, &self.id, &self.folder_name, &self.view, self.is_flat)
            && let Some(first_comp) = ecproj_first_comp_name(&ecproj)
        {
            return first_comp;
        }
        format!("{}-{}", self.id, self.view)
    }

    pub fn file(&self, ext: &str) -> PathBuf {
        let found = match ext {
            "ecproj" => find_dir_ecproj(&self.dir, &self.id, &self.folder_name, &self.view, self.is_flat),
            "pcraft" => find_dir_pcraft(&self.dir, &self.id, &self.folder_name, &self.view, self.is_flat),
            "psd" => find_dir_psd(&self.dir, &self.id, &self.folder_name, &self.view, self.is_flat),
            "vectorcraft" | "svg" => find_by_ext(&self.dir, &self.id, &self.folder_name, &self.view, self.is_flat, ext),
            _ => None,
        };
        found.unwrap_or_else(|| self.dir.join(format!("{}-{}.{ext}", self.id, self.view)))
    }

    pub fn photo_doc(&self) -> Option<PathBuf> {
        find_dir_photo_doc(&self.dir, &self.id, &self.folder_name, &self.view, self.is_flat)
    }

    pub fn vector_doc(&self) -> Option<PathBuf> {
        find_dir_vector_doc(&self.dir, &self.id, &self.folder_name, &self.view, self.is_flat)
    }

    pub fn require_photo_doc(&self, made_by: &str) -> Result<PathBuf> {
        self.photo_doc().ok_or_else(|| {
            let expected = self.dir.join(format!("{}-{}.pcraft", self.id, self.view));
            bad(format!("{} (or .psd) is missing; run {made_by} first", expected.display()))
        })
    }

    pub fn require(&self, ext: &str, made_by: &str) -> Result<PathBuf> {
        if ext == "psd" || ext == "pcraft" {
            return self.require_photo_doc(made_by);
        }
        let path = self.file(ext);
        if path.is_file() {
            if ext == "ecproj" {
                let _ = check_ecproj_head(&path)?;
            }
            Ok(path)
        } else {
            Err(bad(format!("{} is missing; run {made_by} first", path.display())))
        }
    }

    pub fn parts(&self) -> Result<Value> {
        let path = self.dir.join("parts.json");
        let text = std::fs::read_to_string(&path).map_err(|_| bad(format!("{} is missing; run character.mark first", path.display())))?;
        serde_json::from_str(&text).map_err(|e| bad(format!("parts.json: {e}")))
    }

    pub fn write_parts(&self, parts: &Value) -> Result<()> {
        crate::doc_state::write_json(&self.dir.join("parts.json"), parts)?;
        Ok(())
    }
}

pub(crate) fn point(v: &Value) -> Option<[f64; 2]> {
    let a = v.as_array().filter(|a| a.len() == 2)?;
    let (x, y) = (a[0].as_f64()?, a[1].as_f64()?);
    (x.is_finite() && y.is_finite()).then_some([x, y])
}

pub(crate) fn part_list(parts: &Value) -> Result<&Vec<Value>> {
    parts["parts"].as_array().filter(|p| !p.is_empty()).ok_or_else(|| bad("parts.json has no parts"))
}

pub(crate) fn root_part(parts: &[Value]) -> Result<&str> {
    let roots: Vec<&str> = parts.iter().filter(|p| p["parent"].is_null() || p["parent"].as_str() == Some("")).filter_map(|p| p["id"].as_str()).collect();
    match roots.as_slice() {
        [root] => Ok(root),
        [] => Err(bad("parts.json needs one root part with parent: null")),
        _ => Err(bad("parts.json has more than one root part")),
    }
}

pub(crate) fn step(command: &str, params: Value) -> Value {
    json!({"command": command, "params": params})
}

pub(crate) fn effectcraft(_cwd: &Path, project: Option<&Path>, steps: &[Value], save: bool) -> Result<Vec<Value>> {
    let mut pairs: Vec<(String, Value)> = Vec::with_capacity(steps.len());
    for s in steps {
        let cmd = s["command"].as_str().or_else(|| s["id"].as_str()).ok_or_else(|| bad(format!("missing command in {s}")))?.to_string();
        let params = s.get("params").cloned().unwrap_or_else(|| json!({}));
        pairs.push((cmd, params));
    }
    crate::CraftHub::run_effect_steps(project, save, &pairs)
}

pub(crate) fn effectcraft_frame(project: &Path, comp: &str, seconds: f64, max_side: u32, output: &Path) -> Result<()> {
    let side = if max_side == 0 { 900 } else { max_side };
    crate::CraftHub::render_effect_frame(project, (!comp.is_empty()).then_some(comp), seconds, side, output)
}

pub(crate) fn effectcraft_render(project: &Path, params: Value) -> Result<Value> {
    crate::CraftHub::render_effect_clip(project, &params)
}
