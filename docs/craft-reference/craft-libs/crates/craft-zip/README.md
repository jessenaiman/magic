# `craft-zip` Reference (`storytold/craft-libs`)

Upstream crate: [`storytold/craft-libs` `crates/zip`](https://github.com/storytold/craft-libs) (`craft-zip`).

Budget-bounded, `unsafe`-free ZIP archive reader, strict raw-byte `preflight` verifier, and deterministic 1980-timestamp ZIP writer built around `zip` from PKWARE `APPNOTE.TXT`.

## Where Magic Studio Uses `craft-zip`

- **Verified Project Folder Archive ([`crates/engine/src/cmd/project/archive.rs`](../../../../crates/engine/src/cmd/project/archive.rs))**: Compresses original legacy project directories into deterministic `.zip` archives (`Writer::new`, `Compression::Deflated`, `preflight::preflight`) and verifies archive budgets via `Archive::open(..., Limits { max_entries, max_total_bytes })` and `has_duplicate_names()`.
- **DesignCraft Package Verification ([`crates/engine/src/cmd/storyboard/crud.rs`](../../../../crates/engine/src/cmd/storyboard/crud.rs))**: Preflights and reads bounded package entries without `flate2` or unbounded central-directory allocations.
