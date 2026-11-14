//! The games the editor knows, where each keeps its saves, and finding them on this
//! computer (the "Select game" list).

use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// A folder to look in, relative to one of the usual places.
#[derive(Clone, Copy, Debug)]
pub enum Base {
    /// %APPDATA% (Roaming).
    AppData,
    /// %LOCALAPPDATA%.
    LocalAppData,
    /// %USERPROFILE%\AppData\LocalLow (Unity games).
    LocalLow,
    /// The Documents folder (wherever Windows has it).
    Documents,
    /// %USERPROFILE%\Saved Games.
    SavedGames,
    /// Steam\userdata\<account>\<app id>\ for this app id.
    SteamUser(&'static str),
    /// A game's install folder in any Steam library.
    SteamGame(&'static str),
    /// %ProgramFiles(x86)% (Ubisoft Connect keeps saves there).
    ProgramFilesX86,
    /// %ProgramData% (old PopCap games keep profiles there).
    ProgramData,
    /// Unity PlayerPrefs: the exported registry values (Windows), `~/.config/unity3d` (Linux)
    /// and `~/Library/Preferences` (macOS).
    UnityPrefs,
    /// The Xbox app / PC Game Pass save container of this Store package (`*`: every package).
    GamePass(&'static str),
}

#[derive(Clone, Copy, Debug)]
pub struct GameInfo {
    pub name: &'static str,
    /// What the editor can do with it.
    pub support: &'static str,
    /// The usual folder, as people would write it.
    pub folder: &'static str,
    /// What the files are called.
    pub files: &'static str,
    /// Where to look; `*` in a folder name matches any folder (account ids, profiles).
    pub places: &'static [(Base, &'static str)],
    /// File names to list (`*` and `?` wildcards, any case).
    pub patterns: &'static [&'static str],
    /// How many folders below each place to search.
    pub depth: usize,
}

const FULL: &str = "Game screen";
const KENSHI: &str = "Game screen: the faction's cats (quick.save) and, in each squad's .platoon file, every member's attributes and skills, the health of each body part, blood, hunger, bleeding, lost or robotic limbs and the dead, unconscious and coma flags, with a button to max everyone's stats. Checked on public saves, not yet tried in the game";
const VALUES: &str = "Values and Tree";
const HEX: &str = "Hex view and Value search (find a number you know, then change it)";
const METRO: &str = "Game screen for the level saves (.player); the auto save is unpacked for Value search. Metro Exodus has a Max crafting materials button that fills metal and chemicals to 300 (the Reader and Ranger Hardcore limit; Normal allows more)";
const RGG_Y6: &str = "Game screen: money, HP and Kiryu's five experience pools; decrypted, with its checksum kept right. Checked on real saves, not yet tried in the game";
const RGG_UNTESTED: &str = "Decrypted, with its checksum kept right (no save of this game has been tried yet); values in the Tree";
const HEX_UNPACKED: &str = "Unpacked for Value search (find a number you know, then change it); packed again on save";
const XCOM2: &str = "Game screen: HQ resources and soldiers (rank, kills, missions, ability points, stats including armour and shield); unpacked and packed again on save. Not yet tried in the game";
const CIV6: &str = "Game screen: settings, mods and each civilisation (human or AI, whose turn it is); gold and the map are in the packed part and aren't mapped";
const BORDERLANDS: &str = "Game screen: level, experience, skill points, money, Eridium (Moonstone in The Pre-Sequel; Seraph crystals and Torgue tokens in Borderlands 2), ammo and ammo SDUs; in Borderlands 2 and The Pre-Sequel also the backpack and bank SDUs (with the room they give), weapon slots and playthroughs completed (which opens True and Ultimate Vault Hunter Mode); gear is listed (level, slot), not edited. Packing and checksums are redone on save. Edited saves not yet tried in the game";
const BORDERLANDS3: &str = "Game screen: level and experience, skill points, money, Eridium, ammo and SDU levels; gear is listed (slot, favourite/junk), not edited. Encrypted again on save. Edited saves not yet tried in the game";
const WONDERLANDS: &str = "Game screen: level and experience (to 40), skill points, gold, moon orbs and ammo; gear is listed (slot, favourite/junk), not edited. Encrypted again on save. Edited saves not yet tried in the game";
const TRUCKSIM: &str = "Game screen: money, loan limit, experience, skills and ADR classes, each vehicle's fuel, odometer and wear, with buttons to max skills, fill every tank and repair everything. Packed (ScsC) saves are unpacked and packed again with their checksum. Not yet tried on a save of the game itself";
const MGSV: &str = "Game screen: GMP (and the FOB wallet), heroism and demon points, every Mother Base staff member (rank, skill, section, morale, proficiency, stat bonus; buttons to promote everyone to S++ and max morale), resource stockpiles with a Max crafting materials button, a button to open every Mother Base section, and the Mother Base and lifetime-statistics counters; every other variable in the save is in the Tree under the game's own variable names. The save's encryption and checksum are redone on write. Tried on a real save";
const X4: &str = "Game screen: the player's credits, written to all three places the save keeps them (info line, player faction account, statistics); everything else in the XML tree. Opens the .xml.gz directly and packs it again. Not yet tried on a save of the game itself";
const TOTALWAR: &str = "Game screen: every faction's treasury, yours first, with a button to fill it. The campaign inside the save is unpacked, edited with every size above it recomputed, and packed again (the file grows). Read and round-tripped on Warhammer II and Troy campaign saves; not yet tried in the game";
const FALLOUT12: &str = "Game screen: S.P.E.C.I.A.L., hit points, level, experience, skill points, skills, traits, tagged skills, perk ranks, radiation, poison, kills, bottle caps and inventory counts. Edits haven't been tried in the game yet";
const DEUS_EX: &str = "Game screen for JC Denton: credits, skill points, bioelectric energy, health per body part, ammo, medkits and other item counts, skill and augmentation levels (values the game left out can't be set). Edits haven't been tried in the game yet";
const KOTOR: &str = "Game screen: the player and party members (attributes, skills, HP and Force points, XP) and how many of each item the party carries from SAVEGAME.sav; every story variable (the global flags and numbers: choices and quest steps, by the game's own names) from GLOBALVARS.res; credits, party XP and (KOTOR II) influence, components and chemicals from PARTYTABLE.res, with a Max crafting materials button that fills KOTOR II's components and chemicals to 9999 (an estimate). Not yet tried in the game";
const COTL: &str = "Game screen: level, XP, hearts and fervour, cult faith, level and doctrine levels, and the inventory by item name, with a Max crafting materials button that fills lumber, stone, bones, grass, planks, stone blocks, gold, crystal shards, spider silk and shells to 999 (an estimate). Checked on a public sample save, not yet tried in the game";
const FALLOUT_SHELTER: &str = "Game screen: caps, food, power, water, Stimpaks, RadAway, Lunchboxes, Mr. Handy, pet carriers and Nuka-Cola Quantum, and every dweller's level, happiness, health, radiation and S.P.E.C.I.A.L. Decrypted and encrypted again on save. Checked on a public sample save, not yet tried in the game";
const DRG: &str = "Game screen: credits, perk points, vanity mastery and forge XP, each dwarf's XP and retirements, and the minerals and brewing ingredients, with a Max crafting materials button that fills Bismor, Magnite, Croppa, Umanite, Jadiz, Enor, Phazyonite and the brewing ingredients to 9999 (an estimate). Checked on a public sample save, not yet tried in the game";
const SCHEDULE_I: &str = "Game screens for the save's own files: Money.json (bank balance, net worth, lifetime earnings), Rank.json (rank, tier, XP), Time.json (days, time of day) and Game.json (organisation name, seed, console). Checked on a public sample save, not yet tried in the game";
const SOTF: &str = "Game screens for PlayerStateSaveData.json (health, stamina, hydration, rest, strength) and PlayerInventorySaveData.json (items carried by name), with a Max crafting materials button that fills logs, sticks, rocks, bones, cloth, wire, rope, leaves and other materials to the game's own carrying limits. The JSON held inside the save's strings is opened and written back as strings. Checked on a public sample save, not yet tried in the game";
const DEATH_MUST_DIE: &str = "Game screen: gold, runs, wins, streaks and play time. The JSON held inside the save's strings is opened and written back as strings. Checked on a public sample save, not yet tried in the game";
const CUPHEAD: &str = "Game screen: coins and deaths for both players, Djimmi's wishes and the unlocks (expert mode, filters, Boatman). Checked on a public sample save, not yet tried in the game";
const ANTIMATTER: &str = "Game screen: antimatter, infinity points, eternity points, reality machines and the counters behind them (galaxies, boosts, replicanti, dilation, perk points). Opens the text from Options > Saving > Export save as file (PC and mobile formats). Checked on public sample saves, not yet tried in the game";
const P3R: &str = "Game screen: yen, each party member's level, experience, HP and SP, the three social stats and every Social Link rank. Opens Steam saves (decrypted and re-encrypted for you). Checked on public sample saves, not yet tried in the game";
const HL1: &str = "Game screen: health, armour, flashlight, ammo by type and each weapon's magazine, changed in place. Values the game left out because they're zero (no armour, an empty magazine) can't be set. Not yet tried in the game; the 25th-anniversary build hasn't been checked";
const HL2: &str = "Game screen: health, armour, suit power, ammo by type and each weapon's magazines, changed in place. Values the game left out because they're zero (no armour, an empty magazine) can't be set. Not yet tried in the game; the 20th-anniversary build hasn't been checked";
const HL2_EPISODES: &str = "Game screen: health, armour, suit power, ammo by type and each weapon's magazines, changed in place. Values the game left out because they're zero (no armour, an empty magazine) can't be set. Not yet tried in the game; the 20th-anniversary build and Lost Coast saves haven't been checked";
const BLACK_MESA: &str = "Game screen: health, armour, ammo by type (9mm, .357, buckshot, uranium and the rest) and each weapon's magazine, changed in place. Values the game left out because they're zero (no armour, an empty magazine) can't be set. Not yet tried in the game";
const HL_ALYX: &str = "Game screen: health, resin (with a Max crafting materials button that fills it to 999, an estimate), and pistol, shotgun and rapidfire ammo in the backpack, changed in place; the weapons Alyx has are listed. The Load menu keeps showing the old numbers until the game saves again. Not yet tried in the game";
const STARBOUND: &str = "Game screen: pixels and other currencies, item counts in every bag with a Max crafting materials button (ores, bars, cloth and other base-game materials to 1000), health, energy and food, ship level, crew size and fuel tank, deaths and play time; each value is rewritten on its own and the rest of the file kept. Early-access characters (before 1.0) open with what their older layout has. Checked on a real character save, not yet tried in the game";
const FFRESONANCE: &str = "Game screen: Gil and play time; each character's level (shown from the experience), experience, HP, MP, enemies defeated and bonus stats; each Vision's awakening and mastery points; item quantities with a Max consumables button (99 each, key items left alone); the artifacts you hold, switched on or off. Decrypted and encrypted again with the checksum rewritten. Checked on a synthetic test save built from the public format notes, not yet on a real save or in the game";
const EXPEDITION33: &str = "Game screen: gold; each party member's level, experience, action points, health, attribute points and Lumina from Colours of Lumina; each weapon's level; item counts (cosmetics left out), with a Max crafting materials button that fills Chroma Catalysts to 999 (an estimate). Checked on a real save, not yet tried in the game";
const FALLOUT3NV: &str = "Game screen: bottle caps, experience, karma, S.P.E.C.I.A.L. and the points added to each skill; the level, place and play time are shown from the save. Checked on real saves, not yet tried in the game";
const WASTELAND3: &str = "Game screen: money, each ranger's level, XP, hit points, attributes, skills by name and unspent points, item quantities, fame and faction reputation; unpacked (LZF) and packed again on save. Not yet tried in the game";
const VALHEIM: &str = "Game screen for characters saved with profile versions 33 to 37: health, stamina, skills, item stacks (gold is Coins), quality and durability, with the SHA-512 written again, and a Max crafting materials button that fills the wood, stone, ores, metals, hides and other plain materials you carry to their stack size, leaving boss drops, summoning items and key fragments alone; newer characters open read-only. Not yet tried in the game";
const TERRARIA_PLR: &str = "Game screen: life and mana, permanent upgrades, inventory, equipment and storage as item grids, with a Max crafting materials button (characters from 1.4.4 on) that fills the ores, bars, gems, woods, monster drops and other materials you already have to 9999, leaving critters, fish and boss summons alone. The button hasn't been tried in the game yet";
const STARDEW: &str = "Game screen: gold, Qi Gems, Golden Walnuts, health, energy, skills and farm details, with a Max crafting materials button that fills the wood, stone, fiber, clay, ores, bars and monster loot in the backpack to 999, leaving quest items, minerals and artifacts alone (checked on public 1.5 saves; not yet tried in the game)";
const MINECRAFT: &str = "Game screen: health, food, XP, game mode, world settings, inventory and ender chest, with a Max crafting materials button that fills the ingots, raw ores, gems, string, leather and other materials already carried to 64, leaving ender pearls, eyes of ender and other progression items alone. The button hasn't been tried in the game yet";
const PDX_BINARY: &str = "Binary saves (the game's default, and ironman) are edited in place and stay binary; the game doesn't publish its field names, so most fields show as numbers unless you add a token file. Debug-mode text saves open with every name";
const PDX_EU5: &str = "Binary saves (the game's default, and ironman) are edited in place and stay binary, with names read from the save's own string table. Debug-mode text saves open too";
const IE_BG: &str = "Game screen: party gold and reputation; each party member's experience, hit points, levels, abilities, alignment and item stacks or charges (BALDUR.gam); every global variable by name (quest stages, who has joined, story choices)";
const IE_IWD: &str = "Game screen: party gold and reputation; each party member's experience, hit points, levels, abilities, alignment and item stacks or charges (BALDUR.gam, the same layout as Baldur's Gate: EE). No Icewind Dale: EE save has been tried yet";
const IE_PST: &str = "Game screen: the party's coppers; the Nameless One's and each companion's experience, level, hit points, abilities, alignment and item stacks (BALDUR.gam, same layout as the other Enhanced Editions); every global variable by name (quest stages, story choices). Checked on real saves, not yet tried in the game";
const DAO: &str = "Game screen: party money, the Warden's and followers' level, experience, attributes and unspent points, approval, backpack stack sizes. Tried on saves from one playthrough; Awakening saves not yet tried";
const GOLDEN_SUN: &str = "Game screen: every file in the flash with its coins, each party member's level, experience, HP/PP, base stats and items, and the Djinn per element (give one to a character, set it or put it on standby, or take it away); in Golden Sun, the story choices the password carries into The Lost Age; checksums kept right. Not yet tried in the game";
const SNES_FOLDER: &str = r"The emulator's save folder: Snes9x keeps <ROM name>.srm in Saves\, bsnes and higan next to the ROM, RetroArch in saves\";
const SWITCH_FOLDER: &str = r"An emulator's save folder or a dump from your own console: yuzu-style emulators keep it under nand\user\save\0000000000000000\<profile>\<title id>, Ryujinx under bis\user\save\<number>\0";
const N64_FOLDER: &str = r"The emulator's save folder: Project64 keeps <ROM name>.eep in Save\, Mupen64Plus and RetroArch next to their data (RetroArch: saves\)";
const GBA_FOLDER: &str = r"The emulator's save folder: mGBA and VisualBoyAdvance-M keep <ROM name>.sav next to the ROM (or in the save folder set in their options)";

macro_rules! game {
    ($name:expr, $support:expr, $folder:expr, $files:expr, [$($place:expr),*], [$($pat:expr),*], $depth:expr) => {
        GameInfo { name: $name, support: $support, folder: $folder, files: $files, places: &[$($place),*], patterns: &[$($pat),*], depth: $depth }
    };
}

use Base::*;

