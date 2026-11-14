//! Pokémon Infinite Fusion (and other Pokémon Essentials v19+ fan games): trainer,
//! badges, difficulty, party, PC boxes and bag.

use crate::i18n::{tr, trf};
use super::{cell_grid, columns, fact, hero, meter, panel, quick_button, select, selection, tabs, Cell, GameCtx};
use crate::fmt::essentials::{self, NATURES};
use eframe::egui::{self, Color32, RichText, Ui};
use serde_json::Value;

const ACCENT: Color32 = Color32::from_rgb(120, 90, 200);
const GOLD: Color32 = Color32::from_rgb(200, 160, 40);
const P: &str = "/Part 1/player";
const BOXES: &str = "/Part 1/storage_system/@boxes";
const POCKETS: [&str; 8] = ["Items", "Medicine", "Poké Balls", "TMs & HMs", "Berries", "Mail", "Battle Items", "Key Items"];
const BADGES: [&str; 16] = ["Boulder", "Cascade", "Thunder", "Rainbow", "Soul", "Marsh", "Volcano", "Earth", "Zephyr", "Hive", "Plain", "Fog", "Storm", "Mineral", "Glacier", "Rising"];
const STATS: [(&str, &str, &str); 6] = [("HP", "HP", "@totalhp"), ("ATTACK", "Atk", "@attack"), ("DEFENSE", "Def", "@defense"), ("SPECIAL_ATTACK", "Sp. Atk", "@spatk"), ("SPECIAL_DEFENSE", "Sp. Def", "@spdef"), ("SPEED", "Speed", "@speed")];

fn len(cx: &GameCtx, p: &str) -> usize {
    cx.get(p).and_then(Value::as_array).map_or(0, Vec::len)
}

