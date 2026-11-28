//! One unsaved change in words, drawn the same way in the save dialog as in the Changes panel
//! (the old value struck through, \u{203A}, the new value in the "changed" colour) and written as
//! plain text ("Gold: 20 -> 22") for Copy changes as text.

use crate::theme;
use eframe::egui::{self, RichText};

pub struct ChangeLine {
    /// Where: "Inventory \u{203A} Scrap Iron", or the whole line when there is no old and new value
    /// ("Offset 0x1A: 4 bytes changed").
    pub at: String,
    /// The value before and after.
    pub change: Option<(String, String)>,
    /// A risk note shown with it.
    pub note: Option<String>,
}

impl ChangeLine {
    pub fn text(at: impl Into<String>) -> Self {
        Self { at: at.into(), change: None, note: None }
    }

    /// The line as plain text, with "->" between old and new and ">" between the parts of the
    /// name, so it pastes cleanly anywhere.
    pub fn plain(&self) -> String {
        let at = self.at.replace('\u{203A}', ">");
        let mut s = match &self.change {
            Some((old, new)) => format!("{at}: {old} -> {new}"),
            None => at,
        };
        if let Some(n) = &self.note {
            s.push_str(&format!("  (note: {n})"));
        }
        s
    }

    /// The line as the Changes panel draws a change: old struck through, \u{203A}, new.
    pub fn show(&self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label("\u{2022}");
            match &self.change {
                Some((old, new)) => {
                    ui.label(format!("{}:", self.at));
                    ui.label(RichText::new(old).weak().strikethrough());
                    ui.label("\u{203A}");
                    ui.label(RichText::new(new).color(theme::fg(ui, theme::MODIFIED)));
                }
                None => {
                    ui.label(&self.at);
                }
            }
            if let Some(n) = &self.note {
                ui.weak(crate::i18n::trf("(note: {0})", &[n]));
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_uses_ascii_arrows() {
        let l = ChangeLine { at: "Inventory \u{203A} Scrap Iron".into(), change: Some(("12".into(), "99".into())), note: Some("big".into()) };
        assert_eq!(l.plain(), "Inventory > Scrap Iron: 12 -> 99  (note: big)");
        assert_eq!(ChangeLine::text("The text was edited").plain(), "The text was edited");
    }
}
