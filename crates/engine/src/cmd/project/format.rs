//! Canonical `.magic` v1 project document schema (`format = "magic.project"`, `schema_version = 1`),
//! strict path validation via [`craft_docpath`], and `TooNew` rejection.

use std::path::{Path, PathBuf};

use craft_docpath::{Refused, resolve_absolute};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::cmd::song::song_metadata::SongMetadata;
use crate::{EngineError, ProjectAsset, Result};

pub const PROJECT_FORMAT: &str = "magic.project";
pub const PROJECT_SCHEMA_VERSION: u32 = 1;
pub const PROJECT_GENERATOR: &str = concat!("Magic Studio ", env!("CARGO_PKG_VERSION"));

fn default_format() -> String {
    PROJECT_FORMAT.to_owned()
}

const fn default_schema_version() -> u32 {
    PROJECT_SCHEMA_VERSION
}

fn default_generator() -> String {
    PROJECT_GENERATOR.to_owned()
}

fn default_storyboard() -> String {
    crate::cmd::storyboard::DEFAULT_STORYBOARD_FILE.to_owned()
}

fn default_target() -> String {
    "filmcraft".to_owned()
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkshopDoc {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<PathBuf>,
    pub storyboard: String,
    pub draft_approved: bool,
    pub target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_project: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcript: Option<Value>,
}

impl Default for WorkshopDoc {
    fn default() -> Self {
        Self { audio: None, storyboard: default_storyboard(), draft_approved: false, target: default_target(), video_project: None, transcript: None }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CharacterRecord {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub animation_style: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ecproj: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anicraft: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_3d: Option<PathBuf>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub views: Vec<Value>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sprites: Vec<Value>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub scripts: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Project {
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default = "default_generator")]
    pub generator: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "is_default_song")]
    pub song: SongMetadata,
    pub workshop: WorkshopDoc,
    pub assets: Vec<ProjectAsset>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub characters: Vec<CharacterRecord>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub animation_cast: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lyrics: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub motion: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub studio: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theatre: Option<Value>,
}

fn is_default_song(song: &SongMetadata) -> bool {
    *song == SongMetadata::default()
}

impl Default for Project {
    fn default() -> Self {
        Self {
            format: default_format(),
            schema_version: PROJECT_SCHEMA_VERSION,
            generator: default_generator(),
            name: String::new(),
            mode: None,
            song: SongMetadata::default(),
            workshop: WorkshopDoc::default(),
            assets: Vec::new(),
            characters: Vec::new(),
            animation_cast: Vec::new(),
            lyrics: None,
            motion: None,
            studio: None,
            theatre: None,
        }
    }
}

pub(crate) fn saved_path(path: &Path, project: &Path) -> PathBuf {
    let parent = project.parent().unwrap_or(Path::new("."));
    if let Ok(rel) = path.strip_prefix(parent) {
        return rel.to_path_buf();
    }
    if let (Ok(canon_path), Ok(canon_parent)) = (path.canonicalize(), parent.canonicalize())
        && let Ok(rel) = canon_path.strip_prefix(&canon_parent)
    {
        return rel.to_path_buf();
    }
    path.to_path_buf()
}

fn portable_path_string(path: &Path, project: &Path) -> String {
    saved_path(path, project).to_string_lossy().replace('\\', "/")
}

fn format_refused(field: &str, raw: &str, err: Refused) -> EngineError {
    EngineError::InvalidParams(format!("refused path in `{field}` ({raw}): {err}"))
}

fn resolve_stored_path(field: &str, path: &Path, abs_project: &Path) -> Result<PathBuf> {
    let raw = path.to_str().ok_or_else(|| EngineError::InvalidParams(format!("non-UTF-8 path in `{field}`")))?;
    if raw.trim().is_empty() {
        return Err(EngineError::InvalidParams(format!("empty path in `{field}`")));
    }
    let resolved = resolve_absolute(raw, Some(abs_project)).map_err(|e| format_refused(field, raw, e))?;
    // Prevent `..` traversal from escaping the project directory when the stored path was relative.
    if !Path::new(raw).is_absolute()
        && let Some(project_dir) = abs_project.parent()
        && !resolved.starts_with(project_dir)
    {
        return Err(EngineError::InvalidParams(format!("refused relative path escaping project directory in `{field}`: {raw}")));
    }
    Ok(resolved)
}

fn resolve_opt_path(field: &str, opt: &mut Option<PathBuf>, abs_project: &Path) -> Result<()> {
    if let Some(p) = opt.take() {
        *opt = Some(resolve_stored_path(field, &p, abs_project)?);
    }
    Ok(())
}

