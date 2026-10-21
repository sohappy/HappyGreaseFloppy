//! "Format disk" page: a blank disk image built in memory and written with gw.
//!
//! Amiga (AmigaDOS OFS/FFS), FAT12 (PC, Atari ST, MSX), Commodore DOS (1541, 1571, 1581),
//! Acorn DFS (BBC Micro) and Apple II (DOS 3.3, ProDOS).

use crate::app::{App, JobExtras, Level, Outcome, Page};
use crate::theme;
use crate::widgets::{self, card};
use eframe::egui::{self, RichText, Ui};
use egui_phosphor::regular as ph;
use hgf_core::amigados::{self, Filesystem};
use hgf_core::fat12::{self, Kind, System};
use hgf_core::gw::{Action, WriteOpts};
use hgf_core::{acorndfs, apple2, cbmdos};

/// Machine the blank disk is for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Machine {
    #[default]
    Amiga,
    Pc,
    AtariSt,
    Msx,
    Commodore,
    Bbc,
    Apple2,
}

impl Machine {
    const ALL: [Machine; 7] = [
        Machine::Amiga,
        Machine::Pc,
        Machine::AtariSt,
        Machine::Msx,
        Machine::Commodore,
        Machine::Bbc,
        Machine::Apple2,
    ];

    fn label(self) -> &'static str {
        match self {
            Machine::Amiga => "Amiga",
            Machine::Pc => "PC",
            Machine::AtariSt => "Atari ST",
            Machine::Msx => "MSX",
            Machine::Commodore => "Commodore",
            Machine::Bbc => "BBC Micro",
            Machine::Apple2 => "Apple II",
        }
    }

    fn fat_system(self) -> Option<System> {
        match self {
            Machine::Pc => Some(System::Pc),
            Machine::AtariSt => Some(System::AtariSt),
            Machine::Msx => Some(System::Msx),
            _ => None,
        }
    }
}

/// What the Format page will produce: image bytes, gw format, extension and descriptions.
struct Blank {
    image: Vec<u8>,
    gw_format: &'static str,
    extension: &'static str,
    /// For the confirmation, e.g. `blank DD disk "Empty" (OFS, DOS0)`.
    describe: String,
    /// Shown when the write succeeded.
    success: String,
    /// Default file name for "Save as image".
    file_stem: String,
}

