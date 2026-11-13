//! Local ArtCraft extension & theme directory inspection (`storytold/craft-extensions` rules)
//! and validated manifest loading for `extensions/scribbles/` and `AnimationHost`/`AssetHost` plugins.
//!
//! Enforces `storytold/craft-extensions` invariants without adding a provisional crate dependency:
//! - Every manifest and data file read from disk is hostile input: byte size capped (`<= 256 KiB`)
//!   before parsing, strict character-class checks on `id` and relative `files` paths, and zero
//!   path traversal or symlink escape outside the extension directory.
//! - A package never states its own trust level; the host assigns [`TrustTier`] strictly from origin.
//! - Themes carry no code; plug-in `permissions` must be empty (`[]`) in v1; third-party marketplace
//!   fetching is switched off and this module never touches the network.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use serde_json::{Value, json};

use crate::{EngineError, Result, Session};

/// Hard pre-parse size ceiling (`256 KiB`) for any extension manifest or JSON resource file.
pub const MAX_EXT_FILE_BYTES: u64 = 256 * 1024;

/// Host-assigned extension trust tier (`storytold/craft-extensions`: packages never declare their own trust).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrustTier {
    Bundled,
    SharedStore,
    UserConfig,
    ProjectLocal,
}

impl TrustTier {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bundled => "bundled",
            Self::SharedStore => "shared_store",
            Self::UserConfig => "user_config",
            Self::ProjectLocal => "project_local",
        }
    }
}

/// Validated on-disk extension manifest with host-assigned [`TrustTier`].
#[derive(Clone, Debug)]
pub struct ValidatedExtension {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub version: String,
    pub role: String,
    pub trust: TrustTier,
    pub dir: PathBuf,
    pub files: BTreeMap<String, PathBuf>,
    pub raw: Value,
}

fn bad(msg: impl Into<String>) -> EngineError {
    EngineError::InvalidParams(msg.into())
}

/// Validate extension package ID (`[a-z0-9._-]+`, starts with `[a-z0-9]`, `<= 96` bytes, no `..`).
pub fn is_valid_extension_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 96
        && !id.contains("..")
        && (bytes[0].is_ascii_lowercase() || bytes[0].is_ascii_digit())
        && bytes.iter().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(*b, b'.' | b'_' | b'-'))
}

/// Validate a relative file path inside an extension folder by character class and path components.
pub fn validate_relative_ext_path(dir: &Path, rel: &str) -> Result<PathBuf> {
    if rel.is_empty() || rel.len() > 180 || rel.starts_with('/') || rel.contains('\\') || rel.contains("..") {
        return Err(bad(format!("invalid extension relative path `{rel}`")));
    }
    if !rel.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-' | b'/')) {
        return Err(bad(format!("extension path `{rel}` contains disallowed characters")));
    }
    let rel_path = Path::new(rel);
    if rel_path.is_absolute() || !rel_path.components().all(|c| matches!(c, Component::Normal(_))) {
        return Err(bad(format!("extension path `{rel}` may not traverse directories")));
    }
    let canon_dir = dir.canonicalize().map_err(|e| bad(format!("{}: {e}", dir.display())))?;
    let candidate = canon_dir.join(rel_path);
    let sym = std::fs::symlink_metadata(&candidate).map_err(|e| bad(format!("{}: {e}", candidate.display())))?;
    if sym.file_type().is_symlink() || !sym.is_file() {
        return Err(bad(format!("extension file `{rel}` must be a regular file inside {}", canon_dir.display())));
    }
    let canon_file = candidate.canonicalize().map_err(|e| bad(format!("{}: {e}", candidate.display())))?;
    if !canon_file.starts_with(&canon_dir) {
        return Err(bad(format!("extension file `{rel}` escapes extension root")));
    }
    Ok(canon_file)
}

