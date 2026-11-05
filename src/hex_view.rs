//! Hex editor with a data inspector and cheat-engine-style value search.

use eframe::egui::{self, Color32, RichText, ScrollArea, Sense, Ui};
use std::collections::{BTreeMap, HashSet};

const BYTES_PER_ROW: usize = 16;
pub const MAX_RESULTS: usize = 100_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumType {
    U8, I8, U16, I16, U32, I32, U64, I64, F32, F64, Ascii, Utf16,
}

impl NumType {
    pub const ALL: [NumType; 12] = [
        NumType::U8, NumType::I8, NumType::U16, NumType::I16, NumType::U32, NumType::I32,
        NumType::U64, NumType::I64, NumType::F32, NumType::F64, NumType::Ascii, NumType::Utf16,
    ];

    pub fn name(self) -> &'static str {
        match self {
            NumType::U8 => "u8", NumType::I8 => "i8",
            NumType::U16 => "u16", NumType::I16 => "i16",
            NumType::U32 => "u32", NumType::I32 => "i32",
            NumType::U64 => "u64", NumType::I64 => "i64",
            NumType::F32 => "f32", NumType::F64 => "f64",
            NumType::Ascii => "text (ASCII/UTF-8)", NumType::Utf16 => "text (UTF-16)",
        }
    }

    pub fn from_name(s: &str) -> Option<NumType> {
        NumType::ALL.into_iter().find(|t| t.name() == s)
    }

    pub fn size(self) -> usize {
        match self {
            NumType::U8 | NumType::I8 => 1,
            NumType::U16 | NumType::I16 => 2,
            NumType::U32 | NumType::I32 | NumType::F32 => 4,
            NumType::U64 | NumType::I64 | NumType::F64 => 8,
            NumType::Ascii | NumType::Utf16 => 0,
        }
    }

    fn is_float(self) -> bool {
        matches!(self, NumType::F32 | NumType::F64)
    }
}

macro_rules! num_bytes {
    ($t:ty, $s:expr, $be:expr) => {{
        let v: $t = $s.trim().parse().map_err(|e| format!("{e}"))?;
        if $be { v.to_be_bytes().to_vec() } else { v.to_le_bytes().to_vec() }
    }};
}

macro_rules! num_read {
    ($t:ty, $d:expr, $be:expr) => {{
        let arr = $d.try_into().ok()?;
        (if $be { <$t>::from_be_bytes(arr) } else { <$t>::from_le_bytes(arr) }).to_string()
    }};
}

/// Convert user text into the byte pattern for `ty`.
pub fn encode_value(ty: NumType, text: &str, be: bool) -> Result<Vec<u8>, String> {
    Ok(match ty {
        NumType::U8 => num_bytes!(u8, text, be),
        NumType::I8 => num_bytes!(i8, text, be),
        NumType::U16 => num_bytes!(u16, text, be),
        NumType::I16 => num_bytes!(i16, text, be),
        NumType::U32 => num_bytes!(u32, text, be),
        NumType::I32 => num_bytes!(i32, text, be),
        NumType::U64 => num_bytes!(u64, text, be),
        NumType::I64 => num_bytes!(i64, text, be),
        NumType::F32 => num_bytes!(f32, text, be),
        NumType::F64 => num_bytes!(f64, text, be),
        NumType::Ascii => {
            if text.is_empty() { return Err("empty text".into()); }
            text.as_bytes().to_vec()
        }
        NumType::Utf16 => {
            if text.is_empty() { return Err("empty text".into()); }
            text.encode_utf16()
                .flat_map(|u| if be { u.to_be_bytes() } else { u.to_le_bytes() })
                .collect()
        }
    })
}

/// Read a value of `ty` at `off` for display.
pub fn read_value(ty: NumType, data: &[u8], off: usize, be: bool) -> Option<String> {
    let size = ty.size();
    if size == 0 {
        return None;
    }
    let d = data.get(off..off + size)?;
    Some(match ty {
        NumType::U8 => d[0].to_string(),
        NumType::I8 => (d[0] as i8).to_string(),
        NumType::U16 => num_read!(u16, d, be),
        NumType::I16 => num_read!(i16, d, be),
        NumType::U32 => num_read!(u32, d, be),
        NumType::I32 => num_read!(i32, d, be),
        NumType::U64 => num_read!(u64, d, be),
        NumType::I64 => num_read!(i64, d, be),
        NumType::F32 => {
            let arr: [u8; 4] = d.try_into().ok()?;
            fmt_float(if be { f32::from_be_bytes(arr) } else { f32::from_le_bytes(arr) } as f64, true)
        }
        NumType::F64 => {
            let arr: [u8; 8] = d.try_into().ok()?;
            fmt_float(if be { f64::from_be_bytes(arr) } else { f64::from_le_bytes(arr) }, false)
        }
        NumType::Ascii | NumType::Utf16 => unreachable!(),
    })
}

/// Readable float: plain for everyday magnitudes, scientific for tiny/huge ones.
fn fmt_float(v: f64, single: bool) -> String {
    let a = v.abs();
    if v == 0.0 || (1e-4..1e15).contains(&a) {
        if single {
            (v as f32).to_string()
        } else {
            v.to_string()
        }
    } else if single {
        format!("{:e}", v as f32)
    } else {
        format!("{v:e}")
    }
}

fn float_at(ty: NumType, data: &[u8], off: usize, be: bool) -> Option<f64> {
    read_value(ty, data, off, be)?.parse().ok()
}

fn float_close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 0.001_f64.max(b.abs() * 1e-5)
}

