//! Detection, decoding and re-encoding of save-file containers.
//!
//! A save is modelled as a stack of `Layer`s (compression / encoding / encryption
//! wrappers) around a final `Content` (JSON, a structured tree, text or raw bytes).
//! Decoding peels layers off one at a time; encoding re-applies them in reverse so
//! the saved file has the same shape the game expects.

use crate::fmt;
use crate::pickle;
use crate::tree::TreeDoc;
use base64::{engine::general_purpose::STANDARD, Engine};
// bufread decoders hand back the unread remainder, so we can detect trailing data.
use flate2::{bufread::GzDecoder, bufread::ZlibDecoder, write::GzEncoder, write::ZlibEncoder, Compression};
use serde_json::Value;
use std::io::{Cursor, Read, Write};
use std::sync::Arc;

const MAX_LAYERS: usize = 10;
const BOM: &[u8] = &[0xEF, 0xBB, 0xBF];

/// Extra information for decoding: a password for encrypted saves, which entry of a
/// zip archive to open, and the file extension (some formats are only trusted by it).
#[derive(Clone, Debug, Default)]
pub struct Options {
    pub password: Option<String>,
    pub zip_entry: Option<String>,
    pub ext: String,
    /// Where the file is: RE Engine saves are keyed to the Steam account in their path.
    pub path: Option<std::path::PathBuf>,
    /// A checksum the player chose to keep fixed (Tools › Find checksum).
    pub sum: Option<fmt::sumfind::Sum>,
}

