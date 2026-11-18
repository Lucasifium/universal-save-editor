#!/bin/sh
# Opens public sample saves on their game screens and checks each one: it must land on its
# screen, hold editable values and write back unchanged. The saves are not part of the repository.
#
#   scripts/check-samples.sh samples.tsv
#
# samples.tsv: one `<screen><TAB><path to a save>` line per save. Screen names are the ones in
# tests/screen-samples.tsv. Set SAVE_EDITOR_STRICT=1 to also fail when a screen marked `sample`
# there has no listed save that opened.
set -e
[ -f "$1" ] || { echo "usage: $0 samples.tsv" >&2; exit 2; }
SAVE_EDITOR_SAMPLES="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")" exec cargo test --release screens_open_their_samples -- --nocapture
