//! Changing values: single edits, mass edits, undo and redo, and reverting.

use super::*;

/// True (and forgets it) when `ptr` is the field that put up the current error.
pub(super) fn take_if_same(slot: &mut Option<String>, ptr: &str) -> bool {
    if slot.as_deref() == Some(ptr) {
        *slot = None;
        true
    } else {
        false
    }
}

impl App {
    pub(super) fn root_mut(&mut self) -> Option<&mut Value> {
        match &mut self.doc.as_mut()?.content {
            Content::Json { value, .. } => Some(value),
            Content::Tree(t) => Some(t.view_and_editable().0),
            _ => None,
        }
    }

    /// Replace the value at `ptr`. Returns the old value and how many other places
    /// changed with it (values shared between variables), or why the format can't
    /// store it (e.g. out of range).
    pub(super) fn set_value(&mut self, ptr: &str, v: Value) -> Result<Option<(Value, usize)>, String> {
        if self.saving.is_some() {
            return Err(tr("the file is being saved").into());
        }
        if let Some(Doc { content: Content::Tree(t), .. }) = &mut self.doc {
            return t.set(ptr, v);
        }
        let Some(target) = self.root_mut().and_then(|r| r.pointer_mut(ptr)) else { return Ok(None) };
        Ok(Some((std::mem::replace(target, v), 0)))
    }

    /// Apply a user edit, recording it for undo. Consecutive edits to the same field are merged.
    pub(super) fn apply_edit(&mut self, ptr: String, v: Value, now: f64) {
        match self.try_edit(ptr.clone(), v, now) {
            Err(e) => {
                self.fail(trf("Can't use that value: {0}.", &[&e]));
                self.error_ptr = Some(ptr);
            }
            Ok(_) => self.clear_edit_error(&ptr),
        }
    }

    /// A value was accepted: drop the "can't use that value" message that field left behind.
    pub(super) fn clear_edit_error(&mut self, ptr: &str) {
        if take_if_same(&mut self.error_ptr, ptr) {
            if self.banner.as_deref() == Some(self.status.as_str()) {
                self.banner = None;
            }
            if self.status_kind == StatusKind::Err {
                self.status.clear();
                self.status_kind = StatusKind::Info;
            }
        }
    }

    /// `apply_edit` that hands back why the format refused the value instead of showing it.
    /// `Ok(false)`: there is no such value, or it already held that.
    pub(super) fn try_edit(&mut self, ptr: String, v: Value, now: f64) -> Result<bool, String> {
        let (before, aliases) = match self.set_value(&ptr, v.clone())? {
            Some(r) => r,
            None => return Ok(false),
        };
        let derived = match &mut self.doc {
            Some(Doc { content: Content::Tree(t), .. }) => t.take_derived(),
            _ => vec![],
        };
        // The format may have adjusted the value (e.g. rounded to a 32-bit float).
        let v = self.doc.as_ref().and_then(|d| root(&d.content)).and_then(|r| r.pointer(&ptr)).cloned().unwrap_or(v);
        if before == v && derived.is_empty() {
            return Ok(false);
        }
        let merge = matches!(&self.last_edit, Some((p, t)) if *p == ptr && now - t < 1.5);
        match (merge, self.undo.last_mut()) {
            (true, Some(Change::Value { ptr: p, before: b, after, extra })) if *p == ptr => {
                *after = v;
                for (dp, db, da) in &derived {
                    match extra.iter_mut().find(|(ep, _, _)| ep == dp) {
                        Some(e) => e.2 = da.clone(),
                        None => extra.push((dp.clone(), db.clone(), da.clone())),
                    }
                }
                extra.retain(|(_, b, a)| b != a);
                // Typed back to where it started: drop the no-op undo step.
                if b == after && extra.is_empty() {
                    self.undo.pop();
                }
            }
            _ => self.undo.push(Change::Value { ptr: ptr.clone(), before, after: v, extra: derived.clone() }),
        }
        self.redo.clear();
        for (dp, _, _) in &derived {
            self.after_change(dp);
        }
        self.after_change(&ptr);
        self.last_edit = Some((ptr, now));
        if !derived.is_empty() {
            let names: Vec<String> = derived.iter().map(|(p, _, _)| p.rsplit('/').next().unwrap_or("").replace("~1", "/").replace("~0", "~")).collect();
            self.info(trf("Also updated: {0}.", &[&names.join(", ")]));
        } else if aliases > 0 {
            self.info(if aliases == 1 {
                tr("This value is shared with 1 other place in the save, which changed too.").to_string()
            } else {
                trf("This value is shared with {0} other places in the save, which changed too.", &[&aliases])
            });
        }
        Ok(true)
    }

