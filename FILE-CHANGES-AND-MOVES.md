# Pre-Alpha Refactoring Log (`FILE-CHANGES-AND-MOVES.md`)

## Stripped Cross-App Middleman (`Break` Phase)
- **Deleted `crates/engine/src/craft_hub/`** (`mod.rs`, `control.rs`, `launch.rs`, `cli.rs`, `media_ops.rs`, `target.rs` — 2,065 lines): Removed all cross-app `--control` TCP proxying, sibling token-file generation, and hardcoded `<product>-cli` subprocess wrappers.
- **Deleted `crates/util/src/target.rs`** (312 lines): Removed `CraftTarget`, `ControlAuth`, hardcoded sibling port numbers, and hardcoded CLI bridge flag strings.
- **Deleted `crates/engine/src/cmd/film/bridge.rs`** (679 lines), **`crates/engine/src/cmd/photo_vector.rs`** (133 lines), and **`apps/magic/src/studio.rs`** (89 lines): Removed `bridge.*`, `ui.filmcraft`, `ui.effectcraft`, `ui.craft`, `photo.exec`, `effect.exec`, `vector.exec`, `anicraft.exec`, `video.exec`, `effect.lottie`, `effect.import_aep`, `workspace.close_opened`, and the `bridge_craft` MCP tool.
- **Removed UI & CLI Bridge Surfaces**: Removed `:9876`/`:9877` footer bridge pills (`shell_bridge.rs`), the `Bridge (:9876/:9877)` Options modal tab, `magic-cli bridge`, `--filmcraft-control`, `--effectcraft-control`, `--filmcraft-port`, and `--effectcraft-port`.

## Standard ArtCraft Alignment (`Build` Phase)
- **Fixed `.mcp.json`**: Pointed `magic-app` to `127.0.0.1:9875` (was `127.0.0.1:9876`, FilmCraft's port).
- **Aligned `apps/magic/src/control.rs` & `crates/mcp/src/lib.rs`**: Standard loopback JSON-lines `--control` (`127.0.0.1:9875`) works unauthenticated by default like `EffectCraft`, `DeckCraft`, and `DesignCraft` (with optional constant-time check only when `MAGIC_CONTROL_TOKEN` is set in the environment). Added standard `document.inspect` and `command` aliases in `apps/magic/src/rpc.rs`.
- **Added Pure-Rust `crates/engine/src/native_media.rs`**: In-process RGBA background cutouts, 3x3 parts-sheet splitting, SVG silhouette contours, `craft-doc` preview extraction, and `craft-audio`/`craft-media` container probing with zero external binaries required.
- **Standard OS File Handoff (`crates/engine/src/workspace.rs`)**: `workspace.open` opens project documents on disk via the user's native editor/OS opener without passing `--control` or `--bridge` flags, paired with `link_watch.rs` save detection.
- **Fixed `xtask/src/policy.rs`**: Repaired syntax error at line 137 (`)); }` leftover) and updated layer isolation checks.
