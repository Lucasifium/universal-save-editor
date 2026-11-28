//! How-to guides for the game engines whose saves look alike from game to game (Ren'Py, RPG Maker,
//! Unity, Unreal Engine and Godot). A guide is a block of the game's page when the catalog has
//! an entry for the engine, and a page of its own (Unreal Engine, Godot) when it has not.

pub(super) struct Guide {
    /// The catalog entry whose page carries the guide; `None` for a guide page of its own.
    pub game: Option<&'static str>,
    /// The engine's name as the guide titles use it.
    pub engine: &'static str,
    /// The anchor of the guide inside its page, and the name the "queue" and sitemap use.
    pub anchor: &'static str,
    /// Page name and folder for a guide page of its own (`<slug>/`).
    pub slug: &'static str,
    pub title: &'static str,
    pub desc: &'static str,
    pub lead: &'static str,
    /// A short sentence for the home page's list of guides.
    pub blurb: &'static str,
    /// Where a save is kept: (system, folder) rows.
    pub where_rows: &'static [(&'static str, &'static str)],
    /// The guide's sections as HTML (`<h3>` headings, paragraphs and lists).
    pub body: &'static str,
    /// Questions and answers (HTML) for a guide page of its own.
    pub faq: &'static [(&'static str, &'static str)],
}

