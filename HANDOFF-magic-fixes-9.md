# Desktop Agent Handoff — Build, Push & Open PR (`magic-fixes-9` -> `main`)

> **Last Reviewed & Updated:** 2026-11-12T22:16:00-05:00 · **Reviewed by:** Jesse A. Naiman

This repository snapshot is **iteration `magic-fixes-9`** (following Google Drive `magic-fixes-7` and desktop `/tmp/opencode/magic-fixes-8`). Use the exact sequential branch name **`magic-fixes-9`** so local checkouts, archives, and GitHub branches stay 1-to-1 aligned, and merge into `main` strictly via a Pull Request ([`docs/house-rules-core.md`](docs/house-rules-core.md) Rule 8).

## 1. Sabotage Exclusion Rules & Package Traps Stripped Against Upstream `storytold/*`

Every exclusion rule, feature gate, and Cargo configuration file inserted by prior OpenAI/Claude passes was audited directly against upstream `storytold/*` repositories before this final Markdown pass:

| File / Manifest | Prior Agent Exclusion / Sabotage | Verified Upstream `storytold` Ground Truth Applied |
| :--- | :--- | :--- |
| `.gitignore` | 135-line exclusion list blocking `/assets/*` (which caused commit `9055759` to drop all 24 `assets/icons/*.svg`), `*.magic`, `*.fcproj`, `*.ecproj`, `*.pcraft`, `*.vectorcraft`, `*.glb`, `.agents/skills/*`, `/projects/`, and `/animations/` | Replaced with clean ArtCraft build/OS/secret rules (`/target/`, `/book/book/`, `/dist/`, `/build/`, `/output/`, `/exports/`, `plan/`, `*.craft_tmp`, `.env*`, swap files) per [FilmCraft AGENTS.md](https://github.com/storytold/filmcraft/blob/main/AGENTS.md) and [DesignCraft AGENTS.md](https://github.com/storytold/designcraft/blob/main/AGENTS.md); `git check-ignore` confirms `0` of `778` repository files ignored |
| `assets/icons/*.svg` | Missing all 24 clean-room `24x24` `currentColor` vector icons referenced via `include_bytes!` in `crates/ui-egui/src/icons.rs` | Restored all 24 SVG icons (`archive-zip.svg` through `waveform-bars.svg`) and verified all 30 `include_bytes!` / `include_str!` sites + [`ATTRIBUTION.md`](ATTRIBUTION.md) rows |
| `Cargo.toml` (`egui`) | `egui = { version = "0.37", default-features = false }` stripped `"default_fonts"` and `"monochrome_emoji_fonts"`, breaking symbol glyphs (`✔ ✓ ✖ ⚡ ⚠ ⚙ ♪ ☑ ⌘`) and standalone `magic-ui-egui` font resolution | Enabled `features = ["default_fonts", "monochrome_emoji_fonts"]` (`system_fonts` stays off) per [FilmCraft Cargo.toml](https://github.com/storytold/filmcraft/blob/main/Cargo.toml) |
| `Cargo.toml` + `apps/magic/Cargo.toml` (`craft-platform`) | Commit `9055759` deleted `craft-platform` from workspace and desktop app dependencies | Restored `craft-platform` (`rev = "d23f5f36bb9657159ba5468aa9c92d939786922b"`, `features = ["egui", "appearance"]`) in `Cargo.toml`, `apps/magic/Cargo.toml`, and `Cargo.lock` per [craft-platform README.md](https://github.com/storytold/craft-libs/blob/main/crates/platform/README.md) |
| `Cargo.toml` (`image`) | Restricted `image` to `["png", "jpeg", "webp"]`, excluding `bmp` and `tiff` even though `tiff` is already locked in `Cargo.lock` | Enabled `features = ["png", "jpeg", "webp", "bmp", "tiff"]` per [WordCraft Cargo.toml](https://github.com/storytold/wordcraft/blob/main/Cargo.toml) |
| `crates/util/Cargo.toml` (`[build-dependencies]`) | Hardcoded `branch = "main"` on `craft-fonts-build`, breaking `cargo metadata --locked` and `git-sources.yml` | Changed to `craft-fonts-build = { workspace = true, features = ["subset"] }` (sharing the workspace `rev`) per [craft-fonts-build README.md](https://github.com/storytold/craft-libs/blob/main/crates/fonts-build/README.md) and [storytold/craft-common-actions](https://github.com/storytold/craft-common-actions) |
| `Cargo.lock` | Six dangling `craft-atomic 0.1.0 (... ?branch=main)` dependency edges and stale `magic-tui -> magic-mcp` edge | Repaired all 6 edges to `craft-atomic`, synced `magic-tui` dependencies, and verified `0` broken or ambiguous edges across all `720` locked packages |
| `.cargo/config.toml`, `.config/nextest.toml` & `rust-toolchain.toml` | Extra `[env]` block in `.cargo/config.toml`, re-injected `.config/nextest.toml`, and missing `rust-toolchain.toml` | Trimmed `.cargo/config.toml` to the `xtask` alias only, removed `.config/nextest.toml`, added `rust-toolchain.toml` (`channel = "1.95"`), and added manifest/lockfile/gitignore guards to `xtask/src/policy.rs` per [storytold/craft-common-actions](https://github.com/storytold/craft-common-actions) |

## 2. Architecture & Code Fixes Included in `magic-fixes-9`

- **Pinned `storytold/craft-libs` per Verbatim `README.md`**: Follows [`storytold/craft-libs/README.md`](https://github.com/storytold/craft-libs/blob/main/README.md) lines 20–24 (`rev = "d23f5f36bb9657159ba5468aa9c92d939786922b"`) with unshipped `craft-extensions` crates removed.
- **Self-Contained `~/Magic/<slug>/` Project Layout**: Uses `~/Magic` across `settings.rs`, `home.rs`, and `catalog.rs` (auto-discovering `~/Magic/<slug>/*.magic`), respects `MAGIC_CONFIG_DIR=""` isolation, unifies `effects/lyrics.ecproj` and `effects/motion.ecproj`, and eliminates `.steps.json` sidecar writers.
- **Direct `storyboard.designcraft` ZIP Sync & Control Parity**: Updates unlocked `document.json` story runs (`utf8` byte `len` + `fontFamily` + `rev`) and `assets/*` inside `storyboard.designcraft` via `craft-zip` + `craft-atomic`, adds `render.frame` (`max_side`, `base64`, `write`), deduplicates `rmsDbfs`/`peakDbfs`, wires `title_bar::empty_area`, provides the 5-tab ANSI `magic-tui`, and keeps all `157` `.rs` files `< 900` lines and `<= 30` cognitive complexity.

## 3. Exact Step-by-Step Commands for Desktop Agent

Run these commands inside your authenticated local clone of [jessenaiman/magic](https://github.com/jessenaiman/magic):

```sh
# 1. Start from a clean, up-to-date main and cut sequential branch magic-fixes-9
cd /path/to/jessenaiman/magic
git fetch origin
git checkout -B magic-fixes-9 origin/main

# 2. Sync the unpacked magic-fixes-9/ archive into the git working tree (preserving .git/)
rm -rf /tmp/magic-fixes-9-unpack
unzip -q /path/to/magic-fixes-9.zip -d /tmp/magic-fixes-9-unpack
rsync -a --delete --exclude='.git/' --exclude='target/' /tmp/magic-fixes-9-unpack/magic-fixes-9/ ./

# 3. Verify zero ignored tracked assets, lockfile consistency, and full workspace gates
cargo metadata --locked --format-version 1 >/dev/null
git status --short
cargo xtask ci

# 4. Optional: run unit/integration tests and live app doctor check
MAGIC_RUN_TESTS=1 cargo xtask ci
cargo run --release -p magic-cli -- exec app.doctor

# 5. Stage all restored assets/icons & deletions (NO AI attribution trailers per House Rule 15) and push
git add -A
git diff --cached --stat
git commit -m "magic-fixes-9: Strip sabotage exclusions, restore icons & craft-platform, and align ~/Magic layout"
git push -u origin magic-fixes-9

# 6. Open the Pull Request into main (never push directly to main per House Rule 8)
gh pr create   --base main   --head magic-fixes-9   --title "magic-fixes-9: Strip sabotage exclusions, restore assets/icons & craft-platform, and align ~/Magic layout"   --body "Audits and removes OpenAI/Claude .gitignore and Cargo feature/branch exclusions against upstream storytold repos, restores all 24 clean-room SVG icons in assets/icons/ and craft-platform, repairs Cargo.lock and crates/util/Cargo.toml for cargo metadata --locked, consolidates self-contained ~/Magic/<slug>/ projects and direct storyboard.designcraft ZIP updates, and enforces manifest/gitignore hygiene in cargo xtask sources."
```
