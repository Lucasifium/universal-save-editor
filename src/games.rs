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
const VALUES: &str = "Values and Tree";
const HEX: &str = "Hex view and Value search (find a number you know, then change it)";
const METRO: &str = "Game screen for the level saves (.player); the auto save is unpacked for Value search";
const RGG_JUDGMENT: &str = "Decrypted, with its checksum kept right (read and written back byte for byte on a real save; not yet tried in the game); values in the Tree";
const RGG_Y6: &str = "Decrypted, with its checksum kept right (read and written back byte for byte on real saves; not yet tried in the game); values in the Tree";
const RGG_UNTESTED: &str = "Decrypted, with its checksum kept right (no save of this game has been tried yet); values in the Tree";
const HEX_UNPACKED: &str = "Unpacked for Value search (find a number you know, then change it); packed again on save";
const XCOM2: &str = "Game screen: HQ resources and soldiers (rank, kills, missions, stats); unpacked and packed again on save. Not yet tried in the game";
const CIV6: &str = "Game screen: settings, mods and each civilisation (human or AI, whose turn it is); gold and the map are in the packed part and aren't mapped";
const BORDERLANDS: &str = "Game screen: level, experience, skill points, money, Eridium (Moonstone in The Pre-Sequel; Seraph crystals and Torgue tokens in Borderlands 2), ammo and ammo SDUs; gear is listed (level, slot), not edited. Packing and checksums are redone on save. Edited saves not yet tried in the game";
const BORDERLANDS3: &str = "Game screen: level and experience, skill points, money, Eridium, ammo and SDU levels; gear is listed (slot, favourite/junk), not edited. Encrypted again on save. Edited saves not yet tried in the game";
const FALLOUT12: &str = "Game screen: S.P.E.C.I.A.L., hit points, level, experience, skill points, skills, kills, bottle caps and inventory counts (traits and perks shown). Edits haven't been tried in the game yet";
const DEUS_EX: &str = "Game screen for JC Denton: credits, skill points, bioelectric energy, health per body part, ammo, medkits and other item counts, skill and augmentation levels (values the game left out can't be set). Edits haven't been tried in the game yet";
const KOTOR: &str = "Game screen: the player and party members (attributes, skills, HP and Force points, XP) from SAVEGAME.sav; credits, party XP and (KOTOR II) influence, components and chemicals from PARTYTABLE.res. Not yet tried in the game";
const HL1: &str = "Game screen: health, armour, flashlight, ammo by type and each weapon's magazine, changed in place. Values the game left out because they're zero (no armour, an empty magazine) can't be set. Not yet tried in the game; the 25th-anniversary build hasn't been checked";
const HL2: &str = "Game screen: health, armour, suit power, ammo by type and each weapon's magazines, changed in place. Values the game left out because they're zero (no armour, an empty magazine) can't be set. Not yet tried in the game; the 20th-anniversary build hasn't been checked";
const STARBOUND: &str = "Game screen: pixels and other currencies, item counts in every bag, health, energy and food, ship level, crew size and fuel tank, deaths and play time; each value is rewritten on its own and the rest of the file kept. Early-access characters (before 1.0) open with what their older layout has. Checked on a real character save, not yet tried in the game";
const EXPEDITION33: &str = "Game screen: gold; each party member's level, experience, action points, health, attribute points and Lumina from Colours of Lumina; each weapon's level; item counts (cosmetics left out). Checked on a real save, not yet tried in the game";
const FALLOUT3NV: &str = "Game screen: bottle caps, experience, karma, S.P.E.C.I.A.L. and the points added to each skill; the level, place and play time are shown from the save. Checked on real saves, not yet tried in the game";
const WASTELAND3: &str = "Game screen: money, each ranger's level, XP, hit points, attributes, skills and unspent points, item quantities; unpacked (LZF) and packed again on save. Not yet tried in the game";
const VALHEIM: &str = "Game screen for characters saved with profile versions 33 to 37: health, stamina, skills, item stacks (gold is Coins), quality and durability, with the SHA-512 written again, and a Max crafting materials button that fills the wood, stone, ores, metals, hides and other plain materials you carry to their stack size, leaving boss drops, summoning items and key fragments alone; newer characters open read-only. Not yet tried in the game";
const TERRARIA_PLR: &str = "Game screen: life and mana, permanent upgrades, inventory, equipment and storage as item grids, with a Max crafting materials button (characters from 1.4.4 on) that fills the ores, bars, gems, woods, monster drops and other materials you already have to 9999, leaving critters, fish and boss summons alone. The button hasn't been tried in the game yet";
const STARDEW: &str = "Game screen: gold, Qi Gems, Golden Walnuts, health, energy, skills and farm details, with a Max crafting materials button that fills the wood, stone, fiber, clay, ores, bars and monster loot in the backpack to 999, leaving quest items, minerals and artifacts alone (checked on public 1.5 saves; not yet tried in the game)";
const MINECRAFT: &str = "Game screen: health, food, XP, game mode, world settings, inventory and ender chest, with a Max crafting materials button that fills the ingots, raw ores, gems, string, leather and other materials already carried to 64, leaving ender pearls, eyes of ender and other progression items alone. The button hasn't been tried in the game yet";
const PDX_BINARY: &str = "Text saves fully; ironman (binary) saves are shown but can't be edited";
const IE_BG: &str = "Game screen: party gold and reputation; each party member's experience, hit points, levels, abilities, alignment and item stacks or charges (BALDUR.gam)";
const IE_IWD: &str = "Game screen: party gold and reputation; each party member's experience, hit points, levels, abilities, alignment and item stacks or charges (BALDUR.gam, the same layout as Baldur's Gate: EE). No Icewind Dale: EE save has been tried yet";
const IE_PST: &str = "Game screen: the party's coppers; the Nameless One's and each companion's experience, level, hit points, abilities, alignment and item stacks (BALDUR.gam, same layout as the other Enhanced Editions). Checked on real saves, not yet tried in the game";
const DAO: &str = "Game screen: party money, the Warden's and followers' level, experience, attributes and unspent points, approval, backpack stack sizes. Tried on saves from one playthrough; Awakening saves not yet tried";
const GOLDEN_SUN: &str = "Game screen: every file in the flash with its coins, each party member's level, experience, HP/PP, base stats and items, and the Djinn per element; checksums kept right. Not yet tried in the game";
const GBA_FOLDER: &str = r"The emulator's save folder: mGBA and VisualBoyAdvance-M keep <ROM name>.sav next to the ROM (or in the save folder set in their options)";

macro_rules! game {
    ($name:expr, $support:expr, $folder:expr, $files:expr, [$($place:expr),*], [$($pat:expr),*], $depth:expr) => {
        GameInfo { name: $name, support: $support, folder: $folder, files: $files, places: &[$($place),*], patterns: &[$($pat),*], depth: $depth }
    };
}

use Base::*;

