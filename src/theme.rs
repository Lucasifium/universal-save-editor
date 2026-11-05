//! Look & feel: Windows system fonts (with egui fallbacks), icon glyphs and colours.

use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, FontId, Rounding, Stroke, TextStyle};

pub const ACCENT: Color32 = Color32::from_rgb(98, 110, 240);
pub const MODIFIED: Color32 = Color32::from_rgb(240, 170, 60);
pub const OK: Color32 = Color32::from_rgb(90, 200, 120);
pub const WARN: Color32 = Color32::from_rgb(235, 190, 80);
pub const ERR: Color32 = Color32::from_rgb(240, 110, 110);

/// Segoe Fluent Icons / MDL2 code points (only used when the font was found).
pub mod icon {
    pub const OPEN: &str = "\u{E8E5}";
    pub const SAVE: &str = "\u{E74E}";
    pub const UNDO: &str = "\u{E7A7}";
    pub const REDO: &str = "\u{E7A6}";
    pub const SEARCH: &str = "\u{E721}";
    pub const HISTORY: &str = "\u{E81C}";
    pub const DOWN: &str = "\u{E70D}";
    pub const RIGHT: &str = "\u{E76C}";
    pub const REVERT: &str = "\u{E7A7}";
    pub const WARNING: &str = "\u{E7BA}";
    pub const CHECK: &str = "\u{E73E}";
}

pub fn icons_family() -> FontFamily {
    FontFamily::Name("icons".into())
}

pub fn semibold_family() -> FontFamily {
    FontFamily::Name("semibold".into())
}

fn load(path: &str) -> Option<FontData> {
    std::fs::read(path).ok().map(FontData::from_owned)
}

/// Install fonts. Returns true if an icon font is available.
pub fn install_fonts(ctx: &egui::Context) -> bool {
    let mut fonts = FontDefinitions::default();
    let win = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
    let f = |name: &str| format!("{win}\\Fonts\\{name}");

    let mut prop_front = Vec::new();
    if let Some(d) = load(&f("segoeui.ttf")) {
        fonts.font_data.insert("segoe".into(), d);
        prop_front.push("segoe".to_string());
    }
    for (i, name) in prop_front.iter().enumerate() {
        fonts.families.entry(FontFamily::Proportional).or_default().insert(i, name.clone());
    }
    if let Some(d) = load(&f("consola.ttf")) {
        fonts.font_data.insert("consolas".into(), d);
        fonts.families.entry(FontFamily::Monospace).or_default().insert(0, "consolas".into());
    }
    // Symbol fallback (arrows, shapes) after the main fonts.
    if let Some(d) = load(&f("seguisym.ttf")) {
        fonts.font_data.insert("segoe_sym".into(), d);
        for fam in [FontFamily::Proportional, FontFamily::Monospace] {
            fonts.families.entry(fam).or_default().push("segoe_sym".into());
        }
    }

    let mut semibold = Vec::new();
    if let Some(d) = load(&f("seguisb.ttf")) {
        fonts.font_data.insert("segoe_sb".into(), d);
        semibold.push("segoe_sb".to_string());
    }
    semibold.extend(fonts.families[&FontFamily::Proportional].iter().cloned());
    fonts.families.insert(semibold_family(), semibold);

    let icon_font = load(&f("SegoeIcons.ttf")).or_else(|| load(&f("segmdl2.ttf")));
    let has_icons = icon_font.is_some();
    let mut icon_fam = Vec::new();
    if let Some(d) = icon_font {
        fonts.font_data.insert("icons".into(), d);
        icon_fam.push("icons".to_string());
    }
    icon_fam.extend(fonts.families[&FontFamily::Proportional].iter().cloned());
    fonts.families.insert(icons_family(), icon_fam);

    ctx.set_fonts(fonts);
    has_icons
}

pub fn apply(ctx: &egui::Context, dark: bool) {
    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (TextStyle::Small, FontId::new(12.5, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(15.0, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(15.0, FontFamily::Proportional)),
        (TextStyle::Heading, FontId::new(22.0, semibold_family())),
        (TextStyle::Monospace, FontId::new(14.0, FontFamily::Monospace)),
    ]
    .into();
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 5.0);
    style.spacing.interact_size.y = 26.0;

    let mut v = if dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    let rounding = Rounding::same(6.0);
    v.window_rounding = Rounding::same(10.0);
    v.menu_rounding = rounding;
    for w in [&mut v.widgets.noninteractive, &mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
        w.rounding = rounding;
    }
    v.selection.bg_fill = if dark { Color32::from_rgb(70, 80, 190) } else { Color32::from_rgb(190, 198, 255) };
    v.selection.stroke = Stroke::new(1.0_f32, if dark { Color32::WHITE } else { Color32::from_rgb(30, 40, 120) });
    v.hyperlink_color = if dark { Color32::from_rgb(150, 165, 255) } else { Color32::from_rgb(60, 70, 200) };
    if dark {
        v.panel_fill = Color32::from_rgb(24, 25, 30);
        v.window_fill = Color32::from_rgb(32, 33, 40);
        v.extreme_bg_color = Color32::from_rgb(16, 17, 21);
        v.faint_bg_color = Color32::from_rgb(30, 31, 38);
        v.widgets.inactive.weak_bg_fill = Color32::from_rgb(44, 46, 56);
        v.widgets.inactive.bg_fill = Color32::from_rgb(44, 46, 56);
        v.widgets.hovered.weak_bg_fill = Color32::from_rgb(58, 61, 76);
    } else {
        v.panel_fill = Color32::from_rgb(246, 247, 251);
        v.window_fill = Color32::WHITE;
        v.extreme_bg_color = Color32::WHITE;
        v.faint_bg_color = Color32::from_rgb(236, 238, 246);
    }
    style.visuals = v;
    ctx.set_style(style);
}

/// Text version of a status colour that stays readable on the current background.
pub fn fg(ui: &egui::Ui, c: Color32) -> Color32 {
    if ui.visuals().dark_mode {
        c
    } else {
        let d = |v: u8| (v as f32 * 0.55) as u8;
        Color32::from_rgb(d(c.r()), d(c.g()), d(c.b()))
    }
}

/// A rounded "card" container for grouping related controls.
pub fn card<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let fill = ui.visuals().faint_bg_color;
    egui::Frame::none()
        .fill(fill)
        .rounding(Rounding::same(8.0_f32))
        .inner_margin(egui::Margin::same(12.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}
