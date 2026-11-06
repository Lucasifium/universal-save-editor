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

/// The README row describing this catalog game, if any.
fn description(g: &GameInfo, rows: &[(String, String)]) -> Option<String> {
    // The table describes the game screens; other games get the general note.
    if g.support != "Game screen" {
        return None;
    }
    // Games the table lists under another name.
    let alias = match g.name {
        n if n.starts_with("Dark Souls") => Some("Elden Ring, Dark Souls"),
        n if n.starts_with("Enderal") => Some("Skyrim, Fallout 4"),
        "Final Fantasy VII Rebirth" => Some("Final Fantasy VII Remake"),
        "Final Fantasy X / X-2 HD Remaster" => Some("Final Fantasy X (HD Remaster"),
        n if n.starts_with("Mass Effect Legendary Edition (ME2") || n.starts_with("Mass Effect Legendary Edition (ME3") => Some("Mass Effect 2, Mass Effect 3"),
        "Yakuza: Like a Dragon" => Some("Yakuza Kiwami 2, Like a Dragon"),
        "Yakuza Kiwami 3 & Dark Ties" => Some("Yakuza 6, Kiwami 3"),
        _ => None,
    };
    if let Some(a) = alias {
        return rows.iter().find(|(g, _)| g.starts_with(a)).map(|(_, d)| d.clone());
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
    best.map(|(_, d)| d.clone())
}

fn level(g: &GameInfo) -> (&'static str, &'static str) {
    if g.support == "Game screen" {
        ("Full editor", "A screen laid out for this game, with names for its items and values.")
    } else if g.support.starts_with("Hex") || g.support.starts_with("Unpacked") {
        ("Value search", "Find a number you know (money, ammo, experience) and change it.")
    } else {
        ("Values editor", "Every value in the save, by name, with a filter.")
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
.grid a{display:block;background:var(--card);border:1px solid var(--line);border-radius:10px;padding:10px 14px;color:var(--text);text-decoration:none}
.grid a:hover{border-color:var(--accent2)}.grid small{display:block;color:var(--muted)}
#q{width:100%;max-width:420px;padding:10px 14px;border-radius:10px;border:1px solid var(--line);background:var(--card);color:var(--text);font-size:1rem}
ol li,ul li{margin:4px 0}footer{border-top:1px solid var(--line);margin-top:48px;color:var(--muted);font-size:.9rem}footer .in{padding:20px 16px}
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
<meta property="og:title" content="{t}">
<meta property="og:description" content="{d}">
<meta property="og:url" content="{SITE}/{path}">
<meta name="twitter:card" content="summary">
<link rel="stylesheet" href="{root}style.css">
<script type="application/ld+json">{schema}</script>
</head>
<body>
<header><div class="in"><a class="brand" href="{root}">Universal Save Editor</a><nav><a href="{root}#games">Games</a><a href="{DOWNLOAD}">Download</a><a href="{REPO}">Source</a></nav></div></header>
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
        "operatingSystem": "Windows, Linux, Android",
        "offers": {"@type": "Offer", "price": "0", "priceCurrency": "USD"},
        "license": "https://www.gnu.org/licenses/gpl-3.0.html",
        "downloadUrl": DOWNLOAD,
    })
    .to_string()
    .replace("</", "<\\/")
}

const WHY: &str = r#"<ul>
<li><strong>Free, forever.</strong> No price, no ads, no account, no paid tier.</li>
<li><strong>Offline.</strong> Your save never leaves your computer: nothing is uploaded, so there are no file size limits, queues or rate limits.</li>
<li><strong>Safe.</strong> Every save is checked by reading it back before it's written, and the old file is backed up first (File &rsaquo; Restore previous version).</li>
<li><strong>Open source.</strong> Anyone can read the code (GPL-3.0).</li>
</ul>"#;

