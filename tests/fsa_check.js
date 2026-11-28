// Checks web/fsa.js with a made-up file handle and an in-memory IndexedDB stand-in.
const path = process.argv[2];
globalThis.crypto = require('crypto').webcrypto;
// Minimal IndexedDB stand-in: one store, add/openCursor/delete/getAll/get.
const rows = new Map();
let nextId = 1;
function req(result) {
  const r = {};
  setTimeout(() => { r.result = result; r.onsuccess && r.onsuccess(); }, 0);
  return r;
}
globalThis.indexedDB = {
  open() {
    const r = {};
    setTimeout(() => {
      r.result = {
        close() {},
        transaction() {
          const tx = {};
          let pending = 0;
          const done = () => setTimeout(() => { if (pending === 0) tx.oncomplete && tx.oncomplete(); }, 5);
          tx.objectStore = () => ({
            add(v) { const id = nextId++; rows.set(id, { ...v, id }); done(); return req(id); },
            delete(id) { rows.delete(id); },
            getAll() { done(); return req([...rows.values()]); },
            get(id) { done(); return req(rows.get(id)); },
            openCursor() {
              const list = [...rows.values()];
              let i = 0;
              const c = {};
              const step = () => setTimeout(() => {
                c.result = i < list.length ? { value: list[i], continue() { i++; step(); } } : null;
                c.onsuccess && c.onsuccess();
              }, 0);
              step();
              return c;
            },
          });
          return tx;
        },
      };
      r.onsuccess && r.onsuccess();
    }, 0);
    return r;
  },
};
require(path);
const F = globalThis.saveEditorFiles;
function handle(bytes) {
  const h = {
    name: 'save.dat',
    data: Uint8Array.from(bytes),
    async queryPermission() { return 'granted'; },
    async requestPermission() { return 'granted'; },
    async getFile() { const d = h.data; return { name: h.name, arrayBuffer: async () => d.slice().buffer }; },
    async createWritable() { let buf; return { async write(b) { buf = Uint8Array.from(b); }, async close() { h.data = buf; }, async abort() {} }; },
  };
  return h;
}
(async () => {
  const h = handle([1, 2, 3]);
  globalThis.showOpenFilePicker = async () => [h];
  const picked = await F.pick();
  if (!picked) throw new Error('pick');
  // The game writes the file; the page reads it again but has not shown it yet.
  h.data = Uint8Array.from([9, 9, 9, 9]);
  await F.read(h);
  // A save of what is on screen (made from the old file) must still find the file changed.
  let r = await F.write(h, Uint8Array.from([1, 2, 4]), false);
  if (r.status !== 'changed') throw new Error('a read not yet shown let a save write over the newer file: ' + JSON.stringify(r));
  if (h.data.length !== 4) throw new Error('written over');
  // Once shown, a save goes through.
  F.accept(h);
  r = await F.write(h, Uint8Array.from([9, 9, 9, 8]), false);
  if (r.status !== 'saved') throw new Error('accepted read still refused: ' + JSON.stringify(r));
  console.log('ok');
})().catch((e) => { console.error(e.message); process.exit(1); });
