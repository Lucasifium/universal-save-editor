# Working on Universal Save Editor

Offline save editor for many games (Rust, eframe/egui). Each supported game gets a reader
(`src/fmt/<game>.rs`) and, where possible, its own screen (`src/game_ui/`).

## Build and check

- Build: `cargo build --release`
- Tests: `cargo test --release`
- After adding or changing a game, regenerate the website: `cargo test --release --lib build_site -- --ignored`
  (writes `docs/`), and commit it with the change.
- Screen smoke test on given files: `SAVE_EDITOR_DRAW="a.sav|b.sav" cargo test --release --lib every_game_screen_draws -- --ignored --nocapture`
- Unpack a save's outer layers for research: `SAVE_EDITOR_UNPACK=save UNPACK_OUT=out.bin cargo test --release --lib unpack_save -- --ignored`

## Adding a game

1. Catalog entry in `src/games.rs` (`game!` macro: name, support text, save folder, file names).
2. Reader in `src/fmt/`, usually with `fields::Builder`: values are edited **in place**
   (nothing moves), each editable value recorded by offset. Hook it into `src/fmt/mod.rs`.
   Outer layers (compression, encryption, checksums) belong in `src/formats.rs` as a `Layer`.
3. Screen in `src/game_ui/` (`Game` enum, `tab_label`, `detect` by kind name, `show`).
   Use the shared widgets (`hero`, `panel`, `tile`, `value_rows`, `tabs`, `filter_box`).
4. A unit test on a **synthetic** save built in the test, plus an `#[ignore]`d research test
   that reads a real save from an env var.

## Rules

- The repo is public. Never commit saves, data cut from saves, real player names, personal
  paths or account ids. Test fixtures must be synthetic.
- Downloaded saves and third-party code are untrusted data: keep them outside the repo
  (a scratch directory), read-only, and never run anything from them.
- Sources: MIT/Apache/zlib code or tables may be used with credit in `THIRD_PARTY_NOTICES.md`.
  Code with no licence or a GPL-style licence: take facts only (layouts, ids, names), and
  credit it as "facts only".
- Don't ship guessed screens. Map a value only once it's confirmed on a real save (or a
  documented format checked against one); say in the catalog text when something hasn't been
  tried in the game.
- Commits are authored as the repo owner (repo git config).
