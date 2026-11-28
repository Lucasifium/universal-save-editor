# Building from source

You need a recent stable Rust toolchain.

    cargo run --release -- path/to/savefile    # run (path is optional)
    cargo build --release                      # build
    cargo test --release                       # tests

## Other platforms

`build_all.ps1` (PowerShell) builds the desktop targets it finds tooling for
(`-Only windows-x64|windows-arm64|linux-x64|linux-arm64|web`). The tools involved are
the Rust targets for each platform, Visual Studio Build Tools (Windows ARM64), `cargo-zigbuild`
(Linux).

**Android** is not released as a download. To build the APK yourself, run `.\build_all.ps1 -Only android`
(Windows) or `packaging/build_android.sh out.apk` (Linux), with the Android SDK/NDK and a JDK installed. The
first build makes a signing key of your own (kept in `packaging/android/`, never committed); Android only
installs updates signed with the same key, so keep it.

The browser version is built with `web/build.sh` (needs the `wasm32-unknown-unknown` target and a
matching `wasm-bindgen-cli`) into `docs/app/`. Its last step, `web/finish.py`, copies the page and
`web/fsa.js` (saving in place in Chrome and Edge) and writes `sw.js`, the service worker that keeps the
app's files for offline use, with a cache named after the version and a hash of the files.

## Adding a simple game

Games whose save is a plain tree can be described in a single file in `data/games/` (no Rust). The format is in `data/games/README.md`.

## Package-manager manifests

After a release is published, run `python3 -I packaging/manifests.py` to refresh the Scoop manifest
(`packaging/scoop/`) and the winget manifests (`packaging/winget/<version>/`) from that release's
`SHA256SUMS.txt`, and commit them. The winget files go into a pull request to `microsoft/winget-pkgs` under
`manifests/l/Lucasifium/UniversalSaveEditor/<version>/`.

## Checking game screens against real saves

Every game screen has a line in `tests/screen-samples.tsv`: `sample` when a public save of the game was opened on it and checked, `none` when no public save has been found. `cargo test` fails when a screen is added without a line, so a new screen has to be opened on a real save (or marked `none` with the reason).

Saves are never committed. To check yours, list them as `<screen>	<path>` lines and run `scripts/check-samples.sh list.tsv`: each save must open on its own screen, hold editable values and write back unchanged (or read back the same where the container is rebuilt, such as a zip or a compressed save). `which_screen` (`SAVE_EDITOR_SCAN=files.txt cargo test --release which_screen -- --ignored --nocapture`) prints which screen each file in a list opens on.
