//! WORLD OF HORROR (`%LOCALAPPDATA%\wohgame\save.ito`, a plain GameMaker INI file).
//!
//! Unlocks are not stored as "yes": when a run ends the game writes each unlocked
//! achievement, mystery ending, enemy, event and pantheon entry as that entry's own code,
//! and its unlock checks compare against the code. The codes below are the ones the game
//! writes (read from public completed saves); entries whose code isn't known are left alone.

use super::{fact, hero, panel, GameCtx};
use crate::json_view::pointer_escape;
use eframe::egui::{self, Color32, Ui};
use serde_json::{json, Value};

const ACCENT: Color32 = Color32::from_rgb(214, 196, 150);

/// `(section, [(key, code)])`.
pub const CODES: &[(&str, &[(&str, &str)])] = &[
    ("achievement", &[("18", "1901"), ("16", "3859"), ("12,1", "7853"), ("10,8", "2334"), ("10,7", "7655"), ("9,8", "3455"), ("9,7", "6574"), ("8,8", "3657"), ("8,7", "6685"), ("7,8", "1363"), ("7,7", "5744"), ("6,8", "5534"), ("6,7", "1943"), ("5,8", "1055"), ("5,7", "3857"), ("4,8", "1923"), ("4,7", "3854"), ("3,8", "9504"), ("1,1", "9475"), ("1,2", "1499"), ("1,3", "8836"), ("1,4", "5288"), ("1,5", "7566"), ("1,6", "4237"), ("2,1", "5838"), ("2,2", "7125"), ("2,3", "2444"), ("2,4", "1319"), ("2,5", "3750"), ("2,6", "6784"), ("3,1", "9684"), ("3,2", "6748"), ("3,3", "1029"), ("3,4", "6542"), ("3,5", "6013"), ("3,6", "1928"), ("4,1", "5731"), ("4,2", "9635"), ("4,3", "2860"), ("4,4", "0056"), ("4,5", "2251"), ("4,6", "5831"), ("5,1", "8665"), ("5,2", "9234"), ("5,3", "9536"), ("5,4", "0986"), ("5,5", "3768"), ("5,6", "1259"), ("6,1", "6773"), ("6,2", "2446"), ("6,3", "4530"), ("6,4", "0074"), ("6,5", "4865"), ("6,6", "1823"), ("7,1", "6932"), ("7,2", "0683"), ("7,3", "2654"), ("7,4", "8857"), ("7,5", "1234"), ("7,6", "5439"), ("8,1", "3852"), ("8,2", "2855"), ("8,3", "0685"), ("8,4", "0345"), ("8,5", "9443"), ("8,6", "2486"), ("9,1", "9653"), ("9,2", "3467"), ("9,3", "1894"), ("9,4", "0074"), ("9,5", "2357"), ("9,6", "7221"), ("10,1", "5853"), ("10,2", "2789"), ("10,3", "1688"), ("10,4", "6544"), ("10,5", "3675"), ("10,6", "7853"), ("1", "9657"), ("2", "7950"), ("3", "2284"), ("4", "6947"), ("6", "2284"), ("7", "1256"), ("10", "0986"), ("11", "5845"), ("12", "3755"), ("1,7", "5749"), ("1,8", "0110"), ("2,7", "6841"), ("2,8", "0054"), ("3,7", "1992"), ("1,0", "1980"), ("2,0", "4832"), ("3,0", "1493"), ("4,0", "2269"), ("5,0", "1612"), ("6,0", "7295"), ("7,0", "2498"), ("8,0", "3920"), ("11,1", "7372"), ("11,2", "0321"), ("11,3", "4032"), ("11,4", "3567"), ("11,5", "8756"), ("11,6", "4462"), ("11,7", "9800"), ("11,8", "3567"), ("20", "5732")]),
    ("mysteries", &[("jeane_b", "484932"), ("jeane_a", "047321"), ("teacher_b", "484932"), ("teacher_a", "047321"), ("janitor_a", "194428"), ("janitor_b", "525273"), ("janitor_c", "129168"), ("janitor_d", "668669"), ("janitor_e", "067843"), ("forest_a", "906847"), ("forest_b", "987113"), ("forest_c", "466599"), ("abnormal_a", "544557"), ("abnormal_b", "226276"), ("coma_a", "286887"), ("coma_b", "343058"), ("household_a", "306152"), ("household_b", "890826"), ("household_c", "711647"), ("sailor_a", "768384"), ("sailor_b", "938655"), ("school_a", "926612"), ("school_b", "669964"), ("school_c", "455921"), ("hospital_a", "573860"), ("hospital_b", "057631"), ("crimson_a", "357894"), ("crimson_b", "373993"), ("festival_a", "488932"), ("festival_b", "077832"), ("festival_c", "749217"), ("painting_a", "758632"), ("painting_b", "244893"), ("painting_c", "387794"), ("eels_a", "673225"), ("eels_b", "199574"), ("eels_c", "064474"), ("seaside_a", "355789"), ("nails_a", "998535"), ("nails_b", "873435"), ("ramen_a", "223635"), ("ramen_b", "759954"), ("pax_a", "117784"), ("pax_b", "779061"), ("pax_c", "854367"), ("pax03_a", "869543"), ("pax03_b", "387654"), ("pax03_c", "094754"), ("sailor_c", "684732"), ("school_d", "675932"), ("botany_a", "967485"), ("botany_b", "092354"), ("botany_c", "576943"), ("botany_d", "306775"), ("bulletin_a", "605329"), ("bulletin_b", "737553"), ("bulletin_c", "594883")]),
    ("pantheon", &[("001", "478423"), ("002", "234645"), ("003", "195573"), ("004", "593821"), ("005", "555039")]),
    ("enemies", &[("097", "hjfur"), ("096", "vgghh"), ("095", "tghhh"), ("094", "ngfvv"), ("093", "mjklh"), ("092", "iygfv"), ("091", "bjjhg"), ("090", "vfdss"), ("089", "qwdds"), ("088", "gjdll"), ("087", "potui"), ("086", "znvbf"), ("085", "bmkgj"), ("084", "tufjg"), ("083", "vjdls"), ("082", "akguf"), ("081", "bhjgu"), ("001", "yutjd"), ("002", "vfder"), ("003", "rtgbv"), ("004", "vbnmj"), ("005", "jkloi"), ("006", "uyhgf"), ("007", "dsazx"), ("008", "xswed"), ("009", "dcvfd"), ("010", "dfrtg"), ("011", "ghnbv"), ("012", "cdfre"), ("013", "tghyu"), ("014", "ujhgf"), ("015", "fvbng"), ("016", "ghjkl"), ("017", "loikj"), ("018", "juyhg"), ("019", "gtyhj"), ("020", "juikl"), ("021", "nhgfr"), ("022", "rttgf"), ("023", "fdsaq"), ("024", "qwsaz"), ("025", "zxcsd"), ("026", "fvfvf"), ("027", "fgbnm"), ("028", "mjhyt"), ("029", "tyhgf"), ("030", "fghyu"), ("031", "uiopl"), ("032", "lkmnj"), ("036", "ertyu"), ("037", "ujnbg"), ("038", "ghfgd"), ("039", "dewsa"), ("040", "aqwws"), ("041", "polkj"), ("042", "juyhn"), ("043", "nmjki"), ("044", "iujhg"), ("045", "gbvcx"), ("046", "xsdew"), ("047", "werfr"), ("048", "rfgty"), ("049", "yhnbn"), ("050", "nmjhg"), ("051", "ghyuj"), ("052", "hgfds"), ("053", "asdew"), ("055", "xcvbn"), ("056", "nmkjh"), ("057", "plmko"), ("058", "ikjhg"), ("059", "fdert"), ("060", "qweds"), ("061", "hgfrd"), ("062", "hfdvf"), ("063", "ssddf"), ("064", "ytrfd"), ("065", "bvchg"), ("066", "jytfd"), ("067", "sdftt"), ("068", "jhnnm"), ("069", "lkjgf"), ("070", "sdvty"), ("071", "dfytu"), ("072", "nbvff"), ("073", "hyfds"), ("074", "njuyt"), ("075", "zzxas"), ("076", "renia"), ("077", "ggfrt"), ("078", "ftyih"), ("079", "hhyyu"), ("080", "hgjki")]),
    ("events", &[("160", "fgsw"), ("157", "apls"), ("156", "vkdh"), ("151", "afgd"), ("001", "nauth"), ("002", "hsgso"), ("003", "gvsan"), ("004", "jhoyw"), ("005", "qeiou"), ("006", "rhyop"), ("007", "ijgpo"), ("008", "xmlkx"), ("009", "nlvzx"), ("010", "vzxvp"), ("011", "lpzsk"), ("012", "donas"), ("013", "gjnab"), ("014", "sythv"), ("015", "zmueg"), ("016", "rpsuh"), ("017", "rjdew"), ("018", "adlib"), ("019", "msuai"), ("020", "trzyn"), ("022", "vhdyu"), ("023", "zmsha"), ("024", "qusly"), ("025", "abhgg"), ("026", "apudd"), ("027", "zzxnx"), ("028", "tytui"), ("029", "plkjs"), ("030", "bgfre"), ("031", "jhnbv"), ("032", "ikjhy"), ("033", "asdxc"), ("034", "yujhg"), ("035", "bvfre"), ("036", "rtgfd"), ("037", "dfvbg"), ("038", "ikmnj"), ("039", "lkjhn"), ("040", "cvbgf"), ("041", "qasdf"), ("042", "frtgh"), ("043", "yujki"), ("044", "iolkj"), ("045", "jmnhg"), ("046", "bvfcx"), ("047", "xzasd"), ("048", "derff"), ("049", "gtyhg"), ("050", "juyhg"), ("051", "gbvfd"), ("052", "dcxss"), ("053", "sdfgh"), ("054", "hnbgg"), ("055", "ghjuy"), ("056", "yuikj"), ("057", "kjmnh"), ("058", "hgbvf"), ("059", "freaw"), ("060", "qaszx"), ("061", "ujmnb"), ("062", "bgfre"), ("063", "edsaz"), ("064", "zxcvb"), ("065", "bnmlk"), ("066", "jhgfd"), ("067", "dsaqw"), ("068", "wedcx"), ("069", "xcvcx"), ("070", "cvbgt"), ("071", "yujhn"), ("072", "nmjuy"), ("074", "vccew"), ("075", "wsxza"), ("076", "yuikj"), ("077", "ertyu"), ("078", "jhgfv"), ("079", "vbgfd"), ("080", "nhyui"), ("081", "vfrtg"), ("082", "gbvfg"), ("083", "fghyu"), ("084", "ujhnm"), ("085", "mjkiu"), ("086", "ytred"), ("087", "dfgbv"), ("088", "vfder"), ("089", "rtyuj"), ("090", "jkloi"), ("091", "iujhg"), ("092", "gbnhj"), ("094", "ikmnj"), ("095", "vfgtr"), ("096", "rtghj"), ("097", "juytr"), ("098", "qazxs"), ("099", "sdewr"), ("100", "rtyhg"), ("101", "mnbgf"), ("102", "frtgh"), ("103", "hgbvc"), ("104", "xcdsw"), ("105", "edcvf"), ("106", "fgtyh"), ("107", "nmjki"), ("108", "lkiop"), ("109", "qazxs"), ("110", "sdrew"), ("111", "edcvf"), ("112", "fvbgh"), ("113", "hgtyu"), ("114", "jmnbv"), ("115", "yhjui"), ("116", "oljhg"), ("117", "fvbnm"), ("118", "ertyh"), ("119", "tghnb"), ("120", "bnhgt"), ("121", "yhnbg"), ("122", "fghds"), ("123", "derwt"), ("124", "hyuik"), ("125", "kkjhb"), ("126", "asder"), ("127", "gfttr"), ("128", "hjnnb"), ("129", "gftye"), ("130", "jkoiu"), ("131", "mnhjj"), ("132", "qqasw"), ("133", "wedsc"), ("134", "vcbdf"), ("135", "gfghh"), ("136", "yujjh"), ("137", "ghnvf"), ("138", "olkiu"), ("139", "ttghf"), ("140", "yufdc"), ("141", "fgyyt"), ("142", "djhuf"), ("143", "trans"), ("144", "vkffh"), ("145", "pskdd"), ("146", "renna"), ("147", "asuit"), ("148", "nmngh"), ("149", "tuoid"), ("161", "mkjf"), ("162", "bcsg"), ("163", "jyio"), ("164", "ndfj"), ("165", "rhds"), ("166", "zadt"), ("167", "nkoy"), ("168", "serg"), ("169", "tyjf"), ("174", "ndfj"), ("176", "zadt"), ("177", "nkoy"), ("178", "serg"), ("179", "tyjf"), ("180", "ngfd")]),
];

