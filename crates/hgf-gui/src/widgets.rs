//! Reusable widgets: cards, file pickers, format picker, option rows.

use crate::theme;
use eframe::egui::{self, Color32, RichText, Ui};
use egui_phosphor::regular as ph;
use hgf_core::formats::{self, Catalog, PRESETS};
use hgf_core::gw::{Densel, FluxOpts, TrackOpts};
use hgf_core::hints::{Hint, HintAction};
use std::path::{Path, PathBuf};

/// A titled card.
pub fn card<R>(ui: &mut Ui, icon: &str, title: &str, color: Color32, add: impl FnOnce(&mut Ui) -> R) -> R {
    let text_color = theme::readable(color, ui.visuals().dark_mode);
    egui::Frame::new()
        .fill(ui.visuals().faint_bg_color)
        .stroke(egui::Stroke::new(1.0, theme::tint(color, 70)))
        .corner_radius(8)
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(format!("{icon}  {title}")).strong().color(text_color));
            ui.add_space(4.0);
            add(ui)
        })
        .inner
}

/// Coloured banner (errors, warnings, results).
pub fn banner(ui: &mut Ui, icon: &str, text: &str, color: Color32) -> bool {
    let mut closed = false;
    let fg = theme::readable(color, ui.visuals().dark_mode);
    egui::Frame::new()
        .fill(theme::tint(color, 38))
        .stroke(egui::Stroke::new(1.0, theme::tint(color, 150)))
        .corner_radius(6)
        .inner_margin(egui::Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(icon).size(theme::fs(18.0)).color(fg));
                ui.add(egui::Label::new(RichText::new(text).color(fg)).wrap());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    closed = ui.small_button(ph::X).on_hover_text("Dismiss").clicked();
                });
            });
        });
    closed
}

/// Diagnosis and next steps, with an optional button. Returns the clicked action.
pub fn hint_box(ui: &mut Ui, hint: &Hint, enabled: bool) -> Option<HintAction> {
    let dark = ui.visuals().dark_mode;
    let mut clicked = None;
    ui.horizontal_top(|ui| {
        ui.label(
            RichText::new(ph::LIGHTBULB).size(theme::fs(17.0)).color(theme::readable(theme::WARN, dark)),
        );
        ui.vertical(|ui| {
            ui.add(egui::Label::new(RichText::new(&hint.title).strong()).wrap());
            for step in &hint.steps {
                ui.horizontal_top(|ui| {
                    ui.label(RichText::new("•").color(theme::MUTED));
                    ui.add(egui::Label::new(step.as_str()).wrap());
                });
            }
            if let Some(a) = hint.action {
                ui.add_space(2.0);
                if ui
                    .add_enabled(enabled, egui::Button::new(format!("{}  {}", ph::ARROW_RIGHT, a.label())))
                    .clicked()
                {
                    clicked = Some(a);
                }
            }
        });
    });
    clicked
}

/// Result banner with an optional hint. Returns (dismissed, hint action clicked).
pub fn outcome_banner(
    ui: &mut Ui,
    icon: &str,
    text: &str,
    color: Color32,
    hint: Option<&Hint>,
    enabled: bool,
) -> (bool, Option<HintAction>) {
    let mut closed = false;
    let mut action = None;
    let fg = theme::readable(color, ui.visuals().dark_mode);
    egui::Frame::new()
        .fill(theme::tint(color, 38))
        .stroke(egui::Stroke::new(1.0, theme::tint(color, 150)))
        .corner_radius(6)
        .inner_margin(egui::Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(icon).size(theme::fs(18.0)).color(fg));
                ui.add(egui::Label::new(RichText::new(text).color(fg)).wrap());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    closed = ui.small_button(ph::X).on_hover_text("Dismiss").clicked();
                });
            });
            if let Some(h) = hint {
                ui.separator();
                action = hint_box(ui, h, enabled);
            }
        });
    (closed, action)
}

/// Visible reminder that the format's own track set is overridden, with a reset button.
pub fn tracks_badge(ui: &mut Ui, t: &mut TrackOpts) {
    let Some(spec) = t.spec() else { return };
    let dark = ui.visuals().dark_mode;
    egui::Frame::new()
        .fill(theme::tint(theme::WARN, 40))
        .stroke(egui::Stroke::new(1.0, theme::tint(theme::WARN, 180)))
        .corner_radius(6)
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(ph::WARNING).color(theme::readable(theme::WARN, dark)));
                ui.label(RichText::new("Custom tracks:").strong().color(theme::readable(theme::WARN, dark)));
                ui.label(RichText::new(&spec).monospace());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .small_button(format!("{}  Use format default", ph::X))
                        .on_hover_text("Clear the Tracks field, double step and head swap")
                        .clicked()
                    {
                        *t = TrackOpts::default();
                    }
                });
            });
        });
    ui.add_space(4.0);
}

