# Changelog

All notable changes to HappyGreaseFloppy (formerly FluxMyFluffyFloppy).

## v0.17.1

* **Format disk for Commodore, BBC Micro and Apple II**:
  * Commodore 1541 (D64), 1571 (D71) and 1581 (D81), with disk name and ID: byte for byte the
    image that VICE's `c1541 -format "name,id"` creates (664, 1328 and 3160 blocks free);
  * Acorn DFS (BBC Micro, Master, Electron with DFS), 40 or 80 tracks, single- or double-sided
    (two DFS drives), with a title: catalogue identical to dfsimage's;
  * Apple II DOS 3.3 (volume number) and ProDOS (volume name) data disks: the same volume
    structures as AppleCommander, without the boot code (boot DOS or ProDOS from another disk).
* Double step option for 40-track disks (1541, 1571, Apple II, DFS 40, PC 360K) written in an
  80-track 5.25" drive.
* Tests compare the images with VICE c1541, dfsimage and AppleCommander when they are installed
  (`HGF_APPLECOMMANDER=/path/AppleCommander-ac.jar` for the latter), and write a file into each
  blank image with those tools.

## v0.17.0

* **Format disk for PC, Atari ST and MSX**: blank FAT12 disks (PC 360K, 720K, 1.2M, 1.44M,
  2.88M; Atari ST 360K single-sided and 720K double-sided; MSX 1DD and 2DD) with a volume label,
  written with the matching gw format and verified, or saved as an image. The boot sector has
  the standard BIOS parameter block (identical to `mformat`'s), a "Non-system disk" boot code
  on PCs and a non-executable boot sector on the ST. Writing images was never limited to the
  Amiga: the Write page takes every format gw supports.
* Features of FluxMyFluffyFloppy (the Windows GUI by FrankieTheFluff) that were missing:
  * **templates**: named Read and Write settings with a description, saved, replaced, deleted,
    marked "modified" when changed after loading (custom track ranges included);
  * **disk numbering** for reads, with a preview of the next file name;
  * **image file options** (`disk.hfe::bitrate=250:version=3`): HFE bitrate, version,
    interface mode, encoding, double step, uniform; SCP disk type, revolutions, legacy
    single-sided; KryoFlux sample clock; D88 disk index; with a warning for HFE without bitrate;
  * **command line preview** (Read, Write, Convert, Erase) with a copy button;
  * **log file**: every command and its output appended to one file per day;
  * **serial port list** for `--device` (Linux by-id names, macOS usbmodem, Windows registry);
  * `--bt` (backtrace) option; **open folder** buttons; Convert "next to the source";
  * Help links to the Greaseweazle wiki (getting started, troubleshooting, drive select, track
    specification, image types, precompensation, downloads).
* Taskbar/Dock notification when an operation ends while the window is in the background (a
  request made to FluxMyFluffyFloppy, open there).
* "Restore defaults" keeps saved templates and the log folder.
* Fixed: two theme tests could fail when run in parallel (shared text scale).

### Compared with FluxMyFluffyFloppy 5.2.9 (FrankieTheFluff's Windows GUI)

Every gw command and option FluxMyFluffyFloppy drives is available here, and its interface
features above have been added. Not carried over: the Windows code page option (gw output is
read as UTF-8, `PYTHONIOENCODING` is set) and the separate "Diskdefs folder" (gw takes one
`--diskdefs` file, set in Settings). Only here: Linux and macOS, track-by-track visualizer,
hints, re-read of failed tracks, drive health check, disk copy, Format disk, image inspector,
head alignment chart, Amiga compatibility check.

## v0.16.0

* **Amiga compatibility check** on the Write page, for disks that verify fine in the PC drive
  but do not work on the Amiga: flags options left on from earlier sessions that gw's verify
  cannot catch (swap heads, double step, custom tracks, reverse, hard sectors, fake index,
  forced density) with a one-click reset; reads the ADF and warns when it needs Kickstart 2.0+
  (FFS, International, DirCache) and so shows "Not a DOS disk" on an A500 with Kickstart 1.3;
  explains non-DOS (game) and non-bootable disks; troubleshooting tips; a one-click Amiga
  precompensation preset (`--precomp 40=140`, as trackdisk.device does on DD disks).
* **JetBrains Mono** for the whole interface and the console (clear 0/O and 1/l/I, readable on
  dark backgrounds), embedded in the executable; its SIL OFL licence ships in every archive.
* **Text size** setting (80 % to 160 %) in *Settings → Appearance*, applied immediately and
  remembered; widths that hold text (sidebar, forms) follow it.
* **Theme**: Dark, Light or **System** (follows GNOME, KDE, Windows or macOS). Fixed: the chosen
  theme (and text size) was replaced by the desktop's theme at start-up; both egui styles are now
  customised and the preference is explicit (regression tests simulate the desktop theme).
  Settings are now also saved every 5 seconds, so they survive a crash or a killed process
  (previously only a normal close saved them).
* Production review fixes: cancelling a confirmation no longer interrupts an unrelated wizard;
  temporary images are removed when the app closes during a copy or a health check; default read
  destination in the home folder; correct hint when the gw path is not executable; feedback when
  an action is started while gw is still being detected; better contrast of selected items in
  the light theme; bounded sector numbers from gw output; the pin confirmation names the pin and level; temporary files left by a crash are
  removed at the next start; the disk visualizer fits the window at every text size (discs stack
  when they are too wide side by side).
* Release hygiene: CI builds and tests on Windows and macOS too, `.gitattributes` keeps LF line
  endings (scripts stay runnable when checked out on Windows), `THIRD-PARTY-LICENSES.txt` in every
  archive, yanked `yoke-derive` updated.
* Application icon redrawn as a disk seen from above (cylinders as rings, 11 sectors, active
  track in white, a retried track in amber), with simpler designs for small sizes;
  `tools/make-icons.py` regenerates every icon file.
* macOS: `packaging/package.sh macos` also packages a plain `cargo build --release` made on a Mac.

## v0.15.0

* Renamed **HappyGreaseFloppy** (executable `happygreasefloppy`, crates `hgf-core` / `hgf-gui`,
  environment variable `HGF_GW`). Settings of FluxMyFluffyFloppy are copied over on first start.
* **Hints** on gw errors and suspicious results (`hints.rs`, tested with real messages).
* **Re-read failed tracks** (`DiskMap::start_reread`).
* **Custom tracks badge**; the free *Tracks* field is no longer persisted.
* **Format disk** page (`amigados.rs`: blank AmigaDOS images, verified with xdftool and a gw
  round trip).
* **Copy disk** page (one or two drives, sector or flux).
* New **application icon** (a disk's cylinders and sectors) for Linux (all hicolor sizes + SVG), Windows (multi-size .ico) and
  macOS (.icns), with a simplified design for small sizes.

## v0.14.0

* **Drive health** page: guided diagnosis with verdict and report (logic unit-tested with
  captures from a real Greaseweazle V4.1).
* **Automatic detection of gw** at start-up and from Settings.
* **Ready-to-use executables** for Windows, macOS and Linux built by GitHub Actions; application
  icon (window, title bar, Windows executable, macOS bundle, Linux menu entry); Linux user
  installer, also usable from a source build.
* Application id `happygreasefloppy`, so that GNOME and KDE on Wayland show the icon.

## v0.13.0 (since v0.12.1, as FluxMyFluffyFloppy)

* **Removed** the Ratatui TUI; the GUI is the only front-end.
* **All gw commands** are now available, with all their options.
* **Fixes:**
  * device detection no longer reports "connected" on an error message, and is reset when
    the device disappears;
  * `Command Failed:` (which gw reports with exit status 0) is now treated as a failure;
  * operations can be stopped, and gw is interrupted cleanly when the window closes;
  * the format's own geometry is used: `--tracks` is only passed when you ask for it
    (the old 1541 preset truncated disks to 35 tracks and never double-stepped);
  * the *Recovery* image is saved to your file instead of a hidden temporary one;
    *Quick scan* cleans up after itself; the old *Diagnostic* mode was identical to a normal read;
  * an existing file is never overwritten without confirmation (`--no-clobber` otherwise);
  * the image extension follows the chosen preset;
  * `gw seek` can no longer hang waiting for a console answer;
  * a track state can no longer be downgraded by an unrelated output line; partial tracks of
    `gw convert` are marked bad;
  * regexes are compiled once; the console is capped and virtualised;
  * AmigaDOS explorer followed the parent pointer instead of the hash chain, and an empty root
    block was reported as valid;
  * other failures that gw reports with exit status 0 are detected too: `ERROR: …` (device in
    update mode, unsupported firmware, garbled USB data), `** UPDATE FAILED`, and
    `** SKIPPING UPDATE` (shown as a warning with its reason);
  * closing the window during an operation asks for confirmation (an interrupted write leaves
    the disk incomplete); the drive selector is locked while gw runs;
  * a stopped or failed read/convert warns that the image left on disk may be incomplete;
  * risky firmware-update flags (force, bootloader) are never remembered between sessions;
  * Windows: no console window flashes when gw runs.
