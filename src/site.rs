//! Builds the project website in `docs/` (served by GitHub Pages): a home page and one page per
//! game in the catalog, from the catalog itself and the README's "Game screens" table, plus a
//! sitemap. Run after adding games:
//!
//!     cargo test --release --lib build_site -- --ignored

use crate::games::{GameInfo, GAMES};
use std::fmt::Write as _;

const SITE: &str = "https://universalsaveeditor.com";
/// Public: served at {SITE}/{key}.txt.
const INDEXNOW_KEY: &str = "7c0fcf00800db5fd15744abb0d1bf7e3";
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
    let readme = include_str!("../GAMES.md");
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

const STYLE: &str = r#":root{--bg:#0d111f;--bg2:#131930;--card:#161d36;--line:#27304d;--text:#eaeefb;--muted:#9ba6c6;--accent:#ffc247;--focus:#ffc247;--on-accent:#1a1400;--accent2:#9fb0ff;--track:#232c4b;--shadow:0 18px 50px rgba(0,0,0,.45);--r:14px;--head:ui-rounded,"SF Pro Rounded","Hiragino Maru Gothic ProN","Arial Rounded MT Bold","Segoe UI Variable Display","Segoe UI",system-ui,sans-serif}
html[data-theme="light"]{color-scheme:light;--bg:#eef1f8;--bg2:#e3e8f4;--card:#fff;--line:#d3daea;--text:#121a30;--muted:#4f5b78;--accent:#f2a900;--focus:#2a45d6;--on-accent:#1a1400;--accent2:#2a45d6;--track:#dbe1f1;--shadow:0 18px 40px rgba(30,45,100,.14)}
:root{color-scheme:dark}
.theme-btn{background:none;border:1px solid var(--line);color:var(--muted);font:600 .9rem/1 system-ui,sans-serif;padding:8px 12px;border-radius:10px;cursor:pointer;min-height:36px}.theme-btn:hover{color:var(--text);border-color:var(--accent)}.theme-btn:focus-visible{outline:3px solid var(--focus);outline-offset:3px}
*{box-sizing:border-box}html{scroll-behavior:smooth}
body{margin:0;background:var(--bg);color:var(--text);font:16px/1.65 system-ui,-apple-system,"Segoe UI",Roboto,sans-serif;-webkit-text-size-adjust:100%}
a{color:var(--accent2);text-underline-offset:3px}a:focus-visible,summary:focus-visible,input:focus-visible,.btn:focus-visible{outline:3px solid var(--focus);outline-offset:3px;border-radius:8px}
main,header .in,footer .in{max-width:1040px;margin:0 auto;padding:0 20px}
header{position:sticky;top:0;z-index:5;background:color-mix(in srgb,var(--bg) 88%,transparent);backdrop-filter:blur(10px);border-bottom:1px solid var(--line)}
header .in{display:flex;align-items:center;gap:16px;min-height:60px;flex-wrap:wrap}
header .brand{font:800 1.1rem/1 var(--head);color:var(--text);text-decoration:none;letter-spacing:-.01em;display:flex;align-items:center;gap:10px}
header .brand img{width:26px;height:26px;border-radius:6px;flex:none}
header nav{margin-left:auto;display:flex;gap:20px}header nav a{color:var(--muted);text-decoration:none;font-weight:600;font-size:.95rem;padding:11px 2px;display:inline-block}header nav a:hover{color:var(--text)}
h1{font:800 clamp(2rem,5vw,3.3rem)/1.08 var(--head);letter-spacing:-.025em;margin:0 0 16px;text-wrap:balance}
h2{font:800 1.55rem/1.2 var(--head);letter-spacing:-.015em;margin:48px 0 14px;padding-left:14px;border-left:5px solid var(--accent)}h3{font:700 1.15rem var(--head);margin:28px 0 8px}
p,li{max-width:70ch}
.hero{display:grid;grid-template-columns:minmax(0,1.15fr) minmax(0,.85fr);gap:40px;align-items:center;padding:56px 0 28px}
.hero.game{grid-template-columns:1fr;padding:44px 0 12px}
.lead{font-size:1.18rem;color:var(--muted);max-width:60ch}
.badges{display:flex;gap:8px;flex-wrap:wrap;margin:20px 0}.badge{border:1px solid var(--line);background:var(--card);border-radius:8px;padding:3px 12px;font-size:.88rem;color:var(--muted)}
.badge.free{background:var(--accent);border-color:var(--accent);color:var(--on-accent);font-weight:700}
.btn{display:inline-block;background:var(--accent);color:var(--on-accent);text-decoration:none;font-weight:800;padding:13px 24px;border-radius:12px;margin:8px 10px 8px 0;box-shadow:0 3px 0 rgba(0,0,0,.28);transition:transform .12s,box-shadow .12s}
.btn:hover{transform:translateY(-1px);box-shadow:0 5px 0 rgba(0,0,0,.28)}.btn:active{transform:translateY(2px);box-shadow:0 1px 0 rgba(0,0,0,.28)}
.btn.alt{background:var(--card);color:var(--text);border:1px solid var(--line);box-shadow:none}
.mock{background:var(--card);border:1px solid var(--line);border-radius:var(--r);box-shadow:var(--shadow);overflow:hidden;font-family:ui-monospace,"SF Mono",Consolas,monospace}
.mock .bar{display:flex;gap:6px;padding:10px 14px;background:var(--bg2);border-bottom:1px solid var(--line)}.mock .bar i{width:10px;height:10px;border-radius:50%;background:var(--line)}
.mock .row{padding:14px 18px;border-bottom:1px solid var(--line)}.mock .row:last-child{border:0}
.mock .k{display:flex;justify-content:space-between;font-size:.92rem;color:var(--muted)}.mock .k b{color:var(--text);font-weight:700}
.mock .t{height:8px;border-radius:99px;background:var(--track);margin-top:10px;position:relative;overflow:hidden}
.mock .t::after{content:"";position:absolute;inset:0 auto 0 0;width:var(--w);background:var(--accent);border-radius:99px;animation:fill 1.1s .15s cubic-bezier(.2,.8,.2,1) both}
@keyframes fill{from{width:0}}
@media (prefers-reduced-motion:reduce){.mock .t::after{animation:none}html{scroll-behavior:auto}.btn{transition:none}}
.card{background:var(--card);border:1px solid var(--line);border-radius:var(--r);padding:18px 22px;margin:16px 0}
code{background:var(--bg2);border:1px solid var(--line);border-radius:6px;padding:1px 6px;font-size:.88em;word-break:break-all}
.grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(250px,1fr));gap:12px;padding:0;list-style:none}
.grid li{margin:0;display:flex;max-width:none}
.grid a,.grid div{flex:1;display:flex;flex-direction:column;gap:2px;background:var(--card);border:1px solid var(--line);border-radius:12px;padding:12px 14px 10px;color:var(--text);text-decoration:none;line-height:1.35;font-weight:700;transition:transform .12s,border-color .12s}
.grid a:hover,.grid a:focus-visible{border-color:var(--accent);transform:translateY(-2px)}.grid small{display:block;color:var(--muted);font-size:.85rem;font-weight:400}
.grid .tag{align-self:flex-start;margin-top:auto;font-size:.76rem;font-weight:600;color:var(--muted);padding:2px 8px;margin-top:8px;border-radius:6px;background:var(--bg2)}.grid .tag.screen{background:var(--accent);color:var(--on-accent)}
.empty{color:var(--muted);display:none}
details#all{margin:12px 0}details#all>summary{cursor:pointer;display:inline-block;padding:11px 18px;border:1px solid var(--line);border-radius:12px;background:var(--card);font-weight:700;list-style:none}
details#all>summary::-webkit-details-marker,details.letter>summary::-webkit-details-marker{display:none}details#all>summary::before{content:"\25B8";display:inline-block;margin-right:8px;transition:transform .15s}
details#all[open]>summary::before{transform:rotate(90deg)}details#all>summary:hover{border-color:var(--accent)}
details.letter{margin:10px 0;border-top:1px solid var(--line)}details.letter>summary{cursor:pointer;font:800 1.2rem var(--head);padding:12px 0;list-style:none}details.letter>summary::before{content:"\25B8";display:inline-block;width:1.2em;color:var(--accent);transition:transform .15s}details.letter[open]>summary::before{transform:rotate(90deg)}details.letter>summary span{color:var(--muted);font:400 .95rem system-ui}
#q{width:100%;max-width:440px;padding:12px 16px;border-radius:12px;border:1px solid var(--line);background:var(--card);color:var(--text);font-size:1rem}
ol li,ul li{margin:6px 0}
@media (max-width:600px){.shot{max-width:340px}}
.feature{background:var(--card);border:1px solid var(--line);border-left:4px solid var(--accent);border-radius:var(--r);padding:8px 24px 16px;margin:8px 0 24px}.feature h2{margin-top:16px}
.dl-more{margin:4px 0 0;font-size:.92rem;color:var(--muted);line-height:1.9}.dl-more span{font-weight:600;margin-right:4px}.dl-more a{white-space:nowrap}
.shot{margin:32px auto;max-width:760px}.shot img{display:block;width:100%;max-width:100%;height:auto;border:1px solid var(--line);border-radius:var(--r);box-shadow:var(--shadow)}.shot figcaption{color:var(--muted);font-size:.9rem;margin-top:8px}
.gshot{margin:24px 0}.gshot img{display:block;width:100%;height:auto;border:1px solid var(--line);border-radius:var(--r);box-shadow:var(--shadow)}.gshot figcaption{color:var(--muted);font-size:.9rem;margin-top:8px}
footer{border-top:1px solid var(--line);margin-top:64px;color:var(--muted);font-size:.9rem}footer .in{padding:24px 20px}
@media (max-width:760px){.hero{grid-template-columns:1fr;gap:28px;padding-top:32px}header nav{margin-left:0;gap:4px 14px;flex-wrap:wrap}header nav a{padding:6px 2px}header .in{padding-top:8px;padding-bottom:8px;gap:6px}header{position:static}}
"#;

