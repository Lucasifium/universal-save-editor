//! Tools › Compare with another save: lists every value that differs from a second file.

use super::*;

/// One row of the list: (pointer, this save's value, the other save's value).
pub(super) type Diff = (String, Option<Value>, Option<Value>);

/// Result of comparing the open save with another file.
pub(super) struct Compare {
    pub other: PathBuf,
    pub diffs: Vec<Diff>,
    /// Text typed in the filter box.
    filter: String,
}

/// How a changed number moved, as `+120` or `-3.5`; `None` when either side isn't a number.
pub(super) fn delta(mine: &Option<Value>, theirs: &Option<Value>) -> Option<String> {
    let (Value::Number(a), Value::Number(b)) = (mine.as_ref()?, theirs.as_ref()?) else { return None };
    if let (Some(a), Some(b)) = (a.as_i64(), b.as_i64()) {
        return b.checked_sub(a).map(|d| format!("{d:+}"));
    }
    if let (Some(a), Some(b)) = (a.as_u64(), b.as_u64()) {
        return Some(if b >= a { format!("+{}", b - a) } else { format!("-{}", a - b) });
    }
    let d = b.as_f64()? - a.as_f64()?;
    if !d.is_finite() {
        return None;
    }
    let s = format!("{d:+.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    (s != "+" && s != "-" && s != "+0" && s != "-0").then(|| s.to_string())
}

/// Whether a row passes the filter box: the text is looked for, ignoring case, in the name and
/// in both values.
pub(super) fn matches(filter: &str, name: &str, mine: &str, theirs: &str) -> bool {
    let f = filter.trim().to_lowercase();
    f.is_empty() || [name, mine, theirs].iter().any(|s| s.to_lowercase().contains(&f))
}

fn hex16(x: &[u8]) -> Value {
    let mut s = x.iter().take(16).map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" ");
    if x.len() > 16 {
        s.push_str(" \u{2026}");
    }
    Value::String(s)
}

/// The differences between two decoded saves, by value pointer or (for unknown formats) byte range.
pub(super) fn diff_contents(a: &Content, b: &Content) -> Option<Vec<Diff>> {
    Some(match (a, b) {
        (Content::Binary(a), Content::Binary(b)) => {
            let mut out: Vec<Diff> = byte_ranges(a, b)
                .into_iter()
                .take(5000)
                .map(|(s, e)| (format!("0x{s:X}"), Some(hex16(&a[s..e])), b.get(s..e.min(b.len())).map(hex16)))
                .collect();
            if a.len() != b.len() {
                out.push(("(file size)".into(), Some(Value::from(a.len())), Some(Value::from(b.len()))));
            }
            out
        }
        (c, t) => {
            let (mine, other_root) = (root(c)?, root(t)?);
            let (mut x, mut y) = (Vec::new(), Vec::new());
            flatten(mine, "", &mut x);
            flatten(other_root, "", &mut y);
            let theirs: HashMap<String, Value> = y.into_iter().collect();
            let mut seen = HashSet::new();
            let mut out = Vec::new();
            for (p, v) in x {
                seen.insert(p.clone());
                match theirs.get(&p) {
                    Some(o) if *o == v => {}
                    o => out.push((p, Some(v), o.cloned())),
                }
            }
            let mut only_theirs: Vec<_> = theirs.into_iter().filter(|(p, _)| !seen.contains(p)).collect();
            only_theirs.sort_by(|a, b| a.0.cmp(&b.0));
            out.extend(only_theirs.into_iter().map(|(p, v)| (p, None, Some(v))));
            out
        }
    })
}

impl App {
    pub(super) fn start_compare(&mut self) {
        self.ask(Ask::Compare, None, None);
    }

    pub(super) fn compare_with(&mut self, other: PathBuf) {
        let Some(d) = &self.doc else { return };
        let raw = match crate::vfs::read(&other) {
            Ok(r) => r,
            Err(e) => return self.fail(trf("Couldn't open {0}: {1}", &[&other.display(), &e])),
        };
        let mut opts = formats::Options::for_path(&other);
        opts.password = d.opts.password.clone();
        let theirs = formats::decode_with(&raw, &opts);
        let Some(diffs) = diff_contents(&d.content, &theirs.content) else {
            return self.fail(tr("Those two files aren't the same kind of save, so they can't be compared."));
        };
        let n = diffs.len();
        self.compare = Some(Compare { other, diffs, filter: String::new() });
        self.info(if n == 1 { tr("1 difference found.").to_string() } else { trf("{0} differences found.", &[&n]) });
    }