pub(super) const GUIDES: &[Guide] = &[
    Guide {
        game: Some("Ren'Py visual novels"),
        engine: "Ren'Py",
        anchor: "renpy-guide",
        slug: "",
        title: "",
        desc: "",
        lead: "",
        blurb: "Visual novels: where the saves live, signed saves and what the variables are.",
        where_rows: &[
            ("Windows", r"%APPDATA%\RenPy\<game>-<number>"),
            ("Linux", "~/.renpy/<game>-<number>"),
            ("macOS", "~/Library/RenPy/<game>-<number>"),
        ],
        body: r#"<p>Ren'Py is the engine behind most visual novels. Its saves look the same in every game, so one guide covers them all. Some games, and older versions of the engine, keep saves in a <code>saves</code> folder next to the game instead.</p>
<h3>The files</h3>
<ul>
<li><code>1-1-LT1.save</code>, <code>1-2-LT1.save</code> and so on: one file per save slot (page, slot). <code>auto-1-LT1.save</code> and <code>quick-1-LT1.save</code> are the automatic and quick saves.</li>
<li><code>persistent</code>: what the game remembers between playthroughs, such as gallery unlocks, endings seen and settings. Its values are edited the same way.</li>
<li>A <code>.save</code> file is a zip archive holding the game's variables, a screenshot and a signature. The editor opens the archive for you, and puts it back together on save.</li>
</ul>
<h3>What the values mean</h3>
<p>The editor lists the story's own variables: numbers (money, affection, stats), true/false flags (a route finished, a scene seen) and text (the player's name). Names come from the game's script, so they differ from game to game: use the filter box, and look for <code>money</code>, <code>gold</code>, <code>love</code>, <code>points</code> and <code>route</code>. The usual ones are shown first. The engine's own internals sit under a separate heading and are best left alone.</p>
<h3>Signed saves (Ren'Py 8.1 and newer)</h3>
<p>Newer versions of Ren'Py sign every save with a key kept on your computer (on Windows, <code>%APPDATA%\RenPy\tokens\security_keys.txt</code>). A save whose data no longer matches its signature makes the game ask whether to trust it, and may refuse it. On Windows, Universal Save Editor signs the edited save again with your own key, which is what the game does itself when it saves, so the game loads it normally. If the key isn't found, it leaves the signature out and the game asks once before loading the save. A <code>persistent</code> file that carries a signature is not saved without the key, because the game would throw it away.</p>
<h3>Rollback</h3>
<p>A Ren'Py save also keeps a short history of earlier values, for rolling back. On load the game steps back to the last choice using that history, which would undo an edit made only to the current value. The editor changes the earlier copies too, so the edit is still there after loading.</p>
<h3>Things to know</h3>
<ul>
<li>Close the game before you edit, and load the edited slot from the game's own load screen.</li>
<li>Changing a number is safe; changing a true/false flag can skip or repeat a scene, so make a backup first (the editor does, automatically) and change one flag at a time.</li>
</ul>"#,
        faq: &[],
    },
    Guide {
        game: Some("RPG Maker MV / MZ games"),
        engine: "RPG Maker MV and MZ",
        anchor: "rpg-maker-mv-mz-guide",
        slug: "",
        title: "",
        desc: "",
        lead: "",
        blurb: "MV and MZ games: gold, party members, items and the save folder inside the game.",
        where_rows: &[("MV", r"<game folder>\www\save\file1.rpgsave"), ("MZ", r"<game folder>\save\file1.rmmzsave")],
        body: r#"<p>RPG Maker MV and MZ are built on web technology, and their saves are small text files kept inside the game's own folder (for a Steam game, under <code>steamapps\common\&lt;game&gt;</code>). Every MV or MZ game saves the same way, so this guide covers them all.</p>
<h3>The files</h3>
<ul>
<li><code>file1.rpgsave</code>, <code>file2.rpgsave</code> and so on (MV), or <code>file1.rmmzsave</code> (MZ): one per save slot. Slot 0 is the automatic save in some games.</li>
<li><code>global.rpgsave</code> (<code>global.rmmzsave</code>) lists the slots the load screen shows, and <code>config.rpgsave</code> holds the options. Open the numbered files to change your game.</li>
<li>The text inside is packed (LZ-String) JSON; the editor unpacks it and packs it again on save.</li>
<li>A game played in a web browser keeps its saves in the browser's storage instead, not in files, so there is nothing to open.</li>
</ul>
<h3>What can be changed</h3>
<ul>
<li><strong>Gold, steps and play time.</strong> The party's gold is the number most people want.</li>
<li><strong>Each party member.</strong> Name, level, experience, HP, MP and TP, the bonuses added to each stat, and the skills they have learned (by number).</li>
<li><strong>Items, weapons and armor.</strong> Shown by their number in the game's database. Change a count, remove an entry, or add one.</li>
</ul>
<p>The game's database is what gives a number its name, and it isn't part of the save: item 1 is whatever the game's maker put first. The names are in the game's <code>data</code> folder (<code>Items.json</code>, <code>Weapons.json</code>, <code>Armors.json</code>, <code>Skills.json</code>). Open the one you need in a text editor to look a number up.</p>
<h3>Things to know</h3>
<ul>
<li>Only add item numbers that exist in the game's database: an unknown number can stop the game when the item is shown.</li>
<li>Change a level and its experience together. The game works the level out from the experience when the party next gains any, so a level raised on its own can drop back.</li>
<li>Switches and variables that a game's story uses are kept in the save too, under the game's own system data. Change them one at a time and keep a backup.</li>
<li>The editor has a ready-made <em>max gold</em> preset for these games, which only ever raises the number.</li>
<li>Close the game first. Plugin-heavy games may add their own data to the save; it opens as part of the same tree.</li>
</ul>"#,
        faq: &[],
    },
    Guide {
        game: Some("RPG Maker XP games"),
        engine: "RPG Maker XP, VX and VX Ace",
        anchor: "rpg-maker-xp-guide",
        slug: "",
        title: "",
        desc: "",
        lead: "",
        blurb: "XP, VX and VX Ace games: the Ruby save files and the database numbers.",
        where_rows: &[("XP", r"<game folder>\Save1.rxdata"), ("VX", r"<game folder>\Save1.rvdata"), ("VX Ace", r"<game folder>\Save1.rvdata2")],
        body: r#"<p>The older RPG Maker versions (XP, VX and VX Ace) write their saves as Ruby data. Games made with them, including many well-known indie RPGs, usually keep the files in the game's own folder; a few use a folder named after the game under your user folder.</p>
<h3>The files</h3>
<ul>
<li><code>Save1.rxdata</code> (XP), <code>Save1.rvdata</code> (VX) and <code>Save1.rvdata2</code> (VX Ace), numbered for each slot (<code>Save2</code>, <code>Save3</code> and so on).</li>
<li>The data is a Ruby object dump (Marshal). The editor reads it and writes it back as it was, with only the values you changed different.</li>
</ul>
<h3>What can be changed</h3>
<ul>
<li><strong>Gold and steps.</strong></li>
<li><strong>The party and every actor.</strong> Level, experience, HP, SP and the stat bonuses.</li>
<li><strong>Items, weapons and armor you carry.</strong> Listed by database number.</li>
</ul>
<h3>Things to know</h3>
<ul>
<li>Numbers refer to the game's database, which lives in the game's <code>Data</code> folder (<code>Items.rxdata</code> and similar) and is not part of the save. The game's own item screen is the quickest way to match a number to a name: note the order of the items you carry.</li>
<li>Keep the item numbers you add inside the database's range, or the game may stop when it draws the item.</li>
<li>Close the game before you edit, and keep the backup the editor makes until you have tried the edit in the game.</li>
</ul>"#,
        faq: &[],
    },
    Guide {
        game: Some("Unity games (PlayerPrefs)"),
        engine: "Unity",
        anchor: "unity-guide",
        slug: "",
        title: "",
        desc: "",
        lead: "",
        blurb: "Unity games: PlayerPrefs in the registry, save files in LocalLow and Easy Save.",
        where_rows: &[
            ("Windows, PlayerPrefs", r"Registry: HKCU\Software\<company>\<game>"),
            ("Windows, save files", r"%USERPROFILE%\AppData\LocalLow\<company>\<game>"),
            ("Linux", "~/.config/unity3d/<company>/<game>/ (prefs and save files)"),
            ("macOS", "~/Library/Preferences/unity.<company>.<game>.plist and ~/Library/Application Support/<company>/<game>"),
        ],
        body: r#"<p>Unity is used for thousands of games, and each one decides for itself how to save. Most use one of a few ways, and the editor handles each.</p>
<h3>PlayerPrefs</h3>
<p>Small games often keep coins, unlocks and settings in PlayerPrefs. On Windows they are in the registry under <code>HKCU\Software\&lt;company&gt;\&lt;game&gt;</code>; on Linux in a <code>prefs</code> file under <code>~/.config/unity3d</code>; on macOS in a <code>unity.*.plist</code> file. Choose <em>Unity games (PlayerPrefs)</em> in <em>Find my game's saves</em>: the registry key opens like a save, every pref by name with its type (whole number, decimal or text) kept, and saving writes it back. <strong>Close the game first</strong>: Unity writes its prefs when the game quits, so an open game would put the old values back.</p>
<h3>Save files in LocalLow</h3>
<p>Larger games write real save files into their data folder: <code>%USERPROFILE%\AppData\LocalLow\&lt;company&gt;\&lt;game&gt;</code> on Windows. What is inside depends on the game:</p>
<ul>
<li><strong>JSON or XML text.</strong> Opens as a list of named values.</li>
<li><strong>.NET BinaryFormatter</strong> (<code>.dat</code>, <code>.sav</code> and similar). Opens as a tree of the game's own objects, with numbers and text editable.</li>
<li><strong>Easy Save 3</strong> (<code>.es3</code>), a very common save plugin. It is JSON, sometimes encrypted with AES and compressed with gzip. The editor tries the plugin's default password by itself; if the game uses its own, enter it in <em>Tools &rsaquo; Decrypt with password</em>.</li>
<li><strong>Anything else.</strong> A file the editor doesn't recognize opens as raw data, where value search can find a number you know.</li>
</ul>
<h3>Things to know</h3>
<ul>
<li>Several games keep both: options in PlayerPrefs and progress in files. Check both places if one doesn't have what you want.</li>
<li>A save with a checksum or hash next to it stops loading after an edit unless the checksum is made again. The editor looks for one near the start or end of the file and writes it again on every save.</li>
<li>Steam Cloud may copy an older save over an edited one the next time the game starts. Edit with the game closed and, if needed, turn cloud sync off for the game first.</li>
</ul>"#,
        faq: &[],
    },
    Guide {
        game: None,
        engine: "Unreal Engine",
        anchor: "unreal-engine-guide",
        slug: "unreal-engine-save-editor",
        title: "Unreal Engine Save Editor | Universal Save Editor",
        desc: "Free, offline save editor for Unreal Engine 4 and 5 games: open the .sav files of games that use SaveGame, change numbers and text, and back up first.",
        lead: "Many games made with Unreal Engine 4 and 5 write their saves as <code>.sav</code> files in the same layout (GVAS). Universal Save Editor opens them, shows every property by name, and writes your changes back. It works offline, backs up every save first, and some of these games have a screen of their own.",
        blurb: "Unreal Engine 4 and 5: the SaveGames folder, GVAS saves and what the editor can change.",
        where_rows: &[
            ("Windows", r"%LOCALAPPDATA%\<project>\Saved\SaveGames"),
            ("Steam on Linux (Proton)", "…/compatdata/<app id>/pfx/drive_c/users/steamuser/AppData/Local/<project>/Saved/SaveGames"),
            ("Some games", r"Steam\userdata\<account>\<app id>\remote"),
        ],
        body: r#"<h2>What an Unreal Engine save is</h2>
<p>When an Unreal Engine game saves with the engine's standard <em>SaveGame</em> system, it writes a <code>.sav</code> file that starts with the letters <code>GVAS</code>. Inside are the game's own variables, each with a name, a type and a value: whole numbers, decimals, true/false flags, text, lists and structures. The names are the ones the game's developers gave their variables, so a coin counter might be called <code>Coins</code>, <code>Money</code> or <code>Currency</code>, and a level <code>PlayerLevel</code>.</p>
<h2>Finding the folder</h2>
<p>The folder is named after the game's internal project name, which is not always the name on the box. Look in <code>%LOCALAPPDATA%</code> for a folder with a <code>Saved\SaveGames</code> folder inside it. The editor's <em>Find my game's saves</em> does this for the games it knows.</p>
<h2>What can be changed</h2>
<ul>
<li><strong>Numbers and text.</strong> Whole numbers, decimals and names are changed in place. The editor also fixes the size fields the format keeps for each property, so the file stays consistent when text gets longer or shorter.</li>
<li><strong>True/false flags.</strong> They open as ticks.</li>
<li><strong>Lists and maps.</strong> Saves written by Unreal Engine 5.4 and newer describe their lists, sets and maps fully, so the entries are shown one by one. Older saves show them as data that cannot be edited yet.</li>
<li><strong>Whatever the editor doesn't understand</strong> is shown read-only, and left exactly as it was.</li>
</ul>
<h2>How to find the value you want</h2>
<ol>
<li>Use the filter box with words like <code>coin</code>, <code>money</code>, <code>gold</code>, <code>xp</code>, <code>level</code>, <code>health</code> or <code>stamina</code>.</li>
<li>Not sure which value it is? Save in the game, note the number you want to change, spend or gain some of it, save again, and use <em>Tools &rsaquo; Compare with another save</em>. It lists exactly what changed.</li>
<li>Many games keep the character in one file and the world in another; the one with the number you saw in the game is the one to open.</li>
</ol>
<h2>Games that wrap the file</h2>
<p>Some games compress or encrypt the file around the GVAS data, or add a checksum (Palworld, Ace Combat 8, Stellar Blade and others). For those the editor removes the wrapper, and puts it back and writes the checksum again on save. For an unknown wrapper, the file opens as raw data with value search instead.</p>
<h2>Things to know</h2>
<ul>
<li>Close the game first, and keep the automatic backup until you've tried the edit in the game.</li>
<li>Games sometimes keep a second copy of a value (a number for the menu, a number for the world). If one doesn't stick, look for another with a similar name.</li>
<li>Don't edit saves of online or multiplayer modes.</li>
</ul>"#,
        faq: &[
            ("Which Unreal Engine games does it work with?", "Any game that saves with the engine's SaveGame system opens as a list of named values. Games with a screen of their own are marked in the game list; the others open in the values editor with a filter and value search."),
            ("How do I know a save is an Unreal Engine save?", "The first four letters of the file are <code>GVAS</code>, and the file ends in <code>.sav</code> inside a <code>Saved\\SaveGames</code> folder. The editor recognizes it by itself when you open it."),
            ("Why doesn't my edit show up in the game?", "Some games keep a second copy of the value, or read it from the cloud save. Close the game, check that the file you edited is the one the game loads, and try a similarly named value."),
        ],
    },
    Guide {
        game: None,
        engine: "Godot",
        anchor: "godot-guide",
        slug: "godot-save-editor",
        title: "Godot Save Editor | Universal Save Editor",
        desc: "Free, offline save editor for Godot games: open JSON, config and binary Variant saves from the game's user folder, change values, and back up first.",
        lead: "Godot games keep their saves in a <code>user://</code> folder, in whatever format the developer picked. Universal Save Editor opens the common ones (JSON, config files, binary Variant files and password-protected files), shows each value by name, and backs up every save first. It works offline.",
        blurb: "Godot: the app_userdata folder, JSON, config and binary saves, and encrypted files.",
        where_rows: &[
            ("Windows", r"%APPDATA%\Godot\app_userdata\<project>"),
            ("Linux", "~/.local/share/godot/app_userdata/<project>"),
            ("macOS", "~/Library/Application Support/Godot/app_userdata/<project>"),
        ],
        body: r#"<h2>Finding the folder</h2>
<p>A Godot game saves to the <code>user://</code> folder. On Windows it is <code>%APPDATA%\Godot\app_userdata\&lt;project name&gt;</code>; Linux and macOS are in the list above. Some games choose their own folder name instead, which is then right beside it under <code>%APPDATA%</code>. The folder holds whatever files the game's developer wrote, so look at the names and the dates to find the one that changes when you save.</p>
<h2>The formats</h2>
<ul>
<li><strong>JSON</strong> (<code>.json</code>, and often <code>.save</code> or <code>.dat</code> files that hold text). The most common choice. Opens as a tree of named values.</li>
<li><strong>Config files</strong> (<code>.cfg</code>, <code>.ini</code>), Godot's <code>ConfigFile</code> format with <code>[sections]</code> and <code>key=value</code> lines. Opens as sections of named values.</li>
<li><strong>Binary Variant files</strong> written with <code>store_var</code> in Godot 4. Opens as a typed tree (numbers, text, lists, dictionaries, vectors). The editor only accepts a file if writing it back unchanged would reproduce it byte for byte; a file that uses a type it doesn't know opens as raw data instead of risking a bad save.</li>
<li><strong>Password-protected files</strong> (<code>open_encrypted_with_pass</code>). They need the game's password: use <em>Tools &rsaquo; Decrypt with password</em>, and the file is encrypted again with the same password on save. The editor cannot guess a password; some games publish theirs, or their community does.</li>
<li><strong>Resource files</strong> (<code>.tres</code>, <code>.res</code>, <code>.scn</code>) are not decoded: they open as text or as raw data, where value search finds a number you know.</li>
</ul>
<h2>What can be changed</h2>
<p>Numbers, text and true/false values in whatever the file holds: coins, health, experience, unlocked levels, settings. A game with a screen of its own in the editor (for example <em>Luck be a Landlord</em>) shows its values under familiar names.</p>
<h2>Things to know</h2>
<ul>
<li>Close the game before you edit, and keep the backup the editor makes.</li>
<li>Godot stores whole numbers and decimals as different types, and a game that expects a whole number can fail on a decimal. The editor keeps the type the file had.</li>
<li>A game's exported data (<code>.pck</code> files) is not a save and isn't touched. Settings that the game keeps in <code>project.godot</code> are not saves either.</li>
<li>Games made with Godot 3 may use a different binary layout. JSON, config and text saves open the same in both.</li>
</ul>"#,
        faq: &[
            ("How do I find a Godot game's save folder?", "Open <code>%APPDATA%\\Godot\\app_userdata</code> (Windows) and look for the folder named after the game. If it isn't there, the game uses its own folder name directly under <code>%APPDATA%</code>. The editor's <em>Find my game's saves</em> lists these for the games it knows."),
            ("My Godot save is unreadable. What now?", "If it looks like random bytes, the game probably encrypts it with a password. Use <em>Tools &rsaquo; Decrypt with password</em> if you have the password; without it the file can't be opened. A text file that starts with <code>{</code> is JSON and opens normally."),
            ("Is it safe for the game's save?", "The editor writes the file back in the same format, checks it by reading it again, and backs up the old one first. A Godot file it cannot reproduce exactly is shown as raw data instead of being rewritten."),
        ],
    },
];

/// The guide carried by a catalog game's page.
pub(super) fn guide_for(game: &str) -> Option<&'static Guide> {
    GUIDES.iter().find(|g| g.game == Some(game))
}

/// The guides that have a page of their own.
pub(super) fn own_pages() -> impl Iterator<Item = &'static Guide> {
    GUIDES.iter().filter(|g| g.game.is_none())
}