const DOWNLOAD_BUTTON: &str = r#"<a class="btn" href="https://github.com/Lucasifium/universal-save-editor/releases/latest">Download for free</a>"#;
const RELEASES: &str = "https://github.com/Lucasifium/universal-save-editor/releases";
const DISCUSSIONS: &str = "https://github.com/Lucasifium/universal-save-editor/discussions";
const REPORT: &str = "https://github.com/Lucasifium/universal-save-editor/issues/new/choose";

/// The direct downloads of the current version: (id, label, file name ending).
const DOWNLOADS: &[(&str, &str, &str)] = &[
    ("win-x64", "Windows (64-bit)", "windows-x64.exe"),
    ("win-arm64", "Windows on ARM", "windows-arm64.exe"),
    ("linux-x64", "Linux (64-bit)", "linux-x64.tar.gz"),
    ("linux-arm64", "Linux on ARM", "linux-arm64.tar.gz"),
    ("macos", "macOS", "macos.zip"),
];

fn download_url(end: &str) -> String {
    let v = env!("CARGO_PKG_VERSION");
    format!("{RELEASES}/download/v{v}/UniversalSaveEditor-{v}-{end}")
}

/// Turns a page's main "Download for free" button into the Windows download, with every other
/// download (and all releases) listed under the buttons. The script picks the visitor's system
/// and, once the latest release is known, points the links at its files.
fn with_downloads(body: &str) -> String {
    let Some(at) = body.find(DOWNLOAD_BUTTON) else { return body.to_string() };
    let main = format!(r#"<a class="btn" data-dl-main href="{}">Download for Windows</a>"#, download_url(DOWNLOADS[0].2));
    let links: String = DOWNLOADS
        .iter()
        .map(|(id, label, end)| format!(r#"<a data-dl="{id}" data-end="{end}" href="{}">{label}</a>"#, download_url(end)))
        .collect::<Vec<_>>()
        .join(" · ");
    let more = format!(
        r#"<p class="dl-more" id="download"><span>Version {v}:</span> {links}<br><a href="{RELEASES}">All versions and release notes</a></p>"#,
        v = env!("CARGO_PKG_VERSION")
    );
    let after = at + DOWNLOAD_BUTTON.len();
    let close = body[after..].find("</div>").map_or(body.len(), |i| after + i);
    format!("{}{main}{}{more}{}", &body[..at], &body[after..close], &body[close..])
}

const DOWNLOAD_SCRIPT: &str = r#"<script>(function(){var m=document.querySelectorAll('[data-dl-main]');if(!m.length)return;var ua=navigator.userAgent,d=navigator.userAgentData,p=((d&&d.platform)||navigator.platform||'')+' '+ua;var os=/Android|iPhone|iPad/i.test(ua)?null:/Win/i.test(p)?'win':/Mac/i.test(p)?'mac':/Linux|X11|CrOS/i.test(p)?'linux':null;var arch=/arm|aarch64/i.test(p)?'arm64':'x64';function pick(){if(!os)return;var a=document.querySelector('[data-dl="'+(os==='mac'?'macos':os+'-'+arch)+'"]');if(!a)return;m.forEach(function(b){b.href=a.href;b.textContent='Download for '+a.textContent})}if(d&&d.getHighEntropyValues){d.getHighEntropyValues(['architecture']).then(function(v){if(v.architecture)arch=v.architecture==='arm'?'arm64':'x64';pick()},pick)}else pick();if(window.fetch)fetch('https://api.github.com/repos/Lucasifium/universal-save-editor/releases/latest').then(function(r){return r.ok?r.json():null}).then(function(j){if(!j||!j.assets)return;document.querySelectorAll('[data-dl]').forEach(function(a){j.assets.forEach(function(x){if(x.name.slice(-a.dataset.end.length)===a.dataset.end)a.href=x.browser_download_url})});var s=document.querySelector('.dl-more span');if(s&&j.tag_name)s.textContent='Version '+j.tag_name.replace(/^v/,'')+':';pick()}).catch(function(){})})()</script>"#;

fn page(title: &str, desc: &str, path: &str, body: &str, schema: &str) -> String {
    let body = with_downloads(body);
    let has_dl = body.contains("data-dl-main");
    format!(
        r##"<!doctype html>
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
<meta property="og:image" content="{SITE}/social-preview.png">
<meta property="og:image:width" content="1200">
<meta property="og:image:height" content="630">
<meta property="og:image:alt" content="Universal Save Editor: free, offline save editor for 550+ games">
<meta name="twitter:card" content="summary_large_image">
<meta name="twitter:image" content="{SITE}/social-preview.png">
<link rel="icon" href="{root}favicon.ico" sizes="any">
<link rel="icon" type="image/png" sizes="32x32" href="{root}favicon-32.png">
<link rel="icon" type="image/png" sizes="16x16" href="{root}favicon-16.png">
<link rel="apple-touch-icon" href="{root}apple-touch-icon.png">
<link rel="manifest" href="{root}site.webmanifest">
<meta name="theme-color" content="#0d111f">
<script>try{{if(localStorage.getItem("use-theme")==="light")document.documentElement.setAttribute("data-theme","light")}}catch(e){{}}</script>
<link rel="stylesheet" href="{root}style.css">
<script type="application/ld+json">{schema}</script>
</head>
<body>
<header><div class="in"><a class="brand" href="{root}"><img src="{root}logo-64.png" width="26" height="26" alt="">Universal Save Editor</a><nav><a href="{root}#games">Games</a><a href="{root}save-manager/">Save manager</a><a href="{root}{WEB_APP}">Use in browser</a><a href="{dl_nav}">Download</a><a href="{REPO}">Source</a><button type="button" class="theme-btn" id="theme-toggle" aria-label="Switch to light theme">Light theme</button></nav></div></header>
<main>
{body}
</main>
<footer><div class="in">Universal Save Editor is free, and always will be. Open source under the GPL-3.0. Not affiliated with any game developer or publisher; game names and trademarks belong to their owners. Always keep a backup of your saves.<br><a href="{DISCUSSIONS}">Questions and ideas</a> · <a href="{REPORT}">Report a problem</a> · <a href="{RELEASES}">All versions and release notes</a> · <a href="{REPO}">Source code on GitHub</a> · <a href="{root}code-signing/">Code signing policy</a></div></footer>
{script}
<script>(function(){{var b=document.getElementById("theme-toggle"),h=document.documentElement,m=document.querySelector('meta[name="theme-color"]');if(!b)return;function paint(){{var l=h.getAttribute("data-theme")==="light";b.textContent=l?"Dark theme":"Light theme";b.setAttribute("aria-label",l?"Switch to dark theme":"Switch to light theme");if(m)m.setAttribute("content",l?"#eef1f8":"#0d111f")}}b.addEventListener("click",function(){{var l=h.getAttribute("data-theme")==="light";if(l)h.removeAttribute("data-theme");else h.setAttribute("data-theme","light");try{{localStorage.setItem("use-theme",l?"dark":"light")}}catch(e){{}}paint()}});paint()}})();</script>
</body>
</html>
"##,
        t = esc(title),
        d = esc(desc),
        root = if path.is_empty() { "./" } else { "../" },
        dl_nav = if has_dl { "#download".to_string() } else if path.is_empty() { "./#download".to_string() } else { "../#download".to_string() },
        script = if has_dl { DOWNLOAD_SCRIPT } else { "" },
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

/// The site's name and, on game pages, the page's place in the site.
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
<li><strong>Optional update alert.</strong> Off by default. If you turn it on (Preferences), the program asks GitHub once at startup whether a newer release exists and shows a banner with a download link; nothing else is sent.</li>
<li><strong>Safe.</strong> Every save is checked by reading it back before it's written, and the old file is backed up first (Backups &rsaquo; Open save manager).</li>
<li><strong>Works on games it doesn't know.</strong> Value search finds a number you know, or one you don't (a health bar, a hidden counter): scan, play, save, then keep what increased, decreased, changed or stayed the same.</li>
<li><strong>Linux and Steam Deck.</strong> <em>Find my game's saves</em> also looks inside Proton and Wine prefixes, where Windows games run through Steam Play keep their saves.</li>
<li><strong>Xbox app and PC Game Pass.</strong> Saves hidden in the Xbox app's container are listed under the names the game gave them and open with the same screens as the Steam saves.</li>
<li><strong>Unity PlayerPrefs.</strong> Coins and unlocks that Unity games keep in the Windows registry (or prefs files on Linux and macOS) open by name and are written back on save.</li>
<li><strong>Flash and Haxe saves.</strong> Flash and AIR <code>.sol</code> files and the Haxe saves of OpenFL and HaxeFlixel games (Friday Night Funkin' among them) open with every value by name.</li>
<li><strong>Emulator memory cards.</strong> PlayStation and PlayStation 2 memory card images open one save at a time and are written back into the card, with the PS2 card's error-correction codes made again. Single saves exported by cheat devices and PC tools (.mcs, .psu, .max, .cbs, .xps and PS3 .psv) open too and are packed back in their own format.</li>
<li><strong>Checksum finder.</strong> For a save it doesn't know, the editor looks for a CRC, sum or hash near the start or end of the file and writes it again every time you save.</li>
<li><strong>Find a value, step by step.</strong> On a game it doesn't know, a short guide walks you through finding a number and changing it, and the search can refine by itself each time the game saves.</li>
<li><strong>Your own templates.</strong> Name the values you found and press <em>Use for every save</em>, or write a small JSON file saying where each value is; matching saves then open as a list of those values, with the checksum kept right. Names and templates are easy to share.</li>
<li><strong>Made for phones too.</strong> On a phone-size screen the toolbar is one line, and large section chips jump to each part of a game screen, marked where you made changes.</li>
<li><strong>Bulk edit and presets.</strong> Tick many values and set, add to or multiply them all at once; keep a set of changes as a preset and make it again on another save or slot.</li>
<li><strong>Console cheat patches.</strong> About 2,400 PlayStation 1 to 4, PSP, Vita and Dreamcast games have ready-made cheats from the Apollo patch collection built in: pick the codes and apply them. Encrypted saves from about 80 PS3, PS4, PSP and Vita games open already decrypted, and are encrypted again on save.</li>
<li><strong>Dreamcast saves.</strong> VMU <code>.vms</code> files and Nexus <code>.dci</code> files open with their header CRC and byte order handled, and are written back the same way.</li>
<li><strong>Sega Saturn saves.</strong> Single <code>.bup</code>, <code>.sro</code>, <code>.xml</code>, <code>.b64</code> and <code>.cms</code> saves, and whole backup RAM images from Mednafen, Beetle, Yabause, Kronos and SAROO, one save at a time.</li>
<li><strong>In your language.</strong> Menus and messages in English, German, Spanish, French, Brazilian Portuguese, Japanese and Simplified Chinese, following the system language.</li>
<li><strong>Several saves in tabs.</strong> Open more than one save at a time and copy a value, or a whole group such as an inventory, from one save to another.</li>
<li><strong>Steam account switch.</strong> Finds the Steam IDs inside a save and changes them to another account, for saves copied between accounts or co-op hosts.</li>
<li><strong>Remembers where your saves are.</strong> Folders that held saves are remembered and checked first, so starting the editor doesn't search the whole computer again; <em>Search again</em> looks for newly added games on every fixed drive, including Steam libraries elsewhere and a moved Documents folder.</li>
<li><strong>Finds your games.</strong> The start page lists the games whose saves are on this computer, newest first, one click from the latest save; <em>Find my game's saves</em> shows them first, with a badge for how much of each game can be edited.</li>
<li><strong>Never writes over a newer save.</strong> If the game or a cloud sync saved the file after you opened it, saving stops and asks (reload, or save anyway with the newer file backed up first).</li>
<li><strong>Japanese, Chinese and Korean names.</strong> Shown with your computer's own fonts in the downloaded app.</li>
<li><strong>Save manager (toolbar button).</strong> Automatic and named backups kept in a folder next to each save, where no game loads them. Restore any backup (the current save is backed up first), compare it, or back up and start a new game on a one-save game and restore the old one later. Every backup is flushed to disk and checked before anything is replaced.</li>
<li><strong>Command line too.</strong> <code>info</code>, <code>list</code>, <code>get</code>, <code>set</code>, <code>decode</code> and <code>encode</code> work from scripts, with the same checks and backups as the window.</li>
<li><strong>Open source.</strong> Anyone can read the code (GPL-3.0).</li>
</ul>"#;

const SAVE_MANAGER_DESC: &str = "Free, offline save manager for PC games: automatic and named backups, one-click restore, compare, and start a new game without losing your old save.";

fn save_manager_body() -> String {
    format!(
        r#"<section class="hero game"><div><h1>Save manager for your game saves</h1>
<p class="lead">Universal Save Editor keeps backups of your saves on your own computer. Make a named backup before a hard fight or a big choice, restore any backup in one click, or put a save aside to start a new game and come back to it later. It works for every game the editor opens, whether or not you edit anything.</p>
<div class="badges"><span class="badge free">Free forever</span><span class="badge">Offline</span><span class="badge">Open source</span></div>
<a class="btn" href="{DOWNLOAD}">Download for free</a></div>
<div class="mock" aria-hidden="true"><div class="bar"><i></i><i></i><i></i></div><div class="row"><div class="k"><span>Before the final boss</span><b>Named</b></div></div><div class="row"><div class="k"><span>Today 14:02</span><b>Automatic</b></div></div><div class="row"><div class="k"><span>Today 13:47</span><b>Automatic</b></div></div><div class="row"><div class="k"><span>Restore this backup</span><b>&#8634;</b></div></div></div></section>
<h2>What it does</h2>
<ul>
<li><strong>Automatic backups.</strong> Every time the editor saves a file, it first backs up the file as it was. The last 30 are kept, and you can change that number in <em>Edit › Preferences</em>.</li>
<li><strong>Named backups.</strong> <em>Backups › Back up now…</em> makes a backup with a name you choose, such as "before the final boss". It's kept until you delete it.</li>
<li><strong>Restore.</strong> The <em>Save manager</em> button in the toolbar (or <em>Backups › Save manager…</em>) lists every backup of the open save. <em>Restore this backup</em> puts it back, and the current file is backed up first, so a restore can be undone too.</li>
<li><strong>Quick restore.</strong> <em>Backups › Quick restore</em> puts back a recent backup without opening the list.</li>
<li><strong>Compare.</strong> See what changed between a backup and the current save, byte by byte or value by value.</li>
<li><strong>Start a new game, keep the old one.</strong> Many games only have one save. <em>Back up and start a new game…</em> makes a named backup and then moves the current save out of the way, so the game starts fresh. Restore the backup whenever you want your old game back.</li>
<li><strong>Save gone?</strong> <em>Recent files</em> offers <em>Restore a backup…</em> even when the save itself no longer exists.</li>
</ul>
<h2>Where the backups are kept</h2>
<div class="card"><p>Next to each save, in a folder of its own: <code>Save Editor Backups/&lt;file name&gt;/</code>. The backups have a <code>.sebak</code> extension, so no game loads them or writes over them. If the save's folder can't be written to, they go to the editor's own folder instead. You can also choose another folder, such as a synced folder or another drive.</p></div>
<h2>Why you can trust it with your saves</h2>
<ul>
<li>Every backup and every restore is written to a temporary file, flushed to disk, read back and compared before it replaces anything.</li>
<li>Nothing is replaced if the current save can't be read and backed up first, for example while a game holds it locked.</li>
<li>If the game or a cloud sync changes a save after you opened it, the editor stops and asks before writing.</li>
<li>Everything stays on your computer. Nothing is uploaded.</li>
</ul>
<p>The save manager is part of the downloaded app for Windows and Linux. <a href="../{WEB_APP}">The browser version</a> can edit saves but can't keep backups, so keep your own copy when you use it.</p>
<p><a href="../#games">See all {n} supported games</a></p>
"#,
        n = GAMES.len(),
    )
}

const POKEMON_ITEM: &str = r#"<li><a href="pokemon-save-editor/"><span>Pokémon (Gen 1-9)</span><small>Every mainline game, Red and Blue to Scarlet and Violet: trainer, money, badges, bag, party and PC boxes</small><span class="tag screen">Game screen</span></a></li>"#;

const POKEMON_DESC: &str = "Free, offline Pokémon save editor for every generation, Red and Blue to Scarlet and Violet: trainer, money, badges, bag, party and PC boxes, IVs, EVs and moves.";

fn pokemon_body(rows: &[(String, String)], pages: &[(&str, String)]) -> String {
    let row = |prefix: &str| rows.iter().find(|r| r.0.starts_with(prefix)).map(|r| md(&r.1)).unwrap_or_default();
    let fusion = pages.iter().find(|p| p.0.starts_with("Pokémon Infinite Fusion")).map(|p| format!(r#"<p>Looking for the fan game? <a href="../{}/">Pokémon Infinite Fusion save editor</a>.</p>"#, p.1)).unwrap_or_default();
    format!(
        r#"<section class="hero game"><div><h1>Pokémon save editor</h1>
<p class="lead">Edit the mainline Pokémon games of every generation, from Red and Blue to Scarlet and Violet, with Universal Save Editor: a free, offline save editor that runs on your own computer. Change your trainer, money and badges, fill your bag, and edit any Pokémon in your party or PC boxes. The games' encryption and checksums are handled for you.</p>
<div class="badges"><span class="badge free">Free forever</span><span class="badge">Offline</span><span class="badge">Open source</span><span class="badge">Game screen</span></div>
<a class="btn" href="{DOWNLOAD}">Download for free</a><a class="btn alt" href="../{WEB_APP}">Use it in your browser</a></div></section>
<h2>Generations I to III</h2>
<p>Red, Blue and Yellow; Gold, Silver and Crystal; Ruby, Sapphire, Emerald, FireRed and LeafGreen.</p>
<div class="card">{gen13}</div>
<h2>Generations IV to IX</h2>
<p>Diamond, Pearl and Platinum; HeartGold and SoulSilver; Black and White, Black 2 and White 2; X and Y; Omega Ruby and Alpha Sapphire; Sun and Moon, Ultra Sun and Ultra Moon; Sword and Shield; Scarlet and Violet.</p>
<div class="card">{gen49}</div>
<h2>How to edit your Pokémon save</h2>
<ol>
<li>Get the save file. On an emulator, it's the game's save file (often <code>.sav</code>). On a console, back the save up to your computer with a save manager tool first.</li>
<li>Keep a copy of the original somewhere safe.</li>
<li><a href=#download>Download Universal Save Editor</a>, or <a href="../{WEB_APP}">use it in your browser</a>, and open the save (or drag it onto the window).</li>
<li>Change what you want, then <em>Save</em>. The editor checks the new file and, in the downloaded app, backs up the old one first.</li>
<li>Put the edited save back where it came from.</li>
</ol>
<h2>Questions</h2>
<h3>Which games does it open?</h3>
<p>The mainline games listed above. It recognises the game from the save itself, so you don't pick it from a list.</p>
<h3>Will the game still accept the save?</h3>
<p>The editor rewrites the checksums and encryption the games use, and keeps linked values in step: a Pokémon's level sets its experience and stats. Always keep a copy of the original anyway.</p>
<h3>Can I use edited Pokémon online?</h3>
<p>No. Edited saves are for single-player play. Don't trade or battle online with edited Pokémon.</p>
<h3>Is it free?</h3>
<p>Yes, and it always will be: no ads, no account, no upload, no limits. The code is open source under the GPL-3.0.</p>
{fusion}
<p>Pokémon is a trademark of its owners. This editor is not affiliated with or endorsed by them.</p>
<p><a href="../#games">See all {n} supported games</a></p>
"#,
        gen13 = row("Pokémon (Gen I"),
        gen49 = row("Pokémon (Gen IV"),
        n = GAMES.len(),
    )
}

/// A short list of reasons for a game's page; the home page keeps the full list.
fn why_short() -> String {
    let keep = ["Free, forever.", "Offline.", "Safe.", "Works on games it doesn't know.", "Linux and Steam Deck.", "Save manager", "In your language.", "Open source."];
    let items: String = WHY
        .lines()
        .filter(|l| keep.iter().any(|k| l.starts_with(&format!("<li><strong>{k}"))))
        .map(|l| format!("{l}\n"))
        .collect();
    format!("<ul>\n{items}</ul>")
}

/// Rewrites "N+ (M) PC games" (or "N+ PC games") to the current registry numbers.
fn with_game_count(text: &str) -> String {
    const TAIL: &str = " PC games";
    let mut out = String::new();
    let mut rest = text;
    while let Some(i) = rest.find(TAIL) {
        let head = &rest[..i];
        let start = head.rfind(|c: char| !(c.is_ascii_digit() || "+ ()".contains(c))).map(|k| k + 1).unwrap_or(0);
        let start = start + head[start..].len() - head[start..].trim_start_matches(' ').len();
        out.push_str(&rest[..start]);
        let _ = write!(out, "{}+ ({}){TAIL}", GAMES.len() / 10 * 10, GAMES.len());
        rest = &rest[i + TAIL.len()..];
    }
    out.push_str(rest);
    out
}

/// The consoles a README row's game is for: from its own name, or else from the registry entry it
/// names ("Mario Kart 7" is "Mario Kart 7 (3DS, decrypted saves)").
fn row_consoles(name: &str) -> Vec<&'static str> {
    let own = crate::games::consoles_of(name);
    if !own.is_empty() {
        return own;
    }
    let lower = name.to_lowercase();
    GAMES
        .iter()
        .find(|g| {
            let n = g.name.to_lowercase();
            n.strip_prefix(&lower).is_some_and(|rest| rest.starts_with(" ("))
        })
        .map(|g| crate::games::consoles_of(g.name))
        .unwrap_or_default()
}

/// The README's "Game screens" section as one collapsible table per first letter (digits and
/// symbols under #) for PC games, then one per console for console games, rows sorted by name.
/// Rows added anywhere in the section, in any order, are folded into their group; `build_site`
/// rewrites the README with this.
fn group_readme_screens(text: &str) -> String {
    let Some(at) = text.find("## Game screens") else { return text.to_string() };
    let end = text[at..].find("\n## ").map_or(text.len(), |e| at + e);
    let section = &text[at..end];
    let intro: Vec<&str> = section.lines().take_while(|l| !l.starts_with("<details") && !l.starts_with('|')).collect();
    let mut rows: Vec<(char, String, &str)> = vec![];
    let mut console_rows: Vec<(usize, String, &str)> = vec![];
    for l in section.lines().filter(|l| l.starts_with("| ") && !l.starts_with("| Game |")) {
        let raw = l[2..].split(" | ").next().unwrap_or("").trim();
        let name = raw.to_lowercase();
        let consoles = row_consoles(raw);
        if consoles.is_empty() {
            let c = name.chars().next().and_then(|c| c.to_uppercase().next()).filter(|c| c.is_alphabetic()).unwrap_or('#');
            rows.push((c, name.clone(), l));
        }
        for c in consoles {
            let at = crate::games::CONSOLES.iter().position(|(n, _)| *n == c).unwrap_or(usize::MAX);
            console_rows.push((at, name.clone(), l));
        }
    }
    rows.sort_by(|a, b| ((a.0 != '#'), a.0, &a.1).cmp(&((b.0 != '#'), b.0, &b.1)));
    console_rows.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
    console_rows.dedup_by(|a, b| a.0 == b.0 && a.2 == b.2);
    let mut out = intro.join("\n").trim_end().to_string();
    out.push_str("\n\n");
    let mut i = 0;
    while i < rows.len() {
        let c = rows[i].0;
        let n = rows[i..].iter().take_while(|r| r.0 == c).count();
        let _ = write!(out, "<details>\n<summary><strong>{c}</strong> ({n})</summary>\n\n| Game | Screen |\n|---|---|\n");
        for r in &rows[i..i + n] {
            out.push_str(r.2);
            out.push('\n');
        }
        out.push_str("\n</details>\n\n");
        i += n;
    }
    if !console_rows.is_empty() {
        out.push_str("**Console games**, grouped by console (emulator, memory card and decrypted console saves):\n\n");
    }
    let mut i = 0;
    while i < console_rows.len() {
        let c = console_rows[i].0;
        let n = console_rows[i..].iter().take_while(|r| r.0 == c).count();
        let title = crate::games::CONSOLES.get(c).map_or("Other", |c| c.0);
        let _ = write!(out, "<details>\n<summary><strong>{title}</strong> ({n})</summary>\n\n| Game | Screen |\n|---|---|\n");
        for r in &console_rows[i..i + n] {
            out.push_str(r.2);
            out.push('\n');
        }
        out.push_str("\n</details>\n\n");
        i += n;
    }
    format!("{}{}{}", &text[..at], out.trim_end_matches('\n'), &text[end..])
}

/// Folder of the editor pictures shown on game pages (`<slug>-640.webp`, `-1200.webp` and `-2000.webp`).
/// They are made once from the editor's own window and kept here between site builds.
fn shots_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("docs").join("shots")
}

/// Width every game picture is made at; each page gets the height from `docs/shots/index.txt`
/// (`<slug> <height at 1200 wide>` per line), so the pictures are cut to their content.
const SHOT_W: u32 = 1200;

fn shot_heights() -> std::collections::HashMap<String, u32> {
    let text = std::fs::read_to_string(shots_dir().join("index.txt")).unwrap_or_default();
    text.lines().filter_map(|l| l.split_once(' ')).filter_map(|(k, v)| Some((k.to_string(), v.trim().parse().ok()?))).collect()
}

/// The game's screen as the editor draws it, when there is a picture for it (none for games without a
/// public sample save). The page works the same without it.
fn shot_html(slug_dir: &str, game: &str, what: Option<&str>) -> String {
    let name = slug_dir.trim_end_matches("-save-editor");
    let Some(h) = shot_heights().get(name).copied() else { return String::new() };
    let shows = what.map(card_blurb).filter(|b| !b.is_empty()).map(|b| format!(": {}", b.trim_end_matches('\u{2026}').to_lowercase())).unwrap_or_default();
    let alt = format!("{game} save editor screen with a save open{shows}");
    format!(
        r#"<figure class="gshot"><a href="../shots/{name}-2000.webp"><img src="../shots/{name}-{SHOT_W}.webp" srcset="../shots/{name}-640.webp 640w, ../shots/{name}-{SHOT_W}.webp {SHOT_W}w, ../shots/{name}-2000.webp 2000w" sizes="(min-width: 960px) 1040px, 100vw" width="{SHOT_W}" height="{h}" alt="{}" loading="lazy" decoding="async"></a><figcaption>The {} screen in Universal Save Editor, opened on an example save. Select the picture to see it full size.</figcaption></figure>
"#,
        esc(&alt),
        esc(game)
    )
}

/// Common questions about one game's saves, answered only from what the catalog and the
/// supported-games list say about that game.
fn faq_html(g: &GameInfo, what: Option<&str>) -> String {
    let n = esc(g.name);
    let mut out = format!("<h2>Questions about {n} saves</h2>\n");
    let mut q = |question: String, answer: String| {
        let _ = write!(out, "<h3>{question}</h3>\n<p>{answer}</p>\n");
    };
    q(
        format!("Where is the {n} save file?"),
        format!("Usually in <code>{}</code>, in files named <code>{}</code>. The downloaded editor finds it for you: choose <em>Find my game's saves</em> and pick {n}.", esc(g.folder), esc(g.files)),
    );
    let untried = what.is_some_and(|w| w.contains("not yet tried in the game"));
    q(
        format!("Will {n} still load an edited save?"),
        if untried {
            "The editor reads and writes this save correctly on the test saves it was checked with, but loading an edited save in the game itself hasn't been tried yet. The original is backed up before every save, so if the game doesn't accept the edit, restore it from <em>Backups</em>.".to_string()
        } else {
            "The editor writes the save back in the same format the game uses and reads the new file back to check it before replacing anything. The original is backed up first, so if anything looks wrong in the game, restore it from <em>Backups</em>.".to_string()
        },
    );
    if !g.name.contains("Xbox app") {
    q(
        format!("Can I edit {n} saves on Steam Deck or Linux?"),
        "Yes. There's a Linux build, and <em>Find my game's saves</em> also looks inside the Proton and Wine prefixes where Steam Play keeps Windows games' saves.".to_string(),
    );
    }
    q(
        format!("Can I edit my {n} save in the browser?"),
        if needs_app(g) {
            "Not this one: the browser version can't open this game's saves. Use the downloaded app for Windows or Linux.".to_string()
        } else {
            format!(r#"Yes. <a href="../{WEB_APP}">The browser version</a> runs on your own computer and never uploads the save. Open the file yourself and keep a copy, because saving downloads the edited file instead of backing up the old one."#)
        },
    );
    q(
        "Is it safe to use with online play?".to_string(),
        "Edited saves are for single-player play. Don't use them in online or multiplayer modes, where they can break rules or get an account flagged.".to_string(),
    );
    q("Does it cost anything?".to_string(), "No. It's free and always will be: no ads, no account and no limits. The code is open source under the GPL-3.0.".to_string());
    out
}

/// Up to six links to other game pages: the same series first (same first word), then games
/// spread through the list.
/// Genre pages: (key, page slug, heading, short description).
const GENRES: &[(&str, &str, &str, &str)] = &[
    ("rpg", "rpg-save-editors", "RPG", "role-playing games, from classic party RPGs to open worlds and action RPGs"),
    ("soulslike", "soulslike-save-editors", "Soulslike and Metroidvania", "soulslikes and Metroidvanias, where runes, souls and upgrade materials are the usual edits"),
    ("action", "action-adventure-save-editors", "Action and adventure", "action and adventure games, open worlds, platformers and brawlers"),
    ("shooter", "shooter-save-editors", "Shooter", "first- and third-person shooters, looter shooters and co-op shooters"),
    ("roguelike", "roguelike-save-editors", "Roguelike, roguelite and idle", "roguelikes, roguelites, deckbuilders and idle games, where unlocks and meta currency carry between runs"),
    ("survival", "survival-crafting-save-editors", "Survival and crafting", "survival, crafting and sandbox games"),
    ("simulation", "simulation-save-editors", "Simulation, farming and management", "simulation, farming, driving, building and management games"),
    ("strategy", "strategy-save-editors", "Strategy", "grand strategy, 4X, tactics and tower defense games"),
];

fn genre_of(name: &str) -> Option<&'static (&'static str, &'static str, &'static str, &'static str)> {
    let key = crate::genres::GAME_GENRES.iter().find(|(n, _)| *n == name)?.1;
    GENRES.iter().find(|g| g.0 == key)
}

fn related_html(g: &GameInfo, pages: &[(&str, String)]) -> String {
    let key = |n: &str| search_name(n).split_whitespace().next().unwrap_or("").to_lowercase();
    let mine = key(g.name);
    let at = pages.iter().position(|p| p.0 == g.name).unwrap_or(0);
    let mut picked: Vec<usize> = vec![];
    if mine.len() > 3 && !["the", "final", "super", "star", "dead"].contains(&mine.as_str()) {
        picked.extend((0..pages.len()).filter(|&i| i != at && key(pages[i].0) == mine).take(4));
    }
    // Then games of the same genre, spread through the list.
    let genre = genre_of(g.name).map(|x| x.0);
    let same: Vec<usize> = (0..pages.len()).filter(|&i| i != at && !picked.contains(&i) && genre.is_some() && genre_of(pages[i].0).map(|x| x.0) == genre).collect();
    let mut k = 0;
    while picked.len() < 6 && k < same.len() {
        let i = same[(at * 7 + k * 13) % same.len()];
        if !picked.contains(&i) {
            picked.push(i);
        }
        k += 1;
    }
    let mut step = 1;
    while picked.len() < 6 && step < pages.len() {
        let i = (at + step * 37 + 11) % pages.len();
        if i != at && !picked.contains(&i) {
            picked.push(i);
        }
        step += 1;
    }
    let links: String = picked.iter().map(|&i| format!(r#"<li><a href="../{}/">{}</a></li>"#, pages[i].1, esc(pages[i].0))).collect();
    let hub = genre_of(g.name).map(|x| format!(r#"<a href="../{}/">All {} save editors</a> · "#, x.1, x.2)).unwrap_or_default();
    format!("<h2>More save editors</h2>\n<ul class=\"grid more\">{links}</ul>\n<p>{hub}<a href=\"../#games\">See all {} supported games</a></p>\n", GAMES.len())
}

#[test]
#[ignore]
fn build_site() {
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("docs");
    // Keep the bracketed game count in the README and Cargo.toml current.
    for f in ["README.md", "GAMES.md", "Cargo.toml"] {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(f);
        let text = std::fs::read_to_string(&path).unwrap();
        let mut fixed = with_game_count(&text);
        if f == "GAMES.md" {
            fixed = group_readme_screens(&fixed);
        }
        if fixed != text {
            std::fs::write(&path, fixed).unwrap();
        }
    }
    // Start over, except for the browser version (web/build.sh builds it separately) and the game pictures.
    for e in std::fs::read_dir(&out).into_iter().flatten().flatten() {
        if e.file_name() != "app" && e.file_name() != "shots" {
            let p = e.path();
            let _ = if p.is_dir() { std::fs::remove_dir_all(&p) } else { std::fs::remove_file(&p) };
        }
    }
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("style.css"), STYLE).unwrap();
    std::fs::write(out.join(".nojekyll"), "").unwrap();
    std::fs::write(out.join("CNAME"), format!("{}\n", SITE.trim_start_matches("https://"))).unwrap();
    // IndexNow key file (see .github/workflows/indexnow.yml).
    std::fs::write(out.join(format!("{INDEXNOW_KEY}.txt")), INDEXNOW_KEY).unwrap();
    // The picture shown when a page is shared (Open Graph / Twitter card).
    std::fs::write(out.join("social-preview.png"), include_bytes!("../assets/social-preview.png")).unwrap();
    std::fs::write(out.join("screenshot.png"), include_bytes!("../assets/screenshot.png")).unwrap();
    std::fs::write(out.join("screenshot.webp"), include_bytes!("../assets/screenshot.webp")).unwrap();
    std::fs::write(out.join("screenshot-phone.png"), include_bytes!("../assets/screenshot-phone.png")).unwrap();
    std::fs::write(out.join("screenshot-phone.webp"), include_bytes!("../assets/screenshot-phone.webp")).unwrap();
    // Site icons, drawn from the app's own icon.
    std::fs::write(out.join("favicon.ico"), crate::icon::ico(&[16, 32, 48])).unwrap();
    for (name, size) in [("logo-64.png", 64), ("favicon-16.png", 16), ("favicon-32.png", 32), ("apple-touch-icon.png", 180), ("icon-192.png", 192), ("icon-512.png", 512)] {
        std::fs::write(out.join(name), crate::icon::png(size)).unwrap();
    }
    std::fs::write(
        out.join("site.webmanifest"),
        serde_json::json!({
            "name": "Universal Save Editor",
            "short_name": "Save Editor",
            "start_url": "./app/",
            "display": "standalone",
            "background_color": "#0d111f",
            "theme_color": "#0d111f",
            "icons": [
                {"src": "icon-192.png", "sizes": "192x192", "type": "image/png"},
                {"src": "icon-512.png", "sizes": "512x512", "type": "image/png"}
            ]
        })
        .to_string(),
    )
    .unwrap();
    // Site ownership file.
    std::fs::write(out.join("google9d120e6c8e9d3fa5.html"), "google-site-verification: google9d120e6c8e9d3fa5.html").unwrap();
    let rows = readme_rows();
    let mut urls = vec![format!("{SITE}/"), format!("{SITE}/{WEB_APP}")];
    // The game list, one collapsible section per first letter (digits and symbols under #).
    let mut letters: std::collections::BTreeMap<(bool, char), (usize, String)> = Default::default();
    // Console games go under their console instead (by position in the console list).
    let mut console_cards: std::collections::BTreeMap<usize, (usize, String)> = Default::default();
    let mut unmatched = vec![];
    // SITE_SKIP="Game A|Game B" leaves those games off the site for now.
    let skip: Vec<String> = std::env::var("SITE_SKIP").unwrap_or_default().split('|').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    let mut games: Vec<&GameInfo> = GAMES.iter().filter(|g| !skip.iter().any(|s| s == g.name)).collect();
    games.sort_by_key(|g| (!has_screen(g), g.name.to_lowercase()));

    // Every game that gets a page, for the "More save editors" lists.
    let pages: Vec<(&str, String)> = games.iter().filter(|g| crate::games::consoles_of(g.name).is_empty()).map(|g| (g.name, format!("{}-save-editor", slug(&search_name(g.name))))).collect();
    let mut pages = pages;
    let mut pokemon_listed = false;
    let mut genre_items: std::collections::HashMap<&str, Vec<(String, String)>> = Default::default();
    genre_items.entry("rpg").or_default().push(("pokémon".into(), POKEMON_ITEM.replace(r#"<a href=""#, r#"<a href="../"#)));
    pages.push(("Pokémon", "pokemon-save-editor".to_string()));
    for g in &games {
        // Console and emulated games have no page of their own: they only appear in their console's section.
        let consoles = crate::games::consoles_of(g.name);
        if !consoles.is_empty() {
            let (lvl, _) = level(g);
            let blurb = description(g, &rows).as_deref().map(card_blurb).unwrap_or_default();
            let blurb_html = if blurb.is_empty() { String::new() } else { format!("<small>{}</small>", esc(&blurb)) };
            let tag_class = if has_screen(g) { "tag screen" } else { "tag" };
            let item = format!(r#"<li><div><span>{}</span>{blurb_html}<span class="{tag_class}">{lvl}</span></div></li>"#, esc(g.name));
            for c in consoles {
                let at = crate::games::CONSOLES.iter().position(|(n, _)| *n == c).unwrap_or(usize::MAX);
                let (count, cards) = console_cards.entry(at).or_default();
                *count += 1;
                cards.push_str(&item);
            }
            continue;
        }
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
                // End on a whole list item where there is one, never on a dangling "/" or "(".
                // Never leave a "(" open: cut before it instead.
                let cut = match (cut.matches('(').count() > cut.matches(')').count(), cut.rfind('(')) {
                    (true, Some(open)) => cut[..open].trim_end().to_string(),
                    _ => cut,
                };
                match cut.rfind(", ").filter(|&e| e > limit / 3) {
                    Some(end) => format!("{} and more", cut[..end].trim_end_matches(|c: char| !c.is_alphanumeric() && c != ')')),
                    None => {
                        let end = cut.rfind(' ').unwrap_or(cut.len());
                        format!("{}\u{2026}", cut[..end].trim_end_matches(|c: char| !c.is_alphanumeric() && c != ')'))
                    }
                }
            };
            let first = short.chars().next().map_or(String::new(), |c| c.to_lowercase().collect());
            let dot = if short.ends_with('\u{2026}') { "" } else { "." };
            format!("Free, offline {search} save editor for PC. Edit {first}{}{dot}{tail}", short.chars().skip(1).collect::<String>())
        };
        // Descriptions stay under 160 characters: keep the game name and what
        // can be edited, and drop the boilerplate first.
        let desc = [(120, true), (120, false), (80, false), (50, false)]
            .iter()
            .map(|&(l, t)| build_desc(l, t))
            .find(|d| d.chars().count() <= 160)
            .unwrap_or_else(|| build_desc(50, false));
        // Titles stay near 60 characters: the game name always comes first.
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
            r#"<section class="hero game"><div><h1>{h} save editor</h1>
<p class="lead">Edit your {n} save on your own PC with Universal Save Editor, a free, offline save editor and manager for {many}+ ({total}) games. Back up, restore and start new runs from the same window. Nothing is uploaded and there are no limits.</p>
<div class="badges"><span class="badge free">Free forever</span><span class="badge">Offline</span><span class="badge">Open source</span><span class="badge">{lvl}</span></div>
<a class="btn" href="{DOWNLOAD}">Download for free</a><a class="btn alt" href="../{WEB_APP}">Use it in your browser</a></div></section>
<h2>What you can edit</h2>
<div class="card">{what}</div>
{shot}<h2>Where {n} keeps its saves</h2>
<div class="card"><p>Folder: <code>{folder}</code></p><p>Files: <code>{files}</code></p><p>You don't need to find it yourself: choose <em>Find my game's saves</em> in the editor, pick {n}, and it lists the saves it finds. (The browser version can't look through your folders: open the file from this folder yourself.)</p>{browser}</div>
<h2>How to edit your {n} save</h2>
<ol>
<li><a href=#download>Download Universal Save Editor</a> for Windows or Linux; just run it. Rather not install anything? <a href="../{WEB_APP}">Use it in your browser</a> instead: it runs on your computer too, and saving downloads the edited file.</li>
<li>Close the game, so it doesn't write over your changes.</li>
<li>Choose <em>Find my game's saves</em>, pick <strong>{n}</strong> and open your save (or drag the save file onto the window).</li>
<li>Change what you want, then <em>Save</em>. The editor checks the new file and backs up the old one first.</li>
</ol>
{faq}<h2>Why use it</h2>
{why}
<p>Edited saves are for single-player play. Don't use them in online or multiplayer modes.</p>
{related}"#,
            why = why_short(),
            faq = faq_html(g, what.as_deref()),
            shot = shot_html(&s, g.name, what.as_deref()),
            related = related_html(g, &pages),
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
        let first = g.name.chars().next().and_then(|c| c.to_uppercase().next()).filter(|c| c.is_alphabetic());
        let item = format!(r#"<li><a href="{s}/"><span>{}</span>{blurb_html}<span class="{tag_class}">{lvl}</span></a></li>"#, esc(g.name));
        if let Some(gen) = genre_of(g.name) {
            genre_items.entry(gen.0).or_default().push((g.name.to_lowercase(), item.replace(r#"<a href=""#, r#"<a href="../"#)));
        }
        let (count, cards) = letters.entry((first.is_some(), first.unwrap_or('#'))).or_default();
        // The Pokémon page goes where "Pokémon" sorts among the game screens under P.
        if !pokemon_listed && first == Some('P') && (!has_screen(g) || g.name.to_lowercase().as_str() > "pokémon") {
            cards.push_str(POKEMON_ITEM);
            *count += 1;
            pokemon_listed = true;
        }
        *count += 1;
        cards.push_str(&item);
    }

    if !pokemon_listed {
        let (count, cards) = letters.entry((true, 'P')).or_default();
        cards.push_str(POKEMON_ITEM);
        *count += 1;
    }
    let mut cards = String::new();
    for ((_, c), (count, items)) in &letters {
        let _ = write!(cards, r#"<details class="letter"><summary>{c} <span>({count})</span></summary><ul class="grid">{items}</ul></details>"#);
    }
    if !console_cards.is_empty() {
        cards.push_str("<h3>Console games</h3>");
    }
    for (at, (count, items)) in &console_cards {
        let title = crate::games::CONSOLES.get(*at).map_or("Other", |c| c.0);
        let _ = write!(cards, r#"<details class="letter"><summary>{title} <span>({count})</span></summary><ul class="grid">{items}</ul></details>"#);
    }
    let full = GAMES.iter().filter(|g| has_screen(g)).count();
    let desc = format!("Universal Save Editor is a free, offline save editor and manager for {}+ ({}) PC games, including Elden Ring, Skyrim, Stardew Valley and Cyberpunk 2077. Nothing is uploaded.", GAMES.len() / 10 * 10, GAMES.len());
    let body = format!(
        r#"<section class="hero"><div><h1>Universal Save Editor: a free save editor and manager for {n} games</h1>
<p class="lead">Universal Save Editor edits and manages your saves on your own computer (backups, named save slots, restore, and back up and start a new game): no uploads, no accounts, no rate limits. It's free, and always will be.</p>
<div class="badges"><span class="badge free">Free forever</span><span class="badge">Offline</span><span class="badge">Open source</span><span class="badge">Windows · Linux · Browser</span></div>
<a class="btn" href="{DOWNLOAD}">Download for free</a><a class="btn alt" href="{WEB_APP}">Use it in your browser</a></div><div class="mock" aria-hidden="true"><div class="bar"><i></i><i></i><i></i></div><div class="row"><div class="k"><span>Gold</span><b>99,999</b></div><div class="t" style="--w:92%"></div></div><div class="row"><div class="k"><span>Level</span><b>60</b></div><div class="t" style="--w:60%"></div></div><div class="row"><div class="k"><span>Crafting materials</span><b>Max</b></div><div class="t" style="--w:100%"></div></div></div></section>
<section class="feature"><div><h2>Save manager built in</h2><p>Every time you save, the editor backs up the file first, automatically. Make a named backup before a boss or a big choice, restore any backup with one click, compare two of them, or put a save aside to start a new game and come back to it later. Backups stay next to your saves, on your computer.</p><a class="btn alt" href="save-manager/">How the save manager works</a></div></section>
<div class="card"><strong>Don't want to run a program?</strong> <a href="{WEB_APP}">The browser version</a> is the same editor, running inside the web page on your own computer: your save is never uploaded. It can't search your folders for saves or back them up for you, so open the file yourself and keep a copy; saving downloads the edited file. A few saves need the downloaded app (SQLite saves, and Palworld or Hogwarts Legacy saves).</div>
<p><strong>Verify your download:</strong> every release has a <code>SHA256SUMS.txt</code>. Compare it with <code>sha256sum UniversalSaveEditor-&lt;version&gt;-linux-x64.tar.gz</code> (Linux) or <code>certutil -hashfile UniversalSaveEditor-&lt;version&gt;-windows-x64.exe SHA256</code> (Windows). The builds are not code-signed, so Windows may warn about a new file; a matching checksum shows it is the one published here. If Windows shows "Windows protected your PC", the file is new and unsigned, not harmful: click <em>More info</em>, then <em>Run anyway</em>.</p>
<p><strong>Install with Scoop (Windows):</strong> <code>scoop install https://raw.githubusercontent.com/Lucasifium/universal-save-editor/main/packaging/scoop/universal-save-editor.json</code> installs it with a Start menu shortcut and checks the download for you; <code>scoop update universal-save-editor</code> gets new releases.</p>
<figure class="shot"><picture><source media="(max-width: 600px)" type="image/webp" srcset="screenshot-phone.webp" width="780" height="1500"><source media="(max-width: 600px)" srcset="screenshot-phone.png" width="780" height="1500"><source type="image/webp" srcset="screenshot.webp"><img src="screenshot.png" width="2000" height="1322" alt="The editor with a save open: named values in groups, with number fields, text fields and switches" loading="lazy" decoding="async"></picture><figcaption>A save open in the editor: every value has a name, and numbers, text and switches are edited in place.</figcaption></figure>
<h2>Why download a save editor?</h2>
{WHY}
<h2>Browse by genre</h2>
<ul class="grid genres">{genre_links}</ul>
<h2 id="games">Games ({n})</h2>
<p>{full} games open on a screen made for them; the rest can be edited value by value. Type to find yours, or open the full list:</p>
<input id="q" type="search" placeholder="Find a game" aria-label="Find a game">
<details id="all"><summary>Show all {n} supported games</summary>
<div id="list">{cards}</div>
</details>
<p class="empty" id="none">No game matches. The editor can still open saves of games that aren't listed: try it, or <a href="{REPO}/issues">ask for the game</a>.</p>
<script>var all=document.getElementById('all');if(location.hash==='#games')all.open=true;window.addEventListener('hashchange',function(){{if(location.hash==='#games')all.open=true}});document.getElementById('q').addEventListener('input',function(){{var q=this.value.toLowerCase().trim(),n=0;if(q)all.open=true;document.querySelectorAll('#list details.letter').forEach(function(d){{var h=0;d.querySelectorAll('li').forEach(function(li){{var hit=li.querySelector('span').textContent.toLowerCase().indexOf(q)>=0;li.style.display=hit?'':'none';if(hit){{h++;n++}}}});d.style.display=h?'':'none';d.open=!!q&&h>0}});document.getElementById('none').style.display=n?'none':'block'}})</script>
"#,
        n = GAMES.len(),
        genre_links = GENRES.iter().filter(|g| genre_items.contains_key(g.0)).map(|g| format!(r#"<li><a href="{}/"><span>{}</span><small>{} games</small></a></li>"#, g.1, g.2, genre_items[g.0].len())).collect::<String>(),
    );
    std::fs::write(out.join("index.html"), page(&format!("Universal Save Editor - Free Offline Save Editor and Manager for {}+ ({}) Games", GAMES.len() / 10 * 10, GAMES.len()), &desc, "", &body, &website_schema(&schema("Universal Save Editor", &desc, &format!("{SITE}/")), None))).unwrap();

    // Pages about one feature or one series rather than one game.
    for (path, title, desc, body, crumb) in [
        ("save-manager/", "Save Manager for PC Game Saves | Universal Save Editor", SAVE_MANAGER_DESC.to_string(), save_manager_body(), "Save manager"),
        ("pokemon-save-editor/", "Pokémon Save Editor (Gen 1-9) | Universal Save Editor", POKEMON_DESC.to_string(), pokemon_body(&rows, &pages), "Pokémon save editor"),
    ] {
        let dir = out.join(path.trim_end_matches('/'));
        std::fs::create_dir_all(&dir).unwrap();
        let url = format!("{SITE}/{path}");
        std::fs::write(dir.join("index.html"), page(title, &desc, path, &body, &website_schema(&schema(&format!("Universal Save Editor ({crumb})"), &desc, &url), Some((crumb, &url))))).unwrap();
        urls.push(url);
    }

    for (key, slug, name, about) in GENRES {
        let Some(items) = genre_items.get_mut(key) else { continue };
        items.sort();
        let list: String = items.iter().map(|i| i.1.as_str()).collect();
        let others: String = GENRES.iter().filter(|g| g.0 != *key).map(|g| format!(r#"<a href="../{}/">{}</a>"#, g.1, g.2)).collect::<Vec<_>>().join(" · ");
        let path = format!("{slug}/");
        let title = format!("{name} Save Editors: {} games | Universal Save Editor", items.len());
        let desc = format!("Free, offline save editor for {} {about}. Edit money, items, stats and more, with a backup before every save.", items.len());
        let body = format!(
            r#"<section class="hero game"><div><h1>{name} save editors</h1>
<p class="lead">Universal Save Editor opens the saves of {n} {about}. Pick your game to see what it can change and where its saves are. It's free, works offline and backs up every save before changing it.</p>
<a class="btn" href="{DOWNLOAD}">Download for free</a><a class="btn alt" href="../{WEB_APP}">Use it in your browser</a></div></section>
<ul class="grid">{list}</ul>
<h2>Other genres</h2>
<p>{others}</p>
<p><a href="../#games">See all {total} supported games</a></p>
"#,
            n = items.len(),
            total = GAMES.len(),
        );
        let dir = out.join(slug);
        std::fs::create_dir_all(&dir).unwrap();
        let url = format!("{SITE}/{path}");
        let crumb = format!("{name} save editors");
        std::fs::write(dir.join("index.html"), page(&title, &desc, &path, &body, &website_schema(&schema(&format!("Universal Save Editor ({crumb})"), &desc, &url), Some((&crumb, &url))))).unwrap();
        urls.push(url);
    }

    let mut sm = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");
    for u in &urls {
        let _ = writeln!(sm, "<url><loc>{u}</loc></url>");
    }
    sm.push_str("</urlset>\n");
    std::fs::write(out.join("sitemap.xml"), sm).unwrap();
    // GitHub Pages serves this for unknown addresses.
    let nf = page("Page not found | Universal Save Editor", "That page doesn't exist. Find your game on the Universal Save Editor home page.", "", "<h1>Page not found</h1>\n<p class=\"lead\">That address doesn't exist. Every supported game is listed on the <a href=\"/#games\">home page</a>.</p>", &schema("Universal Save Editor", "Free, offline save editor and manager for PC games.", &format!("{SITE}/")))
        .replace("<meta charset=\"utf-8\">", "<meta charset=\"utf-8\">\n<meta name=\"robots\" content=\"noindex\">")
        .replace("href=\"./", "href=\"/");
    std::fs::write(out.join("404.html"), nf).unwrap();
    let signing = r#"<h1>Code signing policy</h1>
<p class="lead">How the Windows downloads of Universal Save Editor are built and signed.</p>
<p>Every download on the <a href="https://github.com/Lucasifium/universal-save-editor/releases">Releases page</a> is built by GitHub Actions from the source code in the <a href="https://github.com/Lucasifium/universal-save-editor">public repository</a>, by the release workflow kept in that repository. Nothing is built or uploaded by hand. Each release lists a <code>SHA256SUMS.txt</code> so a download can be checked against what the workflow produced.</p>
<p>The Windows downloads are not code-signed yet, so Windows may warn about them the first time they run; the checksums above are the way to check a download. Signing will be added once the project qualifies for a free open-source signing program.</p>
<h2>Team roles</h2>
<ul>
<li>Committers and reviewers: <a href="https://github.com/Lucasifium">Lucasifium</a></li>
<li>Approvers: <a href="https://github.com/Lucasifium">Lucasifium</a></li>
</ul>
<h2>Privacy</h2>
<p>This program will not transfer any information to other networked systems unless specifically requested by the user or the person installing or operating it. It edits save files on your own computer and makes no network connections of its own; the Help menu only opens web pages in your browser when you choose to.</p>"#;
    let dir = out.join("code-signing");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("index.html"), page("Code signing policy | Universal Save Editor", "How the Universal Save Editor downloads are built and signed.", "code-signing/", signing, &schema("Universal Save Editor", "Free, offline save editor and manager for PC games.", &format!("{SITE}/")))).unwrap();
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
    for f in ["README.md", "GAMES.md", "Cargo.toml"] {
        let text = std::fs::read_to_string(root.join(f)).unwrap();
        assert!(with_game_count(&text) == text, "{f} has a stale game count: run `cargo test --release --lib build_site -- --ignored`");
    }
}

/// Every row of the README's game-screens list sits in its own letter's collapsible table (a row
/// outside one renders as a run-on paragraph on GitHub).
#[test]
fn readme_screens_are_grouped() {
    let text = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("GAMES.md")).unwrap();
    assert!(group_readme_screens(&text) == text, "GAMES.md's game screens are not grouped by letter: run `cargo test --release --lib build_site -- --ignored`");
}

/// The game pictures stay small: 40 / 90 / 140 KB at most per size, and 36 MB in all; every one has
/// all three sizes and a height in the index.
#[test]
fn game_pictures_stay_small() {
    let mut total = 0u64;
    let mut names = std::collections::BTreeSet::new();
    for f in std::fs::read_dir(shots_dir()).into_iter().flatten().flatten() {
        let n = f.file_name().to_string_lossy().to_string();
        if n == "index.txt" {
            continue;
        }
        let size = f.metadata().unwrap().len();
        total += size;
        let limit = if n.ends_with("-640.webp") { 40_000 } else if n.ends_with("-1200.webp") { 90_000 } else { 140_000 };
        assert!(n.ends_with(".webp") && size <= limit, "{n} is {size} bytes");
        names.insert(n);
    }
    let heights = shot_heights();
    for n in &names {
        let stem = n.rsplit_once('-').map_or(n.as_str(), |(a, _)| a);
        assert!(heights.contains_key(stem), "{n} is not in index.txt");
    }
    for key in heights.keys() {
        for size in ["640", "1200", "2000"] {
            assert!(names.contains(&format!("{key}-{size}.webp")), "{key} is in index.txt but {key}-{size}.webp is missing");
        }
    }
    assert!(total <= 36_000_000, "pictures weigh {total} bytes");
}
