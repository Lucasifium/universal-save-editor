//! Oxygen Not Included: the map of each asteroid and rocket interior (pan, zoom, paint
//! elements, temperature and mass, clear germs, reveal), its points of interest, and the
//! worlds' and rockets' settings.

use super::{panel, select, GameCtx};
use crate::fmt::oni_map::{self as om, Cell, Op, OniMap, State};
use crate::json_view::pointer_escape;
use eframe::egui::{self, Color32, ColorImage, PointerButton, Pos2, Rect, RichText, Sense, Stroke, TextureHandle, TextureOptions, Ui, Vec2};
use serde_json::Value;
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Tool {
    #[default]
    Look,
    Paint,
    Vacuum,
    Heat,
    Mass,
    Clean,
}

const TOOLS: [(Tool, &str); 6] = [
    (Tool::Look, "Look / move"),
    (Tool::Paint, "Paint element"),
    (Tool::Vacuum, "Dig out (vacuum)"),
    (Tool::Heat, "Set temperature"),
    (Tool::Mass, "Set mass"),
    (Tool::Clean, "Clear germs"),
];

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Overlay {
    #[default]
    Elements,
    Temperature,
    Germs,
}

struct MapView {
    key: usize,
    applied: String,
    cells: Vec<Cell>,
    visible: Option<Vec<u8>>,
    world: i32,
    tex: Option<TextureHandle>,
    tex_world: i32,
    dirty: bool,
    zoom: f32,
    center: Vec2,
    tool: Tool,
    overlay: Overlay,
    size: u16,
    element: u32,
    mass: f32,
    temp_c: f32,
    neutronium: bool,
    dim: bool,
    stroke: Option<Op>,
    shown: Vec<bool>,
}

impl Default for MapView {
    fn default() -> Self {
        Self {
            key: 0,
            applied: String::new(),
            cells: vec![],
            visible: None,
            world: i32::MIN,
            tex: None,
            tex_world: i32::MIN,
            dirty: true,
            zoom: 0.0,
            center: Vec2::ZERO,
            tool: Tool::Look,
            overlay: Overlay::Elements,
            size: 3,
            element: om::hash("Water"),
            mass: 1000.0,
            temp_c: 20.0,
            neutronium: false,
            dim: false,
            stroke: None,
            shown: om::POI_KINDS.iter().map(|k| k.shown).collect(),
        }
    }
}

const KELVIN: f32 = 273.15;

impl MapView {
    fn stamp(&mut self, map: &OniMap, op: &Op, pt: (u32, u32)) {
        if op.base() == om::REVEAL {
            if let (Some(v), Some(w)) = (self.visible.as_mut(), map.world(op.world as i32)) {
                for y in w.y..w.y + w.h {
                    v[y * map.width + w.x..y * map.width + w.x + w.w].fill(255);
                }
                self.dirty = true;
            }
            return;
        }
        for (x, y) in op.stamp(pt, map) {
            let i = y * map.width + x;
            self.cells[i] = om::apply_cell(op, self.cells[i], map.protected[i]);
        }
        self.dirty = true;
    }

    fn apply_text(&mut self, map: &OniMap, text: &str) {
        for op in om::parse_ops(text).unwrap_or_default() {
            if op.base() == om::REVEAL {
                self.stamp(map, &op, (0, 0));
            }
            for &pt in &op.points {
                self.stamp(map, &op, pt);
            }
        }
    }

    fn sync(&mut self, map: &Arc<OniMap>, edits: &str) {
        let key = Arc::as_ptr(map) as usize;
        if self.key != key {
            self.key = key;
            self.cells = map.cells.clone();
            self.visible = map.visible.clone();
            self.applied.clear();
            self.dirty = true;
            if map.world(self.world).is_none() {
                self.world = map.worlds.iter().find(|w| w.start).or(map.worlds.first()).map_or(0, |w| w.id);
                self.zoom = 0.0;
            }
        }
        if self.applied == edits {
            return;
        }
        if !self.applied.is_empty() && edits.starts_with(&self.applied) && edits[self.applied.len()..].starts_with(';') {
            let more = edits[self.applied.len() + 1..].to_string();
            self.apply_text(map, &more);
        } else {
            self.cells.copy_from_slice(&map.cells);
            self.visible = map.visible.clone();
            self.apply_text(map, edits);
            self.dirty = true;
        }
        self.applied = edits.to_string();
    }

