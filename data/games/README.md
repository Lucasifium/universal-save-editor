# Game definitions

A game whose save reads as a plain tree (JSON, Easy Save 3, Unreal GVAS, MessagePack and the
like) can be described in a file here instead of in Rust. Every `*.toml` file in this folder is
embedded when the program is built and becomes one game screen, the same as the tables in
`src/game_ui/simple/`. Adding a game is adding one file; nothing else needs registering.

Definitions are checked after the Rust tables. Values are found by pointer and are left out of
the screen when a save does not have them.

## File format

```toml
name = "Example Game"               # shown as the screen's title; unique
tab = "Hero"                        # the tab label
accent = [90, 150, 220]             # colour of the screen, RGB
kind = "Easy Save 3 data"           # the kind name the reader gives the save; leave out for plain JSON
markers = ["/Hero/Gold", "/Force/Fire"]   # pointers that must all exist for the save to be this game's
subtitle = "Where the file lives and what the screen edits. Close the game first."
tiles = [                           # the big figures at the top: [caption, pointer]
    ["Gold", "/Hero/Gold"],
]

[[sections]]                        # a panel of labelled values; repeat for more panels
title = "Hero"
rows = [
    ["Gold", "/Hero/Gold"],         # [label, pointer]
    ["Level", "/Hero/Level"],
]

[[lists]]                           # an item list with names (repeat for more lists)
title = "Stash"
path = "/Stash"                     # pointer of the array or object holding the items
shape = "array"                     # array | map | records
id_key = "ItemId"                   # array: the key naming each item
qty_keys = ["Count"]                # array: the count's key, then keys of copies kept equal to it
# records: qty_key = "Count"        # an object of records keyed by id
# map: nothing more (an object of name: count)
names = [                           # [id, name]; ids without a name show as "Item <id>"
    ["Base_Carbon", "Carbon"],
]

[lists.max]                         # optional "Max crafting materials" button for this list
ids = ["Base_Carbon"]               # only these plain materials are raised (never key or story items)
cap = 9999                          # the game's own stack limit
caps = [["Base_Carbon", 500]]       # optional: ids whose cap differs
estimate = false                    # true when no public source states the cap
about = "What the button does, for its hover text."

[[tables]]                          # one row per entry of an array (party members, saved companies)
title = "Party"
path = "/Party"
name = ["/Name"]                    # pointers inside an entry whose text names the row
cols = [["Level", "/Level"]]        # editable columns: [heading, pointer inside the entry]
# only = ["Aria"]                   # optional: only rows with one of these names
# names = [["id_7", "Aria"]]        # optional: names for rows whose name text is an id
```

### Pointers

Pointers are JSON pointers into the decoded save (`/a/b/0/c`). Two extra steps help with
shapes that change order between saves: `/@123` selects the array entry whose `_Hash` is 123
(RE Engine saves), and `/=name` selects the entry whose `key` is `name`.

### Rules

- Unknown keys, a pointer that does not start with `/`, or a list without the keys its shape
  needs make the file invalid, and the tests fail.
- Every text in a file (subtitle, titles, labels, headings, `about`) must be translated in
  `src/i18n/*.txt` (English line, `= translation`, blank line, in all six languages). The
  translation tests list what is missing. Names of games and items stay as they are.
- A "Max crafting materials" list must only hold materials with a researched cap; leave the
  `[lists.max]` table out when the cap cannot be confirmed.
- Check a new game against a real save: it must open on its own screen, read sensibly and write
  back unchanged. Add the save's neutral sample entry to `tests/screen-samples.tsv` when one
  can be shared.

## Moving a Rust screen here

`DUMP_GAMES="Game name|Other game" cargo test --release --lib dump_definitions -- --ignored`
writes `data/games/<name>.toml` for the named Rust screens. Delete the Rust table afterwards
and run the full tests.
