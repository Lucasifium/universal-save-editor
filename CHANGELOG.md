# Changelog

## Unreleased
- Backup & restore saves (File › Backup & restore saves): automatic and named backups stored next to the save, restore with a backup of the current file first, and back up and start a new game
- Backups and restores are written to a temporary file, flushed, read back and checked before they replace anything; a save that can't be read is never overwritten; a save set aside for a new game can be restored straight from the window or Recent files

## 0.3.0

### New game screens
- LEGO Batman: Legacy of the Dark Knight (decrypts the save; studs screen)
- The Sims (2000) / Legacy Collection and The Sims 2: families, funds, skills and personality
- Plants vs. Zombies (Game of the Year): money, shop, Survival progress, Zen Garden
- Nioh 1-3, Wo Long, Fatal Frame II Remake, Stranger of Paradise (own-key decrypt and re-encrypt)
- Monster Hunter Rise (Steam) and Resident Evil 4 (2023): encrypted saves read and written
- Fallout 3 and New Vegas, NieR:Automata, Yakuza 6, Clair Obscur: Expedition 33
- Half-Life: Alyx, Black Mesa, Dead Rising 1/2/Off the Record, Cuphead, Antimatter Dimensions
- Schedule I, Sons of the Forest, Death Must Die, Deep Rock Galactic, Cult of the Lamb, Fallout Shelter
- Phasmophobia, Lethal Company, R.E.P.O.
- Paradox ironman (binary) saves edited in place: Europa Universalis IV/V, Crusader Kings II/III, Hearts of Iron IV, Victoria 3, Imperator: Rome
- Catalog additions: Against the Storm, Brotato, Farming Simulator 25, Kingdom Two Crowns, Quasimorph, Risk of Rain Returns, The Planet Crafter, Touhou Mystia's Izakaya

### Existing games expanded
- Max crafting materials button on many more screens
- FFX abilities, items and arena; Fallout 1/2 perks and traits; Wasteland 3 reputation; Diablo II waypoints; Grim Dawn factions; Pokémon Gen III badges
- Mass Effect story flags, Golden Sun Djinn, Borderlands 2 storage, KOTOR and Neverwinter Nights item stacks
- RPG Maker party inventory, Hollow Knight spells and stag stations, Stardew friendships and backpack
- Slay the Spire add/remove, Undertale route flags, Lethal Company ship items, RimWorld stockpiles

### Features
- Command line: info, list, get, set, decode, encode and games, with JSON output, verification and backups
- Value search: unknown-value, range and compare scans
- Checksum finder: locate and rewrite a CRC, sum or hash in unknown saves
- Change the Steam account in a save
- Backups: named permanent copies, chosen folder, compare versions, import edited data back
- Emulator memory cards (PlayStation, PS2, DeSmuME .dsv), Xbox app / Game Pass containers
- Flash .sol, Haxe (OpenFL, HaxeFlixel), Unity PlayerPrefs, Unreal maps and sets, Dead Cells container
- Select game finds saves in Proton and Wine prefixes (Linux, Steam Deck)
- Android APK is no longer distributed; its source and build steps stay in the repo

## 0.2.0
Earlier release.
