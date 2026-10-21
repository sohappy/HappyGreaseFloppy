//! Application state, job lifecycle and the window shell (top bar, sidebar, console, dialogs).

use crate::{theme, visualizer, widgets};
use eframe::egui::{self, Color32, RichText, Ui};
use egui_phosphor::regular as ph;
use hgf_core::formats::{self, Catalog};
use hgf_core::gw::*;
use hgf_core::health::ScanStats;
use hgf_core::hints::{self, Hint, HintAction};
use hgf_core::image::ImageReport;
use hgf_core::parse::{Output, TrackOutcome};
use hgf_core::runner::{self, JobEvent};
use hgf_core::{DiskMap, Job, MapMode, OutputParser, TrackSet, TrackState};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

const CONSOLE_CAP: usize = 10_000;

/// Application icon, raw RGBA (also used as the window icon); rendered from packaging/icon.svg.
pub const LOGO_RGBA: &[u8] = include_bytes!("../assets/icon128.rgba");
pub const LOGO_SIZE: u32 = 128;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Page {
    #[default]
    Read,
    Write,
    Copy,
    Format,
    Convert,
    Erase,
    Health,
    Rpm,
    Align,
    Seek,
    Clean,
    Info,
    Delays,
    Pins,
    Bandwidth,
    Reset,
    Update,
    Inspector,
    Settings,
}

impl Page {
    pub const GROUPS: &[(&str, &[Page])] = &[
        ("DISK", &[Page::Read, Page::Write, Page::Copy, Page::Format, Page::Convert, Page::Erase]),
        ("DRIVE", &[Page::Health, Page::Rpm, Page::Align, Page::Seek, Page::Clean]),
        ("DEVICE", &[Page::Info, Page::Delays, Page::Pins, Page::Bandwidth, Page::Reset, Page::Update]),
        ("IMAGES", &[Page::Inspector]),
        ("APP", &[Page::Settings]),
    ];

    pub fn icon(self) -> &'static str {
        match self {
            Page::Read => ph::DOWNLOAD_SIMPLE,
            Page::Copy => ph::COPY,
            Page::Format => ph::SPARKLE,
            Page::Write => ph::UPLOAD_SIMPLE,
            Page::Convert => ph::SWAP,
            Page::Erase => ph::ERASER,
            Page::Health => ph::STETHOSCOPE,
            Page::Rpm => ph::GAUGE,
            Page::Align => ph::CROSSHAIR,
            Page::Seek => ph::ARROWS_DOWN_UP,
            Page::Clean => ph::BROOM,
            Page::Info => ph::INFO,
            Page::Delays => ph::TIMER,
            Page::Pins => ph::PLUGS,
            Page::Bandwidth => ph::LIGHTNING,
            Page::Reset => ph::ARROW_COUNTER_CLOCKWISE,
            Page::Update => ph::CLOUD_ARROW_DOWN,
            Page::Inspector => ph::FILE_MAGNIFYING_GLASS,
            Page::Settings => ph::GEAR,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Page::Read => "Read disk",
            Page::Copy => "Copy disk",
            Page::Format => "Format disk",
            Page::Write => "Write disk",
            Page::Convert => "Convert image",
            Page::Erase => "Erase disk",
            Page::Health => "Drive health",
            Page::Rpm => "Drive RPM",
            Page::Align => "Head alignment",
            Page::Seek => "Seek head",
            Page::Clean => "Clean drive",
            Page::Info => "Device info",
            Page::Delays => "Drive delays",
            Page::Pins => "Interface pins",
            Page::Bandwidth => "USB bandwidth",
            Page::Reset => "Reset device",
            Page::Update => "Firmware update",
            Page::Inspector => "Image inspector",
            Page::Settings => "Settings",
        }
    }

    pub fn color(self) -> Color32 {
        match self {
            Page::Read => theme::READ,
            Page::Copy => theme::COPY,
            Page::Format => theme::FORMAT,
            Page::Write => theme::WRITE,
            Page::Convert => theme::CONVERT,
            Page::Erase => theme::ERASE,
            Page::Health | Page::Rpm | Page::Align | Page::Seek | Page::Clean => theme::DRIVE,
            Page::Inspector => theme::IMAGE,
            _ => theme::DEVICE,
        }
    }

    pub fn for_kind(k: ActionKind) -> Page {
        match k {
            ActionKind::Info => Page::Info,
            ActionKind::Read => Page::Read,
            ActionKind::Write => Page::Write,
            ActionKind::Convert => Page::Convert,
            ActionKind::Erase => Page::Erase,
            ActionKind::Clean => Page::Clean,
            ActionKind::Seek => Page::Seek,
            ActionKind::Delays => Page::Delays,
            ActionKind::Update => Page::Update,
            ActionKind::PinGet | ActionKind::PinSet => Page::Pins,
            ActionKind::Reset => Page::Reset,
            ActionKind::Bandwidth => Page::Bandwidth,
            ActionKind::Rpm => Page::Rpm,
            ActionKind::Align => Page::Align,
        }
    }
}

/// Everything remembered between sessions.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub cfg: GwConfig,
    pub drive: Drive,
    pub page: Page,
    pub dark: bool,
    /// Follow the desktop's light/dark preference (overrides `dark`).
    pub follow_system: bool,
    /// Text size, 1.0 = 100 %.
    pub text_scale: f32,
    pub show_console: bool,
    pub probe_on_start: bool,
    pub last_dir: Option<PathBuf>,
    pub read: ReadOpts,
    pub write: WriteOpts,
    pub convert: ConvertOpts,
    pub erase: EraseOpts,
    pub clean: CleanOpts,
    pub seek: SeekOpts,
    pub rpm_nr: u32,
    pub align: AlignOpts,
    pub update: UpdateOpts,
    pub pin: u32,
    pub pin_high: bool,
    pub reset_delays: bool,
    pub info_bootloader: bool,
    pub delays_edit: DelaysOpts,
    pub health_format: String,
    pub health_write: bool,
    pub format: hgf_core::amigados::FormatOpts,
    /// Machine chosen on the Format page.
    pub format_machine: crate::format_ui::Machine,
    pub format_fat: hgf_core::fat12::FatOpts,
    pub format_cbm: hgf_core::cbmdos::CbmOpts,
    pub format_dfs: hgf_core::acorndfs::DfsOpts,
    pub format_apple: hgf_core::apple2::AppleOpts,
    /// Double step for 40-track formats in an 80-track 5.25" drive.
    pub format_double_step: bool,
    /// Automatic numbering of read images (`disk_001.adf`…).
    pub read_naming: hgf_core::naming::Numbering,
    /// Saved Read / Write settings.
    pub templates: Vec<crate::templates::Template>,
    /// Folder of the automatic log files; empty = no log file.
    pub log_dir: String,
    /// Ask for attention (taskbar / Dock) when an operation ends while the window is in the background.
    pub notify_done: bool,
    pub copy: crate::copy_ui::CopySettings,
}

