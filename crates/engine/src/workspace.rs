//! Workspace path helpers and unified routing to `CraftHub` for external Craft apps.
//!
//! All ArtCraft binary discovery, `--control` desktop launches, and `<product>-cli` command
//! execution delegate to [`crate::CraftHub`] so every Craft product is accessed consistently.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::craft_hub::discovery::{require_executable, resolve_executable_path};
use crate::{CraftHub, CraftTarget, EngineError, Result};

fn control_token_file() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(base.join("magic/control-token"))
}

/// The control token: `MAGIC_CONTROL_TOKEN`, else the private token file the desktop creates.
pub fn control_token() -> Option<String> {
    std::env::var("MAGIC_CONTROL_TOKEN").ok().filter(|t| !t.is_empty()).or_else(|| Some(std::fs::read_to_string(control_token_file()?).ok()?.trim().to_owned()))
}

/// The token for a desktop starting its control port, creating the private file on first use.
pub fn ensure_control_token() -> Result<String> {
    if let Some(token) = control_token() {
        return Ok(token);
    }
    use std::io::{Read, Write};
    let path = control_token_file().ok_or_else(|| EngineError::InvalidParams("no home folder for the control token".into()))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut bytes = [0_u8; 32];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    let token: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&path)?.write_all(token.as_bytes())?;
    }
    #[cfg(not(unix))]
    {
        craft_atomic::atomic_write(&path, token.as_bytes())?;
    }
    Ok(token)
}

/// Removes credentials this engine holds from a child process's environment.
pub fn scrub_inherited_credentials(command: &mut std::process::Command) {
    for (key, _) in std::env::vars_os() {
        let Some(name) = key.to_str() else { continue };
        if name == "MAGIC_CONTROL_TOKEN"
            || name.starts_with("CRAFT_AI_SECRET_")
            || name == "OPENAI_API_KEY"
            || name == "OPENAI_ORG"
            || name == "OPENAI_PROJECT"
            || name == "CODEX_API_KEY"
            || name == "ANTHROPIC_API_KEY"
            || name == "GEMINI_API_KEY"
            || name == "GOOGLE_API_KEY"
            || name == "GOOGLE_GENAI_USE_VERTEXAI"
        {
            command.env_remove(name);
        }
    }
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

/// Resolve a potentially relative path against the session workspace root.
pub fn resolve_workspace_path(s: &crate::Session, raw: &str) -> PathBuf {
    if let Ok(resolved) = craft_docpath::resolve_absolute(raw, s.project_path.as_deref())
        && resolved.exists()
    {
        return resolved;
    }
    let p = Path::new(raw);
    if p.is_absolute() {
        return p.to_path_buf();
    }
    let from_cwd = s.cwd.join(p);
    if from_cwd.exists() {
        return from_cwd;
    }
    workspace_root(s).join(p)
}

/// The user's portable data directory; relative XDG values are not valid roots.
pub(crate) fn user_data_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from).filter(|path| path.is_absolute()).map(|home| home.join(".local/share")))
}

/// Resolve an installed executable via `craft_hub::discovery`.
pub(crate) fn find_executable(name: &str) -> Result<PathBuf> {
    require_executable(name)
}

/// Hand off a document for editing. ArtCraft desktop apps always launch or focus with `--control`
/// enabled via [`CraftHub::open_live`] so live designer edits and CLI `--bridge` calls share one window.
pub(crate) fn open_editor(app: &str, document: &Path) -> Result<()> {
    if let Some(target) = CraftTarget::from_name(app) {
        let _ = CraftHub::open_live(target, document)?;
        return Ok(());
    }
    use std::process::{Command, Stdio};
    let mut cmd = Command::new(resolve_tool(app));
    scrub_inherited_credentials(&mut cmd);
    let mut editor = cmd
        .arg(document)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| EngineError::ExternalTool(app.to_owned(), e.to_string()))?;
    std::thread::spawn(move || {
        let _ = editor.wait();
    });
    Ok(())
}

