//! Modal dialogs: the dimmed backdrop, keyboard focus kept inside the dialog, and the
//! Save / Discard / Cancel buttons.

use super::*;

pub(super) fn modal(ctx: &egui::Context, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| d.insert_temp(modal_key(), pass));
    let screen = ctx.screen_rect();
    egui::Area::new(egui::Id::new("modal_dim")).fixed_pos(screen.min).sense(widgets::BLOCK_CLICKS).order(egui::Order::Middle).show(ctx, |ui| {
        ui.allocate_response(screen.size(), widgets::BLOCK_CLICKS);
        ui.painter().rect_filled(screen, 0.0, Color32::from_black_alpha(140));
    });
    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .max_width(fit(ctx, 640.0))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, add);
}

pub(super) fn modal_key() -> egui::Id {
    egui::Id::new("modal_shown")
}

/// A dialog that blocks the window was drawn this frame or the one before: keys and dropped
/// files that would close or switch the tab under it are ignored, as clicks there are.
pub(crate) fn modal_shown(ctx: &egui::Context) -> bool {
    let pass = ctx.cumulative_pass_nr();
    ctx.data(|d| d.get_temp::<u64>(modal_key())).is_some_and(|n| n + 1 >= pass)
}

/// The first and last control of the dialog showing now, for [`wrap_dialog_tab`].
#[derive(Clone, Copy)]
pub(super) struct TabRing {
    pub(super) first: egui::Id,
    pub(super) last: egui::Id,
}

pub(super) fn tab_ring_key() -> egui::Id {
    egui::Id::new("dialog_tab_ring")
}

/// Start of a frame: no dialog has said where its Tab order begins and ends yet.
pub(super) fn begin_dialogs(ctx: &egui::Context) {
    ctx.data_mut(|d| d.remove::<TabRing>(tab_ring_key()));
}

/// Keeps Tab inside the dialog showing now: Tab on its `last` control goes round to its `first`,
/// and Shift+Tab on `first` to `last`. (Left to egui, Tab would pass through the controls behind
/// the dialog first.)
pub(crate) fn tab_ring(first: &egui::Response, last: &egui::Response) {
    let ring = TabRing { first: first.id, last: last.id };
    first.ctx.data_mut(|d| d.insert_temp(tab_ring_key(), ring));
}

/// Before egui sees a frame's input: takes a Tab that would leave the dialog of the last frame
/// and moves focus round to the dialog's other end instead, so no control behind the dialog
/// ever has focus, even for a frame (a screen reader would announce it, and an Enter in the
/// same input would press it).
pub(super) fn wrap_dialog_tab(ctx: &egui::Context, raw: &mut egui::RawInput) {
    let Some(ring) = ctx.data(|d| d.get_temp::<TabRing>(tab_ring_key())) else { return };
    let Some(focused) = ctx.memory(|m| m.focused()) else { return };
    let Some(i) = raw.events.iter().position(|e| matches!(e, egui::Event::Key { key: egui::Key::Tab, pressed: true, .. })) else { return };
    let egui::Event::Key { modifiers, .. } = &raw.events[i] else { return };
    let to = match (modifiers.shift, focused) {
        (false, f) if f == ring.last => ring.first,
        (true, f) if f == ring.first => ring.last,
        _ => return,
    };
    raw.events.remove(i);
    ctx.memory_mut(|m| m.request_focus(to));
}

/// The buttons of an "Unsaved changes" dialog: save (0), drop the changes (1) or Cancel (2).
/// Focus starts on Cancel, and Tab goes round the three.
pub(super) fn save_discard_cancel(ui: &mut egui::Ui, save: &str, discard: &str) -> Option<u8> {
    let mut choice = None;
    let first = ui.add(egui::Button::new(RichText::new(save).color(Color32::WHITE)).fill(theme::ACCENT));
    if first.clicked() {
        choice = Some(0);
    }
    if ui.button(discard).clicked() {
        choice = Some(1);
    }
    let cancel = safe_choice(ui.button(tr("Cancel")));
    if cancel.clicked() {
        choice = Some(2);
    }
    tab_ring(&first, &cancel);
    choice
}

/// Gives keyboard focus to a dialog's safe choice (`r`, its Cancel) whenever focus is outside the
/// dialog (as when it opens): Enter or Space can't press the destructive button, or the control
/// behind the dialog, by accident. Tab then goes round the dialog's controls (see [`tab_ring`]).
pub(crate) fn safe_choice(r: egui::Response) -> egui::Response {
    let focused = r.ctx.memory(|m| m.focused());
    let inside = focused.filter(|&id| id != r.layer_id.id.with("move")).and_then(|id| r.ctx.read_response(id)).is_some_and(|f| f.layer_id == r.layer_id);
    if !inside {
        r.request_focus();
    }
    r
}
