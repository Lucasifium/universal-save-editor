# Changelog

## Unreleased

- Fixed: Metal Gear Solid V: edits to staff, resources, Mother Base platforms and other packed values couldn't be saved ("didn't read back exactly as edited"); every field now saves and reads back

## 0.8.0

- Backups › Quick restore asks before restoring: it names the backup and the save it replaces, and says the save on disk is backed up first
- Delete preset asks before deleting the preset; Cancel or Escape keeps it
- Fixed: backups dated in the future (taken while the computer's clock was set ahead) made each new backup be deleted on the next save, so only one recent backup was kept; they now go first when old backups are cleared out, and show their date without an age
- The open view tab (Values, Tree, …) and the open file tab are underlined, so they stand out from the others in both themes
- Fixed: choosing Save and close when quitting with several unsaved tabs saved one tab and closed the window; it now asks about each unsaved tab. Saving is also held back while a file is still being opened
- Fixed: a save that is a link to a file elsewhere (for example in a synced folder) kept its link when saved or restored; before, the link was replaced by a plain file and the linked file was left unchanged
- Fixed: Export decoded data, and the command line's decode -o, back up a file they replace; exporting over the save itself could leave a file the game can't load, with no backup
- Fixed: when a file the game checks against its save (such as Persona 4 Golden's .binslot) can't be written, the save is put back so the two still match, in the app, when restoring a backup and on the command line; restoring no longer says it worked when that file couldn't be updated
- Messages about saving, restoring, backing up and importing say whether the file on disk was changed and what to do next; confirm buttons name what they do (Restore now, Delete permanently, Open without saving); the app says "backup" throughout
- More of the app is translated: file dialog titles, the backup list, toolbar chips, the hex view guide, keyboard shortcut list and backup ages
- Easier to read and use: text, status colours and control outlines meet contrast guidelines in the light and dark themes; changed values and warnings are marked by more than colour
- Keyboard: a focus ring shows where Tab is; confirm dialogs start on Cancel and Tab stays inside them
- Screen readers: icon buttons, switches, section headers, search boxes and value boxes have names

- Fixed: Metal Gear Solid V: The Phantom Pain saves (TPP_GAME_DATA, PERSONAL_DATA, TPP_CONFIG_DATA) opened as hex instead of the game screen
- Metal Gear Solid V: Ground Zeroes saves are decrypted for the hex view, with their checksum redone on save
- Find my game's saves also looks in The Phantom Pain's 287700\local save folder
- Phone-size windows (the Android app, a narrow browser window): the menus fold into one ☰ button so the toolbar is a single line (menu, file name, Undo, Save); game screens get a row of large section chips to swipe through and tap to jump to each part, marked when a part holds a change; header tiles sit two to a row and keep their − and + buttons; the Changes list takes the whole width
- Save shows how many changes it will write ("Save (3)"), and the Changes button sits next to it
- The note about the opened file takes one line under the toolbar (More shows all of it); Dismiss hides it for every file with the same note, and View › Show the note about this file brings it back

- Light theme: text boxes have an outline, changed values and the unsaved-changes count are readable on white, the open Changes button no longer shows white text on a pale background, and switches that are off are visible
- Windows and dialogs have more room inside their edges (phone-width screens keep the narrow margin)
- The − and + buttons next to number boxes are as tall as the box, so they are easier to hit
- Values view without the Windows icon font (Linux, macOS, browser): section headers show drawn arrows instead of the letters "v" and ">", and the revert button says "Revert" in the chosen language
- Game screens: a value shown both in the header tile and in a panel no longer shares one text box's focus between the two
- Game screens: panels side by side switch to fewer columns when their rows don't fit, instead of running under the next panel and hiding its − and + buttons
- The toolbar wraps onto a second line when a long file name, game name or translation doesn't fit, instead of drawing the Changes button over the format label; the game screen's find box is wide enough for its translated hint
- Start page, toolbar labels and the backup folder text in Preferences are translated; "Label: text" lines wrap evenly; the start page no longer shows a "Common places…" heading with nothing under it
- Divinity: Original Sin 2: a character without attributes in the save says so instead of showing an empty Attributes panel
- Find my game's saves: each game's description (what can be edited, where its saves are, the file names) is now shown in the chosen language
- Values view: the list of values is built about twice as fast and takes well under half the memory for very large saves (half a million values), so loading, saving and edits that rebuild it are quicker
- Total War campaign saves open about 30% faster
- Number boxes: Up and Down arrows step the value by 1 while the box has focus (Shift: 10), within the field's allowed range
- Long drop-down lists (hundreds of items) only draw the rows in view, so they open instantly; Find my game's saves shows the first 60 saves of a game with a "Show all" link
- Find my game's saves: hovering the search box explains that words in any order and short names work
- Saving a large file no longer freezes the window: files over 256 KB (or with 20,000 values or more) are written, backed up and verified in the background behind a "Saving the file" box, nothing can be edited meanwhile, closing the window waits for the save to finish, and the Values list is no longer rebuilt after a save that leaves it the same
- Opening is faster still: more format checks turn a file down from its first bytes, and a large JSON save is read once instead of up to five times (a 23 MB save opens in about a quarter of the time)
- Faster opening: the checks that work out which game a file belongs to now look at its first bytes before doing the expensive work, so large saves open sooner; files over 256 KB open in the background and the window stays responsive
- Find my game's saves: the search box understands words in any order and short names (bg3, gta5, ff7, botw); the list of found saves is no longer worked out again on every frame
- Changes panel and unsaved-changes tracking are much faster after large bulk edits; the panel lists the first 300 changes and says so
- Status messages, hints and labels that were English-only are now translated into all six languages
- Fixed: importing values into a structured save did not mark it as changed; some saves (Star Wars Jedi: Fallen Order, Gothic 2) were mistaken for Dragon Quest Builders 2 data
- Optional new-version alert: Preferences › Tell me when a new version is out (off by default) checks GitHub for the latest release at startup and shows a dismissible banner with a download link; Help › Check for a new version does it once on demand. Only the request for the public latest-release page is sent

## 0.7.0

- New game screens: Kingdom Rush, Frontiers, Origins, Alliance TD and Vengeance (gems, difficulty, upgrades, heroes); Lua saves in the local-variable form open
- New game screens: Sea of Stars (level, experience, gold, deaths; .NET length-prefixed JSON opened) and The Messenger (play time, deaths, New Game Plus; scrambled text opened)
- Sonic Adventure 2 (PC): game screen for the Chao garden file (stat levels and points of every Chao), with its checksum rewritten
- Tales of Vesperia: Definitive Edition (PC): game screen (gald and the level, experience, HP, TP and stats of all nine characters)
- New game screens: Darksiders Genesis (souls, coins, keys, shards), Journey to the Savage Planet (stash resources with Max crafting materials), My Hero One's Justice and My Hero One's Justice 2 (cash, hero coins), Vikings: Wolves of Midgard (gold, iron, steel, blood, jotunjarn and ores)
- Batman: Arkham City (PC): game screen (difficulty, experience, player level, upgrades bought for Batman and Catwoman); the LZO-packed save is unpacked and packed again
- Grand Theft Auto: San Andreas (2005 PC release): game screen (money, health and armor, body stats, skills, weapons and ammunition, story line, options), with the save checksum rewritten
- Single Player Tarkov (SPT): game screen for profile files (level, experience, skills, trader standing, health, currency stacks)
- New game screens for ten Assassin's Creed games, from Brotherhood to Odyssey: money, Creed Points (Unity), XP and ability points (Origins, Odyssey), crafting materials with Max crafting materials (Black Flag, Rogue, Syndicate, Odyssey), trade goods and ammunition. The saves are unscrambled with the key worked out from the file, unpacked and packed again with their block checks (not yet tried in the games)
- New game screens: The Elder Scrolls III: Morrowind and IV: Oblivion (level, gold, attributes, skills, vitals; Morrowind also the inventory counts)
- New game screens: WorldBox (units and village resources), Core Keeper (skills, coins, inventory), HoloCure (HoloCoins and upgrades), Potion Craft (gold, popularity, karma, inventory with Max crafting materials), World War Z (currency, coins, level, class perks), Roboquest (wrenches, difficulty, statistics), Car Mechanic Simulator 2018 (money, level, experience), The Long Drive (vitals and trip), and Total War: Warhammer, Attila and Thrones of Britannia treasuries
- More game screens: Borderlands (2009) and the Enhanced edition (level, experience, skill points, money, skills, ammunition), The Evil Within (green gel, trap parts, weapon and stat upgrades from inventory.zwei), Wreckfest (credits, experience and fame from career.cres, unpacked and packed again), Generation Zero (skill points, experience, level, inventory amounts, resource store and safehouse storage with Max crafting materials), NieR Replicant (health, level, experience, money, recovery items, materials with Max crafting materials per slot), Devil May Cry 1 from the HD Collection (orbs and play time per slot) and Mewgenics (house gold and food, adventure coins, day); Bloodstained: Ritual of the Night saves now open (decoded, with its trailing block kept) in Values and Tree
- JSON saves with bare `Infinity` or `NaN` numbers (Unity and .NET writers) now open and are written back unchanged
- Find my game's saves: long game names are shortened (full name on hover) so the badge never overlaps the details, the right-hand side scrolls as a whole and starts at the top for each game, and the window uses its full width
- Scrolling a list or table inside a screen stays in that list at its top and bottom instead of scrolling the page; column headings of the long tables (XCOM, XCOM 2, Final Fantasy VII Remake materia) stay visible while scrolling
- Number boxes in tables are wide enough for their value (decimals were cut off); on phone-width screens the -/+ buttons give way instead of running off the edge
- Fixed a crash when opening some text saves with non-English characters near the start
- West of Loathing: game screen (meat, level, experience, stats, skills and perks, inventory)
- Shape of Dreams: game screen (stardust, mastery and traveler records)
- Travellers Rest: game screen (money, upgrades, inventory)
- Youtubers Life: game screen (money, channel, youtuber skills)
- Luck be a Landlord: game screen (coins, tokens, rent); Godot JSON-lines saves open as one object keyed by each node's path
- Touhou Luna Nights: game screen (gold, level, gems, upgrades); saves made of one base64 value per line open as a list
- EVERSPACE: game screen (DNA fragments, records, perks)
- Streets of Rogue: game screen (nuggets, unlocks)
- Ys VIII: Lacrimosa of DANA: game screen (play time, equipment, item counts)
- Dragon Quest XI S: game screen (party stats, item bags), with the save's encryption opened
- Dynasty Warriors 8: Xtreme Legends, One Piece: Pirate Warriors 3 and 4, Persona 5 Strikers: game screens, with the Koei Tecmo save encryption opened
- Torchlight III: game screen (character traits, gear), with the JSON text inside the Unreal save opened
- Dragon Ball Xenoverse: game screen (custom characters)
- Chronicon: game screen (level, free skill and mastery points), with the hex-text character opened as bytes
- Sword Art Online: Fatal Bullet: game screen (inventory quantities), with the encrypted save opened and re-sealed
- Gothic and Gothic II: game screen (script variables and dialogue flags)
- Dragon Quest Builders 2: game screen (builder stats, mini medals, pockets and bag), with the compressed data opened and re-sealed

## 0.6.1

- Windows: the C runtime is now built into the program, so it starts without the Visual C++ Redistributable (fixes the "VCRUNTIME140.dll was not found" error)
- Download file names now include the version, e.g. UniversalSaveEditor-0.6.1-windows-x64.exe

## 0.6.0

### New game screens
- Final Fantasy XVI: gil, Ability Points, consumables and materials with Max crafting materials (the save's archive is read with the compression dictionary from your own installed game)
- Dishonored (coins), Dishonored 2 (Raw Whalebone with Max crafting materials, runes, ammunition) and Deathloop (Residuum, stress, loop progress)
- Pokémon Infinite Fusion: trainer, badges, difficulty, party and PC Pokémon, bag (not yet tried in the game)
- Red Dead Redemption and Red Dead Redemption 2 (PC): money, honor, cores, camp funds, satchel supplies, horses, statistics; RDR2 also has story challenge ranks and compendium ticks
- Final Fantasy Resonance: Gil, characters, Visions, items and artifacts
- Atomic Heart (crafting materials with a max button, neuropolymer), Deus Ex: Mankind Divided (credits, Praxis kits, crafting parts, ammo), World of Horror (unlocks, last run, settings)
- Story flags and choices for Baldur's Gate 3, Pathfinder: Kingmaker, The Witcher 3, Fallout: New Vegas and Golden Sun

### Experimental
- Red Dead Redemption 2: adding supplies the satchel lacks is experimental and has not been tried in the game yet; keep a backup

### Value editing and game screens
- Numbers show the range the save accepts on hover; a right-click menu on every value (copy value, name or path, set to minimum or maximum, revert, show in Tree)
- The Tree has a filter box; game screens have a find box (Ctrl+F) that highlights and scrolls to a label
- Save-wide buttons on game screens are gathered in one Quick actions strip
- Short risk notes on hover and in the save confirmation; read-only values give a reason
- Slay the Spire's add box offers the ids already in the save
- Skyrim and Fallout 4 saves show character, level and place in the save finder

### Clearer interface
- Arrows, the remove button and other symbols no longer show as empty boxes on Linux, macOS, Android and in the browser
- Escape closes dialogs; File has Reload from disk and Close file; View has Review changes before saving
- The Backups menu says each thing once (Back up now, Backups for this save, Quick restore); the backup folder setting is in the save manager
- Menu shortcuts line up; recent files show their folder; greyed-out Tools items say why
- The file-type chip names the game on saves that are plain JSON or XML
- Long notes about an opened file show under the toolbar instead of being cut off in the status bar
- A reminder about Steam Cloud after saving a file in a Steam userdata folder
- Number boxes fit their value; big numbers lose the -/+ buttons, Shift and Ctrl step by 10 and 100
- Game screens no longer run slightly wider than the window; Slay the Spire and Diablo II columns are labelled

### Tools for games without a screen
- Bulk edit in the Values tab: tick values (or all a search shows) and set, add to or multiply them in one step
- Presets: keep a set of changes and make them again on another save or slot (Tools › Presets, Changes panel)
- Names given to values in the hex view can become a template for every save of the game, and can be exported and imported
- The hex view has a step-by-step guide for finding a value, and the search can refine by itself each time the game saves
- Copy the list of changes as text (Edit menu, Changes panel)

### Safer saving and an easier start
- Saving stops and asks when the save changed on disk since it was opened (the game autosaved, cloud sync): reload, save anyway (the newer file is backed up first) or cancel; a notice with Reload appears when you come back to the window
- Failures show as a red banner under the toolbar until dismissed
- Japanese, Chinese and Korean names show with the system's fonts on Windows, Linux and macOS
- The start page lists Your games: games whose saves were found, newest first, one click to open; Find my game's saves is the main button
- Find my game's saves lists the games found on this computer first, can hide the rest, and shows a Screen, Values or Hex badge per game with a filter
- Edit › Preferences (theme, zoom, review before saving, the hex-view guide, how many automatic backups to keep, backup folder) and a Help menu (shortcuts, supported games, website, Report a problem, Request a game, licences, About)
- Issue forms on GitHub for save problems and game requests
- Dialogs and the save manager fit narrow windows

### Languages
- The interface is in English, German, Spanish, French, Brazilian Portuguese, Japanese and Simplified Chinese; it follows the system language, and Edit › Preferences › Language changes it

### Several saves at once
- Open several saves in tabs; each keeps its own changes and undo history
- Copy a value or a whole group (an inventory, a character) and paste it into another save

### Installing
- Scoop manifest for Windows, and a note on the SmartScreen warning for new downloads

## 0.5.1

### New game screens
- Sleeping Dogs: Definitive Edition, Horizon Zero Dawn, Judgment, Yakuza Kiwami 3 and Dark Ties
- Subnautica 2, Saints Row IV, Slime Rancher, Stellar Blade, Pragmata, Wandering Sword, Fable: The Lost Chapters, The Planet Crafter
- Stud wallets for more LEGO games: Marvel Super Heroes 1 and 2, The Lord of the Rings, The Hobbit, Jurassic World, Batman 3, The Force Awakens, The Incredibles, DC Super-Villains

### Existing games expanded
- Watch Dogs 2 opens every progress save; Jedi: Survivor uses the Jedi screen
- Death Stranding shows and edits likes received
- Onimusha: Max crafting materials only changes the save slot on screen

### Features
- Releases include SHA256SUMS.txt for checking downloads

## 0.5.0

### New game screens
- Metal Gear Solid V: The Phantom Pain (decrypts the save; GMP, heroism, staff, resources, Mother Base and FOB), Tiny Tina's Wonderlands, Euro Truck Simulator 2 and American Truck Simulator, X4: Foundations, Total War: Warhammer II and Troy
- Mortal Shell and Mortal Shell II, Shadow Warrior 2, Everspace 2, Moonlighter, Wasteland 2, ATOM RPG, Ori and the Blind Forest, OMORI, ENDER LILIES
- Warhammer 40,000: Mechanicus, Mutant Year Zero, Car Mechanic Simulator 2021, Medieval Dynasty, For The King, Resident Evil 5 (Steam), Anno 1800
- Lies of P, Tokyo Xtreme Racer, Dave the Diver, My Summer Car, ULTRAKILL, RPG Maker XP games, Divinity: Original Sin Enhanced Edition
- Alpha Protocol, Drakensang, Salt and Sanctuary, theHunter: Call of the Wild, BeamNG.drive, The Forest
- Yakuza 3, 4 and 5 Remastered, Like a Dragon: Ishin! and Gaiden, Tyranny, Death Stranding facility stocks
- Star Wars Jedi: Fallen Order, Saints Row: The Third, Watch Dogs 2
- LEGO stud counters and wallets: Star Wars (2005), The Complete Saga, III, The Skywalker Saga, Batman 1 and 2, Indiana Jones 1 and 2, Harry Potter 1-7, Pirates of the Caribbean, City Undercover

### Existing games expanded
- Difficulty selectors for Minecraft, Grim Dawn, Torchlight II and Half-Life
- Baldur's Gate 3 gold and item stack sizes; KOTOR story variables; Original Sin item stacks
- Max crafting materials now uses each game's own cap (Death Stranding 2, Metro Exodus, MGSV plants); Onimusha upgrade materials added to slots that lack them
- Unreal saves: UE 4.25 timestamp headers and UE 5 save-object lists; UTF-16 JSON without a byte-order mark

### Features
- The save manager has its own Backups menu

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