/// Alphabetical.
pub const GAMES: &[GameInfo] = &[
    game!("A Total War Saga: Thrones of Britannia", TOTALWAR, r"%APPDATA%\The Creative Assembly\ThronesofBritannia\save_games", r"<name>.save", [(AppData, r"The Creative Assembly\ThronesofBritannia\save_games")], ["*.save"], 1),
    game!("Abiotic Factor", "Opens as an Unreal save: every property by name, numbers and text changed in place (no save of this game has been tried yet)", r"%LOCALAPPDATA%\AbioticFactor\Saved\SaveGames\<Steam id>\Worlds", r"<world>\*.sav", [(LocalAppData, r"AbioticFactor\Saved\SaveGames\*\Worlds")], ["*.sav"], 3),
    game!("Ace Combat 8: Wings of Theve", "Opens as an Unreal save: the packed save data is unpacked, every property shown by name in sections, numbers and text changed in place, and the save's checksum written again (no save of this game has been tried yet)", r"%LOCALAPPDATA%\BANDAI NAMCO Entertainment\ACE COMBAT 8\Saved\SaveGames", "Campaign.sav, System.sav", [(LocalAppData, r"BANDAI NAMCO Entertainment\ACE COMBAT 8\Saved\SaveGames")], ["Campaign.sav", "System.sav"], 2),
    game!("Against the Storm", "Opens as JSON: goods in storage, reputation and impatience, hostility, the year and season, orders and hearths in Save.save; food stockpiles, machinery, artifacts and the Smoldering City level in MetaSave.save. Every value by name; the game's own backups are left alone", r"%USERPROFILE%\AppData\LocalLow\Eremite Games\Against the Storm", r"Save.save (the settlement), MetaSave.save (world map)", [(LocalLow, r"Eremite Games\Against the Storm")], ["Save.save", "MetaSave.save"], 1),
    game!("Alpha Protocol", "Game screen: money, level, experience, skill points, skill ranks, ammunition and gadget counts from a checkpoint. Checked on a public save, not yet tried in the game", r"Documents\Alpha Protocol\Checkpoints", "<name>.sav", [(Documents, r"Alpha Protocol\Checkpoints")], ["*.sav"], 1),
    game!("American Truck Simulator", TRUCKSIM, r"Documents\American Truck Simulator\profiles\<profile>\save\<save name>", "game.sii (a packed ScsC file or plain text)", [(Documents, r"American Truck Simulator\profiles")], ["game.sii"], 4),
    game!("Anno 1800", "Game screen: your company's coins (the load screen's total is kept equal) and the rest of its storage by GUID; the save archive and its game data are unpacked and packed again. Checked on a public save, not yet tried in the game", r"Documents\Anno 1800\accounts\<account id>\<profile>\savegame", "<name>.a7s", [(Documents, r"Anno 1800\accounts")], ["*.a7s"], 4),
    game!("Antimatter Dimensions", ANTIMATTER, r"wherever you saved the export (Options > Saving > Export save as file)", r"the exported .txt save", [(Documents, "Antimatter Dimensions")], ["*.txt"], 1),
    game!("Assassin's Creed (Ubisoft Connect games)", "Hex view and Value search for the games without their own screen (Valhalla and later need a decoder that isn't built yet; Brotherhood to Odyssey have game screens)", r"Ubisoft Connect: <Program Files (x86)>\Ubisoft\Ubisoft Game Launcher\savegames\<user>\<game number>", r"<number>.save (zlib-packed ones end in .save.z)", [(ProgramFilesX86, r"Ubisoft\Ubisoft Game Launcher\savegames\*\*")], ["*.save", "*.save.z"], 1),
    game!("Assassin's Creed Brotherhood", "Game screen: Ezio's florins, smoke bombs, throwing knives, poison and bullets. The save is unscrambled (the account's key is worked out from the file), unpacked and packed again with its block checks. Checked on public saves, not yet tried in the game", r"%USERPROFILE%\Saved Games\Assassin's Creed Brotherhood\SAVES (or the Ubisoft Connect savegames folder, game 26)", "ACBROTHERHOODSAVEGAME0.SAV", [(SavedGames, r"Assassin's Creed Brotherhood\SAVES"), (ProgramFilesX86, r"Ubisoft\Ubisoft Game Launcher\savegames\*\26")], ["*.SAV", "*.save"], 1),
    game!("Assassin's Creed III Remastered", "Game screen: money, smoke bombs, poison, rope darts, arrows, bullets and snares. The save is unscrambled (the account's key is worked out from the file), unpacked and packed again with its block checks. Checked on public saves, not yet tried in the game. The save ends with a 4-byte value that isn't understood yet and is kept as it is", r"Ubisoft Connect: <Program Files (x86)>\Ubisoft\Ubisoft Game Launcher\savegames\<user>\5183 (Steam: 5184)", "<number>.save", [(ProgramFilesX86, r"Ubisoft\Ubisoft Game Launcher\savegames\*\5183"), (ProgramFilesX86, r"Ubisoft\Ubisoft Game Launcher\savegames\*\5184")], ["*.save"], 1),
    game!("Assassin's Creed IV Black Flag", "Game screen: reales, the Jackdaw's wood, metal and cloth (with a Max crafting materials button that fills the hold to 2,000), rum and sugar, and ammunition (smoke bombs, rope darts, bullets, mortar shots, heavy shot, harpoons…). The save is unscrambled (the account's key is worked out from the file), unpacked and packed again with its block checks. Checked on public saves, not yet tried in the game", r"Ubisoft Connect: <Program Files (x86)>\Ubisoft\Ubisoft Game Launcher\savegames\<user>\273 (Steam: 437)", "<number>.save", [(ProgramFilesX86, r"Ubisoft\Ubisoft Game Launcher\savegames\*\273"), (ProgramFilesX86, r"Ubisoft\Ubisoft Game Launcher\savegames\*\437")], ["*.save"], 1),
    game!("Assassin's Creed Liberation HD", "Game screen: Aveline's money, smoke bombs, throwing knives, rope darts and bullets. The save is unscrambled (the account's key is worked out from the file), unpacked and packed again with its block checks. Checked on public saves, not yet tried in the game", r"Ubisoft Connect: <Program Files (x86)>\Ubisoft\Ubisoft Game Launcher\savegames\<user>\625 or 632", "<number>.save", [(ProgramFilesX86, r"Ubisoft\Ubisoft Game Launcher\savegames\*\625"), (ProgramFilesX86, r"Ubisoft\Ubisoft Game Launcher\savegames\*\632")], ["*.save"], 1),
    game!("Assassin's Creed Odyssey", "Game screen: drachmae, XP and ability points, and the six crafting materials (olive wood, iron, soft leather, obsidian glass, precious gems, ancient tablets) with a Max crafting materials button (99,999, an estimate). The save is unscrambled (the account's key is worked out from the file), unpacked and packed again with its block checks. Checked on public saves, not yet tried in the game", r"Ubisoft Connect: <Program Files (x86)>\Ubisoft\Ubisoft Game Launcher\savegames\<user>\5059 (Steam: 5092)", "<number>.save", [(ProgramFilesX86, r"Ubisoft\Ubisoft Game Launcher\savegames\*\5059"), (ProgramFilesX86, r"Ubisoft\Ubisoft Game Launcher\savegames\*\5092")], ["*.save"], 1),
    game!("Assassin's Creed Origins", "Game screen: drachmae, XP, ability points and hard leather. The save is unscrambled (the account's key is worked out from the file), unpacked and packed again with its block checks. Checked on public saves, not yet tried in the game. The save ends with a 4-byte value that isn't understood yet and is kept as it is", r"Ubisoft Connect: <Program Files (x86)>\Ubisoft\Ubisoft Game Launcher\savegames\<user>\3539 (Steam: 4923)", "<number>.save", [(ProgramFilesX86, r"Ubisoft\Ubisoft Game Launcher\savegames\*\3539"), (ProgramFilesX86, r"Ubisoft\Ubisoft Game Launcher\savegames\*\4923")], ["*.save"], 1),
    game!("Assassin's Creed Revelations", "Game screen: Ezio's akçe, smoke bombs, throwing knives, poison and bullets. The save is unscrambled (the account's key is worked out from the file), unpacked and packed again with its block checks. Checked on public saves, not yet tried in the game", r"Ubisoft Connect: <Program Files (x86)>\Ubisoft\Ubisoft Game Launcher\savegames\<user>\40", "<number>.save", [(ProgramFilesX86, r"Ubisoft\Ubisoft Game Launcher\savegames\*\40")], ["*.save"], 1),
    game!("Assassin's Creed Rogue", "Game screen: reales, the Morrigan's wood, metal, cloth and stone (with a Max crafting materials button that fills the hold to 2,500), tobacco, and ammunition (smoke bombs, rope darts, bullets, mortar shots, heavy shot, burning oil…). The save is unscrambled (the account's key is worked out from the file), unpacked and packed again with its block checks. Checked on public saves, not yet tried in the game", r"Ubisoft Connect: <Program Files (x86)>\Ubisoft\Ubisoft Game Launcher\savegames\<user>\895 (Steam: 934)", "<number>.save", [(ProgramFilesX86, r"Ubisoft\Ubisoft Game Launcher\savegames\*\895"), (ProgramFilesX86, r"Ubisoft\Ubisoft Game Launcher\savegames\*\934")], ["*.save"], 1),
    game!("Assassin's Creed Syndicate", "Game screen: pounds, crafting materials (leather, metal, chemicals, silk) with a Max crafting materials button (9,999, an estimate), smoke bombs, darts, medicine, bullets and voltaic bombs. The save is unscrambled (the account's key is worked out from the file), unpacked and packed again with its block checks. Checked on public saves, not yet tried in the game. The save ends with a 4-byte value that isn't understood yet and is kept as it is", r"Ubisoft Connect: <Program Files (x86)>\Ubisoft\Ubisoft Game Launcher\savegames\<user>\1875 (Steam: 1957)", "<number>.save", [(ProgramFilesX86, r"Ubisoft\Ubisoft Game Launcher\savegames\*\1875"), (ProgramFilesX86, r"Ubisoft\Ubisoft Game Launcher\savegames\*\1957")], ["*.save"], 1),
    game!("Assassin's Creed Unity", "Game screen: francs and Creed Points, smoke bombs, berserk darts, medicine, bullets, cherry bombs and stun grenades. The save is unscrambled (the account's key is worked out from the file), unpacked and packed again with its block checks. Checked on public saves, not yet tried in the game", r"Ubisoft Connect: <Program Files (x86)>\Ubisoft\Ubisoft Game Launcher\savegames\<user>\720 (Steam: 857)", "<number>.save", [(ProgramFilesX86, r"Ubisoft\Ubisoft Game Launcher\savegames\*\720"), (ProgramFilesX86, r"Ubisoft\Ubisoft Game Launcher\savegames\*\857")], ["*.save"], 1),
    game!("Astroneer", HEX_UNPACKED, r"%LOCALAPPDATA%\Astro\Saved\SaveGames (Steam version; Microsoft Store: the Xbox app container)", "SAVE_<n>.savegame", [(LocalAppData, r"Astro\Saved\SaveGames")], ["*.savegame"], 1),
    game!("ATOM RPG", "Game screen: every party member's level, experience, attributes, skills and free points, and each inventory (rubles are the Money item). The save's gzip packing is opened and closed for you. Checked on a public save, not yet tried in the game", r"%USERPROFILE%\AppData\LocalLow\AtomTeam\Atom", "Save_<n>_v10.as (one per slot)", [(LocalLow, r"AtomTeam\Atom")], ["Save_*_v10.as"], 1),
    game!("Atomic Heart", "Game screen: your metal parts, synthetic material, biomaterial, superconductors, chemicals, energy cells and microelectronics with a Max crafting materials button, plus neuropolymer. The save's Oodle data is unpacked and packed again (needs the Oodle library of any game on this PC) and its SHA-1 is made again. Checked on public and real saves, not yet tried in the game", r"%LOCALAPPDATA%\AtomicHeart\Saved\SaveGames", "<n>.sav (one per slot)", [(LocalAppData, r"AtomicHeart\Saved\SaveGames")], ["*.sav"], 1),
    game!("Attack on Titan 2", HEX, r"Documents\KoeiTecmo\Attack on Titan2_AOT2\SAVEDATA", r"Attack on Titan2_AOT2_PK_WIN0000.dat", [(Documents, r"KoeiTecmo\Attack on Titan2_AOT2\SAVEDATA"), (Documents, r"KoeiTecmo\Attack on Titan 2")], ["*.dat"], 2),
    game!("Avatar: Frontiers of Pandora", HEX, r"Ubisoft Connect: <Program Files (x86)>\Ubisoft\Ubisoft Game Launcher\savegames\<user>\<game number>", r"<number>.save", [(ProgramFilesX86, r"Ubisoft\Ubisoft Game Launcher\savegames\*\*")], ["*.save"], 1),
    game!("Balatro", FULL, r"%APPDATA%\Balatro\<profile>", "save.jkr (the run), profile.jkr", [(AppData, "Balatro")], ["save.jkr", "profile.jkr"], 2),
    game!("Baldur's Gate 3", "Game screen: gold and the stack size of every stacked item each companion carries (potions, arrows, scrolls), from the save's packed entity store, the party summary (origin, class, level, experience) and story flags by area and companion, kept in step with the story's own flag list; the package is packed again with its checksum on save. Checked on about a hundred public saves, not yet tried in the game", r"%LOCALAPPDATA%\Larian Studios\Baldur's Gate 3\PlayerProfiles\Public\Savegames\Story\<save>", r"<save>.lsv", [(LocalAppData, r"Larian Studios\Baldur's Gate 3\PlayerProfiles\Public\Savegames\Story")], ["*.lsv"], 2),
    game!("Baldur's Gate II: Enhanced Edition", IE_BG, r"Documents\Baldur's Gate II - Enhanced Edition\save\<nnnnnnnnn-Name>", r"BALDUR.gam (BALDUR.SAV holds the areas)", [(Documents, r"Baldur's Gate II - Enhanced Edition\save"), (Documents, r"Baldur's Gate II - Enhanced Edition\mpsave")], ["*.gam"], 2),
    game!("Baldur's Gate: Enhanced Edition / Siege of Dragonspear", IE_BG, r"Documents\Baldur's Gate - Enhanced Edition\save\<nnnnnnnnn-Name>", r"BALDUR.gam (BALDUR.SAV holds the areas)", [(Documents, r"Baldur's Gate - Enhanced Edition\save"), (Documents, r"Baldur's Gate - Enhanced Edition\mpsave")], ["*.gam"], 2),
    game!("BeamNG.drive", "Game screen: career money and BeamXP from playerAttributes.json; branch levels and vehicles open by name in the Tree. Written from the game's own Lua source, not yet checked on a real save or in the game", r"%LOCALAPPDATA%\BeamNG\BeamNG.drive\current\settings\cloud\saves\<slot>\autosaveN\career", "playerAttributes.json, general.json, inventory.json", [(LocalAppData, r"BeamNG\BeamNG.drive\*\settings\cloud\saves\*\*\career"), (LocalAppData, r"BeamNG.drive\*\settings\cloud\saves\*\*\career")], ["playerAttributes.json"], 1),
    game!("BioShock Infinite", "Game screen: Silver Eagles, salts, lockpicks and the Health, Shield and Salts infusion levels; unpacked and packed again with the save's MD5. Not yet tried in the game", r"Steam\userdata\<account>\8870\remote\savedata", r"<area>-0_<date>.bkm / .sav", [(SteamUser("8870"), r"remote\savedata")], ["*.bkm", "*.sav"], 1),
    game!("BioShock Remastered / BioShock 2 Remastered", HEX_UNPACKED, r"Documents\BioshockHD\Bioshock (or Bioshock2)\SaveGames", r"<date>.bsb", [(Documents, r"BioshockHD"), (AppData, r"BioshockHD")], ["*.bsb"], 3),
    game!("Bitburner", "Game screen: money, the three groups of skills and experience, BitNode, karma and HP. Open an exported save (bitburnerSave_*.json); the nested JSON inside it is handled. Checked on a real save, not yet tried in the game", r"%APPDATA%\bitburner\saves (or the browser's Export Save button)", "bitburnerSave_<time>.json", [(AppData, r"bitburner\saves")], ["bitburnerSave*.json"], 1),
    game!("Black Mesa", BLACK_MESA, r"<Steam library>\steamapps\common\Black Mesa\bms\save", r"quick.sav, autosave.sav, bms-001.sav …", [(SteamGame("Black Mesa"), r"bms\save")], ["*.sav"], 1),
    game!("Blasphemous", "Game screen: Tears of Atonement, health and fervour, bile flasks, every permanent stat upgrade, completion and play time. The save's base64 wrapping is opened and closed for you. Checked on a public save, not yet tried in the game", r"%USERPROFILE%\AppData\LocalLow\TheGameKitchen\Blasphemous\Savegames\<Steam id>", r"savegame_<n>.save", [(LocalLow, r"TheGameKitchen\Blasphemous\Savegames")], ["savegame_*.save"], 2),
    game!("Bloodstained: Ritual of the Night", VALUES, r"%LOCALAPPDATA%\BloodstainedRotN\Saved\SaveGames", r"Story_Slot0.sav (and System_Slot.sav)", [(LocalAppData, r"BloodstainedRotN\Saved\SaveGames")], ["Story_Slot*.sav"], 1),
    game!("Borderlands 2", BORDERLANDS, r"Documents\My Games\Borderlands 2\WillowGame\SaveData\<Steam id>", r"Save0001.sav … (one per character; .sav.bak is the backup)", [(Documents, r"My Games\Borderlands 2\WillowGame\SaveData")], ["Save*.sav"], 2),
    game!("Borderlands 3", BORDERLANDS3, r"Documents\My Games\Borderlands 3\Saved\SaveGames\<Steam or Epic id>", r"<n>.sav (1.sav, 2.sav … one per character; profile.sav is the profile)", [(Documents, r"My Games\Borderlands 3\Saved\SaveGames")], ["?.sav", "??.sav", "???.sav"], 2),
    game!("Borderlands: The Pre-Sequel", BORDERLANDS, r"Documents\My Games\Borderlands The Pre-Sequel\WillowGame\SaveData\<Steam id>", r"Save0001.sav … (one per character; .sav.bak is the backup)", [(Documents, r"My Games\Borderlands The Pre-Sequel\WillowGame\SaveData")], ["Save*.sav"], 2),
    game!("Bounty of One", "Opens as JSON: gold nuggets, meta bonuses, achievements, characters and stats, every value by name. The save's XOR encryption is removed and put back on save (checked on a public sample save)", r"%USERPROFILE%\AppData\LocalLow\OptizOnion\BountyOfOne\<Steam id>", "gamedata.sav", [(LocalLow, r"OptizOnion\BountyOfOne")], ["gamedata.sav"], 2),
    game!("Brotato", "Opens as JSON: the run in progress and the progress file, every value by name", r"%APPDATA%\Brotato\<number>", r"run_v3_0.json (the run), save_v3.json (progress)", [(AppData, r"Brotato\*")], ["*.json"], 1),
    game!("Cairn", FULL, r"%USERPROFILE%\Saved Games\TheGameBakers\Cairn_RETAIL\SAVEGAMES\RETAIL\STORY (SUMMIT for summit runs)", r"SLOT_A.CPK … SLOT_D.CPK", [(SavedGames, r"TheGameBakers\Cairn_RETAIL\SAVEGAMES\RETAIL"), (SavedGames, r"TheGameBakers\Cairn\SAVEGAMES")], ["*.CPK"], 2),
    game!("Car Mechanic Simulator 2018", "Game screen: money, level and experience, from the profile's GlobalData.dat (a .NET BinaryFormatter file, read and written in place). Checked on a public save, not yet tried in the game", r"%USERPROFILE%\AppData\LocalLow\Red Dot Games\Car Mechanic Simulator 2018\Saves", r"profileN\GlobalData.dat", [(LocalLow, r"Red Dot Games\Car Mechanic Simulator 2018")], ["GlobalData.dat"], 3),
    game!("Car Mechanic Simulator 2021", "Game screen: money, level, XP and scrap, the profile's name, play time and difficulty, the unlocked parking levels, barns and warehouses. Checked on a public save, not yet tried in the game", r"%USERPROFILE%\AppData\LocalLow\Red Dot Games\Car Mechanic Simulator 2021\Save", "profile<N>.cms21", [(LocalLow, r"Red Dot Games\Car Mechanic Simulator 2021\Save")], ["*.cms21"], 1),
    game!("Cave Story+", "Game screen: every save slot's difficulty (Normal, Easy, Hard), health, max health and Whimsical Stars, and each weapon's type, level, energy and ammo; equipment, items and the stage are shown. The freeware Cave Story's Profile.dat opens too. Checked on public saves, not yet tried in the game", r"<install folder> (Steam: Steam\steamapps\common\Cave Story+)", "Profile.dat", [(SteamGame("Cave Story+"), "")], ["Profile.dat"], 1),
    game!("Celeste", "Game screen: strawberries, deaths and other totals, chapters unlocked, the assist options, and each chapter's cassette, completion, full clear and heart for the A, B and C sides. Checked on a real save, not yet tried in the game", r"<install folder>\Saves (Steam: Steam\steamapps\common\Celeste\Saves)", r"0.celeste, 1.celeste, 2.celeste (settings.celeste holds the options)", [(SteamGame("Celeste"), "Saves")], ["?.celeste"], 1),
    game!("Civilization V", HEX, r"Documents\My Games\Sid Meier's Civilization 5\Saves\single (or ModdedSaves\single)", "<Leader>_<year>.Civ5Save", [(Documents, r"My Games\Sid Meier's Civilization 5\Saves"), (Documents, r"My Games\Sid Meier's Civilization 5\ModdedSaves")], ["*.Civ5Save"], 3),
    game!("Civilization VI", CIV6, r"Documents\My Games\Sid Meier's Civilization VI\Saves\Single", r"*.Civ6Save (auto\AutoSave_####.Civ6Save)", [(Documents, r"My Games\Sid Meier's Civilization VI\Saves")], ["*.Civ6Save"], 3),
    game!("Clair Obscur: Expedition 33", EXPEDITION33, r"%LOCALAPPDATA%\Sandfall\Saved\SaveGames\<Steam id>", r"EXPEDITION_*.sav", [(LocalAppData, r"Sandfall\Saved\SaveGames"), (GamePass("KeplerInteractive.Expedition33_ymj30pw7xe604"), "")], ["*.sav"], 2),
    game!("Clicker Heroes", "Game screen: gold, rubies, hero souls, ancient souls, zones reached, multipliers and every hero's and outsider's level. The save's base64 and deflate packing is handled. Checked on a public save, not yet tried in the game", r"%USERPROFILE%\AppData\LocalLow\Playsaurus\Clicker Heroes (the saved game file; the in-game Save to file option also works)", r"the exported save .txt", [(LocalLow, r"Playsaurus\Clicker Heroes"), (Documents, r"Clicker Heroes")], ["*.txt", "*.sav"], 2),
    game!("CloverPit", "Opens as JSON, every value by name. The save's XOR encryption is removed and put back on save (not yet tried on a real save)", r"%USERPROFILE%\AppData\LocalLow\Panik\CloverPit\SaveData\GameData", "GameDataFull.json", [(LocalLow, r"Panik\CloverPit\SaveData\GameData")], ["*.json"], 1),
    game!("Control", HEX_UNPACKED, r"Steam\userdata\<account>\870780\remote", r"savegame-slot-00_global …", [(SteamUser("870780"), r"remote"), (LocalAppData, r"Remedy\Control"), (GamePass("505GAMESS.P.A.ControlPCGP_tefn33qh9azfc"), "")], ["savegame-*"], 2),
    game!("Core Keeper", "Game screen: the character's skill experience, ancient coins, maximum health and the inventory counts (items named as far as the public sample shows them). Plain JSON; checked on a public save, not yet tried in the game", r"%USERPROFILE%\AppData\LocalLow\Pugstorm\Core Keeper\Steam\<id>\saves", "<slot>.json", [(LocalLow, r"Pugstorm\Core Keeper")], ["*.json"], 4),
    game!("Craftopia", "Unpacked from gzip and opened as JSON: every value by name; packed again on save", r"%USERPROFILE%\AppData\LocalLow\PocketPair\Craftopia\Save", "*.ocs (players and worlds)", [(LocalLow, r"PocketPair\Craftopia\Save")], ["*.ocs"], 2),
    game!("Crisis Core: Final Fantasy VII Reunion", VALUES, r"Documents\My Games\CRISIS CORE FINAL FANTASY VII REUNION\Steam\<Steam id>", r"SAVEDATA_PROGRESS.sav", [(Documents, r"My Games\CRISIS CORE FINAL FANTASY VII REUNION\*\*")], ["*.sav"], 1),
    game!("Crusader Kings II", FULL, r"Documents\Paradox Interactive\Crusader Kings II\save games", "*.ck2", [(Documents, r"Paradox Interactive\Crusader Kings II\save games")], ["*.ck2"], 1),
    game!("Crusader Kings III", PDX_BINARY, r"Documents\Paradox Interactive\Crusader Kings III\save games", "*.ck3", [(Documents, r"Paradox Interactive\Crusader Kings III\save games")], ["*.ck3"], 1),
    game!("Cult of the Lamb", COTL, r"%USERPROFILE%\AppData\LocalLow\Massive Monster\Cult Of The Lamb\saves", "slot_0.json … (the .json save; a .mp file is a different format)", [(LocalLow, r"Massive Monster\Cult Of The Lamb\saves")], ["slot_*.json"], 1),
    game!("Cuphead", CUPHEAD, r"%APPDATA%\Cuphead", r"cuphead_player_data_v1_slot_0.sav (slots 0 to 2)", [(AppData, "Cuphead")], ["cuphead_player_data_v1_slot_*.sav"], 1),
    game!("Cyberpunk 2077", FULL, r"%USERPROFILE%\Saved Games\CD Projekt Red\Cyberpunk 2077\<save name>", r"sav.dat", [(SavedGames, r"CD Projekt Red\Cyberpunk 2077")], ["sav.dat"], 2),
    game!("Dark Souls II: Scholar of the First Sin", FULL, r"%APPDATA%\DarkSoulsII\<Steam id>", "DS2SOFS0000.sl2", [(AppData, "DarkSoulsII")], ["DS2SOFS0000.sl2"], 2),
    game!("Dark Souls III", FULL, r"%APPDATA%\DarkSoulsIII\<Steam id>", "DS30000.sl2", [(AppData, "DarkSoulsIII")], ["DS30000.sl2"], 2),
    game!("Dark Souls Remastered", FULL, r"Documents\NBGI\DARK SOULS REMASTERED\<id>", "DRAKS0005.sl2", [(Documents, r"NBGI\DARK SOULS REMASTERED")], ["DRAKS0005.sl2"], 2),
    game!("Dark Souls: Prepare to Die Edition", "Game screen: each character's name, soul level, souls, humanity and the eight attributes, with HP and stamina shown (the item list of this edition is not read yet). The save's MD5 checksums are rewritten. Checked on a public save (the character stats read as plausible values and a changed save keeps its checksums), not yet tried in the game", r"Documents\NBGI\DarkSouls", r"DRAKS0005.sl2", [(Documents, r"NBGI\DarkSouls")], ["DRAKS0005.sl2"], 1),
    game!("Darkest Dungeon", "Game screen: the estate (gold, busts, portraits, deeds, crests, blueprints, shards) how many of each trinket and estate item it holds, and the roster (each hero's resolve XP, stress, HP, weapon and armour rank, skill upgrade levels), changed in place. Open persist.estate.json or persist.roster.json; the profile's other files show in Values", r"Steam\userdata\<account>\262060\remote\profile_<n> (GOG: Documents\Darkest\profile_<n>)", r"persist.estate.json, persist.roster.json (binary, despite the name)", [(SteamUser("262060"), r"remote\profile_*"), (Documents, r"Darkest\profile_*")], ["persist.estate.json", "persist.roster.json"], 1),
    game!("Darkest Dungeon II", "Game screen: Candles of Hope (profile), Relics, torch and inventory (run.json), heroes' HP and stress (actors.json). Checked on saves made before the Kingdoms update; Kingdoms' own files aren't covered", r"%USERPROFILE%\AppData\LocalLow\RedHook\Darkest Dungeon II\SaveFiles\<id>\profiles (runs in profile_<n>_runs\<run>\Save_<nnnn>_<stage>)", r"profile_<n>.json; the newest Save_<nnnn>_* folder's run.json and actors.json", [(LocalLow, r"RedHook\Darkest Dungeon II\SaveFiles\*\profiles")], ["profile_*.json", "run.json", "actors.json"], 5),
    game!("Dave the Diver", "Game screen: gold, Bei, chef flames, research and trust points, and every sushi ingredient's stock with a Max crafting materials button (999, the stack the game shows). The save's XOR encryption is removed and put back on save. Checked on public saves, not yet tried in the game", r"%USERPROFILE%\AppData\LocalLow\nexon\DAVE THE DIVER\SteamSData\<Steam id>", "GameSave_00_GD.sav (one per slot: 00, 01, 02)", [(LocalLow, r"nexon\DAVE THE DIVER\SteamSData")], ["GameSave_??_GD.sav"], 2),
    game!("Dead Cells", "The save's SHA-1 checksum and zlib packing are handled, so the unpacked data can be changed in the hex view or found with the value search (cells, gold) and saved back for the game to accept. Checked on test files, not yet on a real save", r"<Steam library>\steamapps\common\Dead Cells\save (Steam Cloud: Steam\userdata\<account>\588650\remote; GOG: <install folder>\save)", r"user_0.dat … (one per slot) and its backups", [(SteamUser("588650"), r"remote"), (SteamGame("Dead Cells"), "save")], ["*.dat"], 1),
    game!("Dead Rising", "Game screen: Frank's level, PP, maximum health and skills (with buttons to learn every skill or reach level 50); the save's block checksums are rewritten. Checked on public speedrun practice saves; not yet tried in the game", r"Steam\userdata\<account>\427190\remote", r"DeadRising_01_Main … DeadRising_05_Main (one per slot; _Photo holds the photos)", [(SteamUser("427190"), r"remote")], ["DeadRising_*_Main"], 1),
    game!("Dead Rising 2", "Game screen: Chuck's level, PP, money and the attack, speed, health, item-slot and throwing upgrades (with a button to max them); the CRC-32 is rewritten. Checked on public speedrun saves; not yet tried in the game", r"Documents\My Games\Dead Rising 2\saves (Steam: Steam\userdata\<account>\45740\remote)", r"DR2SAVE.DR2S", [(Documents, r"My Games\Dead Rising 2\saves"), (SteamUser("45740"), r"remote")], ["*.DR2S"], 2),
    game!("Dead Rising 2: Off the Record", "Game screen: Frank's level, PP, money and the attack, speed, health, item-slot and throwing upgrades (with a button to max them), with the load screen's copy kept in step; the file's five CRC-32s are rewritten. Checked on public speedrun saves; not yet tried in the game", r"Documents\My Games\Dead Rising 2 Off The Record\saves (Steam: Steam\userdata\<account>\45770\remote)", r"DCSAV00.DSF … (one per slot)", [(Documents, r"My Games\Dead Rising 2 Off The Record\saves"), (SteamUser("45770"), r"remote")], ["*.DSF", "*.DSF.tmp"], 2),
    game!("Dead Rising 3", HEX, r"Documents\My Games\dead rising 3\saves\x64_build", r"D3S.*", [(Documents, r"My Games\dead rising 3\saves")], ["D3S.*", "d3s.*"], 2),
    game!("Dead Rising Deluxe Remaster", VALUES, r"Steam\userdata\<account>\2527390\remote\win64_save", "data*.bin", [(SteamUser("2527390"), r"remote\win64_save")], ["*.bin"], 1),
    game!("Death Must Die", DEATH_MUST_DIE, r"%USERPROFILE%\AppData\LocalLow\Realm Archive\Death Must Die\Saves", r"Slot_0.sav", [(LocalLow, r"Realm Archive\Death Must Die\Saves")], ["Slot_*.sav"], 1),
    game!("Death Stranding (Director's Cut)", FULL, r"%LOCALAPPDATA%\KojimaProductions\DeathStrandingDC\<account>\<slot>", r"checkpoint.dat", [(LocalAppData, r"KojimaProductions\DeathStrandingDC"), (LocalAppData, r"KojimaProductions\DeathStranding")], ["checkpoint.dat"], 3),
    game!("Death Stranding 2: On the Beach", FULL, r"Documents\DEATH STRANDING 2 - ON THE BEACH\<Steam id>", r"manualsave<n>.dat, checkpointsave<n>.dat", [(Documents, r"DEATH STRANDING 2 - ON THE BEACH\*")], ["*save*.dat"], 1),
    game!("Deathloop", "Game screen: Residuum, stress level, the loop's day and location, and the progress flags (first loop, Golden Loop, Julianna, void essence ability). The save's Oodle data is unpacked and packed again with the Oodle library of any game on this PC, and the file's checksum is made again. Checked on a real save and in the game. Close the game and turn Steam Cloud off for it first, or Steam may bring the old save back", r"%USERPROFILE%\Saved Games\Arkane Studios\Deathloop\base\savegame\<Steam id>\SLOT01", r"save.dloop", [(SavedGames, r"Arkane Studios\Deathloop\base\savegame")], ["*.dloop"], 3),
    game!("Deep Rock Galactic", DRG, r"<Steam library>\steamapps\common\Deep Rock Galactic\FSD\Saved\SaveGames", "<Steam id>_Player.sav", [(SteamGame("Deep Rock Galactic"), r"FSD\Saved\SaveGames")], ["*_Player.sav"], 1),
    game!("Deep Rock Galactic: Survivor", "Game screen: credits and the minerals, player and class ranks with their XP, stat upgrade levels. Checked on a real save, not yet tried in the game", r"%USERPROFILE%\AppData\LocalLow\Funday Games\DRG Survivor", r"drgs.dat (and drgs.dat_backup)", [(LocalLow, r"Funday Games\DRG Survivor")], ["drgs.dat"], 2),
    game!("Deltarune", "Game screen: Chapter 1 fully (party, inventory as item numbers, Light World); Chapters 2\u{2013}4: dark dollars and the party's stats and equipment. Every line of the file can be edited. Not yet tried in the game", r"%LOCALAPPDATA%\DELTARUNE", r"filech1_0 … filech4_2 (chapter and slot; dr.ini is the load menu)", [(LocalAppData, "DELTARUNE")], ["filech*"], 1),
    game!("Deltatraveler", "Game screen: name, gold, experience, play time, party members and the item, weapon and armor ids; the save's .NET BinaryFormatter data is opened and written back. Checked on public saves, not yet tried in the game", r"the folder the game writes SAVE0.sav to (a Deltatraveler folder under LocalLow is searched; otherwise open the file with the file browser)", "SAVE0.sav, SAVE1.sav, SAVE2.sav", [(LocalLow, "Deltatraveler")], ["SAVE*.sav"], 1),
    game!("Deus Ex (2000)", DEUS_EX, r"<install folder>\Save\Save#### (Steam: steamapps\common\Deus Ex\Save; some installs: Documents\Deus Ex\Save)", r"<map>.dxs: open the map named in SaveInfo.dxs, which holds JC (open SaveInfo.dxs to see which)", [(SteamGame("Deus Ex"), r"Save"), (Documents, r"Deus Ex\Save")], ["*.dxs"], 1),
    game!("Deus Ex: Human Revolution", HEX, r"Steam\userdata\<account>\238010\remote", r"GAMEA1_4 …", [(SteamUser("238010"), r"remote")], ["GAME*"], 1),
    game!("Deus Ex: Mankind Divided", "Game screen: credits, Praxis kits, crafting parts (with a Max crafting materials button) and ammo stacks; experience is shown. The game state is unpacked and packed again on save. Checked on real saves and in the game. Close the game and turn Steam Cloud off for it first, or Steam may bring the old save back", r"Steam\userdata\<account>\337000\remote", r"DXNGsavegame0000.dat", [(SteamUser("337000"), r"remote")], ["DXNGsavegame*.dat"], 1),
    game!("Devil May Cry HD Collection (Devil May Cry 1)", "Game screen for Devil May Cry 1 (dmc1.sav): red, yellow and blue orbs and play time for each used slot. Checked on a public save, not yet tried in the game", r"Steam\userdata\<account>\631510\remote", r"dmc1.sav (dmc2.sav, dmc3.sav)", [(SteamUser("631510"), r"remote")], ["dmc1.sav"], 1),
    game!("Diablo II (Lord of Destruction 1.10+)", FULL, r"%USERPROFILE%\Saved Games\Diablo II (1.13 and older: <install folder>\Save)", r"<name>.d2s", [(SavedGames, r"Diablo II")], ["*.d2s"], 1),
    game!("Diablo II: Resurrected", FULL, r"%USERPROFILE%\Saved Games\Diablo II Resurrected", r"<name>.d2s (SharedStash*.d2i is the shared stash)", [(SavedGames, r"Diablo II Resurrected")], ["*.d2s"], 2),
    game!("Disco Elysium", "Game screen: money, level, experience, skill points, the four attributes and all 24 skills, from the save's .ntwtf.zip (the zip is repacked with its other files kept). Checked on public saves, not yet tried in the game", r"%USERPROFILE%\Documents\ZAUM Studio\Disco Elysium\SaveGames", r"<name>.ntwtf.zip", [(Documents, r"ZAUM Studio\Disco Elysium\SaveGames")], ["*.ntwtf.zip"], 1),
    game!("Dishonored", "Game screen: coins. The save's zlib chunks are unpacked and packed again on save; the coin count is found by its surroundings and only offered when exactly one record fits. Checked on 11 real saves; the coin edit was tried in the game. Runes, rewire tools and elixirs aren't located yet (hex view)", r"Steam\userdata\<account>\205100\remote\userdata\<account>\205100\remote", r"dishonored0.sav, DisAutoSave0.sav", [(SteamUser("205100"), r"remote")], ["*.sav"], 1),
    game!("Dishonored 2 / Death of the Outsider", "Game screen: Raw Whalebone with a Max crafting materials button, runes, and bolts, bullets, grenades, spring razors and stun mines (each up to what the player can carry), plus the map and how many bonecharm traits are learned, for Dishonored 2. checkpoint.dat is unpacked and packed again with its check value on save; game.details opens as values (chapter, chaos level…). Checked on real saves and tried in the game", r"%USERPROFILE%\Saved Games\Arkane Studios\Dishonored2\base\savegame\SLOT<n>", r"checkpoint.dat (the game state), game.details", [(SavedGames, r"Arkane Studios\Dishonored2\base\savegame"), (SavedGames, r"Arkane Studios\Dishonored2_DOTO\base\savegame")], ["checkpoint.dat", "game.details"], 2),
    game!("Divinity: Original Sin 2 Definitive Edition", "Game screen: each player character's experience, vitality, attributes, unspent points and gold and the stack size of each carried item; packed again with the package checksum on save. Not yet tried in the game", r"Documents\Larian Studios\Divinity Original Sin 2 Definitive Edition\PlayerProfiles\<profile>\Savegames\Story\<save>", r"<save>.lsv", [(Documents, r"Larian Studios\Divinity Original Sin 2 Definitive Edition\PlayerProfiles\*\Savegames\Story")], ["*.lsv"], 2),
    game!("Divinity: Original Sin Enhanced Edition", "Game screen: each party member's experience, vitality, gold and item stack sizes, on the same screen as Original Sin 2; the save package is unpacked and packed again. Checked on a public save, not yet tried in the game", r"Documents\Larian Studios\Divinity Original Sin Enhanced Edition\PlayerProfiles\<profile>\Savegames_patch\<save>", "<save>.lsv", [(Documents, r"Larian Studios\Divinity Original Sin Enhanced Edition\PlayerProfiles")], ["*.lsv"], 4),
    game!("Dragon Age: Origins (and Awakening)", DAO, r"Documents\BioWare\Dragon Age\Characters\<name>\Saves\<slot>", r"<name>.das, savegame.das (auto and quick saves)", [(Documents, r"BioWare\Dragon Age\Characters\*\Saves")], ["*.das"], 2),
    game!("Dragon's Dogma 2", VALUES, r"Steam\userdata\<account>\2054970\remote\win64_save", "data*.bin", [(SteamUser("2054970"), r"remote\win64_save")], ["*.bin"], 1),
    game!("Drakensang: River of Time", "Game screen: each hero's level, adventure points (earned and to spend), life, endurance, astral energy, eight attributes and every talent, with the party slot to tell your party from the cutscene copies. The SQLite save is edited in place; items and money are in the Tree. Checked on a public save, not yet tried in the game", r"Documents\Drakensang_TRoT\profiles\default\save\savegame_<n>", "savegame.dsa (the screenshot and .nfo beside it are not needed)", [(Documents, r"Drakensang_TRoT\profiles\default\save\*")], ["savegame.dsa"], 2),
    game!("Drakensang: The Dark Eye", "Game screen: each hero's level, adventure points (earned and to spend), life, endurance, astral energy, eight attributes and every talent, with the party slot to tell your party from the cutscene copies. The SQLite save is edited in place; items and money are in the Tree. Checked on a public save, not yet tried in the game", r"Documents\Drakensang\profiles\default\save\savegame_<n>", "savegame.dsa (the screenshot and .nfo beside it are not needed)", [(Documents, r"Drakensang\profiles\default\save\*")], ["savegame.dsa"], 2),
    game!("Dying Light", FULL, r"Steam\userdata\<account>\239140\remote\out\save", r"save_main_0.sav (gzip)", [(SteamUser("239140"), r"remote\out\save")], ["*.sav"], 1),
    game!("Dying Light 2 / The Beast", FULL, r"Steam\userdata\<account>\534380\remote\out\save (backups in Documents\dying light 2\out\-save_backups)", r"save_main_0.sav (gzip)", [(SteamUser("534380"), r"remote\out\save"), (SteamUser("3008130"), r"remote\out\save"), (Documents, r"dying light 2\out\-save_backups"), (Documents, r"dying light the beast\out\-save_backups")], ["*.sav", "*.sav.bak"], 1),
    game!("Dynasty Warriors 3: Complete Edition Remastered", "Opens as an Unreal save: every property by name, numbers and text changed in place (no save of this game has been tried yet)", r"%LOCALAPPDATA%\KoeiTecmo\DW3CE_RE\Saved\SaveGames", "GameStatusData.sav", [(LocalAppData, r"KoeiTecmo\DW3CE_RE\Saved\SaveGames")], ["*.sav"], 2),
    game!("Eiyuden Chronicle: Hundred Heroes", "Opens as JSON: money, the roster, stats, equipment and inventory, every value by name. The save's Triple-DES encryption is removed and put back on save (not yet tried on a real save)", r"%USERPROFILE%\AppData\LocalLow\505 Games S_p_A\EiyudenChronicle\<Steam id>\SaveData", "UserData<n>.dat", [(LocalLow, r"505 Games S_p_A\EiyudenChronicle\*\SaveData")], ["UserData*.dat"], 1),
    game!("Elden Ring", FULL, r"%APPDATA%\EldenRing\<Steam id>", "ER0000.sl2", [(AppData, "EldenRing")], ["ER0000.sl2"], 2),
    game!("ENDER LILIES: Quietus of the Knights", "Game screen: Lily's level, blight (experience) and prayer heals, Stagnant Blight, Furious Blight and Ancient Souls with a Max crafting materials button, and every spirit's level. Checked on public saves, not yet tried in the game", r"%LOCALAPPDATA%\EnderLilies\Saved\SaveGames\<Steam id>", "GameSlot_0.sav, GameSlot_1.sav, GameSlot_2.sav", [(LocalAppData, r"EnderLilies\Saved\SaveGames")], ["GameSlot_*.sav"], 2),
    game!("Enderal: Forgotten Stories (Special Edition)", FULL, r"Documents\My Games\Enderal Special Edition\Saves", "*.ess", [(Documents, r"My Games\Enderal Special Edition\Saves"), (Documents, r"My Games\Enderal\Saves")], ["*.ess"], 1),
    game!("Enter the Gungeon", "Game screen: every Gungeoneer's Hegemony credits, runs, deaths, kills, money, play time and floors cleared. The save's XOR encryption is handled for you. Checked on a public save, not yet tried in the game", r"%USERPROFILE%\AppData\LocalLow\Dodge Roll\Enter the Gungeon (Steam Cloud: Steam\userdata\<account>\311690\remote)", r"SlotA.save", [(SteamUser("311690"), r"remote"), (LocalLow, r"Dodge Roll\Enter the Gungeon")], ["Slot?.save"], 1),
    game!("Euro Truck Simulator 2", TRUCKSIM, r"Documents\Euro Truck Simulator 2\profiles\<profile>\save\<save name>", "game.sii (a packed ScsC file or plain text)", [(Documents, r"Euro Truck Simulator 2\profiles")], ["game.sii"], 4),
    game!("Europa Universalis IV", FULL, r"Documents\Paradox Interactive\Europa Universalis IV\save games", "*.eu4", [(Documents, r"Paradox Interactive\Europa Universalis IV\save games")], ["*.eu4"], 1),
    game!("Europa Universalis V", PDX_EU5, r"Documents\Paradox Interactive\Europa Universalis V\save games", "*.eu5", [(Documents, r"Paradox Interactive\Europa Universalis V\save games")], ["*.eu5"], 1),
    game!("EVERSPACE", "Game screen: DNA fragments, the pilot's records and every perk with its level and lock, from CurrentPlayerSave.sav (the unlocked perks and blueprints stay in the Tree). Checked on a public save, not yet tried in the game", r"<install folder>\RSG\Saved\SaveGames (Steam: Steam\steamapps\common\EVERSPACE)", "CurrentPlayerSave.sav (the pilot); the run saves beside it", [(SteamGame("EVERSPACE"), r"RSG\Saved\SaveGames"), (LocalAppData, r"RSG\Saved\SaveGames")], ["CurrentPlayerSave.sav"], 1),
    game!("Everspace 2", "Game screen: credits, level, experience and device upgrade tokens, and the crafting resources by name with a Max crafting materials button (ores, crystals, scrap and basic resources). The packed save is unpacked and packed again for you; ships and items are in the Tree. Checked on a public save, not yet tried in the game", r"%LOCALAPPDATA%\ES2\Saved\SaveGames", "ES2_<date>.sav (the game), ES2__PREVIEW_<date>.sav and MetaData.sav (small, no progress)", [(LocalAppData, r"ES2\Saved\SaveGames")], ["ES2_2*.sav", "ES2_[0-9]*.sav"], 1),
    game!("Fable: The Lost Chapters", "Game screen: the hero's gold, morality, renown level and age. The save's packing and checksum are undone and made again on save. Checked on public saves, not yet tried in the game", r"Documents\My Games\Fable\Saves\<hero name>", "AutoSave, AutoSave.qs, Manual - Save1 ... (no extension)", [(Documents, r"My Games\Fable\Saves")], ["AutoSave*", "Manual - Save*"], 2),
    game!("Fallout (1997)", FALLOUT12, r"<install folder>\data\SAVEGAME\SLOT01 … SLOT10 (Steam: steamapps\common\Fallout; GOG: GOG Games\Fallout)", r"SAVE.DAT (the .SAV files next to it are the maps)", [(SteamGame("Fallout"), r"data\SAVEGAME")], ["SAVE.DAT"], 1),
    game!("Fallout 2", FALLOUT12, r"<install folder>\data\SAVEGAME\SLOT01 … SLOT10 (Steam: steamapps\common\Fallout 2; GOG: GOG Games\Fallout 2)", r"SAVE.DAT (the .SAV files next to it are the maps)", [(SteamGame("Fallout 2"), r"data\SAVEGAME")], ["SAVE.DAT"], 1),
    game!("Fallout 3", FALLOUT3NV, r"Documents\My Games\Fallout3\Saves", "*.fos", [(Documents, r"My Games\Fallout3\Saves")], ["*.fos"], 2),
    game!("Fallout 4", FULL, r"Documents\My Games\Fallout4\Saves", "*.fos", [(Documents, r"My Games\Fallout4\Saves"), (Documents, r"My Games\Fallout4VR\Saves")], ["*.fos"], 2),
    game!("Fallout Shelter", FALLOUT_SHELTER, r"%LOCALAPPDATA%\FalloutShelter (Steam, Windows Store) or Steam\userdata\<account>\588430\remote", "Vault1.sav, Vault2.sav, Vault3.sav", [(LocalAppData, "FalloutShelter"), (SteamUser("588430"), "remote")], ["Vault*.sav"], 1),
    game!("Fallout: New Vegas", FALLOUT3NV, r"Documents\My Games\FalloutNV\Saves", "*.fos", [(Documents, r"My Games\FalloutNV\Saves")], ["*.fos"], 2),
    game!("FANTASIAN Neo Dimension", "Opens as JSON, one save slot at a time: money, play time, items and equipment inside each slot, every value by name. The slot is decrypted and encrypted again on save; the other slots are left as they were (not yet tried on a real save)", r"Documents\My Games\FANTASIAN Neo Dimension\Steam\<Steam id>\_data", "root.json", [(Documents, r"My Games\FANTASIAN Neo Dimension\Steam\*\_data")], ["root.json"], 1),
    game!("Far Cry (Ubisoft Connect games)", HEX, r"Ubisoft Connect: <Program Files (x86)>\Ubisoft\Ubisoft Game Launcher\savegames\<user>\<game number>", r"<number>.save", [(ProgramFilesX86, r"Ubisoft\Ubisoft Game Launcher\savegames\*\*")], ["*.save", "*.save.z"], 1),
    game!("Farming Simulator 25", "Opens as XML: money and loans (farms.xml), vehicles, fields, placeables and the environment, every value by name. Each savegame is a folder of XML files", r"Documents\My Games\FarmingSimulator2025\savegame<n>", r"farms.xml, vehicles.xml, fields.xml, careerSavegame.xml …", [(Documents, r"My Games\FarmingSimulator2025\savegame*")], ["*.xml"], 1),
    game!("Final Fantasy IX", FULL, r"%USERPROFILE%\AppData\LocalLow\SquareEnix\FINAL FANTASY IX\Steam\EncryptedSavedData", r"SavedData_ww.dat (encrypted)", [(LocalLow, r"SquareEnix\FINAL FANTASY IX")], ["*.dat"], 3),
    game!("Final Fantasy Pixel Remaster (I-VI)", FULL, r"Documents\My Games\FINAL FANTASY <numeral> PR\Steam\<Steam id>", "Slot files with a long name ending in = (decrypted and encrypted again on save)", [(Documents, r"My Games\FINAL FANTASY*PR\Steam\*")], ["*="], 1),
    game!("Final Fantasy Resonance", FFRESONANCE, r"%LOCALAPPDATA%\FFRS\Steam\<Steam id>", "GameSaveData<n>.sav (0 is the autosave)", [(LocalAppData, r"FFRS\Steam\*")], ["GameSaveData*.sav"], 1),
    game!("Final Fantasy VII (2013 Steam)", FULL, r"Documents\Square Enix\FINAL FANTASY VII Steam\user_<id>", r"save00.ff7 …", [(Documents, r"Square Enix\FINAL FANTASY VII Steam")], ["*.ff7"], 2),
    game!("Final Fantasy VII Rebirth", FULL, r"Documents\My Games\FINAL FANTASY VII REBIRTH\Steam\<Steam id>", r"ff7rebirth000.sav …", [(Documents, r"My Games\FINAL FANTASY VII REBIRTH\*\*")], ["ff7rebirth*.sav"], 1),
    game!("Final Fantasy VII Remake Intergrade", FULL, r"Documents\My Games\FINAL FANTASY VII REMAKE\(Steam or EOS)\<id>", r"ff7remake000.sav …", [(Documents, r"My Games\FINAL FANTASY VII REMAKE\*\*")], ["ff7remake*.sav"], 1),
    game!("Final Fantasy VIII Remastered", HEX, r"Documents\My Games\FINAL FANTASY VIII Remastered\Steam\<Steam id>\game_data\user\saves", "slot1_save01.ff8 (the auto save) … slot2_save30.ff8", [(Documents, r"My Games\FINAL FANTASY VIII Remastered\Steam\*\game_data\user\saves")], ["*.ff8"], 1),
    game!("Final Fantasy X / X-2 HD Remaster", FULL, r"Documents\SQUARE ENIX\FINAL FANTASY X&X-2 HD Remaster", r"ffx_###", [(Documents, r"SQUARE ENIX\FINAL FANTASY X&X-2 HD Remaster")], ["ffx_*", "ffx2_*"], 3),
    game!("Final Fantasy XIII", "Game screen: gil. The save is decrypted, edited and encrypted again, with its checksum rewritten. Checked on public saves, not yet tried in the game", r"%LOCALAPPDATA%\SquareEnix\FinalFantasyXIII\save", "ff13-NN.dat", [(LocalAppData, r"SquareEnix\FinalFantasyXIII\save")], ["ff13-*.dat"], 1),
    game!("Final Fantasy XV", FULL, r"Documents\My Games\FINAL FANTASY XV\Steam\<Steam id>\savestorage (also Steam\userdata\<account>\637650\remote)", r"gameplay slot files", [(Documents, r"My Games\FINAL FANTASY XV"), (SteamUser("637650"), r"remote"), (GamePass("39EA002F.FINALFANTASYXVforPC_n746a19ndrrjg"), "")], ["*.save", "*.sav", "*.dat", "gameplay*"], 4),
    game!("Final Fantasy XVI", "Game screen: gil, Ability Points, consumables and materials, and Max crafting materials (99, 999 or 9999 per material); level and max HP shown. The save inside each slot picture is unpacked with a dictionary read from your own installed copy of the game and packed again with its CRC-32 rewritten; the autosave's ten saves are picked with Tools › Open another file in this archive. Tried in the game", r"Documents\My Games\FINAL FANTASY XVI\Steam\<Steam id> (EOS for Epic)", r"one .png file per slot, plus the autosave", [(Documents, r"My Games\FINAL FANTASY XVI\*\*")], ["*.png", "*.sav"], 1),
    game!("Flash and AIR games (.sol saves)", "Every value by name (numbers, flags, text, lists), written back with its own type; dates, byte arrays and class names are kept as they were. Checked on 66 public sample files, each written back byte for byte", r"%APPDATA%\Macromedia\Flash Player\#SharedObjects\<id>\<site or localhost> (AIR games: %APPDATA%\<game>\Local Store\#SharedObjects)", r"<name>.sol", [(AppData, r"Macromedia\Flash Player\#SharedObjects\*")], ["*.sol"], 6),
    game!("For The King", "Game screen: each hero's gold, level, XP, health and focus, and the party's life pool. The gzip and UTF-16 wrapping is removed and put back on save. Checked on a public save, not yet tried in the game", r"%USERPROFILE%\AppData\LocalLow\IronOak Games\FTK\save", "<adventure>_<date>.run (one per adventure in progress)", [(LocalLow, r"IronOak Games\FTK\save"), (LocalLow, r"IronOak Games\FTKsave")], ["*.run"], 1),
    game!("Forspoken", "Decrypted for you (AES, with the save's MD5 worked out again) and opened with its slot, chapter and play time shown; edit values with Value search. Not yet tried on a real save", r"Documents\Forspoken\<id> (the folder of your Steam account)", "auto.save, slot01.save …", [(Documents, r"Forspoken\*"), (Documents, "Forspoken")], ["*.save"], 2),
    game!("God of War (2018)", FULL, r"%USERPROFILE%\Saved Games\God of War\<account>", r"game.sav (all slots)", [(SavedGames, r"God of War\*")], ["*.sav"], 1),
    game!("God of War Ragnarök", FULL, r"%USERPROFILE%\Saved Games\God of War Ragnarök\<account>", r"GOWRSAVE10 …", [(SavedGames, r"God of War Ragnarök\*")], ["GOWRSAVE*"], 1),
    game!("Golden Sun (GBA, emulator saves)", GOLDEN_SUN, GBA_FOLDER, r"Golden Sun (USA, Europe).sav … (64 KiB flash)", [(Documents, "mGBA"), (AppData, "visualboyadvance-m"), (Documents, "ROMs")], ["*Golden Sun.sav", "*Golden Sun (*.sav", "*Golden Sun [*.sav"], 3),
    game!("Golden Sun: The Lost Age (GBA, emulator saves)", GOLDEN_SUN, GBA_FOLDER, r"Golden Sun - The Lost Age (USA, Europe).sav … (64 KiB flash)", [(Documents, "mGBA"), (AppData, "visualboyadvance-m"), (Documents, "ROMs")], ["*Lost Age*.sav"], 3),
    game!("Grand Theft Auto III", "Game screen: money (both copies), hidden packages and the main statistics (progress, missions, stunt jumps, days, kills); the save's checksum is rewritten. Checked on public saves, not yet tried in the game", r"Documents\GTA3 User Files", r"GTA3sf1.b … GTA3sf8.b", [(Documents, "GTA3 User Files")], ["GTA3sf*.b"], 1),
    game!("Grand Theft Auto: Vice City", "Game screen: money (both copies), hidden packages and the main statistics (progress, stunt jumps, days, kills); the save's checksum is rewritten. Checked on public saves, not yet tried in the game", r"Documents\GTA Vice City User Files", r"GTAVCsf1.b … GTAVCsf8.b", [(Documents, "GTA Vice City User Files")], ["GTAVCsf*.b"], 1),
    game!("Graveyard Keeper", FULL, r"%USERPROFILE%\AppData\LocalLow\Lazy Bear Games\Graveyard Keeper", "<slot>.dat (and .hh)", [(LocalLow, r"Lazy Bear Games\Graveyard Keeper")], ["*.dat"], 2),
    game!("Green Hell", FULL, r"Steam\userdata\<account>\815370\remote", r"SPSlot0.sav …", [(SteamUser("815370"), r"remote")], ["*Slot*.sav"], 1),
    game!("Grim Dawn", "Game screen: money, level, experience, unspent attribute/skill/devotion points, attributes, skill ranks, item stacks and faction reputation; decrypted, then encrypted again with fresh checksums. Not yet tried in the game", r"Documents\My Games\Grim Dawn\save\main\_<name> (Steam Cloud: Steam\userdata\<account>\219990\remote\save\main\_<name>)", r"player.gdc", [(Documents, r"My Games\Grim Dawn\save\main"), (SteamUser("219990"), r"remote\save\main")], ["player.gdc"], 1),
    game!("Hades", FULL, r"Documents\Saved Games\Hades", "Profile1.sav … Profile4.sav", [(Documents, r"Saved Games\Hades"), (SavedGames, "Hades"), (GamePass("SupergiantGamesLLC.Hades_q53c1yqmx7pha"), "")], ["Profile*.sav"], 1),
    game!("Hades II", FULL, r"Saved Games\Hades II", "Profile1.sav …", [(SavedGames, "Hades II"), (Documents, r"Saved Games\Hades II")], ["Profile*.sav"], 1),
    game!("Half-Life / Opposing Force / Blue Shift", HL1, r"<Steam library>\steamapps\common\Half-Life\valve\SAVE (Opposing Force: gearbox\SAVE, Blue Shift: bshift\SAVE)", r"quick.sav, autosave.sav, Half-Life-001.sav …", [(SteamGame("Half-Life"), r"valve\SAVE"), (SteamGame("Half-Life"), r"gearbox\SAVE"), (SteamGame("Half-Life"), r"bshift\SAVE")], ["*.sav"], 1),
    game!("Half-Life 2 / Episode One / Episode Two", HL2_EPISODES, r"<Steam library>\steamapps\common\Half-Life 2\hl2\save (Episode One: episodic\save, Episode Two: ep2\save, Lost Coast: lostcoast\save)", r"quick.sav, autosave.sav, half-life-001.sav …", [(SteamGame("Half-Life 2"), r"hl2\save"), (SteamGame("Half-Life 2"), r"episodic\save"), (SteamGame("Half-Life 2"), r"ep2\save"), (SteamGame("Half-Life 2"), r"lostcoast\save")], ["*.sav"], 1),
    game!("Half-Life: Alyx", HL_ALYX, r"<Steam library>\steamapps\common\Half-Life Alyx\game\hlvr\SAVE\S<slot>", r"quick.sav, autosave.sav, manual saves …", [(SteamGame("Half-Life Alyx"), r"game\hlvr\SAVE\*")], ["*.sav"], 1),
    game!("Half-Life: Source", HL2, r"<Steam library>\steamapps\common\Half-Life 2\hl1\save", r"quick.sav, autosave.sav, half-life-001.sav …", [(SteamGame("Half-Life 2"), r"hl1\save")], ["*.sav"], 1),
    game!("Hearts of Iron IV", FULL, r"Documents\Paradox Interactive\Hearts of Iron IV\save games", "*.hoi4", [(Documents, r"Paradox Interactive\Hearts of Iron IV\save games")], ["*.hoi4"], 1),
    game!("Hogwarts Legacy", FULL, r"%LOCALAPPDATA%\Hogwarts Legacy\Saved\SaveGames\<account>", r"HL-00-00.sav … (needs the downloaded app, not the browser version)", [(LocalAppData, r"Hogwarts Legacy\Saved\SaveGames\*")], ["HL-*.sav"], 1),
    game!("Hollow Knight", FULL, r"%USERPROFILE%\AppData\LocalLow\Team Cherry\Hollow Knight", "user1.dat … user4.dat", [(LocalLow, r"Team Cherry\Hollow Knight")], ["user?.dat"], 1),
    game!("Hollow Knight: Silksong", "Game screen: rosaries, shell shards, masks, silk, needle, abilities, crests, tools and items; decrypted and encrypted again on save. Not yet tried in the game", r"%USERPROFILE%\AppData\LocalLow\Team Cherry\Hollow Knight Silksong\<Steam id>", "user1.dat … user4.dat", [(LocalLow, r"Team Cherry\Hollow Knight Silksong")], ["user?.dat"], 2),
    game!("HoloCure - Save the Fans!", "Game screen: HoloCoins and the levels of every Holo House upgrade (ATK, HP, SPD, crit, pickup range, haste, damage resistance, regen, EXP, money gain, rerolls and the rest). The save_n.dat is base64 JSON, unpacked and packed again. Checked on a public save, not yet tried in the game", r"%LOCALAPPDATA%\HoloCure", "save_n.dat", [(LocalAppData, r"HoloCure")], ["save_*.dat"], 1),
    game!("Horizon Zero Dawn", "Game screen: metal shards and skill points, with level and experience shown. The save's MD5 is made again on save. Checked on public saves of the PC Complete Edition, not yet tried in the game", r"Documents\Horizon Zero Dawn\Saved Game\<slot>", "checkpoint.dat (slotinfo.ini next to it is the menu text)", [(Documents, r"Horizon Zero Dawn\Saved Game\*")], ["checkpoint.dat"], 1),
    game!("Icewind Dale: Enhanced Edition", IE_IWD, r"Documents\Icewind Dale - Enhanced Edition\save\<nnnnnnnnn-Name>", r"ICEWIND.gam", [(Documents, r"Icewind Dale - Enhanced Edition\save"), (Documents, r"Icewind Dale - Enhanced Edition\mpsave")], ["*.gam"], 2),
    game!("Idle Slayer", "Opens as JSON, every value by name. The save's XOR encryption is removed and put back on save (not yet tried on a real save)", r"%USERPROFILE%\AppData\LocalLow\Pablo Leban\Idle Slayer", "savedata.sav", [(LocalLow, r"Pablo Leban\Idle Slayer")], ["*.sav"], 1),
    game!("Judgment", "Game screen: Yagami's money and HP; decrypted with the game's key and the checksum kept right, every other value in the Tree. Checked on 40 public saves, not yet tried in the game", r"%APPDATA%\Sega\Judgment\Steam\<Steam id>", r"data.sav", [(AppData, r"Sega\Judgment\Steam\*")], ["*.sav"], 2),
    game!("Kenshi", KENSHI, r"%LOCALAPPDATA%\kenshi\save\<save name>", r"quick.save (the faction), platoon\Nameless_0.platoon (your squad, named after your faction)", [(LocalAppData, r"kenshi\save"), (SteamGame("Kenshi"), r"save")], ["quick.save", "Nameless_*.platoon"], 3),
    game!("Kerbal Space Program", FULL, r"<game folder>\saves\<save name> (Steam: steamapps\common\Kerbal Space Program\saves)", "persistent.sfs (and the quicksaves beside it)", [(SteamGame("Kerbal Space Program"), r"saves\*")], ["*.sfs"], 2),
    game!("Kingdom Hearts III", FULL, r"Documents\KINGDOM HEARTS III\Steam\<Steam id>\SaveGames\kh3sv2\data (Epic: …\Epic Games Store\1638\…)", "KHIII_slot0.bin …", [(Documents, r"KINGDOM HEARTS III\*\*\SaveGames\kh3sv2\data")], ["KHIII_*.bin"], 1),
    game!("Kingdom Two Crowns", "Unpacked from gzip and opened as JSON: coins, gems, islands, units and mounts, every value by name; packed again on save", r"%USERPROFILE%\AppData\LocalLow\noio\Kingdom Two Crowns\Release (macOS: ~/Library/Application Support/nl.noio.kingdom-two-crowns/Release)", r"global-v35 and the slot files beside it", [(LocalLow, r"noio\Kingdom Two Crowns\Release"), (LocalLow, r"noio\KingdomTwoCrowns\Release")], ["global-v*"], 1),
    game!("Koei Tecmo games (Nioh, Wo Long, Stranger of Paradise)", "Decrypted and encrypted again on save, keeping the file's own keys: Nioh, Nioh 2 and Nioh 3 (the data opens for Value search), Wo Long: Fallen Dynasty and Fatal Frame II: Crimson Butterfly Remake (the JSON opens with every value by name; both checksums are made again), Stranger of Paradise: Final Fantasy Origin (Value search). Checked on the test files of KatanaSaveDataResigner, not yet tried in the games", r"Documents\KoeiTecmo\<game>\Savedata\<Steam id>\SAVEDATA00", r"SAVEDATA.BIN (one folder per slot)", [(Documents, r"KoeiTecmo\*")], ["*.BIN", "*.bin"], 5),
    game!("Kunitsu-Gami: Path of the Goddess", VALUES, r"Steam\userdata\<account>\2510710\remote\win64_save", "data*.bin", [(SteamUser("2510710"), r"remote\win64_save")], ["*.bin"], 1),
    game!("Last Epoch", "Game screen: level, experience, deaths, respecs and the hardcore and masochist flags of an offline character. The save's EPOCH header is kept. Checked on a public save, not yet tried in the game", r"%USERPROFILE%\AppData\LocalLow\Eleventh Hour Games\Last Epoch\Saves", r"1CHARACTERSLOT_BETA_<n> (and Epoch_Local_Global_Data_Beta, the stash files)", [(LocalLow, r"Eleventh Hour Games\Last Epoch\Saves")], ["1CHARACTERSLOT_BETA_?", "STASH_*", "Epoch_Local_Global_Data*"], 1),
    game!("LEGO Batman 2: DC Super Heroes", "Game screen: the stud wallet (the save has no checksum). Checked on public saves, not yet tried in the game", r"%APPDATA%\Warner Bros. Interactive Entertainment\LEGO Batman 2\SavedGames", r"slot saves", [(AppData, r"Warner Bros. Interactive Entertainment\LEGO Batman 2\SavedGames")], ["*"], 2),
    game!("LEGO Batman 3: Beyond Gotham", "Game screen: the stud wallet (the game keeps it in several places in the save; all are written, and it has no checksum). Checked on public saves, not yet tried in the game", r"Steam\userdata\<account>\313690\remote\savedgames", r"slot saves", [(SteamUser("313690"), r"remote\savedgames")], ["*"], 2),
    game!("LEGO Batman: Legacy of the Dark Knight", "Game screen: studs (every wallet field written together) and play time; the encryption is handled for you and everything else is in the Tree. Not yet tried in the game", r"%LOCALAPPDATA%\Warner Bros. Interactive Entertainment\LEGO Batman - Legacy of the Dark Knight\SaveGames\steam\<Steam id>", r"SaveSlot_0_TT.sav and BackupCopy_SaveSlot_0_TT.sav", [(LocalAppData, r"Warner Bros. Interactive Entertainment\LEGO Batman - Legacy of the Dark Knight\SaveGames\*\*")], ["SaveSlot_*_TT.sav", "BackupCopy_SaveSlot_*_TT.sav"], 1),
    game!("LEGO Batman: The Videogame", "Game screen: the stud counter (the save's checksum is rewritten). Checked on public saves, not yet tried in the game", r"%LOCALAPPDATA%\Warner Bros. Interactive Entertainment\LEGO Batman\Saved Games", r"slot saves", [(LocalAppData, r"Warner Bros. Interactive Entertainment\LEGO Batman\Saved Games")], ["*"], 2),
    game!("LEGO City Undercover", "Game screen: the stud wallet (the save has no checksum). Checked on public saves, not yet tried in the game", r"Steam\userdata\<account>\578330\remote\savedgames", r"slot saves", [(SteamUser("578330"), r"remote\savedgames")], ["*"], 2),
    game!("LEGO DC Super-Villains", "Game screen: the stud wallet (the game keeps it in several places in the save; all are written, and it has no checksum). Checked on public saves, not yet tried in the game", r"%APPDATA%\Warner Bros. Interactive Entertainment\LEGO DC Super-villains\SAVEDGAMES", r"slot saves", [(AppData, r"Warner Bros. Interactive Entertainment\LEGO DC Super-villains\SAVEDGAMES")], ["*"], 2),
    game!("LEGO Harry Potter Collection", "Opens in the Hex view for Value search (find your stud count, then change it); the older TT Games saves are not encrypted (checked on LEGO Marvel Super Heroes). No dedicated screen yet", r"%APPDATA%\LegoHarry1-4 and LegoHarry5-7", r"game saves", [(AppData, r"LegoHarry1-4"), (AppData, r"LegoHarry5-7")], ["*"], 2),
    game!("LEGO Harry Potter: Years 1-4", "Game screen: the stud wallet (the save has no checksum). Checked on public saves, not yet tried in the game", r"%APPDATA%\WB Games\LEGO® Harry Potter™\SavedGames", r"slot saves", [(AppData, r"WB Games\LEGO® Harry Potter™\SavedGames")], ["*"], 2),
    game!("LEGO Harry Potter: Years 5-7", "Game screen: the stud wallet (the save has no checksum). Checked on public saves, not yet tried in the game", r"%APPDATA%\WB Games\LEGO Harry Potter 2\SavedGames", r"slot saves", [(AppData, r"WB Games\LEGO Harry Potter 2\SavedGames")], ["*"], 2),
    game!("LEGO Indiana Jones 2: The Adventure Continues", "Game screen: the stud counter (the save's checksum is rewritten). Checked on public saves, not yet tried in the game", r"%APPDATA%\LucasArts\LEGO® Indiana Jones™ 2\SavedGames", r"slot saves", [(AppData, r"LucasArts\LEGO® Indiana Jones™ 2\SavedGames")], ["*"], 2),
    game!("LEGO Indiana Jones: The Original Adventures", "Game screen: the stud counter (the save's checksum is rewritten). Checked on public saves, not yet tried in the game", r"%LOCALAPPDATA%\LucasArts\LEGOIndianaJones\SavedGames", r"SaveGame*", [(LocalAppData, r"LucasArts\LEGOIndianaJones\SavedGames")], ["SaveGame*"], 2),
    game!("LEGO Jurassic World", "Game screen: the stud wallet (the game keeps it in several places in the save; all are written, and it has no checksum). Checked on public saves, not yet tried in the game", r"Steam\userdata\<account>\352400", r"slot saves", [(SteamUser("352400"), r"")], ["*SAVEGAMEDATA", "*.sav"], 2),
    game!("LEGO Marvel Super Heroes", "Game screen: the stud wallet (the save has no checksum). Checked on public saves, not yet tried in the game", r"Steam\userdata\<account>\249130\remote\savedgames", r"slot1\GAME1.LEGOMarvelSuperHeroesSaveGameData", [(SteamUser("249130"), r"remote\savedgames")], ["GAME*"], 2),
    game!("LEGO Marvel Super Heroes 2", "Game screen: the stud wallet (the game keeps it in several places in the save; all are written, and it has no checksum). Checked on public saves, not yet tried in the game", r"%APPDATA%\Warner Bros. Interactive Entertainment\LEGO Marvel Super Heroes 2\SAVEDGAMES", r"slot saves", [(AppData, r"Warner Bros. Interactive Entertainment\LEGO Marvel Super Heroes 2\SAVEDGAMES")], ["*"], 2),
    game!("LEGO Marvel's Avengers", "Opens in the Hex view for Value search (find your stud count, then change it); the older TT Games saves are not encrypted (checked on LEGO Marvel Super Heroes). No dedicated screen yet", r"Steam\userdata\<account>\405310\remote\savedgames", r"slot saves", [(SteamUser("405310"), r"remote\savedgames")], ["*"], 2),
    game!("LEGO Pirates of the Caribbean", "Game screen: the stud wallet (the save keeps five copies of it; all are written, and it has no checksum). Checked on public saves, not yet tried in the game", r"%APPDATA%\DisneyInteractiveStudios\LEGOPirates\SavedGames\Slot1", r"GAME1.LEGOPiratesSaveGameData", [(AppData, r"DisneyInteractiveStudios\LEGOPirates\SavedGames\*")], ["GAME*"], 2),
    game!("LEGO Star Wars II: The Original Trilogy", "Game screen: the stud counter (the save's checksum is rewritten). Checked on public saves, not yet tried in the game", r"<install folder> (often %LOCALAPPDATA%\VirtualStore\Program Files (x86)\LucasArts\LEGO Star Wars II)", r"savegame.bin and code.bin", [(LocalAppData, r"VirtualStore\Program Files (x86)\LucasArts\LEGO Star Wars II\*")], ["savegame.bin"], 1),
    game!("LEGO Star Wars III: The Clone Wars", "Game screen: the stud wallet (the save has no checksum). Checked on public saves, not yet tried in the game", r"%APPDATA%\Lucasarts\LEGOStarWarsIII\SavedGames\Slot1", r"GAME1.LEGOStarWarsIIISaveGameData", [(AppData, r"Lucasarts\LEGOStarWarsIII\SavedGames\*")], ["GAME*.LEGOStarWarsIIISaveGameData"], 2),
    game!("LEGO Star Wars: The Complete Saga", "Game screen: the stud counter (the save's checksum is rewritten). Checked on public saves, not yet tried in the game", r"%LOCALAPPDATA%\LucasArts\LEGO Star Wars - The Complete Saga\SavedGames", r"SaveGame*.LEGO Star Wars - The Complete Saga_SavedGame", [(LocalAppData, r"LucasArts\LEGO Star Wars - The Complete Saga\SavedGames")], ["SaveGame*"], 2),
    game!("LEGO Star Wars: The Force Awakens", "Game screen: the stud wallet (the game keeps it in several places in the save; all are written, and it has no checksum). Checked on public saves, not yet tried in the game", r"Steam\userdata\<account>\438640\remote\savedgames", r"slot saves", [(SteamUser("438640"), r"remote\savedgames")], ["*"], 2),
    game!("LEGO Star Wars: The Skywalker Saga", "Game screen: the stud wallet (the game saves it twice; both copies are written). Open COLLECTABLES.BLOB in a SLOTn folder. Checked on a public 100% save, not yet tried in the game", r"%APPDATA%\Warner Bros. Interactive Entertainment\LEGO Star Wars - The Skywalker Saga\SAVEDGAMES\STEAM\<number>\SLOTn", r"COLLECTABLES.BLOB", [(AppData, r"Warner Bros. Interactive Entertainment\LEGO Star Wars - The Skywalker Saga\SAVEDGAMES\*\*\*"), (GamePass("WarnerBros.Interactive.LEGOStarWarsTheSkywalkerSag_ktmk1xygcecda"), r"")], ["COLLECTABLES.BLOB"], 1),
    game!("LEGO Star Wars: The Video Game", "Game screen: the stud counter (the save's checksum is rewritten, and CODE.BIN beside LSW.BIN is rewritten with it). Checked on public saves, not yet tried in the game", r"<install folder> (often %LOCALAPPDATA%\VirtualStore\Program Files (x86)\Giant\LEGO Star Wars Game)", r"LSW.BIN and CODE.BIN", [(LocalAppData, r"VirtualStore\Program Files (x86)\Giant\LEGO Star Wars Game\*")], ["LSW.BIN"], 1),
    game!("LEGO The Hobbit", "Game screen: the stud wallet (the game keeps it in several places in the save; all are written, and it has no checksum). Checked on public saves, not yet tried in the game", r"Steam\userdata\<account>\285160\remote\savedgames", r"slot saves", [(SteamUser("285160"), r"remote\savedgames")], ["*"], 2),
    game!("LEGO The Incredibles", "Game screen: the stud wallet (the game keeps it in several places in the save; all are written, and it has no checksum). Checked on public saves, not yet tried in the game", r"%APPDATA%\Warner Bros. Interactive Entertainment\LEGO The Incredibles\SAVEDGAMES", r"slot saves", [(AppData, r"Warner Bros. Interactive Entertainment\LEGO The Incredibles\SAVEDGAMES")], ["*"], 2),
    game!("LEGO The Lord of the Rings", "Game screen: the stud wallet (the save has no checksum). Checked on public saves, not yet tried in the game", r"Steam\userdata\<account>\214510\remote\savedgames", r"slot saves", [(SteamUser("214510"), r"remote\savedgames")], ["*"], 2),
    game!("LEGO Worlds", "Opens in the Hex view for Value search (find your stud count, then change it); the older TT Games saves are not encrypted (checked on LEGO Marvel Super Heroes). No dedicated screen yet", r"%APPDATA%\Warner Bros. Interactive Entertainment\LEGOWorlds\SavedGames", r"SlotN\GAME1.SAVEGAMEDATA", [(AppData, r"Warner Bros. Interactive Entertainment\LEGOWorlds\SavedGames\*")], ["GAME*.SAVEGAMEDATA"], 2),
    game!("Lethal Company", "Game screen: credits, the profit quota and what is paid towards it, quotas passed, the deadline, the current moon, ship upgrades in storage and the run's statistics (LCSaveFile1 to 3); your experience and rank (LCGeneralSaveData). Decrypted and encrypted again on save. Checked on public saves, not yet tried in the game", r"%USERPROFILE%\AppData\LocalLow\ZeekerssRBLX\Lethal Company", r"LCSaveFile1 … LCSaveFile3 (one per save slot), LCGeneralSaveData", [(LocalLow, r"ZeekerssRBLX\Lethal Company")], ["LCSaveFile*", "LCGeneralSaveData"], 1),
    game!("Lies of P", "Game screen: Ergo, level, humanity, the six stats, and every item's count, with a Max crafting materials button for Moonstones and Legion Calibers (99, an estimate). Checked on a public save, not yet tried in the game", r"%LOCALAPPDATA%\LiesofP\Saved\SaveGames\<Steam id>", "SaveData-<n>_Character_<slot>.sav", [(LocalAppData, r"LiesofP\Saved\SaveGames")], ["*_Character_*.sav"], 2),
    game!("Like a Dragon Gaiden: The Man Who Erased His Name", "Game screen: money; decrypted, with its checksum kept right, and every other value in the Tree. Read, edited and written back on a public save, not yet tried in the game", r"%APPDATA%\Sega\LikeADragonGaiden\Steam\<Steam id>", r"data.sav", [(AppData, r"Sega\LikeADragonGaiden\Steam\*"), (GamePass("SEGAofAmericaInc.l1b05f489e_s751p9cej88mt"), "")], ["*.sav"], 2),
    game!("Like a Dragon: Infinite Wealth", RGG_UNTESTED, r"%APPDATA%\Sega\LikeADragon8\Steam\<Steam id>", r"data.sav", [(AppData, r"Sega\LikeADragon8\Steam\*")], ["*.sav"], 2),
    game!("Like a Dragon: Ishin!", "Game screen: food, seafood and crafting materials (up to 999 each, with a Max crafting materials button), the valuables (virtue, wooden tags, tickets), ammunition, style experience and training orbs; decrypted, with its checksum kept right. Ryo isn't mapped yet. Checked on a public save, not yet tried in the game", r"%APPDATA%\SEGA\LikeADragonIshin\<Steam id>", r"data.sav (binary inside)", [(AppData, r"SEGA\LikeADragonIshin\*"), (GamePass("SEGAofAmericaInc.ProjectMacan_s751p9cej88mt"), "")], ["*.sav"], 2),
    game!("Like a Dragon: Pirate Yakuza in Hawaii", RGG_UNTESTED, r"%APPDATA%\Sega\LikeADragonPirates\Steam\<Steam id>", r"data.sav (MessagePack inside)", [(AppData, r"Sega\LikeADragonPirates\Steam\*")], ["*.sav"], 2),
    game!("Lost Judgment", RGG_UNTESTED, r"%APPDATA%\Sega\LostJudgment\Steam\<Steam id>", r"data.sav", [(AppData, r"Sega\LostJudgment\Steam\*")], ["*.sav"], 2),
    game!("Luck be a Landlord", "Game screen: coins, removal and reroll tokens, spins, the floor, the rent due and the run's progress. The save's JSON lines are opened as one object keyed by each node's path and written back line by line. Checked on a public save, not yet tried in the game", r"%APPDATA%\Godot\app_userdata\Luck be a Landlord", "LBAL.save (the run in progress)", [(AppData, r"Godot\app_userdata\Luck be a Landlord")], ["LBAL.save"], 1),
    game!("Mario & Luigi: Dream Team (3DS, decrypted saves)", "Game screen: coins, each brother's level, experience, HP, BP, Power, Defense, Speed and Stache, and the counts of recovery items, battle items and beans. Checked on a public save, not yet tried in the game", r"The decrypted save from Checkpoint, JKSM or GodMode9, or Citra's sdmc folder", "ML4_001.sav (96,688 bytes)", [(AppData, r"Citra\sdmc")], ["ML4_*.sav"], 4),
    game!("Mario Kart 7 (3DS, decrypted saves)", "Game screen: coins, StreetPass count, online wins and losses and VR points, with the save's CRC-32 written again. Checked on a public save, not yet tried in the game", r"The decrypted save from Checkpoint, JKSM or GodMode9, or Citra's sdmc folder", "system0.dat (20,692 bytes)", [(AppData, r"Citra\sdmc")], ["system0.dat"], 4),
    game!("Marvel's Spider-Man 2", HEX, r"Documents\Marvel's Spider-Man 2\<Steam id>", r"slot<n>-autosave.save", [(Documents, r"Marvel's Spider-Man 2\*")], ["slot*.save"], 1),
    game!("Marvel's Spider-Man Remastered", HEX, r"Documents\Marvel's Spider-Man Remastered\<Steam id>", r"slot<n>-s.save", [(Documents, r"Marvel's Spider-Man Remastered\*")], ["slot*.save"], 1),
    game!("Marvel's Spider-Man: Miles Morales", HEX, r"Documents\Marvel's Spider-Man Miles Morales\<Steam id>", r"slot<n>-s.save", [(Documents, r"Marvel's Spider-Man Miles Morales\*")], ["slot*.save"], 1),
    game!("Mass Effect (2008)", FULL, r"Documents\BioWare\Mass Effect\Save", r"<name>_<nn>.MassEffectSave, <name>_AutoSave.MassEffectSave", [(Documents, r"BioWare\Mass Effect\Save")], ["*.MassEffectSave"], 1),
    game!("Mass Effect 2", FULL, r"Documents\BioWare\Mass Effect 2\Save\<Shepard's career folder>", r"Save_0001.pcsav, AutoSave.pcsav, QuickSave.pcsav, ChapterSave.pcsav", [(Documents, r"BioWare\Mass Effect 2\Save")], ["*.pcsav"], 2),
    game!("Mass Effect 3", FULL, r"Documents\BioWare\Mass Effect 3\Save\<Shepard's career folder>", r"Save_0001.pcsav, AutoSave.pcsav, QuickSave.pcsav", [(Documents, r"BioWare\Mass Effect 3\Save")], ["*.pcsav"], 2),
    game!("Mass Effect Legendary Edition (ME1)", FULL, r"Documents\BioWare\Mass Effect Legendary Edition\Save\ME1\<career folder>", r"<name>_<nn>.pcsav", [(Documents, r"BioWare\Mass Effect Legendary Edition\Save\ME1")], ["*.pcsav"], 2),
    game!("Mass Effect Legendary Edition (ME2)", FULL, r"Documents\BioWare\Mass Effect Legendary Edition\Save\ME2\<career folder>", r"Save_0001.pcsav, AutoSave.pcsav, QuickSave.pcsav", [(Documents, r"BioWare\Mass Effect Legendary Edition\Save\ME2")], ["*.pcsav"], 2),
    game!("Mass Effect Legendary Edition (ME3)", FULL, r"Documents\BioWare\Mass Effect Legendary Edition\Save\ME3\<career folder>", r"Save_0001.pcsav, AutoSave.pcsav, QuickSave.pcsav", [(Documents, r"BioWare\Mass Effect Legendary Edition\Save\ME3")], ["*.pcsav"], 2),
    game!("Mass Effect: Andromeda", FULL, r"Documents\BioWare\Mass Effect Andromeda\Save", r"Career<id>-<slot> (no extension)", [(Documents, r"BioWare\Mass Effect Andromeda\Save")], ["Career*"], 1),
    game!("Medieval Dynasty", "Game screen: health, stamina, food and water, reputation, tax, the technology points, the skill levels and every stack in the bag (money is the Coin item), with a Max crafting materials button. Checked on a public save, not yet tried in the game", r"%LOCALAPPDATA%\Medieval_Dynasty\Saved\SaveGames", "Quicksave_<N>.sav, Autosave_<N>.sav, <name>.sav", [(LocalAppData, r"Medieval_Dynasty\Saved\SaveGames")], ["*.sav"], 1),
    game!("Megabonk", "Opens as JSON: unlocks, progression and stats, every value by name. The save's AES encryption is removed and put back on save (saves up to version 1.0.17; not yet tried on a real save)", r"%USERPROFILE%\AppData\LocalLow\Ved\Megabonk\Saves\CloudDir\<Steam id>", "progression.json, stats.json", [(LocalLow, r"Ved\Megabonk\Saves\CloudDir")], ["progression.json", "stats.json"], 2),
    game!("Metal Gear Solid V: The Phantom Pain", MGSV, r"Steam\userdata\<account>\287700\remote (also 311340)", r"TPP_GAME_DATA, TPP_GAME_DATA0, PERSONAL_DATA, TPP_CONFIG_DATA", [(SteamUser("287700"), "remote"), (SteamUser("311340"), "remote")], ["TPP_GAME_DATA*", "PERSONAL_DATA*", "TPP_CONFIG_DATA*"], 1),
    game!("Metro 2033 Redux", METRO, r"Documents\4A Games\Metro 2033\<account>", r"game save files", [(Documents, r"4A Games\Metro 2033\*"), (Documents, r"4A Games\Metro Redux\*")], ["*"], 1),
    game!("Metro Exodus", METRO, r"Steam\userdata\<account>\412020\remote (and its game<n> folders)", r"m3_auto_save, m3_quick_save, m3_<level>.player", [(SteamUser("412020"), r"remote"), (SteamUser("412020"), r"remote\*")], ["m3_*"], 1),
    game!("Metro: Last Light Redux", METRO, r"Documents\4A Games\Metro Last Light\<account>", r"game save files", [(Documents, r"4A Games\Metro Last Light\*")], ["*"], 1),
    game!("Mewgenics", "Game screen: house gold and food, adventure coins and food, the current day and the save's completion percentage from the SQLite save's properties table (the cats' own data stays packed and is kept as it is). Checked on a public save, not yet tried in the game", r"%APPDATA%\Glaiel Games\Mewgenics\<Steam id>\saves", r"steamcampaign01.sav", [(AppData, r"Glaiel Games\Mewgenics")], ["steamcampaign*.sav"], 3),
    game!("Minecraft (Java Edition)", MINECRAFT, r"%APPDATA%\.minecraft\saves\<world>", r"level.dat (and region\*.mca)", [(AppData, r".minecraft\saves")], ["level.dat"], 2),
    game!("Minecraft Dungeons", FULL, r"%USERPROFILE%\Saved Games\Mojang Studios\Dungeons\<id>\Players\Active", "<hero id>.dat (decrypted, and encrypted again on save)", [(SavedGames, r"Mojang Studios\Dungeons")], ["*.dat"], 4),
    game!("Monster Hunter Rise", "Decrypted with your Steam account and encrypted again on save (the layer is checked on test files; no real save has been tried yet); values in the Tree", r"Steam\userdata\<account>\1446780\remote\win64_save", "data001Slot.bin … data003Slot.bin", [(SteamUser("1446780"), r"remote\win64_save")], ["data00*Slot.bin"], 1),
    game!("Monster Hunter Stories 3: Twisted Reflection", VALUES, r"Steam\userdata\<account>\2852190\remote\win64_save", "data*.bin", [(SteamUser("2852190"), r"remote\win64_save")], ["*.bin"], 1),
    game!("Monster Hunter Wilds", VALUES, r"Steam\userdata\<account>\2246340\remote\win64_save", "data*.bin", [(SteamUser("2246340"), r"remote\win64_save")], ["*.bin"], 1),
    game!("Moonlighter", "Game screen: Will's gold, days passed, the banker's invested money, and the backpack and equipped items by name with their counts and enchantment levels in the Tree. Opens as JSON, written back the same way. Checked on a public save, not yet tried in the game", r"%USERPROFILE%\AppData\LocalLow\11BitStudios\Moonlighter", "gameslot (one file per save slot)", [(LocalLow, r"11BitStudios\Moonlighter")], ["gameslot*"], 1),
    game!("Mortal Shell", "Game screen: Tar, the shell points of each shell, max health, deaths, New Game+ level and the Storm Mode run's Tar and Glimpses (inventory item names are game text and aren't shown). Checked on public saves, not yet tried in the game", r"Documents\My Games\MortalShell\Dungeonhaven\Saved\SaveGames", "GPData_<slot>.sav", [(Documents, r"My Games\MortalShell\Dungeonhaven\Saved\SaveGames")], ["GPData_*.sav"], 1),
    game!("Mortal Shell II", "Game screen: Gold, Gloom and Glimpses, their lifetime totals, every item's count and every unlocked shell, weapon and seal, with a Max crafting materials button for the Tarforge smithing materials (99, an estimate). Checked on public saves, not yet tried in the game", r"%LOCALAPPDATA%\MortalShell2\Saved\SaveGames", "WorldState_<n>.sav", [(LocalAppData, r"MortalShell2\Saved\SaveGames")], ["WorldState_*.sav"], 1),
    game!("Mutant Year Zero: Road to Eden", "Game screen: scrap, weapon parts and medpacks, the Ark's artifact points, population and levels, the squad level, and each squad member's level, XP, health, sanity and mutation points, with a Max crafting materials button for scrap and weapon parts. Checked on a public save, not yet tried in the game", r"%LOCALAPPDATA%\ZoneUE4\Saved\SaveGames\Session<N>", "Slot<N>.sav, Autosave.sav", [(LocalAppData, r"ZoneUE4\Saved\SaveGames")], ["Slot*.sav", "Autosave.sav"], 2),
    game!("My Summer Car", "Game screen: money and the needs (hunger, thirst, fatigue, stress, urine, dirtiness); every other single value of the Easy Save 2 file by name. Checked on a public save, not yet tried in the game", r"%USERPROFILE%\AppData\LocalLow\Amistech\My Summer Car", "defaultES2File.txt, items.txt", [(LocalLow, r"Amistech\My Summer Car")], ["defaultES2File.txt", "items.txt"], 1),
    game!("Neverwinter Nights: Enhanced Edition", "Game screen: gold, experience, ability scores, hit points, alignment, skills and item stack sizes of the player (the module .sav, with player.bic kept in step) or a vault character (.bic). Not yet tried in the game", r"Documents\Neverwinter Nights\saves\<nnnnnn> - <name> (characters: Documents\Neverwinter Nights\localvault)", r"<Module>.sav (the player as the game loads it), player.bic, localvault\*.bic", [(Documents, r"Neverwinter Nights\saves"), (Documents, r"Neverwinter Nights\localvault")], ["*.sav", "*.bic"], 2),
    game!("NieR:Automata", "Game screen: money, experience (with the level it gives), item and material counts, weapon levels, and Max crafting materials. Checked on a real save, not yet tried in the game", r"Documents\My Games\NieR_Automata", r"SlotData_0.dat … SlotData_2.dat (one per slot)", [(Documents, r"My Games\NieR_Automata")], ["SlotData_*.dat"], 1),
    game!("Noita", "Game screen: gold, gold spent and health from player.xml. Checked on a public save, not yet tried in the game", r"%USERPROFILE%\AppData\LocalLow\Nolla_Games_Noita\save00", r"player.xml (and world_state.xml)", [(LocalLow, r"Nolla_Games_Noita\save00")], ["player.xml"], 1),
    game!("Octopath Traveler", "Game screen: money (leaves) and the eight travellers' level, EXP, JP and current HP/SP, changed in place. Read from a sample save; changing them hasn't been tried in the game yet", r"Documents\My Games\Octopath_Traveler\<Steam id>\SaveGames", r"SaveData0.sav (autosave), SaveData1.sav …", [(Documents, r"My Games\Octopath_Traveler\*\SaveGames"), (GamePass("39EA002F.FrigateMS_n746a19ndrrjg"), "")], ["SaveData*.sav"], 1),
    game!("Octopath Traveler II", "Game screen: money (leaves) and the eight travellers' level, EXP, JP and current HP/SP, changed in place", r"Documents\My Games\Octopath_Traveler2\Steam\<Steam id>\SaveGames", r"SaveData0.sav (autosave), SaveData1.sav …", [(Documents, r"My Games\Octopath_Traveler2\*\*\SaveGames")], ["SaveData*.sav"], 1),
    game!("Okami HD", HEX, r"Steam\userdata\<account>\587620\remote", "OKAMI (all 30 save slots in one file)", [(SteamUser("587620"), r"remote")], ["OKAMI"], 1),
    game!("OMORI", "Game screen: Clams, each friend's level, heart and juice in Headspace and Faraway Town, and the count of every snack, toy, weapon and charm by name. Checked on public saves, not yet tried in the game", r"Steam\steamapps\common\OMORI\www\save", "file1.rpgsave, file2.rpgsave, file3.rpgsave …", [(SteamGame("OMORI"), r"www\save")], ["file*.rpgsave"], 1),
    game!("Onimusha: Way of the Sword", FULL, r"Steam\userdata\<account>\2638890\remote\win64_save", "data001Slot.bin", [(SteamUser("2638890"), r"remote\win64_save")], ["*.bin"], 1),
    game!("OpenFL and HaxeFlixel games (Haxe saves)", "Every value by name (numbers, flags, text, lists, maps), written back with its own kind; dates, bytes and shared references are kept as they were. Covers Friday Night Funkin' and its engines and mods. Checked on a public Friday Night Funkin' save, written back byte for byte", r"%APPDATA%\<company>\<game>", r"<name>.sol (text, unlike Flash's .sol)", [(AppData, "*")], ["*.sol"], 4),
    game!("Ori and the Blind Forest", "Game screen: level, experience, ability points, life and energy cells, keystones, mapstones, deaths, and every skill and ability-tree upgrade to tick on or off; the load menu's copy of life and energy is kept in step. Definitive Edition and the original. Checked on a public save, not yet tried in the game", r"%LOCALAPPDATA%\Ori and the Blind Forest DE (the original: %LOCALAPPDATA%\Ori and the Blind Forest)", "saveFile0.sav … saveFile9.sav (and _bkup copies)", [(LocalAppData, "Ori and the Blind Forest DE"), (LocalAppData, "Ori and the Blind Forest")], ["saveFile*.sav"], 1),
    game!("Oxygen Not Included", FULL, r"Documents\Klei\OxygenNotIncluded\cloud_save_files\<Steam id>\<colony> (or save_files\<colony>)", "<colony>.sav", [(Documents, r"Klei\OxygenNotIncluded\cloud_save_files\*"), (Documents, r"Klei\OxygenNotIncluded\save_files")], ["*.sav"], 2),
    game!("Palworld", FULL, r"%LOCALAPPDATA%\Pal\Saved\SaveGames\<Steam id>\<world>", r"Level.sav, Players\*.sav", [(LocalAppData, r"Pal\Saved\SaveGames"), (GamePass("PocketpairInc.Palworld_ad4psfrxyesvt"), "")], ["*.sav"], 4),
    game!("Pathfinder: Kingmaker", FULL, r"%USERPROFILE%\AppData\LocalLow\Owlcat Games\Pathfinder Kingmaker\Saved Games", r"Manual_*.zks, Auto_*.zks, Quick_*.zks", [(LocalLow, r"Owlcat Games\Pathfinder Kingmaker\Saved Games")], ["*.zks"], 1),
    game!("Pathfinder: Wrath of the Righteous", FULL, r"%USERPROFILE%\AppData\LocalLow\Owlcat Games\Pathfinder Wrath Of The Righteous\Saved Games", r"Manual_*.zks, Auto_*.zks, Quick_*.zks", [(LocalLow, r"Owlcat Games\Pathfinder Wrath Of The Righteous\Saved Games")], ["*.zks"], 1),
    game!("Persona 3 Portable", FULL, r"Steam\userdata\<account>\1809700\remote", r"P3PSAVE0001.BIN …", [(SteamUser("1809700"), r"remote")], ["P3PSAVE*.BIN"], 1),
    game!("Persona 3 Reload", P3R, r"%APPDATA%\SEGA\P3R\Steam\<Steam id>", r"SaveData*.sav", [(AppData, r"SEGA\P3R\Steam\*")], ["*.sav"], 2),
    game!("Persona 4 Golden", FULL, r"Steam\userdata\<account>\1113000\remote", r"data0001.bin …", [(SteamUser("1113000"), r"remote")], ["data*.bin"], 1),
    game!("Persona 5 Royal", FULL, r"%APPDATA%\SEGA\P5R\Steam\<Steam id>\savedata\DATA<nn>", r"DATA.DAT (encrypted)", [(AppData, r"SEGA\P5R\Steam\*\savedata"), (GamePass("SEGAofAmericaInc.F0cb6b3aer_s751p9cej88mt"), "")], ["DATA.DAT", "SYSTEM.DAT"], 2),
    game!("Phasmophobia", "Game screen: money, level, prestige and experience, how many of each item you own, item tiers and their unlocks, and the journal's statistics; each value is changed in place and the file encrypted again on save. Checked on public test saves, not yet tried in the game", r"%USERPROFILE%\AppData\LocalLow\Kinetic Games\Phasmophobia", r"SaveFile.txt", [(LocalLow, r"Kinetic Games\Phasmophobia")], ["SaveFile.txt"], 1),
    game!("Pizza Tower", "Game screen: completion, enemies killed and play time, and every level's score, rank, treasure and secrets, bosses included. Checked on a public save, not yet tried in the game", r"%LOCALAPPDATA%\PizzaTower_GM2", r"saveData.ini (and the numbered save slots)", [(LocalAppData, r"PizzaTower_GM2")], ["saveData*.ini", "*.ini"], 1),
    game!("Planescape: Torment: Enhanced Edition", IE_PST, r"Documents\Planescape Torment - Enhanced Edition\save\<nnnnnnnnn-Name>", r"BALDUR.gam (BALDUR.SAV holds the areas)", [(Documents, r"Planescape Torment - Enhanced Edition\save")], ["*.gam"], 2),
    game!("Plants vs. Zombies (Game of the Year Edition)", "Game screen: money, Adventure level and completions, Crazy Dave's shop, supplies, Survival progress, and the Zen Garden's plants (mini-game and puzzle trophies are on the Values tab). Checked on a real save, not yet tried in the game", r"C:\ProgramData\PopCap Games\PlantsVsZombies\userdata (Steam: C:\ProgramData\Steam\PlantsVsZombies\userdata, or Steam\userdata\<account>\3590\remote)", r"user1.dat, user2.dat … (users.dat lists the profiles)", [(ProgramData, r"PopCap Games\PlantsVsZombies\userdata"), (ProgramData, r"Steam\PlantsVsZombies\userdata"), (SteamUser("3590"), "remote")], ["user*.dat"], 2),
    game!("PlayStation 2 memory cards (PCSX2)", "Every file in every save folder on the card opens on its own (Tools › Open another file in this archive) and is written back with the card's error-correction codes made again; use the value search for the numbers. .ps2 images with ECC and .bin images without. Checked on cards made by mymc, the reference tool", r"Documents\PCSX2\memcards", r"Mcd001.ps2, Mcd002.ps2", [(Documents, r"PCSX2\memcards")], ["*.ps2", "*.bin"], 1),
    game!("PlayStation memory cards (emulators)", "Each save on the card opens on its own (Tools › Open another file in this archive lists them by product code and title) and is written back into its own blocks; use the value search for the numbers. Raw, DexDrive and VGS images. Checked on a public card image", r"Documents\DuckStation\memcards (ePSXe: <install folder>\memcards)", r"shared_card_1.mcd, <game>_1.mcd, epsxe000.mcr …", [(Documents, r"DuckStation\memcards")], ["*.mcd", "*.mcr", "*.mc", "*.mem", "*.vgs", "*.gme"], 1),
    game!("Pokémon Infinite Fusion", "Game screen: money, coins, Battle Points, badges and the difficulty; every Pokémon in the party and PC boxes (fusions shown with their head and body) with level and Exp. kept in step, nature, shiny, happiness, IVs and EVs, and stats worked out again with the game's own formula; and how many of each item is in the bag. Checked on real saves (game versions 5 and 6), not yet tried in the game", r"%APPDATA%\infinitefusion", r"File A.rxdata, File B.rxdata … (backups\ holds the game's own backups)", [(AppData, "infinitefusion")], ["File *.rxdata"], 1),
    game!("Potion Craft: Alchemist Simulator", "Game screen: gold, popularity, karma, experience and the whole inventory (ingredients, crafted potions) with a Max crafting materials button for the plant, mushroom and crystal ingredients. The .pcsave is decoded to JSON and encoded again on save. Checked on a public save, not yet tried in the game", r"%USERPROFILE%\AppData\LocalLow\niceplay games\Potion Craft\SavesSteam", "<slot>.pcsave", [(LocalLow, r"niceplay games\Potion Craft")], ["*.pcsave"], 2),
    game!("Pragmata", "Game screen: the difficulty rank points, New Game+ count and game time, every item and currency by its game code with its quantity, each weapon's level and unlock, and the perks. Items, weapons and perks use the game's codes because its text files aren't read. Checked on real saves and in the game. Close the game and turn Steam Cloud off for it first, or Steam may bring the old save back", r"Steam\userdata\<account>\3357650\remote\win64_save", "data*.bin", [(SteamUser("3357650"), r"remote\win64_save")], ["*.bin"], 1),
    game!("Prey (2017)", "Unpacked for Value search: the RC4 encryption and zlib chunks are removed and put back, with the save's MD5 written again (no save of this game has been tried yet)", r"%USERPROFILE%\Saved Games\Arkane Studios\Prey\SaveGames (Prey_MS for the Microsoft Store version)", r"Campaign<n>\<slot>\save.CSF (the player), <level>.level (each area)", [(SavedGames, r"Arkane Studios\Prey\SaveGames"), (SavedGames, r"Arkane Studios\Prey_MS\SaveGames")], ["save.CSF"], 3),
    game!("Prison Architect", "Game screen: the bank balance, play time and the main prison settings, changed in place in the (often very large) text save. Checked on public saves, not yet tried in the game", r"%LOCALAPPDATA%\Introversion\Prison Architect\saves", "<name>.prison", [(LocalAppData, r"Introversion\Prison Architect\saves")], ["*.prison"], 1),
    game!("Quasimorph", "Opens as JSON (with its byte-order mark kept): the session in each slot, every value by name. Most numbers are stored as text (\"4355\"), so keep them in quotes", r"%USERPROFILE%\AppData\LocalLow\Magnum Scriptum LTD\Quasimorph", r"slot_0_session.dat … slot_2_session.dat", [(LocalLow, r"Magnum Scriptum LTD\Quasimorph")], ["slot_*_session.dat"], 1),
    game!("R.E.P.O.", "Game screen: level, money, lives, the charging station and total haul; each player's health and upgrades; the items in the truck and bought in total. Decrypted and encrypted again on save. Checked on a public save, not yet tried in the game", r"%USERPROFILE%\AppData\LocalLow\semiwork\Repo\saves\<save>", r"<save>.es3", [(LocalLow, r"semiwork\Repo\saves")], ["*.es3"], 2),
    game!("Red Dead Redemption", "Game screen: money, honor, fame, the Dead Eye meter and consumables (snake oil, moonshine, chewing tobacco, medicine, tonics, bait, horse pills…); weapons, horses and kit listed as owned. Decrypted and encrypted again with its CRC-32. Story, Hardcore and Undead Nightmare saves of the PC release; PS4 and Switch saves use the same format. Checked on real saves, not yet tried in the game", r"Documents\Rockstar Games\Red Dead Redemption\Profiles\<id>", r"RDR2SAVE0.SAV, REDDEAD2HCOR0.SAV, RDR2ZOMBIESAVE0.SAV … (the PC port's own names)", [(Documents, r"Rockstar Games\Red Dead Redemption\Profiles\*")], ["*.SAV"], 1),
    game!("Red Dead Redemption 2", "Game screen (story mode, PC): cash, honor, the health, stamina and Dead Eye cores and XP, weight, saved health, camp funds, the satchel's ammunition, consumables, herbs and provisions (with a Max crafting materials button for herbs, feathers and animal fat), horses' dirt and hunger, 17 statistics and the clock; bounties are shown. Missing supplies can be added to the satchel (experimental). Challenge ranks for the nine story challenges, and compendium studied, discovered and owned ticks. Decrypted, with every checksum written again and encrypted on save. Checked on real saves, not yet tried in the game", r"Documents\Rockstar Games\Red Dead Redemption 2\Profiles\<id>", r"SRDR30000 … SRDR30015", [(Documents, r"Rockstar Games\Red Dead Redemption 2\Profiles\*")], ["SRDR*"], 1),
    game!("Remnant 2", HEX_UNPACKED, r"%LOCALAPPDATA%\Remnant2\Saved\SaveGames\<Steam id> (Microsoft Store and Epic: the same folder under the account)", "profile.sav, save_0.sav …", [(LocalAppData, r"Remnant2\Saved\SaveGames")], ["*.sav"], 2),
    game!("Ren'Py visual novels", FULL, r"%APPDATA%\RenPy\<game>", "1-1-LT1.save, persistent", [(AppData, "RenPy")], ["*.save", "persistent"], 2),
    game!("Resident Evil 4 (2023)", "Decrypted with your Steam account and encrypted again on save (checked byte for byte on a public test file; edits not yet tried in the game); every value in the Tree", r"Steam\userdata\<account>\2050650\remote\win64_save", "data*.bin", [(SteamUser("2050650"), r"remote\win64_save")], ["*.bin"], 1),
    game!("Resident Evil 5", "Game screen: money and exchange points (Steam edition); the save is decrypted and its checksum made again. The old Games for Windows Live save isn't supported. Checked on a public save, not yet tried in the game", r"Steam\userdata\<account>\21690\remote", "savedata.bin", [(SteamUser("21690"), "remote")], ["savedata.bin"], 1),
    game!("Resident Evil Requiem", FULL, r"Steam\userdata\<account>\3764200\remote\win64_save", "data*.bin", [(SteamUser("3764200"), r"remote\win64_save")], ["*.bin"], 1),
    game!("RimWorld", FULL, r"%USERPROFILE%\AppData\LocalLow\Ludeon Studios\RimWorld by Ludeon Studios\Saves", "<colony>.rws", [(LocalLow, r"Ludeon Studios\RimWorld by Ludeon Studios\Saves")], ["*.rws"], 1),
    game!("Risk of Rain 2", "Game screen: Lunar coins, coins collected, runs and play time of a profile XML. Unlocks and logbook lists are in the Tree. Checked on a public profile, not yet tried in the game", r"Steam\userdata\<account>\632360\remote\UserProfiles", r"<profile id>.xml", [(SteamUser("632360"), r"remote\UserProfiles")], ["*.xml"], 1),
    game!("Risk of Rain Returns", "Opens as JSON, every value by name", r"Steam\userdata\<account>\1337520\remote", r"save.json", [(SteamUser("1337520"), r"remote")], ["save.json"], 1),
    game!("Roboquest", "Game screen: wrenches, wrenches spent, difficulty, class, player level and the run statistics. Plain Unreal GVAS; checked on a public save, not yet tried in the game", r"%LOCALAPPDATA%\RoboQuest\Saved\SaveGames", r"RQProfile.sav", [(LocalAppData, r"RoboQuest\Saved\SaveGames")], ["RQProfile*.sav"], 1),
    game!("Rogue Legacy 2", "Opens as .NET data: gold (GoldCollected), ore (EquipmentOreCollected) and aether (RuneOreCollected) in Player.rc2dat, with every other value by name. Values are changed in place; there is no checksum", r"%USERPROFILE%\AppData\LocalLow\Cellar Door Games\Rogue Legacy 2\Saves\Steam\<user id>\<profile>", "Player.rc2dat (and the other .rc2dat files)", [(LocalLow, r"Cellar Door Games\Rogue Legacy 2\Saves")], ["*.rc2dat"], 4),
    game!("RPG Maker MV / MZ games", FULL, r"Inside each game's folder: www\save (MV) or save (MZ); Steam games are found in steamapps\common", r"file1.rpgsave, file2.rpgsave … (MZ: file1.rmmzsave)", [(SteamGame(""), r"*\www\save"), (SteamGame(""), r"*\save")], ["file*.rpgsave", "file*.rmmzsave"], 1),
    game!("RPG Maker XP games", "Game screen: gold, steps, the party and every actor (level, EXP, HP, SP and stat bonuses) and the items, weapons and armour you carry, by database number. Checked on public To the Moon saves", r"Inside each game's folder (Steam games are found in steamapps\common)", "Save1.rxdata, Save2.rxdata …", [(SteamGame(""), "*")], ["Save*.rxdata"], 2),
    game!("Rune Factory 4 Special", FULL, r"%APPDATA%\Rune Factory 4 Special", "rf4_s01.sav … (rf4_sys.sav is the system save)", [(AppData, "Rune Factory 4 Special")], ["rf4_s*.sav"], 1),
    game!("Saints Row IV", "Game screen: cash (in cents), data clusters, respect level and respect; the save's checksum (CRC-32) is rewritten. Checked on public saves, not yet tried in the game", r"Steam\userdata\<account>\206420\remote", "sr4save_0.sr4s_pc …", [(SteamUser("206420"), "remote")], ["sr4save_*.sr4s_pc"], 1),
    game!("Saints Row: The Third (and Remastered)", "Game screen: cash and the crib stash (in cents), respect level and respect, stunt jumps, barnstorms and difficulty; the save's checksum (CRC-32) is rewritten. Checked on public saves, not yet tried in the game", r"Steam\userdata\<account>\55230\remote (Remastered: 978300)", "sr3save_00.sr3s_pc …", [(SteamUser("55230"), "remote"), (SteamUser("978300"), "remote")], ["sr3save_*.sr3s_pc"], 1),
    game!("Salt and Sanctuary", "Game screen: level, salt, gold, kills, deaths, sanctuary rests, corruption and the playthrough number. Items and the skill tree are not edited yet. Checked on public saves, not yet tried in the game", r"Documents\Salt and Sanctuary\savedata", "dat<n>.slv (one per character)", [(Documents, r"Salt and Sanctuary\savedata")], ["dat*.slv"], 1),
    game!("Satisfactory", HEX_UNPACKED, r"%LOCALAPPDATA%\FactoryGame\Saved\SaveGames\<Steam id>", "<session name>_autosave_0.sav", [(LocalAppData, r"FactoryGame\Saved\SaveGames")], ["*.sav"], 2),
    game!("Schedule I", SCHEDULE_I, r"%USERPROFILE%\AppData\LocalLow\TVGS\Schedule I\Saves\<Steam id>\SaveGame_<n>", r"Money.json, Rank.json, Time.json, Game.json (open one file at a time)", [(LocalLow, r"TVGS\Schedule I\Saves")], ["Money.json", "Rank.json", "Time.json", "Game.json"], 3),
    game!("Sekiro: Shadows Die Twice", FULL, r"%APPDATA%\Sekiro\<Steam id>", "S0000.sl2", [(AppData, "Sekiro")], ["S0000.sl2"], 2),
    game!("Shadow Warrior 2", "Game screen: level, experience, money, crystal points, perk points, health, mana, rage and every kind of ammunition; weapons and upgrades are in the Tree. Each .bin is a zip whose text is opened and packed again for you. Checked on a public save, not yet tried in the game", r"%USERPROFILE%\Saved Games\Flying Wild Hog\Shadow Warrior 2\profiles\<id>", "save_auto.bin (the player), progress.bin and save_auto_level_*.bin", [(SavedGames, r"Flying Wild Hog\Shadow Warrior 2\profiles\*")], ["save_auto*.bin", "progress*.bin"], 2),
    game!("Shape of Dreams", "Game screen: stardust, spent stardust, completed reveries and the preferred traveler and difficulty from the profile file, and the mastery level, mastery points and record of every traveler from the stats file. The unlocked items are in an encrypted file that is not opened. Checked on public saves, not yet tried in the game", r"%USERPROFILE%\AppData\LocalLow\Lizard Smoothie\Shape of Dreams\QuickSave", "r_profile<id>.json, r_stats<id>.json", [(LocalLow, r"Lizard Smoothie\Shape of Dreams\QuickSave")], ["r_profile*.json", "r_stats*.json"], 1),
    game!("Sid Meier's Civilization VII", "Unpacked for Value search and packed again on save (the header, chunk layout and footer are kept). Players' gold and influence are on the Tree for the game versions that keep them in the known place. Checked on public sample saves for unpack and repack; not yet tried in the game", r"Documents\My Games\Sid Meier's Civilization VII\Saves (Epic: …VII (Epic)\Saves)", "<name>.Civ7Save", [(Documents, r"My Games\Sid Meier's Civilization VII\Saves"), (Documents, r"My Games\Sid Meier's Civilization VII (Epic)\Saves")], ["*.Civ7Save"], 3),
    game!("Skyrim (Special / Anniversary Edition)", FULL, r"Documents\My Games\Skyrim Special Edition\Saves", "*.ess", [(Documents, r"My Games\Skyrim Special Edition\Saves"), (Documents, r"My Games\Skyrim Special Edition GOG\Saves"), (Documents, r"My Games\Skyrim\Saves"), (Documents, r"My Games\Skyrim VR\Saves")], ["*.ess"], 2),
    game!("Slay the Spire", FULL, r"<Steam library>\steamapps\common\SlayTheSpire\saves", "IRONCLAD.autosave …", [(SteamGame("SlayTheSpire"), "saves")], ["*.autosave"], 1),
    game!("Slay the Spire 2", "Game screen: each character's highest and preferred Ascension, wins, losses, win streaks and play time, and the profile's score, from progress.save. Checked on a public save, not yet tried in the game", r"%APPDATA%\SlayTheSpire2\steam\<Steam id>\profile1\saves", r"progress.save", [(AppData, r"SlayTheSpire2\steam")], ["progress.save"], 3),
    game!("Sleeping Dogs: Definitive Edition", "Game screen: money and face, cop and triad XP. The save is unscrambled and scrambled again, with its checksum, on save. Checked on public saves and a real save, not yet tried in the game", r"<install folder>\data (Steam: steamapps\common\SleepingDogsDefinitiveEdition\data)", "HK GameSlot 1 ... , HK Autosave Slot (no extension)", [(SteamGame("SleepingDogsDefinitiveEdition"), "data")], ["HK GameSlot*", "HK Autosave*"], 1),
    game!("Slime Rancher", "Game screen: newbucks (the load screen's copy follows), health, energy, radiation and slime keys, changed in place. Checked on public saves of version 1.4, not yet tried in the game; Slime Rancher 2 isn't read yet", r"%USERPROFILE%\AppData\LocalLow\Monomi Park\Slime Rancher", "<date>_<name>_<n>.sav", [(LocalLow, r"Monomi Park\Slime Rancher")], ["*.sav"], 1),
    game!("SnowRunner", "Game screen: money, rank, experience, the statistics, the difficulty factors and the trucks in the warehouse. The save is JSON followed by a NUL byte, kept as it is. Checked on two real saves, not yet tried in the game", r"Steam\userdata\<account>\1465360\remote (Epic and Windows Store: Documents\My Games\SnowRunner\base\storage\<id>)", r"CompleteSave.cfg, CompleteSave1.cfg … (the profile is in user_profile.cfg)", [(SteamUser("1465360"), "remote"), (Documents, r"My Games\SnowRunner\base\storage\*")], ["CompleteSave*.cfg"], 1),
    game!("Sonic Mania", "Game screen: every Mania Mode and Encore Mode slot (state, character, zone, lives, score, Chaos Emeralds, continues, next special stage), changed in place. Checked on public saves, not yet tried in the game", r"Steam\userdata\<account>\584400\remote", "SaveData.bin", [(SteamUser("584400"), "remote")], ["SaveData.bin"], 1),
    game!("Sons of the Forest", SOTF, r"%USERPROFILE%\AppData\LocalLow\Endnight\SonsOfTheForest\Saves\<Steam id>\Singleplayer (or Multiplayer)\<save id>", r"PlayerStateSaveData.json, PlayerInventorySaveData.json (open one file at a time)", [(LocalLow, r"Endnight\SonsOfTheForest\Saves")], ["PlayerStateSaveData.json", "PlayerInventorySaveData.json"], 4),
    game!("Space Haven", "Game screen: credits, the day and time, the difficulty options and a table of each ship's crew (health, food, rest, comfort, mood, the four attributes and the skill levels). Checked on a real save, not yet tried in the game", r"<install folder>\savegames\<save name>\save (Steam: Steam\steamapps\common\Space Haven\savegames)", "game (no extension)", [(SteamGame("Space Haven"), r"savegames\*\save")], ["game"], 1),
    game!("Star Wars Jedi: Fallen Order", "Game screen: skill points (unspent and total), stims, health and Force upgrades, secrets and map progress, deaths. The zlib-packed actor records are opened and packed again on save. Checked on public saves, not yet tried in the game", r"%USERPROFILE%\Saved Games\Respawn\JediFallenOrder", "SaveGame00.sav, Backup00.sav", [(SavedGames, r"Respawn\JediFallenOrder")], ["SaveGame*.sav", "Backup*.sav", "savegame*.sav"], 1),
    game!("Star Wars Jedi: Survivor", "Game screen (shared with Fallen Order): skill points (unspent and total), stims, health and Force upgrades, secrets and map progress, deaths. The zlib-packed actor records are opened and packed again on save. Checked on public saves, not yet tried in the game", r"%USERPROFILE%\Saved Games\Respawn\JediSurvivor", "SaveGame00.sav … SaveGame09.sav", [(SavedGames, r"Respawn\JediSurvivor")], ["*SaveGame*.sav"], 1),
    game!("Star Wars: Knights of the Old Republic", KOTOR, r"<install folder>\saves\<nnnnnn> - <name> (Steam: steamapps\common\swkotor\saves)", r"SAVEGAME.sav, PARTYTABLE.res (auto-saves: pifo.ifo holds the player)", [(SteamGame("swkotor"), "saves")], ["SAVEGAME.sav", "PARTYTABLE.res", "pifo.ifo"], 2),
    game!("Star Wars: Knights of the Old Republic II \u{2013} The Sith Lords", KOTOR, r"<install folder>\saves\<nnnnnn> - <name> (Steam Cloud: <install folder>\cloudsaves\<Steam id>\<nnnnnn> - <name>)", r"SAVEGAME.sav, PARTYTABLE.res (auto-saves: pifo.ifo holds the player)", [(SteamGame("Knights of the Old Republic II"), "saves"), (SteamGame("Knights of the Old Republic II"), r"cloudsaves\*")], ["SAVEGAME.sav", "PARTYTABLE.res", "pifo.ifo"], 2),
    game!("Starbound", STARBOUND, r"<Steam library>\steamapps\common\Starbound\storage\player", "<id>.player (one per character)", [(SteamGame("Starbound"), r"storage\player")], ["*.player"], 1),
    game!("Stardew Valley", STARDEW, r"%APPDATA%\StardewValley\Saves\<Farm>_<id>", "<Farm>_<id> (no extension)", [(AppData, r"StardewValley\Saves")], ["*_[0-9]*"], 2),
    game!("Stellar Blade", "Game screen: Eve's health, shield, gauges, level, experience and skill points, the play time and difficulty, and an inventory table with counts and levels (item names are the game's internal ids). The save's CRC-32 is written again. Checked on a real early-game save, not yet tried in the game", r"%LOCALAPPDATA%\SB\Saved\SaveGames\<Steam id>", "StellarBladeSave00.sav …", [(LocalAppData, r"SB\Saved\SaveGames")], ["StellarBladeSave*.sav"], 2),
    game!("Stellaris", FULL, r"Documents\Paradox Interactive\Stellaris\save games\<empire>", "autosave_<date>.sav, ironman.sav", [(Documents, r"Paradox Interactive\Stellaris\save games")], ["*.sav"], 2),
    game!("Stoneshard", FULL, r"%LOCALAPPDATA%\Ink\Stoneshard\characters_v1\character_<n>\<save folder>", "data.sav (unpacked, with its MD5 made again on save)", [(LocalAppData, r"Ink\Stoneshard\characters_v1")], ["data.sav"], 3),
    game!("Street Fighter 6", "Decrypted with your Steam account and encrypted again on save (the layer is the one Monster Hunter Rise uses; not yet tried on a real save); values in the Tree", r"Steam\userdata\<account>\1364780\remote\win64_save", "data*.bin", [(SteamUser("1364780"), r"remote\win64_save")], ["data*.bin"], 1),
    game!("Streets of Rogue", "Game screen: nuggets, wins, deaths and games played, and every agent, item, trait and floor with whether it is unlocked and its cost. Checked on a public unlocks file, not yet tried in the game", r"Documents\Streets of Rogue\CloudData", "GameUnlocks.dat", [(Documents, r"Streets of Rogue\CloudData")], ["GameUnlocks.dat"], 1),
    game!("Subnautica", FULL, r"<Steam library>\steamapps\common\Subnautica\SNAppData\SavedGames\slot0000", "gameinfo.json (game mode)", [(SteamGame("Subnautica"), r"SNAppData\SavedGames\*")], ["gameinfo.json"], 1),
    game!("Subnautica 2", "Game screen: health, oxygen, food and water with their maximums, the player inventory and every locker, vehicle and storage container, changed in place in the save (checked on a real early-access save; needs the Oodle library of any installed game, the same way the game reads it)", r"%LOCALAPPDATA%\Subnautica2\Saved\SaveGames", "savegame_<n>.sav", [(LocalAppData, r"Subnautica2\Saved\SaveGames")], ["savegame_*.sav"], 2),
    game!("Subnautica: Below Zero", FULL, r"<Steam library>\steamapps\common\SubnauticaZero\SNAppData\SavedGames\slot0000", "gameinfo.json (game mode)", [(SteamGame("SubnauticaZero"), r"SNAppData\SavedGames\*")], ["gameinfo.json"], 1),
    game!("Suikoden I & II HD Remaster", "Opens as JSON: party, recruited stars, items, runes and story flags, every value by name. The save's AES encryption is removed and put back on save with the same salt (not yet tried on a real save)", r"<install folder>\Save\<user id>\gsd1 (Suikoden) and gsd2 (Suikoden II)", "Data1 … Data16", [(SteamGame("Suikoden I and II HD Remaster"), "Save")], ["Data*"], 3),
    game!("Super Kirby Clash (3DS, decrypted saves)", "Game screen: apple gems and the red, blue, yellow and rare shards. Checked on a public save, not yet tried in the game", r"The decrypted save from Checkpoint, JKSM or GodMode9, or Citra's sdmc folder", "savedata.dat (54,344 bytes)", [(AppData, r"Citra\sdmc")], ["savedata.dat"], 4),
    game!("Super Mario 64 (N64, emulator saves)", "Game screen: for each of the four files the caps, keys, switches, secret stars, every course's six stars, 100-coin star, cannon and coin record. Both copies of the file and the checksums are kept right. Checked on public saves, not yet tried in the game", N64_FOLDER, r"<ROM name>.eep (512 or 2048 bytes)", [(Documents, "Project64"), (AppData, "Project64"), (Documents, "ROMs")], ["*Mario 64*.eep"], 3),
    game!("Super Metroid (SNES, emulator saves)", "Game screen: for each of the three slots energy and reserve tanks, missiles, super missiles, power bombs, play time and the beams, suits and boots Samus has and has equipped. Each slot's sums, and their copies at the end of the file, are kept right. Checked on a public save, not yet tried in the game", SNES_FOLDER, r"<ROM name>.srm (8 KiB)", [(Documents, "Snes9x"), (Documents, "ROMs")], ["*Super Metroid*.srm", "*Super Metroid*.sav"], 3),
    game!("Super Smash Bros. Ultimate (Switch)", "Game screen: gold, tickets, hammers, snacks, SP and the spirit board item counts. Checked on a public save, not yet tried in the game", SWITCH_FOLDER, "system_data.bin (title id 01006A800016E000)", [(AppData, r"yuzu\nand\user\save\0000000000000000"), (AppData, r"Ryujinx\bis\user\save")], ["system_data.bin"], 4),
    game!("Tales of Symphonia", "Game screen: gald, grade points, the records, all nine characters (level, experience, HP, TP and the stats) and every item. Gald is written in both places the game keeps it. Checked on a real save, not yet tried in the game", r"Steam\userdata\<account>\372360\remote", "TOSSaveData0.dat (and TOSSaveData1.dat … for more slots)", [(SteamUser("372360"), "remote")], ["TOSSaveData*.dat"], 1),
    game!("Teardown", "Game screen: cash, every tool's ammunition and range, damage and size upgrades. Checked on a public save, not yet tried in the game", r"Documents\Teardown", r"savegame.xml", [(Documents, r"Teardown")], ["savegame.xml"], 1),
    game!("Terra Invicta", "Game screen: every faction's money, influence, operations, research, boost, mission control and space resources. Checked on public saves, not yet tried in the game", r"Documents\My Games\TerraInvicta\Saves", "<name>.gz", [(Documents, r"My Games\TerraInvicta\Saves")], ["*.gz"], 1),
    game!("Terraria (characters)", TERRARIA_PLR, r"Documents\My Games\Terraria\Players", "<character>.plr", [(Documents, r"My Games\Terraria\Players"), (Documents, r"My Games\Terraria\tModLoader\Players")], ["*.plr"], 1),
    game!("Terraria (worlds)", FULL, r"Documents\My Games\Terraria\Worlds", "<world>.wld (Terraria 1.4 or later)", [(Documents, r"My Games\Terraria\Worlds"), (Documents, r"My Games\Terraria\tModLoader\Worlds")], ["*.wld"], 1),
    game!("The Binding of Isaac: Rebirth", "Game screen: Eden tokens, win streaks, deaths and the other counters, and every achievement, seen collectible and challenge as an unlock flag, for Rebirth, Afterbirth, Afterbirth+ and Repentance. The save's checksum is written again. Checked on public saves of all four versions, not yet tried in the game", r"Steam\userdata\<account>\250900\remote (non-Steam: Documents\My Games\Binding of Isaac Repentance)", r"persistentgamedata<n>.dat, ab_/abp_/rep_/rep+ persistentgamedata<n>.dat", [(SteamUser("250900"), r"remote"), (Documents, r"My Games\Binding of Isaac Repentance"), (Documents, r"My Games\Binding of Isaac Afterbirth+"), (Documents, r"My Games\Binding of Isaac Rebirth")], ["*persistentgamedata?.dat"], 1),
    game!("The Elder Scrolls II: Daggerfall (Daggerfall Unity)", "Game screen: gold carried, level, health, magicka and fatigue, the eight attributes, all 35 skills, resistances, reputations and the gold in each region's bank, from SaveData.txt. Checked on a public save, not yet tried in the game", r"%USERPROFILE%\AppData\LocalLow\Daggerfall Workshop\Daggerfall Unity\Saves\<name>-<n>", "SaveData.txt", [(LocalLow, r"Daggerfall Workshop\Daggerfall Unity\Saves")], ["SaveData.txt"], 2),
    game!("The Elder Scrolls III: Morrowind", "Game screen: level, gold, the eight attributes, all 27 skills, health, fatigue and magicka (now and full), and the inventory counts by item id; spells are listed. The save is edited in place, no checksum to keep. Checked on public saves, not yet tried in the game", r"<install folder>\Saves (Steam: Steam\steamapps\common\Morrowind\Saves; GOG: <install folder>\Saves)", "Quicksave.ess, Autosave.ess, Save <n>.ess", [(SteamGame("Morrowind"), "Saves"), (SteamGame("Morrowind"), r"Data Files\Saves")], ["*.ess"], 1),
    game!("The Elder Scrolls IV: Oblivion", "Game screen: level, gold, the eight attributes, all 21 skills and base health; the level is kept equal in the load screen. The save is edited in place, no checksum to keep. Checked on public saves, not yet tried in the game", r"Documents\My Games\Oblivion\Saves", "Save <n> - <name>....ess, quiksave.ess, autosave.ess", [(Documents, r"My Games\Oblivion\Saves"), (Documents, r"My Games\Oblivion GOTY\Saves")], ["*.ess"], 1),
    game!("The Forest", "Game screen: every item the player carries with its amount and the game's stored limit, with a Max crafting materials button that raises the plain materials to those limits. Only the amounts change; the rest of the save stays byte for byte. Checked on public saves, not yet tried in the game", r"%USERPROFILE%\AppData\LocalLow\SKS\TheForest\<Steam id>\SinglePlayer\Slot<N>", "__RESUME__ (also in each Multiplayer slot)", [(LocalLow, r"SKS\TheForest\*\SinglePlayer\*"), (LocalLow, r"SKS\TheForest\*\Multiplayer\*")], ["__RESUME__"], 3),
    game!("The Legend of Zelda: A Link to the Past (SNES, emulator saves)", "Game screen: for each of the three slots hearts, magic, rupees, bombs and arrows with their capacity upgrades, the sword, shield, tunic and every item, and the four bottles. The slot's sum is kept right. Checked on a public save, not yet tried in the game", SNES_FOLDER, r"<ROM name>.srm (8 KiB)", [(Documents, "Snes9x"), (Documents, "ROMs")], ["*Link to the Past*.srm", "*Link to the Past*.sav"], 3),
    game!("The Long Dark", "Opens as JSON, every value by name: the save's LZF layers (the slot file, its boot and global entries, and the JSON kept inside strings) are unpacked and packed again on save. Condition, hunger, thirst, fatigue, skills, inventory and the player's position are in the Tree (not yet tried on a real save)", r"%LOCALAPPDATA%\Hinterland\TheLongDark", "sandbox1 … (no extension)", [(LocalAppData, r"Hinterland\TheLongDark")], ["sandbox*", "ep*", "story*", "challenge*"], 1),
    game!("The Long Drive", "Game screen: the driver's health, food, water and sleep, the distance driven and the world time, from the .tlds save (a .NET BinaryFormatter file, read and written in place). Checked on a public save, not yet tried in the game", r"%USERPROFILE%\AppData\LocalLow\Genesz\TheLongDrive", r"<save>\TLDsaveData.tlds (and TLDsaveProp.tldsp)", [(LocalLow, r"Genesz\TheLongDrive")], ["*.tlds"], 3),
    game!("The Planet Crafter", "Game screen: Terra tokens, each planet's terraformation levels, your oxygen, thirst and health, the game settings, and the raw materials in your backpack (with a Max crafting materials button that fills the free slots, since every item takes one slot). Checked on a real late-game save, not yet tried in the game", r"%USERPROFILE%\AppData\LocalLow\MijuGames\Planet Crafter", r"Survival-1.json … (one per save)", [(LocalLow, r"MijuGames\Planet Crafter")], ["*.json"], 1),
    game!("The Sims (2000) / The Sims Legacy Collection", "Game screen: each family's funds and family friends, and every Sim's skills and personality. Checked on a real neighbourhood, not yet tried in the game", r"%USERPROFILE%\Saved Games\Electronic Arts\The Sims 25\UserData (Legacy Collection); <install folder>\UserData (original)", r"Neighborhood.iff (one per neighbourhood: UserData, UserData2 …)", [(SavedGames, r"Electronic Arts\The Sims 25")], ["Neighborhood.iff"], 2),
    game!("The Sims 2 / The Sims 2 Legacy Collection", "Game screen: each family's funds and every Sim's skills, personality, job performance and aspiration score. The package is written again with the changed parts uncompressed (as SimPE does). Checked on a real neighbourhood, not yet tried in the game", r"Documents\EA Games\The Sims 2\Neighborhoods\N001 (and N002 …); Ultimate Collection / Legacy: Documents\EA Games\The Sims 2 Ultimate Collection", r"N001_Neighborhood.package (one per neighbourhood)", [(Documents, r"EA Games\The Sims 2"), (Documents, r"EA Games\The Sims 2 Ultimate Collection"), (Documents, r"EA Games\The Sims 2 Legacy")], ["*_Neighborhood.package"], 3),
    game!("The Sims 4", "Game screen: one tab per household with its funds and its Sims (names, age, gender). The funds are written back into the game data, which is stored uncompressed, so the file grows; skills and traits aren't mapped. Checked on a real save, not yet tried in the game", r"Documents\Electronic Arts\The Sims 4\saves", r"Slot_00000001.save … (and .save.ver0 backups)", [(Documents, r"Electronic Arts\The Sims 4\saves")], ["Slot_*.save"], 1),
    game!("The Witcher 3", FULL, r"Documents\The Witcher 3\gamesaves", r"ManualSave_*.sav, AutoSave_*.sav", [(Documents, r"The Witcher 3\gamesaves")], ["*.sav"], 1),
    game!("theHunter: Call of the Wild", "Game screen: cash, level, XP and the skill and perk points of the profile, read from the ADF file's own type definitions and edited in place. Checked on public saves, not yet tried in the game", r"Documents\Avalanche Studios\COTW\Saves\<Steam id> (Steam)", "thp_player_profile_adf (also in each slots folder)", [(Documents, r"Avalanche Studios\COTW\Saves\*"), (Documents, r"Avalanche Studios\theHunter Call of the Wild\Saves\*")], ["thp_player_profile_adf"], 2),
    game!("Tiny Tina's Wonderlands", WONDERLANDS, r"Documents\My Games\Tiny Tina's Wonderlands\Saved\SaveGames\<Steam or Epic id>", r"<n>.sav (one per character; profile.sav is the profile)", [(Documents, r"My Games\Tiny Tina's Wonderlands\Saved\SaveGames")], ["?.sav", "??.sav", "???.sav"], 2),
    game!("Titan Quest Anniversary Edition", "Game screen: level, experience, money, unspent points, the five attributes and skill levels (stash files aren't read). Not yet tried in the game", r"Documents\My Games\Titan Quest - Immortal Throne\SaveData\Main\_<name>", r"Player.chr", [(Documents, r"My Games\Titan Quest - Immortal Throne\SaveData\Main"), (Documents, r"My Games\Titan Quest - Immortal Throne\SaveData\User")], ["Player.chr"], 1),
    game!("Tokyo Xtreme Racer (2025)", "Game screen: CP (money) and its limit, PP, garage size and the race records. Checked on a public save, not yet tried in the game", r"%LOCALAPPDATA%\TokyoXtremeRacer\Saved\SaveGames\<Steam id>", "UserData_00.sav", [(LocalAppData, r"TokyoXtremeRacer\Saved\SaveGames")], ["UserData_??.sav"], 2),
    game!("Torchlight II", "Game screen: level, experience, fame, gold, unspent points, the four stats, health, mana and item stacks; scrambled again with its checksum. Saves in the older 0x40 format aren't supported. Not yet tried in the game", r"Documents\My Games\Runic Games\Torchlight 2\save\<Steam id> (modsave\<Steam id> with mods)", r"<NAME>_{<id>}.SVB", [(Documents, r"My Games\Runic Games\Torchlight 2\save"), (Documents, r"My Games\Runic Games\Torchlight 2\modsave")], ["*.svb"], 1),
    game!("Total War Saga: Troy", TOTALWAR, r"%APPDATA%\The Creative Assembly\Troy\save_games", "<name>.save", [(AppData, r"The Creative Assembly\Troy\save_games")], ["*.save"], 1),
    game!("Total War: Attila", TOTALWAR, r"%APPDATA%\The Creative Assembly\Attila\save_games", r"<name>.save", [(AppData, r"The Creative Assembly\Attila\save_games")], ["*.save"], 1),
    game!("Total War: ROME II", "Game screen: every faction's treasury, yours first, with a button to fill it. The campaign inside the save is unpacked, edited with every size above it recomputed, and packed again (the file grows). Read on public campaign saves of the game, not yet tried in the game", r"%APPDATA%\The Creative Assembly\Rome2\save_games", r"<name>.save", [(AppData, r"The Creative Assembly\Rome2\save_games")], ["*.save"], 1),
    game!("Total War: SHOGUN 2", "Game screen: every faction's treasury, yours first, with a button to fill it. The campaign inside the save is unpacked, edited with every size above it recomputed, and packed again (the file grows). Read on public campaign saves of the game, not yet tried in the game", r"%APPDATA%\The Creative Assembly\Shogun2\save_games", r"<name>.save", [(AppData, r"The Creative Assembly\Shogun2\save_games")], ["*.save"], 1),
    game!("Total War: THREE KINGDOMS", "Game screen: every faction's treasury, yours first, with a button to fill it. The campaign inside the save is unpacked, edited with every size above it recomputed, and packed again (the file grows). Read on public campaign saves of the game, not yet tried in the game", r"%APPDATA%\The Creative Assembly\ThreeKingdoms\save_games", r"<name>.save", [(AppData, r"The Creative Assembly\ThreeKingdoms\save_games")], ["*.save"], 1),
    game!("Total War: Warhammer", TOTALWAR, r"%APPDATA%\The Creative Assembly\Warhammer\save_games", r"<name>.save", [(AppData, r"The Creative Assembly\Warhammer\save_games")], ["*.save"], 1),
    game!("Total War: Warhammer II", TOTALWAR, r"%APPDATA%\The Creative Assembly\Warhammer2\save_games", "<name>.save", [(AppData, r"The Creative Assembly\Warhammer2\save_games")], ["*.save"], 1),
    game!("Touhou Luna Nights", "Game screen: gold, level, experience, HP, MP, knife and clock upgrades, the seven gems, the abilities and keys, and the shop counters. The save's base64 lines are opened as one list of 1,000 values and written back the same way. Checked on a public save, not yet tried in the game", r"%LOCALAPPDATA%\touhou_luna_nights", "game0.sav (game1.sav … one per slot)", [(LocalAppData, "touhou_luna_nights")], ["game?.sav"], 1),
    game!("Touhou Mystia's Izakaya", "Opens as JSON: money, level, experience, ingredients and drinks, every value by name", r"%USERPROFILE%\AppData\LocalLow\Epicomic\Touhou Mystia Izakaya\Memory\Save", r"*.memory (one per save)", [(LocalLow, r"Epicomic\Touhou Mystia Izakaya\Memory\Save")], ["*.memory"], 1),
    game!("Trails in the Sky 1st Chapter", FULL, r"Steam\userdata\<account>\3375780\remote (the save folder isn't confirmed yet)", "the game's save files (zstd-packed)", [(SteamUser("3375780"), "remote")], ["*"], 2),
    game!("Trails to Azure / Ao no Kiseki KAI", FULL, r"%USERPROFILE%\Saved Games\FALCOM\Ao\data<nnnn> (KAI: its own Saved Games folder)", "savedata.dat (KAI: NNN_save.bin)", [(SavedGames, r"FALCOM\Ao\*")], ["savedata.dat", "*_save.bin"], 2),
    game!("Travellers Rest", "Game screen: money (in copper), the crafting room, tavern floor and cellar upgrades, and the inventory and action bar by item id with their stack sizes. Checked on a public save, not yet tried in the game", r"%USERPROFILE%\AppData\LocalLow\Louqou\TravellersRest\GameSaves", "SaveFile-<day>-<month>-<year>-<time>.save", [(LocalLow, r"Louqou\TravellersRest\GameSaves")], ["*.save"], 1),
    game!("Tyranny", "Game screen: money, camping supplies and each character's experience, attribute and talent points, attributes and skills. The save's zip is repacked; the rest of its data is kept. Checked on a public save, not yet tried in the game", r"Saved Games\Tyranny", "<id>.savegame", [(SavedGames, "Tyranny")], ["*.savegame"], 1),
    game!("ULTRAKILL", "Game screen: points (P) and every weapon variant and arm you own (generalprogress.bepis); level ranks and secrets in the level files. Checked on public saves, not yet tried in the game", r"<install folder>\Saves\Slot<n> (Steam: Steam\steamapps\common\ULTRAKILL\Saves)", "generalprogress.bepis, lvl<N>progress.bepis", [(SteamGame("ULTRAKILL"), "Saves")], ["generalprogress.bepis"], 2),
    game!("Undertale", "Game screen: stats, gold, EXP, items, phone and equipment (by name), and the route record (kills per area, Genocide and Pacifist flags, boss outcomes); every line of the file can be edited. Not yet tried in the game", r"%LOCALAPPDATA%\UNDERTALE", r"file0 (file9 is a copy; undertale.ini is the load menu)", [(LocalAppData, "UNDERTALE")], ["file0", "file9", "file8"], 1),
    game!("Undertale Yellow", "Game screen: LV, EXP, gold, HP and the other stats, equipment names, the eight item slots and play time from the save file. Checked on public saves, not yet tried in the game", r"%LOCALAPPDATA%\UNDERTALE_YELLOW", r"save.sav (the other files are save02 and tempsave)", [(LocalAppData, "UNDERTALE_YELLOW")], ["*.sav"], 1),
    game!("Unity games (PlayerPrefs)", "Every PlayerPref by name (coins, unlocks, settings): ints, decimals and text, written back with their types. On Windows they're read from the registry and imported back on save; close the game first", r"Windows: the registry (HKCU\Software\<company>\<game>); Linux: ~/.config/unity3d/<company>/<game>; macOS: ~/Library/Preferences", r"<company> - <game>.reg (exported for you), prefs, unity.<company>.<game>.plist", [(UnityPrefs, "")], ["*.reg", "prefs", "unity.*.plist"], 3),
    game!("Valheim (characters)", VALHEIM, r"%USERPROFILE%\AppData\LocalLow\IronGate\Valheim\characters_local (older or Steam Cloud: characters)", "<name>.fch", [(LocalLow, r"IronGate\Valheim\characters_local"), (LocalLow, r"IronGate\Valheim\characters"), (SteamUser("892970"), r"remote\characters")], ["*.fch"], 1),
    game!("Vampire Survivors", "Game screen: coins, power-up ranks and what's unlocked; the checksum is worked out again on save. Not yet tried in the game", r"Steam\userdata\<account>\1794680\remote (older builds: %APPDATA%\Vampire_Survivors\saves)", r"SaveData (older builds: SaveData.sav)", [(SteamUser("1794680"), "remote"), (AppData, r"Vampire_Survivors\saves"), (AppData, "Vampire_Survivors_Data")], ["SaveData", "SaveData.sav"], 1),
    game!("Victoria 3", PDX_BINARY, r"Documents\Paradox Interactive\Victoria 3\save games", "*.v3", [(Documents, r"Paradox Interactive\Victoria 3\save games")], ["*.v3"], 1),
    game!("Wandering Sword", "Game screen: your money, with the map, mount, play time and difficulty shown; the save's zlib wrapping is removed and put back. Checked on public saves, not yet tried in the game", r"%LOCALAPPDATA%\JH\Saved\SaveGames (the game's Unreal project is called JH)", "<number>.sav", [(LocalAppData, r"JH\Saved\SaveGames")], ["*.sav"], 2),
    game!("Warhammer 40,000: Mechanicus", "Game screen: Blackstone, the Ark's bonus points and slots, and each Techpriest's level, XP, skill points, slot points and augment capacity. Values are patched in place and the checksum in SaveState.sav is written again. Checked on public saves, not yet tried in the game", r"%USERPROFILE%\AppData\LocalLow\BulwarkStudios\Mechanicus\Saves\<slot>", "Meta.sav (with SaveState.sav beside it)", [(LocalLow, r"BulwarkStudios\Mechanicus\Saves\*")], ["Meta.sav"], 1),
    game!("Warhammer 40,000: Rogue Trader", "Game screen: scrap, Profit Factor, the Navigator's insight, difficulty settings and reputation with each faction; every other value is in the Tree. Read from the same save family as the Pathfinder games; no Rogue Trader save has been tried yet", r"%USERPROFILE%\AppData\LocalLow\Owlcat Games\Warhammer 40000 Rogue Trader\Saved Games", r"Manual_*.zks, Auto_*.zks, Quick_*.zks", [(LocalLow, r"Owlcat Games\Warhammer 40000 Rogue Trader\Saved Games")], ["*.zks"], 1),
    game!("Wasteland 2", "Game screen: each ranger's level, experience, health, attribute, skill and trait points, kills and the seven attributes; skills and items are in the Tree. Open <save name>.xml. Checked on a public save, not yet tried in the game", r"Documents\My Games\Wasteland2DC\Save Games\<save name> (Wasteland2 for the original release)", "<save name>.xml (the .bin beside it is not edited)", [(Documents, r"My Games\Wasteland2DC\Save Games\*"), (Documents, r"My Games\Wasteland2\Save Games\*")], ["*.xml"], 2),
    game!("Wasteland 3", WASTELAND3, r"Documents\My Games\Wasteland3\Save Games", r"<save name>.xml (packed, despite the name)", [(Documents, r"My Games\Wasteland3\Save Games")], ["*.xml"], 3),
    game!("Watch Dogs 2", "Game screen: money, followers and research points (while some are unspent), and the lifetime totals, changed in place in the save's object tree. Checked on public saves, not yet tried in the game", r"<Program Files (x86)>\Ubisoft\Ubisoft Game Launcher\savegames\<user>\2688 (or 3619)", "1.save, 2.save …", [(ProgramFilesX86, r"Ubisoft\Ubisoft Game Launcher\savegames\*\2688"), (ProgramFilesX86, r"Ubisoft\Ubisoft Game Launcher\savegames\*\3619")], ["*.save"], 1),
    game!("West of Loathing", "Game screen: meat (money), level, experience, the class and resistances, the stats, skills and perks by name, and the inventory with quantities. Checked on a public save, not yet tried in the game", r"%USERPROFILE%\AppData\LocalLow\Asymmetric Software\West of Loathing", "save-<name>-<number>.json", [(LocalLow, r"Asymmetric Software\West of Loathing")], ["save-*.json"], 1),
    game!("Wolcen: Lords of Mayhem", "Game screen: gold, level, experience, the four attributes and the attribute and passive skill points to spend, difficulty and league, from a character file. Checked on a public save, not yet tried in the game", r"%USERPROFILE%\Saved Games\wolcen\savegames\characters", r"<character>.json", [(SavedGames, r"wolcen\savegames\characters")], ["*.json"], 1),
    game!("World of Horror", "Game screen: unlock every achievement, mystery ending, enemy, event and pantheon entry the way the game itself records them (each with its own code), the unlock counts, the last run's character, items and mystery board, and the settings. Checked on a public completed save, not yet tried in the game", r"%LOCALAPPDATA%\wohgame (macOS: ~/Library/Application Support/com.panstasz.wohgame)", "save.ito", [(LocalAppData, "wohgame")], ["save.ito"], 1),
    game!("World War Z (and Aftermath)", "Game screen: currency, coins, account level and the class perk levels. The progression file is unpacked (its key comes from the file name) and packed again on save. Checked on a public save, not yet tried in the game", r"%LOCALAPPDATA%\Saber\WWZ\client\storage\slots", r"user_progression.dat (and user_profile.dat)", [(LocalAppData, r"Saber\WWZ\client\storage\slots")], ["user_progression.dat"], 2),
    game!("WorldBox - God Simulator", "Game screen: every unit's level, experience, health, hunger and ruler stats (diplomacy, intelligence, stewardship, warfare) and the first village's stored resources. The map.wbox is unpacked and packed again on save. Checked on a public save, not yet tried in the game", r"%USERPROFILE%\AppData\LocalLow\mkarpenko\WorldBox\saves", r"saveN\map.wbox (and map.meta)", [(LocalLow, r"mkarpenko\WorldBox\saves")], ["map.wbox"], 2),
    game!("X4: Foundations", X4, r"Documents\Egosoft\X4\<id>\save", "quicksave.xml.gz, autosave.xml.gz, save_001.xml.gz …", [(Documents, r"Egosoft\X4")], ["*.xml.gz"], 3),
    game!("Xbox app / PC Game Pass saves (any game)", "Every save the Xbox app keeps, listed under the names the game gave them; each opens like the same game's Steam save (the game's screen if it has one) and is written back in place. Let the Xbox app finish syncing first", r"%LOCALAPPDATA%\Packages\<game package>\SystemAppData\wgs\<Xbox user>_<title>", r"containers.index, plus one folder per save slot", [(GamePass("*"), "")], ["*"], 1),
    game!("XCOM 2", XCOM2, r"Documents\My Games\XCOM2\XComGame\SaveData", r"save1, save2 … (no extension)", [(Documents, r"My Games\XCOM2\XComGame\SaveData")], ["save*"], 1),
    game!("XCOM 2: War of the Chosen", XCOM2, r"Documents\My Games\XCOM2 War of the Chosen\XComGame\SaveData", r"save_AUTOSAVE- Campaign …, save_<name>", [(Documents, r"My Games\XCOM2 War of the Chosen\XComGame\SaveData")], ["save*"], 1),
    game!("XCOM: Chimera Squad", HEX_UNPACKED, r"Documents\My Games\XCOM Chimera Squad\XComGame\SaveData", r"save*", [(Documents, r"My Games\XCOM Chimera Squad\XComGame\SaveData")], ["save*"], 1),
    game!("XCOM: Enemy Unknown / Enemy Within", FULL, r"Documents\My Games\XCOM - Enemy Unknown (or XCOM - Enemy Within)\XComGame\SaveData", r"save1, save2 … (no extension)", [(Documents, r"My Games\XCOM - Enemy Unknown\XComGame\SaveData"), (Documents, r"My Games\XCOM - Enemy Within\XComGame\SaveData")], ["save*"], 1),
    game!("Xenoblade Chronicles 3 (Switch, emulator saves)", "Game screen: gold, play time, every character's level, experience and bonus experience, and the amounts of ether cylinders, gem crystals and collectibles (by id, up to the game's stack size). Checked on a public save, not yet tried in the game", SWITCH_FOLDER, "bf3game*.sav (title id 010074F013262000)", [(AppData, r"yuzu\nand\user\save\0000000000000000"), (AppData, r"Ryujinx\bis\user\save")], ["bf3game*.sav"], 4),
    game!("Yakuza 0", FULL, r"Steam\userdata\<account>\638970\remote", r"SaveData00NN.sav", [(SteamUser("638970"), r"remote"), (GamePass("SEGAofAmericaInc.Yakuza0PC_s751p9cej88mt"), "")], ["SaveData*.sav"], 1),
    game!("Yakuza 3 Remastered", "Game screen: Kiryu's money, the hostess club's money and his experience. The save has no checksum. Checked on a public save, not yet tried in the game", r"Steam\userdata\<account>\1088710\remote", r"savegameN.sav", [(SteamUser("1088710"), r"remote")], ["savegame*.sav"], 1),
    game!("Yakuza 4 Remastered", "Game screen: each of the four playable characters' money, experience and skill orbs, and the eco points. The save has no checksum. Checked on a public save, not yet tried in the game", r"Steam\userdata\<account>\1105500\remote", r"save files", [(SteamUser("1105500"), r"remote")], ["*.sav"], 1),
    game!("Yakuza 5 Remastered", "Game screen: money, experience and skill orbs for each of the five playable characters. The save has no checksum. Checked on a public save, not yet tried in the game", r"Steam\userdata\<account>\1105510\remote", r"save files", [(SteamUser("1105510"), r"remote")], ["*.sav"], 1),
    game!("Yakuza 6: The Song of Life", RGG_Y6, r"Steam\userdata\<account>\1388590\remote", r"save files", [(SteamUser("1388590"), r"remote")], ["*.sav"], 2),
    game!("Yakuza Kiwami", FULL, r"Steam\userdata\<account>\834530\remote", r"SaveData00NN.sav, autosave00NN.sav", [(SteamUser("834530"), r"remote")], ["SaveData*.sav", "autosave*.sav"], 1),
    game!("Yakuza Kiwami 2", FULL, r"%APPDATA%\SEGA\YakuzaKiwami2\<Steam account>\saveNN", r"data.sav (one folder per slot)", [(AppData, r"SEGA\YakuzaKiwami2\*"), (AppData, r"SEGA\YakuzaKiwami2_GOG")], ["data.sav"], 2),
    game!("Yakuza Kiwami 3 & Dark Ties", "Game screen: Kiryu's money and each character's HP and heat; the MessagePack save is decrypted with the game's key and its checksum kept right, every other value in the Tree. Checked on public chapter saves, not yet tried in the game", r"%APPDATA%\Sega\YakuzaKiwami3\Steam\<Steam id>", r"data.sav (MessagePack inside)", [(AppData, r"Sega\YakuzaKiwami3\Steam\*")], ["*.sav"], 2),
    game!("Yakuza: Like a Dragon", FULL, r"%APPDATA%\SEGA\YakuzaLikeADragon\<Steam account>\saveNNN", r"data.sav (one folder per slot)", [(AppData, r"SEGA\YakuzaLikeADragon\*"), (AppData, r"SEGA\YakuzaLikeADragon_GOG\*"), (GamePass("SEGAofAmericaInc.Yazawa_s751p9cej88mt"), "")], ["data.sav"], 2),
    game!("Youtubers Life", "Game screen: money, house, work hours, your channel (subscribers, views, status) and every youtuber's needs, skills, levels and experience. The SQLite save is written back as a database. Checked on a public save, not yet tried in the game", r"Documents\U-play online\Youtubers Life\Saves", "<name>_<date>.yls", [(Documents, r"U-play online\Youtubers Life\Saves")], ["*.yls"], 1),
];

