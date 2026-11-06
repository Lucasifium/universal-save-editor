#!/usr/bin/env bash
# Builds the browser version into docs/app/ (served with the website).
# Needs: rustup target add wasm32-unknown-unknown
#        cargo install wasm-bindgen-cli --version <the wasm-bindgen version in Cargo.lock>
set -euo pipefail
cd "$(dirname "$0")/.."
# Paths inside the program (panic messages) say ~ instead of this computer's home folder.
export CARGO_ENCODED_RUSTFLAGS="--remap-path-prefix=$HOME=~"
cargo rustc --profile web --lib --crate-type cdylib --target wasm32-unknown-unknown
out=docs/app
rm -rf "$out"
mkdir -p "$out"
wasm-bindgen --target web --no-typescript --out-dir "$out" target/wasm32-unknown-unknown/web/save_editor_offline.wasm
cp web/index.html "$out/index.html"
ls -la "$out"
