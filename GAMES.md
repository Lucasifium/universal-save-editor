# Supported games

Every game with a screen of its own, and what it can edit. The same list, with one page per game, is on the [website](https://universalsaveeditor.com/#games).

## Game screens

The games below open on a screen of their own (the first tab: *Character*, *Trainer*, *Player*,
*House* or *Run*), laid out like the game shows things. Edits there go through the same checks,
undo and checksums as everywhere else, and the *Values* and *Tree* tabs still show everything.

<details>
<summary><strong>A</strong> (32)</summary>

| Game | Screen |
|---|---|
| A Total War Saga: Thrones of Britannia | Every faction's treasury, yours first, with a button to fill it. The campaign inside the save is unpacked, edited with every size above it recomputed, and packed again (the file grows). Read on a public campaign save, not yet tried in the game |
| Aliens: Fireteam Elite | *Marine*: account level and experience, rank, difficulty, and the level and experience of every class and weapon, from `char.sav` (the game writes every byte one lower; that is undone and redone; checked on a public save, not yet tried in the game) |
| Alpha Protocol | *Agent*: money, level, experience, skill points, skill ranks, ammunition and gadget counts from a checkpoint `.sav` (checked on a public save, not yet tried in the game) |
| American Truck Simulator | The same screen as Euro Truck Simulator 2: money, loan limit, experience, skills, and each vehicle's fuel, odometer and wear. |
| Anno 1800 | *Company*: your coins (the load screen's total is changed with them) and the rest of your company's storage by GUID; the `.a7s` archive and its 100+ MB game data are unpacked and packed again for you (checked on a public save, not yet tried in the game) |
| Antimatter Dimensions | *Dimensions*: antimatter, infinity points, eternity points, reality machines, the counters behind them, replicanti, dilation and perk points; opens the exported save text from Options > Saving (checked on public sample saves, not yet tried in the game) |
| Assassin's Creed Brotherhood | *Inventory*: the player's money and gadget and ammunition counts, with other characters' and older copies listed apart; the save's packed blocks are opened and written again with their checksums (checked on public saves, not yet tried in the game) |
| Assassin's Creed Brotherhood | *Ezio*: florins, smoke bombs, throwing knives, poison and bullets. The save is unscrambled (the key for your account is worked out from the file itself), unpacked and packed again with its block checks. Checked on public saves, not yet tried in the game; close the game and turn cloud saves off in Ubisoft Connect first |
| Assassin's Creed III Remastered | *Inventory*: money and gadget and ammunition counts; the Ubisoft Connect wrapper, its encryption and the packed blocks are opened and written again with their checksums (checked on public saves, not yet tried in the game) |
| Assassin's Creed III Remastered | *Connor*: money, smoke bombs, poison, rope darts, arrows, bullets and snares. The save is unscrambled (the key for your account is worked out from the file itself), unpacked and packed again with its block checks. Checked on public saves, not yet tried in the game; close the game and turn cloud saves off in Ubisoft Connect first; the save ends with a 4-byte value that isn't understood yet and is kept as it is |
| Assassin's Creed IV Black Flag | *Inventory*: money, gadget and ammunition counts, crew, heavy shot, harpoons and the ship's cargo (wood, metal, cloth, rum, sugar) with a Max crafting materials button (checked on public saves, not yet tried in the game) |
| Assassin's Creed IV Black Flag | *Edward*: reales; the Jackdaw's wood, metal and cloth with a **Max crafting materials** button that fills the hold to 2,000 (its fully upgraded size); rum and sugar; smoke bombs, rope darts, bullets, mortar shots, heavy shot and harpoons. The save is unscrambled (the key for your account is worked out from the file itself), unpacked and packed again with its block checks. Checked on public saves, not yet tried in the game; close the game and turn cloud saves off in Ubisoft Connect first |
| Assassin's Creed Liberation HD | *Inventory*: money and gadget and ammunition counts (checked on public saves, not yet tried in the game) |
| Assassin's Creed Liberation HD | *Aveline*: money, smoke bombs, throwing knives, rope darts and bullets. The save is unscrambled (the key for your account is worked out from the file itself), unpacked and packed again with its block checks. Checked on public saves, not yet tried in the game; close the game and turn cloud saves off in Ubisoft Connect first |
| Assassin's Creed Odyssey | *Character*: drachmae, experience and ability points; materials and gear are not mapped yet (checked on public saves, not yet tried in the game) |
| Assassin's Creed Odyssey | *Misthios*: drachmae, XP and ability points; olive wood, iron, soft leather, obsidian glass, precious gems and ancient tablets with a **Max crafting materials** button (99,999, an estimate: no public source states a limit). The save is unscrambled (the key for your account is worked out from the file itself), unpacked and packed again with its block checks. Checked on public saves, not yet tried in the game; close the game and turn cloud saves off in Ubisoft Connect first |
| Assassin's Creed Origins | *Character*: drachmae, experience and ability points; materials and gear are not mapped yet (checked on public saves, not yet tried in the game) |
| Assassin's Creed Origins | *Bayek*: drachmae, XP, ability points and hard leather. The save is unscrambled (the key for your account is worked out from the file itself), unpacked and packed again with its block checks. Checked on public saves, not yet tried in the game; close the game and turn cloud saves off in Ubisoft Connect first; the save ends with a 4-byte value that isn't understood yet and is kept as it is |
| Assassin's Creed Revelations | *Ezio*: akçe, smoke bombs, throwing knives, poison and bullets. The save is unscrambled (the key for your account is worked out from the file itself), unpacked and packed again with its block checks. Checked on public saves, not yet tried in the game; close the game and turn cloud saves off in Ubisoft Connect first |
| Assassin's Creed Rogue | *Inventory*: money, gadget and ammunition counts, crew, heavy shot, harpoons and the ship's cargo (wood, metal, cloth, tobacco) with a Max crafting materials button (checked on public saves, not yet tried in the game) |
| Assassin's Creed Rogue | *Shay*: reales; the Morrigan's wood, metal, cloth and stone with a **Max crafting materials** button that fills the hold to 2,500 (its fully upgraded size); tobacco; smoke bombs, rope darts, bullets, mortar shots, heavy shot, harpoons and burning oil. The save is unscrambled (the key for your account is worked out from the file itself), unpacked and packed again with its block checks. Checked on public saves, not yet tried in the game; close the game and turn cloud saves off in Ubisoft Connect first |
| Assassin's Creed Syndicate | *Inventory*: money, lockpicks and gadget and ammunition counts (checked on public saves, not yet tried in the game) |
| Assassin's Creed Syndicate | *Frye twins*: pounds; leather, metal, chemicals and silk with a **Max crafting materials** button (9,999, an estimate: no public source states a limit); smoke bombs, berserk darts, medicine, bullets and voltaic bombs. The save is unscrambled (the key for your account is worked out from the file itself), unpacked and packed again with its block checks. Checked on public saves, not yet tried in the game; close the game and turn cloud saves off in Ubisoft Connect first; the save ends with a 4-byte value that isn't understood yet and is kept as it is |
| Assassin's Creed Unity | *Inventory*: money and gadget and ammunition counts (checked on public saves, not yet tried in the game) |
| Assassin's Creed Unity | *Arno*: francs and Creed Points, smoke bombs, berserk darts, medicine, bullets, cherry bombs and stun grenades. The save is unscrambled (the key for your account is worked out from the file itself), unpacked and packed again with its block checks. Checked on public saves, not yet tried in the game; close the game and turn cloud saves off in Ubisoft Connect first |
| ASTLIBRA Revision | Gold, level, experience towards the next level and the six force pools (fire, earth, water, darkness, wind, holy). The save's two checksums are written again. Checked on public saves, not yet tried in the game |
| Atelier Firis: The Alchemist and the Mysterious Journey | *Resources*: Cole (original 2017 release; item qualities and recipes are left alone; checked on a public save, not yet tried in the game) |
| Atelier Sophie: The Alchemist of the Mysterious Book | *Resources*: Cole and Tess exchange tickets (original 2017 release; item qualities and recipes are left alone; checked on a public save, not yet tried in the game) |
| ATOM RPG | *Party*: every party member's level, experience, seven attributes, seventeen skills and free skill, stat and perk points; each member's inventory with counts (the player's rubles are the Money item). The gzip packing is handled for you (checked on a public save, not yet tried in the game) |
| Atomic Heart | *P-3*: metal parts, synthetic material, biomaterial, superconductors, chemicals, energy cells and microelectronics with a Max crafting materials button, plus neural modules and neuropolymer (stacks the save already holds). The save's compressed data is unpacked and packed again with the compression library of any game on this PC, and its SHA-1 is made again (checked on public and real saves, not yet tried in the game) |
| Attack on Titan (A.O.T.: Wings of Freedom) | *Resources*: funds and the quantity of every item you own, shown by number rather than name; the stream cipher and word checksum are handled for you (checked on a public save, not yet tried in the game) |
| Attack on Titan 2 | *Resources*: regiment funds and Wings of Freedom (up to 999), your player level (up to 120) and experience, the friendship level (0 to 10) and friendship points (up to 1,000) of every character you've met, every crafting material by name, including ones you don't own yet, with a *Max crafting materials* button (999 each), every gift by name (up to 99 each, including ones you don't own yet), and which skills are equipped, in the Final Battle save; the stream cipher and word checksum are handled for you (checked on public saves and in the game) |

</details>

<details>
<summary><strong>B</strong> (16)</summary>

| Game | Screen |
|---|---|
| Balatro | Money, ante, hands and discards; this round; jokers and consumables; poker hand levels |
| Baldur's Gate 3 | Gold and the stack size of every stacked item each companion carries (potions, arrows, scrolls, camp supplies), read from the save's packed entity store, plus the party summary (origin, class, level, experience) and *Story flags*: every named global story flag by area and companion (Emerald Grove, camp, Nautiloid, each companion and so on), ticked on or off, with a search box and filters for set, cleared, changed and risky flags. Flags that are riskier to change (someone living or dying, party membership, romance, story progress, the tadpole story, scenes in progress, testing switches) are marked with the reason; the story script's own list of set flags is kept in step when the save is written, and a backup is offered before editing. *Add items*: copies of any kind of item in the party's bags can be put into a chosen character's bag (new entities in the entity store with their inventory and owner records, and the matching item records; stacked items, worn gear, containers, keys and story items left out). Checked on about a hundred public saves; not yet tried in the game |
| Barotrauma | *Campaign*: money, levels passed and reputation with each faction; *Crew*: each hired crew member's experience, talent points, salary and skills, from the campaign's `.save` (the gzip bundle is opened and packed again with the submarine kept as it was; checked on public saves, not yet tried in the game) |
| Batman: Arkham City | *Progress*: difficulty, experience, player level and the upgrades bought for Batman and Catwoman; the LZO-packed save is unpacked and packed again (saves of a brand-new game have no experience yet; checked on public saves, not yet tried in the game) |
| BeamNG.drive | *Career*: career money and BeamXP from `career/playerAttributes.json` of a save slot; the other career values (branch levels, vehicles) open by name in the Tree. Written from the game's own Lua source, not yet checked on a real save |
| BioShock Infinite | *Booker*: Silver Eagles (up to 99,999), salts, lockpicks and the Health, Shield and Salts infusion levels (0 to 10; a kind never picked has no entry yet); unpacked and packed again with the save's MD5. Not yet tried in the game |
| Bitburner | *Player*: money, BitNode, karma, HP, the skill levels and the experience behind them (an exported save; the JSON nested inside it is opened as one; checked on a real save, not yet tried in the game) |
| Black Mesa | *Player*: health and armour; spare ammo by type (9mm, .357, buckshot, uranium, hornets and the rest); each carried weapon's magazine, with the weapon in hand marked. Changed in place |
| Black Myth: Wukong | *Destined One*: Will, Sparks, level, experience and play time from `ArchiveSaveFile.<n>.sav`; the header checksum is written again when you save. checked on a public sample save, not yet tried in the game |
| Blasphemous | *Penitent One*: Tears of Atonement, health and fervour, bile flasks, every permanent stat upgrade, completion and play time (the base64 wrapping is handled); owned beads, prayers and relics are in the Tree. checked on a public save, not yet tried in the game |
| Borderlands (2009) and Borderlands Game of the Year Enhanced | *Vault Hunter*: level, experience, skill points, money, skill levels and ammunition from the character saves `Save0001.sav` ... (items and weapons are left as they are; checked on public saves, not yet tried in the game) |
| Borderlands 2 | *Vault Hunter*: level, experience, skill points; money, Eridium, Seraph crystals and Torgue tokens; ammo and ammo SDUs, with buttons to fill ammo and max the SDUs; backpack and bank SDUs (with the room they give), weapon slots and playthroughs completed, which opens True and Ultimate Vault Hunter Mode; gear is listed (level, slot), not edited. Packing and checksums are redone on save. Edited saves not yet tried in the game |
| Borderlands 4 | *Character*: cash, Eridium, level and experience, specialization, skill points to spend, play time and ammunition, from each character's `.sav`. The file is unlocked with the Steam account named in its folder and packed again when you save. checked on public sample saves, not yet tried in the game |
| Borderlands: The Pre-Sequel | *Vault Hunter*: level, experience, skill points; money and Moonstone; ammo and ammo SDUs; backpack and bank SDUs, weapon slots and playthroughs completed; gear is listed, not edited. Packing and checksums are redone on save. Edited saves not yet tried in the game |
| Bravely Default II | *Party*: pg (money), the difficulty, each party member's level, experience, HP, MP, special ability points and stat raises, and how many of each item the party carries; the `SAVE` header and zlib packing are handled for you (checked on a public save, not yet tried in the game) |
| Bug Fables: The Everlasting Sapling | *Team Snakemouth*: berries, rank, experience, TP, MP, inventory and storage sizes, play time, and the HP, attack and defense of Vi, Kabbu and Leif in `save0.dat` … `save2.dat` (PC); every other value is kept (checked on a public save, not yet tried in the game) |

</details>

<details>
<summary><strong>C</strong> (16)</summary>

| Game | Screen |
|---|---|
| Cairn (Story and On the Trail) | *Climber*: who climbs (Aava or Marco); play time, falls and deaths; the running buffs (Focus, Grip, Temperature, Grit, Burst and the rest: seconds left, source, strength; start, stretch or end them); the backpack, piton belt and chalk pouch as item grids with the game's own item names (add, remove or swap items, uses and sips left, what a bottle holds); the slot's saves, marking the one the game continues from |
| Car Mechanic Simulator 2018 | *Mechanic*: money, level, experience and the garage-shed counts, from the profile's `GlobalData.dat` (.NET BinaryFormatter, read and written in place; checked on a public save, not yet tried in the game) |
| Car Mechanic Simulator 2021 | *Mechanic*: money, level, XP and scrap, the profile's name, play time and difficulty, the unlocked parking levels, barns and warehouses (checked on a public save, not yet tried in the game) |
| Cat Goes Fishing | *Angler*: money, level, experience, the chosen rod and gear (checked on a public save, not yet tried in the game) |
| Cave Story+ | *Quote*: every save slot's difficulty (Normal, Easy or Hard), health, max health and Whimsical Stars, and each weapon's type, level, energy and ammo; equipment, items and the stage are listed. The freeware Cave Story's `Profile.dat` opens too (checked on public saves, not yet tried in the game) |
| Celeste | *Save*: strawberries, deaths, jumps and the other totals, chapters unlocked, the assist options, and for every chapter the cassette and the A, B and C sides' completion, full clear and heart. Open `0.celeste` (checked on a public save, not yet tried in the game) |
| Chef: A Restaurant Tycoon Game | *Restaurant*: the budget, ingredient points, restaurant level, rating, seats and city map points (checked on a public save, not yet tried in the game) |
| Clair Obscur: Expedition 33 | Gold; each party member's level, experience, action points, health, attribute points and Lumina from Colours of Lumina; each weapon's level; item counts, with a **Max crafting materials** button that fills every Chroma Catalyst you have to 999 (an estimate: the game states no limit) (checked on a real save, not yet tried in the game) |
| Clicker Heroes | *Clicker*: gold, rubies, hero souls, ancient souls, primal souls, the highest zones, multipliers, and every hero's and outsider's level from the Steam version's save (the base64 and deflate packing is handled). checked on a public save, not yet tried in the game |
| Cookie Clicker | *Bakery*: cookies in the bank and baked, counters, ascensions and the number of every building, in the text from Options > Export save (checked on a public save, not yet tried in the game) |
| Core Keeper | *Character*: skill experience (mining, running, melee, vitality, crafting, ranged, gardening, fishing, cooking, magic, summoning), ancient coins, maximum health and the inventory counts, from the plain-JSON character file (checked on a public save, not yet tried in the game) |
| Crusader Kings II | *Empire*: the ruler's wealth, prestige, piety and attributes. Ironman (binary) saves are edited in place, dates included (checked on a save converted to the binary layout; no save from a real ironman campaign has been tried yet) |
| Cult of the Lamb | *Cult*: the Lamb's level, XP, hearts and fervour; cult faith, level, shrine and doctrine levels; the inventory and storage by item name, with a **Max crafting materials** button that fills lumber, stone, bones, grass, planks, stone blocks, gold nuggets and bars, crystal shards, spider silk and shells to 999 (an estimate), leaving coins, food, seeds, gifts, necklaces and boss items alone (checked on a public sample save, not yet tried in the game) |
| Cuphead | *Inkwell Isle*: coins and deaths for both players, Djimmi's wishes, expert mode, the filters, Boatman and the other unlocks (checked on a public sample save, not yet tried in the game) |
| Cyberdimension Neptunia: 4 Goddesses Online | Money, guild rank, and the totals the game keeps (money earned, enemies defeated, dungeons cleared, longest combo, time played). Checked on a public save, not yet tried in the game |
| Cyberpunk 2077 | *V*: eurodollars, from V's own inventory (found through the save's section table; the table's position is updated when the save is repacked) |

</details>

<details>
<summary><strong>D</strong> (42)</summary>

| Game | Screen |
|---|---|
| Dark Souls: Prepare to Die Edition | *Character*: name, soul level, souls, humanity and the eight attributes of each character slot, with HP and stamina shown; the item list of this edition is not read yet (the checksums are rewritten). checked on a public save (the character stats read as plausible values and a changed save keeps its checksums), not yet tried in the game |
| Darkest Dungeon | *Hamlet*: the estate (gold, busts, portraits, deeds, crests, blueprints, shards), how many of each trinket and estate item it holds, and the roster (each hero's resolve XP, stress, HP, weapon and armour rank, skill upgrade levels), changed in place. Open persist.estate.json or persist.roster.json |
| Darksiders Genesis | *Horsemen*: souls, coins, keys, health potions, Strife's and War's powers, play time, and each horseman's wrath and health and wrath shards, from the plain Unreal save `SaveSlot_N.sav` (checked on a public save, not yet tried in the game) |
| Darkwood | *Survivor*: level, experience, health, stamina and lives; karma, the day and workbench level; every item in the backpack and hotbar by name and amount, from `sav.dat` (plain JSON, read and written in place; checked on public saves, not yet tried in the game) |
| Dave the Diver | *Dave*: gold, Bei, chef flames, research and trust points, and every sushi ingredient's stock, with a **Max crafting materials** button that fills the ingredients you have to 999; the save's encryption is removed and put back for you (checked on public saves, not yet tried in the game) |
| Dead Island | *Survivor*: money, level, maximum health and health, from `save_N.sav` (the gzip wrapper is removed and put back and every other byte kept; checked on a public Definitive Edition save, not yet tried in the game) |
| Dead Island: Riptide | *Survivor*: the same four values from `save_N.sav`; the layout matched a Dead Island sample, no Riptide save has been checked yet |
| Dead Rising | *Survivor*: Frank's level, PP, maximum health and skills, with buttons to learn every skill or reach level 50; the save's block checksums are rewritten (checked on public speedrun saves, not yet tried in the game) |
| Dead Rising 2 | *Survivor*: Chuck's level, PP and money; attack, speed, health, item slots and throwing as bars up to the game's maximum, with a button to max them; the CRC-32 is rewritten (checked on public speedrun saves, not yet tried in the game) |
| Dead Rising 2: Off the Record | *Survivor*: Frank's level, PP and money (the load screen's copy kept in step); attack, speed, health, item slots and throwing as bars up to the game's maximum, with a button to max them; all five CRC-32s are rewritten (checked on public speedrun saves, not yet tried in the game) |
| Death Must Die | *Profile*: gold, runs, wins, current and best streak, time in runs and in the game, selected character. The JSON the game keeps inside the save's strings is opened and written back as strings (checked on a public sample save, not yet tried in the game) |
| Death Stranding (Director's Cut) | *Facilities*: the material stocks (chiral crystals, resins, metals, ceramics, chemicals, special alloys) at every facility you've found, with its connection level, and a **Max crafting materials** button that fills them to an estimate of the most each facility stores at five stars (knot cities, distribution centers and preppers). Likes received. Unpacked (XOR, compression) and packed again with the save's MD5 on save, using the compression library a game on your computer already has. Checked on public saves, not yet tried in the game |
| Death Stranding 2: On the Beach | *Facilities*: the material stocks (chiral crystals, resins, metals, ceramics, chemicals, special alloys) at every facility you've found, with its connection level, and a **Max crafting materials** button that fills them to the most that kind of facility stores at five stars (preppers, labs and observatories, distribution centers, knot cities); likes from NPCs and other porters and likes given; day, time of day and play time. Unpacked (XOR, LZ4) and packed again with the save's MD5 on save; pieces that didn't change keep their bytes. Not yet tried in the game |
| Deathloop | *Colt*: Residuum, stress level and stress XP, the day, location and progress flags of the loop (first loop, Golden Loop, Julianna, void essence ability). The save's compressed data is unpacked and packed again with the compression library of any game on this PC, and its checksum is made again (checked on a real save and in the game; close the game and turn Steam Cloud off for it first, or Steam may bring the old save back) |
| Deep Rock Galactic | *Dwarves*: credits, perk points, vanity mastery and forge XP; each dwarf's XP and retirements; the minerals and brewing ingredients, with a **Max crafting materials** button that fills Bismor, Magnite, Croppa, Umanite, Jadiz, Enor, Phazyonite and the brewing ingredients to 9999 (an estimate), leaving credits, Data Cells, Error Cubes and Blank Cores alone (checked on a public sample save, not yet tried in the game) |
| Deep Rock Galactic: Survivor | *Profile*: credits, the minerals and Power Cores, player rank, each class's XP and rank, and the stat upgrade levels. Open `drgs.dat` (checked on a public save, not yet tried in the game) |
| Deltarune | Chapter 1 fully (party, inventory as item numbers, Light World); Chapters 2 to 4: dark dollars and the party's stats and equipment. Every line of the file can be edited. Not yet tried in the game |
| Deltatraveler | *Party*: name, gold, experience, play time, party members and the item, weapon and armor ids (checked on public saves, not yet tried in the game) |
| Destroy All Humans! (2020) | Crypto's DNA, the currency for weapon, saucer and ability upgrades. Checked on a public save, not yet tried in the game |
| Deus Ex (2000) | JC Denton: credits, skill points, bioelectric energy, health per body part, ammo, medkits and other item counts, skill and augmentation levels (values the game left out can't be set). Not yet tried in the game |
| Deus Ex: Mankind Divided | *Adam*: credits, Praxis kits, crafting parts (with a **Max crafting materials** button) and each ammo stack, with total experience and progress to the next Praxis kit shown (checked on real saves and in the game; close the game and turn Steam Cloud off for it first, or Steam may bring the old save back) |
| Devil May Cry HD Collection (Devil May Cry 1) | *Slots*: red, yellow and blue orbs and play time of each used slot of the Devil May Cry 1 save `dmc1.sav` (checked on a public save, not yet tried in the game; Devil May Cry 2 and 3 saves open as hex) |
| Diablo II, Diablo II: Resurrected | *Character*: level, experience, gold and stash gold; the four attributes, stat and skill points; life, mana and stamina (base values); points in every skill; *Waypoints*: every waypoint in Normal, Nightmare and Hell, with an open-all button |
| Disco Elysium | *Detective*: money, level, experience, skill points to spend, health and morale, the four attributes and all 24 skills from a `.ntwtf.zip` save (the zip is repacked with its other files kept). checked on public saves, not yet tried in the game |
| Dishonored | *Corvo*: coins (the save's zlib chunks are unpacked and packed again; the coin count is found by its surroundings and offered only when one record fits; checked on 11 real saves, the coin edit tried in the game; runes, rewire tools and elixirs not located yet) |
| Dishonored 2 | *Bonecharms*: Raw Whalebone with a Max crafting materials button, runes, and crossbow bolts, pistol bullets, grenades, spring razors and stun mines up to what the player can carry; `checkpoint.dat` is unpacked and packed again with its check value (checked on real saves and tried in the game) |
| Divinity: Original Sin 2 Definitive Edition | Each player character's experience, vitality, attributes, unspent points and gold, and the stack size of every item each carries (not inside bags); *Add items*: any kind of item found in an inventory in the save can be added to a chosen character's bag (as a copy of one the game made, entered in the save's item, inventory and entity records; keys, containers and story items left out). The package is packed again with its checksum on save. Not yet tried in the game |
| Divinity: Original Sin Enhanced Edition | *Party*: each party member's experience, vitality, gold and item stack sizes, on the Original Sin 2 screen; the `.lsv` package is unpacked and packed again (checked on a public save, not yet tried in the game) |
| Don't Starve | *Survivor*: health, hunger, sanity, temperature, time alive and the day count, from `survival_N` (the base64 and zlib wrapper is removed and put back and the Lua text inside edited by name; `profile` opens as JSON; checked on a public save, not yet tried in the game) |
| Door Kickers | *Squad*: the squad's experience, each trooper's experience, kills, injuries and mission results, plus the doctrine points and the stars used (checked on a public save, not yet tried in the game) |
| Dragon Age: Origins, Awakening | Party money; the Warden's and followers' level, experience, attributes and unspent points, approval, and backpack stack sizes. Tried on saves from one playthrough; Awakening saves not yet tried |
| Dragon Quest Builders 2 | *Builder*: level, experience, hit points, attack and defence; *Mini medals*; *Equipment*; *Pockets* and *Bag*: item ids and quantities, from `CMNDAT.BIN` (the zlib data is opened and re-sealed; island files not opened; checked on a public save, not yet tried in the game) |
| Dragon Quest XI S: Echoes of an Elusive Age | *Party*: each member's level, HP, MP, attack, defence, agility, deftness, charm, magical might and mending, max HP and MP and experience; *Item bags*: the count of every item in every bag, up to the item's own maximum. The AES-encrypted save is opened and its CRC is kept right (checked on a public save, not yet tried in the game) |
| Dragon's Dogma: Dark Arisen | *Arisen*: gold, Rift Crystals, level, experience, health, stamina and base stats, and every vocation's rank and experience. The save's packing (header, size and checksum) is opened and written again, and the Arisen's data, which the save keeps twice, is written to both copies (checked on public saves, not yet tried in the game) |
| Drakensang: River of Time | *Heroes*: each hero's level, adventure points (earned and to spend), life, endurance, astral energy, the eight attributes and every talent, with the party slot to tell your party from the cutscene copies (the SQLite `savegame.dsa` is edited in place; checked on a public save, not yet tried in the game) |
| Drakensang: The Dark Eye | *Heroes*: each hero's level, adventure points (earned and to spend), life, endurance, astral energy, the eight attributes and every talent, with the party slot to tell your party from the cutscene copies (the SQLite `savegame.dsa` is edited in place; checked on a public save, not yet tried in the game) |
| DREDGE | *Angler*: funds, hull tier, sanity and the catch counts (rod, nets, crabs, aberrations, relics), from `dredge-save0.bin` (.NET BinaryFormatter, read and written in place; checked on a public save, not yet tried in the game) |
| Dying Light | *Survivor*: the skill tree levels (every copy the save keeps) |
| Dying Light 2 / The Beast | *Survivor*: money, health, the Traversal / Combat / Inhibitor levels and points, every skill's level (Techland's self-describing save format; every saved number is in *Values*) |
| Dynasty Warriors 4 Hyper | *Officers*: the difficulty, each officer's playable flag, life, Musou, attack, defense, experience and weapon experience, and the level of every item (or whether you own it); the checksum is handled for you (checked on a public save, not yet tried in the game) |
| Dynasty Warriors 7: Xtreme Legends Definitive Edition | *Army*: gold and each officer's health, attack, defense, power, speed and skill points; the stream cipher and word checksum are handled for you (checked on a public save, not yet tried in the game) |
| Dynasty Warriors 8: Xtreme Legends Complete Edition | *Resources*: gold, gems, facility and weapon materials; *Officers*: each of the 82 officers' attack, defence and health, and the four weapon-compatibility star ratings. The encrypted `save.dat` is opened and both checksums are kept right (checked on a public save, not yet tried in the game) |

</details>

<details>
<summary><strong>E</strong> (10)</summary>

| Game | Screen |
|---|---|
| Elden Ring, Dark Souls III / II / I | Name, class and level; currency; attribute bars (level follows); HP / FP / Stamina meters; runes or souls to the next level (Elden Ring, Dark Souls III and I); inventory grids (with a search box across every list) for items, key items, storage, weapons (level and affinity / infusion), armor and talismans / rings; a *Max upgrade materials* button (Elden Ring: every Smithing Stone, Somber Smithing Stone and Glovewort to 999; Dark Souls: every titanite to 99); Elden Ring also has a **Max crafting materials** button that fills the cookbook materials you carry or store (meat, bones, flowers, mushrooms, butterflies, stones…) to 999, leaving key items alone |
| ENDER LILIES: Quietus of the Knights | *Lily*: level, blight (experience), prayer level and prayers left, Stagnant Blight, Furious Blight and Ancient Souls with a **Max crafting materials** button that fills both blights to 999 (the most players report; Ancient Souls are left alone), and every spirit's level (checked on public saves, not yet tried in the game) |
| Enter the Gungeon | *Gungeoneers*: each Gungeoneer's Hegemony credits, runs, deaths, enemies killed, money collected, play time, keys, blanks and floors cleared; the save's repeating-key XOR is handled. checked on a public save, not yet tried in the game |
| Escape from Duckov | Money, experience and the day. Easy Save 3 data (the game's own quirk of writing a value without its key is read); checked on a public save, not yet tried in the game |
| Etherlords | *Wizards*: each player's elemental resources and remaining mana and the campaign's points and day (checked on a public save, not yet tried in the game) |
| Euro Truck Simulator 2 | *Driver*: money, loan limit, experience, the five skills and ADR classes; every vehicle's fuel, odometer and wear, with buttons to max skills, fill every tank and repair everything. Packed saves are unpacked and packed again with their checksum. |
| Europa Universalis III | *Empire*: your country's treasury, prestige, stability, manpower, inflation, war exhaustion and army and navy tradition (every other country by tag in the Tree); edited in place in the plain-text save (checked on a public save, not yet tried in the game) |
| Europa Universalis IV | *Empire*: treasury, manpower, prestige, stability, monarch power and every country value. Ironman (binary) saves are edited in place and stay ironman; fields outside this screen show as numbers (checked on public saves, not yet tried in the game) |
| EVERSPACE | *Pilot*: DNA fragments, flashback progression, the pilot's records and every perk with its level and lock from `CurrentPlayerSave.sav` (checked on a public save, not yet tried in the game) |
| Everspace 2 | *Pilot*: credits, level, experience and device upgrade tokens; every crafting resource by name, with a **Max crafting materials** button (ores, crystals, scrap metal and basic crafting resources to 99,999, an estimate: the game states no cap; catalysts, rift and faction resources left alone). The save's zlib wrapper is handled for you; ships, items and missions are in the Tree (checked on a public save, not yet tried in the game) |

</details>

<details>
<summary><strong>F</strong> (19)</summary>

| Game | Screen |
|---|---|
| Fable: The Lost Chapters | *Albion*: the hero's gold, morality, renown level and age; the save's packing and checksum are handled for you (checked on public saves, not yet tried in the game) |
| Fallout 3 | Bottle caps, experience, karma, S.P.E.C.I.A.L. and the points added to each skill (checked on real saves, not yet tried in the game) |
| Fallout Shelter | *Vault*: caps, food, power, water, Stimpaks, RadAway, Lunchboxes, Mr. Handy, pet carriers and Nuka-Cola Quantum; every dweller's level, happiness, health, radiation and S.P.E.C.I.A.L. The save's encryption is removed and put back on save (checked on a public sample save, not yet tried in the game) |
| Fallout, Fallout 2 | *Pip-Boy*: level, experience, skill points, hit points, radiation, poison, S.P.E.C.I.A.L., skills, the two traits, the tagged skills, every perk's rank, kill counts, bottle caps and the count of each item carried (with loaded rounds). The save's own layout is worked out each time, so mods that add items still open |
| Fallout: New Vegas | Bottle caps, experience, karma, S.P.E.C.I.A.L. and the points added to each skill; fame and infamy with each faction you have dealt with (NCR, Caesar's Legion, the Strip, the Brotherhood and nine more) (checked on real saves, not yet tried in the game) |
| Final Fantasy IX (Steam) | *Party*: every saved game in `SavedData_ww.dat` (decrypted and encrypted again): gil, and each character's level, experience, HP / MP and base stats |
| Final Fantasy Pixel Remaster (I–VI, PC) | *Party*: one save slot, decrypted (PBKDF2 key, Rijndael with a 256-bit block, deflate) with the character and inventory data that the game keeps as text inside it opened too, and encrypted again on save; gil, total gil, steps, battle and chest counters, every character's experience, HP / MP and bonus stats, and the item and key-item counts. Checked on public sample saves of FF I–IV against the reference tool's own decryption; not yet tried in the game |
| Final Fantasy Resonance | *Party*: Gil and play time; each character's level (shown from the experience), experience, HP, MP, enemies defeated and bonus stats, plus the equipped weapon and armor ids; each Vision's awakening and mastery points; item quantities with a **Max consumables** button (99 each, key items left alone); the artifacts you hold, switched on or off. The save (an SVHD header around AES-256 encrypted Unreal data) is decrypted and encrypted again with its checksum rewritten. The game has no crafting. Checked on a synthetic test save built from the public format notes, not on a real save or in the game |
| Final Fantasy VII (PC / 2013 Steam) | *Party*: each slot's gil and play time; every character's level, experience, kills, limit level and gauge, HP / MP with the base maximum the game builds them from, and the six stats. Slot checksums are updated |
| Final Fantasy VII Rebirth | Gil, Moogle Medals, Gold Saucer Points and the inventory: add or remove items, crafting materials, chocobo gear and Queen's Blood cards, and change amounts of those and of armor and accessories, with a **Max crafting materials** button (99 each); materia to add or remove, with each one's level and AP. On the *Party* panel each character's level, HP / max HP, MP / max MP and experience |
| Final Fantasy VII Remake (Intergrade) | Gil, and adding or removing items, armor, accessories and music discs or changing their amounts (key items are shown but left alone, and worn armor stays); materia to add, or to remove when nobody has it equipped. On the *Party* panel each character's level, HP / max HP, MP / max MP and experience |
| Final Fantasy X (HD Remaster, PC) | *Party*: gil, and every character's and aeon's HP, MP, maxima and stats (with and without equipment), AP, Sphere levels, overdrive gauge and mode. *Abilities*: every skill, special, white and black magic per character and aeon, with a learn-all button. *Overdrives*: each character's overdrives and overdrive modes. *Items*: all 112 item slots (item and count, with a set-all-to-99 button) and the key items. *Monster Arena*: how many of each fiend is captured. The checksum (the game's own CRC-16) is updated |
| Final Fantasy X-2 (HD Remaster, PC) | *Gullwings*: gil; Yuna, Rikku and Paine's level, experience, HP / MP and stats. Checksum updated |
| Final Fantasy XIII | *Party*: gil (the save is decrypted to edit and encrypted again, with its checksum rewritten; checked on public saves, not yet tried in the game) |
| Final Fantasy XV (Windows Edition) | *Party*: gil, ability points and medals; every character's level, HP / MP and stats, the experience waiting to be banked at the next rest, total experience and kills; item amounts in every bag (by item name), with a **Max crafting materials** button that fills every treasure to 99. Decrypted (AES-192, shuffled blocks) and encrypted again on save. Not yet tried in the game |
| Final Fantasy XVI (PC) | *Clive*: gil and Ability Points, level and max HP (shown); every consumable and material you have picked up, with a **Max crafting materials** button (99, 999 or 9999 per material, as the game's Items menu shows them). The slot picture's save is unpacked with a dictionary read from your own installed copy of the game and packed again with its CRC-32 rewritten. Tried in the game |
| Food Truck Simulator | *Truck*: money, prestige and the truck's fuel (checked on a public save, not yet tried in the game) |
| For The King | *Party*: each hero's gold, level, XP, health and focus, and the party's life pool; the save's gzip and UTF-16 wrapping is handled for you (checked on a public save, not yet tried in the game) |
| FTL: Faster Than Light | *Run*: scrap, fuel, missiles, drone parts and hull, the run's difficulty, sector and counters, and each crew member's health and records, from `continue.sav` (file format 11, the current version; checked on a public save, not yet tried in the game) |

</details>

<details>
<summary><strong>G</strong> (14)</summary>

| Game | Screen |
|---|---|
| Game Dev Tycoon | *Saved companies*: every saved company's cash, fans, research points, staff limit and level, from the game's Local Storage database (the load menu's copy of cash and fans is kept equal; checked on a public save, not yet tried in the game) |
| Gas Station Simulator | *Station*: the player's and the station's money, fuel income and purchases, popularity level, gas station level and quest progress, from `GameSave_N.sav` / `AutoSaveN.sav` (plain Unreal GVAS; checked on public saves, not yet tried in the game) |
| Generation Zero | *Characters*: skill points, experience, level, inventory amounts, the crafting resource store and safehouse storage with Max crafting materials, from `savegame` (checked on a public save, not yet tried in the game; the cap is an estimate) |
| Ghost of Tsushima Director's Cut | *Jin Sakai*: supplies, flowers, Ghost Flowers, bamboo, yew wood, wax wood, iron, steel, gold, linen, leather, silk and predator hides, with a **Max crafting materials** button, and technique points to spend; the legend earned on Tsushima and Iki Island is shown. The save's checksum is made again (checked on public saves, not yet tried in the game; charms, gear levels and the difficulty are kept in a part of the save that isn't decoded yet) |
| Gladiator Guild Manager | *Guild*: gold, wood, iron, stone, mana crystals, loan and the day (checked on a public save, not yet tried in the game) |
| God of War (2018) | *Kratos*: every save in `game.sav` (newest first, by title and date): XP, hacksilver, every resource and Yggdrasil's dew (a **Max crafting materials** button fills the crafting resources to 999, an estimate, leaving the story-locked Frozen and Chaos Flames alone), the equipment each save owns, and armour, pommels and talismans it doesn't have yet to add (at their base level) |
| God of War Ragnarök | *Kratos*: XP for Kratos, Atreus and Freya, hacksilver and divine ashes, resources (a **Max crafting materials** button fills the crafting resources to 999, an estimate), Yggdrasil's dew, health and rage, the level of every weapon, armour piece, attachment, shield and relic, and new armour, shields and attachments to add at any level (campaign saves; a piece of the same type you own is the model) |
| Grand Theft Auto III | *Player*: money (both copies the game keeps), hidden packages, and the statistics (progress, missions, stunt jumps, days, times arrested and died, kills); the save's checksum is rewritten (checked on public saves, not yet tried in the game) |
| Grand Theft Auto: San Andreas | *Player*: money (both copies the game keeps), health and armor, fat, stamina, muscle, respect, sex appeal and story line; *Skills*: driving, flying, lung, bike, cycling, gambling and every weapon skill; *Time and statistics*; *Options*; *Weapons*: weapon and ammunition per slot; the save's checksum is rewritten (the 2005 PC release; checked on a public save, not yet tried in the game) |
| Grand Theft Auto: Vice City | *Player*: money (both copies the game keeps), hidden packages, and the statistics (progress, stunt jumps, days, times arrested and died, kills); the save's checksum is rewritten (checked on public saves, not yet tried in the game) |
| Grandia HD Remaster | *Party*: gold pieces, each character's level, experience, HP, SP and attributes, magic and weapon levels, equipment, the items in each pack and every learned skill (checked on a real save, not yet tried in the game) |
| Graveyard Keeper | *Keeper*: money (in silver; 100 = 1 gold) and health, with every other value of the save (technology points, inventory, world objects) in the Tree under the names the game uses. The typed value tree of the `.dat` is read and written again, keeping each value's type. Built from the published layout; not yet tried on a real save |
| Green Hell | *Survivor*: health, energy, hydration, the three nutrients, dirtiness, sanity, every skill and the statistics |
| Grim Dawn | *Character*: difficulty and highest difficulty unlocked, money, level, experience, unspent attribute, skill and devotion points, attributes, skill ranks, item stack sizes, and *Factions*: each faction's reputation (up to the 25000 cap) and whether it is unlocked. The save is decrypted, then encrypted again with fresh checksums. Not yet tried in the game |

</details>

<details>
<summary><strong>H</strong> (19)</summary>

| Game | Screen |
|---|---|
| Hades, Hades II | Main currencies; runs, play time and last death; all resources by kind (currencies, plants, ores, fish, reagents) |
| Half-Life / Opposing Force / Blue Shift | *Save*: difficulty (Easy, Medium, Hard); *Player*: health, armour and flashlight; spare ammo by type; each carried weapon's magazine, with the weapon in hand marked. Read from the current map's state and changed in place |
| Half-Life 2 / Episode One / Episode Two | *Player*: health, armour and suit power; spare ammo by type (pulse rounds, SMG grenades, energy balls and the rest); each carried weapon's two magazines, with the weapon in hand marked. Changed in place |
| Half-Life: Alyx | *Player*: health and resin (a **Max crafting materials** button fills resin to 999, an estimate); pistol, shotgun and rapidfire ammo in the backpack; the weapons Alyx has. Changed in place |
| Half-Life: Source | *Player*: health, armour, spare ammo by type and each carried weapon's magazine. Changed in place |
| Hearts of Iron IV | *Empire*: political power, stability, war support, command power and experience. Binary saves (the game's default, and ironman) are edited in place and stay binary; fields outside this screen show as numbers (checked on public saves, not yet tried in the game) |
| Heroes of Might and Magic III | *Heroes*: every player's gold and resources, and every hero's level, experience, attack, defense, spell power, knowledge, spell points, movement, army (creature and count) and the skills they already have, from a `.GM1`, `.CGM` or `.GM2` save (Shadow of Death, Armageddon's Blade and Horn of the Abyss 1.8+; the game's gzip packing is opened and closed for you; checked on public saves, not yet tried in the game) |
| High on Life | Pesos. Every other inventory count and story flag is in the Tree. Checked on public saves, not yet tried in the game |
| Hogwarts Legacy | *Character*: gold, experience, talent points, every resource in the bag (with a **Max crafting materials** button that fills ingredients, beast by-products and Moonstone to 999, an estimate) and talent levels, from the game's own database inside the save (unpacked and packed again when you save); if the database can't be opened, the load-screen name, house and level |
| Hollow Knight | Geo, masks, notches, abilities and charms; spell levels, nail arts, Steel Soul mode, Kingsoul state and stag stations (those this save lists); a *Max upgrade materials* button (all 6 Pale Ore) |
| Hollow Knight: Silksong | Rosaries, shell shards, masks, silk, needle, abilities, crests, tools and items; decrypted and encrypted again on save. Not yet tried in the game |
| HoloCure - Save the Fans! | *Fans*: HoloCoins and the level of every upgrade (ATK, HP, SPD, crit, pickup range, haste, damage resistance, regen, EXP, money gain, rerolls and the rest), from `save_n.dat` (base64 JSON, unpacked and packed again; checked on a public save, not yet tried in the game) |
| Horizon Chase Turbo | *Driver*: campaign tokens and race counts, each stored in three places and all shown (checked on a public save from the Epic Games Store build, not yet tried in the game) |
| Horizon Forbidden West | *Aloy*: metal shards, skill points and every named resource (a Max crafting materials button raises common and uncommon resources to 200, rare to 35 and very rare to 15; legendary resources, food, ammunition and key items are left alone), with level and experience (PC Complete Edition; checked on public saves, not yet tried in the game) |
| Horizon Zero Dawn | *Aloy*: metal shards, skill points and every named resource, with level and experience (PC Complete Edition; checked on public saves, not yet tried in the game) |
| Hotline Miami | *Progress*: overall score, masks, levels and weapons unlocked, the hidden puzzle's letters, and each level's rank and high score in `SaveData.sav` (checked on a public save, not yet tried in the game) |
| House Flipper | *Flipper*: money (both copies the game keeps are written together), unused perk points of each skill and the statistics, from `PlayerProgress.dat` (plain JSON; checked on a public save, not yet tried in the game) |
| Hunting Simulator 2 | Credits, shots fired, animals spotted, tracks inspected and time played. Checked on a public save, not yet tried in the game |
| Hyper Light Drifter | Opens as JSON from `HyperLight_RecordOfTheDrifter_N.sav` (the hex header and base64 wrapper are removed and put back; every value by name; checked on a public save, not yet tried in the game) |

</details>

<details>
<summary><strong>I</strong> (3)</summary>

| Game | Screen |
|---|---|
| Icewind Dale: Enhanced Edition | Party gold and reputation; each party member's experience, hit points, levels, abilities, alignment and item stacks or charges (`BALDUR.gam`, the Baldur's Gate: EE layout); every global variable by name (quest stages, who has joined). No Icewind Dale save has been tried yet |
| Inscryption | *Cabin*: act 1 currency and lives, act 2 gold and packs opened, play time, in `SaveFile.gwsave` (checked on a public save, not yet tried in the game) |
| Into the Breach | Which squads are unlocked and the totals the game keeps (games played, enemies killed, islands secured, pods, total score). The Lua profile is edited in place; checked on a public profile, not yet tried in the game |

</details>

<details>
<summary><strong>J</strong> (4)</summary>

| Game | Screen |
|---|---|
| Journey to the Savage Planet | *Stash*: the stash resources (carbon, silicon, aluminium, alien alloy) with a **Max crafting materials** button (9,999, an estimate), the bag's items and the completion percentage, from the plain Unreal save `PlayerSave_N.sav` (checked on a public save, not yet tried in the game) |
| Judgment | *Yagami*: money and HP; decrypted with the game's key and its checksum kept right, every other value in the tree (checked on 40 public saves, not yet tried in the game) |
| Jurassic World Evolution | *Islands*: the money a career keeps for each island, with the file's checksum written again; career, island and play time are shown (checked on a public career save, not yet tried in the game) |
| Jurassic World Evolution 2 | *Career*: opens as a career file with its career, island and play time shown; a career money table is edited as in the first game when the save has one (checked on a public sandbox save, not yet tried in the game) |

</details>

<details>
<summary><strong>K</strong> (12)</summary>

| Game | Screen |
|---|---|
| Kenshi | *Squad*: every member's attributes and skills (up to 100, with a button to max the whole squad), the health of each body part, blood, hunger, bleeding and lost or robotic limbs, and reviving the dead, unconscious or those in a coma. Open a squad's file from the save's `platoon` folder (yours is named after your faction, `Nameless_0.platoon` by default); `quick.save` opens on a *Faction* screen with the cats. Values are found by name in the file and changed in place (checked on public saves, not yet tried in the game) |
| Kerbal Space Program | *Space Center*: funds, science, reputation, the level of each Space Center facility, and a table of every Kerbal (courage, stupidity, veteran, extra XP, trait, state). The `persistent.sfs` text is read as a tree and only the values you change are rewritten (checked on real career saves, not yet tried in the game) |
| Kerbal Space Program | *Space program*: career funds, science and reputation, from the save's ConfigNode text (only the values you change are rewritten; every other value, vessel and Kerbal is in the Tree). Checked on public sample saves, not yet tried in the game |
| King Arthur: Knight's Tale | *Camelot*: gold, relic dust, soul, spire shards and every hero's level, experience and skill points; the save container is opened and packed again (checked on public saves, not yet tried in the game) |
| Kingdom Hearts Final Mix (1.5+2.5 ReMIX, PC) | *Sora*: munny, difficulty, magic levels, the ten party members' level, experience, HP, MP, AP, strength, defense and weapon, and the stock of every item, for each save slot in `KHFM_WW.png` (checked on a real save, not yet tried in the game) |
| Kingdom Hearts II Final Mix (1.5+2.5 ReMIX, PC) | *Sora*: munny, difficulty, magic levels, each party member's level, experience, HP, MP and bonuses, and the stock of every item, for each save slot in `KHIIFM_WW.png`; both checksums are written again (checked on a real save, not yet tried in the game) |
| Kingdom Hearts III (PC) | *Sora*: decrypted (the key comes from the Steam id in the save's folder, or Epic's) and encrypted again with its CRC and MD5; munny (with the earned and spent totals kept in step), level, EXP, the bonus stats, Sora, Donald and Goofy's HP and MP, every synthesis material with a *Max crafting materials* button, and the items already held. Checked against the reference editor's test vectors; not yet tried on a real save |
| Kingdom Rush | *Kingdom*: gems, the difficulty, the upgrade ranks (archers, barracks, mages, engineers, rain of fire, reinforcements) and each hero's experience, from the plain Lua text `slot_N.lua` (checked on a public save, not yet tried in the game) |
| Kingdom Rush 5: Alliance TD | *Alliance*: gems, the difficulty, the upgrade ranks, the consumables and each hero's experience, from the plain Lua text `slot_N.lua` (checked on a public save, not yet tried in the game) |
| Kingdom Rush Frontiers | *Kingdom*: gems, the difficulty, the upgrade ranks and each hero's experience, from the plain Lua text `slot_N.lua` (the same screen as Kingdom Rush; checked on a public save, not yet tried in the game) |
| Kingdom Rush Origins | *Forest*: gems, the difficulty, the upgrade ranks and each hero's experience, from the plain Lua text `slot_N.lua` (checked on a public save, not yet tried in the game) |
| Kingdom Rush Vengeance | *Dark army*: gems and the stars on each campaign level, from the property-list XML `slot_N.plist` (checked on a public save, not yet tried in the game) |

</details>

<details>
<summary><strong>L</strong> (31)</summary>

| Game | Screen |
|---|---|
| Last Epoch | *Character*: level, experience, deaths, respecs, class id and the hardcore and masochist flags of an offline character (the `EPOCH` header is kept); items, passives and the monolith are in the Tree. checked on a public save, not yet tried in the game |
| Legends of Kingdom Rush | *Heroes*: each hero's experience and unlocked flag, and the gems, stars, food, loot, fame and guild experience of a run in progress (checked on a public save, not yet tried in the game) |
| LEGO Batman 2: DC Super Heroes | *Studs*: the stud wallet (the save has no checksum). Checked on public saves, not yet tried in the game |
| LEGO Batman 3: Beyond Gotham | *Studs*: the stud wallet (the game keeps it in several places in the save; all of them are written). Checked on public saves, not yet tried in the game |
| LEGO Batman: Legacy of the Dark Knight | *Studs*: the studs wallet (every field the game keeps it in is written together) and play time; the save encryption is handled for you, collectibles are in the Tree (not yet tried in the game) |
| LEGO Batman: The Videogame | *Studs*: the stud counter (the save's checksum is rewritten). Checked on public saves, not yet tried in the game |
| LEGO City Undercover | *Studs*: the stud wallet (the save has no checksum). Checked on public saves, not yet tried in the game |
| LEGO DC Super-Villains | *Studs*: the stud wallet (the game keeps it in several places in the save; all of them are written). Checked on public saves, not yet tried in the game |
| LEGO Harry Potter: Years 1-4 | *Studs*: the stud wallet (the save has no checksum). Checked on public saves, not yet tried in the game |
| LEGO Harry Potter: Years 5-7 | *Studs*: the stud wallet (the save has no checksum). Checked on public saves, not yet tried in the game |
| LEGO Indiana Jones 2: The Adventure Continues | *Studs*: the stud counter (the save's checksum is rewritten). Checked on public saves, not yet tried in the game |
| LEGO Indiana Jones: The Original Adventures | *Studs*: the stud counter (the save's checksum is rewritten). Checked on public saves, not yet tried in the game |
| LEGO Jurassic World | *Studs*: the stud wallet (the game keeps it in several places in the save; all of them are written). Checked on public saves, not yet tried in the game |
| LEGO Marvel Super Heroes | *Studs*: the stud wallet (the save has no checksum). Checked on public saves, not yet tried in the game |
| LEGO Marvel Super Heroes 2 | *Studs*: the stud wallet (the game keeps it in several places in the save; all of them are written). Checked on public saves, not yet tried in the game |
| LEGO Pirates of the Caribbean | *Studs*: the stud wallet (the save keeps five copies of it; all are written). Checked on public saves, not yet tried in the game |
| LEGO Star Wars II: The Original Trilogy | *Studs*: the stud counter (`savegame.bin`; the checksum is rewritten). Checked on public saves, not yet tried in the game |
| LEGO Star Wars III: The Clone Wars | *Studs*: the stud wallet (the save has no checksum). Checked on public saves, not yet tried in the game |
| LEGO Star Wars: The Complete Saga | *Studs*: the stud counter (the save's checksum is rewritten). Checked on public saves, not yet tried in the game |
| LEGO Star Wars: The Force Awakens | *Studs*: the stud wallet (the game keeps it in several places in the save; all of them are written). Checked on public saves, not yet tried in the game |
| LEGO Star Wars: The Skywalker Saga | *Studs*: the stud wallet, from `COLLECTABLES.BLOB` in a `SLOTn` folder (the game saves it twice; both copies are written). Checked on a public 100% save, not yet tried in the game |
| LEGO Star Wars: The Video Game | *Studs*: the stud counter (`LSW.BIN`; the checksum and the copy in `CODE.BIN` are rewritten). Checked on public saves, not yet tried in the game |
| LEGO The Hobbit | *Studs*: the stud wallet (the game keeps it in several places in the save; all of them are written). Checked on public saves, not yet tried in the game |
| LEGO The Incredibles | *Studs*: the stud wallet (the game keeps it in several places in the save; all of them are written). Checked on public saves, not yet tried in the game |
| LEGO The Lord of the Rings | *Studs*: the stud wallet (the save has no checksum). Checked on public saves, not yet tried in the game |
| Lethal Company | Credits, the profit quota and what is paid towards it, quotas passed, the deadline, the current moon (by name), ship items and suits owned (add or remove), every piece of scrap's sell value, ship upgrades in storage and statistics (`LCSaveFile1`–`3`); experience and rank (`LCGeneralSaveData`); decrypted and encrypted again on save (checked on public saves, not yet tried in the game) |
| Lies of P | *Pinocchio*: Ergo, level, humanity, the six stats and every item's count, with a **Max crafting materials** button that raises the Moonstones for normal and special weapons and Legion Calibers you have to 99 (an estimate), leaving quest items and consumables alone (checked on a public save, not yet tried in the game) |
| Like a Dragon Gaiden | Money on the Yakuza screen; decrypted with the game's key and checksum kept right, every other value in the Tree (read, edited and written back on a public save, not yet tried in the game) |
| Like a Dragon: Ishin! | Food, seafood and crafting materials (to 999, with a Max crafting materials button), virtue, tokens and tickets, ammunition, style experience and training orbs; decrypted, checksum kept right. Ryo isn't mapped yet (checked on a public save, not yet tried in the game) |
| Lost Judgment, Infinite Wealth, Pirate Yakuza (Dragon Engine) | Decrypted with each game's key and checksum kept right (keys from rgg_save_tool_py); values in the tree, and the Yakuza screen where the save has the same layout. Not tried on sample saves of these games yet |
| Luck be a Landlord | *Run*: coins, removal and reroll tokens, spins, the floor, the rent due and the run's progress from `LBAL.save` (the JSON lines are opened as one object keyed by each node's path and written back line by line; checked on a public save, not yet tried in the game) |

</details>

<details>
<summary><strong>M</strong> (22)</summary>

| Game | Screen |
|---|---|
| Mad Max | *Road warrior*: scrap, from `GameSave01.sav` to `GameSave10.sav` (the scrambling is undone and redone and the check value kept valid; checked on a public save, not yet tried in the game; switch Steam Cloud off for the game) |
| Magicraft | *Wizard*: magic crystals, ancient blood, chaos cores, the permanent upgrade levels, deaths and play time, from `WorldData0.Json` (its XOR key is removed and put back; checked on a public save, not yet tried in the game) |
| Maneater | *Shark*: shark level, play time and story progress, from `_<Steam id>_maneater_save_0.sav` to `_2.sav` (checked on a public save, not yet tried in the game) |
| Mass Effect (2008) and Legendary Edition ME1 | *Shepard*: level, experience, talent points and power ranks; credits, medi-gel, omni-gel and grenades; each squad member's level, talent points and talents. Legendary Edition saves also show origin, reputation, difficulty and the inventory, and a *Story* panel of about 100 named story flags and numbers to change (who is in the party, Wrex, the Council, romances, mission outcomes) |
| Mass Effect 2, Mass Effect 3 (original and Legendary Edition) | *Shepard*: level, experience, talent points, difficulty; credits, medi-gel, element zero, iridium, palladium, platinum, probes, fuel, grenades; power ranks; spare ammo; each squad member's level, talent points and powers; Paragon and Renegade points (Mass Effect 3: reputation too) and a *Story* panel of named story flags to switch on or off (who is loyal, alive or romanced, the choices imported from the earlier games, research upgrades; over 500 in Mass Effect 2 and about 400 in Mass Effect 3), checked on public sample saves. The save's checksum is updated |
| Mass Effect: Andromeda | Credits, Ryder's level and skill points, each squad member's level and skill points; research data (Milky Way, Heleus, Remnant), minerals, salvage, biological and faction materials, augmentations and consumables in the inventory, with a **Max crafting materials** button that fills the minerals, biological and faction materials to 999, the game's resource limit; location, completion and play time. Both checksums are written again on save. Not yet tried in the game |
| Master of Orion II | *Empires*: each empire's treasury (BC), research points and ship range, and each colony's stored production, marines and tanks from `SAVEn.GAM` (checked on public saves, not yet tried in the game) |
| Medieval Dynasty | *Settler*: health, stamina, food and water, reputation, tax, the four technology point pools, the six skill levels and every stack in the bag (money is the Coin item), with a **Max crafting materials** button that raises the plain building and crafting materials to 999 (an estimate), leaving money, tools, food and clothing alone (checked on a public save, not yet tried in the game) |
| Metal Gear Solid V: The Phantom Pain | GMP (and the FOB wallet), heroism and demon points; every Mother Base staff member (rank, skill, section, morale, proficiency, stat bonus) with buttons to promote everyone to S++ and max morale; resource stockpiles with a Max crafting materials button (materials to 1,000,000 and plants to 30,000 online, the game's caps); a research and development list (every weapon, suit, item and support gear entry by development number, with developed and available switches and a Develop everything available button that leaves out DLC, enemy and unused entries, blueprint-gated items and story-locked lines); a button to open every Mother Base section; and the Mother Base and lifetime-statistics counters; the Mother Base and FOB platform grades with a finish-everything button. Every other variable is in the Tree under the game's own variable names; encryption and checksum are redone on save. Ground Zeroes saves (MGSV_GZ_Save_Data_GL_EN) are decrypted for the hex view, with the checksum redone on save. |
| Metro 2033 / Last Light Redux, Exodus | *Artyom*: military-grade rounds, every ammo type and (Exodus) crafting metal and chemicals in the level saves (`.player`), with a **Max crafting materials** button that fills metal and chemicals to 300 (the Reader and Ranger Hardcore limit; Normal allows more); auto and quick saves are unpacked (LZ4) for Value search. CRC-32 updated |
| Mewgenics | *House*: house gold and food, adventure coins and food, the current day and the completion percentage, from the SQLite save's properties table (the cats' own packed data is kept as it is; checked on a public save, not yet tried in the game) |
| Minecraft (`level.dat`) | Health, food and XP; game mode; difficulty (Peaceful to Hard); world settings; inventory and ender chest grids; a **Max crafting materials** button that fills the ingots, raw ores, gems, string, leather and other materials already carried to 64, leaving ender pearls, eyes of ender and other progression items alone |
| Minecraft Dungeons | *Hero*: the character `.dat` is decrypted (AES, the key as published by MCDSaveEdit) and encrypted again on save; experience, total gear power and the emerald, gold and Eye of Ender counts, with items and enchantments in the Tree. The decryption was checked on a public sample save; not yet tried in the game |
| Mon Bazou | *Driver*: cash, the player's meters and the hard mode and permadeath flags, from `marguerite.txt` (.NET BinaryFormatter, read and written in place; the vehicle, parts and garden files open for Tree editing; checked on a public save, not yet tried in the game) |
| Monster Train 2 | The Covenant level you play at, the highest Covenant unlocked and the story progress. Plain JSON; checked on a public save, not yet tried in the game |
| Moonlighter | *Will*: gold, days passed and the banker's invested money; the backpack and equipped items by name with count (enchantment levels in the Tree). The `gameslot` file opens as JSON (checked on a public save, not yet tried in the game) |
| Mortal Shell | *Fallgrim*: Tar, the shell points of each shell, max health, deaths, New Game+ level, and the Storm Mode run's Tar and Glimpses in `GPData_<slot>.sav` (inventory item names are stored as game text and aren't shown; checked on public saves, not yet tried in the game) |
| Mortal Shell II | *Harbinger*: Gold, Gloom and Glimpses, their lifetime totals, every item's count and every unlocked shell, weapon and seal in `WorldState_<n>.sav`, with a **Max crafting materials** button that raises the Tarforge smithing materials (Ventrium, Laterite, Dorsalite, Thoracium) you have to 99 (an estimate), leaving keys, artifacts and upgrade items alone (checked on public saves, not yet tried in the game) |
| Mutant Year Zero: Road to Eden | *Squad*: scrap, weapon parts and medpacks, the Ark's artifact points, population and levels, the squad level, and each squad member's level, XP, health, sanity and mutation points, with a **Max crafting materials** button that raises scrap and weapon parts to 999 (an estimate), leaving artifacts and gear alone (checked on a public save, not yet tried in the game) |
| My Hero One's Justice | *Heroes*: cash, from the plain Unreal save `systemdata.sav` (checked on a public save, not yet tried in the game) |
| My Hero One's Justice 2 | *Heroes*: hero coins, from the plain Unreal save `systemdata*.sav` (checked on a public save, not yet tried in the game) |
| My Summer Car | *Finland*: money and the needs (hunger, thirst, fatigue, stress, urine, dirtiness); every other single value of `defaultES2File.txt` (Easy Save 2) is in the Tree by name (checked on a public save, not yet tried in the game) |

</details>

<details>
<summary><strong>N</strong> (7)</summary>

| Game | Screen |
|---|---|
| Necromunda: Hired Gun | *Hired Gun*: credits, level, experience and each faction's reputation experience (checked on a public save, not yet tried in the game) |
| Need for Speed: Most Wanted (2005) | *Driver*: money and pursuit bounty of a profile file (63,596 bytes on PC, 62,689 on PlayStation 2), with the file's MD5 made again on save. Opens by its layout, wherever the file is kept; built from the published offsets, not yet tried on a real profile |
| Neverwinter Nights: Enhanced Edition | *Character*: gold, experience, ability scores, hit points, alignment, skills and item stack sizes of the player (the module .sav, with player.bic kept in step) or a vault character (.bic). Not yet tried in the game |
| NieR Replicant ver.1.22474487139... | *Slots*: health, level, experience, money, recovery items and materials with Max crafting materials (99 each), from `GAMEDATA` (checked on a public save, not yet tried in the game) |
| NieR:Automata | Money, experience (with the level it gives), item and material counts (99 each), weapon levels (1–4) and a *Max crafting materials* button for the Materials list; no checksum (checked on a real save, not yet tried in the game) |
| No Man's Sky | *Currencies*: units, nanites and quicksilver. *Exosuit inventory*, *Exosuit cargo*, *Freighter inventory* and *Storage container 1* and *2*: the amount in every stack, with a **Max crafting materials** button per container that raises the raw materials you hold (elements, refined metals, plants; story items, salvage and junk are left alone) to the limit stored with each stack. The save's LZ4 chunks are unpacked with the scrambled key names shown, and packed again; the `mf_` file beside it is made to agree (checked on a public save, not yet tried in the game) |
| Noita | *Player*: gold, gold spent, health and maximum health (the game shows 25 times the stored value) from `player.xml`; wands, spells and perks are in the Tree. checked on a public save, not yet tried in the game |

</details>

<details>
<summary><strong>O</strong> (12)</summary>

| Game | Screen |
|---|---|
| Octopath Traveler, Octopath Traveler II | Money (leaves) and the eight travellers' level, EXP, JP and current HP/SP, changed in place. Read from a sample save; not yet tried in the game |
| Older LEGO games (TT Games: Harry Potter Collection, Marvel's Avengers, Worlds) | The saves are found for you (Find my game's saves); they open in the Hex view for Value search. No dedicated screen yet |
| OMORI | *OMORI*: Clams, each friend's level, heart and juice in Headspace and Faraway Town, and every snack, toy, weapon and charm by name, in `file1.rpgsave`… (`www\save` in the game folder) (checked on public saves, not yet tried in the game) |
| One Piece: Pirate Warriors 3 | *Characters*: each of the 47 character slots' health, attack, defence, special bars and skill slots (a level-up may reset the stats); the Beli is shown but not editable. The encrypted save is opened and its checksum kept right (checked on a public save, not yet tried in the game) |
| One Piece: Pirate Warriors 4 | *Resources*: Beli, and the quantity of every coin you own (by coin id). The encrypted slot is opened and its object checksums kept right (checked on a public save, not yet tried in the game) |
| Onimusha: Way of the Sword | Each save slot (place, play time, difficulty, objective); red souls, health and Oni energy; items by name (on hand and in storage) and every charm of the game (charm level, worn, and **Add** for the ones you don't have yet), with **Add material** to give a slot an upgrade material it doesn't have yet, and a **Max crafting materials** button that fills the upgrade materials on hand in the slot you're viewing (cloths, leather, whetstones, iron, Tamahagane, Malicite, Byakue's Talisman, Dohatsu-ten's Horn) to 999, adding any that are missing, and leaving Power Stones, Oni Stones, Oni Cloth and key items alone; an **Appearances** panel that unlocks the base-game sword looks, outfits and haori coats for every slot (looks that come with paid editions are left alone); equipment experience. Decrypts Capcom's RE Engine encryption with your Steam account, found automatically |
| Onimusha: Way of the Sword | Each save slot (place, play time, difficulty, objective); red souls, health and Oni energy; items by name (on hand and in storage) and every charm of the game (charm level, worn, and **Add** for the ones you don't have yet), with **Add material** to give a slot an upgrade material it doesn't have yet, and a **Max crafting materials** button that fills the upgrade materials on hand (cloths, leather, whetstones, iron, Tamahagane, Malicite, Byakue's Talisman, Dohatsu-ten's Horn) to 999, adding any that are missing, and leaving Power Stones, Oni Stones, Oni Cloth and key items alone; an **Appearances** panel that unlocks the base-game sword looks, outfits and haori coats for every slot (looks that come with paid editions are left alone); equipment experience. Decrypts Capcom's RE Engine encryption with your Steam account, found automatically |
| OpenXcom (X-COM: UFO Defense) | *Base*: funds, months and days passed, difficulty, scientists and engineers, every soldier's rank, missions, kills and stats, and the stores of the first three bases, from an OpenXcom or OpenXcom Extended `.sav` (the data part of the file is rewritten on save; checked on a public save, not yet tried in the game) |
| Ori and the Blind Forest | *Ori*: level, experience, ability points, life and energy cells, keystones, mapstones and deaths, and every skill and ability-tree upgrade to tick on or off, in Definitive Edition and original saves (`saveFile0.sav`…); the load menu's copy of life and energy is kept in step (checked on a public save, not yet tried in the game) |
| Outer Wilds | *Time loop*: loop counters, marshmallows eaten, the launch codes and the Eye of the Universe warp in `data.owsave` (checked on a real save, not yet tried in the game) |
| Outward | *Adventurer*: silver, health, stamina, mana, food, drink, sleep and the burnt (reduced maximum) health, stamina and mana, from `CharN.charc` (the gzip-compressed XML is unpacked and packed again; checked on a public save, not yet tried in the game) |
| Oxygen Not Included | *Colony*: every duplicant's attribute levels and experience, total experience and learned skills; a *Map* of every asteroid and rocket interior (elements, temperature and germ views; paint elements with mass and temperature, dig out to vacuum, set temperature or mass, clear germs, reveal the fog; geysers, teleporters, Gravitas finds, story traits, artifacts and the Printing Pod as toggleable markers); *Worlds & rockets*: names, discovered, sunlight and cosmic radiation, and what's on the star map |

</details>

<details>
<summary><strong>P</strong> (24)</summary>

| Game | Screen |
|---|---|
| Palworld | *Player*: technology points, ancient technology points, Lifmunk effigies and the record counters |
| Pathfinder: Kingmaker, Wrath of the Righteous | *Party*: money; the player and each companion's ability scores, experience, classes and levels, and base hit points (Wrath: mythic rank too). Kingmaker: a *Story flags* list of the save's named story and unlock flags (romances, kingdom buildings, quest choices) |
| Pathway | *Expedition*: cash, fuel, ammunition, supplies and the day of the expedition (checked on a public save, not yet tried in the game) |
| PAYDAY 2 | *Crew*: spending cash, offshore account, level, infamy rank and points, skill points and Continental coins, from `saveNNN.sav` (the XOR encryption, block checksums and closing hash are handled for you; checked on a public save, not yet tried in the game) |
| PC Building Simulator | *Builder*: cash, kudos and hard mode, from the career `.binary` save (.NET BinaryFormatter, read and written in place; checked on public saves, not yet tried in the game) |
| Persona 3 Portable (PC) | *Protagonist*: yen, level, experience and the three social stats; every Persona slot in use with its level, experience and five stats; the count of every item that can be held (key items left out, the game needs a flag for them); the matching `.BINslot` checksum is updated. Checked on a synthetic save, not yet tried in the game |
| Persona 3 Reload | *Protagonist*: yen, every party member's level, experience, HP and SP, the social stats and each Social Link rank; Steam saves are decrypted and re-encrypted for you (checked on public sample saves, not yet tried in the game) |
| Persona 4 Golden (PC) | *Protagonist*: yen, level, experience and the five social stats; the matching `.binslot` checksum is updated |
| Persona 5 Royal (PC) | *Phantom Thieves*: decrypted, unpacked and packed again; money, HP / SP, levels, experience and Persona stats for the party and Joker's Personas |
| Persona 5 Strikers | *Save slots*: money, Persona points, unspent BOND points, and the stacks you hold of consumables, cooking ingredients, incenses, ailment remedies and skill cards. The encrypted `SAVEDATA.BIN` is opened and its checksum kept right (checked on a public save, not yet tried in the game) |
| Phasmophobia | Money, level, prestige and experience, how many of each item you own, item tiers and their unlocks, and the journal's statistics (`SaveFile.txt`); decrypted and encrypted again on save (checked on public test saves, not yet tried in the game) |
| Phoenix Wright: Ace Attorney Trilogy | *Episodes*: how many episodes of each of the three games are unlocked on the episode select screen, the volumes and the window size (checked on a real save, not yet tried in the game) |
| Pizza Tower | *Tower*: completion %, enemies killed and play time; every level's score, rank (D to P), treasure and secrets found, and the four boss scores and ranks. checked on a public save, not yet tried in the game |
| Planescape: Torment: Enhanced Edition | Coppers; the Nameless One and each companion: experience, level, hit points, abilities, alignment and item stacks; every global variable by name (quest stages, story choices) (checked on real saves, not yet tried in the game) |
| Planetbase | *Colony*: coins, extra prestige, landing permissions, and the health, food, water, oxygen, sleep and morale of every colonist and the condition of every bot (plain XML; checked on public saves, not yet tried in the game) |
| Plants vs. Zombies (Game of the Year Edition) | *Profile*: money (stored divided by 10), Adventure level and completions, Crazy Dave's shop (the nine plants, tools, cleaners), fertilizer, bug spray, chocolate and tree food, seed slots, Tree of Wisdom, Survival progress; the Zen Garden's plants; mini-game and puzzle trophies on the *Values* tab. Open `user1.dat` (checked on a public save, not yet tried in the game) |
| Pokémon (Gen IV–IX: Diamond/Pearl/Platinum/HeartGold/SoulSilver, Black/White and Black 2/White 2, X/Y, Omega Ruby/Alpha Sapphire, Sun/Moon and Ultra Sun/Ultra Moon, Sword/Shield, Scarlet/Violet) | Trainer, money and the game's other currencies, badges, play time, bag pockets (Scarlet/Violet lists the items you own); party cards and every PC box with species, nickname, level (sets experience and stats), nature, shiny, ability, held item, ball, moves with PP and PP Ups, IVs and EVs. Pokémon encryption and the save checksums or seals are handled for you |
| Pokémon (Gen I–III) | Trainer card and badges (Gen III: each Gym badge switchable); party cards (species, level, nickname, moves and PP, HP, IVs and more); PC boxes as a grid; bag pockets |
| Pokémon Infinite Fusion | *Trainer*: money, coins, Battle Points, badges and the difficulty (Easy, Normal, Hard); every Pokémon in the party and the PC boxes, fusions shown with their head and body, with level and Exp. Points kept in step, nature (from the game's 25), shiny, happiness, IVs and EVs, and the stats worked out again with the game's own formula; how many of each item is in the bag (up to 999). *Max money* and *Max IVs (party)* buttons. The `File A.rxdata` saves in `%APPDATA%\infinitefusion` are plain Ruby Marshal and are edited in place (checked on real saves from game versions 5 and 6, not yet tried in the game) |
| Police Simulator: Patrol Officers | *Career*: play time, shifts, encounters and each district's experience and duty stars, from `Career_0.sav` (plain Unreal GVAS; checked on a public save, not yet tried in the game) |
| Potion Craft: Alchemist Simulator | *Alchemist*: gold, popularity, karma, experience and the inventory (ingredients and potions) with Max crafting materials for the plant, mushroom and crystal ingredients, from `.pcsave` (decoded to JSON and encoded again; checked on a public save, not yet tried in the game; the cap is an estimate) |
| Pragmata | *Progress*: the difficulty rank points, New Game+ count and game time; every item and currency by the game's own code with its quantity, each weapon's level and unlock, and the perks (the save is decrypted with your Steam account; checked on real saves, not yet tried in the game) |
| Prison Architect | *Prison*: the bank balance, play time and the main prison settings, changed in place in the large text save (checked on public saves, not yet tried in the game) |
| Project Warlock | *Warlock*: score, lives, level, health, mana, the four stats, points to spend and every weapon's ammunition (checked on a public save, not yet tried in the game) |

</details>

<details>
<summary><strong>R</strong> (17)</summary>

| Game | Screen |
|---|---|
| R.E.P.O. | Level, money, lives, the charging station and total haul; each player's health and upgrades; the items in the truck and bought in total (`<save>.es3`); decrypted and encrypted again on save (checked on a public save, not yet tried in the game) |
| Raft | *World*: the day, time played, time of day, weather and radio tower flag of a world `.rgd`, the game mode (Normal, Easy, Peaceful, Hardcore), the amount of every stack in every storage box with a Max crafting materials button, and the learned and researched items. A second *Player* screen opens `RGD_Users.rgd` (in the `Player` folder): health, hunger and thirst, and the hotbar and inventory by item name in every world, with a **Max crafting materials** button for the plain materials you carry (planks, palm leaves, scrap, plastic, rope, stone and the like, to the stack size of 20; tools, food and blueprints are left alone). .NET BinaryFormatter, read and written in place; the rest opens for Tree editing; the player screen was checked on a real save, not yet tried in the game |
| Ready or Not | *Operations*: the best rating, best time and times completed of every mission, from `LevelStats.sav` (plain Unreal GVAS; the roster and unlock tags open for Tree editing; checked on a public save, not yet tried in the game) |
| Red Dead Redemption (PC; the PS4 and Switch release uses the same format) | *John*: money, honor, fame and the Dead Eye meter; *Consumables*: snake oil, moonshine, chewing tobacco, medicine, tonics, bait, apples and horse pills; weapons, horses and kit are listed as owned. Story, Hardcore and Undead Nightmare saves; the save is decrypted and encrypted again with its CRC-32 (checked on real saves, not yet tried in the game) |
| Red Dead Redemption 2 (PC, story mode) | *Arthur*: cash, honor (its saved statistic kept in step), the health, stamina and Dead Eye cores and XP, weight and saved health; *Camp*: camp funds; the satchel's ammunition, consumables, herbs and provisions, with a Max crafting materials button for herbs, feathers and animal fat (99, the Legend of the East satchel's stack size); horses' dirt and hunger; 17 statistics; the clock; bounties are shown. Supplies the satchel lacks can be added (experimental: the save grows). *Challenges*: the rank (0 to 10) of the nine story challenges. *Compendium*: studied/discovered ticks for animals, plants, fish and horses, and owned ticks for weapons and equipment. Every checksum is written again and the save encrypted (checked on real saves, not yet tried in the game) |
| Ren'Py visual novels | *Story*: the game's numbers, flags and text variables with a filter, money and the like up front |
| Resident Evil 5 | *BSAA*: money and exchange points (Steam `savedata.bin`); the save is decrypted and its checksum made again for you (checked on a public save, not yet tried in the game) |
| Resident Evil Requiem (and RE Engine games with the same layout) | *Slot*: health of the player and other characters, inventory counts (with the ammo or contents they hold), save count |
| RimWorld | *Colony*: every colonist's skills (level and passion, modded passions included) and every silver stack, plus steel, plasteel, components, gold, uranium, wood, cloth, medicine, chemfuel and the other stockpiled resources stack by stack, edited in place even in 100+ MB saves |
| Risk of Rain 2 | *Survivor*: Lunar coins, coins collected, runs played and play time of a profile `.xml`; unlocks, discovered items and the logbook are in the Tree. checked on a public profile, not yet tried in the game |
| Road Redemption | *Rider*: the permanent upgrade ranks (health, damage, armor, nitro, critical hits, XP and money gain, store prices, ammo, durability, jump jets), kills, wins and the XP carried over, from the plain XML `N_N.save` (checked on a public save, not yet tried in the game) |
| Roboquest | *Robot*: wrenches (and wrenches spent), the selected difficulty and class, the saved run's player level and the lifetime statistics (enemies destroyed, bosses taken down, damage dealt), from `RQProfile.sav` (checked on a public save, not yet tried in the game) |
| RollerCoaster Tycoon | *Park*: cash, bank loan, maximum loan and the entrance fee from a saved park (`.SV4`, including Added Attractions and Loopy Landscapes); the file's run-length packing is opened and closed for you (checked on public parks, not yet tried in the game) |
| RollerCoaster Tycoon 2 | *Park*: cash, bank loan, maximum loan and the entrance fee from a saved park (`.SV6`); the park's chunk is unpacked and packed again with the file's sum written right, and the game's scrambled cash is handled for you (checked on public parks, not yet tried in the game) |
| RPG Maker MV / MZ | *Party*: gold, steps and play time; each character's name, level, EXP, HP / MP / TP, stat bonuses and learned skills (by id); the party's items, weapons and armor by database id (change a count, remove one, add one) |
| RPG Maker XP games | *Party*: gold, steps, the party and every actor (level, EXP, HP, SP and stat bonuses), and the items, weapons and armour you carry by database number (checked on public To the Moon saves) |
| Rune Factory 4 Special (PC, extracted Switch) | *Farmer*: money, HP and RP, strength, intelligence and vitality, order points, tourists, and lumber, material stone and fertilizer with a *Max crafting materials* button; the save's CRC is written again. Not yet tried on a real save |

</details>

<details>
<summary><strong>S</strong> (39)</summary>

| Game | Screen |
|---|---|
| Saints Row IV | *President*: cash (the game keeps it in cents), data clusters, respect level and respect (`sr4save_N.sr4s_pc`); the save's CRC-32 is rewritten (checked on public saves, not yet tried in the game) |
| Saints Row: The Third (and Remastered) | *Boss*: cash and the crib stash (the game keeps them in cents), respect level and respect, stunt jumps, barnstorms and difficulty (`sr3save_NN.sr3s_pc`); the save's CRC-32 is rewritten (checked on public saves, not yet tried in the game) |
| Salt and Sanctuary | *Character*: level, salt, gold, kills, deaths, sanctuary rests, corruption and the playthrough number from `datN.slv` (items and the skill tree are not edited yet; checked on public saves, not yet tried in the game) |
| Samurai Warriors 2 | *Army*: money and the stored stats of each unlocked officer of the original PC release (the stat limits are estimates); the checksum is handled for you (checked on a public save, not yet tried in the game) |
| Samurai Warriors 4-II | *Army*: Gold, the five strategy tomes and every officer's Health, Attack, Defense, Riding and Speed bonuses; the cipher, section sums and checksum are handled for you (checked on a public save, not yet tried in the game) |
| Samurai Warriors 5 | *Resources*: gold, the Dojo's stock EXP, shared skill points and stock weapon mastery, and the castle's lumber, mineral and fabric (with *Max crafting materials*, 9,999 each); the save is decrypted with your Steam account number (it is found from the Steam accounts on your computer or the `steam_autocloud.vdf` beside the file) and encrypted again on save (checked on a public save, not yet tried in the game) |
| Satellite Reign | *Syndicate*: your funds and the researcher, scientist, doctor and kill counters (checked on a public save, not yet tried in the game) |
| Schedule I | One screen per save file: *Money* (bank balance, net worth, lifetime earnings, weekly deposits), *Rank* (rank, tier, XP), *Time* (days, time of day, play time) and *Game* (organisation name, seed, console). Cash you carry is in the player's own files and isn't covered (checked on a public sample save, not yet tried in the game) |
| Sea of Stars | *Party*: the party's level, total experience, gold and deaths, from the length-prefixed JSON `SOS_SaveSlotN.sos` (unpacked and packed again; checked on a public save, not yet tried in the game) |
| Sekiro | Sen, Attack Power, carried Spirit Emblems, skill points to spend, skill experience (points earned and total experience shown), Vitality and Posture; items, key items and storage as grids with an item picker and quantity; a *Max upgrade materials* button (the nine prosthetic materials to the 99 you can hold); skills and prosthetic tools |
| Shadow Warrior 2 | *Lo Wang*: level, experience, money, crystal points, perk points, health, mana, rage and every kind of ammunition; weapons and upgrades are in the Tree. Each `.bin` save is a zip whose text is opened and packed again for you (checked on a public save, not yet tried in the game) |
| Shape of Dreams | *Traveler*: stardust, spent stardust, completed reveries, preferred traveler and difficulty from `r_profile<id>.json`; mastery level, mastery points and the record of every traveler from `r_stats<id>.json` (the unlocked items are in an encrypted file that is not opened; checked on public saves, not yet tried in the game) |
| Single Player Tarkov (SPT) | *Character*: level, experience, skills, trader standing and loyalty, health, and the roubles, dollars and euros stacks in the stash; a plain JSON profile (checked on a public profile, not yet tried in the game) |
| Skyrim, Fallout 4 | Character header; game time; every statistic. Skyrim: the character's real level (in the character itself and the header) |
| Slay the Spire | Gold, HP, potion slots; the deck, relics and potions (change an id or upgrade level, add a card, relic or potion by its internal id, remove one); ascension level, floor, act, shop removal cost and the three Heart keys |
| Slay the Spire 2 | *Progress*: each character's highest and preferred Ascension, wins, losses, win streaks and play time, plus the profile's score, from `progress.save` (plain JSON). checked on a public save, not yet tried in the game |
| Sleeping Dogs: Definitive Edition | *Wei*: money and face, cop and triad XP (checked on public saves and a real save, not yet tried in the game) |
| Slime Rancher | *Ranch*: newbucks (kept equal on the load screen), health, energy, radiation and slime keys (checked on public saves, not yet tried in the game) |
| SnowRunner | *Garage*: money, rank, experience, the statistics (money earned and spent, trucks and add-ons bought and sold, sessions), the difficulty factors and the trucks in the warehouse. The save is JSON followed by a NUL byte, which is kept (checked on two real saves, not yet tried in the game) |
| Sonic Adventure 2 | *Chao* (the garden file `SONIC2B__ALF`): the level and the points of the seven stats of every Chao; the garden's checksum is rewritten (the main save with rings and emblems is not edited yet; checked on a public save, not yet tried in the game) |
| Sonic Mania | *Slots*: all eight Mania Mode and three Encore Mode slots (state, character, zone, lives, score, Chaos Emeralds, continues, next special stage), changed in place (checked on public saves, not yet tried in the game) |
| Sons of the Forest | *Game* (GameStateSaveData.json): the day and time. *Weather* (WeatherSystemSaveData.json): the season. *Player* (PlayerStateSaveData.json): health, stamina, hydration, rest, strength and its level. *Inventory* (PlayerInventorySaveData.json): the items carried, by name, with a **Max crafting materials** button that fills logs, sticks, rocks, bones, cloth, circuit boards, wire, duct tape, rope, feathers, leaves, small rocks, tech mesh, Solafite ore and printer resin to the game's own carrying limit for each, leaving weapons, ammo, food, blueprints and story items alone. The JSON the game keeps inside the file's strings is opened and written back as strings (checked on public sample saves, not yet tried in the game) |
| Space Haven | *Fleet*: credits, the day and time, the difficulty options, and a table of each ship's crew with health, food, rest, comfort, mood, the four attributes and every skill level. The save's XML is edited in place, so everything you don't change stays as it was. Resources in storage are not covered (checked on a real save, not yet tried in the game) |
| Star Wars Jedi: Fallen Order | *Cal Kestis*: unspent and total skill points, XP toward the next point, stims carried and capacity, health and Force upgrades and fragments, secrets and map sections, deaths and revives. The zlib-packed actor records are opened and packed again on save (checked on a public save, not yet tried in the game) |
| Star Wars Jedi: Survivor | *Cal Kestis* (the same screen as Fallen Order): unspent and total skill points, XP toward the next point, stims, health and Force upgrades, secrets and map sections, deaths and revives. The zlib-packed actor records are opened and packed again on save (checked on public saves, not yet tried in the game) |
| Star Wars: Knights of the Old Republic | *Party*: the player and party members (attributes, skills, HP and Force points, XP) and how many of each item the party carries, from SAVEGAME.sav; credits and party XP from PARTYTABLE.res; every story variable (the global flags and numbers: choices, quest steps) by the game's own name from GLOBALVARS.res. Not yet tried in the game |
| Star Wars: Knights of the Old Republic II | *Party*: the player and party members (attributes, skills, HP and Force points, XP) and how many of each item the party carries, from SAVEGAME.sav; every story variable (global flags and numbers) from GLOBALVARS.res; credits, party XP, influence, components and chemicals from PARTYTABLE.res, with a **Max crafting materials** button that fills components and chemicals to 9999 (an estimate). Not yet tried in the game |
| Starbound | Pixels and the other currencies; item counts in every bag, with a **Max crafting materials** button that fills ores, bars, fabric, leather, silk, plant fibre, wire, glass and other base-game materials to 1000, the usual stack size; health, energy and food; ship level, crew and fuel; deaths and play time (checked on a real character save, not yet tried in the game) |
| Stardew Valley | Gold, Qi Gems, casino coins, Golden Walnuts; health and energy; every villager's friendship points (shown as hearts, with a raise-everyone-to-8-hearts button); the whole backpack with stack sizes, item quality and tool levels; the six skills (level and XP, with the XP the next level needs); farm details; the crafting materials in the backpack (wood, stone, fiber, clay, ores, bars, monster loot…) with a **Max crafting materials** button that fills them to 999, leaving quest items, minerals and artifacts alone |
| Stellar Blade | *Eve*: health, shield, stamina, the Beta, Burst and Tachy gauges, level, experience and skill points, the play time and difficulty, and an inventory table with counts and levels; the save's CRC-32 is written again (checked on a real early-game save, not yet tried in the game) |
| Stellaris | *Empire*: energy, minerals, food, alloys, consumer goods, influence and unity up front, and every resource in the stockpile. Ironman saves are text like any other Stellaris save and open the same way (checked on saves with the ironman flag set; no save from a real ironman campaign has been tried yet); the game may stop awarding achievements for an edited campaign |
| Stoneshard | *Character*: level, experience, ability points, unspent attribute points and the six attributes of a character's `data.sav`, which is unpacked (zlib) and packed again with its MD5 (the salt comes from the save's folder names, so open it from its own folder). Built from the published layout; not yet tried on a real save |
| Stranded Deep | *Survivor*: health, food, water, sleep, the five skills and the day count (checked on a public save, not yet tried in the game) |
| Streets of Rogue | *Unlocks*: nuggets, wins, deaths and games played, and every agent, item, trait and floor with whether it is unlocked and what it costs, from `GameUnlocks.dat` (checked on a public unlocks file, not yet tried in the game) |
| Subnautica 2 | *Survival*: health, oxygen, food and water with their maximums, the player inventory and every locker, vehicle and storage container, changed in place in the save (checked on a real early-access save; needs the compression library of any installed game) |
| Subnautica, Below Zero | *Slot*: game mode (Survival / Freedom / Hardcore / Creative), play time, vehicles |
| Supermarket Simulator | Money, store level and points, the store upgrade and storage levels and the day. Easy Save 3 data; checked on a public save, not yet tried in the game |
| Supraland | *Hero*: coins, the coin limit, sword damage and range, progress points, deaths, kills, jumps, play time and completion, from the plain Unreal GVAS `.sav` (checked on public saves, not yet tried in the game) |
| Sword Art Online: Fatal Bullet | *Inventory*: the quantity of every weapon, costume, accessory, bullet and material, with item names, from `SaveData.sav` (encrypted blocks and the end-of-file HMAC handled; checked on a public save, not yet tried in the game) |

</details>

<details>
<summary><strong>T</strong> (45)</summary>

| Game | Screen |
|---|---|
| Tales of Symphonia | *Party*: gald (written in both places the game keeps it), grade points, the records, all nine characters (level, experience, HP, TP, strength, attack, defense, luck, accuracy, evasion, intelligence, overlimit, affection) and every item up to 20 (checked on a real save, not yet tried in the game) |
| Tales of Vesperia: Definitive Edition | *Party*: gald (also in the save list), play time; *Characters*: level, experience, HP, TP, attack, magic attack, defense, magic defense, agility and luck of all nine characters (checked on public saves, not yet tried in the game) |
| Tavern Master | *Tavern*: money, quest points, the tavern's name, and every employee's level and salary, from the plain JSON save (checked on public saves, not yet tried in the game) |
| Teardown | *Crew*: cash, the hub score, every tool's ammunition and its range, damage and size upgrades (savegame.xml); checked on a public save, not yet tried in the game |
| Terra Invicta | *Factions*: every faction's money, influence, operations, research, boost, mission control and space resources (checked on public saves, not yet tried in the game) |
| Terraria (characters) | Life and mana; permanent upgrades; inventory, coins and ammo, equipment (armor, vanity, dyes, pets) and storage (piggy bank, safe, forge, void vault) as item grids; a **Max crafting materials** button (1.4.4 and later) that fills the ores, bars, gems, woods, monster drops and other materials you already have to 9999, leaving critters, fish and boss summons alone |
| Terraria (worlds) | A map editor: pan and zoom the whole world, see what every tile is, points of interest (shimmer, Enchanted Sword shrines, dungeon biome chests and other dungeon chests, Jungle Temple chests, Plantera's Bulbs, Life Fruit, orbs and hearts, altars, larvae, the Jungle Temple, hives, marble and granite caves, spider nests, glowing mushrooms, meteorites; each kind and every marker can be hidden), paint blocks, walls and liquids, erase them, and move the spawn point (chests and furniture are left alone; each stroke is one undo step). Also the world settings (name, difficulty, evil, hardmode, time, spawn and dungeon), special seeds, bosses defeated, events, rescued NPCs, every chest's contents (with search), signs and the town NPCs' homes |
| The Banner Saga 3 | *Caravan*: renown, supplies, morale and counts, and a table of each hero's rank, kills and stat bonuses (checked on a public save, not yet tried in the game) |
| The Binding of Isaac: Rebirth | *Basement*: Eden tokens, win and loss streaks, deaths, donation machine coins and the other counters, plus every achievement, collectible seen and challenge as a 0/1 unlock flag, for Rebirth, Afterbirth, Afterbirth+ and Repentance (the checksum is written again). checked on public saves of all four versions, not yet tried in the game |
| The Elder Scrolls II: Daggerfall (Daggerfall Unity) | *Character*: gold carried, level, health, magicka and fatigue, the eight attributes, all 35 skills, resistances, reputations and the gold and loan in each region's bank, from a save folder's `SaveData.txt` (checked on a public save, not yet tried in the game; classic `SAVETREE.DAT` saves are not supported yet) |
| The Elder Scrolls III: Morrowind | *Character*: level, gold, the eight attributes, all 27 skills, health, fatigue and magicka (now and full) and the inventory counts by item id, with the spell list shown (an `.ess` save is edited in place; checked on public saves, not yet tried in the game) |
| The Elder Scrolls IV: Oblivion | *Character*: level (the load screen's copy is kept equal), gold, the eight attributes, all 21 skills and base health (an `.ess` save is edited in place; checked on public saves, not yet tried in the game) |
| The Evil Within | *Sebastian*: green gel, trap parts, weapon upgrade levels and stat upgrades from `inventory.zwei` (checksum written again on save; checked on public saves, not yet tried in the game) |
| The Forest | *Survivor*: every item the player carries (named from the game's own ids) with its amount and the limit the game stores for it, with a **Max crafting materials** button that raises rocks, sticks, skulls, rope, bones, skins, rabbit fur and circuit boards to those stored limits; the Base64 and UnitySerializer wrapping is untouched (checked on public single-player and multiplayer saves, not yet tried in the game) |
| The Legend of Heroes: Trails in the Sky the 3rd | *Party*: Mira, the seven kinds of sepith, each character's level and how many of every item, quartz, weapon and meal the party carries, by name (checked on public saves, not yet tried in the game) |
| The Legend of Heroes: Trails of Cold Steel | *Party*: Mira, the eight kinds of sepith, each party member's level, experience, HP, EP, CP and nine stats, and how many of every item, quartz, weapon and meal is in the bag, by name (checked on a public save, not yet tried in the game) |
| The Legend of Zelda: Breath of the Wild | *Player*: rupees, monster parts, max hearts and stamina, Korok seeds, Hinox, Talus and Molduga counts, play time, and the amounts of materials (with a Max crafting materials button at 999), food and arrows by name (Wii U or Switch `game_data.sav`, any game version; checked on a public save, not yet tried in the game) |
| The Long Drive | *Driver*: health, food, water and sleep, the distance driven and the world time, from `TLDsaveData.tlds` and `TLDsaveProp.tldsp` (.NET BinaryFormatter, read and written in place; checked on a public save, not yet tried in the game) |
| The Messenger | *Ninja*: each save slot's play time, deaths and New Game Plus count, from `SaveGame.txt` (every character is XORed; unscrambled and scrambled again; checked on a public save, not yet tried in the game) |
| The Planet Crafter | *Terraformer*: Terra tokens, the six terraformation levels of each planet, your oxygen, thirst and health, the game settings (free crafting, everything unlocked, unlocked trading, extractors, teleporters, drones and autocrafter), and the raw materials in your backpack by name (iron, cobalt, silicon, titanium, magnesium, aluminium, iridium, super alloy, zeolite, sulfur, osmium and the rest), with a **Max crafting materials** button that fills the free backpack slots, since every item takes one slot and the game has no stack size. Raising a count adds items to the backpack; the rest of the save stays byte for byte (checked on a real late-game save, not yet tried in the game) |
| The Sims (2000), The Sims Legacy Collection | One tab per family: funds (up to 9,999,999) and family friends, and for every Sim in the family the seven skills and six personality traits (0–1000, 100 per point on the game's bars) and job performance; townies and visitors are on the *Values* tab with Advanced on. Open `Neighborhood.iff`; Sims carry their file name (`user00012`) |
| The Sims 2 (incl. Legacy and Ultimate Collection) | One tab per family: funds, and for every Sim the seven skills and five personality traits (0–1000, 100 per point), job performance and aspiration score; townies and the default families are on the *Values* tab with Advanced on. Open `N001_Neighborhood.package`; edited parts are stored uncompressed when the package is saved (the way SimPE does). Sims carry their id, not their name |
| The Sims 3 | One tab per household (the most-played first): funds (up to 99,999,999), and for every Sim their lifetime reward points, lifetime happiness, celebrity level and points, job and school performance, needs (with a **Fill every need** button) and skills of every age group, including toddler, child and expansion skills, each up to its own highest level (with a **Max every skill** button). The supernatural type is shown but can't be changed. Open `<world>_0x….nhd` inside the town's `.sims3` folder in `Documents\Electronic Arts\The Sims 3\Saves`; the game data is stored uncompressed when saved, so the file grows (checked on public saves, not yet tried in the game) |
| The Sims 4 | One tab per household: funds (up to 99,999,999) and its Sims with their names, ages and genders. Open `Slot_0000000N.save`; the game data is stored uncompressed when saved, so the file grows (checked on a public save, not yet tried in the game). Skills and traits aren't mapped yet |
| The Witcher 3 | *Geralt*: crowns, level, unspent skill points and experience; a *Story* panel of about 3,000 quest facts by quest type and area (choices made, who lived, who ruled, romances), checked against a public 100% save with a known ending (repacked with LZ4 on save; not yet tried in the game) |
| theHunter: Call of the Wild | *Hunter*: cash, level, XP and the skill and perk points of the profile (`thp_player_profile_adf`, an Avalanche ADF file read from its own type definitions and edited in place); no crafting materials (checked on public saves, not yet tried in the game) |
| Thief Simulator | *Thief*: cash, level, experience, skill points, hard mode and New Game Plus, from the plain JSON `playerdata_…` file (checked on public saves, not yet tried in the game) |
| Thymesia | *Corvus*: experience, healing potions, play time and a table of the carried items with their quantities (checked on a public save, not yet tried in the game) |
| Tiny Tina's Wonderlands | *Hero*: level (to 40) and experience, skill points; gold, moon orbs and ammo; gear is listed (slot, favourite, junk), not edited. Encrypted again on save. |
| Titan Quest Anniversary Edition | *Character*: level, experience, money, unspent points, the five attributes and skill levels (stash files aren't read). Not yet tried in the game |
| Tokyo Xtreme Racer (2025) | *Driver*: CP (money) and its limit, PP, garage size, the day, and the battle records (checked on a public save, not yet tried in the game) |
| Tooth and Tail | *Campaign*: which campaign missions are marked complete (checked on a public save, not yet tried in the game) |
| Torchlight II | *Character*: difficulty (Casual to Elite), level, experience, fame, gold, unspent points, the four stats, health, mana and item stacks; scrambled again with its checksum (the older 0x40 format isn't supported). Not yet tried in the game |
| Torchlight III | *Character*: level, hit points, experience, skill points and difficulty; *Gear*: the level and quantity of every carried item, consumable and pet item. The Unreal save holds the character as JSON text, which is opened and written back as that string (checked on a public save, not yet tried in the game) |
| Total War: Attila | Every faction's treasury, yours first, with a button to fill it. The campaign inside the save is unpacked, edited with every size above it recomputed, and packed again (the file grows). Read on a public campaign save, not yet tried in the game |
| Total War: ROME II | *Treasury*: every faction's treasury, the player's first, with a button to fill it, in Rome II campaign saves (the campaign inside the save is unpacked, edited and packed again). read on public campaign saves of the game, not yet tried in the game |
| Total War: SHOGUN 2 | *Treasury*: every faction's treasury, the player's first, with a button to fill it, in Shogun 2 campaign saves (the campaign inside the save is unpacked, edited and packed again). read on public campaign saves of the game, not yet tried in the game |
| Total War: THREE KINGDOMS | *Treasury*: every faction's treasury, the player's first, with a button to fill it, in Three Kingdoms campaign saves (the campaign inside the save is unpacked, edited and packed again). read on public campaign saves of the game, not yet tried in the game |
| Total War: Warhammer | Every faction's treasury, yours first, with a button to fill it. The campaign inside the save is unpacked, edited with every size above it recomputed, and packed again (the file grows). Read on a public campaign save, not yet tried in the game |
| Total War: Warhammer II, Total War Saga: Troy | Every faction's treasury, yours first, with a button to fill it. The campaign inside the save is unpacked, edited with every size above it recomputed, and packed again (the file grows). |
| Touhou Luna Nights | *Sakuya*: gold, level, experience, HP, MP, knife and clock upgrades, the seven gems, the abilities and keys, and the shop counters from `gameN.sav` (the base64 lines are opened as one list of 1,000 values and written back the same way; checked on a public save, not yet tried in the game) |
| Trails in the Sky 1st Chapter (2025) | *Bracers*: mira, the eight sepith with a *Max crafting materials* button, and each character's experience, HP and EP; unpacked (zstd) and packed again with the save's checksum. Not yet tried on a real save |
| Trails to Azure / Ao no Kiseki KAI | *Party*: Mira, DP, medals, difficulty, play time; the seven Sepith; every character's level, HP, EP, CP, experience and stats; the bonding values. NISA `savedata.dat` (zstd) and KAI `NNN_save.bin` (AES) are both opened and written back with their checksums. Checked on synthetic saves, not yet tried in the game |
| Travellers Rest | *Tavern*: money (in copper), the crafting room, tavern floor and cellar upgrades, and the inventory and action bar by item id with their stack sizes (checked on a public save, not yet tried in the game) |
| Tyranny | *Party*: money, camping supplies, and each character's experience, attribute and talent points, attributes and skills from the `.savegame` zip (checked on a public save, not yet tried in the game) |

</details>

<details>
<summary><strong>U</strong> (3)</summary>

| Game | Screen |
|---|---|
| ULTRAKILL | *V1*: points (P) and every weapon variant and arm, owned or not, in `generalprogress.bepis`; each level file's ranks and secrets open too (checked on public saves, not yet tried in the game) |
| Undertale | Name, LV, HP, attack, defence, EXP, gold, kills; the item, phone and equipment slots by name; the route record (kills per area, Genocide and Pacifist flags, boss outcomes, spared and fled counts, shop spending); room and play time |
| Undertale Yellow | *Clover*: LV, EXP, gold, HP and the other stats, equipment names, the eight item slots and play time from the save file (checked on public saves, not yet tried in the game) |

</details>

<details>
<summary><strong>V</strong> (3)</summary>

| Game | Screen |
|---|---|
| Valheim (characters, profile versions 33 to 37) | Health, stamina, skills, item stacks (gold is Coins), quality and durability; the SHA-512 is written again. A **Max crafting materials** button fills the wood, stone, ores, metals, hides and other plain materials you carry to their stack size, leaving boss drops, summoning items and key fragments alone. Newer characters open read-only. Not yet tried in the game |
| Vampire Survivors | Coins, power-up ranks and what's unlocked; the checksum is worked out again on save. Not yet tried in the game |
| Vikings: Wolves of Midgard | *Clan*: gold, iron, steel, blood, jotunjarn, the ores and bloodstone, secrets found and finished runs, from the plain JSON `game_….sav` (checked on a public save, not yet tried in the game) |

</details>

<details>
<summary><strong>W</strong> (19)</summary>

| Game | Screen |
|---|---|
| Wandering Sword | *Jianghu*: money, with the map, mount, play time and difficulty shown; the save's zlib wrapping is removed and put back for you (checked on public saves, not yet tried in the game) |
| Warhammer 40,000: Mechanicus | *Cadmus*: Blackstone, the Ark's bonus points and slots, and each Techpriest's level, XP, skill points, slot points and augment capacity; the save's own text dialect is patched in place and the checksum in `SaveState.sav` is rewritten for you (checked on public saves, not yet tried in the game) |
| Warhammer 40,000: Rogue Trader | *Dynasty*: scrap, Profit Factor, the Navigator's insight, difficulty settings and reputation with each faction (same save family as Pathfinder; no Rogue Trader save tried yet) |
| Warm Snow | *Swordsman*: memento souls, red souls, souls, soul jars, the life, kill and refine levels, health, difficulty and runs, from `save_0` (.NET BinaryFormatter, read and written in place; checked on a public save, not yet tried in the game) |
| Warriors All-Stars | *Campaigns*: the gold and the materials you own in each of the nine campaign slots in use (with *Max crafting materials*, 9,999 each); all ten encrypted blocks and their seeds are handled for you (checked on a public save, not yet tried in the game) |
| Warriors Orochi | *Army*: the shared Growth Points and each officer's Life, Musou, Attack, Defense and Speed of the original 2008 release; the checksum is handled for you (checked on a public save, not yet tried in the game) |
| Warriors Orochi 3 Ultimate Definitive Edition | *Resources*: Growth Points, Gems, each officer's health, Musou, attack, defense and speed, and the weapon attribute orbs and crafting materials you own (with *Max crafting materials*, 99 each, the highest value in a completed save); the save is plain data (checked on a public save, not yet tried in the game) |
| Warriors Orochi 4 | *Resources*: Gems, Crystals, Growth Points, Skill Orbs (lower only), the weapon-element stacks you own (with *Max crafting materials*, 999 each) and each progressed officer's skill points; works for the original and Ultimate saves, both AES blocks and their seeds are handled for you (checked on public saves of both, not yet tried in the game) |
| Wasteland 2 | *Rangers*: each ranger's level, experience, health, attribute, skill and trait points, kills and the seven attributes (skills and items are in the Tree); the save's `<@>` name markers are kept as they are (checked on a public save, not yet tried in the game) |
| Wasteland 3 | *Rangers*: money; each ranger's level, XP, hit points, attributes, unspent points and skills by name (Automatic Weapons, First Aid, Lockpicking and the rest); item quantities in the inventory and stash; Fame and each faction's reputation (negative allowed) and the quest counters the game keeps. Unpacked (LZF) and packed again on save. Not yet tried in the game |
| Watch Dogs 2 | *Marcus*: money, followers and research points (while some are unspent), and the lifetime totals, changed in place in the save's object tree (checked on public saves, not yet tried in the game) |
| West of Loathing | *Cowpoke*: meat (money), level, experience, the class and resistances, and the stats, skills and perks by name; the inventory with quantities (the game stores most numbers as text) (checked on a public save, not yet tried in the game) |
| Wizard of Legend | *Wizard*: chaos gems, play time and the unlocked outfits, from `GameData.gd` (.NET BinaryFormatter, read and written in place; checked on a public save, not yet tried in the game) |
| Wolcen: Lords of Mayhem | *Character*: gold, level, experience, primordial affinity, the four attributes, attribute and passive skill points to spend, difficulty mode and league from a character `.json`; items and skills are in the Tree. checked on a public save, not yet tried in the game |
| Workshop Simulator | *Workshop*: the account balance, total money earned and orders finished; large saves take a moment to open (checked on a public save, not yet tried in the game) |
| World of Horror | *Unlocks*: unlock every achievement, mystery ending, enemy, event and pantheon entry the way the game itself records them (each with its own code, since a plain "yes" unlocks nothing), with the unlock counts, the last run's character, items and mystery board, and the settings in `save.ito` (checked on a public completed save, not yet tried in the game) |
| World War Z (and Aftermath) | *Survivor*: currency, coins, account level and the class perk levels, from `user_progression.dat` (unpacked with the key the game derives from the file name, and packed again; checked on a public save, not yet tried in the game) |
| WorldBox - God Simulator | *World*: every unit's level, experience, health, hunger and ruler stats (diplomacy, intelligence, stewardship, warfare), and the first village's stored resources, from `map.wbox` (zlib-compressed JSON, unpacked and packed again; checked on a public save, not yet tried in the game) |
| Wreckfest | *Driver*: credits, experience and tournament fame from `career.cres` (unpacked from its block compression and packed again with fresh checksums; checked on a public fresh profile, not yet tried in the game; the garage and parts files open as hex) |

</details>

<details>
<summary><strong>X</strong> (6)</summary>

| Game | Screen |
|---|---|
| X Rebirth | *Pilot*: your credits and name; large saves take a few seconds to open (checked on a public save, not yet tried in the game) |
| X-COM: Terror from the Deep (original saves) | *Funds*: the money from `LIGLOB.DAT` in a save slot folder of the original game; the other `.DAT` files are not edited yet (checked on a public save, not yet tried in the game) |
| X-COM: UFO Defense (original saves) | *Funds*: the money from `LIGLOB.DAT` in a save slot folder (`GAME_1` to `GAME_10`) of the original game; the other `.DAT` files are not edited yet (not yet checked on a save of this game) |
| X4: Foundations | The player's credits, written to all three places the save keeps them (info line, player faction account, statistics); the rest of the XML tree is open for editing. Opens `.xml.gz` saves directly and packs them again. |
| XCOM 2 | *HQ*: supplies, intel, alloys, elerium and the other HQ resources; every soldier's rank, kills, missions, ability points and stats (HP, aim, mobility, will, hacking, defense, dodge, psi, armour, shield, sight); storage counts by the game's item name. Unpacked and packed again on save. Not yet tried in the game |
| XCOM: Enemy Unknown / Within | *HQ*: money; every soldier's rank, XP, kills, HP, aim, mobility and will (the fallen listed last); storage counts. Every saved object's numbers are in *Values* |

</details>

<details>
<summary><strong>Y</strong> (11)</summary>

| Game | Screen |
|---|---|
| Yakuza 0 | *Kiryu & Majima*: both characters' money (up to ¥10 trillion) and CP, the uncollected real estate and cabaret takings, and the difficulty |
| Yakuza 3 Remastered | *Kiryu*: money, the hostess club's money and his experience (the save has no checksum; checked on a public save, not yet tried in the game) |
| Yakuza 4 Remastered | *Akiyama, Saejima, Tanimura & Kiryu*: each one's money, experience and skill orbs, plus the eco points (checked on a public save, not yet tried in the game) |
| Yakuza 5 Remastered | *Kiryu, Saejima, Akiyama, Shinada & Haruka*: each one's money, experience and skill orbs (checked on a public save, not yet tried in the game) |
| Yakuza 6: The Song of Life | Money, HP and Kiryu's five experience pools; decrypted (key from rgg_save_tool_py) with its checksum kept right, every other value in the tree (checked on real saves, not yet tried in the game) |
| Yakuza Kiwami | *Kiryu*: money and the experience he hasn't spent yet (save and autosave files) |
| Yakuza Kiwami 2, Like a Dragon (Dragon Engine) | *Player*: decrypted (XOR) and its checksum updated (Adler-32 or CRC-32); money, HP, Kiwami 2's experience pools and Like a Dragon's whole party (level, experience, HP, MP), with every other value in the tree |
| Yakuza Kiwami 3 & Dark Ties | *Kiryu*: money, plus each character's HP and heat; the MessagePack save is decrypted with the game's key and its checksum kept right, every other value in the tree (checked on public chapter saves, not yet tried in the game) |
| Youtubers Life | *Youtuber*: money, house, work hours, your channel (subscribers, views, status) and every youtuber's needs, skills, levels and experience from the SQLite save (checked on a public save, not yet tried in the game) |
| Ys VIII: Lacrimosa of DANA | *Party*: play time, Adol's and Laxia's weapon, armour and accessories (by item id) and every item you hold with its count, up to 999. Both checksums are kept right (checked on public saves, not yet tried in the game) |
| Ys: The Oath in Felghana | *Adol*: level, maximum HP, experience, gold and the difficulty (all six levels, written everywhere the game stores it); the cipher over the save and its two checksums are handled for you (checked on a public save, not yet tried in the game) |

</details>

**Console games**, grouped by console (emulator, memory card and decrypted console saves):

<details>
<summary><strong>PlayStation</strong> (15)</summary>

| Game | Screen |
|---|---|
| Castlevania: Symphony of the Night | *Alucard* (PlayStation memory card, `.mcs`, `.psx` or `.psv` save): level, experience, HP, MP, hearts, the four stats, gold, kills, play time, subweapon, equipped gear, every relic, form and familiar card, spells, familiar levels, warp rooms, and the count of every weapon, shield, armor, accessory and potion (checked on public saves, not yet tried in the game) |
| Chrono Cross (PlayStation) | *Party* (PlayStation memory card, `.mcs`, `.psx` or `.psv` save, USA or Japanese): gold, each character's HP, stats and equipment, and every inventory and element slot (the game's packed data and the save's checksum are written again; checked on public saves, not yet tried in the game) |
| Crash Bandicoot (PlayStation) | *Bandicoot* (PlayStation memory card, `.mcs`, `.psx` or `.psv` save of the 1996 game): the level reached, every gem, both keys and the volumes; the gem and key counts, the completion rate and the checksum are written again (checked on public saves, not yet tried in the game) |
| Crash Team Racing (PlayStation) | *Racer* (PlayStation memory card, `.mcs`, `.psx` or `.psv` save): the unlocked characters, tracks and menus, and for each of the four adventure slots the world doors and every trophy, relic, CTR token, boss key, gem and N. Oxide challenge; the save's checksum is written again (checked on public saves, not yet tried in the game) |
| Final Fantasy Tactics | *Ramza* (PlayStation memory card, `.mcs`, `.psx` or `.psv` save): war funds, each unit's job, level, experience, Brave, Faith and equipment, and the count of every weapon, helmet, armor, accessory and item (the save's 64-bit checksum is written again; checked on public saves, not yet tried in the game) |
| Final Fantasy VIII (PlayStation) | *Squall* (PlayStation memory card, `.mcs`, `.psx` or `.psv` save): gil, story progression, Triple Triad record and cards, each party slot's experience, HP and stat bonuses and stocked magic, the Guardian Forces, and all 198 bag slots (the save's CRC-16 is written again; checked on public saves, not yet tried in the game) |
| Gran Turismo (PlayStation) | *Driver* (PlayStation memory card, `.mcs`, `.psx` or `.psv` save of the 1997 game): credits, days, every licence test medal and the result of every league, special event and spot race; the save's checksum is written again (cars and tuning parts are not edited yet; checked on public saves, not yet tried in the game) |
| Grandia (PlayStation) | *Party* (PlayStation memory card, `.mcs`, `.psx` or `.psv` save, USA, Europe, France, Germany and Japan): gold pieces, each character's level, experience, HP, SP and attributes, magic and weapon levels, equipment, the items in each pack, and every learned skill |
| Rayman (PlayStation) | *Rayman* (PlayStation memory card, `.mcs`, `.psx` or `.psv` save): continues, lives, health, the powers, and every level's unlock and Electoon cages; the level totals and the completion rate are written again (checked on public saves, not yet tried in the game) |
| Resident Evil (PlayStation) | *Survivor* (PlayStation memory card, `.mcs`, `.psx` or `.psv` save, Director's Cut too): Chris or Jill, health, poison, ink ribbons used, the eight inventory slots and all 48 item box slots with item and quantity (checked on public saves, not yet tried in the game) |
| Silent Hill (PlayStation) | *Slots* (PlayStation memory card, `.mcs`, `.psx` or `.psv` save): the options and unlocked endings, and for every save slot in the file the difficulty, health, save and continue counts, shooting statistics and the whole inventory with item and quantity; the checksums are written again (checked on public saves, not yet tried in the game) |
| Tekken (PlayStation) | *Characters* (PlayStation memory card, `.mcs`, `.psx` or `.psv` save): the unlocked characters, the characters the arcade mode was completed with and the game options (checked on public saves, not yet tried in the game) |
| Tekken 2 (PlayStation) | *Characters* (PlayStation memory card, `.mcs`, `.psx` or `.psv` save): the unlocked characters, the characters the arcade mode was completed with, the game options and each character's arcade and versus statistics; the scrambling and checksum are written again (checked on public saves, not yet tried in the game) |
| Tekken 3 (PlayStation) | *Characters* (PlayStation memory card, `.mcs`, `.psx` or `.psv` save): the unlocked characters, costumes, modes and movies, the characters the arcade mode was completed with, the Tekken Force key and high score, the game options and each character's arcade and versus statistics; the scrambling and checksum are written again (checked on public saves, not yet tried in the game) |
| Tony Hawk's Skateboarding (PlayStation) | *Skater* (PlayStation memory card, `.mcs`, `.psx` or `.psv` save): every skater's tapes in each level, the medals of the three competitions, the high scores and the sound levels; the tape counts and the checksum are written again (checked on public saves, not yet tried in the game) |

</details>

<details>
<summary><strong>PlayStation 2</strong> (10)</summary>

| Game | Screen |
|---|---|
| .hack (PlayStation 2) | *Party* (PlayStation 2 `.psu`, `.psv` or memory card save of .hack//Infection, Mutation, Outbreak or Quarantine; each `dhdata` file is one slot): play time, unlocked servers, each party member's class, level, experience, money, affection and stats, and the items in Elf's Haven (the slot's checksum and preview in the system file are written again; checked on public saves, not yet tried in the game) |
| Dead or Alive 2 (PlayStation 2) | *General* (PlayStation 2 `.psu`, `.psv` or memory card save, USA, Europe and Japan): points, games played, play time and play counts, the unlocked characters and menus, each fighter's unlocked costumes and story completions, and the count of every collection item |
| Devil May Cry (PlayStation 2) | *Dante* (PlayStation 2 `.psu`, `.psv` or memory card save, USA, Europe, Japanese and Korean; each `SaveData-NN` file is one slot): clear and save counts, character, difficulty, mission, maximum health, red, yellow and blue orbs, the Alastor and Ifrit skills, every mission rank, the secret missions, statistics and combo rankings; the slot's checksum is written again (checked on public saves, not yet tried in the game) |
| Dragon Ball Z: Budokai Tenkaichi 2 (PlayStation 2) | *General* (PlayStation 2 `.psu`, `.psv` or memory card save, USA, Europe, Japanese Sparking! NEO and Korean): zeni, every unlockable character and form, and the count of every Z-item and Dragon Ball |
| Dragon Quest VIII (PlayStation 2) | *Party* (PlayStation 2 `.psu`, `.psv` or memory card save, USA, Europe and Japanese): gold coins, bank vault, casino tokens, mini medals, every warp place, each party member's level, experience, HP, MP, stats and skill attributes, the bag and the battle records (the save's checksum is written again; checked on public saves, not yet tried in the game) |
| Grandia II (PlayStation 2) | *Party* (PlayStation 2 `.psu`, `.psv` or memory card save, USA, Europe and Japan; each slot is one file of the save folder): gold and coins, each party member's level, experience, HP, MP, SP, attributes, equipment and conditions, and the stock of every item, weapon, armor, accessory, mana egg and valuable (checked on public saves, not yet tried in the game) |
| Kingdom Hearts (PlayStation 2) | *Sora* (PlayStation 2 `.psu`, `.psv` or memory card save, original and Final Mix): munny, difficulty, magic levels, the ten party members' level, experience, HP, MP, AP, strength, defense and weapon, and the stock of every item, weapon, accessory, key item and synthesis material, with a **Max crafting materials** button for the synthesis materials (99 each, the game's own limit) (checked on public saves, not yet tried in the game) |
| Kingdom Hearts II (PlayStation 2) | *Sora* (PlayStation 2 `.psu`, `.psv` or memory card save, USA, Europe, Japanese and Final Mix releases): munny, difficulty, magic levels, each party member's level, experience, HP, MP and bonuses, and the stock of every item, weapon, armor, accessory, key item and synthesis material, with a **Max crafting materials** button for the synthesis materials (99 each, the most the stock holds; an estimate, no public source states it). The save's checksum is written again (checked on public saves, not yet tried in the game) |
| Shin Megami Tensei: Lucifer's Call (PlayStation 2) | *Party* (PlayStation 2 `.psu`, `.psv` or memory card save, USA and Europe): Macca and the playthrough cycle, the protagonist's and each demon's level, experience, HP, MP, stats and skills, the stock of every item and gem, and the special items |
| The Bouncer (PlayStation 2) | *Sion Barzahd* (PlayStation 2 `.psu`, `.psv` or memory card save, USA, Europe and Japan; the save file holds 16 slots): each slot's clear count, the Bouncer Points of Sion, Volt and Kou, their stat levels, ranks and skills, the unlocked stages, characters and costumes, and the survival mode records |

</details>

<details>
<summary><strong>SNES</strong> (6)</summary>

| Game | Screen |
|---|---|
| Chrono Trigger (SNES, emulator `.srm` saves) | *Slots*: gold and silver points, each character's experience, tech points, HP, MP, base stats and equipment (each slot's sum is kept right; checked against the sums of a public save, not yet tried in the game) |
| Final Fantasy VI (SNES, emulator `.srm` saves) | *Slots*: gil, steps and play time, every party member's level, HP, MP, experience, stats, esper and equipment, and the inventory's items and quantities (each slot's sum is kept right; checked against the sums of a public save, not yet tried in the game) |
| Mystic Quest (SNES, emulator `.srm` saves) | *Slots*: money, and the hero's and partner's level, experience, HP and spell charges (each slot's checksum is kept right; checked against public saves, not yet tried in the game) |
| Secret of Mana (SNES, emulator `.srm` saves) | *Slots*: money, each hero's level, experience, HP and MP, and weapon and spirit levels (each slot's checksum is kept right; checked against a public save, not yet tried in the game) |
| Super Metroid | *Samus A / B / C* (SNES emulator `.srm` saves): energy and reserve tanks, missiles, super missiles, power bombs, play time and the beams, suits and boots she has and has equipped (each slot's sums and their copies at the end of the file are kept right; checked against the sums of a public save, not yet tried in the game) |
| The Legend of Zelda: A Link to the Past | *Slots* (SNES emulator `.srm` saves): hearts, magic, rupees, bombs and arrows with their capacity upgrades, the sword, shield, tunic and every item, and the four bottles (the slot's sum is kept right; checked against the sum of a public save, not yet tried in the game) |

</details>

<details>
<summary><strong>N64</strong> (14)</summary>

| Game | Screen |
|---|---|
| Banjo-Kazooie | *Banjo* (N64 emulator `.eep` saves): per save slot jiggies, Mumbo tokens, health, eggs and feathers with their maximums, every move, each world's jiggies and the Stop 'n' Swop items, with all other flags in the Tree (the slot's sum is made again on save; checked on public saves, not yet tried in the game) |
| Banjo-Tooie | *Banjo-Tooie* (N64 emulator `.eep` / `.srm` saves): per save slot eggs, feathers, honeycombs, Cheato's pages, Glowbos and the other carried items, every move, the cheats, the minigame high scores and the time spent in each world (jiggies, notes and world flags are left out because the American release stores them elsewhere; the slot's sum is made again on save; checked on public saves, not yet tried in the game) |
| Donkey Kong 64 | *DK64* (N64 emulator `.eep` / `.srm` saves): per save slot play time, melons, the Kongs freed, each Kong's Banana Coins and move upgrades, the moves and items, the Mystery menu unlocks, and in the Tree the Banana Fairies, Battle Crowns, Boss Keys and every level's Golden Bananas, medals, blueprints and coloured bananas (Golden Banana counts follow the bananas ticked; every CRC is made again on save; checked on public saves, not yet tried in the game) |
| F-Zero X | *Pilot* (N64 emulator `.sra` / `.srm` saves): the everything-unlocked switch, VS and sound options, each cup's Grand Prix progress, the cups won with all thirty machines in every class, the Time Attack times of all twenty-four courses and the Death Race time (every block's sum is made again on save; checked on public saves, not yet tried in the game) |
| Gex 64: Enter the Gecko | *Gecko* (N64 emulator Controller Pak `.mpk` / `.srm` saves): for every save the lives, each level's red remotes, the hidden and reward silver remotes and the gold remotes, with the totals (the game's password is coded again for each changed save; checked on public saves, not yet tried in the game) |
| Goemon's Great Adventure | *Goemon* (N64 emulator Controller Pak `.mpk` / `.srm` and Japanese `.sra` saves): per slot location, ryo, armour, weapon level, item, the money given to Kenzo and Kensuke, the Chain Pipe and Underwater Device, plus the language and sound options (each block's CRC is made again on save; checked on public saves, not yet tried in the game) |
| GoldenEye 007 | *Agent* (N64 emulator `.eep` saves): per player slot the best time of every mission on each difficulty, the cheats and the control options (the slot's sum is made again on save; checked on public saves, not yet tried in the game) |
| Mario Kart 64 | *Racer* (N64 emulator `.eep` saves): Grand Prix cups for each class, Time Trial times and characters for all sixteen courses, sound mode (the game's sums and best-records list are kept right; checked on public saves, not yet tried in the game) |
| Mystical Ninja Starring Goemon | *Goemon* (N64 emulator Controller Pak `.mpk` / `.srm` saves): per slot ryo, play time, food, armour, fortune doll pieces, fish, keys, each character's unlock, weapon level and items, the key and miracle items, and every castle's map and boss (each slot's CRC is made again on save; checked on public saves, not yet tried in the game) |
| Paper Mario | *Paper Mario* (N64 emulator `.fla` / `.srm` saves): per file coins, Star Pieces, Star Points, level, HP, FP and BP with their maximums, Star Spirits and Star Energy, boots and hammer, the bag items, partners and their ranks, and the battle records, with badges, stored and key items, recipes and found star pieces in the Tree (the newest save of each file is edited and both of its sums are made again; checked on public saves, not yet tried in the game) |
| Super Mario 64 | *Files* (N64 emulator `.eep` saves): for each of the four files the cap, key and switch flags, secret stars, every course's six stars, 100-coin star, cannon and coin record (both copies of the file and the checksums are kept right; checked on public saves, not yet tried in the game) |
| Super Smash Bros. | *Fighters* (N64 emulator `.sra` / `.srm` saves): unlocked fighters and stage, every fighter's 1P records and 2P records, and the options (the game's sum is made again on save; checked on public saves, not yet tried in the game) |
| The Legend of Zelda: Majora's Mask | *Majora* (N64 `.fla` / `.srm` and GameCube `.gci` saves): per slot rupees and wallet, health, heart pieces, magic, double defense, the sword, shield, quiver and bomb bag with all ammunition, every item, mask, song and boss remains, each temple's stray fairies, keys, map, compass and boss key, and the owl statues, with the Bombers' Notebook and other events in the Tree (an owl-statue save is edited when there is one; the sum and backup copy are kept right; checked on public saves, not yet tried in the game) |
| The Legend of Zelda: Ocarina of Time | *Link* (N64 `.sra` / `.srm` and GameCube `.gci` saves): per file health, magic, rupees, equipment, every item with ammunition and upgrades, songs, medallions, stones and each dungeon's map, compass and boss key, with all other flags in the Tree (the file's sum and its backup copy are kept right; checked on public saves, not yet tried in the game) |

</details>

<details>
<summary><strong>GameCube</strong> (5)</summary>

| Game | Screen |
|---|---|
| Luigi's Mansion | *Luigi* (GameCube `.gci` saves): per slot the treasures, medals, Mario's items, gallery, boos, portrait ghosts and every mansion room's flags, plus the options (the newer copy of the save is edited and its sum made again; checked on public saves, not yet tried in the game) |
| Super Mario Sunshine | *Mario* (GameCube `.gci` saves): per file lives, nozzles, Yoshi and, for each area, shine sprites, blue coins, events and scores, with the shine sprite total kept right (the newer copy of the save is edited and its sum made again; checked on public saves, not yet tried in the game) |
| The Legend of Zelda: Majora's Mask | *Majora* (N64 `.fla` / `.srm` and GameCube `.gci` saves): per slot rupees and wallet, health, heart pieces, magic, double defense, the sword, shield, quiver and bomb bag with all ammunition, every item, mask, song and boss remains, each temple's stray fairies, keys, map, compass and boss key, and the owl statues, with the Bombers' Notebook and other events in the Tree (an owl-statue save is edited when there is one; the sum and backup copy are kept right; checked on public saves, not yet tried in the game) |
| The Legend of Zelda: Ocarina of Time | *Link* (N64 `.sra` / `.srm` and GameCube `.gci` saves): per file health, magic, rupees, equipment, every item with ammunition and upgrades, songs, medallions, stones and each dungeon's map, compass and boss key, with all other flags in the Tree (the file's sum and its backup copy are kept right; checked on public saves, not yet tried in the game) |
| The Legend of Zelda: The Wind Waker | *Link* (GameCube `.gci` saves): per slot health, magic, rupees, equipment, items, bags, quantities, figurines, dungeon items, songs and events (the slot's sum is made again and the sword, shield and bracelet flags follow your choice; checked on public saves, not yet tried in the game) |

</details>

<details>
<summary><strong>Switch</strong> (2)</summary>

| Game | Screen |
|---|---|
| Super Smash Bros. Ultimate | *Main*: gold, tickets, hammers, snacks, SP and the spirit board item counts (Switch `system_data.bin`; checked on a public save, not yet tried in the game) |
| Xenoblade Chronicles 3 | *Party*: gold, play time, every character's level, experience and bonus experience, and the amounts of ether cylinders, gem crystals and collectibles by id (Switch saves from an emulator or a dump; checked on a public save, not yet tried in the game) |

</details>

<details>
<summary><strong>Game Boy</strong> (1)</summary>

| Game | Screen |
|---|---|
| The Legend of Zelda: Oracle of Ages and Oracle of Seasons (Game Boy Color, emulator `.sav` saves) | *Slots*: health, heart pieces, rupees, deaths, sword and shield levels, bombs, bombchus and every kind of seed (each slot's sum is kept right; checked against public saves, not yet tried in the game) |

</details>

<details>
<summary><strong>GBA</strong> (2)</summary>

| Game | Screen |
|---|---|
| Golden Sun | *Party*: every file in the flash with its coins; each party member's level, experience, HP / PP, base stats and items; the Djinn grid, where each Djinni can be given to a character (set or on standby) or taken away, with the game's own counts kept in step (checked on public sample saves); a *Story* panel with the choices the password carries into The Lost Age (saved Hammet, beat Colosso, Hsu died, beat Deadbeard, returned to Vale or to the Vault). Checksums kept right. Not yet tried in the game |
| Golden Sun: The Lost Age | *Party*: every file in the flash with its coins; each party member's level, experience, HP / PP, base stats and items; all 72 Djinn, each to give to a character (set or on standby) or take away (checked on public sample saves). Checksums kept right. Not yet tried in the game |

</details>

<details>
<summary><strong>3DS</strong> (5)</summary>

| Game | Screen |
|---|---|
| Kid Icarus: Uprising | *Hearts*: hearts, hearts offered to Palutena and Viridi, and the play statistics (decrypted 3DS `0x.sav`; checked on a public save, not yet tried in the game) |
| Mario & Luigi: Dream Team | *Brothers*: coins, each brother's level, experience, HP, BP, Power, Defense, Speed and Stache, and the counts of recovery items, battle items and beans (decrypted 3DS save; checked on a public save, not yet tried in the game) |
| Mario Kart 7 | *Player*: coins, StreetPass count, online wins and losses and VR points; the save's CRC-32 is written again (decrypted 3DS `system0.dat`; checked on a public save, not yet tried in the game) |
| Super Kirby Clash | *Resources*: apple gems and the red, blue, yellow and rare shards (decrypted 3DS `savedata.dat`; checked on a public save, not yet tried in the game) |
| Team Kirby Clash Deluxe | *Resources*: apple gems, the four kinds of shards and the profile's counters (decrypted 3DS `savedata.dat`; checked on a public save, not yet tried in the game) |

</details>