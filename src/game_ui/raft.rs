//! Raft: the game mode, the stacks in every storage box, the learned and researched items,
//! and the "Max crafting materials" button. Shown below the world values of the Raft screen.

use super::raft_items::items;
use super::{panel, GameCtx};
use crate::i18n::{tr, trf};
use crate::widgets::ListBox;
use eframe::egui::{self, Ui};
use serde_json::Value;

pub const ABOUT: &str = "Raises the planks, palm leaves, scrap, plastic, rope, nails, stone, sand, clay, glass, ores, ingots, bolts, hinges, circuit boards, vine goo, feathers, dirt, wool and leather already in your storage boxes to the game's stack size of 20. Tools, food, blueprints, plants and everything else are left alone.";

pub const ABOUT_PLAYER: &str = "Raises the planks, palm leaves, scrap, plastic, rope, nails, stone, sand, clay, glass, ores, ingots, bolts, hinges, circuit boards, vine goo, feathers, dirt, wool and leather you already carry, in every world's hotbar and inventory, to the game's stack size of 20. Tools, food, blueprints, plants and everything else are left alone.";

/// Plain crafting materials: ids from the game's item table, all with a stack size of 20.
const MATERIALS: &[i32] = &[19, 20, 21, 22, 23, 24, 25, 50, 53, 65, 66, 67, 70, 72, 73, 95, 125, 154, 155, 177, 178, 179, 239, 243, 246, 247, 254, 301, 307];

pub fn item(id: i32) -> Option<(&'static str, i32)> {
    let t = items();
    t.binary_search_by_key(&id, |x| x.0).ok().map(|i| (t[i].1, t[i].2))
}

pub fn item_name(id: i64) -> String {
    i32::try_from(id).ok().and_then(item).map_or_else(|| format!("Item {id}"), |(n, _)| n.to_string())
}

/// One stack in a storage box: pointers of its item id and amount.
pub struct Stack {
    pub item: i64,
    pub amount_ptr: String,
    pub amount: i64,
}

pub struct Chest {
    pub label: String,
    pub id: i64,
    pub stacks: Vec<Stack>,
}

/// Every storage box of the world (a behaviour that has a `Slots` array of item stacks).
pub fn chests(root: &Value) -> Vec<Chest> {
    let Some(Value::Array(b)) = root.pointer("/Behaviours") else { return vec![] };
    let mut out = vec![];
    for (bi, e) in b.iter().enumerate() {
        let Some(Value::Array(slots)) = e.get("Slots") else { continue };
        let (Some(block), Some(id)) = (e.get("BlockIndex").and_then(Value::as_i64), e.get("ObjectIndex").and_then(Value::as_i64)) else { continue };
        let stacks: Vec<Stack> = slots
            .iter()
            .enumerate()
            .filter_map(|(si, s)| {
                let item = s.get("ItemIndex")?.as_i64()?;
                let amount = s.get("ItemAmount")?.as_i64()?;
                Some(Stack { item, amount_ptr: format!("/Behaviours/{bi}/Slots/{si}/ItemAmount"), amount })
            })
            .collect();
        out.push(Chest { label: trf("{0} #{1}", &[&item_name(block), &id]), id, stacks });
    }
    out
}

/// What the "Max crafting materials" button does; returns the stacks raised.
pub fn max_materials(cx: &mut GameCtx) -> usize {
    let mut n = 0;
    for c in chests(cx.root) {
        for s in c.stacks {
            let Ok(id) = i32::try_from(s.item) else { continue };
            if !MATERIALS.contains(&id) {
                continue;
            }
            let cap = item(id).map_or(20, |(_, cap)| cap) as i64;
            if s.amount < cap && cx.can_edit(&s.amount_ptr) {
                cx.set(&s.amount_ptr, Value::from(cap));
                n += 1;
            }
        }
    }
    n
}

fn names(root: &Value, p: &str) -> Vec<String> {
    match root.pointer(p) {
        Some(Value::Array(a)) => a.iter().filter_map(Value::as_i64).map(item_name).collect(),
        _ => vec![],
    }
}

/// One stack in the player file (`RGD_Users.rgd`): hotbar slots are 0 to 9, the inventory grid
/// follows from 10.
pub struct PlayerSlot {
    pub index: i64,
    pub item: i64,
    pub amount_ptr: String,
    pub amount: i64,
}

