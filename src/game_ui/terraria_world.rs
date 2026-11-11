//! Terraria world: a map you can pan, zoom and paint (blocks, walls, liquids, the spawn
//! point), the world's settings, bosses and events, chests, signs and town NPCs.

use super::terraria::slots;
use super::{fact, hero, panel, select, selection, tabs, GameCtx};
use crate::fmt::terraria_world::{self as tw, Op, WorldMap};
use crate::json_view::pointer_escape;
use eframe::egui::{self, Color32, ColorImage, PointerButton, Pos2, Rect, RichText, Sense, Stroke, TextureHandle, TextureOptions, Ui, Vec2};
use serde_json::Value;
use std::sync::{Arc, Mutex};

const ACCENT: Color32 = Color32::from_rgb(80, 150, 210);
const CHUNK: usize = 512;
const TABS: [&str; 6] = ["Map", "World", "Bosses & events", "Chests", "Signs", "NPCs"];

pub fn show(ui: &mut Ui, cx: &mut GameCtx) {
    let name = cx.text("/World/Name").unwrap_or_else(|| "World".into());
    let mut sub = vec![];
    for p in ["/World/Size", "/World/Difficulty", "/World/Evil"] {
        if let Some(t) = cx.text(p) {
            sub.push(t);
        }
    }
    if cx.flag("/World/Hardmode") {
        sub.push("Hardmode".into());
    }
    let count = |cx: &GameCtx, p: &str| cx.get(p).and_then(Value::as_object).map_or(0, |m| m.len());
    let (chests, signs, npcs) = (count(cx, "/Chests"), count(cx, "/Signs"), count(cx, "/NPCs"));
    hero(ui, ACCENT, &name, &sub.join("  \u{00B7}  "), |ui| {
        fact(ui, "Chests", &chests.to_string(), ACCENT);
        fact(ui, "Signs", &signs.to_string(), ACCENT);
        fact(ui, "Town NPCs", &npcs.to_string(), ACCENT);
    });
    let names: Vec<String> = TABS.iter().map(|s| s.to_string()).collect();
    match tabs(ui, "tw_tab", &names) {
        0 => map_tab(ui, cx),
        1 => world_tab(ui, cx),
        2 => progress_tab(ui, cx),
        3 => chests_tab(ui, cx),
        4 => signs_tab(ui, cx),
        _ => npcs_tab(ui, cx),
    }
}

// ------------------------------------------------------------------ settings tabs

fn keys(cx: &GameCtx, p: &str) -> Vec<String> {
    cx.get(p).and_then(Value::as_object).map(|m| m.keys().cloned().collect()).unwrap_or_default()
}

fn world_tab(ui: &mut Ui, cx: &mut GameCtx) {
    panel(ui, "World", |ui| {
        egui::Grid::new("tw_world").num_columns(2).spacing([14.0, 6.0]).striped(true).show(ui, |ui| {
            for k in keys(cx, "/World") {
                let p = format!("/World/{}", pointer_escape(&k));
                if matches!(cx.get(&p), Some(Value::Bool(_))) {
                    continue;
                }
                cx.grid_row(ui, &k, &p, 220.0);
            }
        });
        ui.horizontal_wrapped(|ui| {
            for k in keys(cx, "/World") {
                let p = format!("/World/{}", pointer_escape(&k));
                if matches!(cx.get(&p), Some(Value::Bool(_))) {
                    cx.chip(ui, &k, &p, ACCENT);
                }
            }
        });
        ui.weak("Time is in ticks: a day runs 0 to 54,000 from dawn, a night 0 to 32,400 from dusk. Spawn and dungeon are tile positions (you can also set the spawn point on the Map).");
    });
    chip_panel(ui, cx, "Special seeds", "/Special seeds", "Turning a seed on after the world was made changes some behaviour, not the terrain that was generated.");
}

fn chip_panel(ui: &mut Ui, cx: &mut GameCtx, title: &str, base: &str, note: &str) {
    let ks = keys(cx, base);
    if ks.is_empty() {
        return;
    }
    panel(ui, title, |ui| {
        ui.horizontal_wrapped(|ui| {
            for k in &ks {
                cx.chip(ui, k, &format!("{base}/{}", pointer_escape(k)), ACCENT);
            }
        });
        if !note.is_empty() {
            ui.weak(note);
        }
    });
}

