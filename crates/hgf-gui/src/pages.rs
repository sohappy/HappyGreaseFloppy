//! One page per Greaseweazle command.

use crate::app::{App, DeviceState, Page};
use crate::theme;
use crate::widgets::{self, card, PickMode};
use eframe::egui::{self, RichText, Sense, Shape, Stroke, Ui};
use egui_phosphor::regular as ph;
use hgf_core::formats;
use hgf_core::gw::*;

const FORM_WIDTH: f32 = 400.0;

impl App {
    pub fn page(&mut self, ui: &mut Ui) {
        match self.s.page {
            Page::Read => self.page_read(ui),
            Page::Write => self.page_write(ui),
            Page::Copy => self.page_copy(ui),
            Page::Format => self.page_format(ui),
            Page::Convert => self.page_convert(ui),
            Page::Erase => self.page_erase(ui),
            Page::Health => self.page_health(ui),
            Page::Rpm => self.page_rpm(ui),
            Page::Align => self.page_align(ui),
            Page::Seek => narrow(ui, theme::w(560.0), |ui| self.page_seek(ui)),
            Page::Clean => narrow(ui, theme::w(560.0), |ui| self.page_clean(ui)),
            Page::Info => narrow(ui, theme::w(680.0), |ui| self.page_info(ui)),
            Page::Delays => self.page_delays(ui),
            Page::Pins => narrow(ui, theme::w(560.0), |ui| self.page_pins(ui)),
            Page::Bandwidth => narrow(ui, theme::w(680.0), |ui| self.page_bandwidth(ui)),
            Page::Reset => narrow(ui, theme::w(560.0), |ui| self.page_reset(ui)),
            Page::Update => narrow(ui, theme::w(680.0), |ui| self.page_update(ui)),
            Page::Inspector => narrow(ui, theme::w(780.0), |ui| self.page_inspector(ui)),
            Page::Settings => narrow(ui, theme::w(720.0), |ui| self.page_settings(ui)),
        }
    }

