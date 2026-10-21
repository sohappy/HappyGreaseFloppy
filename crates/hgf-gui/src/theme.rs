//! Colours, visuals and fonts.

use eframe::egui::{self, Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle};
use hgf_core::TrackState;
use std::sync::atomic::{AtomicU32, Ordering};

pub const ACCENT: Color32 = Color32::from_rgb(78, 168, 222);
pub const OK: Color32 = Color32::from_rgb(67, 196, 120);
pub const RECOVERED: Color32 = Color32::from_rgb(46, 196, 182);
pub const WARN: Color32 = Color32::from_rgb(242, 174, 55);
pub const BAD: Color32 = Color32::from_rgb(226, 72, 72);
pub const FLUX: Color32 = Color32::from_rgb(155, 123, 234);
pub const ACTIVE: Color32 = Color32::from_rgb(90, 200, 250);
pub const MUTED: Color32 = Color32::from_rgb(130, 136, 148);

/// One colour per family of actions, used for buttons and section headers.
pub const READ: Color32 = Color32::from_rgb(58, 134, 255);
pub const WRITE: Color32 = Color32::from_rgb(245, 130, 50);
pub const CONVERT: Color32 = Color32::from_rgb(155, 93, 229);
pub const ERASE: Color32 = Color32::from_rgb(220, 60, 70);
pub const DRIVE: Color32 = Color32::from_rgb(20, 170, 150);
pub const DEVICE: Color32 = Color32::from_rgb(100, 116, 139);
pub const COPY: Color32 = Color32::from_rgb(0, 160, 210);
pub const FORMAT: Color32 = Color32::from_rgb(232, 98, 152);
pub const IMAGE: Color32 = Color32::from_rgb(214, 170, 40);

pub fn state_color(s: TrackState, dark: bool) -> Color32 {
    match s {
        TrackState::Pending => {
            if dark {
                Color32::from_gray(52)
            } else {
                Color32::from_gray(215)
            }
        }
        TrackState::Active => ACTIVE,
        TrackState::Ok => OK,
        TrackState::Recovered => RECOVERED,
        TrackState::Retrying => WARN,
        TrackState::Bad => BAD,
        TrackState::Empty => Color32::from_rgb(92, 103, 115),
        TrackState::Flux => FLUX,
        TrackState::OutOfRange => Color32::from_rgb(120, 110, 90),
    }
}

/// Text colour with enough contrast on the current background (darkened in light mode).
pub fn readable(c: Color32, dark: bool) -> Color32 {
    if dark {
        c
    } else {
        let f = |v: u8| (v as f32 * 0.62) as u8;
        Color32::from_rgb(f(c.r()), f(c.g()), f(c.b()))
    }
}

/// Softer version of a colour for backgrounds.
pub fn tint(c: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha)
}

/// JetBrains Mono (SIL Open Font License, see assets/fonts/JetBrainsMono-OFL.txt).
const JETBRAINS_MONO: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf");

/// Text scale chosen in Settings (1.0 = 100 %), stored as f32 bits.
static TEXT_SCALE: AtomicU32 = AtomicU32::new(0x3f80_0000);

pub const TEXT_SCALE_RANGE: std::ops::RangeInclusive<f32> = 0.8..=1.6;

pub fn text_scale() -> f32 {
    f32::from_bits(TEXT_SCALE.load(Ordering::Relaxed))
}

/// A hard-coded text size, scaled by the user's text size setting.
pub fn fs(size: f32) -> f32 {
    (size * text_scale()).round()
}

/// A layout width that holds text, scaled with it so that the text keeps fitting.
pub fn w(width: f32) -> f32 {
    width * text_scale()
}

pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    fonts.font_data.insert("jetbrains-mono".into(), egui::FontData::from_static(JETBRAINS_MONO).into());
    // First choice for all text; egui's default fonts stay as fallbacks for missing glyphs.
    // Phosphor icons resolve from their own font, except the few private-use codepoints that
    // JetBrains Mono also defines (e.g. ARROWS_LEFT_RIGHT): do not use those icons.
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        fonts.families.entry(family).or_default().insert(0, "jetbrains-mono".into());
    }
    ctx.set_fonts(fonts);
}

/// Theme chosen in Settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeChoice {
    Dark,
    Light,
    /// Follow the desktop's light/dark preference (GNOME, KDE, Windows, macOS).
    System,
}

impl ThemeChoice {
    /// From the persisted settings (`dark` predates `follow_system`).
    pub fn from_settings(dark: bool, follow_system: bool) -> Self {
        match (follow_system, dark) {
            (true, _) => ThemeChoice::System,
            (false, true) => ThemeChoice::Dark,
            (false, false) => ThemeChoice::Light,
        }
    }

    fn preference(self) -> egui::ThemePreference {
        match self {
            ThemeChoice::Dark => egui::ThemePreference::Dark,
            ThemeChoice::Light => egui::ThemePreference::Light,
            ThemeChoice::System => egui::ThemePreference::System,
        }
    }
}

