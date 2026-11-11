# Agent Instructions (`Magic Studio`)

> **Notice on Legacy Repository**: Do NOT read, reference, or import code/patterns from the legacy prototype repository (`jessenaiman/magic-studio-app`) unless the user explicitly requests it, as historical churn and divergent iterations can introduce AI psychosis and regressions. Historical development, early prototypes, and past discussions are preserved for human record at https://github.com/jessenaiman/magic-studio-app.

Magic Studio is a pure-Rust ArtCraft coordinator application (`magic`, `magic-cli`, `magic-mcp`) built on `eframe` + `egui` 0.37 + [`storytold/craft-libs`](https://github.com/storytold/craft-libs) and governed by [`storytold/craft-contrib`](https://github.com/storytold/craft-contrib/blob/main/AGENTS.md).

## 1. Drive Magic Through `magic-cli mcp` and Live `--control`

Follow the exact ArtCraft MCP pattern in [`docs/mcp.md`](docs/mcp.md) (matching [DesignCraft MCP](https://github.com/storytold/designcraft/blob/main/docs/mcp.md) and VectorCraft):

- **Live Watching (`--control`)**: When the user asks to watch the app live, start or connect to `magic --control 9876` via `magic-cli mcp --connect 127.0.0.1:9876` (or `magic-cli --connect 127.0.0.1:9876 <cmd>`, or `magic-mcp --connect 127.0.0.1:9876`).
- **Headless**: Use `magic-cli mcp --headless` (with `--automation-read-root` / `--automation-write-root` when confining file access) or `magic-cli --project <file.magic> exec <id>`.
- Discover commands dynamically with `list_commands` or `magic-cli commands`, inspect schemas with `magic-cli describe <id>`, and execute via `run_command` or `run_batch`.

## 2. Single-Responsibility ArtCraft Delegation (Never Reimplement Craft Engines)

- **Audio & Master NLE Timeline**: Delegate to `craft-audio` + [FilmCraft](https://github.com/storytold/filmcraft) ([CLI & automation](https://github.com/storytold/filmcraft/blob/main/docs/cli-and-automation.md), [captions & speech](https://github.com/storytold/filmcraft/blob/main/docs/captions-and-speech.md)).
- **Animation (`V2`), Kinetic Lyrics (`V3`), and Song-Action Motion (`V4`)**: Author each domain as its own [EffectCraft](https://github.com/storytold/effectcraft) `.ecproj` composition ([EffectCraft CLI](https://github.com/storytold/effectcraft/blob/main/docs/cli.md), [animation & expressions](https://github.com/storytold/effectcraft/blob/main/docs/animation-and-expressions.md), [text & presets](https://github.com/storytold/effectcraft/blob/main/docs/text-and-presets.md), [masks, mattes & 3D](https://github.com/storytold/effectcraft/blob/main/docs/masks-mattes-and-3d.md)) that exports ProRes 4444 RGBA tracks into `FilmCraft`.
- **Facial Expressions & Mouth Movements**: Always use EffectCraft Puppet Controls (`character.puppet.rig_face`, `character.puppet.expression`, `character.puppet.lipsync`, `character.puppet.follow`, `character.puppet.record`, `character.puppet.addPin`) — never hand-rolled pixel warps or external Blender scripts.
- **Background & Subject Removal**: Always use [PhotoCraft](https://github.com/storytold/photocraft) (`photo.remove_background` -> `layer.removeBackground` + `select.refineEdge`) and [VectorCraft](https://github.com/storytold/vectorcraft) (`vector.trace` -> `imageTrace.make` with `ignoreWhite: true`).
- **Storyboard Workflow**: Use [DesignCraft](https://github.com/storytold/designcraft) (`storyboard.*`). Complete the 4 Main Section Title Pages (`A-Cover`, `B-World`, `C-Character`, `D-Motion`) first, run `storyboard.bulk_generate`, and confirm `storyboard.check` is green before video production.
- **AniCraft Safety Boundary**: [AniCraft](https://github.com/storytold/AniCraft) is early alpha. Use **only `EffectCraft` and `FilmCraft` UI components/windows** for live studio work, and invoke `anicraft-cli` **strictly headlessly** (`anicraft.exec`, `character.sprite` PNG sequence export into `EffectCraft`).

## 3. Repository Hygiene & House Rules (`storytold/craft-contrib`)

- Do **not** create custom paraphrased animation skill files or markdown research/log dumps in the repository. Link directly to official outside ArtCraft documentation and verified tutorials.
- Keep every source file focused on one responsibility and under 900 lines; leave a `// TODO:` comment (never a `todo!()` panic) when a headless CLI integration step needs follow-up verification on the host.
- Never introduce Tauri, webviews, JavaScript, or TypeScript; all desktop UI lives in `crates/ui-egui` (`magic-ui-egui`) and `apps/magic`.
- Run `cargo xtask ci` (fmt, layers, Clippy `-D warnings`, workspace tests) before opening a PR, and report honestly which checks ran vs which were skipped.
- Never add `Co-Authored-By` agent trailers, `Generated with ...` footers, or session transcripts to commits, PRs, or issues ([Rule 15](https://github.com/storytold/craft-contrib/blob/main/docs/house-rules-core.md)).
