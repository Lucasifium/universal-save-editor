//! The Sims (2000) `Neighborhood.iff`: one tab per family with its funds and, for each Sim
//! in it, personality and skills as bars out of ten.

use crate::i18n::{tr, trf};
use super::screens2::value_rows;
use super::{fact, hero, panel, tabs, GameCtx};
use crate::json_view::pointer_escape;
use eframe::egui::{Color32, Ui};
use serde_json::Value;

fn keys(cx: &GameCtx, base: &str) -> Vec<String> {
    match cx.get(base) {
        Some(Value::Object(m)) => m.keys().cloned().collect(),
        _ => vec![],
    }
}

pub fn show(ui: &mut Ui, cx: &mut GameCtx, two: bool) {
    let accent = if two { Color32::from_rgb(70, 160, 220) } else { Color32::from_rgb(88, 190, 96) };
    let families = keys(cx, "/Families");
    if families.is_empty() {
        ui.weak(tr("This neighbourhood has no families yet."));
        return;
    }
    let t = tabs(ui, "sims1_family", &families).min(families.len() - 1);
    let base = format!("/Families/{}", pointer_escape(&families[t]));
    let sims = keys(cx, &format!("{base}/Sims"));
    let mut sub = vec![if two { "The Sims 2" } else { "The Sims" }.to_string()];
    if let Some(h) = cx.text(&format!("{base}/{}", if two { "Lot" } else { "House" })) {
        sub.push(if h.starts_with("None") { tr("not moved in").into() } else if two { trf("lot {0}", &[&h]) } else { trf("house {0}", &[&h]) });
    }
    sub.push(if sims.len() == 1 { tr("1 Sim").to_string() } else { trf("{0} Sims", &[&sims.len()]) });
    hero(ui, accent, &families[t], &sub.join("  \u{00B7}  "), |ui| {
        cx.tile(ui, "Funds (\u{00A7})", &format!("{base}/Funds"), accent);
        if !two {
            cx.tile(ui, "Family friends", &format!("{base}/Family friends"), accent);
        }
        if let Some(v) = cx.int(&format!("{base}/House value")) {
            fact(ui, "House value", &super::fmt_num(v as f64), accent);
        }
    });
    for (n, sim) in sims.iter().enumerate() {
        let s = format!("{base}/Sims/{}", pointer_escape(sim));
        panel(ui, sim, |ui| {
            let mut about = cx.text(&format!("{s}/About")).unwrap_or_default();
            if let Some(c) = cx.text(&format!("{s}/Career")) {
                about = format!("{about}  \u{00B7}  {c}");
            }
            ui.weak(about); // i18n-ok: a name or value from the save, or already translated
            for group in ["Skills", "Personality"] {
                ui.add_space(4.0);
                ui.strong(crate::i18n::tr_any(group));
                let rows: Vec<(String, String)> =
                    keys(cx, &format!("{s}/{group}")).into_iter().map(|k| (k.clone(), format!("{s}/{group}/{}", pointer_escape(&k)))).collect();
                value_rows(ui, cx, &format!("sims1_{t}_{n}_{group}"), &rows);
            }
            let extra: Vec<(String, String)> =
                ["Job performance", "Aspiration score"].into_iter().filter(|k| cx.get(&format!("{s}/{k}")).is_some()).map(|k| (k.to_string(), format!("{s}/{k}"))).collect();
            if !extra.is_empty() {
                ui.add_space(4.0);
                value_rows(ui, cx, &format!("sims1_{t}_{n}_job"), &extra);
            }
        });
    }
    if two {
        ui.weak(tr("Skills and personality go from 0 to 1000; every 100 is one point on the game's bars. Sims are listed by their id (their names are in the character files). Townies and the default families are under Other Sims on the Values tab with Advanced on. Not yet tried in the game."));
        return;
    }
    ui.weak(tr("Skills and personality go from 0 to 1000; every 100 is one point on the game's bars. Sims carry their file name (user00012 is Characters\\User00012.iff). Townies and visitors are under Other Sims on the Values tab with Advanced on. Not yet tried in the game."));
}

/// The Sims 4: one tab per household with its funds and the Sims in it.
pub fn show4(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(60, 190, 200);
    let houses = keys(cx, "/Households");
    if houses.is_empty() {
        ui.weak(tr("This save has no households."));
        return;
    }
    let t = tabs(ui, "sims4_house", &houses).min(houses.len() - 1);
    let base = format!("/Households/{}", pointer_escape(&houses[t]));
    let sims = cx.text(&format!("{base}/Sims")).unwrap_or_default();
    let n = if sims.is_empty() { 0 } else { sims.matches("), ").count() + 1 };
    hero(ui, accent, &houses[t], &format!("The Sims 4  \u{00B7}  {}", if n == 1 { tr("1 Sim").to_string() } else { trf("{0} Sims", &[&n]) }), |ui| {
        cx.tile(ui, "Funds (\u{00A7})", &format!("{base}/Funds"), accent);
    });
    if !sims.is_empty() {
        panel(ui, "Household members", |ui| {
            for s in sims.split("), ") {
                ui.label(if s.ends_with(')') { s.to_string() } else { format!("{s})") });
            }
        });
    }
    ui.weak(tr("Funds go up to 99,999,999. The game data is stored uncompressed when you save, so the file grows. Sim skills and traits aren't mapped yet. Not yet tried in the game."));
}