    /// The name a row goes by in the list: section, group and value name as in the Values tab.
    fn compare_name(&self, p: &str) -> String {
        match self.model.by_ptr.get(p) {
            Some(&i) => {
                let r = &self.model.rows[i];
                let section = &self.model.sections[r.section].title;
                let mut parts: Vec<&str> = Vec::new();
                if section != "General" {
                    parts.push(section);
                }
                if !r.context().is_empty() {
                    parts.push(r.context());
                }
                parts.push(r.name());
                parts.join(" \u{203A} ")
            }
            None => p.to_string(),
        }
    }

    pub(super) fn compare_window(&mut self, ctx: &egui::Context, now: f64) {
        let Some(cmp) = &self.compare else { return };
        let binary = matches!(self.doc.as_ref().map(|d| &d.content), Some(Content::Binary(_)));
        let show = |v: &Option<Value>| v.as_ref().map(widgets::display_shown).unwrap_or_else(|| tr("(missing)").into());
        // This save's *current* value, so rows update after edits.
        let mine_of = |p: &str, original: &Option<Value>| -> Option<Value> {
            if binary {
                original.clone()
            } else {
                self.doc.as_ref().and_then(|d| root(&d.content)).and_then(|r| r.pointer(p)).cloned()
            }
        };
        let filter = cmp.filter.clone();
        // With no filter typed every row is listed and only the visible ones are worked out.
        let rows: Vec<usize> = if filter.trim().is_empty() {
            (0..cmp.diffs.len()).collect()
        } else {
            cmp.diffs
                .iter()
                .enumerate()
                .filter(|(_, (p, original, theirs))| matches(&filter, &self.compare_name(p), &show(&mine_of(p, original)), &show(theirs)))
                .map(|(i, _)| i)
                .collect()
        };
        let total = cmp.diffs.len();
        let other_name = file_name(&cmp.other);
        let mut open = true;
        let mut jump: Option<String> = None;
        let mut take: Option<(String, Value)> = None;
        let mut new_filter = filter.clone();
        let mut copy = false;
        egui::Window::new(trf("Compare with {0}", &[&other_name]))
            .open(&mut open)
            .default_size([fit(ctx, 620.0), 460.0])
            .max_width(fit(ctx, 1400.0))
            .show(ctx, |ui| {
                ui.weak(tr("Values that differ between this save and the other one. \u{201C}Use theirs\u{201D} copies the other save's value into this one."));
                ui.horizontal(|ui| {
                    ui.add(TextEdit::singleline(&mut new_filter).hint_text(tr("Filter by name or value")).desired_width(220.0));
                    ui.label(if rows.len() == total {
                        trf("{0} differences", &[&total])
                    } else {
                        trf("{0} of {1} differences shown", &[&rows.len(), &total])
                    });
                    if ui.add_enabled(!rows.is_empty(), egui::Button::new(tr("Copy as text"))).on_hover_text(tr("Copy the listed differences to the clipboard")).clicked() {
                        copy = true;
                    }
                });
                ui.separator();
                if total == 0 {
                    ui.label(tr("No differences. The saves hold the same values."));
                } else if rows.is_empty() {
                    ui.label(tr("No difference matches the filter."));
                }
                let diffs = &self.compare.as_ref().map(|c| &c.diffs[..]).unwrap_or(&[]);
                ScrollArea::vertical().auto_shrink([false, false]).show_rows_locked(ui, 26.0, rows.len(), |ui, range| {
                    for &i in &rows[range] {
                        let (p, original, theirs) = &diffs[i];
                        let name = &self.compare_name(p);
                        let mine = &mine_of(p, original);
                        ui.horizontal(|ui| {
                            ui.allocate_ui_with_layout(egui::vec2(260.0, 22.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                ui.set_width(260.0);
                                let color = ui.visuals().hyperlink_color;
                                let link = ui.add(egui::Label::new(RichText::new(name).color(color)).truncate().sense(egui::Sense::click()));
                                if link.on_hover_text(p).clicked() {
                                    jump = Some(p.clone());
                                }
                            });
                            if mine == theirs {
                                ui.label(RichText::new(trf("{0}  (now the same)", &[&show(mine)])).weak());
                                return;
                            }
                            ui.label(RichText::new(show(mine)).color(theme::fg(ui, theme::MODIFIED)));
                            ui.label("\u{203A}");
                            ui.label(show(theirs));
                            if let Some(d) = delta(mine, theirs) {
                                ui.label(RichText::new(d).weak());
                            }
                            let editable = self.model.by_ptr.get(p.as_str()).is_some_and(|&i| self.model.rows[i].editable);
                            let scalar = theirs.as_ref().is_some_and(|t| t.is_number() || t.is_string() || t.is_boolean());
                            if let (true, false, true, Some(t), Some(_)) = (editable, binary, scalar, theirs, mine) {
                                if crate::widgets::small_button(ui, tr("Use theirs")).clicked() {
                                    take = Some((p.clone(), t.clone()));
                                }
                            }
                        });
                    }
                });
            });
        if copy {
            let text: Vec<String> = rows
                .iter()
                .filter_map(|&i| {
                    let (p, original, theirs) = &self.compare.as_ref()?.diffs[i];
                    Some(format!("{}: {} \u{203A} {}", self.compare_name(p), show(&mine_of(p, original)), show(theirs)))
                })
                .collect();
            ctx.copy_text(text.join("\n"));
            self.info(tr("Copied the differences."));
        }
        if let Some(c) = &mut self.compare {
            c.filter = new_filter;
        }
        if !open {
            self.compare = None;
        }
        if let Some((p, v)) = take {
            self.apply_edit(p, v, now);
        }
        if let Some(p) = jump {
            if let Some(&i) = self.model.by_ptr.get(p.as_str()) {
                self.jump_to_row(i);
            } else if let Some(off) = p.strip_prefix("0x").and_then(|h| usize::from_str_radix(h, 16).ok()) {
                self.hex.jump(off);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn j(v: Value) -> Content {
        Content::Json { value: v, pretty: true }
    }

    #[test]
    fn delta_shows_direction_of_numbers() {
        assert_eq!(delta(&Some(json!(100)), &Some(json!(250))).as_deref(), Some("+150"));
        assert_eq!(delta(&Some(json!(5)), &Some(json!(2))).as_deref(), Some("-3"));
        assert_eq!(delta(&Some(json!(1.5)), &Some(json!(1.25))).as_deref(), Some("-0.25"));
        assert_eq!(delta(&Some(json!(u64::MAX)), &Some(json!(u64::MAX - 1))).as_deref(), Some("-1"));
        assert_eq!(delta(&Some(json!("a")), &Some(json!(2))), None);
        assert_eq!(delta(&None, &Some(json!(2))), None);
    }

    #[test]
    fn filter_looks_in_names_and_values() {
        assert!(matches("", "Gold", "1", "2"));
        assert!(matches("gold", "Player \u{203A} Gold", "1", "2"));
        assert!(matches(" 250 ", "Gold", "100", "250"));
        assert!(!matches("silver", "Gold", "100", "250"));
    }

    #[test]
    fn structured_saves_list_changed_added_and_removed_values() {
        let a = j(json!({"gold": 10, "name": "A", "only_a": 1}));
        let b = j(json!({"gold": 99, "name": "A", "only_b": 2}));
        let d = diff_contents(&a, &b).unwrap();
        let ptrs: Vec<&str> = d.iter().map(|x| x.0.as_str()).collect();
        assert_eq!(ptrs, ["/gold", "/only_a", "/only_b"]);
        assert_eq!(d[1].2, None);
        assert_eq!(d[2].1, None);
    }

    #[test]
    fn unknown_formats_list_byte_ranges_and_size() {
        let a = Content::Binary(vec![0u8; 40]);
        let mut y = vec![0u8; 40];
        y[3] = 7;
        y[30] = 1;
        let d = diff_contents(&a, &Content::Binary(y)).unwrap();
        assert_eq!(d.iter().map(|x| x.0.as_str()).collect::<Vec<_>>(), ["0x3", "0x1E"]);
        let d = diff_contents(&a, &Content::Binary(vec![0u8; 41])).unwrap();
        assert_eq!(d.last().unwrap().0, "(file size)");
        assert!(diff_contents(&a, &j(json!({}))).is_none());
    }
}
