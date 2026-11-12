//! Established ArtCraft architecture & repository gates (`layers` and `assets`):
//! - Downward-only workspace crate DAG (`L4` util/scribbles -> `L5` engine -> `L6` mcp / ui-egui; `apps/*` & `xtask` exempt).
<<<<<<< HEAD
//! - UI / windowing crates (`egui`, `eframe`, `winit`, `rfd`, `arboard`, `craft-ui`, `craft-window`) forbidden below `L6`.
//! - Pure-Rust non-Tauri rule (`tauri`, `wry`, `tao`, `webkit2gtk` forbidden everywhere).
//! - Sibling ArtCraft app crates (`deckcraft-*`, `filmcraft-*`, etc.) forbidden in `Cargo.toml`.
//! - `ATTRIBUTION.md` completeness check across `assets/`, `docs/brand/`, and `figcraft/diagrams/`.
=======
//! - UI, windowing, and webview-host crates (`egui`, `eframe`, `winit`, `wry`, `craft-ui`, `craft-web-host`, etc.) forbidden below `L6`.
//! - Upstream GUI test harnesses (`egui_kittest`) and non-`eframe` desktop shells are forbidden.
//! - Sibling ArtCraft app crates (`deckcraft-*`, `filmcraft-*`, etc.) forbidden in `Cargo.toml`.
//! - `ATTRIBUTION.md` completeness check across `assets/`, `docs/brand/`, and `figcraft/`.
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

use crate::Result;

/// Library crates in `crates/*` and their layers (`apps/*` and `xtask` are top-level binaries exempt from inward edges).
const CRATE_LAYERS: &[(&str, u8)] = &[("magic-util", 4), ("magic-scribbles", 4), ("magic-engine", 5), ("magic-mcp", 6), ("magic-ui-egui", 6)];

/// Top-level binary packages in `apps/*` and `xtask`.
const APP_BINARIES: &[&str] = &["magic", "magic-cli", "magic-tui", "xtask"];

<<<<<<< HEAD
/// UI and windowing dependencies forbidden in headless `L0..=L5` crates.
const UI_CRATES: &[&str] = &[
    "craft-ui",
    "craft-menu",
    "craft-window",
    "craft-quit",
    "craft-platform",
    "craft-dialogs",
    "craft-keymap",
    "egui",
    "egui_extras",
    "eframe",
    "winit",
=======
/// UI, windowing, and web-host dependencies forbidden in headless `L0..=L5` crates (allowed in `L6` UI and `L7` app binaries).
const UI_CRATES: &[&str] = &[
    "craft-ai-egui",
    "craft-dialogs",
    "craft-font-manager",
    "craft-keymap",
    "craft-menu",
    "craft-platform",
    "craft-quit",
    "craft-ui",
    "craft-web-api",
    "craft-web-host",
    "craft-window",
    "eframe",
    "egui",
    "egui-wgpu",
    "egui_extras",
    "egui_glow",
    "winit",
    "wry",
    "webkit2gtk",
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    "rfd",
    "arboard",
];

<<<<<<< HEAD
/// Non-Tauri / pure-Rust enforcement across the entire workspace (`photocraft/AGENTS.md`, `deckcraft/AGENTS.md`).
const FORBIDDEN_WEBVIEW: &[&str] = &["tauri", "tauri-build", "wry", "tao", "webkit2gtk"];

/// Sibling ArtCraft product crates are never linked as Cargo dependencies (`storytold/craft-libs` only).
const FORBIDDEN_SIBLING_PREFIXES: &[&str] = &[
    "anicraft-",
    "cadcraft-",
    "deckcraft-",
=======
/// Upstream GUI test harnesses forbidden in Magic Studio (upstream `egui` tests belong in `egui` / `craft-libs`, not maintained here).
const FORBIDDEN_TEST_HARNESSES: &[&str] = &["egui_kittest"];

/// Sibling ArtCraft product crates are never linked as Cargo dependencies (`storytold/craft-libs` only).
const FORBIDDEN_SIBLING_PREFIXES: &[&str] = &[
    "cadcraft-",
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    "designcraft-",
    "effectcraft-",
    "filmcraft-",
    "figcraft-",
    "gridcraft-",
    "lightcraft-",
    "materialcraft-",
<<<<<<< HEAD
    "pdfcraft-",
=======
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    "photocraft-",
    "solidcraft-",
    "soundcraft-",
    "texturecraft-",
    "vectorcraft-",
    "wordcraft-",
];

fn workspace_layer(name: &str) -> Option<u8> {
    CRATE_LAYERS.iter().find(|(n, _)| *n == name).map(|(_, l)| *l)
}

pub(crate) fn metadata() -> Result<Value> {
    let out = Command::new(env!("CARGO")).args(["metadata", "--format-version", "1", "--no-deps"]).output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).into());
    }
    serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())
}

