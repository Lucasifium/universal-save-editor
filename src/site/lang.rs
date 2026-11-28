//! The website in the app's other languages: the home page and the pages of the most popular games.
//!
//! Texts are written in English here and passed through `tl`, which looks them up in the
//! language's catalog (`src/site/i18n/<code>.txt`, the same layout as the app's catalogs: the
//! English line, then a line starting with "= " holding the translation, then a blank line).
//! `{0}`, `{1}` stand for names and numbers filled in by `tlf`. A test builds every page in
//! every language and fails when a text has no translation or a translation changes the tags or
//! placeholders of its English text. Game names, folders, file names and the list of what a game
//! screen edits stay as the game and the editor show them (the editor's game screens are English).
//!
//! Every page waits in the page queue under its own name, `<folder>: home page` or
//! `<folder>: <game>`, like a new game page does.

use super::*;
use std::cell::RefCell;
use std::collections::HashMap;

pub(super) struct SiteLang {
    /// The folder on the site: `de`, `pt-br`.
    pub dir: &'static str,
    /// The `lang` of the page (BCP 47): `de`, `pt-BR`.
    pub html: &'static str,
    /// The language's own name.
    pub name: &'static str,
    catalog: &'static str,
}

pub(super) const LANGS: &[SiteLang] = &[
    SiteLang { dir: "de", html: "de", name: "Deutsch", catalog: include_str!("i18n/de.txt") },
    SiteLang { dir: "es", html: "es", name: "Español", catalog: include_str!("i18n/es.txt") },
    SiteLang { dir: "fr", html: "fr", name: "Français", catalog: include_str!("i18n/fr.txt") },
    SiteLang { dir: "pt-br", html: "pt-BR", name: "Português (Brasil)", catalog: include_str!("i18n/pt-BR.txt") },
    SiteLang { dir: "ja", html: "ja", name: "日本語", catalog: include_str!("i18n/ja.txt") },
    SiteLang { dir: "zh-cn", html: "zh-CN", name: "简体中文", catalog: include_str!("i18n/zh-CN.txt") },
];

/// The games whose pages are translated: the pages (without `-save-editor`) of twenty popular
/// games that have a screen of their own.
pub(super) const TOP: &[&str] = &[
    "elden-ring",
    "skyrim",
    "stardew-valley",
    "the-witcher-3",
    "baldurs-gate-3",
    "cyberpunk-2077",
    "fallout-4",
    "dark-souls-iii",
    "sekiro-shadows-die-twice",
    "hades",
    "hollow-knight",
    "terraria-character",
    "minecraft",
    "red-dead-redemption-2",
    "persona-5-royal",
    "valheim",
    "subnautica",
    "rimworld",
    "final-fantasy-vii",
    "stellaris",
];

fn catalog(l: &SiteLang) -> &'static HashMap<String, String> {
    use std::sync::{Mutex, OnceLock};
    static ALL: OnceLock<Mutex<HashMap<&'static str, &'static HashMap<String, String>>>> = OnceLock::new();
    let all = ALL.get_or_init(Default::default);
    let mut all = all.lock().unwrap_or_else(|e| e.into_inner());
    all.entry(l.dir).or_insert_with(|| Box::leak(Box::new(parse(l.catalog).into_iter().collect())))
}

/// (English, translation) pairs of a catalog file.
pub(super) fn parse(text: &str) -> Vec<(String, String)> {
    let mut out = vec![];
    let mut key: Option<&str> = None;
    for line in text.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            key = None;
        } else if let Some(t) = line.strip_prefix("= ") {
            if let Some(k) = key.take() {
                out.push((k.to_string(), t.to_string()));
            }
        } else {
            key = Some(line);
        }
    }
    out
}

