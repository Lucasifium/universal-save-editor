# Changelog

## 0.4.0

### New game screens
- Persona 3 Reload and Persona 3 Portable; Celeste; Sons of the Forest clock and weather
- The Sims 4 households; Deep Rock Galactic: Survivor
- Kingdom Hearts III, Trails to Azure / Ao no Kiseki KAI, Trails in the Sky 1st Chapter, Rune Factory 4 Special, FANTASIAN Neo Dimension
- Final Fantasy XIII (PC), Final Fantasy VIII Remastered and Pixel Remaster I-VI, Tales of Symphonia, GTA III and Vice City
- Warhammer 40,000: Rogue Trader, BioShock Infinite, Prey (2017), Ace Combat 8, Stellar Blade
- SnowRunner garage, Space Haven fleet, Terra Invicta factions, Bitburner, Prison Architect, Sonic Mania, Need for Speed: Most Wanted (2005)
- FF7 Remake and Rebirth: gil, inventory and materia; Kenshi squad stats; Cave Story+ and Cave Story
- Many more catalog entries (Kerbal Space Program, The Long Dark, Satisfactory, Civilization VII, Astroneer and others); about 265 games in total

### Features
- Save manager (its own Backups menu next to File, Edit, View and Tools): automatic and named backups stored next to the save, restore with a backup of the current file first, and set a save aside to start a new game
- Verified writes: backups and restores go to a temporary file, are flushed, read back and checked before anything is replaced; an unreadable save is never overwritten
- Console cheat patches (Save Wizard and script-engine patch files) with a Tools window and `patch` command
- Single PS1/PS2/PS3 save files (.mcs, .psx, .psu, .max, .cbs, .xps/.sps, .psv), Dreamcast and Sega Saturn saves, encrypted console saves through their patch codes
- Templates: a JSON file per unknown game names its values and checksum
- Keyed JSON, CBOR, Snappy, Unreal multi-section and typed-tree saves open as values; keyed-JSON games such as Megabonk and Eiyuden Chronicle

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
