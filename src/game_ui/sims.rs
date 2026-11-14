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
            ui.weak(about);
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
/// Sim, lifetime happiness, lifetime reward points and skills.
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
    let skill_paths: Vec<String> = sims
        .iter()
        .flat_map(|s| {
            let sp = format!("{base}/Sims/{}/Skills", pointer_escape(s));
            keys(cx, &sp).into_iter().map(move |k| format!("{sp}/{}", pointer_escape(&k)))
        })
        .collect();
    let max = crate::fmt::sims3::SKILL_MAX;
    let can = skill_paths.iter().any(|p| cx.int(p).is_some_and(|v| v < max));
    if super::quick_action(ui, "Max every skill", "Sets every listed skill of this household's Sims to level 10.", can, "Every listed skill is already at level 10.") {
        for p in &skill_paths {
            cx.set(p, Value::from(max));
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
            });
            let rows: Vec<(String, String)> = keys(cx, &format!("{s}/Skills")).into_iter().map(|k| (k.clone(), format!("{s}/Skills/{}", pointer_escape(&k)))).collect();
            if !rows.is_empty() {
                ui.add_space(4.0);
                ui.strong(tr("Skills"));
                value_rows(ui, cx, &format!("sims3_{t}_{n}_skills"), &rows);
            }
        });
    }
    ui.weak(tr("Lifetime reward points are what the Rewards store spends; lifetime happiness is the total the Sim has ever earned. Skills go from 0 to 10; skills kept only as hidden counters and toddler or child skills aren't listed. Funds go up to 99,999,999. The game data is stored uncompressed when you save, so the file grows. Not yet tried in the game."));
}
