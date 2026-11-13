# Magic Studio Roadmap

Magic Studio coordinates [`DeckCraft`](https://github.com/storytold/deckcraft), [`AniCraft`](https://github.com/storytold/anicraft), [`PhotoCraft`](https://github.com/storytold/photocraft), [`VectorCraft`](https://github.com/storytold/vectorcraft), [`EffectCraft`](https://github.com/storytold/effectcraft), and [`FilmCraft`](https://github.com/storytold/filmcraft) on top of [`storytold/craft-libs`](https://github.com/storytold/craft-libs).

## Alpha Foundation

- [x] **Project Intake & Self-Contained Song Folders** ([`crates/engine/src/workspace.rs`](crates/engine/src/workspace.rs)): `magic-cli new [existing_project] --audio song.wav --name "..."` creating or attaching onto a project directory with `audio/`, `assets/`, `effects/`, `film/`, `exports/`, and user-editable `extensions/scribbles/`.
- [x] **DeckCraft Creative Storyboard & Saved-Slide Scribbles Gate**: Native `.deckcraft` / `.pptx` storyboards with PDF export via [`DeckCraft`](https://github.com/storytold/deckcraft) (`deckcraft --control 7990` / `deckcraft-cli`), paired with `extensions/scribbles/` (`craft-ai-core`) gated on saved Storyboard slides.
- [x] **Open Multi-Tool `02 Animations` (`V2`)**: Bring renders and 3D assets from Blender, Three.js, After Effects, `AniCraft`, `EffectCraft`, `PhotoCraft`, and `VectorCraft` in any format recognized by `FilmCraft`, plus `craft-mesh` 3D inspection.
- [x] **Live `FilmCraft` Editing Views & 2-Tab `05 Studio` Dock** ([`crates/ui-egui/src/docking.rs`](crates/ui-egui/src/docking.rs)): Synchronized `03 Lyrics` (`V3` + `C1`), `04 Motion` (`V4`), and `05 Studio` (`Live Breakdown` & `Finishing` tabs), `magic-cli studio map-tracks`, `magic-cli mcp` with `magic://` resources, and `magic-tui` Backstage snapshots.

## Upcoming Milestones

- [ ] **Live Multi-App `--control` & Host GPU Verification**: End-to-end host runs across `Magic` (`:9875`), `DeckCraft` (`:7990`), `EffectCraft` (`:9877`), and `FilmCraft` (`:9876`).
- [ ] **Upstream FilmCraft Linked Comps & Whisper Transcripts**: Track upstream [`storytold/filmcraft#388`](https://github.com/storytold/filmcraft/issues/388) for native linked `.ecproj` compositions and `filmcraft-cli` transcript generation (`--features whisper`).
- [ ] **Release Signing & Community Wiki**: Wire [`storytold/craft-common-actions`](https://github.com/storytold/craft-common-actions) release signing secrets and publish [`docs/wiki/magic/`](docs/wiki/README.md) to [`storytold/craft-wiki`](https://github.com/storytold/craft-wiki).