/// A save found for a game.
#[derive(Clone, Debug)]
pub struct Found {
    pub path: PathBuf,
    pub modified: Option<SystemTime>,
    pub size: u64,
    /// The name to show when the file's own name says nothing (Game Pass blobs).
    pub label: Option<String>,
}

/// `*` (any run), `?` (one character) and `[0-9]` (a digit), ignoring case.
pub fn wildcard(pat: &str, name: &str) -> bool {
    fn go(p: &[char], n: &[char]) -> bool {
        match p.first() {
            None => n.is_empty(),
            Some('*') => (0..=n.len()).any(|i| go(&p[1..], &n[i..])),
            Some('?') => !n.is_empty() && go(&p[1..], &n[1..]),
            Some('[') if p.len() >= 5 && p[2] == '-' && p[4] == ']' => {
                n.first().is_some_and(|c| (p[1]..=p[3]).contains(c)) && go(&p[5..], &n[1..])
            }
            Some(c) => n.first().is_some_and(|x| x == c) && go(&p[1..], &n[1..]),
        }
    }
    let p: Vec<char> = pat.to_lowercase().chars().collect();
    let n: Vec<char> = name.to_lowercase().chars().collect();
    go(&p, &n)
}

fn env(k: &str) -> Option<PathBuf> {
    std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from)
}