    fn color(&self, map: &OniMap, i: usize) -> Color32 {
        let c = &self.cells[i];
        let vac = c.element == om::vacuum();
        let rgb = match self.overlay {
            Overlay::Elements => om::color_of(c),
            Overlay::Temperature if vac => [10, 10, 14],
            Overlay::Temperature => heat(c.temp - KELVIN),
            Overlay::Germs if c.germs <= 0 => {
                let b = om::color_of(c);
                [b[0] / 4, b[1] / 4, b[2] / 4]
            }
            Overlay::Germs => {
                let k = ((c.germs as f32).log10() / 5.0).clamp(0.3, 1.0);
                let base = germ_color(c.germ);
                [(base[0] as f32 * k) as u8, (base[1] as f32 * k) as u8, (base[2] as f32 * k) as u8]
            }
        };
        let col = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
        if self.dim && self.visible.as_ref().is_some_and(|v| v[i] == 0) {
            col.linear_multiply(0.25)
        } else {
            col
        }
        .to_opaque()
        .gamma_multiply(if map.protected[i] && self.overlay == Overlay::Elements { 0.8 } else { 1.0 })
    }

    fn upload(&mut self, ctx: &egui::Context, map: &OniMap) {
        if !self.dirty && self.tex_world == self.world && self.tex.is_some() {
            return;
        }
        let Some(w) = map.world(self.world) else { return };
        let mut px = Vec::with_capacity(w.w * w.h);
        // Drawn with y up, like the game.
        for row in 0..w.h {
            let y = w.y + w.h - 1 - row;
            for x in w.x..w.x + w.w {
                px.push(self.color(map, y * map.width + x));
            }
        }
        let img = ColorImage { size: [w.w, w.h], pixels: px };
        let opts = TextureOptions { magnification: egui::TextureFilter::Nearest, minification: egui::TextureFilter::Linear, ..Default::default() };
        match &mut self.tex {
            Some(t) => t.set(img, opts),
            slot => *slot = Some(ctx.load_texture("oni_map", img, opts)),
        }
        self.dirty = false;
        self.tex_world = self.world;
    }
}

/// The game's thermal overlay, roughly: cold blue to hot red.
fn heat(c: f32) -> [u8; 3] {
    const STOPS: [(f32, [u8; 3]); 7] = [(-100.0, [40, 40, 160]), (-10.0, [60, 140, 230]), (10.0, [80, 200, 220]), (25.0, [80, 210, 90]), (40.0, [230, 220, 70]), (100.0, [240, 130, 40]), (400.0, [220, 30, 30])];
    if c <= STOPS[0].0 {
        return STOPS[0].1;
    }
    for w in STOPS.windows(2) {
        let ((a, ca), (b, cb)) = (w[0], w[1]);
        if c <= b {
            let f = (c - a) / (b - a);
            return [0, 1, 2].map(|k| (ca[k] as f32 + (cb[k] as f32 - ca[k] as f32) * f) as u8);
        }
    }
    STOPS[6].1
}

fn germ_color(id: u32) -> [u8; 3] {
    match om::germ_name(id).as_str() {
        "Food Poisoning" => [200, 160, 60],
        "Slimelung" => [120, 220, 80],
        "Floral Scents" => [230, 120, 200],
        "Zombie Spores" => [200, 80, 60],
        "Radioactive Contaminants" => [150, 255, 60],
        _ => [200, 200, 200],
    }
}

fn mass_text(kg: f32) -> String {
    if kg >= 1000.0 {
        format!("{:.1} t", kg / 1000.0)
    } else if kg >= 1.0 {
        format!("{kg:.1} kg")
    } else {
        format!("{:.0} g", kg * 1000.0)
    }
}

fn state() -> impl Fn(&mut Ui) -> Arc<Mutex<MapView>> {
    |ui: &mut Ui| ui.data_mut(|d| d.get_temp_mut_or_insert_with(egui::Id::new("oni_map"), || Arc::new(Mutex::new(MapView::default()))).clone())
}

