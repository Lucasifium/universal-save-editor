//! Hex editor with a data inspector and cheat-engine-style value search.

use eframe::egui::{self, Color32, RichText, ScrollArea, Sense, Ui};
use std::collections::{BTreeMap, HashSet};

const BYTES_PER_ROW: usize = 16;
pub const MAX_RESULTS: usize = 100_000;
/// When this few locations are left, the guided search says to look at them and name one.
pub const GUIDE_FEW: usize = 5;

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

/// How a scan keeps or drops locations. The first four start a scan; the rest compare the
/// value each location held before with what it holds now (after the game saved again, or in a
/// second save file).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScanMode {
    Exact,
    Between,
    Unknown,
    Changed,
    Unchanged,
    Increased,
    Decreased,
    IncreasedBy,
    DecreasedBy,
}

impl ScanMode {
    /// Modes that start a new scan.
    pub const FIRST: [ScanMode; 3] = [ScanMode::Exact, ScanMode::Between, ScanMode::Unknown];
    /// Modes that narrow an existing scan.
    pub const NEXT: [ScanMode; 8] = [
        ScanMode::Exact, ScanMode::Changed, ScanMode::Unchanged, ScanMode::Increased,
        ScanMode::Decreased, ScanMode::IncreasedBy, ScanMode::DecreasedBy, ScanMode::Between,
    ];

    pub fn name(self) -> &'static str {
        match self {
            ScanMode::Exact => crate::i18n::tr("Exact value"),
            ScanMode::Between => crate::i18n::tr("Value between"),
            ScanMode::Unknown => crate::i18n::tr("Unknown starting value"),
            ScanMode::Changed => crate::i18n::tr("Changed"),
            ScanMode::Unchanged => crate::i18n::tr("Unchanged"),
            ScanMode::Increased => crate::i18n::tr("Increased"),
            ScanMode::Decreased => crate::i18n::tr("Decreased"),
            ScanMode::IncreasedBy => crate::i18n::tr("Increased by"),
            ScanMode::DecreasedBy => crate::i18n::tr("Decreased by"),
        }
    }

    /// How many numbers the mode needs typed in (0, 1 or 2).
    pub fn inputs(self) -> usize {
        match self {
            ScanMode::Exact | ScanMode::IncreasedBy | ScanMode::DecreasedBy => 1,
            ScanMode::Between => 2,
            _ => 0,
        }
    }

    /// Does the mode compare an earlier copy of the data with a later one?
    pub fn compares(self) -> bool {
        matches!(self, ScanMode::Changed | ScanMode::Unchanged | ScanMode::Increased | ScanMode::Decreased | ScanMode::IncreasedBy | ScanMode::DecreasedBy)
    }
}

/// The number at `off`, for comparisons. Text types have no number.
pub fn number_at(ty: NumType, data: &[u8], off: usize, be: bool) -> Option<f64> {
    let d = data.get(off..off + ty.size().max(1))?;
    macro_rules! n {
        ($t:ty) => {{
            let a = d.try_into().ok()?;
            (if be { <$t>::from_be_bytes(a) } else { <$t>::from_le_bytes(a) }) as f64
        }};
    }
    Some(match ty {
        NumType::U8 => d[0] as f64,
        NumType::I8 => d[0] as i8 as f64,
        NumType::U16 => n!(u16),
        NumType::I16 => n!(i16),
        NumType::U32 => n!(u32),
        NumType::I32 => n!(i32),
        NumType::U64 => n!(u64),
        NumType::I64 => n!(i64),
        NumType::F32 => n!(f32),
        NumType::F64 => n!(f64),
        NumType::Ascii | NumType::Utf16 => return None,
    })
}

fn parse_num(text: &str) -> Result<f64, String> {
    text.trim().parse::<f64>().map_err(|_| format!("\"{}\" is not a number", text.trim()))
}

fn near(ty: NumType, a: f64, b: f64) -> bool {
    if ty.is_float() { float_close(a, b) } else { a == b }
}

