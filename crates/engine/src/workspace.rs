//! App-level default folders (`~/.local/share/magic/{templates,recovery,logs}`), unified
//! `craft-docpath` path resolution, and OS editor handoff (`workspace.open`).

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{Value, json};

use crate::{EngineError, Result};

pub fn control_token() -> Option<String> {
    std::env::var("MAGIC_CONTROL_TOKEN").ok().map(|t| t.trim().to_owned()).filter(|t| !t.is_empty())
}

pub fn workspace_root(s: &crate::Session) -> PathBuf {
    if let Ok(exe) = std::env::current_exe()
        && let Some(home) = exe.parent().and_then(Path::parent)
    {
        let runtime = home.join("runtime");
        if runtime.join("installation.json").is_file() {
            return runtime;
        }
    }
    let manifest_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    if let Ok(canon) = manifest_root.canonicalize()
        && canon.join("Cargo.toml").is_file()
        && canon.join("crates/engine/Cargo.toml").is_file()
    {
        return canon;
    }
    s.cwd.clone()
}

pub fn expand_tilde(raw: &str) -> PathBuf {
    let trimmed = raw.trim();
    let home = || std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from);
    if trimmed == "~"
        && let Some(h) = home()
    {
        return h;
    }
    if let Some(rest) = trimmed.strip_prefix("~/").or_else(|| trimmed.strip_prefix("~\\"))
        && let Some(h) = home()
    {
        return h.join(rest);
    }
    PathBuf::from(trimmed)
}

/// Single `craft-docpath`-backed path resolver shared across project, film, and 3D mesh commands.
pub fn resolve_path_in(s: &crate::Session, base_dir: Option<&Path>, raw: &str) -> PathBuf {
    let expanded = expand_tilde(raw);
    if expanded.is_absolute() {
        return expanded;
    }
    if let Some(dir) = base_dir {
        let candidate = dir.join(&expanded);
        if candidate.exists() || expanded.components().count() > 1 {
            return candidate;
        }
    }
    if let Ok(resolved) = craft_docpath::resolve_absolute(raw, s.project_path.as_deref())
        && resolved.exists()
    {
        return resolved;
    }
    let from_cwd = s.cwd.join(&expanded);
    if from_cwd.exists() { from_cwd } else { workspace_root(s).join(expanded) }
}

pub fn resolve_workspace_path(s: &crate::Session, raw: &str) -> PathBuf {
    resolve_path_in(s, None, raw)
}

pub fn user_data_dir() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        return std::env::var_os("HOME").map(PathBuf::from).filter(|p| p.is_absolute()).map(|h| h.join("Library/Application Support"));
    }
    if cfg!(windows) {
        return std::env::var_os("LOCALAPPDATA").or_else(|| std::env::var_os("APPDATA")).map(PathBuf::from).filter(|p| p.is_absolute());
    }
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from).filter(|path| path.is_absolute()).map(|home| home.join(".local/share")))
}

/// App-level Magic Studio data root (`~/.local/share/magic` / `~/Library/Application Support/magic`).
/// Returns `None` when `MAGIC_CONFIG_DIR` is set to empty (`""`) for isolated test/agent runs (`craft-settings#12`).
pub fn magic_data_dir() -> Option<PathBuf> {
    if std::env::var_os("MAGIC_CONFIG_DIR").is_some_and(|v| v.is_empty()) {
        return None;
    }
    if let Some(explicit) = std::env::var_os("MAGIC_DATA_DIR").filter(|v| !v.is_empty()).map(PathBuf::from).filter(|p| p.is_absolute()) {
        return Some(explicit);
    }
    user_data_dir().map(|base| base.join("magic"))
}

/// Ensure `~/.local/share/magic/{templates,recovery,logs}` exist and seed `templates/storyboard.designcraft`.
pub fn ensure_default_app_folders(seed_storyboard: &[u8]) -> Option<PathBuf> {
    let base = magic_data_dir()?;
    for sub in ["templates", "recovery", "logs"] {
        let _ = std::fs::create_dir_all(base.join(sub));
    }
    let tpl = base.join("templates/storyboard.designcraft");
    if !tpl.is_file() && !seed_storyboard.is_empty() {
        let _ = craft_atomic::atomic_write(&tpl, seed_storyboard);
    }
    Some(base)
}

pub fn resolve_executable_path(name: &str) -> Option<PathBuf> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }
    let stored = crate::project_settings::stored_settings();
    if let Some(cfg) = stored.pointer(&format!("/tools/{trimmed}")).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()) {
        let candidate = expand_tilde(cfg);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    let direct = Path::new(trimmed);
    if direct.components().count() > 1 && direct.is_file() {
        return Some(direct.to_path_buf());
    }
    for dir in std::env::split_paths(&std::env::var_os("PATH")?) {
        let candidate = dir.join(trimmed);
        if candidate.is_file() {
            return Some(candidate);
        }
        if cfg!(windows) {
            let exe = dir.join(format!("{trimmed}.exe"));
            if exe.is_file() {
                return Some(exe);
            }
        }
    }
    None
}

pub fn resolve_tool(name: &str) -> String {
    resolve_executable_path(name).map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|| name.to_string())
}

