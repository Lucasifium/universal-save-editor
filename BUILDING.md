# Building from source

You need a recent stable Rust toolchain.

    cargo run --release -- path/to/savefile    # run (path is optional)
    cargo build --release                      # build
    cargo test --release                       # tests

## Other platforms

`build_all.ps1` (PowerShell) builds the desktop and Android targets it finds tooling for
(`-Only windows-x64|windows-arm64|linux-x64|linux-arm64|android|web`). The tools involved are
the Rust targets for each platform, Visual Studio Build Tools (Windows ARM64), `cargo-zigbuild`
(Linux), and the Android SDK/NDK with a JDK (Android).

The browser version is built with `web/build.sh` (needs the `wasm32-unknown-unknown` target and a
matching `wasm-bindgen-cli`) into `docs/app/`.
