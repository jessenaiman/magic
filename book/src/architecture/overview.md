# Lightweight Coordinator Overview

Magic Studio is a **lightweight shell and coordinator**, not a duplicate media engine. All heavy raster, vector, layout, compositing, and NLE timeline operations execute in their dedicated ArtCraft applications via `CraftHub` (`crates/engine/src/craft_hub/`).

## Workspace Layer Hierarchy (`cargo xtask layers`)

Following [`storytold/craft-contrib`](https://github.com/storytold/craft-contrib/blob/main/docs/house-rules-core.md), dependency edges strictly flow downward:

| Layer | Crate | Responsibility |
|---|---|---|
| **L3 (Upstream)** | [`storytold/craft-libs`](https://github.com/storytold/craft-libs) & [`craft-ai-integrations`](https://github.com/storytold/craft-ai-integrations) | `craft-atomic`, `craft-doc`, `craft-docpath`, `craft-audio`, `craft-media`, `craft-ecproj-format`, `craft-pcraft-format`, `craft-vectorcraft-format`, `craft-settings`, `craft-dialogs`, `craft-keymap`, `craft-update`. |
| **L4** | `crates/craft-util`, `crates/scribbles` | `CraftTarget` discovery, rational `Tick` conversions, `plush-9` rig slot geometry, and Scribbles prompt manifests. |
| **L5** | `crates/engine` (`magic-engine`) | `Session`, `AutomationRoots`, `CommandSpec` registry, `.magic` persistence, and `CraftHub` CLI/`--control` delegation. Zero windowing or sibling app crate dependencies. |
| **L6** | `crates/mcp` (`magic-mcp`) | Lightweight stdio JSON-RPC 2.0 MCP server (`Headless` + `Remote` `--connect` backends). |
| **L7** | `apps/magic`, `apps/magic-cli`, `apps/magic-tui`, `xtask` | Tauri v2 desktop shell, `magic-cli` / `magic-mcp` binaries, terminal dashboard, and workspace policy gates. |

`xtask/src/policy.rs` forbids linking any sibling product crate (`filmcraft-*`, `effectcraft-*`, `photocraft-*`, `vectorcraft-*`, `designcraft-*`, `anicraft-*`) inside `Cargo.toml`.
