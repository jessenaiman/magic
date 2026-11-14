//! App-level default folders (`~/.local/share/magic/{templates,recovery,logs}`), unified
//! `craft-docpath` path resolution, and OS editor handoff (`workspace.open`).

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use serde_json::{Value, json};

use crate::{EngineError, Result};

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
    let resolved = resolve_path_in(s, None, raw);
    if resolved.exists() {
        return resolved;
    }
    let mapped = match raw.trim() {
        "lettering/lyrics.ecproj" => Some("effects/lyrics.ecproj"),
        "lettering/lyrics.mov" => Some("effects/lyrics.mov"),
        "motion/actions.ecproj" => Some("effects/motion.ecproj"),
        "motion/actions.mov" => Some("effects/motion.mov"),
        _ => None,
    };
    mapped.map_or(resolved, |canonical| resolve_path_in(s, None, canonical))
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

/// Ensure `~/.local/share/magic/{templates,recovery,logs}` exist without writing embedded template blobs.
pub fn ensure_default_app_folders() -> Option<PathBuf> {
    let base = magic_data_dir()?;
    for sub in ["templates", "recovery", "logs"] {
        let _ = std::fs::create_dir_all(base.join(sub));
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
        (_, "designcraft") => vec![proj_dir.join("storyboard.designcraft"), proj_dir.join("storyboard.idml")],
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

pub const CRAFT_APPS: &[(&str, &str, u16)] = &[
    ("designcraft", "designcraft-cli", 7979),
    ("photocraft", "photocraft-cli", 7878),
    ("vectorcraft", "vectorcraft-cli", 7980),
    ("effectcraft", "effectcraft-cli", 9877),
    ("filmcraft", "filmcraft-cli", 9876),
];

const CONTROL_REPLY_LIMIT: u64 = 16 * 1024 * 1024;

pub fn control_endpoint(app: &str) -> Option<(SocketAddr, Option<String>)> {
    let (_, _, default_port) = CRAFT_APPS.iter().find(|(id, _, _)| *id == app)?;
    let prefix = app.to_ascii_uppercase();
    let token = std::env::var(format!("{prefix}_CONTROL_TOKEN")).ok().map(|s| s.trim().to_owned()).filter(|s| !s.is_empty());
    if matches!(app, "filmcraft" | "photocraft") && token.is_none() {
        return None;
    }
    let port = std::env::var(format!("{prefix}_CONTROL_PORT")).ok().and_then(|s| s.trim().parse::<u16>().ok()).unwrap_or(*default_port);
    Some((SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port)), token))
}

pub fn control_probe(app: &str) -> bool {
    let Some((addr, _)) = control_endpoint(app) else {
        return false;
    };
    TcpStream::connect_timeout(&addr, Duration::from_millis(80)).is_ok()
}

pub fn control_call(app: &str, method: &str, params: Value) -> std::result::Result<Value, String> {
    let (addr, token) = control_endpoint(app).ok_or_else(|| format!("{app} --control endpoint is unavailable or requires auth token"))?;
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_millis(500)).map_err(|e| format!("{app} ({addr}): {e}"))?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
    let mut req = json!({"id": 1, "method": method, "params": params});
    if let Some(tok) = token {
        req["token"] = json!(tok);
    }
    let line = serde_json::to_string(&req).map_err(|e| e.to_string())?;
    stream.write_all(line.as_bytes()).map_err(|e| format!("{app} write: {e}"))?;
    stream.write_all(b"\n").map_err(|e| format!("{app} write: {e}"))?;
    let _ = stream.flush();

    let mut reply_line = String::new();
    let mut reader = BufReader::new(stream).take(CONTROL_REPLY_LIMIT);
    reader.read_line(&mut reply_line).map_err(|e| format!("{app} read: {e}"))?;
    if reply_line.trim().is_empty() {
        return Err(format!("{app} closed connection without a reply"));
    }
    let resp: Value = serde_json::from_str(reply_line.trim()).map_err(|e| format!("{app} reply JSON: {e}"))?;
    if resp["ok"] == true {
        Ok(resp.get("result").cloned().unwrap_or(Value::Null))
    } else {
        let msg = resp.pointer("/error/message").and_then(Value::as_str).or_else(|| resp["error"].as_str()).unwrap_or("unknown control error");
        Err(format!("{app} {method}: {msg}"))
    }
}

pub fn control_exec(app: &str, command: &str, params: Value) -> std::result::Result<Value, String> {
    control_call(app, "engine.execute", json!({"command": command, "params": params}))
}

pub fn run_cli<S: AsRef<str>>(cli_name: &str, args: &[S], stdin_text: Option<&str>) -> std::result::Result<String, String> {
    let exe = resolve_executable_path(cli_name).ok_or_else(|| format!("`{cli_name}` is not installed on PATH"))?;
    let mut cmd = Command::new(exe);
    for arg in args {
        cmd.arg(arg.as_ref());
    }
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    if stdin_text.is_some() {
        cmd.stdin(Stdio::piped());
    } else {
        cmd.stdin(Stdio::null());
    }
    let mut child = cmd.spawn().map_err(|e| format!("{cli_name}: {e}"))?;
    if let Some(input) = stdin_text
        && let Some(mut stdin) = child.stdin.take()
    {
        let _ = stdin.write_all(input.as_bytes());
    }
    let out = child.wait_with_output().map_err(|e| format!("{cli_name}: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_owned();
    if !out.status.success() {
        let detail = if !stderr.is_empty() { stderr } else { stdout };
        return Err(format!("{cli_name} failed ({}): {detail}", out.status));
    }
    Ok(stdout)
}

pub fn run_cli_json<S: AsRef<str>>(cli_name: &str, args: &[S], stdin_text: Option<&str>) -> std::result::Result<Value, String> {
    let out = run_cli(cli_name, args, stdin_text)?;
    if out.is_empty() {
        return Ok(json!({"ok": true}));
    }
    serde_json::from_str(&out).or_else(|_| {
        out.lines().rev().find(|l| !l.trim().is_empty()).map_or(Ok(json!({"raw": out})), |last| serde_json::from_str(last).or_else(|_| Ok(json!({"raw": out}))))
    })
}