thread_local! {
    /// Texts asked for that had no translation, and every text asked for (checked by the tests).
    static MISSING: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static USED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// The translation of an English text; the English text itself when there is none.
fn tl(l: &SiteLang, en: &str) -> String {
    #[cfg(test)]
    USED.with(|u| u.borrow_mut().push(en.to_string()));
    match catalog(l).get(en) {
        Some(t) => t.clone(),
        None => {
            MISSING.with(|m| m.borrow_mut().push(en.to_string()));
            en.to_string()
        }
    }
}

/// `tl`, with `{0}`, `{1}` … replaced by the arguments (already escaped where they go into HTML).
fn tlf(l: &SiteLang, en: &str, args: &[&str]) -> String {
    let mut t = tl(l, en);
    for (i, a) in args.iter().enumerate() {
        t = t.replace(&format!("{{{i}}}"), a);
    }
    t
}

/// The page's name in the page queue.
pub(super) fn queue_name(l: &SiteLang, game: Option<&GameInfo>) -> String {
    match game {
        None => format!("{}: home page", l.dir),
        Some(g) => format!("{}: {}", l.dir, g.name),
    }
}

/// The catalog game a `TOP` entry names.
pub(super) fn top_games() -> Vec<&'static GameInfo> {
    TOP.iter().filter_map(|t| GAMES.iter().find(|g| slug(&search_name(g.name)) == *t)).collect()
}

/// Every translated page's queue name, home pages first, then the games one language after the other.
pub(super) fn all_queue_names() -> Vec<String> {
    let mut v: Vec<String> = LANGS.iter().map(|l| queue_name(l, None)).collect();
    for l in LANGS {
        v.extend(top_games().iter().map(|g| queue_name(l, Some(g))));
    }
    v
}

/// The same page in every language that has it out of the queue, with the English page and the
/// default: `(hreflang, address)`. Empty when no translation of the page is published.
pub(super) fn alternates(skip: &[String], game: Option<&GameInfo>) -> Vec<(String, String)> {
    if game.is_some_and(|g| !top_games().iter().any(|t| t.name == g.name)) {
        return vec![];
    }
    let tail = game.map(|g| format!("{}-save-editor/", slug(&search_name(g.name)))).unwrap_or_default();
    let mut v = vec![];
    for l in LANGS.iter().filter(|l| !skip.contains(&queue_name(l, game))) {
        v.push((l.html.to_string(), format!("{SITE}/{}/{tail}", l.dir)));
    }
    if v.is_empty() {
        return v;
    }
    v.insert(0, ("en".to_string(), format!("{SITE}/{tail}")));
    v.push(("x-default".to_string(), format!("{SITE}/{tail}")));
    v
}

/// The name of a language by its hreflang, for the language links in the footer.
pub(super) fn display_name(hreflang: &str) -> Option<&'static str> {
    if hreflang == "en" {
        return Some("English");
    }
    LANGS.iter().find(|l| l.html == hreflang).map(|l| l.name)
}

fn chrome(l: &SiteLang, depth: usize) -> Chrome {
    let root = "../".repeat(depth);
    let own = if depth == 1 { String::new() } else { "../".repeat(depth - 1) };
    let t = |s: &str| tl(l, s);
    let for_label = t("Download for");
    let for_label = if for_label.ends_with(|c: char| c.is_alphanumeric() || c == ':') { format!("{for_label} ") } else { for_label };
    Chrome {
        html_lang: l.html.to_string(),
        brand_href: if own.is_empty() { "./".into() } else { own.clone() },
        games_href: format!("{}#games", if own.is_empty() { "" } else { own.as_str() }),
        games: t("Games"),
        save_manager: t("Save manager"),
        whats_new: t("What's new"),
        browser: t("Use in browser"),
        download: t("Download"),
        source: t("Source"),
        light: t("Light theme"),
        dark: t("Dark theme"),
        to_light: t("Switch to light theme"),
        to_dark: t("Switch to dark theme"),
        footer: format!(
            r#"{}<br><a href="{DISCUSSIONS}">{}</a> · <a href="{REPORT}">{}</a> · <a href="{root}whats-new/">{}</a> · <a href="{RELEASES}">{}</a> · <a href="{REPO}">{}</a> · <a href="{root}code-signing/">{}</a>"#,
            t("Universal Save Editor is free, and always will be. Open source under the GPL-3.0. Not affiliated with any game developer or publisher; game names and trademarks belong to their owners. Always keep a backup of your saves."),
            t("Questions and ideas"),
            t("Report a problem"),
            t("What's new"),
            t("All versions and release notes"),
            t("Source code on GitHub"),
            t("Code signing policy"),
        ),
        og_alt: tlf(l, "Universal Save Editor: free, offline save editor for {0}+ ({1}) games", &[&many().to_string(), &GAMES.len().to_string()]),
        dl_windows: t("Download for Windows"),
        dl_version: t("Version {0}:"),
        dl_for: for_label,
        dl_all: t("All versions and release notes"),
        root,
    }
}

