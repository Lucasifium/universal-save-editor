// Keeps the browser version's own files (the page, its script and the program file) so it opens
// again without a connection. Only files under the folder this worker lives in (/app/) are ever
// kept or answered; everything else goes to the network as usual. web/finish.py fills in the
// cache name (the app version and a hash of the files) and the files with their SHA-256.
'use strict';
const CACHE = "__CACHE__";
const FILES = __FILES__;
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
