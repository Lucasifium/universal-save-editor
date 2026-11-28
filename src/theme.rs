//! Look & feel: Windows system fonts (with egui fallbacks and the system's Japanese, Chinese and
//! Korean fonts when a save needs them), icon glyphs and colours.

use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, FontId, Rounding, Stroke, TextStyle};

/// Also the fill of the main buttons, under white text (4.6:1).
pub const ACCENT: Color32 = Color32::from_rgb(90, 102, 235);
pub const MODIFIED: Color32 = Color32::from_rgb(240, 170, 60);
pub const OK: Color32 = Color32::from_rgb(90, 200, 120);
pub const WARN: Color32 = Color32::from_rgb(235, 190, 80);
pub const ERR: Color32 = Color32::from_rgb(240, 110, 110);
/// The save's own key, shown small beside a friendlier name (as text: through `fg`).
pub const RAW_KEY: Color32 = Color32::from_rgb(120, 150, 185);

/// The type scale: every font size in the program is one of these four steps.
/// Screen titles (the header card's game name, the start page), panel and dialog titles (and
/// big figures on tiles), body text (labels, buttons, values), and captions (tile captions,
/// paths, notes under a row). Only icon glyphs set sizes of their own.
pub mod size {
    pub const TITLE: f32 = 26.0;
    pub const PANEL: f32 = 17.0;
    pub const BODY: f32 = 15.0;
    pub const CAPTION: f32 = 12.5;
}

/// A screen title: the header card's name, the start page's name.
pub fn title(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text).family(semibold_family()).size(size::TITLE)
}

/// The title of a panel, card or dialog section.
pub fn panel_title(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text).family(semibold_family()).size(size::PANEL)
}

/// Caption-sized text: tile captions, paths, notes under a row.
pub fn caption(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text).size(size::CAPTION)
}

/// A proportional font at one of the steps of [`size`] (for text painted directly).
pub fn font(size: f32) -> FontId {
    FontId::proportional(size)
}

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
    let (fonts, has_icons) = definitions(false);
    ctx.set_fonts(fonts);
    has_icons
}

/// The language (index in `i18n::LANGS`, plus one) the CJK fonts were last installed for; 0 = not yet.
static CJK_ON: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Whether `s` has Japanese, Chinese or Korean characters, which the main fonts don't have.
pub fn has_cjk(s: &str) -> bool {
    s.chars().any(|c| {
        matches!(c as u32,
            0x1100..=0x11FF | 0x2E80..=0x2FDF | 0x3000..=0x30FF | 0x3100..=0x31FF | 0x3200..=0x9FFF
            | 0xA960..=0xA97F | 0xAC00..=0xD7FF | 0xF900..=0xFAFF | 0xFE30..=0xFE4F | 0xFF00..=0xFFEF | 0x20000..=0x3134F)
    })
}

/// Adds the computer's Japanese, Chinese and Korean fonts as fallbacks, the first time text
/// that needs them shows up (they are large, so they aren't read at start-up). Returns false
/// when none was found (always, in the browser version).
pub fn enable_cjk(ctx: &egui::Context) -> bool {
    use std::sync::atomic::Ordering;
    // The fonts are ordered by the interface language (a Traditional Chinese interface wants
    // Traditional glyph shapes first), so a language change installs them again.
    let want = crate::i18n::current_index() + 1;
    if CJK_ON.load(Ordering::Relaxed) == want {
        return true;
    }
    if cjk_fonts().is_empty() {
        return false;
    }
    CJK_ON.store(want, Ordering::Relaxed);
    let (fonts, _) = definitions(true);
    ctx.set_fonts(fonts);
    true
}

/// `enable_cjk` when `text` needs it.
pub fn cjk_for(ctx: &egui::Context, text: &str) {
    if has_cjk(text) {
        enable_cjk(ctx);
    }
}