impl Options {
    pub fn for_path(path: &std::path::Path) -> Self {
        #[cfg_attr(target_arch = "wasm32", allow(unused_mut))]
        let mut ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        // A Game Pass blob is named by a GUID: take the extension from the name the game gave it.
        #[cfg(not(target_arch = "wasm32"))]
        if ext.is_empty() {
            if let Some(n) = crate::xgp::logical_name(path) {
                ext = n.rsplit_once('.').map(|(_, e)| e.to_lowercase()).filter(|e| !e.contains('/')).unwrap_or_default();
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        let sum = fmt::sumfind::remembered(path);
        #[cfg(target_arch = "wasm32")]
        let sum = None;
        Self { ext, path: Some(path.to_path_buf()), sum, ..Default::default() }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Layer {
    /// Ren'Py `.save` zip. Holds the original archive so every other entry
    /// (screenshot, metadata, signatures) is copied back untouched.
    RenpyZip { archive: Arc<Vec<u8>>, signed: bool },
    /// Ren'Py `persistent` file: zlib data followed by signature lines over the
    /// uncompressed data. Ren'Py silently discards it if the signature is wrong.
    RenpyPersistent { signed: bool },
    /// One file inside a zip archive; every other entry is copied back untouched.
    ZipEntry { archive: Arc<Vec<u8>>, name: String },
    /// UTF-8 byte-order mark in front of the text.
    Utf8Bom,
    /// UTF-16 text with a byte-order mark (converted to UTF-8 for editing).
    Utf16 { big_endian: bool },
    /// gzip stream (magic 1F 8B).
    Gzip,
    /// Raw zlib stream.
    Zlib,
    /// Headerless deflate (Balatro `.jkr`, LÖVE `love.data.compress("deflate")`).
    Deflate,
    Zstd,
    Lz4,
    Snappy,
    /// Stellar Blade's EVAS v1: an 8-byte prefix, the GVAS save, then a CRC-32 of the save.
    Evas,
    /// Prey (2017): RC4 over zlib chunks and an MD5'd XMLCPB header.
    Prey { header: Arc<Vec<u8>> },
    /// Ace Combat 8: the packed property lists inside the save, with their checksum.
    Ac8(fmt::ac8::Packed),
    Xz,
    Bzip2,
    /// Plain standard base64 text.
    Base64,
    /// Base64 wrapped into fixed-width lines.
    Base64Lines { width: usize, crlf: bool },
    /// LZString `compressToBase64` (RPG Maker MV `.rpgsave`, Twine/SugarCube).
    LzString,
    /// zlib bytes stored as one UTF-8 char per byte (RPG Maker MZ `.rmmzsave`).
    RpgMzZlib,
    /// JavaScript `escape()` encoding (TyranoScript / TyranoBuilder `.sav`).
    JsEscape,
    /// Every byte XORed with a single key (simple obfuscation).
    Xor(u8),
    /// XOR with a repeating text key some games are known to use (Slay the Spire: "key").
    XorKey(String),
    /// A JSON save encrypted with a key built into the game (Megabonk, Eiyuden Chronicle,
    /// Suikoden I & II HD).
    Keyed(fmt::keyed::Wrap),
    /// A PNG with the data in a private zlib chunk (mGBA save states: `gbAs`).
    PngChunk { png: Arc<Vec<u8>>, chunk: [u8; 4] },
    /// A bundle of named files (Barotrauma `.save`: UTF-16 name, length, bytes, …).
    FileBundle { files: Arc<Vec<(String, Vec<u8>)>>, name: String },
    /// Palworld `.sav`: header + zlib (once or twice) around an Unreal GVAS save.
    Palworld { double: bool },
    /// Palworld 0.6+ `.sav`: the same header ("PlM1") around Oodle-compressed GVAS data.
    PalworldOodle,
    /// Final Fantasy IX (Steam): every section of SavedData_ww.dat encrypted on its own.
    Ff9,
    /// Persona 5 Royal (PC): encrypted, zlib-packed game data; `head` is the file up to the data, decrypted.
    P5r { head: Arc<Vec<u8>> },
    /// Metro `.player`: "artm" + build string, one LZ4 block, CRC-32.
    MetroPlayer { head: Arc<Vec<u8>> },
    /// BioShock Infinite: MD5 header, Unreal Engine 3 zlib chunks unpacked in place.
    Bioshock { chunks: Arc<fmt::bioshock::Chunks> },
    /// Final Fantasy XV `.save`: AES-192, shuffled blocks and a tweak; `foot` is the plain footer.
    Ffxv { foot: Arc<Vec<u8>> },
    /// Forspoken `.save`: the same layer with its own key, and an MD5 of the payload.
    Forspoken { foot: Arc<Vec<u8>> },
    /// Trails to Azure / Ao no Kiseki KAI: a zstd frame, or (KAI) AES over the converted payload.
    Ao { kai: Option<Arc<Vec<u8>>> },
    /// Death Stranding 2: XORed chunks, the game data in LZ4 pieces with an MD5.
    Ds2(Arc<fmt::ds2::Wrap>),
    /// Control (Remedy): two u32 versions, then a CRC-32 of the rest.
    RemedyCrc { head: [u8; 8] },
    /// Yakuza (Dragon Engine): XORed JSON and a check of it at the end.
    Rgg(fmt::rgg::Wrap),
    /// BioShock Remastered: zlib blocks unpacked in place, packed back into the same space.
    Rgmh { original: Arc<Vec<u8>>, blocks: Arc<fmt::rgmh::Blocks> },
    /// Cairn (The Game Bakers) "COPECK" slot: a fixed-size pack of named files; one of them
    /// is edited in place (`at`, `size`).
    Copeck { pack: Arc<Vec<u8>>, name: String, entry: usize },
    /// Metro auto save: "DEZP", three sections of LZ4 blocks, CRC-32.
    MetroDezp { layout: Arc<fmt::metro::Dezp> },
    /// Hogwarts Legacy: the game's SQLite database, kept Oodle-compressed in the save's
    /// `RawDatabaseImage` byte array. `head` is the save up to the array's data (its size
    /// fields at `size_at` and `count_at`), `tail` everything after it.
    HogwartsDb { head: Arc<Vec<u8>>, size_at: usize, count_at: usize, tail: Arc<Vec<u8>>, block: u64 },
    /// Easy Save 3 (Unity) AES-128-CBC encryption.
    Es3 { password: String, iv: Vec<u8> },
    /// Godot `FileAccess.open_encrypted_with_pass`.
    GodotEnc(fmt::crypto::GodotEnc),
    /// Terraria player (`.plr`) encryption.
    Terraria,
    /// Supergiant (Hades, Hades II) `.sav`: checksummed header + LZ4 game state.
    Supergiant { header: Arc<Vec<u8>> },
    /// One `USER_DATA` entry of a FromSoftware save (Elden Ring, Dark Souls, Sekiro):
    /// MD5-checked and, in Dark Souls II/III, AES-encrypted.
    Bnd4 { archive: Arc<Vec<u8>>, name: String, key: Option<[u8; 16]>, iv: Vec<u8> },
    /// One chunk of a Minecraft region file (`.mca`).
    Region { archive: Arc<Vec<u8>>, index: usize, kind: u8 },
    /// Hollow Knight `user*.dat`: .NET string of base64 AES-encrypted JSON.
    HollowKnight,
    FalloutShelter,
    /// Persona 3 Reload Steam save (XOR key and bit permutation over a GVAS file).
    P3rSteam,
    /// Antimatter Dimensions export text (platform index, three letter version).
    Antimatter { kind: u8, version: String },
    /// JSON held in JSON strings: the pointers of the strings opened, and whether the file is indented.
    NestedJson { paths: Vec<String>, pretty: bool },
    /// Capcom RE Engine "Mandarin" encryption (index into `mandarin::GAMES`, user id); the
    /// original file supplies the header and footer bytes when writing it back.
    Mandarin { original: Arc<Vec<u8>>, game: usize, user: u64 },
    /// Capcom RE Engine "Lime" encryption (Resident Evil 4, 2023): the file it came from (for
    /// its key banks) and the user id.
    Lime { original: Arc<Vec<u8>>, user: u64 },
    /// Capcom RE Engine "Citrus" encryption (Monster Hunter Rise on Steam): the file it came
    /// from (for its block keys), the curve and the Steam ID.
    Citrus { original: Arc<Vec<u8>>, curve: usize, steam: u64 },
    /// A Windows registry export (Unity PlayerPrefs): its values as JSON.
    RegFile(Arc<fmt::regfile::Meta>),
    /// A Flash / AIR shared object (`.sol`): the original file, whose values are replaced.
    Sol { original: Arc<Vec<u8>> },
    /// The Long Dark slot file: LZF JSON holding more LZF/base64 and JSON-in-string levels.
    Tld { wrapped: fmt::tld::Wrapped },
    /// Civilization VII: header, chunked zlib, footer.
    Civ7 { info: fmt::civ7::Info },
    /// Astroneer container: zlib streams of a GVAS file.
    Astroneer,
    /// Satisfactory: a header, then zlib chunks.
    Satisfactory { info: fmt::uepack::SatInfo },
    /// Stoneshard `data.sav`: zlib, JSON, then an MD5 with a salt made from the folder names.
    Stoneshard { salt: String },
    /// Minecraft Dungeons: `D001` header, AES-256-ECB, JSON.
    Mcd,
    /// Final Fantasy Pixel Remaster slot: base64, Rijndael-256, deflate, JSON with JSON in strings.
    Ffpr { wrapped: fmt::tld::Wrapped, bom: bool },
    /// Haxe serialization (OpenFL and HaxeFlixel saves): the original text, whose values are
    /// replaced.
    Haxe { original: Arc<Vec<u8>> },
    /// Dead Cells: the 59-byte header; the chunks are packed again under a new SHA-1.
    DeadCells { header: Arc<Vec<u8>> },
    /// One save on a PlayStation memory card image: the image, where the raw card starts in
    /// it, and the save's blocks.
    Ps1Card { image: Arc<Vec<u8>>, offset: usize, blocks: Vec<usize> },
    /// Koei Tecmo "Katana" encryption: the game and the whole save decrypted (header and keys).
    Katana { kind: fmt::katana::Kind, plain: Arc<Vec<u8>> },
    /// LEGO Batman: Legacy of the Dark Knight: the whole save RC4-encrypted (an Unreal GVAS save).
    LegoTt,
    /// A checksum found with Tools › Find checksum, written again on save.
    Sum(fmt::sumfind::Sum),
    /// One file on a PlayStation 2 memory card image: the image and the file.
    Ps2Card { image: Arc<Vec<u8>>, file: fmt::memcard::Ps2File },
    /// A console save unpacked by its patch file's own codes (decrypt, decompress), packed
    /// again by the rest of them; `var` is the unpacked stream shown, if the codes made one.
    Apollo { id: String, file: String, var: Option<String>, original: Arc<Vec<u8>> },
    /// A Nexus `.DCI` Dreamcast save: the card's directory entry in front of the blocks.
    Dci { header: Arc<Vec<u8>> },
    /// A Dreamcast VMS file: its header CRC (at `off`) shows as zeros and is written on save.
    Vms { off: usize },
    /// A single PS1/PS2 save exported by a cheat device or PC tool; `name` is the file open.
    PsSave { kind: fmt::pssave::Kind, original: Arc<Vec<u8>>, name: String },
    /// A single Sega Saturn save exported by a PC or flash-cart tool.
    Saturn { kind: fmt::saturn::Kind, original: Arc<Vec<u8>>, name: String },
    /// One save in a Saturn backup memory image, written back to the bytes it was read from.
    SaturnImage { kind: fmt::saturn::Image, image: Arc<Vec<u8>>, save: fmt::saturn::Save },
    /// FANTASIAN Neo Dimension `root.json`: one encrypted record of it.
    Fantasian { original: Arc<Vec<u8>>, record: fmt::fantasian::Record },
    /// Kingdom Hearts III (PC): AES under the owner's account id, a CRC and an MD5.
    Kh3 { account: String },
    /// DeSmuME `.dsv`: the footer after the raw DS save, kept as it is.
    Dsv { footer: Arc<Vec<u8>> },
    /// Civilization V `.Civ5Save`: an uncompressed header, then the game as one zlib stream
    /// cut into chunks of up to 64 KiB, each after its u32 length (`chunked`; very old saves
    /// may hold one plain stream), then (older builds) a trailer kept as it is.
    Civ5 { header: Arc<Vec<u8>>, chunked: bool, trailer: Arc<Vec<u8>> },
    /// CD Projekt RED's chunked saves (Cyberpunk 2077 `sav.dat`, The Witcher 3 `.sav`): a
    /// header with an `FZLC` table of LZ4 chunks, then a footer.
    /// `tagged`: each chunk starts "4ZLX" and its unpacked size (Cyberpunk).
    Cdpr { prefix: Arc<Vec<u8>>, fzlc: usize, sizes: Vec<u32>, footer: Arc<Vec<u8>>, tagged: bool },
    /// XCOM: Enemy Unknown / Within and XCOM 2 (incl. War of the Chosen): a header, then
    /// Unreal chunks of LZO1X data. `crc_at`: where the header keeps the CRC of the
    /// compressed data; the header has its own CRC too (both CRC-32/BZIP2).
    Xcom { header: Arc<Vec<u8>>, crc_at: usize, xcom2: bool, sizes: Vec<u32>, block: u32 },
    /// The Sims 2 neighbourhood package (DBPF 1.x): the family and Sim resources are shown
    /// as one flat document; changed ones are stored uncompressed when saving.
    Dbpf { original: Arc<Vec<u8>> },
    /// The Sims 4 save (DBPF 2.1 with one protobuf resource), shown as households and Sims.
    Dbpf4 { original: Arc<Vec<u8>> },
    /// Mass Effect Legendary Edition (ME1) save: an Unreal chunk of zlib blocks, then the
    /// CRC-32/BZIP2 of everything before it, 1, and the unpacked size.
    Me1Le { block: u32 },
    /// Mass Effect (2008): "RGMH", a version, the header's length, the save image; then a zip.
    Me1Header { header: Arc<Vec<u8>> },
    /// Paradox `SAV…` save (Crusader Kings III, Victoria 3, Imperator: Rome, Europa
    /// Universalis V): a header line, sometimes the metadata, then from `zip_at` a zip
    /// archive that holds `gamestate`.
    PdxSav { original: Arc<Vec<u8>>, zip_at: usize },
    /// Pathfinder (Owlcat) `.zks`: header, player and party JSON shown as one document.
    Owlcat { archive: Arc<Vec<u8>> },
    /// Borderlands 2 / The Pre-Sequel `.sav`: SHA-1, size, one LZO1X block around the
    /// "WSG" character data. Unchanged data is written back as the original file.
    Borderlands { original: Arc<Vec<u8>>, plain: Arc<Vec<u8>> },
    /// Grim Dawn character: rolling-XOR encryption with checksummed blocks; `plan` says how
    /// each part of the plain data is encrypted again.
    GrimDawn { seed: u32, plan: Arc<Vec<fmt::grimdawn::Seg>> },
    /// Torchlight II character (version 0x44): scrambled nibbles, a checksum and the file size.
    Torchlight2,
    /// Larian `.lsv` (Divinity: Original Sin 2, Baldur's Gate 3): one LSF file of the
    /// package, unpacked; packed again with the package's checksum on save.
    Larian { archive: Arc<Vec<u8>>, file: String },
    /// Wasteland 3 `.xml`: a `Key:=Value` text header, then the XML packed with LZF.
    Wasteland3 { original: Arc<Vec<u8>> },
}

impl Layer {
    pub fn name(&self) -> String {
        match self {
            Layer::RenpyZip { .. } => "Ren'Py save archive".into(),
            Layer::RenpyPersistent { .. } => "Ren'Py persistent (zlib + signature)".into(),
            Layer::ZipEntry { name, .. } => format!("zip archive ({name})"),
            Layer::Utf8Bom => "UTF-8 BOM".into(),
            Layer::Utf16 { big_endian } => format!("UTF-16 {}", if *big_endian { "BE" } else { "LE" }),
            Layer::Gzip => "gzip".into(),
            Layer::Zlib => "zlib".into(),
            Layer::Deflate => "deflate".into(),
            Layer::Zstd => "zstd".into(),
            Layer::Lz4 => "LZ4".into(),
            Layer::Snappy => "Snappy".into(),
            Layer::Evas => "EVAS envelope (CRC-32 written again)".into(),
            Layer::Prey { .. } => "Prey save (RC4, zlib chunks, MD5 written again)".into(),
            Layer::Ac8(_) => "Ace Combat 8 packed data (checksum written again)".into(),
            Layer::Xz => "xz".into(),
            Layer::Bzip2 => "bzip2".into(),
            Layer::Base64 => "Base64".into(),
            Layer::Base64Lines { .. } => "Base64 (wrapped lines)".into(),
            Layer::LzString => "LZString".into(),
            Layer::RpgMzZlib => "zlib-as-text (RPG Maker MZ)".into(),
            Layer::JsEscape => "URL-escaped (TyranoScript)".into(),
            Layer::Xor(k) => format!("XOR 0x{k:02X}"),
            Layer::XorKey(k) => match KNOWN_XOR_KEYS.iter().find(|(_, key)| key.as_bytes() == k.as_bytes()) {
                Some((game, _)) if !game.is_empty() => format!("XOR with {game}'s key"),
                _ => format!("XOR \u{201C}{k}\u{201D}"),
            },
            Layer::Keyed(w) => w.label().into(),
            Layer::PngChunk { chunk, .. } => format!("mGBA save state ({})", String::from_utf8_lossy(chunk)),
            Layer::FileBundle { name, .. } => format!("file bundle ({name})"),
            Layer::Palworld { .. } => "Palworld container".into(),
            Layer::PalworldOodle => "Palworld container (Oodle)".into(),
            Layer::Ff9 => "Final Fantasy IX save encryption".into(),
            Layer::P5r { .. } => "Persona 5 Royal save (encrypted, zlib)".into(),
            Layer::MetroPlayer { .. } => "Metro level save (LZ4)".into(),
            Layer::Rgmh { blocks, .. } => format!("BioShock Remastered save ({} zlib blocks)", blocks.len()),
            Layer::RemedyCrc { .. } => "Control save (CRC-32)".into(),
            Layer::Ffxv { .. } => "Final Fantasy XV save encryption".into(),
            Layer::Ao { kai } => if kai.is_some() { "Ao no Kiseki KAI save encryption (AES)".into() } else { "Trails to Azure save (zstd)".into() },
            Layer::Forspoken { .. } => "Forspoken save encryption (AES, MD5)".into(),
            Layer::Ds2(_) => "Death Stranding 2 save (XOR, LZ4, MD5)".into(),
            Layer::Rgg(w) => format!("Yakuza save (Dragon Engine: {})", fmt::rgg::games(w)),
            Layer::Bioshock { chunks } => format!("BioShock Infinite save (MD5, {} zlib chunks)", chunks.len()),
            Layer::Copeck { name, .. } => format!("Cairn save pack ({name})"),
            Layer::MetroDezp { .. } => "Metro auto save (LZ4)".into(),
            Layer::HogwartsDb { .. } => "Hogwarts Legacy save (database, Oodle)".into(),
            Layer::Es3 { .. } => "Easy Save 3 encryption".into(),
            Layer::GodotEnc(_) => "Godot encryption".into(),
            Layer::Terraria => "Terraria player encryption".into(),
            Layer::Supergiant { .. } => "Supergiant save (checksum + LZ4)".into(),
            Layer::Bnd4 { name, key, .. } => format!("FromSoftware save ({name}{})", if key.is_some() { ", encrypted" } else { "" }),
            Layer::Region { index, .. } => format!("Minecraft region (chunk {},{})", index % 32, index / 32),
            Layer::HollowKnight => "Hollow Knight encryption".into(),
            Layer::FalloutShelter => "Fallout Shelter save encryption".into(),
            Layer::P3rSteam => "Persona 3 Reload save encryption".into(),
            Layer::Antimatter { .. } => "Antimatter Dimensions export text".into(),
            Layer::NestedJson { paths, .. } => format!("JSON inside JSON strings ({})", paths.len()),
            Layer::Mandarin { game, .. } => format!("{} save encryption", fmt::mandarin::GAMES[*game].title),
            Layer::Lime { .. } => "Resident Evil 4 save encryption".into(),
            Layer::Citrus { .. } => "Monster Hunter Rise save encryption".into(),
            Layer::RegFile(_) => "Registry values (Unity PlayerPrefs)".into(),
            Layer::Sol { .. } => "Flash shared object (.sol)".into(),
            Layer::Mcd => "Minecraft Dungeons save encryption (AES)".into(),
            Layer::Stoneshard { .. } => "Stoneshard save (zlib and MD5)".into(),
            Layer::Astroneer => "Astroneer save (zlib)".into(),
            Layer::Satisfactory { .. } => "Satisfactory save (zlib chunks)".into(),
            Layer::Civ7 { .. } => "Civilization VII save (zlib chunks)".into(),
            Layer::Tld { .. } => "The Long Dark save (nested LZF)".into(),
            Layer::Ffpr { .. } => "Final Fantasy Pixel Remaster save encryption".into(),
            Layer::Haxe { .. } => "Haxe serialized save (OpenFL, HaxeFlixel)".into(),
            Layer::DeadCells { .. } => "Dead Cells save (zlib, SHA-1)".into(),
            Layer::LegoTt => "LEGO Batman: Legacy of the Dark Knight save encryption".into(),
            Layer::Katana { kind, .. } => format!("Koei Tecmo encryption ({})", kind.name()),
            Layer::Sum(s) => format!("Checksum kept fixed ({})", s.describe()),
            Layer::Ps2Card { file, .. } => format!("PlayStation 2 memory card ({})", file.name),
            Layer::PsSave { kind, name, .. } => format!("{} ({name})", kind.label()),
            Layer::Apollo { id, .. } => format!("Console save unpacked by its patch file ({id})"),
            Layer::Dci { .. } => "Dreamcast save (.dci, Nexus)".into(),
            Layer::Vms { .. } => "Dreamcast VMU file (CRC kept right)".into(),
            Layer::Saturn { kind, name, .. } => format!("{} ({name})", kind.label()),
            Layer::SaturnImage { kind, save, .. } => format!("{} ({})", kind.label(), save.name),
            Layer::Fantasian { record, .. } => format!("FANTASIAN Neo Dimension save ({})", record.name),
            Layer::Kh3 { .. } => "Kingdom Hearts III save encryption".into(),
            Layer::Dsv { .. } => "DeSmuME save (.dsv footer)".into(),
            Layer::Ps1Card { blocks, .. } => format!("PlayStation memory card (save in block {})", blocks[0]),
            Layer::Civ5 { .. } => "Civilization V save (compressed game data)".into(),
            Layer::Cdpr { sizes, .. } => format!("CD Projekt RED save ({} LZ4 chunks)", sizes.len()),
            Layer::Me1Le { .. } => "Mass Effect (Legendary Edition) save (zlib)".to_string(),
            Layer::Me1Header { .. } => "Mass Effect save (image header)".to_string(),
            Layer::PdxSav { .. } => "Paradox save archive (gamestate)".to_string(),
            Layer::Owlcat { .. } => "Pathfinder save archive".to_string(),
            Layer::Borderlands { .. } => "Borderlands save (SHA-1, LZO)".to_string(),
            Layer::GrimDawn { .. } => "Grim Dawn character (encrypted, checksummed blocks)".into(),
            Layer::Torchlight2 => "Torchlight II save (scrambled, checksum)".into(),
            Layer::Larian { file, .. } => format!("Larian save package ({file})"),
            Layer::Wasteland3 { .. } => "Wasteland 3 save (LZF)".to_string(),
            Layer::Dbpf { .. } => "The Sims 2 package (DBPF)".to_string(),
            Layer::Dbpf4 { .. } => "The Sims 4 save (DBPF 2.1)".to_string(),
            Layer::Xcom { xcom2, sizes, .. } => format!("{} save ({} LZO chunks)", if *xcom2 { "XCOM 2" } else { "XCOM: Enemy Unknown" }, sizes.len()),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Content {
    Json { value: Value, pretty: bool },
    Text(String),
    Binary(Vec<u8>),
    /// Any structured format with an editable tree (Ren'Py, XML, NBT, …).
    Tree(Box<TreeDoc>),
}

impl Content {
    pub fn kind(&self) -> String {
        match self {
            Content::Json { .. } => "JSON".into(),
            Content::Text(_) => "Text".into(),
            Content::Binary(_) => "Binary".into(),
            Content::Tree(t) => t.kind_name.clone(),
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        Ok(match self {
            Content::Json { value, pretty } => {
                let s = if *pretty {
                    serde_json::to_string_pretty(value)
                } else {
                    serde_json::to_string(value)
                };
                s.map_err(|e| e.to_string())?.into_bytes()
            }
            Content::Text(t) => t.clone().into_bytes(),
            Content::Binary(b) => b.clone(),
            Content::Tree(t) => t.to_bytes()?,
        })
    }
}

pub struct Decoded {
    pub layers: Vec<Layer>,
    pub content: Content,
    /// Entries of a generic zip archive (to let the player pick another one).
    pub zip_entries: Vec<String>,
    /// Advice when the data couldn't be decoded (e.g. "looks encrypted").
    pub hint: Option<String>,
}

pub fn describe(layers: &[Layer], content: &Content) -> String {
    let mut parts: Vec<String> = layers.iter().map(Layer::name).collect();
    parts.push(content.kind());
    parts.join("  \u{203A}  ")
}

/// Same encoding stack? Compares layer kinds only — a rebuilt zip never matches the
/// original archive byte-for-byte, and that's expected.
pub fn same_layers(a: &[Layer], b: &[Layer]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| std::mem::discriminant(x) == std::mem::discriminant(y))
}

/// Does the file read back (`check`) hold what was edited (`edited`)? Usually the same data;
/// after an edit that changes the layout (adding items), the same values wherever both have them.
pub fn verified(check: &Content, edited: &Content) -> bool {
    same_data(check, edited)
        || matches!((check, edited), (Content::Tree(c), Content::Tree(e)) if e.reshaped() && e.agrees_with(c))
}

/// Do two decoded contents carry the same data? Used to verify a save before writing it.
pub fn same_data(a: &Content, b: &Content) -> bool {
    match (a, b) {
        (Content::Json { value: x, .. }, Content::Json { value: y, .. }) => x == y,
        (Content::Tree(x), Content::Tree(y)) => x.view == y.view,
        (Content::Text(x), Content::Text(y)) => x == y,
        (Content::Binary(x), Content::Binary(y)) => x == y,
        // An edit can change how the data is detected (e.g. text that is now valid
        // JSON). That's fine as long as the bytes are identical.
        _ => matches!((a.to_bytes(), b.to_bytes()), (Ok(x), Ok(y)) if x == y),
    }
}

pub fn decode(raw: &[u8]) -> Decoded {
    decode_with(raw, &Options::default())
}

pub fn decode_with(raw: &[u8], opts: &Options) -> Decoded {
    let mut ctx = Ctx { opts, zip_entries: vec![], hint: None };
    let mut layers = Vec::new();
    let mut data = raw.to_vec();
    // Console saves (PS3, PS4, PSP, Vita…) whose folder names the game: the patch
    // collection's own codes decrypt or unpack them.
    if let Some(path) = &opts.path {
        let path = path.to_string_lossy();
        if let Some(o) = crate::apollo::console::open(&path, raw) {
            ctx.hint = Some(if o.plain[..] == raw[..] {
                format!("{} ({}) save: its checksums are written again on save, with the codes from its Apollo patch file.", o.title, o.platform)
            } else {
                format!(
                    "{} ({}) save: {} with the codes from its Apollo patch file, and packed again the same way when you save.",
                    o.title,
                    o.platform,
                    if o.var.is_some() { "unpacked" } else { "decrypted" }
                )
            });
            let file = path.replace('\\', "/").rsplit('/').next().unwrap_or_default().to_string();
            layers.push(Layer::Apollo { id: o.id, file, var: o.var, original: Arc::new(raw.to_vec()) });
            data = o.plain;
        }
    }
    while layers.len() < MAX_LAYERS {
        match peel(&data, &mut ctx) {
            Some((layer, inner)) => {
                layers.push(layer);
                data = inner;
            }
            None => break,
        }
    }
    // Cyberpunk 2077 keeps its table of sections in the file's tail, outside the packed data.
    let cyberpunk = match layers.last() {
        Some(Layer::Cdpr { prefix, footer, .. }) => fmt::cyberpunk::parse(&data, prefix.len(), footer),
        // Larian saves: the screen also reads the package's other files (summary, metadata).
        Some(Layer::Larian { archive, file }) => fmt::larian::parse(&data, archive, file),
        Some(Layer::PdxSav { original, .. }) => pdx_sav_doc(&data, original, opts),
        // Europa Universalis IV and Crusader Kings II keep the date and the player beside
        // the game state, in the archive's `meta` entry.
        Some(Layer::ZipEntry { archive, name }) if name == "gamestate" && fmt::pdxbin::looks(&data) => {
            let mut meta = Vec::new();
            if let Ok(mut zip) = zip::ZipArchive::new(Cursor::new(archive.as_slice())) {
                if let Ok(mut f) = zip.by_name("meta") {
                    let _ = f.read_to_end(&mut meta);
                }
            }
            fmt::pdxbin::parse_with_meta(&data, &meta).ok()
        }
        _ => None,
    };
    // A template the player wrote for this game comes before the editor's own guesses.
    let cyberpunk = cyberpunk.or_else(|| {
        let (t, file) = fmt::template::find(&data, &opts.ext)?;
        if let Some(s) = t.sum() {
            layers.push(Layer::Sum(s));
        }
        Some(t.parse(&data, &file))
    });
    let content = match cyberpunk {
        Some(doc) => Content::Tree(Box::new(doc)),
        None => classify(data, opts),
    };
    // A checksum kept fixed shows as zeros: it is filled in when the file is saved.
    let content = match (opts.sum, content) {
        (Some(s), Content::Binary(mut b)) if s.holds(&b) => {
            b[s.at..s.at + s.algo.width()].fill(0);
            layers.push(Layer::Sum(s));
            Content::Binary(b)
        }
        (_, c) => c,
    };
    if let (Content::Binary(b), None) = (&content, &ctx.hint) {
        let battery = [512, 8192, 32768, 65536, 131072].iter().any(|&n| (n..=n + 48).contains(&b.len()))
            && matches!(opts.ext.as_str(), "sav" | "srm" | "sa1" | "sa2" | "fla" | "eep" | "dsv");
        if battery && layers.is_empty() {
            ctx.hint = Some(
                "This looks like a Game Boy / GBA battery save. Use Value search to find a number you know (money, a stat), \
                 then edit it. Pokémon Red/Blue/Yellow, Gold/Silver/Crystal and Ruby/Sapphire/Emerald/FireRed/LeafGreen saves get a full editor automatically."
                    .into(),
            );
        } else if matches!(layers.last(), Some(Layer::Bnd4 { .. })) {
            ctx.hint = Some(
                "Checksums and encryption are handled for you. Use Value search to find a number you know (souls, a stat), \
                 edit it and save. Tools \u{203A} Open another file in this archive picks another slot."
                    .into(),
            );
        } else if let Some(game) = fmt::pdx::binary_kind(b) {
            ctx.hint = Some(format!(
                "This is a binary (ironman) {game} save, but its data couldn't be read, so it is shown as hex. \
                 Value search still finds a number you know."
            ));
        } else if matches!(layers.last(), Some(Layer::PngChunk { .. })) {
            ctx.hint = Some("This is the emulator's memory snapshot. Use Value search to find a number you know, then load this state in mGBA.".into());
        }
    }
    if matches!(content, Content::Binary(ref b) if ctx.hint.is_none() && fmt::crypto::looks_encrypted(b)) {
        ctx.hint = Some(
            "This data looks encrypted. If you know the game's password, use Tools \u{203A} Decrypt with password.".into(),
        );
    }
    Decoded { layers, content, zip_entries: ctx.zip_entries, hint: ctx.hint }
}

pub fn encode(layers: &[Layer], content: &Content) -> Result<Vec<u8>, String> {
    let mut data = content.to_bytes()?;
    for layer in layers.iter().rev() {
        data = wrap(layer, data)?;
    }
    Ok(data)
}

struct Ctx<'a> {
    opts: &'a Options,
    zip_entries: Vec<String>,
    hint: Option<String>,
}

fn renpy_unzip(data: &[u8]) -> Option<(Layer, Vec<u8>)> {
    let mut zip = zip::ZipArchive::new(Cursor::new(data)).ok()?;
    let signed = zip.file_names().any(|n| n == "signatures");
    let mut log = Vec::new();
    zip.by_name("log").ok()?.read_to_end(&mut log).ok()?;
    Some((Layer::RenpyZip { archive: Arc::new(data.to_vec()), signed }, log))
}

/// Any other zip: open the chosen entry, or the first one that decodes to something editable.
fn generic_unzip(data: &[u8], ctx: &mut Ctx) -> Option<(Layer, Vec<u8>)> {
    let mut zip = zip::ZipArchive::new(Cursor::new(data)).ok()?;
    let names: Vec<String> = (0..zip.len())
        .filter_map(|i| zip.by_index(i).ok().filter(|f| f.is_file()).map(|f| f.name().to_string()))
        .collect();
    if names.is_empty() {
        return None;
    }
    let read = |zip: &mut zip::ZipArchive<Cursor<&[u8]>>, n: &str| -> Option<Vec<u8>> {
        let mut out = Vec::new();
        zip.by_name(n).ok()?.read_to_end(&mut out).ok()?;
        Some(out)
    };
    let chosen = match ctx.opts.zip_entry.as_ref().filter(|n| names.contains(n)) {
        Some(n) => n.clone(),
        None => {
            let mut best: Option<(String, (u8, usize))> = None;
            let mut fallback: Option<(String, usize)> = None;
            for n in &names {
                let Some(bytes) = read(&mut zip, n) else { continue };
                let len = bytes.len();
                // Decoding a big entry just to rank it is slow (Stellaris' gamestate is tens
                // of MB): text-looking big entries count as editable without it.
                let editable = if len > 1 << 20 {
                    !bytes[..len.min(4096)].contains(&0) || fmt::pdxbin::looks(&bytes)
                } else {
                    !matches!(decode(&bytes).content, Content::Binary(_))
                };
                let score = (save_like(n), len);
                if editable && best.as_ref().is_none_or(|b| score > b.1) {
                    best = Some((n.clone(), score));
                }
                if fallback.as_ref().is_none_or(|b| len > b.1) {
                    fallback = Some((n.clone(), len));
                }
            }
            best.map(|b| b.0).or(fallback.map(|f| f.0))?
        }
    };
    let bytes = read(&mut zip, &chosen)?;
    ctx.zip_entries = names;
    Some((Layer::ZipEntry { archive: Arc::new(data.to_vec()), name: chosen }, bytes))
}

/// A Paradox `SAV…` file that ends in a zip: the game state inside it.
fn peel_pdx_sav(data: &[u8]) -> Option<(Layer, Vec<u8>)> {
    let h = fmt::pdxbin::SavHeader::read(data).filter(|h| h.zipped())?;
    let after = h.len + h.meta_len;
    let zip_at = if data.get(after..after + 4) == Some(b"PK\x03\x04") { after } else { h.len + data[h.len..].windows(4).position(|w| w == b"PK\x03\x04")? };
    let mut zip = zip::ZipArchive::new(Cursor::new(data)).ok()?;
    let mut out = Vec::new();
    zip.by_name("gamestate").ok()?.read_to_end(&mut out).ok()?;
    Some((Layer::PdxSav { original: Arc::new(data.to_vec()), zip_at }, out))
}

/// The game state of a Paradox `SAV…` archive: binary (ironman) or text.
fn pdx_sav_doc(data: &[u8], original: &[u8], opts: &Options) -> Option<TreeDoc> {
    use fmt::pdxbin::Game;
    let h = fmt::pdxbin::SavHeader::read(original)?;
    let game = Game::from_ext(&opts.ext);
    if !h.binary() {
        let mut doc = fmt::pdx::parse(data).ok()?;
        if game == Game::Ck3 {
            doc.kind_name = "Crusader Kings III save".into();
        }
        return Some(doc);
    }
    let mut lookup = vec![];
    if let Ok(mut zip) = zip::ZipArchive::new(Cursor::new(original)) {
        if let Ok(mut f) = zip.by_name("string_lookup") {
            let mut raw = Vec::new();
            if f.read_to_end(&mut raw).is_ok() {
                lookup = fmt::pdxbin::string_lookup(&raw);
            }
        }
    }
    fmt::pdxbin::parse_as(data, game, &lookup).ok()
}

/// Rebuild a zip, replacing `name` with `data` (and re-signing Ren'Py saves).
fn rezip(archive: &[u8], name: &str, data: &[u8], renpy: bool) -> Result<Vec<u8>, String> {
    rezip_after(Vec::new(), archive, &[(name, data)], renpy)
}

/// Writes a Paradox `SAV…` archive back around an edited game state. The metadata is kept
/// twice, at the start of the game state and on its own (after the header line, or as the
/// archive's `meta` entry), so the separate copy follows an edit to it.
fn pdx_sav_wrap(original: &[u8], zip_at: usize, data: &[u8]) -> Result<Vec<u8>, String> {
    let h = fmt::pdxbin::SavHeader::read(original).ok_or("the save's header can't be read")?;
    let mut prefix = original[..zip_at].to_vec();
    let inline = &original[h.len.min(zip_at)..zip_at];
    if h.binary() && !inline.is_empty() && !data.starts_with(inline) {
        if let Some(n) = fmt::pdxbin::token_prefix_len(inline, data) {
            prefix.truncate(h.len);
            prefix[15..23].copy_from_slice(format!("{n:08x}").as_bytes());
            prefix.extend_from_slice(&data[..n]);
        }
    }
    let mut meta_entry = None;
    if h.binary() {
        if let Ok(mut zip) = zip::ZipArchive::new(Cursor::new(original)) {
            if let Ok(mut f) = zip.by_name("meta") {
                let mut old = Vec::new();
                if f.read_to_end(&mut old).is_ok() && !old.is_empty() && !data.starts_with(&old) {
                    meta_entry = fmt::pdxbin::token_prefix_len(&old, data).map(|n| &data[..n]);
                }
            }
        }
    }
    let mut files = vec![("gamestate", data)];
    if let Some(m) = meta_entry {
        files.push(("meta", m));
    }
    rezip_after(prefix, original, &files, false)
}

/// `rezip` for several entries, with the new archive written after `prefix` (its offsets
/// count from the start of the whole file, as in the original).
fn rezip_after(prefix: Vec<u8>, archive: &[u8], files: &[(&str, &[u8])], renpy: bool) -> Result<Vec<u8>, String> {
    use zip::write::SimpleFileOptions;
    let err = |e: zip::result::ZipError| e.to_string();
    let deflated = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut src = zip::ZipArchive::new(Cursor::new(archive)).map_err(err)?;
    let mut start = Cursor::new(prefix);
    start.set_position(start.get_ref().len() as u64);
    let mut out = zip::ZipWriter::new(start);
    for i in 0..src.len() {
        let f = src.by_index_raw(i).map_err(err)?;
        let entry = f.name().to_string();
        if let Some((_, data)) = files.iter().find(|(n, _)| *n == entry) {
            drop(f);
            out.start_file(entry, deflated).map_err(err)?;
            out.write_all(data).map_err(|e| e.to_string())?;
        } else if renpy && entry == "signatures" {
            // The old signature no longer matches the edited log, and Ren'Py refuses
            // to load a save with a wrong signature even after the player says yes.
            // Re-sign with the player's own key; without a key, drop the entry so the
            // game asks once and then loads it.
            drop(f);
            if let Some(sig) = files.first().and_then(|(_, log)| crate::renpy_sign::sign(log)) {
                out.start_file("signatures", deflated).map_err(err)?;
                out.write_all(sig.as_bytes()).map_err(|e| e.to_string())?;
            }
        } else {
            out.raw_copy_file(f).map_err(err)?;
        }
    }
    Ok(out.finish().map_err(err)?.into_inner())
}

fn compress<W: Write>(mut enc: W, data: &[u8]) -> Result<W, String> {
    enc.write_all(data).map_err(|e| e.to_string())?;
    Ok(enc)
}

fn wrap(layer: &Layer, data: Vec<u8>) -> Result<Vec<u8>, String> {
    let io = |e: std::io::Error| e.to_string();
    Ok(match layer {
        Layer::RenpyZip { archive, .. } => rezip(archive, "log", &data, true)?,
        Layer::ZipEntry { archive, name } => rezip(archive, name, &data, false)?,
        Layer::PdxSav { original, zip_at } => pdx_sav_wrap(original, *zip_at, &data)?,
        Layer::RenpyPersistent { signed } => {
            let mut out = compress(ZlibEncoder::new(Vec::new(), Compression::new(3)), &data)?.finish().map_err(io)?;
            if *signed {
                let sig = crate::renpy_sign::sign(&data).ok_or(
                    "this persistent file must be signed, but your Ren'Py key wasn't found \
                     (%APPDATA%\\RenPy\\tokens). Saving it unsigned would make the game discard it.",
                )?;
                out.extend_from_slice(sig.as_bytes());
            }
            out
        }
        Layer::Utf8Bom => [BOM, &data].concat(),
        Layer::Utf16 { big_endian } => {
            let s = String::from_utf8(data).map_err(|_| "UTF-16 layer needs text".to_string())?;
            let mut out = if *big_endian { vec![0xFE, 0xFF] } else { vec![0xFF, 0xFE] };
            for u in s.encode_utf16() {
                out.extend_from_slice(&if *big_endian { u.to_be_bytes() } else { u.to_le_bytes() });
            }
            out
        }
        Layer::Gzip => compress(GzEncoder::new(Vec::new(), Compression::default()), &data)?.finish().map_err(io)?,
        Layer::Zlib => compress(ZlibEncoder::new(Vec::new(), Compression::default()), &data)?.finish().map_err(io)?,
        Layer::Deflate => compress(flate2::write::DeflateEncoder::new(Vec::new(), Compression::default()), &data)?.finish().map_err(io)?,
        Layer::Zstd => zstd::encode_all(&data[..], 3).map_err(io)?,
        Layer::Lz4 => compress(lz4_flex::frame::FrameEncoder::new(Vec::new()), &data)?.finish().map_err(|e| e.to_string())?,
        Layer::Ac8(p) => fmt::ac8::close(p, &data)?,
        Layer::Prey { header } => fmt::prey::close(header, &data)?,
        Layer::Evas => {
            let mut h = flate2::Crc::new();
            h.update(&data);
            let mut out = b"EVAS\x01\x00\x00\x00".to_vec();
            out.extend_from_slice(&data);
            out.extend_from_slice(&h.sum().to_le_bytes());
            out
        }
        Layer::Snappy => compress(snap::write::FrameEncoder::new(Vec::new()), &data)?.into_inner().map_err(|e| e.to_string())?,
        Layer::Xz => {
            let mut out = Vec::new();
            lzma_rs::xz_compress(&mut &data[..], &mut out).map_err(io)?;
            out
        }
        Layer::Bzip2 => compress(bzip2::write::BzEncoder::new(Vec::new(), bzip2::Compression::default()), &data)?
            .finish()
            .map_err(io)?,
        Layer::Base64 => STANDARD.encode(&data).into_bytes(),
        Layer::Base64Lines { width, crlf } => {
            let b64 = STANDARD.encode(&data);
            let nl = if *crlf { "\r\n" } else { "\n" };
            b64.as_bytes().chunks(*width).map(|c| std::str::from_utf8(c).unwrap()).collect::<Vec<_>>().join(nl).into_bytes()
        }
        Layer::LzString => {
            let s = String::from_utf8(data).map_err(|_| "LZString layer needs UTF-8 text".to_string())?;
            lz_str::compress_to_base64(s.as_str()).into_bytes()
        }
        Layer::RpgMzZlib => {
            // RPG Maker MZ uses pako.deflate(level 1, to:"string") and writes that string as UTF-8.
            let zipped = compress(ZlibEncoder::new(Vec::new(), Compression::new(1)), &data)?.finish().map_err(io)?;
            zipped.iter().map(|&b| b as char).collect::<String>().into_bytes()
        }
        Layer::JsEscape => {
            let s = String::from_utf8(data).map_err(|_| "escaped layer needs text".to_string())?;
            js_escape(&s).into_bytes()
        }
        Layer::Xor(k) => data.iter().map(|b| b ^ k).collect(),
        Layer::XorKey(k) => xor_key(&data, k.as_bytes()),
        Layer::Keyed(w) => fmt::keyed::close(w, &data),
        Layer::PngChunk { png, chunk } => png_replace(png, chunk, &compress(ZlibEncoder::new(Vec::new(), Compression::default()), &data)?.finish().map_err(io)?)?,
        Layer::FileBundle { files, name } => {
            let mut out = Vec::new();
            for (n, bytes) in files.iter() {
                let units: Vec<u16> = n.encode_utf16().collect();
                out.extend((units.len() as i32).to_le_bytes());
                units.iter().for_each(|u| out.extend(u.to_le_bytes()));
                let body = if n == name { &data } else { bytes };
                out.extend((body.len() as i32).to_le_bytes());
                out.extend(body);
            }
            out
        }
        Layer::Palworld { double } => {
            let inner = compress(ZlibEncoder::new(Vec::new(), Compression::default()), &data)?.finish().map_err(io)?;
            let payload = if *double {
                compress(ZlibEncoder::new(Vec::new(), Compression::default()), &inner)?.finish().map_err(io)?
            } else {
                inner.clone()
            };
            let mut out = (data.len() as u32).to_le_bytes().to_vec();
            out.extend((inner.len() as u32).to_le_bytes());
            out.extend(b"PlZ");
            out.push(if *double { 0x32 } else { 0x31 });
            out.extend(payload);
            out
        }
        Layer::HogwartsDb { head, size_at, count_at, tail, block } => {
            // The array holds [len + 4][len][database], cut into chunks that Oodle packs.
            let mut inner = ((data.len() + 4) as u32).to_le_bytes().to_vec();
            inner.extend((data.len() as u32).to_le_bytes());
            inner.extend_from_slice(&data);
            let parts: Vec<&[u8]> = inner.chunks(*block as usize).collect();
            let packed = crate::par::map(&parts, |p| crate::oodle::compress(p));
            let mut arr = Vec::with_capacity(inner.len() / 3);
            for (c, p) in packed.into_iter().zip(&parts) {
                let c = c.ok_or("Oodle couldn't pack the database again (no Oodle library found)")?;
                for v in [0x9E2A_83C1u64, *block, c.len() as u64, p.len() as u64, c.len() as u64, p.len() as u64] {
                    arr.extend(v.to_le_bytes());
                }
                arr.extend(c);
            }
            let mut out = head.as_ref().clone();
            out[*count_at..count_at + 4].copy_from_slice(&(arr.len() as u32).to_le_bytes());
            out[*size_at..size_at + 8].copy_from_slice(&((arr.len() + 4) as u64).to_le_bytes());
            out.extend(arr);
            out.extend_from_slice(tail);
            out
        }
        Layer::Ff9 => fmt::ff9::encrypt(&data)?,
        Layer::Ffxv { foot } => fmt::ffxv::encrypt(&data, foot)?,
        Layer::Ao { kai: None } => fmt::ao::zstd_pack(&data)?,
        Layer::Ao { kai: Some(o) } => {
            let mut d = data.clone();
            fmt::ao::fix_sums(&mut d);
            fmt::ao::cle_pack(&d, o)?
        }
        Layer::Forspoken { foot } => fmt::forspoken::encrypt(&data, foot)?,
        Layer::Ds2(w) => fmt::ds2::pack(w, &data)?,
        Layer::P5r { head } => fmt::p5r::pack(head, &data),
        Layer::MetroPlayer { head } => fmt::metro::pack_player(head, &data),
        Layer::Bioshock { chunks } => fmt::bioshock::pack(chunks, &data)?,
        Layer::Rgg(w) => fmt::rgg::encrypt(w, &data),
        Layer::RemedyCrc { head } => {
            let mut h = flate2::Crc::new();
            h.update(&data);
            let mut out = head.to_vec();
            out.extend(h.sum().to_le_bytes());
            out.extend(&data);
            out
        }
        Layer::Rgmh { original, blocks } => fmt::rgmh::pack_from(original, blocks, &data)?,
        Layer::Copeck { pack, entry, .. } => copeck_put(pack, *entry, &data)?,
        Layer::MetroDezp { layout } => fmt::metro::pack_dezp(layout, &data)?,
        Layer::PalworldOodle => {
            let packed = crate::oodle::compress(&data).ok_or("Oodle couldn't pack the save again (no Oodle library found)")?;
            let mut out = (data.len() as u32).to_le_bytes().to_vec();
            out.extend((packed.len() as u32).to_le_bytes());
            out.extend(b"PlM1");
            out.extend(packed);
            out
        }
        Layer::Es3 { password, iv } => fmt::crypto::es3_encrypt(&data, password, iv)?,
        Layer::GodotEnc(info) => fmt::crypto::godot_encrypt(&data, info)?,
        Layer::Terraria => fmt::crypto::terraria_encrypt(&data),
        Layer::Supergiant { header } => fmt::luabins::close(header, &data),
        Layer::Bnd4 { archive, name, key, iv } => fmt::containers::bnd4_replace(archive, name, &data, *key, iv)?,
        Layer::Region { archive, index, kind } => fmt::containers::region_replace(archive, *index, &data, *kind)?,
        Layer::HollowKnight => fmt::crypto::hollow_knight_encrypt(&data),
        Layer::FalloutShelter => fmt::crypto::fallout_shelter_encrypt(&data),
        Layer::P3rSteam => fmt::p3r::steam_encrypt(&data),
        Layer::Antimatter { kind, version } => fmt::antimatter::close(*kind, version, &data)?,
        Layer::NestedJson { paths, pretty } => fmt::nested::close(&data, paths, *pretty)?,
        Layer::Mandarin { original, game, user } => fmt::mandarin::encrypt(&data, original, &fmt::mandarin::GAMES[*game], *user)?,
        Layer::Lime { original, user } => fmt::lime::encrypt(&data, original, *user)?,
        Layer::Citrus { original, curve, steam } => fmt::citrus::encrypt(&data, original, *curve, *steam)?,
        Layer::RegFile(meta) => fmt::regfile::save(meta, &data)?,
        Layer::Sol { original } => fmt::sol::save(original, &data)?,
        Layer::Mcd => fmt::mcd::pack(&data),
        Layer::Stoneshard { salt } => fmt::stoneshard::pack(salt, &data)?,
        Layer::Astroneer => fmt::uepack::astro_pack(&data),
        Layer::Satisfactory { info } => fmt::uepack::sat_pack(info, &data)?,
        Layer::Civ7 { info } => fmt::civ7::pack(info, &data)?,
        Layer::Tld { wrapped } => fmt::tld::pack(wrapped, &data)?,
        Layer::Ffpr { wrapped, bom } => fmt::ffpr::pack(wrapped, *bom, &data)?,
        Layer::Haxe { original } => fmt::haxe::save(original, &data)?,
        Layer::DeadCells { header } => fmt::deadcells::close(header, &data)?,
        Layer::Katana { kind, plain } => fmt::katana::close(*kind, plain, &data)?,
        Layer::LegoTt => fmt::ttgames::close(&data),
        Layer::Sum(s) => {
            let mut d = data;
            s.fix(&mut d)?;
            d
        }
        Layer::Ps2Card { image, file } => {
            let card = fmt::memcard::Ps2Card::open(image).ok_or("the memory card image no longer reads")?;
            card.write_file(image, file, &data)?
        }
        Layer::Dsv { footer } => [&data[..], footer].concat(),
        Layer::Kh3 { account } => fmt::kh3::close(account, &data)?,
        Layer::Fantasian { original, record } => fmt::fantasian::close(original, record, &data)?,
        Layer::Saturn { kind, original, .. } => fmt::saturn::save(*kind, original, &data)?,
        Layer::SaturnImage { image, save, .. } => fmt::saturn::write(image, save, &data)?,
        Layer::PsSave { kind, original, name } => fmt::pssave::save(*kind, original, name, &data)?,
        Layer::Dci { header } => fmt::dreamcast::dci_close(header, &data)?,
        Layer::Vms { off } => fmt::dreamcast::vms_close(&data, *off)?,
        Layer::Apollo { id, file, var, original } => crate::apollo::console::close(id, file, var, original, &data)?,
        Layer::Ps1Card { image, offset, blocks } => {
            fmt::memcard::ps1_write(image, *offset, &fmt::memcard::Ps1Save { name: String::new(), blocks: blocks.clone() }, &data)?
        }
        Layer::Cdpr { prefix, fzlc, sizes, footer, tagged } => {
            if data.len() != sizes.iter().map(|&n| n as usize).sum::<usize>() {
                return Err("this save's data can't change size".into());
            }
            let mut out = prefix.as_ref().clone();
            let mut at = 0;
            let parts: Vec<(usize, u32)> = sizes
                .iter()
                .map(|&n| {
                    let from = at;
                    at += n as usize;
                    (from, n)
                })
                .collect();
            let chunks = crate::par::map(&parts, |&(from, n)| {
                let mut chunk = if *tagged { [&b"4ZLX"[..], &n.to_le_bytes()].concat() } else { vec![] };
                chunk.extend(lz4_flex::block::compress(&data[from..from + n as usize]));
                chunk
            });
            let mut end = out.len();
            for (k, chunk) in chunks.into_iter().enumerate() {
                end += chunk.len();
                let e = fzlc + 12 + 12 * k;
                out[e..e + 4].copy_from_slice(&(chunk.len() as u32).to_le_bytes());
                if k + 1 < sizes.len() {
                    out[e + 8..e + 12].copy_from_slice(&(end as u32).to_le_bytes());
                }
                out.extend(chunk);
            }
            // Cyberpunk's tail ends with the file position of its section table ("EDON") and
            // "DONE"; the table moves when the packed data changes size.
            let mut tail = footer.as_ref().clone();
            if tail.len() >= 8 && tail.ends_with(b"ENOD") {
                if let Some(pos) = tail.windows(4).position(|w| w == b"EDON") {
                    let n = tail.len();
                    tail[n - 8..n - 4].copy_from_slice(&((out.len() + pos) as u32).to_le_bytes());
                }
            }
            out.extend_from_slice(&tail);
            out
        }
        Layer::Owlcat { archive } => fmt::owlcat::save(archive, &data)?,
        Layer::Borderlands { original, plain } => {
            if data == **plain {
                original.to_vec()
            } else {
                fmt::borderlands::pack(&data)?
            }
        }
        Layer::GrimDawn { seed, plan } => fmt::grimdawn::encrypt(*seed, plan, &data)?,
        Layer::Torchlight2 => fmt::torchlight2::pack(&data),
        Layer::Larian { archive, file } => fmt::larian::save(archive, file, &data)?,
        Layer::Wasteland3 { original } => fmt::wasteland3::pack(original, &data)?,
        Layer::Me1Header { header } => {
            let mut out = header.to_vec();
            out.extend_from_slice(&data);
            out
        }
        Layer::Me1Le { block } => {
            let bs = (*block as usize).max(1);
            let parts: Vec<&[u8]> = data.chunks(bs).collect();
            let packed = crate::par::map(&parts, |p| compress(ZlibEncoder::new(Vec::new(), Compression::default()), p).and_then(|e| e.finish().map_err(io)));
            let mut table = Vec::new();
            let mut body = Vec::new();
            for (c, p) in packed.into_iter().zip(&parts) {
                let c = c?;
                table.extend((c.len() as u32).to_le_bytes());
                table.extend((p.len() as u32).to_le_bytes());
                body.extend(c);
            }
            let mut out = Vec::with_capacity(body.len() + 64);
            for v in [0x9E2A_83C1u32, *block, body.len() as u32, data.len() as u32] {
                out.extend(v.to_le_bytes());
            }
            out.extend(table);
            out.extend(body);
            let crc = fmt::crc32_bzip2(&out);
            out.extend(crc.to_le_bytes());
            out.extend(1u32.to_le_bytes());
            out.extend((data.len() as u32).to_le_bytes());
            out
        }
        Layer::Dbpf { original } => fmt::dbpf::rebuild(original, &data, fmt::sims2::editable)?,
        Layer::Dbpf4 { original } => fmt::sims4::rebuild(original, &data)?,
        Layer::Xcom { header, crc_at, xcom2, sizes, block } => {
            if data.len() != sizes.iter().map(|&n| n as usize).sum::<usize>() {
                return Err("this save's data can't change size".into());
            }
            let mut at = 0;
            let parts: Vec<(usize, u32)> = sizes
                .iter()
                .map(|&n| {
                    let from = at;
                    at += n as usize;
                    (from, n)
                })
                .collect();
            let packed = crate::par::map(&parts, |&(from, n)| lzokay_native::compress(&data[from..from + n as usize]).map_err(|e| format!("{e:?}")));
            let mut body = Vec::with_capacity(data.len() / 2);
            for (c, &(_, n)) in packed.into_iter().zip(&parts) {
                let c = c?;
                for v in [0x9E2A_83C1u32, *block, c.len() as u32, n, c.len() as u32, n] {
                    body.extend(v.to_le_bytes());
                }
                body.extend(c);
            }
            let mut out = header.as_ref().clone();
            out[*crc_at..crc_at + 4].copy_from_slice(&fmt::crc32_bzip2(&body).to_le_bytes());
            if *xcom2 {
                out[8..12].fill(0);
                let crc = fmt::crc32_bzip2(&out);
                out[8..12].copy_from_slice(&crc.to_le_bytes());
            } else {
                let len = u32::from_le_bytes(out[1016..1020].try_into().unwrap()) as usize;
                let crc = fmt::crc32_bzip2(&out[..len]);
                out[1020..1024].copy_from_slice(&crc.to_le_bytes());
            }
            out.extend(body);
            out
        }
        Layer::Civ5 { header, chunked, trailer } => {
            // Written the way the game does: everything, then a sync flush and no final block,
            // cut into 64 KiB chunks.
            let mut out = header.as_ref().clone();
            let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::new(6));
            e.write_all(&data).map_err(|e| e.to_string())?;
            e.flush().map_err(|e| e.to_string())?;
            if *chunked {
                for c in e.get_ref().chunks(CIV5_CHUNK) {
                    out.extend((c.len() as u32).to_le_bytes());
                    out.extend_from_slice(c);
                }
            } else {
                out.extend_from_slice(e.get_ref());
            }
            out.extend_from_slice(trailer);
            out
        }
    })
}

fn peel(data: &[u8], ctx: &mut Ctx) -> Option<(Layer, Vec<u8>)> {
    if data.starts_with(b"GVAS") {
        if let Some((p, inner)) = fmt::ac8::open(data) {
            return Some((Layer::Ac8(p), inner));
        }
    }
    if data.starts_with(b"CRY3SDK") {
        if let Some((inner, header)) = fmt::prey::open(data) {
            return Some((Layer::Prey { header: Arc::new(header) }, inner));
        }
    }
    // EVAS v1 (Stellar Blade): only when the stored CRC-32 matches.
    if data.len() >= 16 && data.starts_with(b"EVAS\x01\x00\x00\x00GVAS") {
        let body = &data[8..data.len() - 4];
        let mut h = flate2::Crc::new();
        h.update(body);
        if h.sum().to_le_bytes() == data[data.len() - 4..] {
            return Some((Layer::Evas, body.to_vec()));
        }
    }
    if data.starts_with(b"SAV") {
        if let Some(found) = peel_pdx_sav(data) {
            return Some(found);
        }
    }
    if data.starts_with(b"DBPF") {
        if fmt::sims4::looks_package(data) {
            if let Some(flat) = fmt::sims4::flatten(data) {
                ctx.hint = Some("The Sims 4 save: households and their funds are shown; funds are written back into the game data, which is stored uncompressed (the file grows). Close the game first.".into());
                return Some((Layer::Dbpf4 { original: Arc::new(data.to_vec()) }, flat));
            }
        }
        if let Some(flat) = fmt::sims2::flatten(data) {
            ctx.hint = Some("The Sims 2 neighbourhood: families, funds and Sims are shown; changed parts are stored uncompressed when you save. Close the game first.".into());
            return Some((Layer::Dbpf { original: Arc::new(data.to_vec()) }, flat));
        }
    }
    if data.starts_with(b"PK\x03\x04") {
        if let Some(json) = fmt::owlcat::open(data) {
            ctx.hint = Some("Pathfinder save: its party, money and header are shown together; changed parts are written back with their checksums.".into());
            return Some((Layer::Owlcat { archive: Arc::new(data.to_vec()) }, json));
        }
        return renpy_unzip(data).or_else(|| generic_unzip(data, ctx));
    }
    if fmt::lspk::looks(data) {
        if let Some((file, lsf)) = fmt::larian::open(data) {
            ctx.hint = Some(if file.eq_ignore_ascii_case("meta.lsf") {
                "Baldur's Gate 3 save: the party summary is shown read-only.".into()
            } else {
                "Larian save package: its game data is unpacked here and packed again, with the package checksum, when you save.".into()
            });
            return Some((Layer::Larian { archive: Arc::new(data.to_vec()), file }, lsf));
        }
    }
    if data.starts_with(b"XLZF\n") {
        if let Some(xml) = fmt::wasteland3::open(data) {
            return Some((Layer::Wasteland3 { original: Arc::new(data.to_vec()) }, xml));
        }
    }
    if data.starts_with(b"CIV7") {
        if let Some((info, un)) = fmt::civ7::open(data) {
            ctx.hint = Some("Civilization VII save: unpacked for you and packed again on save. Where the save keeps its players' gold and influence in a known place they are on the Tree; any other number can be found with Value search.".into());
            return Some((Layer::Civ7 { info }, un));
        }
    }
    if let Some(body) = fmt::uepack::astro_open(data) {
        ctx.hint = Some("Astroneer save: unpacked for you and packed again on save. Use Value search to find a number you know (for example your Bytes), then change it.".into());
        return Some((Layer::Astroneer, body));
    }
    if data.len() > 400 && ctx.opts.ext.eq_ignore_ascii_case("sav") {
        if let Some((info, body)) = fmt::uepack::sat_open(data) {
            ctx.hint = Some("Satisfactory save: unpacked for you and packed again on save. Use Value search to find a number you know, then change it.".into());
            return Some((Layer::Satisfactory { info }, body));
        }
    }
    if data.first() == Some(&0x78) {
        let path = ctx.opts.path.clone().unwrap_or_default();
        if let Some((salt, ok, json)) = fmt::stoneshard::open(data, &path.to_string_lossy()) {
            ctx.hint = Some(if ok {
                "Stoneshard save: unpacked for you; its MD5 is worked out again and it is packed again on save.".into()
            } else {
                "Stoneshard save: unpacked for you. Its checksum depends on the folder names (characters_v1\\character_N\\<save folder>\\data.sav) and didn't match here, so open the file from its own folder before saving.".into()
            });
            return Some((Layer::Stoneshard { salt }, json));
        }
    }
    if let Some(json) = fmt::mcd::open(data) {
        ctx.hint = Some("Minecraft Dungeons save: decrypted for you and encrypted again on save.".into());
        return Some((Layer::Mcd, json));
    }
    if let Some((wrapped, json)) = fmt::ffpr::open(data) {
        ctx.hint = Some("Final Fantasy Pixel Remaster save: decrypted for you (the character data kept as text inside it is opened too) and encrypted again on save.".into());
        return Some((Layer::Ffpr { wrapped, bom: data.starts_with(b"\xEF\xBB\xBF") }, json));
    }
    if let Some((wrapped, json)) = fmt::tld::open(data) {
        ctx.hint = Some("The Long Dark save: its compressed levels are opened for you and packed again on save.".into());
        return Some((Layer::Tld { wrapped }, json));
    }
    if data.starts_with(BOM) {
        return Some((Layer::Utf8Bom, data[BOM.len()..].to_vec()));
    }
    if fmt::sol::looks(data) {
        if let Some(json) = fmt::sol::open(data) {
            return Some((Layer::Sol { original: Arc::new(data.to_vec()) }, json));
        }
    }
    if let Some(at) = fmt::memcard::dsv_split(data) {
        return Some((Layer::Dsv { footer: Arc::new(data[at..].to_vec()) }, data[..at].to_vec()));
    }
    if let Some(plain) = fmt::ttgames::open(data) {
        ctx.hint = Some("LEGO Batman save: decrypted for you and encrypted again on save. Each SaveSlot file has a BackupCopy file next to it; the game checks the pair, so edit both.".into());
        return Some((Layer::LegoTt, plain));
    }
    if let Some((kind, plain)) = fmt::katana::open(data) {
        let inner = fmt::katana::content(kind, &plain);
        return Some((Layer::Katana { kind, plain: Arc::new(plain) }, inner));
    }
    if ctx.opts.ext == "dci" {
        if let Some((header, blocks)) = fmt::dreamcast::dci_open(data) {
            return Some((Layer::Dci { header: Arc::new(header) }, blocks));
        }
    }
    if matches!(ctx.opts.ext.as_str(), "vms" | "dci") {
        if let Some(off) = fmt::dreamcast::vms_header(data) {
            ctx.hint = Some("Dreamcast VMU save: the file's CRC shows as zeros and is written again on save. Use the value search to find a number you know, and test the edit in the emulator.".into());
            return Some((Layer::Vms { off }, fmt::dreamcast::vms_open(data, off)));
        }
    }
    if let Some((kind, name, inner)) = fmt::saturn::open(data, &ctx.opts.ext) {
        ctx.hint = Some(format!(
            "{}: the save's data is open without its header{}. Each game checks its own data its own way, so use the value search and test the edit in the emulator.",
            kind.label(),
            match kind {
                fmt::saturn::Kind::Sro => ", and its CRC is worked out again on save",
                fmt::saturn::Kind::Cms => ", with its block headers left out",
                _ => "",
            }
        ));
        return Some((Layer::Saturn { kind, original: Arc::new(data.to_vec()), name }, inner));
    }
    if let Some((kind, saves)) = fmt::saturn::image(data) {
        if saves.is_empty() {
            ctx.hint = Some(format!("This {} holds no saves.", kind.label()));
            return None;
        }
        let s = ctx.opts.zip_entry.as_ref().and_then(|n| saves.iter().find(|s| &s.name == n)).unwrap_or(&saves[0]).clone();
        ctx.zip_entries = saves.iter().map(|s| s.name.clone()).collect();
        ctx.hint = Some(format!(
            "{}: one save is open; Tools › Open another file in this archive lists the others. Its bytes are written back where they came from, so the save keeps its size. Each game checks its own data its own way, so use the value search and test the edit in the emulator.",
            kind.label()
        ));
        let inner = fmt::saturn::read(data, &s);
        return Some((Layer::SaturnImage { kind, image: Arc::new(data.to_vec()), save: s }, inner));
    }
    if let Some((kind, files)) = fmt::pssave::open(data, &ctx.opts.ext) {
        let pick = |f: &&fmt::pssave::Entry| {
            let n = f.name.rsplit('/').next().unwrap_or("").to_ascii_lowercase();
            n != "icon.sys" && !n.ends_with(".ico") && !n.ends_with(".icn")
        };
        // The biggest file that isn't the icon or its description is usually the game's data.
        let f = ctx.opts.zip_entry.as_ref().and_then(|n| files.iter().find(|f| &f.name == n)).or_else(|| files.iter().filter(pick).max_by_key(|f| f.data.len())).unwrap_or(&files[0]).clone();
        if files.len() > 1 {
            ctx.zip_entries = files.iter().map(|f| f.name.clone()).collect();
        }
        ctx.hint = Some(format!(
            "{}: the save's own file is open{}. It is packed back the same way on save{}; each game checks its own data its own way, so test the edit in the emulator.",
            kind.label(),
            if files.len() > 1 { " (Tools › Open another file in this archive lists the rest)" } else { "" },
            if matches!(kind, fmt::pssave::Kind::Psv1 | fmt::pssave::Kind::Psv2) { ", and signed again for the PS3" } else { "" }
        ));
        return Some((Layer::PsSave { kind, original: Arc::new(data.to_vec()), name: f.name }, f.data));
    }
    if let Some(card) = fmt::memcard::Ps2Card::open(data) {
        let files = card.files(data).unwrap_or_default();
        if files.is_empty() {
            ctx.hint = Some("This PlayStation 2 memory card holds no saves the editor can read.".into());
            return None;
        }
        // Default to the first file that isn't the folder's icon or its description.
        let pick = |f: &&fmt::memcard::Ps2File| {
            let n = f.name.rsplit('/').next().unwrap_or("").to_ascii_lowercase();
            n != "icon.sys" && !n.ends_with(".ico") && !n.ends_with(".icn")
        };
        let f = ctx.opts.zip_entry.as_ref().and_then(|n| files.iter().find(|f| &f.name == n)).or_else(|| files.iter().find(pick)).unwrap_or(&files[0]).clone();
        ctx.zip_entries = files.iter().map(|f| f.name.clone()).collect();
        ctx.hint = Some("PlayStation 2 memory card: Tools › Open another file in this archive lists every file in every save folder. The card's ECC is written again on save; each game checks its own data its own way, so test the edit in the emulator.".into());
        let bytes = card.read_file(data, &f)?;
        return Some((Layer::Ps2Card { image: Arc::new(data.to_vec()), file: f }, bytes));
    }
    if let Some(offset) = fmt::memcard::ps1_offset(data) {
        let saves = fmt::memcard::ps1_saves(&data[offset..]);
        if saves.is_empty() {
            ctx.hint = Some("This PlayStation memory card is empty.".into());
            return None;
        }
        let s = ctx.opts.zip_entry.as_ref().and_then(|n| saves.iter().find(|s| &s.name == n)).unwrap_or(&saves[0]).clone();
        ctx.zip_entries = saves.iter().map(|s| s.name.clone()).collect();
        ctx.hint = Some("PlayStation memory card: the first save is open; Tools › Open another file in this archive lists the others. Each game checks its own data its own way, so use the value search and test the edit in the emulator.".into());
        let save = fmt::memcard::ps1_read(&data[offset..], &s);
        return Some((Layer::Ps1Card { image: Arc::new(data.to_vec()), offset, blocks: s.blocks }, save));
    }
    if data.len() > 33 && (data.len() - 17) % 16 == 0 && data[data.len() - 17] == 0x08 {
        let path = ctx.opts.path.clone().unwrap_or_default();
        let mut extra: Vec<String> = ctx.opts.password.as_deref().map(|p| p.trim().to_string()).into_iter().collect();
        extra.extend(fmt::mandarin::accounts_for(&path).into_iter().map(|n| if n <= u32::MAX as u64 { n + 76_561_197_960_265_728 } else { n }.to_string()));
        if let Some((account, plain)) = fmt::kh3::open(data, &fmt::kh3::candidates(&path.to_string_lossy(), &extra)) {
            ctx.hint = Some("Kingdom Hearts III save: decrypted for you; its CRC and MD5 are worked out again and it is encrypted again on save.".into());
            return Some((Layer::Kh3 { account }, plain));
        }
        if path.to_string_lossy().to_ascii_lowercase().contains("kingdom hearts iii") {
            ctx.hint = Some("This looks like an encrypted Kingdom Hearts III save, but the Steam account that opens it wasn't found. Open it from its own folder (…\\KINGDOM HEARTS III\\Steam\\<Steam id>\\SaveGames), or type the 17-digit Steam id into Tools \u{203A} Decrypt with password.".into());
        }
    }
    if fmt::fantasian::looks(data) {
        if let Some(recs) = fmt::fantasian::records(data) {
            let pick = ctx.opts.zip_entry.as_ref().and_then(|n| recs.iter().find(|r| &r.name == n)).or_else(|| recs.iter().find(|r| r.span.1 > r.span.0)).or(recs.first());
            if let Some((record, shown)) = pick.cloned().and_then(|r| fmt::fantasian::open(data, r)) {
                ctx.zip_entries = recs.iter().map(|r| r.name.clone()).collect();
                ctx.hint = Some("FANTASIAN Neo Dimension save: one save slot is open, decrypted (Tools \u{203A} Open another file in this archive lists the others). It is encrypted again on save and the other slots are left as they were.".into());
                return Some((Layer::Fantasian { original: Arc::new(data.to_vec()), record }, shown));
            }
        }
    }
    if fmt::deadcells::looks(data) {
        if let Some((header, chunks)) = fmt::deadcells::open(data) {
            ctx.hint = Some("Dead Cells save: its checksum and compression are handled for you. The values inside are hxbit binary data, so use the value search to find gold or cells.".into());
            return Some((Layer::DeadCells { header: Arc::new(header) }, chunks));
        }
    }
    if fmt::haxe::looks(data) {
        if let Some(json) = fmt::haxe::open(data) {
            return Some((Layer::Haxe { original: Arc::new(data.to_vec()) }, json));
        }
    }
    if data.starts_with(b"Windows Registry Editor") {
        if let Some((meta, json)) = std::str::from_utf8(data).ok().and_then(fmt::regfile::open) {
            return Some((Layer::RegFile(Arc::new(meta)), json));
        }
    }
    if let Some(o) = fmt::nested::open(data) {
        return Some((Layer::NestedJson { paths: o.paths, pretty: o.pretty }, o.bytes));
    }
    if let Some(r) = peel_arpg(data, ctx) {
        return r;
    }
    if let Some((header, state)) = fmt::luabins::open(data) {
        return Some((Layer::Supergiant { header: Arc::new(header) }, state));
    }
    if let Some(r) = peel_bnd4(data, ctx) {
        return Some(r);
    }
    if fmt::lime::looks(data) {
        return peel_lime(data, ctx);
    }
    if fmt::citrus::looks(data) {
        let Some((curve, steam)) = fmt::citrus::find_key(data, &ree_accounts(ctx)) else {
            ctx.hint = Some(
                "This is an encrypted Monster Hunter Rise save, but none of the Steam accounts on this computer opens it. \
                 Open it from its own folder (Steam\\userdata\\<account>\\1446780\\remote\\win64_save) so the account can be found, \
                 or enter your Steam ID (17 digits) in Tools \u{203A} Decrypt with password."
                    .into(),
            );
            return None;
        };
        let plain = fmt::citrus::decrypt(data, curve, steam)?;
        return Some((Layer::Citrus { original: Arc::new(data.to_vec()), curve, steam }, plain));
    }
    if fmt::mandarin::looks(data) {
        return peel_mandarin(data, ctx);
    }
    if data.starts_with(b"CIV5") {
        return peel_civ5(data, ctx);
    }
    if let Some(r) = peel_cdpr(data, ctx) {
        return Some(r);
    }
    if data.starts_with(b"RGMH") {
        let len = data.get(8..12).map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize)?;
        if data.get(len..len + 4) == Some(&b"PK\x03\x04"[..]) {
            return Some((Layer::Me1Header { header: Arc::new(data[..len].to_vec()) }, data[len..].to_vec()));
        }
    }
    if let Some(r) = peel_xcom(data, ctx) {
        return Some(r);
    }
    if let Some(inner) = fmt::borderlands::unpack(data) {
        return Some((Layer::Borderlands { original: Arc::new(data.to_vec()), plain: Arc::new(inner.clone()) }, inner));
    }
    if data.len() >= 64 && data.iter().all(|&b| b == 0) {
        ctx.hint = Some("This file is all zero bytes: there's nothing in it to edit. If the game crashed while saving, its backup (a .bak file next to it) may still be good.".into());
        return None;
    }
    if let Some(r) = peel_me1le(data, ctx) {
        return Some(r);
    }
    if matches!(ctx.opts.ext.as_str(), "mca" | "mcr" | "mcc") && ctx.zip_entries.is_empty() {
        if let Some(r) = peel_region(data, ctx) {
            return Some(r);
        }
    }
    if let Some(r) = peel_utf16(data) {
        return Some(r);
    }
    if data.starts_with(&[0x1F, 0x8B]) {
        if let Ok(v) = gunzip(data) {
            return Some((Layer::Gzip, v));
        }
    }
    if looks_zlib(data) {
        if let Ok((v, rest)) = inflate_with_rest(data) {
            if rest.is_empty() {
                return Some((Layer::Zlib, v));
            }
            // Ren'Py persistent: signature lines follow the compressed data.
            if std::str::from_utf8(rest)
                .is_ok_and(|t| t.lines().all(|l| l.trim().is_empty() || l.starts_with("signature ")))
            {
                return Some((Layer::RenpyPersistent { signed: true }, v));
            }
        }
    }
    if data.starts_with(&[0x28, 0xB5, 0x2F, 0xFD]) {
        if let Ok(v) = zstd::decode_all(data) {
            return Some((Layer::Zstd, v));
        }
    }
    if data.starts_with(b"\xff\x06\x00\x00sNaPpY") {
        let mut v = Vec::new();
        if snap::read::FrameDecoder::new(data).read_to_end(&mut v).is_ok() {
            return Some((Layer::Snappy, v));
        }
    }
    if data.starts_with(&[0x04, 0x22, 0x4D, 0x18]) {
        let mut v = Vec::new();
        if lz4_flex::frame::FrameDecoder::new(data).read_to_end(&mut v).is_ok() {
            return Some((Layer::Lz4, v));
        }
    }
    if data.starts_with(&[0xFD, b'7', b'z', b'X', b'Z', 0x00]) {
        let mut v = Vec::new();
        if lzma_rs::xz_decompress(&mut &data[..], &mut v).is_ok() {
            return Some((Layer::Xz, v));
        }
    }
    if data.len() > 10 && data.starts_with(b"BZh") && (b'1'..=b'9').contains(&data[3]) {
        let mut v = Vec::new();
        if bzip2::read::BzDecoder::new(data).read_to_end(&mut v).is_ok() {
            return Some((Layer::Bzip2, v));
        }
    }
    if data.len() > 12 && &data[8..11] == b"PlZ" && matches!(data[11], 0x31 | 0x32) {
        let double = data[11] == 0x32;
        let once = inflate(&data[12..]).ok();
        let plain = if double { once.and_then(|i| inflate(&i).ok()) } else { once };
        if let Some(p) = plain.filter(|p| p.len() == u32::from_le_bytes(data[0..4].try_into().unwrap()) as usize) {
            return Some((Layer::Palworld { double }, p));
        }
    }
    if data.len() > 64 {
        if let Some((chunks, p)) = fmt::bioshock::unpack(data) {
            return Some((Layer::Bioshock { chunks: Arc::new(chunks) }, p));
        }
    }
    if data.len() > 16 && data[..4] == data[4..8] && u32::from_le_bytes(data[..4].try_into().unwrap()) < 64 {
        let mut h = flate2::Crc::new();
        h.update(&data[12..]);
        if h.sum().to_le_bytes() == data[8..12] {
            return Some((Layer::RemedyCrc { head: data[..8].try_into().unwrap() }, data[12..].to_vec()));
        }
    }
    if let Some((w, p)) = fmt::rgg::decrypt(data) {
        return Some((Layer::Rgg(w), p));
    }
    if fmt::rgmh::looks(data) {
        if let Some((blocks, p)) = fmt::rgmh::unpack(data) {
            return Some((Layer::Rgmh { original: Arc::new(data.to_vec()), blocks: Arc::new(blocks) }, p));
        }
    }
    if data.starts_with(b"COPECK") {
        if let Some(r) = peel_copeck(data, ctx) {
            return Some(r);
        }
    }
    if data.starts_with(b"artm") {
        if let Some((head, p)) = fmt::metro::unpack_player(data) {
            return Some((Layer::MetroPlayer { head: Arc::new(head) }, p));
        }
    }
    if data.starts_with(b"DEZP") {
        if let Some((layout, p)) = fmt::metro::unpack_dezp(data) {
            return Some((Layer::MetroDezp { layout: Arc::new(layout) }, p));
        }
    }
    if data.starts_with(b"DATA") {
        if let Some((head, p)) = fmt::p5r::unpack(data) {
            return Some((Layer::P5r { head: Arc::new(head) }, p));
        }
    }
    if fmt::ds2::looks(data) {
        if let Some((w, p)) = fmt::ds2::unpack(data) {
            return Some((Layer::Ds2(Arc::new(w)), p));
        }
    }
    if data.len() > 117 && data[data.len() - 1] == 2 {
        if let Some((p, foot)) = fmt::ffxv::decrypt(data) {
            return Some((Layer::Ffxv { foot: Arc::new(foot) }, p));
        }
        if let Some((p, foot)) = fmt::forspoken::decrypt(data) {
            return Some((Layer::Forspoken { foot: Arc::new(foot) }, p));
        }
    }
    if let Some(p) = fmt::ao::unzstd(data) {
        return Some((Layer::Ao { kai: None }, p));
    }
    if let Some((p, orig)) = fmt::ao::uncle(data) {
        return Some((Layer::Ao { kai: Some(Arc::new(orig)) }, p));
    }
    if data.len() == fmt::ff9::FILE_LEN {
        if let Some(p) = fmt::ff9::decrypt(data) {
            return Some((Layer::Ff9, p));
        }
    }
    if let Some(r) = peel_hogwarts(data, ctx) {
        return Some(r);
    }
    if data.len() > 12 && &data[8..11] == b"PlM" {
        let raw = u32::from_le_bytes(data[0..4].try_into().unwrap()) as usize;
        let packed = u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;
        if packed + 12 == data.len() {
            if let Some(p) = crate::oodle::decompress(&data[12..], raw) {
                if let Some(lib) = crate::oodle::location() {
                    ctx.hint = Some(format!("Palworld's Oodle compression is read and written with the Oodle library from {}.", lib.display()));
                }
                return Some((Layer::PalworldOodle, p));
            }
        }
        ctx.hint = Some(crate::oodle::MISSING.into());
        return None;
    }
    if std::str::from_utf8(data).is_err() {
        if let Ok(v) = inflate_raw(data) {
            if ctx.opts.ext == "jkr" || (v.len() > 32 && is_recognisable(&v) && !looks_zlib(&v)) {
                return Some((Layer::Deflate, v));
            }
        }
    }
    if let Some(r) = peel_bundle(data, ctx) {
        return Some(r);
    }
    if let Some((info, plain)) = fmt::crypto::godot_decrypt(data, ctx.opts.password.as_deref()) {
        return Some((Layer::GodotEnc(info), plain));
    }
    if data.starts_with(b"GDEC") {
        ctx.hint = Some("This is an encrypted Godot save. Use Tools \u{203A} Decrypt with password if you know the game's password.".into());
        return None;
    }
    if let Some(plain) = fmt::crypto::terraria_decrypt(data) {
        return Some((Layer::Terraria, plain));
    }
    if let Some(plain) = fmt::crypto::hollow_knight_decrypt(data) {
        return Some((Layer::HollowKnight, plain));
    }
    if let Some((kind, version, plain)) = fmt::antimatter::open(data) {
        return Some((Layer::Antimatter { kind, version }, plain));
    }
    if let Some(plain) = fmt::p3r::steam_decrypt(data) {
        return Some((Layer::P3rSteam, plain));
    }
    if let Some(plain) = fmt::crypto::fallout_shelter_decrypt(data) {
        return Some((Layer::FalloutShelter, plain));
    }
    if let Some((password, iv, plain)) = fmt::crypto::es3_decrypt_any(data, ctx.opts.password.as_deref(), is_recognisable) {
        return Some((Layer::Es3 { password, iv }, plain));
    }
    if let Some((w, plain)) = fmt::keyed::open(data) {
        ctx.hint = Some(format!("{} save: decrypted for you and encrypted again on save.", w.game()));
        return Some((Layer::Keyed(w), plain));
    }
    if let Some((k, plain)) = peel_known_xor(data) {
        return Some((Layer::XorKey(k), plain));
    }
    if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        if let Some((chunk, body)) = png_find(data, &[*b"gbAs"]) {
            if let Ok(v) = inflate(&body) {
                return Some((Layer::PngChunk { png: Arc::new(data.to_vec()), chunk }, v));
            }
        }
    }
    if let Some((k, plain)) = peel_xor(data) {
        return Some((Layer::Xor(k), plain));
    }

    let text = std::str::from_utf8(data).ok()?;
    let trimmed = text.trim();
    if trimmed.is_empty() || parse_json(trimmed).is_some() || trimmed.starts_with('<') {
        return None;
    }

    if text.starts_with('x') && text.chars().all(|c| (c as u32) < 256) {
        let bytes: Vec<u8> = text.chars().map(|c| c as u8).collect();
        if looks_zlib(&bytes) {
            if let Ok(v) = inflate(&bytes) {
                return Some((Layer::RpgMzZlib, v));
            }
        }
    }

    if trimmed.len() >= 8 && is_base64_charset(trimmed) {
        if let Some(wide) = lz_str::decompress_from_base64(trimmed) {
            if let Ok(s) = String::from_utf16(&wide) {
                if parse_json(s.trim()).is_some() {
                    return Some((Layer::LzString, s.into_bytes()));
                }
            }
        }
        if let Ok(bytes) = STANDARD.decode(trimmed) {
            if is_recognisable(&bytes) || peel_known_xor(&bytes).is_some() {
                return Some((Layer::Base64, bytes));
            }
        }
    }
    if let Some(r) = peel_base64_lines(text) {
        return Some(r);
    }
    if trimmed.contains('%') && !trimmed.contains(char::is_whitespace) {
        if let Some(plain) = js_unescape(trimmed) {
            if parse_json(plain.trim()).is_some() && js_escape(&plain) == trimmed {
                return Some((Layer::JsEscape, plain.into_bytes()));
            }
        }
    }
    None
}

/// Grim Dawn and Torchlight II characters. `Some(None)`: recognised, but can't be opened.
fn peel_arpg(data: &[u8], ctx: &mut Ctx) -> Option<Option<(Layer, Vec<u8>)>> {
    match fmt::grimdawn::decrypt(data) {
        Some(Ok((seed, plain, plan))) => return Some(Some((Layer::GrimDawn { seed, plan: Arc::new(plan) }, plain))),
        Some(Err(e)) => {
            ctx.hint = Some(format!("This is a Grim Dawn character, but part of it is laid out in a way this editor doesn't know ({e}), so it can't be edited safely."));
            return Some(None);
        }
        None => {}
    }
    if let Some(plain) = fmt::torchlight2::unpack(data) {
        return Some(Some((Layer::Torchlight2, plain)));
    }
    if fmt::torchlight2::is_old(data) {
        ctx.hint = Some("This is a Torchlight II character from an older version of the game (format 0x40). Its layout is different and isn't supported.".into());
        return Some(None);
    }
    None
}

/// FromSoftware BND4 save: open the chosen `USER_DATA` entry (default: first character).
fn peel_bnd4(data: &[u8], ctx: &mut Ctx) -> Option<(Layer, Vec<u8>)> {
    let entries = fmt::containers::bnd4_entries(data)?;
    let chosen = match ctx.opts.zip_entry.as_ref().and_then(|n| entries.iter().position(|e| &e.name == n)) {
        Some(i) => i,
        None => fmt::containers::bnd4_default(data, &entries)?,
    };
    let (plain, key, iv) = fmt::containers::bnd4_open(data, &entries[chosen])?;
    ctx.zip_entries = entries.iter().map(|e| e.name.clone()).collect();
    Some((Layer::Bnd4 { archive: Arc::new(data.to_vec()), name: entries[chosen].name.clone(), key, iv }, plain))
}

/// Minecraft region file: open the chosen chunk (default: the first one).
/// Hogwarts Legacy: `RawDatabaseImage` (an ArrayProperty of bytes) holds the game's
/// SQLite database in Unreal compressed chunks (each: tag, block size, packed and unpacked
/// size, the same for its one block, then Oodle data).
fn peel_hogwarts(data: &[u8], ctx: &mut Ctx) -> Option<(Layer, Vec<u8>)> {
    const NAME: &[u8] = b"\x11\x00\x00\x00RawDatabaseImage\x00";
    const TYPE: &[u8] = b"\x0e\x00\x00\x00ArrayProperty\x00";
    const INNER: &[u8] = b"\x0d\x00\x00\x00ByteProperty\x00";
    if !data.starts_with(b"GVAS") {
        return None;
    }
    let at = data.windows(NAME.len()).position(|w| w == NAME)? + NAME.len();
    if data.get(at..at + TYPE.len())? != TYPE {
        return None;
    }
    let size_at = at + TYPE.len();
    let p = size_at + 8;
    if data.get(p..p + INNER.len())? != INNER {
        return None;
    }
    let count_at = p + INNER.len() + 1; // after the "has GUID" byte
    let count = u32::from_le_bytes(data.get(count_at..count_at + 4)?.try_into().ok()?) as usize;
    let start = count_at + 4;
    let end = start.checked_add(count).filter(|&e| e <= data.len())?;
    let u64_at = |o: usize| data.get(o..o + 8).map(|b| u64::from_le_bytes(b.try_into().unwrap()));
    let mut q = start;
    let mut parts = vec![];
    let mut block = 0;
    while q < end {
        if u64_at(q)? != 0x9E2A_83C1 {
            return None;
        }
        block = u64_at(q + 8)?;
        let (c, u) = (u64_at(q + 16)? as usize, u64_at(q + 24)? as usize);
        if u64_at(q + 32)? as usize != c || u64_at(q + 40)? as usize != u {
            return None;
        }
        parts.push((data.get(q + 48..q + 48 + c)?, u));
        q += 48 + c;
    }
    if q != end || parts.is_empty() {
        return None;
    }
    let mut inner = Vec::new();
    for b in crate::par::map(&parts, |&(b, u)| crate::oodle::decompress(b, u)) {
        match b {
            Some(b) => inner.extend(b),
            None => {
                ctx.hint = Some(format!("Hogwarts Legacy keeps gold, experience and the inventory in an Oodle-compressed database. {}", crate::oodle::MISSING));
                return None;
            }
        }
    }
    let n = u32::from_le_bytes(inner.get(4..8)?.try_into().ok()?) as usize;
    let db = inner.get(8..8 + n)?.to_vec();
    if !db.starts_with(b"SQLite format 3\0") {
        return None;
    }
    ctx.hint = Some("Hogwarts Legacy: the game's database is unpacked here (gold, experience, talent points, inventory); it's packed again with Oodle when you save.".into());
    Some((Layer::HogwartsDb { head: Arc::new(data[..start].to_vec()), size_at, count_at, tail: Arc::new(data[end..].to_vec()), block }, db))
}

/// Mass Effect 1 (Legendary Edition): tag, block size, packed and unpacked totals, a
/// (packed, unpacked) size per block, the zlib blocks, then CRC, 1 and the unpacked size.
fn peel_me1le(data: &[u8], ctx: &mut Ctx) -> Option<(Layer, Vec<u8>)> {
    let u = |o: usize| data.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize);
    if u(0)? != 0x9E2A_83C1 || data.len() < 40 {
        return None;
    }
    let (bs, cs, us) = (u(4)?, u(8)?, u(12)?);
    if bs == 0 || us == 0 {
        return None;
    }
    let n = us.div_ceil(bs);
    let mut at = 16 + 8 * n;
    if at + cs + 12 != data.len() || u(data.len() - 4)? != us || u(data.len() - 8)? != 1 {
        return None;
    }
    if fmt::crc32_bzip2(&data[..data.len() - 12]) as usize != u(data.len() - 12)? {
        return None;
    }
    let mut plain = Vec::with_capacity(us);
    for i in 0..n {
        let c = u(16 + 8 * i)?;
        let mut d = ZlibDecoder::new(data.get(at..at + c)?);
        std::io::Read::read_to_end(&mut d, &mut plain).ok()?;
        at += c;
    }
    if plain.len() != us {
        return None;
    }
    ctx.hint = Some(
        "Mass Effect (Legendary Edition) save: unpacked here so Value search can find a number you know (credits,          experience); it's packed again and its checksum updated when you save."
            .into(),
    );
    Some((Layer::Me1Le { block: bs as u32 }, plain))
}

/// XCOM: the header (1024 bytes in Enemy Unknown / Within, its own size at offset 4 in
/// XCOM 2), then chunks: tag, block size, packed and unpacked size, the same for its one
/// block, and the LZO1X block.
fn peel_xcom(data: &[u8], ctx: &mut Ctx) -> Option<(Layer, Vec<u8>)> {
    const TAG: [u8; 4] = [0xC1, 0x83, 0x2A, 0x9E];
    let u = |o: usize| data.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize);
    let version = u(0)?;
    let (start, xcom2) = if matches!(version, 0x0F | 0x10) && data.get(1024..1028) == Some(&TAG[..]) {
        (1024, false)
    } else if (0x13..0x40).contains(&version) && u(4).is_some_and(|h| h > 16 && data.get(h..h + 4) == Some(&TAG[..])) {
        (u(4)?, true)
    } else {
        return None;
    };
    let mut at = start;
    let mut blocks = vec![];
    let mut block = 0;
    while at < data.len() {
        if data.get(at..at + 4) != Some(&TAG[..]) {
            return None;
        }
        let (bs, cs, us) = (u(at + 4)?, u(at + 8)?, u(at + 12)?);
        if us > bs || u(at + 16)? != cs || u(at + 20)? != us {
            return None;
        }
        block = bs as u32;
        blocks.push((data.get(at + 24..at + 24 + cs)?, us));
        at += 24 + cs;
    }
    let crc = fmt::crc32_bzip2(&data[start..]).to_le_bytes();
    let crc_at = data[..start].windows(4).position(|w| w == crc)?;
    let mut plain = Vec::new();
    for b in crate::par::map(&blocks, |&(b, us)| std::panic::catch_unwind(|| lzokay_native::decompress_all(b, Some(us))).ok().and_then(|r| r.ok())) {
        plain.extend(b?);
    }
    let sizes = blocks.iter().map(|b| b.1 as u32).collect();
    ctx.hint = Some(format!(
        "{} save: the game data is unpacked here so Value search can find a number you know (money, a soldier's stat); \
         it's packed again and its checksums updated when you save.",
        if xcom2 { "XCOM 2" } else { "XCOM: Enemy Unknown / Within" }
    ));
    Some((Layer::Xcom { header: Arc::new(data[..start].to_vec()), crc_at, xcom2, sizes, block }, plain))
}

/// Cyberpunk 2077 / The Witcher 3: `FZLC`, chunk count, where the first chunk starts, then
/// (compressed size, unpacked size, end) per chunk (the last one's end is left 0).
fn peel_cdpr(data: &[u8], ctx: &mut Ctx) -> Option<(Layer, Vec<u8>)> {
    let fzlc = data[..data.len().min(64)].windows(4).position(|w| w == b"FZLC")?;
    let u = |o: usize| data.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize);
    let n = u(fzlc + 4)?;
    let start = u(fzlc + 8)?;
    if n == 0 || n > 4096 || start > data.len() || fzlc + 12 + 12 * n > start {
        return None;
    }
    let mut at = start;
    let mut parts = vec![];
    let mut sizes = vec![];
    for k in 0..n {
        let (c, un) = (u(fzlc + 12 + 12 * k)?, u(fzlc + 16 + 12 * k)?);
        let chunk = data.get(at..at + c)?;
        parts.push((if chunk.starts_with(b"4ZLX") { chunk.get(8..)? } else { chunk }, un));
        sizes.push(un as u32);
        at += c;
        if k + 1 < n && u(fzlc + 20 + 12 * k)? != at {
            return None;
        }
    }
    let mut plain = Vec::new();
    for c in crate::par::map(&parts, |&(body, un)| lz4_flex::block::decompress(body, un).ok()) {
        plain.extend(c?);
    }
    ctx.hint = Some(format!(
        "The game data is unpacked here so Value search can find a number you know (money, a stat); \
         it's packed again when you save. Check the save in the game afterwards \u{2014} the previous version is kept as a backup."
    ));
    let tagged = data[start..].starts_with(b"4ZLX");
    Some((Layer::Cdpr { prefix: Arc::new(data[..start].to_vec()), fzlc, sizes, footer: Arc::new(data[at..].to_vec()), tagged }, plain))
}