/// Runs one scan step.
///
/// `before` is the data the existing locations were found in (or `None` on a first scan),
/// `now` the data to test. `candidates` are the locations still in play; `None` means every
/// location (a first scan, or after an unknown-value start). `aligned` tests only offsets that
/// are a multiple of the type's size, which is how most games lay numbers out.
#[allow(clippy::too_many_arguments)]
pub fn scan(
    ty: NumType,
    mode: ScanMode,
    before: Option<&[u8]>,
    now: &[u8],
    candidates: Option<&[usize]>,
    a: &str,
    b: &str,
    be: bool,
    aligned: bool,
) -> Result<Vec<usize>, String> {
    let size = ty.size();
    if size == 0 {
        // Text: only an exact search makes sense.
        if mode != ScanMode::Exact {
            return Err("Comparison scans work on numbers \u{2014} pick a number type".into());
        }
        let pat = encode_value(ty, a, be)?;
        let hits = |o: &usize| now.get(*o..*o + pat.len()) == Some(&pat[..]);
        return Ok(match candidates {
            Some(c) => c.iter().copied().filter(hits).collect(),
            None => search(ty, now, a, be)?,
        });
    }
    if mode == ScanMode::Unknown {
        return Err("An unknown-value scan keeps every location; it has nothing to test yet".into());
    }
    if mode.compares() && before.is_none() {
        return Err("This scan compares with an earlier copy \u{2014} start a scan first".into());
    }
    let (x, y) = match mode.inputs() {
        0 => (0.0, 0.0),
        1 => (parse_num(a)?, 0.0),
        _ => {
            let (lo, hi) = (parse_num(a)?, parse_num(b)?);
            (lo.min(hi), lo.max(hi))
        }
    };
    let test = |o: usize| -> bool {
        let Some(v) = number_at(ty, now, o, be) else { return false };
        if ty.is_float() && !v.is_finite() {
            return false;
        }
        let old = || before.and_then(|d| number_at(ty, d, o, be));
        let raw_same = || before.and_then(|d| d.get(o..o + size)) == now.get(o..o + size);
        match mode {
            ScanMode::Exact => near(ty, v, x),
            ScanMode::Between => v >= x && v <= y,
            ScanMode::Unknown => true,
            ScanMode::Changed => !raw_same(),
            ScanMode::Unchanged => raw_same(),
            ScanMode::Increased => old().is_some_and(|p| v > p),
            ScanMode::Decreased => old().is_some_and(|p| v < p),
            ScanMode::IncreasedBy => old().is_some_and(|p| near(ty, v - p, x)),
            ScanMode::DecreasedBy => old().is_some_and(|p| near(ty, p - v, x)),
        }
    };
    let out: Vec<usize> = match candidates {
        Some(c) => c.iter().copied().filter(|&o| test(o)).take(MAX_RESULTS).collect(),
        None => {
            let step = if aligned { size } else { 1 };
            let end = now.len().saturating_sub(size - 1);
            (0..end).step_by(step).filter(|&o| test(o)).take(MAX_RESULTS).collect()
        }
    };
    Ok(out)
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
    /// Scan options: how the first scan and each refine pick locations.
    pub first_mode: ScanMode,
    pub next_mode: ScanMode,
    pub search_text2: String,
    pub aligned: bool,
    /// An unknown-value scan is running: every location is still in play.
    pub all_locations: bool,
    /// Refine by itself each time the game writes the file again.
    pub auto_refine: bool,
    /// The file's (modified time in ms, size) when last read, for `auto_refine`.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub watch: Option<(u128, u64)>,
    /// When the file was last looked at for `auto_refine` (window time, seconds).
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub watch_checked: f64,
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
            first_mode: ScanMode::Exact,
            next_mode: ScanMode::Exact,
            search_text2: String::new(),
            aligned: true,
            all_locations: false,
            auto_refine: false,
            watch: None,
            watch_checked: 0.0,
        }
    }
}

impl HexState {
    pub fn set_results(&mut self, results: Vec<usize>) {
        self.all_locations = false;
        self.result_set = results.iter().copied().collect();
        self.results = results;
        self.searched = true;
    }

    /// Candidates for the next scan step (`None` while an unknown-value scan keeps everything).
    pub fn candidates(&self) -> Option<&[usize]> {
        if self.all_locations { None } else { Some(&self.results) }
    }

