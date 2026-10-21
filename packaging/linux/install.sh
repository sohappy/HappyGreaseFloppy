#!/usr/bin/env bash
# Installs HappyGreaseFloppy for the current user (no root needed): program, menu entry, icon.
#
#   ./install.sh                       from an extracted release archive
#   packaging/linux/install.sh         from the source tree, after `cargo build --release`
#   install.sh /path/to/hgf-gui       any binary
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"

src="${1:-}"
if [ -z "$src" ]; then
  for c in "$here/happygreasefloppy" "$here/../../target/release/happygreasefloppy"; do
    if [ -x "$c" ]; then src="$c"; break; fi
  done
fi
if [ -z "$src" ] || [ ! -x "$src" ]; then
  echo "Program not found. Build it first (cargo build --release -p hgf-gui) or pass its path." >&2
  exit 1
fi
# Icons: every hicolor size plus the scalable SVG (release archive: ./icons, source tree: ../icons).
icondir="$here/icons"
[ -d "$icondir" ] || icondir="$here/../icons"
svg="$icondir/happygreasefloppy.svg"
[ -f "$svg" ] || svg="$here/../icon.svg"

bin="$HOME/.local/bin"
apps="$HOME/.local/share/applications"
icons="$HOME/.local/share/icons/hicolor"
mkdir -p "$bin" "$apps"
install -m 755 "$src" "$bin/happygreasefloppy"
for png in "$icondir"/*.png; do
  size="$(basename "$png" .png)"
  mkdir -p "$icons/${size}x${size}/apps"
  install -m 644 "$png" "$icons/${size}x${size}/apps/happygreasefloppy.png"
done
if [ -f "$svg" ]; then
  mkdir -p "$icons/scalable/apps"
  install -m 644 "$svg" "$icons/scalable/apps/happygreasefloppy.svg"
fi
sed "s|^Exec=.*|Exec=$bin/happygreasefloppy|" "$here/happygreasefloppy.desktop" > "$apps/happygreasefloppy.desktop"
command -v update-desktop-database > /dev/null && update-desktop-database "$apps" || true
command -v gtk-update-icon-cache > /dev/null && gtk-update-icon-cache -q -t "$icons" || true
echo "Installed. Start HappyGreaseFloppy from your applications menu (icon included)."
if ! command -v gw > /dev/null; then
  echo "Note: the Greaseweazle tools (gw) were not found. Install them with: pip install --user greaseweazle"
fi
if ! groups | grep -qw dialout; then
  echo "Note: to access the Greaseweazle without root, add yourself to the dialout group:"
  echo "      sudo usermod -aG dialout \$USER   (then log out and back in)"
fi