fn spawn_detached(mut cmd: Command, label: &str) -> Result<()> {
    let mut child =
        cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().map_err(|e| EngineError::ExternalTool(label.to_owned(), e.to_string()))?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

pub fn launch_app_only(app: &str) -> Result<()> {
    let trimmed = app.trim();
    let Some(exe) = resolve_executable_path(trimmed) else {
        return Err(EngineError::InvalidParams(format!("`{trimmed}` is not installed on PATH (configure `[tools]` in config.toml or open a project file)")));
    };
    spawn_detached(Command::new(exe), trimmed)
}

pub fn open_editor(app: &str, document: &Path) -> Result<()> {
    let trimmed = app.trim();
    let mut cmd = if !trimmed.is_empty() && trimmed != "system" && resolve_executable_path(trimmed).is_some() {
        Command::new(resolve_tool(trimmed))
    } else if cfg!(target_os = "macos") {
        Command::new("open")
    } else if cfg!(target_os = "windows") {
        let mut c = Command::new("cmd");
        c.args(["/C", "start", ""]);
        c
    } else {
        Command::new("xdg-open")
    };
    cmd.arg(document);
    spawn_detached(cmd, trimmed)
}

fn infer_project_doc_for_app(s: &crate::Session, app: &str, domain: Option<&str>) -> Option<PathBuf> {
    let proj_dir = s.project_path.as_deref()?.parent()?;
    let candidates: Vec<PathBuf> = match (domain.unwrap_or_default(), app) {
        ("lyrics", _) => vec![proj_dir.join("effects/lyrics.ecproj"), proj_dir.join("lettering/lyrics.ecproj")],
        ("motion", _) => vec![proj_dir.join("effects/motion.ecproj"), proj_dir.join("motion/actions.ecproj")],
        (_, "filmcraft") => {
            let mut v: Vec<PathBuf> = s.video_project_path.iter().cloned().collect();
            if let Ok(rd) = std::fs::read_dir(proj_dir.join("film")) {
                v.extend(rd.flatten().map(|e| e.path()).filter(|p| has_extension(p, &["fcproj"])));
            }
            v
        }
        (_, "effectcraft") => vec![
            proj_dir.join("effects/animation.ecproj"),
            proj_dir.join("effects/lyrics.ecproj"),
            proj_dir.join("effects/motion.ecproj"),
            proj_dir.join("lettering/lyrics.ecproj"),
            proj_dir.join("motion/actions.ecproj"),
        ],
        (_, "designcraft") => vec![proj_dir.join("storyboard.designcraft")],
        (_, "vectorcraft") => ["assets", "assets/vectors", "motion/svg"]
            .into_iter()
            .flat_map(|sub| std::fs::read_dir(proj_dir.join(sub)).into_iter().flatten().flatten().map(|e| e.path()))
            .filter(|p| has_extension(p, &["vectorcraft", "svg"]))
            .collect(),
        (_, "photocraft") => std::fs::read_dir(proj_dir.join("assets"))
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| has_extension(p, &["pcraft", "psd", "png"]))
            .collect(),
        _ => Vec::new(),
    };
    candidates.into_iter().find(|p| p.exists())
}

pub fn cmd_workspace_open(s: &mut crate::Session, params: &Value) -> Result<Value> {
    let app = params.get("app").and_then(Value::as_str).map(str::trim).filter(|a| !a.is_empty()).unwrap_or("system");
    let domain = params.get("domain").and_then(Value::as_str).map(str::trim).filter(|d| !d.is_empty());
    if let Some(raw) = params.get("path").and_then(Value::as_str).map(str::trim).filter(|p| !p.is_empty()) {
        let path = resolve_workspace_path(s, raw);
        if !path.exists() && !magic_util::media::is_file_or_pcraft_bundle(&path) {
            return Err(EngineError::InvalidParams(format!("file does not exist: {}", path.display())));
        }
        open_editor(app, &path)?;
        return Ok(json!({"ok": true, "app": app, "path": path, "document": {"app": app, "file": path}}));
    }
    if let Some(inferred) = infer_project_doc_for_app(s, app, domain) {
        open_editor(app, &inferred)?;
        return Ok(json!({"ok": true, "app": app, "path": inferred, "document": {"app": app, "file": inferred}}));
    }
    if app != "system" {
        launch_app_only(app)?;
        return Ok(json!({"ok": true, "app": app, "launched": true}));
    }
    Err(EngineError::InvalidParams("missing `path` or `app` for workspace.open".into()))
}

pub(crate) const AUDIO_EXTENSIONS: &[&str] = &["wav", "mp3", "flac", "m4a", "aac", "ogg"];

pub(crate) fn has_extension(path: &Path, allowed: &[&str]) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| allowed.iter().any(|a| a.eq_ignore_ascii_case(e)))
}

pub(crate) fn song_audio_path(s: &crate::Session, raw: &str) -> Result<PathBuf> {
    let path = resolve_workspace_path(s, raw);
    if !path.is_file() || !has_extension(&path, AUDIO_EXTENSIONS) {
        return Err(EngineError::InvalidParams(format!("not an audio file: {}", path.display())));
    }
    Ok(path)
}

pub(crate) fn try_sanitize_asset_id(raw: &str) -> Option<String> {
    let mut out = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if matches!(ch, '_' | '-' | ' ') && !out.ends_with('_') {
            out.push('_');
        }
    }
    let trimmed = out.trim_matches('_').to_string();
    (!trimmed.is_empty()).then_some(trimmed)
}

pub fn sanitize_asset_id(raw: &str) -> String {
    try_sanitize_asset_id(raw).unwrap_or_else(|| "asset".to_string())
}

pub fn asset_id_for_import(typed_name: Option<&str>, path: &Path) -> String {
    typed_name
        .and_then(try_sanitize_asset_id)
        .or_else(|| path.file_stem().and_then(|st| st.to_str()).and_then(try_sanitize_asset_id))
        .unwrap_or_else(|| "asset".to_string())
}