fn progress_tab(ui: &mut Ui, cx: &mut GameCtx) {
    chip_panel(ui, cx, "Bosses defeated", "/Bosses defeated", "These are the flags the game checks (progress, NPCs moving in, the Bestiary and achievements aren't changed).");
    chip_panel(ui, cx, "Events", "/Events", "");
    chip_panel(ui, cx, "Town NPCs", "/Town NPCs", "A rescued NPC can move in once there's a free house.");
}

fn chests_tab(ui: &mut Ui, cx: &mut GameCtx) {
    let all = keys(cx, "/Chests");
    if all.is_empty() {
        ui.weak("This world has no chests.");
        return;
    }
    let fid = egui::Id::new("tw_chest_filter");
    let mut filter = ui.data(|d| d.get_temp::<String>(fid)).unwrap_or_default();
    panel(ui, "Chests", |ui| {
        ui.horizontal(|ui| {
            ui.label("Find:");
            ui.add(egui::TextEdit::singleline(&mut filter).hint_text("chest name or item").desired_width(240.0));
        });
        let f = filter.trim().to_lowercase();
        let rows: Vec<(usize, String)> = all
            .iter()
            .enumerate()
            .filter_map(|(i, k)| {
                let base = format!("/Chests/{}", pointer_escape(k));
                let c = cx.get(&base)?.as_object()?;
                let items: Vec<&str> = c.values().filter_map(|s| s.get("Item")?.as_str()).filter(|s| *s != "(empty)").collect();
                let name = c.get("Name").and_then(Value::as_str).unwrap_or("");
                if !f.is_empty() && !name.to_lowercase().contains(&f) && !items.iter().any(|it| it.to_lowercase().contains(&f)) {
                    return None;
                }
                let pos = c.get("Position").and_then(Value::as_str).unwrap_or("");
                let title = if name.is_empty() { k.clone() } else { format!("{k} \u{201C}{name}\u{201D}") };
                Some((i, format!("{title}  \u{00B7}  at {pos}  \u{00B7}  {} item{}", items.len(), if items.len() == 1 { "" } else { "s" })))
            })
            .collect();
        let sel = selection(ui, "tw_chest");
        egui::ScrollArea::vertical().id_salt("tw_chest_list").max_height(220.0).show(ui, |ui| {
            for (i, label) in &rows {
                if ui.selectable_label(sel == Some(*i), label).clicked() {
                    select(ui, "tw_chest", Some(*i));
                }
            }
            if rows.is_empty() {
                ui.weak("No chest matches.");
            }
        });
    });
    ui.data_mut(|d| d.insert_temp(fid, filter));
    let Some(i) = selection(ui, "tw_chest").filter(|&i| i < all.len()) else {
        ui.weak("Pick a chest to change what's in it (or click one on the Map).");
        return;
    };
    let base = format!("/Chests/{}", pointer_escape(&all[i]));
    panel(ui, &all[i], |ui| {
        egui::Grid::new("tw_chest_name").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            cx.grid_row(ui, "Name", &format!("{base}/Name"), 220.0);
            cx.grid_row(ui, "Position", &format!("{base}/Position"), 220.0);
        });
        let ks: Vec<String> = keys(cx, &base).into_iter().filter(|k| k.starts_with("Slot ")).collect();
        slots(ui, cx, &format!("tw_chest_slots_{i}"), &base, &ks, 10);
    });
}

fn signs_tab(ui: &mut Ui, cx: &mut GameCtx) {
    let all = keys(cx, "/Signs");
    if all.is_empty() {
        ui.weak("This world has no signs.");
        return;
    }
    panel(ui, "Signs", |ui| {
        egui::Grid::new("tw_signs").num_columns(2).spacing([14.0, 6.0]).striped(true).show(ui, |ui| {
            for k in &all {
                let base = format!("/Signs/{}", pointer_escape(k));
                let pos = cx.text(&format!("{base}/Position")).unwrap_or_default();
                cx.label(ui, &format!("{k} ({pos})"), &format!("{base}/Text"));
                cx.editor(ui, &format!("{base}/Text"), 420.0);
                ui.end_row();
            }
        });
    });
}

