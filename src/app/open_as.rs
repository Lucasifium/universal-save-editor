//! File › Open as: reading a file another way when the editor guessed wrong (a game screen
//! meant for another game, a format taken for another), or to see its bytes.

use super::*;
use crate::formats::ReadAs;

/// The ways offered, with their menu text and what each does.
fn choices() -> [(ReadAs, &'static str, &'static str); 5] {
    [
        (ReadAs::Detected, tr("As detected"), tr("The way the editor reads this file on its own.")),
        (ReadAs::NoScreen, tr("Values only, without the game screen"), tr("For when the game screen belongs to another game: the same data, as the list of values.")),
        (ReadAs::Unpacked, tr("Unpacked data in the hex view"), tr("The data inside the file's packing (compression, encryption, archive) as bytes. Saving packs it again the same way.")),
        (ReadAs::Text, tr("The whole file as text"), tr("The file as plain text, nothing unpacked. Saving writes the text as it is.")),
        (ReadAs::Bytes, tr("The file's own bytes in the hex view"), tr("Exactly what is on disk, nothing unpacked. Saving writes the bytes as they are.")),
    ]
}

impl App {
    /// The Open as entries, for the File menu and the format chip's pop-up. Returns the way
    /// picked, if any.
    pub(super) fn open_as_items(&self, ui: &mut egui::Ui) -> Option<ReadAs> {
        let Some(d) = &self.doc else { return None };
        let current = d.opts.read_as;
        let mut picked = None;
        ui.weak(tr("Read this file another way, for when the editor guessed wrong."));
        ui.separator();
        for (way, label, tip) in choices() {
            let fits = match way {
                ReadAs::NoScreen => d.game.is_some(),
                ReadAs::Unpacked => !d.layers.is_empty(),
                _ => true,
            };
            let on = way == current;
            if ui.add_enabled(fits || on, egui::SelectableLabel::new(on, label)).on_hover_text(tip).clicked() {
                if !on {
                    picked = Some(way);
                }
                ui.close_menu();
            }
        }
        picked
    }

    /// Opens the file again the way picked in Open as.
    pub(super) fn open_as(&mut self, way: ReadAs) {
        if self.saving.is_some() || self.loading.is_some() {
            return;
        }
        if self.dirty {
            return self.fail(tr("Save or revert your changes before opening the file another way."));
        }
        self.reopen_with(|o| o.read_as = way);
        if way == ReadAs::Text && self.doc.as_ref().is_some_and(|d| d.opts.read_as == ReadAs::Text && !matches!(d.content, Content::Text(_))) {
            self.info(tr("This file isn't plain text, so it is shown as bytes in the hex view."));
        }
    }

    /// The format chip's pop-up: the Open as entries under the chip.
    pub(super) fn open_as_popup(&self, ui: &mut egui::Ui, chip: &egui::Response) -> Option<ReadAs> {
        let id = ui.make_persistent_id("open_as_chip");
        if chip.clicked() {
            ui.memory_mut(|m| m.toggle_popup(id));
        }
        let mut picked = None;
        egui::popup_below_widget(ui, id, chip, egui::PopupCloseBehavior::CloseOnClickOutside, |ui| {
            ui.set_min_width(280.0);
            ui.label(RichText::new(tr("Open as")).strong());
            picked = self.open_as_items(ui);
            if picked.is_some() {
                ui.memory_mut(|m| m.close_popup());
            }
        });
        picked
    }
}

/// The chip text for a file opened another way than detected.
pub(super) fn chosen_label(way: ReadAs) -> Option<&'static str> {
    match way {
        ReadAs::Detected => None,
        ReadAs::NoScreen => Some(tr("Opened without the game screen")),
        ReadAs::Unpacked => Some(tr("Opened as unpacked bytes")),
        ReadAs::Text => Some(tr("Opened as text")),
        ReadAs::Bytes => Some(tr("Opened as the file's bytes")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::review::tests::Fixture;

    #[test]
    fn a_file_opened_as_its_bytes_saves_exactly_those_bytes_and_goes_back_as_detected() {
        let raw = br#"{"gold": 100}"#;
        let f = Fixture::new("open-as-bytes", raw);
        let mut app = f.app();
        assert!(matches!(app.doc.as_ref().unwrap().content, Content::Json { .. }));
        app.open_as(ReadAs::Bytes);
        let d = app.doc.as_ref().unwrap();
        assert!(d.layers.is_empty() && d.opts.read_as == ReadAs::Bytes);
        assert!(matches!(&d.content, Content::Binary(b) if b == raw));
        // `{"gold": 100}`: the 1 of 100 is byte 9.
        app.apply_bytes(9, b"2".to_vec());
        // Not while there are unsaved changes.
        app.open_as(ReadAs::Detected);
        assert!(matches!(app.doc.as_ref().unwrap().content, Content::Binary(_)));
        app.save_to(&f.path.clone());
        assert_eq!(std::fs::read(&f.path).unwrap(), br#"{"gold": 200}"#, "{}", app.status);
        app.open_as(ReadAs::Detected);
        assert!(matches!(&app.doc.as_ref().unwrap().content, Content::Json { value, .. } if value["gold"] == 200));
    }

    #[test]
    fn a_wrong_game_screen_can_be_left_out_and_text_is_offered() {
        let f = Fixture::new("open-as-screen", br#"{"GAME": {"dollars": 4}, "cardAreas": {}}"#);
        let mut app = f.app();
        assert!(app.doc.as_ref().unwrap().game.is_some(), "the sample should open on a game screen");
        assert!(app.tab == Tab::Game);
        app.open_as(ReadAs::NoScreen);
        let d = app.doc.as_ref().unwrap();
        assert!(d.game.is_none() && matches!(d.content, Content::Json { .. }));
        assert!(app.tab == Tab::Values);
        app.open_as(ReadAs::Text);
        assert!(matches!(&app.doc.as_ref().unwrap().content, Content::Text(t) if t.contains("dollars")));
        assert_eq!(chosen_label(ReadAs::Text), Some("Opened as text"));
        assert_eq!(chosen_label(ReadAs::Detected), None);
    }

    #[test]
    fn unpacked_data_is_packed_again_on_save() {
        use std::io::Write as _;
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        gz.write_all(br#"{"gold": 100}"#).unwrap();
        let packed = gz.finish().unwrap();
        let f = Fixture::new("open-as-unpacked", &packed);
        let mut app = f.app();
        assert!(!app.doc.as_ref().unwrap().layers.is_empty(), "the sample should be unpacked");
        app.open_as(ReadAs::Unpacked);
        let d = app.doc.as_ref().unwrap();
        assert!(!d.layers.is_empty());
        assert!(matches!(&d.content, Content::Binary(b) if b == br#"{"gold": 100}"#));
        app.apply_bytes(9, b"7".to_vec());
        app.save_to(&f.path.clone());
        let back = formats::decode(&std::fs::read(&f.path).unwrap());
        assert!(matches!(&back.content, Content::Json { value, .. } if value["gold"] == 700), "{}", app.status);
    }
}