    pub(super) fn apply_bytes(&mut self, off: usize, bytes: Vec<u8>) {
        if self.saving.is_some() {
            return;
        }
        let Some(Doc { content: Content::Binary(data), .. }) = &mut self.doc else { return };
        let end = off + bytes.len();
        if end > data.len() {
            return;
        }
        let before = data[off..end].to_vec();
        data[off..end].copy_from_slice(&bytes);
        self.undo.push(Change::Bytes { off, before, after: bytes });
        self.redo.clear();
        self.last_edit = None;
        self.after_change("");
    }

    pub(super) fn after_change(&mut self, ptr: &str) {
        self.touched.insert(ptr);
        match &mut self.trace {
            Some(t) => t.push(ptr.to_string()),
            // An edit made alone (compare, tools...) is no longer part of a button's group.
            None => self.touched.leave_group(ptr),
        }
        if ptr.is_empty() && root(&self.doc.as_ref().unwrap().content).is_some() {
            self.rebuild_model();
        }
        self.hex.label_edits.clear();
        self.sync_raw();
        self.edit_gen += 1;
        // This value differing from the file is enough to know there are unsaved changes; only
        // when it was put back does every edited value have to be looked at again. (Looking at
        // them all after each of the thousands of edits of a bulk edit took seconds.)
        let differs = self.doc.as_ref().is_some_and(|d| match (root(&d.content), root(&d.baseline)) {
            (Some(cv), Some(bv)) => cv.pointer(ptr) != bv.pointer(ptr),
            _ => false,
        });
        if differs {
            self.dirty = true;
        } else if self.trace.is_none() {
            self.recompute_dirty();
        }
    }

    /// Keep the Raw JSON text in step with the data, so undo/revert aren't
    /// silently re-applied from stale text on the next save or tab switch.
    pub(super) fn sync_raw(&mut self) {
        if self.tab == Tab::Raw {
            if let Some(Doc { content: Content::Json { value, .. }, .. }) = &self.doc {
                self.raw_text = serde_json::to_string_pretty(value).unwrap_or_default();
                self.raw_error = None;
            }
        }
    }

    pub(super) fn undo_redo(&mut self, undo: bool) {
        if self.saving.is_some() {
            return;
        }
        let Some(change) = (if undo { self.undo.pop() } else { self.redo.pop() }) else { return };
        let ptrs = self.replay(&change, undo);
        if undo { self.redo.push(change) } else { self.undo.push(change) }
        self.last_edit = None;
        for p in &ptrs {
            self.after_change(p);
        }
        self.info(if undo { tr("Undone.") } else { tr("Redone.") });
    }

    /// Puts a change back (`undo`) or makes it again. Returns the values it touched (`""` for
    /// bytes), for `after_change`.
    pub(super) fn replay(&mut self, change: &Change, undo: bool) -> Vec<String> {
        match change {
            Change::Value { ptr, before, after, extra } => {
                // Undo/redo only restores values that were valid before, so this can't fail.
                let _ = self.set_value(ptr, if undo { before.clone() } else { after.clone() });
                if let Some(Doc { content: Content::Tree(t), .. }) = &mut self.doc {
                    t.take_derived();
                    for (p, b, a) in extra.iter() {
                        t.set_plain(p, if undo { b.clone() } else { a.clone() });
                    }
                }
                let mut out: Vec<String> = extra.iter().map(|(p, _, _)| p.clone()).collect();
                out.push(ptr.clone());
                out
            }
            Change::Bytes { off, before, after } => {
                if let Some(Doc { content: Content::Binary(data), .. }) = &mut self.doc {
                    // The two sides can differ in length (a patch that inserts or deletes bytes).
                    let (cur, src) = if undo { (after, before) } else { (before, after) };
                    let end = (*off + cur.len()).min(data.len());
                    data.splice(*off..end, src.iter().copied());
                }
                vec![String::new()]
            }
            Change::Batch(list) => {
                let mut out = Vec::new();
                if undo {
                    for c in list.iter().rev() {
                        out.extend(self.replay(c, true));
                    }
                } else {
                    for c in list {
                        out.extend(self.replay(c, false));
                    }
                }
                out.dedup();
                out
            }
        }
    }