/// Large filled action button.
pub fn action_button(ui: &mut Ui, icon: &str, label: &str, color: Color32, enabled: bool) -> bool {
    let text = RichText::new(format!("{icon}  {label}")).strong().size(theme::fs(16.0)).color(Color32::WHITE);
    let fill = if enabled { color } else { theme::tint(color, 70) };
    ui.add_enabled(
        enabled,
        egui::Button::new(text).fill(fill).corner_radius(8).min_size(egui::vec2(ui.available_width(), 42.0)),
    )
    .clicked()
}

/// Small pill label.
pub fn pill(ui: &mut Ui, text: &str, color: Color32) -> egui::Response {
    let fg = theme::readable(color, ui.visuals().dark_mode);
    egui::Frame::new()
        .fill(theme::tint(color, 45))
        .stroke(egui::Stroke::new(1.0, theme::tint(color, 160)))
        .corner_radius(12)
        .inner_margin(egui::Margin::symmetric(10, 3))
        .show(ui, |ui| ui.label(RichText::new(text).strong().color(fg)))
        .response
}

pub enum PickMode {
    Open,
    Save,
}

/// Path text field with a file dialog button. Returns `true` when changed.
pub fn path_row(
    ui: &mut Ui,
    path: &mut PathBuf,
    mode: PickMode,
    hint: &str,
    default_name: &str,
    last_dir: &mut Option<PathBuf>,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        let mut text = path.to_string_lossy().into_owned();
        let w = ui.available_width() - 44.0;
        if ui.add(egui::TextEdit::singleline(&mut text).hint_text(hint).desired_width(w)).changed() {
            *path = PathBuf::from(text);
            changed = true;
        }
        let (icon, tip) = match mode {
            PickMode::Open => (ph::FOLDER_OPEN, "Browse…"),
            PickMode::Save => (ph::FLOPPY_DISK_BACK, "Save as…"),
        };
        if ui.button(icon).on_hover_text(tip).clicked() {
            let mut d = rfd::FileDialog::new();
            if let Some(dir) =
                path.parent().filter(|p| p.is_dir()).map(Path::to_path_buf).or_else(|| last_dir.clone())
            {
                d = d.set_directory(dir);
            }
            let picked = match mode {
                PickMode::Open => d.pick_file(),
                PickMode::Save => {
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| default_name.to_string());
                    d.set_file_name(name).save_file()
                }
            };
            if let Some(p) = picked {
                *last_dir = p.parent().map(Path::to_path_buf);
                *path = p;
                changed = true;
            }
        }
    });
    changed
}

/// Replace the extension of `path` (or create `disk.<ext>`).
pub fn set_extension(path: &mut PathBuf, ext: &str) {
    if ext.is_empty() {
        return;
    }
    if path.as_os_str().is_empty() {
        *path = PathBuf::from(format!("disk.{ext}"));
    } else {
        path.set_extension(ext);
    }
}

/// One-click machine presets. Returns the chosen preset.
pub fn preset_chips(ui: &mut Ui, format: &str) -> Option<&'static formats::Preset> {
    let mut chosen = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
        for p in PRESETS {
            let selected = p.format == format && !p.format.is_empty();
            if ui.selectable_label(selected, RichText::new(p.label).size(theme::fs(12.5))).clicked() {
                chosen = Some(p);
            }
        }
    });
    chosen
}

/// Searchable format picker grouped by machine family.
pub fn format_picker(
    ui: &mut Ui,
    id: &str,
    format: &mut String,
    catalog: &Catalog,
    none_label: &str,
) -> bool {
    let before = format.clone();
    let shown = if format.is_empty() { none_label.to_string() } else { format.clone() };
    let search_id = ui.id().with(id).with("search");
    egui::ComboBox::from_id_salt(id)
        .selected_text(shown)
        .width(ui.available_width() - 8.0)
        .height(420.0)
        .show_ui(ui, |ui| {
            let mut search: String = ui.data_mut(|d| d.get_temp::<String>(search_id).unwrap_or_default());
            ui.horizontal(|ui| {
                ui.label(ph::MAGNIFYING_GLASS);
                ui.add(
                    egui::TextEdit::singleline(&mut search)
                        .hint_text("Filter formats…")
                        .desired_width(theme::w(220.0)),
                );
            });
            ui.data_mut(|d| d.insert_temp(search_id, search.clone()));
            let q = search.to_lowercase();
            ui.selectable_value(format, String::new(), none_label);
            for (fam, list) in catalog.families() {
                let fam_name = formats::family_name(&fam);
                let items: Vec<&String> = list
                    .iter()
                    .filter(|f| q.is_empty() || f.contains(&q) || fam_name.to_lowercase().contains(&q))
                    .collect();
                if items.is_empty() {
                    continue;
                }
                ui.separator();
                ui.label(RichText::new(fam_name).small().strong().color(theme::MUTED));
                for f in items {
                    ui.selectable_value(format, f.clone(), f.as_str());
                }
            }
        });
    *format != before
}

