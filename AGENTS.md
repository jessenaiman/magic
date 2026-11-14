<!-- craft-house-rules:start core source=dde09ea31e2e457f5b4653a6e76f7cdf55944bdf sha256=540e6cea75d12d0710c3dca89658328e81e508ecef102446ca5bb95b7c0b92bb -->
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
<!-- craft-house-rules:end -->
<!-- craft-house-rules:start rust source=dde09ea31e2e457f5b4653a6e76f7cdf55944bdf sha256=387e0ccb4b49e567f60951d7d6fff7305054c89f43ad812b9993dc7c38528013 -->
## Rust Craft repositories

These apply to native Craft apps and to shared Rust libraries. Select them by repository type. A repository that merely contains some Rust does not take this block. Rules later in a repository's file may be stricter, but not weaker. The source is `standards/house-rules-rust.md` in [storytold/craft-contrib](https://github.com/storytold/craft-contrib).

1. **Never crash.** Non-test code does not panic and has no `unsafe`, except where the repository file names an exception. Return errors, treat input as hostile, and bound recursion and allocations. New production crate roots copy the `deny` attribute from an existing crate: `clippy::unwrap_used`, `clippy::expect_used`, `clippy::panic`, `clippy::unimplemented`, `clippy::todo`, `clippy::unreachable`. Every crash fix comes with a test that panicked before the fix.
2. **Clean room.** Learn from observed behaviour and public specifications. Do not copy another product's code, assets, strings or artwork. Check a dependency's licence before adding it.
3. **Assets and fonts.** Every asset is original or suitably licensed, and is recorded in `ATTRIBUTION.md` in the same change. Font files live in [storytold/craft-fonts](https://github.com/storytold/craft-fonts), not in the app repository.
4. **Rust only for native Craft apps.** No handwritten JavaScript or TypeScript, and no Tauri, Electron or webview shell, in those apps. Shared tooling follows its documented technical scope and maintainer-approved exceptions. Craft PR Review's existing Tauri and TypeScript UI is one such exception. It does not relax this rule for native apps. Preserve any existing web build.
5. **Time goes through `chrono`.** Do not hand-roll calendar conversion. For dates beyond what `chrono` can represent, follow [standards/time.md](https://github.com/storytold/craft-contrib/blob/main/standards/time.md).
6. **Contributor credits.** Add or edit only your own person's entry in `contributors/people.toml`, and only after asking them. Never guess a name or hand-edit `contributors.json`.

## Licence allowlist

Dependencies and assets use SPDX identifiers. An expression is allowed when every identifier is in this list. A repository may name a shorter list. It may not add an identifier here by editing its own copy of these rules.

- `ISC`
- `MIT`
- `BSD-2-Clause`
- `BSD-3-Clause`
- `Apache-2.0`
- `OFL-1.1`
- `Ubuntu-font-1.0`
- `Bitstream-Vera`
- `CC0-1.0`
- `CC-BY-4.0`
- `CC-BY-SA-4.0`
- `LicenseRef-Public-Domain`

`LicenseRef-Storyteller-Trademark` is not in this list. It covers Storyteller brand marks only, where a repository already says so.
<!-- craft-house-rules:end -->

# Magic Studio — instructions for agents

Magic Studio (`magic`, `magic-cli`) is a pure-Rust ArtCraft production coordinator built on `eframe` + `egui` 0.37 and [`storytold/craft-libs`](https://github.com/storytold/craft-libs/blob/main/README.md). *(Root workspace license: `LicenseRef-Magic-Studio-NonResale` in [`LICENSE`](LICENSE); all shared crates and dependencies follow the Craft allowlist above.)*

## 1. Workspace & Floating Dependencies

- **Crates**: `crates/util` (`magic-util`), `crates/engine` (`magic-engine`), `crates/mcp` (`magic-mcp`), `crates/ui-egui` (`magic-ui-egui`), `apps/magic`, `apps/magic-cli`, and `xtask`.
- **Floating `storytold/*` branches (never pin `rev =`)**: [`Cargo.toml`](Cargo.toml) tracks [`storytold/craft-libs`](https://github.com/storytold/craft-libs) (`branch = "main"`), [`storytold/craft-ai-integrations`](https://github.com/storytold/craft-ai-integrations) (`branch = "main"`), [`storytold/egui`](https://github.com/storytold/egui) (`branch = "craft/0.37-pin"`), and [`storytold/winit`](https://github.com/storytold/winit) (`branch = "craft/0.30.13"`). Never hardcode `rev =` commits or build with `--locked`; install and update via `./install.sh --from-source` (`cargo update` + release build into `~/.local/bin`) and verify with `cargo xtask ci` (writing `target/ci-report.txt` and `target/ci-report.jsonl`).
- **Read actual upstream docs before integrating**:
  - [`storytold/craft-libs`](https://github.com/storytold/craft-libs/blob/main/README.md) & [`docs/ai-shared-layer.md`](https://github.com/storytold/craft-libs/blob/main/docs/ai-shared-layer.md)
  - [`storytold/craft-ai-integrations`](https://github.com/storytold/craft-ai-integrations/blob/main/README.md)
  - [`DesignCraft`](https://github.com/storytold/designcraft/blob/main/README.md), [`EffectCraft`](https://github.com/storytold/effectcraft/blob/main/README.md), [`FilmCraft`](https://github.com/storytold/filmcraft/blob/main/README.md), [`PhotoCraft`](https://github.com/storytold/photocraft/blob/main/README.md), [`VectorCraft`](https://github.com/storytold/vectorcraft/blob/main/README.md), and [`AniCraft`](https://github.com/storytold/anicraft/blob/main/README.md)

## 2. Architecture, Control & Stage Boundaries

Follow [`README.md`](README.md), [`docs/architecture.md`](docs/architecture.md), [`docs/control-protocol.md`](docs/control-protocol.md), and [`docs/mcp.md`](docs/mcp.md):

- **All docs live under `docs/`**: developer/agent specifications live in `docs/*.md`, brand marks in `docs/brand/`, and user documentation in `docs/user/`.
- **Single-point save/load/recovery**: `.magic` reads and writes flow through `magic_engine::cmd::project` and `craft_atomic`; recovery journals write exclusively under `<MAGIC_CONFIG_DIR>/Recovery` (or settings dir), never overwriting committed `.magic` files or creating project-local `.history/` backups.
- **Control & MCP**: `magic --control 9875` (`MAGIC_CONTROL_PORT`), `magic-cli mcp` (headless stdio with `magic://` resources), and `magic-cli mcp --connect 9875`.
- **Suite tools**: `01 Storyboard` uses [`DesignCraft`](https://github.com/storytold/designcraft) (`designcraft --control 7979` / `designcraft-cli`, `.designcraft` / `.idml` / `.indt`); `02 Animations` (`V2`) is an open multi-tool lane; `03 Lyrics` (`V3` + `C1`) & `04 Motion` (`V4`) target [`EffectCraft`](https://github.com/storytold/effectcraft) (`:9877` / `effectcraft-cli`); `05 Studio` reconciles tracks into [`FilmCraft`](https://github.com/storytold/filmcraft) (`:9876` / `filmcraft-cli`).
