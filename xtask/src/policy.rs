//! Layer, UI-boundary, and cross-app isolation gates (`docs/architecture.md › Layers`).
use crate::Result;
use serde_json::Value;
use std::process::Command;

/// Magic's own workspace crates and their layers (`docs/architecture.md › Layers`).
const LAYERS: &[(&str, u8)] =
    &[("craft-util", 4), ("craft-scribbles", 4), ("magic-engine", 5), ("magic-mcp", 6), ("magic", 7), ("magic-cli", 7), ("magic-tui", 7), ("xtask", 7)];

/// UI / windowing crates (L6): allowed only in L7 shells (`magic`), never in headless L0–L5 crates
/// (`craft-util`, `craft-scribbles`, `magic-engine`).
const UI_CRATES: &[&str] = &["craft-ui", "craft-dialogs", "craft-keymap", "egui", "eframe", "winit", "rfd", "tauri", "webkit2gtk"];

/// Sibling ArtCraft product crates are strictly forbidden as Cargo dependencies: Magic shares only
/// `storytold/craft-libs` (`craft-*`) and `storytold/craft-ai-integrations`, and drives installed
/// sibling applications via `--control` and `<product>-cli` (`crates/craft-util` & `craft_hub`).
const FORBIDDEN_SIBLING_PREFIXES: &[&str] = &[
    "anicraft-",
    "cadcraft-",
    "deckcraft-",
    "designcraft-",
    "effectcraft-",
    "filmcraft-",
    "gridcraft-",
    "materialcraft-",
    "photocraft-",
    "solidcraft-",
    "soundcraft-",
    "texturecraft-",
    "vectorcraft-",
    "wordcraft-",
];

fn layer_of(name: &str) -> Option<u8> {
    if let Some((_, l)) = LAYERS.iter().find(|(n, _)| *n == name) {
        return Some(*l);
    }
    if UI_CRATES.contains(&name) || name.ends_with("-ui-egui") || name.starts_with("tauri-") {
        return Some(6);
    }
    name.starts_with("craft-").then_some(3)
}

pub(crate) fn metadata() -> Result<Value> {
    let out = Command::new(env!("CARGO")).args(["metadata", "--format-version", "1", "--no-deps"]).output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).into());
    }
    serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())
}

fn workspace_crates(md: &Value) -> Vec<(String, Vec<String>)> {
    md["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|p| p["name"] != "xtask")
        .map(|p| {
            let deps = p["dependencies"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|d| d["kind"].is_null() || d["kind"] == "build")
                .filter_map(|d| d["name"].as_str().map(str::to_string))
                .collect();
            (p["name"].as_str().unwrap_or_default().to_string(), deps)
        })
        .collect()
}

pub(crate) fn layers() -> Result<()> {
    let md = metadata()?;
    let crates = workspace_crates(&md);
    let mut errors = Vec::new();
    for (name, deps) in &crates {
        let Some(l) = layer_of(name) else {
            errors.push(format!("{name}: not assigned a layer (add it to xtask LAYERS and docs/architecture.md › Layers)"));
            continue;
        };
        for d in deps {
            if FORBIDDEN_SIBLING_PREFIXES.iter().any(|p| d.starts_with(p)) {
                errors.push(format!("{name} depends on sibling app crate {d}: drive installed Craft apps via CraftHub (--control / <product>-cli) instead"));
                continue;
            }
            let Some(dl) = layer_of(d) else { continue };
            if dl >= l {
                errors.push(format!("{name} (L{l}) depends on {d} (L{dl}): edges must go down"));
            }
        }
    }
    if errors.is_empty() {
        println!("layers: ok");
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}