/// ":THUNDER_STONE" → "Thunder Stone".
fn nice(sym: &str) -> String {
    let s = sym.trim_start_matches(':');
    if let Some(rest) = s.strip_prefix("DNA").filter(|r| !r.is_empty()) {
        return format!("DNA {}", nice(rest));
    }
    s
        .split('_')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            c.next().map_or_else(String::new, |f| f.to_uppercase().chain(c.flat_map(char::to_lowercase)).collect())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Species shown the way the game names it: the fusion's name and its head and body.
fn species(cx: &GameCtx, m: &str) -> (String, Option<String>) {
    let sd = format!("{m}/@species_data");
    let name = cx.text(&format!("{sd}/@real_name"));
    let head = cx.text(&format!("{sd}/@head_pokemon/@real_name"));
    let body = cx.text(&format!("{sd}/@body_pokemon/@real_name"));
    let sym = cx.text(&format!("{m}/@species")).unwrap_or_default();
    let parts = match (head, body) {
        (Some(h), Some(b)) => Some(crate::i18n::trf("{0} head, {1} body", &[&h, &b])),
        _ => fused_ids(&sym).map(|(b, h)| crate::i18n::trf("head #{0}, body #{1}", &[&h, &b])),
    };
    let name = name.unwrap_or_else(|| if fused_ids(&sym).is_some() { crate::i18n::tr("Fusion").into() } else { nice(&sym) });
    (name, parts)
}

/// ":B130H390" → (body 130, head 390).
fn fused_ids(sym: &str) -> Option<(u32, u32)> {
    let s = sym.trim_start_matches(':').strip_prefix('B')?;
    let (b, h) = s.split_once('H')?;
    Some((b.parse().ok()?, h.parse().ok()?))
}

fn is_mon(cx: &GameCtx, m: &str) -> bool {
    cx.text(&format!("{m}/__class__")).as_deref() == Some("Pokemon")
}

fn shiny(cx: &GameCtx, m: &str) -> bool {
    cx.get(&format!("{m}/@shiny")) == Some(&Value::Bool(true))
}

pub fn show(ui: &mut Ui, cx: &mut GameCtx) {
    let name = cx.text(&format!("{P}/@name")).unwrap_or_else(|| "Trainer".into());
    let mut sub = vec![];
    if let Some(id) = cx.int(&format!("{P}/@id")) {
        sub.push(crate::i18n::trf("ID No. {0}", &[&format!("{:05}", id & 0xFFFF)]));
    }
    let badges = (0..len(cx, &format!("{P}/@badges"))).filter(|i| cx.flag(&format!("{P}/@badges/{i}"))).count();
    sub.push(crate::i18n::trf("{0} badges", &[&badges]));
    if let Some(v) = cx.text("/Part 1/game_version") {
        sub.push(crate::i18n::trf("game version {0}", &[&v]));
    }
    hero(ui, ACCENT, &name, &sub.join("  \u{00B7}  "), |ui| {
        cx.tile(ui, "Money", &format!("{P}/@money"), ACCENT);
        cx.tile(ui, "Coins", &format!("{P}/@coins"), ACCENT);
        fact(ui, "Party", &format!("{} / 6", len(cx, &format!("{P}/@party"))), ACCENT);
    });

    let money = format!("{P}/@money");
    if cx.can_edit(&money) && quick_button(ui, tr("Max money"), tr("Sets money to 999,999, the most the game holds")) {
        cx.set(&money, Value::from(essentials::MAX_MONEY));
    }
    let party: Vec<String> = (0..len(cx, &format!("{P}/@party"))).map(|i| format!("{P}/@party/{i}")).filter(|m| is_mon(cx, m)).collect();
    if quick_button(ui, tr("Max IVs (party)"), tr("Sets every IV of the Pokémon in the party to 31; their stats are worked out again")) {
        for m in &party {
            for (k, _, _) in STATS {
                let p = format!("{m}/@iv/{k}");
                if cx.can_edit(&p) {
                    cx.set(&p, Value::from(31));
                }
            }
        }
    }

    panel(ui, "Trainer", |ui| {
        egui::Grid::new("pif_trainer").num_columns(2).spacing([16.0, 6.0]).show(ui, |ui| {
            cx.grid_row(ui, "Name", &format!("{P}/@name"), 160.0);
            cx.grid_row(ui, "Money", &money, 120.0);
            cx.grid_row(ui, "Coins", &format!("{P}/@coins"), 120.0);
            cx.grid_row(ui, "Battle Points", &format!("{P}/@battle_points"), 120.0);
            cx.grid_row(ui, "Soot", &format!("{P}/@soot"), 120.0);
        });
        difficulty(ui, cx);
    });

    let nb = len(cx, &format!("{P}/@badges"));
    if nb > 0 {
        panel(ui, "Badges", |ui| {
            ui.horizontal_wrapped(|ui| {
                for i in 0..nb {
                    let label = BADGES.get(i).map_or_else(|| crate::i18n::trf("Badge {0}", &[&(i + 1)]), |b| crate::i18n::trf("{0} Badge", &[b]));
                    cx.chip(ui, &label, &format!("{P}/@badges/{i}"), GOLD);
                }
            });
        });
    }

    panel(ui, "Party", |ui| {
        if party.is_empty() {
            ui.weak(tr("No Pokémon in the party."));
        }
        columns(ui, 340.0, party.len(), |ui, i| mon_card(ui, cx, &party[i], &format!("pif_party{i}")));
    });

    let nboxes = len(cx, BOXES);
    if nboxes > 0 {
        panel(ui, "PC boxes", |ui| {
            let labels: Vec<String> = (0..nboxes)
                .map(|b| {
                    let bp = format!("{BOXES}/{b}");
                    let name = cx.text(&format!("{bp}/@name")).unwrap_or_else(|| format!("Box {}", b + 1));
                    let n = (0..len(cx, &format!("{bp}/@pokemon"))).filter(|s| is_mon(cx, &format!("{bp}/@pokemon/{s}"))).count();
                    if n > 0 {
                        format!("{name} ({n})")
                    } else {
                        name
                    }
                })
                .collect();
            let t = tabs(ui, "pif_box_tab", &labels);
            let bp = format!("{BOXES}/{t}/@pokemon");
            let slots: Vec<String> = (0..len(cx, &bp)).map(|s| format!("{bp}/{s}")).collect();
            let cells: Vec<Cell> = slots
                .iter()
                .map(|m| {
                    let empty = !is_mon(cx, m);
                    let title = if empty { String::new() } else { format!("{}\nLv {}", species(cx, m).0, cx.int(&format!("{m}/@level")).unwrap_or(0)) };
                    Cell { title, count: None, empty, modified: false, tint: (!empty && shiny(cx, m)).then(|| Color32::from_rgb(80, 70, 20)) }
                })
                .collect();
            let id = format!("pif_box_sel_{t}");
            let sel = selection(ui, &id);
            if let Some(i) = cell_grid(ui, &cells, 6, sel, ACCENT) {
                select(ui, &id, if sel == Some(i) { None } else { Some(i) });
            }
            match selection(ui, &id).filter(|&i| i < slots.len() && is_mon(cx, &slots[i])) {
                Some(i) => mon_card(ui, cx, &slots[i], &id),
                None => {
                    ui.weak(tr("Click a Pokémon to edit it."));
                }
            }
        });
    }

    let npockets = len(cx, "/Part 1/bag/@pockets");
    if npockets > 1 {
        panel(ui, "Bag", |ui| {
            let names: Vec<String> = (1..npockets).map(|i| POCKETS.get(i - 1).map_or_else(|| crate::i18n::trf("Pocket {0}", &[&i]), |s| s.to_string())).collect();
            let t = tabs(ui, "pif_bag_tab", &names) + 1;
            let pp = format!("/Part 1/bag/@pockets/{t}");
            let n = len(cx, &pp);
            if n == 0 {
                ui.weak(tr("Nothing in this pocket."));
                return;
            }
            egui::Grid::new(("pif_bag", t)).num_columns(2).striped(true).spacing([12.0, 4.0]).show(ui, |ui| {
                for i in 0..n {
                    let item = cx.text(&format!("{pp}/{i}/0")).unwrap_or_default();
                    cx.label(ui, &nice(&item), &format!("{pp}/{i}/1"));
                    cx.editor(ui, &format!("{pp}/{i}/1"), 60.0);
                    ui.end_row();
                }
            });
            ui.weak(tr("Up to 999 of each item. Items you don't have yet can't be added here."));
        });
    }
}

/// Easy / Normal / Hard, which the game keeps as two switches.
fn difficulty(ui: &mut Ui, cx: &mut GameCtx) {
    let (e, h) = (essentials::SWITCH_EASY, essentials::SWITCH_HARD);
    if !(cx.can_edit(e) && cx.can_edit(h)) {
        return;
    }
    let cur = match (cx.flag(e), cx.flag(h)) {
        (true, false) => Some(0),
        (false, false) => Some(1),
        (false, true) => Some(2),
        _ => None,
    };
    let modified = cx.modified(e) || cx.modified(h);
    let mut pick = None;
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        let c = if modified { crate::theme::MODIFIED } else { ui.visuals().text_color() };
        ui.label(RichText::new(tr("Difficulty")).color(c));
        for (i, n) in ["Easy", "Normal", "Hard"].iter().enumerate() {
            if ui.selectable_label(cur == Some(i), crate::i18n::tr_any(n)).clicked() && cur != Some(i) {
                pick = Some(i);
            }
        }
    });
    if let Some(i) = pick {
        cx.set(e, Value::Bool(i == 0));
        cx.set(h, Value::Bool(i == 2));
    }
}