/// Bounded file reader enforcing [`MAX_EXT_FILE_BYTES`] before JSON parsing.
pub fn read_bounded_json(path: &Path) -> Result<Value> {
    let meta = std::fs::symlink_metadata(path).map_err(|e| bad(format!("{}: {e}", path.display())))?;
    if meta.file_type().is_symlink() || !meta.is_file() {
        return Err(bad(format!("{} must be a regular file", path.display())));
    }
    if meta.len() > MAX_EXT_FILE_BYTES {
        return Err(bad(format!("{} exceeds {} byte hostile-input cap", path.display(), MAX_EXT_FILE_BYTES)));
    }
    let file = std::fs::File::open(path).map_err(|e| bad(format!("{}: {e}", path.display())))?;
    let mut buf = Vec::with_capacity((meta.len() as usize).min(MAX_EXT_FILE_BYTES as usize));
    file.take(MAX_EXT_FILE_BYTES + 1).read_to_end(&mut buf).map_err(|e| bad(format!("{}: {e}", path.display())))?;
    if buf.len() as u64 > MAX_EXT_FILE_BYTES {
        return Err(bad(format!("{} exceeds {} byte hostile-input cap", path.display(), MAX_EXT_FILE_BYTES)));
    }
    serde_json::from_slice(&buf).map_err(|e| bad(format!("{}: {e}", path.display())))
}

/// Parse and validate `manifest.json` inside `dir`, assigning `trust` strictly from host origin.
pub fn load_validated_manifest(dir: &Path, trust: TrustTier) -> Result<ValidatedExtension> {
    let manifest_path = validate_relative_ext_path(dir, "manifest.json")?;
    let raw = read_bounded_json(&manifest_path)?;
    let obj = raw.as_object().ok_or_else(|| bad(format!("{}: manifest must be a JSON object", manifest_path.display())))?;

    for forbidden_trust_key in ["trust", "trust_tier", "trustLevel", "trust_level"] {
        if obj.contains_key(forbidden_trust_key) {
            return Err(bad(format!("{}: package may never state its own `{forbidden_trust_key}`", manifest_path.display())));
        }
    }
    if let Some(perms) = obj.get("permissions") {
        let Some(arr) = perms.as_array() else {
            return Err(bad(format!("{}: `permissions` must be an array", manifest_path.display())));
        };
        if !arr.is_empty() {
            return Err(bad(format!("{}: plug-in `permissions` list must be empty in v1", manifest_path.display())));
        }
    }

    let id = obj.get("id").and_then(Value::as_str).unwrap_or_default();
    if !is_valid_extension_id(id) {
        return Err(bad(format!("{}: invalid extension id `{id}`", manifest_path.display())));
    }
    let kind = obj.get("kind").and_then(Value::as_str).unwrap_or("plugin").to_owned();
    if !matches!(kind.as_str(), "plugin" | "theme" | "resource") {
        return Err(bad(format!("{}: unsupported extension kind `{kind}`", manifest_path.display())));
    }
    if kind == "theme" {
        for code_key in ["provider_command", "command", "entry", "executable", "script", "wasm"] {
            if obj.get(code_key).is_some_and(|v| !v.is_null() && v.as_str() != Some("")) {
                return Err(bad(format!("{}: themes carry no code (`{code_key}` forbidden)", manifest_path.display())));
            }
        }
    }

    let mut files = BTreeMap::new();
    if let Some(files_obj) = obj.get("files").and_then(Value::as_object) {
        if files_obj.len() > 32 {
            return Err(bad(format!("{}: too many declared files (max 32)", manifest_path.display())));
        }
        for (slot, rel_val) in files_obj {
            if slot.is_empty() || slot.len() > 64 || !slot.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-')) {
                return Err(bad(format!("{}: invalid file slot `{slot}`", manifest_path.display())));
            }
            let rel = rel_val.as_str().ok_or_else(|| bad(format!("{}: file `{slot}` path must be a string", manifest_path.display())))?;
            files.insert(slot.clone(), validate_relative_ext_path(dir, rel)?);
        }
    }

    let canon_dir = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    Ok(ValidatedExtension {
        id: id.to_owned(),
        kind,
        name: obj.get("name").and_then(Value::as_str).unwrap_or(id).to_owned(),
        version: obj.get("version").and_then(Value::as_str).unwrap_or("0.1.0").to_owned(),
        role: obj.get("role").and_then(Value::as_str).unwrap_or_default().to_owned(),
        trust,
        dir: canon_dir,
        files,
        raw,
    })
}

