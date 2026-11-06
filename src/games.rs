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
const RGG_UNTESTED: &str = "Decrypted, with its checksum kept right (no save of this game has been tried yet); values in the Tree";
const HEX_UNPACKED: &str = "Unpacked for Value search (find a number you know, then change it); packed again on save";
const XCOM2: &str = "Game screen: HQ resources and soldiers (rank, kills, missions, stats); unpacked and packed again on save. Not yet tried in the game";
const CIV6: &str = "Game screen: settings, mods and each civilisation (human or AI, whose turn it is); gold and the map are in the packed part and aren't mapped";
const PDX_BINARY: &str = "Text saves fully; ironman (binary) saves are shown but can't be edited";

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
    game!("BioShock Infinite", HEX_UNPACKED, r"Steam\userdata\<account>\8870\remote\savedata", r"<area>-0_<date>.bkm / .sav", [(SteamUser("8870"), r"remote\savedata")], ["*.bkm", "*.sav"], 1),
    game!("BioShock Remastered / BioShock 2 Remastered", HEX_UNPACKED, r"Documents\BioshockHD\Bioshock (or Bioshock2)\SaveGames", r"<date>.bsb", [(Documents, r"BioshockHD"), (AppData, r"BioshockHD")], ["*.bsb"], 3),
    game!("Cairn", FULL, r"%USERPROFILE%\Saved Games\TheGameBakers\Cairn_RETAIL\SAVEGAMES\RETAIL\STORY (SUMMIT for summit runs)", r"SLOT_A.CPK … SLOT_D.CPK", [(SavedGames, r"TheGameBakers\Cairn_RETAIL\SAVEGAMES\RETAIL"), (SavedGames, r"TheGameBakers\Cairn\SAVEGAMES")], ["*.CPK"], 2),
    game!("Civilization V", HEX, r"Documents\My Games\Sid Meier's Civilization 5\Saves\single (or ModdedSaves\single)", "<Leader>_<year>.Civ5Save", [(Documents, r"My Games\Sid Meier's Civilization 5\Saves"), (Documents, r"My Games\Sid Meier's Civilization 5\ModdedSaves")], ["*.Civ5Save"], 3),
    game!("Civilization VI", CIV6, r"Documents\My Games\Sid Meier's Civilization VI\Saves\Single", r"*.Civ6Save (auto\AutoSave_####.Civ6Save)", [(Documents, r"My Games\Sid Meier's Civilization VI\Saves")], ["*.Civ6Save"], 3),
    game!("Control", HEX_UNPACKED, r"Steam\userdata\<account>\870780\remote", r"savegame-slot-00_global …", [(SteamUser("870780"), r"remote"), (LocalAppData, r"Remedy\Control")], ["savegame-*"], 2),
    game!("Crisis Core: Final Fantasy VII Reunion", VALUES, r"Documents\My Games\CRISIS CORE FINAL FANTASY VII REUNION\Steam\<Steam id>", r"SAVEDATA_PROGRESS.sav", [(Documents, r"My Games\CRISIS CORE FINAL FANTASY VII REUNION\*\*")], ["*.sav"], 1),
    game!("Crusader Kings II", VALUES, r"Documents\Paradox Interactive\Crusader Kings II\save games", "*.ck2", [(Documents, r"Paradox Interactive\Crusader Kings II\save games")], ["*.ck2"], 1),
    game!("Crusader Kings III", PDX_BINARY, r"Documents\Paradox Interactive\Crusader Kings III\save games", "*.ck3", [(Documents, r"Paradox Interactive\Crusader Kings III\save games")], ["*.ck3"], 1),
    game!("Cyberpunk 2077", FULL, r"%USERPROFILE%\Saved Games\CD Projekt Red\Cyberpunk 2077\<save name>", r"sav.dat", [(SavedGames, r"CD Projekt Red\Cyberpunk 2077")], ["sav.dat"], 2),
    game!("Dark Souls II: Scholar of the First Sin", FULL, r"%APPDATA%\DarkSoulsII\<Steam id>", "DS2SOFS0000.sl2", [(AppData, "DarkSoulsII")], ["DS2SOFS0000.sl2"], 2),
    game!("Dark Souls III", FULL, r"%APPDATA%\DarkSoulsIII\<Steam id>", "DS30000.sl2", [(AppData, "DarkSoulsIII")], ["DS30000.sl2"], 2),
    game!("Dark Souls Remastered", FULL, r"Documents\NBGI\DARK SOULS REMASTERED\<id>", "DRAKS0005.sl2", [(Documents, r"NBGI\DARK SOULS REMASTERED")], ["DRAKS0005.sl2"], 2),
    game!("Dead Rising 2 / Off the Record", HEX, r"Documents\My Games\Dead Rising 2 (Off The Record)\saves", r"DCSAV00.DSF …", [(Documents, r"My Games\Dead Rising 2\saves"), (Documents, r"My Games\Dead Rising 2 Off The Record\saves")], ["*.DSF", "*.DSF.tmp"], 1),
    game!("Dead Rising 3", HEX, r"Documents\My Games\dead rising 3\saves\x64_build", r"D3S.*", [(Documents, r"My Games\dead rising 3\saves")], ["D3S.*", "d3s.*"], 2),
    game!("Dead Rising Deluxe Remaster", VALUES, r"Steam\userdata\<account>\2527390\remote\win64_save", "data*.bin", [(SteamUser("2527390"), r"remote\win64_save")], ["*.bin"], 1),
    game!("Death Stranding (Director's Cut)", HEX, r"%LOCALAPPDATA%\KojimaProductions\DeathStrandingDC\<account>\<slot>", r"checkpoint.dat", [(LocalAppData, r"KojimaProductions\DeathStrandingDC"), (LocalAppData, r"KojimaProductions\DeathStranding")], ["checkpoint.dat"], 3),
    game!("Death Stranding 2: On the Beach", HEX, r"Documents\DEATH STRANDING 2 - ON THE BEACH\<Steam id>", r"manualsave<n>.dat, checkpointsave<n>.dat", [(Documents, r"DEATH STRANDING 2 - ON THE BEACH\*")], ["*save*.dat"], 1),
    game!("Deathloop", HEX, r"%USERPROFILE%\Saved Games\Arkane Studios\Deathloop\base\savegame\<Steam id>\SLOT01", r"save.dloop", [(SavedGames, r"Arkane Studios\Deathloop\base\savegame")], ["*.dloop"], 3),
    game!("Deus Ex: Human Revolution", HEX, r"Steam\userdata\<account>\238010\remote", r"GAMEA1_4 …", [(SteamUser("238010"), r"remote")], ["GAME*"], 1),
    game!("Deus Ex: Mankind Divided", HEX, r"Steam\userdata\<account>\337000\remote", r"DXNGsavegame0000.dat", [(SteamUser("337000"), r"remote")], ["DXNGsavegame*.dat"], 1),
    game!("Diablo II (Lord of Destruction 1.10+)", FULL, r"%USERPROFILE%\Saved Games\Diablo II (1.13 and older: <install folder>\Save)", r"<name>.d2s", [(SavedGames, r"Diablo II")], ["*.d2s"], 1),
    game!("Diablo II: Resurrected", FULL, r"%USERPROFILE%\Saved Games\Diablo II Resurrected", r"<name>.d2s (SharedStash*.d2i is the shared stash)", [(SavedGames, r"Diablo II Resurrected")], ["*.d2s"], 2),
    game!("Dishonored", HEX, r"Steam\userdata\<account>\205100\remote", r"dishonored0.sav, DisAutoSave0.sav", [(SteamUser("205100"), r"remote")], ["*.sav"], 1),
    game!("Dishonored 2 / Death of the Outsider", VALUES, r"%USERPROFILE%\Saved Games\Arkane Studios\Dishonored2\base\savegame\SLOT<n>", r"game.details (chapter, chaos level…)", [(SavedGames, r"Arkane Studios\Dishonored2\base\savegame"), (SavedGames, r"Arkane Studios\Dishonored2_DOTO\base\savegame")], ["game.details"], 2),
    game!("Dragon's Dogma 2", VALUES, r"Steam\userdata\<account>\2054970\remote\win64_save", "data*.bin", [(SteamUser("2054970"), r"remote\win64_save")], ["*.bin"], 1),
    game!("Dying Light", FULL, r"Steam\userdata\<account>\239140\remote\out\save", r"save_main_0.sav (gzip)", [(SteamUser("239140"), r"remote\out\save")], ["*.sav"], 1),
    game!("Dying Light 2 / The Beast", FULL, r"Steam\userdata\<account>\534380\remote\out\save (backups in Documents\dying light 2\out\-save_backups)", r"save_main_0.sav (gzip)", [(SteamUser("534380"), r"remote\out\save"), (SteamUser("3008130"), r"remote\out\save"), (Documents, r"dying light 2\out\-save_backups"), (Documents, r"dying light the beast\out\-save_backups")], ["*.sav", "*.sav.bak"], 1),
    game!("Elden Ring", FULL, r"%APPDATA%\EldenRing\<Steam id>", "ER0000.sl2", [(AppData, "EldenRing")], ["ER0000.sl2"], 2),
    game!("Enderal: Forgotten Stories (Special Edition)", FULL, r"Documents\My Games\Enderal Special Edition\Saves", "*.ess", [(Documents, r"My Games\Enderal Special Edition\Saves"), (Documents, r"My Games\Enderal\Saves")], ["*.ess"], 1),
    game!("Europa Universalis IV", VALUES, r"Documents\Paradox Interactive\Europa Universalis IV\save games", "*.eu4", [(Documents, r"Paradox Interactive\Europa Universalis IV\save games")], ["*.eu4"], 1),
    game!("Europa Universalis V", PDX_BINARY, r"Documents\Paradox Interactive\Europa Universalis V\save games", "*.eu5", [(Documents, r"Paradox Interactive\Europa Universalis V\save games")], ["*.eu5"], 1),
    game!("Fallout 4", FULL, r"Documents\My Games\Fallout4\Saves", "*.fos", [(Documents, r"My Games\Fallout4\Saves"), (Documents, r"My Games\Fallout4VR\Saves")], ["*.fos"], 2),
    game!("Far Cry (Ubisoft Connect games)", HEX, r"Ubisoft Connect: <Program Files (x86)>\Ubisoft\Ubisoft Game Launcher\savegames\<user>\<game number>", r"<number>.save", [(ProgramFilesX86, r"Ubisoft\Ubisoft Game Launcher\savegames\*\*")], ["*.save", "*.save.z"], 1),
    game!("Final Fantasy IX", FULL, r"%USERPROFILE%\AppData\LocalLow\SquareEnix\FINAL FANTASY IX\Steam\EncryptedSavedData", r"SavedData_ww.dat (encrypted)", [(LocalLow, r"SquareEnix\FINAL FANTASY IX")], ["*.dat"], 3),
    game!("Final Fantasy VII (2013 Steam)", FULL, r"Documents\Square Enix\FINAL FANTASY VII Steam\user_<id>", r"save00.ff7 …", [(Documents, r"Square Enix\FINAL FANTASY VII Steam")], ["*.ff7"], 2),
    game!("Final Fantasy VII Rebirth", FULL, r"Documents\My Games\FINAL FANTASY VII REBIRTH\Steam\<Steam id>", r"ff7rebirth000.sav …", [(Documents, r"My Games\FINAL FANTASY VII REBIRTH\*\*")], ["ff7rebirth*.sav"], 1),
    game!("Final Fantasy VII Remake Intergrade", FULL, r"Documents\My Games\FINAL FANTASY VII REMAKE\(Steam or EOS)\<id>", r"ff7remake000.sav …", [(Documents, r"My Games\FINAL FANTASY VII REMAKE\*\*")], ["ff7remake*.sav"], 1),
    game!("Final Fantasy X / X-2 HD Remaster", FULL, r"Documents\SQUARE ENIX\FINAL FANTASY X&X-2 HD Remaster", r"ffx_###", [(Documents, r"SQUARE ENIX\FINAL FANTASY X&X-2 HD Remaster")], ["ffx_*", "ffx2_*"], 3),
    game!("Final Fantasy XV", HEX, r"Documents\My Games\FINAL FANTASY XV\Steam\<Steam id>\savestorage (also Steam\userdata\<account>\637650\remote)", r"gameplay slot files", [(Documents, r"My Games\FINAL FANTASY XV"), (SteamUser("637650"), r"remote")], ["*.save", "*.sav", "*.dat", "gameplay*"], 4),
    game!("Final Fantasy XVI", HEX, r"Documents\My Games\FINAL FANTASY XVI\Steam\<Steam id> (EOS for Epic)", r"one .png file per slot, plus the autosave", [(Documents, r"My Games\FINAL FANTASY XVI\*\*")], ["*.png", "*.sav"], 1),
    game!("God of War (2018)", FULL, r"%USERPROFILE%\Saved Games\God of War\<account>", r"game.sav (all slots)", [(SavedGames, r"God of War\*")], ["*.sav"], 1),
    game!("God of War Ragnarök", FULL, r"%USERPROFILE%\Saved Games\God of War Ragnarök\<account>", r"GOWRSAVE10 …", [(SavedGames, r"God of War Ragnarök\*")], ["GOWRSAVE*"], 1),
    game!("Green Hell", HEX, r"Steam\userdata\<account>\815370\remote", r"SPSlot0.sav …", [(SteamUser("815370"), r"remote")], ["*Slot*.sav"], 1),
    game!("Hades", FULL, r"Documents\Saved Games\Hades", "Profile1.sav … Profile4.sav", [(Documents, r"Saved Games\Hades"), (SavedGames, "Hades")], ["Profile*.sav"], 1),
    game!("Hades II", FULL, r"Saved Games\Hades II", "Profile1.sav …", [(SavedGames, "Hades II"), (Documents, r"Saved Games\Hades II")], ["Profile*.sav"], 1),
    game!("Hearts of Iron IV", FULL, r"Documents\Paradox Interactive\Hearts of Iron IV\save games", "*.hoi4", [(Documents, r"Paradox Interactive\Hearts of Iron IV\save games")], ["*.hoi4"], 1),
    game!("Hogwarts Legacy", FULL, r"%LOCALAPPDATA%\Hogwarts Legacy\Saved\SaveGames\<account>", r"HL-00-00.sav … (the character data inside is Oodle-compressed and can't be edited)", [(LocalAppData, r"Hogwarts Legacy\Saved\SaveGames\*")], ["HL-*.sav"], 1),
    game!("Hollow Knight", FULL, r"%USERPROFILE%\AppData\LocalLow\Team Cherry\Hollow Knight", "user1.dat … user4.dat", [(LocalLow, r"Team Cherry\Hollow Knight")], ["user?.dat"], 1),
    game!("Judgment", RGG_UNTESTED, r"%APPDATA%\Sega\Judgment\Steam\<Steam id>", r"data.sav", [(AppData, r"Sega\Judgment\Steam\*")], ["*.sav"], 2),
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
    game!("Mass Effect: Andromeda", HEX, r"Documents\BioWare\Mass Effect Andromeda\Save", r"Career<id>-<slot> (no extension)", [(Documents, r"BioWare\Mass Effect Andromeda\Save")], ["Career*"], 1),
    game!("Metro 2033 Redux", METRO, r"Documents\4A Games\Metro 2033\<account>", r"game save files", [(Documents, r"4A Games\Metro 2033\*"), (Documents, r"4A Games\Metro Redux\*")], ["*"], 1),
    game!("Metro Exodus", METRO, r"Steam\userdata\<account>\412020\remote (and its game<n> folders)", r"m3_auto_save, m3_quick_save, m3_<level>.player", [(SteamUser("412020"), r"remote"), (SteamUser("412020"), r"remote\*")], ["m3_*"], 1),
    game!("Metro: Last Light Redux", METRO, r"Documents\4A Games\Metro Last Light\<account>", r"game save files", [(Documents, r"4A Games\Metro Last Light\*")], ["*"], 1),
    game!("Minecraft (Java Edition)", FULL, r"%APPDATA%\.minecraft\saves\<world>", r"level.dat (and region\*.mca)", [(AppData, r".minecraft\saves")], ["level.dat"], 2),
    game!("Monster Hunter Stories 3: Twisted Reflection", VALUES, r"Steam\userdata\<account>\2852190\remote\win64_save", "data*.bin", [(SteamUser("2852190"), r"remote\win64_save")], ["*.bin"], 1),
    game!("Monster Hunter Wilds", VALUES, r"Steam\userdata\<account>\2246340\remote\win64_save", "data*.bin", [(SteamUser("2246340"), r"remote\win64_save")], ["*.bin"], 1),
    game!("Onimusha: Way of the Sword", FULL, r"Steam\userdata\<account>\2638890\remote\win64_save", "data001Slot.bin", [(SteamUser("2638890"), r"remote\win64_save")], ["*.bin"], 1),
    game!("Oxygen Not Included", FULL, r"Documents\Klei\OxygenNotIncluded\cloud_save_files\<Steam id>\<colony> (or save_files\<colony>)", "<colony>.sav", [(Documents, r"Klei\OxygenNotIncluded\cloud_save_files\*"), (Documents, r"Klei\OxygenNotIncluded\save_files")], ["*.sav"], 2),
    game!("Palworld", VALUES, r"%LOCALAPPDATA%\Pal\Saved\SaveGames\<Steam id>\<world>", r"Level.sav, Players\*.sav", [(LocalAppData, r"Pal\Saved\SaveGames")], ["*.sav"], 4),
    game!("Pathfinder: Kingmaker", FULL, r"%USERPROFILE%\AppData\LocalLow\Owlcat Games\Pathfinder Kingmaker\Saved Games", r"Manual_*.zks, Auto_*.zks, Quick_*.zks", [(LocalLow, r"Owlcat Games\Pathfinder Kingmaker\Saved Games")], ["*.zks"], 1),
    game!("Pathfinder: Wrath of the Righteous", FULL, r"%USERPROFILE%\AppData\LocalLow\Owlcat Games\Pathfinder Wrath Of The Righteous\Saved Games", r"Manual_*.zks, Auto_*.zks, Quick_*.zks", [(LocalLow, r"Owlcat Games\Pathfinder Wrath Of The Righteous\Saved Games")], ["*.zks"], 1),
    game!("Persona 3 Reload", HEX, r"%APPDATA%\SEGA\P3R\Steam\<Steam id>", r"SaveData*.sav", [(AppData, r"SEGA\P3R\Steam\*")], ["*.sav"], 2),
    game!("Persona 4 Golden", FULL, r"Steam\userdata\<account>\1113000\remote", r"data0001.bin …", [(SteamUser("1113000"), r"remote")], ["data*.bin"], 1),
    game!("Persona 5 Royal", FULL, r"%APPDATA%\SEGA\P5R\Steam\<Steam id>\savedata\DATA<nn>", r"DATA.DAT (encrypted)", [(AppData, r"SEGA\P5R\Steam\*\savedata")], ["DATA.DAT", "SYSTEM.DAT"], 2),
    game!("Pragmata", VALUES, r"Steam\userdata\<account>\3357650\remote\win64_save", "data*.bin", [(SteamUser("3357650"), r"remote\win64_save")], ["*.bin"], 1),
    game!("Prey (2017)", HEX, r"%USERPROFILE%\Saved Games\Arkane Studios\Prey\SaveGames (Prey_MS for the Microsoft Store version)", r"Campaign<n>\<slot>\save.CSF (the player), <level>.level (each area)", [(SavedGames, r"Arkane Studios\Prey\SaveGames"), (SavedGames, r"Arkane Studios\Prey_MS\SaveGames")], ["save.CSF"], 3),
    game!("Red Dead Redemption", HEX, r"Documents\Rockstar Games\Red Dead Redemption\Profiles\<id>", r"RDR2SAVE0.SAV … (the PC port's own name)", [(Documents, r"Rockstar Games\Red Dead Redemption\Profiles\*")], ["*.SAV"], 1),
    game!("Red Dead Redemption 2", HEX, r"Documents\Rockstar Games\Red Dead Redemption 2\Profiles\<id>", r"SRDR3xxxxx", [(Documents, r"Rockstar Games\Red Dead Redemption 2\Profiles\*")], ["SRDR*"], 1),
    game!("Ren'Py visual novels", VALUES, r"%APPDATA%\RenPy\<game>", "1-1-LT1.save, persistent", [(AppData, "RenPy")], ["*.save", "persistent"], 2),
    game!("Resident Evil Requiem", VALUES, r"Steam\userdata\<account>\3764200\remote\win64_save", "data*.bin", [(SteamUser("3764200"), r"remote\win64_save")], ["*.bin"], 1),
    game!("RimWorld", FULL, r"%USERPROFILE%\AppData\LocalLow\Ludeon Studios\RimWorld by Ludeon Studios\Saves", "<colony>.rws", [(LocalLow, r"Ludeon Studios\RimWorld by Ludeon Studios\Saves")], ["*.rws"], 1),
    game!("RPG Maker MV / MZ games", FULL, r"Inside each game's folder: www\save (MV) or save (MZ); Steam games are found in steamapps\common", r"file1.rpgsave, file2.rpgsave … (MZ: file1.rmmzsave)", [(SteamGame(""), r"*\www\save"), (SteamGame(""), r"*\save")], ["file*.rpgsave", "file*.rmmzsave"], 1),
    game!("Sekiro: Shadows Die Twice", FULL, r"%APPDATA%\Sekiro\<Steam id>", "S0000.sl2", [(AppData, "Sekiro")], ["S0000.sl2"], 2),
    game!("Skyrim (Special / Anniversary Edition)", FULL, r"Documents\My Games\Skyrim Special Edition\Saves", "*.ess", [(Documents, r"My Games\Skyrim Special Edition\Saves"), (Documents, r"My Games\Skyrim Special Edition GOG\Saves"), (Documents, r"My Games\Skyrim\Saves"), (Documents, r"My Games\Skyrim VR\Saves")], ["*.ess"], 2),
    game!("Slay the Spire", FULL, r"<Steam library>\steamapps\common\SlayTheSpire\saves", "IRONCLAD.autosave …", [(SteamGame("SlayTheSpire"), "saves")], ["*.autosave"], 1),
    game!("Stardew Valley", FULL, r"%APPDATA%\StardewValley\Saves\<Farm>_<id>", "<Farm>_<id> (no extension)", [(AppData, r"StardewValley\Saves")], ["*_[0-9]*"], 2),
    game!("Stellaris", FULL, r"Documents\Paradox Interactive\Stellaris\save games\<empire>", "autosave_<date>.sav, ironman.sav", [(Documents, r"Paradox Interactive\Stellaris\save games")], ["*.sav"], 2),
    game!("Subnautica", FULL, r"<Steam library>\steamapps\common\Subnautica\SNAppData\SavedGames\slot0000", "gameinfo.json (game mode)", [(SteamGame("Subnautica"), r"SNAppData\SavedGames\*")], ["gameinfo.json"], 1),
    game!("Subnautica: Below Zero", FULL, r"<Steam library>\steamapps\common\SubnauticaZero\SNAppData\SavedGames\slot0000", "gameinfo.json (game mode)", [(SteamGame("SubnauticaZero"), r"SNAppData\SavedGames\*")], ["gameinfo.json"], 1),
    game!("Terraria (characters)", FULL, r"Documents\My Games\Terraria\Players", "<character>.plr", [(Documents, r"My Games\Terraria\Players"), (Documents, r"My Games\Terraria\tModLoader\Players")], ["*.plr"], 1),
    game!("Terraria (worlds)", FULL, r"Documents\My Games\Terraria\Worlds", "<world>.wld (Terraria 1.4 or later)", [(Documents, r"My Games\Terraria\Worlds"), (Documents, r"My Games\Terraria\tModLoader\Worlds")], ["*.wld"], 1),
    game!("The Witcher 3", FULL, r"Documents\The Witcher 3\gamesaves", r"ManualSave_*.sav, AutoSave_*.sav", [(Documents, r"The Witcher 3\gamesaves")], ["*.sav"], 1),
    game!("Victoria 3", PDX_BINARY, r"Documents\Paradox Interactive\Victoria 3\save games", "*.v3", [(Documents, r"Paradox Interactive\Victoria 3\save games")], ["*.v3"], 1),
    game!("XCOM 2", XCOM2, r"Documents\My Games\XCOM2\XComGame\SaveData", r"save1, save2 … (no extension)", [(Documents, r"My Games\XCOM2\XComGame\SaveData")], ["save*"], 1),
    game!("XCOM 2: War of the Chosen", XCOM2, r"Documents\My Games\XCOM2 War of the Chosen\XComGame\SaveData", r"save_AUTOSAVE- Campaign …, save_<name>", [(Documents, r"My Games\XCOM2 War of the Chosen\XComGame\SaveData")], ["save*"], 1),
    game!("XCOM: Chimera Squad", HEX_UNPACKED, r"Documents\My Games\XCOM Chimera Squad\XComGame\SaveData", r"save*", [(Documents, r"My Games\XCOM Chimera Squad\XComGame\SaveData")], ["save*"], 1),
    game!("XCOM: Enemy Unknown / Enemy Within", HEX_UNPACKED, r"Documents\My Games\XCOM - Enemy Unknown (or XCOM - Enemy Within)\XComGame\SaveData", r"save1, save2 … (no extension)", [(Documents, r"My Games\XCOM - Enemy Unknown\XComGame\SaveData"), (Documents, r"My Games\XCOM - Enemy Within\XComGame\SaveData")], ["save*"], 1),
    game!("Yakuza 0", FULL, r"Steam\userdata\<account>\638970\remote", r"SaveData00NN.sav", [(SteamUser("638970"), r"remote")], ["SaveData*.sav"], 1),
    game!("Yakuza 3 Remastered", HEX, r"Steam\userdata\<account>\1088710\remote", r"savegameN.sav", [(SteamUser("1088710"), r"remote")], ["savegame*.sav"], 1),
    game!("Yakuza 4 Remastered", HEX, r"Steam\userdata\<account>\1105500\remote", r"save files", [(SteamUser("1105500"), r"remote")], ["*.sav"], 1),
    game!("Yakuza 5 Remastered", HEX, r"Steam\userdata\<account>\1105510\remote", r"save files", [(SteamUser("1105510"), r"remote")], ["*.sav"], 1),
    game!("Yakuza 6: The Song of Life", RGG_UNTESTED, r"Steam\userdata\<account>\1388590\remote", r"save files", [(SteamUser("1388590"), r"remote")], ["*.sav"], 2),
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
