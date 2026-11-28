//! Remembering the window's size between launches. Only the size is kept, not the position:
//! a position saved on a monitor that is gone (or with other scaling) could put the window
//! out of reach, and the system places a new window sensibly anyway.

use crate::settings::Settings;
use eframe::egui;

/// The smallest and largest window size kept, in points; anything outside is ignored.
pub const MIN: [f32; 2] = [700.0, 400.0];
pub const MAX: [f32; 2] = [10_000.0, 10_000.0];

/// A remembered size pulled into range, or `None` when it makes no sense.
pub fn sane(size: [f32; 2]) -> Option<[f32; 2]> {
    if !size.iter().all(|v| v.is_finite() && *v > 0.0) {
        return None;
    }
    Some([size[0].clamp(MIN[0], MAX[0]), size[1].clamp(MIN[1], MAX[1])])
}

/// The size the window opens at: the remembered one, unless a picture of the window is being
/// taken at a set height (`SAVE_EDITOR_SHOT_HEIGHT`), which always uses the standard width.
pub fn initial(settings: &Settings) -> [f32; 2] {
    if let Some(h) = std::env::var("SAVE_EDITOR_SHOT_HEIGHT").ok().and_then(|h| h.parse::<f32>().ok()) {
        return [1150.0, h.clamp(400.0, 4000.0)];
    }
    settings.window_size.and_then(sane).unwrap_or([1150.0, 760.0])
}

/// Called every frame: once the window has kept a new size for a second (the resize is over),
/// the size is saved. A maximized or full-screen window is not remembered as a size.
pub fn remember(ctx: &egui::Context, settings: &mut Settings) {
    if cfg!(any(test, target_arch = "wasm32", target_os = "android")) || std::env::var_os("SAVE_EDITOR_SHOT_HEIGHT").is_some() {
        return;
    }
    let (rect, maximized, full) = ctx.input(|i| {
        let v = i.viewport();
        (v.inner_rect, v.maximized.unwrap_or(false), v.fullscreen.unwrap_or(false))
    });
    let Some(rect) = rect.filter(|_| !maximized && !full) else { return };
    // The viewport's size is in zoomed points; the window is opened in unzoomed ones.
    let z = ctx.zoom_factor();
    let Some(size) = sane([(rect.width() * z).round(), (rect.height() * z).round()]) else { return };
    if settings.window_size == Some(size) {
        return;
    }
    let id = egui::Id::new("window_size_seen");
    let now = ctx.input(|i| i.time);
    let seen = ctx.data(|d| d.get_temp::<([f32; 2], f64)>(id));
    match seen {
        Some((s, t)) if s == size => {
            if now - t >= 1.0 && !ctx.input(|i| i.pointer.any_down()) {
                settings.window_size = Some(size);
                settings.save();
            } else {
                ctx.request_repaint_after(std::time::Duration::from_millis(500));
            }
        }
        _ => {
            ctx.data_mut(|d| d.insert_temp(id, (size, now)));
            ctx.request_repaint_after(std::time::Duration::from_millis(1100));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remembered_sizes_are_kept_in_range() {
        assert_eq!(sane([1300.0, 900.0]), Some([1300.0, 900.0]));
        assert_eq!(sane([100.0, 50.0]), Some(MIN));
        assert_eq!(sane([1e9, 1e9]), Some(MAX));
        assert_eq!(sane([f32::NAN, 800.0]), None);
        assert_eq!(sane([-5.0, 800.0]), None);
        let mut s = Settings::default();
        if std::env::var_os("SAVE_EDITOR_SHOT_HEIGHT").is_none() {
            assert_eq!(initial(&s), [1150.0, 760.0]);
            s.window_size = Some([1400.0, 1000.0]);
            assert_eq!(initial(&s), [1400.0, 1000.0]);
        }
    }
}