    /// Form on the left, disk visualizer on the right.
    fn split(&mut self, ui: &mut Ui, form: impl FnOnce(&mut Self, &mut Ui)) {
        let page = self.s.page;
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(theme::w(FORM_WIDTH));
                egui::ScrollArea::vertical()
                    .id_salt(("form", page))
                    .auto_shrink([false, false])
                    .max_height(ui.available_height())
                    .show(ui, |ui| form(self, ui));
            });
            ui.separator();
            ui.vertical(|ui| self.visualizer(ui));
        });
    }

    /// The Read action as it will run (disk number applied to the destination).
    fn read_action(&self) -> Action {
        let mut o = self.s.read.clone();
        if o.profile != ReadProfile::QuickScan {
            o.output = self.s.read_naming.apply(&o.output);
        }
        Action::Read(o)
    }

    fn page_read(&mut self, ui: &mut Ui) {
        self.split(ui, |app, ui| {
            let busy = app.busy();
            let c = theme::READ;
            app.templates_card(ui, crate::templates::TemplateKind::Read);
            let mut folder_error = None;
            let o = &mut app.s.read;
            card(ui, ph::DISC, "Disk format", c, |ui| {
                if let Some(p) = widgets::preset_chips(ui, &o.format) {
                    o.format = p.format.to_string();
                    widgets::set_extension(&mut o.output, p.extension);
                }
                ui.add_space(4.0);
                widgets::format_picker(ui, "read_fmt", &mut o.format, &app.catalog, "None — raw flux only");
            });
            ui.add_space(8.0);
            card(ui, ph::FILE_ARROW_DOWN, "Output image", c, |ui| {
                let quick = o.profile == ReadProfile::QuickScan;
                ui.add_enabled_ui(!quick, |ui| {
                    widgets::path_row(
                        ui,
                        &mut o.output,
                        PickMode::Save,
                        "disk.adf",
                        "disk.adf",
                        &mut app.s.last_dir,
                    );
                });
                let ext = formats::extension(&o.output).unwrap_or_default();
                let hint = if quick {
                    "Quick scan: nothing is saved.".to_string()
                } else if formats::FLUX_SUFFIXES.contains(&ext.as_str()) {
                    "Flux image: preserves everything, including copy protection.".to_string()
                } else if ext.is_empty() {
                    "The extension selects the image type.".to_string()
                } else {
                    match ext.as_str() {
                        "hfe" => "HFE image for HxC and Gotek (FlashFloppy) emulators.".to_string(),
                        _ => format!(".{ext} sector image."),
                    }
                };
                ui.label(RichText::new(hint).small().color(theme::MUTED));
                if !quick {
                    let n = &mut app.s.read_naming;
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut n.enabled, "Number the images").on_hover_text(
                            "disk_001.adf, disk_002.adf… The number goes up after each successful read.",
                        );
                        ui.add_enabled_ui(n.enabled, |ui| {
                            ui.label("next");
                            ui.add(egui::DragValue::new(&mut n.next).range(0..=999_999));
                            ui.label("digits");
                            ui.add(egui::DragValue::new(&mut n.digits).range(1..=6));
                        });
                    });
                    let target = n.apply(&o.output);
                    ui.horizontal(|ui| {
                        folder_error = widgets::folder_button(ui, &target);
                        let name =
                            target.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
                        let exists = target.exists();
                        ui.label(
                            RichText::new(if exists {
                                format!("{name} exists: you will be asked before it is replaced")
                            } else if n.enabled {
                                format!("Next image: {name}")
                            } else {
                                format!("Saved as {name}")
                            })
                            .small()
                            .color(if exists {
                                theme::WARN
                            } else {
                                theme::MUTED
                            }),
                        );
                    });
                    widgets::image_opts(
                        ui,
                        "read_out",
                        hgf_core::imageopts::for_output(&o.output),
                        &o.output,
                        &mut o.file_opts,
                    );
                }
            });
            ui.add_space(8.0);
            card(ui, ph::FIRST_AID_KIT, "Profile", c, |ui| {
                ui.horizontal(|ui| {
                    for p in [ReadProfile::Standard, ReadProfile::Recovery, ReadProfile::QuickScan] {
                        ui.selectable_value(&mut o.profile, p, p.label());
                    }
                });
                ui.label(RichText::new(o.profile.help()).small().color(theme::MUTED));
            });
            ui.add_space(8.0);
            card(ui, ph::SLIDERS_HORIZONTAL, "Tracks & options", c, |ui| {
                widgets::tracks(ui, &mut o.tracks);
                egui::CollapsingHeader::new("Advanced").id_salt("read_adv").show(ui, |ui| {
                    widgets::opt_u32(
                        ui,
                        "Revolutions",
                        &mut o.revs,
                        3,
                        50,
                        "--revs: revolutions read per track",
                    );
                    widgets::opt_u32(
                        ui,
                        "Retries",
                        &mut o.retries,
                        3,
                        99,
                        "--retries per seek-retry (gw default 3)",
                    );
                    widgets::opt_u32(
                        ui,
                        "Seek retries",
                        &mut o.seek_retries,
                        0,
                        20,
                        "--seek-retries: re-seek the head and try again",
                    );
                    ui.checkbox(&mut o.raw, "Raw output (--raw): format only verifies");
                    widgets::flux(ui, &mut o.flux);
                    widgets::densel(ui, "read_densel", &mut o.densel, &mut o.gen_tg43);
                });
            });
            ui.add_space(10.0);
            let label = match o.profile {
                ReadProfile::QuickScan => "SCAN DISK",
                _ => "READ DISK TO IMAGE",
            };
            widgets::tracks_badge(ui, &mut o.tracks);
            let numbered = app.s.read_naming.enabled && app.s.read.profile != ReadProfile::QuickScan;
            let action = app.read_action();
            if widgets::action_button(ui, ph::DOWNLOAD_SIMPLE, label, c, !busy) {
                app.next.numbered = numbered;
                app.request(action.clone());
            }
            widgets::command_preview(ui, &action.command_line(&app.s.cfg, app.s.drive));
            if folder_error.is_some() {
                app.notice = folder_error;
            }
        });
    }

    fn page_write(&mut self, ui: &mut Ui) {
        self.split(ui, |app, ui| {
            let busy = app.busy();
            let c = theme::WRITE;
            app.templates_card(ui, crate::templates::TemplateKind::Write);
            let mut new_input = None;
            card(ui, ph::FILE_ARROW_UP, "Source image", c, |ui| {
                let mut p = app.s.write.input.clone();
                if widgets::path_row(
                    ui,
                    &mut p,
                    PickMode::Open,
                    "image to write (or drop a file)",
                    "",
                    &mut app.s.last_dir,
                ) {
                    new_input = Some(p);
                }
                let input = &app.s.write.input;
                if let Ok(m) = std::fs::metadata(input) {
                    let guess = formats::guess_format(input, m.len());
                    ui.label(
                        RichText::new(format!(
                            "{}  ·  {}",
                            widgets::human_size(m.len()),
                            guess
                                .map(|g| format!("looks like {g}"))
                                .unwrap_or_else(|| "format not recognised from size".into())
                        ))
                        .small()
                        .color(theme::MUTED),
                    );
                } else if !input.as_os_str().is_empty() {
                    ui.label(
                        RichText::new(format!("{}  File not found", ph::WARNING)).color(theme::BAD).small(),
                    );
                }
                let w = &mut app.s.write;
                widgets::image_opts(
                    ui,
                    "write_in",
                    hgf_core::imageopts::for_input(&w.input),
                    &w.input,
                    &mut w.file_opts,
                );
            });
            if let Some(p) = new_input {
                app.set_write_input(p);
            }
            ui.add_space(8.0);
            app.amiga_check(ui);
            let o = &mut app.s.write;
            ui.add_space(8.0);
            card(ui, ph::DISC, "Disk format", c, |ui| {
                if let Some(p) = widgets::preset_chips(ui, &o.format) {
                    o.format = p.format.to_string();
                }
                ui.add_space(4.0);
                widgets::format_picker(ui, "write_fmt", &mut o.format, &app.catalog, "Auto — from the image");
                ui.label(
                    RichText::new("Required for .img/.st images; flux and self-describing images need none.")
                        .small()
                        .color(theme::MUTED),
                );
            });
            ui.add_space(8.0);
            card(ui, ph::SLIDERS_HORIZONTAL, "Tracks & options", c, |ui| {
                widgets::tracks(ui, &mut o.tracks);
                ui.checkbox(&mut o.pre_erase, "Erase each track before writing");
                ui.checkbox(&mut o.erase_empty, "Erase tracks that are empty in the image");
                ui.checkbox(&mut o.no_verify, "Skip verification (not recommended)");
                egui::CollapsingHeader::new("Advanced").id_salt("write_adv").show(ui, |ui| {
                    widgets::opt_u32(
                        ui,
                        "Verify retries",
                        &mut o.retries,
                        3,
                        20,
                        "--retries on verify failure (default 3)",
                    );
                    widgets::text_opt(
                        ui,
                        "Precomp",
                        &mut o.precomp,
                        "e.g. type=mfm:40=125",
                        "--precomp: write precompensation",
                    );
                    if hgf_core::compat::is_amiga_write(o)
                        && o.precomp != hgf_core::compat::AMIGA_PRECOMP
                        && ui
                            .small_button(format!("{}  Amiga: 140 ns from cylinder 40", ph::MAGIC_WAND))
                            .on_hover_text(
                                "The precompensation the Amiga itself uses on DD disks. \
                                 Try it if a disk verifies here but the Amiga cannot read it.",
                            )
                            .clicked()
                    {
                        o.precomp = hgf_core::compat::AMIGA_PRECOMP.into();
                    }
                    widgets::text_opt(
                        ui,
                        "Fake index",
                        &mut o.fake_index,
                        "e.g. 300rpm",
                        "--fake-index SPEED",
                    );
                    ui.checkbox(&mut o.hard_sectors, "Hard-sectored disk");
                    ui.checkbox(&mut o.reverse, "Reverse track data (flippy disk)");
                    widgets::densel(ui, "write_densel", &mut o.densel, &mut o.gen_tg43);
                });
            });
            ui.add_space(10.0);
            widgets::tracks_badge(ui, &mut o.tracks);
            let action = Action::Write(o.clone());
            if widgets::action_button(ui, ph::UPLOAD_SIMPLE, "WRITE IMAGE TO DISK", c, !busy) {
                app.request(action.clone());
            }
            widgets::command_preview(ui, &action.command_line(&app.s.cfg, app.s.drive));
        });
    }

    fn page_convert(&mut self, ui: &mut Ui) {
        self.split(ui, |app, ui| {
            let busy = app.busy();
            let c = theme::CONVERT;
            let o = &mut app.s.convert;
            let mut folder_error = None;
            card(ui, ph::FILE_ARROW_UP, "Input image", c, |ui| {
                widgets::path_row(
                    ui,
                    &mut o.input,
                    PickMode::Open,
                    "source image (flux or sectors)",
                    "",
                    &mut app.s.last_dir,
                );
                widgets::image_opts(
                    ui,
                    "conv_in",
                    hgf_core::imageopts::for_input(&o.input),
                    &o.input,
                    &mut o.in_opts,
                );
            });
            ui.add_space(8.0);
            card(ui, ph::FILE_ARROW_DOWN, "Output image", c, |ui| {
                widgets::path_row(
                    ui,
                    &mut o.output,
                    PickMode::Save,
                    "destination (.adf, .img, .hfe…)",
                    "converted.adf",
                    &mut app.s.last_dir,
                );
                ui.horizontal(|ui| {
                    folder_error = widgets::folder_button(ui, &o.output);
                    let ext = formats::extension(&o.output).unwrap_or_else(|| "adf".into());
                    let can = !o.input.as_os_str().is_empty();
                    if ui
                        .add_enabled(
                            can,
                            egui::Button::new(format!("{}  Next to the source", ph::ARROW_BEND_DOWN_RIGHT))
                                .small(),
                        )
                        .on_hover_text("Same folder and name as the input image, with the output extension")
                        .clicked()
                    {
                        o.output = o.input.with_extension(&ext);
                        if o.output == o.input {
                            let stem = o
                                .input
                                .file_stem()
                                .map(|s| s.to_string_lossy().into_owned())
                                .unwrap_or_default();
                            o.output = o.input.with_file_name(format!("{stem}-converted.{ext}"));
                        }
                    }
                });
                widgets::image_opts(
                    ui,
                    "conv_out",
                    hgf_core::imageopts::for_output(&o.output),
                    &o.output,
                    &mut o.out_opts,
                );
            });
            ui.add_space(8.0);
            card(ui, ph::DISC, "Disk format", c, |ui| {
                if let Some(p) = widgets::preset_chips(ui, &o.format) {
                    o.format = p.format.to_string();
                    widgets::set_extension(&mut o.output, p.extension);
                }
                ui.add_space(4.0);
                widgets::format_picker(ui, "conv_fmt", &mut o.format, &app.catalog, "Auto — from the images");
            });
            ui.add_space(8.0);
            card(ui, ph::SLIDERS_HORIZONTAL, "Tracks & options", c, |ui| {
                widgets::tracks(ui, &mut o.tracks);
                widgets::text_opt(
                    ui,
                    "Output tracks",
                    &mut o.out_tracks,
                    "same as input",
                    "--out-tracks TSPEC",
                );
                egui::CollapsingHeader::new("Advanced").id_salt("conv_adv").show(ui, |ui| {
                    widgets::text_opt(
                        ui,
                        "Adjust speed",
                        &mut o.adjust_speed,
                        "e.g. 300rpm",
                        "--adjust-speed SPEED",
                    );
                    widgets::text_opt(ui, "PLL", &mut o.pll, "period=5:phase=60", "--pll PLLSPEC");
                    ui.checkbox(&mut o.hard_sectors, "Convert index positions to hard sectors");
                    ui.checkbox(&mut o.reverse, "Reverse track data (flippy disk)");
                });
            });
            ui.add_space(10.0);
            widgets::tracks_badge(ui, &mut o.tracks);
            let action = Action::Convert(o.clone());
            if widgets::action_button(ui, ph::SWAP, "CONVERT", c, !busy) {
                app.request(action.clone());
            }
            widgets::command_preview(ui, &action.command_line(&app.s.cfg, app.s.drive));
            if folder_error.is_some() {
                app.notice = folder_error;
            }
        });
    }

    fn page_erase(&mut self, ui: &mut Ui) {
        self.split(ui, |app, ui| {
            let busy = app.busy();
            let c = theme::ERASE;
            let o = &mut app.s.erase;
            card(ui, ph::WARNING, "Destructive operation", c, |ui| {
                ui.label(
                    "Erasing wipes the magnetic surface. It does not create a filesystem: \
                          use Format disk (or write an image) to get a usable disk.",
                );
            });
            ui.add_space(8.0);
            card(ui, ph::SLIDERS_HORIZONTAL, "Options", c, |ui| {
                widgets::tracks(ui, &mut o.tracks);
                widgets::opt_u32(ui, "Revolutions", &mut o.revs, 1, 20, "--revs per track (default 1)");
                ui.checkbox(&mut o.hfreq, "High-frequency erase (--hfreq)")
                    .on_hover_text("Write a high-frequency signal instead of DC erase");
                widgets::text_opt(ui, "Fake index", &mut o.fake_index, "e.g. 300rpm", "--fake-index SPEED");
            });
            ui.add_space(10.0);
            widgets::tracks_badge(ui, &mut o.tracks);
            let action = Action::Erase(o.clone());
            if widgets::action_button(ui, ph::ERASER, "ERASE DISK", c, !busy) {
                app.request(action.clone());
            }
            widgets::command_preview(ui, &action.command_line(&app.s.cfg, app.s.drive));
        });
    }

    fn page_rpm(&mut self, ui: &mut Ui) {
        let busy = self.busy();
        let c = theme::DRIVE;
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(theme::w(FORM_WIDTH));
                card(ui, ph::GAUGE, "Measure spindle speed", c, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Measurements");
                        ui.add(egui::Slider::new(&mut self.s.rpm_nr, 1..=100));
                    });
                    ui.label(
                        RichText::new("Insert any disk: the index pulse is timed on every revolution.")
                            .small()
                            .color(theme::MUTED),
                    );
                });
                ui.add_space(10.0);
                if widgets::action_button(ui, ph::PLAY, "MEASURE RPM", c, !busy) {
                    let nr = self.s.rpm_nr;
                    self.request(Action::Rpm { nr });
                }
                if !self.rpm.stats.is_empty() {
                    ui.add_space(10.0);
                    card(ui, ph::LIST_NUMBERS, "Statistics", c, |ui| {
                        let rows: Vec<(String, String)> =
                            self.rpm.stats.iter().map(|(k, v)| (k.clone(), format!("{v:.3} rpm"))).collect();
                        widgets::kv_grid(ui, "rpm_stats", &rows);
                    });
                }
            });
            ui.separator();
            ui.vertical(|ui| {
                let last = self.rpm.samples.last().copied();
                let (nominal, dev) = last
                    .map(|r| {
                        let nominal = if (r - 360.0).abs() < (r - 300.0).abs() { 360.0 } else { 300.0 };
                        (nominal, (r - nominal) / nominal * 100.0)
                    })
                    .unwrap_or((300.0, 0.0));
                let col = match dev.abs() {
                    d if d < 1.0 => theme::OK,
                    d if d < 2.5 => theme::WARN,
                    _ => theme::BAD,
                };
                ui.add_space(20.0);
                ui.vertical_centered(|ui| match last {
                    Some(r) => {
                        ui.label(RichText::new(format!("{r:.2}")).size(theme::fs(64.0)).strong().color(col));
                        ui.label(
                            RichText::new(format!("rpm   ·   {dev:+.2}% vs {nominal:.0} rpm"))
                                .size(theme::fs(16.0)),
                        );
                    }
                    None => {
                        ui.label(RichText::new("— rpm").size(theme::fs(64.0)).color(theme::MUTED));
                    }
                });
                ui.add_space(20.0);
                sparkline(ui, &self.rpm.samples, nominal, 220.0);
            });
        });
    }

    fn page_align(&mut self, ui: &mut Ui) {
        let busy = self.busy();
        let c = theme::DRIVE;
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(theme::w(FORM_WIDTH));
                egui::ScrollArea::vertical().id_salt("align_form").show(ui, |ui| {
                    let o = &mut self.s.align;
                    card(ui, ph::CROSSHAIR, "Alignment disk", c, |ui| {
                        ui.label(
                            RichText::new(
                                "Repeatedly reads the same track while you adjust the head. \
                                                Use a known-good or alignment disk.",
                            )
                            .small()
                            .color(theme::MUTED),
                        );
                        widgets::format_picker(
                            ui,
                            "align_fmt",
                            &mut o.format,
                            &self.catalog,
                            "None — raw flux",
                        );
                        widgets::text_opt(
                            ui,
                            "Track",
                            &mut o.tracks,
                            "c=0:h=0",
                            "--tracks: one cylinder (one or both heads)",
                        );
                        ui.horizontal(|ui| {
                            for (lbl, spec) in [
                                ("Cyl 0", "c=0:h=0"),
                                ("Cyl 40", "c=40:h=0"),
                                ("Cyl 79", "c=79:h=0"),
                                ("Both heads", "c=0:h=0-1"),
                            ] {
                                if ui.small_button(lbl).clicked() {
                                    o.tracks = spec.into();
                                }
                            }
                        });
                        widgets::opt_u32(
                            ui,
                            "Reads",
                            &mut o.reads,
                            100,
                            10_000,
                            "--reads: number of reads (default 10)",
                        );
                        widgets::opt_u32(
                            ui,
                            "Revolutions",
                            &mut o.revs,
                            3,
                            20,
                            "--revs per read (default 3)",
                        );
                        ui.checkbox(&mut o.raw, "Raw flux (no decoding)");
                        egui::CollapsingHeader::new("Advanced").id_salt("align_adv").show(ui, |ui| {
                            widgets::flux(ui, &mut o.flux);
                            widgets::densel(ui, "align_densel", &mut o.densel, &mut o.gen_tg43);
                        });
                    });
                    ui.add_space(10.0);
                    if widgets::action_button(ui, ph::CROSSHAIR, "START ALIGNMENT", c, !busy) {
                        let a = Action::Align(o.clone());
                        self.request(a);
                    }
                });
            });
            ui.separator();
            ui.vertical(|ui| align_chart(ui, &self.align));
        });
    }

    fn page_seek(&mut self, ui: &mut Ui) {
        let busy = self.busy();
        let c = theme::DRIVE;
        card(ui, ph::ARROWS_DOWN_UP, "Move the head", c, |ui| {
            ui.horizontal(|ui| {
                ui.label("Cylinder");
                ui.add(egui::DragValue::new(&mut self.s.seek.cylinder).range(0..=100));
                for cyl in [0, 40, 79, 83] {
                    if ui.small_button(cyl.to_string()).clicked() {
                        self.s.seek.cylinder = cyl;
                    }
                }
            });
            ui.checkbox(&mut self.s.seek.motor_on, "Spin the motor while seeking");
            if self.s.seek.is_extreme() {
                ui.label(
                    RichText::new(format!(
                        "{}  Beyond cylinder 83: you will be asked to confirm.",
                        ph::WARNING
                    ))
                    .color(theme::WARN),
                );
            }
        });
        ui.add_space(10.0);
        if widgets::action_button(ui, ph::ARROWS_DOWN_UP, "SEEK", c, !busy) {
            let a = Action::Seek(self.s.seek.clone());
            self.request(a);
        }
    }

    fn page_clean(&mut self, ui: &mut Ui) {
        let busy = self.busy();
        let c = theme::DRIVE;
        card(ui, ph::BROOM, "Head cleaning", c, |ui| {
            ui.label(
                RichText::new("Insert a wet CLEANING disk. The head sweeps across it in a zig-zag pattern.")
                    .color(theme::WARN),
            );
            let o = &mut self.s.clean;
            egui::Grid::new("clean").num_columns(2).show(ui, |ui| {
                ui.label("Cylinders");
                ui.add(egui::DragValue::new(&mut o.cyls).range(1..=100));
                ui.end_row();
                ui.label("Passes");
                ui.add(egui::DragValue::new(&mut o.passes).range(1..=50));
                ui.end_row();
                ui.label("Linger per step");
                ui.add(egui::DragValue::new(&mut o.linger).range(0..=2000).suffix(" ms"));
                ui.end_row();
            });
        });
        ui.add_space(10.0);
        if widgets::action_button(ui, ph::BROOM, "CLEAN DRIVE", c, !busy) {
            let a = Action::Clean(self.s.clean.clone());
            self.request(a);
        }
    }

    fn page_info(&mut self, ui: &mut Ui) {
        let busy = self.busy();
        let c = theme::DEVICE;
        card(ui, ph::USB, "Greaseweazle", c, |ui| {
            let (txt, col) = match self.device.state {
                DeviceState::Connected => ("Connected", theme::OK),
                DeviceState::Probing => ("Detecting…", theme::WARN),
                DeviceState::NotFound => ("No device found — check the USB cable", theme::BAD),
                DeviceState::NoTools => ("Host tools (gw) not found — see Settings", theme::BAD),
                DeviceState::Unknown => ("Not probed yet", theme::MUTED),
            };
            ui.label(RichText::new(txt).strong().color(col));
            ui.add_space(4.0);
            widgets::kv_grid(ui, "info", &self.device.fields);
            ui.checkbox(&mut self.s.info_bootloader, "Show bootloader info (F7 models)");
        });
        if let Some(v) = self.device.firmware_update.clone() {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("{}  Firmware {v} is available.", ph::CLOUD_ARROW_DOWN))
                        .color(theme::WARN)
                        .strong(),
                );
                if ui.button("Open firmware update").clicked() {
                    self.s.page = Page::Update;
                }
            });
        }
        ui.add_space(10.0);
        if widgets::action_button(ui, ph::MAGNIFYING_GLASS, "DETECT DEVICE", c, !busy) {
            self.probe();
        }
    }

    fn page_delays(&mut self, ui: &mut Ui) {
        let busy = self.busy();
        let c = theme::DEVICE;
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(theme::w(FORM_WIDTH));
                card(ui, ph::LIST_NUMBERS, "Current values", c, |ui| {
                    if self.delays.is_empty() {
                        ui.label(RichText::new("Not read yet.").color(theme::MUTED));
                    }
                    widgets::kv_grid(ui, "delays_now", &self.delays);
                });
                ui.add_space(8.0);
                if widgets::action_button(ui, ph::ARROWS_CLOCKWISE, "READ DELAYS", c, !busy) {
                    self.request(Action::Delays(DelaysOpts::default()));
                }
            });
            ui.separator();
            ui.vertical(|ui| {
                ui.set_width(theme::w(FORM_WIDTH));
                card(ui, ph::TIMER, "Change values", theme::WARN, |ui| {
                    let d = &mut self.s.delays_edit;
                    widgets::opt_u32(
                        ui,
                        "Select delay (µs)",
                        &mut d.select,
                        10,
                        100_000,
                        "Delay after asserting drive select",
                    );
                    widgets::opt_u32(
                        ui,
                        "Step delay (µs)",
                        &mut d.step,
                        5000,
                        100_000,
                        "Delay after a head-step pulse",
                    );
                    widgets::opt_u32(ui, "Settle time (ms)", &mut d.settle, 15, 10_000, "Delay after a seek");
                    widgets::opt_u32(ui, "Motor delay (ms)", &mut d.motor, 750, 10_000, "Spin-up time");
                    widgets::opt_u32(
                        ui,
                        "Watchdog (ms)",
                        &mut d.watchdog,
                        10_000,
                        1_000_000,
                        "Idle time before drives are deselected",
                    );
                    widgets::opt_u32(
                        ui,
                        "Pre-write (µs)",
                        &mut d.pre_write,
                        100,
                        100_000,
                        "Min. time from track change to write",
                    );
                    widgets::opt_u32(
                        ui,
                        "Post-write (µs)",
                        &mut d.post_write,
                        1000,
                        100_000,
                        "Min. time from write end to track change",
                    );
                    widgets::opt_u32(
                        ui,
                        "Index mask (µs)",
                        &mut d.index_mask,
                        200,
                        100_000,
                        "Index post-trigger mask time",
                    );
                });
                ui.add_space(8.0);
                let has_changes = !self.s.delays_edit.is_query();
                if widgets::action_button(ui, ph::CHECK, "APPLY", theme::WRITE, !busy && has_changes) {
                    let a = Action::Delays(self.s.delays_edit.clone());
                    self.request(a);
                }
                ui.add_space(4.0);
                if ui
                    .add_enabled(
                        !busy,
                        egui::Button::new(format!("{}  Restore factory delays", ph::ARROW_COUNTER_CLOCKWISE)),
                    )
                    .clicked()
                {
                    self.request(Action::Reset { delays: true });
                }
            });
        });
    }

    fn page_pins(&mut self, ui: &mut Ui) {
        let busy = self.busy();
        let c = theme::DEVICE;
        card(ui, ph::PLUGS, "User-modifiable interface pin", c, |ui| {
            ui.horizontal(|ui| {
                ui.label("Pin");
                ui.add(egui::DragValue::new(&mut self.s.pin).range(1..=34));
                ui.label(
                    RichText::new("(pin 2 = density select on most drives)").small().color(theme::MUTED),
                );
            });
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(!busy, egui::Button::new(format!("{}  Get level", ph::MAGNIFYING_GLASS)))
                    .clicked()
                {
                    let pin = self.s.pin;
                    self.request(Action::PinGet { pin });
                }
                ui.separator();
                ui.selectable_value(&mut self.s.pin_high, true, "High");
                ui.selectable_value(&mut self.s.pin_high, false, "Low");
                if ui
                    .add_enabled(!busy, egui::Button::new(format!("{}  Set level", ph::TOGGLE_LEFT)))
                    .clicked()
                {
                    let (pin, high) = (self.s.pin, self.s.pin_high);
                    self.request(Action::PinSet { pin, high });
                }
            });
            if let Some(r) = &self.pin_result {
                ui.add_space(6.0);
                ui.label(RichText::new(r).size(theme::fs(18.0)).strong().color(theme::ACCENT));
            }
        });
    }

    fn page_bandwidth(&mut self, ui: &mut Ui) {
        let busy = self.busy();
        let c = theme::DEVICE;
        card(ui, ph::LIGHTNING, "USB bandwidth test", c, |ui| {
            if self.bandwidth.is_empty() {
                ui.label(RichText::new("Checks the USB link can sustain flux streaming. Use a direct port, not a hub, if it fails.")
                    .color(theme::MUTED));
            } else {
                egui::Grid::new("bw").num_columns(4).striped(true).spacing([24.0, 6.0]).show(ui, |ui| {
                    for h in ["", "Min", "Mean", "Max"] {
                        ui.label(RichText::new(h).strong());
                    }
                    ui.end_row();
                    for (d, min, mean, max) in &self.bandwidth {
                        ui.label(RichText::new(d).strong());
                        for v in [min, mean, max] {
                            ui.label(format!("{v:.3} Mbps"));
                        }
                        ui.end_row();
                    }
                });
            }
            for n in &self.bandwidth_notes {
                let col =
                    if n.contains("WARNING") || n.contains("ERROR") { theme::BAD } else { theme::MUTED };
                ui.label(RichText::new(n).color(col));
            }
        });
        ui.add_space(10.0);
        if widgets::action_button(ui, ph::LIGHTNING, "RUN TEST", c, !busy) {
            self.request(Action::Bandwidth);
        }
    }

    fn page_reset(&mut self, ui: &mut Ui) {
        let busy = self.busy();
        let c = theme::DEVICE;
        card(ui, ph::ARROW_COUNTER_CLOCKWISE, "Reset to power-on state", c, |ui| {
            ui.label("Deselects all drives, stops motors and restores pins.");
            ui.checkbox(&mut self.s.reset_delays, "Also restore factory drive delays");
        });
        ui.add_space(10.0);
        if widgets::action_button(ui, ph::POWER, "RESET DEVICE", c, !busy) {
            let delays = self.s.reset_delays;
            self.request(Action::Reset { delays });
        }
    }

    fn page_update(&mut self, ui: &mut Ui) {
        let busy = self.busy();
        let c = theme::DEVICE;
        let o = &mut self.s.update;
        card(ui, ph::CLOUD_ARROW_DOWN, "Firmware source", c, |ui| {
            ui.label(
                RichText::new("Leave both empty to download the latest release from GitHub.")
                    .small()
                    .color(theme::MUTED),
            );
            ui.label("Update file (.upd)");
            widgets::path_row(
                ui,
                &mut o.file,
                PickMode::Open,
                "optional local file",
                "",
                &mut self.s.last_dir,
            );
            widgets::text_opt(ui, "GitHub tag", &mut o.tag, "e.g. v1.6", "--tag: specific release");
            ui.checkbox(&mut o.force, "Force (allow downgrade)");
            ui.checkbox(
                &mut o.bootloader,
                RichText::new("Update the bootloader (dangerous)").color(theme::BAD),
            );
        });
        ui.add_space(8.0);
        ui.label(
            RichText::new(format!("{}  Keep the device plugged in until the update completes.", ph::WARNING))
                .color(theme::WARN),
        );
        ui.add_space(10.0);
        if widgets::action_button(ui, ph::CLOUD_ARROW_DOWN, "UPDATE FIRMWARE", theme::WRITE, !busy) {
            let a = Action::Update(o.clone());
            self.request(a);
        }
    }

    fn page_inspector(&mut self, ui: &mut Ui) {
        let c = theme::IMAGE;
        let mut path = self.inspect_path.clone();
        card(ui, ph::FILE_MAGNIFYING_GLASS, "Image file", c, |ui| {
            if widgets::path_row(
                ui,
                &mut path,
                PickMode::Open,
                "choose or drop an image",
                "",
                &mut self.s.last_dir,
            ) && path.is_file()
            {
                self.inspect(path.clone());
            }
            self.inspect_path = path;
        });
        ui.add_space(8.0);
        let Some((p, report)) = &self.inspector else {
            ui.label(RichText::new("Drop any disk image on the window to inspect it.").color(theme::MUTED));
            return;
        };
        let p = p.clone();
        match report {
            Err(e) => {
                ui.label(RichText::new(e).color(theme::BAD));
            }
            Ok(r) => {
                let mut go: Option<Page> = None;
                egui::ScrollArea::vertical().id_salt("insp").show(ui, |ui| {
                    card(ui, ph::INFO, "Summary", c, |ui| {
                        let rows = vec![
                            ("File".to_string(), r.file_name.clone()),
                            ("Size".into(), widgets::human_size(r.size)),
                            (
                                "Type".into(),
                                if r.is_flux {
                                    format!(".{} (flux)", r.extension)
                                } else {
                                    format!(".{}", r.extension)
                                },
                            ),
                            (
                                "Format".into(),
                                r.guessed_format.unwrap_or("not recognised from size").to_string(),
                            ),
                        ];
                        widgets::kv_grid(ui, "insp_sum", &rows);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("SHA-256").color(theme::MUTED));
                            ui.label(RichText::new(&r.sha256).monospace().small());
                            if ui.small_button(ph::COPY).on_hover_text("Copy").clicked() {
                                ui.ctx().copy_text(r.sha256.clone());
                            }
                        });
                        ui.horizontal(|ui| {
                            if ui.button(format!("{}  Write to disk", ph::UPLOAD_SIMPLE)).clicked() {
                                go = Some(Page::Write);
                            }
                            if ui.button(format!("{}  Convert", ph::SWAP)).clicked() {
                                go = Some(Page::Convert);
                            }
                        });
                    });
                    if let Some(adf) = &r.adf {
                        ui.add_space(8.0);
                        card(ui, ph::FLOPPY_DISK, "AmigaDOS", theme::READ, |ui| match adf {
                            Err(e) => {
                                ui.label(RichText::new(e).color(theme::WARN));
                            }
                            Ok(a) => {
                                let yes_no = |b: bool| if b { "yes" } else { "no" }.to_string();
                                let rows = vec![
                                    ("Volume".to_string(), a.volume.clone()),
                                    ("Filesystem".into(), a.filesystem.clone()),
                                    ("Bootable".into(), yes_no(a.bootable)),
                                    (
                                        "Boot block checksum".into(),
                                        if a.boot_checksum_ok { "valid".into() } else { "invalid".into() },
                                    ),
                                    (
                                        "Root block".into(),
                                        format!(
                                            "{} ({})",
                                            a.root_block,
                                            if a.root_checksum_ok { "checksum OK" } else { "BAD checksum" }
                                        ),
                                    ),
                                    ("Root entries".into(), a.entries.len().to_string()),
                                ];
                                widgets::kv_grid(ui, "adf", &rows);
                                ui.add_space(6.0);
                                egui::Grid::new("adf_files")
                                    .num_columns(3)
                                    .striped(true)
                                    .spacing([16.0, 4.0])
                                    .show(ui, |ui| {
                                        for e in &a.entries {
                                            let icon = if e.is_dir { ph::FOLDER } else { ph::FILE_TEXT };
                                            ui.label(format!("{icon}  {}", e.name));
                                            ui.label(if e.is_dir {
                                                "—".into()
                                            } else {
                                                widgets::human_size(e.size as u64)
                                            });
                                            ui.label(
                                                RichText::new(format!("block {}", e.block))
                                                    .small()
                                                    .color(theme::MUTED),
                                            );
                                            ui.end_row();
                                        }
                                    });
                            }
                        });
                    }
                });
                match go {
                    Some(Page::Write) => {
                        self.set_write_input(p);
                        self.s.page = Page::Write;
                    }
                    Some(Page::Convert) => {
                        self.s.convert.input = p;
                        self.s.page = Page::Convert;
                    }
                    _ => {}
                }
            }
        }
    }

    fn page_settings(&mut self, ui: &mut Ui) {
        let c = theme::DEVICE;
        egui::ScrollArea::vertical().id_salt("settings").show(ui, |ui| {
            card(ui, ph::TERMINAL, "Greaseweazle host tools", c, |ui| {
                ui.label("gw executable");
                let mut prog = std::path::PathBuf::from(&self.s.cfg.program);
                if widgets::path_row(ui, &mut prog, PickMode::Open, "gw", "", &mut self.s.last_dir) {
                    self.s.cfg.program = prog.to_string_lossy().into_owned();
                }
                ui.horizontal(|ui| {
                    let idle = !self.busy() && !self.detecting;
                    if ui
                        .add_enabled(idle, egui::Button::new(format!("{}  Test & reload formats", ph::ARROWS_CLOCKWISE)))
                        .clicked()
                    {
                        self.reload_catalog();
                        self.probe();
                    }
                    if ui
                        .add_enabled(idle, egui::Button::new(format!("{}  Auto-detect", ph::MAGNIFYING_GLASS)))
                        .on_hover_text(
                            "Search the PATH, this program's folder, Python installs and the unzipped Windows release",
                        )
                        .clicked()
                    {
                        self.detect_gw(true);
                    }
                    if self.detecting {
                        ui.spinner();
                    }
                    let (txt, col) = if self.catalog.from_gw {
                        (format!("{} formats read from gw", self.catalog.formats.len()), theme::OK)
                    } else if let Some(e) = &self.catalog_error {
                        (format!("built-in format list ({e})"), theme::WARN)
                    } else {
                        ("built-in format list".to_string(), theme::MUTED)
                    };
                    ui.label(RichText::new(txt).small().color(col));
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label("Device (--device)").on_hover_text("Serial port of the Greaseweazle when several are connected");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.s.cfg.device)
                            .hint_text("auto-detect — e.g. COM3 or /dev/ttyACM0")
                            .desired_width(theme::w(230.0)),
                    );
                    let shown = if self.s.cfg.device.is_empty() { "Auto" } else { "Ports" };
                    let r = egui::ComboBox::from_id_salt("ports").selected_text(shown).show_ui(ui, |ui| {
                        if ui.selectable_label(self.s.cfg.device.is_empty(), "Auto-detect").clicked() {
                            self.s.cfg.device.clear();
                        }
                        match &self.ports {
                            Some(list) if list.is_empty() => {
                                ui.label(RichText::new("No serial port found").small().color(theme::MUTED));
                            }
                            Some(list) => {
                                for p in list {
                                    let mut text = p.name.clone();
                                    if p.greaseweazle {
                                        text = format!("{}  {text}", ph::STAR);
                                    }
                                    let r = ui.selectable_label(self.s.cfg.device == p.name, text);
                                    let r = if p.description.is_empty() { r } else { r.on_hover_text(&p.description) };
                                    if r.clicked() {
                                        self.s.cfg.device = p.name.clone();
                                    }
                                }
                            }
                            None => {}
                        }
                    });
                    // List the ports when the menu is opened, so that a device plugged in later shows up.
                    if r.response.clicked() || self.ports.is_none() {
                        self.ports = Some(hgf_core::ports::list());
                    }
                });
                ui.label("Custom disk definitions (--diskdefs)");
                let mut dd = std::path::PathBuf::from(&self.s.cfg.diskdefs);
                if widgets::path_row(ui, &mut dd, PickMode::Open, "built-in", "", &mut self.s.last_dir) {
                    self.s.cfg.diskdefs = dd.to_string_lossy().into_owned();
                }
                ui.checkbox(&mut self.s.cfg.time, "Print elapsed time after each command (--time)");
                ui.checkbox(&mut self.s.cfg.backtrace, "Detailed error messages for bug reports (--bt)")
                    .on_hover_text("gw prints a Python backtrace when a command fails");
                ui.checkbox(&mut self.s.probe_on_start, "Detect the device at start-up");
            });
            ui.add_space(8.0);
            card(ui, ph::PALETTE, "Appearance", c, |ui| {
                ui.horizontal(|ui| {
                    let before = self.s.theme_choice();
                    for (choice, label) in [
                        (theme::ThemeChoice::Dark, format!("{}  Dark", ph::MOON)),
                        (theme::ThemeChoice::Light, format!("{}  Light", ph::SUN)),
                        (theme::ThemeChoice::System, format!("{}  System", ph::DESKTOP)),
                    ] {
                        if ui.selectable_label(before == choice, label).clicked() {
                            self.s.follow_system = choice == theme::ThemeChoice::System;
                            if choice != theme::ThemeChoice::System {
                                self.s.dark = choice == theme::ThemeChoice::Dark;
                            }
                        }
                    }
                    if before != self.s.theme_choice() {
                        theme::apply(ui.ctx(), self.s.theme_choice(), self.s.text_scale);
                        // Wake up once the autosave is due, even if the window stays idle.
                        ui.ctx().request_repaint_after(crate::app::SAVE_INTERVAL + std::time::Duration::from_secs(1));
                    }
                });
                ui.add_space(6.0);
                ui.label(format!("{}  Text size", ph::TEXT_AA));
                ui.horizontal_wrapped(|ui| {
                    let before = self.s.text_scale;
                    for (pct, label) in [(80, "80 %"), (90, "90 %"), (100, "100 %"), (115, "115 %"), (130, "130 %"), (150, "150 %"), (160, "160 %")] {
                        let v = pct as f32 / 100.0;
                        let selected = (self.s.text_scale - v).abs() < 0.001;
                        if ui.selectable_label(selected, label).clicked() {
                            self.s.text_scale = v;
                        }
                    }
                    if (before - self.s.text_scale).abs() > f32::EPSILON {
                        theme::apply(ui.ctx(), self.s.theme_choice(), self.s.text_scale);
                        // Wake up once the autosave is due, even if the window stays idle.
                        ui.ctx().request_repaint_after(crate::app::SAVE_INTERVAL + std::time::Duration::from_secs(1));
                    }
                });
                ui.label(
                    RichText::new("Track T17.1: 0/11 sectors, Il1| O0 — font: JetBrains Mono")
                        .monospace()
                        .color(theme::MUTED),
                );
            });
            ui.add_space(8.0);
            card(ui, ph::NOTEPAD, "Log & notifications", c, |ui| {
                ui.label("Log file folder");
                let mut dir = std::path::PathBuf::from(&self.s.log_dir);
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.s.log_dir)
                            .hint_text("no log file")
                            .desired_width(theme::w(300.0)),
                    );
                    if ui.button(ph::FOLDER_OPEN).on_hover_text("Choose the folder").clicked() {
                        let mut d = rfd::FileDialog::new();
                        if dir.is_dir() {
                            d = d.set_directory(&dir);
                        }
                        if let Some(p) = d.pick_folder() {
                            dir = p;
                            self.s.log_dir = dir.to_string_lossy().into_owned();
                        }
                    }
                    if let Some(e) = widgets::folder_button(ui, &dir) {
                        self.notice = Some(e);
                    }
                    if !self.s.log_dir.is_empty() && ui.small_button(ph::X).on_hover_text("No log file").clicked() {
                        self.s.log_dir.clear();
                    }
                });
                ui.label(
                    RichText::new(
                        "Every command and its full output are appended to happygreasefloppy-DATE.log \
                         in this folder (one file per day).",
                    )
                    .small()
                    .color(theme::MUTED),
                );
                ui.checkbox(&mut self.s.notify_done, "Flash the taskbar / Dock when an operation ends in the background");
            });
            ui.add_space(8.0);
            card(ui, ph::ARROW_COUNTER_CLOCKWISE, "Reset", c, |ui| {
                if ui
                    .button("Restore all options to their defaults")
                    .on_hover_text("Saved templates, the gw path and the appearance are kept")
                    .clicked()
                {
                    let keep = (
                        self.s.cfg.clone(),
                        self.s.dark,
                        self.s.follow_system,
                        self.s.text_scale,
                        std::mem::take(&mut self.s.templates),
                        self.s.log_dir.clone(),
                    );
                    self.s = Default::default();
                    self.s.cfg = keep.0;
                    self.s.dark = keep.1;
                    self.s.follow_system = keep.2;
                    self.s.text_scale = keep.3;
                    self.s.templates = keep.4;
                    self.s.log_dir = keep.5;
                    self.s.page = Page::Settings;
                }
            });
            ui.add_space(8.0);
            card(ui, ph::QUESTION, "Help", c, |ui| {
                for (label, url) in [
                    ("Getting started (Greaseweazle wiki)", "https://github.com/keirf/greaseweazle/wiki/Getting-Started"),
                    ("Troubleshooting FAQ: No Index, Track 0 not found…", "https://github.com/keirf/greaseweazle/wiki/Troubleshooting-FAQ"),
                    ("Drive select: A, B, 0, 1…", "https://github.com/keirf/greaseweazle/wiki/Drive-Select"),
                    ("Specifying tracks (TSPEC)", "https://github.com/keirf/greaseweazle/wiki/Specifying-Tracks"),
                    ("Supported image types and file options", "https://github.com/keirf/greaseweazle/wiki/Supported-Image-Types"),
                    ("Write precompensation", "https://github.com/keirf/greaseweazle/wiki/Write-Precompensation"),
                    ("Download the host tools", "https://github.com/keirf/greaseweazle/wiki/Download-Host-Tools"),
                ] {
                    ui.hyperlink_to(format!("{}  {label}", ph::ARROW_SQUARE_OUT), url);
                }
            });
            ui.add_space(8.0);
            ui.label(
                RichText::new(format!(
                    "HappyGreaseFloppy {} — a front-end for Keir Fraser's Greaseweazle host tools. \
                     Inspired by FluxMyFluffyFloppy by FrankieTheFluff.",
                    hgf_core::VERSION
                ))
                .small()
                .color(theme::MUTED),
            );
        });
    }
}

