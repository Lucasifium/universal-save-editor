//! Keyboard focus in an "Unsaved changes" dialog: it starts on Cancel, Tab goes round the
//! dialog's buttons, and never lands on (or passes through) the controls behind it.

use super::*;

fn tab(shift: bool) -> egui::Event {
    let modifiers = if shift { egui::Modifiers::SHIFT } else { egui::Modifiers::NONE };
    egui::Event::Key { key: egui::Key::Tab, physical_key: Some(egui::Key::Tab), pressed: true, repeat: false, modifiers }
}

/// One frame of a window with a button behind the dialog. Returns where the focused control
/// is (its left edge and layer), if anything has focus.
fn frame(ctx: &egui::Context, events: Vec<egui::Event>) -> Option<(f32, egui::Order)> {
    let mut raw = egui::RawInput { events, screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 700.0))), ..Default::default() };
    wrap_dialog_tab(ctx, &mut raw);
    let _ = ctx.run(raw, |ctx| {
        begin_dialogs(ctx);
        egui::CentralPanel::default().show(ctx, |ui| {
            let _ = ui.button("Behind 1");
            let _ = ui.button("Behind 2");
        });
        modal(ctx, "Unsaved changes", |ui| {
            ui.horizontal(|ui| save_discard_cancel(ui, "Save", "Don't save"));
        });
    });
    let id = ctx.memory(|m| m.focused())?;
    ctx.read_response(id).map(|r| (r.rect.left(), r.layer_id.order))
}

#[test]
fn tab_goes_round_the_dialog_starting_on_cancel() {
    let ctx = egui::Context::default();
    for _ in 0..3 {
        frame(&ctx, vec![]);
    }
    let start = frame(&ctx, vec![]).expect("the dialog gives focus to Cancel");
    assert_eq!(start.1, egui::Order::Foreground, "focus starts in the dialog");
    // A full round and a bit: Cancel, Save, Don't save, Cancel, Save.
    let mut xs = vec![start.0];
    for _ in 0..4 {
        let (x, order) = frame(&ctx, vec![tab(false)]).expect("something keeps focus");
        assert_eq!(order, egui::Order::Foreground, "focus left the dialog");
        xs.push(x);
        // The next frame without input must not move it either (egui hands focus on a frame late).
        let (x2, order2) = frame(&ctx, vec![]).unwrap();
        assert_eq!((x2, order2), (x, order), "focus moved without a key");
    }
    let (cancel, save, discard) = (xs[0], xs[1], xs[2]);
    assert!(save < discard && discard < cancel, "order is Save, Don't save, Cancel: {xs:?}");
    assert_eq!(xs[3], cancel, "Tab from Don't save reaches Cancel");
    assert_eq!(xs[4], save, "Tab from Cancel goes round to Save, the first button");
    // Shift+Tab from the first button goes round to the last.
    let (x, order) = frame(&ctx, vec![tab(true)]).unwrap();
    assert_eq!((x, order), (cancel, egui::Order::Foreground));
}