/// "Windows · Linux · macOS · Browser" with the last word in the language.
fn platforms(l: &SiteLang) -> String {
    format!("Windows · Linux · macOS · {}", tl(l, "Browser"))
}

fn many() -> usize {
    GAMES.len() / 10 * 10
}

/// The `<li>` lines of the short list of reasons (the game pages' "Why use it").
fn why_items() -> Vec<&'static str> {
    let keep = ["Free, forever.", "Offline.", "Safe.", "Works on games it doesn't know.", "Linux and Steam Deck.", "Save manager", "In your language.", "Open source."];
    WHY.lines().filter(|l| keep.iter().any(|k| l.starts_with(&format!("<li><strong>{k}")))).collect()
}

fn why_html(l: &SiteLang) -> String {
    let items: String = why_items().iter().map(|i| format!("{}\n", tl(l, i))).collect();
    format!("<ul>\n{items}</ul>")
}

/// A link to the translated page of a top game, or to the English page while the translation waits in the queue.
fn game_href(l: &SiteLang, skip: &[String], g: &GameInfo, from_game_page: bool) -> String {
    let s = format!("{}-save-editor/", slug(&search_name(g.name)));
    let up = if from_game_page { "../" } else { "" };
    if skip.contains(&queue_name(l, Some(g))) {
        // The English page is in the site's root: out of the language folder, then into it.
        format!("{}../{s}", up)
    } else {
        format!("{up}{s}")
    }
}

