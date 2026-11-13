---
title: Troubleshooting
description: Resolve missing ArtCraft binaries, --control connection errors, confinement blocks, and saved-Storyboard-slide gates.
---

## `cannot connect to 127.0.0.1:9875`

`magic` does not open a TCP port when launched from the desktop menu without `--control`. Quit any running `magic` instance, start `magic --control 9875` in a terminal, wait for `magic: control listening on 127.0.0.1:9875`, and restart your MCP client or re-run `magic-cli --connect 127.0.0.1:9875 ui inspect`.

## Missing `<craft>-cli` Binary

Run `magic-cli exec app.doctor`. Ensure `deckcraft-cli`, `photocraft-cli`, `vectorcraft-cli`, `effectcraft-cli`, `anicraft-cli` (optional), and `filmcraft-cli` are on `PATH` (for Cargo installs, prepend `~/.cargo/bin` and `~/.local/bin`).

## Scribbles Blocked Until Storyboard Slides Are Saved

Templates and AI recipes under `extensions/scribbles/` require saved Storyboard slides on disk in `01 Storyboard` (`.deckcraft` or `.pptx`) before generating character turnarounds or scene plates. Save your DeckCraft storyboard deck first, then re-run scribble generation.

## Path Blocked by `--automation-read-root` or `--automation-write-root`

When running confined automation, every canonicalized input path must resolve inside an `--automation-read-root` and every output path inside an `--automation-write-root`. Add an additional `--automation-read-root <DIR>` flag if your audio or shared assets live in a separate folder.
