//! Saved settings ("templates"): named sets of Read or Write options, with a description.
//!
//! A template keeps everything about how to read or write a kind of disk (format, profile,
//! retries, tracks, flux options, destination and numbering for reads), not the image to write.

use crate::app::App;
use crate::theme;
use eframe::egui::{self, RichText, Ui};
use egui_phosphor::regular as ph;
use hgf_core::gw::{ReadOpts, WriteOpts};
use hgf_core::naming::Numbering;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TemplateKind {
    #[default]
    Read,
    Write,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Template {
    pub name: String,
    pub description: String,
    pub kind: TemplateKind,
    pub read: ReadOpts,
    pub naming: Numbering,
    pub write: WriteOpts,
    /// Custom TSPEC: not kept by `TrackOpts` itself (a forgotten offset must not come back on
    /// its own), but part of a template on purpose (e.g. `c=0-34` for 35-track C64 disks).
    pub tracks_custom: String,
}

/// State of the "Save as…" form (not persisted).
#[derive(Default)]
pub struct TemplateEdit {
    pub kind: Option<TemplateKind>,
    pub name: String,
    pub description: String,
}

impl App {
    /// The "Saved settings" card at the top of the Read and Write pages.
    pub fn templates_card(&mut self, ui: &mut Ui, kind: TemplateKind) {
        let color = match kind {
            TemplateKind::Read => theme::READ,
            TemplateKind::Write => theme::WRITE,
        };
        let names: Vec<(String, String)> = self
            .s
            .templates
            .iter()
            .filter(|t| t.kind == kind)
            .map(|t| (t.name.clone(), t.description.clone()))
            .collect();
        let selected = self.template_selected(kind).clone();
        crate::widgets::card(ui, ph::BOOKMARKS_SIMPLE, "Saved settings", color, |ui| {
            ui.horizontal(|ui| {
                let text = selected.clone().unwrap_or_else(|| {
                    if names.is_empty() {
                        "none saved yet".into()
                    } else {
                        "choose…".into()
                    }
                });
                let mut load = None;
                ui.add_enabled_ui(!names.is_empty(), |ui| {
                    egui::ComboBox::from_id_salt(("templates", kind as u8))
                        .selected_text(text)
                        .width(theme::w(170.0))
                        .show_ui(ui, |ui| {
                            for (n, d) in &names {
                                let r = ui.selectable_label(selected.as_deref() == Some(n), n);
                                let r = if d.is_empty() { r } else { r.on_hover_text(d) };
                                if r.clicked() {
                                    load = Some(n.clone());
                                }
                            }
                        });
                });
                if let Some(n) = load {
                    self.load_template(kind, &n);
                }
                if ui
                    .button(format!("{}  Save…", ph::FLOPPY_DISK))
                    .on_hover_text("Save the current settings under a name")
                    .clicked()
                {
                    self.template_edit = TemplateEdit {
                        kind: Some(kind),
                        name: selected.clone().unwrap_or_default(),
                        description: selected
                            .as_ref()
                            .and_then(|n| names.iter().find(|(x, _)| x == n))
                            .map(|(_, d)| d.clone())
                            .unwrap_or_default(),
                    };
                }
                if ui
                    .add_enabled(selected.is_some(), egui::Button::new(ph::TRASH))
                    .on_hover_text("Delete this template")
                    .clicked()
                {
                    if let Some(n) = &selected {
                        self.s.templates.retain(|t| !(t.kind == kind && &t.name == n));
                    }
                    *self.template_selected_mut(kind) = None;
                }
            });
            if let Some(d) =
                selected.as_ref().and_then(|n| names.iter().find(|(x, _)| x == n)).map(|(_, d)| d)
            {
                let modified = selected.as_deref().is_some_and(|n| !self.matches_template(kind, n));
                let mut text = d.clone();
                if modified {
                    text = if text.is_empty() {
                        "Modified since loaded".into()
                    } else {
                        format!("{text}  ·  modified")
                    };
                }
                if !text.is_empty() {
                    ui.label(RichText::new(text).small().color(if modified {
                        theme::WARN
                    } else {
                        theme::MUTED
                    }));
                }
            }
            if self.template_edit.kind == Some(kind) {
                ui.add_space(4.0);
                let e = &mut self.template_edit;
                egui::Grid::new(("tpl_edit", kind as u8)).num_columns(2).show(ui, |ui| {
                    ui.label("Name");
                    ui.add(
                        egui::TextEdit::singleline(&mut e.name)
                            .char_limit(40)
                            .hint_text("e.g. C64 35 tracks"),
                    );
                    ui.end_row();
                    ui.label("Description");
                    ui.add(
                        egui::TextEdit::singleline(&mut e.description).char_limit(120).hint_text("optional"),
                    );
                    ui.end_row();
                });
                let name = e.name.trim().to_string();
                let exists = names.iter().any(|(n, _)| *n == name);
                ui.horizontal(|ui| {
                    let label = if exists { "Replace" } else { "Save" };
                    if ui
                        .add_enabled(!name.is_empty(), egui::Button::new(format!("{}  {label}", ph::CHECK)))
                        .clicked()
                    {
                        let description = self.template_edit.description.trim().to_string();
                        self.save_template(kind, name.clone(), description);
                        self.template_edit = TemplateEdit::default();
                    }
                    if ui.button(format!("{}  Cancel", ph::X)).clicked() {
                        self.template_edit = TemplateEdit::default();
                    }
                });
            }
        });
        ui.add_space(8.0);
    }

    /// Do the current settings equal this template?
    fn matches_template(&self, kind: TemplateKind, name: &str) -> bool {
        let Some(t) = self.s.templates.iter().find(|t| t.kind == kind && t.name == name) else {
            return false;
        };
        match kind {
            TemplateKind::Read => {
                let mut r = t.read.clone();
                r.tracks.custom = t.tracks_custom.clone();
                r == self.s.read && t.naming == self.s.read_naming
            }
            TemplateKind::Write => {
                let mut w = t.write.clone();
                w.tracks.custom = t.tracks_custom.clone();
                w.input = self.s.write.input.clone();
                w.file_opts = self.s.write.file_opts.clone();
                w == self.s.write
            }
        }
    }

    fn template_selected(&self, kind: TemplateKind) -> &Option<String> {
        &self.template_selected[kind as usize]
    }

    fn template_selected_mut(&mut self, kind: TemplateKind) -> &mut Option<String> {
        &mut self.template_selected[kind as usize]
    }

    fn save_template(&mut self, kind: TemplateKind, name: String, description: String) {
        let t = match kind {
            TemplateKind::Read => Template {
                read: self.s.read.clone(),
                naming: self.s.read_naming.clone(),
                tracks_custom: self.s.read.tracks.custom.clone(),
                ..Default::default()
            },
            TemplateKind::Write => {
                let mut w = self.s.write.clone();
                w.input = Default::default();
                w.file_opts.clear();
                Template { write: w, tracks_custom: self.s.write.tracks.custom.clone(), ..Default::default() }
            }
        };
        let t = Template { name: name.clone(), description, kind, ..t };
        match self.s.templates.iter_mut().find(|x| x.kind == kind && x.name == name) {
            Some(x) => *x = t,
            None => self.s.templates.push(t),
        }
        self.s.templates.sort_by_key(|t| t.name.to_lowercase());
        *self.template_selected_mut(kind) = Some(name);
    }

    fn load_template(&mut self, kind: TemplateKind, name: &str) {
        let Some(t) = self.s.templates.iter().find(|t| t.kind == kind && t.name == name).cloned() else {
            return;
        };
        match kind {
            TemplateKind::Read => {
                self.s.read = t.read;
                self.s.read.tracks.custom = t.tracks_custom;
                self.s.read_naming = t.naming;
            }
            TemplateKind::Write => {
                let input = std::mem::take(&mut self.s.write.input);
                let file_opts = std::mem::take(&mut self.s.write.file_opts);
                self.s.write = t.write;
                self.s.write.input = input;
                self.s.write.file_opts = file_opts;
                self.s.write.tracks.custom = t.tracks_custom;
            }
        }
        *self.template_selected_mut(kind) = Some(name.to_string());
    }
}