fn npcs_tab(ui: &mut Ui, cx: &mut GameCtx) {
    let all = keys(cx, "/NPCs");
    if all.is_empty() {
        ui.weak("No town NPCs live in this world yet.");
        return;
    }
    panel(ui, "Town NPCs", |ui| {
        egui::Grid::new("tw_npcs").num_columns(3).spacing([14.0, 4.0]).striped(true).show(ui, |ui| {
            ui.strong("Who");
            ui.strong("Name");
            ui.strong("Home");
            ui.end_row();
            for k in &all {
                let base = format!("/NPCs/{}", pointer_escape(k));
                for f in ["Role", "Name", "Home"] {
                    ui.label(cx.text(&format!("{base}/{f}")).unwrap_or_default());
                }
                ui.end_row();
            }
        });
        ui.weak("Homes are tile positions; they're shown on the Map.");
    });
}

// ------------------------------------------------------------------ map

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Tool {
    #[default]
    Look,
    Block,
    Wall,
    Liquid,
    EraseBlock,
    EraseWall,
    Drain,
    Spawn,
}

const TOOLS: [(Tool, &str); 8] = [
    (Tool::Look, "Look / move"),
    (Tool::Block, "Block"),
    (Tool::Wall, "Wall"),
    (Tool::Liquid, "Liquid"),
    (Tool::EraseBlock, "Erase block"),
    (Tool::EraseWall, "Erase wall"),
    (Tool::Drain, "Drain liquid"),
    (Tool::Spawn, "Set spawn"),
];

struct MapView {
    /// Which world this view was built for.
    key: usize,
    /// The map edits already applied to `cells`.
    applied: String,
    cells: Vec<u32>,
    tex: Vec<Option<TextureHandle>>,
    dirty: Vec<bool>,
    zoom: f32,
    center: Vec2,
    tool: Tool,
    size: u16,
    block: u16,
    wall: u16,
    liquid: u16,
    stroke: Option<Op>,
    /// Which kinds of points of interest are drawn (see `tw::POI_KINDS`).
    shown: Vec<bool>,
    /// Spawn, dungeon, town NPC homes, chests (see `MARKS`).
    marks: [bool; 4],
}

const MARKS: [&str; 4] = ["Spawn", "Dungeon", "Town NPC homes", "Chests"];

impl Default for MapView {
    fn default() -> Self {
        Self { key: 0, applied: String::new(), cells: vec![], tex: vec![], dirty: vec![], zoom: 0.0, center: Vec2::ZERO, tool: Tool::Look, size: 3, block: 1, wall: 4, liquid: 1, stroke: None, shown: tw::POI_KINDS.iter().map(|k| k.shown).collect(), marks: [true; 4] }
    }
}

impl MapView {
    fn chunks_x(map: &WorldMap) -> usize {
        map.width.div_ceil(CHUNK)
    }

    fn mark(&mut self, map: &WorldMap, x: usize, y: usize) {
        let i = (y / CHUNK) * Self::chunks_x(map) + x / CHUNK;
        if let Some(d) = self.dirty.get_mut(i) {
            *d = true;
        }
    }

    fn stamp(&mut self, map: &WorldMap, op: &Op, pt: (u32, u32)) {
        for (x, y) in op.stamp(pt, map.width, map.height) {
            let c = &mut self.cells[y * map.width + x];
            *c = tw::apply_packed(op.mode, op.value, *c, &map.important);
            self.mark(map, x, y);
        }
    }

    fn apply_text(&mut self, map: &WorldMap, text: &str) {
        for op in tw::parse_ops(text).unwrap_or_default() {
            for &pt in &op.points {
                self.stamp(map, &op, pt);
            }
        }
    }

    /// Brings the drawn map in line with the save's map edits (after undo, redo or a reload).
    fn sync(&mut self, map: &Arc<WorldMap>, edits: &str) {
        let key = Arc::as_ptr(map) as usize;
        if self.key != key {
            let shown = std::mem::take(&mut self.shown);
            *self = MapView { tool: self.tool, size: self.size, block: self.block, wall: self.wall, liquid: self.liquid, shown, marks: self.marks, ..Default::default() };
            self.key = key;
            self.cells = map.cells.clone();
            let n = Self::chunks_x(map) * map.height.div_ceil(CHUNK);
            self.tex = vec![None; n];
            self.dirty = vec![true; n];
        }
        if self.applied == edits {
            return;
        }
        if !self.applied.is_empty() && edits.starts_with(&self.applied) && edits[self.applied.len()..].starts_with(';') {
            let more = edits[self.applied.len() + 1..].to_string();
            self.apply_text(map, &more);
        } else {
            self.cells.copy_from_slice(&map.cells);
            self.dirty.iter_mut().for_each(|d| *d = true);
            self.apply_text(map, edits);
        }
        self.applied = edits.to_string();
    }