/// Whether this system has a font for Japanese and Chinese text (the browser version has none).
pub fn cjk_available() -> bool {
    !cjk_fonts().is_empty()
}

/// Font files with Japanese, Chinese and Korean characters on this computer, one per script
/// where the system splits them. The script of the interface language comes first, so shared
/// Han characters take that language's glyph shapes; the rest keep a fixed order.
fn cjk_fonts() -> Vec<String> {
    if cfg!(target_arch = "wasm32") {
        return vec![];
    }
    let win = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
    let w = |name: &str| format!("{win}\\Fonts\\{name}");
    // (script, candidate files); the first file that exists in a group is used.
    let mut groups: Vec<(&str, Vec<String>)> = if cfg!(windows) {
        vec![
            ("ja", vec![w("YuGothM.ttc"), w("YuGothR.ttc"), w("meiryo.ttc"), w("msgothic.ttc")]),
            ("sc", vec![w("msyh.ttc"), w("msyh.ttf"), w("simsun.ttc")]),
            ("tc", vec![w("msjh.ttc"), w("msjh.ttf"), w("mingliu.ttc")]),
            ("ko", vec![w("malgun.ttf"), w("gulim.ttc")]),
        ]
    } else if cfg!(target_os = "macos") {
        vec![
            ("ja", vec!["/System/Library/Fonts/Hiragino Sans GB.ttc".into(), "/System/Library/Fonts/STHeiti Light.ttc".into(), "/System/Library/Fonts/Supplemental/Arial Unicode.ttf".into(), "/Library/Fonts/Arial Unicode.ttf".into()]),
            ("tc", vec!["/System/Library/Fonts/STHeiti Medium.ttc".into(), "/System/Library/Fonts/PingFang.ttc".into()]),
            ("ko", vec!["/System/Library/Fonts/AppleSDGothicNeo.ttc".into()]),
        ]
    } else if cfg!(target_os = "android") {
        vec![("ja", vec!["/system/fonts/NotoSansCJK-Regular.ttc".into(), "/system/fonts/NotoSansSC-Regular.otf".into(), "/system/fonts/DroidSansFallback.ttf".into()])]
    } else {
        let dirs = ["/usr/share/fonts/opentype/noto", "/usr/share/fonts/noto-cjk", "/usr/share/fonts/google-noto-cjk"];
        // Per-script Noto files, where the distribution splits them, give the right glyph shapes.
        let per = |s: &str| dirs.iter().map(|d| format!("{d}/NotoSansCJK{s}-Regular.otf")).collect::<Vec<_>>();
        vec![
            ("sc", per("sc")),
            ("tc", per("tc")),
            ("ko", per("kr")),
            (
                "ja",
                vec![
                    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc".into(),
                    "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc".into(),
                    "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc".into(),
                    "/usr/share/fonts/google-noto-sans-cjk-vf-fonts/NotoSansCJK-VF.ttc".into(),
                    "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc".into(),
                    "/usr/share/fonts/wenquanyi/wqy-microhei/wqy-microhei.ttc".into(),
                    "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf".into(),
                    "/usr/share/fonts/google-droid/DroidSansFallbackFull.ttf".into(),
                ],
            ),
        ]
    };
    let first = match crate::i18n::current().code {
        "zh-CN" => "sc",
        "zh-TW" => "tc",
        "ko" => "ko",
        _ => "ja",
    };
    // Stable: the language's own script first, the others as listed.
    groups.sort_by_key(|(script, _)| *script != first);
    let found: Vec<String> = groups.into_iter().filter_map(|(_, g)| g.into_iter().find(|p| std::path::Path::new(p).is_file())).collect();
    // The Linux per-script files and the all-in-one collection overlap: the first two suffice.
    if cfg!(all(unix, not(target_os = "macos"), not(target_os = "android"))) {
        return found.into_iter().take(2).collect();
    }
    found
}