/// Apply the theme and text size.
///
/// Both the dark and the light style are customised and the preference is set explicitly:
/// egui follows the system theme by default and switches between its two styles on its own,
/// so changing only the current style (as `set_visuals` does) is lost as soon as the desktop
/// reports its theme.
pub fn apply(ctx: &egui::Context, choice: ThemeChoice, text_scale: f32) {
    let scale = text_scale.clamp(*TEXT_SCALE_RANGE.start(), *TEXT_SCALE_RANGE.end());
    TEXT_SCALE.store(scale.to_bits(), Ordering::Relaxed);
    ctx.set_visuals_of(egui::Theme::Dark, visuals(true));
    ctx.set_visuals_of(egui::Theme::Light, visuals(false));
    ctx.all_styles_mut(|s| {
        s.spacing.item_spacing = egui::vec2(8.0, 6.0);
        s.spacing.button_padding = egui::vec2(10.0, 5.0);
        s.spacing.interact_size.y = (26.0 * scale).round();
        s.text_styles.insert(TextStyle::Heading, FontId::new(fs(21.0), FontFamily::Proportional));
        s.text_styles.insert(TextStyle::Body, FontId::new(fs(13.5), FontFamily::Proportional));
        s.text_styles.insert(TextStyle::Button, FontId::new(fs(13.5), FontFamily::Proportional));
        s.text_styles.insert(TextStyle::Small, FontId::new(fs(11.0), FontFamily::Proportional));
        s.text_styles.insert(TextStyle::Monospace, FontId::new(fs(12.5), FontFamily::Monospace));
    });
    ctx.set_theme(choice.preference());
}

fn visuals(dark: bool) -> egui::Visuals {
    let mut v = if dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    let r = CornerRadius::same(6);
    if dark {
        v.panel_fill = Color32::from_rgb(22, 24, 29);
        v.window_fill = Color32::from_rgb(28, 31, 38);
        v.extreme_bg_color = Color32::from_rgb(14, 15, 19);
        v.faint_bg_color = Color32::from_rgb(30, 33, 40);
        v.code_bg_color = Color32::from_rgb(18, 20, 24);
        v.widgets.inactive.weak_bg_fill = Color32::from_rgb(38, 42, 51);
        v.widgets.inactive.bg_fill = Color32::from_rgb(38, 42, 51);
        v.widgets.hovered.weak_bg_fill = Color32::from_rgb(50, 56, 68);
        v.widgets.active.weak_bg_fill = Color32::from_rgb(60, 68, 82);
    } else {
        v.panel_fill = Color32::from_rgb(244, 245, 248);
        v.window_fill = Color32::WHITE;
        v.faint_bg_color = Color32::from_rgb(236, 238, 243);
    }
    // Selected items: light text on a strong tint in dark mode, dark text on a pale tint in light mode.
    v.selection.bg_fill = if dark { tint(ACCENT, 140) } else { tint(ACCENT, 70) };
    v.selection.stroke = Stroke::new(1.0, readable(ACCENT, dark));
    v.hyperlink_color = ACCENT;
    v.window_corner_radius = CornerRadius::same(10);
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = r;
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `apply` sets the global text scale: tests that call it must not run concurrently.
    static GLOBAL_SCALE: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn lock() -> std::sync::MutexGuard<'static, ()> {
        GLOBAL_SCALE.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// One egui frame in which the desktop reports `system` as its theme.
    fn frame(ctx: &egui::Context, system: Option<egui::Theme>) {
        let input = egui::RawInput { system_theme: system, ..Default::default() };
        let mut out = ctx.run_ui(input, |_| {});
        out.textures_delta.clear(); // no renderer in tests
    }

    fn shows_dark(ctx: &egui::Context) -> bool {
        ctx.global_style().visuals.dark_mode
    }

    #[test]
    fn chosen_theme_survives_the_system_theme() {
        let _guard = lock();
        // Regression: the theme was applied before the desktop reported its own, then lost.
        for (choice, system, expect_dark) in [
            (ThemeChoice::Dark, egui::Theme::Light, true),
            (ThemeChoice::Light, egui::Theme::Dark, false),
            (ThemeChoice::System, egui::Theme::Light, false),
            (ThemeChoice::System, egui::Theme::Dark, true),
        ] {
            let ctx = egui::Context::default();
            apply(&ctx, choice, 1.0);
            frame(&ctx, None);
            frame(&ctx, Some(system));
            frame(&ctx, Some(system));
            assert_eq!(shows_dark(&ctx), expect_dark, "{choice:?} with a {system:?} desktop");
            // Our customised style is the one shown, not egui's stock one.
            let panel = ctx.global_style().visuals.panel_fill;
            assert_eq!(panel, visuals(expect_dark).panel_fill, "{choice:?}: custom visuals");
        }
    }

    #[test]
    fn text_size_applies_to_both_themes() {
        let _guard = lock();
        let ctx = egui::Context::default();
        apply(&ctx, ThemeChoice::System, 1.3);
        for system in [egui::Theme::Dark, egui::Theme::Light] {
            frame(&ctx, Some(system));
            let body = ctx.global_style().text_styles[&TextStyle::Body].size;
            assert_eq!(body, (13.5f32 * 1.3).round(), "body size with a {system:?} desktop");
        }
        apply(&ctx, ThemeChoice::System, 1.0);
    }

    #[test]
    fn legacy_settings() {
        assert_eq!(ThemeChoice::from_settings(true, false), ThemeChoice::Dark);
        assert_eq!(ThemeChoice::from_settings(false, false), ThemeChoice::Light);
        assert_eq!(ThemeChoice::from_settings(false, true), ThemeChoice::System);
    }
}