/// Optional numeric option: checkbox to override gw's default.
pub fn opt_u32(ui: &mut Ui, label: &str, v: &mut Option<u32>, default: u32, max: u32, tip: &str) {
    ui.horizontal(|ui| {
        let mut on = v.is_some();
        if ui.checkbox(&mut on, label).on_hover_text(tip).changed() {
            *v = on.then_some(v.unwrap_or(default));
        }
        if let Some(x) = v {
            ui.add(egui::DragValue::new(x).range(0..=max));
        } else {
            ui.label(RichText::new("default").italics().color(theme::MUTED));
        }
    });
}

/// Free-text option with a hint and tooltip.
pub fn text_opt(ui: &mut Ui, label: &str, v: &mut String, hint: &str, tip: &str) {
    ui.horizontal(|ui| {
        ui.label(label).on_hover_text(tip);
        ui.add(egui::TextEdit::singleline(v).hint_text(hint).desired_width(ui.available_width() - 4.0));
    });
}

pub fn densel(ui: &mut Ui, id: &str, d: &mut Densel, tg43: &mut bool) {
    ui.horizontal(|ui| {
        ui.label("Density select (pin 2)").on_hover_text("--densel: force the level of pin 2");
        egui::ComboBox::from_id_salt(id)
            .selected_text(match d {
                Densel::Auto => "Auto",
                Densel::High => "High",
                Densel::Low => "Low",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(d, Densel::Auto, "Auto");
                ui.selectable_value(d, Densel::High, "High");
                ui.selectable_value(d, Densel::Low, "Low");
            });
    });
    ui.checkbox(tg43, "Generate TG43 on pin 2 (8\" drives)");
}

pub fn tracks(ui: &mut Ui, t: &mut TrackOpts) {
    text_opt(
        ui,
        "Tracks",
        &mut t.custom,
        "format default — e.g. c=0-39:h=0",
        "TSPEC: c=SET cylinders, h=SET heads, step=N, hswap, h0.off=±N",
    );
    ui.horizontal_wrapped(|ui| {
        ui.checkbox(&mut t.double_step, "Double step")
            .on_hover_text("step=2: 40-track disk (C64, Apple II, 360K…) in an 80-track drive");
        ui.checkbox(&mut t.hswap, "Swap heads").on_hover_text("hswap: swap the physical heads");
    });
    if let Some(spec) = t.spec() {
        if let Err(e) = hgf_core::trackset::validate_tspec(&spec) {
            ui.label(RichText::new(format!("{}  {e}", ph::WARNING)).color(theme::BAD).small());
        }
    }
}

pub fn flux(ui: &mut Ui, f: &mut FluxOpts) {
    text_opt(
        ui,
        "Fake index",
        &mut f.fake_index,
        "e.g. 300rpm",
        "--fake-index SPEED: generate index pulses (no index hole)",
    );
    text_opt(ui, "Adjust speed", &mut f.adjust_speed, "e.g. 300rpm", "--adjust-speed SPEED: rescale flux to the effective drive speed (e.g. 360rpm drive reading 300rpm media)");
    text_opt(
        ui,
        "PLL",
        &mut f.pll,
        "period=5:phase=60",
        "--pll PLLSPEC: manual PLL override (period, phase, lowpass)",
    );
    ui.checkbox(&mut f.hard_sectors, "Hard-sectored disk");
    ui.checkbox(&mut f.reverse, "Reverse track data (flippy disk)");
}

/// Two-column key/value grid.
pub fn kv_grid(ui: &mut Ui, id: &str, rows: &[(String, String)]) {
    egui::Grid::new(id).num_columns(2).striped(true).spacing([18.0, 6.0]).show(ui, |ui| {
        for (k, v) in rows {
            ui.label(RichText::new(k).color(theme::MUTED));
            ui.label(RichText::new(v).strong());
            ui.end_row();
        }
    });
}