fn find_first_with_ext(dir: &Path, exts: &[&str], depth: usize) -> Option<PathBuf> {
    if depth == 0 {
        return None;
    }
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| !p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with('.') || n == "node_modules" || n == "frames"))
        .collect();
    entries.sort();
    for entry in &entries {
        if entry.is_file() && has_extension(entry, exts) {
            return Some(entry.clone());
        }
    }
    for entry in &entries {
        if entry.is_dir()
            && let Some(found) = find_first_with_ext(entry, exts, depth - 1)
        {
            return Some(found);
        }
    }
    None
}

fn find_project_craft_doc(s: &crate::Session, target: CraftTarget) -> Option<PathBuf> {
    let dir = s.project_path.as_deref()?.parent()?;
    match target {
        CraftTarget::Design => [dir.join("storyboard.designcraft"), dir.join("storyboard/storyboard.designcraft")]
            .into_iter()
            .find(|p| p.is_file())
            .or_else(|| find_first_with_ext(dir, &["designcraft"], 3)),
        CraftTarget::Deck => find_first_with_ext(dir, &["deckcraft", "pptx"], 3),
        CraftTarget::Effect => {
            let lyrics = dir.join("lettering/lyrics.ecproj");
            lyrics.is_file().then_some(lyrics).or_else(|| find_first_with_ext(dir, &["ecproj"], 3))
        }
        CraftTarget::Film => {
            let stem = s.project_path.as_deref().and_then(|p| p.file_stem()).and_then(|n| n.to_str()).unwrap_or("Film");
            let primary = dir.join("film").join(format!("{stem}.fcproj"));
            primary
                .is_file()
                .then_some(primary)
                .or_else(|| crate::cmd::project::workshop::video_project_path(dir))
                .or_else(|| find_first_with_ext(dir, &["fcproj"], 3))
        }
        CraftTarget::Photo => find_first_with_ext(dir, &["pcraft", "psd"], 4),
        CraftTarget::Vector => find_first_with_ext(dir, &["vectorcraft", "vcraft", "drawcraft"], 4),
        CraftTarget::Ani => find_first_with_ext(dir, &["anicraft", "ani"], 4),
        CraftTarget::Texture => find_first_with_ext(dir, &["tcraft", "mcraft"], 4),
    }
}

fn craft_doc_info(path: &Path) -> Option<Value> {
    let bytes = std::fs::read(path).ok()?;
    let info = craft_doc::info(&bytes, &craft_doc::Limits::default()).ok()?;
    if !info.supported {
        return Some(json!({
            "kind": info.kind.extension(),
            "version": info.version,
            "generator": info.generator,
            "supported": false,
            "error": "document version is newer than supported craft-doc schema",
        }));
    }
    Some(json!({
        "kind": info.kind.extension(),
        "version": info.version,
        "generator": info.generator,
        "supported": info.supported,
    }))
}

/// Open a project file in the Craft app that owns its format (with `--control` enabled), launch a
/// Craft app directly, or invoke the system opener.
pub fn cmd_workspace_open(s: &mut crate::Session, params: &Value) -> Result<Value> {
    let ws_dir = s.project_path.as_deref().and_then(Path::parent);
    if params.get("path").and_then(Value::as_str).is_none_or(|p| p.trim().is_empty())
        && let Some(app) = params.get("app").and_then(Value::as_str).map(str::trim).filter(|a| !a.is_empty())
    {
        let target = CraftTarget::from_name(app).ok_or_else(|| EngineError::InvalidParams(format!("unsupported Craft app '{app}'")))?;
        let doc = find_project_craft_doc(s, target);
        return CraftHub::launch_with_control(target, doc.as_deref(), None, ws_dir);
    }
    let raw = params["path"].as_str().filter(|p| !p.trim().is_empty()).ok_or_else(|| EngineError::InvalidParams("missing 'path'".into()))?;
    let path = resolve_workspace_path(s, raw);
    let requested_app = params.get("app").and_then(Value::as_str).map(str::trim).filter(|a| !a.is_empty());
    if let Some(app) = requested_app {
        if let Some(target) = CraftTarget::from_name(app) {
            let resolved_target = if target == CraftTarget::Photo && path.with_extension("pcraft").is_file() {
                path.with_extension("pcraft")
            } else if target == CraftTarget::Vector && path.with_extension("vectorcraft").is_file() {
                path.with_extension("vectorcraft")
            } else {
                path
            };
            return CraftHub::launch_with_control(target, Some(&resolved_target), None, ws_dir);
        }
        return Err(EngineError::InvalidParams(format!("unsupported Craft app '{app}'")));
    }
    if let Some(target) = CraftTarget::from_path(&path) {
        return CraftHub::launch_with_control(target, Some(&path), None, ws_dir);
    }
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase();
    let app = match ext.as_str() {
        "cadcraft" | "dxf" | "dwg" => "cadcraft",
        "mcraft" => "materialcraft",
        "solidcraft" | "step" | "stp" => "solidcraft",
        "soundcraft" => "soundcraft",
        "wordcraft" | "docx" => "wordcraft",
        "glb" | "gltf" | "fbx" | "obj" => "texturecraft",
        _ => "xdg-open",
    };
    open_editor(app, &path)?;
    Ok(json!({"ok": true, "app": app, "path": path}))
}