    /// Applies what one screen action asked for. Several edits from one press (a mass-edit
    /// button) become one undo step and one change group, so Undo and the Changes panel's
    /// Revert put them all back together; a single edit is as before.
    pub(super) fn apply_edits(&mut self, edits: Vec<(String, Value)>, now: f64) {
        if edits.is_empty() {
            return;
        }
        let many = edits.len() > 1;
        if many {
            self.last_edit = None;
        }
        let start = self.undo.len();
        self.trace = Some(vec![]);
        for (ptr, v) in edits {
            self.apply_edit(ptr, v, now);
        }
        let mut seen = HashSet::new();
        let mut touched: Vec<String> = self.trace.take().unwrap_or_default();
        touched.retain(|p| !p.is_empty() && seen.insert(p.clone()));
        // Looked at once, not after each of possibly thousands of edits.
        self.recompute_dirty();
        if many {
            self.group_undo_from(start);
        }
        if many && touched.len() > 1 {
            self.touched.set_group(&touched);
        } else {
            for p in &touched {
                self.touched.leave_group(p);
            }
        }
    }

    /// Folds the undo steps made since the stack held `start` into one, so a bulk edit or a
    /// preset is undone in one go.
    pub(super) fn group_undo_from(&mut self, start: usize) {
        if self.undo.len() > start + 1 {
            let list: Vec<Change> = self.undo.drain(start..).collect();
            self.undo.push(Change::Batch(list));
        }
        self.last_edit = None;
    }

    pub(super) fn recompute_dirty(&mut self) {
        let Some(d) = &self.doc else {
            self.dirty = false;
            return;
        };
        self.dirty = match (&d.content, &d.baseline) {
            (Content::Binary(a), Content::Binary(b)) => a != b,
            (Content::Text(a), Content::Text(b)) => a != b,
            (c, b) => match (root(c), root(b)) {
                (Some(cv), Some(bv)) => self.touched.iter().any(|p| cv.pointer(p) != bv.pointer(p)),
                _ => true,
            },
        };
    }

    pub(super) fn revert_all(&mut self) {
        if self.saving.is_some() {
            return;
        }
        let Some(d) = &mut self.doc else { return };
        d.content = d.baseline.clone();
        self.undo.clear();
        self.redo.clear();
        self.touched.clear();
        self.hex.label_edits.clear();
        self.sync_raw();
        self.rebuild_model();
        self.edit_gen += 1;
        self.recompute_dirty();
        self.ok(tr("All changes reverted."));
    }

    pub(super) fn commit_raw(&mut self) -> bool {
        if self.tab != Tab::Raw {
            return true;
        }
        let Some(Doc { content: Content::Json { value, .. }, .. }) = &self.doc else { return true };
        match serde_json::from_str::<Value>(&self.raw_text) {
            Ok(v) => {
                if *value != v {
                    self.apply_edit(String::new(), v, f64::NEG_INFINITY);
                }
                self.raw_error = None;
                true
            }
            Err(e) => {
                self.raw_error = Some(format!("Line {}, column {}: {e}", e.line(), e.column()));
                false
            }
        }
    }

    /// Replaces the whole binary content (its length may change) as one undo step.
    pub(super) fn replace_bytes(&mut self, after: Vec<u8>) {
        let Some(Doc { content: Content::Binary(data), .. }) = &mut self.doc else { return };
        let before = std::mem::replace(data, after.clone());
        self.undo.push(Change::Bytes { off: 0, before, after });
        self.redo.clear();
        self.last_edit = None;
        self.after_change("");
    }
}

#[cfg(test)]
mod edit_error_tests {
    #[test]
    fn error_clears_only_for_its_own_field() {
        let mut slot = Some("/level".to_string());
        assert!(!super::take_if_same(&mut slot, "/gold"), "another field's fix keeps the error");
        assert!(super::take_if_same(&mut slot, "/level"), "fixing the failing field clears it");
        assert!(slot.is_none());
        assert!(!super::take_if_same(&mut slot, "/level"));
    }
}