    fn upload(&mut self, ctx: &egui::Context, map: &WorldMap) {
        let cxn = Self::chunks_x(map);
        let colors = Colors::get();
        let opts = TextureOptions { magnification: egui::TextureFilter::Nearest, minification: egui::TextureFilter::Linear, ..Default::default() };
        for i in 0..self.dirty.len() {
            if !self.dirty[i] {
                continue;
            }
            self.dirty[i] = false;
            let (x0, y0) = ((i % cxn) * CHUNK, (i / cxn) * CHUNK);
            let (w, h) = (CHUNK.min(map.width - x0), CHUNK.min(map.height - y0));
            let mut px = Vec::with_capacity(w * h);
            for y in y0..y0 + h {
                let row = &self.cells[y * map.width + x0..y * map.width + x0 + w];
                px.extend(row.iter().map(|&p| colors.of(p, y, map)));
            }
            let img = ColorImage { size: [w, h], pixels: px };
            match &mut self.tex[i] {
                Some(t) => t.set(img, opts),
                slot => *slot = Some(ctx.load_texture(format!("tw_map_{i}"), img, opts)),
            }
        }
    }
}

struct Colors {
    tiles: Vec<Color32>,
    walls: Vec<Color32>,
}

impl Colors {
    fn get() -> &'static Colors {
        static C: std::sync::OnceLock<Colors> = std::sync::OnceLock::new();
        C.get_or_init(|| {
            let p = tw::palette();
            let conv = |k: &Option<tw::Kind>, fallback: Color32| k.as_ref().map_or(fallback, |k| Color32::from_rgb(k.color[0], k.color[1], k.color[2]));
            Colors {
                tiles: p.tiles.iter().map(|k| conv(k, Color32::from_rgb(200, 0, 200))).collect(),
                walls: p.walls.iter().map(|k| conv(k, Color32::from_rgb(60, 50, 60))).collect(),
            }
        })
    }

    fn of(&self, p: u32, y: usize, map: &WorldMap) -> Color32 {
        if let Some(t) = tw::packed_tile(p) {
            let c = self.tiles.get(t as usize).copied().unwrap_or(Color32::from_rgb(200, 0, 200));
            return if tw::packed_actuated(p) { c.linear_multiply(0.5) } else { c };
        }
        match tw::packed_liquid(p) {
            1 => return Color32::from_rgb(9, 61, 191),
            2 => return Color32::from_rgb(253, 32, 3),
            3 => return Color32::from_rgb(254, 194, 20),
            4 => return Color32::from_rgb(190, 150, 230),
            _ => {}
        }
        let w = tw::packed_wall(p);
        if w != 0 {
            return self.walls.get(w as usize).copied().unwrap_or(Color32::from_rgb(60, 50, 60));
        }
        let y = y as f64;
        if y < map.surface {
            Color32::from_rgb(132, 170, 248)
        } else if y < map.rock {
            Color32::from_rgb(88, 61, 46)
        } else if y < map.height as f64 - 200.0 {
            Color32::from_rgb(74, 67, 60)
        } else {
            Color32::from_rgb(51, 20, 18)
        }
    }
}

fn paintable_blocks(map: &WorldMap) -> (Vec<String>, Vec<u16>) {
    let p = tw::palette();
    let v: Vec<(String, u16)> = (0..map.important.len())
        .filter(|&t| !map.important[t])
        .filter_map(|t| p.tiles.get(t)?.as_ref().map(|k| (format!("#{t} {}", k.name), t as u16)))
        .collect();
    v.into_iter().unzip()
}

fn paintable_walls() -> (Vec<String>, Vec<u16>) {
    let p = tw::palette();
    let v: Vec<(String, u16)> = (1..p.walls.len().min(0x400)).filter_map(|w| p.walls[w].as_ref().map(|k| (format!("#{w} {}", k.name), w as u16))).collect();
    v.into_iter().unzip()
}

