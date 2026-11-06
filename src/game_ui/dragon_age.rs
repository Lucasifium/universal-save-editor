//! Dragon Age: Origins: party money, the Warden's sheet, followers (approval and stats) and
//! the backpack's stacks.

use super::{columns, fact, hero, panel, tabs, GameCtx};
use crate::json_view::pointer_escape;
use eframe::egui::{self, Color32, Ui};
use serde_json::Value;

fn join(base: &str, key: &str) -> String {
    format!("{base}/{}", pointer_escape(key))
}

fn keys(cx: &GameCtx, p: &str) -> Vec<String> {
    match cx.get(p) {
        Some(Value::Object(m)) => m.keys().cloned().collect(),
        _ => vec![],
    }
}

/// Copper → "11g 53s 93c" (100 copper to a silver, 100 silver to a gold).
pub fn money_text(copper: i64) -> String {
    let c = copper.max(0);
    format!("{}g {}s {}c", c / 10_000, c / 100 % 100, c % 100)
}

const ATTRS: [&str; 6] = ["Strength", "Dexterity", "Willpower", "Magic", "Cunning", "Constitution"];
const POINTS: [&str; 4] = ["Attribute points", "Skill points", "Talent points", "Specialization points"];

pub fn show(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(150, 30, 30);
    let name = cx.text("/Player/Name").unwrap_or_else(|| "Warden".into());
    let mut sub = vec!["Dragon Age: Origins".to_string()];
    if let Some(l) = cx.num("/Player/Level") {
        sub.push(format!("Level {l}"));
    }
    if let Some(c) = cx.text("/Party/Campaign").filter(|c| c.starts_with("Awakening")) {
        sub.push(c);
    }
    if let Some(a) = cx.text("/Party/Area") {
        sub.push(a);
    }
    hero(ui, accent, &name, &sub.join("  \u{00B7}  "), |ui| {
        cx.tile(ui, "Money (copper)", "/Party/Money (copper)", accent);
        if let Some(c) = cx.int("/Party/Money (copper)") {
            fact(ui, "Money", &money_text(c), accent);
        }
        cx.tile(ui, "Level", "/Player/Level", accent);
        cx.tile(ui, "Experience", "/Player/Experience", accent);
    });
    sheet(ui, cx, "/Player", "dao_pl", accent);

    let followers = keys(cx, "/Followers");
    if !followers.is_empty() {
        panel(ui, "Followers", |ui| {
            let t = tabs(ui, "dao_followers", &followers);
            let base = join("/Followers", &followers[t]);
            if let Some(n) = cx.text(&join(&base, "Name")) {
                ui.weak(format!("Named {n}"));
            }
            let ap = join(&base, "Approval");
            if let Some(a) = cx.num(&ap) {
                ui.horizontal(|ui| {
                    cx.label(ui, "Approval", &ap);
                    cx.editor(ui, &ap, 60.0);
                    let w = (ui.available_width() - 8.0).max(40.0);
                    super::bar(ui, w, ((a + 100.0) / 200.0) as f32, accent, Some(&format!("{a}")));
                });
                ui.add_space(4.0);
            }
            ui.horizontal_wrapped(|ui| {
                for k in ["Level", "Experience"] {
                    cx.tile(ui, k, &join(&base, k), accent);
                }
            });
            sheet(ui, cx, &base, &format!("dao_f{t}"), accent);
        });
    }
    let others = keys(cx, "/Other approval");
    if !others.is_empty() {
        panel(ui, "Other approval", |ui| {
            egui::Grid::new("dao_other").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                for k in &others {
                    cx.grid_row(ui, k, &join("/Other approval", k), 70.0);
                }
            });
            ui.weak("Approval entries of followers who aren't in the party camp now (left, or not met yet), by the game's object id.");
        });
    }
    let items = keys(cx, "/Backpack");
    if !items.is_empty() {
        panel(ui, "Backpack", |ui| {
            let sid = egui::Id::new("dao_bag_q");
            let mut q = ui.data(|d| d.get_temp::<String>(sid)).unwrap_or_default();
            ui.horizontal(|ui| {
                ui.label("Filter:");
                ui.add(egui::TextEdit::singleline(&mut q).hint_text("item").desired_width(220.0));
            });
            ui.data_mut(|d| d.insert_temp(sid, q.clone()));
            let q = q.to_lowercase();
            let shown: Vec<&String> = items.iter().filter(|k| q.is_empty() || k.to_lowercase().contains(&q)).collect();
            egui::ScrollArea::vertical().id_salt("dao_bag_s").max_height(380.0).show(ui, |ui| {
                columns(ui, 340.0, shown.len(), |ui, i| {
                    let p = join("/Backpack", shown[i]);
                    let label_w = (ui.available_width() - 150.0).clamp(80.0, 240.0);
                    cx.row_fixed(ui, shown[i], &p, label_w, 70.0);
                });
            });
            ui.weak("Stack sizes of the party's backpack. Items are shown by their internal resource name; items can't be added here.");
        });
    }
}

/// Attributes and unspent points of one character.
fn sheet(ui: &mut Ui, cx: &mut GameCtx, base: &str, id: &str, accent: Color32) {
    ui.push_id(id, |ui| {
        columns(ui, 360.0, 2, |ui, i| {
            if i == 0 {
                panel(ui, "Attributes", |ui| {
                    for k in ATTRS {
                        cx.attribute(ui, k, &join(base, k), 60.0, accent);
                    }
                    ui.weak("Base values, before equipment and talents.");
                });
            } else {
                panel(ui, "Unspent points", |ui| {
                    egui::Grid::new("pts").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                        for k in POINTS {
                            cx.grid_row(ui, k, &join(base, k), 70.0);
                        }
                    });
                    let mut info = vec![];
                    for k in ["Health", "Mana or stamina", "Talents and spells known", "Skills known"] {
                        if let Some(v) = cx.text(&join(base, k)) {
                            info.push(format!("{k}: {v}"));
                        }
                    }
                    if !info.is_empty() {
                        ui.add_space(4.0);
                        ui.weak(info.join("  \u{00B7}  "));
                    }
                    if let Some(e) = cx.text(&join(base, "Equipped")) {
                        ui.weak(format!("Equipped: {e}"));
                    }
                });
            }
        });
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn money_in_coins() {
        assert_eq!(super::money_text(115393), "11g 53s 93c");
        assert_eq!(super::money_text(5), "0g 0s 5c");
    }
}