<<<<<<< HEAD
fn workspace_packages(md: &Value) -> Vec<(String, Vec<String>)> {
=======
struct PackageDeps {
    name: String,
    normal_and_build: Vec<String>,
    all_deps: Vec<String>,
}

fn workspace_packages(md: &Value) -> Vec<PackageDeps> {
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    md["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|p| {
<<<<<<< HEAD
            let deps = p["dependencies"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|d| d["kind"].is_null() || d["kind"] == "build")
                .filter_map(|d| d["name"].as_str().map(str::to_string))
                .collect();
            (p["name"].as_str().unwrap_or_default().to_string(), deps)
=======
            let raw_deps = p["dependencies"].as_array().into_iter().flatten();
            let mut normal_and_build = Vec::new();
            let mut all_deps = Vec::new();
            for d in raw_deps {
                if let Some(dep_name) = d["name"].as_str() {
                    all_deps.push(dep_name.to_string());
                    if d["kind"].is_null() || d["kind"] == "build" {
                        normal_and_build.push(dep_name.to_string());
                    }
                }
            }
            PackageDeps { name: p["name"].as_str().unwrap_or_default().to_string(), normal_and_build, all_deps }
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
        })
        .collect()
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap_or_else(|| Path::new(".")).to_path_buf()
}

pub(crate) fn layers() -> Result<()> {
    let md = metadata()?;
    let packages = workspace_packages(&md);
    let mut errors = Vec::new();

<<<<<<< HEAD
    for (name, deps) in &packages {
=======
    for pkg in &packages {
        let name = &pkg.name;
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
        if name.starts_with("craft-") {
            errors.push(format!("{name}: local workspace crates must use `magic-*` and never impersonate `storytold/craft-libs` (`craft-*`)"));
            continue;
        }
        let is_app = APP_BINARIES.contains(&name.as_str());
        let src_layer = workspace_layer(name);
        if !is_app && src_layer.is_none() {
            errors.push(format!("{name}: unregistered workspace package (add it to `CRATE_LAYERS` or `APP_BINARIES` in `xtask/src/policy.rs`)"));
            continue;
        }

<<<<<<< HEAD
        for dep in deps {
            if FORBIDDEN_WEBVIEW.contains(&dep.as_str()) || dep.starts_with("tauri-") {
                errors.push(format!("{name} depends on forbidden webview/Tauri crate `{dep}`: Magic Studio is pure Rust (`eframe` + `egui`)"));
                continue;
            }
=======
        for dep in &pkg.all_deps {
            if FORBIDDEN_TEST_HARNESSES.contains(&dep.as_str()) {
                errors.push(format!(
                    "{name} depends on forbidden upstream GUI test harness `{dep}`: do not maintain upstream `egui` widget tests in Magic Studio"
                ));
            }
        }

        for dep in &pkg.normal_and_build {
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
            if FORBIDDEN_SIBLING_PREFIXES.iter().any(|p| dep.starts_with(p)) {
                errors.push(format!("{name} depends on sibling app crate `{dep}`: exchange files on disk via `storytold/craft-libs` instead"));
                continue;
            }
            if APP_BINARIES.contains(&dep.as_str()) {
                errors.push(format!("{name} depends on binary package `{dep}`"));
                continue;
            }
            if let Some(l) = src_layer {
                if l < 6 && (UI_CRATES.contains(&dep.as_str()) || dep.ends_with("-ui-egui")) {
<<<<<<< HEAD
                    errors.push(format!("{name} (L{l}) depends on UI crate `{dep}`: nothing below L6 may depend on egui/eframe/winit/rfd/craft-ui"));
=======
                    errors.push(format!("{name} (L{l}) depends on UI/web-host crate `{dep}`: nothing below L6 may depend on UI or webview crates"));
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
                    continue;
                }
                if let Some(dl) = workspace_layer(dep)
                    && dl >= l
                {
                    errors.push(format!("{name} (L{l}) depends on workspace crate `{dep}` (L{dl}): edges must point strictly downward"));
                }
            }
        }
    }

    if errors.is_empty() {
        println!("layers: ok ({} workspace packages verified)", packages.len());
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

fn collect_files(dir: &Path, root: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, root, out);
        } else if let Ok(rel) = path.strip_prefix(root) {
            let rel_slash = rel.to_string_lossy().replace('\\', "/");
            if !rel_slash.ends_with("/README.md") {
                out.push(rel_slash);
            }
        }
    }
}

pub(crate) fn assets() -> Result<()> {
    let root = workspace_root();
    let attr_path = root.join("ATTRIBUTION.md");
    let attr_text = std::fs::read_to_string(&attr_path).map_err(|e| format!("could not read ATTRIBUTION.md: {e}"))?;

    let mut tracked = Vec::new();
    for folder in ["assets", "docs/brand", "figcraft"] {
        let dir = root.join(folder);
        if dir.is_dir() {
            collect_files(&dir, &root, &mut tracked);
        }
    }
    tracked.sort();

    let mut missing = Vec::new();
    for rel in &tracked {
        if !attr_text.contains(rel) {
            missing.push(format!("{rel}: missing attribution row in ATTRIBUTION.md"));
        }
    }
    if missing.is_empty() {
        println!("assets: ok ({} asset files attributed)", tracked.len());
        Ok(())
    } else {
        Err(missing.join("\n"))
    }
}