fn game_cards(l: &SiteLang, skip: &[String], games: &[&GameInfo], from_game_page: bool) -> String {
    let screen = tl(l, "Game screen");
    games
        .iter()
        .map(|g| format!(r#"<li><a href="{}"><span>{}</span><span class="tag screen">{screen}</span></a></li>"#, game_href(l, skip, g, from_game_page), esc(&search_name(g.name))))
        .collect()
}

/// The home page in a language.
fn home_body(l: &SiteLang, skip: &[String]) -> String {
    let (many, n) = (many().to_string(), GAMES.len().to_string());
    let full = GAMES.iter().filter(|g| has_screen(g)).count().to_string();
    let t = |s: &str| tl(l, s);
    let tf = |s: &str, a: &[&str]| tlf(l, s, a);
    let top = top_games();
    let hero_game = top.first().copied();
    let hero = hero_game.map(|g| {
        let s = slug(&search_name(g.name));
        let h = shot_heights().get(&s).copied().unwrap_or(SHOT_W) * 640 / SHOT_W;
        let name = esc(&search_name(g.name));
        format!(
            r#"<figure class="hero-shot"><div class="frame"><a href="{}" class="slide on"><img src="../shots/{s}-640.webp" srcset="../shots/{s}-640.webp 640w, ../shots/{s}-{SHOT_W}.webp {SHOT_W}w" sizes="(min-width: 1040px) 420px, (min-width: 761px) 42vw, calc(100vw - 40px)" width="640" height="{h}" alt="{}" decoding="async" fetchpriority="high"></a></div><figcaption>{}</figcaption></figure>"#,
            game_href(l, skip, g, false),
            esc(&tf("The {0} screen in Universal Save Editor, with a save open", &[&name])),
            tf("{0} in the editor", &[&name]),
        )
    });
    let questions = [
        ("Is it safe to edit a save file?", "The downloaded app backs up the old file before every save, then reads the new file back and checks it before replacing anything. If the game doesn't like an edit, restore the backup from <em>Backups</em>. In the browser version, keep a copy of the original yourself (in Chrome and Edge, <em>Open to save in place</em> also keeps one in the browser before the first save)."),
        ("Do I have to upload my save?", "No. The editor is a program on your own computer and works offline. <a href=\"{0}\">The browser version</a> also runs on your computer, inside the page, and never uploads the save."),
        ("Which games does it work with?", "{1}+ ({2}) games are <a href=\"{3}\">listed on the English home page</a>, each with a page saying what can be changed and where its saves are. Saves of games that aren't listed often open too: as named values when the format is a common one, or through value search."),
        ("Does it work on Steam Deck, Linux and Mac?", "Yes. There are downloads for Windows, Linux and macOS. On Linux and the Steam Deck, <em>Find my game's saves</em> also looks inside the Proton and Wine prefixes where Steam Play keeps Windows games' saves."),
        ("Can I use edited saves online?", "Edited saves are for single-player play. Don't use them in online or multiplayer modes, where they can break rules or get an account flagged."),
        ("Does it cost anything?", "No. It's free and always will be: no ads, no account and no limits. The code is open source under the GPL-3.0."),
    ];
    let app = "../app/";
    let questions_html: String = questions.iter().map(|(q, a)| format!("<h3>{}</h3>\n<p>{}</p>\n", t(q), tf(a, &[app, &many, &n, "../#games"]))).collect();
    format!(
        r#"<section class="hero"><div><h1>{h1}</h1>
<p class="lead">{lead}</p>
<div class="badges"><span class="badge free">{free}</span><span class="badge">{offline}</span><span class="badge">{open}</span><span class="badge">{platforms}</span></div>
{DOWNLOAD_BUTTON_OPEN}{download}</a><a class="btn alt" href="{app}">{browser}</a></div>{hero}</section>
<section class="feature"><div><h2>{sm_h}</h2><p>{sm_p}</p><a class="btn alt" href="../save-manager/">{sm_b}</a></div></section>
<div class="card">{browser_card}</div>
<h2>{what_h}</h2>
<p>{what_p}</p>
<h2>{how_h}</h2>
<ol>
<li>{how1}</li>
<li>{how2}</li>
<li>{how3}</li>
<li>{how4}</li>
</ol>
<h2>{why_h}</h2>
{why}
<h2 id="questions">{q_h}</h2>
{questions_html}<h2 id="games">{games_h}</h2>
<p>{games_p}</p>
<ul class="grid">{cards}</ul>
<p><a href="../#games">{all_games}</a></p>
<p>{unlisted}</p>
"#,
        h1 = tf("Free offline save editor for {0}+ ({1}) games", &[&many, &n]),
        lead = t("Universal Save Editor is a save editor and save manager for PC games. Open a save file, change money, levels, items and stats, and save it back. It runs on your own computer: no uploads, no accounts, no rate limits, and the downloaded app backs up every save before changing it. It's free, and always will be."),
        free = t("Free forever"),
        offline = t("Offline"),
        open = t("Open source"),
        platforms = platforms(l),
        download = t("Download for free"),
        browser = t("Use it in your browser"),
        hero = hero.unwrap_or_default(),
        sm_h = t("Save manager built in"),
        sm_p = t("Every time you save, the editor backs up the file first, automatically. Make a named backup before a boss or a big choice, restore any backup with one click, compare two of them, or put a save aside to start a new game and come back to it later. Backups stay next to your saves, on your computer."),
        sm_b = t("How the save manager works"),
        browser_card = tf("<strong>Don't want to run a program?</strong> <a href=\"{0}\">The browser version</a> is the same editor, running inside the web page on your own computer: your save is never uploaded. It can't search your folders for saves or back them up for you, so open the file yourself and keep a copy; saving downloads the edited file (Chrome and Edge can also save back to the same file, keeping the original in the browser first). After the first visit it opens without a connection too.", &[app]),
        what_h = t("What is a save editor?"),
        what_p = tf("A save editor opens the file a game writes when you save and lets you change what is stored in it: money, experience, items, stats, unlocks. Games keep their saves in many formats, often compressed, encrypted or guarded by a checksum, so each one needs its own handling. Universal Save Editor does that for {0}+ ({1}) games in one program and writes the file back the way the game expects it.", &[&many, &n]),
        how_h = t("How to edit a save file"),
        how1 = tf("<a href=\"{0}\">Download the editor</a>, or open <a href=\"{1}\">the browser version</a>.", &[DOWNLOAD, app]),
        how2 = t("In the downloaded app, choose <em>Find my game's saves</em> and pick your game. In the browser, open the save file yourself."),
        how3 = tf("Change what you want. {0} games open on a screen made for them, with every value named; other saves open as a list of values, or through value search.", &[&full]),
        how4 = t("Save. The downloaded app backs up the old file first and reads the new one back to check it before replacing anything; the browser version downloads the edited file (or, in Chrome and Edge, can write it back to the same file), so keep a copy of the original. Then load the save in your game."),
        why_h = t("Why download a save editor?"),
        why = why_html(l),
        q_h = t("Questions about save editing"),
        games_h = t("Popular games"),
        games_p = tf("{0} games open on a screen made for them; the rest can be edited value by value. These are some of the most popular. The game pages are in English, apart from the ones that are translated.", &[&full]),
        cards = game_cards(l, skip, &top, false),
        all_games = tf("See all {0} supported games (English)", &[&n]),
        unlisted = tf("No game on the list? The editor can still open saves of games that aren't listed: try it, or <a href=\"{0}\">ask for the game</a>.", &[&format!("{REPO}/issues")]),
    )
}

/// One game's page in a language.
fn game_body(l: &SiteLang, skip: &[String], g: &GameInfo, rows: &[(String, String)]) -> String {
    let t = |s: &str| tl(l, s);
    let tf = |s: &str, a: &[&str]| tlf(l, s, a);
    let name = search_name(g.name);
    let n = esc(&name);
    let what = description(g, rows);
    let s = slug(&name);
    let (many, total) = (many().to_string(), GAMES.len().to_string());
    let untried = what.as_deref().is_some_and(|w| w.contains("not yet tried in the game"));
    let (folder, files) = (esc(g.folder), esc(g.files));
    let faq = [
        (tf("Where is the {0} save file?", &[&n]), tf("Usually in <code>{0}</code>, in files named <code>{1}</code>. The downloaded editor finds it for you: choose <em>Find my game's saves</em> and pick {2}.", &[&folder, &files, &n])),
        (
            tf("Will {0} still load an edited save?", &[&n]),
            if untried {
                t("The editor reads and writes this save correctly on the test saves it was checked with, but loading an edited save in the game itself hasn't been tried yet. The original is backed up before every save, so if the game doesn't accept the edit, restore it from <em>Backups</em>.")
            } else {
                t("The editor writes the save back in the same format the game uses and reads the new file back to check it before replacing anything. The original is backed up first, so if anything looks wrong in the game, restore it from <em>Backups</em>.")
            },
        ),
        (tf("Can I edit {0} saves on Steam Deck or Linux?", &[&n]), t("Yes. There's a Linux build, and <em>Find my game's saves</em> also looks inside the Proton and Wine prefixes where Steam Play keeps Windows games' saves.")),
        (
            tf("Can I edit my {0} save in the browser?", &[&n]),
            tf("Yes. <a href=\"{0}\">The browser version</a> runs on your own computer and never uploads the save. Open the file yourself and keep a copy, because saving downloads the edited file instead of backing up the old one. In Chrome and Edge it can also save back to the same file, keeping the original in the browser first.", &["../../app/"]),
        ),
        (t("Is it safe to use with online play?"), t("Edited saves are for single-player play. Don't use them in online or multiplayer modes, where they can break rules or get an account flagged.")),
        (t("Does it cost anything?"), t("No. It's free and always will be: no ads, no account and no limits. The code is open source under the GPL-3.0.")),
    ];
    let faq_html: String = faq.iter().map(|(q, a)| format!("<h3>{q}</h3>\n<p>{a}</p>\n")).collect();
    let shot = {
        let shows = String::new();
        let alt = tf("{0} save editor screen with a save open", &[&n]) + &shows;
        shot_html_with("../../", &s, &alt, &tf("The {0} screen in Universal Save Editor, opened on an example save. Select the picture to see it full size.", &[&n]))
    };
    let confirmed = confirmed_in_game(g.name);
    let top = top_games();
    let at = top.iter().position(|t| t.name == g.name).unwrap_or(0);
    let more: Vec<&GameInfo> = (1..=6).map(|k| top[(at + k) % top.len()]).collect();
    format!(
        r#"<section class="hero game"><div><h1>{h1}</h1>
<p class="lead">{lead}</p>
<div class="badges"><span class="badge free">{free}</span><span class="badge">{offline}</span><span class="badge">{open}</span><span class="badge">{screen}</span>{confirmed_badge}</div>
{DOWNLOAD_BUTTON_OPEN}{download}</a><a class="btn alt" href="../../app/">{browser}</a></div></section>
<h2>{what_h}</h2>
<div class="card what">{what}<p>{what_note}</p></div>
{shot}<h2>{where_h}</h2>
<div class="card"><p>{folder_l} <code>{folder}</code></p><p>{files_l} <code>{files}</code></p><p>{find}</p></div>
<h2>{how_h}</h2>
<ol>
<li>{how1}</li>
<li>{how2}</li>
<li>{how3}</li>
<li>{how4}</li>
</ol>
<h2>{faq_h}</h2>
{faq_html}<h2>{did_h}</h2>
<div class="card">{did_ok}<p>{did_p}</p></div>
<h2>{why_h}</h2>
{why}
<p>{online}</p>
<h2>{more_h}</h2>
<ul class="grid more">{more}</ul>
<p><a href="../../{s}-save-editor/">{english_page}</a> · <a href="../../#games">{all_games}</a></p>
"#,
        h1 = tf("{0} save editor", &[&n]),
        lead = tf("Edit your {0} save on your own PC with Universal Save Editor, a free, offline save editor and manager for {1}+ ({2}) games. Back up, restore and start new runs from the same window. Nothing is uploaded and there are no limits.", &[&n, &many, &total]),
        free = t("Free forever"),
        offline = t("Offline"),
        open = t("Open source"),
        screen = t("Game screen"),
        confirmed_badge = if confirmed { format!(r#"<span class="badge ok">{}</span>"#, t("Confirmed in game")) } else { String::new() },
        download = t("Download for free"),
        browser = t("Use it in your browser"),
        what_h = t("What you can edit"),
        what = what.as_deref().map(what_html).unwrap_or_default(),
        what_note = t("The editor shows these value names in English."),
        where_h = tf("Where {0} keeps its saves", &[&n]),
        folder_l = t("Folder:"),
        files_l = t("Files:"),
        find = tf("You don't need to find it yourself: choose <em>Find my game's saves</em> in the editor, pick {0}, and it lists the saves it finds. (The browser version can't look through your folders: open the file from this folder yourself.)", &[&n]),
        how_h = tf("How to edit your {0} save", &[&n]),
        how1 = tf("<a href=\"{0}\">Download Universal Save Editor</a> for Windows, macOS or Linux; just run it. Rather not install anything? <a href=\"{1}\">Use it in your browser</a> instead: it runs on your computer too, and saving downloads the edited file (Chrome and Edge can save back to the same file).", &["#download", "../../app/"]),
        how2 = t("Close the game, so it doesn't write over your changes."),
        how3 = tf("Choose <em>Find my game's saves</em>, pick <strong>{0}</strong> and open your save (or drag the save file onto the window).", &[&n]),
        how4 = t("Change what you want, then <em>Save</em>. The editor checks the new file and backs up the old one first."),
        faq_h = tf("Questions about {0} saves", &[&n]),
        did_h = t("Did it work in your game?"),
        did_ok = if confirmed { format!("<p>{}</p>", t("Edited saves of this game have been reported to load and work in the game.")) } else { String::new() },
        did_p = tf("Tried an edit in {0}? <a href=\"{1}\">Tell us whether it worked</a>. It takes a minute, and no save is sent.", &[&n, &format!("{IN_GAME_REPORT}&amp;game={}", url_encode(g.name))]),
        why_h = t("Why use it"),
        why = why_html(l),
        online = t("Edited saves are for single-player play. Don't use them in online or multiplayer modes."),
        more_h = t("More save editors"),
        more = game_cards(l, skip, &more, true),
        english_page = t("This page in English"),
        all_games = tf("See all {0} supported games (English)", &[&total]),
    )
}

/// The translated pages that are out of the queue, as (path, full HTML). `skip` is the page queue.
pub(super) fn pages(skip: &[String], rows: &[(String, String)]) -> Vec<(String, String)> {
    let mut out = vec![];
    for l in LANGS {
        if !skip.contains(&queue_name(l, None)) {
            let path = format!("{}/", l.dir);
            let desc = tlf(l, "Free offline save editor for PC games: change money, items and stats in the saves of {0}+ ({1}) games. Nothing is uploaded, and every save is backed up first.", &[&many().to_string(), &GAMES.len().to_string()]);
            let title = tlf(l, "Free offline save editor for {0}+ ({1}) games | Universal Save Editor", &[&many().to_string(), &GAMES.len().to_string()]);
            let body = home_body(l, skip);
            let alts = alternates(skip, None);
            let sch = website_schema(&schema("Universal Save Editor", &desc, &format!("{SITE}/{path}")), None);
            out.push((path.clone(), page_in(&chrome(l, 1), &title, &desc, &path, &body, &sch, &alts)));
        }
        for g in top_games().into_iter().filter(|g| !skip.contains(&queue_name(l, Some(g)))) {
            let search = search_name(g.name);
            let path = format!("{}/{}-save-editor/", l.dir, slug(&search));
            let desc = tlf(l, "Free, offline {0} save editor for PC. Change your values, with a backup before every save. Nothing is uploaded, no limits.", &[&search]);
            let title = tlf(l, "{0} save editor: free and offline | Universal Save Editor", &[&search]);
            let body = game_body(l, skip, g, rows);
            let alts = alternates(skip, Some(g));
            let url = format!("{SITE}/{path}");
            let crumb = tlf(l, "{0} save editor", &[&search]);
            let sch = website_schema(&schema(&format!("Universal Save Editor ({search})"), &desc, &url), Some((&crumb, &url)));
            out.push((path.clone(), page_in(&chrome(l, 2), &title, &desc, &path, &body, &sch, &alts)));
        }
    }
    out
}

/// Every text the pages ask for that a catalog lacks, for each language (tests).
#[cfg(test)]
pub(super) fn take_missing() -> Vec<String> {
    MISSING.with(|m| std::mem::take(&mut *m.borrow_mut()))
}

/// Every text the pages asked for since the last call (tests).
#[cfg(test)]
pub(super) fn take_used() -> Vec<String> {
    USED.with(|u| std::mem::take(&mut *u.borrow_mut()))
}

#[cfg(test)]
pub(super) fn catalog_of(l: &SiteLang) -> &'static HashMap<String, String> {
    catalog(l)
}
