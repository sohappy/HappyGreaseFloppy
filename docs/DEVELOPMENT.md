# Development

## Requirements

* **Rust 1.95 or later** (required by egui/eframe 0.36): install with [rustup](https://rustup.rs),
  update with `rustup update`.
* A C toolchain for linking: `gcc` on Linux, Xcode command-line tools on macOS
  (`xcode-select --install`), *Build Tools for Visual Studio* ("Desktop development with C++") on
  Windows.
* The Greaseweazle host tools (`gw`) to use real hardware, or the simulator below.

## Build and run

```bash
cargo test --workspace
cargo build --release -p hgf-gui
./target/release/happygreasefloppy          # target\release\happygreasefloppy.exe on Windows
```

The first build downloads and compiles every dependency (5 to 10 minutes); later builds are
incremental.

### macOS application

```bash
cargo build --release -p hgf-gui
bash packaging/package.sh macos            # → dist/HappyGreaseFloppy-dev-macos.zip with the .app
```

The device appears as `/dev/cu.usbmodem…` (auto-detected, or chosen in Settings).

### Linux menu entry and icon

Wayland desktops (GNOME, KDE) show the icon of the installed `.desktop` file whose name matches
the application id (`happygreasefloppy`), not the icon embedded in the window:

```bash
cargo build --release -p hgf-gui
packaging/linux/install.sh                 # per-user install: program, menu entry, icons
```

## Trying it without hardware

`tools/fake-gw.py` simulates every `gw` command with output copied from the real tools: a damaged
Amiga disk for `read`, a verify retry for `write`, live `align` reads, `clean` progress, a skipped
`update`, and so on. `convert` is forwarded to the real `gw` when it is installed.

```bash
HGF_GW=$PWD/tools/fake-gw.py ./target/release/happygreasefloppy
```

`FAKE_LOG=/tmp/gw.log` records every command line the application runs; `FAKE_VERIFY_FAIL=1`,
`FAKE_NOINDEX=1` and `FAKE_DISK=erased` reproduce common failures.

## Tests

`cargo test --workspace` runs everything. The blank-disk builders are also compared with
reference implementations when these are installed (the tests are skipped otherwise):

| Builder | Reference | Check |
|---|---|---|
| `amigados.rs` | amitools `xdftool` (reference values recorded in the test) | same boot, root and bitmap blocks as `xdftool format` |
| `fat12.rs` | mtools `mdir`, dosfstools `fsck.fat` | BPB identical to `mformat`, filesystem clean |
| `cbmdos.rs` | VICE `c1541` | byte-for-byte identical to `c1541 -format`, free blocks |
| `acorndfs.rs` | `dfsimage` | identical catalogue, readable on both sides |
| `apple2.rs` | AppleCommander (`HGF_APPLECOMMANDER=/path/AppleCommander-ac.jar`) | identical volume structures |

`crates/hgf-core/tests/fixtures` holds real `gw` captures replayed by `tests/end_to_end.rs`.

## Checklist before a release

1. `cargo fmt --all && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo test --workspace --locked`
2. After a dependency change: `python3 tools/third-party.py` (regenerates
   `THIRD-PARTY-LICENSES.txt`, shipped in every archive) and `cargo audit`.
3. The version in `Cargo.toml` (workspace) and in the title of `CHANGELOG.md`.

## Building the executables with GitHub Actions

* `.github/workflows/ci.yml`: `fmt` and `clippy` on Linux, build and tests on Linux, Windows and
  macOS, on every push and pull request.
* `.github/workflows/release.yml`: *Actions → Release → Run workflow* builds the three archives
  as downloadable artifacts, without publishing anything. Pushing a tag such as `v0.17.1`
  (`git tag v0.17.1 && git push origin v0.17.1`) also publishes them as a GitHub release:
  Windows x86_64 (with embedded icon), macOS universal `.app`, Linux x86_64 (built on
  Ubuntu 22.04 for broad glibc compatibility).

## Application icon

The icon shows a disk seen from above, like the visualizer: its cylinders as rings split into the
11 sectors of an AmigaDOS track, the track under the head in white and one being retried in amber.
`tools/make-icons.py` draws it (`packaging/icon.svg`, with simpler `icon-medium.svg` for 32-64 px
and `icon-small.svg` for 16-24 px) and renders every file below (needs `rsvg-convert` and Pillow).

| System | File | Used by |
|---|---|---|
| Linux | `packaging/icons/<size>.png` (16-512) + `icon.svg` | `install.sh` → `~/.local/share/icons/hicolor` |
| Windows | `packaging/icon.ico` (16-256) | embedded in the `.exe` by `build.rs` |
| macOS | `packaging/icon.icns` (16-1024) | `HappyGreaseFloppy.app` |
| All | `crates/hgf-gui/assets/icon128.rgba` | window icon and title-bar logo |

## Project layout

```
crates/hgf-core/      no GUI dependency, unit-tested
  gw.rs               every gw action and option, arguments, pre-flight validation
  runner.rs           background process, live output, cancellation (SIGINT to the group)
  parse.rs            gw output: tracks, retries, verify failures, geometry, RPM, bandwidth…
  diskmap.rs          per-track and per-sector state
  formats.rs          format catalogue, image suffixes, machine presets, size detection
  trackset.rs         TSPEC parsing and validation
  image.rs            image inspection and AmigaDOS explorer
  health.rs           drive health tests and verdict
  hints.rs            plain-language explanations of gw errors
  locate.rs           automatic detection of gw
  amigados.rs fat12.rs cbmdos.rs acorndfs.rs apple2.rs   blank disk images
  imageopts.rs        image file options (file.hfe::bitrate=250)
  naming.rs ports.rs compat.rs                            disk numbering, serial ports, Amiga checks
  tests/              end-to-end tests on real gw captures
crates/hgf-gui/       egui/eframe application
  app.rs              shell, settings, job lifecycle
  pages.rs            Read, Write, Convert, Erase, drive and device pages, Settings
  copy_ui.rs format_ui.rs health_ui.rs templates.rs       wizards and saved settings
  visualizer.rs widgets.rs theme.rs                        disk view, widgets, colours and fonts
packaging/            icons, Linux installer, macOS Info.plist, package.sh
tools/                fake-gw.py, make-icons.py, third-party.py
docs/                 screenshots, this file
```