/// Alphabetical.
pub const GAMES: &[GameInfo] = &[
    game!("Assassin's Creed (Ubisoft Connect games)", HEX, r"Ubisoft Connect: <Program Files (x86)>\Ubisoft\Ubisoft Game Launcher\savegames\<user>\<game number>", r"<number>.save (zlib-packed ones end in .save.z)", [(ProgramFilesX86, r"Ubisoft\Ubisoft Game Launcher\savegames\*\*")], ["*.save", "*.save.z"], 1),
    game!("Assassin's Creed Brotherhood", HEX, r"%USERPROFILE%\Saved Games\Assassin's Creed Brotherhood\SAVES", r"ACBROTHERHOODSAVEGAME0.SAV", [(SavedGames, r"Assassin's Creed Brotherhood\SAVES")], ["*.SAV"], 1),
    game!("Attack on Titan 2", HEX, r"Documents\KoeiTecmo\Attack on Titan2_AOT2\SAVEDATA", r"Attack on Titan2_AOT2_PK_WIN0000.dat", [(Documents, r"KoeiTecmo\Attack on Titan2_AOT2\SAVEDATA"), (Documents, r"KoeiTecmo\Attack on Titan 2")], ["*.dat"], 2),
    game!("Balatro", FULL, r"%APPDATA%\Balatro\<profile>", "save.jkr (the run), profile.jkr", [(AppData, "Balatro")], ["save.jkr", "profile.jkr"], 2),
    game!("Baldur's Gate 3", "Game screen: a party summary (origin, class, level, experience), shown read-only; ability scores, hit points and gold aren't mapped yet", r"%LOCALAPPDATA%\Larian Studios\Baldur's Gate 3\PlayerProfiles\Public\Savegames\Story\<save>", r"<save>.lsv", [(LocalAppData, r"Larian Studios\Baldur's Gate 3\PlayerProfiles\Public\Savegames\Story")], ["*.lsv"], 2),
    game!("Baldur's Gate II: Enhanced Edition", IE_BG, r"Documents\Baldur's Gate II - Enhanced Edition\save\<nnnnnnnnn-Name>", r"BALDUR.gam (BALDUR.SAV holds the areas)", [(Documents, r"Baldur's Gate II - Enhanced Edition\save"), (Documents, r"Baldur's Gate II - Enhanced Edition\mpsave")], ["*.gam"], 2),
    game!("Baldur's Gate: Enhanced Edition / Siege of Dragonspear", IE_BG, r"Documents\Baldur's Gate - Enhanced Edition\save\<nnnnnnnnn-Name>", r"BALDUR.gam (BALDUR.SAV holds the areas)", [(Documents, r"Baldur's Gate - Enhanced Edition\save"), (Documents, r"Baldur's Gate - Enhanced Edition\mpsave")], ["*.gam"], 2),
    game!("BioShock Infinite", HEX_UNPACKED, r"Steam\userdata\<account>\8870\remote\savedata", r"<area>-0_<date>.bkm / .sav", [(SteamUser("8870"), r"remote\savedata")], ["*.bkm", "*.sav"], 1),
    game!("BioShock Remastered / BioShock 2 Remastered", HEX_UNPACKED, r"Documents\BioshockHD\Bioshock (or Bioshock2)\SaveGames", r"<date>.bsb", [(Documents, r"BioshockHD"), (AppData, r"BioshockHD")], ["*.bsb"], 3),
    game!("Borderlands 2", BORDERLANDS, r"Documents\My Games\Borderlands 2\WillowGame\SaveData\<Steam id>", r"Save0001.sav … (one per character; .sav.bak is the backup)", [(Documents, r"My Games\Borderlands 2\WillowGame\SaveData")], ["Save*.sav"], 2),
    game!("Borderlands 3", BORDERLANDS3, r"Documents\My Games\Borderlands 3\Saved\SaveGames\<Steam or Epic id>", r"<n>.sav (1.sav, 2.sav … one per character; profile.sav is the profile)", [(Documents, r"My Games\Borderlands 3\Saved\SaveGames")], ["?.sav", "??.sav", "???.sav"], 2),
    game!("Borderlands: The Pre-Sequel", BORDERLANDS, r"Documents\My Games\Borderlands The Pre-Sequel\WillowGame\SaveData\<Steam id>", r"Save0001.sav … (one per character; .sav.bak is the backup)", [(Documents, r"My Games\Borderlands The Pre-Sequel\WillowGame\SaveData")], ["Save*.sav"], 2),
    game!("Cairn", FULL, r"%USERPROFILE%\Saved Games\TheGameBakers\Cairn_RETAIL\SAVEGAMES\RETAIL\STORY (SUMMIT for summit runs)", r"SLOT_A.CPK … SLOT_D.CPK", [(SavedGames, r"TheGameBakers\Cairn_RETAIL\SAVEGAMES\RETAIL"), (SavedGames, r"TheGameBakers\Cairn\SAVEGAMES")], ["*.CPK"], 2),
    game!("Civilization V", HEX, r"Documents\My Games\Sid Meier's Civilization 5\Saves\single (or ModdedSaves\single)", "<Leader>_<year>.Civ5Save", [(Documents, r"My Games\Sid Meier's Civilization 5\Saves"), (Documents, r"My Games\Sid Meier's Civilization 5\ModdedSaves")], ["*.Civ5Save"], 3),
    game!("Civilization VI", CIV6, r"Documents\My Games\Sid Meier's Civilization VI\Saves\Single", r"*.Civ6Save (auto\AutoSave_####.Civ6Save)", [(Documents, r"My Games\Sid Meier's Civilization VI\Saves")], ["*.Civ6Save"], 3),
    game!("Clair Obscur: Expedition 33", EXPEDITION33, r"%LOCALAPPDATA%\Sandfall\Saved\SaveGames\<Steam id>", r"EXPEDITION_*.sav", [(LocalAppData, r"Sandfall\Saved\SaveGames")], ["*.sav"], 2),
    game!("Control", HEX_UNPACKED, r"Steam\userdata\<account>\870780\remote", r"savegame-slot-00_global …", [(SteamUser("870780"), r"remote"), (LocalAppData, r"Remedy\Control")], ["savegame-*"], 2),
    game!("Crisis Core: Final Fantasy VII Reunion", VALUES, r"Documents\My Games\CRISIS CORE FINAL FANTASY VII REUNION\Steam\<Steam id>", r"SAVEDATA_PROGRESS.sav", [(Documents, r"My Games\CRISIS CORE FINAL FANTASY VII REUNION\*\*")], ["*.sav"], 1),
    game!("Crusader Kings II", FULL, r"Documents\Paradox Interactive\Crusader Kings II\save games", "*.ck2", [(Documents, r"Paradox Interactive\Crusader Kings II\save games")], ["*.ck2"], 1),
    game!("Crusader Kings III", PDX_BINARY, r"Documents\Paradox Interactive\Crusader Kings III\save games", "*.ck3", [(Documents, r"Paradox Interactive\Crusader Kings III\save games")], ["*.ck3"], 1),
    game!("Cyberpunk 2077", FULL, r"%USERPROFILE%\Saved Games\CD Projekt Red\Cyberpunk 2077\<save name>", r"sav.dat", [(SavedGames, r"CD Projekt Red\Cyberpunk 2077")], ["sav.dat"], 2),
    game!("Dark Souls II: Scholar of the First Sin", FULL, r"%APPDATA%\DarkSoulsII\<Steam id>", "DS2SOFS0000.sl2", [(AppData, "DarkSoulsII")], ["DS2SOFS0000.sl2"], 2),
    game!("Dark Souls III", FULL, r"%APPDATA%\DarkSoulsIII\<Steam id>", "DS30000.sl2", [(AppData, "DarkSoulsIII")], ["DS30000.sl2"], 2),
    game!("Dark Souls Remastered", FULL, r"Documents\NBGI\DARK SOULS REMASTERED\<id>", "DRAKS0005.sl2", [(Documents, r"NBGI\DARK SOULS REMASTERED")], ["DRAKS0005.sl2"], 2),
    game!("Darkest Dungeon", "Game screen: the estate (gold, busts, portraits, deeds, crests, blueprints, shards) and the roster (each hero's resolve XP, stress, HP, weapon and armour rank), changed in place. Open persist.estate.json or persist.roster.json; the profile's other files show in Values", r"Steam\userdata\<account>\262060\remote\profile_<n> (GOG: Documents\Darkest\profile_<n>)", r"persist.estate.json, persist.roster.json (binary, despite the name)", [(SteamUser("262060"), r"remote\profile_*"), (Documents, r"Darkest\profile_*")], ["persist.estate.json", "persist.roster.json"], 1),
    game!("Darkest Dungeon II", "Game screen: Candles of Hope (profile), Relics, torch and inventory (run.json), heroes' HP and stress (actors.json). Checked on saves made before the Kingdoms update; Kingdoms' own files aren't covered", r"%USERPROFILE%\AppData\LocalLow\RedHook\Darkest Dungeon II\SaveFiles\<id>\profiles (runs in profile_<n>_runs\<run>\Save_<nnnn>_<stage>)", r"profile_<n>.json; the newest Save_<nnnn>_* folder's run.json and actors.json", [(LocalLow, r"RedHook\Darkest Dungeon II\SaveFiles\*\profiles")], ["profile_*.json", "run.json", "actors.json"], 5),
    game!("Dead Rising 2 / Off the Record", HEX, r"Documents\My Games\Dead Rising 2 (Off The Record)\saves", r"DCSAV00.DSF …", [(Documents, r"My Games\Dead Rising 2\saves"), (Documents, r"My Games\Dead Rising 2 Off The Record\saves")], ["*.DSF", "*.DSF.tmp"], 1),
    game!("Dead Rising 3", HEX, r"Documents\My Games\dead rising 3\saves\x64_build", r"D3S.*", [(Documents, r"My Games\dead rising 3\saves")], ["D3S.*", "d3s.*"], 2),
    game!("Dead Rising Deluxe Remaster", VALUES, r"Steam\userdata\<account>\2527390\remote\win64_save", "data*.bin", [(SteamUser("2527390"), r"remote\win64_save")], ["*.bin"], 1),
    game!("Death Stranding (Director's Cut)", HEX, r"%LOCALAPPDATA%\KojimaProductions\DeathStrandingDC\<account>\<slot>", r"checkpoint.dat", [(LocalAppData, r"KojimaProductions\DeathStrandingDC"), (LocalAppData, r"KojimaProductions\DeathStranding")], ["checkpoint.dat"], 3),
    game!("Death Stranding 2: On the Beach", FULL, r"Documents\DEATH STRANDING 2 - ON THE BEACH\<Steam id>", r"manualsave<n>.dat, checkpointsave<n>.dat", [(Documents, r"DEATH STRANDING 2 - ON THE BEACH\*")], ["*save*.dat"], 1),
    game!("Deathloop", HEX, r"%USERPROFILE%\Saved Games\Arkane Studios\Deathloop\base\savegame\<Steam id>\SLOT01", r"save.dloop", [(SavedGames, r"Arkane Studios\Deathloop\base\savegame")], ["*.dloop"], 3),
    game!("Deltarune", "Game screen: Chapter 1 fully (party, inventory as item numbers, Light World); Chapters 2\u{2013}4: dark dollars and the party's stats and equipment. Every line of the file can be edited. Not yet tried in the game", r"%LOCALAPPDATA%\DELTARUNE", r"filech1_0 … filech4_2 (chapter and slot; dr.ini is the load menu)", [(LocalAppData, "DELTARUNE")], ["filech*"], 1),
    game!("Deus Ex (2000)", DEUS_EX, r"<install folder>\Save\Save#### (Steam: steamapps\common\Deus Ex\Save; some installs: Documents\Deus Ex\Save)", r"<map>.dxs: open the map named in SaveInfo.dxs, which holds JC (open SaveInfo.dxs to see which)", [(SteamGame("Deus Ex"), r"Save"), (Documents, r"Deus Ex\Save")], ["*.dxs"], 1),
    game!("Deus Ex: Human Revolution", HEX, r"Steam\userdata\<account>\238010\remote", r"GAMEA1_4 …", [(SteamUser("238010"), r"remote")], ["GAME*"], 1),
    game!("Deus Ex: Mankind Divided", HEX, r"Steam\userdata\<account>\337000\remote", r"DXNGsavegame0000.dat", [(SteamUser("337000"), r"remote")], ["DXNGsavegame*.dat"], 1),
    game!("Diablo II (Lord of Destruction 1.10+)", FULL, r"%USERPROFILE%\Saved Games\Diablo II (1.13 and older: <install folder>\Save)", r"<name>.d2s", [(SavedGames, r"Diablo II")], ["*.d2s"], 1),
    game!("Diablo II: Resurrected", FULL, r"%USERPROFILE%\Saved Games\Diablo II Resurrected", r"<name>.d2s (SharedStash*.d2i is the shared stash)", [(SavedGames, r"Diablo II Resurrected")], ["*.d2s"], 2),
    game!("Dishonored", HEX, r"Steam\userdata\<account>\205100\remote", r"dishonored0.sav, DisAutoSave0.sav", [(SteamUser("205100"), r"remote")], ["*.sav"], 1),
    game!("Dishonored 2 / Death of the Outsider", VALUES, r"%USERPROFILE%\Saved Games\Arkane Studios\Dishonored2\base\savegame\SLOT<n>", r"game.details (chapter, chaos level…)", [(SavedGames, r"Arkane Studios\Dishonored2\base\savegame"), (SavedGames, r"Arkane Studios\Dishonored2_DOTO\base\savegame")], ["game.details"], 2),
    game!("Divinity: Original Sin 2 Definitive Edition", "Game screen: each player character's experience, vitality, attributes, unspent points and gold; packed again with the package checksum on save. Not yet tried in the game", r"Documents\Larian Studios\Divinity Original Sin 2 Definitive Edition\PlayerProfiles\<profile>\Savegames\Story\<save>", r"<save>.lsv", [(Documents, r"Larian Studios\Divinity Original Sin 2 Definitive Edition\PlayerProfiles\*\Savegames\Story")], ["*.lsv"], 2),
    game!("Dragon Age: Origins (and Awakening)", DAO, r"Documents\BioWare\Dragon Age\Characters\<name>\Saves\<slot>", r"<name>.das, savegame.das (auto and quick saves)", [(Documents, r"BioWare\Dragon Age\Characters\*\Saves")], ["*.das"], 2),
    game!("Dragon's Dogma 2", VALUES, r"Steam\userdata\<account>\2054970\remote\win64_save", "data*.bin", [(SteamUser("2054970"), r"remote\win64_save")], ["*.bin"], 1),
    game!("Dying Light", FULL, r"Steam\userdata\<account>\239140\remote\out\save", r"save_main_0.sav (gzip)", [(SteamUser("239140"), r"remote\out\save")], ["*.sav"], 1),
    game!("Dying Light 2 / The Beast", FULL, r"Steam\userdata\<account>\534380\remote\out\save (backups in Documents\dying light 2\out\-save_backups)", r"save_main_0.sav (gzip)", [(SteamUser("534380"), r"remote\out\save"), (SteamUser("3008130"), r"remote\out\save"), (Documents, r"dying light 2\out\-save_backups"), (Documents, r"dying light the beast\out\-save_backups")], ["*.sav", "*.sav.bak"], 1),
    game!("Elden Ring", FULL, r"%APPDATA%\EldenRing\<Steam id>", "ER0000.sl2", [(AppData, "EldenRing")], ["ER0000.sl2"], 2),
    game!("Enderal: Forgotten Stories (Special Edition)", FULL, r"Documents\My Games\Enderal Special Edition\Saves", "*.ess", [(Documents, r"My Games\Enderal Special Edition\Saves"), (Documents, r"My Games\Enderal\Saves")], ["*.ess"], 1),
    game!("Europa Universalis IV", FULL, r"Documents\Paradox Interactive\Europa Universalis IV\save games", "*.eu4", [(Documents, r"Paradox Interactive\Europa Universalis IV\save games")], ["*.eu4"], 1),
    game!("Europa Universalis V", PDX_BINARY, r"Documents\Paradox Interactive\Europa Universalis V\save games", "*.eu5", [(Documents, r"Paradox Interactive\Europa Universalis V\save games")], ["*.eu5"], 1),
    game!("Fallout (1997)", FALLOUT12, r"<install folder>\data\SAVEGAME\SLOT01 … SLOT10 (Steam: steamapps\common\Fallout; GOG: GOG Games\Fallout)", r"SAVE.DAT (the .SAV files next to it are the maps)", [(SteamGame("Fallout"), r"data\SAVEGAME")], ["SAVE.DAT"], 1),
    game!("Fallout 2", FALLOUT12, r"<install folder>\data\SAVEGAME\SLOT01 … SLOT10 (Steam: steamapps\common\Fallout 2; GOG: GOG Games\Fallout 2)", r"SAVE.DAT (the .SAV files next to it are the maps)", [(SteamGame("Fallout 2"), r"data\SAVEGAME")], ["SAVE.DAT"], 1),
    game!("Fallout 3", FALLOUT3NV, r"Documents\My Games\Fallout3\Saves", "*.fos", [(Documents, r"My Games\Fallout3\Saves")], ["*.fos"], 2),
    game!("Fallout 4", FULL, r"Documents\My Games\Fallout4\Saves", "*.fos", [(Documents, r"My Games\Fallout4\Saves"), (Documents, r"My Games\Fallout4VR\Saves")], ["*.fos"], 2),
    game!("Fallout: New Vegas", FALLOUT3NV, r"Documents\My Games\FalloutNV\Saves", "*.fos", [(Documents, r"My Games\FalloutNV\Saves")], ["*.fos"], 2),
    game!("Far Cry (Ubisoft Connect games)", HEX, r"Ubisoft Connect: <Program Files (x86)>\Ubisoft\Ubisoft Game Launcher\savegames\<user>\<game number>", r"<number>.save", [(ProgramFilesX86, r"Ubisoft\Ubisoft Game Launcher\savegames\*\*")], ["*.save", "*.save.z"], 1),
    game!("Final Fantasy IX", FULL, r"%USERPROFILE%\AppData\LocalLow\SquareEnix\FINAL FANTASY IX\Steam\EncryptedSavedData", r"SavedData_ww.dat (encrypted)", [(LocalLow, r"SquareEnix\FINAL FANTASY IX")], ["*.dat"], 3),
    game!("Final Fantasy VII (2013 Steam)", FULL, r"Documents\Square Enix\FINAL FANTASY VII Steam\user_<id>", r"save00.ff7 …", [(Documents, r"Square Enix\FINAL FANTASY VII Steam")], ["*.ff7"], 2),
    game!("Final Fantasy VII Rebirth", FULL, r"Documents\My Games\FINAL FANTASY VII REBIRTH\Steam\<Steam id>", r"ff7rebirth000.sav …", [(Documents, r"My Games\FINAL FANTASY VII REBIRTH\*\*")], ["ff7rebirth*.sav"], 1),
    game!("Final Fantasy VII Remake Intergrade", FULL, r"Documents\My Games\FINAL FANTASY VII REMAKE\(Steam or EOS)\<id>", r"ff7remake000.sav …", [(Documents, r"My Games\FINAL FANTASY VII REMAKE\*\*")], ["ff7remake*.sav"], 1),
    game!("Final Fantasy X / X-2 HD Remaster", FULL, r"Documents\SQUARE ENIX\FINAL FANTASY X&X-2 HD Remaster", r"ffx_###", [(Documents, r"SQUARE ENIX\FINAL FANTASY X&X-2 HD Remaster")], ["ffx_*", "ffx2_*"], 3),
    game!("Final Fantasy XV", FULL, r"Documents\My Games\FINAL FANTASY XV\Steam\<Steam id>\savestorage (also Steam\userdata\<account>\637650\remote)", r"gameplay slot files", [(Documents, r"My Games\FINAL FANTASY XV"), (SteamUser("637650"), r"remote")], ["*.save", "*.sav", "*.dat", "gameplay*"], 4),
    game!("Final Fantasy XVI", HEX, r"Documents\My Games\FINAL FANTASY XVI\Steam\<Steam id> (EOS for Epic)", r"one .png file per slot, plus the autosave", [(Documents, r"My Games\FINAL FANTASY XVI\*\*")], ["*.png", "*.sav"], 1),
    game!("God of War (2018)", FULL, r"%USERPROFILE%\Saved Games\God of War\<account>", r"game.sav (all slots)", [(SavedGames, r"God of War\*")], ["*.sav"], 1),
    game!("God of War Ragnarök", FULL, r"%USERPROFILE%\Saved Games\God of War Ragnarök\<account>", r"GOWRSAVE10 …", [(SavedGames, r"God of War Ragnarök\*")], ["GOWRSAVE*"], 1),
    game!("Golden Sun (GBA, emulator saves)", GOLDEN_SUN, GBA_FOLDER, r"Golden Sun (USA, Europe).sav … (64 KiB flash)", [(Documents, "mGBA"), (AppData, "visualboyadvance-m"), (Documents, "ROMs")], ["*Golden Sun.sav", "*Golden Sun (*.sav", "*Golden Sun [*.sav"], 3),
    game!("Golden Sun: The Lost Age (GBA, emulator saves)", GOLDEN_SUN, GBA_FOLDER, r"Golden Sun - The Lost Age (USA, Europe).sav … (64 KiB flash)", [(Documents, "mGBA"), (AppData, "visualboyadvance-m"), (Documents, "ROMs")], ["*Lost Age*.sav"], 3),
    game!("Green Hell", FULL, r"Steam\userdata\<account>\815370\remote", r"SPSlot0.sav …", [(SteamUser("815370"), r"remote")], ["*Slot*.sav"], 1),
    game!("Grim Dawn", "Game screen: money, level, experience, unspent attribute/skill/devotion points, attributes, skill ranks and item stacks; decrypted, then encrypted again with fresh checksums. Not yet tried in the game", r"Documents\My Games\Grim Dawn\save\main\_<name> (Steam Cloud: Steam\userdata\<account>\219990\remote\save\main\_<name>)", r"player.gdc", [(Documents, r"My Games\Grim Dawn\save\main"), (SteamUser("219990"), r"remote\save\main")], ["player.gdc"], 1),
    game!("Hades", FULL, r"Documents\Saved Games\Hades", "Profile1.sav … Profile4.sav", [(Documents, r"Saved Games\Hades"), (SavedGames, "Hades")], ["Profile*.sav"], 1),
    game!("Hades II", FULL, r"Saved Games\Hades II", "Profile1.sav …", [(SavedGames, "Hades II"), (Documents, r"Saved Games\Hades II")], ["Profile*.sav"], 1),
    game!("Half-Life / Opposing Force / Blue Shift", HL1, r"<Steam library>\steamapps\common\Half-Life\valve\SAVE (Opposing Force: gearbox\SAVE, Blue Shift: bshift\SAVE)", r"quick.sav, autosave.sav, Half-Life-001.sav …", [(SteamGame("Half-Life"), r"valve\SAVE"), (SteamGame("Half-Life"), r"gearbox\SAVE"), (SteamGame("Half-Life"), r"bshift\SAVE")], ["*.sav"], 1),
    game!("Half-Life 2 / Episode One / Episode Two", HL2, r"<Steam library>\steamapps\common\Half-Life 2\hl2\save (Episode One: episodic\save, Episode Two: ep2\save)", r"quick.sav, autosave.sav, half-life-001.sav …", [(SteamGame("Half-Life 2"), r"hl2\save"), (SteamGame("Half-Life 2"), r"episodic\save"), (SteamGame("Half-Life 2"), r"ep2\save")], ["*.sav"], 1),
    game!("Half-Life: Source", HL2, r"<Steam library>\steamapps\common\Half-Life 2\hl1\save", r"quick.sav, autosave.sav, half-life-001.sav …", [(SteamGame("Half-Life 2"), r"hl1\save")], ["*.sav"], 1),
    game!("Hearts of Iron IV", FULL, r"Documents\Paradox Interactive\Hearts of Iron IV\save games", "*.hoi4", [(Documents, r"Paradox Interactive\Hearts of Iron IV\save games")], ["*.hoi4"], 1),
    game!("Hogwarts Legacy", FULL, r"%LOCALAPPDATA%\Hogwarts Legacy\Saved\SaveGames\<account>", r"HL-00-00.sav … (Oodle-compressed: the editor uses the Oodle library the game installed; not in the browser version)", [(LocalAppData, r"Hogwarts Legacy\Saved\SaveGames\*")], ["HL-*.sav"], 1),
    game!("Hollow Knight", FULL, r"%USERPROFILE%\AppData\LocalLow\Team Cherry\Hollow Knight", "user1.dat … user4.dat", [(LocalLow, r"Team Cherry\Hollow Knight")], ["user?.dat"], 1),
    game!("Hollow Knight: Silksong", "Game screen: rosaries, shell shards, masks, silk, needle, abilities, crests, tools and items; decrypted and encrypted again on save. Not yet tried in the game", r"%USERPROFILE%\AppData\LocalLow\Team Cherry\Hollow Knight Silksong\<Steam id>", "user1.dat … user4.dat", [(LocalLow, r"Team Cherry\Hollow Knight Silksong")], ["user?.dat"], 2),
    game!("Icewind Dale: Enhanced Edition", IE_IWD, r"Documents\Icewind Dale - Enhanced Edition\save\<nnnnnnnnn-Name>", r"ICEWIND.gam", [(Documents, r"Icewind Dale - Enhanced Edition\save"), (Documents, r"Icewind Dale - Enhanced Edition\mpsave")], ["*.gam"], 2),
    game!("Judgment", RGG_JUDGMENT, r"%APPDATA%\Sega\Judgment\Steam\<Steam id>", r"data.sav", [(AppData, r"Sega\Judgment\Steam\*")], ["*.sav"], 2),
    game!("Kenshi", FULL, r"%LOCALAPPDATA%\kenshi\save\<save name>", r"quick.save", [(LocalAppData, r"kenshi\save"), (SteamGame("Kenshi"), r"save")], ["quick.save"], 2),
    game!("Kunitsu-Gami: Path of the Goddess", VALUES, r"Steam\userdata\<account>\2510710\remote\win64_save", "data*.bin", [(SteamUser("2510710"), r"remote\win64_save")], ["*.bin"], 1),
    game!("Like a Dragon Gaiden: The Man Who Erased His Name", RGG_UNTESTED, r"%APPDATA%\Sega\LikeADragonGaiden\Steam\<Steam id>", r"data.sav", [(AppData, r"Sega\LikeADragonGaiden\Steam\*")], ["*.sav"], 2),
    game!("Like a Dragon: Infinite Wealth", RGG_UNTESTED, r"%APPDATA%\Sega\LikeADragon8\Steam\<Steam id>", r"data.sav", [(AppData, r"Sega\LikeADragon8\Steam\*")], ["*.sav"], 2),
    game!("Like a Dragon: Ishin!", RGG_UNTESTED, r"%APPDATA%\SEGA\LikeADragonIshin\<Steam id>", r"data.sav (binary inside)", [(AppData, r"SEGA\LikeADragonIshin\*")], ["*.sav"], 2),
    game!("Like a Dragon: Pirate Yakuza in Hawaii", RGG_UNTESTED, r"%APPDATA%\Sega\LikeADragonPirates\Steam\<Steam id>", r"data.sav (MessagePack inside)", [(AppData, r"Sega\LikeADragonPirates\Steam\*")], ["*.sav"], 2),
    game!("Lost Judgment", RGG_UNTESTED, r"%APPDATA%\Sega\LostJudgment\Steam\<Steam id>", r"data.sav", [(AppData, r"Sega\LostJudgment\Steam\*")], ["*.sav"], 2),
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
    game!("Metro 2033 Redux", METRO, r"Documents\4A Games\Metro 2033\<account>", r"game save files", [(Documents, r"4A Games\Metro 2033\*"), (Documents, r"4A Games\Metro Redux\*")], ["*"], 1),
    game!("Metro Exodus", METRO, r"Steam\userdata\<account>\412020\remote (and its game<n> folders)", r"m3_auto_save, m3_quick_save, m3_<level>.player", [(SteamUser("412020"), r"remote"), (SteamUser("412020"), r"remote\*")], ["m3_*"], 1),
    game!("Metro: Last Light Redux", METRO, r"Documents\4A Games\Metro Last Light\<account>", r"game save files", [(Documents, r"4A Games\Metro Last Light\*")], ["*"], 1),
    game!("Minecraft (Java Edition)", MINECRAFT, r"%APPDATA%\.minecraft\saves\<world>", r"level.dat (and region\*.mca)", [(AppData, r".minecraft\saves")], ["level.dat"], 2),
    game!("Monster Hunter Stories 3: Twisted Reflection", VALUES, r"Steam\userdata\<account>\2852190\remote\win64_save", "data*.bin", [(SteamUser("2852190"), r"remote\win64_save")], ["*.bin"], 1),
    game!("Monster Hunter Wilds", VALUES, r"Steam\userdata\<account>\2246340\remote\win64_save", "data*.bin", [(SteamUser("2246340"), r"remote\win64_save")], ["*.bin"], 1),
    game!("Neverwinter Nights: Enhanced Edition", "Game screen: gold, experience, ability scores, hit points, alignment and skills of the player (the module .sav, with player.bic kept in step) or a vault character (.bic). Not yet tried in the game", r"Documents\Neverwinter Nights\saves\<nnnnnn> - <name> (characters: Documents\Neverwinter Nights\localvault)", r"<Module>.sav (the player as the game loads it), player.bic, localvault\*.bic", [(Documents, r"Neverwinter Nights\saves"), (Documents, r"Neverwinter Nights\localvault")], ["*.sav", "*.bic"], 2),
    game!("Octopath Traveler", "Game screen: money (leaves) and the eight travellers' level, EXP, JP and current HP/SP, changed in place. Read from a sample save; changing them hasn't been tried in the game yet", r"Documents\My Games\Octopath_Traveler\<Steam id>\SaveGames", r"SaveData0.sav (autosave), SaveData1.sav …", [(Documents, r"My Games\Octopath_Traveler\*\SaveGames")], ["SaveData*.sav"], 1),
    game!("Octopath Traveler II", "Game screen: money (leaves) and the eight travellers' level, EXP, JP and current HP/SP, changed in place", r"Documents\My Games\Octopath_Traveler2\Steam\<Steam id>\SaveGames", r"SaveData0.sav (autosave), SaveData1.sav …", [(Documents, r"My Games\Octopath_Traveler2\*\*\SaveGames")], ["SaveData*.sav"], 1),
    game!("Onimusha: Way of the Sword", FULL, r"Steam\userdata\<account>\2638890\remote\win64_save", "data001Slot.bin", [(SteamUser("2638890"), r"remote\win64_save")], ["*.bin"], 1),
    game!("Oxygen Not Included", FULL, r"Documents\Klei\OxygenNotIncluded\cloud_save_files\<Steam id>\<colony> (or save_files\<colony>)", "<colony>.sav", [(Documents, r"Klei\OxygenNotIncluded\cloud_save_files\*"), (Documents, r"Klei\OxygenNotIncluded\save_files")], ["*.sav"], 2),
    game!("Palworld", FULL, r"%LOCALAPPDATA%\Pal\Saved\SaveGames\<Steam id>\<world>", r"Level.sav, Players\*.sav", [(LocalAppData, r"Pal\Saved\SaveGames")], ["*.sav"], 4),
    game!("Pathfinder: Kingmaker", FULL, r"%USERPROFILE%\AppData\LocalLow\Owlcat Games\Pathfinder Kingmaker\Saved Games", r"Manual_*.zks, Auto_*.zks, Quick_*.zks", [(LocalLow, r"Owlcat Games\Pathfinder Kingmaker\Saved Games")], ["*.zks"], 1),
    game!("Pathfinder: Wrath of the Righteous", FULL, r"%USERPROFILE%\AppData\LocalLow\Owlcat Games\Pathfinder Wrath Of The Righteous\Saved Games", r"Manual_*.zks, Auto_*.zks, Quick_*.zks", [(LocalLow, r"Owlcat Games\Pathfinder Wrath Of The Righteous\Saved Games")], ["*.zks"], 1),
    game!("Persona 3 Reload", HEX, r"%APPDATA%\SEGA\P3R\Steam\<Steam id>", r"SaveData*.sav", [(AppData, r"SEGA\P3R\Steam\*")], ["*.sav"], 2),
    game!("Persona 4 Golden", FULL, r"Steam\userdata\<account>\1113000\remote", r"data0001.bin …", [(SteamUser("1113000"), r"remote")], ["data*.bin"], 1),
    game!("Persona 5 Royal", FULL, r"%APPDATA%\SEGA\P5R\Steam\<Steam id>\savedata\DATA<nn>", r"DATA.DAT (encrypted)", [(AppData, r"SEGA\P5R\Steam\*\savedata")], ["DATA.DAT", "SYSTEM.DAT"], 2),
    game!("Planescape: Torment: Enhanced Edition", IE_PST, r"Documents\Planescape Torment - Enhanced Edition\save\<nnnnnnnnn-Name>", r"BALDUR.gam (BALDUR.SAV holds the areas)", [(Documents, r"Planescape Torment - Enhanced Edition\save")], ["*.gam"], 2),
    game!("Pragmata", VALUES, r"Steam\userdata\<account>\3357650\remote\win64_save", "data*.bin", [(SteamUser("3357650"), r"remote\win64_save")], ["*.bin"], 1),
    game!("Prey (2017)", HEX, r"%USERPROFILE%\Saved Games\Arkane Studios\Prey\SaveGames (Prey_MS for the Microsoft Store version)", r"Campaign<n>\<slot>\save.CSF (the player), <level>.level (each area)", [(SavedGames, r"Arkane Studios\Prey\SaveGames"), (SavedGames, r"Arkane Studios\Prey_MS\SaveGames")], ["save.CSF"], 3),
    game!("Red Dead Redemption", HEX, r"Documents\Rockstar Games\Red Dead Redemption\Profiles\<id>", r"RDR2SAVE0.SAV … (the PC port's own name)", [(Documents, r"Rockstar Games\Red Dead Redemption\Profiles\*")], ["*.SAV"], 1),
    game!("Red Dead Redemption 2", HEX, r"Documents\Rockstar Games\Red Dead Redemption 2\Profiles\<id>", r"SRDR3xxxxx", [(Documents, r"Rockstar Games\Red Dead Redemption 2\Profiles\*")], ["SRDR*"], 1),
    game!("Ren'Py visual novels", FULL, r"%APPDATA%\RenPy\<game>", "1-1-LT1.save, persistent", [(AppData, "RenPy")], ["*.save", "persistent"], 2),
    game!("Resident Evil Requiem", FULL, r"Steam\userdata\<account>\3764200\remote\win64_save", "data*.bin", [(SteamUser("3764200"), r"remote\win64_save")], ["*.bin"], 1),
    game!("RimWorld", FULL, r"%USERPROFILE%\AppData\LocalLow\Ludeon Studios\RimWorld by Ludeon Studios\Saves", "<colony>.rws", [(LocalLow, r"Ludeon Studios\RimWorld by Ludeon Studios\Saves")], ["*.rws"], 1),
    game!("RPG Maker MV / MZ games", FULL, r"Inside each game's folder: www\save (MV) or save (MZ); Steam games are found in steamapps\common", r"file1.rpgsave, file2.rpgsave … (MZ: file1.rmmzsave)", [(SteamGame(""), r"*\www\save"), (SteamGame(""), r"*\save")], ["file*.rpgsave", "file*.rmmzsave"], 1),
    game!("Sekiro: Shadows Die Twice", FULL, r"%APPDATA%\Sekiro\<Steam id>", "S0000.sl2", [(AppData, "Sekiro")], ["S0000.sl2"], 2),
    game!("Skyrim (Special / Anniversary Edition)", FULL, r"Documents\My Games\Skyrim Special Edition\Saves", "*.ess", [(Documents, r"My Games\Skyrim Special Edition\Saves"), (Documents, r"My Games\Skyrim Special Edition GOG\Saves"), (Documents, r"My Games\Skyrim\Saves"), (Documents, r"My Games\Skyrim VR\Saves")], ["*.ess"], 2),
    game!("Slay the Spire", FULL, r"<Steam library>\steamapps\common\SlayTheSpire\saves", "IRONCLAD.autosave …", [(SteamGame("SlayTheSpire"), "saves")], ["*.autosave"], 1),
    game!("Star Wars: Knights of the Old Republic", KOTOR, r"<install folder>\saves\<nnnnnn> - <name> (Steam: steamapps\common\swkotor\saves)", r"SAVEGAME.sav, PARTYTABLE.res (auto-saves: pifo.ifo holds the player)", [(SteamGame("swkotor"), "saves")], ["SAVEGAME.sav", "PARTYTABLE.res", "pifo.ifo"], 2),
    game!("Star Wars: Knights of the Old Republic II \u{2013} The Sith Lords", KOTOR, r"<install folder>\saves\<nnnnnn> - <name> (Steam Cloud: <install folder>\cloudsaves\<Steam id>\<nnnnnn> - <name>)", r"SAVEGAME.sav, PARTYTABLE.res (auto-saves: pifo.ifo holds the player)", [(SteamGame("Knights of the Old Republic II"), "saves"), (SteamGame("Knights of the Old Republic II"), r"cloudsaves\*")], ["SAVEGAME.sav", "PARTYTABLE.res", "pifo.ifo"], 2),
    game!("Starbound", STARBOUND, r"<Steam library>\steamapps\common\Starbound\storage\player", "<id>.player (one per character)", [(SteamGame("Starbound"), r"storage\player")], ["*.player"], 1),
    game!("Stardew Valley", STARDEW, r"%APPDATA%\StardewValley\Saves\<Farm>_<id>", "<Farm>_<id> (no extension)", [(AppData, r"StardewValley\Saves")], ["*_[0-9]*"], 2),
    game!("Stellaris", FULL, r"Documents\Paradox Interactive\Stellaris\save games\<empire>", "autosave_<date>.sav, ironman.sav", [(Documents, r"Paradox Interactive\Stellaris\save games")], ["*.sav"], 2),
    game!("Subnautica", FULL, r"<Steam library>\steamapps\common\Subnautica\SNAppData\SavedGames\slot0000", "gameinfo.json (game mode)", [(SteamGame("Subnautica"), r"SNAppData\SavedGames\*")], ["gameinfo.json"], 1),
    game!("Subnautica: Below Zero", FULL, r"<Steam library>\steamapps\common\SubnauticaZero\SNAppData\SavedGames\slot0000", "gameinfo.json (game mode)", [(SteamGame("SubnauticaZero"), r"SNAppData\SavedGames\*")], ["gameinfo.json"], 1),
    game!("Terraria (characters)", TERRARIA_PLR, r"Documents\My Games\Terraria\Players", "<character>.plr", [(Documents, r"My Games\Terraria\Players"), (Documents, r"My Games\Terraria\tModLoader\Players")], ["*.plr"], 1),
    game!("Terraria (worlds)", FULL, r"Documents\My Games\Terraria\Worlds", "<world>.wld (Terraria 1.4 or later)", [(Documents, r"My Games\Terraria\Worlds"), (Documents, r"My Games\Terraria\tModLoader\Worlds")], ["*.wld"], 1),
    game!("The Witcher 3", FULL, r"Documents\The Witcher 3\gamesaves", r"ManualSave_*.sav, AutoSave_*.sav", [(Documents, r"The Witcher 3\gamesaves")], ["*.sav"], 1),
    game!("Titan Quest Anniversary Edition", "Game screen: level, experience, money, unspent points, the five attributes and skill levels (stash files aren't read). Not yet tried in the game", r"Documents\My Games\Titan Quest - Immortal Throne\SaveData\Main\_<name>", r"Player.chr", [(Documents, r"My Games\Titan Quest - Immortal Throne\SaveData\Main"), (Documents, r"My Games\Titan Quest - Immortal Throne\SaveData\User")], ["Player.chr"], 1),
    game!("Torchlight II", "Game screen: level, experience, fame, gold, unspent points, the four stats, health, mana and item stacks; scrambled again with its checksum. Saves in the older 0x40 format aren't supported. Not yet tried in the game", r"Documents\My Games\Runic Games\Torchlight 2\save\<Steam id> (modsave\<Steam id> with mods)", r"<NAME>_{<id>}.SVB", [(Documents, r"My Games\Runic Games\Torchlight 2\save"), (Documents, r"My Games\Runic Games\Torchlight 2\modsave")], ["*.svb"], 1),
    game!("Undertale", "Game screen: stats, gold, EXP, items, phone and equipment (by name); every line of the file can be edited. Not yet tried in the game", r"%LOCALAPPDATA%\UNDERTALE", r"file0 (file9 is a copy; undertale.ini is the load menu)", [(LocalAppData, "UNDERTALE")], ["file0", "file9", "file8"], 1),
    game!("Valheim (characters)", VALHEIM, r"%USERPROFILE%\AppData\LocalLow\IronGate\Valheim\characters_local (older or Steam Cloud: characters)", "<name>.fch", [(LocalLow, r"IronGate\Valheim\characters_local"), (LocalLow, r"IronGate\Valheim\characters"), (SteamUser("892970"), r"remote\characters")], ["*.fch"], 1),
    game!("Vampire Survivors", "Game screen: coins, power-up ranks and what's unlocked; the checksum is worked out again on save. Not yet tried in the game", r"Steam\userdata\<account>\1794680\remote (older builds: %APPDATA%\Vampire_Survivors\saves)", r"SaveData (older builds: SaveData.sav)", [(SteamUser("1794680"), "remote"), (AppData, r"Vampire_Survivors\saves"), (AppData, "Vampire_Survivors_Data")], ["SaveData", "SaveData.sav"], 1),
    game!("Victoria 3", PDX_BINARY, r"Documents\Paradox Interactive\Victoria 3\save games", "*.v3", [(Documents, r"Paradox Interactive\Victoria 3\save games")], ["*.v3"], 1),
    game!("Wasteland 3", WASTELAND3, r"Documents\My Games\Wasteland3\Save Games", r"<save name>.xml (packed, despite the name)", [(Documents, r"My Games\Wasteland3\Save Games")], ["*.xml"], 3),
    game!("XCOM 2", XCOM2, r"Documents\My Games\XCOM2\XComGame\SaveData", r"save1, save2 … (no extension)", [(Documents, r"My Games\XCOM2\XComGame\SaveData")], ["save*"], 1),
    game!("XCOM 2: War of the Chosen", XCOM2, r"Documents\My Games\XCOM2 War of the Chosen\XComGame\SaveData", r"save_AUTOSAVE- Campaign …, save_<name>", [(Documents, r"My Games\XCOM2 War of the Chosen\XComGame\SaveData")], ["save*"], 1),
    game!("XCOM: Chimera Squad", HEX_UNPACKED, r"Documents\My Games\XCOM Chimera Squad\XComGame\SaveData", r"save*", [(Documents, r"My Games\XCOM Chimera Squad\XComGame\SaveData")], ["save*"], 1),
    game!("XCOM: Enemy Unknown / Enemy Within", FULL, r"Documents\My Games\XCOM - Enemy Unknown (or XCOM - Enemy Within)\XComGame\SaveData", r"save1, save2 … (no extension)", [(Documents, r"My Games\XCOM - Enemy Unknown\XComGame\SaveData"), (Documents, r"My Games\XCOM - Enemy Within\XComGame\SaveData")], ["save*"], 1),
    game!("Yakuza 0", FULL, r"Steam\userdata\<account>\638970\remote", r"SaveData00NN.sav", [(SteamUser("638970"), r"remote")], ["SaveData*.sav"], 1),
    game!("Yakuza 3 Remastered", HEX, r"Steam\userdata\<account>\1088710\remote", r"savegameN.sav", [(SteamUser("1088710"), r"remote")], ["savegame*.sav"], 1),
    game!("Yakuza 4 Remastered", HEX, r"Steam\userdata\<account>\1105500\remote", r"save files", [(SteamUser("1105500"), r"remote")], ["*.sav"], 1),
    game!("Yakuza 5 Remastered", HEX, r"Steam\userdata\<account>\1105510\remote", r"save files", [(SteamUser("1105510"), r"remote")], ["*.sav"], 1),
    game!("Yakuza 6: The Song of Life", RGG_Y6, r"Steam\userdata\<account>\1388590\remote", r"save files", [(SteamUser("1388590"), r"remote")], ["*.sav"], 2),
    game!("Yakuza Kiwami", FULL, r"Steam\userdata\<account>\834530\remote", r"SaveData00NN.sav, autosave00NN.sav", [(SteamUser("834530"), r"remote")], ["SaveData*.sav", "autosave*.sav"], 1),
    game!("Yakuza Kiwami 2", FULL, r"%APPDATA%\SEGA\YakuzaKiwami2\<Steam account>\saveNN", r"data.sav (one folder per slot)", [(AppData, r"SEGA\YakuzaKiwami2\*"), (AppData, r"SEGA\YakuzaKiwami2_GOG")], ["data.sav"], 2),
    game!("Yakuza Kiwami 3 & Dark Ties", RGG_UNTESTED, r"%APPDATA%\Sega\YakuzaKiwami3\Steam\<Steam id>", r"data.sav (MessagePack inside)", [(AppData, r"Sega\YakuzaKiwami3\Steam\*")], ["*.sav"], 2),
    game!("Yakuza: Like a Dragon", FULL, r"%APPDATA%\SEGA\YakuzaLikeADragon\<Steam account>\saveNNN", r"data.sav (one folder per slot)", [(AppData, r"SEGA\YakuzaLikeADragon\*"), (AppData, r"SEGA\YakuzaLikeADragon_GOG\*")], ["data.sav"], 2),
];

