---
title: Troubleshooting
description: Resolve missing ArtCraft binaries, --control connection errors, confinement blocks, and storyboard gates.
---

## `cannot connect to 127.0.0.1:9876`

`magic` does not open a TCP port when launched from the desktop menu without `--control`. Quit any running `magic` instance, start `magic --control 9876` in a terminal, wait for `magic: control listening on 127.0.0.1:9876`, and restart your MCP client or re-run `magic-cli --connect 127.0.0.1:9876 ui inspect`.

## Missing `<craft>-cli` Binary

Run `magic-cli exec app.doctor`. Ensure `designcraft-cli`, `photocraft-cli`, `vectorcraft-cli`, `effectcraft-cli`, and `filmcraft-cli` are on `PATH` (for Cargo installs, prepend `~/.cargo/bin`).

## `slot '<name>_cli' is a read-only magic-cli fine-print instruction frame`

Frames ending in `_cli` inside `storyboard.designcraft` are locked fine-print reference labels on Layer 2 (`"CLI Fineprint (Read-Only)"`). Target the editable slot name without `_cli` (for example `label=cover_title` instead of `label=cover_title_cli`).

## Path Blocked by `--automation-read-root` or `--automation-write-root`

When running confined automation, every input path must sit inside an `--automation-read-root` and every output path inside an `--automation-write-root` without crossing symbolic links. Add an additional `--automation-read-root <DIR>` flag if your audio or shared assets live in a separate folder.