fn definitions(cjk: bool) -> (FontDefinitions, bool) {
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

    // Japanese, Chinese and Korean after the text fonts, so Latin text keeps its look, but
    // before the symbol fonts: those carry heavy picture forms of the full-width "?", "!" and
    // brackets that Japanese and Chinese sentences are full of.
    if cjk {
        for (i, path) in cjk_fonts().iter().enumerate() {
            if let Some(d) = load(path) {
                let name = format!("cjk{i}");
                fonts.font_data.insert(name.clone(), d);
                for fam in [FontFamily::Proportional, FontFamily::Monospace] {
                    let list = fonts.families.entry(fam).or_default();
                    let at = list.iter().position(|n| n == "NotoEmoji-Regular" || n == "emoji-icon-font" || n == "segoe_sym").unwrap_or(list.len());
                    list.insert(at, name.clone());
                }
            }
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

    (fonts, has_icons)
}


pub fn apply(ctx: &egui::Context, dark: bool) {
    set_style(ctx, dark);
    // Pin the theme: otherwise egui follows the system (or browser) theme and, where that
    // differs from this setting, draws its own default style instead of this one.
    ctx.set_theme(if dark { egui::Theme::Dark } else { egui::Theme::Light });
}

/// The theme the player picked: dark, light, or (`follow_system`) whichever the system uses,
/// changing with it while the program runs. Where the system says nothing, the last of dark and
/// light chosen is used.
pub fn apply_choice(ctx: &egui::Context, dark: bool, follow_system: bool) {
    if !follow_system {
        return apply(ctx, dark);
    }
    set_style(ctx, true);
    set_style(ctx, false);
    ctx.options_mut(|o| o.fallback_theme = if dark { egui::Theme::Dark } else { egui::Theme::Light });
    ctx.set_theme(egui::ThemePreference::System);
}

/// Sets this program's style for the dark or the light theme.
fn set_style(ctx: &egui::Context, dark: bool) {
    let mut style = (*ctx.style_of(if dark { egui::Theme::Dark } else { egui::Theme::Light })).clone();
    style.text_styles = [
        (TextStyle::Small, FontId::new(size::CAPTION, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(size::BODY, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(size::BODY, FontFamily::Proportional)),
        // Window titles and `ui.heading`: the panel step.
        (TextStyle::Heading, FontId::new(size::PANEL, semibold_family())),
        (TextStyle::Monospace, FontId::new(14.0, FontFamily::Monospace)),
    ]
    .into();
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 5.0);
    style.spacing.interact_size.y = 26.0;

    style.visuals = visuals(dark);
    let theme = if dark { egui::Theme::Dark } else { egui::Theme::Light };
    ctx.set_style_of(theme, style);
}

/// The colours of the light or dark theme. Text keeps at least 4.5:1 against every background it
/// is drawn on, and the edges of controls and the focus ring 3:1 (the tests below check).
pub fn visuals(dark: bool) -> egui::Visuals {
    let mut v = if dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    let rounding = Rounding::same(6.0);
    v.window_rounding = Rounding::same(10.0);
    v.menu_rounding = rounding;
    for w in [&mut v.widgets.noninteractive, &mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
        w.rounding = rounding;
    }
    v.selection.bg_fill = if dark { Color32::from_rgb(62, 70, 170) } else { Color32::from_rgb(190, 198, 255) };
    v.selection.stroke = Stroke::new(1.0_f32, if dark { Color32::WHITE } else { Color32::from_rgb(30, 40, 120) });
    v.hyperlink_color = focus_color(dark);
    // The edge of text boxes, check boxes, drop-downs and buttons: without it a text box is
    // hard to tell from the panel around it.
    v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, edge_color(dark));
    if dark {
        v.panel_fill = Color32::from_rgb(24, 25, 30);
        v.window_fill = Color32::from_rgb(32, 33, 40);
        v.extreme_bg_color = Color32::from_rgb(16, 17, 21);
        v.faint_bg_color = Color32::from_rgb(30, 31, 38);
        v.widgets.inactive.weak_bg_fill = Color32::from_rgb(44, 46, 56);
        v.widgets.inactive.bg_fill = Color32::from_rgb(44, 46, 56);
        v.widgets.hovered.weak_bg_fill = Color32::from_rgb(58, 61, 76);
        // Faint ("weak") text is half-way between the text colour and this one, as are disabled
        // controls: these two keep faint text readable.
        v.widgets.noninteractive.fg_stroke.color = Color32::from_gray(200);
        // Button text, and the text in text boxes (also where it is selected, on the selection fill).
        v.widgets.inactive.fg_stroke.color = Color32::from_gray(200);
        v.widgets.noninteractive.weak_bg_fill = Color32::from_gray(76);
    } else {
        v.panel_fill = Color32::from_rgb(246, 247, 251);
        v.window_fill = Color32::WHITE;
        v.extreme_bg_color = Color32::WHITE;
        v.faint_bg_color = Color32::from_rgb(236, 238, 246);
        v.widgets.noninteractive.fg_stroke.color = Color32::from_gray(40);
        v.widgets.noninteractive.weak_bg_fill = Color32::from_gray(172);
    }
    // egui's own error and warning text (pure red and orange are too faint on these backgrounds).
    v.error_fg_color = readable_on(ERR, dark, &v);
    v.warn_fg_color = readable_on(WARN, dark, &v);
    v
}

/// Room inside windows and dialogs: egui's 6 px left their text and buttons touching the
/// window's edge. Phone-width screens keep 6 px, where every pixel of width counts.
pub fn fit_window_margin(ctx: &egui::Context) {
    let m = if ctx.screen_rect().width() < 640.0 { 6.0 } else { 12.0 };
    if ctx.style().spacing.window_margin.left != m {
        ctx.style_mut(|s| s.spacing.window_margin = egui::Margin::same(m));
    }
}

/// Links and the keyboard focus ring.
pub fn focus_color(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(150, 165, 255)
    } else {
        Color32::from_rgb(60, 70, 200)
    }
}

/// The edge of a control (a text box, a switch that is off).
pub fn edge_color(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(112, 115, 128)
    } else {
        Color32::from_rgb(128, 131, 142)
    }
}