/// Civilization V: the header's readable entries (civilization, difficulty, eras, speed,
/// map size) for the status line, and the zlib-compressed game for value search.
const CIV5_CHUNK: usize = 0x10000;

/// The compressed game data of a Civilization V save: chunks of up to 64 KiB, each after its
/// u32 length, starting at `at` (the first chunk's length). Returns the stream put back
/// together and where the last chunk ends.
fn civ5_chunks(data: &[u8], at: usize) -> Option<(Vec<u8>, usize)> {
    let mut z = Vec::new();
    let mut p = at;
    loop {
        let len = u32::from_le_bytes(data.get(p..p + 4)?.try_into().ok()?) as usize;
        if len == 0 || len > CIV5_CHUNK || p + 4 + len > data.len() {
            return None;
        }
        z.extend_from_slice(&data[p + 4..p + 4 + len]);
        p += 4 + len;
        if len < CIV5_CHUNK || p == data.len() {
            return Some((z, p));
        }
    }
}

fn civ5_inflate(z: &[u8]) -> Option<(Vec<u8>, usize)> {
    // The game ends the stream with a sync flush instead of a final block, so the decoder
    // stops with "incomplete" once everything is out.
    let mut d = flate2::read::ZlibDecoder::new(z);
    let mut plain = Vec::new();
    match d.read_to_end(&mut plain) {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {}
        Err(_) => return None,
    }
    (plain.len() > 1024).then_some((plain, d.total_in() as usize))
}

