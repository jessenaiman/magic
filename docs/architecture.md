# Architecture & Single-Responsibility Layers

Magic Studio is a pure-Rust coordinator over the [ArtCraft suite](https://github.com/storytold/craft-libs), enforced at CI time by `cargo xtask layers` (`xtask/src/policy.rs`).

## Layers

| Layer | Crates | Responsibility |
|---|---|---|
| **L3** | [`storytold/craft-libs`](https://github.com/storytold/craft-libs) (`craft-atomic`, `craft-doc`, `craft-docpath`, `craft-audio`, `craft-media`, `craft-font-catalog`, `craft-ecproj-format`, `craft-pcraft-format`, `craft-vectorcraft-format`, `craft-settings`, `craft-update`) | Crash-safe atomic file writes, `.ecproj`/`.pcraft`/`.vectorcraft` envelope outlines & renditions, `Tick` timebase (`254,016,000,000` ticks/s), in-process audio & ISO BMFF container probing, font store discovery, TOML settings, and SHA-256 release verification. |
| **L4** | `crates/craft-util`, `crates/scribbles` | `CraftTarget` discovery (`allows_live_ui() == false` for early-alpha [AniCraft](https://github.com/storytold/AniCraft)), credential scrubbing, zero-subprocess media/rendition preflight, `.craft_tmp` staging sweeps, and Scribbles prompt/grid geometry. |
| **L5** | `crates/engine` (`magic`) | `Session`, command registry (`command_sets/{project,creative,system}`), `AutomationRoots` confinement, and `CraftHub` routing to live `--control` sockets or `<product>-cli`. Headless — zero UI/window crates allowed. |
| **L6** | `crates/ui-egui` (`magic-ui-egui`), `crates/mcp` (`magic-mcp` lib), [`craft-ui`](https://github.com/storytold/craft-libs), [`craft-menu`](https://github.com/storytold/craft-libs), [`craft-window`](https://github.com/storytold/craft-libs), [`craft-dialogs`](https://github.com/storytold/craft-libs), [`craft-keymap`](https://github.com/storytold/craft-libs), [`craft-platform`](https://github.com/storytold/craft-libs), [`craft-quit`](https://github.com/storytold/craft-libs) | Pure-Rust `egui` 0.37 workstation shell, 9-menu `craft-menu` system bar, 8 stage views, per-frame `AutomationRegistry`, and stdio MCP server (`rmcp`) with `Remote` (`--connect 127.0.0.1:9876`) and `Headless` backends ([`docs/mcp.md`](mcp.md)). |
| **L7** | `apps/magic`, `apps/magic-cli` (`magic-cli` & `magic-mcp`), `apps/magic-tui`, `xtask` | Pure-Rust `eframe` 0.37 + `wgpu` desktop coordinator (`--control`), CLI/MCP binaries, TUI monitor, and CI policy/build gates. |

Sibling ArtCraft application crates (`filmcraft-*`, `effectcraft-*`, `designcraft-*`, `photocraft-*`, `vectorcraft-*`, `anicraft-*`) are **forbidden** in `Cargo.toml`; Magic talks to installed sibling apps solely through `--control` and `<product>-cli`.

## Producer-to-Timeline Track Isolation

Every visual overlay domain owns an isolated [EffectCraft](https://github.com/storytold/effectcraft) `.ecproj` file that renders ProRes 4444 RGBA onto a dedicated [FilmCraft](https://github.com/storytold/filmcraft) track:

| Track | Label | Producer Document | Primary Commands |
|---|---|---|---|
| `A1` | Song Audio | Original recording (`craft-audio` + `timing-reference.json`) | `project.new` / `file.new`, `audio.analyze` |
| `V1` | Storyboard Scenes | `storyboard.designcraft` ([DesignCraft](https://github.com/storytold/designcraft)) | `storyboard.wizard`, `storyboard.bulk_generate`, `video.assemble` |
| `V2` | Character Animation | `assets/characters/<id>/<view>/<id>-<view>.ecproj` ([EffectCraft](https://github.com/storytold/effectcraft)) | `character.cut`, `character.rig`, `character.puppet.*`, `character.render` |
| `V3` | Kinetic Lyrics | `lettering/lyrics.ecproj` ([EffectCraft](https://github.com/storytold/effectcraft)) | `words.letter`, `words.save`, `words.motion.set` |
| `V4` | Action Motion | `motion/actions.ecproj` ([VectorCraft](https://github.com/storytold/vectorcraft) -> [EffectCraft](https://github.com/storytold/effectcraft)) | `motion.save`, `motion.publish` |