/// The contrast ratio of two colours, as WCAG defines it (1 to 21).
pub fn contrast(a: Color32, b: Color32) -> f32 {
    fn lum(c: Color32) -> f32 {
        let ch = |v: u8| {
            let s = v as f32 / 255.0;
            if s <= 0.04045 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * ch(c.r()) + 0.7152 * ch(c.g()) + 0.0722 * ch(c.b())
    }
    let (x, y) = (lum(a), lum(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// `c` drawn at `alpha` over `bg` (how a tinted chip or strip ends up looking).
fn over(c: Color32, alpha: f32, bg: Color32) -> Color32 {
    let m = |f: u8, b: u8| (f as f32 * alpha + b as f32 * (1.0 - alpha)).round() as u8;
    Color32::from_rgb(m(c.r(), bg.r()), m(c.g(), bg.g()), m(c.b(), bg.b()))
}

/// The theme's colours, made once.
fn theme_visuals(dark: bool) -> &'static egui::Visuals {
    static THEMES: std::sync::OnceLock<[egui::Visuals; 2]> = std::sync::OnceLock::new();
    &THEMES.get_or_init(|| [visuals(false), visuals(true)])[dark as usize]
}

/// A version of a status colour (`OK`, `WARN`, `ERR`, `MODIFIED`, `ACCENT`…) that reads as text
/// (4.5:1) on every background of the theme: the colour itself where it already does, otherwise
/// moved toward white (dark theme) or black (light theme) until it does.
pub fn readable(c: Color32, dark: bool) -> Color32 {
    readable_on(c, dark, theme_visuals(dark))
}

/// `readable` against the backgrounds of `v`: panels, dialogs, cards, text boxes, and chips and
/// strips tinted with the colour itself.
fn readable_on(c: Color32, dark: bool, v: &egui::Visuals) -> Color32 {
    let mut bgs = vec![v.panel_fill, v.window_fill, v.faint_bg_color, v.extreme_bg_color];
    for bg in [v.panel_fill, v.window_fill, v.faint_bg_color] {
        bgs.push(over(c, 0.18, bg));
    }
    let mut out = if dark {
        c
    } else {
        let d = |v: u8| (v as f32 * 0.55) as u8;
        Color32::from_rgb(d(c.r()), d(c.g()), d(c.b()))
    };
    let target = if dark { 255.0 } else { 0.0 };
    for _ in 0..20 {
        if bgs.iter().all(|&bg| contrast(out, bg) >= 4.6) {
            break;
        }
        let m = |v: u8| (v as f32 + (target - v as f32) * 0.1).round() as u8;
        out = Color32::from_rgb(m(out.r()), m(out.g()), m(out.b()));
    }
    out
}

/// Underlines the selected tab: its fill alone is too close to the panel behind it to tell
/// which tab is open.
pub fn mark_selected(ui: &egui::Ui, r: &egui::Response, selected: bool) {
    if selected {
        let y = r.rect.bottom() - 1.0;
        ui.painter().hline(r.rect.x_range(), y, egui::Stroke::new(2.0, ui.visuals().strong_text_color()));
    }
}

/// Text version of a status colour that stays readable on the current background.
pub fn fg(ui: &egui::Ui, c: Color32) -> Color32 {
    readable(c, ui.visuals().dark_mode)
}

/// Draws a ring around the control that has keyboard focus while the keyboard is being used to
/// move between controls (Tab; a click hides it again). egui's own focus look is a thin edge that
/// frameless buttons, switches and section rows don't have. Call it once a frame, last.
pub fn focus_ring(ctx: &egui::Context) {
    let key = egui::Id::new("keyboard_focus_ring");
    let tabbed = ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Key { key: egui::Key::Tab, pressed: true, .. })));
    let clicked = ctx.input(|i| i.pointer.any_pressed());
    let mut on = ctx.data(|d| d.get_temp::<bool>(key)).unwrap_or(false);
    if tabbed {
        on = true;
    }
    if clicked {
        on = false;
    }
    ctx.data_mut(|d| d.insert_temp(key, on));
    if !on {
        return;
    }
    let Some(id) = ctx.memory(|m| m.focused()) else { return };
    let Some(r) = ctx.read_response(id) else { return };
    // Windows and scroll areas take part in Tab too, but have nothing to press: no ring around them.
    if !r.sense.click || id == r.layer_id.id.with("move") || !r.interact_rect.is_positive() {
        return;
    }
    let color = focus_color(ctx.style().visuals.dark_mode);
    ctx.layer_painter(r.layer_id).rect_stroke(r.interact_rect.expand(2.0), Rounding::same(5.0), Stroke::new(2.0_f32, color));
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
            // Not justified, even inside `ui.columns` (whose columns are): a justified label
            // spreads its wrapped lines across the width.
            ui.with_layout(egui::Layout::top_down(egui::Align::Min), add).inner
        })
        .inner
}

