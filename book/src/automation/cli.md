# `magic-cli` Reference

> **Last Reviewed & Updated:** 2026-11-12T16:59:00-05:00 · **Reviewed by:** Jesse A. Naiman

## Global Flags

| Flag | Description |
|---|---|
| `--project <FILE.magic>` | Open `.magic` project in a headless `Session` (mutually exclusive with `--connect`). |
| `--save` / `--save-as <PATH>` | Atomically save `.magic` and dirty session documents after running the command. |
| `--connect <ADDR>` / `--bridge <ADDR>` | Forward commands and `ui` actions to a live `magic --control <PORT>` window (`127.0.0.1:9875`). |
| `--automation-read-root <DIR>` | Confine headless reads to `<DIR>` (repeatable; symlinks rejected). |
| `--automation-write-root <DIR>` | Confine headless writes to `<DIR>` (repeatable; symlinks rejected). |
| `--dry-run` | Print `{dry_run, id, params}` without mutating disk or live state. |

## Core Subcommands

```sh
magic-cli new --audio song.wav --name "Song Title" [--wizard]
magic-cli commands [filter]
magic-cli describe <command.id>
magic-cli --project song.magic inspect [all|project|storyboard|timeline]
magic-cli --project song.magic --save exec <command.id> key=value
magic-cli --project song.magic --save run commands.jsonl
magic-cli --connect 127.0.0.1:9875 ui inspect
magic-cli mcp [--connect 127.0.0.1:9875 | --bridge 127.0.0.1:9875 | --headless]
```
