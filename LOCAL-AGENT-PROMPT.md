# Local Agent Prompt — Build, Verify & Push `magic-fixes-10`

Give your local coding agent your local `magic` git clone directory plus [`magic-fixes-10.zip`](https://drive.google.com/file/d/1B3JTnOit_9rOsqV0Msm1AwRx9ujX3qb4/view?usp=drivesdk) from [`drop/`](https://drive.google.com/drive/folders/1ov6fvMVmz1UAihgNw1Ft8ocX-j0UHOLl), then paste the prompt below.

## Prompt to Paste

You are operating in lazy senior-dev (`ponytail`) mode: make the smallest possible fixes needed for a clean compile, lint, test, and push. Deletion beats addition.

1. **Sync `magic-fixes-10.zip` into the git working tree (preserving `.git/`)**:  
     
   - Unzip `magic-fixes-10.zip` to a temp directory and sync `magic-fixes-10/` onto this repository root with `rsync -a --delete --exclude='.git/' <unzipped>/magic-fixes-10/ ./` so all deleted files (`docs/craft-reference/`, `crates/scribbles/`, `xtask/src/policy.rs`, `crates/engine/src/archive.rs`, `crates/engine/src/cmd/words/lyrics_layout.rs`, `crates/engine/src/cmd/character/model3d.rs`, `crates/engine/src/cmd/character/model3d_catalog.json`, `pipeline/templates/kinetic-library.json`, `HANDOFF-magic-fixes-9.md`) are removed from the working tree too. Ignore any standalone `.patch` files in `drop/` — all 7 tracks are already combined in `magic-fixes-10.zip`.

   

2. **Refresh `Cargo.lock` & run full workspace verification**:  
     
   - Run `cargo update` so `Cargo.lock` resolves against latest `main` of [storytold/craft-libs](https://github.com/storytold/craft-libs/blob/main/README.md) (keep `storytold/craft-libs` unpinned in `Cargo.toml`; keep ONLY the two `[patch.crates-io]` pins for `storytold/egui` and `storytold/winit`).  
   - Run `cargo fmt --all -- --check` (or `cargo fmt --all` if formatting differs), `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace`.

   

3. **Fix ONLY real compiler / upstream `craft-libs` drift or test failures — preserve all `magic-fixes-10` invariants**:  
     
   - **Never re-add deleted modules or catalogs**: do not recreate `model3d.rs`, `crates/scribbles/`, `archive.rs`, `lyrics_layout.rs`, `policy.rs`, or `docs/craft-reference/`.  
   - **Keep `02 Animations` visually unchanged**: all `AssetHost` / `AnimationHost` / `animation.promote` / `animation.locate_from_studio` changes stay strictly under the hood; only `05 Studio` uses the 2-tab (`Live Breakdown` \+ `Finishing`) right dock.  
   - **Keep failures noisy**: never swallow host CLI or `--control` errors from [FilmCraft](https://github.com/storytold/filmcraft/blob/main/README.md) (`:9876` / `filmcraft-cli`), [EffectCraft](https://github.com/storytold/effectcraft/blob/main/README.md) (`:9877` / `effectcraft-cli`), [PhotoCraft](https://github.com/storytold/photocraft/blob/main/README.md), [VectorCraft](https://github.com/storytold/vectorcraft/blob/main/README.md), or [DeckCraft](https://github.com/storytold/deckcraft/blob/main/README.md) — surface `EngineError::ExternalTool`.  
   - **Keep [storytold/craft-extensions](https://github.com/storytold/craft-extensions) v1 rules intact**: do not add `craft-extensions` to `Cargo.toml`; keep `crates/engine/src/extensions.rs` enforcing `<= 256 KiB` pre-parse caps, strict character-class IDs/paths with zero traversal, host-assigned `TrustTier` (reject self-declared `trust`), empty `"permissions": []`, and no code in themes.  
   - **House rules**: zero `unsafe`, zero non-test `unwrap` / `expect` / `panic!` / `todo!`, zero non-test `std::fs::write` (use `craft_atomic` / `doc_state::write_file`), and no wildcard imports (`use ...::*;`).

   

4. **Optional host CLI smoke check (if Craft CLIs are on `PATH`) & push**:  
     
   - Run `cargo run -p magic-cli -- animation hosts`, `cargo run -p magic-cli -- app.extensions`, and (if `filmcraft-cli` / `effectcraft-cli` are installed) verify `magic-cli film exec` and `magic-cli effect exec` against a temp project.  
   - Stage all changes (`git add -A`), commit with `"magic-fixes-10: craft-libs unpin, FilmCraft/EffectCraft/Animations API bridges, craft-extensions & Studio debloat"`, and `git push`.  
   - Report back in 5 bullets max: what compiled cleanly, any `craft-libs` drift you patched, test count passed, smoke-check status, and commit SHA pushed.