impl App {
    pub fn page_format(&mut self, ui: &mut Ui) {
        let c = theme::FORMAT;
        let busy = self.busy() || self.detecting;
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(theme::w(400.0));
                egui::ScrollArea::vertical().id_salt("format_form").auto_shrink([false, false]).show(
                    ui,
                    |ui| {
                        card(ui, ph::DESKTOP_TOWER, "Computer", c, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                for m in Machine::ALL {
                                    if ui.selectable_label(self.s.format_machine == m, m.label()).clicked()
                                        && self.s.format_machine != m
                                    {
                                        self.s.format_machine = m;
                                        if let Some(sys) = m.fat_system() {
                                            let f = &mut self.s.format_fat;
                                            if f.kind.system() != sys {
                                                f.kind = Kind::default_for(sys);
                                            }
                                            f.system = sys;
                                        }
                                    }
                                }
                            });
                        });
                        ui.add_space(8.0);
                        let valid = match self.s.format_machine {
                            Machine::Amiga => self.amiga_form(ui),
                            Machine::Commodore => self.cbm_form(ui),
                            Machine::Bbc => self.dfs_form(ui),
                            Machine::Apple2 => self.apple_form(ui),
                            _ => self.fat_form(ui),
                        };
                        if self.forty_tracks() {
                            ui.add_space(8.0);
                            card(ui, ph::GEAR, "5.25\" drive", c, |ui| {
                                ui.checkbox(&mut self.s.format_double_step, "80-track drive (PC 1.2M): double step")
                                    .on_hover_text(
                                        "40-track disks need double stepping in an 80-track (96 tpi) drive. \
                                         Leave off with a 40-track (360K) drive.",
                                    );
                                ui.label(
                                    RichText::new(
                                        "A 40-track disk written in an 80-track drive is best read back on the \
                                         original machine with a new or bulk-erased disk.",
                                    )
                                    .small()
                                    .color(theme::MUTED),
                                );
                            });
                        }
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(
                                "Like a full format: every track is written and verified. \
                                 The disk is not made bootable.",
                            )
                            .small()
                            .color(theme::MUTED),
                        );
                        ui.add_space(8.0);
                        if widgets::action_button(ui, ph::SPARKLE, "FORMAT DISK", c, !busy && valid) {
                            self.format_disk();
                        }
                        ui.add_space(4.0);
                        if ui
                            .add_enabled(
                                valid,
                                egui::Button::new(format!("{}  Save as image…", ph::FLOPPY_DISK_BACK)),
                            )
                            .on_hover_text("Create the blank image without writing a disk")
                            .clicked()
                        {
                            self.save_blank_image();
                        }
                    },
                );
            });
            ui.separator();
            ui.vertical(|ui| self.visualizer(ui));
        });
    }

    /// Amiga options. Returns whether they are valid.
    fn amiga_form(&mut self, ui: &mut Ui) -> bool {
        let c = theme::FORMAT;
        let o = &mut self.s.format;
        card(ui, ph::FLOPPY_DISK, "Amiga disk", c, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut o.hd, false, "DD 880K");
                ui.selectable_value(&mut o.hd, true, "HD 1.76M");
            });
            ui.label(
                RichText::new(if o.hd {
                    "HD media in an HD-capable Amiga drive (A4000, A1200 with HD drive)."
                } else {
                    "DD media (one square hole). Read by every Amiga."
                })
                .small()
                .color(theme::MUTED),
            );
        });
        ui.add_space(8.0);
        card(ui, ph::TEXT_AA, "Volume", c, |ui| {
            ui.horizontal(|ui| {
                ui.label("Name");
                ui.add(egui::TextEdit::singleline(&mut o.name).char_limit(30).desired_width(theme::w(260.0)));
            });
            if let Err(e) = amigados::validate_name(&o.name) {
                ui.label(RichText::new(format!("{}  {e}", ph::WARNING)).small().color(theme::BAD));
            }
            ui.add_space(4.0);
            ui.label("Filesystem");
            for fs in [Filesystem::Ofs, Filesystem::Ffs] {
                ui.radio_value(&mut o.filesystem, fs, fs.label());
            }
            ui.checkbox(&mut o.international, "International mode (accented file names)")
                .on_hover_text("DOS2/DOS3: needs Kickstart 2.0 or later");
            let t = o.dos_type();
            ui.label(RichText::new(format!("Disk type: DOS{t}")).small().color(theme::MUTED));
            if o.international && o.filesystem == Filesystem::Ofs {
                ui.label(
                    RichText::new("OFS International is not readable by Kickstart 1.x.")
                        .small()
                        .color(theme::WARN),
                );
            }
        });
        amigados::validate_name(&self.s.format.name).is_ok()
    }

    /// Does the chosen disk have 40 tracks (double step option)?
    fn forty_tracks(&self) -> bool {
        match self.s.format_machine {
            Machine::Commodore => self.s.format_cbm.drive.is_525(),
            Machine::Apple2 => true,
            Machine::Bbc => !self.s.format_dfs.tracks80,
            Machine::Pc => self.s.format_fat.kind == Kind::Pc360,
            _ => false,
        }
    }

    fn error_line(ui: &mut Ui, e: &str) {
        ui.label(RichText::new(format!("{}  {e}", ph::WARNING)).small().color(theme::BAD));
    }

    fn muted(ui: &mut Ui, text: &str) {
        ui.label(RichText::new(text).small().color(theme::MUTED));
    }

    /// Commodore 1541 / 1571 / 1581 options.
    fn cbm_form(&mut self, ui: &mut Ui) -> bool {
        let c = theme::FORMAT;
        let o = &mut self.s.format_cbm;
        card(ui, ph::FLOPPY_DISK, "Drive", c, |ui| {
            for d in cbmdos::Drive::ALL {
                ui.radio_value(&mut o.drive, d, d.label());
            }
            Self::muted(ui, o.drive.note());
            Self::muted(
                ui,
                &format!("{} blocks free  ·  gw format {}", o.drive.blocks_free(), o.drive.gw_format()),
            );
        });
        ui.add_space(8.0);
        card(ui, ph::TEXT_AA, "Header", c, |ui| {
            egui::Grid::new("cbm_header").num_columns(2).show(ui, |ui| {
                ui.label("Disk name");
                ui.add(egui::TextEdit::singleline(&mut o.name).char_limit(16).desired_width(theme::w(200.0)));
                ui.end_row();
                ui.label("ID");
                ui.add(egui::TextEdit::singleline(&mut o.id).char_limit(2).desired_width(theme::w(40.0)));
                ui.end_row();
            });
            match cbmdos::validate_name(&o.name).and_then(|_| cbmdos::validate_id(&o.id)) {
                Err(e) => Self::error_line(ui, &e),
                Ok(()) => Self::muted(
                    ui,
                    &format!(
                        "Like HEADER \"{},{}\" (stored in capitals).",
                        o.name.to_uppercase(),
                        o.id.to_uppercase()
                    ),
                ),
            }
        });
        cbmdos::validate_name(&o.name).is_ok() && cbmdos::validate_id(&o.id).is_ok()
    }

    /// Acorn DFS options.
    fn dfs_form(&mut self, ui: &mut Ui) -> bool {
        let c = theme::FORMAT;
        let o = &mut self.s.format_dfs;
        card(ui, ph::FLOPPY_DISK, "Disk", c, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut o.tracks80, false, "40 tracks");
                ui.selectable_value(&mut o.tracks80, true, "80 tracks");
            });
            ui.horizontal(|ui| {
                ui.selectable_value(&mut o.double_sided, false, "Single-sided");
                ui.selectable_value(&mut o.double_sided, true, "Double-sided");
            });
            Self::muted(
                ui,
                &format!("{}  ·  FM (single density)  ·  gw format {}", o.label(), o.gw_format()),
            );
            if o.double_sided {
                Self::muted(ui, "Each side is a separate DFS drive: side 1 is drive 0, side 2 is drive 2.");
            }
        });
        ui.add_space(8.0);
        card(ui, ph::TEXT_AA, "Title", c, |ui| {
            ui.horizontal(|ui| {
                ui.label("Title");
                ui.add(
                    egui::TextEdit::singleline(&mut o.title)
                        .char_limit(12)
                        .hint_text("none")
                        .desired_width(theme::w(160.0)),
                );
            });
            match acorndfs::validate_title(&o.title) {
                Err(e) => Self::error_line(ui, &e),
                Ok(()) => Self::muted(ui, "Up to 12 characters, as *TITLE. 31 files per side."),
            }
        });
        acorndfs::validate_title(&o.title).is_ok()
    }

    /// Apple II DOS 3.3 / ProDOS options.
    fn apple_form(&mut self, ui: &mut Ui) -> bool {
        let c = theme::FORMAT;
        let o = &mut self.s.format_apple;
        card(ui, ph::FLOPPY_DISK, "Operating system", c, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut o.os, apple2::Os::Dos33, "DOS 3.3");
                ui.selectable_value(&mut o.os, apple2::Os::ProDos, "ProDOS");
            });
            Self::muted(
                ui,
                &format!(
                    "5.25\" disk, 35 tracks, 140K, GCR  ·  {} free  ·  gw format {}",
                    if o.os == apple2::Os::Dos33 { "528 sectors" } else { "273 blocks" },
                    o.os.gw_format()
                ),
            );
            Self::muted(ui, "A data disk: boot DOS or ProDOS from another disk, then use this one.");
        });
        ui.add_space(8.0);
        card(ui, ph::TEXT_AA, "Volume", c, |ui| match o.os {
            apple2::Os::Dos33 => {
                ui.horizontal(|ui| {
                    ui.label("Volume number");
                    ui.add(egui::DragValue::new(&mut o.volume).range(1..=254));
                });
                Self::muted(ui, "254 is the usual value (INIT default).");
            }
            apple2::Os::ProDos => {
                ui.horizontal(|ui| {
                    ui.label("Volume name");
                    ui.add(
                        egui::TextEdit::singleline(&mut o.name).char_limit(15).desired_width(theme::w(180.0)),
                    );
                });
                match apple2::validate_prodos_name(&o.name) {
                    Err(e) => Self::error_line(ui, &e),
                    Ok(()) => Self::muted(ui, &format!("/{}/", o.name.trim().to_uppercase())),
                }
            }
        });
        apple2::validate(o).is_ok()
    }

    /// PC / Atari ST / MSX options. Returns whether they are valid.
    fn fat_form(&mut self, ui: &mut Ui) -> bool {
        let c = theme::FORMAT;
        let o = &mut self.s.format_fat;
        let system = self.s.format_machine.fat_system().unwrap_or_default();
        o.system = system;
        if o.kind.system() != system {
            o.kind = Kind::default_for(system);
        }
        card(ui, ph::FLOPPY_DISK, "Disk size", c, |ui| {
            for k in Kind::of(system) {
                let l = k.layout();
                ui.radio_value(&mut o.kind, k, l.label);
            }
            let l = o.kind.layout();
            ui.label(RichText::new(l.note).small().color(theme::MUTED));
            ui.label(
                RichText::new(format!(
                    "{} free  ·  {} tracks × {} side{} × {} sectors  ·  gw format {}",
                    widgets::human_size(l.free_bytes() as u64),
                    l.cyls,
                    l.heads,
                    if l.heads > 1 { "s" } else { "" },
                    l.spt,
                    l.gw_format
                ))
                .small()
                .color(theme::MUTED),
            );
            if l.hd {
                ui.label(
                    RichText::new(format!("{}  Needs HD media (two square holes on 3.5\" disks).", ph::INFO))
                        .small()
                        .color(theme::MUTED),
                );
            }
        });
        ui.add_space(8.0);
        card(ui, ph::TEXT_AA, "Volume", c, |ui| {
            ui.horizontal(|ui| {
                ui.label("Label");
                ui.add(
                    egui::TextEdit::singleline(&mut o.label)
                        .char_limit(11)
                        .hint_text("none")
                        .desired_width(theme::w(160.0)),
                );
            });
            match fat12::validate_label(&o.label) {
                Err(e) => {
                    ui.label(RichText::new(format!("{}  {e}", ph::WARNING)).small().color(theme::BAD));
                }
                Ok(()) => {
                    ui.label(
                        RichText::new("Up to 11 characters, stored in capitals. FAT12 filesystem.")
                            .small()
                            .color(theme::MUTED),
                    );
                }
            }
        });
        fat12::validate_label(&self.s.format_fat.label).is_ok()
    }

    fn blank(&self) -> Result<Blank, String> {
        match self.s.format_machine {
            Machine::Amiga => {
                let o = &self.s.format;
                let fs = if o.filesystem == Filesystem::Ffs { "FFS" } else { "OFS" };
                let kind = if o.hd { "HD" } else { "DD" };
                Ok(Blank {
                    image: amigados::blank_image(o)?,
                    gw_format: o.gw_format(),
                    extension: "adf",
                    describe: format!("blank {kind} disk \"{}\" ({fs}, DOS{})", o.name, o.dos_type()),
                    success: format!("Disk formatted: \"{}\", {fs}, all tracks verified.", o.name),
                    file_stem: o.name.clone(),
                })
            }
            Machine::Commodore => {
                let o = &self.s.format_cbm;
                let d = o.drive;
                let name = format!("\"{},{}\"", o.name.to_uppercase(), o.id.to_uppercase());
                Ok(Blank {
                    image: cbmdos::blank_image(o)?,
                    gw_format: d.gw_format(),
                    extension: d.extension(),
                    describe: format!("blank Commodore {} disk {name}", d.label()),
                    success: format!("Disk formatted: Commodore {} {name}, all tracks verified.", d.label()),
                    file_stem: o.name.trim().to_string(),
                })
            }
            Machine::Bbc => {
                let o = &self.s.format_dfs;
                let title = if o.title.trim().is_empty() {
                    "no title".to_string()
                } else {
                    o.title.trim().to_string()
                };
                Ok(Blank {
                    image: acorndfs::blank_image(o)?,
                    gw_format: o.gw_format(),
                    extension: o.extension(),
                    describe: format!("blank BBC Micro DFS disk, {} ({title})", o.label()),
                    success: format!("Disk formatted: DFS {} ({title}), all tracks verified.", o.label()),
                    file_stem: if o.title.trim().is_empty() {
                        "blank-dfs".into()
                    } else {
                        o.title.trim().to_string()
                    },
                })
            }
            Machine::Apple2 => {
                let o = &self.s.format_apple;
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let what = match o.os {
                    apple2::Os::Dos33 => format!("DOS 3.3, volume {}", o.volume),
                    apple2::Os::ProDos => format!("ProDOS /{}/", o.name.trim().to_uppercase()),
                };
                Ok(Blank {
                    image: apple2::blank_image(o, now)?,
                    gw_format: o.os.gw_format(),
                    extension: o.os.extension(),
                    describe: format!("blank Apple II disk ({what})"),
                    success: format!("Disk formatted: Apple II {what}, all tracks verified."),
                    file_stem: match o.os {
                        apple2::Os::Dos33 => "blank-dos33".into(),
                        apple2::Os::ProDos => o.name.trim().to_string(),
                    },
                })
            }
            m => {
                let o = &self.s.format_fat;
                let l = o.kind.layout();
                let name =
                    if o.label.trim().is_empty() { "no label".to_string() } else { o.label.to_uppercase() };
                Ok(Blank {
                    image: fat12::blank_image(o)?,
                    gw_format: l.gw_format,
                    extension: l.extension,
                    describe: format!("blank {} {} disk ({name})", m.label(), l.label),
                    success: format!(
                        "Disk formatted: {} {} ({name}), all tracks verified.",
                        m.label(),
                        l.label
                    ),
                    file_stem: if o.label.trim().is_empty() {
                        format!("blank-{}", l.gw_format.replace('.', "-"))
                    } else {
                        o.label.trim().to_string()
                    },
                })
            }
        }
    }

    fn format_disk(&mut self) {
        let b = match self.blank() {
            Ok(b) => b,
            Err(e) => return self.notice = Some(e),
        };
        let path = std::env::temp_dir().join(format!("hgf-format-{}.{}", std::process::id(), b.extension));
        if let Err(e) = std::fs::write(&path, &b.image) {
            return self.notice = Some(format!("Cannot create the blank image: {e}"));
        }
        let what = match self.s.format_machine {
            Machine::Amiga => "an empty AmigaDOS volume",
            Machine::Commodore => "an empty Commodore DOS disk",
            Machine::Bbc => "an empty DFS catalogue",
            Machine::Apple2 => "an empty Apple II volume",
            _ => "an empty FAT12 disk",
        };
        let tracks = hgf_core::gw::TrackOpts {
            double_step: self.forty_tracks() && self.s.format_double_step,
            ..Default::default()
        };
        self.next = JobExtras {
            temp: Some(path.clone()),
            page: Some(Page::Format),
            danger: Some(format!("Everything on the disk will be erased and replaced by {what}.")),
            describe: Some(b.describe),
            success: Some(b.success),
            ..Default::default()
        };
        self.request(Action::Write(WriteOpts {
            input: path,
            format: b.gw_format.into(),
            tracks,
            ..Default::default()
        }));
    }

    fn save_blank_image(&mut self) {
        let b = match self.blank() {
            Ok(b) => b,
            Err(e) => return self.notice = Some(e),
        };
        let mut d = rfd::FileDialog::new()
            .set_file_name(format!("{}.{}", b.file_stem, b.extension))
            .add_filter("Disk image", &[b.extension]);
        if let Some(dir) = &self.s.last_dir {
            d = d.set_directory(dir);
        }
        let Some(p) = d.save_file() else { return };
        self.s.last_dir = p.parent().map(|d| d.to_path_buf());
        match std::fs::write(&p, &b.image) {
            Ok(()) => {
                self.outcome = Some(Outcome {
                    page: Page::Format,
                    level: Level::Good,
                    message: format!("Blank image saved to {}.", p.display()),
                    hint: None,
                })
            }
            Err(e) => self.notice = Some(format!("Cannot save the image: {e}")),
        }
    }
}
