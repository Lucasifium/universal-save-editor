//! Tycoon and management games for the table-driven screens: one `Spec` per game, as in `specs.rs`.

use super::{Section, Spec, Table};
use crate::i18n::tx;

pub static TYCOON: &[Spec] = &[JURASSIC_WORLD_EVOLUTION, GAME_DEV_TYCOON, ROLLERCOASTER_TYCOON];

// ------------------------------------------------------------------ Jurassic World Evolution

const JURASSIC_WORLD_EVOLUTION: Spec = Spec {
    name: "Jurassic World Evolution",
    tab: "Islands",
    accent: (230, 140, 40),
    kind: Some(crate::fmt::jwe::KIND),
    markers: &["/Money"],
    subtitle: tx("A .career save from Saved Games\\Frontier Developments\\Jurassic World Evolution\\<id>\\Saves (the .sandbox saves hold no career money). The money the career keeps for each island, up to the 100,000,000 the game reaches. Everything else in the save is kept as it was, with the file's checksum written again. Close the game first."),
    tiles: &[],
    sections: &[
        Section {
            title: tx("Money kept for each island"),
            rows: &[("Tacano", "/Money/Tacano"), ("Matanceros", "/Money/Matanceros"), ("Nublar", "/Money/Nublar"), ("Muerta", "/Money/Muerta"), ("Sorna", "/Money/Sorna"), ("Pena", "/Money/Pena")],
        },
        Section {
            title: tx("Career"),
            rows: &[("Island", "/Career/Island"), ("Hours played", "/Career/Hours played"), ("Minutes played", "/Career/Minutes played")],
        },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Game Dev Tycoon

const GAME_DEV_TYCOON: Spec = Spec {
    name: "Game Dev Tycoon",
    tab: "Studio",
    accent: (70, 160, 230),
    kind: Some(crate::fmt::gdt::KIND),
    markers: &["/Saves/0/Cash"],
    subtitle: tx("file__0.localstorage from AppData\\Local\\Game Dev Tycoon\\Local Storage (the game keeps its saves like a web page does). Each saved company's cash, fans, research points, staff limit and level; the load menu's own copy of cash and fans is kept equal. Close the game first."),
    tiles: &[],
    sections: &[],
    lists: &[],
    tables: &[Table {
        title: tx("Saved companies"),
        path: "/Saves",
        name: &["/Company", "/Save"],
        only: &[],
        cols: &[("Cash", "/Cash"), ("Fans", "/Fans"), ("Research points", "/Research points"), ("Staff limit", "/Staff limit"), ("Company level", "/Company level")],
        names: &[],
    }],
};

// ------------------------------------------------------------------ RollerCoaster Tycoon

const ROLLERCOASTER_TYCOON: Spec = Spec {
    name: "RollerCoaster Tycoon",
    tab: "Park",
    accent: (230, 90, 70),
    kind: Some(crate::fmt::sv4::KIND),
    markers: &["/Park/Cash", "/Park/Bank loan"],
    subtitle: tx("A saved park (.SV4) from the game's Saved Games folder: Added Attractions and Loopy Landscapes parks too. The park's cash, bank loan, maximum loan and entrance fee. The file is unpacked and packed again on save. Close the game first."),
    tiles: &[("Cash", "/Park/Cash"), ("Bank loan", "/Park/Bank loan")],
    sections: &[Section {
        title: tx("Park finances"),
        rows: &[("Cash", "/Park/Cash"), ("Bank loan", "/Park/Bank loan"), ("Maximum loan", "/Park/Maximum loan"), ("Entrance fee (tenths of a unit)", "/Park/Entrance fee (tenths of a unit)")],
    }],
    lists: &[],
    tables: &[],
};