/// Does the value at `off` equal `text` interpreted as `ty`?
pub fn matches_at(ty: NumType, data: &[u8], off: usize, text: &str, be: bool) -> bool {
    if ty.is_float() {
        let Ok(target) = text.trim().parse::<f64>() else { return false };
        return float_at(ty, data, off, be).is_some_and(|v| float_close(v, target));
    }
    match encode_value(ty, text, be) {
        Ok(pat) => data.get(off..off + pat.len()) == Some(&pat[..]),
        Err(_) => false,
    }
}

pub fn search(ty: NumType, data: &[u8], text: &str, be: bool) -> Result<Vec<usize>, String> {
    if ty.is_float() {
        let target: f64 = text.trim().parse().map_err(|e| format!("{e}"))?;
        let size = ty.size();
        return Ok((0..data.len().saturating_sub(size - 1))
            .filter(|&o| float_at(ty, data, o, be).is_some_and(|v| float_close(v, target)))
            .take(MAX_RESULTS)
            .collect());
    }
    let pat = encode_value(ty, text, be)?;
    Ok(data
        .windows(pat.len())
        .enumerate()
        .filter(|(_, w)| *w == &pat[..])
        .map(|(i, _)| i)
        .take(MAX_RESULTS)
        .collect())
}

/// A user-named offset ("Gold" at 0x40 as i32), persisted next to the save file.
#[derive(Clone)]
pub struct OffsetLabel {
    pub name: String,
    pub ty: NumType,
}

pub struct HexState {
    pub labels: BTreeMap<usize, OffsetLabel>,
    pub label_name: String,
    pub label_edits: BTreeMap<usize, String>,
    pub selected: usize,
    pub big_endian: bool,
    pub goto_text: String,
    pub scroll_to: Option<usize>,

    pub edit_type: NumType,
    pub edit_text: String,

    pub search_type: NumType,
    pub search_text: String,
    pub results: Vec<usize>,
    pub result_set: HashSet<usize>,
    pub searched: bool,
}

impl Default for HexState {
    fn default() -> Self {
        Self {
            labels: BTreeMap::new(),
            label_name: String::new(),
            label_edits: BTreeMap::new(),
            selected: 0,
            big_endian: false,
            goto_text: String::new(),
            scroll_to: None,
            edit_type: NumType::I32,
            edit_text: String::new(),
            search_type: NumType::I32,
            search_text: String::new(),
            results: Vec::new(),
            result_set: HashSet::new(),
            searched: false,
        }
    }
}

impl HexState {
    pub fn set_results(&mut self, results: Vec<usize>) {
        self.result_set = results.iter().copied().collect();
        self.results = results;
        self.searched = true;
    }

    pub fn jump(&mut self, off: usize) {
        self.selected = off;
        self.scroll_to = Some(off);
    }
}

/// The scrolling hex grid. Only visible rows are laid out, so large files stay fast.
pub fn hex_grid(ui: &mut Ui, data: &[u8], baseline: &[u8], st: &mut HexState) {
    let row_h = ui.text_style_height(&egui::TextStyle::Monospace) + 2.0;
    let rows = data.len().div_ceil(BYTES_PER_ROW).max(1);
    let search_len = match st.search_type {
        NumType::Ascii | NumType::Utf16 => encode_value(st.search_type, &st.search_text, st.big_endian).map(|p| p.len()).unwrap_or(1),
        t => t.size(),
    };

    let mut area = ScrollArea::vertical().auto_shrink([false, false]);
    if let Some(off) = st.scroll_to.take() {
        let row = off / BYTES_PER_ROW;
        let spacing = ui.spacing().item_spacing.y;
        area = area.vertical_scroll_offset(((row as f32) - 5.0).max(0.0) * (row_h + spacing));
    }

    area.show_rows(ui, row_h, rows, |ui, range| {
        for row in range {
            let start = row * BYTES_PER_ROW;
            let end = (start + BYTES_PER_ROW).min(data.len());
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 5.0;
                ui.monospace(RichText::new(format!("{start:08X}")).color(Color32::GRAY));
                ui.add_space(8.0);
                for off in start..start + BYTES_PER_ROW {
                    if off >= end {
                        ui.monospace("  ");
                        continue;
                    }
                    let in_hit = (off.saturating_sub(search_len.max(1) - 1)..=off)
                        .any(|s| st.result_set.contains(&s));
                    let label = st
                        .labels
                        .range(..=off)
                        .next_back()
                        .filter(|(&s, l)| off < s + l.ty.size().max(1));
                    let mut text = RichText::new(format!("{:02X}", data[off])).monospace();
                    if off == st.selected {
                        text = text.background_color(Color32::from_rgb(70, 80, 190)).color(Color32::WHITE);
                    } else if baseline.get(off) != Some(&data[off]) {
                        text = text.color(crate::theme::MODIFIED).strong();
                    } else if label.is_some() {
                        text = text.background_color(Color32::from_rgb(30, 110, 60)).color(Color32::WHITE);
                    } else if in_hit {
                        text = text.background_color(Color32::from_rgb(120, 90, 20)).color(Color32::WHITE);
                    } else if data[off] == 0 {
                        text = text.color(Color32::DARK_GRAY);
                    }
                    let mut resp = ui.add(egui::Label::new(text).sense(Sense::click()));
                    if let Some((_, l)) = label {
                        resp = resp.on_hover_text(format!("{} ({})", l.name, l.ty.name()));
                    }
                    if resp.clicked() {
                        st.selected = off;
                    }
                }
                ui.add_space(8.0);
                let ascii: String = data[start..end]
                    .iter()
                    .map(|&b| if b.is_ascii_graphic() || b == b' ' { b as char } else { '.' })
                    .collect();
                ui.monospace(ascii);
            });
        }
    });
}
