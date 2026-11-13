# `magic-cli` Reference

## Global Flags

| Flag | Description |
|---|---|
| `--project <FILE.magic>` | Open `.magic` project in a headless `Session` (mutually exclusive with `--connect`). |
| `--save` / `--save-as <PATH>` | Atomically save `.magic` and dirty session documents after running the command. |
| `--connect <ADDR>` / `--bridge <ADDR>` | Forward commands and `ui` actions to a live `magic --control <PORT>` window (`127.0.0.1:9875`). |
| `--automation-read-root <DIR>` | Confine headless reads to `<DIR>` (repeatable; canonicalized via `std::fs::canonicalize`). |
| `--automation-write-root <DIR>` | Confine headless writes to `<DIR>` (repeatable; canonicalized via `std::fs::canonicalize`). |
| `--dry-run` | Print `{dry_run, id, params}` without mutating disk or live state. |

## Core Subcommands

```sh
magic-cli new [existing_project] --audio song.wav --name "Song Title"
magic-cli commands [filter]
magic-cli describe <command.id>
magic-cli --project song.magic inspect [all|project|storyboard|timeline]
magic-cli --project song.magic studio map-tracks --save
magic-cli --project song.magic --save exec <command.id> key=value
magic-cli --project song.magic --save run commands.jsonl
magic-cli --connect 127.0.0.1:9875 ui inspect
magic-cli mcp [--connect 9875]
```