    /// Starts a scan on `data` with the first-scan mode.
    pub fn start_scan(&mut self, data: &[u8]) -> Result<String, String> {
        if self.first_mode == ScanMode::Unknown {
            self.set_results(Vec::new());
            self.all_locations = true;
            return Ok("Every location is in play. Change the value in-game, save, then refine with Increased, Decreased or Changed.".into());
        }
        let res = scan(self.search_type, self.first_mode, None, data, None, &self.search_text, &self.search_text2, self.big_endian, self.aligned)?;
        let n = res.len();
        self.set_results(res);
        Ok(format!("{n} possible location(s)"))
    }

    /// Narrows the scan: `before` is the data the current results came from, `now` the new data.
    pub fn refine(&mut self, before: &[u8], now: &[u8]) -> Result<(usize, usize), String> {
        let was = if self.all_locations { now.len() / if self.aligned { self.search_type.size().max(1) } else { 1 } } else { self.results.len() };
        let res = scan(self.search_type, self.next_mode, Some(before), now, self.candidates(), &self.search_text, &self.search_text2, self.big_endian, self.aligned)?;
        let n = res.len();
        self.set_results(res);
        Ok((was, n))
    }

    /// Which step of the guided search the player is on: 1 search for a number, 2 narrow it
    /// down by changing it in the game, 3 name one of the few left, 4 change it.
    pub fn guide_step(&self) -> usize {
        let scanning = self.searched && (self.all_locations || !self.results.is_empty());
        if !scanning {
            1
        } else if self.all_locations || self.results.len() > GUIDE_FEW {
            2
        } else if self.results.iter().any(|o| self.labels.contains_key(o)) {
            4
        } else {
            3
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn file(vals: &[i32]) -> Vec<u8> {
        vals.iter().flat_map(|v| v.to_le_bytes()).collect()
    }

    #[test]
    fn guide_steps() {
        let mut h = HexState::default();
        assert_eq!(h.guide_step(), 1);
        h.set_results((0..40).collect());
        assert_eq!(h.guide_step(), 2);
        h.set_results(vec![4, 8]);
        assert_eq!(h.guide_step(), 3);
        h.labels.insert(8, OffsetLabel { name: "Gold".into(), ty: NumType::U32 });
        assert_eq!(h.guide_step(), 4);
        h.set_results(vec![]);
        assert_eq!(h.guide_step(), 1);
    }

    #[test]
    fn unknown_then_compare_scans() {
        let before = file(&[10, 500, 7, 7, 900]);
        let after = file(&[10, 450, 7, 9, 905]);
        let mut st = HexState { first_mode: ScanMode::Unknown, ..Default::default() };
        st.start_scan(&before).unwrap();
        assert!(st.all_locations);
        st.next_mode = ScanMode::Decreased;
        st.refine(&before, &after).unwrap();
        assert_eq!(st.results, vec![4]);

        let mut st = HexState { first_mode: ScanMode::Unknown, ..Default::default() };
        st.start_scan(&before).unwrap();
        st.next_mode = ScanMode::IncreasedBy;
        st.search_text = "5".into();
        st.refine(&before, &after).unwrap();
        assert_eq!(st.results, vec![16]);

        st.next_mode = ScanMode::Unchanged;
        st.refine(&after, &after).unwrap();
        assert_eq!(st.results, vec![16]);

        let mut st = HexState { first_mode: ScanMode::Between, search_text: "400".into(), search_text2: "600".into(), ..Default::default() };
        st.start_scan(&before).unwrap();
        assert_eq!(st.results, vec![4]);
        st.next_mode = ScanMode::Changed;
        st.refine(&before, &after).unwrap();
        assert_eq!(st.results, vec![4]);
    }

    #[test]
    fn unaligned_scan_and_floats() {
        let mut d = vec![0u8; 3];
        d.extend(2.5f32.to_le_bytes());
        let mut st = HexState { search_type: NumType::F32, search_text: "2.5".into(), aligned: false, ..Default::default() };
        st.start_scan(&d).unwrap();
        assert_eq!(st.results, vec![3]);
        st.aligned = true;
        st.start_scan(&d).unwrap();
        assert!(st.results.is_empty());
    }
}