/// One Pokémon: name, level, nature and shininess, then IVs, EVs and the stats they give.
fn mon_card(ui: &mut Ui, cx: &mut GameCtx, m: &str, id: &str) {
    let f = |k: &str| format!("{m}/{k}");
    egui::Frame::none().fill(super::inset_fill(ui, ACCENT)).rounding(10.0).inner_margin(10.0).show(ui, |ui| {
        ui.set_width(ui.available_width());
        let (sp, parts) = species(cx, m);
        let nick = cx.text(&f("@name")).filter(|s| !s.is_empty());
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(nick.clone().unwrap_or_else(|| sp.clone())).size(16.0).strong());
            if nick.is_some() {
                ui.weak(&sp);
            }
            if shiny(cx, m) {
                ui.label(RichText::new(tr("\u{2605} Shiny")).color(GOLD));
            }
        });
        if let Some(p) = parts {
            ui.weak(p);
        }
        let types: Vec<String> = ["@type1", "@type2"].iter().filter_map(|t| cx.text(&f(&format!("@species_data/{t}")))).map(|t| nice(&t)).collect();
        let mut facts = vec![];
        if !types.is_empty() {
            facts.push(types.join(" / "));
        }
        if let Some(a) = cx.text(&f("@ability")) {
            facts.push(crate::i18n::trf("Ability: {0}", &[&nice(&a)]));
        }
        if let Some(it) = cx.text(&f("@item")) {
            facts.push(crate::i18n::trf("Holds {0}", &[&nice(&it)]));
        }
        if !facts.is_empty() {
            ui.label(RichText::new(facts.join("  \u{00B7}  ")).size(12.5).color(ui.visuals().weak_text_color()));
        }
        if let (Some(c), Some(t)) = (cx.num(&f("@hp")), cx.num(&f("@totalhp")).filter(|&t| t > 0.0)) {
            meter(ui, "HP", c, t, Color32::from_rgb(80, 180, 90));
        }
        egui::Grid::new((id, "main")).num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
            if cx.get(&f("@name")).is_some_and(Value::is_string) {
                cx.grid_row(ui, "Nickname", &f("@name"), 150.0);
            }
            cx.grid_row(ui, "Level", &f("@level"), 60.0);
            cx.grid_row(ui, "Exp. Points", &f("@exp"), 110.0);
            nature_row(ui, cx, m, id);
            cx.grid_row(ui, "Happiness", &f("@happiness"), 60.0);
            if cx.can_edit(&f("@shiny")) {
                let on = shiny(cx, m);
                let c = if cx.modified(&f("@shiny")) { crate::theme::MODIFIED } else { ui.visuals().text_color() };
                ui.label(RichText::new(tr("Shiny")).color(c));
                let mut v = on;
                if ui.checkbox(&mut v, "").changed() {
                    cx.set(&f("@shiny"), Value::Bool(v));
                }
                ui.end_row();
            }
        });
        if cx.get(&f("@species_data/@base_stats")).is_none() {
            ui.weak(tr("This save doesn't carry the species' data (older game versions): a level is refused when the growth rates this Pokémon could have disagree on its Exp., and the stats shown here change when the game next works them out (on a level-up)."));
        }
        ui.add_space(4.0);
        egui::Grid::new((id, "stats")).num_columns(4).spacing([10.0, 3.0]).show(ui, |ui| {
            ui.weak("");
            ui.weak(crate::i18n::tr("IV (0–31)"));
            ui.weak(crate::i18n::tr("EV (0–252)"));
            ui.weak(tr("Stat"));
            ui.end_row();
            for (k, label, stat) in STATS {
                ui.label(label);
                cx.editor(ui, &f(&format!("@iv/{k}")), 44.0);
                cx.editor(ui, &f(&format!("@ev/{k}")), 52.0);
                let c = if cx.modified(&f(stat)) { crate::theme::MODIFIED } else { ui.visuals().strong_text_color() };
                ui.label(RichText::new(cx.int(&f(stat)).map_or_else(String::new, |v| v.to_string())).color(c));
                ui.end_row();
            }
        });
        let total: i64 = STATS.iter().filter_map(|(k, _, _)| cx.int(&f(&format!("@ev/{k}")))).sum();
        let msg = crate::i18n::trf("EVs in all: {0} of 510", &[&total]);
        if total > 510 {
            ui.colored_label(Color32::from_rgb(220, 120, 60), format!("{msg}. {}", crate::i18n::tr("The game never gives more than 510 in all.")));
        } else {
            ui.weak(msg);
        }
        let moves: Vec<String> = (0..4).filter_map(|i| cx.text(&f(&format!("@moves/{i}/@id")))).map(|s| nice(&s)).collect();
        if !moves.is_empty() {
            ui.weak(trf("Moves: {0}", &[&(moves.join(", "))]));
        }
    });
    ui.add_space(6.0);
}