/// Single-column page of bounded width (cards and buttons line up).
fn narrow(ui: &mut Ui, width: f32, add: impl FnOnce(&mut Ui)) {
    let w = width.min(ui.available_width());
    ui.vertical(|ui| {
        ui.set_width(w);
        add(ui);
    });
}

/// RPM history with a band around the nominal speed.
fn sparkline(ui: &mut Ui, samples: &[f64], nominal: f64, height: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), height), Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 8.0, ui.visuals().extreme_bg_color);
    let (lo, hi) = (nominal * 0.97, nominal * 1.03);
    let y = |v: f64| rect.bottom() - ((v.clamp(lo, hi) - lo) / (hi - lo)) as f32 * rect.height();
    let band = egui::Rect::from_x_y_ranges(rect.x_range(), y(nominal * 1.01)..=y(nominal * 0.99));
    p.rect_filled(band, 0.0, theme::tint(theme::OK, 30));
    p.line_segment(
        [egui::pos2(rect.left(), y(nominal)), egui::pos2(rect.right(), y(nominal))],
        Stroke::new(1.0, theme::tint(theme::OK, 120)),
    );
    p.text(
        rect.left_top() + egui::vec2(6.0, 4.0),
        egui::Align2::LEFT_TOP,
        format!("{:.0} rpm ±3%", nominal),
        egui::FontId::proportional(theme::fs(11.0)),
        theme::MUTED,
    );
    if samples.len() >= 2 {
        let n = samples.len() - 1;
        let pts: Vec<egui::Pos2> = samples
            .iter()
            .enumerate()
            .map(|(i, &v)| egui::pos2(rect.left() + 8.0 + (rect.width() - 16.0) * i as f32 / n as f32, y(v)))
            .collect();
        p.add(Shape::line(pts, Stroke::new(2.0, theme::ACCENT)));
    }
}

