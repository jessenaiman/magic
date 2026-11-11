# Magic Studio Documentation

**Magic Studio** (`magic`, `magic-cli`, `magic-mcp`) is a pure-Rust song-to-video coordinator application (`eframe` + `egui` 0.37 + [`storytold/craft-libs`](https://github.com/storytold/craft-libs)) for the [ArtCraft](https://github.com/storytold) desktop creative suite.

Like [`PhotoCraft`](https://github.com/storytold/photocraft), [`VectorCraft`](https://github.com/storytold/vectorcraft), [`DesignCraft`](https://github.com/storytold/designcraft), [`EffectCraft`](https://github.com/storytold/effectcraft), and [`FilmCraft`](https://github.com/storytold/filmcraft), Magic Studio separates a clean Rust coordinator engine (`crates/engine`), a shared `egui` 0.37 shell crate (`crates/ui-egui`), a headless CLI (`magic-cli`), a stdio [Model Context Protocol](https://modelcontextprotocol.io) server (`magic-cli mcp` / `magic-mcp`), and a native `eframe` desktop app (`magic --control 9876`) with zero Tauri, Electron, or webview dependencies.

## What Magic Studio Coordinates

1. **Song Intake & Lyric Sync ([`FilmCraft`](https://github.com/storytold/filmcraft) + `craft-audio`)** — `magic-cli new --audio song.wav` transcribes word/line timestamps (`timing-reference.json`) and links `A1` audio plus synced captions in `film/<song>.fcproj`.
2. **Production Storyboard ([`DesignCraft`](https://github.com/storytold/designcraft))** — `storyboard.designcraft` organizes `A-Cover`, `B-World`, `C-Character`, `D-Motion`, and scene spreads, with locked read-only fine-print `magic-cli` instructions (`*_cli`) embedded on every frame and traffic-light readiness gating (`storyboard.check`).
3. **Subject Isolation & Vector Tracing ([`PhotoCraft`](https://github.com/storytold/photocraft) + [`VectorCraft`](https://github.com/storytold/vectorcraft))** — `photo.remove_background` (`layer.removeBackground` + `select.refineEdge`) and `vector.trace` (`imageTrace.make` with `ignoreWhite: true`).
4. **Character Puppet Rigging, Kinetic Lyrics & Song-Action Motion ([`EffectCraft`](https://github.com/storytold/effectcraft))** — 11-pin facial & mouth viseme Puppet Controls (`character.puppet.*` -> `V2`), Kinetic Teaching Lyrics (`words.*` -> `V3`), and Song-Action Trim-Path Motion (`motion.*` -> `V4`).
5. **Master NLE Assembly & Export ([`FilmCraft`](https://github.com/storytold/filmcraft))** — `video.assemble` and `video.export` composite `A1` + `V1`–`V4` without cross-domain coupling.

## Documentation Surfaces

- **This mdBook (`book/`)** covers architecture, `.magic` and `.designcraft` formats, `magic-cli`, `--control`, `magic-cli mcp`, security confinement, and open `- [ ]` gap checklists.
- **User Wiki (`docs/wiki/magic/`)** provides Starlight user guides formatted for [`storytold/craft-wiki`](https://github.com/storytold/craft-wiki).