#[test]
#[ignore]
fn build_site() {
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("docs");
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("style.css"), STYLE).unwrap();
    std::fs::write(out.join(".nojekyll"), "").unwrap();
    let rows = readme_rows();
    let mut urls = vec![format!("{SITE}/")];
    let mut cards = String::new();
    let mut unmatched = vec![];
    let mut games: Vec<&GameInfo> = GAMES.iter().collect();
    games.sort_by_key(|g| (g.support != "Game screen", g.name.to_lowercase()));

    for g in &games {
        let s = format!("{}-save-editor", slug(&search_name(g.name)));
        let search = search_name(g.name);
        let (lvl, lvl_note) = level(g);
        let what = description(g, &rows);
        if what.is_none() && g.support == "Game screen" {
            unmatched.push(g.name);
        }
        // For search results: the description without its tab label, about one line long.
        let short_what = what
            .as_deref()
            .map(|w| {
                let w = w.replace(['*', '`'], "");
                let w = match w.split_once(": ") {
                    Some((label, rest)) if label.len() < 20 && !label.contains(' ') => rest.to_string(),
                    _ => w,
                };
                let w = w.replace("; ", ", ");
                if w.chars().count() <= 120 {
                    return w.trim_end_matches('.').to_string();
                }
                let cut: String = w.chars().take(120).collect();
                let end = cut.rfind([',', ' ']).unwrap_or(cut.len());
                format!("{}\u{2026}", cut[..end].trim_end_matches([',', ' ']))
            })
            .unwrap_or_default();
        let desc = if short_what.is_empty() {
            format!("Free, offline {search} save editor for PC. {lvl_note} Nothing is uploaded, no limits.")
        } else {
            let first = short_what.chars().next().map_or(String::new(), |c| c.to_lowercase().collect());
            let dot = if short_what.ends_with('\u{2026}') { "" } else { "." };
            format!("Free, offline {search} save editor for PC. Edit {first}{}{dot} Nothing is uploaded, no limits.", short_what.chars().skip(1).collect::<String>())
        };
        let title = format!("{search} Save Editor: free and offline | Universal Save Editor");
        let mut body = String::new();
        let _ = write!(
            body,
            r#"<h1>{h} save editor</h1>
<p class="lead">Edit your {n} save on your own PC with Universal Save Editor, a free, offline save editor for 100+ games. Nothing is uploaded and there are no limits.</p>
<div class="badges"><span class="badge free">Free forever</span><span class="badge">Offline</span><span class="badge">Open source</span><span class="badge">{lvl}</span></div>
<a class="btn" href="{DOWNLOAD}">Download for free</a><a class="btn alt" href="{REPO}">Source code</a>
<h2>What you can edit</h2>
<div class="card">{what}</div>
<h2>Where {n} keeps its saves</h2>
<div class="card"><p>Folder: <code>{folder}</code></p><p>Files: <code>{files}</code></p><p>You don't need to find it yourself: choose <em>Select game</em> in the editor, pick {n}, and it lists the saves it finds.</p></div>
<h2>How to edit your {n} save</h2>
<ol>
<li><a href="{DOWNLOAD}">Download Universal Save Editor</a> for Windows, Linux or Android. On Windows and Linux just run it; on Android install the APK.</li>
<li>Close the game, so it doesn't write over your changes.</li>
<li>Choose <em>Select game</em>, pick <strong>{n}</strong> and open your save (or drag the save file onto the window).</li>
<li>Change what you want, then <em>Save</em>. The editor checks the new file and backs up the old one first.</li>
</ol>
<h2>Why use it</h2>
{WHY}
<p>Edited saves are for single-player play. Don't use them in online or multiplayer modes.</p>
"#,
            h = esc(&search),
            n = esc(g.name),
            what = what.as_deref().map(md).unwrap_or_else(|| esc(lvl_note)),
            folder = esc(g.folder),
            files = esc(g.files),
        );
        let path = format!("{s}/");
        let dir = out.join(&s);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("index.html"), page(&title, &desc, &path, &body, &schema(&format!("Universal Save Editor ({search} save editor)"), &desc, &format!("{SITE}/{path}")))).unwrap();
        urls.push(format!("{SITE}/{path}"));
        let _ = write!(cards, r#"<li><a href="{s}/">{}<small>{lvl}</small></a></li>"#, esc(g.name));
    }

    let full = GAMES.iter().filter(|g| g.support == "Game screen").count();
    let desc = format!("Free, offline save editor for {}+ PC games, including Cairn, Terraria, Oxygen Not Included, RimWorld, Stardew Valley, Elden Ring and Cyberpunk 2077. Nothing is uploaded, no limits.", GAMES.len() / 10 * 10);
    let body = format!(
        r#"<h1>A free save editor you download, for {n}+ games</h1>
<p class="lead">Universal Save Editor edits your saves on your own computer: no uploads, no accounts, no rate limits. It's free, and always will be.</p>
<div class="badges"><span class="badge free">Free forever</span><span class="badge">Offline</span><span class="badge">Open source</span><span class="badge">Windows · Linux · Android</span></div>
<a class="btn" href="{DOWNLOAD}">Download for free</a><a class="btn alt" href="{REPO}">Source code</a>
<h2>Why download a save editor?</h2>
{WHY}
<h2 id="games">Games ({n})</h2>
<p>{full} games open on a screen made for them; the rest can be edited value by value. Type to filter:</p>
<input id="q" type="search" placeholder="Find a game" aria-label="Find a game">
<ul class="grid" id="list">{cards}</ul>
<script>document.getElementById('q').addEventListener('input',function(){{var q=this.value.toLowerCase();document.querySelectorAll('#list li').forEach(function(li){{li.style.display=li.textContent.toLowerCase().indexOf(q)<0?'none':''}})}})</script>
"#,
        n = GAMES.len(),
    );
    std::fs::write(out.join("index.html"), page("Universal Save Editor: free, offline save editor for 100+ games", &desc, "", &body, &schema("Universal Save Editor", &desc, &format!("{SITE}/")))).unwrap();

    let mut sm = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");
    for u in &urls {
        let _ = writeln!(sm, "<url><loc>{u}</loc></url>");
    }
    sm.push_str("</urlset>\n");
    std::fs::write(out.join("sitemap.xml"), sm).unwrap();
    std::fs::write(out.join("robots.txt"), format!("User-agent: *\nAllow: /\nSitemap: {SITE}/sitemap.xml\n")).unwrap();
    println!("{} pages; full-editor games without a README row: {unmatched:?}", urls.len());
    // Every page needs its own slug.
    let mut slugs: Vec<String> = GAMES.iter().map(|g| slug(&search_name(g.name))).collect();
    slugs.sort();
    let n = slugs.len();
    slugs.dedup();
    assert_eq!(n, slugs.len(), "two games share a page name");
}