pub struct PlayerSave {
    pub world: String,
    pub base: String,
    pub slots: Vec<PlayerSlot>,
}

/// Every player entry of the player file: one per world the player has been in.
pub fn players(root: &Value) -> Vec<PlayerSave> {
    let Some(Value::Array(users)) = root.pointer("/Users") else { return vec![] };
    users
        .iter()
        .enumerate()
        .filter_map(|(ui, u)| {
            let slots = u.pointer("/Player/Inventory/Slots")?.as_array()?;
            let base = format!("/Users/{ui}/Player");
            let slots = slots
                .iter()
                .enumerate()
                .filter_map(|(si, s)| {
                    Some(PlayerSlot {
                        index: s.get("SlotIndex")?.as_i64()?,
                        item: s.get("ItemIndex")?.as_i64()?,
                        amount_ptr: format!("{base}/Inventory/Slots/{si}/ItemAmount"),
                        amount: s.get("ItemAmount")?.as_i64()?,
                    })
                })
                .collect();
            let world = u.get("WorldGuid").and_then(Value::as_str).unwrap_or("").to_string();
            Some(PlayerSave { world, base, slots })
        })
        .collect()
}

/// "Max crafting materials" for the player file; returns the stacks raised.
pub fn max_player_materials(cx: &mut GameCtx) -> usize {
    let mut n = 0;
    for p in players(cx.root) {
        for s in p.slots {
            let Ok(id) = i32::try_from(s.item) else { continue };
            if !MATERIALS.contains(&id) {
                continue;
            }
            let cap = item(id).map_or(20, |(_, cap)| cap) as i64;
            if s.amount < cap && cx.can_edit(&s.amount_ptr) {
                cx.set(&s.amount_ptr, Value::from(cap));
                n += 1;
            }
        }
    }
    n
}

fn show_players(ui: &mut Ui, cx: &mut GameCtx) {
    let all = players(cx.root);
    if all.iter().any(|p| !p.slots.is_empty()) && super::quick_button(ui, tr("Max crafting materials"), tr(ABOUT_PLAYER)) {
        max_player_materials(cx);
    }
    for (n, p) in all.iter().enumerate() {
        let title = trf("World {0}", &[&p.world.chars().take(8).collect::<String>()]);
        egui::CollapsingHeader::new(title).id_salt(("raft_player", n)).default_open(n + 1 == all.len()).show(ui, |ui| {
            egui::Grid::new(("raft_player_vitals", n)).num_columns(2).spacing([14.0, 4.0]).show(ui, |ui| {
                for (name, key) in [(tr("Health"), "Health"), (tr("Hunger"), "Hunger"), (tr("Thirst"), "Thirst")] {
                    let ptr = format!("{}/{key}", p.base);
                    cx.label(ui, name, &ptr);
                    ui.horizontal(|ui| cx.editor(ui, &ptr, 90.0));
                    ui.end_row();
                }
            });
            for (title, lo, hi) in [(tr("Hotbar"), 0, 10), (tr("Inventory"), 10, i64::MAX)] {
                let rows: Vec<&PlayerSlot> = p.slots.iter().filter(|s| s.index >= lo && s.index < hi).collect();
                if rows.is_empty() {
                    continue;
                }
                panel(ui, title, |ui| {
                    egui::Grid::new(("raft_player_grid", n, lo)).num_columns(3).spacing([14.0, 4.0]).show(ui, |ui| {
                        for s in rows {
                            ui.label(trf("Slot {0}", &[&(s.index - lo + 1)]));
                            cx.label(ui, &item_name(s.item), &s.amount_ptr);
                            ui.horizontal(|ui| cx.editor(ui, &s.amount_ptr, 90.0));
                            ui.end_row();
                        }
                    });
                });
            }
        });
    }
}