fn parse_step_pairs(p: &Value, default_cmd: Option<&str>) -> Result<Vec<(String, Value)>> {
    let steps: Vec<Value> = if let Some(arr) = p["steps"].as_array() {
        arr.clone()
    } else if let Some(id) = p["id"].as_str().or_else(|| p["command"].as_str()) {
        vec![json!({"id": id, "params": p.get("params").cloned().unwrap_or_else(|| json!({}))})]
    } else if let Some(def) = default_cmd {
        vec![json!({"id": def, "params": {}})]
    } else {
        return Err(EngineError::InvalidParams("id=<command> or steps=[...] is required".into()));
    };
    let mut out = Vec::with_capacity(steps.len());
    for step in &steps {
        let id = step["id"].as_str().or_else(|| step["command"].as_str()).ok_or_else(|| EngineError::InvalidParams("step missing command id".into()))?;
        let params = step.get("params").cloned().unwrap_or_else(|| json!({}));
        out.push((id.to_owned(), params));
    }
    Ok(out)
}

/// Run PhotoCraft commands via `CraftHub` (live `photocraft --control` window when open in-place,
/// else installed `photocraft-cli`).
pub fn cmd_photo_exec(s: &mut crate::Session, p: &Value) -> Result<Value> {
    let raw = p["path"].as_str().ok_or_else(|| EngineError::InvalidParams("path is required".into()))?;
    let path = resolve_workspace_path(s, raw);
    let owned = parse_step_pairs(p, Some("document.inspect"))?;
    let is_query = owned.len() == 1 && matches!(owned[0].0.as_str(), "document.inspect" | "session.inspect" | "command.list" | "info");
    let save = p["save"].as_str().map(|out| resolve_workspace_path(s, out)).or_else(|| (!is_query).then(|| path.with_extension("pcraft")));
    let pairs: Vec<(&str, Value)> = owned.iter().map(|(id, params)| (id.as_str(), params.clone())).collect();
    let results = CraftHub::run_batch(CraftTarget::Photo, Some(&path), save.as_deref(), &pairs)?;
    let info = save.as_deref().or(Some(path.as_path())).and_then(craft_doc_info);
    Ok(json!({"ok": true, "path": path, "pcraft": save, "info": info, "results": results, "via": "photocraft"}))
}

/// Run EffectCraft commands via `CraftHub` (live `effectcraft --control` window when open, else
/// installed `effectcraft-cli`).
pub fn cmd_effect_exec(s: &mut crate::Session, p: &Value) -> Result<Value> {
    let raw = p["path"].as_str().ok_or_else(|| EngineError::InvalidParams("path=<.ecproj> is required".into()))?;
    let path = resolve_workspace_path(s, raw);
    let steps: Vec<Value> = if let Some(arr) = p["steps"].as_array() {
        arr.clone()
    } else if let Some(id) = p["id"].as_str().or_else(|| p["command"].as_str()) {
        vec![json!({"id": id, "params": p.get("params").cloned().unwrap_or_else(|| json!({}))})]
    } else {
        return Err(EngineError::InvalidParams("id=<effectcraft command> or steps=[...] is required".into()));
    };
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let results = crate::cmd::character::effectcraft(dir, Some(&path), &steps, p["save"] == true)?;
    let info = craft_doc_info(&path);
    Ok(json!({"ok": true, "path": path, "info": info, "results": results, "via": "effectcraft"}))
}