/// The unlock sections, as shown.
const SECTIONS: &[(&str, &str)] = &[
    ("achievement", "Achievements"),
    ("mysteries", "Mystery endings"),
    ("enemies", "Enemies"),
    ("events", "Events"),
    ("pantheon", "Pantheon"),
];

/// The settings shown: `(key, label)`.
const OPTIONS: &[(&str, &str)] = &[
    ("volume", "Music volume (0\u{2013}1)"),
    ("sfxvolume", "Sound volume (0\u{2013}1)"),
    ("palette", "Colour palette"),
    ("bit", "Colour depth"),
    ("scale", "Window scale"),
    ("ful", "Fullscreen (1 on)"),
    ("stretch", "Stretch (1 on)"),
    ("border", "Border"),
    ("filter", "Filter"),
    ("wavy", "Wavy effect"),
    ("trail", "Trail effect"),
    ("bright", "Brightness"),
    ("language", "Language"),
    ("fps", "Frame rate"),
    ("intro", "Intro"),
];

pub fn is_woh(kind: &str, root: &Value) -> bool {
    kind == "INI / config" && ["/achievement", "/mysteries", "/pantheon", "/enemies", "/events"].iter().all(|p| root.pointer(p).is_some_and(Value::is_object))
}

fn ptr(section: &str, key: &str) -> String {
    format!("/{}/{}", pointer_escape(section), pointer_escape(key))
}

