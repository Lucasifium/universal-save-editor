# Universal Save Editor

> **Free, and it always will be.** No price, no ads, no account, no paid version, no online service, no
> rate limits. If anyone is charging you for this, it isn't from here.

An offline save editor for 100+ PC games: Cairn, Terraria (characters and a world map editor),
Oxygen Not Included, RimWorld, Stardew Valley, Onimusha: Way of the Sword, Elden Ring, Dark Souls,
Cyberpunk 2077, the Persona, Yakuza and Final Fantasy games, Ren'Py visual novels, RPG Maker games
and more (the full list is below). Written in Rust (egui). Nothing is uploaded anywhere, and there are
no rate limits.

**Download:** grab the latest build from the [Releases](../../releases/latest) page (Windows, Linux,
Android). The program file is still called `SaveEditorOffline`.

## Why this exists

I got sick of online-only save editors that were rate limited, so I set out to make an offline
save editor for a few RPG Maker games and Ren'Py. From there the scope ballooned: why not make it
edit as many games as possible? I went through my game list and added games as I went. It started
as what I'd call Save Editor Offline. Now it's closer to a universal save editor.

My favourites so far:

- **Cairn save editor**: as far as I know, no other save editor can add items, infusions and so on.
  Edit the backpack, piton belt and chalk pouch, and running buffs. It can also swap you to Marco on
  the story summit climbs. That will probably cause some issues, since I haven't tested a full
  playthrough with it.
- **Terraria world editor and map editor**: paint blocks, walls and liquids on the map, with points of
  interest (Shimmer, Plantera bulbs, Enchanted Sword shrines, chests). Characters have their own
  Terraria character editor.
- **Onimusha: Way of the Sword save editor**: it just came out, so many fields are still missing as
  far as I can tell.
- **RimWorld save editor**
- **Stardew Valley save editor**
- **Oxygen Not Included save editor and map editor**: paint elements, temperature and mass on every
  asteroid and rocket interior, plus duplicants, rockets and world settings.

## Run

    cargo run --release -- path\to\savefile      # path is optional

