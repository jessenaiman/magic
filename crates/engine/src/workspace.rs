//! Workspace path resolution and OS file handoff (`workspace.open`).
//!
//! Following the ArtCraft cross-editor file pattern (`photocraft/docs/embedded-vector-editing.md`),
//! Magic Studio hands project files to external editors on disk without opening cross-app `--control`
//! sockets or generating sibling token files, and watches saved file changes via `link_watch`.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{Value, json};

use crate::{EngineError, Result};

/// Optional bearer token from `MAGIC_CONTROL_TOKEN` when explicitly configured by the user.
pub fn control_token() -> Option<String> {
    std::env::var("MAGIC_CONTROL_TOKEN")
        .ok()
        .map(|t| t.trim().to_owned())
        .filter(|t| !t.is_empty())
}

pub fn workspace_root(s: &crate::Session) -> PathBuf {
    if let Ok(exe) = std::env::current_exe()
        && let Some(home) = exe.parent().and_then(|p| p.parent())
    {
        let runtime = home.join("runtime");
        if runtime.join("installation.json").is_file() {
            return runtime;
        }
    }
    let current_root = |root: &Path| root.join("Cargo.toml").is_file() && root.join("crates/engine/Cargo.toml").is_file();
    let manifest_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    if let Ok(canon) = manifest_root.canonicalize()
        && current_root(&canon)
    {
        return canon;
    }
    s.cwd.clone()
}

/// Expand a leading `~` or `~/` segment against `HOME` (or `USERPROFILE` on Windows).
pub fn expand_tilde(raw: &str) -> PathBuf {
    let trimmed = raw.trim();
    if trimmed == "~" {
        if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
            return PathBuf::from(home);
        }
    } else if let Some(rest) = trimmed.strip_prefix("~/").or_else(|| trimmed.strip_prefix("~\\"))
        && let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))
    {
        return PathBuf::from(home).join(rest);
    }
    PathBuf::from(trimmed)
}

/// Resolve a potentially relative path against the session workspace root.
pub fn resolve_workspace_path(s: &crate::Session, raw: &str) -> PathBuf {
    let expanded = expand_tilde(raw);
    if expanded.is_absolute() {
        return expanded;
    }
    if let Ok(resolved) = craft_docpath::resolve_absolute(raw, s.project_path.as_deref())
        && resolved.exists()
    {
        return resolved;
    }
    let from_cwd = s.cwd.join(&expanded);
    if from_cwd.exists() {
        return from_cwd;
    }
    workspace_root(s).join(expanded)
}

/// The user's portable data directory; relative XDG values are not valid roots.
pub(crate) fn user_data_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .filter(|path| path.is_absolute())
                .map(|home| home.join(".local/share"))
        })
}

/// Resolve an installed executable via `config.toml` `[tools]` or `PATH`.
pub fn resolve_executable_path(name: &str) -> Option<PathBuf> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }
    let stored = crate::project_settings::stored_settings();
    if let Some(cfg) = stored
        .pointer(&format!("/tools/{trimmed}"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let candidate = expand_tilde(cfg);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    let direct = Path::new(trimmed);
    if direct.components().count() > 1 && direct.is_file() {
        return Some(direct.to_path_buf());
    }
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
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

pub(crate) fn find_executable(name: &str) -> Result<PathBuf> {
    resolve_executable_path(name).ok_or_else(|| {
        EngineError::ExternalTool(name.to_owned(), "executable not found on PATH".into())
    })
}

pub fn resolve_tool(name: &str) -> String {
    resolve_executable_path(name)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| name.to_string())
}

/// Open a document on disk using an optional app binary or the platform default file opener,
/// without passing any `--control` or `--bridge` flags.
pub fn open_editor(app: &str, document: &Path) -> Result<()> {
    let trimmed = app.trim();
    let mut cmd = if !trimmed.is_empty() && trimmed != "system" && resolve_executable_path(trimmed).is_some() {
        let mut c = Command::new(resolve_tool(trimmed));
        c.arg(document);
        c
    } else if cfg!(target_os = "macos") {
        let mut c = Command::new("open");
        c.arg(document);
        c
    } else if cfg!(target_os = "windows") {
        let mut c = Command::new("cmd");
        c.args(["/C", "start", ""]).arg(document);
        c
    } else {
        let mut c = Command::new("xdg-open");
        c.arg(document);
        c
    };
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| EngineError::ExternalTool(trimmed.to_owned(), e.to_string()))?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

/// Open a project file in its native desktop editor on disk (`workspace.open`).
pub fn cmd_workspace_open(s: &mut crate::Session, params: &Value) -> Result<Value> {
    let raw = params["path"]
        .as_str()
        .filter(|p| !p.trim().is_empty())
        .ok_or_else(|| EngineError::InvalidParams("missing `path`".into()))?;
    let path = resolve_workspace_path(s, raw);
    if !magic_util::media::is_craft_doc_path(&path) {
        return Err(EngineError::InvalidParams(format!("file does not exist: {}", path.display())));
    }
    let app = params
        .get("app")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|a| !a.is_empty())
        .unwrap_or("system");
    if app.eq_ignore_ascii_case("anicraft") {
        return Err(EngineError::InvalidParams(
            "AniCraft desktop UI is disabled while in early alpha".into(),
        ));
    }
    open_editor(app, &path)?;
    Ok(json!({
        "ok": true,
        "app": app,
        "path": path,
        "document": {"app": app, "file": path}
    }))
}

pub(crate) const AUDIO_EXTENSIONS: &[&str] = &["wav", "mp3", "flac", "m4a", "aac", "ogg"];

pub(crate) fn has_extension(path: &Path, allowed: &[&str]) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| allowed.iter().any(|a| a.eq_ignore_ascii_case(e)))
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
        } else if (ch == '_' || ch == '-' || ch == ' ') && !out.ends_with('_') {
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
