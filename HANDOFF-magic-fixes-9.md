# Desktop Agent Handoff — Build, Push & Open PR (`magic-fixes-9` -> `main`)

> **Last Reviewed & Updated:** 2026-11-12T16:59:00-05:00 · **Reviewed by:** Jesse A. Naiman

This repository snapshot is **iteration `magic-fixes-9`** (following Google Drive `magic-fixes-7` and desktop `/tmp/opencode/magic-fixes-8`). Use the exact sequential branch name **`magic-fixes-9`** so local checkouts, archives, and GitHub branches stay 1-to-1 aligned, and merge into `main` strictly via a Pull Request ([`docs/house-rules-core.md`](docs/house-rules-core.md) Rule 8).

## 1. Upstream `README.md` Ground Truth Verified in `magic-fixes-9`

1. **`storytold/craft-libs` Git Dependency Rule** ([`storytold/craft-libs/README.md`](https://github.com/storytold/craft-libs/blob/main/README.md) lines 20–24):
   ```toml
   craft-<name> = { git = "https://github.com/storytold/craft-libs", rev = "<commit>" }
   ```
   `Cargo.toml` and `Cargo.lock` are pinned cleanly to `rev = "d23f5f36bb9657159ba5468aa9c92d939786922b"` with unused `craft-extensions` (`craft-ext-core`, `craft-ext-store`) and duplicate `craft-atomic` entries removed.
2. **Code Fixes Completed Before the Final Markdown Pass**:
   - **Conflict & Orphan Cleanup**: Resolved all 123 merge-conflicted files cleanly to working `9055759`, removed legacy `0.17.0` orphans, and split `command_sets/creative.rs` + `film.rs` so every `.rs` file (`157` total) is `< 900` lines.
   - **Self-Contained `~/Magic/<slug>/` Architecture (`F8-05`, `U04`)**: Default root `~/Magic` across `settings.rs`, `home.rs`, and `catalog.rs` (auto-discovering `~/Magic/<slug>/*.magic` alongside `config.toml` recents), `~/.local/share/magic/{templates,recovery,logs}` seeding (respecting `MAGIC_CONFIG_DIR=""` isolation per `craft-settings#12`), 5-folder project scaffold (`audio/`, `assets/`, `effects/lyrics.ecproj`, `effects/motion.ecproj`, `film/<slug>.fcproj`, `exports/`), migration-only `original-project.zip` (`project.migrate`), and zero `.steps.json` sidecar writers.
   - **Direct `storyboard.designcraft` Package Writes (`U09`)**: `storyboard.new` honors `template_seed_bytes(Some(s))` and `storyboard.set` updates unlocked `document.json` stories (`utf8` byte `len` + `fontFamily` + `rev`) and `assets/*` directly inside `storyboard.designcraft` via `craft-zip` + `craft-atomic`.
   - **Control Channel, Marker Warnings, TUI & `xtask` Gates (`F8-07`, `U06`, `U08`, `U12`)**: Added `sync_warnings` on marker interchange, 5-tab zero-polling ANSI `magic-tui`, `render.frame` (`max_side`, `base64`, `write`), deduplicated `rmsDbfs`/`peakDbfs` in `ui.inspect` + `ui.playback`, `magic_menu_bar` `title_bar::empty_area`, Rust 1.95 Edition 2024 Clippy fixes (`.is_multiple_of()`, explicit imports, `craft_atomic` build output), decomposed high-cognitive-complexity functions (`rig_track_native_steps`, `trace_silhouette_svg`, `cmd_project_trunk`, `write_state_into_designcraft_zip`, `validate_line_item`, `mode_tab`, `font_family_combo`, `policy::sources`) below Clippy's `cognitive-complexity-threshold = 30`, and added `cargo xtask sources`.

## 2. Exact Step-by-Step Commands for Desktop Agent

Run these commands inside your authenticated local clone of `https://github.com/jessenaiman/magic`:

```sh
# 1. Start from a clean, up-to-date main and cut sequential branch magic-fixes-9
cd /path/to/jessenaiman/magic
git fetch origin
git checkout -B magic-fixes-9 origin/main

# 2. Sync the unpacked magic-fixes-9/ archive into the git working tree (preserving .git/)
unzip -q /path/to/magic-fixes-9.zip -d /tmp/magic-fixes-9-unpack
rsync -a --delete   --exclude='.git/'   --exclude='target/'   /tmp/magic-fixes-9-unpack/magic-fixes-9/ ./

# 3. Verify diff stat and run all workspace quality gates (fmt, layers, assets, sources, clippy -D warnings)
git status --short
cargo xtask ci

# 4. Optional: run unit/integration tests and live app doctor check
MAGIC_RUN_TESTS=1 cargo xtask ci
cargo run --release -p magic-cli -- exec app.doctor

# 5. Stage, commit (NO Co-Authored-By or AI attribution trailers per House Rule 15), and push magic-fixes-9
git add -A
git diff --cached --stat
git commit -m "Consolidate self-contained project layout, DesignCraft ZIP sync, and Rust 1.95 CI gates"
git push -u origin magic-fixes-9

# 6. Open the Pull Request into main (never push directly to main per House Rule 8)
gh pr create   --base main   --head magic-fixes-9   --title "magic-fixes-9: Self-contained ~/Magic/<slug>/ layout, DesignCraft ZIP sync & Rust 1.95 CI gates"   --body "Resolves merge conflicts, pins storytold/craft-libs per README.md, consolidates ~/Magic/<slug>/ 5-folder projects, writes storyboard.designcraft stories/assets directly via craft-zip, adds render.frame base64/max_side and xtask sources gates, and replaces paraphrased reference stubs with direct upstream README.md signposts."
```