fn validate_schema_header(doc: &Value, display_name: &Path) -> Result<u32> {
    let Some(obj) = doc.as_object() else {
        return Err(EngineError::InvalidParams(format!("{} must be a JSON object", display_name.display())));
    };
    if obj.get("format").and_then(Value::as_str) != Some(PROJECT_FORMAT) {
        return Err(EngineError::InvalidParams(format!("{} is not a Magic Studio project (missing format = {PROJECT_FORMAT:?})", display_name.display())));
    }
    let version_val = obj.get("schema_version").or_else(|| obj.get("version"));
    let version = match version_val {
        None => PROJECT_SCHEMA_VERSION,
        Some(v) => {
            let n = v.as_u64().ok_or_else(|| EngineError::InvalidParams(format!("invalid project schema_version in {}", display_name.display())))?;
            u32::try_from(n).unwrap_or(u32::MAX)
        }
    };
    if version == 0 {
        return Err(EngineError::InvalidParams(format!("invalid schema_version 0 in {}", display_name.display())));
    }
    if version > PROJECT_SCHEMA_VERSION {
        return Err(EngineError::InvalidParams(format!(
            "TooNew: {} uses project schema_version {version}, which is newer than supported version {PROJECT_SCHEMA_VERSION}",
            display_name.display()
        )));
    }
    Ok(version)
}

impl Project {
    pub fn new_for_path(path: &Path, mode: Option<String>) -> Self {
        let name = path.file_stem().and_then(|n| n.to_str()).unwrap_or("Untitled").to_owned();
        Self { name, mode, ..Self::default() }
    }

    pub fn from_value(doc: &Value, project_path: &Path) -> Result<Self> {
        let schema_version = validate_schema_header(doc, project_path)?;
        let mut project: Self =
            serde_json::from_value(doc.clone()).map_err(|e| EngineError::InvalidParams(format!("invalid project file {}: {e}", project_path.display())))?;
        project.format = PROJECT_FORMAT.to_owned();
        project.schema_version = schema_version;
        if project.name.trim().is_empty() {
            project.name = project_path.file_stem().and_then(|n| n.to_str()).unwrap_or("Untitled").to_owned();
        }
        if let Some(ref mode) = project.mode
            && mode != "animation"
            && mode != "project"
        {
            return Err(EngineError::InvalidParams(format!("mode must be animation or project, not {mode}")));
        }
        if project.mode.as_deref() == Some("project") {
            project.mode = None;
        }
        if !crate::cmd::project::workshop::VIDEO_TARGETS.contains(&project.workshop.target.as_str()) {
            project.workshop.target = default_target();
        }
        let abs_project = if project_path.is_absolute() { project_path.to_path_buf() } else { std::path::absolute(project_path).map_err(EngineError::from)? };
        resolve_opt_path("workshop.audio", &mut project.workshop.audio, &abs_project)?;
        resolve_opt_path("workshop.video_project", &mut project.workshop.video_project, &abs_project)?;
        for (i, asset) in project.assets.iter_mut().enumerate() {
            resolve_opt_path(&format!("assets[{i}].path"), &mut asset.path, &abs_project)?;
        }
        for (i, ch) in project.characters.iter_mut().enumerate() {
            resolve_opt_path(&format!("characters[{i}].ecproj"), &mut ch.ecproj, &abs_project)?;
            resolve_opt_path(&format!("characters[{i}].anicraft"), &mut ch.anicraft, &abs_project)?;
            resolve_opt_path(&format!("characters[{i}].model_3d"), &mut ch.model_3d, &abs_project)?;
        }
        Ok(project)
    }

    pub fn read_from_file(path: &Path) -> Result<Self> {
        let raw = std::fs::read(path)?;
        let doc: Value = serde_json::from_slice(&raw).map_err(|e| EngineError::InvalidParams(format!("invalid project file: {e}")))?;
        Self::from_value(&doc, path)
    }

    pub fn normalized_for_save(&self, project_path: &Path) -> Self {
        let mut out = self.clone();
        out.format = PROJECT_FORMAT.to_owned();
        out.schema_version = PROJECT_SCHEMA_VERSION;
        out.generator = PROJECT_GENERATOR.to_owned();
        if out.name.trim().is_empty() {
            out.name = project_path.file_stem().and_then(|n| n.to_str()).unwrap_or("Untitled").to_owned();
        }
        out.workshop.storyboard = default_storyboard();
        if let Some(p) = out.workshop.audio.take() {
            out.workshop.audio = Some(PathBuf::from(portable_path_string(&p, project_path)));
        }
        if let Some(p) = out.workshop.video_project.take() {
            out.workshop.video_project = Some(PathBuf::from(portable_path_string(&p, project_path)));
        }
        for asset in &mut out.assets {
            if let Some(p) = asset.path.take() {
                asset.path = Some(PathBuf::from(portable_path_string(&p, project_path)));
            }
        }
        for ch in &mut out.characters {
            if let Some(p) = ch.ecproj.take() {
                ch.ecproj = Some(PathBuf::from(portable_path_string(&p, project_path)));
            }
            if let Some(p) = ch.anicraft.take() {
                ch.anicraft = Some(PathBuf::from(portable_path_string(&p, project_path)));
            }
            if let Some(p) = ch.model_3d.take() {
                ch.model_3d = Some(PathBuf::from(portable_path_string(&p, project_path)));
            }
        }
        out
    }

    pub fn to_persisted_value(&self, project_path: &Path) -> Value {
        serde_json::to_value(self.normalized_for_save(project_path)).unwrap_or(json!({"format": PROJECT_FORMAT, "schema_version": PROJECT_SCHEMA_VERSION}))
    }
}