fn peel_civ5(data: &[u8], ctx: &mut Ctx) -> Option<(Layer, Vec<u8>)> {
    // The first chunked zlib stream that unpacks completely.
    let chunked = (4..data.len().min(1 << 20).saturating_sub(2)).find_map(|i| {
        if data[i] != 0x78 || !matches!(data[i + 1], 0x9C | 0xDA | 0x01) {
            return None;
        }
        let (z, end) = civ5_chunks(data, i - 4)?;
        let (plain, used) = civ5_inflate(&z)?;
        (used == z.len()).then_some((i - 4, plain, end))
    });
    if let Some((at, plain, end)) = chunked {
        civ5_hint(data, at, ctx);
        let layer = Layer::Civ5 { header: Arc::new(data[..at].to_vec()), chunked: true, trailer: Arc::new(data[end..].to_vec()) };
        return Some((layer, plain));
    }
    // Otherwise the first zlib stream that unpacks completely and runs to the end of the file.
    let (at, plain) = (0..data.len().min(1 << 20).saturating_sub(2)).find_map(|i| {
        if data[i] != 0x78 || !matches!(data[i + 1], 0x9C | 0xDA | 0x01) {
            return None;
        }
        // The game ends the stream with a sync flush instead of a final block, so the
        // decoder stops with "incomplete" once everything is out.
        let mut d = flate2::read::ZlibDecoder::new(&data[i..]);
        let mut plain = Vec::new();
        match d.read_to_end(&mut plain) {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {}
            Err(_) => return None,
        }
        (plain.len() > 1024 && i + d.total_in() as usize == data.len()).then_some((i, plain))
    })?;
    civ5_hint(data, at, ctx);
    Some((Layer::Civ5 { header: Arc::new(data[..at].to_vec()), chunked: false, trailer: Arc::new(vec![]) }, plain))
}