fn map_tab(ui: &mut Ui, cx: &mut GameCtx) {
    let Some(map) = cx.tree.and_then(|t| t.attachment()).and_then(|a| a.downcast::<WorldMap>().ok()) else {
        ui.weak("The map isn't available for this file.");
        return;
    };
    let state = ui.data_mut(|d| d.get_temp_mut_or_insert_with(egui::Id::new("tw_map"), || Arc::new(Mutex::new(MapView::default()))).clone());
    let mut guard = state.lock().unwrap_or_else(|e| e.into_inner());
    let v = &mut *guard;
    let edits = cx.text(tw::EDITS).unwrap_or_default();
    if v.stroke.is_none() {
        v.sync(&map, &edits);
    }

    // Tools.
    ui.horizontal_wrapped(|ui| {
        for (t, label) in TOOLS {
            if ui.selectable_label(v.tool == t, label).clicked() {
                v.tool = t;
            }
        }
    });
    ui.horizontal_wrapped(|ui| {
        let painting = !matches!(v.tool, Tool::Look | Tool::Spawn);
        match v.tool {
            Tool::Block => {
                let (names, ids) = paintable_blocks(&map);
                let cur = ids.iter().position(|&i| i == v.block).map_or_else(String::new, |i| names[i].clone());
                ui.label("Block:");
                if let Some(Value::String(s)) = crate::widgets::choice_editor(ui, "tw_block", &cur, &names, 240.0) {
                    if let Some(i) = names.iter().position(|n| *n == s) {
                        v.block = ids[i];
                    }
                }
            }
            Tool::Wall => {
                let (names, ids) = paintable_walls();
                let cur = ids.iter().position(|&i| i == v.wall).map_or_else(String::new, |i| names[i].clone());
                ui.label("Wall:");
                if let Some(Value::String(s)) = crate::widgets::choice_editor(ui, "tw_wall", &cur, &names, 240.0) {
                    if let Some(i) = names.iter().position(|n| *n == s) {
                        v.wall = ids[i];
                    }
                }
            }
            Tool::Liquid => {
                ui.label("Liquid:");
                let n = if map.shimmer_ok() { 4 } else { 3 };
                for (i, name) in tw::LIQUIDS.iter().take(n).enumerate() {
                    if ui.selectable_label(v.liquid == i as u16 + 1, *name).clicked() {
                        v.liquid = i as u16 + 1;
                    }
                }
            }
            _ => {}
        }
        if painting {
            ui.label("Brush:");
            ui.add(egui::DragValue::new(&mut v.size).range(1..=32).suffix(" tiles"));
        }
        if ui.button("Fit").clicked() {
            v.zoom = 0.0;
        }
    });

    // The map itself.
    // The box is never taller than the map: when the whole world fits, it hugs it (no
    // empty bands above and below).
    let max_h = (ui.ctx().screen_rect().height() * 0.62).clamp(260.0, 900.0);
    let (w, h) = (map.width as f32, map.height as f32);
    let width = ui.available_width();
    let fit = (width / w).min(max_h / h);
    if v.zoom > 0.0 {
        v.zoom = v.zoom.max(fit);
    }
    let height = (h * if v.zoom > 0.0 { v.zoom } else { fit }).clamp(120.0, max_h);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click_and_drag());
    if v.zoom <= 0.0 {
        v.zoom = fit;
        v.center = Vec2::new(w / 2.0, h / 2.0);
    }
    let to_tile = |v: &MapView, p: Pos2| v.center + (p - rect.center()) / v.zoom;
    if resp.hovered() {
        let (scroll, pinch) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
        let factor = 1.0015f32.powf(scroll) * pinch;
        if (factor - 1.0).abs() > 1e-4 {
            if let Some(p) = resp.hover_pos() {
                let before = to_tile(v, p);
                v.zoom = (v.zoom * factor).clamp(fit, 24.0);
                v.center = before - (p - rect.center()) / v.zoom;
            }
            ui.input_mut(|i| i.smooth_scroll_delta = Vec2::ZERO);
        }
    }
    let painting = !matches!(v.tool, Tool::Look | Tool::Spawn);
    let pan = resp.dragged_by(PointerButton::Secondary) || resp.dragged_by(PointerButton::Middle) || (!painting && resp.dragged_by(PointerButton::Primary));
    if pan {
        v.center -= resp.drag_delta() / v.zoom;
    }
    // Keep the view on the world.
    let half = rect.size() / v.zoom / 2.0;
    v.center.x = if half.x * 2.0 >= w { w / 2.0 } else { v.center.x.clamp(half.x, w - half.x) };
    v.center.y = if half.y * 2.0 >= h { h / 2.0 } else { v.center.y.clamp(half.y, h - half.y) };

    let hover_tile = resp.hover_pos().map(|p| to_tile(v, p)).filter(|t| t.x >= 0.0 && t.y >= 0.0 && t.x < w && t.y < h).map(|t| (t.x as u32, t.y as u32));

    // Painting: a stroke runs from press to release and becomes one map edit (one undo step).
    let primary_down = ui.input(|i| i.pointer.primary_down());
    if painting && primary_down && resp.is_pointer_button_down_on() {
        if let Some(pt) = hover_tile {
            let (mode, value) = match v.tool {
                Tool::Block => (tw::PAINT_BLOCK, v.block),
                Tool::Wall => (tw::PAINT_WALL, v.wall),
                Tool::Liquid => (tw::PAINT_LIQUID, v.liquid),
                Tool::EraseBlock => (tw::ERASE_BLOCK, 0),
                Tool::EraseWall => (tw::ERASE_WALL, 0),
                _ => (tw::DRAIN_LIQUID, 0),
            };
            let size = v.size;
            let mut op = v.stroke.take().unwrap_or(Op { mode, value, size, points: vec![] });
            let last = op.points.last().copied();
            if last != Some(pt) {
                // Fill in between fast mouse moves so the stroke has no gaps.
                let mut pts = vec![];
                if let Some((lx, ly)) = last {
                    let (dx, dy) = (pt.0 as f32 - lx as f32, pt.1 as f32 - ly as f32);
                    let step = (size as f32 / 2.0).max(1.0);
                    let n = (dx.abs().max(dy.abs()) / step).ceil() as u32;
                    for k in 1..n {
                        let f = k as f32 / n as f32;
                        pts.push(((lx as f32 + dx * f).round() as u32, (ly as f32 + dy * f).round() as u32));
                    }
                }
                pts.push(pt);
                for p in pts {
                    v.stamp(&map, &op, p);
                    op.points.push(p);
                }
            }
            v.stroke = Some(op);
        }
    } else if let Some(op) = v.stroke.take() {
        if !op.points.is_empty() {
            let new = if edits.is_empty() { op.to_text() } else { format!("{edits};{}", op.to_text()) };
            cx.set(tw::EDITS, Value::from(new.clone()));
            v.applied = new;
        }
    }
    if resp.clicked_by(PointerButton::Primary) {
        if let Some((x, y)) = hover_tile {
            match v.tool {
                Tool::Spawn => {
                    cx.set("/World/Spawn X", Value::from(x));
                    cx.set("/World/Spawn Y", Value::from(y));
                }
                Tool::Look => {
                    if let Some(i) = map.chests.iter().position(|&(cxp, cyp)| (cxp..cxp + 2).contains(&(x as i32)) && (cyp..cyp + 2).contains(&(y as i32))) {
                        select(ui, "tw_chest", Some(i));
                        select(ui, "tw_tab", Some(3));
                    }
                }
                _ => {}
            }
        }
    }

    v.upload(ui.ctx(), &map);
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, Color32::BLACK);
    let to_screen = |x: f32, y: f32| rect.center() + (Vec2::new(x, y) - v.center) * v.zoom;
    let cxn = MapView::chunks_x(&map);
    for (i, t) in v.tex.iter().enumerate() {
        let Some(t) = t else { continue };
        let (x0, y0) = ((i % cxn) * CHUNK, (i / cxn) * CHUNK);
        let [tw_, th] = t.size();
        let r = Rect::from_min_max(to_screen(x0 as f32, y0 as f32), to_screen((x0 + tw_) as f32, (y0 + th) as f32));
        if r.intersects(rect) {
            painter.image(t.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
        }
    }
    // Markers: chests (when zoomed in), town NPC homes, the spawn point.
    if v.marks[3] && v.zoom >= 2.0 {
        for &(x, y) in &map.chests {
            let r = Rect::from_min_max(to_screen(x as f32, y as f32), to_screen(x as f32 + 2.0, y as f32 + 2.0));
            if r.intersects(rect) {
                painter.rect_stroke(r, 1.0, Stroke::new(1.5_f32, Color32::from_rgb(255, 210, 60)));
            }
        }
    }
    let font = egui::FontId::proportional(12.0);
    for (role, name, x, y, homeless) in &map.npcs {
        if *homeless || !v.marks[2] {
            continue;
        }
        let p = to_screen(*x as f32 + 0.5, *y as f32 - 1.0);
        painter.circle(p, 5.0, Color32::from_rgb(240, 240, 240), Stroke::new(1.0_f32, Color32::BLACK));
        if v.zoom >= 1.5 {
            let label = if name.is_empty() { role.clone() } else { format!("{name} ({role})") };
            painter.text(p + Vec2::new(8.0, 0.0), egui::Align2::LEFT_CENTER, label, font.clone(), Color32::WHITE);
        }
    }
    let mut near: Option<(f32, usize)> = None;
    for (i, poi) in map.pois.iter().enumerate() {
        if !v.shown.get(poi.kind).copied().unwrap_or(true) {
            continue;
        }
        let k = &tw::POI_KINDS[poi.kind];
        let p = to_screen(poi.x as f32 + 0.5, poi.y as f32 + 0.5);
        if !rect.expand(8.0).contains(p) {
            continue;
        }
        let c = Color32::from_rgb(k.color[0], k.color[1], k.color[2]);
        let r = if poi.size > 1 { 5.5 } else { 4.0 };
        // A diamond, so points of interest stand apart from the round NPC and spawn markers.
        let pts = vec![p + Vec2::new(0.0, -r), p + Vec2::new(r, 0.0), p + Vec2::new(0.0, r), p + Vec2::new(-r, 0.0)];
        painter.add(egui::Shape::convex_polygon(pts, c, Stroke::new(1.0_f32, Color32::BLACK)));
        if let Some(hp) = resp.hover_pos() {
            let d = hp.distance(p);
            if d < 10.0 && near.is_none_or(|(best, _)| d < best) {
                near = Some((d, i));
            }
        }
        if v.zoom >= 3.0 {
            painter.text(p + Vec2::new(8.0, 0.0), egui::Align2::LEFT_CENTER, k.name, font.clone(), Color32::WHITE);
        }
    }
    if let Some((_, i)) = near {
        let poi = &map.pois[i];
        let p = to_screen(poi.x as f32 + 0.5, poi.y as f32 + 0.5);
        let k = &tw::POI_KINDS[poi.kind];
        let text = if poi.size > 1 {
            format!("{} ({} tiles)", k.name, poi.size)
        } else if !poi.note.is_empty() {
            format!("{}: {}", k.name, poi.note)
        } else {
            k.name.to_string()
        };
        let galley = painter.layout_no_wrap(text, font.clone(), Color32::WHITE);
        let at = p + Vec2::new(10.0, -galley.size().y / 2.0);
        painter.rect_filled(Rect::from_min_size(at - Vec2::new(3.0, 1.0), galley.size() + Vec2::new(6.0, 2.0)), 3.0, Color32::from_black_alpha(200));
        painter.galley(at, galley, Color32::WHITE);
    }
    if let (true, Some(sx), Some(sy)) = (v.marks[0], cx.int("/World/Spawn X"), cx.int("/World/Spawn Y")) {
        let p = to_screen(sx as f32 + 0.5, sy as f32 + 0.5);
        painter.circle(p, 6.0, Color32::from_rgb(60, 220, 90), Stroke::new(1.5_f32, Color32::BLACK));
        painter.text(p + Vec2::new(9.0, 0.0), egui::Align2::LEFT_CENTER, "Spawn", font.clone(), Color32::WHITE);
    }
    if let (true, Some(dx), Some(dy)) = (v.marks[1], cx.int("/World/Dungeon X"), cx.int("/World/Dungeon Y")) {
        let p = to_screen(dx as f32 + 0.5, dy as f32 + 0.5);
        painter.circle(p, 5.0, Color32::from_rgb(80, 120, 255), Stroke::new(1.5_f32, Color32::BLACK));
        painter.text(p + Vec2::new(9.0, 0.0), egui::Align2::LEFT_CENTER, "Dungeon", font.clone(), Color32::WHITE);
    }
    if let (true, Some((x, y))) = (painting, hover_tile) {
        let half = ((v.size.max(1) - 1) / 2) as f32;
        let x0 = x as f32 - half;
        let y0 = y as f32 - half;
        let r = Rect::from_min_max(to_screen(x0, y0), to_screen(x0 + v.size as f32, y0 + v.size as f32));
        painter.rect_stroke(r, 0.0, Stroke::new(1.5_f32, Color32::WHITE));
    }

    // What's under the pointer.
    let info = match hover_tile {
        Some((x, y)) => {
            let p = v.cells[y as usize * map.width + x as usize];
            let feet_x = (x as i64 * 2 - map.width as i64).abs();
            let side = if (x as usize) * 2 >= map.width { "east" } else { "west" };
            let depth = ((y as f64 - map.surface) * 2.0).round() as i64;
            let level = if depth > 0 { format!("{depth} ft below the surface") } else { format!("{} ft above the surface", -depth) };
            let mut parts = vec![format!("{x}, {y}"), format!("{feet_x} ft {side}, {level}")];
            if let Some(t) = tw::packed_tile(p) {
                parts.push(format!("Block: {}{}", tw::tile_name(t), if tw::packed_actuated(p) { " (actuated)" } else { "" }));
            }
            let wl = tw::packed_wall(p);
            if wl != 0 {
                parts.push(format!("Wall: {}", tw::wall_name(wl)));
            }
            if let l @ 1..=4 = tw::packed_liquid(p) {
                parts.push(tw::LIQUIDS[l as usize - 1].to_string());
            }
            parts.join("  \u{00B7}  ")
        }
        None => "Scroll or pinch to zoom; drag to move (right-drag while painting). Click a chest with Look / move to open it.".into(),
    };
    ui.label(RichText::new(info).size(13.0));
    ui.weak(
        "Painting changes blocks, walls and liquids; furniture, chests, trees and other objects are left alone. Each stroke is one undo step. \
         Liquid only fills empty tiles and settles when the game runs.",
    );
    poi_panel(ui, &map, v);
    if v.stroke.is_some() || !v.dirty.iter().all(|d| !d) {
        ui.ctx().request_repaint();
    }
}