/// Bars of decoded sectors per alignment read (green = all sectors, red = none).
fn align_chart(ui: &mut Ui, data: &std::collections::VecDeque<(u8, u32, u32)>) {
    let last = data.back();
    ui.vertical_centered(|ui| match last {
        Some((h, ok, total)) => {
            let col = if ok == total {
                theme::OK
            } else if *ok > 0 {
                theme::WARN
            } else {
                theme::BAD
            };
            ui.label(RichText::new(format!("{ok}/{total}")).size(theme::fs(60.0)).strong().color(col));
            ui.label(format!("sectors on the last read (side {h})"));
        }
        None => {
            ui.label(RichText::new("—").size(theme::fs(60.0)).color(theme::MUTED));
            ui.label(
                RichText::new(
                    "Start the alignment: sectors decoded on each read appear here (needs a disk format).",
                )
                .color(theme::MUTED),
            );
        }
    });
    ui.add_space(12.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 220.0), Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 8.0, ui.visuals().extreme_bg_color);
    if data.is_empty() {
        return;
    }
    let n = data.len().max(60) as f32;
    let w = (rect.width() - 12.0) / n;
    for (i, (h, ok, total)) in data.iter().enumerate() {
        let frac = if *total > 0 { *ok as f32 / *total as f32 } else { 0.0 };
        let col = if frac >= 1.0 {
            theme::OK
        } else if frac > 0.0 {
            theme::WARN
        } else {
            theme::BAD
        };
        let col = if *h == 1 { theme::tint(col, 170) } else { col };
        let x = rect.left() + 6.0 + i as f32 * w;
        let bar_h = (rect.height() - 12.0) * frac.max(0.02);
        p.rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(x, rect.bottom() - 6.0 - bar_h),
                egui::pos2(x + (w - 1.0).max(1.0), rect.bottom() - 6.0),
            ),
            1.0,
            col,
        );
    }
}