/// What the save's header says about the game, as the hint above the data.
fn civ5_hint(data: &[u8], at: usize, ctx: &mut Ctx) {
    let words: Vec<String> = data[..at]
        .split(|&b| b < 0x20 || b > 0x7E)
        .filter(|w| w.len() >= 6 && w.iter().all(|c| c.is_ascii_uppercase() || *c == b'_'))
        .map(|w| String::from_utf8_lossy(w).to_string())
        .filter(|w| ["CIVILIZATION_", "HANDICAP_", "ERA_", "GAMESPEED_", "WORLDSIZE_"].iter().any(|p| w.starts_with(p)))
        .collect();
    let nice = |w: &str| crate::game_ui::words(&w.splitn(2, '_').nth(1).unwrap_or(w).to_lowercase());
    let mut parts = vec![];
    for (p, label) in [("CIVILIZATION_", ""), ("HANDICAP_", " difficulty"), ("GAMESPEED_", " speed"), ("WORLDSIZE_", " map")] {
        if let Some(w) = words.iter().find(|w| w.starts_with(p)) {
            parts.push(format!("{}{label}", nice(w)));
        }
    }
    if let Some(e) = words.iter().filter(|w| w.starts_with("ERA_")).last() {
        parts.push(format!("{} era", nice(e)));
    }
    ctx.hint = Some(format!(
        "{}. The game data is unpacked here so Value search can find a number you know (gold, faith, a city's population); it's packed again when you save.",
        parts.join(", ")
    ));
}

