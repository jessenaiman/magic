---
title: Install, Update & MCP Setup
description: Install Magic Studio, magic-cli, and magic-mcp, and connect live --control watching.
---

## Install From Source

```sh
git clone https://github.com/jessenaiman/magic.git
cd magic
./install.sh --from-source
export PATH="$HOME/.local/bin:${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
```

This installs `magic`, `magic-cli`, `magic-mcp`, the desktop entry, icon, and `.magic` MIME registration (`io.github.jessenaiman.magic.xml`).

## Register MCP & Watch Live With `--control`

Magic Studio follows the same MCP pattern as [VectorCraft](https://github.com/storytold/vectorcraft) and [DesignCraft](https://github.com/storytold/designcraft/blob/main/docs/mcp.md):

```sh
# Auto-attach to live window when open, else fall back to headless:
claude mcp add magic -- magic-cli mcp

# Pin live desktop watching (start `magic --control 9876` first):
magic --control 9876 &
claude mcp add magic-app -- magic-cli mcp --connect 127.0.0.1:9876
```

## Updating

Pull the latest `main` tag or release and re-run `./install.sh --from-source` (or check availability via `magic-cli exec app.update`).
