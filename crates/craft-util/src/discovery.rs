//! ArtCraft Launcher (`storytold/craft-launcher`) binary discovery, loopback socket resolution,
//! `0600` session token management via `craft-atomic`, and child environment scrubbing.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

use crate::target::{ControlAuth, CraftTarget};
use crate::{Result, UtilError};

const SCRUBBED_CREDENTIAL_ENVS: &[&str] = &[
    "MAGIC_CONTROL_TOKEN",
    "OPENAI_API_KEY",
    "ANTHROPIC_API_KEY",
    "GEMINI_API_KEY",
    "GOOGLE_API_KEY",
    "FAL_KEY",
    "REPLICATE_API_TOKEN",
    "ELEVENLABS_API_KEY",
    "TRIPO_API_KEY",
    "MESHY_API_KEY",
];

/// Strip paid API and parent control secrets from child tool processes.
pub fn scrub_credentials(cmd: &mut Command) {
    for key in SCRUBBED_CREDENTIAL_ENVS {
        cmd.env_remove(key);
    }
    for target in CraftTarget::ALL {
        cmd.env_remove(target.token_env());
    }
    cmd.env_remove("PHOTOCRAFT_CONTROL_TOKEN_FILE");
}

pub fn is_executable(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        path.metadata().is_ok_and(|meta| meta.permissions().mode() & 0o111 != 0)
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// Inspect `~/.local/share/artcraft-launcher/installed.json` written by `storytold/craft-launcher`.
fn launcher_installed_binary(home: &Path, name: &str) -> Option<PathBuf> {
    let manifest = home.join(".local/share/artcraft-launcher/installed.json");
    let raw = std::fs::read_to_string(manifest).ok()?;
    let doc: Value = serde_json::from_str(&raw).ok()?;
    let app_base = name.strip_suffix("-cli").unwrap_or(name);
    let entries = doc.get("apps").or(Some(&doc))?;
    let item = entries.get(app_base).or_else(|| entries.as_array()?.iter().find(|v| v["id"].as_str() == Some(app_base)))?;
    for key in ["binary", "bin", "exec", "path", "install_dir", "dir"] {
        if let Some(s) = item.get(key).and_then(Value::as_str) {
            let p = PathBuf::from(s);
            let cand = if p.is_dir() {
                p.join(name)
            } else if name.ends_with("-cli") {
                p.parent()?.join(name)
            } else {
                p
            };
            if is_executable(&cand) {
                return Some(cand);
            }
        }
    }
    None
}

/// Parse `Exec=` registered by `craft-launcher` in `~/.local/share/applications/ai.storyteller.<app>.desktop`.
fn launcher_desktop_binary(home: &Path, app_name: &str) -> Option<PathBuf> {
    let entry = home.join(".local/share/applications").join(format!("ai.storyteller.{app_name}.desktop"));
    let text = std::fs::read_to_string(entry).ok()?;
    let exec = text.lines().find_map(|l| l.strip_prefix("Exec="))?.trim();
    let program = match exec.strip_prefix('"') {
        Some(rest) => rest.split('"').next()?,
        None => exec.split_whitespace().next()?,
    };
    let path = PathBuf::from(program);
    is_executable(&path).then_some(path)
}

/// Resolve a system-installed executable via env override, optional user config, ArtCraft Launcher
/// (`installed.json` + `.desktop`), `PATH`, `~/.local/bin`, or `/usr/bin`.
pub fn resolve_executable(name: &str, config_override: Option<&str>) -> Option<PathBuf> {
    let env_key = name.to_ascii_uppercase().replace('-', "_");
    if let Ok(val) = std::env::var(&env_key) {
        let candidate = PathBuf::from(val.trim());
        if is_executable(&candidate) {
            return std::path::absolute(candidate).ok();
        }
    }
    if let Some(cfg) = config_override.map(str::trim).filter(|s| !s.is_empty()) {
        let candidate = PathBuf::from(cfg);
        if is_executable(&candidate) {
            return std::path::absolute(candidate).ok();
        }
    }
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        if let Some(found) = launcher_installed_binary(&home, name) {
            return Some(found);
        }
        if let Some(app_bin) = launcher_desktop_binary(&home, name) {
            return Some(app_bin);
        }
        if let Some(app_base) = name.strip_suffix("-cli")
            && let Some(app_bin) = launcher_desktop_binary(&home, app_base)
            && let Some(sibling) = app_bin.parent().map(|dir| dir.join(name))
            && is_executable(&sibling)
        {
            return Some(sibling);
        }
    }
    let direct = Path::new(name);
    if direct.components().count() > 1 && is_executable(direct) {
        return std::path::absolute(direct).ok();
    }
    if let Some(found) = std::env::var_os("PATH").and_then(|paths| std::env::split_paths(&paths).map(|d| d.join(name)).find(|p| is_executable(p))) {
        return std::path::absolute(found).ok();
    }
    if let Some(local) = std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/bin").join(name)).filter(|p| is_executable(p)) {
        return Some(local);
    }
    let usr = PathBuf::from("/usr/bin").join(name);
    is_executable(&usr).then_some(usr)
}