/// Steam accounts to try on an RE Engine save: from its path, this computer, and a Steam ID
/// typed into Tools › Decrypt with password.
fn ree_accounts(ctx: &Ctx) -> Vec<u64> {
    let path = ctx.opts.path.clone().unwrap_or_default();
    let mut accounts = fmt::mandarin::accounts_for(&path);
    if let Some(id) = ctx.opts.password.as_deref().and_then(|p| p.trim().parse::<u64>().ok()) {
        accounts.insert(0, id);
    }
    accounts
}

/// Resident Evil 4 (2023): find the Steam account that opens it.
fn peel_lime(data: &[u8], ctx: &mut Ctx) -> Option<(Layer, Vec<u8>)> {
    let Some(user) = fmt::lime::find_user(data, &ree_accounts(ctx)) else {
        ctx.hint = Some(
            "This is an encrypted Resident Evil 4 save, but none of the Steam accounts on this computer opens it. \
             Open it from its own folder (Steam\\userdata\\<account>\\2050650\\remote\\win64_save) so the account can be found, \
             or enter your Steam ID (17 digits) in Tools \u{203A} Decrypt with password."
                .into(),
        );
        return None;
    };
    let plain = fmt::lime::decrypt(data, user)?;
    Some((Layer::Lime { original: Arc::new(data.to_vec()), user }, plain))
}

/// A Capcom RE Engine save: find the game and Steam account that open it.
fn peel_mandarin(data: &[u8], ctx: &mut Ctx) -> Option<(Layer, Vec<u8>)> {
    use fmt::mandarin;
    let path = ctx.opts.path.clone().unwrap_or_default();
    let mut accounts = mandarin::accounts_for(&path);
    // A Steam ID typed into Tools › Decrypt with password (needed away from Steam's folders,
    // e.g. on a phone): the 64-bit id or the account number.
    if let Some(id) = ctx.opts.password.as_deref().and_then(|p| p.trim().parse::<u64>().ok()) {
        accounts.insert(0, if id > u32::MAX as u64 { id & 0xFFFF_FFFF } else { id });
        if id > u32::MAX as u64 {
            accounts.insert(0, id);
        }
    }
    let found = mandarin::find_key(data, &path.to_string_lossy(), &accounts);
    let Some((game, user)) = found else {
        ctx.hint = Some(
            "This is an encrypted Capcom (RE Engine) save, but none of the Steam accounts on this computer opens it. \
             Open it from its own folder (Steam\\userdata\\<account>\\<game>\\remote\\win64_save) so the account can be found, \
             or enter your Steam ID (17 digits) in Tools \u{203A} Decrypt with password."
                .into(),
        );
        return None;
    };
    let plain = mandarin::decrypt(data, game, user)?;
    let index = mandarin::GAMES.iter().position(|g| std::ptr::eq(g, game))?;
    Some((Layer::Mandarin { original: Arc::new(data.to_vec()), game: index, user }, plain))
}

fn peel_region(data: &[u8], ctx: &mut Ctx) -> Option<(Layer, Vec<u8>)> {
    let chunks = fmt::containers::region_chunks(data)?;
    let (index, _) = ctx.opts.zip_entry.as_ref().and_then(|n| chunks.iter().find(|c| &c.1 == n)).unwrap_or(&chunks[0]).clone();
    let (nbt, kind) = fmt::containers::region_open(data, index)?;
    ctx.zip_entries = chunks.into_iter().map(|c| c.1).collect();
    Some((Layer::Region { archive: Arc::new(data.to_vec()), index, kind }, nbt))
}

/// Repeating-key XOR keys used by known games, with the game's name.
const KNOWN_XOR_KEYS: &[(&str, &str)] = &[
    ("", "key"),
    // Bounty of One (gamedata.sav), as game-bounty-of-one-save-editor (n0isy, MIT) gives it.
    ("Bounty of One", "cheh"),
    // Idle Slayer (savedata.sav), the phrase the game itself leaves in the file.
    ("Idle Slayer", "If you manage to get this string you are allowed to hack the game all you want"),
    // CloverPit (GameDataFull.json): the game's password "uoiyiuh_+=-5216gh;lj??!/345" with each
    // byte XORed with the one (its value mod the length) places further on, as
    // cloverpit-save-editor (jexxme, GPL-3.0) and Serialize-Deserialize-Save-CloverPit (MIT) do.
    ("CloverPit", "H\x06\\\x11\x06C\x01`\x18UB\x18\x19\x1a\x00JZ\x1a\x00QVFNG\x0c\x1b\x01"),
];

fn xor_key(data: &[u8], key: &[u8]) -> Vec<u8> {
    data.iter().zip(key.iter().cycle()).map(|(b, k)| b ^ k).collect()
}

fn peel_known_xor(data: &[u8]) -> Option<(String, Vec<u8>)> {
    if std::str::from_utf8(data).is_ok_and(|t| parse_json(t.trim()).is_some()) {
        return None;
    }
    KNOWN_XOR_KEYS.iter().find_map(|(_, k)| {
        let plain = xor_key(data, k.as_bytes());
        std::str::from_utf8(&plain).ok().and_then(|t| parse_json(t.trim())).map(|_| (k.to_string(), plain))
    })
}

/// Find the first chunk of one of `kinds` in a PNG; returns its type and data.
fn png_find(png: &[u8], kinds: &[[u8; 4]]) -> Option<([u8; 4], Vec<u8>)> {
    let mut i = 8;
    while i + 12 <= png.len() {
        let len = u32::from_be_bytes(png[i..i + 4].try_into().ok()?) as usize;
        let kind: [u8; 4] = png[i + 4..i + 8].try_into().ok()?;
        let data = png.get(i + 8..i + 8 + len)?;
        if kinds.contains(&kind) {
            return Some((kind, data.to_vec()));
        }
        i += 12 + len;
    }
    None
}

/// Replace a chunk's data (with a fresh CRC), keeping every other chunk as it was.
fn png_replace(png: &[u8], kind: &[u8; 4], data: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = png[..8].to_vec();
    let mut i = 8;
    let mut done = false;
    while i + 12 <= png.len() {
        let len = u32::from_be_bytes(png[i..i + 4].try_into().unwrap()) as usize;
        let end = i + 12 + len;
        if end > png.len() {
            return Err("damaged PNG".into());
        }
        if &png[i + 4..i + 8] == kind && !done {
            let mut crc = flate2::Crc::new();
            crc.update(kind);
            crc.update(data);
            out.extend_from_slice(&(data.len() as u32).to_be_bytes());
            out.extend_from_slice(kind);
            out.extend_from_slice(data);
            out.extend_from_slice(&crc.sum().to_be_bytes());
            done = true;
        } else {
            out.extend_from_slice(&png[i..end]);
        }
        i = end;
    }
    out.extend_from_slice(&png[i..]);
    done.then_some(out).ok_or_else(|| "chunk not found".into())
}

/// How much a file name looks like the main save data (higher = more likely).
fn save_like(name: &str) -> u8 {
    let n = name.to_lowercase();
    ["session", "save", "player", "profile", "progress", "game"].iter().filter(|k| n.contains(*k)).count() as u8
}

/// Cairn "COPECK" pack: from 0x10, directory entries of 0xA0 bytes (u32 offset, u32 size,
/// u32 size unpacked (gzip files; 0 when stored), u64 modification time, a byte, the
/// NUL-terminated name) until one without a name;
/// deleted entries start with FFFFFFFF. Files are stored from 0x50000 on, each at a 16-byte
/// boundary after the last.
fn copeck_entries(data: &[u8]) -> Option<Vec<(usize, String, usize, usize, u64)>> {
    let u32_at = |p: usize| data.get(p..p + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize);
    let mut out = Vec::new();
    let mut p = 0x10;
    while p + 0xA0 <= data.len() {
        let raw = data[p + 0x15..p + 0xA0].split(|&b| b == 0).next()?;
        if raw.is_empty() {
            break;
        }
        if !raw.iter().all(|&c| (32..127).contains(&c)) {
            return None;
        }
        let (at, size) = (u32_at(p)?, u32_at(p + 4)?);
        if at != 0xFFFF_FFFF {
            if at.checked_add(size)? > data.len() {
                return None;
            }
            let time = u64::from_le_bytes(data[p + 0x0C..p + 0x14].try_into().ok()?) & 0x3FFF_FFFF_FFFF_FFFF;
            out.push((p, String::from_utf8_lossy(raw).into_owned(), at, size, time));
        }
        p += 0xA0;
    }
    (!out.is_empty()).then_some(out)
}

fn peel_copeck(data: &[u8], ctx: &mut Ctx) -> Option<(Layer, Vec<u8>)> {
    let files = copeck_entries(data)?;
    // The file asked for, else the newest save.
    let chosen = ctx
        .opts
        .zip_entry
        .as_ref()
        .and_then(|n| files.iter().find(|f| &f.1 == n))
        .or_else(|| files.iter().filter(|f| f.1.ends_with(".SAV")).max_by_key(|f| f.4))
        .or_else(|| files.iter().max_by_key(|f| f.3))?
        .clone();
    ctx.zip_entries = files.iter().map(|f| f.1.clone()).collect();
    let bytes = data[chosen.2..chosen.2 + chosen.3].to_vec();
    Some((Layer::Copeck { pack: Arc::new(data.to_vec()), name: chosen.1, entry: chosen.0 }, bytes))
}

/// The save Cairn continues from: the newest .SAV in the pack.
pub fn copeck_newest(layers: &[Layer]) -> Option<String> {
    layers.iter().find_map(|l| match l {
        Layer::Copeck { pack, .. } => copeck_entries(pack)?.into_iter().filter(|f| f.1.ends_with(".SAV")).max_by_key(|f| f.4).map(|f| f.1),
        _ => None,
    })
}

/// Puts a file back: in its place if it fits, else after the last file.
fn copeck_put(pack: &[u8], entry: usize, data: &[u8]) -> Result<Vec<u8>, String> {
    let files = copeck_entries(pack).ok_or("the Cairn save pack can't be read")?;
    let (_, _, at, size, _) = files.iter().find(|f| f.0 == entry).cloned().ok_or("the file is missing from the pack")?;
    let mut out = pack.to_vec();
    let new_at = if data.len() <= size {
        out[at..at + size].fill(0);
        at
    } else {
        let end = files.iter().map(|f| f.2 + f.3).max().unwrap_or(0);
        let a = (end + 15) & !15;
        if a + data.len() > out.len() {
            return Err("the Cairn save pack is full".into());
        }
        out[at..at + size].fill(0);
        a
    };
    out[new_at..new_at + data.len()].copy_from_slice(data);
    out[entry..entry + 4].copy_from_slice(&(new_at as u32).to_le_bytes());
    out[entry + 4..entry + 8].copy_from_slice(&(data.len() as u32).to_le_bytes());
    // Then the size unpacked: gzip keeps it in its last four bytes (0 for a stored file).
    // The game checks it, so a save that grew must say so.
    if data.starts_with(&[0x1F, 0x8B]) && data.len() >= 18 {
        let isize: [u8; 4] = data[data.len() - 4..].try_into().unwrap();
        out[entry + 8..entry + 12].copy_from_slice(&isize);
    }
    Ok(out)
}

/// Barotrauma-style bundle: repeated [i32 name length (UTF-16 units), name, i32 size, bytes].
/// Must consume the data exactly, with sensible names.
fn peel_bundle(data: &[u8], ctx: &mut Ctx) -> Option<(Layer, Vec<u8>)> {
    let i32_at = |p: usize| data.get(p..p + 4).map(|b| i32::from_le_bytes(b.try_into().unwrap()));
    let mut files = Vec::new();
    let mut pos = 0;
    while pos < data.len() {
        let nc = i32_at(pos)?;
        if !(1..=512).contains(&nc) {
            return None;
        }
        pos += 4;
        let units: Vec<u16> = data.get(pos..pos + nc as usize * 2)?.chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        let name = String::from_utf16(&units).ok()?;
        if name.chars().any(|c| c.is_control()) {
            return None;
        }
        pos += nc as usize * 2;
        let len = usize::try_from(i32_at(pos)?).ok()?;
        pos += 4;
        files.push((name, data.get(pos..pos + len)?.to_vec()));
        pos += len;
    }
    if files.is_empty() {
        return None;
    }
    let names: Vec<String> = files.iter().map(|f| f.0.clone()).collect();
    let chosen = match ctx.opts.zip_entry.as_ref().filter(|n| names.contains(n)) {
        Some(n) => n.clone(),
        None => {
            let mut ranked: Vec<&(String, Vec<u8>)> = files.iter().collect();
            ranked.sort_by_key(|(n, _)| std::cmp::Reverse(save_like(n)));
            ranked
                .iter()
                .find(|(_, b)| !matches!(decode(b).content, Content::Binary(_)))
                .or_else(|| ranked.iter().max_by_key(|(_, b)| b.len()))?
                .0
                .clone()
        }
    };
    let bytes = files.iter().find(|f| f.0 == chosen)?.1.clone();
    ctx.zip_entries = names;
    Some((Layer::FileBundle { files: Arc::new(files), name: chosen }, bytes))
}

fn peel_utf16(data: &[u8]) -> Option<(Layer, Vec<u8>)> {
    let big_endian = match data.get(..2)? {
        [0xFF, 0xFE] => false,
        [0xFE, 0xFF] => true,
        _ => return None,
    };
    let body = &data[2..];
    if body.len() % 2 != 0 {
        return None;
    }
    let units: Vec<u16> = body
        .chunks(2)
        .map(|c| if big_endian { u16::from_be_bytes([c[0], c[1]]) } else { u16::from_le_bytes([c[0], c[1]]) })
        .collect();
    let s = String::from_utf16(&units).ok()?;
    is_printable(&s).then(|| (Layer::Utf16 { big_endian }, s.into_bytes()))
}

/// Single-byte XOR obfuscation: try the keys that would turn the first byte into
/// '{', '[' or '<', and accept only if the result is valid JSON/XML.
fn peel_xor(data: &[u8]) -> Option<(u8, Vec<u8>)> {
    if data.len() < 8 {
        return None;
    }
    // XOR with a key below 0x80 keeps ASCII as ASCII, so the input may look like text;
    // but never "decode" data that already is JSON/XML.
    if let Ok(t) = std::str::from_utf8(data) {
        let t = t.trim_start();
        if t.starts_with('<') || parse_json(t.trim_end()).is_some() {
            return None;
        }
    }
    for target in [b'{', b'[', b'<'] {
        let k = data[0] ^ target;
        if k == 0 {
            continue;
        }
        let plain: Vec<u8> = data.iter().map(|b| b ^ k).collect();
        let Ok(text) = std::str::from_utf8(&plain) else { continue };
        let t = text.trim();
        if parse_json(t).is_some() || (target == b'<' && fmt::xml::looks_like_xml(t)) {
            return Some((k, plain));
        }
    }
    None
}

fn peel_base64_lines(text: &str) -> Option<(Layer, Vec<u8>)> {
    let crlf = text.contains("\r\n");
    let lines: Vec<&str> = text.trim_end().split(if crlf { "\r\n" } else { "\n" }).collect();
    if lines.len() < 2 {
        return None;
    }
    let width = lines[0].len();
    let uniform = width >= 16
        && lines[..lines.len() - 1].iter().all(|l| l.len() == width)
        && lines.last().is_some_and(|l| !l.is_empty() && l.len() <= width)
        && lines.iter().all(|l| is_base64_charset(l));
    if !uniform || text.trim_end() != text.trim_end_matches(['\r', '\n']) {
        return None;
    }
    let joined: String = lines.concat();
    let bytes = STANDARD.decode(&joined).ok()?;
    // Must round-trip exactly (apart from a trailing newline, which we don't keep).
    let layer = Layer::Base64Lines { width, crlf };
    (is_recognisable(&bytes) && wrap(&layer, bytes.clone()).ok()?.as_slice() == text.trim_end().as_bytes())
        .then_some((layer, bytes))
}

/// JavaScript `escape()`.
fn js_escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() || "@*_+-./".contains(c) {
            out.push(c);
        } else if (c as u32) < 256 {
            out.push_str(&format!("%{:02X}", c as u32));
        } else {
            let mut buf = [0u16; 2];
            for u in c.encode_utf16(&mut buf) {
                out.push_str(&format!("%u{:04X}", u));
            }
        }
    }
    out
}

/// JavaScript `unescape()`.
fn js_unescape(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut units: Vec<u16> = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            if b.get(i + 1) == Some(&b'u') {
                units.push(u16::from_str_radix(s.get(i + 2..i + 6)?, 16).ok()?);
                i += 6;
            } else {
                units.push(u16::from_str_radix(s.get(i + 1..i + 3)?, 16).ok()?);
                i += 3;
            }
        } else {
            units.push(b[i] as u16);
            i += 1;
        }
    }
    String::from_utf16(&units).ok()
}

