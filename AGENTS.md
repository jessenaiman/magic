# Agent Instructions (`Magic Studio`)

Magic Studio (`magic`, `magic-cli`, `magic-tui`) is a pure-Rust ArtCraft production studio built on `eframe` + `egui` 0.37 + [`storytold/craft-libs`](https://github.com/storytold/craft-libs/blob/main/README.md) and governed by [`docs/house-rules-core.md`](docs/house-rules-core.md) ([`storytold/craft-contrib`](https://github.com/storytold/craft-contrib/blob/main/standards/house-rules-core.md)).

## 1. Upstream References & Workspace Build

- **Read Actual Upstream `README.md` & `docs/` Pages**: Consult [`README.md`](README.md) and the live upstream repositories directly before changing integrations:
  - Shared Rust libraries & `craft-ai-core`: [`storytold/craft-libs/README.md`](https://github.com/storytold/craft-libs/blob/main/README.md) and [`docs/ai-shared-layer.md`](https://github.com/storytold/craft-libs/blob/main/docs/ai-shared-layer.md)
  - Crafting Apps: [`DeckCraft`](https://github.com/storytold/deckcraft/blob/main/README.md), [`AniCraft`](https://github.com/storytold/anicraft/blob/main/README.md), [`PhotoCraft`](https://github.com/storytold/photocraft/blob/main/README.md), [`VectorCraft`](https://github.com/storytold/vectorcraft/blob/main/README.md), [`EffectCraft`](https://github.com/storytold/effectcraft/blob/main/README.md), [`DesignCraft`](https://github.com/storytold/designcraft/blob/main/README.md), and [`FilmCraft`](https://github.com/storytold/filmcraft/blob/main/README.md)
  - Shared CI & contribution standards: [`storytold/craft-common-actions`](https://github.com/storytold/craft-common-actions/blob/main/README.md) and [`storytold/craft-contrib`](https://github.com/storytold/craft-contrib/blob/main/README.md)
- **Floating `craft-libs` & `cargo xtask ci`**: Workspace dependencies in [`Cargo.toml`](Cargo.toml) track `storytold/craft-libs` `main` (`./install.sh --from-source` runs `cargo update` before building). Run `cargo xtask ci` (`fmt`, `clippy --workspace --all-targets -- -D warnings`, and opt-in `MAGIC_RUN_TESTS=1`), which writes diagnostics to `target/ci-report.txt` and `target/ci-report.jsonl`.

## 2. Drive Magic Through `magic-cli`, `--control 9875`, and `magic-cli mcp`

Follow [`docs/mcp.md`](docs/mcp.md) and [`docs/control-protocol.md`](docs/control-protocol.md):

- **Headless & Live Desktop (`--control`)**: Install binaries via `./install.sh --from-source` (or `cargo install --path apps/magic-cli`). Register headless MCP with `claude mcp add magic -- magic-cli mcp` (serving commands and read-only `magic://` resources), or launch `magic --control 9875` and attach live via `magic-cli mcp --connect 9875`, `magic-cli --connect 127.0.0.1:9875`, or `magic-tui --connect 127.0.0.1:9875`.
- **Stage Architecture**:
  - **`01 Storyboard` & `extensions/scribbles/`**: Creative storyboards live in native [`DeckCraft`](https://github.com/storytold/deckcraft) `.deckcraft` / `.pptx` decks with PDF export (`deckcraft --control 7990` / `deckcraft-cli`). User-editable `extensions/scribbles/` templates (`craft-ai-core`) require saved Storyboard slides on disk before generating turnaround or scene plates.
  - **`02 Animations` (`V2`)**: Open multi-tool lane accepting renders and meshes from Blender, Three.js, After Effects, `AniCraft`, `EffectCraft`, `PhotoCraft`, and `VectorCraft` in any format recognized by `FilmCraft`.
  - **`03 Lyrics` (`V3` + `C1`), `04 Motion` (`V4`), & `05 Studio`**: Live `FilmCraft`-aligned editing views, `magic-cli studio map-tracks` reconciliation, and a 2-tab `Live Breakdown` & `Finishing` dock in `05 Studio`.

## House rules

These apply in every Craft repository. A later rule may be stricter, not weaker. The source is `standards/house-rules-core.md` in `storytold/craft-contrib`. Propose changes there, not in a copy.

1. **A person is accountable.** Whoever runs an agent answers for what it opens and for its review comments.
2. **Check, then claim.** Before writing code, read the issue comments, search pull requests for `#N` and the keywords, and check recent commits on `main`. Comment on the issue first. Release a claim you will not finish.
3. **Verify, don't guess.** Open a definition before using it. Confirm the cause in the code, even when someone names a location.
4. **Keep fixes narrow and tested.** Fix what the issue describes, where the bug is. Add a test that fails without the fix.
5. **Review the diff before opening a pull request.** Identifiers exist as used, the test fails without the fix, nothing unrelated changed, and the description is true.
6. **Look for a better pull request.** If another fixes the same thing, close yours when it is better or equal, and say why.
7. **Report only what ran.** Say what you tested and what you could not run. Without a local build or test, say so and rely on CI before marking ready.
8. **`main` changes through a pull request.** Do not push to it. Branch protection enforces this. There is no exception.
9. **Stop and ask only about** licensing outside the allowlist, publishing, releases, a new remote, secrets, spending money, destructive git, and anything else hard to undo. Otherwise keep working. `plan/` is a gitignored notebook, not rules. Without it, do not ask for it and do not stop.
10. **Keep secrets and personal details out** of the repository, logs and requests.
11. **CI stays shared.** Workflows call `storytold/craft-common-actions`. Do not change CI in a feature or fix pull request. If CI cannot cover a change, open an issue there.
12. **Guards say why.** A check that looks removable says what it prevents. Read that before changing it.
13. **Instructions must be readable by everyone.** No notes about one machine, and no links to files a reader cannot open. `craftrules` is retired. Do not link to it.
14. **Keep pull requests moving.** Concurrent pull requests are fine; unanswered actionable feedback is not. Respond in 7 days, a reminder at 14, closure eligible at 30, from when an actionable request waits on the author. Suggestions and unconfirmed bot findings do not start the clock. A fix, explanation or agreed plan resets it; unrelated activity does not. Waiting on a reviewer or maintainer pauses it and does not count against the author. A maintainer decides closure; it is never automatic. Reuse search results, do not poll, and stop at a rate limit.
15. **No agent attribution.** A pull request does not say an agent wrote it. A repository rule may not require that.
