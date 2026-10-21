#!/usr/bin/env bash
# Packages a release build: packaging/package.sh <linux-x86_64|windows-x86_64|macos-universal|macos> [version]
set -euo pipefail
name="$1"
version="${2:-dev}"
app="HappyGreaseFloppy"
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
mkdir -p dist
dir="$app-$version-$name"
rm -rf "$dir"

# The embedded JetBrains Mono font is under the SIL OFL, whose text must ship with it.
add_licences() {
  cp crates/hgf-gui/assets/fonts/JetBrainsMono-OFL.txt THIRD-PARTY-LICENSES.txt "$1/"
}

case "$name" in
  linux-*)
    mkdir -p "$dir"
    cp target/release/happygreasefloppy "$dir/happygreasefloppy"
    cp -r packaging/icons "$dir/icons"
    cp packaging/icon.svg "$dir/icons/happygreasefloppy.svg"
    cp packaging/linux/happygreasefloppy.desktop packaging/linux/install.sh README.md README.fr.md "$dir/"
    chmod +x "$dir/happygreasefloppy" "$dir/install.sh"
    add_licences "$dir"
    tar czf "dist/$dir.tar.gz" "$dir"
    ;;
  windows-*)
    mkdir -p "$dir"
    cp target/release/happygreasefloppy.exe "$dir/$app.exe"
    cp README.md README.fr.md "$dir/"
    add_licences "$dir"
    cat > "$dir/GREASEWEAZLE.txt" <<'TXT'
HappyGreaseFloppy needs the Greaseweazle host tools (gw.exe).

Easiest: unzip the official Windows release of the Greaseweazle tools
(https://github.com/keirf/greaseweazle/releases) into this folder, so that you get
    HappyGreaseFloppy.exe
    greaseweazle-x.y\gw.exe
HappyGreaseFloppy finds it automatically. It also finds gw.exe on the PATH,
in Python "Scripts" folders, and in greaseweazle-* folders in Downloads,
Desktop or Documents. Otherwise set its location in Settings.
TXT
    7z a -tzip "dist/$dir.zip" "$dir" > /dev/null
    ;;
  macos-*)
    bundle="$dir/$app.app"
    mkdir -p "$bundle/Contents/MacOS" "$bundle/Contents/Resources"
    # Universal binary from the release workflow, or a plain `cargo build --release` on a Mac.
    bin=target/universal/happygreasefloppy
    [ -x "$bin" ] || bin=target/release/happygreasefloppy
    cp "$bin" "$bundle/Contents/MacOS/$app"
    sed "s/@VERSION@/${version#v}/g" packaging/macos/Info.plist > "$bundle/Contents/Info.plist"
    cp packaging/icon.icns "$bundle/Contents/Resources/AppIcon.icns"
    cp README.md README.fr.md "$dir/"
    add_licences "$dir"
    ditto -c -k --keepParent "$dir" "dist/$dir.zip"
    ;;
  *)
    echo "unknown target: $name" >&2
    exit 1
    ;;
esac
rm -rf "$dir"
ls -l dist
