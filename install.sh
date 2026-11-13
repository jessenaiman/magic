#!/usr/bin/env bash
# Magic Studio installer (`magic`, `magic-cli`, `magic-mcp`, `magic-tui`, desktop launcher, icon, and `.magic` MIME type).
#
# Usage:
#   ./install.sh                              # Install release binaries or build from source into ~/.local
#   ./install.sh --from-source                # Build pure-Rust `magic`, `magic-cli`, `magic-mcp`, and `magic-tui` via Cargo
#   sudo ./install.sh --prefix /usr/local     # System-wide installation
#   ./install.sh --uninstall                  # Cleanly remove installed binaries, desktop entry, and MIME registration

set -euo pipefail

APP_ID="io.github.jessenaiman.magic"
PREFIX="${PREFIX:-$HOME/.local}"
FROM_SOURCE=0
UNINSTALL=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --prefix)
      PREFIX="${2:?--prefix requires a path}"
      shift 2
      ;;
    --prefix=*)
      PREFIX="${1#*=}"
      shift
      ;;
    --from-source)
      FROM_SOURCE=1
      shift
      ;;
    --uninstall)
      UNINSTALL=1
      shift
      ;;
    -h|--help)
      sed -n '2,9p' "$0"
      exit 0
      ;;
    *)
      echo "install.sh: unknown option: $1" >&2
      exit 2
      ;;
  esac
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="$PREFIX/bin"
SHARE_DIR="$PREFIX/share"
APPS_DIR="$SHARE_DIR/applications"
MIME_DIR="$SHARE_DIR/mime/packages"
ICON_DIR="$SHARE_DIR/icons/hicolor/512x512/apps"

refresh_desktop_caches() {
  if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$APPS_DIR" >/dev/null 2>&1 || true
  fi
  if command -v update-mime-database >/dev/null 2>&1; then
    update-mime-database "$SHARE_DIR/mime" >/dev/null 2>&1 || true
  fi
  if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "$SHARE_DIR/icons/hicolor" >/dev/null 2>&1 || true
  fi
}

if [[ "$UNINSTALL" -eq 1 ]]; then
  rm -f "$BIN_DIR/magic" "$BIN_DIR/magic-cli" "$BIN_DIR/magic-mcp" "$BIN_DIR/magic-tui"
  rm -f "$APPS_DIR/$APP_ID.desktop" "$APPS_DIR/magic.desktop"
  rm -f "$MIME_DIR/$APP_ID.xml"
  rm -f "$ICON_DIR/$APP_ID.png"
  refresh_desktop_caches
  echo "Uninstalled Magic Studio from $PREFIX"
  exit 0
fi

resolve_bin() {
  local name="$1"
  if [[ -x "$SCRIPT_DIR/bin/$name" ]]; then
    printf '%s\n' "$SCRIPT_DIR/bin/$name"
  elif [[ -x "$SCRIPT_DIR/target/release/$name" ]]; then
    printf '%s\n' "$SCRIPT_DIR/target/release/$name"
  else
    return 1
  fi
}

if [[ "$FROM_SOURCE" -eq 1 ]] || ! resolve_bin "magic-cli" >/dev/null || ! resolve_bin "magic-mcp" >/dev/null; then
  echo "Building Magic Studio pure-Rust workspace (magic, magic-cli, magic-mcp, magic-tui)..."
  (cd "$SCRIPT_DIR" && cargo build --release -p magic-cli --bins -p magic -p magic-tui)
fi

mkdir -p "$BIN_DIR" "$APPS_DIR" "$MIME_DIR" "$ICON_DIR"

for bin_name in magic-cli magic-mcp magic magic-tui; do
  if src_bin="$(resolve_bin "$bin_name")"; then
    install -m 0755 "$src_bin" "$BIN_DIR/$bin_name"
    echo "Installed $BIN_DIR/$bin_name"
  elif [[ "$bin_name" == "magic-cli" || "$bin_name" == "magic-mcp" ]]; then
    echo "install.sh: missing required binary '$bin_name' (run ./install.sh --from-source)" >&2
    exit 1
  fi
done

if [[ -f "$SCRIPT_DIR/apps/magic/magic.desktop" ]]; then
  sed "s|^Exec=magic |Exec=$BIN_DIR/magic |" "$SCRIPT_DIR/apps/magic/magic.desktop" > "$APPS_DIR/$APP_ID.desktop"
  chmod 0644 "$APPS_DIR/$APP_ID.desktop"
fi

if [[ -f "$SCRIPT_DIR/packaging/linux/$APP_ID.xml" ]]; then
  install -m 0644 "$SCRIPT_DIR/packaging/linux/$APP_ID.xml" "$MIME_DIR/$APP_ID.xml"
fi

if [[ -f "$SCRIPT_DIR/assets/app-icon/icon.png" ]]; then
  install -m 0644 "$SCRIPT_DIR/assets/app-icon/icon.png" "$ICON_DIR/$APP_ID.png"
fi

refresh_desktop_caches

echo ""
echo "Magic Studio installed to $PREFIX"
echo "  Live Studio + MCP:  $BIN_DIR/magic --control 9875"
echo "  Backstage TUI:      $BIN_DIR/magic-tui --connect 127.0.0.1:9875"
echo "  Register MCP:       claude mcp add magic -- $BIN_DIR/magic-cli mcp"
echo "  Suite Diagnostics:  $BIN_DIR/magic-cli exec app.doctor"