or run one of the builds in `dist\` (see below). On desktop you can also drag a save onto the program or
the window. The welcome screen lists recent files and common save folders (Ren'Py games, Unity, Saved
Games; on Android: RetroArch, Downloads).

## Builds

    .\build_all.ps1                     # everything the machine can build, into dist\
    .\build_all.ps1 -Only android       # or windows-x64, windows-arm64, linux-x64, linux-arm64

| File in `dist\` | Runs on |
|---|---|
| `SaveEditorOffline-windows-x64.exe` | Windows 10/11, Intel/AMD |
| `SaveEditorOffline-windows-arm64.exe` | Windows 11 on ARM (Snapdragon laptops) |
| `SaveEditorOffline-linux-x64.tar.gz` | Linux with glibc 2.28+ (2018 on), X11 or Wayland |
| `SaveEditorOffline-linux-arm64.tar.gz` | ARM64 Linux (Raspberry Pi 4/5 64-bit OS, etc.) |
| `SaveEditorOffline-android.apk` | Android 8+ phones and tablets (ARM64, plus x86_64 for emulators) |
| `SaveEditorOffline-macos.zip` | macOS 11+, Apple Silicon and Intel; built on GitHub when started by hand |

What the script needs: Rust targets for each platform, Visual Studio Build Tools with the ARM64
compiler, `cargo-zigbuild` + `pip install ziglang` (Linux), and the Android SDK/NDK plus a JDK (Android).
The SDK is found through `ANDROID_HOME`, then Android Studio's usual place
(`%LOCALAPPDATA%\Android\Sdk`), then an `android-sdk` folder next to this one.

**macOS** can only be built on a Mac, so `.github/workflows/build.yml` builds a universal `.app` on
GitHub's Macs when you start it by hand (*Actions* › *macOS build* › *Run workflow*; pushing
doesn't start it, so no Actions minutes are used otherwise); download it from the run's *Artifacts*. It isn't
notarized: the first time, right-click the app › *Open*.

**Android**: the first build creates a signing key in `packaging\android\` (`release.keystore` and its
password). Keep both: Android only installs updates signed with the same key. On the phone, the app asks
for *All files access* the first time you open a file, so it can read and write saves where emulators
keep them. Android's NativeActivity has no on-screen keyboard support, so `packaging\android\java` adds a
small activity that passes the keyboard's typing to the app.

## Game screens

The games below open on a screen of their own (the first tab: *Character*, *Trainer*, *Player*,
*House* or *Run*), laid out like the game shows things. Edits there go through the same checks,
undo and checksums as everywhere else, and the *Values* and *Tree* tabs still show everything.

| Game | Screen |
|---|---|
| Elden Ring, Dark Souls III / II / I | Name, class and level; currency; attribute bars (level follows); HP / FP / Stamina meters; runes or souls to the next level (Elden Ring, Dark Souls III and I); inventory grids (with a search box across every list) for items, key items, storage, weapons (level and affinity / infusion), armor and talismans / rings; a *Max upgrade materials* button (Elden Ring: every Smithing Stone, Somber Smithing Stone and Glovewort to 999; Dark Souls: every titanite to 99) |
| Sekiro | Sen, Attack Power, carried Spirit Emblems, skill points to spend, skill experience (points earned and total experience shown), Vitality and Posture; items, key items and storage as grids with an item picker and quantity; a *Max upgrade materials* button (the nine prosthetic materials to the 99 you can hold); skills and prosthetic tools |
| Terraria (characters) | Life and mana; permanent upgrades; inventory, coins and ammo, equipment (armor, vanity, dyes, pets) and storage (piggy bank, safe, forge, void vault) as item grids |
| Terraria (worlds) | A map editor: pan and zoom the whole world, see what every tile is, points of interest (shimmer, Enchanted Sword shrines, dungeon biome chests and other dungeon chests, Jungle Temple chests, Plantera's Bulbs, Life Fruit, orbs and hearts, altars, larvae, the Jungle Temple, hives, marble and granite caves, spider nests, glowing mushrooms, meteorites; each kind and every marker can be hidden), paint blocks, walls and liquids, erase them, and move the spawn point (chests and furniture are left alone; each stroke is one undo step). Also the world settings (name, difficulty, evil, hardmode, time, spawn and dungeon), special seeds, bosses defeated, events, rescued NPCs, every chest's contents (with search), signs and the town NPCs' homes |
| Pokémon (Gen I–III) | Trainer card and badges; party cards (species, level, nickname, moves and PP, HP, IVs and more); PC boxes as a grid; bag pockets |
| Skyrim, Fallout 4 | Character header; game time; every statistic. Skyrim: the character's real level (in the character itself and the header) |
| Hades, Hades II | Main currencies; runs, play time and last death; all resources by kind (currencies, plants, ores, fish, reagents) |
| Stardew Valley | Gold, Qi Gems, casino coins, Golden Walnuts; health and energy; the six skills (level and XP, with the XP the next level needs); farm details |
| Minecraft (`level.dat`) | Health, food and XP; game mode; world settings; inventory and ender chest grids |
| Balatro | Money, ante, hands and discards; this round; jokers and consumables; poker hand levels |
| Hollow Knight, Slay the Spire | Geo, masks, notches, abilities and charms; a *Max upgrade materials* button (all 6 Pale Ore); gold, HP, deck, relics and potions |
| RPG Maker MV / MZ | *Party*: gold, steps and play time; each character's name, level, EXP, HP / MP / TP, stat bonuses and learned skills (by id) |
| Cairn (Story and On the Trail) | *Climber*: who climbs (Aava or Marco); play time, falls and deaths; the running buffs (Focus, Grip, Temperature, Grit, Burst and the rest: seconds left, source, strength; start, stretch or end them); the backpack, piton belt and chalk pouch as item grids with the game's own item names (add, remove or swap items, uses and sips left, what a bottle holds); the slot's saves, marking the one the game continues from |
| Onimusha: Way of the Sword | Each save slot (place, play time, difficulty, objective); red souls, money, health and Oni energy; items and charms by name (on hand and in storage; charm level); equipment experience. Decrypts Capcom's RE Engine encryption with your Steam account, found automatically |
| Stellaris, Hearts of Iron IV, EU4, CK2 | *Empire*: Stellaris stockpiles (energy, minerals, alloys…) and every resource; HOI4 political power, stability, war support, command power and experience; EU4 treasury, manpower, prestige, stability, monarch power and every country value; CK2 ruler wealth, prestige, piety and attributes. Ironman (binary) saves are recognised |
| RimWorld | *Colony*: every colonist's skills (level and passion, modded passions included) and every silver stack, edited in place even in 100+ MB saves |
| Oxygen Not Included | *Colony*: every duplicant's attribute levels and experience, total experience and learned skills; a *Map* of every asteroid and rocket interior (elements, temperature and germ views; paint elements with mass and temperature, dig out to vacuum, set temperature or mass, clear germs, reveal the fog; geysers, teleporters, Gravitas finds, story traits, artifacts and the Printing Pod as toggleable markers); *Worlds & rockets*: names, discovered, sunlight and cosmic radiation, and what's on the star map |
| Subnautica, Below Zero | *Slot*: game mode (Survival / Freedom / Hardcore / Creative), play time, vehicles |
| Mass Effect 2, Mass Effect 3 (original and Legendary Edition) | *Shepard*: level, experience, talent points, difficulty; credits, medi-gel, element zero, iridium, palladium, platinum, probes, fuel, grenades; power ranks; spare ammo; each squad member's level, talent points and powers. The save's checksum is updated |
| XCOM: Enemy Unknown / Within | *HQ*: money; every soldier's rank, XP, kills, HP, aim, mobility and will (the fallen listed last); storage counts. Every saved object's numbers are in *Values* |
| Final Fantasy VII (PC / 2013 Steam) | *Party*: each slot's gil and play time; every character's level, experience, kills, limit level and gauge, HP / MP and the six stats. Slot checksums are updated |
| Final Fantasy VII Remake (Intergrade), Rebirth | *Party*: each character's level, HP / max HP, MP / max MP and experience |
| Green Hell | *Survivor*: health, energy, hydration, the three nutrients, dirtiness, sanity, every skill and the statistics |
| Resident Evil Requiem (and RE Engine games with the same layout) | *Slot*: health of the player and other characters, inventory counts (with the ammo or contents they hold), save count |
| Hogwarts Legacy | *Character*: gold, experience, talent points, every resource in the bag and talent levels, from the game's own database inside the save (unpacked and packed again with Oodle, see below); without an Oodle library, the load-screen name, house and level |
| The Witcher 3 | *Geralt*: crowns, level, unspent skill points and experience (repacked with LZ4 on save) |
| Cyberpunk 2077 | *V*: eurodollars, from V's own inventory (found through the save's section table; the table's position is updated when the save is repacked) |
| Dying Light 2 / The Beast | *Survivor*: money, health, the Traversal / Combat / Inhibitor levels and points, every skill's level (Techland's self-describing save format; every saved number is in *Values*) |
| Dying Light | *Survivor*: the skill tree levels (every copy the save keeps) |
| Kenshi | *Faction*: the player's cats |
| Palworld | *Player*: technology points, ancient technology points, Lifmunk effigies and the record counters (Oodle, see below) |
| Final Fantasy IX (Steam) | *Party*: every saved game in `SavedData_ww.dat` (decrypted and encrypted again): gil, and each character's level, experience, HP / MP and base stats |
| Final Fantasy X (HD Remaster, PC) | *Party*: gil, and every character's and aeon's HP, MP, maxima and stats. The checksum (the game's own CRC-16) is updated |
| Final Fantasy X-2 (HD Remaster, PC) | *Gullwings*: gil; Yuna, Rikku and Paine's level, experience, HP / MP and stats. Checksum updated |
| Persona 4 Golden (PC) | *Protagonist*: yen, level, experience and the five social stats; the matching `.binslot` checksum is updated |
| Persona 5 Royal (PC) | *Phantom Thieves*: decrypted, unpacked and packed again; money, HP / SP, levels, experience and Persona stats for the party and Joker's Personas |
| Metro 2033 / Last Light Redux, Exodus | *Artyom*: military-grade rounds, every ammo type and (Exodus) crafting metal and chemicals in the level saves (`.player`); auto and quick saves are unpacked (LZ4) for Value search. CRC-32 updated |
| Yakuza 0 | *Kiryu & Majima*: both characters' money (up to ¥10 trillion) and CP, the uncollected real estate and cabaret takings, and the difficulty |
| God of War (2018) | *Kratos*: every save in `game.sav` (newest first, by title and date): XP, hacksilver, every resource and Yggdrasil's dew, the equipment each save owns, and armour, pommels and talismans it doesn't have yet to add (at their base level) |
| God of War Ragnarök | *Kratos*: XP for Kratos, Atreus and Freya, hacksilver and divine ashes, resources, Yggdrasil's dew, health and rage, the level of every weapon, armour piece, attachment, shield and relic, and new armour, shields and attachments to add at any level (campaign saves; a piece of the same type you own is the model) |
| Yakuza Kiwami | *Kiryu*: money and the experience he hasn't spent yet (save and autosave files) |
| Yakuza Kiwami 2, Like a Dragon (Dragon Engine) | *Player*: decrypted (XOR) and its checksum updated (Adler-32 or CRC-32); money, HP, Kiwami 2's experience pools and Like a Dragon's whole party (level, experience, HP, MP), with every other value in the tree |
| Yakuza 6, Kiwami 3, Judgment, Lost Judgment, Like a Dragon Gaiden, Infinite Wealth, Ishin!, Pirate Yakuza (Dragon Engine) | Decrypted with each game's key and checksum kept right (keys from rgg_save_tool_py); values in the tree, and the Yakuza screen where the save has the same layout. Not tried on sample saves of these games yet |
| Yakuza 3, 4, 5 Remastered | Hex view and Value search |
| Ren'Py visual novels | *Story*: the game's numbers, flags and text variables with a filter, money and the like up front |

## Credits for God of War

The God of War formats and item names come from the Forja save editors by wilypizza-svg (José Murilo), which credit kg971 and xcier.

## Oodle (Palworld, Hogwarts Legacy)

Palworld 0.6+ and Hogwarts Legacy compress their saves with Oodle, which has no open-source
implementation the editor may include. Instead it uses the Oodle library that many games already
install (`oo2core_*_win64.dll`, e.g. with Sekiro or Telltale games), loading it while running, just as
those games do; nothing of Oodle is part of this program. It's found in your Steam and Epic game
folders automatically (remembered afterwards), or set `SAVE_EDITOR_OODLE` to the library's path.

## Select game

*Select game* (toolbar and File menu) lists every game the editor knows — about 95, Mass Effect 1–3, the Legendary Edition and Andromeda among them — with where it
usually keeps its saves and what the files are called, and finds them on your computer (Steam
libraries and `userdata`, Documents wherever Windows has it, AppData, Saved Games, Ubisoft
Connect). Pick a game, then a save, to open it; if none are found it says so. *Open…* and dragging
a file onto the window work as before for anything else.

Capcom RE Engine saves need the Steam account they belong to; away from Steam's folders (on a
phone, or a copied save) enter your 17-digit Steam ID in *Tools › Decrypt with password*.

Games whose format isn't documented open in the hex view with Value search; for packed formats
the data is unpacked first and packed again on save: Cyberpunk 2077 and The Witcher 3 (LZ4
chunks), XCOM 2 / War of the Chosen (LZO chunks, both checksums updated), Mass Effect 1
Legendary Edition (zlib, checksum updated), Civilization V (zlib), Dying Light (gzip), Ubisoft `.save.z` (zlib), BioShock Infinite (Unreal Engine 3 zlib chunks, MD5 updated), BioShock Remastered (zlib blocks, each packed back into its own space), Control (CRC-32 updated) and Cairn (the files inside each slot pack; the newest save, the one the game continues from, opens first; the pack's size fields are kept right when a save grows). Other RE Engine
games (Resident Evil Requiem, Dragon's Dogma 2, Monster Hunter Wilds and Stories 3, Pragmata,
Dead Rising Deluxe Remaster, Kunitsu-Gami) decrypt and open with their values.

Big files open on a worker thread, and unpacking and decryption use every core.

## What it opens

Wrappers are peeled off automatically and re-applied in the same order when you save. Wherever
possible only the values you edit are rewritten — everything else stays byte-for-byte identical.

**Specific games** (full editors with friendly names)

| Game | Files | What you can edit | Verified against |
|---|---|---|---|
| Pokémon Red / Blue / Yellow and Gold / Silver / Crystal (GB/GBC, English releases) | `.sav` (32 KB) | Trainer, money, coins, badges, play time, every bag pocket (kept gap-free like the game), party and all PC boxes: species, nickname, level (sets experience and stats), moves, PP and PP Ups, IVs, stat experience, held item and friendship (Gen II); shiny and gender follow the IVs | spec-based tests (Bulbapedia, PKHeX, pret decompilations): main, bank, box and backup checksums |
| Pokémon Ruby / Sapphire / Emerald / FireRed / LeafGreen (GBA, any emulator) | `.sav` (128 KB) | Trainer, money, coins, play time, bag (item drop-downs), party and PC Pokémon: species, nickname, level (sets experience and stats), moves, PP, IVs, EVs, ability, held item, ball | sample saves: section checksums, Pokémon checksums and the game's own stored stats all match |
| Terraria (1.4 – 1.4.5) | `.plr` | Name, difficulty, life/mana, permanent upgrades, inventory, coins/ammo, equipment, piggy bank, safe, forge, void vault (item and prefix drop-downs) | a sample 1.4.5 character |
| Terraria worlds (1.4 – 1.4.5) | `.wld` | World info, special seeds, bosses, events, rescued NPCs, chests, signs; map painting (blocks, walls, liquids, spawn) | sample 1.4.4 and 1.4.5 worlds (small, medium and large) |
| Elden Ring | `ER0000.sl2` | Runes and the eight attributes (level follows automatically); items, key items and storage (change items and quantities up to 999, add new ones); weapons (swap, add, remove, upgrade level and affinity within what each weapon can have), armor, talismans and Ashes of War (swap, add, remove; an ash fitted to a weapon stays put); Sites of Grace and bosses defeated (tick, or unlock all); Shadow of the Erdtree item names; *Tools › Copy character to another slot* duplicates a character; shows class, rune memory, HP/FP/Stamina and the runes needed for the next level; other slots via *Open another file in this archive* | sample saves + upgrade data from the game's regulation 1.17 |
| Dark Souls III | `DS30000.sl2` | Souls and the nine attributes (soul level follows); items (99 carried, 600 stored), weapons (add, swap, remove, level up to +10 for weapons that take infusions and +5 for the rest, infusion), armor and rings; shows soul memory, HP/FP/Stamina and the souls for the next level | a sample save (gear read, weapons and armor added) |
| Dark Souls II: Scholar of the First Sin | `DS2SOFS0000.sl2` | Souls and the nine attributes (soul level follows); items (99 carried, 999 in the box), weapons (add, swap, remove, level up to +5 or the level they have), armor, rings and key items; shows soul memory and HP | a sample save |
| Dark Souls Remastered | `DRAKS0005.sl2` | Souls, humanity and the eight attributes (soul level follows the starting class); items, key items, weapons (level up to +5 or the level they have; equipped weapons too), armor and rings (add, swap and remove; quantities up to 99, ammunition 999); shows class, HP/Stamina and the souls for the next level | not yet tried on a sample save: built from er-save-manager's and alfizari's notes on the format |
| Sekiro | `S0000.sl2` | Sen, Attack Power, Spirit Emblem capacity, skill points and skill experience, Vitality and Posture; items, key items and storage (change, add, remove, set quantities); learned skills and tools listed | sample saves + alfizari's Sekiro editor's offsets |
| Skyrim (LE/SE/AE/VR, Enderal), Fallout 4 | `.ess`, `.fos` | Game time, misc stats, global variables; header shown (LZ4 recompressed) | sample saves |
| Hades, Hades II | `.sav` | The whole game state (resources, upgrades, flags…); search by name | sample saves |
| Balatro | `.jkr` | Run, profile and settings values | sample saves |
| Minecraft (Java/Bedrock) | `level.dat`, `playerdata/*.dat`, region `.mca` chunks | Everything in the NBT | sample worlds |
| Stardew Valley, RimWorld, Celeste… | XML saves | Everything (formatting kept) | sample Stardew save |
| Hollow Knight | `user*.dat` | Everything in the JSON | spec-based tests |
| Slay the Spire | `.autosave` (base64 + XOR) | Everything in the JSON | spec-based tests |
| Other GB / GBA games (and Japanese Gen I/II, ROM hacks) | `.sav`, `.srm` | Hex editor with value search | — |
| mGBA save states | `.ss0`–`.ss9` | Emulator memory (hex, value search) | sample save states |

**Game engines**

| Format | Typical files | Verified against |
|---|---|---|
| Ren'Py saves + persistent | `1-1-LT1.save`, `persistent` | Ren'Py 8.5's own Python + signature code |
| RPG Maker MV / MZ | `.rpgsave`, `.rmmzsave` | round-trip tests |
| RPG Maker XP / VX / VX Ace | `.rxdata`, `.rvdata`, `.rvdata2` (Ruby Marshal) | spec-based tests |
| Unreal Engine 4/5 | `.sav` (GVAS) | spec tests + a sample save |
| Unity — .NET BinaryFormatter | `.dat`, `.sav`, … | .NET's own BinaryFormatter; ~20 sample saves |
| Unity — Easy Save 3 | `.es3` (JSON, optionally AES + gzip) | .NET's AES/PBKDF2; default password tried automatically |
| Godot | `store_var` binary saves; `open_encrypted_with_pass` files | spec-based tests (unsupported variants fall back to hex) |
| LÖVE / Lua games | `return { ... }` table saves (plain or deflated) | sample saves |
| TyranoScript / TyranoBuilder | `.sav` (URL-escaped JSON) | JavaScript `escape()` test vectors |
| Twine / SugarCube | exported saves (LZString) | round-trip tests |
| Barotrauma | `.save` (gzip file bundle) | sample saves |
| Palworld (older) | `.sav` with `PlZ` header | round-trip tests (`PlM`/Oodle saves aren't supported) |

**Data formats** — JSON, XML (span-patched, keeps formatting), INI/CFG, TOML (keeps comments),
YAML (`.yml`/`.yaml`), Apple plist (binary and XML), MessagePack, BSON, SQLite (real `UPDATE`s;
IDs and keys read-only, 0/1 check-constrained columns shown as switches), Python pickle.

**Wrappers** — gzip, zlib, raw deflate, zstd, LZ4, xz, bzip2, zip (pick any entry), Base64 (plain or
line-wrapped), UTF-8 BOM, UTF-16, XOR, Easy Save 3 AES, Godot AES, Terraria AES, Hollow Knight AES,
FromSoftware BND4 (MD5 + AES), Minecraft region chunks, mGBA PNG save states, Supergiant (Adler-32 + LZ4).

**Anything else** opens in the hex editor with value search and named offsets.

Encrypted saves: *Tools › Decrypt with password* (Easy Save 3, Godot). Archives with several files:
*Tools › Open another file in this archive*.

## Finding values

*Tools › Compare with another save* lists every value that differs between the open save and
another one (e.g. before and after you earned gold), with a one-click **Use theirs**.

## Editing

- **Values** (default view): every editable value in one aligned list, grouped into collapsible
  sections, with search (Ctrl+F) across names *and* current values.
  - Numbers use a checked text box: invalid input turns red and is never written. Whole numbers
    stay whole, and `2,500` is accepted. Use −/+ for quick tweaks.
  - On/off values use a switch.
  - **Advanced** shows engine internals (Ren'Py internals; RPG Maker system/map/player…) and
    read-only values.
- **Tree**: the full nested structure. **Raw JSON**: edit JSON text directly.
- Changed values are highlighted in orange with a one-click revert.
- **Undo / Redo** (Ctrl+Z / Ctrl+Y), and a **Changes** panel listing old › new for each edit.

## Friendly names

- Keys are shown as readable labels (`store.max_hp` → **Max HP**). Turn on **Raw keys** to see
  the original key beside them.
- **RPG Maker**: the game's `data/` folder is found automatically and used to name variables,
  switches, items, weapons, armors and actors. If it isn't found, click the yellow chip or use
  *Tools › Locate game data folder*.
- **Binary saves**: use *Value search* to find a value, click the result, then give it a name under
  *Named values*. Names are stored in `<save>.labels.json` and come back next time.

## Safety

- **Verified saves**: the new file is decoded again *before* anything is written. If it doesn't
  read back exactly as shown on screen, the save is aborted and your file is left untouched.
- **Atomic writes**: the save goes to a temp file and is then swapped in, so a crash can't leave a
  half-written save.
- **Backups**: every save first copies the file as it was into the app's folder (`backups\<name>-<hash>`, last 30 kept); *File › Restore previous version* brings any of them back. The very first original is also kept as `<name>.bak`
- **Game running**: if the game that owns the save is running, the app says so when opening and warns before saving (games write their save back when they autosave or quit)
- **Review before saving**: Save lists every change in words first (can be turned off in that dialog)
  (*File › Restore original from backup*).
- You're asked before closing or opening another file with unsaved changes.
- Values are range-checked for their real type (a 16-bit number can't be set to 70000, etc.).
- Lists of known values (Pokémon, moves, items, Terraria items and prefixes…) are drop-downs with
  a filter box, so you can't pick something the game doesn't have.
- Values that depend on each other are kept consistent (a Pokémon's level, experience and stats;
  Elden Ring's level and attributes), and undo restores all of them together.
- Game checksums (Pokémon sections and Pokémon data, FromSoftware MD5s, Hades Adler-32) are
  recalculated on save.
- Ren'Py 8.1+ saves are re-signed with your own Ren'Py key so the game loads them normally.

## Preferences

Theme (dark/light), zoom (Ctrl +/−), recent files and view toggles are remembered in
`settings.json` in `%APPDATA%\SaveEditorOffline` (Windows), `~/Library/Application Support/SaveEditorOffline`
(macOS), `~/.config/SaveEditorOffline` (Linux) or the app's private storage (Android). Settings from
before the rename are copied over automatically.

## Tests

    cargo test

Manual checks (ignored by default): `cli_edit` edits any file from the command line,
`scan_real_saves` surveys sample saves read-only (detect, re-encode, simulated edit + verification), and
`bug_hunt` (`SAVE_EDITOR_SCAN=list.txt`) stress-tests a list of saves: ~150 edits of every kind per file,
save + re-open comparison, and damaged/truncated copies to make sure nothing crashes.


## Credits and legal

- **Free forever.** This project will never be sold or put behind an account, ads or a paywall, and the GPL licence keeps every shared copy and fork free and open too.
- **Not affiliated with any game company.** All game names, item names and trademarks belong to their
  owners. This project isn't affiliated with or endorsed by any developer or publisher. Game names
  appear here only to say which saves the editor can open.
- **No game files are shipped.** The editor contains no game code, art, sound or other assets. It
  ships only the ids and names it needs to show what's in a save. Oodle-compressed saves use the
  Oodle library the game itself installed; it isn't included.
- **Built on others' work.** Save layouts and item tables from other open-source editors are used under
  their licences (MIT, Apache-2.0, zlib), and every crate compiled into the program is listed with its
  licence. See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
- **Use at your own risk.** Back up your saves first. Editing saves is meant for single-player games:
  don't use edited saves in online or multiplayer modes, where it can break the game's rules and get
  an account banned. The editor never touches online services or anti-cheat.

## Licence

Copyright (C) 2026 Lucasifium. Universal Save Editor is free software under the
[GNU General Public License v3.0 or later](LICENSE): you can use, share and change it, and any version
you share must stay free and open source under the same licence. It comes with no warranty.
