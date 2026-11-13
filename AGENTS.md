# Agent Instructions (`Magic Studio`)

> **Last Reviewed & Updated:** 2026-11-12T16:59:00-05:00 · **Reviewed by:** Jesse A. Naiman

Magic Studio (`magic`, `magic-cli`, `magic-mcp`, `magic-tui`) is a pure-Rust ArtCraft coordinator application built on `eframe` + `egui` 0.37 + [`storytold/craft-libs`](https://github.com/storytold/craft-libs/blob/main/README.md) and governed by [`docs/house-rules-core.md`](docs/house-rules-core.md) ([`storytold/craft-contrib`](https://github.com/storytold/craft-contrib/blob/main/standards/house-rules-core.md)).

## 1. Read Actual Upstream `README.md` Words First (Zero Paraphrasing)

- **Start in `README.md`, Then Explore `docs/`**: Before modifying any crate, dependency, or command, read the **actual words** of [`README.md`](README.md) and the corresponding upstream `storytold/*` `README.md` — never infer architectural rules from prior agent code or paraphrased summaries:
  - Shared Rust libraries (`rev = "<commit>"` in `Cargo.toml`): [`storytold/craft-libs/README.md`](https://github.com/storytold/craft-libs/blob/main/README.md)
  - Shared AI runtime: [`storytold/craft-ai-integrations/README.md`](https://github.com/storytold/craft-ai-integrations/blob/main/README.md)
  - Host Craft applications: [`storytold/designcraft/README.md`](https://github.com/storytold/designcraft/blob/main/README.md), [`storytold/photocraft/README.md`](https://github.com/storytold/photocraft/blob/main/README.md), [`storytold/vectorcraft/README.md`](https://github.com/storytold/vectorcraft/blob/main/README.md), [`storytold/effectcraft/README.md`](https://github.com/storytold/effectcraft/blob/main/README.md), and [`storytold/filmcraft/README.md`](https://github.com/storytold/filmcraft/blob/main/README.md)
  - Shared CI & contribution standards: [`storytold/craft-common-actions/README.md`](https://github.com/storytold/craft-common-actions/blob/main/README.md) and [`storytold/craft-contrib/README.md`](https://github.com/storytold/craft-contrib/blob/main/README.md)
- **Sequential Branch & PR Workflow**: Never push directly to `main`. Cut sequentially named branches (`magic-fixes-9`, `magic-fixes-10`, ...) from `main`, verify `cargo xtask ci`, push the branch, and open a Pull Request into `main` (see [`HANDOFF-magic-fixes-9.md`](HANDOFF-magic-fixes-9.md)).

## 2. Drive Magic Through `magic-cli mcp` and Live `--control`

Follow [`docs/mcp.md`](docs/mcp.md) and [`docs/control-protocol.md`](docs/control-protocol.md):

- **Live Desktop (`--control`)**: Start `magic --control 9875` and attach via `magic-cli mcp --connect 127.0.0.1:9875`, `magic-cli app --port 9875 <cmd>`, or the zero-polling snapshot console `magic-tui --connect 127.0.0.1:9875`.
- **Host Craft Applications**: Magic exchanges native files inside `~/Magic/<slug>/` (`audio/`, `assets/`, `effects/`, `film/`, `exports/`) and drives installed host Craft apps over their documented `--control` sockets and `<app>-cli` binaries (`DesignCraft` `:7979`, `PhotoCraft` `:7878`, `VectorCraft` `:7980`, `EffectCraft` `:9877`, and `FilmCraft` `:9876` authenticated with 64-hex `FILMCRAFT_CONTROL_TOKEN` per [`filmcraft/docs/control-protocol.md`](https://github.com/storytold/filmcraft/blob/main/docs/control-protocol.md)).
- **Modifiable `DesignCraft` Storyboard (`storyboard.designcraft`)**: Seeded from `~/.local/share/magic/templates/storyboard.designcraft` (or custom `project.storyboard_template`). Unlocked text/font slots and placed `assets/*` entries update `storyboard.designcraft` directly via `craft-zip` + `craft-atomic`, while spread additions/deletions and page renders delegate to host `DesignCraft`.
- **Migration-Only `.zip` Rule**: Active projects are plain folders under `~/Magic/<slug>/`; a `.zip` (`<slug>/original-project.zip`) is written strictly when migrating an external legacy folder via `project.migrate`.
- **Single UI Design File**: `figcraft/magic-studio-master.figcraft` (tracked in [`ATTRIBUTION.md`](ATTRIBUTION.md)) is the sole design package.

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
