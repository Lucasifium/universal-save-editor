// Saving back to the file the player opened, where the browser allows it (Chrome and Edge:
// the File System Access API), and copies of each file as it was before it was written,
// kept in this browser's own storage (IndexedDB). Used by src/web.rs through `saveEditorFiles`.
// Nothing here is sent anywhere.
(function (g) {
  'use strict';
  const DB = 'save-editor-originals';
  const STORE = 'originals';
  // What is kept: the newest MAX_COUNT originals, at most MAX_BYTES in all.
  const MAX_COUNT = 10;
  const MAX_BYTES = 200 * 1024 * 1024;
  // Per handle, the SHA-256 of: what this page last read from or wrote to the file (a file
  // that no longer matches was changed by something else), what it last wrote, and the copy
  // it last kept.
  const known = new WeakMap();
  const written = new WeakMap();
  const keptHash = new WeakMap();

  async function sha256(bytes) {
    const d = new Uint8Array(await g.crypto.subtle.digest('SHA-256', bytes));
    return Array.from(d, (b) => b.toString(16).padStart(2, '0')).join('');
  }

  // The file behind `handle` as it is now. It becomes what this page knows the file to hold
  // only once the page shows it (`accept`): until then a save of what is on screen, made from
  // the older file, still finds the file changed and asks first.
  const readHash = new WeakMap();
  async function read(handle) {
    const data = new Uint8Array(await (await handle.getFile()).arrayBuffer());
    readHash.set(handle, await sha256(data));
    return data;
  }

  // The file last read through `handle` is on screen now.
  function accept(handle) {
    if (readHash.has(handle)) {
      known.set(handle, readHash.get(handle));
      readHash.delete(handle);
    }
  }

  function message(e) {
    if (e && typeof e === 'object') return String(e.message || e.name || e);
    return String(e);
  }

  function supported() {
    return typeof g.showOpenFilePicker === 'function' && g.isSecureContext !== false && typeof g.indexedDB === 'object';
  }

  // The picked file: { name, data, handle }, or null when the picker was closed.
  async function pick() {
    let handles;
    try {
      handles = await g.showOpenFilePicker({ multiple: false });
    } catch (e) {
      if (e && e.name === 'AbortError') return null;
      throw e;
    }
    const handle = handles[0];
    const file = await handle.getFile();
    const data = new Uint8Array(await file.arrayBuffer());
    known.set(handle, await sha256(data));
    return { name: file.name, data, handle };
  }

  function request(r) {
    return new Promise((ok, fail) => {
      r.onsuccess = () => ok(r.result);
      r.onerror = () => fail(r.error);
    });
  }

  function finished(tx) {
    return new Promise((ok, fail) => {
      tx.oncomplete = () => ok();
      tx.onerror = () => fail(tx.error || new Error('the browser storage refused the change'));
      tx.onabort = () => fail(tx.error || new Error('the browser storage refused the change'));
    });
  }

  function openDb() {
    return new Promise((ok, fail) => {
      const r = g.indexedDB.open(DB, 1);
      r.onupgradeneeded = () => r.result.createObjectStore(STORE, { keyPath: 'id', autoIncrement: true });
      r.onsuccess = () => ok(r.result);
      r.onerror = () => fail(r.error);
      r.onblocked = () => fail(new Error('the browser storage is busy in another tab'));
    });
  }

  // Which of `rows` (oldest first, { id, size }) to let go so that the newest MAX_COUNT stay,
  // at most MAX_BYTES in all. The newest row (the one just added) always stays.
  function toDrop(rows, maxCount, maxBytes) {
    const drop = [];
    let count = rows.length;
    let total = rows.reduce((n, r) => n + r.size, 0);
    for (let i = 0; i < rows.length - 1 && (count > maxCount || total > maxBytes); i++) {
      drop.push(rows[i].id);
      count -= 1;
      total -= rows[i].size;
    }
    return drop;
  }

  // Keeps `data` (the file as it is before the first save) under `name`; returns its id.
  async function keepOriginal(name, data) {
    if (data.byteLength > MAX_BYTES) {
      throw new Error('the file is larger than the ' + MAX_BYTES / (1024 * 1024) + ' MB this browser keeps for originals');
    }
    try {
      // Asks the browser not to clear this storage when space runs low (Chrome decides by itself, without asking).
      if (g.navigator && g.navigator.storage && g.navigator.storage.persist) g.navigator.storage.persist().catch(() => {});
    } catch (_) {}
    const db = await openDb();
    try {
      const tx = db.transaction(STORE, 'readwrite');
      const done = finished(tx);
      const store = tx.objectStore(STORE);
      const id = await request(store.add({ name, size: data.byteLength, when: Date.now(), data: new Blob([data]) }));
      const rows = [];
      await new Promise((ok, fail) => {
        const c = store.openCursor();
        c.onsuccess = () => {
          const cur = c.result;
          if (!cur) return ok();
          rows.push({ id: cur.value.id, size: cur.value.size });
          cur.continue();
        };
        c.onerror = () => fail(c.error);
      });
      for (const old of toDrop(rows, MAX_COUNT, MAX_BYTES)) store.delete(old);
      await done;
      return id;
    } finally {
      db.close();
    }
  }

  // The kept originals, newest first: [{ id, name, size, when }].
  async function originals() {
    const db = await openDb();
    try {
      const tx = db.transaction(STORE, 'readonly');
      const rows = await request(tx.objectStore(STORE).getAll());
      return rows.map((r) => ({ id: r.id, name: r.name, size: r.size, when: new Date(r.when).toLocaleString() })).reverse();
    } finally {
      db.close();
    }
  }

  // Hands kept original `id` to the browser as a download; false when it isn't kept any more.
  async function downloadOriginal(id) {
    const db = await openDb();
    let row;
    try {
      row = await request(db.transaction(STORE, 'readonly').objectStore(STORE).get(id));
    } finally {
      db.close();
    }
    if (!row) return false;
    const url = URL.createObjectURL(row.data);
    const a = g.document.createElement('a');
    a.href = url;
    a.download = row.name;
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 10000);
    return true;
  }

  function same(a, b) {
    if (a.length !== b.length) return false;
    for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return false;
    return true;
  }

  // What the file holds now: 'new' (`after`), 'old' (`before`), 'other', or 'unknown' (unreadable).
  async function whatIsThere(handle, before, after) {
    try {
      const now = new Uint8Array(await (await handle.getFile()).arrayBuffer());
      if (same(now, after)) return 'new';
      if (before && same(now, before)) return 'old';
      return 'other';
    } catch (_) {
      return 'unknown';
    }
  }

  // Writes `bytes` over the file behind `handle`. Never rejects; the answer says what happened:
  //   { status: 'saved', kept }                 written and read back as written
  //   { status: 'refused', error }              no permission to write; the file wasn't touched
  //   { status: 'nobackup', error }             the original couldn't be kept; the file wasn't touched
  //   { status: 'failed', error, changed, kept } changed: 'no' or 'unknown'
  //   { status: 'changed' }                     the file isn't what this page last read or
  //                                             wrote (something else wrote it); not touched
  // With `anyway`, a changed file is written over all the same (after keeping a copy of it).
  // `kept` is true when this call kept a copy of the file as it was: that happens whenever
  // the file differs both from the copy kept last and from what this page last wrote, so the
  // file as first opened and any version written by something else are kept, but not each
  // of this page's own saves.
  async function write(handle, bytes, anyway) {
    try {
      let p = await handle.queryPermission({ mode: 'readwrite' });
      if (p !== 'granted') p = await handle.requestPermission({ mode: 'readwrite' });
      if (p !== 'granted') return { status: 'refused', error: '' };
    } catch (e) {
      return { status: 'refused', error: message(e) };
    }
    let before, was;
    try {
      before = new Uint8Array(await (await handle.getFile()).arrayBuffer());
      was = await sha256(before);
    } catch (e) {
      return { status: 'nobackup', error: message(e) };
    }
    if (!anyway && known.has(handle) && known.get(handle) !== was) return { status: 'changed' };
    let keptNow = false;
    if (keptHash.get(handle) !== was && written.get(handle) !== was) {
      try {
        await keepOriginal(handle.name, before);
      } catch (e) {
        return { status: 'nobackup', error: message(e) };
      }
      keptHash.set(handle, was);
      keptNow = true;
    }
    // createWritable writes to a temporary file; the browser puts it in place of the file only
    // when close() succeeds, so a failure part way leaves the file as it was.
    let error = null;
    let w = null;
    try {
      w = await handle.createWritable({ keepExistingData: false });
      await w.write(bytes);
      const closing = w;
      w = null;
      await closing.close();
    } catch (e) {
      error = message(e);
      if (w) {
        try {
          await w.abort();
        } catch (_) {}
      }
    }
    const now = await whatIsThere(handle, before, bytes);
    if (now === 'new') {
      const h = await sha256(bytes);
      known.set(handle, h);
      written.set(handle, h);
      return { status: 'saved', kept: keptNow };
    }
    // Still as it was: that is what this page knows it to hold. Otherwise the next save finds
    // it changed and asks.
    // Not after Save anyway: then the file holds another program's version, which this
    // page must go on asking about.
    if (now === 'old' && !anyway) known.set(handle, was);
    return {
      status: 'failed',
      error: error || 'the file did not read back as written',
      changed: now === 'old' ? 'no' : 'unknown',
      kept: keptNow,
    };
  }

  g.saveEditorFiles = { supported, pick, read, accept, write, originals, downloadOriginal, keepOriginal, toDrop, MAX_COUNT, MAX_BYTES };
})(globalThis);