/// Read a validated JSON file inside a validated extension folder with pre-parse size & path checks.
pub fn read_extension_json(ext: &ValidatedExtension, file_or_slot: &str) -> Result<Value> {
    if let Some(mapped) = ext.files.get(file_or_slot) {
        return read_bounded_json(mapped);
    }
    let resolved = validate_relative_ext_path(&ext.dir, file_or_slot)?;
    read_bounded_json(&resolved)
}

/// Default shared extensions directory (`$CRAFT_EXTENSIONS_DIR` or user data root).
pub fn shared_extensions_dir() -> Option<PathBuf> {
    if let Some(explicit) = std::env::var_os("CRAFT_EXTENSIONS_DIR").filter(|v| !v.is_empty()) {
        let path = PathBuf::from(explicit);
        if path.is_absolute() {
            return Some(path);
        }
    }
    crate::workspace::user_data_dir().map(|base| base.join("craft-extensions"))
}

fn user_config_extensions_root() -> Option<PathBuf> {
    if let Some(cfg) = std::env::var_os("MAGIC_CONFIG_DIR").filter(|v| !v.is_empty()) {
        let path = PathBuf::from(cfg);
        if path.is_absolute() {
            return Some(path.join("extensions"));
        }
    }
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()).map(PathBuf::from).filter(|p| p.is_absolute()) {
        return Some(xdg.join("magic/extensions"));
    }
    if cfg!(windows) {
        return std::env::var_os("APPDATA").map(PathBuf::from).filter(|p| p.is_absolute()).map(|p| p.join("magic/extensions"));
    }
    std::env::var_os("HOME").map(PathBuf::from).filter(|p| p.is_absolute()).map(|h| h.join(".config/magic/extensions"))
}

fn extension_search_roots(s: Option<&Session>) -> Vec<(PathBuf, TrustTier)> {
    let mut roots = Vec::with_capacity(6);
    if let Some(proj_dir) = s.and_then(|sess| sess.project_path.as_deref()).and_then(Path::parent) {
        roots.push((proj_dir.join("extensions"), TrustTier::ProjectLocal));
    }
    if let Some(shared) = shared_extensions_dir() {
        roots.push((shared.join("packages"), TrustTier::SharedStore));
        roots.push((shared, TrustTier::SharedStore));
    }
    if let Some(cfg) = user_config_extensions_root() {
        roots.push((cfg, TrustTier::UserConfig));
    }
    if let Some(sess) = s {
        roots.push((crate::workspace::workspace_root(sess).join("extensions"), TrustTier::Bundled));
    }
    let repo_ext = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../extensions");
    roots.push((repo_ext.canonicalize().unwrap_or(repo_ext), TrustTier::Bundled));
    roots
}

/// Resolve and validate the active `scribbles` extension (`io.magic.scribbles`).
pub fn resolve_scribbles_extension(s: Option<&Session>) -> Result<ValidatedExtension> {
    for (root, tier) in extension_search_roots(s) {
        let candidate = root.join("scribbles");
        if candidate.is_dir() {
            return load_validated_manifest(&candidate, tier);
        }
    }
    Err(bad("extensions/scribbles directory not found"))
}

/// Resolve the active user-editable `scribbles` extension directory when valid on disk.
pub fn resolve_scribbles_extension_dir(s: Option<&Session>) -> Option<PathBuf> {
    resolve_scribbles_extension(s).ok().map(|ext| ext.dir)
}

/// Discover all valid local extension packages matching an optional `role_prefix`.
pub fn discover_extensions(s: Option<&Session>, role_prefix: Option<&str>) -> Vec<ValidatedExtension> {
    let mut found = Vec::new();
    for (root, tier) in extension_search_roots(s) {
        let Ok(rd) = std::fs::read_dir(&root) else {
            continue;
        };
        let mut subdirs: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()).take(64).collect();
        subdirs.sort();
        for dir in subdirs {
            if let Ok(ext) = load_validated_manifest(&dir, tier)
                && role_prefix.is_none_or(|pref| ext.role.starts_with(pref))
                && !found.iter().any(|existing: &ValidatedExtension| existing.id == ext.id)
            {
                found.push(ext);
            }
        }
    }
    found
}

