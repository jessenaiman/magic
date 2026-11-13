# Magic Studio Feature Gaps & Host Verification Checklist

Always verify upstream behavior directly in the corresponding `storytold/*` `README.md` and `docs/` files.

## Completed Foundation

- [x] Unpinned [`storytold/craft-libs`](https://github.com/storytold/craft-libs/blob/main/README.md) git dependencies onto `main`, consolidated AI contract types onto `craft-ai-core`, and configured `./install.sh` to run `cargo update` before building.
- [x] Simplified `cargo xtask ci` to run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --message-format=short -- -D warnings`, and opt-in `MAGIC_RUN_TESTS=1` workspace tests, writing `target/ci-report.txt` and `target/ci-report.jsonl`.
- [x] Adopted native [`DeckCraft`](https://github.com/storytold/deckcraft/blob/main/README.md) `.deckcraft`/`.pptx` + PDF storyboards, user-editable `extensions/scribbles/` with the saved-Storyboard-slide gate, open multi-tool `02 Animations`, and live `FilmCraft` views across `03 Lyrics`, `04 Motion`, and `05 Studio` (`Live Breakdown` & `Finishing` dock).
- [x] Aligned `magic --control 9875`, `magic-cli studio map-tracks`, and `magic-cli mcp` (`magic://` resources + `--connect 9875` or headless) with ArtCraft conventions, and added AppStream metadata (`packaging/linux/io.github.jessenaiman.magic.metainfo.xml`).

## Open Host-Only & Upstream Items

- [ ] **Host `cargo xtask ci` & Live Multi-App Socket Proof**: Run `cargo xtask ci` on a desktop GPU host (capturing `target/ci-report.txt`) and verify live `--control` round-trips across `Magic` (`:9875`), `DeckCraft` (`:7990`), `EffectCraft` (`:9877`), and `FilmCraft` (`:9876`).
- [ ] **Upstream FilmCraft Linked Comps**: Track [`storytold/filmcraft#388`](https://github.com/storytold/filmcraft/issues/388) for native linked `.ecproj` compositions.
- [ ] **Host CLI Capability Checks**: Verify `EffectCraft` `renderer: "advanced3D"` glTF 2.0 skinning, `filmcraft-cli` Whisper transcription (`--features whisper`), and installed host CLI discovery via `magic-cli exec app.doctor`.