/// Backgrounds of bytes in the hex view (selected, named by a label, part of a search hit), under white text.
pub const HEX_SELECTED: Color32 = Color32::from_rgb(70, 80, 190);
pub const HEX_LABELED: Color32 = Color32::from_rgb(30, 110, 60);
pub const HEX_FOUND: Color32 = Color32::from_rgb(120, 90, 20);

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: f32 = 4.5;
    const EDGE: f32 = 3.0;

    fn check(what: &str, fg: Color32, bgs: &[(&str, Color32)], min: f32, fails: &mut Vec<String>) {
        for (name, bg) in bgs {
            let r = contrast(fg, *bg);
            if r < min {
                fails.push(format!("{what} {fg:?} on {name} {bg:?}: {r:.2} < {min}"));
            }
        }
    }

    /// "Same as the system" follows the system's theme as it changes while the program runs:
    /// the window reports the theme with every frame's input (egui-winit passes on the
    /// system's change), and egui picks the matching style, so nothing is done per frame here.
    /// Where the system reports nothing, the dark or light last chosen stays.
    #[test]
    fn same_as_the_system_follows_a_change_of_the_system_theme() {
        let ctx = egui::Context::default();
        install_fonts(&ctx);
        apply_choice(&ctx, true, true);
        let dark_with = |sys: Option<egui::Theme>| {
            let mut dark = None;
            let _ = ctx.run(egui::RawInput { system_theme: sys, ..Default::default() }, |ctx| dark = Some(ctx.style().visuals.dark_mode));
            dark.unwrap_or_default()
        };
        assert!(!dark_with(Some(egui::Theme::Light)));
        assert!(dark_with(Some(egui::Theme::Dark)));
        assert!(!dark_with(Some(egui::Theme::Light)), "and back again");
        assert!(dark_with(None), "the system says nothing: the last choice (dark)");
        apply_choice(&ctx, false, true);
        assert!(!dark_with(None), "the last choice (light)");
        // Both styles are this program's own, not egui's plain ones.
        assert_eq!(ctx.style_of(egui::Theme::Light).visuals, visuals(false));
        assert_eq!(ctx.style_of(egui::Theme::Dark).visuals, visuals(true));
        // Picking dark or light stops following the system.
        apply_choice(&ctx, false, false);
        assert!(!dark_with(Some(egui::Theme::Dark)));
    }

    /// Every font size comes from the type scale (`size`): no size written as a number elsewhere,
    /// except icon glyphs and the item cells' fitted titles.
    #[test]
    fn font_sizes_come_from_the_type_scale() {
        fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, out);
                } else if p.extension().is_some_and(|x| x == "rs") {
                    out.push(p);
                }
            }
        }
        let mut files = vec![];
        walk(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut files);
        let literal = |l: &str| {
            [".size(", "FontId::proportional(", "FontId::new("].iter().any(|pat| {
                l.match_indices(pat).any(|(i, _)| l[i + pat.len()..].starts_with(|c: char| c.is_ascii_digit()))
            })
        };
        let mut bad = vec![];
        for f in files {
            if f.ends_with("theme.rs") {
                continue;
            }
            let text = std::fs::read_to_string(&f).unwrap_or_default();
            for (n, l) in text.lines().enumerate() {
                let icon = l.contains("icon(") || l.contains("icons_family") || l.contains("PHONE_MENU");
                if literal(l) && !icon && !l.contains("proportional(pt)") {
                    bad.push(format!("{}:{}: {}", f.display(), n + 1, l.trim()));
                }
            }
        }
        assert!(bad.is_empty(), "font sizes off the type scale (use theme::size, theme::title, panel_title or caption):\n{}", bad.join("\n"));
    }

    #[test]
    fn contrast_formula() {
        assert!((contrast(Color32::BLACK, Color32::WHITE) - 21.0).abs() < 0.01);
        assert!((contrast(Color32::WHITE, Color32::WHITE) - 1.0).abs() < 0.001);
        assert!((contrast(Color32::from_gray(118), Color32::WHITE) - 4.54).abs() < 0.02);
    }

    /// Every text and control colour against the backgrounds it is drawn on, in both themes.
    #[test]
    fn both_themes_meet_contrast_minimums() {
        let mut fails = vec![];
        for dark in [false, true] {
            let t = if dark { "dark" } else { "light" };
            let v = visuals(dark);
            let surfaces = [("panel", v.panel_fill), ("window", v.window_fill), ("card", v.faint_bg_color), ("text box", v.extreme_bg_color)];
            for (what, c) in [("text", v.text_color()), ("faint text", v.weak_text_color()), ("strong text", v.strong_text_color()), ("link", v.hyperlink_color), ("error text", v.error_fg_color), ("warning text", v.warn_fg_color)] {
                check(&format!("{t} {what}"), c, &surfaces, TEXT, &mut fails);
            }
            // Button text on the button, at rest, under the pointer and pressed or focused.
            for (what, w) in [("button", v.widgets.inactive), ("hovered button", v.widgets.hovered), ("active button", v.widgets.active), ("open menu", v.widgets.open)] {
                check(&format!("{t} {what} text"), w.fg_stroke.color, &[(what, w.weak_bg_fill)], TEXT, &mut fails);
            }
            // Selected tabs and list rows, and text selected in a text box.
            check(&format!("{t} selected label"), v.selection.stroke.color, &[("selection", v.selection.bg_fill)], TEXT, &mut fails);
            // A text box draws its text in the inactive widget colour (egui's TextEdit), selected or not.
            let typed = v.widgets.inactive.text_color();
            check(&format!("{t} typed text"), typed, &[("text box", v.extreme_bg_color)], TEXT, &mut fails);
            check(&format!("{t} selected typed text"), typed, &[("selection", v.selection.bg_fill)], TEXT, &mut fails);
            check(&format!("{t} selected label text"), v.text_color(), &[("selection", v.selection.bg_fill)], TEXT, &mut fails);
            // Edges that identify a control, and the focus indicators.
            check(&format!("{t} control edge"), v.widgets.inactive.bg_stroke.color, &surfaces, EDGE, &mut fails);
            check(&format!("{t} hovered edge"), v.widgets.hovered.bg_stroke.color, &surfaces, EDGE, &mut fails);
            check(&format!("{t} focused edge"), v.widgets.active.bg_stroke.color, &surfaces, EDGE, &mut fails);
            check(&format!("{t} text box focus"), v.selection.stroke.color, &[("text box", v.extreme_bg_color), ("panel", v.panel_fill), ("window", v.window_fill)], EDGE, &mut fails);
            check(&format!("{t} focus ring"), focus_color(dark), &surfaces, EDGE, &mut fails);
            check(&format!("{t} switch on"), ACCENT, &surfaces[..3], EDGE, &mut fails);
            // Status colours as text, on plain backgrounds and on chips and strips tinted with them.
            for (what, c) in [("ACCENT", ACCENT), ("MODIFIED", MODIFIED), ("OK", OK), ("WARN", WARN), ("ERR", ERR), ("raw key", RAW_KEY)] {
                let fg = readable(c, dark);
                let mut bgs: Vec<(&str, Color32)> = surfaces.to_vec();
                for (name, bg) in [("chip on panel", v.panel_fill), ("chip on window", v.window_fill), ("chip on card", v.faint_bg_color)] {
                    bgs.push((name, over(c, 0.18, bg)));
                }
                check(&format!("{t} {what}"), fg, &bgs, TEXT, &mut fails);
            }
        }
        // White text on filled buttons and highlighted hex bytes (the same in both themes).
        for (what, bg) in [("main button", ACCENT), ("selected byte", HEX_SELECTED), ("labelled byte", HEX_LABELED), ("found byte", HEX_FOUND)] {
            check("white text", Color32::WHITE, &[(what, bg)], TEXT, &mut fails);
        }
        assert!(fails.is_empty(), "contrast below the minimum:\n{}", fails.join("\n"));
    }

    /// Colours that already pass are left as they were.
    #[test]
    fn readable_keeps_colours_that_already_pass() {
        assert_eq!(readable(OK, true), OK);
        assert_eq!(readable(ERR, false), Color32::from_rgb(132, 60, 60));
    }
}