pub fn detect_craft_launcher() -> Option<PathBuf> {
    if let Some(bin) = resolve_executable("craft-launcher", None) {
        return Some(bin);
    }
    let home = PathBuf::from(std::env::var_os("HOME")?);
    let installed = home.join(".local/share/artcraft-launcher/installed.json");
    if installed.is_file() {
        return Some(installed);
    }
    for desktop_id in ["ai.storyteller.craft-launcher.desktop", "craft-launcher.desktop"] {
        let entry = home.join(".local/share/applications").join(desktop_id);
        if entry.is_file() {
            return Some(entry);
        }
    }
    None
}

pub fn control_addr(target: CraftTarget, configured_port: Option<u16>) -> SocketAddr {
    if let Ok(raw) = std::env::var(target.port_env())
        && let Ok(port) = raw.trim().parse::<u16>()
        && port > 0
    {
        return SocketAddr::from(([127, 0, 0, 1], port));
    }
    let port = configured_port.filter(|&p| p > 0).unwrap_or_else(|| target.default_port());
    SocketAddr::from(([127, 0, 0, 1], port))
}

fn config_base_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
}

pub fn token_file_path(target: CraftTarget, port: u16) -> Option<PathBuf> {
    if target == CraftTarget::Photo
        && let Ok(custom) = std::env::var("PHOTOCRAFT_CONTROL_TOKEN_FILE")
        && !custom.trim().is_empty()
    {
        return Some(PathBuf::from(custom.trim()));
    }
    if target == CraftTarget::Texture {
        let native = config_base_dir()?.join("texturecraft/control.token");
        if native.is_file() {
            return Some(native);
        }
    }
    Some(config_base_dir()?.join("magic/craft-tokens").join(format!("{}-{port}.token", target.name())))
}

pub fn read_control_token(target: CraftTarget, port: u16) -> Option<String> {
    if target.auth() == ControlAuth::None {
        return None;
    }
    if let Ok(env_tok) = std::env::var(target.token_env()) {
        let trimmed = env_tok.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_owned());
        }
    }
    let path = token_file_path(target, port)?;
    std::fs::read_to_string(path).ok().map(|s| s.trim().to_owned()).filter(|s| !s.is_empty())
}

pub fn ensure_control_token(target: CraftTarget, port: u16) -> Result<(Option<String>, Option<PathBuf>)> {
    if target.auth() == ControlAuth::None {
        return Ok((None, None));
    }
    let file_path = token_file_path(target, port);
    if let Some(existing) = read_control_token(target, port) {
        if let Some(ref path) = file_path
            && !path.is_file()
        {
            write_private_token(path, &existing)?;
        }
        return Ok((Some(existing), file_path));
    }
    let path = file_path.ok_or_else(|| UtilError::Invalid("cannot determine config directory for Craft control token".into()))?;
    let token = generate_hex_token()?;
    write_private_token(&path, &token)?;
    Ok((Some(token), Some(path)))
}

fn generate_hex_token() -> Result<String> {
    use std::io::Read;
    let mut bytes = [0_u8; 32];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

fn write_private_token(path: &Path, token: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    craft_atomic::WriteOptions::default()
        .with_new_file_permissions(craft_atomic::NewFilePermissions::Private)
        .atomic_write(path, token.as_bytes())
        .map_err(|e| UtilError::Io(std::io::Error::other(e.to_string())))
}

/// Build a scrubbed `Command` for a Craft desktop or `<product>-cli` binary.
/// Per `photocraft/docs/control-protocol.md`, passing both `PHOTOCRAFT_CONTROL_TOKEN` and
/// `--control-token-file` is an immediate exit-status-2 usage error, so `scrub_credentials`
/// always clears both env vars before `launch` passes `--control-token-file`.
pub fn scrubbed_command(bin: &Path, target: CraftTarget, port: Option<u16>) -> Command {
    let mut cmd = Command::new(bin);
    scrub_credentials(&mut cmd);
    if target == CraftTarget::Film {
        let p = port.unwrap_or_else(|| target.default_port());
        if let Some(tok) = read_control_token(target, p) {
            cmd.env(target.token_env(), tok);
        }
    }
    cmd
}

pub fn same_canonical_path(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => a == b,
    }
}