fn classify(data: Vec<u8>, opts: &Options) -> Content {
    if pickle::looks_like_pickle(&data) {
        if let Ok(doc) = pickle::parse(&data) {
            return Content::Tree(Box::new(doc));
        }
    }
    if let Some(doc) = fmt::parse_aurora(&data, opts.path.as_deref()) {
        return Content::Tree(Box::new(doc));
    }
    if let Some(doc) = fmt::parse_binary(&data) {
        return Content::Tree(Box::new(doc));
    }
    match String::from_utf8(data) {
        Ok(text) => {
            let t = text.trim();
            if let Some(value) = parse_json(t) {
                let pretty = t.contains('\n');
                Content::Json { value, pretty }
            } else if let Some(doc) = fmt::parse_text(&text, &opts.ext) {
                Content::Tree(Box::new(doc))
            } else if is_printable(&text) {
                Content::Text(text)
            } else {
                Content::Binary(text.into_bytes())
            }
        }
        Err(e) => {
            let bytes = e.into_bytes();
            match fmt::parse_binary_late(&bytes) {
                Some(doc) => Content::Tree(Box::new(doc)),
                None => Content::Binary(bytes),
            }
        }
    }
}

pub fn parse_json(s: &str) -> Option<Value> {
    if !(s.starts_with('{') || s.starts_with('[')) {
        return None;
    }
    serde_json::from_str(s).ok()
}

fn is_recognisable(bytes: &[u8]) -> bool {
    if bytes.starts_with(&[0x1F, 0x8B]) || looks_zlib(bytes) {
        return true;
    }
    match std::str::from_utf8(bytes) {
        Ok(s) => parse_json(s.trim()).is_some() || (s.len() >= 4 && is_printable(s)),
        Err(_) => false,
    }
}

fn is_printable(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| !c.is_control() || matches!(c, '\n' | '\r' | '\t'))
}

fn is_base64_charset(s: &str) -> bool {
    s.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'='))
}

fn looks_zlib(d: &[u8]) -> bool {
    d.len() >= 2 && d[0] & 0x0F == 8 && d[0] >> 4 <= 7 && (u16::from(d[0]) * 256 + u16::from(d[1])) % 31 == 0
}

fn trailing_data() -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, "extra data after the compressed stream")
}

/// Decompress gzip, but only if the stream covers the *whole* input. Extra bytes
/// (a second gzip member, a checksum, padding) would be lost on re-save, so in that
/// case we refuse and the file is treated as raw binary instead.
fn gunzip(d: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut dec = GzDecoder::new(d);
    dec.read_to_end(&mut out)?;
    if !dec.into_inner().is_empty() {
        return Err(trailing_data());
    }
    Ok(out)
}

/// Decompress zlib, requiring the stream to cover the whole input (see `gunzip`).
fn inflate(d: &[u8]) -> std::io::Result<Vec<u8>> {
    match inflate_with_rest(d)? {
        (out, []) => Ok(out),
        _ => Err(trailing_data()),
    }
}

/// Decompress headerless deflate, requiring it to cover the whole input.
fn inflate_raw(d: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut dec = flate2::bufread::DeflateDecoder::new(d);
    dec.read_to_end(&mut out)?;
    if !dec.into_inner().is_empty() || out.is_empty() {
        return Err(trailing_data());
    }
    Ok(out)
}