/// Inspect the local shared extensions directory on disk without touching the network.
pub fn extensions_status() -> Value {
    let scribbles = resolve_scribbles_extension(None).ok();
    let discovered: Vec<Value> = discover_extensions(None, None)
        .into_iter()
        .map(|e| json!({"id": e.id, "kind": e.kind, "role": e.role, "version": e.version, "trust": e.trust.as_str(), "dir": e.dir}))
        .collect();
    let Some(root) = shared_extensions_dir() else {
        return json!({
            "configured": false,
            "third_party_enabled": false,
            "scribbles_dir": scribbles.as_ref().map(|e| &e.dir),
            "packages": discovered,
        });
    };
    let state_file = root.join("state.json");
    let store_exists = root.is_dir();
    let installed_count = std::fs::read_dir(root.join("packages")).map_or(0usize, |rd| rd.flatten().count());
    json!({
        "configured": true,
        "store_dir": root,
        "exists": store_exists,
        "state_file_present": state_file.is_file(),
        "installed_packages": installed_count,
        "scribbles_dir": scribbles.as_ref().map(|e| &e.dir),
        "packages": discovered,
        "third_party_enabled": false,
    })
}

/// `app.extensions`: report local ArtCraft extension directory status for Magic Studio.
pub fn cmd_app_extensions(s: &mut Session, _p: &Value) -> Result<Value> {
    let mut ext = extensions_status();
    let active: Vec<Value> = discover_extensions(Some(s), None)
        .into_iter()
        .map(|e| json!({"id": e.id, "kind": e.kind, "role": e.role, "version": e.version, "trust": e.trust.as_str(), "dir": e.dir}))
        .collect();
    ext["scribbles_dir"] = json!(resolve_scribbles_extension_dir(Some(s)));
    ext["packages"] = json!(active);
    Ok(json!({
        "ok": true,
        "extensions": ext,
    }))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{MAX_EXT_FILE_BYTES, TrustTier, load_validated_manifest, resolve_scribbles_extension};

    fn temp_ext_dir(tag: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("magic_ext_{tag}_{nanos}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn bundled_scribbles_manifest_passes_craft_extensions_rules() {
        let ext = resolve_scribbles_extension(None).unwrap();
        assert_eq!(ext.id, "io.magic.scribbles");
        assert_eq!(ext.trust, TrustTier::Bundled);
        assert!(ext.files.contains_key("templates"));
    }

    #[test]
    fn rejects_self_declared_trust_non_empty_permissions_traversal_and_oversized_manifest() {
        let dir = temp_ext_dir("rules");

        let write_mf = |val: serde_json::Value| std::fs::write(dir.join("manifest.json"), serde_json::to_vec(&val).unwrap()).unwrap();
        write_mf(json!({"id": "io.magic.test", "trust": "official", "permissions": []}));
        assert!(load_validated_manifest(&dir, TrustTier::ProjectLocal).unwrap_err().to_string().contains("trust"));

        write_mf(json!({"id": "io.magic.test", "permissions": ["network"]}));
        assert!(load_validated_manifest(&dir, TrustTier::ProjectLocal).unwrap_err().to_string().contains("permissions"));

        write_mf(json!({"id": "io.magic.theme", "kind": "theme", "command": "sh", "permissions": []}));
        assert!(load_validated_manifest(&dir, TrustTier::ProjectLocal).unwrap_err().to_string().contains("themes carry no code"));

        write_mf(json!({"id": "io.magic.test", "permissions": [], "files": {"cfg": "../escape.json"}}));
        assert!(load_validated_manifest(&dir, TrustTier::ProjectLocal).is_err());

        let huge = vec![b' '; (MAX_EXT_FILE_BYTES as usize) + 64];
        std::fs::write(dir.join("manifest.json"), huge).unwrap();
        assert!(load_validated_manifest(&dir, TrustTier::ProjectLocal).unwrap_err().to_string().contains("cap"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