/// A save found for a game.
#[derive(Clone, Debug)]
pub struct Found {
    pub path: PathBuf,
    pub modified: Option<SystemTime>,
    pub size: u64,
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

/// The folders `base` stands for on this computer.
fn bases(base: Base) -> Vec<PathBuf> {
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
                }
            }
        }
        cur = next;
    }
    cur
}

/// The folders this game's saves would be in, that exist here.
pub fn folders(g: &GameInfo) -> Vec<PathBuf> {
    let mut out = vec![];
    for (base, rel) in g.places {
        for root in bases(*base) {
            out.extend(expand(&root, rel));
        }
    }
    out
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
                out.push(Found { path: p, modified: meta.modified().ok(), size: meta.len() });
            }
        }
    }
}

/// Entries that cover many games (an engine or a launcher): their saves are listed by game.
const MANY_GAMES: [&str; 4] = ["Ren'Py visual novels", "RPG Maker MV / MZ games", "Assassin's Creed (Ubisoft Connect games)", "Far Cry (Ubisoft Connect games)"];

pub fn many_games(g: &GameInfo) -> bool {
    MANY_GAMES.contains(&g.name)
}

/// Which game a save belongs to, for entries that cover many games: the folder the game
/// has under the search place (its last `*` part, or the folder right under it).
pub fn group_of(g: &GameInfo, path: &Path) -> String {
    for (base, rel) in g.places {
        let parts: Vec<&str> = rel.split(['\\', '/']).filter(|p| !p.is_empty()).collect();
        let at = parts.iter().rposition(|p| p.contains('*')).unwrap_or(parts.len());
        for root in bases(*base) {
            let Ok(sub) = path.strip_prefix(&root) else { continue };
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
    for dir in folders(g) {
        walk(&dir, g.depth.saturating_sub(1), g, &mut out);
    }
    out.sort_by(|a, b| b.modified.cmp(&a.modified));
    out.dedup_by(|a, b| a.path == b.path);
    out
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
