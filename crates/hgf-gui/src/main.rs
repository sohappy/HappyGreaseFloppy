//! HappyGreaseFloppy — graphical front-end for the Greaseweazle host tools.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod copy_ui;
mod format_ui;
mod health_ui;
mod pages;
mod templates;
mod theme;
mod visualizer;
mod widgets;

const APP_ID: &str = "happygreasefloppy";

/// Bring the settings of FluxMyFluffyFloppy (the former name) over on first start.
fn migrate_settings() {
    let Some(new) = eframe::storage_dir(APP_ID) else { return };
    if new.exists() {
        return;
    }
    for old_id in ["fluxmyfluffyfloppy", "FluxMyFluffyFloppy"] {
        let Some(old) = eframe::storage_dir(old_id) else { continue };
        if old != new && old.is_dir() && copy_dir(&old, &new).is_ok() {
            return;
        }
    }
}

fn copy_dir(from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)? {
        let e = e?;
        if e.file_type()?.is_file() {
            std::fs::copy(e.path(), to.join(e.file_name()))?;
        }
    }
    Ok(())
}

fn main() -> eframe::Result {
    migrate_settings();
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("HappyGreaseFloppy")
            // Must match packaging/linux/happygreasefloppy.desktop: Wayland desktops
            // (GNOME, KDE) take the window icon from the matching installed .desktop file.
            .with_app_id(APP_ID)
            .with_inner_size([1280.0, 880.0])
            .with_min_inner_size([1040.0, 700.0])
            .with_drag_and_drop(true)
            .with_icon(eframe::egui::IconData {
                rgba: app::LOGO_RGBA.to_vec(),
                width: app::LOGO_SIZE,
                height: app::LOGO_SIZE,
            }),
        ..Default::default()
    };
    eframe::run_native("HappyGreaseFloppy", options, Box::new(|cc| Ok(Box::new(app::App::new(cc)))))
}