/// Run VectorCraft commands via `CraftHub` (live `vectorcraft --control` window when open in-place,
/// else installed `vectorcraft-cli`).
pub fn cmd_vector_exec(s: &mut crate::Session, p: &Value) -> Result<Value> {
    let raw = p["path"].as_str().ok_or_else(|| EngineError::InvalidParams("path is required".into()))?;
    let path = resolve_workspace_path(s, raw);
    let owned = parse_step_pairs(p, Some("document.inspect"))?;
    let is_query = owned.len() == 1 && matches!(owned[0].0.as_str(), "document.inspect" | "session.inspect" | "command.list");
    let save = p["save"]
        .as_str()
        .or_else(|| p["out"].as_str())
        .map(|o| resolve_workspace_path(s, o))
        .or_else(|| (!is_query).then(|| path.with_extension("vectorcraft")));
    let pairs: Vec<(&str, Value)> = owned.iter().map(|(id, params)| (id.as_str(), params.clone())).collect();
    let results = CraftHub::run_batch(CraftTarget::Vector, path.is_file().then_some(path.as_path()), save.as_deref(), &pairs)?;
    let info = save.as_deref().or(Some(path.as_path())).and_then(craft_doc_info);
    Ok(json!({"ok": true, "path": path, "vectorcraft": save, "info": info, "results": results, "via": "vectorcraft"}))
}

/// Resolve an installed tool executable from `<TOOL_NAME>` env override, `config.toml`, ArtCraft
/// Launcher `.desktop` entry, `PATH`, or `~/.local/bin/<name>`.
pub fn resolve_tool(name: &str) -> String {
    resolve_executable_path(name).map_or_else(|| name.to_string(), |p| p.to_string_lossy().into_owned())
}

pub(crate) struct ToolSlotGuard;
impl Drop for ToolSlotGuard {
    fn drop(&mut self) {
        let (lock, cvar) = tool_limiter();
        if let Ok(mut active) = lock.lock() {
            *active = active.saturating_sub(1);
            cvar.notify_one();
        }
    }
}

fn tool_limiter() -> &'static (std::sync::Mutex<usize>, std::sync::Condvar) {
    static LIMITER: std::sync::OnceLock<(std::sync::Mutex<usize>, std::sync::Condvar)> = std::sync::OnceLock::new();
    LIMITER.get_or_init(|| (std::sync::Mutex::new(0), std::sync::Condvar::new()))
}

pub(crate) fn acquire_tool_slot() -> ToolSlotGuard {
    let (lock, cvar) = tool_limiter();
    if let Ok(mut active) = lock.lock() {
        while *active >= 4 {
            active = match cvar.wait(active) {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
        }
        *active += 1;
    }
    ToolSlotGuard
}

/// Audio file extensions recognized by the workshop.
pub(crate) const AUDIO_EXTENSIONS: &[&str] = &["wav", "mp3", "flac", "m4a", "aac", "ogg"];

/// True when `path` has one of `allowed` extensions (case-insensitive).
pub(crate) fn has_extension(path: &Path, allowed: &[&str]) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| allowed.iter().any(|a| a.eq_ignore_ascii_case(e)))
}

/// Validate and resolve an audio path for the workshop song.
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

/// Sanitize a user-supplied or file-stem name into a stable asset id.
pub fn sanitize_asset_id(raw: &str) -> String {
    try_sanitize_asset_id(raw).unwrap_or_else(|| "asset".to_string())
}

/// Derive a sanitized asset id from an optional user-typed name, falling back to the file stem
/// only when no name is typed.
pub fn asset_id_for_import(typed_name: Option<&str>, path: &Path) -> String {
    typed_name
        .and_then(try_sanitize_asset_id)
        .or_else(|| path.file_stem().and_then(|st| st.to_str()).and_then(try_sanitize_asset_id))
        .unwrap_or_else(|| "asset".to_string())
}
