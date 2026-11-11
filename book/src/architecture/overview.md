# Lightweight Coordinator Overview

Magic Studio is a **pure-Rust coordinator shell** (`eframe` + `egui` 0.37 + [`storytold/craft-libs`](https://github.com/storytold/craft-libs)), not a duplicate media engine. All heavy raster, vector, layout, compositing, and NLE timeline operations execute in their dedicated ArtCraft applications via `CraftHub` (`crates/engine/src/craft_hub/`).

## Workspace Layer Hierarchy (`cargo xtask layers`)

Following [`storytold/craft-contrib`](https://github.com/storytold/craft-contrib/blob/main/docs/house-rules-core.md), dependency edges strictly flow downward:

| Layer | Crate | Responsibility |
|---|---|---|
| **L3 (Upstream)** | [`storytold/craft-libs`](https://github.com/storytold/craft-libs) & [`craft-ai-integrations`](https://github.com/storytold/craft-ai-integrations) | `craft-atomic`, `craft-audio`, `craft-dialogs`, `craft-doc`, `craft-docpath`, `craft-ecproj-format`, `craft-font-catalog`, `craft-keymap`, `craft-media`, `craft-menu`, `craft-pcraft-format`, `craft-platform`, `craft-quit`, `craft-settings`, `craft-ui`, `craft-update`, `craft-vectorcraft-format`, `craft-window`. |
| **L4** | `crates/craft-util`, `crates/scribbles` | `CraftTarget` discovery, `.pcraft`/`.vectorcraft` preview & PDF extraction, in-process ISOBMFF/WAV probing, `.craft_tmp` sweep, rational `Tick` conversions, `plush-9` rig slot geometry, and Scribbles prompt manifests. |
| **L5** | `crates/engine` (`magic-engine`) | `Session`, `AutomationRoots`, `CommandSpec` registry, `.magic` persistence, and `CraftHub` CLI/`--control` delegation. Zero windowing or sibling app crate dependencies. |
| **L6** | `crates/mcp` (`magic-mcp`), `crates/ui-egui` (`magic-ui-egui`) | Stdio JSON-RPC 2.0 MCP server (`Headless` + `Remote` `--connect` / `--bridge`), and pure-Rust `egui` 0.37 + `craft-ui` + `craft-menu` desktop shell views & `AutomationRegistry`. |
| **L7** | `apps/magic`, `apps/magic-cli`, `apps/magic-tui`, `xtask` | `eframe` 0.37 desktop binary (`--control`, `CaptureSlot` screenshots, native dialogs, `craft-keymap`, `craft-quit`), `magic-cli` / `magic-mcp` binaries, `ratatui` terminal dashboard, and workspace policy gates. |

`xtask/src/policy.rs` forbids Tauri/webview dependencies anywhere in the workspace and forbids linking any sibling product crate (`filmcraft-*`, `effectcraft-*`, `photocraft-*`, `vectorcraft-*`, `designcraft-*`, `anicraft-*`) inside `Cargo.toml`.