/// A value from the registry (Windows), via `reg query`.
#[cfg(windows)]
fn reg(key: &str, value: &str) -> Option<String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let out = std::process::Command::new("reg").args(["query", key, "/v", value]).creation_flags(CREATE_NO_WINDOW).output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    text.lines().find_map(|l| l.split_once("REG_SZ").or_else(|| l.split_once("REG_EXPAND_SZ")).map(|(_, v)| v.trim().to_string()))
}

/// The Documents folder (it may have been moved to another drive).
pub fn documents() -> Option<PathBuf> {
    #[cfg(windows)]
    if let Some(p) = reg(r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\Shell Folders", "Personal") {
        return Some(PathBuf::from(p));
    }
    env("USERPROFILE").or_else(|| env("HOME")).map(|h| h.join("Documents"))
}

fn saved_games() -> Option<PathBuf> {
    #[cfg(windows)]
    if let Some(p) = reg(r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\Shell Folders", "{4C5C32FF-BB9D-43b0-B5B4-2D72E54EAAA4}") {
        return Some(PathBuf::from(p));
    }
    env("USERPROFILE").map(|h| h.join("Saved Games"))
}

/// Every Steam library folder (`…\steamapps\common` is under each).
pub fn steam_libraries() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = vec![];
    for userdata in crate::platform::steam_userdata_dirs() {
        let Some(steam) = userdata.parent() else { continue };
        out.push(steam.to_path_buf());
        let vdf = std::fs::read_to_string(steam.join("steamapps").join("libraryfolders.vdf")).unwrap_or_default();
        for line in vdf.lines() {
            let mut parts = line.split('"').filter(|s| !s.trim().is_empty());
            if parts.next() == Some("path") {
                if let Some(p) = parts.next() {
                    out.push(PathBuf::from(p.replace(r"\\", r"\")));
                }
            }
        }
    }
    let mut seen = std::collections::HashSet::new();
    out.retain(|p| seen.insert(p.to_string_lossy().to_lowercase().replace('/', "\\")));
    out
}

/// Windows user folders inside Wine prefixes (`drive_c/users/<name>`): Steam Play / Proton
/// prefixes in every Steam library (one per game, under `steamapps/compatdata/<app id>/pfx`),
/// plus the default Wine prefix and the usual Heroic, Lutris and Bottles prefix folders. On
/// Linux and the Steam Deck this is where Windows games run through Proton keep their saves.
pub fn wine_user_dirs() -> Vec<PathBuf> {
    if cfg!(windows) {
        return vec![];
    }
    let mut prefixes: Vec<PathBuf> = vec![];
    for lib in steam_libraries() {
        for e in std::fs::read_dir(lib.join("steamapps").join("compatdata")).into_iter().flatten().flatten() {
            prefixes.push(e.path().join("pfx"));
        }
    }
    if let Some(home) = env("HOME") {
        prefixes.push(home.join(".wine"));
        for dir in ["Games/Heroic/Prefixes", "Games/Heroic/Prefixes/default", ".local/share/bottles/bottles", ".var/app/com.usebottles.bottles/data/bottles/bottles", "Games"] {
            for e in std::fs::read_dir(home.join(dir)).into_iter().flatten().flatten() {
                prefixes.push(e.path());
                prefixes.push(e.path().join("pfx"));
            }
        }
    }
    users_in_prefixes(&prefixes)
}

fn users_in_prefixes(prefixes: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = vec![];
    for p in prefixes {
        for e in std::fs::read_dir(p.join("drive_c").join("users")).into_iter().flatten().flatten() {
            let name = e.file_name().to_string_lossy().to_lowercase();
            if name != "public" && e.path().is_dir() {
                out.push(e.path());
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// The first of `names` that exists under `dir` (Wine has used both spellings).
fn first_existing(dir: &Path, names: &[&str]) -> Option<PathBuf> {
    names.iter().map(|n| dir.join(n)).find(|p| p.is_dir())
}

/// The folders `base` stands for inside Wine and Proton prefixes.
fn wine_bases(base: Base) -> Vec<PathBuf> {
    let users = wine_user_dirs();
    let pick = |names: &[&str]| -> Vec<PathBuf> { users.iter().filter_map(|u| first_existing(u, names)).collect() };
    match base {
        AppData => pick(&["AppData/Roaming", "Application Data"]),
        LocalAppData => pick(&["AppData/Local", "Local Settings/Application Data"]),
        LocalLow => pick(&["AppData/LocalLow"]),
        Documents => pick(&["Documents", "My Documents"]),
        SavedGames => pick(&["Saved Games"]),
        ProgramFilesX86 => users
            .iter()
            .filter_map(|u| u.parent()?.parent().map(|c| c.join("Program Files (x86)")))
            .filter(|p| p.is_dir())
            .collect(),
        ProgramData => users
            .iter()
            .filter_map(|u| u.parent()?.parent().map(|c| c.join("ProgramData")))
            .filter(|p| p.is_dir())
            .collect(),
        SteamUser(_) | SteamGame(_) | GamePass(_) | UnityPrefs => vec![],
    }
}

/// The folders `base` stands for on this computer.
fn bases(base: Base) -> Vec<PathBuf> {
    let mut out = native_bases(base);
    for p in wine_bases(base) {
        if !out.contains(&p) {
            out.push(p);
        }
    }
    out
}

fn native_bases(base: Base) -> Vec<PathBuf> {
    let home = env("USERPROFILE").or_else(|| env("HOME"));
    match base {
        AppData => env("APPDATA").into_iter().collect(),
        LocalAppData => env("LOCALAPPDATA").into_iter().collect(),
        LocalLow => home.map(|h| h.join("AppData").join("LocalLow")).into_iter().collect(),
        Documents => documents().into_iter().collect(),
        SavedGames => saved_games().into_iter().collect(),
        SteamUser(app) => crate::platform::steam_userdata_dirs()
            .into_iter()
            .flat_map(|u| std::fs::read_dir(u).into_iter().flatten().flatten().map(|e| e.path().join(app)))
            .filter(|p| p.is_dir())
            .collect(),
        ProgramFilesX86 => env("ProgramFiles(x86)").into_iter().collect(),
        ProgramData => env("ProgramData").into_iter().collect(),
        UnityPrefs => {
            let mut v: Vec<PathBuf> = vec![];
            #[cfg(not(target_arch = "wasm32"))]
            v.extend(crate::playerprefs::export_all());
            if let Some(h) = env("HOME") {
                v.push(h.join(".config").join("unity3d"));
                v.push(h.join("Library").join("Preferences"));
            }
            v.into_iter().filter(|p| p.is_dir()).collect()
        }
        GamePass(pkg) => env("LOCALAPPDATA").map(|l| crate::xgp::user_dirs(&l.join("Packages"), pkg)).unwrap_or_default(),
        SteamGame(dir) => steam_libraries().into_iter().map(|l| l.join("steamapps").join("common").join(dir)).filter(|p| p.is_dir()).collect(),
    }
}

/// Folders matching `rel` (with `*` folder names) under `root`.
fn expand(root: &Path, rel: &str) -> Vec<PathBuf> {
    let mut cur = vec![root.to_path_buf()];
    for part in rel.split(['\\', '/']).filter(|p| !p.is_empty()) {
        let mut next = vec![];
        for dir in cur {
            if part.contains('*') {
                for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
                    if e.path().is_dir() && wildcard(part, &e.file_name().to_string_lossy()) {
                        next.push(e.path());
                    }
                }
            } else {
                let p = dir.join(part);
                if p.is_dir() {
                    next.push(p);
                } else if !cfg!(windows) {
                    // Windows folder names ignore case; inside a Wine prefix they don't.
                    let want = part.to_lowercase();
                    if let Some(e) = std::fs::read_dir(&dir).into_iter().flatten().flatten().find(|e| e.file_name().to_string_lossy().to_lowercase() == want && e.path().is_dir()) {
                        next.push(e.path());
                    }
                }
            }
        }
        cur = next;
    }
    cur
}

fn walk(dir: &Path, depth: usize, g: &GameInfo, out: &mut Vec<Found>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let Ok(meta) = e.metadata() else { continue };
        if meta.is_dir() {
            if depth > 0 {
                walk(&p, depth - 1, g, out);
            }
        } else {
            let name = e.file_name().to_string_lossy().to_string();
            let lower = name.to_lowercase();
            // Backups and the side files some games keep next to a save (Hades' Profile1.v.sav,
            // Profile1_Temp.sav).
            let backup = [".bak", ".old", ".tmp", ".saving", "_old", ".vdf"].iter().any(|x| lower.ends_with(x))
                || lower.contains(".v.")
                || lower.contains("_temp");
            if !backup && g.patterns.iter().any(|pat| wildcard(pat, &name)) {
                // A save's character, level and place, when its header gives them.
                let label = crate::preview::peek(&p).map(|peek| format!("{name} \u{2014} {peek}"));
                out.push(Found { path: p, modified: meta.modified().ok(), size: meta.len(), label });
            }
        }
    }
}

/// Every file in a Game Pass user folder, named the way the game named it.
fn game_pass_saves(dir: &Path, out: &mut Vec<Found>) {
    let Ok(idx) = crate::xgp::read_index(dir) else { return };
    for c in &idx.containers {
        for f in &c.files {
            let Ok(meta) = std::fs::metadata(&f.path) else { continue };
            let label = if c.files.len() == 1 { c.name.clone() } else { format!("{}/{}", c.name, f.name) };
            out.push(Found { path: f.path.clone(), modified: meta.modified().ok(), size: meta.len(), label: Some(format!("{label} (Xbox app)")) });
        }
    }
}

/// Entries that cover many games (an engine or a launcher): their saves are listed by game.
const MANY_GAMES: [&str; 9] = ["RPG Maker XP games", "Koei Tecmo games (Nioh, Wo Long, Stranger of Paradise)", "Flash and AIR games (.sol saves)", "OpenFL and HaxeFlixel games (Haxe saves)", "Xbox app / PC Game Pass saves (any game)", "Ren'Py visual novels", "RPG Maker MV / MZ games", "Assassin's Creed (Ubisoft Connect games)", "Far Cry (Ubisoft Connect games)"];

pub fn many_games(g: &GameInfo) -> bool {
    MANY_GAMES.contains(&g.name)
}

/// Which game a save belongs to, for entries that cover many games: the folder the game
/// has under the search place (its last `*` part, or the folder right under it).
pub fn group_of(g: &GameInfo, path: &Path) -> String {
    if g.places.iter().any(|(b, _)| matches!(b, GamePass(_))) {
        if let Some(p) = crate::xgp::package_of(path) {
            return p;
        }
    }
    for (base, rel) in g.places {
        let parts: Vec<&str> = rel.split(['\\', '/']).filter(|p| !p.is_empty()).collect();
        let at = parts.iter().rposition(|p| p.contains('*')).unwrap_or(parts.len());
        for root in bases(*base) {
            let Ok(sub) = path.strip_prefix(&root) else { continue };
            // Flash Player: <random id>\<site>\…: the site names the game.
            let at = if g.name.starts_with("Flash and AIR") { at + 1 } else { at };
            let Some(name) = sub.components().nth(at).map(|c| c.as_os_str().to_string_lossy().to_string()) else { continue };
            if sub.components().count() <= at + 1 {
                continue; // the save itself, not a folder
            }
            // Ren'Py folders end in "-<digits>"; Ubisoft names games by number.
            let name = match name.rsplit_once('-') {
                Some((n, d)) if !n.is_empty() && !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()) => n.to_string(),
                _ => name,
            };
            if name.bytes().all(|b| b.is_ascii_digit()) {
                return format!("Game #{name}");
            }
            return name;
        }
    }
    path.parent().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
}

/// This game's saves on this computer, newest first.
pub fn find(g: &GameInfo) -> Vec<Found> {
    let mut out = vec![];
    for (base, rel) in g.places {
        let pass = matches!(base, GamePass(_));
        for root in bases(*base) {
            for dir in expand(&root, rel) {
                if pass {
                    game_pass_saves(&dir, &mut out);
                } else {
                    walk(&dir, g.depth.saturating_sub(1), g, &mut out);
                }
            }
        }
    }
    out.sort_by(|a, b| b.modified.cmp(&a.modified));
    out.dedup_by(|a, b| a.path == b.path);
    out
}

#[cfg(test)]
mod proton_tests {
    use super::*;

    #[test]
    fn finds_saves_inside_a_proton_prefix() {
        let tmp = std::env::temp_dir().join(format!("use_proton_{}", std::process::id()));
        let pfx = tmp.join("steamapps/compatdata/1245620/pfx");
        let roaming = pfx.join("drive_c/users/steamuser/AppData/Roaming/EldenRing/7656");
        std::fs::create_dir_all(&roaming).unwrap();
        std::fs::create_dir_all(pfx.join("drive_c/users/Public")).unwrap();
        let users = users_in_prefixes(&[pfx.clone()]);
        assert_eq!(users, vec![pfx.join("drive_c/users/steamuser")]);
        let appdata = first_existing(&users[0], &["AppData/Roaming", "Application Data"]).unwrap();
        // Folder names are matched without regard to case inside a prefix.
        let found = expand(&appdata, r"eldenring\*");
        if !cfg!(windows) {
            assert_eq!(found, vec![roaming.clone()]);
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcards() {
        assert!(wildcard("*.ess", "Save1_Player.ESS"));
        assert!(wildcard("user?.dat", "user3.dat"));
        assert!(!wildcard("user?.dat", "user10.dat"));
        assert!(wildcard("*_[0-9]*", "MyFarm_123456789"));
        assert!(!wildcard("*_[0-9]*", "SaveGameInfo"));
        assert!(wildcard("Profile*.sav", "Profile1.sav"));
    }

    #[test]
    fn catalog_is_sorted_and_complete() {
        let names: Vec<&str> = GAMES.iter().map(|g| g.name).collect();
        let mut sorted = names.clone();
        sorted.sort_by_key(|n| n.to_lowercase());
        assert_eq!(names, sorted);
        assert!(GAMES.iter().all(|g| !g.places.is_empty() && !g.patterns.is_empty() && g.depth >= 1));
    }

    /// Research aid: lists what's found for every game in the catalog.
    #[test]
    #[ignore]
    fn find_on_this_computer() {
        for g in GAMES {
            let f = find(g);
            println!("{}: {} saves {:?}", g.name, f.len(), f.iter().take(2).map(|x| x.path.display().to_string()).collect::<Vec<_>>());
        }
    }
}

/// How far the editor goes with a game, for the badge in Select game.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Level {
    /// A screen of its own (character, inventory…).
    Screen,
    /// Every value listed by name (Values and Tree).
    Values,
    /// Hex view and value search only.
    Hex,
}

impl Level {
    pub const ALL: [Level; 3] = [Level::Screen, Level::Values, Level::Hex];

    pub fn label(self) -> &'static str {
        match self {
            Level::Screen => "Screen",
            Level::Values => "Values",
            Level::Hex => "Hex",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            Level::Screen => "Has a screen of its own for this game",
            Level::Values => "Its values are listed by name in Values and Tree",
            Level::Hex => "Opens in the hex view: find a number you know with Value search, then change it",
        }
    }
}

pub fn level(g: &GameInfo) -> Level {
    let s = g.support.to_lowercase();
    if s.starts_with("game screen") {
        Level::Screen
    } else if s.starts_with("hex") || (s.contains("value search") && !s.contains("tree") && !s.contains("values")) {
        Level::Hex
    } else {
        Level::Values
    }
}

#[cfg(test)]
mod level_tests {
    #[test]
    #[ignore]
    fn dump_levels() {
        for g in super::GAMES {
            if super::level(g) != super::Level::Screen {
                println!("{:?}\t{}\t{}", super::level(g), g.name, g.support);
            }
        }
    }

    #[test]
    fn every_level_used() {
        for l in super::Level::ALL {
            assert!(super::GAMES.iter().any(|g| super::level(g) == l), "{l:?}");
        }
    }
}