pub fn human_size(n: u64) -> String {
    if n >= 1024 * 1024 {
        format!("{:.2} MiB ({n} bytes)", n as f64 / 1_048_576.0)
    } else if n >= 1024 {
        format!("{:.1} KiB ({n} bytes)", n as f64 / 1024.0)
    } else {
        format!("{n} bytes")
    }
}

/// gw image file options (`disk.hfe::bitrate=250`) for `path`, edited with one control per
/// option. Shows nothing when the image type has none.
pub fn image_opts(
    ui: &mut Ui,
    id: &str,
    specs: &[hgf_core::imageopts::OptSpec],
    path: &Path,
    opts: &mut String,
) {
    use hgf_core::imageopts;
    if specs.is_empty() {
        if !opts.trim().is_empty() {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(
                        "{}  File options \"{opts}\" do not apply to this image type.",
                        ph::WARNING
                    ))
                    .small()
                    .color(theme::WARN),
                );
                if ui.small_button("Clear").clicked() {
                    opts.clear();
                }
            });
        }
        return;
    }
    let ext = formats::extension(path).unwrap_or_default().to_uppercase();
    egui::CollapsingHeader::new(RichText::new(format!("{ext} file options")).small())
        .id_salt(("imgopts", id))
        .default_open(!opts.trim().is_empty())
        .show(ui, |ui| {
            egui::Grid::new(("imgopts_grid", id)).num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                for s in specs {
                    let current = imageopts::get(opts, s.key);
                    ui.label(s.key).on_hover_text(s.help);
                    if s.flag {
                        let mut on = current.is_some();
                        if ui.checkbox(&mut on, "").on_hover_text(s.help).changed() {
                            imageopts::set(opts, s.key, if on { "yes" } else { "" });
                        }
                    } else if s.values.is_empty() {
                        let mut v = current.unwrap_or_default();
                        if ui
                            .add(
                                egui::TextEdit::singleline(&mut v)
                                    .hint_text("default")
                                    .desired_width(theme::w(90.0)),
                            )
                            .on_hover_text(s.help)
                            .changed()
                        {
                            imageopts::set(opts, s.key, v.trim());
                        }
                    } else {
                        let shown = current.clone().unwrap_or_else(|| "default".into());
                        egui::ComboBox::from_id_salt(("imgopt", id, s.key))
                            .selected_text(shown)
                            .height(320.0)
                            .show_ui(ui, |ui| {
                                if ui.selectable_label(current.is_none(), "default").clicked() {
                                    imageopts::set(opts, s.key, "");
                                }
                                for v in s.values {
                                    let sel = current.as_deref().is_some_and(|c| c.eq_ignore_ascii_case(v));
                                    if ui.selectable_label(sel, *v).clicked() {
                                        imageopts::set(opts, s.key, v);
                                    }
                                }
                            })
                            .response
                            .on_hover_text(s.help);
                    }
                    ui.end_row();
                }
            });
            if let Err(e) = imageopts::validate(specs, path, opts) {
                ui.label(RichText::new(format!("{}  {e}", ph::WARNING)).small().color(theme::BAD));
            } else if !opts.is_empty() {
                ui.label(
                    RichText::new(format!(
                        "gw argument: {}::{opts}",
                        path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
                    ))
                    .small()
                    .monospace()
                    .color(theme::MUTED),
                );
            }
        });
}

/// The gw command that the button below will run, with a copy button.
pub fn command_preview(ui: &mut Ui, line: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(ph::TERMINAL).color(theme::MUTED)).on_hover_text("Command that will be run");
        if ui.small_button(ph::COPY).on_hover_text("Copy the command").clicked() {
            ui.ctx().copy_text(line.to_string());
        }
        ui.add(egui::Label::new(RichText::new(line).small().monospace().color(theme::MUTED)).wrap());
    });
}

/// Open a folder in the file manager (Explorer, Finder, Files…).
pub fn open_folder(dir: &Path) -> Result<(), String> {
    let program = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    std::process::Command::new(program)
        .arg(dir)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Cannot open {}: {e}", dir.display()))
}

/// Button opening the folder of `path` (when it exists). Returns an error to show.
pub fn folder_button(ui: &mut Ui, path: &Path) -> Option<String> {
    let dir = if path.is_dir() {
        Some(path.to_path_buf())
    } else {
        path.parent().filter(|p| p.is_dir()).map(Path::to_path_buf)
    };
    let r = ui
        .add_enabled(dir.is_some(), egui::Button::new(ph::FOLDER_SIMPLE).small())
        .on_hover_text("Open the folder");
    match (r.clicked(), dir) {
        (true, Some(d)) => open_folder(&d).err(),
        _ => None,
    }
}