impl Settings {
    pub fn theme_choice(&self) -> theme::ThemeChoice {
        theme::ThemeChoice::from_settings(self.dark, self.follow_system)
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            cfg: GwConfig::default(),
            drive: Drive::A,
            page: Page::Read,
            dark: true,
            follow_system: false,
            text_scale: 1.0,
            show_console: true,
            probe_on_start: true,
            last_dir: None,
            read: ReadOpts {
                output: home_dir().join("disk.adf"),
                format: "amiga.amigados".into(),
                ..Default::default()
            },
            write: WriteOpts::default(),
            convert: ConvertOpts::default(),
            erase: EraseOpts::default(),
            clean: CleanOpts::default(),
            seek: SeekOpts::default(),
            rpm_nr: 10,
            align: AlignOpts::default(),
            update: UpdateOpts::default(),
            pin: 2,
            pin_high: true,
            reset_delays: false,
            info_bootloader: false,
            delays_edit: DelaysOpts::default(),
            health_format: "amiga.amigados".into(),
            health_write: false,
            format: Default::default(),
            format_machine: Default::default(),
            format_fat: Default::default(),
            format_cbm: Default::default(),
            format_dfs: Default::default(),
            format_apple: Default::default(),
            format_double_step: false,
            read_naming: Default::default(),
            templates: Vec::new(),
            log_dir: String::new(),
            notify_done: true,
            copy: Default::default(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Command,
    Info,
    Good,
    Warn,
    Error,
}

impl Level {
    fn color(self, dark: bool) -> Color32 {
        match self {
            Level::Command => theme::readable(theme::ACCENT, dark),
            Level::Info => {
                if dark {
                    Color32::from_gray(200)
                } else {
                    Color32::from_gray(40)
                }
            }
            Level::Good => theme::readable(theme::OK, dark),
            Level::Warn => theme::readable(theme::WARN, dark),
            Level::Error => theme::readable(theme::BAD, dark),
        }
    }
}

#[derive(Default)]
pub struct Console {
    lines: VecDeque<(String, Level)>,
    partial: Option<String>,
    errors_only: bool,
    /// Lines of the running job, kept for the log file.
    job_log: Option<Vec<String>>,
}

impl Console {
    pub fn push(&mut self, text: impl Into<String>, level: Level) {
        if self.lines.len() >= CONSOLE_CAP {
            self.lines.pop_front();
        }
        let text = text.into();
        if let Some(log) = &mut self.job_log {
            log.push(text.clone());
        }
        self.lines.push_back((text, level));
    }

    fn text(&self) -> String {
        self.lines.iter().map(|(l, _)| l.as_str()).collect::<Vec<_>>().join("\n")
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DeviceState {
    Unknown,
    Probing,
    Connected,
    NotFound,
    NoTools,
}

pub struct Device {
    pub state: DeviceState,
    pub fields: Vec<(String, String)>,
    pub firmware_update: Option<String>,
}

struct Running {
    job: Job,
    kind: ActionKind,
    page: Page,
    reread: bool,
    success_msg: Option<String>,
    action: Action,
    started: Instant,
    temp: Option<PathBuf>,
    output: Option<PathBuf>,
    failure: Option<String>,
    skipped: Option<String>,
    numbered: bool,
}

pub struct Outcome {
    pub page: Page,
    pub level: Level,
    pub message: String,
    pub hint: Option<Hint>,
}

/// Per-job settings that are not part of the gw action itself.
#[derive(Clone, Debug, Default)]
pub struct JobExtras {
    /// Drive to use instead of the one selected in the top bar.
    pub drive: Option<Drive>,
    /// Temporary file to delete when the job ends.
    pub temp: Option<PathBuf>,
    /// Page the job reports to.
    pub page: Option<Page>,
    /// Re-read of the bad tracks of the current map.
    pub reread: bool,
    /// Confirmation text replacing the action's own.
    pub danger: Option<String>,
    /// How to name the image in the confirmation (instead of its temporary path).
    pub describe: Option<String>,
    /// Message shown when the job succeeds.
    pub success: Option<String>,
    /// A numbered read: advance the disk number when it succeeds.
    pub numbered: bool,
}

pub struct Confirm {
    pub action: Action,
    pub danger: Option<String>,
    pub overwrite: Option<PathBuf>,
    pub warnings: Vec<String>,
    pub extras: JobExtras,
}

#[derive(Default)]
pub struct Rpm {
    pub samples: Vec<f64>,
    pub stats: Vec<(String, f64)>,
}

pub struct App {
    pub s: Settings,
    pub template_edit: crate::templates::TemplateEdit,
    /// Template shown as selected on the Read and Write pages.
    pub template_selected: [Option<String>; 2],
    /// Serial ports found by the last refresh (Settings → Device).
    pub ports: Option<Vec<hgf_core::ports::Port>>,
    /// An operation ended: ask the desktop for attention on the next frame.
    attention: bool,
    pub catalog: Catalog,
    catalog_rx: Option<Receiver<Result<Catalog, String>>>,
    detect_rx: Option<Receiver<Option<String>>>,
    pub detecting: bool,
    pending_probe: bool,
    pub health: crate::health_ui::HealthWizard,
    pub copy: crate::copy_ui::CopyWizard,
    /// Extras for the next `request`.
    pub next: JobExtras,
    /// Last plain read (options, image saved?), used to re-read its bad tracks.
    last_read: Option<(ReadOpts, bool)>,
    reread_before: usize,
    pub catalog_error: Option<String>,
    running: Option<Running>,
    pub console: Console,
    parser: OutputParser,
    pub map: DiskMap,
    pub device: Device,
    pub rpm: Rpm,
    pub align: VecDeque<(u8, u32, u32)>,
    pub delays: Vec<(String, String)>,
    pub bandwidth: Vec<(String, f64, f64, f64)>,
    pub bandwidth_notes: Vec<String>,
    pub pin_result: Option<String>,
    pub inspector: Option<(PathBuf, Result<ImageReport, String>)>,
    pub inspect_path: PathBuf,
    /// AmigaDOS analysis of the Write page's source image, keyed by path, size and mtime.
    pub write_adf: Option<WriteAdf>,
    pub confirm: Option<Confirm>,
    pub outcome: Option<Outcome>,
    pub notice: Option<String>,
    scan_counter: u32,
    logo: egui::TextureHandle,
    /// Page whose operation filled `map`.
    map_page: Page,
    /// The user tried to close the window during an operation.
    close_requested: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::install_fonts(&cc.egui_ctx);
        let s: Settings =
            cc.storage.and_then(|st| eframe::get_value(st, eframe::APP_KEY)).unwrap_or_default();
        let mut s = s;
        // HGF_GW overrides the gw executable (handy for portable installs and testing).
        if let Ok(p) = std::env::var("HGF_GW") {
            s.cfg.program = p;
        }
        theme::apply(&cc.egui_ctx, s.theme_choice(), s.text_scale);
        let mut app = Self {
            s,
            catalog: Catalog::default(),
            catalog_rx: None,
            detect_rx: None,
            detecting: false,
            pending_probe: false,
            health: Default::default(),
            copy: Default::default(),
            next: JobExtras::default(),
            last_read: None,
            reread_before: 0,
            catalog_error: None,
            running: None,
            console: Console::default(),
            parser: OutputParser::new(),
            map: DiskMap::default(),
            device: Device { state: DeviceState::Unknown, fields: Vec::new(), firmware_update: None },
            rpm: Rpm::default(),
            align: VecDeque::new(),
            delays: Vec::new(),
            bandwidth: Vec::new(),
            bandwidth_notes: Vec::new(),
            pin_result: None,
            inspector: None,
            inspect_path: PathBuf::new(),
            write_adf: None,
            template_edit: Default::default(),
            template_selected: [None, None],
            ports: None,
            attention: false,
            confirm: None,
            outcome: None,
            notice: None,
            scan_counter: 0,
            logo: cc.egui_ctx.load_texture(
                "logo",
                egui::ColorImage::from_rgba_unmultiplied([LOGO_SIZE as usize; 2], LOGO_RGBA),
                egui::TextureOptions::LINEAR,
            ),
            map_page: Page::Read,
            close_requested: false,
        };
        app.console.push(format!("HappyGreaseFloppy {} ready.", hgf_core::VERSION), Level::Good);
        remove_stale_temps();
        app.detect_gw(app.s.probe_on_start);
        app
    }

    pub fn busy(&self) -> bool {
        self.running.is_some()
    }

    pub fn running_kind(&self) -> Option<ActionKind> {
        self.running.as_ref().map(|r| r.kind)
    }

    /// Find a working gw (configured path first, then usual locations) in the background,
    /// then load the format list and optionally probe the device.
    pub fn detect_gw(&mut self, probe_after: bool) {
        let preferred = self.s.cfg.program.clone();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(hgf_core::locate::detect(&preferred));
        });
        self.detect_rx = Some(rx);
        self.detecting = true;
        self.device.state = DeviceState::Probing;
        self.pending_probe = probe_after;
    }

    /// Load the format list from the installed gw in the background.
    pub fn reload_catalog(&mut self) {
        let program = self.s.cfg.program.clone();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let r = runner::capture(&program, &["read", "--help"], Duration::from_secs(20))
                .and_then(|text| formats::parse_help(&text).ok_or_else(|| "unexpected gw output".into()));
            let _ = tx.send(r);
        });
        self.catalog_rx = Some(rx);
    }

    pub fn probe(&mut self) {
        let bootloader = self.s.info_bootloader;
        self.request(Action::Info { bootloader });
    }

    /// Validate and, if needed, ask for confirmation before running an action.
    pub fn request(&mut self, mut action: Action) {
        let extras = std::mem::take(&mut self.next);
        if self.busy() || self.detecting {
            if self.detecting {
                self.notice =
                    Some("Still looking for the Greaseweazle tools (gw): try again in a moment.".into());
            }
            remove_temp(&extras);
            return;
        }
        self.notice = None;
        if let Action::Read(o) = &mut action {
            if o.profile == ReadProfile::QuickScan {
                self.scan_counter += 1;
                o.output = std::env::temp_dir().join(format!(
                    "hgf-scan-{}-{}.scp",
                    std::process::id(),
                    self.scan_counter
                ));
            }
        }
        let warnings = match action.validate(&self.catalog) {
            Ok(w) => w,
            Err(e) => {
                self.notice = Some(e);
                remove_temp(&extras);
                return;
            }
        };
        let overwrite = action.output_path().filter(|p| p.exists()).map(PathBuf::from);
        let danger = extras.danger.clone().or_else(|| action.danger());
        if overwrite.is_some() || danger.is_some() {
            self.confirm = Some(Confirm { action, danger, overwrite, warnings, extras });
        } else {
            for w in warnings {
                self.console.push(format!("WARNING: {w}"), Level::Warn);
            }
            self.start(action, false, extras);
        }
    }

    fn start(&mut self, mut action: Action, overwrite_confirmed: bool, extras: JobExtras) {
        // Never let gw overwrite a file the user did not explicitly accept.
        action.set_no_clobber(!overwrite_confirmed);
        let kind = action.kind();
        let drive = extras.drive.unwrap_or(self.s.drive);
        let page = extras.page.unwrap_or_else(|| self.job_page(kind));
        let args = action.args(&self.s.cfg, drive);
        let cmdline = action.command_line(&self.s.cfg, drive);
        if let Action::Read(o) = &action {
            if !extras.reread && self.health.pending.is_none() && self.copy.pending.is_none() {
                self.last_read = Some((o.clone(), o.profile != ReadProfile::QuickScan));
            }
        }
        let (temp, output) = match &action {
            _ if extras.temp.is_some() => (extras.temp.clone(), None),
            Action::Read(o) if o.profile == ReadProfile::QuickScan => (Some(o.output.clone()), None),
            Action::Read(o) => (None, Some(o.output.clone())),
            Action::Convert(o) => (None, Some(o.output.clone())),
            _ => (None, None),
        };
        if extras.reread {
            self.map_page = page;
        } else if kind.uses_disk_map() {
            let (mode, fmt) = match &action {
                Action::Write(o) => (MapMode::Write, o.format.clone()),
                Action::Erase(_) => (MapMode::Erase, String::new()),
                Action::Read(o) => (MapMode::Read, o.format.clone()),
                Action::Convert(o) => (MapMode::Read, o.format.clone()),
                _ => (MapMode::Read, String::new()),
            };
            let geo = formats::PRESETS
                .iter()
                .find(|p| !fmt.is_empty() && p.format == fmt)
                .map(|p| TrackSet::dense(p.cyls, p.heads))
                .unwrap_or_else(|| TrackSet::dense(80, 2));
            self.map = DiskMap::new(mode, geo);
            self.map_page = page;
        }
        match kind {
            ActionKind::Info => {
                self.device.state = DeviceState::Probing;
                self.device.fields.clear();
                self.device.firmware_update = None;
            }
            ActionKind::Rpm => self.rpm = Rpm::default(),
            ActionKind::Align => self.align.clear(),
            ActionKind::Delays => self.delays.clear(),
            ActionKind::Bandwidth => {
                self.bandwidth.clear();
                self.bandwidth_notes.clear();
            }
            ActionKind::PinGet | ActionKind::PinSet => self.pin_result = None,
            _ => {}
        }
        self.parser = OutputParser::new();
        self.outcome = None;
        self.console.partial = None;
        self.console.job_log = (!self.s.log_dir.trim().is_empty()).then(Vec::new);
        self.console.push(format!("$ {cmdline}"), Level::Command);
        match runner::spawn(&self.s.cfg.program, &args) {
            Ok(job) => {
                self.running = Some(Running {
                    job,
                    kind,
                    page,
                    reread: extras.reread,
                    success_msg: extras.success.clone(),
                    action: action.clone(),
                    started: Instant::now(),
                    temp,
                    output,
                    failure: None,
                    skipped: None,
                    numbered: extras.numbered,
                });
            }
            Err(e) => {
                if kind == ActionKind::Info || e.contains("not found") {
                    self.device.state = DeviceState::NoTools;
                }
                self.console.push(e.clone(), Level::Error);
                remove_temp(&extras);
                let hint = hints::for_failure(kind, &e);
                self.outcome = Some(Outcome { page, level: Level::Error, message: e, hint });
            }
        }
    }

    pub fn cancel(&mut self) {
        if let Some(r) = &self.running {
            r.job.cancel();
            self.console.push("Stopping… (the drive is released cleanly)", Level::Warn);
        }
    }

    /// Drain pending output; returns true if something changed.
    fn poll(&mut self) -> bool {
        if let Some(rx) = &self.detect_rx {
            if let Ok(found) = rx.try_recv() {
                self.detect_rx = None;
                self.detecting = false;
                match found {
                    Some(p) => {
                        if p != self.s.cfg.program {
                            self.console.push(format!("Greaseweazle host tools found: {p}"), Level::Good);
                            self.s.cfg.program = p;
                        }
                        self.reload_catalog();
                        if std::mem::take(&mut self.pending_probe) {
                            self.probe();
                        } else {
                            self.device.state = DeviceState::Unknown;
                        }
                    }
                    None => {
                        self.pending_probe = false;
                        self.device.state = DeviceState::NoTools;
                        let msg = "Greaseweazle host tools (gw) not found. Install them                                    (Windows: unzip the official release next to this program;                                    Linux/macOS: pip install greaseweazle), or set the path in Settings.";
                        self.console.push(msg, Level::Error);
                        self.notice = Some(msg.into());
                    }
                }
            }
        }
        if let Some(rx) = &self.catalog_rx {
            if let Ok(r) = rx.try_recv() {
                self.catalog_rx = None;
                match r {
                    Ok(c) => {
                        self.catalog = c;
                        self.catalog_error = None;
                    }
                    Err(e) => self.catalog_error = Some(e),
                }
            }
        }
        let mut events = Vec::new();
        if let Some(r) = &self.running {
            while let Some(e) = r.job.try_next() {
                events.push(e);
                if events.len() > 2000 {
                    break;
                }
            }
        }
        let changed = !events.is_empty();
        for e in events {
            match e {
                JobEvent::Line(l) => self.on_line(l),
                JobEvent::Partial(p) => self.console.partial = Some(p),
                JobEvent::Finished { code, cancelled } => self.on_finished(code, cancelled),
            }
        }
        changed
    }

    fn on_line(&mut self, line: String) {
        self.console.partial = None;
        let Some(kind) = self.running_kind() else { return };
        let fatal_message = self.parser.is_fatal_message();
        let out = self.parser.feed(&line);
        let mut level = if out.is_error() || fatal_message {
            Level::Error
        } else if out.is_warning() {
            Level::Warn
        } else {
            Level::Info
        };
        if line.starts_with("*** TEST/PRE-RELEASE") || line.starts_with("*** Use these tools") {
            level = Level::Warn;
        }
        let failure = match &out {
            _ if fatal_message => Some(line.trim().to_string()),
            Output::CommandFailed(m) => Some(m.clone()),
            _ => None,
        };
        if let (Some(f), Some(r)) = (failure, self.running.as_mut()) {
            r.failure.get_or_insert(f);
        }
        match &out {
            Output::DeviceNotFound => self.device.state = DeviceState::NotFound,
            Output::KeyValue { key, value } => match kind {
                ActionKind::Info => {
                    if key == "Port" || key == "Model" {
                        self.device.state = DeviceState::Connected;
                    }
                    self.device.fields.push((key.clone(), value.clone()));
                }
                ActionKind::Delays => self.delays.push((key.clone(), value.clone())),
                ActionKind::Bandwidth => self.bandwidth_notes.push(format!("{key}: {value}")),
                _ => {}
            },
            Output::FirmwareUpdate(v) => self.device.firmware_update = Some(v.clone()),
            Output::Rpm { rpm, .. } => self.rpm.samples.push(*rpm),
            Output::RpmStat { label, rpm } => self.rpm.stats.push((label.clone(), *rpm)),
            Output::Bandwidth { direction, min, mean, max } => {
                self.bandwidth.push((direction.clone(), *min, *mean, *max));
            }
            Output::Warning(w) | Output::CommandFailed(w) if kind == ActionKind::Bandwidth => {
                self.bandwidth_notes.push(w.clone())
            }
            Output::UpdateSkipped(m) => {
                if let Some(r) = self.running.as_mut() {
                    r.skipped = Some(m.clone());
                }
            }
            Output::Pin { pin, level: l } => self.pin_result = Some(format!("Pin {pin} is {l}")),
            Output::Track { head, outcome: TrackOutcome::Decoded { ok, total, .. }, .. }
                if kind == ActionKind::Align =>
            {
                if self.align.len() >= 400 {
                    self.align.pop_front();
                }
                self.align.push_back((*head, *ok, *total));
            }
            _ => {}
        }
        if kind == ActionKind::Bandwidth && line.trim_start().starts_with("->") {
            self.bandwidth_notes.push(line.trim().trim_start_matches("->").trim().to_string());
        }
        if kind.uses_disk_map() {
            self.map.apply(&out);
        }
        if level == Level::Info && matches!(out, Output::AllVerified | Output::Found { .. }) {
            level = Level::Good;
        }
        self.console.push(line, level);
    }

    fn on_finished(&mut self, code: Option<i32>, cancelled: bool) {
        let Some(r) = self.running.take() else { return };
        if let Some(p) = self.console.partial.take() {
            self.console.push(p, Level::Info);
        }
        let success = code == Some(0) && r.failure.is_none() && !cancelled;
        if r.kind.uses_disk_map() {
            self.map.finish(success);
        }
        if let Some(t) = &r.temp {
            let _ = std::fs::remove_file(t);
        }
        match r.kind {
            ActionKind::Info => {
                if self.device.state == DeviceState::Probing {
                    self.device.state = DeviceState::NotFound;
                }
            }
            _ if self.device.state == DeviceState::NotFound => {}
            _ if success => self.device.state = DeviceState::Connected,
            _ => {}
        }
        let secs = r.started.elapsed().as_secs_f32();
        let failure_text = r.failure.clone();
        let (level, message) = if cancelled {
            (Level::Warn, "Stopped by user.".to_string())
        } else if let Some(f) = r.failure {
            (Level::Error, f)
        } else if code != Some(0) {
            (
                Level::Error,
                match code {
                    Some(c) => format!("gw exited with status {c}."),
                    None => "gw was terminated.".into(),
                },
            )
        } else if let Some(m) = r.skipped {
            (Level::Warn, format!("Update skipped: {m}"))
        } else if let Some(m) = r.success_msg.clone() {
            (Level::Good, m)
        } else if r.reread {
            self.reread_message()
        } else {
            self.success_message(&r.action, r.output.as_ref(), r.temp.is_some())
        };
        let hint = if level == Level::Error && !cancelled {
            hints::for_failure(r.kind, &message)
        } else if success && r.kind == ActionKind::Read {
            let format_set = matches!(&r.action, Action::Read(o) if !o.format.is_empty());
            hints::for_read(&ScanStats::from_map(&self.map), self.map.bad_tracks().len(), format_set)
        } else {
            None
        };
        let mut message = message;
        if level == Level::Error || cancelled {
            if let Some(out) = r.output.as_ref().filter(|p| p.exists()) {
                message += &format!(" The image {} may be incomplete.", out.display());
            }
        }
        let msg = format!("{message}  ({secs:.1} s)");
        self.console.push(msg.clone(), level);
        if let Some(lines) = self.console.job_log.take() {
            if let Err(e) = write_log(&self.s.log_dir, &lines) {
                self.console.push(format!("WARNING: log file not written: {e}"), Level::Warn);
            }
        }
        if success && r.numbered {
            self.s.read_naming.advance();
        }
        if self.health.pending.is_none() && self.copy.pending.is_none() {
            self.attention = true;
        }
        // Wizard steps report in the wizard's own panel, not in the banner.
        self.outcome = if self.health.pending.is_some() || self.copy.pending.is_some() {
            None
        } else {
            Some(Outcome { page: r.page, level, message: msg, hint: hint.clone() })
        };
        if let Some(check) = self.health.pending.take() {
            self.health_job_finished(check, success, cancelled, failure_text);
        } else if let Some(step) = self.copy.pending.take() {
            self.copy_job_finished(step, success, cancelled, failure_text, hint);
        }
    }

    /// Result of a re-read of the bad tracks.
    fn reread_message(&self) -> (Level, String) {
        let set = self.map.reread.clone().unwrap_or_default();
        let still = set.iter().filter(|&&(c, h)| self.map.get(c, h).state == TrackState::Bad).count();
        let fixed = self.reread_before.saturating_sub(still);
        let mut msg =
            format!("Re-read {} track(s): {fixed} recovered, {still} still unreadable.", self.reread_before);
        if matches!(self.last_read, Some((_, true))) && fixed > 0 {
            msg += " The image saved earlier is unchanged: read the whole disk again with the \
                    Recovery profile to save a complete one.";
        }
        (if still == 0 { Level::Good } else { Level::Warn }, msg)
    }

    /// Re-read only the tracks that the last read could not decode, with the Recovery profile.
    pub fn reread_bad(&mut self) {
        if self.busy() {
            return;
        }
        let Some((opts, _)) = self.last_read.clone() else {
            self.notice = Some("Nothing to re-read: read a disk first.".into());
            return;
        };
        let before = self.map.bad_tracks().len();
        let Some(spec) = self.map.start_reread() else {
            self.notice = Some("No unreadable tracks to re-read.".into());
            return;
        };
        self.reread_before = before;
        self.scan_counter += 1;
        let temp =
            std::env::temp_dir().join(format!("hgf-reread-{}-{}.scp", std::process::id(), self.scan_counter));
        let mut o = opts;
        o.profile = ReadProfile::Recovery;
        o.output = temp.clone();
        o.tracks.custom = spec;
        o.raw = false;
        self.next =
            JobExtras { temp: Some(temp), page: Some(self.map_page), reread: true, ..Default::default() };
        self.request(Action::Read(o));
    }

    pub fn run_hint_action(&mut self, a: HintAction) {
        match a {
            HintAction::OpenHealth => self.s.page = Page::Health,
            HintAction::OpenBandwidth => self.s.page = Page::Bandwidth,
            HintAction::OpenUpdate => self.s.page = Page::Update,
            HintAction::OpenSettings => self.s.page = Page::Settings,
            HintAction::RereadBad => self.reread_bad(),
        }
    }

    /// Page an operation belongs to (wizard steps stay on the health page).
    pub fn job_page(&self, kind: ActionKind) -> Page {
        if self.health.pending.is_some() {
            Page::Health
        } else if self.copy.pending.is_some() {
            Page::Copy
        } else {
            Page::for_kind(kind)
        }
    }

    fn success_message(&self, action: &Action, output: Option<&PathBuf>, scan: bool) -> (Level, String) {
        let kind = action.kind();
        let c = self.map.counts();
        let tracks = || {
            let mut s = format!("{} OK", c.ok);
            if c.recovered > 0 {
                s += &format!(", {} recovered", c.recovered);
            }
            if c.bad > 0 {
                s += &format!(", {} BAD", c.bad);
            }
            s
        };
        let lvl = if c.bad > 0 { Level::Warn } else { Level::Good };
        match kind {
            ActionKind::Read if scan => (lvl, format!("Quick scan finished: {}.", tracks())),
            // The copy wizard reads into a temporary image: do not present it as a saved file.
            ActionKind::Read if self.copy.pending.is_some() => {
                (lvl, format!("Source disk read: {}.", tracks()))
            }
            ActionKind::Read | ActionKind::Convert => (
                lvl,
                format!(
                    "Image saved to {}: {}.",
                    output.map(|p| p.display().to_string()).unwrap_or_default(),
                    tracks()
                ),
            ),
            ActionKind::Write => (lvl, format!("Disk written and verified: {}.", tracks())),
            ActionKind::Erase => (Level::Good, "Disk erased.".into()),
            ActionKind::Info => match self.device.state {
                DeviceState::Connected => (Level::Good, "Greaseweazle detected.".into()),
                _ => (Level::Error, "No Greaseweazle device found. Check the USB cable.".into()),
            },
            ActionKind::Rpm => match self.rpm.samples.len() {
                0 => (Level::Warn, "No index pulse measured: is a disk inserted?".into()),
                n => (Level::Good, format!("{n} revolution(s) measured.")),
            },
            ActionKind::Align => (Level::Good, format!("{} read(s) completed.", self.align.len())),
            ActionKind::Seek => match action {
                Action::Seek(o) => (Level::Good, format!("Head moved to cylinder {}.", o.cylinder)),
                _ => (Level::Good, "Done.".into()),
            },
            ActionKind::Clean => (Level::Good, "Cleaning finished. Let the drive dry before use.".into()),
            ActionKind::Delays if matches!(action, Action::Delays(d) if d.is_query()) => {
                (Level::Good, "Delays read from the device.".into())
            }
            ActionKind::Delays => (Level::Good, "Delays updated.".into()),
            ActionKind::Reset => (Level::Good, "Device reset to its power-on state.".into()),
            ActionKind::Bandwidth => (Level::Good, "USB bandwidth measured.".into()),
            ActionKind::PinGet | ActionKind::PinSet => (
                Level::Good,
                self.pin_result.clone().map(|r| format!("{r}.")).unwrap_or_else(|| "Done.".into()),
            ),
            ActionKind::Update => (Level::Good, "Firmware updated.".into()),
        }
    }

    fn handle_drops(&mut self, ctx: &egui::Context) {
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .filter(|p| !p.as_os_str().is_empty())
                .collect()
        });
        let Some(path) = dropped.into_iter().next() else { return };
        match self.s.page {
            Page::Write => self.set_write_input(path),
            Page::Convert => self.s.convert.input = path,
            Page::Update => self.s.update.file = path,
            _ => {
                self.inspect(path);
                self.s.page = Page::Inspector;
            }
        }
    }

    pub fn set_write_input(&mut self, path: PathBuf) {
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        if let Some(f) = formats::guess_format(&path, size) {
            self.s.write.format = f.to_string();
        } else if formats::FLUX_SUFFIXES.contains(&formats::extension(&path).unwrap_or_default().as_str()) {
            self.s.write.format.clear();
        }
        self.s.write.input = path;
    }

    /// AmigaDOS analysis of the image to write, re-read only when the file changes.
    pub fn write_adf_info(&mut self) -> Option<&Result<hgf_core::image::AdfInfo, String>> {
        let path = &self.s.write.input;
        if formats::extension(path).as_deref() != Some("adf") {
            return None;
        }
        let meta = std::fs::metadata(path).ok()?;
        let (len, mtime) = (meta.len(), meta.modified().ok());
        let fresh = matches!(&self.write_adf, Some((p, l, m, _)) if p == path && *l == len && *m == mtime);
        if !fresh {
            let info =
                std::fs::read(path).map_err(|e| e.to_string()).and_then(|d| hgf_core::image::inspect_adf(&d));
            self.write_adf = Some((path.clone(), len, mtime, info));
        }
        self.write_adf.as_ref().map(|(_, _, _, i)| i)
    }

    pub fn inspect(&mut self, path: PathBuf) {
        let r = hgf_core::image::inspect(&path);
        self.inspect_path = path.clone();
        self.inspector = Some((path, r));
    }

    // ───────────────────────────── shell ─────────────────────────────

    fn top_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.add(egui::Image::new(&self.logo).fit_to_exact_size(egui::vec2(28.0, 28.0)));
            ui.label(RichText::new("HappyGreaseFloppy").size(theme::fs(20.0)).strong());
            ui.label(RichText::new(format!("v{}", hgf_core::VERSION)).color(theme::MUTED));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (txt, col) = match self.device.state {
                    DeviceState::Connected => {
                        let model = self
                            .device
                            .fields
                            .iter()
                            .find(|(k, _)| k == "Model")
                            .map(|(_, v)| v.clone())
                            .unwrap_or_else(|| "Greaseweazle".into());
                        (format!("{}  {model}", ph::PLUGS_CONNECTED), theme::OK)
                    }
                    DeviceState::Probing => (format!("{}  Detecting…", ph::PLUGS), theme::WARN),
                    DeviceState::NotFound => (format!("{}  No device", ph::PLUGS), theme::BAD),
                    DeviceState::NoTools => (format!("{}  gw not found", ph::WARNING), theme::BAD),
                    DeviceState::Unknown => (format!("{}  Not probed", ph::PLUGS), theme::MUTED),
                };
                let busy = self.busy();
                if widgets::pill(ui, &txt, col)
                    .interact(egui::Sense::click())
                    .on_hover_text("Click to detect the Greaseweazle (gw info)")
                    .clicked()
                    && !busy
                {
                    self.probe();
                }
                ui.add_space(8.0);
                ui.add_enabled_ui(!busy, |ui| {
                    egui::ComboBox::from_id_salt("drive")
                        .selected_text(format!("{}  {}", ph::HARD_DRIVES, self.s.drive.label()))
                        .show_ui(ui, |ui| {
                            for d in Drive::ALL {
                                ui.selectable_value(&mut self.s.drive, d, d.label());
                            }
                        })
                        .response
                        .on_hover_text("Drive used by every disk and drive command");
                });
                ui.add_space(8.0);
                self.status_badge(ui);
            });
        });
    }

    fn status_badge(&self, ui: &mut Ui) {
        if let Some(r) = &self.running {
            let verb = if r.job.is_cancelling() { "STOPPING" } else { r.kind.verb() };
            ui.add(egui::Spinner::new().color(theme::ACTIVE));
            widgets::pill(ui, verb, theme::ACTIVE);
        } else {
            match self.outcome.as_ref().map(|o| o.level) {
                Some(Level::Error) => widgets::pill(ui, &format!("{}  ERROR", ph::X_CIRCLE), theme::BAD),
                Some(Level::Warn) => widgets::pill(ui, &format!("{}  WARNING", ph::WARNING), theme::WARN),
                _ => widgets::pill(ui, &format!("{}  READY", ph::CHECK_CIRCLE), theme::OK),
            };
        }
    }

    fn sidebar(&mut self, ui: &mut Ui) {
        let running_page = self.running.as_ref().map(|r| r.page);
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                for (group, pages) in Page::GROUPS {
                    ui.add_space(8.0);
                    ui.label(RichText::new(*group).small().strong().color(theme::MUTED));
                    for &p in *pages {
                        let selected = self.s.page == p;
                        let mut text =
                            RichText::new(format!("{}  {}", p.icon(), p.title())).size(theme::fs(14.5));
                        if selected {
                            text = text.color(p.color()).strong();
                        }
                        let resp = ui
                            .scope(|ui| {
                                ui.visuals_mut().selection.bg_fill = theme::tint(p.color(), 45);
                                ui.visuals_mut().selection.stroke = egui::Stroke::new(1.0, p.color());
                                ui.selectable_label(selected, text)
                            })
                            .inner;
                        if running_page == Some(p) {
                            let c = resp.rect.right_center() - egui::vec2(10.0, 0.0);
                            ui.painter().circle_filled(c, 4.0, theme::ACTIVE);
                        }
                        if resp.clicked() {
                            self.s.page = p;
                        }
                    }
                }
            });
        });
    }

    fn console_panel(&mut self, ui: &mut Ui) {
        let dark = ui.visuals().dark_mode;
        ui.horizontal(|ui| {
            let icon = if self.s.show_console { ph::CARET_DOWN } else { ph::CARET_UP };
            if ui.button(format!("{icon}  {}  Console", ph::TERMINAL)).clicked() {
                self.s.show_console = !self.s.show_console;
            }
            if !self.s.show_console {
                if let Some((l, lvl)) = self.console.lines.back() {
                    ui.label(RichText::new(l).monospace().color(lvl.color(dark)));
                }
                return;
            }
            ui.toggle_value(&mut self.console.errors_only, format!("{}  Problems only", ph::FUNNEL));
            if ui.button(format!("{}  Copy", ph::COPY)).clicked() {
                ui.ctx().copy_text(self.console.text());
            }
            if ui.button(format!("{}  Save…", ph::EXPORT)).clicked() {
                if let Some(p) = rfd::FileDialog::new().set_file_name("gw-log.txt").save_file() {
                    if let Err(e) = std::fs::write(&p, self.console.text()) {
                        self.notice = Some(format!("Cannot save log: {e}"));
                    }
                }
            }
            if ui.button(format!("{}  Clear", ph::TRASH)).clicked() {
                self.console.lines.clear();
            }
        });
        if !self.s.show_console {
            return;
        }
        let rows: Vec<&(String, Level)> = self
            .console
            .lines
            .iter()
            .filter(|(_, l)| {
                !self.console.errors_only || matches!(l, Level::Warn | Level::Error | Level::Command)
            })
            .collect();
        let row_h = ui.text_style_height(&egui::TextStyle::Monospace) + 2.0;
        let partial = self.console.partial.clone();
        egui::Frame::new().fill(ui.visuals().extreme_bg_color).corner_radius(6).inner_margin(6).show(
            ui,
            |ui| {
                egui::ScrollArea::both().stick_to_bottom(true).auto_shrink([false, false]).show_rows(
                    ui,
                    row_h,
                    rows.len() + partial.is_some() as usize,
                    |ui, range| {
                        for i in range {
                            match rows.get(i) {
                                Some((text, lvl)) => {
                                    ui.label(RichText::new(text).monospace().color(lvl.color(dark)));
                                }
                                None => {
                                    if let Some(p) = &partial {
                                        ui.label(
                                            RichText::new(format!("{p}_")).monospace().color(theme::ACTIVE),
                                        );
                                    }
                                }
                            }
                        }
                    },
                );
            },
        );
    }

    /// Progress strip, result banner and validation notice above each page.
    fn status_area(&mut self, ui: &mut Ui) {
        if let Some(r) = &self.running {
            let elapsed = r.started.elapsed().as_secs();
            let map_kind = r.kind.uses_disk_map();
            let cancelling = r.job.is_cancelling();
            let mut stop = false;
            egui::Frame::new().fill(theme::tint(theme::ACTIVE, 22)).corner_radius(8).inner_margin(8).show(
                ui,
                |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(r.kind.verb()).strong().color(theme::ACTIVE));
                        let (progress, text) = if map_kind {
                            let p = self.map.progress();
                            let eta = if p > 0.02 {
                                let left = elapsed as f32 * (1.0 - p) / p;
                                format!(" · ~{} left", fmt_secs(left as u64))
                            } else {
                                String::new()
                            };
                            let track =
                                self.map.active.map(|(c, h)| format!("  T{c}.{h}")).unwrap_or_default();
                            (p, format!("{:.0}%{track} · {}{eta}", p * 100.0, fmt_secs(elapsed)))
                        } else {
                            (0.0, fmt_secs(elapsed))
                        };
                        let w = ui.available_width() - 110.0;
                        ui.add_sized(
                            [w, 22.0],
                            egui::ProgressBar::new(progress)
                                .text(RichText::new(text).color(Color32::WHITE).strong())
                                .animate(!map_kind)
                                .fill(theme::READ),
                        );
                        stop = ui
                            .add_enabled(
                                !cancelling,
                                egui::Button::new(
                                    RichText::new(format!("{}  STOP", ph::STOP))
                                        .strong()
                                        .color(Color32::WHITE),
                                )
                                .fill(theme::BAD)
                                .min_size(egui::vec2(theme::w(96.0), 26.0)),
                            )
                            .on_hover_text("Interrupt gw (Esc)")
                            .clicked();
                    });
                },
            );
            if stop || ui.input(|i| i.key_pressed(egui::Key::Escape)) && self.confirm.is_none() {
                self.cancel();
            }
            ui.add_space(6.0);
        } else if let Some(o) = self.outcome.as_ref().filter(|o| o.page == self.s.page) {
            let (icon, col) = match o.level {
                Level::Error => (ph::X_CIRCLE, theme::BAD),
                Level::Warn => (ph::WARNING, theme::WARN),
                _ => (ph::CHECK_CIRCLE, theme::OK),
            };
            let (closed, action) =
                widgets::outcome_banner(ui, icon, &o.message, col, o.hint.as_ref(), !self.busy());
            if closed {
                self.outcome = None;
            }
            if let Some(a) = action {
                self.run_hint_action(a);
            }
            ui.add_space(6.0);
        }
        if let Some(n) = self.notice.clone() {
            if widgets::banner(ui, ph::WARNING_OCTAGON, &n, theme::WARN) {
                self.notice = None;
            }
            ui.add_space(6.0);
        }
    }

    fn confirm_dialog(&mut self, ctx: &egui::Context) {
        let Some(c) = &self.confirm else { return };
        let mut decision: Option<bool> = None;
        let color = if c.danger.is_some() { theme::BAD } else { theme::WARN };
        let title = match c.action.kind() {
            ActionKind::Write => "Write disk?",
            ActionKind::Erase => "Erase disk?",
            ActionKind::Clean => "Clean drive?",
            ActionKind::Update => "Update firmware?",
            ActionKind::Delays => "Change drive delays?",
            ActionKind::Seek => "Seek to extreme cylinder?",
            ActionKind::PinSet => "Change pin level?",
            _ => "Overwrite file?",
        };
        let modal = egui::Modal::new(egui::Id::new("confirm")).show(ctx, |ui| {
            ui.set_width(theme::w(460.0));
            ui.horizontal(|ui| {
                ui.label(RichText::new(ph::WARNING).size(theme::fs(30.0)).color(color));
                ui.heading(title);
            });
            ui.add_space(6.0);
            if let Some(d) = &c.danger {
                ui.label(RichText::new(d).strong());
            }
            if let Some(p) = &c.overwrite {
                ui.label(format!("{} already exists and will be replaced.", p.display()));
            }
            let rows = confirm_details(
                &c.action,
                c.extras.drive.unwrap_or(self.s.drive),
                c.extras.describe.as_deref(),
            );
            if !rows.is_empty() {
                ui.add_space(4.0);
                widgets::kv_grid(ui, "confirm_grid", &rows);
            }
            for w in &c.warnings {
                ui.label(RichText::new(format!("{}  {w}", ph::WARNING)).color(theme::WARN));
            }
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button(format!("{}  Cancel", ph::X)).clicked() {
                    decision = Some(false);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let label = RichText::new(format!(
                        "{}  {}",
                        ph::CHECK,
                        title.trim_end_matches('?').to_uppercase()
                    ))
                    .strong()
                    .color(Color32::WHITE);
                    if ui.add(egui::Button::new(label).fill(color)).clicked() {
                        decision = Some(true);
                    }
                });
            });
        });
        if modal.should_close() && decision.is_none() {
            decision = Some(false);
        }
        match decision {
            Some(true) => {
                if let Some(c) = self.confirm.take() {
                    for w in &c.warnings {
                        self.console.push(format!("WARNING: {w}"), Level::Warn);
                    }
                    let ow = c.overwrite.is_some();
                    self.start(c.action, ow, c.extras);
                }
            }
            Some(false) => {
                if let Some(c) = self.confirm.take() {
                    remove_temp(&c.extras);
                }
                // Only the wizard waiting for this confirmation is cancelled.
                if self.health.pending.is_some() {
                    self.health_abort("Cancelled at the confirmation.");
                } else if self.copy.pending.is_some() {
                    self.copy_abort("Cancelled at the confirmation.");
                }
            }
            None => {}
        }
    }

    /// Closing the window in the middle of a write would leave a half-written disk: ask first.
    fn close_guard(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.viewport().close_requested()) && self.busy() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.close_requested = true;
        }
        if !self.close_requested {
            return;
        }
        if !self.busy() {
            self.close_requested = false;
            return;
        }
        let mut choice: Option<bool> = None;
        let modal = egui::Modal::new(egui::Id::new("close_guard")).show(ctx, |ui| {
            ui.set_width(theme::w(420.0));
            ui.horizontal(|ui| {
                ui.label(RichText::new(ph::WARNING).size(theme::fs(30.0)).color(theme::WARN));
                ui.heading("An operation is running");
            });
            ui.label("Quitting now stops gw. An interrupted write leaves the disk incomplete.");
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button(format!("{}  Keep running", ph::PLAY)).clicked() {
                    choice = Some(false);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let t =
                        RichText::new(format!("{}  STOP AND QUIT", ph::STOP)).strong().color(Color32::WHITE);
                    if ui.add(egui::Button::new(t).fill(theme::BAD)).clicked() {
                        choice = Some(true);
                    }
                });
            });
        });
        if modal.should_close() && choice.is_none() {
            choice = Some(false);
        }
        match choice {
            Some(true) => {
                self.close_requested = false;
                self.on_exit_cleanup();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Some(false) => self.close_requested = false,
            None => {}
        }
    }

    /// Stop gw (the drive is deselected) and remove temporary files.
    fn on_exit_cleanup(&mut self) {
        if let Some(r) = self.running.take() {
            r.job.cancel();
            let deadline = Instant::now() + Duration::from_secs(4);
            while !r.job.is_finished() && Instant::now() < deadline {
                let _ = r.job.next_timeout(Duration::from_millis(100));
            }
            if let Some(t) = &r.temp {
                let _ = std::fs::remove_file(t);
            }
        }
        if let Some(c) = self.confirm.take() {
            remove_temp(&c.extras);
        }
        // Temporary images held by a wizard waiting for the user (disk swap, review…).
        self.copy_cleanup();
        self.health_cleanup();
    }

    fn drop_overlay(&self, ctx: &egui::Context) {
        if ctx.input(|i| i.raw.hovered_files.is_empty()) {
            return;
        }
        let rect = ctx.content_rect();
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("drop")));
        painter.rect_filled(rect, 0.0, Color32::from_black_alpha(170));
        let target = match self.s.page {
            Page::Write => "Drop to write this image",
            Page::Convert => "Drop to convert this image",
            Page::Update => "Drop firmware file",
            _ => "Drop to inspect this image",
        };
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            format!("{}  {target}", ph::FILE_ARROW_DOWN),
            egui::FontId::proportional(theme::fs(28.0)),
            Color32::WHITE,
        );
    }

    pub fn visualizer(&self, ui: &mut Ui) {
        let busy = self.running_kind().is_some_and(ActionKind::uses_disk_map);
        if self.s.page == self.map_page {
            visualizer::show(ui, &self.map, busy);
        } else {
            visualizer::show(ui, &DiskMap::default(), false);
        }
    }
}