pub fn map_of(cx: &GameCtx) -> Option<Arc<OniMap>> {
    cx.tree.and_then(|t| t.attachment()).and_then(|a| a.downcast::<OniMap>().ok())
}

pub fn map_tab(ui: &mut Ui, cx: &mut GameCtx) {
    let Some(map) = map_of(cx) else {
        ui.weak("The map couldn't be read from this save.");
        return;
    };
    let st = state()(ui);
    let mut guard = st.lock().unwrap_or_else(|e| e.into_inner());
    let v = &mut *guard;
    let edits = cx.text(om::EDITS).unwrap_or_default();
    if v.stroke.is_none() {
        v.sync(&map, &edits);
    }

    // Which world.
    ui.horizontal_wrapped(|ui| {
        ui.label("World:");
        for w in &map.worlds {
            let label = if w.rocket { format!("\u{1F680} {}", w.name) } else if w.start { format!("{} (start)", w.name) } else { w.name.clone() };
            let text = if w.discovered { RichText::new(label) } else { RichText::new(label).weak() };
            if ui.selectable_label(v.world == w.id, text).on_hover_text(if w.discovered { "" } else { "Not discovered yet" }).clicked() && v.world != w.id {
                v.world = w.id;
                v.zoom = 0.0;
                v.dirty = true;
            }
        }
    });
    let Some(world) = map.world(v.world).cloned() else { return };

    // Tools.
    ui.horizontal_wrapped(|ui| {
        for (t, label) in TOOLS {
            if ui.selectable_label(v.tool == t, label).clicked() {
                v.tool = t;
            }
        }
    });
    let painting = v.tool != Tool::Look;
    ui.horizontal_wrapped(|ui| {
        if v.tool == Tool::Paint {
            let list = map.paintable();
            let names: Vec<String> = list.iter().map(|e| format!("{} ({})", e.name, e.state.word())).collect();
            let cur = list.iter().position(|e| e.id == v.element).map_or_else(String::new, |i| names[i].clone());
            ui.label("Element:");
            if let Some(Value::String(s)) = crate::widgets::choice_editor(ui, "oni_element", &cur, &names, 240.0) {
                if let Some(i) = names.iter().position(|n| *n == s) {
                    let e = list[i];
                    v.element = e.id;
                    // A sensible amount for a full cell of it.
                    v.mass = match e.state {
                        State::Gas => 2.0,
                        State::Liquid => 1000.0,
                        _ => 1000.0,
                    };
                }
            }
        }
        let gas_or_vac = om::element(v.element).map(|e| e.state);
        if (v.tool == Tool::Paint && gas_or_vac != Some(State::Vacuum)) || v.tool == Tool::Mass {
            ui.label("Mass:");
            ui.add(egui::DragValue::new(&mut v.mass).range(0.001..=om::MAX_MASS).speed(1.0).suffix(" kg"));
        }
        if (v.tool == Tool::Paint && gas_or_vac != Some(State::Vacuum)) || v.tool == Tool::Heat {
            ui.label("Temperature:");
            ui.add(egui::DragValue::new(&mut v.temp_c).range(-273.0..=om::MAX_TEMP - KELVIN).speed(0.5).suffix(" \u{00B0}C"));
        }
        if painting {
            ui.label("Brush:");
            ui.add(egui::DragValue::new(&mut v.size).range(1..=32).suffix(" cells"));
            ui.checkbox(&mut v.neutronium, "Also change Neutronium");
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.label("Show:");
        for (o, label) in [(Overlay::Elements, "Elements"), (Overlay::Temperature, "Temperature"), (Overlay::Germs, "Germs")] {
            if ui.selectable_label(v.overlay == o, label).clicked() {
                v.overlay = o;
                v.dirty = true;
            }
        }
        if v.visible.is_some() {
            if ui.checkbox(&mut v.dim, "Dim unexplored").changed() {
                v.dirty = true;
            }
            if ui.button("Reveal this world's map").on_hover_text("Clears the fog over all of this world").clicked() {
                let op = Op { mode: om::REVEAL, world: world.id as u16, element: 0, mass: 0.0, temp: 0.0, size: 0, points: vec![] };
                let new = if edits.is_empty() { op.to_text() } else { format!("{edits};{}", op.to_text()) };
                cx.set(om::EDITS, Value::from(new));
            }
        }
        if ui.button("Fit").clicked() {
            v.zoom = 0.0;
        }
    });

    // The map.
    let max_h = (ui.ctx().screen_rect().height() * 0.62).clamp(260.0, 900.0);
    let (w, h) = (world.w as f32, world.h as f32);
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
    let to_local = |v: &MapView, p: Pos2| v.center + (p - rect.center()) / v.zoom;
    if resp.hovered() {
        let (scroll, pinch) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
        let factor = 1.0015f32.powf(scroll) * pinch;
        if (factor - 1.0).abs() > 1e-4 {
            if let Some(p) = resp.hover_pos() {
                let before = to_local(v, p);
                v.zoom = (v.zoom * factor).clamp(fit, 40.0);
                v.center = before - (p - rect.center()) / v.zoom;
            }
            ui.input_mut(|i| i.smooth_scroll_delta = Vec2::ZERO);
        }
    }
    let pan = resp.dragged_by(PointerButton::Secondary) || resp.dragged_by(PointerButton::Middle) || (!painting && resp.dragged_by(PointerButton::Primary));
    if pan {
        v.center -= resp.drag_delta() / v.zoom;
    }
    let half = rect.size() / v.zoom / 2.0;
    v.center.x = if half.x * 2.0 >= w { w / 2.0 } else { v.center.x.clamp(half.x, w - half.x) };
    v.center.y = if half.y * 2.0 >= h { h / 2.0 } else { v.center.y.clamp(half.y, h - half.y) };

    // Pointer → cell of the grid (y up).
    let hover = resp.hover_pos().map(|p| to_local(v, p)).filter(|t| t.x >= 0.0 && t.y >= 0.0 && t.x < w && t.y < h).map(|t| ((world.x + t.x as usize) as u32, (world.y + world.h - 1 - t.y as usize) as u32));

    let primary_down = ui.input(|i| i.pointer.primary_down());
    if painting && primary_down && resp.is_pointer_button_down_on() {
        if let Some(pt) = hover {
            let base = match v.tool {
                Tool::Paint => om::PAINT,
                Tool::Vacuum => om::VACUUM_OUT,
                Tool::Heat => om::HEAT,
                Tool::Mass => om::MASS,
                _ => om::CLEAN,
            };
            let mode = base | if v.neutronium { om::ALSO_NEUTRONIUM } else { 0 };
            let (size, temp) = (v.size, (v.temp_c + KELVIN).clamp(0.1, om::MAX_TEMP));
            let mut op = v.stroke.take().unwrap_or(Op { mode, world: world.id as u16, element: v.element, mass: v.mass, temp, size, points: vec![] });
            let last = op.points.last().copied();
            if last != Some(pt) {
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
            cx.set(om::EDITS, Value::from(new.clone()));
            v.applied = new;
        }
    }

    v.upload(ui.ctx(), &map);
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, Color32::BLACK);
    let to_screen = |x: f32, y: f32| rect.center() + (Vec2::new(x, y) - v.center) * v.zoom;
    // A grid cell's place on screen (its bottom-left corner in local coordinates is y+1).
    let cell_screen = |x: f32, y: f32| to_screen(x - world.x as f32, (world.y + world.h) as f32 - y);
    if let Some(t) = &v.tex {
        painter.image(t.id(), Rect::from_min_max(to_screen(0.0, 0.0), to_screen(w, h)), Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
    }
    let font = egui::FontId::proportional(12.0);
    let mut near: Option<(f32, usize)> = None;
    for (i, poi) in map.pois.iter().enumerate() {
        let (x, y) = (poi.x as usize, poi.y as usize);
        if !v.shown.get(poi.kind).copied().unwrap_or(true) || x < world.x || x >= world.x + world.w || y < world.y || y >= world.y + world.h {
            continue;
        }
        let k = &om::POI_KINDS[poi.kind];
        let p = cell_screen(poi.x as f32 + 0.5, poi.y as f32 + 0.5);
        if !rect.expand(8.0).contains(p) {
            continue;
        }
        let c = Color32::from_rgb(k.color[0], k.color[1], k.color[2]);
        if poi.kind == 6 {
            painter.circle(p, 4.5, c, Stroke::new(1.0_f32, Color32::BLACK));
        } else {
            let r = 5.0;
            let pts = vec![p + Vec2::new(0.0, -r), p + Vec2::new(r, 0.0), p + Vec2::new(0.0, r), p + Vec2::new(-r, 0.0)];
            painter.add(egui::Shape::convex_polygon(pts, c, Stroke::new(1.0_f32, Color32::BLACK)));
        }
        if let Some(hp) = resp.hover_pos() {
            let d = hp.distance(p);
            if d < 10.0 && near.is_none_or(|(best, _)| d < best) {
                near = Some((d, i));
            }
        }
        if v.zoom >= 5.0 {
            painter.text(p + Vec2::new(8.0, 0.0), egui::Align2::LEFT_CENTER, &poi.name, font.clone(), Color32::WHITE);
        }
    }
    if let Some((_, i)) = near {
        let poi = &map.pois[i];
        let p = cell_screen(poi.x as f32 + 0.5, poi.y as f32 + 0.5);
        let galley = painter.layout_no_wrap(poi.name.clone(), font.clone(), Color32::WHITE);
        let at = p + Vec2::new(10.0, -galley.size().y / 2.0);
        painter.rect_filled(Rect::from_min_size(at - Vec2::new(3.0, 1.0), galley.size() + Vec2::new(6.0, 2.0)), 3.0, Color32::from_black_alpha(200));
        painter.galley(at, galley, Color32::WHITE);
    }
    if let (true, Some((x, y))) = (painting, hover) {
        let half = ((v.size.max(1) - 1) / 2) as f32;
        let (x0, y0) = (x as f32 - half, y as f32 - half);
        let r = Rect::from_two_pos(cell_screen(x0, y0), cell_screen(x0 + v.size as f32, y0 + v.size as f32));
        painter.rect_stroke(r, 0.0, Stroke::new(1.5_f32, Color32::WHITE));
    }

    // What's under the pointer.
    let info = match hover {
        Some((x, y)) => {
            let i = y as usize * map.width + x as usize;
            let c = &v.cells[i];
            let lx = x as usize - world.x;
            let ly = y as usize - world.y;
            let mut parts = vec![format!("{lx}, {ly}"), om::element_name(c.element)];
            if c.element != om::vacuum() {
                parts.push(mass_text(c.mass));
                parts.push(format!("{:.1} \u{00B0}C", c.temp - KELVIN));
            }
            if c.germs > 0 {
                parts.push(format!("{} {}", c.germs, om::germ_name(c.germ)));
            }
            if let Some(r) = map.radiation.get(i).filter(|r| **r > 0.0) {
                parts.push(format!("{r:.0} rads"));
            }
            if map.protected[i] {
                parts.push("a tile is built here".into());
            }
            if v.visible.as_ref().is_some_and(|vis| vis[i] == 0) {
                parts.push("unexplored".into());
            }
            parts.join("  \u{00B7}  ")
        }
        None => "Scroll or pinch to zoom; drag to move (right-drag while painting).".into(),
    };
    ui.label(RichText::new(info).size(13.0));
    ui.weak(
        "Painting changes what fills each cell: its element, mass, temperature and germs. Buildings, critters, plants and debris are left alone, \
         and cells with a tile built in them aren't painted. Neutronium is kept unless you tick \u{201C}Also change Neutronium\u{201D}. \
         Each stroke is one undo step. Liquids and gases settle when the game runs.",
    );
    poi_panel(ui, &map, v, &world);
    if v.stroke.is_some() || v.dirty {
        ui.ctx().request_repaint();
    }
}

fn poi_panel(ui: &mut Ui, map: &OniMap, v: &mut MapView, world: &om::World) {
    let inside = |p: &om::Poi| (p.x as usize) >= world.x && (p.x as usize) < world.x + world.w && (p.y as usize) >= world.y && (p.y as usize) < world.y + world.h;
    let here: Vec<&om::Poi> = map.pois.iter().filter(|p| inside(p)).collect();
    let mut counts = vec![0usize; om::POI_KINDS.len()];
    for p in &here {
        counts[p.kind] += 1;
    }
    panel(ui, &format!("Shown on the map: {}", world.name), |ui| {
        ui.horizontal_wrapped(|ui| {
            for (k, kind) in om::POI_KINDS.iter().enumerate() {
                if counts[k] == 0 {
                    continue;
                }
                let c = Color32::from_rgb(kind.color[0], kind.color[1], kind.color[2]);
                let (r, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                ui.painter().rect_filled(r, 2.0, c);
                ui.checkbox(&mut v.shown[k], format!("{} ({})", kind.name, counts[k]));
            }
        });
        if here.is_empty() {
            ui.weak("Nothing special found in this world.");
            return;
        }
        let list: Vec<&&om::Poi> = here.iter().filter(|p| v.shown[p.kind]).collect();
        egui::CollapsingHeader::new(format!("List ({})", list.len())).id_salt("oni_poi_list").show(ui, |ui| {
            egui::ScrollArea::vertical().id_salt("oni_poi_scroll").max_height(200.0).show(ui, |ui| {
                for p in list {
                    let (lx, ly) = (p.x as usize - world.x, p.y as usize - world.y);
                    if ui.selectable_label(false, format!("{}  \u{00B7}  {lx}, {ly}", p.name)).on_hover_text("Show it on the map").clicked() {
                        v.center = Vec2::new(lx as f32 + 0.5, (world.h - ly) as f32 - 0.5);
                        v.zoom = v.zoom.max(6.0);
                        v.tool = Tool::Look;
                    }
                }
            });
        });
        ui.horizontal(|ui| {
            if ui.small_button("Show all").clicked() {
                v.shown.iter_mut().for_each(|s| *s = true);
            }
            if ui.small_button("Hide all").clicked() {
                v.shown.iter_mut().for_each(|s| *s = false);
            }
        });
        ui.weak("Duplicants start hidden. Positions are cells from the world's bottom-left corner.");
    });
}

/// The asteroids and rocket interiors with their settings, the rockets' names and what's
/// out on the star map.
pub fn worlds_tab(ui: &mut Ui, cx: &mut GameCtx, accent: Color32) {
    let keys: Vec<String> = cx.get("/Worlds").and_then(Value::as_object).map(|m| m.keys().cloned().collect()).unwrap_or_default();
    if keys.is_empty() {
        ui.weak("No worlds found in this save.");
        return;
    }
    let map = map_of(cx);
    for k in &keys {
        let base = format!("/Worlds/{}", pointer_escape(k));
        let kind = cx.text(&format!("{base}/Type")).unwrap_or_default();
        panel(ui, k, |ui| {
            egui::Grid::new(("oni_world", k)).num_columns(2).spacing([14.0, 6.0]).show(ui, |ui| {
                cx.grid_row(ui, "Name", &format!("{base}/Name"), 240.0);
                for f in ["Type", "Kind", "Size"] {
                    if let Some(t) = cx.text(&format!("{base}/{f}")) {
                        ui.label(f);
                        ui.label(t);
                        ui.end_row();
                    }
                }
                cx.grid_row(ui, "Sunlight (lux)", &format!("{base}/Sunlight (lux)"), 120.0);
                cx.grid_row(ui, "Cosmic radiation (rads)", &format!("{base}/Cosmic radiation (rads)"), 120.0);
            });
            ui.horizontal_wrapped(|ui| {
                cx.chip(ui, "Discovered", &format!("{base}/Discovered"), accent);
                cx.chip(ui, "Surface revealed", &format!("{base}/Surface revealed"), accent);
                if cx.flag(&format!("{base}/Start world")) {
                    ui.weak("The colony started here.");
                }
                if let Some(id) = map.as_ref().and_then(|m| m.worlds.iter().find(|w| w.key == *k)).map(|w| w.id) {
                    if ui.button("Open on the map").clicked() {
                        let st = state()(ui);
                        let mut g = st.lock().unwrap_or_else(|e| e.into_inner());
                        g.world = id;
                        g.zoom = 0.0;
                        g.dirty = true;
                        select(ui, "oni_tab", Some(1));
                    }
                }
            });
            if kind == "Rocket interior" {
                ui.weak("A rocket's habitat: its name is the rocket's name.");
            }
        });
    }
    ui.weak("Sunlight and cosmic radiation are what the world gets at its surface. Discovered puts the world on the star map; Surface revealed clears the fog above ground.");
    let rockets: Vec<String> = cx.get("/Rockets").and_then(Value::as_object).map(|m| m.keys().cloned().collect()).unwrap_or_default();
    if !rockets.is_empty() {
        panel(ui, &format!("Rockets ({})", rockets.len()), |ui| {
            egui::Grid::new("oni_rockets").num_columns(2).spacing([14.0, 6.0]).striped(true).show(ui, |ui| {
                for (i, r) in rockets.iter().enumerate() {
                    cx.grid_row(ui, &format!("Rocket {}", i + 1), &format!("/Rockets/{}/Name", pointer_escape(r)), 260.0);
                }
            });
            ui.weak("Rockets with a habitat module also have their interior as a world above, which can be painted on the Map.");
        });
    }
    if let Some(m) = map.filter(|m| !m.starmap.is_empty()) {
        panel(ui, "On the star map", |ui| {
            ui.horizontal_wrapped(|ui| {
                for (name, n) in &m.starmap {
                    ui.label(RichText::new(if *n > 1 { format!("{name} \u{00D7}{n}") } else { name.clone() }).color(accent));
                    ui.label("\u{00B7}");
                }
            });
            ui.weak("Space points of interest (harvestable fields, Gravitas stations, the Temporal Tear) are out on the star map, not in a world.");
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::{decode_with, Content, Options};

    fn bmp(path: &str, w: usize, h: usize, px: &[Color32]) {
        let row = (w * 3).div_ceil(4) * 4;
        let size = 54 + row * h;
        let mut b = vec![0u8; size];
        b[0..2].copy_from_slice(b"BM");
        b[2..6].copy_from_slice(&(size as u32).to_le_bytes());
        b[10..14].copy_from_slice(&54u32.to_le_bytes());
        b[14..18].copy_from_slice(&40u32.to_le_bytes());
        b[18..22].copy_from_slice(&(w as i32).to_le_bytes());
        b[22..26].copy_from_slice(&(-(h as i32)).to_le_bytes());
        b[26..28].copy_from_slice(&1u16.to_le_bytes());
        b[28..30].copy_from_slice(&24u16.to_le_bytes());
        for y in 0..h {
            for x in 0..w {
                let c = px[y * w + x];
                let o = 54 + y * row + x * 3;
                b[o..o + 3].copy_from_slice(&[c.b(), c.g(), c.r()]);
            }
        }
        std::fs::write(path, b).unwrap();
    }

    /// Headless check of the ONI screens: ONI_SAVE = a copy of a save, ONI_OUT = folder for
    /// pictures of each world. Draws every tab and world, paints a stroke with the mouse
    /// and checks the edit lands in the save.
    #[test]
    #[ignore]
    fn oni_screens() {
        let path = std::env::var("ONI_SAVE").expect("ONI_SAVE");
        let out = std::env::var("ONI_OUT").ok();
        let raw = std::fs::read(&path).unwrap();
        let d = decode_with(&raw, &Options::for_path(std::path::Path::new(&path)));
        let Content::Tree(mut doc) = d.content else { panic!("not a tree") };
        let game = crate::game_ui::detect(&doc.kind_name, &doc.view).unwrap();
        let map = doc.attachment().unwrap().downcast::<OniMap>().unwrap();
        let ctx = egui::Context::default();
        crate::theme::install_fonts(&ctx);
        let frame = |doc: &crate::tree::TreeDoc, tab: usize, events: Vec<egui::Event>, setup: &dyn Fn(&mut MapView)| -> Vec<(String, Value)> {
            let mut input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 1000.0))), ..Default::default() };
            input.events = events;
            let mut edits = vec![];
            let view = doc.view.clone();
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    select(ui, "oni_tab", Some(tab));
                    {
                        let st = state()(ui);
                        let mut g = st.lock().unwrap();
                        setup(&mut g);
                    }
                    let mut cx = GameCtx { root: &view, baseline: &view, tree: Some(doc), edits: vec![], archive: None, open_entry: None };
                    crate::game_ui::show(game, ui, &mut cx);
                    edits = cx.edits;
                });
            });
            edits
        };
        for tab in 0..3 {
            for _ in 0..2 {
                frame(&doc, tab, vec![], &|_| {});
            }
        }
        for w in &map.worlds {
            let id = w.id;
            for o in [Overlay::Elements, Overlay::Temperature, Overlay::Germs] {
                frame(&doc, 1, vec![], &|v: &mut MapView| {
                    if v.world != id || v.overlay != o {
                        v.world = id;
                        v.overlay = o;
                        v.zoom = 0.0;
                        v.dirty = true;
                    }
                });
            }
            if let Some(dir) = &out {
                let ctxd = egui::Context::default();
                let _ = ctxd.run(Default::default(), |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let st = state()(ui);
                        let mut v = st.lock().unwrap();
                        v.sync(&map, "");
                        let mut px = vec![];
                        for row in 0..w.h {
                            let y = w.y + w.h - 1 - row;
                            for x in w.x..w.x + w.w {
                                px.push(v.color(&map, y * map.width + x));
                            }
                        }
                        bmp(&format!("{dir}/world{id}.bmp"), w.w, w.h, &px);
                    });
                });
            }
        }
        // Paint: pick the start world, granite brush, drag across the middle of the map.
        let start = map.worlds.iter().find(|w| w.start).unwrap().id;
        let setup = move |v: &mut MapView| {
            if v.tool != Tool::Paint {
                v.world = start;
                v.overlay = Overlay::Elements;
                v.tool = Tool::Paint;
                v.element = om::hash("Granite");
                v.mass = 1500.0;
                v.temp_c = 30.0;
                v.size = 2;
                v.zoom = 0.0;
                v.dirty = true;
            }
        };
        frame(&doc, 1, vec![], &setup);
        // Find the map: the biggest image painted is hard to know headlessly, so scan down
        // the page for the first y where a press starts a stroke.
        let mut got = vec![];
        'find: for y in (200..900).step_by(20) {
            let p = egui::pos2(640.0, y as f32);
            let q = egui::pos2(700.0, y as f32);
            let ev = |pos, pressed| egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
            frame(&doc, 1, vec![egui::Event::PointerMoved(p), ev(p, true)], &setup);
            frame(&doc, 1, vec![egui::Event::PointerMoved(q)], &setup);
            frame(&doc, 1, vec![egui::Event::PointerMoved(q)], &setup);
            let e = frame(&doc, 1, vec![ev(q, false)], &setup);
            let e2 = frame(&doc, 1, vec![], &setup);
            for (p, v) in e.into_iter().chain(e2) {
                if p == om::EDITS {
                    got.push(v);
                    break 'find;
                }
            }
        }
        let text = got.pop().expect("a paint stroke made a map edit");
        println!("stroke: {}", &text.as_str().unwrap()[..text.as_str().unwrap().len().min(200)]);
        doc.set(om::EDITS, text).unwrap();
        let bytes = doc.to_bytes().unwrap();
        let again = decode_with(&bytes, &Options::for_path(std::path::Path::new(&path)));
        let Content::Tree(again) = again.content else { panic!() };
        let m2 = again.attachment().unwrap().downcast::<OniMap>().unwrap();
        let changed: Vec<usize> = (0..map.cells.len()).filter(|&i| map.cells[i] != m2.cells[i]).collect();
        println!("cells changed: {}", changed.len());
        assert!(!changed.is_empty());
        assert!(changed.iter().all(|&i| m2.cells[i].element == om::hash("Granite") && m2.cells[i].mass == 1500.0));
        let w = map.world(start).unwrap();
        assert!(changed.iter().all(|&i| (w.x..w.x + w.w).contains(&(i % map.width)) && (w.y..w.y + w.h).contains(&(i / map.width))));
    }
}