pub fn show(ui: &mut Ui, cx: &mut GameCtx) {
    if cx.root.pointer("/Users").is_some() {
        show_players(ui, cx);
        return;
    }
    if let Some(mode) = cx.int("/Mode/value__") {
        panel(ui, tr("Difficulty"), |ui| {
            // Creative is a separate kind of world: its islands are not loaded in the other modes,
            // so a world is never switched into or out of it.
            let levels: Vec<(&str, Value)> = if mode == 2 {
                vec![(tr("Creative"), Value::from(2))]
            } else {
                vec![(tr("Normal"), Value::from(0)), (tr("Easy"), Value::from(3)), (tr("Peaceful"), Value::from(5)), (tr("Hardcore"), Value::from(1))]
            };
            cx.difficulty(ui, tr("Game mode"), "/Mode/value__", &levels);
        });
    }
    let boxes = chests(cx.root);
    if !boxes.iter().all(|c| c.stacks.is_empty()) {
        if super::quick_button(ui, tr("Max crafting materials"), tr(ABOUT)) {
            max_materials(cx);
        }
        panel(ui, &format!("{} ({})", tr("Storage"), boxes.len()), |ui| {
            let id = egui::Id::new("raft_filter");
            let mut q = ui.data(|d| d.get_temp::<String>(id)).unwrap_or_default();
            ui.horizontal(|ui| {
                ui.label(tr("Filter:"));
                ui.add(egui::TextEdit::singleline(&mut q).desired_width(200.0));
            });
            ui.data_mut(|d| d.insert_temp(id, q.clone()));
            let q = q.to_lowercase();
            egui::ScrollArea::vertical().show_list(ui, "raft_storage", 420.0, |ui| {
                for c in &boxes {
                    let rows: Vec<&Stack> = c.stacks.iter().filter(|s| q.is_empty() || item_name(s.item).to_lowercase().contains(&q)).collect();
                    if rows.is_empty() {
                        continue;
                    }
                    egui::CollapsingHeader::new(&c.label).id_salt(("raft_chest", c.id)).default_open(false).show(ui, |ui| {
                        egui::Grid::new(("raft_chest_grid", c.id)).num_columns(2).spacing([14.0, 4.0]).show(ui, |ui| {
                            for s in rows {
                                cx.label(ui, &item_name(s.item), &s.amount_ptr);
                                ui.horizontal(|ui| cx.editor(ui, &s.amount_ptr, 90.0));
                                ui.end_row();
                            }
                        });
                    });
                }
            });
        });
    }
    for (p, many) in [("/ResearchTableWorld/LearnedItems", false), ("/ResearchTableWorld/ResearchedItems", true)] {
        let list = names(cx.root, p);
        if !list.is_empty() {
            let title = if many { trf("Researched items ({0})", &[&list.len()]) } else { trf("Learned items ({0})", &[&list.len()]) };
            panel(ui, &title, |ui| {
                ui.label(list.join(", "));
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn world() -> Value {
        json!({
            "Mode": {"value__": 0},
            "Behaviours": [
                {"BlockIndex": 274, "ObjectIndex": 1},
                {"BlockIndex": 42, "ObjectIndex": 7, "Slots": [
                    {"SlotIndex": 0, "ItemIndex": 21, "ItemAmount": 3, "ItemUses": 1},
                    {"SlotIndex": 1, "ItemIndex": 32, "ItemAmount": 1, "ItemUses": 90},
                    {"SlotIndex": 2, "ItemIndex": 53, "ItemAmount": 20, "ItemUses": 1},
                    {"SlotIndex": 3, "ItemIndex": 10, "ItemAmount": 2, "ItemUses": 1}
                ]},
                {"BlockIndex": 389, "ObjectIndex": 8, "Slots": []}
            ]
        })
    }

    #[test]
    fn item_table_is_sorted_and_names_resolve() {
        assert!(items().windows(2).all(|w| w[0].0 < w[1].0));
        assert_eq!(item(21), Some(("Plank", 20)));
        assert_eq!(item(42).unwrap().0, "Storage");
        assert!(MATERIALS.iter().all(|&m| item(m).is_some_and(|(_, cap)| cap == 20)));
    }

    #[test]
    fn storage_boxes_are_found() {
        let w = world();
        let c = chests(&w);
        assert_eq!(c.len(), 2);
        assert_eq!(c[0].stacks.len(), 4);
        assert_eq!(c[0].stacks[0].amount_ptr, "/Behaviours/1/Slots/0/ItemAmount");
    }

    #[test]
    fn max_raises_materials_only() {
        let w = world();
        let mut cx = GameCtx { root: &w, baseline: &w, tree: None, edits: vec![], archive: None, open_entry: None };
        assert_eq!(max_materials(&mut cx), 1);
        assert_eq!(cx.edits, vec![("/Behaviours/1/Slots/0/ItemAmount".to_string(), json!(20))]);
    }

    /// `SAVE_EDITOR_RAFT=<world.rgd> cargo test --release --lib raft_real -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn raft_real() {
        let f = std::env::var("SAVE_EDITOR_RAFT").unwrap();
        let raw = std::fs::read(&f).unwrap();
        let dec = crate::formats::decode_with(&raw, &Default::default());
        let crate::formats::Content::Tree(t) = &dec.content else { panic!("not a tree") };
        let c = chests(&t.view);
        println!("{} boxes, {} stacks", c.len(), c.iter().map(|c| c.stacks.len()).sum::<usize>());
        assert!(!c.is_empty());
        assert!(crate::game_ui::detect(&t.kind_name, &t.view).is_some());
    }

    fn users() -> Value {
        json!({"Users": [{"WorldGuid": "f821cb83-f78a-4e5a-bbdd-1daf19b30303", "Player": {"Inventory": {"Slots": [
            {"SlotIndex": 0, "ItemIndex": 35, "ItemAmount": 1, "ItemUses": 40},
            {"SlotIndex": 1, "ItemIndex": 21, "ItemAmount": 12, "ItemUses": 1},
            {"SlotIndex": 2, "ItemIndex": 17, "ItemAmount": 2, "ItemUses": 1},
            {"SlotIndex": 3, "ItemIndex": 23, "ItemAmount": 3, "ItemUses": 1},
            {"SlotIndex": 4, "ItemIndex": 53, "ItemAmount": 5, "ItemUses": 1},
            {"SlotIndex": 5, "ItemIndex": 22, "ItemAmount": 2, "ItemUses": 1},
            {"SlotIndex": 10, "ItemIndex": 25, "ItemAmount": 7, "ItemUses": 1},
            {"SlotIndex": 11, "ItemIndex": 50, "ItemAmount": 1, "ItemUses": 1}]}}}]})
    }

    #[test]
    fn player_hotbar_and_inventory_match_the_game() {
        let u = users();
        let p = players(&u);
        let v: Vec<(i64, String, i64)> = p[0].slots.iter().map(|s| (s.index, item_name(s.item), s.amount)).collect();
        assert_eq!(v[0].1, "Plastic hook");
        assert_eq!(v[1], (1, "Plank".to_string(), 12));
        assert_eq!(v[2].1, "Raw potato");
        assert_eq!(v[3], (3, "Scrap".to_string(), 3));
        assert_eq!(v[4], (4, "Plastic".to_string(), 5));
        assert_eq!(v[5], (5, "Rope".to_string(), 2));
        assert_eq!(v[6], (10, "Palm Leaf".to_string(), 7));
        assert_eq!(v[7], (11, "Stone".to_string(), 1));
    }

    #[test]
    fn player_max_raises_materials_only() {
        let u = users();
        let mut cx = GameCtx { root: &u, baseline: &u, tree: None, edits: vec![], archive: None, open_entry: None };
        // planks, scrap, plastic, rope, palm leaf and stone; the hook and potato stay.
        assert_eq!(max_player_materials(&mut cx), 6);
        assert!(cx.edits.iter().all(|(_, v)| *v == json!(20)));
    }

    /// `SAVE_EDITOR_RAFT_USERS=<RGD_Users.rgd> cargo test --release --lib raft_players_real -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn raft_players_real() {
        let f = std::env::var("SAVE_EDITOR_RAFT_USERS").unwrap();
        let raw = std::fs::read(&f).unwrap();
        let dec = crate::formats::decode_with(&raw, &Default::default());
        let crate::formats::Content::Tree(mut t) = dec.content else { panic!("not a tree") };
        assert!(crate::game_ui::detect(&t.kind_name, &t.view).is_some());
        let p = players(&t.view);
        assert!(!p.is_empty());
        let s = &p[p.len() - 1].slots[1];
        println!("world {} slot {} item {} x{}", p[p.len() - 1].world, s.index, item_name(s.item), s.amount);
        let ptr = s.amount_ptr.clone();
        t.set(&ptr, Value::from(20)).unwrap();
        let out = t.to_bytes().unwrap();
        assert_eq!(out.len(), raw.len());
        let dec2 = crate::formats::decode_with(&out, &Default::default());
        let crate::formats::Content::Tree(t2) = dec2.content else { panic!() };
        assert_eq!(t2.view.pointer(&ptr), Some(&Value::from(20)));
        let diff = raw.iter().zip(&out).filter(|(a, b)| a != b).count();
        println!("bytes changed: {diff}");
        assert!(diff <= 2);
    }
}