fn codes(section: &str) -> &'static [(&'static str, &'static str)] {
    CODES.iter().find(|(s, _)| *s == section).map_or(&[], |(_, c)| c)
}

/// Unlocked entries and all entries of a section: anything but an empty value counts.
pub fn counts(root: &Value, section: &str) -> (usize, usize) {
    match root.get(section) {
        Some(Value::Object(m)) => (m.values().filter(|v| v.as_str().is_some_and(|s| !s.is_empty())).count(), m.len()),
        _ => (0, 0),
    }
}

/// Edits that write the known code of every entry of `section` that the save has and that
/// doesn't hold it yet.
pub fn unlock_edits(root: &Value, section: &str) -> Vec<(String, Value)> {
    codes(section)
        .iter()
        .map(|(k, c)| (ptr(section, k), *c))
        .filter(|(p, c)| root.pointer(p).is_some_and(|v| v.as_str() != Some(c)))
        .map(|(p, c)| (p, json!(c)))
        .collect()
}

pub fn show(ui: &mut Ui, cx: &mut GameCtx) {
    let who = cx.text("/town/character[0,3]").filter(|s| !s.trim().is_empty());
    let sub = match &who {
        Some(n) => format!("Unlocks and settings  \u{00B7}  last run: {n}"),
        None => "Unlocks and settings".to_string(),
    };
    hero(ui, ACCENT, "WORLD OF HORROR", &sub, |ui| {
        for (s, label) in SECTIONS {
            let (on, all) = counts(cx.root, s);
            if all > 0 {
                fact(ui, label, &format!("{on} / {all}"), ACCENT);
            }
        }
    });

    panel(ui, "Unlocks", |ui| {
        let all: Vec<_> = SECTIONS.iter().flat_map(|(s, _)| unlock_edits(cx.root, s)).collect();
        ui.horizontal_wrapped(|ui| {
            if ui.add_enabled(!all.is_empty(), egui::Button::new("Unlock everything")).on_hover_text("Writes the game's own unlock code into every achievement, mystery ending, enemy, event and pantheon entry whose code is known. Characters, outfits and the other rewards that follow from achievements unlock with them.").clicked() {
                cx.edits.extend(all.iter().cloned());
            }
            for (s, label) in SECTIONS {
                let e = unlock_edits(cx.root, s);
                if ui.add_enabled(!e.is_empty(), egui::Button::new(format!("Unlock {}", label.to_lowercase()))).clicked() {
                    cx.edits.extend(e);
                }
            }
        });
        let known: usize = CODES.iter().map(|(_, c)| c.len()).sum();
        ui.weak(format!(
            "The game stores each unlock as that entry's own code (a plain \"yes\" unlocks nothing once a run ends). {known} codes are known; entries without a known code stay as they are. Each entry can also be edited in the Values tab."
        ));
    });

    if cx.get("/town/slot[0]").is_some() {
        panel(ui, "Last run", |ui| {
            if let Some(n) = &who {
                ui.label(format!("Character: {n}"));
            }
            let items: Vec<String> = (0..10).filter_map(|i| cx.text(&format!("/town/slot[{i}]"))).filter(|s| !s.is_empty()).collect();
            if !items.is_empty() {
                ui.label(format!("Items: {}", items.join(", ")));
            }
            let board: Vec<String> = (0..10).filter_map(|i| cx.text(&format!("/town/board[{i}]"))).filter(|s| !s.is_empty() && s != "999").collect();
            if !board.is_empty() {
                ui.label(format!("Mystery board: {}", board.join(", ")));
            }
            ui.weak("The run's stats, doom and items are in the Values tab under town.");
        });
    }

    if cx.get("/options").is_some() {
        panel(ui, "Settings", |ui| {
            egui::Grid::new("woh_options").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                for (k, label) in OPTIONS {
                    cx.grid_row(ui, label, &ptr("options", k), 90.0);
                }
            });
        });
    }
    ui.weak("The save has no checksum. Close the game before editing; it writes the file when you save and exit. Checked on a public completed save, not yet tried in the game.");
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAVE: &str = "[options]\r\nvolume=\"0.150000\"\r\n[achievement]\r\n1,1=\"\"\r\n1,0=\"1980\"\r\n19=\"\"\r\n[mysteries]\r\njeane_a=\"\"\r\n[pantheon]\r\n001=\"yes\"\r\n009=\"\"\r\n[enemies]\r\n001=\"\"\r\n[events]\r\n001=\"\"\r\n[town]\r\nstatus=\"0.000000\"\r\n";

    #[test]
    fn unlocks_write_the_known_codes_only() {
        let doc = crate::fmt::ini::parse(SAVE, "ito").unwrap();
        let root = &doc.view;
        assert!(is_woh(&doc.kind_name, root));
        assert_eq!(counts(root, "achievement"), (1, 3));
        let e = unlock_edits(root, "achievement");
        // 1,0 already holds its code and 19 has none known.
        assert_eq!(e, vec![("/achievement/1,1".to_string(), json!("9475"))]);
        assert_eq!(unlock_edits(root, "pantheon"), vec![("/pantheon/001".to_string(), json!("478423"))]);
        assert_eq!(unlock_edits(root, "mysteries"), vec![("/mysteries/jeane_a".to_string(), json!("047321"))]);
        let mut edited = crate::fmt::ini::parse(SAVE, "ito").unwrap();
        for (p, v) in SECTIONS.iter().flat_map(|(s, _)| unlock_edits(root, s)) {
            edited.set(&p, v).unwrap();
        }
        let out = String::from_utf8(edited.to_bytes().unwrap()).unwrap();
        assert!(out.contains("1,1=\"9475\"\r\n") && out.contains("001=\"478423\"\r\n") && out.contains("19=\"\"\r\n"));
        assert!(out.starts_with("[options]\r\nvolume=\"0.150000\"\r\n"));
    }

    #[test]
    fn codes_have_no_duplicate_keys() {
        for (s, c) in CODES {
            let mut keys: Vec<_> = c.iter().map(|(k, _)| *k).collect();
            keys.sort();
            keys.dedup();
            assert_eq!(keys.len(), c.len(), "{s}");
        }
    }

    #[test]
    fn plain_ini_is_not_world_of_horror() {
        let doc = crate::fmt::ini::parse("[Player]\nGold=5\nName=x\n", "ini").unwrap();
        assert!(!is_woh(&doc.kind_name, &doc.view));
    }
    /// A real completed save (`WOH_SAVE=<save.ito>`): blanking every unlock and pressing
    /// "Unlock everything" gives back the save's own codes, and the file is otherwise unchanged.
    #[test]
    #[ignore]
    fn real_save_round_trip() {
        let path = std::env::var("WOH_SAVE").expect("WOH_SAVE");
        let text = std::fs::read_to_string(path).unwrap();
        let doc = crate::fmt::ini::parse(&text, "ito").unwrap();
        assert!(is_woh(&doc.kind_name, &doc.view));
        assert!(unlock_edits(&doc.view, "achievement").is_empty());
        let mut blank = crate::fmt::ini::parse(&text, "ito").unwrap();
        for (s, c) in CODES {
            for (k, _) in c.iter() {
                blank.set(&ptr(s, k), json!("")).unwrap();
            }
        }
        let blank_text = String::from_utf8(blank.to_bytes().unwrap()).unwrap();
        let mut again = crate::fmt::ini::parse(&blank_text, "ito").unwrap();
        let edits: Vec<_> = SECTIONS.iter().flat_map(|(s, _)| unlock_edits(&again.view, s)).collect();
        assert_eq!(edits.len(), CODES.iter().map(|(_, c)| c.len()).sum::<usize>());
        for (p, v) in edits {
            again.set(&p, v).unwrap();
        }
        assert_eq!(String::from_utf8(again.to_bytes().unwrap()).unwrap(), text);
    }
}
