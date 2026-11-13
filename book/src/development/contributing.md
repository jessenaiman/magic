# Contributing & House Rules

> **Last Reviewed & Updated:** 2026-11-12T16:59:00-05:00 · **Reviewed by:** Jesse A. Naiman

Read the actual words of [`README.md`](../../../README.md), [`CONTRIBUTING.md`](../../../CONTRIBUTING.md), and [`docs/house-rules-core.md`](../../../docs/house-rules-core.md) ([`storytold/craft-contrib`](https://github.com/storytold/craft-contrib/blob/main/standards/house-rules-core.md)) before changing code, and navigate directly to each maintained `https://github.com/storytold/<repo>/blob/main/README.md` rather than relying on paraphrased summaries.

- **Single-Responsibility `< 900` Lines**: Enforced by `cargo xtask sources` inside `cargo xtask ci`.
- **Pinned `storytold/craft-libs` Commit (`rev`)**: Follow [`storytold/craft-libs/README.md`](https://github.com/storytold/craft-libs/blob/main/README.md) (`rev = "<commit>"`); `craft-web-api` and `craft-web-host` remain `L6+` UI crates forbidden below `L6`.
- **Self-Contained `~/Magic/<slug>/` Projects**: Migration-only `original-project.zip` (`project.migrate` via `craft-zip`).
- **Sequential PR Branches Only**: Never push to `main`; open a Pull Request from the next sequential branch (`magic-fixes-9`) with zero AI attribution trailers.
