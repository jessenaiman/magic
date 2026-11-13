# `magic-engine` (`crates/engine`)

Headless project coordinator, command registry (`Session::execute`), `DeckCraft` storyboard & `extensions/scribbles/` gate, multi-tool `02 Animations` pipeline, and Crafting App bridge for Magic Studio. Every button in [`crates/ui-egui`](../ui-egui/README.md), every subcommand in [`apps/magic-cli`](../../apps/magic-cli/README.md), and every tool/resource in [`crates/mcp`](../mcp/README.md) dispatches through `magic-engine`.

## Core Session & Host-App Infrastructure

| File | Upstream `storytold` Docs | Why You Explore This File |
| :--- | :--- | :--- |
| **[`src/lib.rs`](src/lib.rs)** | [`storytold/craft-libs/README.md`](https://github.com/storytold/craft-libs/blob/main/README.md) | `Session`, `EngineError`, `ProjectAsset`, undo/redo document snapshots, and `Session::execute(id, params)`. |
| **[`src/craft_api.rs`](src/craft_api.rs)** & **[`src/doctor.rs`](src/doctor.rs)** | [`DeckCraft`](https://github.com/storytold/deckcraft/blob/main/README.md) · [`FilmCraft`](https://github.com/storytold/filmcraft/blob/main/README.md) | Dynamic `<slug>` / `<slug>-cli` discovery (`DeckCraft` `:7990`, `PhotoCraft` `:7878`, `VectorCraft` `:7980`, `EffectCraft` `:9877`, `FilmCraft` `:9876`, `AniCraft`, `DesignCraft`), live `--control` JSON-lines calls, and `app.doctor` suite checks. |
| **[`src/command_sets/`](src/command_sets/mod.rs)** | [`docs/control-protocol.md`](../../docs/control-protocol.md) | Declarative schema (`CommandSpec`, parameters, shortcuts, menu groups) for every command exposed by `engine.commands` and MCP `list_commands`. |
| **[`src/doc_state.rs`](src/doc_state.rs)** & **[`src/workspace.rs`](src/workspace.rs)** | [`storytold/craft-libs`](https://github.com/storytold/craft-libs/blob/main/README.md) | Crash-safe atomic file writes (`craft_atomic::atomic_write`), host editor launching (`workspace.open`), and `--automation-read-root` / `--automation-write-root` path confinement. |
| **[`src/native_media.rs`](src/native_media.rs)** | [`PhotoCraft`](https://github.com/storytold/photocraft/blob/main/README.md) · [`VectorCraft`](https://github.com/storytold/vectorcraft/blob/main/README.md) | `PhotoCraft` subject isolation, `VectorCraft` contour SVG tracing, and `craft-doc` preview extraction. |
| **[`src/craft_ai.rs`](src/craft_ai.rs)** | [`craft-ai-core` (`docs/ai-shared-layer.md`)](https://github.com/storytold/craft-libs/blob/main/docs/ai-shared-layer.md) | AI data model, consent, and route configuration over `craft-ai-core` (`storytold/craft-libs`) for user-editable `extensions/scribbles/`. |

## Domain Command Modules (`src/cmd/`)

| Domain | Upstream Reference | Responsibility |
| :--- | :--- | :--- |
| **Project Intake** | [`storytold/craft-libs`](https://github.com/storytold/craft-libs/blob/main/README.md) | `magic-cli new [existing_project] --audio song.wav --name "..."` creating or attaching onto a project directory with `audio/`, `assets/`, `effects/`, `film/`, `exports/`, and `extensions/scribbles/`. |
| **Song & Waveform** | [`storytold/craft-libs`](https://github.com/storytold/craft-libs/blob/main/README.md) | Multi-bin stereo/mono PCM waveform envelopes, peak/RMS dBFS computation (`song.waveform`), audio slicing via `craft_audio::wav`, and `timing-reference.json` cue maps. |
| **`01 Storyboard` & `extensions/scribbles/`** | [`DeckCraft`](https://github.com/storytold/deckcraft/blob/main/README.md) | Native `.deckcraft` / `.pptx` storyboard decks and PDF export via `DeckCraft`, plus user-editable `extensions/scribbles/` generation gated on saved Storyboard slides. |
| **`02 Animations` (`V2`)** | [`AniCraft`](https://github.com/storytold/anicraft/blob/main/README.md) · [`EffectCraft`](https://github.com/storytold/effectcraft/blob/main/docs/agents.md) | Open multi-tool lane importing Blender, Three.js, After Effects, `AniCraft`, and `EffectCraft` renders recognized by `FilmCraft`, `PhotoCraft`/`VectorCraft` cutouts, puppet visemes, and `craft-mesh` 3D (`GLB`/`glTF`/`OBJ`/`FBX`/`STL`/`PLY`/`3MF`) inspection. |
| **`03 Lyrics` (`V3` + `C1`)** | [`FilmCraft` `docs/captions.md`](https://github.com/storytold/filmcraft/blob/main/docs/captions.md) | Word-timed lyric blocks, UAX #14/kinsoku line breaking, `craft-font-catalog` lookup, `craft-captions` (`SRT`/`WebVTT`/`TTML`/`SCC`/`MCC`/`EBU-STL`), and `effects/lyrics.ecproj`. |
| **`04 Motion` (`V4`)** | [`EffectCraft`](https://github.com/storytold/effectcraft/blob/main/README.md) | `craft-drawingml` preset vector shapes, preset text warps, theme color schemes, and `effects/motion.ecproj`. |
| **`05 Studio` (`FilmCraft`)** | [`FilmCraft`](https://github.com/storytold/filmcraft/blob/main/README.md) | `studio.mapTracks` (`magic-cli studio map-tracks`), `film/<slug>.fcproj` read/write (`V4`, `V3`, `V2`, `V1`, `C1`, `A1`), master export, and `craft-timeline` NLE interchange (`EDL`, `ALE`, `FCP7 XML`, `FCPXML`, `OTIO`). |
