//! "Drive health" page: a guided sequence of tests with a verdict.

use crate::app::{App, DeviceState};
use crate::theme;
use crate::widgets::{self, card};
use eframe::egui::{self, RichText, Ui};
use egui_phosphor::regular as ph;
use hgf_core::formats;
use hgf_core::gw::{Action, ReadOpts, ReadProfile, TrackOpts, WriteOpts};
use hgf_core::health::{self, Finding, ScanPattern, ScanStats, Status};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Check {
    Device,
    Rpm,
    Bandwidth,
    Reference,
    OffsetPlus,
    OffsetMinus,
    Write,
    Readback,
}

impl Check {
    const ALL: [Check; 8] = [
        Check::Device,
        Check::Rpm,
        Check::Bandwidth,
        Check::Reference,
        Check::OffsetPlus,
        Check::OffsetMinus,
        Check::Write,
        Check::Readback,
    ];

    fn title(self) -> &'static str {
        match self {
            Check::Device => "Greaseweazle connected",
            Check::Rpm => "Motor speed & index",
            Check::Bandwidth => "USB bandwidth",
            Check::Reference => "Reading the reference disk",
            Check::OffsetPlus => "Head offset: one track in",
            Check::OffsetMinus => "Head offset: one track out",
            Check::Write => "Writing a scratch disk",
            Check::Readback => "Reading back the scratch disk",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Stage {
    #[default]
    Idle,
    Running(Check),
    WaitScratch,
    Done,
    Aborted,
}

#[derive(Default)]
pub struct HealthWizard {
    pub stage: Stage,
    /// Step whose gw job is running (set just before the job starts).
    pub pending: Option<Check>,
    findings: Vec<(Check, Finding)>,
    results: health::Results,
    plus: Option<ScanStats>,
    verdict: Option<health::Verdict>,
    abort_reason: Option<String>,
    format: String,
    write_enabled: bool,
    temp_image: Option<PathBuf>,
    /// Map of the reference read, shown once the check is over.
    reference_map: Option<hgf_core::DiskMap>,
}

impl HealthWizard {
    fn finding(&self, c: Check) -> Option<&Finding> {
        self.findings.iter().find(|(k, _)| *k == c).map(|(_, f)| f)
    }
}

fn scan_opts(format: &str, custom_tracks: &str) -> ReadOpts {
    ReadOpts {
        output: PathBuf::from("scan.scp"),
        format: format.to_string(),
        profile: ReadProfile::QuickScan,
        tracks: TrackOpts { custom: custom_tracks.to_string(), ..Default::default() },
        ..Default::default()
    }
}

/// Blank image used for the write test (zeros: every sector is rewritten and verified).
fn blank_image(format: &str) -> Option<PathBuf> {
    let size = *formats::expected_size(format)?.first()?;
    let ext = match format.split('.').next() {
        Some("amiga") => "adf",
        Some("atarist") => "st",
        _ => "img",
    };
    let path = std::env::temp_dir().join(format!("hgf-health-blank-{}.{ext}", std::process::id()));
    std::fs::write(&path, vec![0u8; size as usize]).ok()?;
    Some(path)
}

impl App {
    pub fn health_start(&mut self) {
        self.health = HealthWizard {
            format: self.s.health_format.clone(),
            write_enabled: self.s.health_write && formats::expected_size(&self.s.health_format).is_some(),
            ..Default::default()
        };
        self.health_run(Check::Device);
    }

    fn health_run(&mut self, check: Check) {
        let fmt = self.health.format.clone();
        let action = match check {
            Check::Device => Action::Info { bootloader: false },
            Check::Rpm => Action::Rpm { nr: 10 },
            Check::Bandwidth => Action::Bandwidth,
            Check::Reference | Check::Readback => Action::Read(scan_opts(&fmt, "")),
            Check::OffsetPlus => Action::Read(scan_opts(&fmt, "c=2-10:h0.off=+1:h1.off=+1")),
            Check::OffsetMinus => Action::Read(scan_opts(&fmt, "c=2-10:h0.off=-1:h1.off=-1")),
            Check::Write => match blank_image(&fmt) {
                Some(p) => {
                    self.health.temp_image = Some(p.clone());
                    Action::Write(WriteOpts { input: p, format: fmt.clone(), ..Default::default() })
                }
                None => return self.health_abort("Cannot create the blank test image."),
            },
        };
        self.health.stage = Stage::Running(check);
        self.health.pending = Some(check);
        self.request(action);
        // Not started and not waiting for a confirmation: validation or spawn failed.
        if !self.busy() && self.confirm.is_none() {
            self.health.pending = None;
            let reason = self.notice.clone().unwrap_or_else(|| "The test could not start.".into());
            self.health_abort(&reason);
        }
    }

    pub fn health_abort(&mut self, reason: &str) {
        if matches!(self.health.stage, Stage::Running(_) | Stage::WaitScratch) {
            self.health.pending = None;
            self.health.stage = Stage::Aborted;
            self.health.abort_reason = Some(reason.to_string());
            self.health_cleanup();
        }
    }

    pub(crate) fn health_cleanup(&mut self) {
        if let Some(p) = self.health.temp_image.take() {
            let _ = std::fs::remove_file(p);
        }
    }

    fn health_push(&mut self, c: Check, f: Finding) {
        self.health.findings.retain(|(k, _)| *k != c);
        self.health.findings.push((c, f));
    }

    fn health_finish(&mut self) {
        self.health.verdict = Some(health::verdict(&self.health.results));
        self.health.stage = Stage::Done;
        self.health_cleanup();
    }

    /// Called when the job of a wizard step exits.
    pub fn health_job_finished(
        &mut self,
        check: Check,
        success: bool,
        cancelled: bool,
        failure: Option<String>,
    ) {
        if cancelled {
            self.health.stage = Stage::Running(check);
            return self.health_abort("Stopped by user.");
        }
        match check {
            Check::Device => {
                let ok = self.device.state == DeviceState::Connected;
                self.health.results.device = Some(ok);
                let model = self
                    .device
                    .fields
                    .iter()
                    .find(|(k, _)| k == "Model")
                    .map(|(_, v)| v.clone())
                    .unwrap_or_else(|| "Greaseweazle".into());
                if ok {
                    self.health_push(check, Finding { status: Status::Pass, detail: model });
                    self.health_run(Check::Rpm);
                } else {
                    self.health_push(
                        check,
                        Finding { status: Status::Fail, detail: "No device found.".into() },
                    );
                    self.health_finish();
                }
            }
            Check::Rpm => {
                let f = health::rpm(&self.rpm.samples);
                self.health.results.rpm = Some(f.clone());
                let stop = self.rpm.samples.is_empty();
                self.health_push(check, f);
                if stop {
                    self.health_finish()
                } else {
                    self.health_run(Check::Bandwidth)
                }
            }
            Check::Bandwidth => {
                let f = health::bandwidth(&self.bandwidth, &self.bandwidth_notes);
                self.health.results.bandwidth = Some(f.clone());
                self.health_push(check, f);
                self.health_run(Check::Reference);
            }
            Check::Reference | Check::OffsetPlus | Check::OffsetMinus | Check::Readback if !success => {
                self.health_push(
                    check,
                    Finding { status: Status::Fail, detail: failure.unwrap_or_else(|| "gw failed.".into()) },
                );
                self.health_abort("A read failed before completing (see the console).");
            }
            Check::Reference => {
                self.health.reference_map = Some(self.map.clone());
                let stats = ScanStats::from_map(&self.map);
                let (f, pattern) = health::reference(&stats);
                self.health.results.reference = Some((f.clone(), pattern));
                self.health_push(check, f);
                match pattern {
                    ScanPattern::NothingDecoded | ScanPattern::Poor => self.health_run(Check::OffsetPlus),
                    _ => self.health_after_reads(),
                }
            }
            Check::OffsetPlus => {
                self.health.plus = Some(ScanStats::from_map(&self.map));
                self.health_run(Check::OffsetMinus);
            }
            Check::OffsetMinus => {
                let minus = ScanStats::from_map(&self.map);
                let plus = self.health.plus.unwrap_or_default();
                let (f, dir) = health::offset(&plus, &minus);
                self.health_push(Check::OffsetPlus, f.clone());
                self.health_push(check, f.clone());
                self.health.results.offset = Some((f, dir));
                if dir.is_some() {
                    self.health_finish()
                } else {
                    self.health_after_reads()
                }
            }
            Check::Write => {
                if success {
                    self.health_run(Check::Readback);
                } else {
                    let f = health::write_test(false, failure.as_deref(), None);
                    self.health.results.write = Some(f.clone());
                    self.health_push(check, f);
                    self.health_finish();
                }
            }
            Check::Readback => {
                let stats = ScanStats::from_map(&self.map);
                let f = health::write_test(true, None, Some(&stats));
                self.health.results.write = Some(f.clone());
                self.health_push(
                    Check::Write,
                    Finding { status: Status::Pass, detail: "Written and verified by gw.".into() },
                );
                self.health_push(check, f);
                self.health_finish();
            }
        }
    }

    fn health_after_reads(&mut self) {
        let head_ok = !matches!(&self.health.results.reference, Some((_, ScanPattern::SideFailing(_))));
        if self.health.write_enabled && head_ok {
            self.health.stage = Stage::WaitScratch;
        } else {
            self.health_finish();
        }
    }

    fn health_report(&self) -> String {
        let h = &self.health;
        let mut s = format!(
            "HappyGreaseFloppy {} — drive health check\nDrive: {}   Reference format: {}\n\n",
            hgf_core::VERSION,
            self.s.drive.label(),
            h.format
        );
        for c in Check::ALL {
            if let Some(f) = h.finding(c) {
                let tag = match f.status {
                    Status::Pass => "PASS",
                    Status::Warn => "WARN",
                    Status::Fail => "FAIL",
                    Status::Skipped => "SKIP",
                };
                s += &format!("[{tag}] {}: {}\n", c.title(), f.detail);
            }
        }
        if let Some(v) = &h.verdict {
            s += &format!("\nVerdict: {}\n", v.headline);
            for a in &v.advice {
                s += &format!(" - {a}\n");
            }
        }
        s
    }

    pub fn page_health(&mut self, ui: &mut Ui) {
        let c = theme::DRIVE;
        let busy = self.busy() || self.detecting;
        let running = matches!(self.health.stage, Stage::Running(_) | Stage::WaitScratch);
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(theme::w(430.0));
                egui::ScrollArea::vertical().id_salt("health_form").auto_shrink([false, false]).show(
                    ui,
                    |ui| {
                        card(ui, ph::STETHOSCOPE, "Guided drive check", c, |ui| {
                            ui.label(
                                RichText::new(
                                    "Checks the Greaseweazle, the drive motor, the USB link, then reads a \
                                 reference disk. If it does not read, head offsets are tested. \
                                 Optionally writes and reads back a scratch disk.",
                                )
                                .small()
                                .color(theme::MUTED),
                            );
                            ui.add_space(4.0);
                            ui.label(RichText::new("Reference disk: a disk you know is good").strong());
                            ui.add_enabled_ui(!running, |ui| {
                                if let Some(p) = widgets::preset_chips(ui, &self.s.health_format) {
                                    if !p.format.is_empty() {
                                        self.s.health_format = p.format.to_string();
                                    }
                                }
                                let mut fmt = self.s.health_format.clone();
                                if widgets::format_picker(
                                    ui,
                                    "health_fmt",
                                    &mut fmt,
                                    &self.catalog,
                                    "choose a format",
                                ) && !fmt.is_empty()
                                {
                                    self.s.health_format = fmt;
                                }
                                let can_write = formats::expected_size(&self.s.health_format).is_some();
                                ui.add_enabled(
                                    can_write,
                                    egui::Checkbox::new(
                                        &mut self.s.health_write,
                                        "Also test writing (scratch disk, erased)",
                                    ),
                                )
                                .on_disabled_hover_text(
                                    "Write test available for Amiga, PC and Atari ST formats",
                                );
                            });
                        });
                        ui.add_space(8.0);
                        if widgets::action_button(ui, ph::PLAY, "START CHECK", c, !busy && !running) {
                            self.health_start();
                        }
                        ui.add_space(10.0);
                        self.health_checklist(ui);
                    },
                );
            });
            ui.separator();
            ui.vertical(|ui| {
                self.health_panel(ui);
            });
        });
    }

    fn health_checklist(&mut self, ui: &mut Ui) {
        let dark = ui.visuals().dark_mode;
        let stage = self.health.stage;
        card(ui, ph::CLIPBOARD_TEXT, "Checks", theme::DRIVE, |ui| {
            for check in Check::ALL {
                let finding = self.health.finding(check).cloned();
                let is_running = stage == Stage::Running(check);
                let optional =
                    matches!(check, Check::OffsetPlus | Check::OffsetMinus | Check::Write | Check::Readback);
                ui.horizontal_top(|ui| {
                    if is_running {
                        ui.add(egui::Spinner::new().size(16.0).color(theme::ACTIVE));
                    } else {
                        let (icon, col) = match finding.as_ref().map(|f| f.status) {
                            Some(Status::Pass) => (ph::CHECK_CIRCLE, theme::OK),
                            Some(Status::Warn) => (ph::WARNING, theme::WARN),
                            Some(Status::Fail) => (ph::X_CIRCLE, theme::BAD),
                            Some(Status::Skipped) | None
                                if stage == Stage::Done || stage == Stage::Aborted =>
                            {
                                (ph::MINUS_CIRCLE, theme::MUTED)
                            }
                            _ => (ph::CIRCLE_DASHED, theme::MUTED),
                        };
                        ui.label(RichText::new(icon).size(theme::fs(17.0)).color(theme::readable(col, dark)));
                    }
                    ui.vertical(|ui| {
                        ui.label(RichText::new(check.title()).strong());
                        let detail = match (&finding, is_running, stage) {
                            (Some(f), _, _) => Some(f.detail.clone()),
                            (None, true, _) => Some("Running…".into()),
                            (None, false, Stage::Done | Stage::Aborted)
                                if matches!(check, Check::Write | Check::Readback)
                                    && !self.health.write_enabled =>
                            {
                                Some("Not requested.".into())
                            }
                            (None, false, Stage::Done | Stage::Aborted) if optional => {
                                Some("Not needed.".into())
                            }
                            _ => None,
                        };
                        if let Some(d) = detail {
                            ui.add(egui::Label::new(RichText::new(d).small().color(theme::MUTED)).wrap());
                        }
                    });
                });
                ui.add_space(2.0);
            }
        });
    }

    fn health_panel(&mut self, ui: &mut Ui) {
        let dark = ui.visuals().dark_mode;
        match self.health.stage {
            Stage::WaitScratch => {
                card(ui, ph::FLOPPY_DISK, "Insert a scratch disk", theme::WRITE, |ui| {
                    ui.label(
                        "Remove the reference disk and insert a disk whose content can be destroyed. \
                         A blank image will be written, verified by gw, then read back.",
                    );
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.button(format!("{}  Continue", ph::ARROW_RIGHT)).clicked() {
                            self.health_run(Check::Write);
                        }
                        if ui.button("Skip the write test").clicked() {
                            self.health_finish();
                        }
                    });
                });
                ui.add_space(8.0);
            }
            Stage::Done => {
                if let Some(v) = self.health.verdict.clone() {
                    let col = match v.status {
                        Status::Pass => theme::OK,
                        Status::Warn => theme::WARN,
                        _ => theme::BAD,
                    };
                    let icon = match v.status {
                        Status::Pass => ph::CHECK_CIRCLE,
                        Status::Warn => ph::WARNING,
                        _ => ph::X_CIRCLE,
                    };
                    egui::Frame::new()
                        .fill(theme::tint(col, 30))
                        .stroke(egui::Stroke::new(1.5, theme::tint(col, 170)))
                        .corner_radius(10)
                        .inner_margin(14)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(icon)
                                        .size(theme::fs(34.0))
                                        .color(theme::readable(col, dark)),
                                );
                                ui.label(
                                    RichText::new(&v.headline)
                                        .size(theme::fs(22.0))
                                        .strong()
                                        .color(theme::readable(col, dark)),
                                );
                            });
                            ui.add_space(6.0);
                            for a in &v.advice {
                                ui.horizontal_top(|ui| {
                                    ui.label(RichText::new(ph::ARROW_RIGHT).color(theme::MUTED));
                                    ui.add(egui::Label::new(a.as_str()).wrap());
                                });
                            }
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                if ui.button(format!("{}  Copy report", ph::COPY)).clicked() {
                                    ui.ctx().copy_text(self.health_report());
                                }
                            });
                        });
                    ui.add_space(8.0);
                }
            }
            Stage::Aborted => {
                let reason = self.health.abort_reason.clone().unwrap_or_default();
                widgets::banner(ui, ph::WARNING, &format!("Check interrupted: {reason}"), theme::WARN);
                ui.add_space(8.0);
            }
            _ => {}
        }
        match self.health.stage {
            Stage::Running(
                Check::Reference | Check::OffsetPlus | Check::OffsetMinus | Check::Readback | Check::Write,
            ) => self.visualizer(ui),
            Stage::Done | Stage::Aborted | Stage::WaitScratch => match &self.health.reference_map {
                Some(m) => {
                    ui.label(RichText::new("Reference disk read:").strong());
                    crate::visualizer::show(ui, m, false);
                }
                None => self.visualizer(ui),
            },
            Stage::Running(Check::Rpm) => {
                ui.vertical_centered(|ui| {
                    ui.add_space(30.0);
                    let txt =
                        self.rpm.samples.last().map(|r| format!("{r:.2} rpm")).unwrap_or_else(|| "…".into());
                    ui.label(RichText::new(txt).size(theme::fs(48.0)).strong().color(theme::ACCENT));
                });
            }
            _ => {
                ui.add_space(30.0);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new(ph::STETHOSCOPE).size(theme::fs(64.0)).color(theme::MUTED));
                    ui.label(
                        RichText::new("Insert a known-good disk, choose its format, then press START CHECK.")
                            .color(theme::MUTED),
                    );
                });
            }
        }
    }
}
