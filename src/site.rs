//! Builds the project website in `docs/` (served by GitHub Pages): a home page and one page per
//! game in the catalog, from the catalog itself and the README's "Game screens" table, plus a
//! sitemap. Run after adding games:
//!
//!     cargo test --release --lib build_site -- --ignored

use crate::games::{GameInfo, GAMES};
use std::fmt::Write as _;

const SITE: &str = "https://lucasifium.github.io/universal-save-editor";
const REPO: &str = "https://github.com/Lucasifium/universal-save-editor";
const DOWNLOAD: &str = "https://github.com/Lucasifium/universal-save-editor/releases/latest";
/// The browser version (built by `web/build.sh` into `docs/app/`, which `build_site` keeps).
const WEB_APP: &str = "app/";

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// README cell text (a little Markdown) to HTML.
fn md(s: &str) -> String {
    let mut out = String::new();
    let (mut em, mut code) = (false, false);
    for c in esc(s).chars() {
        match c {
            '`' => {
                out.push_str(if code { "</code>" } else { "<code>" });
                code = !code;
            }
            '*' if !code => {
                out.push_str(if em { "</em>" } else { "<em>" });
                em = !em;
            }
            c => out.push(c),
        }
    }
    out
}

fn slug(name: &str) -> String {
    let mut s = String::new();
    for c in name.to_lowercase().replace(['\'', '\u{2019}'], "").replace('ö', "o").replace('é', "e").replace('&', " and ").chars() {
        if c.is_ascii_alphanumeric() {
            s.push(c);
        } else if !s.ends_with('-') && !s.is_empty() {
            s.push('-');
        }
    }
    s.trim_end_matches('-').to_string()
}

/// The words people type: "Cairn save editor"; a few catalog names read better shortened.
fn search_name(name: &str) -> String {
    if let Some(me) = name.strip_prefix("Mass Effect Legendary Edition (ME").and_then(|r| r.strip_suffix(')')) {
        return format!("Mass Effect {me} Legendary Edition");
    }
    let n = name.split(" (").next().unwrap_or(name);
    match n {
        "Terraria" if name.contains("worlds") => "Terraria world".into(),
        "Terraria" => "Terraria character".into(),
        _ => n.to_string(),
    }
}

/// Game-screen rows from the README: (names in the first cell, description).
fn readme_rows() -> Vec<(String, String)> {
    let readme = include_str!("../README.md");
    let table = readme.split("## Game screens").nth(1).unwrap_or("").split("\n## ").next().unwrap_or("");
    table
        .lines()
        .filter(|l| l.starts_with("| ") && !l.starts_with("| Game |"))
        .filter_map(|l| {
            let cells: Vec<&str> = l.trim_matches('|').split(" | ").collect();
            (cells.len() >= 2).then(|| (cells[0].trim().to_string(), cells[1].trim().trim_end_matches('|').trim().to_string()))
        })
        .collect()
}

