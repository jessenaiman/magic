---
title: Install, Update & MCP Setup
description: Install Magic Studio, magic-cli, and magic-tui from source, and register magic-cli mcp with live --control 9875 watching.
---

## Install From Source

```sh
git clone https://github.com/jessenaiman/magic.git
cd magic
./install.sh --from-source
export PATH="$HOME/.local/bin:${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
```

`./install.sh --from-source` runs `cargo update` automatically to pull the latest [`storytold/craft-libs`](https://github.com/storytold/craft-libs) `main` commits, builds `magic`, `magic-cli`, and `magic-tui`, and installs the desktop launcher, icon, and `.magic` MIME registration (`io.github.jessenaiman.magic.xml`).

## Register MCP & Watch Live With `--control`

Magic Studio follows the same MCP pattern as [DeckCraft](https://github.com/storytold/deckcraft/blob/main/docs/mcp.md) and [DesignCraft](https://github.com/storytold/designcraft/blob/main/docs/mcp.md):

```sh
# Headless stdio MCP session (exposes commands + read-only magic:// resources):
claude mcp add magic -- magic-cli mcp

# Live desktop watching (start `magic --control 9875` first):
magic --control 9875 &
claude mcp add magic-app -- magic-cli mcp --connect 9875
```

## Updating

Pull the latest `main` commits and re-run `./install.sh --from-source` (or check availability via `magic-cli exec app.update`).