/// The Sims 3: one tab per household (the most-played first) with its funds and, for each
/// Sim, reward points, fame, job and school performance, needs and skills.
pub fn show3(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(110, 200, 70);
    let houses = keys(cx, "/Households");
    if houses.is_empty() {
        ui.weak(tr("This save has no households."));
        return;
    }
    let t = tabs(ui, "sims3_house", &houses).min(houses.len() - 1);
    let base = format!("/Households/{}", pointer_escape(&houses[t]));
    let sims = keys(cx, &format!("{base}/Sims"));
    let group = |cx: &GameCtx, g: &str| -> Vec<(String, String)> {
        sims.iter()
            .flat_map(|s| {
                let sp = format!("{base}/Sims/{}/{g}", pointer_escape(s));
                keys(cx, &sp).into_iter().map(move |k| (k.clone(), format!("{sp}/{}", pointer_escape(&k))))
            })
            .collect()
    };
    let skills = group(cx, "Skills");
    let can = skills.iter().any(|(k, p)| cx.int(p).is_some_and(|v| v < crate::fmt::sims3::skill_max(k)));
    if super::quick_action(ui, "Max every skill", "Sets every listed skill of this household's Sims to its highest level.", can, "Every listed skill is already at its highest level.") {
        for (k, p) in &skills {
            cx.set(p, Value::from(crate::fmt::sims3::skill_max(k)));
        }
    }
    let needs = group(cx, "Needs");
    let low = needs.iter().any(|(_, p)| cx.int(p).is_some_and(|v| v < 100));
    if super::quick_action(ui, "Fill every need", "Sets every listed need of this household's Sims to full.", low, "Every listed need is already full.") {
        for (_, p) in &needs {
            cx.set(p, Value::from(100));
        }
    }
    let sub = format!("The Sims 3  \u{00B7}  {}", if sims.len() == 1 { tr("1 Sim").to_string() } else { trf("{0} Sims", &[&sims.len()]) });
    hero(ui, accent, &houses[t], &sub, |ui| {
        cx.tile(ui, "Funds (\u{00A7})", &format!("{base}/Funds"), accent);
    });
    for (n, sim) in sims.iter().enumerate() {
        let s = format!("{base}/Sims/{}", pointer_escape(sim));
        panel(ui, sim, |ui| {
            ui.horizontal_wrapped(|ui| {
                cx.tile(ui, "Lifetime reward points", &format!("{s}/Lifetime reward points"), accent);
                cx.tile(ui, "Lifetime happiness", &format!("{s}/Lifetime happiness"), accent);
                cx.tile(ui, "Celebrity level", &format!("{s}/Celebrity level"), accent);
                cx.tile(ui, "Celebrity points", &format!("{s}/Celebrity points"), accent);
                cx.tile(ui, "Job performance", &format!("{s}/Job performance"), accent);
                cx.tile(ui, "School performance", &format!("{s}/School performance"), accent);
                if let Some(o) = cx.int(&format!("{s}/Supernatural")) {
                    fact(ui, "Supernatural", &occult_names(o as u64), accent);
                }
                if let Some(l) = cx.int(&format!("{s}/Job level")) {
                    fact(ui, "Job level", &l.to_string(), accent);
                }
            });
            for (g, title) in [("Needs", tr("Needs")), ("Skills", tr("Skills"))] {
                let rows: Vec<(String, String)> = keys(cx, &format!("{s}/{g}")).into_iter().map(|k| (k.clone(), format!("{s}/{g}/{}", pointer_escape(&k)))).collect();
                if !rows.is_empty() {
                    ui.add_space(4.0);
                    ui.strong(title); // i18n-ok: a name or value from the save, or already translated
                    value_rows(ui, cx, &format!("sims3_{t}_{n}_{g}"), &rows);
                }
            }
        });
    }
    ui.weak(tr("Lifetime reward points are what the Rewards store spends; lifetime happiness is the total the Sim has ever earned. Needs go from -100 to 100 and are only kept for Sims the game has loaded. Job and school performance go from -100 to 100; celebrity level from 0 to 5. Every skill goes up to its own highest level (toddler and child skills to 1 or 3); a few expansion skills whose ids couldn't be matched to a name aren't listed. The supernatural type is shown but can't be changed here: turning a Sim into one needs data the game creates itself. Funds go up to 99,999,999. The game data is stored uncompressed when you save, so the file grows. Not yet tried in the game."));
}

/// The supernatural types in a Sims 3 occult flag set (none: human).
fn occult_names(flags: u64) -> String {
    let names: [(u64, &str); 13] = [
        (1, tr("Mummy")),
        (2, tr("SimBot")),
        (4, tr("Vampire")),
        (8, tr("Imaginary friend")),
        (16, tr("Unicorn")),
        (32, tr("Genie")),
        (64, tr("Werewolf")),
        (128, tr("Plant Sim")),
        (256, tr("Fairy")),
        (512, tr("Witch")),
        (1024, tr("Ghost")),
        (2048, tr("Mermaid")),
        (4096, tr("Plumbot")),
    ];
    let v: Vec<&str> = names.iter().filter(|(b, _)| flags & b != 0).map(|(_, n)| *n).collect();
    if v.is_empty() { tr("Human").to_string() } else { v.join(", ") }
}