fn confirm_details(a: &Action, drive: Drive, describe: Option<&str>) -> Vec<(String, String)> {
    let fmt = |f: &str| if f.is_empty() { "auto (from image)".to_string() } else { f.to_string() };
    match a {
        Action::Write(o) => vec![
            ("Image".into(), describe.map(String::from).unwrap_or_else(|| o.input.display().to_string())),
            ("Format".into(), fmt(&o.format)),
            ("Drive".into(), drive.label().into()),
            ("Verify".into(), if o.no_verify { "disabled".into() } else { "yes".into() }),
        ],
        Action::Erase(o) => vec![
            ("Drive".into(), drive.label().into()),
            ("Tracks".into(), o.tracks.spec().unwrap_or_else(|| "all (c=0-81:h=0-1)".into())),
        ],
        Action::Clean(o) => vec![
            ("Drive".into(), drive.label().into()),
            ("Passes".into(), format!("{} × {} cylinders", o.passes, o.cyls)),
        ],
        Action::Update(o) => vec![
            (
                "Source".into(),
                if o.file.as_os_str().is_empty() {
                    if o.tag.is_empty() {
                        "latest GitHub release".into()
                    } else {
                        format!("GitHub tag {}", o.tag)
                    }
                } else {
                    o.file.display().to_string()
                },
            ),
            ("Target".into(), if o.bootloader { "BOOTLOADER".into() } else { "main firmware".into() }),
        ],
        _ => Vec::new(),
    }
}

