# Magic Studio Roadmap

Magic Studio coordinates the [ArtCraft](https://github.com/storytold) creative suite — [`DesignCraft`](https://github.com/storytold/designcraft), [`PhotoCraft`](https://github.com/storytold/photocraft), [`VectorCraft`](https://github.com/storytold/vectorcraft), [`EffectCraft`](https://github.com/storytold/effectcraft), and [`FilmCraft`](https://github.com/storytold/filmcraft) — to turn a recorded song into a storyboarded, puppet-animated, kinetic-lyric video.

Detailed technical checklists for unimplemented requests and `// TODO:` host checks live in [`docs/gaps.md`](docs/gaps.md) and [`docs/control-protocol.md`](docs/control-protocol.md).

## Completed Foundation

- [x] Single-Responsibility workspace architecture (`craft-util`, `craft-scribbles`, `magic-engine`, `magic-mcp`, `magic-cli`, `magic`) enforced by `cargo xtask layers`.
- [x] Song-first `magic-cli new --audio` intake writing `timing-reference.json` and initializing a synced [`FilmCraft`](https://github.com/storytold/filmcraft) `.fcproj` timeline (`A1` Song, `V1` Storyboard Scenes, `V2` Character Animation, `V3` Kinetic Lyrics, `V4` Action Motion).
- [x] [`DesignCraft`](https://github.com/storytold/designcraft) `storyboard.designcraft` master template with locked, read-only fine-print `magic-cli` instructions (`*_cli`), traffic-light readiness gating (`storyboard.check`), and title-page-driven `storyboard.bulk_generate`.
- [x] [`EffectCraft`](https://github.com/storytold/effectcraft) 11-pin facial & viseme Puppet Controls (`character.puppet.rig_face`, `character.puppet.expression`, `character.puppet.lipsync`, `character.puppet.follow`, `character.puppet.record`).
- [x] [`PhotoCraft`](https://github.com/storytold/photocraft) subject isolation (`photo.remove_background`) and [`VectorCraft`](https://github.com/storytold/vectorcraft) background-free tracing (`vector.trace`), plus strict headless-only safety isolation for early-alpha [`AniCraft`](https://github.com/storytold/AniCraft).
- [x] `magic-cli mcp` / `magic-mcp` stdio server and `magic --control 9876` live desktop watching with `--automation-read-root` and `--automation-write-root` confinement.
- [x] Magic Studio Source-Available Non-Resale License (`LICENSE` + `docs/brand/LICENSE-brand.txt`) allowing unrestricted commercial video production while prohibiting commercial resale of Magic Studio.

## Next Milestones

### Milestone A — Live `--control` & Cross-Platform Capture Parity
- [ ] Ship macOS (`WKWebView`) and Windows (`WebView2`) window snapshot backends for `ui.screenshot` and panel/element-cropped captures (`ui.screenshot {panel, id}`).
- [ ] Add `--control-input-only`, `ui.viewer.locate` / `ui.timeline.locate`, `render.frame {base64: true}`, and live dBFS audio meters in `ui.playback`.
- [ ] Expose MCP read-only resources (`magic://document`, `magic://storyboard`, `magic://commands`).

### Milestone B — 3D Character Breakout & Host CLI Verification
- [ ] Complete `TextureCraft` CLI inspection and glTF skeletal channel mapping in `crates/engine/src/cmd/character/model3d.rs`.
- [ ] Verify `vectorcraft-cli` flat PNG export verb in `storyboard/crud.rs` and add opt-in host integration tests across all installed `*craft-cli` binaries.

### Milestone C — Phoneme Lip-Sync, Duet Stems & Community Release
- [ ] Upgrade `character.puppet.lipsync` with sub-word phoneme alignment and multi-character vocal stem routing.
- [ ] Configure signed multi-platform installers via [`storytold/craft-common-actions`](https://github.com/storytold/craft-common-actions) and publish user guides to [`storytold/craft-wiki`](https://github.com/storytold/craft-wiki).
