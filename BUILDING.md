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
matching `wasm-bindgen-cli`) into `docs/app/`.

## Package-manager manifests

After a release is published, run `python3 -I packaging/manifests.py` to refresh the Scoop manifest
(`packaging/scoop/`) and the winget manifests (`packaging/winget/<version>/`) from that release's
`SHA256SUMS.txt`, and commit them. The winget files go into a pull request to `microsoft/winget-pkgs` under
`manifests/l/Lucasifium/UniversalSaveEditor/<version>/`.