/// The user's home folder (the current folder if unknown): a predictable default destination,
/// whatever folder the app was started from.
/// How often eframe saves the settings (see `auto_save_interval`).
pub const SAVE_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5);

/// Cached AmigaDOS analysis: path, size, modification time, result.
pub type WriteAdf = (PathBuf, u64, Option<std::time::SystemTime>, Result<hgf_core::image::AdfInfo, String>);

/// Append one operation to the day's log file (`happygreasefloppy-YYYY-MM-DD.log`, UTC dates).
fn write_log(dir: &str, lines: &[String]) -> std::io::Result<()> {
    use std::io::Write;
    let now =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let (y, mo, d, h, mi, se) = hgf_core::fat12::utc(now);
    let dir = PathBuf::from(dir.trim());
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("happygreasefloppy-{y:04}-{mo:02}-{d:02}.log"));
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(f, "==== {y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{se:02} UTC ====")?;
    for l in lines {
        writeln!(f, "{l}")?;
    }
    writeln!(f)
}

pub fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .unwrap_or_default()
}

/// Delete temporary images left by a previous session that did not end normally (crash,
/// power loss). Files younger than a day are kept: another running instance may own them.
fn remove_stale_temps() {
    const PREFIXES: [&str; 6] =
        ["hgf-scan-", "hgf-reread-", "hgf-copy-", "hgf-format-", "hgf-health-", "hgf-blank-"];
    let Ok(dir) = std::fs::read_dir(std::env::temp_dir()) else { return };
    for e in dir.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if !PREFIXES.iter().any(|p| name.starts_with(p)) {
            continue;
        }
        let old = e
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age > Duration::from_secs(24 * 3600));
        if old && e.file_type().is_ok_and(|t| t.is_file()) {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

/// Delete the temporary file of a job that will not run (or has ended).
pub fn remove_temp(extras: &JobExtras) {
    if let Some(t) = &extras.temp {
        let _ = std::fs::remove_file(t);
    }
}

pub fn fmt_secs(s: u64) -> String {
    if s >= 60 {
        format!("{}m{:02}s", s / 60, s % 60)
    } else {
        format!("{s}s")
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll();
        if std::mem::take(&mut self.attention) && self.s.notify_done && !ctx.input(|i| i.focused) {
            ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
                egui::UserAttentionType::Informational,
            ));
        }
        if self.busy() || self.catalog_rx.is_some() || self.detecting {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
        self.handle_drops(&ctx);

        egui::Panel::top("top")
            .frame(
                egui::Frame::new()
                    .fill(ui.visuals().window_fill)
                    .inner_margin(egui::Margin::symmetric(12, 8)),
            )
            .show(ui, |ui| self.top_bar(ui));
        egui::Panel::left("nav")
            .resizable(false)
            .exact_size(theme::w(220.0))
            .frame(
                egui::Frame::new().fill(ui.visuals().window_fill).inner_margin(egui::Margin::symmetric(8, 4)),
            )
            .show(ui, |ui| self.sidebar(ui));
        egui::Panel::bottom("console")
            .resizable(self.s.show_console)
            .default_size(190.0)
            .min_size(if self.s.show_console { 110.0 } else { 34.0 })
            .max_size(if self.s.show_console { 600.0 } else { 34.0 })
            .show(ui, |ui| self.console_panel(ui));
        egui::CentralPanel::default().show(ui, |ui| {
            let page = self.s.page;
            ui.horizontal(|ui| {
                ui.label(RichText::new(page.icon()).size(theme::fs(24.0)).color(page.color()));
                ui.heading(RichText::new(page.title()).strong());
            });
            ui.add_space(6.0);
            self.status_area(ui);
            self.page(ui);
        });
        self.confirm_dialog(&ctx);
        self.close_guard(&ctx);
        self.drop_overlay(&ctx);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.s);
    }

    /// Settings are small: save them often, so that a choice survives a crash or a killed
    /// process, not only a normal close.
    fn auto_save_interval(&self) -> std::time::Duration {
        SAVE_INTERVAL
    }

    fn on_exit(&mut self) {
        self.on_exit_cleanup();
    }
}
