#!/usr/bin/env bash
# Builds the browser version into docs/app/ (served with the website).
# Needs: rustup target add wasm32-unknown-unknown
#        cargo install wasm-bindgen-cli --version <the wasm-bindgen version in Cargo.lock>
set -euo pipefail
cd "$(dirname "$0")/.."
# Paths inside the program (panic messages) name no folder of this computer: the home folder
# becomes ~, and the crate registry and toolchain get neutral names (as in packaging/rustflags.py;
# the last matching prefix wins, so the most specific goes last).
sep=$'\x1f'
export CARGO_ENCODED_RUSTFLAGS="--remap-path-prefix=$HOME=~${sep}--remap-path-prefix=${RUSTUP_HOME:-$HOME/.rustup}=/rustup${sep}--remap-path-prefix=${CARGO_HOME:-$HOME/.cargo}=/cargo"
cargo rustc --profile web --lib --crate-type cdylib --target wasm32-unknown-unknown
out=docs/app
rm -rf "$out"
mkdir -p "$out"
wasm-bindgen --target web --no-typescript --out-dir "$out" target/wasm32-unknown-unknown/web/universal_save_editor.wasm
cp web/index.html "$out/index.html"
python3 -I packaging/check_clean.py "$out"
ls -la "$out"