fn nature_row(ui: &mut Ui, cx: &mut GameCtx, m: &str, id: &str) {
    let p = format!("{m}/@nature");
    let Some(cur) = cx.text(&p) else { return };
    let c = if cx.modified(&p) { crate::theme::MODIFIED } else { ui.visuals().text_color() };
    ui.label(RichText::new(tr("Nature")).color(c));
    let mut pick = None;
    ui.add_enabled_ui(cx.can_edit(&p), |ui| {
        egui::ComboBox::from_id_salt((id, "nature")).selected_text(nice(&cur)).width(150.0).show_ui(ui, |ui| {
            for n in NATURES {
                let v = format!(":{n}");
                if ui.selectable_label(v == cur, nature_label(n)).clicked() && v != cur {
                    pick = Some(v);
                }
            }
        });
    });
    ui.end_row();
    if let Some(v) = pick {
        cx.set(&p, Value::String(v));
    }
    if let Some(s) = cx.text(&format!("{m}/@nature_for_stats")) {
        ui.weak(tr("Stats use"));
        ui.weak(trf("{0} (mint)", &[&(nice(&s))]));
        ui.end_row();
    }
}

/// "Adamant (+Atk, −Sp. Atk)".
fn nature_label(n: &str) -> String {
    const SHORT: [&str; 5] = ["Atk", "Def", "Speed", "Sp. Atk", "Sp. Def"];
    let i = NATURES.iter().position(|x| *x == n).unwrap_or(0);
    let (up, down) = (i / 5, i % 5);
    if up == down {
        crate::i18n::trf("{0} (neutral)", &[&nice(n)])
    } else {
        format!("{} (+{}, \u{2212}{})", nice(n), SHORT[up], SHORT[down])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_read_nicely() {
        assert_eq!(nice(":THUNDER_STONE"), "Thunder Stone");
        assert_eq!(nice(":DNASPLICERS"), "DNA Splicers");
        assert_eq!(fused_ids(":B130H390"), Some((130, 390)));
        assert_eq!(fused_ids(":BULBASAUR"), None);
        assert_eq!(nature_label("ADAMANT"), "Adamant (+Atk, \u{2212}Sp. Atk)");
        assert_eq!(nature_label("SERIOUS"), "Serious (neutral)");
    }
}