/// Decompress a zlib stream, also returning any bytes that follow it.
fn inflate_with_rest(d: &[u8]) -> std::io::Result<(Vec<u8>, &[u8])> {
    let mut out = Vec::new();
    let mut dec = ZlibDecoder::new(d);
    dec.read_to_end(&mut out)?;
    Ok((out, dec.into_inner()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(layers: &[Layer]) {
        let content = Content::Json { value: serde_json::json!({"gold": 1500, "name": "Hero", "items": [1, 2.5, null]}), pretty: false };
        let bytes = encode(layers, &content).unwrap();
        let decoded = decode(&bytes);
        assert_eq!(decoded.layers, layers, "layers for {layers:?}");
        assert!(same_data(&decoded.content, &content), "save verification for {layers:?}");
        match decoded.content {
            Content::Json { value, .. } => assert_eq!(value["gold"], 1500),
            other => panic!("expected JSON, got {}", other.kind()),
        }
    }

    #[test]
    fn plain_json() { roundtrip(&[]); }
    #[test]
    fn rpg_maker_mv() { roundtrip(&[Layer::LzString]); }
    #[test]
    fn rpg_maker_mz() { roundtrip(&[Layer::RpgMzZlib]); }
    #[test]
    fn gzip() { roundtrip(&[Layer::Gzip]); }
    #[test]
    fn zlib() { roundtrip(&[Layer::Zlib]); }
    #[test]
    fn base64_gzip() { roundtrip(&[Layer::Base64, Layer::Gzip]); }
    #[test]
    fn fallout_shelter_layer() { roundtrip(&[Layer::FalloutShelter]); }
    #[test]
    fn antimatter_layer() { roundtrip(&[Layer::Antimatter { kind: 0, version: "AAB".into() }]); }
    #[test]
    fn nested_json_layer() {
        let content = Content::Json { value: serde_json::json!({"Version": 2, "State": "{\"Hp\":12,\"Gold\":5}"}), pretty: false };
        let bytes = encode(&[], &content).unwrap();
        let d = decode(&bytes);
        assert!(matches!(d.layers.as_slice(), [Layer::NestedJson { .. }]), "{:?}", d.layers);
        match &d.content {
            Content::Json { value, .. } => assert_eq!(value["State"]["Gold"], 5),
            other => panic!("expected JSON, got {}", other.kind()),
        }
        assert_eq!(encode(&d.layers, &d.content).unwrap(), bytes);
    }
    #[test]
    fn bom() { roundtrip(&[Layer::Utf8Bom]); }

    fn set(content: &mut Content, ptr: &str, v: Value) {
        let Content::Tree(p) = content else { panic!("expected a tree, got {}", content.kind()) };
        let (view, editable) = p.view_and_editable();
        assert!(editable.contains_key(ptr), "{ptr} should be editable");
        *view.pointer_mut(ptr).unwrap_or_else(|| panic!("missing {ptr}")) = v;
    }

    fn get(content: &Content, ptr: &str) -> Value {
        let Content::Tree(p) = content else { panic!() };
        p.view.pointer(ptr).cloned().unwrap_or(Value::Null)
    }

    fn renpy_edit(name: &str) {
        let raw = std::fs::read(format!("tests/fixtures/{name}")).unwrap();
        let mut d = decode(&raw);
        assert!(matches!(d.layers[0], Layer::RenpyZip { .. }));
        let Content::Tree(p) = &d.content else { panic!("not pickle") };
        assert_eq!(p.kind_name, "Ren'Py save");
        assert_eq!(get(&d.content, "/Variables/gold"), 1500);
        assert_eq!(get(&d.content, "/Variables/e/name"), "Eileen");
        assert!(get(&d.content, "/Ren'Py internals/_rollback").is_boolean());

        // Unchanged content must re-encode to an identical log.
        let same = encode(&d.layers, &d.content).unwrap();
        assert_eq!(decode(&same).content.to_bytes().unwrap(), d.content.to_bytes().unwrap());

        set(&mut d.content, "/Variables/gold", Value::from(99999));
        set(&mut d.content, "/Variables/player_name", Value::from("Robin Hood ★"));
        set(&mut d.content, "/Variables/speed", serde_json::json!(2.25));
        set(&mut d.content, "/Variables/flags/met_alice", Value::from(false));
        set(&mut d.content, "/Variables/inventory/money", Value::from(9000));
        set(&mut d.content, "/Variables/inventory/items/1", Value::from("elixir"));
        set(&mut d.content, "/Variables/big", Value::from(5));
        set(&mut d.content, "/Variables/neg", Value::from(-70000));

        let out = encode(&d.layers, &d.content).unwrap();
        std::fs::create_dir_all("target/test-out").unwrap();
        std::fs::write(format!("target/test-out/{name}"), &out).unwrap();

        let again = decode(&out);
        // The exact check the app runs before writing a save.
        assert!(same_layers(&again.layers, &d.layers), "layer kinds must match");
        assert!(same_data(&again.content, &d.content), "re-read data must match what was edited");
        assert_eq!(get(&again.content, "/Variables/gold"), 99999);
        assert_eq!(get(&again.content, "/Variables/player_name"), "Robin Hood ★");
        assert_eq!(get(&again.content, "/Variables/inventory/items/1"), "elixir");
        assert_eq!(get(&again.content, "/Variables/neg"), -70000);
    }

    #[test]
    fn renpy_protocol2() { renpy_edit("renpy_p2-1-LT1.save"); }
    #[test]
    fn renpy_protocol5_signed() { renpy_edit("renpy_p5-1-LT1.save"); }

    #[test]
    fn renpy_persistent() {
        let raw = std::fs::read("tests/fixtures/persistent").unwrap();
        let mut d = decode(&raw);
        assert_eq!(d.layers, vec![Layer::Zlib]);
        assert_eq!(get(&d.content, "/Variables/gallery_unlocked"), 3);
        set(&mut d.content, "/Variables/gallery_unlocked", Value::from(10));
        set(&mut d.content, "/Variables/ending_good", Value::from(true));
        let out = encode(&d.layers, &d.content).unwrap();
        std::fs::create_dir_all("target/test-out").unwrap();
        std::fs::write("target/test-out/persistent", &out).unwrap();
        assert_eq!(get(&decode(&out).content, "/Variables/gallery_unlocked"), 10);
    }

    #[test]
    fn renpy_shared_values_stay_in_sync() {
        // One string and one list each referenced by two variables (pickle memo).
        let raw = std::fs::read("tests/fixtures/renpy_shared-1-LT1.save").unwrap();
        let mut d = decode(&raw);
        let Content::Tree(p) = &mut d.content else { panic!("not pickle") };
        assert_eq!(p.view.pointer("/Variables/hero_label").unwrap(), "Robin the Brave");

        let (_, aliases) = p.set("/Variables/player_name", Value::from("Robin")).unwrap().unwrap();
        assert_eq!(aliases, 1, "the shared name should be reported as shared");
        assert_eq!(p.view.pointer("/Variables/hero_label").unwrap(), "Robin");
        p.set("/Variables/inventory/0", Value::from("axe")).unwrap().unwrap();
        assert_eq!(p.view.pointer("/Variables/backpack/0").unwrap(), "axe");
        let (_, none) = p.set("/Variables/gold", Value::from(99)).unwrap().unwrap();
        assert_eq!(none, 0);

        // The save verification must pass — before the fix it aborted here.
        let out = encode(&d.layers, &d.content).unwrap();
        std::fs::create_dir_all("target/test-out").unwrap();
        std::fs::write("target/test-out/shared.save", &out).unwrap();
        let again = decode(&out);
        assert!(same_layers(&again.layers, &d.layers));
        assert!(same_data(&again.content, &d.content), "screen and file must agree");
    }

    /// Point the signer at a throwaway key (never the real one) for tests.
    fn use_test_key() -> std::path::PathBuf {
        std::fs::create_dir_all("target/test-out").unwrap();
        let p = std::path::PathBuf::from("target/test-out/test_security_keys.txt");
        if !p.exists() {
            crate::renpy_sign::write_test_keys(&p);
        }
        std::env::set_var("SAVE_EDITOR_RENPY_KEYS", &p);
        p
    }

    #[test]
    fn edited_signed_save_is_resigned() {
        use_test_key();
        let raw = std::fs::read("tests/fixtures/renpy_p5-1-LT1.save").unwrap();
        let mut d = decode(&raw);
        assert!(matches!(d.layers[0], Layer::RenpyZip { signed: true, .. }));
        set(&mut d.content, "/Variables/gold", Value::from(4242));
        let out = encode(&d.layers, &d.content).unwrap();
        std::fs::write("target/test-out/resigned.save", &out).unwrap();

        // The new signatures entry must be valid for the *edited* log.
        let mut zip = zip::ZipArchive::new(Cursor::new(&out)).unwrap();
        let mut log = Vec::new();
        zip.by_name("log").unwrap().read_to_end(&mut log).unwrap();
        let mut sigs = String::new();
        zip.by_name("signatures").unwrap().read_to_string(&mut sigs).unwrap();
        assert!(crate::renpy_sign::verify(&log, &sigs), "edited log must carry a valid signature");
        let names: Vec<_> = zip.file_names().map(String::from).collect();
        assert!(names.contains(&"screenshot.png".to_string()) && names.contains(&"json".to_string()));
    }

    #[test]
    fn edits_survive_renpy_load_rollback() {
        // Ren'Py rolls back to the last checkpoint on load, restoring old values from
        // the rollback log; those must carry the edit too or it's silently undone.
        let raw = std::fs::read("tests/fixtures/renpy_rollback-1-LT1.save").unwrap();
        let mut d = decode(&raw);
        let Content::Tree(p) = &d.content else { panic!() };
        assert_eq!(p.linked_count("/Variables/nScore"), 3, "three history entries hold nScore");
        assert_eq!(p.linked_count("/Variables/nPatience"), 0);
        set(&mut d.content, "/Variables/nScore", Value::from(25));
        set(&mut d.content, "/Variables/nPatience", Value::from(7));
        let out = encode(&d.layers, &d.content).unwrap();
        std::fs::create_dir_all("target/test-out").unwrap();
        std::fs::write("target/test-out/rollback.save", &out).unwrap();
        let again = decode(&out);
        assert!(same_data(&again.content, &d.content));
    }

    #[test]
    fn renpy_persistent_with_signature() {
        use_test_key();
        // Ren'Py 8.1+ persistent: zlib(pickle) followed by signature lines.
        let pickle_bytes = {
            let plain = std::fs::read("tests/fixtures/persistent").unwrap();
            inflate(&plain).unwrap()
        };
        let mut raw = {
            let mut e = ZlibEncoder::new(Vec::new(), Compression::new(3));
            e.write_all(&pickle_bytes).unwrap();
            e.finish().unwrap()
        };
        raw.extend_from_slice(crate::renpy_sign::sign(&pickle_bytes).unwrap().as_bytes());

        let mut d = decode(&raw);
        assert_eq!(d.layers, vec![Layer::RenpyPersistent { signed: true }]);
        set(&mut d.content, "/Variables/gallery_unlocked", Value::from(42));
        let out = encode(&d.layers, &d.content).unwrap();

        let (new_pickle, sig) = inflate_with_rest(&out).unwrap();
        assert!(crate::renpy_sign::verify(&new_pickle, std::str::from_utf8(sig).unwrap()));
        assert_eq!(get(&decode(&out).content, "/Variables/gallery_unlocked"), 42);
    }

    /// Manual check against a sample Ren'Py save and the signing key Ren'Py keeps (read-only):
    ///   set SAVE_EDITOR_REAL_SAVE=<path>; cargo test --release real_renpy_save -- --ignored
    /// Writes the edited copy to target/test-out/real_edit.save; the original is never touched.
    #[test]
    #[ignore]
    fn real_renpy_save() {
        let src = std::env::var("SAVE_EDITOR_REAL_SAVE").expect("set SAVE_EDITOR_REAL_SAVE");
        let raw = std::fs::read(&src).unwrap();
        let mut d = decode(&raw);
        assert!(matches!(d.layers[0], Layer::RenpyZip { signed: true, .. }), "expected a signed Ren'Py save");
        // SAVE_EDITOR_REAL_VARS = "name=value,name=value" (variables of that game to change).
        let vars = std::env::var("SAVE_EDITOR_REAL_VARS").expect("set SAVE_EDITOR_REAL_VARS");
        for (var, v) in vars.split(',').filter_map(|p| p.split_once('=')).map(|(k, v)| (k.trim().to_string(), v.trim().parse::<i64>().unwrap())) {
            let ptr = format!("/Variables/{var}");
            if let Content::Tree(p) = &d.content {
                eprintln!("{var}: {} -> {v}  (history entries kept in step: {})", get(&d.content, &ptr), p.linked_count(&ptr));
            }
            set(&mut d.content, &ptr, Value::from(v));
        }
        let out = encode(&d.layers, &d.content).unwrap();
        let again = decode(&out);
        assert!(same_layers(&again.layers, &d.layers) && same_data(&again.content, &d.content));
        std::fs::create_dir_all("target/test-out").unwrap();
        std::fs::write("target/test-out/real_edit.save", &out).unwrap();
    }

    /// Manual harness for checking formats against external tools:
    ///   SAVE_EDITOR_TEST_IN=<file> SAVE_EDITOR_TEST_EDITS="<ptr>=<json>;…" SAVE_EDITOR_TEST_OUT=<file>
    ///   [SAVE_EDITOR_TEST_PASSWORD=…] cargo test --release cli_edit -- --ignored --nocapture
    #[test]
    #[ignore]
    fn cli_edit() {
        let input = std::env::var("SAVE_EDITOR_TEST_IN").expect("SAVE_EDITOR_TEST_IN");
        let raw = std::fs::read(&input).unwrap();
        let mut opts = Options::for_path(std::path::Path::new(&input));
        opts.password = std::env::var("SAVE_EDITOR_TEST_PASSWORD").ok();
        let mut d = decode_with(&raw, &opts);
        eprintln!("detected: {}", describe(&d.layers, &d.content));
        if let Some(h) = &d.hint {
            eprintln!("hint: {h}");
        }
        for edit in std::env::var("SAVE_EDITOR_TEST_EDITS").unwrap_or_default().split(';').filter(|s| !s.is_empty()) {
            let (ptr, json) = edit.split_once('=').expect("ptr=json");
            let v: Value = serde_json::from_str(json).expect("json value");
            match &mut d.content {
                Content::Tree(t) => {
                    eprintln!("{ptr}: {:?} -> {v}", t.view.pointer(ptr));
                    t.set(ptr, v).expect("valid value").expect("pointer exists");
                }
                Content::Json { value, .. } => {
                    eprintln!("{ptr}: {:?} -> {v}", value.pointer(ptr));
                    *value.pointer_mut(ptr).expect("pointer exists") = v;
                }
                other => panic!("not editable: {}", other.kind()),
            }
        }
        let out = encode(&d.layers, &d.content).unwrap();
        let check = decode_with(&out, &opts);
        assert!(same_layers(&check.layers, &d.layers) && same_data(&check.content, &d.content), "verification failed");
        if let Ok(p) = std::env::var("SAVE_EDITOR_TEST_OUT") {
            std::fs::write(p, &out).unwrap();
        }
    }

    /// Read-only survey of sample saves: SAVE_EDITOR_SCAN = file listing one path per line.
    /// For each file: detect, re-encode unchanged, and simulate one edit + the save
    /// verification — entirely in memory, nothing is written.
    #[test]
    #[ignore]
    fn scan_real_saves() {
        let list = std::fs::read_to_string(std::env::var("SAVE_EDITOR_SCAN").expect("SAVE_EDITOR_SCAN")).unwrap();
        let mut kinds: std::collections::BTreeMap<String, usize> = Default::default();
        for path in list.lines().map(str::trim).filter(|l| !l.is_empty()) {
            let Ok(raw) = std::fs::read(path) else { continue };
            let opts = Options::for_path(std::path::Path::new(path));
            let mut d = decode_with(&raw, &opts);
            let kind = describe(&d.layers, &d.content);
            let mut result = String::new();
            if let Content::Tree(t) = &mut d.content {
                // Unchanged save must be accepted by the verification.
                let same = encode(&d.layers, &Content::Tree(t.clone())).map(|b| {
                    let c = decode_with(&b, &opts);
                    same_layers(&c.layers, &d.layers) && same_data(&c.content, &Content::Tree(t.clone()))
                });
                result.push_str(&format!("unchanged-ok={} ", same.unwrap_or(false)));
                // Simulated edit: first editable number +1 (or first text).
                let target = t.editable.keys().filter_map(|p| t.view.pointer(p).map(|v| (p.clone(), v.clone()))).find(|(_, v)| v.is_i64());
                let leaves = t.editable.len();
                let edited = target.map(|(p, v)| t.set(&p, Value::from(v.as_i64().unwrap().saturating_add(1))));
                match edited {
                    Some(Ok(_)) => {
                        let reread = encode(&d.layers, &d.content).map(|b| decode_with(&b, &opts));
                        let ok = reread.as_ref().is_ok_and(|c| same_layers(&c.layers, &d.layers) && same_data(&c.content, &d.content));
                        result.push_str(&format!("edit-ok={ok} ({leaves} leaves)"));
                        if let Err(e) = &reread {
                            result.push_str(&format!(" ERROR {e}"));
                        }
                        if let (false, Ok(Decoded { content: Content::Tree(c), .. }), Content::Tree(t)) = (ok, &reread, &d.content) {
                            let first_diff = t.editable.keys().find(|p| t.view.pointer(p) != c.view.pointer(p));
                            if let Some(p) = first_diff {
                                result.push_str(&format!(" DIFF {p}: {:?} vs reread {:?}", t.view.pointer(p), c.view.pointer(p)));
                            }
                        }
                    }
                    Some(Err(e)) => result.push_str(&format!("edit-refused: {e}")),
                    None => {}
                }
            }
            *kinds.entry(kind.clone()).or_default() += 1;
            eprintln!("{kind:<45} {result:<40} {path}");
        }
        eprintln!("\nSUMMARY:");
        for (k, n) in kinds {
            eprintln!("  {n:>4}  {k}");
        }
    }

    #[test]
    fn js_escape_matches_javascript() {
        // Expected values from JavaScript's escape().
        assert_eq!(js_escape("A-z_0.9*@+/"), "A-z_0.9*@+/");
        assert_eq!(js_escape("é ü{}\""), "%E9%20%FC%7B%7D%22");
        assert_eq!(js_escape("日本"), "%u65E5%u672C");
        assert_eq!(js_escape("😀"), "%uD83D%uDE00");
        assert_eq!(js_unescape("%u65E5%u672C%20x").unwrap(), "日本 x");
        let json = r#"{"name":"Ren","gold":10}"#;
        let raw = js_escape(json);
        let d = decode(raw.as_bytes());
        assert_eq!(d.layers, vec![Layer::JsEscape]);
        assert_eq!(encode(&d.layers, &d.content).unwrap(), raw.as_bytes());
    }

    #[test]
    fn game_specific_wrappers() {
        // Slay the Spire: base64(XOR "key"(JSON)).
        let json = br#"{"gold":99,"name":"IRONCLAD"}"#;
        let sts = STANDARD.encode(super::xor_key(json, b"key"));
        let d = decode(sts.as_bytes());
        assert_eq!(d.layers, vec![Layer::Base64, Layer::XorKey("key".into())]);
        assert_eq!(get_json(&d.content, "/gold"), 99);
        assert_eq!(encode(&d.layers, &d.content).unwrap(), sts.as_bytes());
        // Hollow Knight: .NET string of base64 AES-256-ECB JSON.
        let hk = fmt::crypto::hollow_knight_encrypt(br#"{"playerData":{"geo":1200}}"#);
        let d = decode(&hk);
        assert_eq!(d.layers, vec![Layer::HollowKnight]);
        assert_eq!(get_json(&d.content, "/playerData/geo"), 1200);
        assert_eq!(encode(&d.layers, &d.content).unwrap(), hk);
        // Balatro: raw deflate around a Lua table.
        let lua = b"return {[\"dollars\"]=4,[\"name\"]=\"Red Deck\",}";
        let jkr = compress(flate2::write::DeflateEncoder::new(Vec::new(), Compression::default()), lua).unwrap().finish().unwrap();
        let mut opts = Options::default();
        opts.ext = "jkr".into();
        let d = decode_with(&jkr, &opts);
        assert_eq!(d.layers, vec![Layer::Deflate]);
        assert!(matches!(&d.content, Content::Tree(t) if t.view["dollars"] == 4));
    }

    #[test]
    fn evas_envelope_round_trip() {
        let body = b"GVAS\x03\x00\x00\x00 rest of the save".to_vec();
        let raw = wrap(&Layer::Evas, body.clone()).unwrap();
        assert_eq!(&raw[..8], b"EVAS\x01\x00\x00\x00");
        let opts = Options::default();
        let mut ctx = Ctx { opts: &opts, zip_entries: vec![], hint: None };
        let (layer, inner) = peel(&raw, &mut ctx).unwrap();
        assert_eq!(layer, Layer::Evas);
        assert_eq!(inner, body);
        let mut bad = raw.clone();
        bad[12] ^= 1;
        assert!(!matches!(peel(&bad, &mut ctx), Some((Layer::Evas, _))));
    }

    #[test]
    fn new_wrappers_round_trip() {
        let json = br#"{"gold":1500,"name":"Ash"}"#.to_vec();
        for layers in [
            vec![Layer::Zstd],
            vec![Layer::Lz4],
            vec![Layer::Snappy],
            vec![Layer::Xz],
            vec![Layer::Bzip2],
            vec![Layer::Utf16 { big_endian: false }],
            vec![Layer::Utf16 { big_endian: true }],
            vec![Layer::Xor(0x5A)],
            vec![Layer::Base64Lines { width: 16, crlf: true }],
            vec![Layer::Es3 { password: "password".into(), iv: vec![3; 16] }],
            vec![Layer::Es3 { password: "password".into(), iv: vec![3; 16] }, Layer::Gzip],
        ] {
            let content = Content::Json { value: serde_json::from_slice(&json).unwrap(), pretty: false };
            let bytes = encode(&layers, &content).unwrap();
            let d = decode(&bytes);
            assert!(same_layers(&d.layers, &layers), "{layers:?} detected as {:?}", d.layers);
            assert!(same_data(&d.content, &content), "{layers:?}");
        }
    }

    #[test]
    fn barotrauma_bundle_and_palworld() {
        // Bundle: a gzipped "submarine" plus an XML game session, inside gzip.
        let files = Arc::new(vec![
            ("Sub.sub".to_string(), gz(b"<Submarine/>")),
            ("gamesession.xml".to_string(), b"<Gamesession money=\"100\"/>".to_vec()),
        ]);
        let layers = vec![Layer::Gzip, Layer::FileBundle { files: files.clone(), name: "gamesession.xml".into() }];
        let raw = encode(&layers, &Content::Text("<Gamesession money=\"100\"/>".into())).unwrap();
        let mut d = decode(&raw);
        assert!(same_layers(&d.layers, &layers), "{:?}", d.layers);
        assert_eq!(d.zip_entries, vec!["Sub.sub".to_string(), "gamesession.xml".to_string()]);
        let Content::Tree(t) = &mut d.content else { panic!("expected XML, got {}", d.content.kind()) };
        t.set("/Gamesession/@money", Value::from(99999)).unwrap();
        let again = decode(&encode(&d.layers, &d.content).unwrap());
        let Content::Tree(t2) = &again.content else { panic!() };
        assert_eq!(t2.view.pointer("/Gamesession/@money").unwrap(), 99999);
        // The other file in the bundle is untouched.
        let Layer::FileBundle { files: f2, .. } = &again.layers[1] else { panic!() };
        assert_eq!(f2[0], files[0]);

        // Palworld PlZ (single and double zlib) around arbitrary data.
        for double in [false, true] {
            let content = Content::Binary(b"GVAS-ish payload bytes \x00\x01\x02".to_vec());
            let raw = encode(&[Layer::Palworld { double }], &content).unwrap();
            let d = decode(&raw);
            assert_eq!(d.layers, vec![Layer::Palworld { double }]);
            assert!(same_data(&d.content, &content));
        }
    }

    #[test]
    fn godot_encrypted_needs_password() {
        let info = crate::fmt::crypto::GodotEnc { password: "hunter2".into(), iv: Some(vec![1; 16]), mode: 1 };
        let bytes = encode(&[Layer::GodotEnc(info)], &Content::Json { value: serde_json::json!({"hp": 5}), pretty: false }).unwrap();
        let locked = decode(&bytes);
        assert!(matches!(locked.content, Content::Binary(_)));
        assert!(locked.hint.unwrap().contains("password"));
        let opts = Options { password: Some("hunter2".into()), ..Default::default() };
        let open = decode_with(&bytes, &opts);
        assert!(matches!(open.layers[0], Layer::GodotEnc(_)));
        assert_eq!(get_json(&open.content, "/hp"), 5);
    }

    fn get_json(c: &Content, ptr: &str) -> Value {
        match c {
            Content::Json { value, .. } => value.pointer(ptr).cloned().unwrap_or(Value::Null),
            _ => Value::Null,
        }
    }

    /// A Paradox `SAV…` archive: binary metadata after the header line, then a zip whose
    /// `gamestate` starts with the same metadata. An edit inside the metadata reaches both.
    #[test]
    fn pdx_sav_archive_round_trip() {
        use zip::write::SimpleFileOptions;
        let id = |n: u16| n.to_le_bytes().to_vec();
        let text = |t: &str| [id(0x000f), (t.len() as u16).to_le_bytes().to_vec(), t.as_bytes().to_vec()].concat();
        let meta = [id(0x3155), id(1), id(3), id(0x058f), id(1), id(0x000c), 9i32.to_le_bytes().to_vec(), id(0x001c), id(1), text("Ada"), id(4)].concat();
        let state = [meta.clone(), id(0x7000), id(1), id(0x0167), 250_000i64.to_le_bytes().to_vec()].concat();
        let mut file = format!("SAV0103aabbccdd{:08x}\n", meta.len()).into_bytes();
        file.extend(&meta);
        let zip_at = file.len();
        let mut start = Cursor::new(file);
        start.set_position(zip_at as u64);
        let mut w = zip::ZipWriter::new(start);
        w.start_file("gamestate", SimpleFileOptions::default()).unwrap();
        w.write_all(&state).unwrap();
        let raw = w.finish().unwrap().into_inner();

        let d = decode_with(&raw, &Options { ext: "ck3".into(), ..Options::default() });
        assert!(matches!(&d.layers[0], Layer::PdxSav { zip_at: z, .. } if *z == zip_at));
        let Content::Tree(mut doc) = d.content else { panic!("not a tree") };
        assert_eq!(doc.kind_name, "Crusader Kings III save (binary)");
        assert_eq!(doc.view["0x7000"], 2.5);
        assert_eq!(encode(&d.layers, &Content::Tree(doc.clone())).map(|b| decode(&b).content.to_bytes().unwrap()).unwrap(), state);
        doc.set("/0x7000", serde_json::json!(7.25)).unwrap();
        doc.set("/0x3155/0x001c", serde_json::json!("Adelaide")).unwrap();
        let out = encode(&d.layers, &Content::Tree(doc)).unwrap();
        let h = fmt::pdxbin::SavHeader::read(&out).unwrap();
        assert_eq!(h.meta_len, meta.len() + 5);
        let back = decode_with(&out, &Options { ext: "ck3".into(), ..Options::default() });
        let Content::Tree(doc) = back.content else { panic!("not a tree") };
        assert_eq!(doc.view["0x7000"], 7.25);
        assert_eq!(doc.view["0x3155"]["0x001c"], "Adelaide");
        // The copy after the header line is the start of the new game state.
        let state = doc.to_bytes().unwrap();
        assert_eq!(&out[h.len..h.len + h.meta_len], &state[..h.meta_len]);
        assert_eq!(&out[h.len + h.meta_len..h.len + h.meta_len + 4], b"PK\x03\x04");
    }

    #[test]
    fn generic_zip_picks_editable_entry() {
        use zip::write::SimpleFileOptions;
        let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
        w.start_file("thumb.png", SimpleFileOptions::default()).unwrap();
        w.write_all(&[0x89, b'P', b'N', b'G', 0, 1, 2, 3, 250, 251]).unwrap();
        w.start_file("save.json", SimpleFileOptions::default()).unwrap();
        w.write_all(br#"{"gold":5}"#).unwrap();
        let raw = w.finish().unwrap().into_inner();
        let mut d = decode(&raw);
        assert!(matches!(&d.layers[0], Layer::ZipEntry { name, .. } if name == "save.json"));
        assert_eq!(d.zip_entries.len(), 2);
        if let Content::Json { value, .. } = &mut d.content {
            value["gold"] = Value::from(77);
        }
        let out = encode(&d.layers, &d.content).unwrap();
        let again = decode(&out);
        assert_eq!(get_json(&again.content, "/gold"), 77);
        let mut z = zip::ZipArchive::new(Cursor::new(out)).unwrap();
        let mut png = Vec::new();
        z.by_name("thumb.png").unwrap().read_to_end(&mut png).unwrap();
        assert_eq!(png[..4], [0x89, b'P', b'N', b'G'], "other entries kept");
    }

    fn gz(data: &[u8]) -> Vec<u8> {
        let mut e = GzEncoder::new(Vec::new(), Compression::default());
        e.write_all(data).unwrap();
        e.finish().unwrap()
    }

    #[test]
    fn gzip_with_trailing_data_is_not_unwrapped() {
        // Unwrapping would silently drop the trailing checksum on re-save.
        let mut raw = gz(br#"{"gold":1}"#);
        raw.extend_from_slice(b"CHECKSUM");
        let d = decode(&raw);
        assert!(d.layers.is_empty(), "got layers {:?}", d.layers);
        assert!(matches!(d.content, Content::Binary(ref b) if *b == raw));
    }

    #[test]
    fn multi_member_gzip_is_not_unwrapped() {
        let mut raw = gz(br#"{"a":1}"#);
        raw.extend(gz(br#"{"b":2}"#));
        let d = decode(&raw);
        assert!(d.layers.is_empty());
        assert!(matches!(d.content, Content::Binary(_)));
    }

    #[test]
    fn zlib_with_trailing_data_is_not_unwrapped() {
        let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
        e.write_all(br#"{"gold":1}"#).unwrap();
        let mut raw = e.finish().unwrap();
        raw.extend_from_slice(&[0, 0, 0, 0]);
        assert!(decode(&raw).layers.is_empty());
    }

    #[test]
    fn verification_accepts_text_that_became_json() {
        // Editing a text file into valid JSON changes how it's detected; the bytes still match.
        let edited = Content::Text("{\"a\":1}".into());
        let reread = decode(&encode(&[], &edited).unwrap());
        assert!(matches!(reread.content, Content::Json { .. }));
        assert!(same_data(&reread.content, &edited));
    }

    #[test]
    fn binary_stays_binary() {
        let d = decode(&[0u8, 1, 2, 255, 0, 7]);
        assert!(d.layers.is_empty());
        assert!(matches!(d.content, Content::Binary(_)));
    }

    #[test]
    fn text_stays_text() {
        let d = decode(b"[Player]\ngold=100\n");
        assert!(matches!(d.content, Content::Text(_)));
    }
}

/// Other files a game checks against a save, rewritten to match `bytes` once the save has
/// been written: Persona 4 Golden keeps the save's MD5 in `dataNNNN.binslot` (0x18–0x27).
pub fn companion_files(path: &std::path::Path, bytes: &[u8]) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    let mut out = vec![];
    if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("bin")) {
        let found = ["binslot", "BINslot", "BINSLOT"].iter().map(|e| path.with_extension(e)).find(|p| p.exists());
        let slot = found.unwrap_or_else(|| path.with_extension("binslot"));
        if let Ok(mut s) = std::fs::read(&slot) {
            if s.len() >= 0x28 && s.starts_with(b"SAVE") {
                use md5::Digest;
                let sum = md5::Md5::digest(bytes);
                if s[0x18..0x28] != sum[..] {
                    s[0x18..0x28].copy_from_slice(&sum);
                    out.push((slot, s));
                }
            }
        }
    }
    // Neverwinter Nights: player.bic beside a module save holds the same player.
    out.extend(fmt::nwn::companions(path, bytes));
    out
}


#[cfg(test)]
mod copeck_tests {
    #[test]
    fn a_grown_file_moves_after_the_last() {
        let mut pack = vec![0u8; 0x60000];
        pack[..6].copy_from_slice(b"COPECK");
        let mut entry = |p: usize, at: u32, size: u32, name: &str, data: &[u8]| {
            pack[p..p + 4].copy_from_slice(&at.to_le_bytes());
            pack[p + 4..p + 8].copy_from_slice(&size.to_le_bytes());
            pack[p + 0x15..p + 0x15 + name.len()].copy_from_slice(name.as_bytes());
            pack[at as usize..at as usize + data.len()].copy_from_slice(data);
        };
        entry(0x10, 0x50000, 4, "A.SAV", b"abcd");
        entry(0xB0, 0x50010, 3, "B.BIN", b"xyz");
        let files = super::copeck_entries(&pack).unwrap();
        assert_eq!(files.len(), 2);
        let out = super::copeck_put(&pack, 0x10, b"longer data").unwrap();
        let again = super::copeck_entries(&out).unwrap();
        assert_eq!((again[0].2, again[0].3), (0x50020, 11));
        assert_eq!(&out[0x50020..0x5002B], b"longer data");
        assert_eq!(&out[0x50010..0x50013], b"xyz");
        let small = super::copeck_put(&pack, 0x10, b"ab").unwrap();
        assert_eq!(super::copeck_entries(&small).unwrap()[0].3, 2);
    }
}

#[cfg(test)]
mod civ5_tests {
    use super::*;

    /// A synthetic save whose game data packs to more than one 64 KiB chunk.
    #[test]
    fn chunked_game_data_round_trips() {
        let mut plain = Vec::new();
        let mut x = 12345u32;
        for i in 0..400_000u32 {
            x = x.wrapping_mul(1_103_515_245).wrapping_add(12345);
            plain.push(if i % 3 == 0 { (x >> 24) as u8 } else { (i % 7) as u8 });
        }
        let mut header = b"CIV5\x08\x00\x00\x00".to_vec();
        header.extend(b"\x0B\x00\x00\x00HANDICAP_KING".iter());
        let layer = Layer::Civ5 { header: Arc::new(header.clone()), chunked: true, trailer: Arc::new(b"\x01\x00\x00\x00tail".to_vec()) };
        let file = wrap(&layer, plain.clone()).unwrap();
        assert!(file.len() > header.len() + 2 * CIV5_CHUNK, "needs several chunks");
        let opts = Options::default();
        let mut ctx = Ctx { opts: &opts, zip_entries: vec![], hint: None };
        let (back, data) = peel_civ5(&file, &mut ctx).unwrap();
        assert_eq!(data, plain);
        let Layer::Civ5 { header: h, chunked, trailer } = &back else { panic!() };
        assert_eq!(h.as_slice(), header.as_slice());
        assert!(*chunked);
        assert_eq!(trailer.as_slice(), b"\x01\x00\x00\x00tail");
        assert_eq!(wrap(&back, data).unwrap(), file);
    }

    #[test]
    fn cloverpit_key_is_the_derived_password() {
        let p = b"uoiyiuh_+=-5216gh;lj??!/345";
        let k: Vec<u8> = (0..p.len()).map(|i| p[(i + p[i] as usize % p.len()) % p.len()] ^ p[i]).collect();
        assert!(super::KNOWN_XOR_KEYS.iter().any(|(g, key)| *g == "CloverPit" && key.as_bytes() == k));
    }

}

#[cfg(test)]
mod gzip_name_tests {
    use super::*;

    #[test]
    fn ocs_gzip_json_opens_by_name() {
        use std::io::Write;
        let mut z = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        z.write_all(br#"{"Name":"Test","Level":5}"#).unwrap();
        let gz = z.finish().unwrap();
        let d = decode_with(&gz, &Options::for_path(std::path::Path::new("player.ocs")));
        assert!(matches!(d.content, Content::Json { .. }), "{}", describe(&d.layers, &d.content));
    }
}