/// Points of interest: a toggle per kind (with how many there are) and a list to jump to them.
fn poi_panel(ui: &mut Ui, map: &WorldMap, v: &mut MapView) {
    let mut counts = vec![0usize; tw::POI_KINDS.len()];
    for p in &map.pois {
        counts[p.kind] += 1;
    }
    panel(ui, "Shown on the map", |ui| {
        ui.horizontal_wrapped(|ui| {
            for (i, m) in MARKS.iter().enumerate() {
                ui.checkbox(&mut v.marks[i], *m);
            }
        });
        ui.horizontal_wrapped(|ui| {
            for (k, kind) in tw::POI_KINDS.iter().enumerate() {
                if counts[k] == 0 {
                    continue;
                }
                let c = Color32::from_rgb(kind.color[0], kind.color[1], kind.color[2]);
                let (r, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                ui.painter().rect_filled(r, 2.0, c);
                let mut on = v.shown[k];
                if ui.checkbox(&mut on, format!("{} ({})", kind.name, counts[k])).changed() {
                    v.shown[k] = on;
                }
            }
        });
        if map.pois.is_empty() {
            ui.weak("Nothing special found in this world.");
            return;
        }
        let list: Vec<&tw::Poi> = map.pois.iter().filter(|p| v.shown[p.kind]).collect();
        egui::CollapsingHeader::new(format!("List ({})", list.len())).id_salt("tw_poi_list").show(ui, |ui| {
            egui::ScrollArea::vertical().id_salt("tw_poi_scroll").max_height(200.0).show(ui, |ui| {
                for p in list {
                    let k = &tw::POI_KINDS[p.kind];
                    let depth = ((p.y as f64 - map.surface) * 2.0).round() as i64;
                    let level = if depth > 0 { format!("{depth} ft deep") } else { "surface".to_string() };
                    if ui.selectable_label(false, format!("{}{}  \u{00B7}  {}, {}  \u{00B7}  {level}", k.name, if p.note.is_empty() { String::new() } else { format!(": {}", p.note) }, p.x, p.y)).on_hover_text("Show it on the map").clicked() {
                        v.center = Vec2::new(p.x as f32, p.y as f32);
                        v.zoom = v.zoom.max(4.0);
                        v.tool = Tool::Look;
                    }
                }
            });
        });
        ui.horizontal(|ui| {
            if ui.small_button("Show all").clicked() {
                v.shown.iter_mut().for_each(|s| *s = true);
                v.marks = [true; 4];
            }
            if ui.small_button("Hide all").clicked() {
                v.shown.iter_mut().for_each(|s| *s = false);
                v.marks = [false; 4];
            }
        });
        ui.weak("Chests are drawn when you zoom in. Life Crystals and Life Fruit are common, so they start hidden.");
    });
}
