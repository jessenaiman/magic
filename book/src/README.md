# Magic Studio Documentation

**Magic Studio** (`magic`, `magic-cli`, `magic-tui`) is a song-to-video production coordinator built in pure Rust (`eframe` + [`emilk/egui`](https://github.com/emilk/egui) + [`storytold/craft-libs`](https://github.com/storytold/craft-libs)) for the [ArtCraft](https://github.com/storytold) creative suite.

Like [`DeckCraft`](https://github.com/storytold/deckcraft), [`AniCraft`](https://github.com/storytold/anicraft), [`PhotoCraft`](https://github.com/storytold/photocraft), [`VectorCraft`](https://github.com/storytold/vectorcraft), [`EffectCraft`](https://github.com/storytold/effectcraft), and [`FilmCraft`](https://github.com/storytold/filmcraft), Magic Studio separates a headless Rust engine (`crates/engine`), a shared `egui` + `craft_ui::docking` workstation crate (`crates/ui-egui`), a CLI (`magic-cli`), a Backstage terminal viewer (`magic-tui`), a stdio [Model Context Protocol](https://modelcontextprotocol.io) server (`magic-cli mcp` with read-only `magic://` resources), and a native `eframe` + `wgpu` desktop app (`magic --control 9875`).

## What Magic Studio Coordinates

1. **Song Intake & Timing ([`FilmCraft`](https://github.com/storytold/filmcraft) + `craft-audio`)** — `magic-cli new [existing_project] --audio song.wav --name "..."` copies or links song audio, computes multi-bin peaks, writes `timing-reference.json`, and initializes `film/<slug>.fcproj`.
2. **Creative Storyboard & `extensions/scribbles/` ([`DeckCraft`](https://github.com/storytold/deckcraft) + `craft-ai-core`)** — Native `.deckcraft` / `.pptx` slide decks with PDF export in `01 Storyboard`, paired with user-editable `extensions/scribbles/` recipes gated on saved Storyboard slides.
3. **Open Multi-Tool `02 Animations` (`V2`)** — Accepts 2D/3D renders and meshes from Blender, Three.js, After Effects, [`AniCraft`](https://github.com/storytold/anicraft), [`EffectCraft`](https://github.com/storytold/effectcraft), [`PhotoCraft`](https://github.com/storytold/photocraft), and [`VectorCraft`](https://github.com/storytold/vectorcraft) across every format [`FilmCraft`](https://github.com/storytold/filmcraft) decodes.
4. **Live `03 Lyrics` (`V3` + `C1`) & `04 Motion` (`V4`) Views** — Kinetic lyrics (`craft-font-catalog`, `FontsPanel`, `craft-captions`) and `craft-drawingml` vector shapes/warps in live `FilmCraft`-aligned views.
5. **`05 Studio` (`Live Breakdown` & `Finishing` Dock)** — `magic-cli studio map-tracks` track mapping, NLE interchange (`OTIO`, `FCPXML`, `FCP7 XML`, `EDL`, `ALE`), linear-light scopes/color wheels, stereo dBFS meters, and master export via [`FilmCraft`](https://github.com/storytold/filmcraft).

## Documentation Surfaces

- **This mdBook (`book/`)** covers architecture, `.magic` and `.deckcraft` project conventions, `magic-cli`, `--control`, `magic-cli mcp`, security confinement, and quality gates.
- **User Wiki (`docs/wiki/magic/`)** provides Starlight user guides formatted for [`storytold/craft-wiki`](https://github.com/storytold/craft-wiki).
