# Universal Save Editor

> **Free, and it always will be.** No price, no ads, no account, no paid version, no online service, no
> rate limits. If anyone is charging you for this, it isn't from here.

An offline save editor and save manager for 300+ (300) PC games (edit saves, and back up, name and restore them): Cairn, Terraria (characters and a world map editor),
Oxygen Not Included, RimWorld, Stardew Valley, Onimusha: Way of the Sword, Elden Ring, Dark Souls,
Cyberpunk 2077, the Persona, Yakuza and Final Fantasy games, Ren'Py visual novels, RPG Maker games
and more (the full list is below). Written in Rust (egui). Nothing is uploaded anywhere, and there are
no rate limits.

**Download:** grab the latest build from the [Releases](../../releases/latest) page (Windows and Linux; macOS in major releases).

**No install:** the [browser version](https://lucasifium.github.io/universal-save-editor/app/) is the same
editor compiled to WebAssembly. It runs inside the page on your own computer and never uploads the save;
saving downloads the edited file. It can't search your folders (*Select game*), keep backups, or open
SQLite saves and the saves of Palworld and Hogwarts Legacy: use the downloaded app for those.

## Why this exists

Online-only save editors are often rate limited, so this is an offline save editor and manager that covers as many games as possible: RPG Maker games, Ren'Py and a growing list of others.

My favourites so far:

- **Cairn save editor**: as far as I know, no other save editor can add items, infusions and so on.
  Edit the backpack, piton belt and chalk pouch, and running buffs. It can also swap you to Marco on
  the story summit climbs. That will probably cause some issues, since I haven't tested a full
  playthrough with it.
- **Terraria world editor and map editor**: paint blocks, walls and liquids on the map, with points of
  interest (Shimmer, Plantera bulbs, Enchanted Sword shrines, chests). Characters have their own
  Terraria character editor.
- **Onimusha: Way of the Sword save editor**: it just came out, so many fields are still missing as
  far as I can tell. It can add upgrade materials you don't have yet, and has a Max crafting materials
  button for them.
- **RimWorld save editor**
- **Stardew Valley save editor**
- **Oxygen Not Included save editor and map editor**: paint elements, temperature and mass on every
  asteroid and rocket interior, plus duplicants, rockets and world settings.

## Run

    cargo run --release -- path\to\savefile      # path is optional

or download a build from Releases (below). On desktop you can also drag a save onto the program or
the window. The welcome screen lists recent files and common save folders (Ren'Py games, Unity, Saved
Games; on Android: RetroArch, Downloads).

## Downloads

Ready-made builds are on the [Releases page](../../releases):

| File | Runs on |
|---|---|
| `UniversalSaveEditor-windows-x64.exe` | Windows 10/11, Intel/AMD |
| `UniversalSaveEditor-windows-arm64.exe` | Windows 11 on ARM |
| `UniversalSaveEditor-linux-x64.tar.gz` | Linux x64 (glibc 2.28+) |
| `UniversalSaveEditor-linux-arm64.tar.gz` | Linux ARM64 (e.g. Raspberry Pi) |
| `UniversalSaveEditor-macos.zip` | macOS 11+ (major releases only) |

There is also a browser version on the project website. There is no ready-made Android app: the source is in the repo, and you can build the APK yourself (Android 8+) as described in [BUILDING.md](BUILDING.md).

## Game screens

The games below open on a screen of their own (the first tab: *Character*, *Trainer*, *Player*,
*House* or *Run*), laid out like the game shows things. Edits there go through the same checks,
undo and checksums as everywhere else, and the *Values* and *Tree* tabs still show everything.

| Game | Screen |
|---|---|
| Elden Ring, Dark Souls III / II / I | Name, class and level; currency; attribute bars (level follows); HP / FP / Stamina meters; runes or souls to the next level (Elden Ring, Dark Souls III and I); inventory grids (with a search box across every list) for items, key items, storage, weapons (level and affinity / infusion), armor and talismans / rings; a *Max upgrade materials* button (Elden Ring: every Smithing Stone, Somber Smithing Stone and Glovewort to 999; Dark Souls: every titanite to 99); Elden Ring also has a **Max crafting materials** button that fills the cookbook materials you carry or store (meat, bones, flowers, mushrooms, butterflies, stones…) to 999, leaving key items alone |
| LEGO Batman: Legacy of the Dark Knight | *Studs*: the studs wallet (every field the game keeps it in is written together) and play time; the save encryption is handled for you, collectibles are in the Tree (not yet tried in the game) |
| LEGO Star Wars: The Skywalker Saga | *Studs*: the stud wallet, from `COLLECTABLES.BLOB` in a `SLOTn` folder (the game saves it twice; both copies are written). Checked on a public 100% save, not yet tried in the game |
| LEGO Star Wars: The Video Game | *Studs*: the stud counter (`LSW.BIN`; the checksum and the copy in `CODE.BIN` are rewritten). Checked on public saves, not yet tried in the game |
| LEGO Star Wars: The Complete Saga | *Studs*: the stud counter (the save's checksum is rewritten). Checked on public saves, not yet tried in the game |
| LEGO Batman: The Videogame | *Studs*: the stud counter (the save's checksum is rewritten). Checked on public saves, not yet tried in the game |
| LEGO Indiana Jones: The Original Adventures | *Studs*: the stud counter (the save's checksum is rewritten). Checked on public saves, not yet tried in the game |
| LEGO Indiana Jones 2: The Adventure Continues | *Studs*: the stud counter (the save's checksum is rewritten). Checked on public saves, not yet tried in the game |
| LEGO Harry Potter: Years 1-4 | *Studs*: the stud wallet (the save has no checksum). Checked on public saves, not yet tried in the game |
| LEGO Harry Potter: Years 5-7 | *Studs*: the stud wallet (the save has no checksum). Checked on public saves, not yet tried in the game |
| LEGO Star Wars III: The Clone Wars | *Studs*: the stud wallet (the save has no checksum). Checked on public saves, not yet tried in the game |
| LEGO Pirates of the Caribbean | *Studs*: the stud wallet (the save keeps five copies of it; all are written). Checked on public saves, not yet tried in the game |
| LEGO Batman 2: DC Super Heroes | *Studs*: the stud wallet (the save has no checksum). Checked on public saves, not yet tried in the game |
| Older LEGO games (TT Games: Star Wars II, Marvel, Harry Potter Collection, Jurassic World, Batman 3, The Hobbit, The Lord of the Rings, City Undercover, DC Super-Villains, The Incredibles, Worlds) | The saves are found for you (Select game); they open in the Hex view for Value search. No dedicated screen yet |
| Everspace 2 | *Pilot*: credits, level, experience and device upgrade tokens; every crafting resource by name, with a **Max crafting materials** button (ores, crystals, scrap metal and basic crafting resources to 99,999, an estimate: the game states no cap; catalysts, rift and faction resources left alone). The save's zlib wrapper is handled for you; ships, items and missions are in the Tree (checked on a public save, not yet tried in the game) |
| Moonlighter | *Will*: gold, days passed and the banker's invested money; the backpack and equipped items by name with count (enchantment levels in the Tree). The `gameslot` file opens as JSON (checked on a public save, not yet tried in the game) |
| Wasteland 2 | *Rangers*: each ranger's level, experience, health, attribute, skill and trait points, kills and the seven attributes (skills and items are in the Tree); the save's `<@>` name markers are kept as they are (checked on a public save, not yet tried in the game) |
| ATOM RPG | *Party*: every party member's level, experience, seven attributes, seventeen skills and free skill, stat and perk points; each member's inventory with counts (the player's rubles are the Money item). The gzip packing is handled for you (checked on a public save, not yet tried in the game) |
| Salt and Sanctuary | *Character*: level, salt, gold, kills, deaths, sanctuary rests, corruption and the playthrough number from `datN.slv` (items and the skill tree are not edited yet; checked on public saves, not yet tried in the game) |
| Shadow Warrior 2 | *Lo Wang*: level, experience, money, crystal points, perk points, health, mana, rage and every kind of ammunition; weapons and upgrades are in the Tree. Each `.bin` save is a zip whose text is opened and packed again for you (checked on a public save, not yet tried in the game) |
| Sekiro | Sen, Attack Power, carried Spirit Emblems, skill points to spend, skill experience (points earned and total experience shown), Vitality and Posture; items, key items and storage as grids with an item picker and quantity; a *Max upgrade materials* button (the nine prosthetic materials to the 99 you can hold); skills and prosthetic tools |
| Terraria (characters) | Life and mana; permanent upgrades; inventory, coins and ammo, equipment (armor, vanity, dyes, pets) and storage (piggy bank, safe, forge, void vault) as item grids; a **Max crafting materials** button (1.4.4 and later) that fills the ores, bars, gems, woods, monster drops and other materials you already have to 9999, leaving critters, fish and boss summons alone |
| Terraria (worlds) | A map editor: pan and zoom the whole world, see what every tile is, points of interest (shimmer, Enchanted Sword shrines, dungeon biome chests and other dungeon chests, Jungle Temple chests, Plantera's Bulbs, Life Fruit, orbs and hearts, altars, larvae, the Jungle Temple, hives, marble and granite caves, spider nests, glowing mushrooms, meteorites; each kind and every marker can be hidden), paint blocks, walls and liquids, erase them, and move the spawn point (chests and furniture are left alone; each stroke is one undo step). Also the world settings (name, difficulty, evil, hardmode, time, spawn and dungeon), special seeds, bosses defeated, events, rescued NPCs, every chest's contents (with search), signs and the town NPCs' homes |
| Pokémon (Gen I–III) | Trainer card and badges (Gen III: each Gym badge switchable); party cards (species, level, nickname, moves and PP, HP, IVs and more); PC boxes as a grid; bag pockets |
| Skyrim, Fallout 4 | Character header; game time; every statistic. Skyrim: the character's real level (in the character itself and the header) |
| Hades, Hades II | Main currencies; runs, play time and last death; all resources by kind (currencies, plants, ores, fish, reagents) |
| Stardew Valley | Gold, Qi Gems, casino coins, Golden Walnuts; health and energy; every villager's friendship points (shown as hearts, with a raise-everyone-to-8-hearts button); the whole backpack with stack sizes, item quality and tool levels; the six skills (level and XP, with the XP the next level needs); farm details; the crafting materials in the backpack (wood, stone, fiber, clay, ores, bars, monster loot…) with a **Max crafting materials** button that fills them to 999, leaving quest items, minerals and artifacts alone |
| Planescape: Torment: Enhanced Edition | Coppers; the Nameless One and each companion: experience, level, hit points, abilities, alignment and item stacks (checked on real saves, not yet tried in the game) |
| Clair Obscur: Expedition 33 | Gold; each party member's level, experience, action points, health, attribute points and Lumina from Colours of Lumina; each weapon's level; item counts, with a **Max crafting materials** button that fills every Chroma Catalyst you have to 999 (an estimate: the game states no limit) (checked on a real save, not yet tried in the game) |
| Dead Rising | *Survivor*: Frank's level, PP, maximum health and skills, with buttons to learn every skill or reach level 50; the save's block checksums are rewritten (checked on public speedrun saves, not yet tried in the game) |
| Dead Rising 2 | *Survivor*: Chuck's level, PP and money; attack, speed, health, item slots and throwing as bars up to the game's maximum, with a button to max them; the CRC-32 is rewritten (checked on public speedrun saves, not yet tried in the game) |
| Dead Rising 2: Off the Record | *Survivor*: Frank's level, PP and money (the load screen's copy kept in step); attack, speed, health, item slots and throwing as bars up to the game's maximum, with a button to max them; all five CRC-32s are rewritten (checked on public speedrun saves, not yet tried in the game) |
| Fallout 3 | Bottle caps, experience, karma, S.P.E.C.I.A.L. and the points added to each skill (checked on real saves, not yet tried in the game) |
| Fallout: New Vegas | Bottle caps, experience, karma, S.P.E.C.I.A.L. and the points added to each skill (checked on real saves, not yet tried in the game) |
| Fallout, Fallout 2 | *Pip-Boy*: level, experience, skill points, hit points, radiation, poison, S.P.E.C.I.A.L., skills, the two traits, the tagged skills, every perk's rank, kill counts, bottle caps and the count of each item carried (with loaded rounds). The save's own layout is worked out each time, so mods that add items still open |
| Cave Story+ | *Quote*: every save slot's difficulty (Normal, Easy or Hard), health, max health and Whimsical Stars, and each weapon's type, level, energy and ammo; equipment, items and the stage are listed. The freeware Cave Story's `Profile.dat` opens too (checked on public saves, not yet tried in the game) |
| Celeste | *Save*: strawberries, deaths, jumps and the other totals, chapters unlocked, the assist options, and for every chapter the cassette and the A, B and C sides' completion, full clear and heart. Open `0.celeste` (checked on a public save, not yet tried in the game) |
| Deep Rock Galactic: Survivor | *Profile*: credits, the minerals and Power Cores, player rank, each class's XP and rank, and the stat upgrade levels. Open `drgs.dat` (checked on a public save, not yet tried in the game) |
| Plants vs. Zombies (Game of the Year Edition) | *Profile*: money (stored divided by 10), Adventure level and completions, Crazy Dave's shop (the nine plants, tools, cleaners), fertilizer, bug spray, chocolate and tree food, seed slots, Tree of Wisdom, Survival progress; the Zen Garden's plants; mini-game and puzzle trophies on the *Values* tab. Open `user1.dat` (checked on a public save, not yet tried in the game) |
| OMORI | *OMORI*: Clams, each friend's level, heart and juice in Headspace and Faraway Town, and every snack, toy, weapon and charm by name, in `file1.rpgsave`… (`www\save` in the game folder) (checked on public saves, not yet tried in the game) |
| ENDER LILIES: Quietus of the Knights | *Lily*: level, blight (experience), prayer level and prayers left, Stagnant Blight, Furious Blight and Ancient Souls with a **Max crafting materials** button that fills both blights to 999 (the most players report; Ancient Souls are left alone), and every spirit's level (checked on public saves, not yet tried in the game) |
| Ori and the Blind Forest | *Ori*: level, experience, ability points, life and energy cells, keystones, mapstones and deaths, and every skill and ability-tree upgrade to tick on or off, in Definitive Edition and original saves (`saveFile0.sav`…); the load menu's copy of life and energy is kept in step (checked on a public save, not yet tried in the game) |
| Phasmophobia | Money, level, prestige and experience, how many of each item you own, item tiers and their unlocks, and the journal's statistics (`SaveFile.txt`); decrypted and encrypted again on save (checked on public test saves, not yet tried in the game) |
| Lethal Company | Credits, the profit quota and what is paid towards it, quotas passed, the deadline, the current moon (by name), ship items and suits owned (add or remove), every piece of scrap's sell value, ship upgrades in storage and statistics (`LCSaveFile1`–`3`); experience and rank (`LCGeneralSaveData`); decrypted and encrypted again on save (checked on public saves, not yet tried in the game) |
| R.E.P.O. | Level, money, lives, the charging station and total haul; each player's health and upgrades; the items in the truck and bought in total (`<save>.es3`); decrypted and encrypted again on save (checked on a public save, not yet tried in the game) |
| The Sims (2000), The Sims Legacy Collection | One tab per family: funds (up to 9,999,999) and family friends, and for every Sim in the family the seven skills and six personality traits (0–1000, 100 per point on the game's bars) and job performance; townies and visitors are on the *Values* tab with Advanced on. Open `Neighborhood.iff`; Sims carry their file name (`user00012`) |
| The Sims 2 (incl. Legacy and Ultimate Collection) | One tab per family: funds, and for every Sim the seven skills and five personality traits (0–1000, 100 per point), job performance and aspiration score; townies and the default families are on the *Values* tab with Advanced on. Open `N001_Neighborhood.package`; edited parts are stored uncompressed when the package is saved (the way SimPE does). Sims carry their id, not their name |
| The Sims 4 | One tab per household: funds (up to 99,999,999) and its Sims with their names, ages and genders. Open `Slot_0000000N.save`; the game data is stored uncompressed when saved, so the file grows (checked on a public save, not yet tried in the game). Skills and traits aren't mapped yet |
| NieR:Automata | Money, experience (with the level it gives), item and material counts (99 each), weapon levels (1–4) and a *Max crafting materials* button for the Materials list; no checksum (checked on a real save, not yet tried in the game) |
| Starbound | Pixels and the other currencies; item counts in every bag, with a **Max crafting materials** button that fills ores, bars, fabric, leather, silk, plant fibre, wire, glass and other base-game materials to 1000, the usual stack size; health, energy and food; ship level, crew and fuel; deaths and play time (checked on a real character save, not yet tried in the game) |
| Minecraft (`level.dat`) | Health, food and XP; game mode; world settings; inventory and ender chest grids; a **Max crafting materials** button that fills the ingots, raw ores, gems, string, leather and other materials already carried to 64, leaving ender pearls, eyes of ender and other progression items alone |
| Balatro | Money, ante, hands and discards; this round; jokers and consumables; poker hand levels |
| Hollow Knight | Geo, masks, notches, abilities and charms; spell levels, nail arts, Steel Soul mode, Kingsoul state and stag stations (those this save lists); a *Max upgrade materials* button (all 6 Pale Ore) |
| Slay the Spire | Gold, HP, potion slots; the deck, relics and potions (change an id or upgrade level, add a card, relic or potion by its internal id, remove one); ascension level, floor, act, shop removal cost and the three Heart keys |
| Undertale | Name, LV, HP, attack, defence, EXP, gold, kills; the item, phone and equipment slots by name; the route record (kills per area, Genocide and Pacifist flags, boss outcomes, spared and fled counts, shop spending); room and play time |
| Valheim (characters, profile versions 33 to 37) | Health, stamina, skills, item stacks (gold is Coins), quality and durability; the SHA-512 is written again. A **Max crafting materials** button fills the wood, stone, ores, metals, hides and other plain materials you carry to their stack size, leaving boss drops, summoning items and key fragments alone. Newer characters open read-only. Not yet tried in the game |
| Hollow Knight: Silksong | Rosaries, shell shards, masks, silk, needle, abilities, crests, tools and items; decrypted and encrypted again on save. Not yet tried in the game |
| Deltarune | Chapter 1 fully (party, inventory as item numbers, Light World); Chapters 2 to 4: dark dollars and the party's stats and equipment. Every line of the file can be edited. Not yet tried in the game |
| Vampire Survivors | Coins, power-up ranks and what's unlocked; the checksum is worked out again on save. Not yet tried in the game |
| RPG Maker MV / MZ | *Party*: gold, steps and play time; each character's name, level, EXP, HP / MP / TP, stat bonuses and learned skills (by id); the party's items, weapons and armor by database id (change a count, remove one, add one) |
| Cairn (Story and On the Trail) | *Climber*: who climbs (Aava or Marco); play time, falls and deaths; the running buffs (Focus, Grip, Temperature, Grit, Burst and the rest: seconds left, source, strength; start, stretch or end them); the backpack, piton belt and chalk pouch as item grids with the game's own item names (add, remove or swap items, uses and sips left, what a bottle holds); the slot's saves, marking the one the game continues from |
| Cult of the Lamb | *Cult*: the Lamb's level, XP, hearts and fervour; cult faith, level, shrine and doctrine levels; the inventory and storage by item name, with a **Max crafting materials** button that fills lumber, stone, bones, grass, planks, stone blocks, gold nuggets and bars, crystal shards, spider silk and shells to 999 (an estimate), leaving coins, food, seeds, gifts, necklaces and boss items alone (checked on a public sample save, not yet tried in the game) |
| Fallout Shelter | *Vault*: caps, food, power, water, Stimpaks, RadAway, Lunchboxes, Mr. Handy, pet carriers and Nuka-Cola Quantum; every dweller's level, happiness, health, radiation and S.P.E.C.I.A.L. The save's encryption is removed and put back on save (checked on a public sample save, not yet tried in the game) |
| Deep Rock Galactic | *Dwarves*: credits, perk points, vanity mastery and forge XP; each dwarf's XP and retirements; the minerals and brewing ingredients, with a **Max crafting materials** button that fills Bismor, Magnite, Croppa, Umanite, Jadiz, Enor, Phazyonite and the brewing ingredients to 9999 (an estimate), leaving credits, Data Cells, Error Cubes and Blank Cores alone (checked on a public sample save, not yet tried in the game) |
| Schedule I | One screen per save file: *Money* (bank balance, net worth, lifetime earnings, weekly deposits), *Rank* (rank, tier, XP), *Time* (days, time of day, play time) and *Game* (organisation name, seed, console). Cash you carry is in the player's own files and isn't covered (checked on a public sample save, not yet tried in the game) |
| Sons of the Forest | *Game* (GameStateSaveData.json): the day and time. *Weather* (WeatherSystemSaveData.json): the season. *Player* (PlayerStateSaveData.json): health, stamina, hydration, rest, strength and its level. *Inventory* (PlayerInventorySaveData.json): the items carried, by name, with a **Max crafting materials** button that fills logs, sticks, rocks, bones, cloth, circuit boards, wire, duct tape, rope, feathers, leaves, small rocks, tech mesh, Solafite ore and printer resin to the game's own carrying limit for each, leaving weapons, ammo, food, blueprints and story items alone. The JSON the game keeps inside the file's strings is opened and written back as strings (checked on public sample saves, not yet tried in the game) |
| Death Must Die | *Profile*: gold, runs, wins, current and best streak, time in runs and in the game, selected character. The JSON the game keeps inside the save's strings is opened and written back as strings (checked on a public sample save, not yet tried in the game) |
| Cuphead | *Inkwell Isle*: coins and deaths for both players, Djimmi's wishes, expert mode, the filters, Boatman and the other unlocks (checked on a public sample save, not yet tried in the game) |
| Antimatter Dimensions | *Dimensions*: antimatter, infinity points, eternity points, reality machines, the counters behind them, replicanti, dilation and perk points; opens the exported save text from Options > Saving (checked on public sample saves, not yet tried in the game) |
| Diablo II, Diablo II: Resurrected | *Character*: level, experience, gold and stash gold; the four attributes, stat and skill points; life, mana and stamina (base values); points in every skill; *Waypoints*: every waypoint in Normal, Nightmare and Hell, with an open-all button |
| Grim Dawn | *Character*: money, level, experience, unspent attribute, skill and devotion points, attributes, skill ranks, item stack sizes, and *Factions*: each faction's reputation (up to the 25000 cap) and whether it is unlocked. The save is decrypted, then encrypted again with fresh checksums. Not yet tried in the game |
| Titan Quest Anniversary Edition | *Character*: level, experience, money, unspent points, the five attributes and skill levels (stash files aren't read). Not yet tried in the game |
| Torchlight II | *Character*: level, experience, fame, gold, unspent points, the four stats, health, mana and item stacks; scrambled again with its checksum (the older 0x40 format isn't supported). Not yet tried in the game |
| Dragon Age: Origins, Awakening | Party money; the Warden's and followers' level, experience, attributes and unspent points, approval, and backpack stack sizes. Tried on saves from one playthrough; Awakening saves not yet tried |
| Divinity: Original Sin 2 Definitive Edition | Each player character's experience, vitality, attributes, unspent points and gold; the package is packed again with its checksum on save. Not yet tried in the game |
| Icewind Dale: Enhanced Edition | Party gold and reputation; each party member's experience, hit points, levels, abilities, alignment and item stacks or charges (`BALDUR.gam`, the Baldur's Gate: EE layout); every global variable by name (quest stages, who has joined). No Icewind Dale save has been tried yet |
| Deus Ex (2000) | JC Denton: credits, skill points, bioelectric energy, health per body part, ammo, medkits and other item counts, skill and augmentation levels (values the game left out can't be set). Not yet tried in the game |
| Octopath Traveler, Octopath Traveler II | Money (leaves) and the eight travellers' level, EXP, JP and current HP/SP, changed in place. Read from a sample save; not yet tried in the game |
| Pathfinder: Kingmaker, Wrath of the Righteous | *Party*: money; the player and each companion's ability scores, experience, classes and levels, and base hit points (Wrath: mythic rank too) |
| Warhammer 40,000: Rogue Trader | *Dynasty*: scrap, Profit Factor, the Navigator's insight, difficulty settings and reputation with each faction (same save family as Pathfinder; no Rogue Trader save tried yet) |
| Onimusha: Way of the Sword | Each save slot (place, play time, difficulty, objective); red souls, money, health and Oni energy; items and charms by name (on hand and in storage; charm level), with **Add material** to give a slot an upgrade material it doesn't have yet, and a **Max crafting materials** button that fills the upgrade materials on hand (cloths, leather, whetstones, iron, Tamahagane, Malicite, Byakue's Talisman, Dohatsu-ten's Horn) to 999, adding any that are missing, and leaving Power Stones, Oni Stones, Oni Cloth and key items alone; equipment experience. Decrypts Capcom's RE Engine encryption with your Steam account, found automatically |
| Stellaris | *Empire*: energy, minerals, food, alloys, consumer goods, influence and unity up front, and every resource in the stockpile. Ironman saves are text like any other Stellaris save and open the same way (checked on saves with the ironman flag set; no save from a real ironman campaign has been tried yet); the game may stop awarding achievements for an edited campaign |
| Hearts of Iron IV | *Empire*: political power, stability, war support, command power and experience. Binary saves (the game's default, and ironman) are edited in place and stay binary; fields outside this screen show as numbers (checked on public saves, not yet tried in the game) |
| Europa Universalis IV | *Empire*: treasury, manpower, prestige, stability, monarch power and every country value. Ironman (binary) saves are edited in place and stay ironman; fields outside this screen show as numbers (checked on public saves, not yet tried in the game) |
| Crusader Kings II | *Empire*: the ruler's wealth, prestige, piety and attributes. Ironman (binary) saves are edited in place, dates included (checked on a save converted to the binary layout; no save from a real ironman campaign has been tried yet) |
| RimWorld | *Colony*: every colonist's skills (level and passion, modded passions included) and every silver stack, plus steel, plasteel, components, gold, uranium, wood, cloth, medicine, chemfuel and the other stockpiled resources stack by stack, edited in place even in 100+ MB saves |
| Oxygen Not Included | *Colony*: every duplicant's attribute levels and experience, total experience and learned skills; a *Map* of every asteroid and rocket interior (elements, temperature and germ views; paint elements with mass and temperature, dig out to vacuum, set temperature or mass, clear germs, reveal the fog; geysers, teleporters, Gravitas finds, story traits, artifacts and the Printing Pod as toggleable markers); *Worlds & rockets*: names, discovered, sunlight and cosmic radiation, and what's on the star map |
| Subnautica, Below Zero | *Slot*: game mode (Survival / Freedom / Hardcore / Creative), play time, vehicles |
| Mass Effect (2008) and Legendary Edition ME1 | *Shepard*: level, experience, talent points and power ranks; credits, medi-gel, omni-gel and grenades; each squad member's level, talent points and talents. Legendary Edition saves also show origin, reputation, difficulty and the inventory, and a *Story* panel of about 100 named story flags and numbers to change (who is in the party, Wrex, the Council, romances, mission outcomes) |
| Mass Effect 2, Mass Effect 3 (original and Legendary Edition) | *Shepard*: level, experience, talent points, difficulty; credits, medi-gel, element zero, iridium, palladium, platinum, probes, fuel, grenades; power ranks; spare ammo; each squad member's level, talent points and powers; Paragon and Renegade points (Mass Effect 3: reputation too) and a *Story* panel of named story flags to switch on or off (who is loyal, alive or romanced, the choices imported from the earlier games, research upgrades; over 500 in Mass Effect 2 and about 400 in Mass Effect 3), checked on public sample saves. The save's checksum is updated |
| Mass Effect: Andromeda | Credits, Ryder's level and skill points, each squad member's level and skill points; research data (Milky Way, Heleus, Remnant), minerals, salvage, biological and faction materials, augmentations and consumables in the inventory, with a **Max crafting materials** button that fills the minerals, biological and faction materials to 999, the game's resource limit; location, completion and play time. Both checksums are written again on save. Not yet tried in the game |
| XCOM: Enemy Unknown / Within | *HQ*: money; every soldier's rank, XP, kills, HP, aim, mobility and will (the fallen listed last); storage counts. Every saved object's numbers are in *Values* |
| Final Fantasy VII (PC / 2013 Steam) | *Party*: each slot's gil and play time; every character's level, experience, kills, limit level and gauge, HP / MP with the base maximum the game builds them from, and the six stats. Slot checksums are updated |
| Final Fantasy VII Remake (Intergrade) | Gil, and adding or removing items, armor, accessories and music discs or changing their amounts (key items are shown but left alone, and worn armor stays); materia to add, or to remove when nobody has it equipped. On the *Party* panel each character's level, HP / max HP, MP / max MP and experience |
| Final Fantasy VII Rebirth | Gil, Moogle Medals, Gold Saucer Points and the inventory: add or remove items, crafting materials, chocobo gear and Queen's Blood cards, and change amounts of those and of armor and accessories, with a **Max crafting materials** button (99 each); materia to add or remove, with each one's level and AP. On the *Party* panel each character's level, HP / max HP, MP / max MP and experience |
| Green Hell | *Survivor*: health, energy, hydration, the three nutrients, dirtiness, sanity, every skill and the statistics |
| Resident Evil Requiem (and RE Engine games with the same layout) | *Slot*: health of the player and other characters, inventory counts (with the ammo or contents they hold), save count |
| Hogwarts Legacy | *Character*: gold, experience, talent points, every resource in the bag (with a **Max crafting materials** button that fills ingredients, beast by-products and Moonstone to 999, an estimate) and talent levels, from the game's own database inside the save (unpacked and packed again when you save); if the database can't be opened, the load-screen name, house and level |
| The Witcher 3 | *Geralt*: crowns, level, unspent skill points and experience (repacked with LZ4 on save) |
| Cyberpunk 2077 | *V*: eurodollars, from V's own inventory (found through the save's section table; the table's position is updated when the save is repacked) |
| Dying Light 2 / The Beast | *Survivor*: money, health, the Traversal / Combat / Inhibitor levels and points, every skill's level (Techland's self-describing save format; every saved number is in *Values*) |
| Dying Light | *Survivor*: the skill tree levels (every copy the save keeps) |
| Kerbal Space Program | *Space Center*: funds, science, reputation, the level of each Space Center facility, and a table of every Kerbal (courage, stupidity, veteran, extra XP, trait, state). The `persistent.sfs` text is read as a tree and only the values you change are rewritten (checked on real career saves, not yet tried in the game) |
| Space Haven | *Fleet*: credits, the day and time, the difficulty options, and a table of each ship's crew with health, food, rest, comfort, mood, the four attributes and every skill level. The save's XML is edited in place, so everything you don't change stays as it was. Resources in storage are not covered (checked on a real save, not yet tried in the game) |
| SnowRunner | *Garage*: money, rank, experience, the statistics (money earned and spent, trucks and add-ons bought and sold, sessions), the difficulty factors and the trucks in the warehouse. The save is JSON followed by a NUL byte, which is kept (checked on two real saves, not yet tried in the game) |
| Grand Theft Auto III | *Player*: money (both copies the game keeps), hidden packages, and the statistics (progress, missions, stunt jumps, days, times arrested and died, kills); the save's checksum is rewritten (checked on public saves, not yet tried in the game) |
| Grand Theft Auto: Vice City | *Player*: money (both copies the game keeps), hidden packages, and the statistics (progress, stunt jumps, days, times arrested and died, kills); the save's checksum is rewritten (checked on public saves, not yet tried in the game) |
| Bitburner | *Player*: money, BitNode, karma, HP, the skill levels and the experience behind them (an exported save; the JSON nested inside it is opened as one; checked on a real save, not yet tried in the game) |
| Terra Invicta | *Factions*: every faction's money, influence, operations, research, boost, mission control and space resources (checked on public saves, not yet tried in the game) |
| Prison Architect | *Prison*: the bank balance, play time and the main prison settings, changed in place in the large text save (checked on public saves, not yet tried in the game) |
| Final Fantasy XIII | *Party*: gil (the save is decrypted to edit and encrypted again, with its checksum rewritten; checked on public saves, not yet tried in the game) |
| Sonic Mania | *Slots*: all eight Mania Mode and three Encore Mode slots (state, character, zone, lives, score, Chaos Emeralds, continues, next special stage), changed in place (checked on public saves, not yet tried in the game) |
| Lies of P | *Pinocchio*: Ergo, level, humanity, the six stats and every item's count, with a **Max crafting materials** button that raises the Moonstones for normal and special weapons and Legion Calibers you have to 99 (an estimate), leaving quest items and consumables alone (checked on a public save, not yet tried in the game) |
| Tokyo Xtreme Racer (2025) | *Driver*: CP (money) and its limit, PP, garage size, the day, and the battle records (checked on a public save, not yet tried in the game) |
| For The King | *Party*: each hero's gold, level, XP, health and focus, and the party's life pool; the save's gzip and UTF-16 wrapping is handled for you (checked on a public save, not yet tried in the game) |
| Dave the Diver | *Dave*: gold, Bei, chef flames, research and trust points, and every sushi ingredient's stock, with a **Max crafting materials** button that fills the ingredients you have to 999; the save's encryption is removed and put back for you (checked on public saves, not yet tried in the game) |
| My Summer Car | *Finland*: money and the needs (hunger, thirst, fatigue, stress, urine, dirtiness); every other single value of `defaultES2File.txt` (Easy Save 2) is in the Tree by name (checked on a public save, not yet tried in the game) |
| ULTRAKILL | *V1*: points (P) and every weapon variant and arm, owned or not, in `generalprogress.bepis`; each level file's ranks and secrets open too (checked on public saves, not yet tried in the game) |
| RPG Maker XP games | *Party*: gold, steps, the party and every actor (level, EXP, HP, SP and stat bonuses), and the items, weapons and armour you carry by database number (checked on public To the Moon saves) |
| Divinity: Original Sin Enhanced Edition | *Party*: each party member's experience, vitality and gold, on the Original Sin 2 screen; the `.lsv` package is unpacked and packed again (checked on a public save, not yet tried in the game) |
| Resident Evil 5 | *BSAA*: money and exchange points (Steam `savedata.bin`); the save is decrypted and its checksum made again for you (checked on a public save, not yet tried in the game) |
| Anno 1800 | *Company*: your coins (the load screen's total is changed with them) and the rest of your company's storage by GUID; the `.a7s` archive and its 100+ MB game data are unpacked and packed again for you (checked on a public save, not yet tried in the game) |
| Tales of Symphonia | *Party*: gald (written in both places the game keeps it), grade points, the records, all nine characters (level, experience, HP, TP, strength, attack, defense, luck, accuracy, evasion, intelligence, overlimit, affection) and every item up to 20 (checked on a real save, not yet tried in the game) |
| Kenshi | *Squad*: every member's attributes and skills (up to 100, with a button to max the whole squad), the health of each body part, blood, hunger, bleeding and lost or robotic limbs, and reviving the dead, unconscious or those in a coma. Open a squad's file from the save's `platoon` folder (yours is named after your faction, `Nameless_0.platoon` by default); `quick.save` opens on a *Faction* screen with the cats. Values are found by name in the file and changed in place (checked on public saves, not yet tried in the game) |
| Kerbal Space Program | *Space program*: career funds, science and reputation, from the save's ConfigNode text (only the values you change are rewritten; every other value, vessel and Kerbal is in the Tree). Checked on public sample saves, not yet tried in the game |
| Palworld | *Player*: technology points, ancient technology points, Lifmunk effigies and the record counters |
| Final Fantasy Pixel Remaster (I–VI, PC) | *Party*: one save slot, decrypted (PBKDF2 key, Rijndael with a 256-bit block, deflate) with the character and inventory data that the game keeps as text inside it opened too, and encrypted again on save; gil, total gil, steps, battle and chest counters, every character's experience, HP / MP and bonus stats, and the item and key-item counts. Checked on public sample saves of FF I–IV against the reference tool's own decryption; not yet tried in the game |
| Minecraft Dungeons | *Hero*: the character `.dat` is decrypted (AES, the key as published by MCDSaveEdit) and encrypted again on save; experience, total gear power and the emerald, gold and Eye of Ender counts, with items and enchantments in the Tree. The decryption was checked on a public sample save; not yet tried in the game |
| Stoneshard | *Character*: level, experience, ability points, unspent attribute points and the six attributes of a character's `data.sav`, which is unpacked (zlib) and packed again with its MD5 (the salt comes from the save's folder names, so open it from its own folder). Built from the published layout; not yet tried on a real save |
| Graveyard Keeper | *Keeper*: money (in silver; 100 = 1 gold) and health, with every other value of the save (technology points, inventory, world objects) in the Tree under the names the game uses. The typed value tree of the `.dat` is read and written again, keeping each value's type. Built from the published layout; not yet tried on a real save |
| Need for Speed: Most Wanted (2005) | *Driver*: money and pursuit bounty of a profile file (63,596 bytes on PC, 62,689 on PlayStation 2), with the file's MD5 made again on save. Opens by its layout, wherever the file is kept; built from the published offsets, not yet tried on a real profile |
| Final Fantasy IX (Steam) | *Party*: every saved game in `SavedData_ww.dat` (decrypted and encrypted again): gil, and each character's level, experience, HP / MP and base stats |
| Death Stranding 2: On the Beach | *Facilities*: the material stocks (chiral crystals, resins, metals, ceramics, chemicals, special alloys) at every facility you've found, with its connection level, and a **Max crafting materials** button that fills them to the most that kind of facility stores at five stars (preppers, labs and observatories, distribution centers, knot cities); likes from NPCs and other porters and likes given; day, time of day and play time. Unpacked (XOR, LZ4) and packed again with the save's MD5 on save; pieces that didn't change keep their bytes. Not yet tried in the game |
| Final Fantasy XV (Windows Edition) | *Party*: gil, ability points and medals; every character's level, HP / MP and stats, the experience waiting to be banked at the next rest, total experience and kills; item amounts in every bag (by the game's item number), with a **Max crafting materials** button that fills every treasure to 99. Decrypted (AES-192, shuffled blocks) and encrypted again on save. Not yet tried in the game |
| Final Fantasy X (HD Remaster, PC) | *Party*: gil, and every character's and aeon's HP, MP, maxima and stats (with and without equipment), AP, Sphere levels, overdrive gauge and mode. *Abilities*: every skill, special, white and black magic per character and aeon, with a learn-all button. *Overdrives*: each character's overdrives and overdrive modes. *Items*: all 112 item slots (item and count, with a set-all-to-99 button) and the key items. *Monster Arena*: how many of each fiend is captured. The checksum (the game's own CRC-16) is updated |
| Final Fantasy X-2 (HD Remaster, PC) | *Gullwings*: gil; Yuna, Rikku and Paine's level, experience, HP / MP and stats. Checksum updated |
| Persona 3 Reload | *Protagonist*: yen, every party member's level, experience, HP and SP, the social stats and each Social Link rank; Steam saves are decrypted and re-encrypted for you (checked on public sample saves, not yet tried in the game) |
| Persona 3 Portable (PC) | *Protagonist*: yen, level, experience and the three social stats; every Persona slot in use with its level, experience and five stats; the count of every item that can be held (key items left out, the game needs a flag for them); the matching `.BINslot` checksum is updated. Checked on a synthetic save, not yet tried in the game |
| Trails to Azure / Ao no Kiseki KAI | *Party*: Mira, DP, medals, difficulty, play time; the seven Sepith; every character's level, HP, EP, CP, experience and stats; the bonding values. NISA `savedata.dat` (zstd) and KAI `NNN_save.bin` (AES) are both opened and written back with their checksums. Checked on synthetic saves, not yet tried in the game |
| Persona 4 Golden (PC) | *Protagonist*: yen, level, experience and the five social stats; the matching `.binslot` checksum is updated |
| Kingdom Hearts III (PC) | *Sora*: decrypted (the key comes from the Steam id in the save's folder, or Epic's) and encrypted again with its CRC and MD5; munny (with the earned and spent totals kept in step), level, EXP, the bonus stats, Sora, Donald and Goofy's HP and MP, every synthesis material with a *Max crafting materials* button, and the items already held. Checked against the reference editor's test vectors; not yet tried on a real save |
| Rune Factory 4 Special (PC, extracted Switch) | *Farmer*: money, HP and RP, strength, intelligence and vitality, order points, tourists, and lumber, material stone and fertilizer with a *Max crafting materials* button; the save's CRC is written again. Not yet tried on a real save |
| Trails in the Sky 1st Chapter (2025) | *Bracers*: mira, the eight sepith with a *Max crafting materials* button, and each character's experience, HP and EP; unpacked (zstd) and packed again with the save's checksum. Not yet tried on a real save |
| BioShock Infinite | *Booker*: Silver Eagles (up to 99,999), salts, lockpicks and the Health, Shield and Salts infusion levels (0 to 10; a kind never picked has no entry yet); unpacked and packed again with the save's MD5. Not yet tried in the game |
| Persona 5 Royal (PC) | *Phantom Thieves*: decrypted, unpacked and packed again; money, HP / SP, levels, experience and Persona stats for the party and Joker's Personas |
| Metro 2033 / Last Light Redux, Exodus | *Artyom*: military-grade rounds, every ammo type and (Exodus) crafting metal and chemicals in the level saves (`.player`), with a **Max crafting materials** button that fills metal and chemicals to 300 (the Reader and Ranger Hardcore limit; Normal allows more); auto and quick saves are unpacked (LZ4) for Value search. CRC-32 updated |
| Yakuza 0 | *Kiryu & Majima*: both characters' money (up to ¥10 trillion) and CP, the uncollected real estate and cabaret takings, and the difficulty |
| God of War (2018) | *Kratos*: every save in `game.sav` (newest first, by title and date): XP, hacksilver, every resource and Yggdrasil's dew (a **Max crafting materials** button fills the crafting resources to 999, an estimate, leaving the story-locked Frozen and Chaos Flames alone), the equipment each save owns, and armour, pommels and talismans it doesn't have yet to add (at their base level) |
| God of War Ragnarök | *Kratos*: XP for Kratos, Atreus and Freya, hacksilver and divine ashes, resources (a **Max crafting materials** button fills the crafting resources to 999, an estimate), Yggdrasil's dew, health and rage, the level of every weapon, armour piece, attachment, shield and relic, and new armour, shields and attachments to add at any level (campaign saves; a piece of the same type you own is the model) |
| Yakuza Kiwami | *Kiryu*: money and the experience he hasn't spent yet (save and autosave files) |
| Yakuza Kiwami 2, Like a Dragon (Dragon Engine) | *Player*: decrypted (XOR) and its checksum updated (Adler-32 or CRC-32); money, HP, Kiwami 2's experience pools and Like a Dragon's whole party (level, experience, HP, MP), with every other value in the tree |
| Yakuza 6: The Song of Life | Money, HP and Kiryu's five experience pools; decrypted (key from rgg_save_tool_py) with its checksum kept right, every other value in the tree (checked on real saves, not yet tried in the game) |
| Yakuza Kiwami 3, Judgment, Lost Judgment, Like a Dragon Gaiden, Infinite Wealth, Ishin!, Pirate Yakuza (Dragon Engine) | Decrypted with each game's key and checksum kept right (keys from rgg_save_tool_py); values in the tree, and the Yakuza screen where the save has the same layout. Not tried on sample saves of these games yet |
| Yakuza 3, 4, 5 Remastered | Hex view and Value search |
| Half-Life / Opposing Force / Blue Shift | *Player*: health, armour and flashlight; spare ammo by type; each carried weapon's magazine, with the weapon in hand marked. Read from the current map's state and changed in place |
| Half-Life 2 / Episode One / Episode Two | *Player*: health, armour and suit power; spare ammo by type (pulse rounds, SMG grenades, energy balls and the rest); each carried weapon's two magazines, with the weapon in hand marked. Changed in place |
| Half-Life: Source | *Player*: health, armour, spare ammo by type and each carried weapon's magazine. Changed in place |
| Black Mesa | *Player*: health and armour; spare ammo by type (9mm, .357, buckshot, uranium, hornets and the rest); each carried weapon's magazine, with the weapon in hand marked. Changed in place |
| Half-Life: Alyx | *Player*: health and resin (a **Max crafting materials** button fills resin to 999, an estimate); pistol, shotgun and rapidfire ammo in the backpack; the weapons Alyx has. Changed in place |
| XCOM 2 | *HQ*: supplies, intel, alloys, elerium and the other HQ resources; every soldier's rank, kills, missions, ability points and stats (HP, aim, mobility, will, hacking, defense, dodge, psi, armour, shield, sight); storage counts by the game's item name. Unpacked and packed again on save. Not yet tried in the game |
| Borderlands 2 | *Vault Hunter*: level, experience, skill points; money, Eridium, Seraph crystals and Torgue tokens; ammo and ammo SDUs, with buttons to fill ammo and max the SDUs; backpack and bank SDUs (with the room they give), weapon slots and playthroughs completed, which opens True and Ultimate Vault Hunter Mode; gear is listed (level, slot), not edited. Packing and checksums are redone on save. Edited saves not yet tried in the game |
| Borderlands: The Pre-Sequel | *Vault Hunter*: level, experience, skill points; money and Moonstone; ammo and ammo SDUs; backpack and bank SDUs, weapon slots and playthroughs completed; gear is listed, not edited. Packing and checksums are redone on save. Edited saves not yet tried in the game |
| Tiny Tina's Wonderlands | *Hero*: level (to 40) and experience, skill points; gold, moon orbs and ammo; gear is listed (slot, favourite, junk), not edited. Encrypted again on save. |
| Euro Truck Simulator 2 | *Driver*: money, loan limit, experience, the five skills and ADR classes; every vehicle's fuel, odometer and wear, with buttons to max skills, fill every tank and repair everything. Packed saves are unpacked and packed again with their checksum. |
| American Truck Simulator | The same screen as Euro Truck Simulator 2: money, loan limit, experience, skills, and each vehicle's fuel, odometer and wear. |
| X4: Foundations | The player's credits, written to all three places the save keeps them (info line, player faction account, statistics); the rest of the XML tree is open for editing. Opens `.xml.gz` saves directly and packs them again. |
| Metal Gear Solid V: The Phantom Pain | GMP (and the FOB wallet), heroism and demon points; every Mother Base staff member (rank, skill, section, morale, proficiency, stat bonus) with buttons to promote everyone to S++ and max morale; resource stockpiles with a Max crafting materials button (materials to 1,000,000 and plants to 30,000 online, the game's caps); a button to open every Mother Base section; and the Mother Base and lifetime-statistics counters; the Mother Base and FOB platform grades with a finish-everything button. Every other variable is in the Tree under the game's own variable names; encryption and checksum are redone on save. |
| Total War: Warhammer II, Total War Saga: Troy | Every faction's treasury, yours first, with a button to fill it. The campaign inside the save is unpacked, edited with every size above it recomputed, and packed again (the file grows). |
| Wasteland 3 | *Rangers*: money; each ranger's level, XP, hit points, attributes, unspent points and skills by name (Automatic Weapons, First Aid, Lockpicking and the rest); item quantities in the inventory and stash; Fame and each faction's reputation (negative allowed) and the quest counters the game keeps. Unpacked (LZF) and packed again on save. Not yet tried in the game |
| Golden Sun | *Party*: every file in the flash with its coins; each party member's level, experience, HP / PP, base stats and items; the Djinn grid, where each Djinni can be given to a character (set or on standby) or taken away, with the game's own counts kept in step (checked on public sample saves). Checksums kept right. Not yet tried in the game |
| Golden Sun: The Lost Age | *Party*: every file in the flash with its coins; each party member's level, experience, HP / PP, base stats and items; all 72 Djinn, each to give to a character (set or on standby) or take away (checked on public sample saves). Checksums kept right. Not yet tried in the game |
| Star Wars: Knights of the Old Republic | *Party*: the player and party members (attributes, skills, HP and Force points, XP) and how many of each item the party carries, from SAVEGAME.sav; credits and party XP from PARTYTABLE.res. Not yet tried in the game |
| Star Wars: Knights of the Old Republic II | *Party*: the player and party members (attributes, skills, HP and Force points, XP) and how many of each item the party carries, from SAVEGAME.sav; credits, party XP, influence, components and chemicals from PARTYTABLE.res, with a **Max crafting materials** button that fills components and chemicals to 9999 (an estimate). Not yet tried in the game |
| Neverwinter Nights: Enhanced Edition | *Character*: gold, experience, ability scores, hit points, alignment, skills and item stack sizes of the player (the module .sav, with player.bic kept in step) or a vault character (.bic). Not yet tried in the game |
| Darkest Dungeon | *Hamlet*: the estate (gold, busts, portraits, deeds, crests, blueprints, shards), how many of each trinket and estate item it holds, and the roster (each hero's resolve XP, stress, HP, weapon and armour rank, skill upgrade levels), changed in place. Open persist.estate.json or persist.roster.json |
| Ren'Py visual novels | *Story*: the game's numbers, flags and text variables with a filter, money and the like up front |
| Mutant Year Zero: Road to Eden | *Squad*: scrap, weapon parts and medpacks, the Ark's artifact points, population and levels, the squad level, and each squad member's level, XP, health, sanity and mutation points, with a **Max crafting materials** button that raises scrap and weapon parts to 999 (an estimate), leaving artifacts and gear alone (checked on a public save, not yet tried in the game) |
| Car Mechanic Simulator 2021 | *Mechanic*: money, level, XP and scrap, the profile's name, play time and difficulty, the unlocked parking levels, barns and warehouses (checked on a public save, not yet tried in the game) |
| Medieval Dynasty | *Settler*: health, stamina, food and water, reputation, tax, the four technology point pools, the six skill levels and every stack in the bag (money is the Coin item), with a **Max crafting materials** button that raises the plain building and crafting materials to 999 (an estimate), leaving money, tools, food and clothing alone (checked on a public save, not yet tried in the game) |
| Warhammer 40,000: Mechanicus | *Cadmus*: Blackstone, the Ark's bonus points and slots, and each Techpriest's level, XP, skill points, slot points and augment capacity; the save's own text dialect is patched in place and the checksum in `SaveState.sav` is rewritten for you (checked on public saves, not yet tried in the game) |
| Mortal Shell | *Fallgrim*: Tar, the shell points of each shell, max health, deaths, New Game+ level, and the Storm Mode run's Tar and Glimpses in `GPData_<slot>.sav` (inventory item names are stored as game text and aren't shown; checked on public saves, not yet tried in the game) |
| Mortal Shell II | *Harbinger*: Gold, Gloom and Glimpses, their lifetime totals, every item's count and every unlocked shell, weapon and seal in `WorldState_<n>.sav`, with a **Max crafting materials** button that raises the Tarforge smithing materials (Ventrium, Laterite, Dorsalite, Thoracium) you have to 99 (an estimate), leaving keys, artifacts and upgrade items alone (checked on public saves, not yet tried in the game) |

| theHunter: Call of the Wild | *Hunter*: cash, level, XP and the skill and perk points of the profile (`thp_player_profile_adf`, an Avalanche ADF file read from its own type definitions and edited in place); no crafting materials (checked on public saves, not yet tried in the game) |
| BeamNG.drive | *Career*: career money and BeamXP from `career/playerAttributes.json` of a save slot; the other career values (branch levels, vehicles) open by name in the Tree. Written from the game's own Lua source, not yet checked on a real save |
| The Forest | *Survivor*: every item the player carries (named from the game's own ids) with its amount and the limit the game stores for it, with a **Max crafting materials** button that raises rocks, sticks, skulls, rope, bones, skins, rabbit fur and circuit boards to those stored limits; the Base64 and UnitySerializer wrapping is untouched (checked on public single-player and multiplayer saves, not yet tried in the game) |

## Credits for God of War

The God of War formats and item names come from the Forja save editors by wilypizza-svg (José Murilo), which credit kg971 and xcier.

## Paradox ironman and binary saves

Ironman saves of Europa Universalis IV, Crusader Kings II and III, Hearts of Iron IV, Victoria 3,
Imperator: Rome and Europa Universalis V are binary, and so are the ordinary saves that Hearts of
Iron IV, Crusader Kings III, Victoria 3 and Europa Universalis V write by default. The editor opens them and changes values in place: the
number you edit is overwritten where it sits and nothing else in the file moves, so the save stays
binary and stays an ironman save. It isn't converted to a normal one.

- **Achievements.** These games check their ironman saves, and an edited one will most likely stop
  earning achievements for that campaign. The editor doesn't try to get around that.
- **Field names.** A binary save stores each field name as a number, and Paradox has asked that the
  tables of those numbers not be shared, so the editor doesn't include them. It knows only the few
  fields its *Empire* screens show for Europa Universalis IV, Hearts of Iron IV and Crusader Kings II;
  other fields appear as their number (`0x2d82`). Country tags, names and other text are stored as
  text, so the *Values* tab stays readable: find your country by its tag and its numbers are listed
  under it.
- **Europa Universalis V** keeps nearly all its names in the save itself, so its saves read in full.
- **Your own token file.** If you have a token file for a game, put it in a `tokens` folder next to
  the program (or in the editor's settings folder) as `eu4.txt`, `hoi4.txt`, `ck2.txt`, `ck3.txt`,
  `vic3.txt`, `imperator.txt` or `eu5.txt`, and every name it lists is shown. Each line is an id and
  a name (`0x284d date`); the semicolon-separated tables some older tools use are read as well. The
  variables `EU4_IRONMAN_TOKENS`, `HOI4_IRONMAN_TOKENS` and so on can point to a file instead.
- **Text instead.** Hearts of Iron IV writes text saves when `save_as_binary=no` is set in its
  `settings.txt`, and the newer games write text saves in debug mode. Text saves open with every name.

## Select game

*Select game* (toolbar and File menu) lists every game the editor knows — nearly 190, Mass Effect 1–3, the Legendary Edition and Andromeda among them — with where it
usually keeps its saves and what the files are called, and finds them on your computer (Steam
libraries and `userdata`, Documents wherever Windows has it, AppData, Saved Games, Ubisoft
Connect). On Linux and the Steam Deck it also looks inside Proton prefixes
(`steamapps/compatdata/<app id>/pfx` in every Steam library, including Flatpak Steam and SD cards)
and the usual Wine, Heroic, Lutris and Bottles prefixes, so Windows games run through Steam Play
show up too.

**Flash and AIR games**: `.sol` shared objects (AMF0 and AMF3) open with every value by name and
are written back with each value's own type; anything the editor doesn't show (dates, byte arrays,
class names, references) is kept as it was. *Flash and AIR games (.sol saves)* lists the Flash
Player folders by site. Read and written with Ruffle's parser; checked on 66 public sample files,
each written back byte for byte (the few the parser can't write back exactly open in the hex view).

**OpenFL and HaxeFlixel games**: their `.sol` files are Haxe serialization text, not Flash's AMF.
They open with every value by name (objects, lists, maps, enums) and are written back with only the
edited numbers, flags and text changed; dates, bytes and shared references are kept as they were.
*OpenFL and HaxeFlixel games (Haxe saves)* lists the `%APPDATA%\<company>` folders, which covers
Friday Night Funkin' and its engines and mods. Checked on a public Friday Night Funkin' save, written
back byte for byte.

**Koei Tecmo games**: Nioh, Nioh 2 and Nioh 3, Wo Long: Fallen Dynasty, Fatal Frame II: Crimson
Butterfly Remake and Stranger of Paradise: Final Fantasy Origin decrypt on open and encrypt again
on save with the file's own keys, so an unchanged save writes back byte for byte. Wo Long and Fatal
Frame II keep JSON inside, which opens with every value by name, and both of their checksums are
made again. Ported from KatanaSaveDataResigner (MIT); checked on its test files, not yet in the games.

**Dead Cells**: the save's SHA-1 and zlib packing are handled, so the unpacked data can be changed
in the hex view or found with the value search and saved back with a fresh checksum.

**Console memory cards**: a PlayStation memory card image (`.mcr`, `.mcd`, `.mem`, `.vgs`,
`.gme`, or a RetroArch `.srm` from Beetle PSX) opens one save at a time, listed by product code and
title under *Tools › Open another file in this archive*, and each save is written back into its own
blocks. A PlayStation 2 card from PCSX2 (`.ps2` with error-correction codes, or `.bin` without)
lists every file in every save folder the same way; saving writes the card's codes again. DeSmuME
`.dsv` saves open without their footer, which is put back on save. The editor doesn't know each
game's own checksums inside these saves, so find values with the value search and test the edit in
the emulator. Checked on a public PlayStation card and on PS2 cards made by mymc.

**Single PlayStation 1 and 2 saves**: the files cheat devices and PC tools export open the same
way, one file of the save folder at a time: `.mcs` and `.psx` (PlayStation), `.psu` (uLaunchELF),
`.max` (Action Replay Max, LZARI-packed), `.cbs` (CodeBreaker, encrypted and zlib-packed),
`.xps`/`.sps` (Xploder and SharkPort) and PS3 `.psv` (PlayStation and PlayStation 2 saves for the
PS3, signed again on save). Each is packed back in its own format. Checked on real `.max` and
`.psu` saves, whose Action Replay packing is reproduced byte for byte, and on `.psv` files signed by
MemcardRex and Apollo, whose signatures come out the same.

**Unity PlayerPrefs**: many Unity games keep coins, unlocks and settings in PlayerPrefs instead of
a save file. *Unity games (PlayerPrefs)* lists them: on Windows each Unity game's registry key
(`HKCU\Software\<company>\<game>`) is exported to a `.reg` file that opens like a save, with every
pref by name and its type kept (whole number, decimal, text), and saving imports it back into the
registry. Close the game first: it writes its prefs back when it quits. On Linux the `prefs` files
under `~/.config/unity3d` and on macOS the `unity.*.plist` files are listed too.

**Xbox app / PC Game Pass saves**: the Xbox app hides each game's saves in a container
(`%LOCALAPPDATA%\Packages\<game>\SystemAppData\wgs`) where every file is named by a GUID. The
editor reads the container's index and lists the files under the names the game gave them, both in
the *Xbox app / PC Game Pass saves (any game)* entry and under the game's own entry (Control,
Hades, Palworld, Persona 5 Royal, Final Fantasy XV, Yakuza 0, Clair Obscur: Expedition 33 and
others). The files inside are the same as the Steam ones, so each opens with the game's own screen,
and saving writes the file back in place. Let the Xbox app finish syncing before you open a save,
and keep the game closed while you edit: cloud sync can replace an edited file. Pick a game, then a save, to open it; if none are found it says so. *Open…* and dragging
a file onto the window work as before for anything else.

Capcom RE Engine saves need the Steam account they belong to; away from Steam's folders (on a
phone, or a copied save) enter your 17-digit Steam ID in *Tools › Decrypt with password*.

Games whose format isn't documented open in the hex view with Value search; for packed formats
the data is unpacked first and packed again on save: Cyberpunk 2077 and The Witcher 3 (LZ4
chunks), XCOM 2 / War of the Chosen (LZO chunks, both checksums updated), Mass Effect 1
Legendary Edition (zlib, checksum updated), Civilization V (zlib, in 64 KiB chunks), Dying Light (gzip), Ubisoft `.save.z` (zlib), BioShock Infinite (Unreal Engine 3 zlib chunks, MD5 updated), BioShock Remastered (zlib blocks, each packed back into its own space), Control (CRC-32 updated) and Cairn (the files inside each slot pack; the newest save, the one the game continues from, opens first; the pack's size fields are kept right when a save grows). Other RE Engine
games (Resident Evil 4 from 2023, with its older "Lime" encryption; Monster Hunter Rise on Steam, with its "Citrus" encryption; Resident Evil Requiem, Dragon's Dogma 2, Monster Hunter Wilds and Stories 3, Pragmata,
Dead Rising Deluxe Remaster, Kunitsu-Gami) decrypt and open with their values.

**Find checksum** (*Tools*, or `checksum` on the command line): for a save the editor doesn't know,
it tries CRC-32, CRC-32C, Adler-32, Fletcher-32, FNV-1a, CRC-16, additive sums, MD5, SHA-1 and
SHA-256 stored at the start or end of the file, over the bytes after it, before it, or the whole
file. Earlier copies of the file (from *Backups › Quick restore of an earlier version*) weed out chance matches.
Pick one and it is written again every time you save that file; it shows as zeros while editing.

**Your own templates** (*Tools › Templates folder*, desktop only): for a game the editor doesn't
know, write a small JSON file that says how to recognise its saves (size, magic bytes, extension)
and where each value is (offset, type, limits, drop-down names, copies kept in step, optional
checksum). A save that matches opens as a list of those values, changed in place. The folder has a
guide with an example; templates are easy to share with other players.

**Change Steam account in this save** (*Tools*, or `steamid` on the command line): finds the 64-bit
Steam IDs stored in the save's data (as bytes, numbers or text) and changes every copy to another
account, picked from the accounts on this computer or typed in, so a save copied from another
account or a co-op host loads. The save's packing and checksums are redone on save as for any edit.

**Cheat patches** (*Tools*, or `patch` on the command line): ready-made cheats for console saves
from the [Apollo Save Tool](https://github.com/bucanero/apollo-ps3)'s patch collection, about 2,400
games across PlayStation 1 to 4, PSP, Vita and Dreamcast, built in. The game is found from the
title ID in the save's file or folder name (`BLUS30490`, `BESCES-50003`, `CUSA01615`), or by
searching its name, and a `.savepatch` file can be opened too. Tick the codes (pick a value where a
code offers several) and apply: Save Wizard and BSD script codes run as Apollo runs them, including
their searches, checksum and hash fixes, ciphers and zlib streams. Codes written as Python scripts
aren't supported. A patch meant for another region or version of the game can damage a save, so
keep the backup.

**JSON saves with a key built into the game**: Megabonk (AES), Eiyuden Chronicle: Hundred Heroes
(Triple-DES), Suikoden I & II HD Remaster (AES with a salt, which is kept), Bounty of One, Idle Slayer
and CloverPit (XOR) open already decrypted, show every value by name, and are encrypted again on save.
A key is only used when it gives back JSON. Bounty of One was checked on a public sample save; the
others follow the published keys and are not yet tried on real saves.

**The Long Dark**: the slot file is LZF-packed JSON that holds more LZF/base64 entries (`boot`,
`global`) and JSON inside strings. All of it opens as one tree, and every level is packed again on
save. Built from the published layout and not yet tried on a real save.

**Compressed containers opened for Value search**: Remnant 2 (`profile.sav`, `save_N.sav`; the unpacked data is
searched and its CRC-32 is made again on save), Astroneer (`.savegame`, written back as zlib with the
4 KiB window header the game wants), Satisfactory (`.sav`, 128 KiB zlib chunks, header kept) and
Civilization VII (`.Civ7Save`, chunked zlib between a header and a footer; each player's gold and influence
are on the Tree where the save keeps them in the known place). Craftopia's `.ocs` is gzip over JSON and opens
by name. Avatar: Frontiers of Pandora, Final Fantasy VIII Remastered and Okami HD are listed with their save
folders and open in the Hex view for Value search. The Satisfactory and Civilization VII containers were checked on public sample saves (unpack, repack and
read back); none has been tried in its game yet.

**FANTASIAN Neo Dimension**: `root.json` holds every save slot, each encrypted on its own. One slot
opens at a time (Tools › Open another file in this archive lists the rest), decrypted and shown as one
object whose values (`GameSystemInfo` and the rest) open as JSON by name. Only that slot is encrypted
again on save; the rest of the file stays byte for byte. Built from the published format and not yet
tried on a real save.

**Encrypted console saves**: about 80 PlayStation 3, PlayStation 4, PSP and Vita games (189 title
IDs) whose saves are encrypted, compressed or checksummed (Grand Theft Auto V, Red Dead Redemption 2,
Shin Megami Tensei V, Dynasty Warriors 8, Metal Gear Solid, Resident Evil, Silent Hill and Like a
Dragon: Ishin! among them) open already
unpacked when the save's folder name carries the title ID, as a save exported from a console or an
emulator does (`BLUS31156-SAVE`, `CUSA12345`). The game's own patch file does the work: its decrypt
and decompress codes run when the save opens, and its checksum, compress and encrypt codes run again
on save. A save is only opened this way when packing it again gives back exactly what it read, and
checksums stored inside the data show as zeros and are filled in on save. Checked byte for byte on 18
real encrypted saves from the save-decrypters project.

**Dreamcast saves**: VMU save files (`.vms`, with or without their `.vmi`) open with the header's
CRC kept right: it shows as zeros and is worked out again on save. Nexus memory card `.dci` files
open the same way, with their swapped byte order undone and put back. Saves whose header CRC is
unused open as plain data. Checked on 300 real saves from the Apollo save database: every save with
a CRC came back byte for byte.

**Sega Saturn saves**: single saves from PC and flash-cart tools open without their header: `.bup`
(Pseudo Saturn Kai), `.sro` (SAROO, CRC-32 worked out again on save), `.xml` (SS Backup RAM Parser),
`.b64` (Save Game Manager) and `.cms` (PC Comms Link). Whole backup memory images open one save at a
time (Tools › Open another file in this archive lists the rest): the console's backup RAM as
emulators store it (Mednafen and Beetle `.bkr`, Yabause and Kronos `bkram.bin`) and SAROO's
`SS_SAVE.BIN` and `SS_MEMS.BIN`. A save in an image keeps its size and its bytes go back where they
came from. Checked on 170 real saves from saturn-save-tools in both `.bup` and `.sro` form, and on
Comms Link block dumps, which read the same as the matching `.bup` files.

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
| Palworld (older) | `.sav` with `PlZ` header | round-trip tests (newer `PlM` saves are covered by the Palworld row) |

**Data formats** — JSON, XML (span-patched, keeps formatting), INI/CFG, TOML (keeps comments),
YAML (`.yml`/`.yaml`), Apple plist (binary and XML), MessagePack, CBOR, BSON, SQLite (real `UPDATE`s;
IDs and keys read-only, 0/1 check-constrained columns shown as switches), Python pickle.

**Wrappers** — gzip, zlib, raw deflate, zstd, LZ4, Snappy (framed), xz, bzip2, zip (pick any entry), Base64 (plain or
line-wrapped), UTF-8 BOM, UTF-16, XOR, Easy Save 3 AES, Godot AES, Terraria AES, Hollow Knight AES,
FromSoftware BND4 (MD5 + AES), Minecraft region chunks, mGBA PNG save states, Supergiant (Adler-32 + LZ4), Stellar Blade's EVAS envelope (CRC-32 written again), Ace Combat 8's packed save data (checksum written again), Prey (2017) (RC4 and zlib chunks, MD5 written again).

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

## Command line

The same program runs commands without opening the window, for scripts and batch edits:

    UniversalSaveEditor info   <save> [--json]          what the file is and which screen opens it
    UniversalSaveEditor list   <save> [--json]          every value, by pointer
    UniversalSaveEditor get    <save> /money /party/0/hp
    UniversalSaveEditor set    <save> /money=99999 /name=Ann [-o <out>] [--no-backup]
    UniversalSaveEditor decode <save> [-o <out>]        the unpacked data (JSON, text or bytes)
    UniversalSaveEditor encode <save> <data> [-o <out>] pack edited data the way <save> is packed
    UniversalSaveEditor checksum <save> [<n>|off]       find a checksum in an unknown save, keep one fixed
    UniversalSaveEditor steamid <save> [<old> <new>]    list the Steam IDs in a save, or move it to another account
    UniversalSaveEditor patch  <save> [<game> [<n>...]] console cheat patches: the game's patch files, its codes, or apply codes by number
    UniversalSaveEditor games  [--json]                 the games the editor knows

`--password <p>` opens encrypted saves, like *Tools › Decrypt with password*; `--entry <name>`
picks one save inside an archive or memory card (`info` lists them). Writing is as careful
as the window: the new file must read back exactly before anything is written, the old file is
backed up first (*Backups › Open save manager…* lists the copies) and the write is atomic.
`decode` then `encode` lets any other tool edit the unpacked data while the editor handles the
compression, encryption and checksums.

## Friendly names

- Keys are shown as readable labels (`store.max_hp` → **Max HP**). Turn on **Raw keys** to see
  the original key beside them.
- **RPG Maker**: the game's `data/` folder is found automatically and used to name variables,
  switches, items, weapons, armors and actors. If it isn't found, click the yellow chip or use
  *Tools › Locate game data folder*.
- **Value search** (hex view) works on any file. Search for an exact value or a range, or start with
  *Unknown starting value* when you can't see the number (a health bar, a hidden counter). Then
  change it in-game, save, and *Refine*: keep what is now a given value, what changed, stayed the
  same, increased, decreased, or went up or down by a given amount. *Aligned* tests only offsets
  that are a multiple of the value size, which is how most games store numbers.
- **Binary saves**: use *Value search* to find a value, click the result, then give it a name under
  *Named values*. Names are stored in `<save>.labels.json` and come back next time.

## Safety

- **Verified saves**: the new file is decoded again *before* anything is written. If it doesn't
  read back exactly as shown on screen, the save is aborted and your file is left untouched.
- **Atomic writes**: the save goes to a temp file and is then swapped in, so a crash can't leave a
  half-written save.
- **Save manager**: the *Backups* menu (next to File, Edit, View and Tools) is built into the editor: *Back up this save…*, *Restore a backup…*, *Back up and start a new game…* and *Open save manager…*. Every save first makes an automatic backup of the file as it was (last 30 kept). Those backups sit next to the save in a folder of their own (`Save Editor Backups/<file name>/`) with a `.sebak` extension, so a game never loads or overwrites them as saves. The save manager window lists them with *Restore this backup* (the current file is backed up first, so a restore can be undone), *Compare* (byte by byte or value by value) and *Delete backup*. *Back up this save* makes a named backup that is kept until you delete it (before a boss, say). *Back up and start a new game* makes a named backup and then removes the current save file, so a one-save game starts a new run (the file is closed in the editor so the old data can't be saved back); restore the backup later to go back. The window stays open after that, and *Recent files* offers *Restore a backup…* for a save that is gone, so a backup can be put back even before the game has made a new save. Every backup and every restore is written to a temporary file, flushed to disk, read back and compared before it replaces anything, and nothing is overwritten if the current file can't be read and backed up first (a game holding it locked, say). If the save's folder can't be written to, backups go to the app's own folder instead. Backups can instead go to a folder you choose (a synced folder, another drive); copies from older versions in the app's folder, and in folders chosen before, are still listed. The very first original is also kept as `<name>.bak`
- **Export and import**: *File › Export decoded data* writes the unpacked data (JSON, text or bytes) for any other tool; *File › Import edited data* brings the edited file back, and saving packs, encrypts and checksums it the way the save was
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
`settings.json` in `%APPDATA%\UniversalSaveEditor` (Windows), `~/Library/Application Support/UniversalSaveEditor`
(macOS), `~/.config/UniversalSaveEditor` (Linux) or the app's private storage (Android).

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
  ships only the ids and names it needs to show what's in a save. Libraries a game itself installs are never included.
- **Built on others' work.** Save layouts and item tables from other open-source editors are used under
  their licences (MIT, Apache-2.0, zlib, GPL), and every crate compiled into the program is listed with its
  licence. See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
- **Built with help from Claude (Anthropic).**
- **Use at your own risk.** Back up your saves first. Editing saves is meant for single-player games:
  don't use edited saves in online or multiplayer modes, where it can break the game's rules and get
  an account banned. The editor never touches online services or anti-cheat.

## Licence

Copyright (C) 2026 Lucasifium. Universal Save Editor is free software under the
[GNU General Public License v3.0 or later](LICENSE): you can use, share and change it, and any version
you share must stay free and open source under the same licence. It comes with no warranty.