fn norm(s: &str) -> String {
    s.to_lowercase().replace('ö', "o").chars().filter(|c| c.is_alphanumeric() || *c == ' ').collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Games that open on a screen of their own ("Game screen", or "Game screen: what it shows").
fn has_screen(g: &GameInfo) -> bool {
    g.support.starts_with("Game screen")
}

/// What the page says can be edited: the README row for this game, or else (for games whose
/// catalog text says it) the catalog text.
fn description(g: &GameInfo, rows: &[(String, String)]) -> Option<String> {
    // The README points to its own sections ("see below"); a game page has none.
    let row = readme_row(g, rows).map(|(s, d)| (s, d.replace(", see below", "")));
    if g.support == "Game screen" {
        return row.map(|r| r.1);
    }
    // Catalog text that describes the screen is more exact than a loosely matched row.
    if let Some((_, r)) = row.filter(|r| r.0 >= 90) {
        return Some(r);
    }
    has_screen(g).then(|| {
        let t = g.support.trim_start_matches("Game screen").trim_start_matches([':', ' ']);
        let mut c = t.chars();
        c.next().map(|f| f.to_uppercase().chain(c).collect::<String>() + ".").unwrap_or_default()
    })
}

/// The README row describing this catalog game, if any.
fn readme_row(g: &GameInfo, rows: &[(String, String)]) -> Option<(usize, String)> {
    // The table describes the game screens; other games get the general note.
    if !has_screen(g) {
        return None;
    }
    // Games the table lists under another name.
    let alias = match g.name {
        n if n.starts_with("Dark Souls") => Some("Elden Ring, Dark Souls"),
        n if n.starts_with("Enderal") => Some("Skyrim, Fallout 4"),
        "Final Fantasy X / X-2 HD Remaster" => Some("Final Fantasy X (HD Remaster"),
        n if n.starts_with("Mass Effect Legendary Edition (ME2") || n.starts_with("Mass Effect Legendary Edition (ME3") => Some("Mass Effect 2, Mass Effect 3"),
        "Yakuza: Like a Dragon" => Some("Yakuza Kiwami 2, Like a Dragon"),
        n if n.starts_with("Metro") => Some("Metro 2033"),
        "Mass Effect (2008)" | "Mass Effect Legendary Edition (ME1)" => Some("Mass Effect (2008)"),
        n if n.starts_with("Diablo II") => Some("Diablo II"),
        n if n.starts_with("Pathfinder") => Some("Pathfinder"),
        "Yakuza Kiwami 3 & Dark Ties" => Some("Yakuza 6, Kiwami 3"),
        _ => None,
    };
    if let Some(a) = alias {
        return rows.iter().find(|(g, _)| g.starts_with(a)).map(|(_, d)| (100, d.clone()));
    }
    let name = norm(g.name);
    let short = norm(g.name.split(" (").next().unwrap_or(g.name));
    let mut best: Option<(usize, &String)> = None;
    for (games, desc) in rows {
        let cell = norm(games);
        // Pieces of the first cell: "Elden Ring, Dark Souls III / II / I" -> each listed game.
        let pieces: Vec<String> = games.split([',', '/']).map(norm).filter(|p| !p.is_empty()).collect();
        let score = if cell == name || cell == short {
            100
        } else if pieces.iter().any(|p| *p == short || *p == name) {
            90
        } else if pieces.iter().any(|p| p.len() > 4 && (short.starts_with(p.as_str()) || p.starts_with(short.as_str()))) {
            60
        } else if short.len() > 5 && cell.contains(&short) {
            50
        } else {
            0
        };
        if score > 0 && best.is_none_or(|(s, _)| score > s) {
            best = Some((score, desc));
        }
    }
    best.map(|(s, d)| (s, d.clone()))
}

/// Games whose saves need the downloaded app.
fn needs_app(g: &GameInfo) -> bool {
    matches!(g.name, "Palworld" | "Hogwarts Legacy")
}

/// One short line for a game card: what the page says can be edited, without asides
/// (parentheses) or notes on how the file is unpacked and checked.
fn card_blurb(what: &str) -> String {
    let mut plain = String::new();
    let mut depth = 0;
    for c in what.replace(['*', '`'], "").chars() {
        match c {
            '(' => depth += 1,
            ')' => depth = (depth - 1).max(0),
            c if depth == 0 => plain.push(c),
            _ => {}
        }
    }
    let plain = plain.replace("  ", " ").replace(" ,", ",").replace(" ;", ";").replace(" :", ":").replace(", changed in place", "").replace(" changed in place", "");
    let mut w = plain.split(". ").next().unwrap_or("").trim();
    // "Kiryu & Majima: money…" -> "money…"; "A map editor: pan…" keeps its label.
    if let Some((label, rest)) = w.split_once(": ") {
        if label.chars().count() < 30 && !label.contains(',') && !label.starts_with("A ") {
            w = rest;
        }
    }
    let technical = |p: &str| {
        let l = p.to_lowercase();
        ["decrypted", "unpacked", "packed again", "encrypted", "checksum", "scrambled", "aren't mapped", "not yet", "can't be"].iter().any(|t| l.contains(t))
    };
    let parts: Vec<&str> = w.split("; ").map(str::trim).filter(|p| !p.is_empty() && !technical(p)).collect();
    let w = parts.join(", ");
    let w = w.trim_end_matches(['.', ',', ' ', '\u{2026}']);
    const MAX: usize = 64;
    let mut out = if w.chars().count() <= MAX {
        w.to_string()
    } else {
        let cut: String = w.chars().take(MAX).collect();
        let at = cut.rfind([',', ' ']).unwrap_or(cut.len());
        let cut = cut[..at].trim_end_matches([',', ' ', '/']);
        let cut = cut.strip_suffix(" and").or(cut.strip_suffix(" the")).unwrap_or(cut);
        format!("{}\u{2026}", cut.trim_end_matches([',', ' ']))
    };
    if let Some(f) = out.chars().next() {
        out = f.to_uppercase().chain(out.chars().skip(1)).collect();
    }
    out
}

fn level(g: &GameInfo) -> (&'static str, &'static str) {
    if has_screen(g) {
        ("Game screen", "A screen laid out for this game, with names for its items and values.")
    } else if g.support.starts_with("Binary saves") {
        // Paradox games: the catalog text says what is and isn't named.
        ("Values editor", g.support)
    } else if g.support.starts_with("Hex") || g.support.starts_with("Unpacked") {
        ("Value search", "Find a number you know (money, ammo, experience), or one you don't by how it changes between saves, and change it. The save is shown as raw data: there is no screen for this game yet.")
    } else {
        ("Values editor", "Every value in the save, by name, with a filter. There is no screen laid out for this game yet.")
    }
}

const STYLE: &str = r#":root{--bg:#0f1217;--card:#181d25;--line:#2a313c;--text:#e8ecf2;--muted:#9aa5b4;--accent:#6fbf5c;--accent2:#4aa3df}
@media (prefers-color-scheme: light){:root{--bg:#f6f7f9;--card:#fff;--line:#dde2e8;--text:#14181e;--muted:#5b6573;--accent:#2f8a1f;--accent2:#1f6fb0}}
*{box-sizing:border-box}body{margin:0;background:var(--bg);color:var(--text);font:16px/1.6 system-ui,-apple-system,"Segoe UI",Roboto,sans-serif}
a{color:var(--accent2)}main,header .in,footer .in{max-width:960px;margin:0 auto;padding:0 16px}
header{border-bottom:1px solid var(--line)}header .in{display:flex;align-items:center;gap:16px;min-height:56px;flex-wrap:wrap}
header .brand{font-weight:700;color:var(--text);text-decoration:none}header nav{margin-left:auto;display:flex;gap:16px}
h1{font-size:2.1rem;line-height:1.2;margin:32px 0 8px}h2{margin-top:36px}
.lead{font-size:1.15rem;color:var(--muted);max-width:720px}
.badges{display:flex;gap:8px;flex-wrap:wrap;margin:16px 0}.badge{border:1px solid var(--line);border-radius:999px;padding:2px 12px;font-size:.9rem;color:var(--muted)}
.badge.free{border-color:var(--accent);color:var(--accent);font-weight:600}
.btn{display:inline-block;background:var(--accent);color:#fff;text-decoration:none;font-weight:700;padding:12px 22px;border-radius:10px;margin:8px 8px 8px 0}
.btn.alt{background:transparent;color:var(--text);border:1px solid var(--line)}
.card{background:var(--card);border:1px solid var(--line);border-radius:12px;padding:16px 20px;margin:16px 0}
code{background:var(--card);border:1px solid var(--line);border-radius:6px;padding:1px 6px;font-size:.9em;word-break:break-all}
.grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(250px,1fr));gap:10px;padding:0;list-style:none}
.grid li{margin:0;display:flex}
.grid a{flex:1;display:flex;flex-direction:column;gap:2px;background:var(--card);border:1px solid var(--line);border-radius:10px;padding:10px 14px;color:var(--text);text-decoration:none;line-height:1.35}
.grid a:hover,.grid a:focus-visible{border-color:var(--accent2)}.grid small{display:block;color:var(--muted);font-size:.85rem}
.grid .tag{align-self:flex-start;margin-top:auto;padding-top:4px;font-size:.75rem;color:var(--muted)}.grid .tag.screen{color:var(--accent)}
.empty{color:var(--muted);display:none}
details#all{margin:12px 0}details#all>summary{cursor:pointer;display:inline-block;padding:10px 16px;border:1px solid var(--line);border-radius:10px;background:var(--card);font-weight:600;list-style:none}
details#all>summary::-webkit-details-marker{display:none}details#all>summary::before{content:"\25B8";display:inline-block;margin-right:8px;transition:transform .15s}
details#all[open]>summary::before{transform:rotate(90deg)}details#all>summary:hover{border-color:var(--accent2)}
#q{width:100%;max-width:420px;padding:10px 14px;border-radius:10px;border:1px solid var(--line);background:var(--card);color:var(--text);font-size:1rem}
ol li,ul li{margin:4px 0}@media (max-width:600px){header nav{margin-left:0;gap:14px;flex-wrap:wrap}header .in{padding-top:10px;padding-bottom:10px;gap:6px}}footer{border-top:1px solid var(--line);margin-top:48px;color:var(--muted);font-size:.9rem}footer .in{padding:20px 16px}
"#;

fn page(title: &str, desc: &str, path: &str, body: &str, schema: &str) -> String {
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{t}</title>
<meta name="description" content="{d}">
<link rel="canonical" href="{SITE}/{path}">
<meta property="og:type" content="website">
<meta property="og:site_name" content="Universal Save Editor">
<meta property="og:title" content="{t}">
<meta property="og:description" content="{d}">
<meta property="og:url" content="{SITE}/{path}">
<meta name="twitter:card" content="summary">
<link rel="stylesheet" href="{root}style.css">
<script type="application/ld+json">{schema}</script>
</head>
<body>
<header><div class="in"><a class="brand" href="{root}">Universal Save Editor</a><nav><a href="{root}#games">Games</a><a href="{root}{WEB_APP}">Use in browser</a><a href="{DOWNLOAD}">Download</a><a href="{REPO}">Source</a></nav></div></header>
<main>
{body}
</main>
<footer><div class="in">Universal Save Editor is free, and always will be. Open source under the GPL-3.0. Not affiliated with any game developer or publisher; game names and trademarks belong to their owners. Always keep a backup of your saves.</div></footer>
</body>
</html>
"#,
        t = esc(title),
        d = esc(desc),
        root = if path.is_empty() { "./" } else { "../" },
    )
}

fn schema(name: &str, desc: &str, url: &str) -> String {
    serde_json::json!({
        "@context": "https://schema.org",
        "@type": "SoftwareApplication",
        "name": name,
        "description": desc,
        "url": url,
        "applicationCategory": "UtilitiesApplication",
        "operatingSystem": "Windows, Linux, web browser",
        "offers": {"@type": "Offer", "price": "0", "priceCurrency": "USD"},
        "license": "https://www.gnu.org/licenses/gpl-3.0.html",
        "downloadUrl": DOWNLOAD,
    })
    .to_string()
    .replace("</", "<\\/")
}

/// Home page only: tells Google the site's name (used for the name shown above search results).
fn website_schema(app: &str, crumb: Option<(&str, &str)>) -> String {
    let mut app: serde_json::Value = serde_json::from_str(app).unwrap();
    app.as_object_mut().unwrap().remove("@context");
    app["isPartOf"] = serde_json::json!({"@id": format!("{SITE}/#website")});
    let mut graph = vec![serde_json::json!({
                "@type": "WebSite",
                "@id": format!("{SITE}/#website"),
                "url": format!("{SITE}/"),
                "name": "Universal Save Editor",
                "alternateName": ["Universal Save Editor (offline)", "Offline Universal Save Editor"],
                "inLanguage": "en"
            }), app];
    if let Some((name, url)) = crumb {
        graph.push(serde_json::json!({
            "@type": "BreadcrumbList",
            "itemListElement": [
                {"@type": "ListItem", "position": 1, "name": "Universal Save Editor", "item": format!("{SITE}/")},
                {"@type": "ListItem", "position": 2, "name": name, "item": url}
            ]
        }));
    }
    serde_json::json!({"@context": "https://schema.org", "@graph": graph})
    .to_string()
    .replace("</", "<\\/")
}

const WHY: &str = r#"<ul>
<li><strong>Free, forever.</strong> No price, no ads, no account, no paid tier.</li>
<li><strong>Offline.</strong> Your save never leaves your computer: nothing is uploaded, so there are no file size limits, queues or rate limits.</li>
<li><strong>Safe.</strong> Every save is checked by reading it back before it's written, and the old file is backed up first (File &rsaquo; Backup &amp; restore saves).</li>
<li><strong>Works on games it doesn't know.</strong> Value search finds a number you know, or one you don't (a health bar, a hidden counter): scan, play, save, then keep what increased, decreased, changed or stayed the same.</li>
<li><strong>Linux and Steam Deck.</strong> <em>Select game</em> also looks inside Proton and Wine prefixes, where Windows games run through Steam Play keep their saves.</li>
<li><strong>Xbox app and PC Game Pass.</strong> Saves hidden in the Xbox app's container are listed under the names the game gave them and open with the same screens as the Steam saves.</li>
<li><strong>Unity PlayerPrefs.</strong> Coins and unlocks that Unity games keep in the Windows registry (or prefs files on Linux and macOS) open by name and are written back on save.</li>
<li><strong>Flash and Haxe saves.</strong> Flash and AIR <code>.sol</code> files and the Haxe saves of OpenFL and HaxeFlixel games (Friday Night Funkin' among them) open with every value by name.</li>
<li><strong>Emulator memory cards.</strong> PlayStation and PlayStation 2 memory card images open one save at a time and are written back into the card, with the PS2 card's error-correction codes made again. Single saves exported by cheat devices and PC tools (.mcs, .psu, .max, .cbs, .xps and PS3 .psv) open too and are packed back in their own format.</li>
<li><strong>Checksum finder.</strong> For a save it doesn't know, the editor looks for a CRC, sum or hash near the start or end of the file and writes it again every time you save.</li>
<li><strong>Your own templates.</strong> For a game it doesn't know, write a small JSON file saying where each value is; matching saves then open as a list of those values, with the checksum kept right. Templates are easy to share.</li>
<li><strong>Console cheat patches.</strong> About 2,400 PlayStation 1 to 4, PSP, Vita and Dreamcast games have ready-made cheats from the Apollo patch collection built in: pick the codes and apply them. Encrypted saves from about 80 PS3, PS4, PSP and Vita games open already decrypted, and are encrypted again on save.</li>
<li><strong>Dreamcast saves.</strong> VMU <code>.vms</code> files and Nexus <code>.dci</code> files open with their header CRC and byte order handled, and are written back the same way.</li>
<li><strong>Sega Saturn saves.</strong> Single <code>.bup</code>, <code>.sro</code>, <code>.xml</code>, <code>.b64</code> and <code>.cms</code> saves, and whole backup RAM images from Mednafen, Beetle, Yabause, Kronos and SAROO, one save at a time.</li>
<li><strong>Steam account switch.</strong> Finds the Steam IDs inside a save and changes them to another account, for saves copied between accounts or co-op hosts.</li>
<li><strong>Backup &amp; restore saves.</strong> Automatic and named backups kept in a folder next to each save, where no game loads them. Restore any backup (the current save is backed up first), compare it, or back up and start a new game on a one-save game and restore the old one later.</li>
<li><strong>Command line too.</strong> <code>info</code>, <code>list</code>, <code>get</code>, <code>set</code>, <code>decode</code> and <code>encode</code> work from scripts, with the same checks and backups as the window.</li>
<li><strong>Open source.</strong> Anyone can read the code (GPL-3.0).</li>
</ul>"#;

/// Rewrites "N+ (M) PC games" (or "N+ PC games") to the current registry numbers.
fn with_game_count(text: &str) -> String {
    const TAIL: &str = " PC games";
    let Some(i) = text.find(TAIL) else { return text.to_string() };
    let head = &text[..i];
    let start = head.rfind(|c: char| !(c.is_ascii_digit() || "+ ()".contains(c))).map(|k| k + 1).unwrap_or(0);
    let start = start + head[start..].len() - head[start..].trim_start_matches(' ').len();
    format!("{}{}+ ({}){}", &text[..start], GAMES.len() / 10 * 10, GAMES.len(), &text[i..])
}

#[test]
#[ignore]
fn build_site() {
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("docs");
    // Keep the bracketed game count in the README and Cargo.toml current.
    for f in ["README.md", "Cargo.toml"] {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(f);
        let text = std::fs::read_to_string(&path).unwrap();
        let fixed = with_game_count(&text);
        if fixed != text {
            std::fs::write(&path, fixed).unwrap();
        }
    }
    // Start over, except for the browser version, which web/build.sh builds separately.
    for e in std::fs::read_dir(&out).into_iter().flatten().flatten() {
        if e.file_name() != "app" {
            let p = e.path();
            let _ = if p.is_dir() { std::fs::remove_dir_all(&p) } else { std::fs::remove_file(&p) };
        }
    }
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("style.css"), STYLE).unwrap();
    std::fs::write(out.join(".nojekyll"), "").unwrap();
    // Google Search Console's proof that this site belongs to the project's owner.
    std::fs::write(out.join("google9d120e6c8e9d3fa5.html"), "google-site-verification: google9d120e6c8e9d3fa5.html").unwrap();
    let rows = readme_rows();
    let mut urls = vec![format!("{SITE}/"), format!("{SITE}/{WEB_APP}")];
    let mut cards = String::new();
    let mut unmatched = vec![];
    // SITE_SKIP="Game A|Game B" leaves those games off the site for now.
    let skip: Vec<String> = std::env::var("SITE_SKIP").unwrap_or_default().split('|').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    let mut games: Vec<&GameInfo> = GAMES.iter().filter(|g| !skip.iter().any(|s| s == g.name)).collect();
    games.sort_by_key(|g| (!has_screen(g), g.name.to_lowercase()));

    for g in &games {
        let s = format!("{}-save-editor", slug(&search_name(g.name)));
        let search = search_name(g.name);
        let (lvl, lvl_note) = level(g);
        let what = description(g, &rows);
        if what.is_none() && g.support == "Game screen" {
            unmatched.push(g.name);
        }
        // For search results: the description without its tab label, short enough for a snippet.
        let clean_what = what.as_deref().map(|w| {
            let w = w.replace(['*', '`'], "");
            let w = match w.split_once(": ") {
                Some((label, rest)) if label.len() < 20 && !label.contains(' ') => rest.to_string(),
                _ => w,
            };
            w.replace("; ", ", ")
        });
        let build_desc = |limit: usize, tail: bool| -> String {
            let tail = if tail { " Nothing is uploaded, no limits." } else { "" };
            let Some(w) = clean_what.as_deref() else {
                return format!("Free, offline {search} save editor for PC. {lvl_note}{tail}");
            };
            let short = if w.chars().count() <= limit {
                w.trim_end_matches('.').to_string()
            } else {
                let cut: String = w.chars().take(limit).collect();
                let end = cut.rfind([',', ' ']).unwrap_or(cut.len());
                format!("{}\u{2026}", cut[..end].trim_end_matches([',', ' ']))
            };
            let first = short.chars().next().map_or(String::new(), |c| c.to_lowercase().collect());
            let dot = if short.ends_with('\u{2026}') { "" } else { "." };
            format!("Free, offline {search} save editor for PC. Edit {first}{}{dot}{tail}", short.chars().skip(1).collect::<String>())
        };
        // Search engines cut descriptions at about 160 characters: keep the game name and what
        // can be edited, and drop the boilerplate first.
        let desc = [(120, true), (120, false), (80, false), (50, false)]
            .iter()
            .map(|&(l, t)| build_desc(l, t))
            .find(|d| d.chars().count() <= 160)
            .unwrap_or_else(|| build_desc(50, false));
        // Titles are cut at about 60 characters: the game name always comes first.
        let title = [
            format!("{search} Save Editor: free and offline | Universal Save Editor"),
            format!("{search} Save Editor | Universal Save Editor"),
            format!("{search} Save Editor: free and offline"),
        ]
        .into_iter()
        .find(|t| t.chars().count() <= 62)
        .unwrap_or_else(|| format!("{search} Save Editor"));
        let mut body = String::new();
        let _ = write!(
            body,
            r#"<h1>{h} save editor</h1>
<p class="lead">Edit your {n} save on your own PC with Universal Save Editor, a free, offline save editor for {many}+ ({total}) games. Nothing is uploaded and there are no limits.</p>
<div class="badges"><span class="badge free">Free forever</span><span class="badge">Offline</span><span class="badge">Open source</span><span class="badge">{lvl}</span></div>
<a class="btn" href="{DOWNLOAD}">Download for free</a><a class="btn alt" href="../{WEB_APP}">Use it in your browser</a>
<h2>What you can edit</h2>
<div class="card">{what}</div>
<h2>Where {n} keeps its saves</h2>
<div class="card"><p>Folder: <code>{folder}</code></p><p>Files: <code>{files}</code></p><p>You don't need to find it yourself: choose <em>Select game</em> in the editor, pick {n}, and it lists the saves it finds. (The browser version can't look through your folders: open the file from this folder yourself.)</p>{browser}</div>
<h2>How to edit your {n} save</h2>
<ol>
<li><a href="{DOWNLOAD}">Download Universal Save Editor</a> for Windows or Linux; just run it. Rather not install anything? <a href="../{WEB_APP}">Use it in your browser</a> instead: it runs on your computer too, and saving downloads the edited file.</li>
<li>Close the game, so it doesn't write over your changes.</li>
<li>Choose <em>Select game</em>, pick <strong>{n}</strong> and open your save (or drag the save file onto the window).</li>
<li>Change what you want, then <em>Save</em>. The editor checks the new file and backs up the old one first.</li>
</ol>
<h2>Why use it</h2>
{WHY}
<p>Edited saves are for single-player play. Don't use them in online or multiplayer modes.</p>
"#,
            h = esc(&search),
            many = GAMES.len() / 10 * 10,
            total = GAMES.len(),
            n = esc(g.name),
            what = what.as_deref().map(md).unwrap_or_else(|| esc(lvl_note)),
            folder = esc(g.folder),
            browser = if needs_app(g) { "<p><strong>Use the downloaded app for this game:</strong> the browser version can't open its saves.</p>" } else { "" },
            files = esc(g.files),
        );
        let path = format!("{s}/");
        let dir = out.join(&s);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("index.html"), page(&title, &desc, &path, &body, &website_schema(&schema(&format!("Universal Save Editor ({search} save editor)"), &desc, &format!("{SITE}/{path}")), Some((&format!("{search} Save Editor"), &format!("{SITE}/{path}")))))).unwrap();
        urls.push(format!("{SITE}/{path}"));
        let blurb = what.as_deref().map(card_blurb).unwrap_or_default();
        let tag_class = if has_screen(g) { "tag screen" } else { "tag" };
        let blurb_html = if blurb.is_empty() { String::new() } else { format!("<small>{}</small>", esc(&blurb)) };
        let _ = write!(cards, r#"<li><a href="{s}/"><span>{}</span>{blurb_html}<span class="{tag_class}">{lvl}</span></a></li>"#, esc(g.name));
    }

    let full = GAMES.iter().filter(|g| has_screen(g)).count();
    let desc = format!("Universal Save Editor is a free, offline save editor for {}+ ({}) PC games, including Elden Ring, Skyrim, Stardew Valley, Cyberpunk 2077 and Dark Souls III. Nothing is uploaded.", GAMES.len() / 10 * 10, GAMES.len());
    let body = format!(
        r#"<h1>Universal Save Editor: a free save editor for {n} games</h1>
<p class="lead">Universal Save Editor edits your saves on your own computer: no uploads, no accounts, no rate limits. It's free, and always will be.</p>
<div class="badges"><span class="badge free">Free forever</span><span class="badge">Offline</span><span class="badge">Open source</span><span class="badge">Windows · Linux · Browser</span></div>
<a class="btn" href="{DOWNLOAD}">Download for free</a><a class="btn alt" href="{WEB_APP}">Use it in your browser</a>
<div class="card"><strong>Don't want to run a program?</strong> <a href="{WEB_APP}">The browser version</a> is the same editor, running inside the web page on your own computer: your save is never uploaded. It can't search your folders for saves or back them up for you, so open the file yourself and keep a copy; saving downloads the edited file. A few saves need the downloaded app (SQLite saves, and Palworld or Hogwarts Legacy saves).</div>
<h2>Why download a save editor?</h2>
{WHY}
<h2 id="games">Games ({n})</h2>
<p>{full} games open on a screen made for them; the rest can be edited value by value. Type to find yours, or open the full list:</p>
<input id="q" type="search" placeholder="Find a game" aria-label="Find a game">
<details id="all"><summary>Show all {n} supported games</summary>
<ul class="grid" id="list">{cards}</ul>
</details>
<p class="empty" id="none">No game matches. The editor can still open saves of games that aren't listed: try it, or <a href="{REPO}/issues">ask for the game</a>.</p>
<script>var all=document.getElementById('all');if(location.hash==='#games')all.open=true;window.addEventListener('hashchange',function(){{if(location.hash==='#games')all.open=true}});document.getElementById('q').addEventListener('input',function(){{var q=this.value.toLowerCase().trim(),n=0;if(q)all.open=true;document.querySelectorAll('#list li').forEach(function(li){{var hit=li.querySelector('span').textContent.toLowerCase().indexOf(q)>=0;li.style.display=hit?'':'none';if(hit)n++}});document.getElementById('none').style.display=n?'none':'block'}})</script>
"#,
        n = GAMES.len(),
    );
    std::fs::write(out.join("index.html"), page(&format!("Universal Save Editor - Free Offline Save Editor for {}+ ({}) Games", GAMES.len() / 10 * 10, GAMES.len()), &desc, "", &body, &website_schema(&schema("Universal Save Editor", &desc, &format!("{SITE}/")), None))).unwrap();

    let mut sm = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");
    for u in &urls {
        let _ = writeln!(sm, "<url><loc>{u}</loc></url>");
    }
    sm.push_str("</urlset>\n");
    std::fs::write(out.join("sitemap.xml"), sm).unwrap();
    // GitHub Pages serves this for unknown addresses; keep it out of the index.
    let nf = page("Page not found | Universal Save Editor", "That page doesn't exist. Find your game on the Universal Save Editor home page.", "", "<h1>Page not found</h1>\n<p class=\"lead\">That address doesn't exist. Every supported game is listed on the <a href=\"/universal-save-editor/#games\">home page</a>.</p>", &schema("Universal Save Editor", "Free, offline save editor for PC games.", &format!("{SITE}/")))
        .replace("<meta charset=\"utf-8\">", "<meta charset=\"utf-8\">\n<meta name=\"robots\" content=\"noindex\">")
        .replace("href=\"./style.css\"", "href=\"/universal-save-editor/style.css\"")
        .replace("href=\"./#games\"", "href=\"/universal-save-editor/#games\"")
        .replace("href=\"./app/\"", "href=\"/universal-save-editor/app/\"")
        .replace("href=\"./\"", "href=\"/universal-save-editor/\"");
    std::fs::write(out.join("404.html"), nf).unwrap();
    std::fs::write(out.join("robots.txt"), format!("User-agent: *\nAllow: /\nSitemap: {SITE}/sitemap.xml\n")).unwrap();
    println!("{} pages; full-editor games without a README row: {unmatched:?}", urls.len());
    // Every page needs its own slug.
    let mut slugs: Vec<String> = GAMES.iter().map(|g| slug(&search_name(g.name))).collect();
    slugs.sort();
    let n = slugs.len();
    slugs.dedup();
    assert_eq!(n, slugs.len(), "two games share a page name");
}

/// The README and Cargo.toml say "N+ (count) games"; keep the bracketed count equal to the registry.
#[test]
fn readme_game_count_matches_registry() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for f in ["README.md", "Cargo.toml"] {
        let text = std::fs::read_to_string(root.join(f)).unwrap();
        assert!(with_game_count(&text) == text, "{f} has a stale game count: run `cargo test --release --lib build_site -- --ignored`");
    }
}
