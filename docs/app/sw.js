// Keeps the browser version's own files (the page, its script and the program file) so it opens
// again without a connection. Only files under the folder this worker lives in (/app/) are ever
// kept or answered; everything else goes to the network as usual. web/finish.py fills in the
// cache name (the app version and a hash of the files) and the files with their SHA-256.
'use strict';
const CACHE = "save-editor-app-0.9.3-cbb1c2a34c7d";
const FILES = {
  "./": "d03d0457c9d8fa180f48673a680eaa44e3c622287977d5a5a9fd60e2664e9cc2",
  "universal_save_editor.js": "73f6b9fa46296083b3a466c8c9c8b3b34067cf8737d850c0fb3c0c3b468b6910",
  "universal_save_editor_bg.wasm": "b4cdd5eee269f60efea407fd0bd4ccb08f2f98c313fec4468e1778d9f8d3cb11",
  "fsa.js": "75b4e9cf552f6b3deb5ed49ec363198694f4af34c48d661911d0c4cbf2486999"
};
const PREFIX = 'save-editor-app-';
const BASE = new URL('./', self.location).href;

async function sha256(buf) {
  const d = new Uint8Array(await crypto.subtle.digest('SHA-256', buf));
  return Array.from(d, (b) => b.toString(16).padStart(2, '0')).join('');
}

// The file as this release has it: from the browser's cache when that copy matches (the page
// has usually just downloaded it), otherwise from the network.
async function fetchChecked(name, want) {
  const url = new URL(name, BASE).href;
  for (const mode of ['default', 'reload']) {
    const r = await fetch(url, { cache: mode });
    if (!r.ok) throw new Error(url + ': ' + r.status);
    const body = await r.clone().arrayBuffer();
    if ((await sha256(body)) === want) return r;
  }
  throw new Error(url + ' is not the file of this release');
}

self.addEventListener('install', (event) => {
  event.waitUntil(
    (async () => {
      const got = await Promise.all(Object.entries(FILES).map(async ([name, want]) => [name, await fetchChecked(name, want)]));
      const cache = await caches.open(CACHE);
      await Promise.all(got.map(([name, r]) => cache.put(new URL(name, BASE).href, r)));
      // The page loads every file it needs at start, so the new version can take over now; it
      // shows on the next visit.
      await self.skipWaiting();
    })()
  );
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    (async () => {
      for (const name of await caches.keys()) {
        if (name.startsWith(PREFIX) && name !== CACHE) await caches.delete(name);
      }
    })()
  );
});

self.addEventListener('fetch', (event) => {
  const req = event.request;
  if (req.method !== 'GET' || !req.url.startsWith(BASE)) return;
  event.respondWith(
    (async () => {
      const cache = await caches.open(CACHE);
      const hit = (await cache.match(req, { ignoreSearch: true })) || (req.mode === 'navigate' ? await cache.match(BASE) : undefined);
      return hit || fetch(req);
    })()
  );
});
