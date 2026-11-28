//! Pinning a value to the top of the Values list for every save of the same game.

use super::*;

impl App {
    /// Pins or unpins the Values row `i` for the game `key` (see `pins::game_key`).
    pub(super) fn toggle_pin(&mut self, key: &str, i: usize) {
        let Some(row) = self.model.rows.get(i) else { return };
        let (ptr, name) = (row.ptr.to_string(), row.name().to_string());
        match self.pins.toggle(key, &ptr) {
            Ok(true) => self.info(trf("Pinned {0}. It stays at the top of the list for every save of this game.", &[&name])),
            Ok(false) => self.info(trf("Unpinned {0}.", &[&name])),
            Err(e) => self.fail(trf("Couldn't keep the pin: {0}", &[&e])),
        }
        // The groups above the sections changed.
        self.collapsed_gen += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::review::tests::Fixture;

    fn run(app: &mut App, ctx: &egui::Context, w: f32, events: Vec<egui::Event>) -> egui::FullOutput {
        let mut raw = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w, 800.0))), events, ..Default::default() };
        raw.time = Some(ctx.input(|i| i.time) + 0.5);
        eframe::App::raw_input_hook(app, ctx, &mut raw);
        ctx.run(raw, |ctx| app.frame(ctx))
    }

    /// Where the widgets named `name` are drawn.
    fn named(out: &egui::FullOutput, name: &str) -> Vec<egui::Rect> {
        let mut found = vec![];
        if let Some(tree) = &out.platform_output.accesskit_update {
            for (_, node) in &tree.nodes {
                if node.name().as_deref() == Some(name) {
                    if let Some(b) = node.bounds() {
                        found.push(egui::Rect::from_min_max(egui::pos2(b.x0 as f32, b.y0 as f32), egui::pos2(b.x1 as f32, b.y1 as f32)));
                    }
                }
            }
        }
        found
    }

    #[test]
    fn a_pin_puts_the_value_on_top_and_fits_at_every_width() {
        let f = Fixture::new("pins-ui", br#"{"zoom": 2, "gold": 100, "hero": {"level": 3, "luck": 7, "name": "Ann"}}"#);
        let mut app = f.app();
        f.ctx.enable_accesskit();
        let key = crate::pins::game_key(&f.path, "JSON");
        let luck = app.model.by_ptr["/hero/luck"];
        for w in [1150.0, 390.0] {
            for _ in 0..3 {
                let out = run(&mut app, &f.ctx, w, vec![]);
                assert!(crate::app::layout_tests::cut_off_text(&out).is_empty(), "at {w}: {:?}", crate::app::layout_tests::cut_off_text(&out));
            }
        }
        // Common edits: gold and level, not the zoom or the name.
        let common: Vec<_> = app.disp.iter().filter_map(|d| if let Disp::Extra(i) = d { Some(app.model.rows[*i].ptr.to_string()) } else { None }).collect();
        assert_eq!(common, ["/gold", "/hero/level"]);
        let out = run(&mut app, &f.ctx, 1150.0, vec![]);
        assert_eq!(named(&out, "Pin").len(), 5 + common.len(), "every editable value has its pin");
        app.toggle_pin(&key, luck);
        assert!(app.pins.is_pinned(&key, "/hero/luck"));
        for _ in 0..2 {
            run(&mut app, &f.ctx, 1150.0, vec![]);
        }
        assert!(matches!(app.disp[..2], [Disp::Group(values_view::Group::Pinned, 1), Disp::Extra(i)] if i == luck));
        // A phone-width list leaves the pin buttons out; the name's menu still pins.
        let out = run(&mut app, &f.ctx, 390.0, vec![]);
        let out = if named(&out, "Pin").is_empty() { out } else { run(&mut app, &f.ctx, 390.0, vec![]) };
        assert!(named(&out, "Pin").is_empty() && named(&out, "Unpin").is_empty());
        app.toggle_pin(&key, luck);
        assert!(!app.pins.is_pinned(&key, "/hero/luck"));
    }
}
