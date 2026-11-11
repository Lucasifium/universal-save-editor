//! The Sims (2000) `Neighborhood.iff`: one tab per family with its funds and, for each Sim
//! in it, personality and skills as bars out of ten.

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

pub fn show(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(88, 190, 96);
    let families = keys(cx, "/Families");
    if families.is_empty() {
        ui.weak("This neighbourhood has no families yet.");
        return;
    }
    let t = tabs(ui, "sims1_family", &families).min(families.len() - 1);
    let base = format!("/Families/{}", pointer_escape(&families[t]));
    let sims = keys(cx, &format!("{base}/Sims"));
    let mut sub = vec!["The Sims".to_string()];
    if let Some(h) = cx.text(&format!("{base}/House")) {
        sub.push(if h.starts_with("None") { "not moved in".into() } else { format!("house {h}") });
    }
    sub.push(format!("{} Sim{}", sims.len(), if sims.len() == 1 { "" } else { "s" }));
    hero(ui, accent, &families[t], &sub.join("  \u{00B7}  "), |ui| {
        cx.tile(ui, "Funds (\u{00A7})", &format!("{base}/Funds"), accent);
        cx.tile(ui, "Family friends", &format!("{base}/Family friends"), accent);
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
                ui.strong(group);
                let rows: Vec<(String, String)> =
                    keys(cx, &format!("{s}/{group}")).into_iter().map(|k| (k.clone(), format!("{s}/{group}/{}", pointer_escape(&k)))).collect();
                value_rows(ui, cx, &format!("sims1_{t}_{n}_{group}"), &rows);
            }
            if cx.get(&format!("{s}/Job performance")).is_some() {
                ui.add_space(4.0);
                value_rows(ui, cx, &format!("sims1_{t}_{n}_job"), &[("Job performance".to_string(), format!("{s}/Job performance"))]);
            }
        });
    }
    ui.weak("Skills and personality go from 0 to 1000; every 100 is one point on the game's bars. Sims carry their file name (user00012 is Characters\\User00012.iff). Townies and visitors are under Other Sims on the Values tab with Advanced on. Not yet tried in the game.");
}
