//! "Copy disk" page: read the source into a temporary image, then write it to the destination,
//! with one drive (disk swap) or two.

use crate::app::{App, JobExtras, Page};
use crate::theme;
use crate::widgets::{self, card, PickMode};
use eframe::egui::{self, RichText, Ui};
use egui_phosphor::regular as ph;
use hgf_core::formats::{self, PRESETS};
use hgf_core::gw::{Action, Drive, ReadOpts, ReadProfile, WriteOpts};
use hgf_core::hints::Hint;
use hgf_core::DiskMap;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CopySettings {
    pub format: String,
    /// Copy the raw flux (keeps copy protections) instead of the decoded sectors.
    pub flux: bool,
    pub source: Drive,
    pub destination: Drive,
    /// Insist on weak sectors (Recovery profile) while reading the source.
    pub recovery: bool,
    /// Optional path where the read image is also kept.
    #[serde(skip)]
    pub keep: PathBuf,
}

impl Default for CopySettings {
    fn default() -> Self {
        Self {
            format: "amiga.amigados".into(),
            flux: false,
            source: Drive::A,
            destination: Drive::A,
            recovery: false,
            keep: PathBuf::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Read,
    Write,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum Stage {
    #[default]
    Idle,
    Reading,
    /// Read finished with unreadable tracks: copy anyway?
    Review,
    WaitDestination,
    Writing,
    Done(bool),
    Aborted(String),
}

#[derive(Default)]
pub struct CopyWizard {
    pub stage: Stage,
    pub pending: Option<Step>,
    opts: CopySettings,
    temp: Option<PathBuf>,
    bad: usize,
    read_map: Option<DiskMap>,
    message: Option<String>,
    hint: Option<Hint>,
    kept: Option<String>,
}

impl CopyWizard {
    fn active(&self) -> bool {
        matches!(self.stage, Stage::Reading | Stage::Review | Stage::WaitDestination | Stage::Writing)
    }
}

/// Image type used for a sector copy of `format`.
fn sector_ext(format: &str) -> &'static str {
    if let Some(p) = PRESETS.iter().find(|p| p.format == format && !p.format.is_empty()) {
        return p.extension;
    }
    match format.split('.').next() {
        Some("amiga") => "adf",
        Some("atarist") => "st",
        Some("ibm") => "img",
        // HFE stores the bitcells of any format and needs no format to be written back.
        _ => "hfe",
    }
}

impl App {
    pub(crate) fn copy_cleanup(&mut self) {
        if let Some(t) = self.copy.temp.take() {
            let _ = std::fs::remove_file(t);
        }
    }

    pub fn copy_abort(&mut self, reason: &str) {
        if self.copy.active() {
            self.copy.pending = None;
            self.copy.stage = Stage::Aborted(reason.to_string());
            self.copy_cleanup();
        }
    }

    fn copy_start(&mut self) {
        let o = self.s.copy.clone();
        if o.format.is_empty() && !o.flux {
            return self.notice = Some("Choose the disk format, or tick “Flux copy”.".into());
        }
        self.copy_cleanup();
        let ext = if o.flux { "scp" } else { sector_ext(&o.format) };
        let temp = std::env::temp_dir().join(format!("hgf-copy-{}.{ext}", std::process::id()));
        let _ = std::fs::remove_file(&temp);
        self.copy = CopyWizard {
            opts: o.clone(),
            temp: Some(temp.clone()),
            stage: Stage::Reading,
            ..Default::default()
        };
        let read = ReadOpts {
            output: temp,
            format: o.format.clone(),
            profile: if o.recovery { ReadProfile::Recovery } else { ReadProfile::Standard },
            ..Default::default()
        };
        self.copy.pending = Some(Step::Read);
        self.next = JobExtras { drive: Some(o.source), page: Some(Page::Copy), ..Default::default() };
        self.request(Action::Read(read));
        if !self.busy() && self.confirm.is_none() {
            let reason = self.notice.clone().unwrap_or_else(|| "The read could not start.".into());
            self.copy.pending = None;
            self.copy.stage = Stage::Reading;
            self.copy_abort(&reason);
        }
    }

    fn copy_after_read(&mut self) {
        if self.copy.opts.source == self.copy.opts.destination {
            self.copy.stage = Stage::WaitDestination;
        } else {
            self.copy_write();
        }
    }

    fn copy_write(&mut self) {
        let Some(temp) = self.copy.temp.clone() else {
            return self.copy_abort("The source image is missing.");
        };
        let o = self.copy.opts.clone();
        self.copy.stage = Stage::Writing;
        self.copy.pending = Some(Step::Write);
        self.next = JobExtras {
            drive: Some(o.destination),
            page: Some(Page::Copy),
            danger: Some(format!("The disk in {} will be overwritten with the copy.", o.destination.label())),
            describe: Some(format!("copy of the source disk ({})", if o.flux { "flux" } else { &o.format })),
            ..Default::default()
        };
        let format = if o.flux { String::new() } else { o.format };
        self.request(Action::Write(WriteOpts { input: temp, format, ..Default::default() }));
        if !self.busy() && self.confirm.is_none() {
            let reason = self.notice.clone().unwrap_or_else(|| "The write could not start.".into());
            self.copy.pending = None;
            self.copy_abort(&reason);
        }
    }

    pub fn copy_job_finished(
        &mut self,
        step: Step,
        success: bool,
        cancelled: bool,
        failure: Option<String>,
        hint: Option<Hint>,
    ) {
        if cancelled {
            return self.copy_abort("Stopped by user.");
        }
        match step {
            Step::Read if !success => {
                self.copy.stage = Stage::Done(false);
                self.copy.message = Some(format!(
                    "The source disk could not be read: {}",
                    failure.unwrap_or_else(|| "gw failed.".into())
                ));
                self.copy.hint = hint;
                self.copy_cleanup();
            }
            Step::Read => {
                self.copy.read_map = Some(self.map.clone());
                self.copy.bad = self.map.bad_tracks().len();
                let keep = self.copy.opts.keep.clone();
                if !keep.as_os_str().is_empty() {
                    if let Some(t) = &self.copy.temp {
                        self.copy.kept = Some(match std::fs::copy(t, &keep) {
                            Ok(_) => format!("Image kept in {}.", keep.display()),
                            Err(e) => format!("Could not keep the image: {e}"),
                        });
                    }
                }
                if self.copy.bad > 0 {
                    self.copy.stage = Stage::Review;
                } else {
                    self.copy_after_read();
                }
            }
            Step::Write => {
                self.copy.stage = Stage::Done(success);
                self.copy.message = Some(if success {
                    "Copy complete: every track was written and verified.".into()
                } else {
                    format!("Writing the copy failed: {}", failure.unwrap_or_else(|| "gw failed.".into()))
                });
                self.copy.hint = if success { None } else { hint };
                self.copy_cleanup();
            }
        }
    }

    pub fn page_copy(&mut self, ui: &mut Ui) {
        let c = theme::COPY;
        let busy = self.busy() || self.detecting;
        let active = self.copy.active();
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(theme::w(420.0));
                egui::ScrollArea::vertical().id_salt("copy_form").auto_shrink([false, false]).show(
                    ui,
                    |ui| {
                        ui.add_enabled_ui(!active, |ui| self.copy_form(ui, c));
                        ui.add_space(10.0);
                        if widgets::action_button(ui, ph::COPY, "START COPY", c, !busy && !active) {
                            self.copy_start();
                        }
                        ui.add_space(10.0);
                        self.copy_steps(ui);
                    },
                );
            });
            ui.separator();
            ui.vertical(|ui| self.copy_panel(ui));
        });
    }

    fn copy_form(&mut self, ui: &mut Ui, c: egui::Color32) {
        let o = &mut self.s.copy;
        card(ui, ph::HARD_DRIVES, "Drives", c, |ui| {
            egui::Grid::new("copy_drives").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                for (label, d) in [("Source", &mut o.source), ("Destination", &mut o.destination)] {
                    ui.label(label);
                    egui::ComboBox::from_id_salt(label).selected_text(d.label()).show_ui(ui, |ui| {
                        for x in Drive::ALL {
                            ui.selectable_value(d, x, x.label());
                        }
                    });
                    ui.end_row();
                }
            });
            let note = if o.source == o.destination {
                "One drive: you will be asked to swap the disks after the read."
            } else {
                "Two drives: the copy runs without swapping disks."
            };
            ui.label(RichText::new(note).small().color(theme::MUTED));
        });
        ui.add_space(8.0);
        card(ui, ph::DISC, "Disk format", c, |ui| {
            if let Some(p) = widgets::preset_chips(ui, &o.format) {
                if p.format.is_empty() {
                    o.flux = true;
                } else {
                    o.format = p.format.to_string();
                }
            }
            ui.add_space(4.0);
            widgets::format_picker(ui, "copy_fmt", &mut o.format, &self.catalog, "None (flux copy only)");
            ui.checkbox(&mut o.flux, "Flux copy (keeps copy protections)").on_hover_text(
                "Copies the raw magnetic signal (SCP). The format above is then only used to check the read.",
            );
            ui.checkbox(&mut o.recovery, "Insist on weak sectors (Recovery profile)");
        });
        ui.add_space(8.0);
        card(ui, ph::FLOPPY_DISK_BACK, "Keep the image (optional)", c, |ui| {
            let ext = if o.flux { "scp" } else { sector_ext(&o.format) };
            widgets::path_row(
                ui,
                &mut o.keep,
                PickMode::Save,
                "not kept",
                &format!("copy.{ext}"),
                &mut self.s.last_dir,
            );
            if !o.keep.as_os_str().is_empty() && formats::extension(&o.keep).as_deref() != Some(ext) {
                ui.label(
                    RichText::new(format!("The image will be a .{ext} file.")).small().color(theme::WARN),
                );
            }
        });
    }

    fn copy_steps(&self, ui: &mut Ui) {
        let dark = ui.visuals().dark_mode;
        let st = &self.copy.stage;
        let one_drive = self.s.copy.source == self.s.copy.destination;
        let mut rows: Vec<(&str, u8)> = vec![("Read the source disk", 0)];
        if one_drive {
            rows.push(("Swap the disks", 1));
        }
        rows.push(("Write and verify the destination", 2));
        let reached = |n: u8| -> (bool, bool) {
            // (done, current)
            match st {
                Stage::Reading => (false, n == 0),
                Stage::Review => (n == 0, false),
                Stage::WaitDestination => (n == 0, n == 1),
                Stage::Writing => (n < 2, n == 2),
                Stage::Done(true) => (true, false),
                _ => (false, false),
            }
        };
        card(ui, ph::CLIPBOARD_TEXT, "Steps", theme::COPY, |ui| {
            for (label, n) in rows {
                let (done, current) = reached(n);
                ui.horizontal(|ui| {
                    if current && self.busy() {
                        ui.add(egui::Spinner::new().size(16.0).color(theme::ACTIVE));
                    } else {
                        let (icon, col) = if done {
                            (ph::CHECK_CIRCLE, theme::OK)
                        } else if current {
                            (ph::ARROW_RIGHT, theme::ACTIVE)
                        } else {
                            (ph::CIRCLE_DASHED, theme::MUTED)
                        };
                        ui.label(RichText::new(icon).size(theme::fs(17.0)).color(theme::readable(col, dark)));
                    }
                    ui.label(RichText::new(label).strong());
                });
            }
        });
    }

    fn copy_panel(&mut self, ui: &mut Ui) {
        let dark = ui.visuals().dark_mode;
        match self.copy.stage.clone() {
            Stage::Review => {
                card(ui, ph::WARNING, "Some tracks could not be read", theme::WARN, |ui| {
                    ui.label(format!(
                        "{} track(s) of the source disk are unreadable, even after retries. The copy \
                         would contain empty sectors there.",
                        self.copy.bad
                    ));
                    if self.copy.opts.flux {
                        ui.label(
                            RichText::new("With a flux copy, undecodable tracks can be a copy protection: the copy may still work.")
                                .small()
                                .color(theme::MUTED),
                        );
                    }
                    ui.horizontal(|ui| {
                        if ui.button(format!("{}  Copy anyway", ph::ARROW_RIGHT)).clicked() {
                            self.copy_after_read();
                        }
                        if ui.button(format!("{}  Cancel", ph::X)).clicked() {
                            self.copy_abort("Cancelled after the read.");
                        }
                    });
                });
                ui.add_space(8.0);
            }
            Stage::WaitDestination => {
                card(ui, ph::FLOPPY_DISK, "Insert the destination disk", theme::WRITE, |ui| {
                    ui.label("The source disk has been read. Remove it and insert the disk to write.");
                    ui.horizontal(|ui| {
                        if ui.button(format!("{}  Continue", ph::ARROW_RIGHT)).clicked() {
                            self.copy_write();
                        }
                        if ui.button(format!("{}  Cancel", ph::X)).clicked() {
                            self.copy_abort("Cancelled before writing.");
                        }
                    });
                });
                ui.add_space(8.0);
            }
            Stage::Done(ok) => {
                let col = if ok { theme::OK } else { theme::BAD };
                let icon = if ok { ph::CHECK_CIRCLE } else { ph::X_CIRCLE };
                let msg = self.copy.message.clone().unwrap_or_default();
                let hint = self.copy.hint.clone();
                egui::Frame::new()
                    .fill(theme::tint(col, 30))
                    .stroke(egui::Stroke::new(1.5, theme::tint(col, 170)))
                    .corner_radius(10)
                    .inner_margin(12)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(icon).size(theme::fs(28.0)).color(theme::readable(col, dark)),
                            );
                            ui.add(
                                egui::Label::new(RichText::new(msg).size(theme::fs(16.0)).strong()).wrap(),
                            );
                        });
                        if ok && self.copy.bad > 0 {
                            ui.label(format!(
                                "{} unreadable source track(s) were copied as empty sectors.",
                                self.copy.bad
                            ));
                        }
                        if let Some(k) = &self.copy.kept {
                            ui.label(RichText::new(k).small().color(theme::MUTED));
                        }
                        if let Some(h) = &hint {
                            ui.separator();
                            if let Some(a) = widgets::hint_box(ui, h, !self.busy()) {
                                self.run_hint_action(a);
                            }
                        }
                    });
                ui.add_space(8.0);
            }
            Stage::Aborted(reason) => {
                widgets::banner(ui, ph::WARNING, &format!("Copy interrupted: {reason}"), theme::WARN);
                ui.add_space(8.0);
            }
            _ => {}
        }
        match (&self.copy.stage, &self.copy.read_map) {
            (Stage::Reading | Stage::Writing, _) | (_, None) => self.visualizer(ui),
            (_, Some(m)) => {
                ui.label(RichText::new("Source disk:").strong());
                crate::visualizer::show(ui, m, false);
            }
        }
    }
}
