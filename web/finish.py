"""Finishes the browser version after wasm-bindgen has written its files into <out> (docs/app):
copies the page and its helper script, writes the program file's size into the page (for the
loading bar), and writes the service worker with the list of files it keeps for offline use.

  finish.py <out>

The worker's cache is named after the app version and a hash of the files, so each release
(or rebuild) replaces the cache of the one before.
"""
import hashlib, json, os, re, shutil, sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
# What the worker keeps: the page (as "./", the address it is opened at) and what it loads.
OFFLINE = ["./", "universal_save_editor.js", "universal_save_editor_bg.wasm", "fsa.js"]


def version():
    with open(os.path.join(ROOT, "Cargo.toml"), encoding="utf-8") as f:
        m = re.search(r'^version\s*=\s*"([^"]+)"', f.read(), re.M)
    if not m:
        sys.exit("finish.py: no version in Cargo.toml")
    return m.group(1)


def main(out):
    wasm = os.path.join(out, "universal_save_editor_bg.wasm")
    if not os.path.isfile(wasm):
        sys.exit(f"finish.py: {wasm} is missing; run wasm-bindgen first")
    with open(os.path.join(HERE, "index.html"), encoding="utf-8") as f:
        page = f.read()
    if 'data-wasm-size=""' not in page:
        sys.exit("finish.py: web/index.html has no data-wasm-size=\"\" to fill in")
    page = page.replace('data-wasm-size=""', f'data-wasm-size="{os.path.getsize(wasm)}"', 1)
    with open(os.path.join(out, "index.html"), "w", encoding="utf-8", newline="\n") as f:
        f.write(page)
    shutil.copyfile(os.path.join(HERE, "fsa.js"), os.path.join(out, "fsa.js"))

    files = {}
    for name in OFFLINE:
        path = os.path.join(out, "index.html" if name == "./" else name)
        with open(path, "rb") as f:
            files[name] = hashlib.sha256(f.read()).hexdigest()
    build = hashlib.sha256("".join(files[n] for n in OFFLINE).encode()).hexdigest()[:12]
    with open(os.path.join(HERE, "sw.js"), encoding="utf-8") as f:
        sw = f.read()
    for key, value in [('"__CACHE__"', json.dumps(f"save-editor-app-{version()}-{build}")), ("__FILES__", json.dumps(files, indent=2))]:
        if key not in sw:
            sys.exit(f"finish.py: web/sw.js has no {key}")
        sw = sw.replace(key, value)
    with open(os.path.join(out, "sw.js"), "w", encoding="utf-8", newline="\n") as f:
        f.write(sw)
    print(f"finish.py: {out} ready (cache {version()}-{build})")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    main(sys.argv[1])