impl App {
    /// "Amiga compatibility" card on the Write page.
    fn amiga_check(&mut self, ui: &mut Ui) {
        use hgf_core::compat::{self, Severity};
        if !compat::is_amiga_write(&self.s.write) {
            return;
        }
        let info = self.write_adf_info().cloned();
        let concerns = compat::amiga_write_concerns(&self.s.write, info.as_ref());
        let dark = ui.visuals().dark_mode;
        let risky_options = concerns.iter().any(|c| c.option);
        card(ui, ph::CHECK_SQUARE_OFFSET, "Amiga compatibility", theme::WRITE, |ui| {
            for c in &concerns {
                let (icon, col) = match c.severity {
                    Severity::Warn => (ph::WARNING, theme::WARN),
                    Severity::Info => (ph::INFO, theme::MUTED),
                    Severity::Good => (ph::CHECK_CIRCLE, theme::OK),
                };
                ui.horizontal_top(|ui| {
                    ui.label(RichText::new(icon).color(theme::readable(col, dark)));
                    ui.add(egui::Label::new(RichText::new(&c.text).small()).wrap());
                });
            }
            if risky_options
                && ui
                    .button(format!("{}  Reset these options", ph::ARROW_COUNTER_CLOCKWISE))
                    .on_hover_text("Format default tracks, no reverse, no fake index, density on Auto")
                    .clicked()
            {
                compat::reset_risky(&mut self.s.write);
            }
            egui::CollapsingHeader::new(RichText::new("Verified here but not readable on the Amiga?").small())
                .id_salt("amiga_tips")
                .show(ui, |ui| {
                    for tip in [
                        "Use DD disks (one square hole). An HD disk written as DD is unreliable on an Amiga drive.",
                        "Prefer new or bulk-erased disks: on a reused disk, a slightly offset PC head leaves \
                         the old track beside the new one, and the Amiga's head reads both.",
                        "Check the Amiga drive with a known-good original, and clean both drives' heads.",
                        "Run Drive health: a failing head-offset test means the PC drive's alignment differs.",
                        "Try the Amiga precompensation (Advanced → Precomp).",
                    ] {
                        ui.horizontal_top(|ui| {
                            ui.label(RichText::new("•").color(theme::MUTED));
                            ui.add(egui::Label::new(RichText::new(tip).small()).wrap());
                        });
                    }
                });
        });
    }
}
