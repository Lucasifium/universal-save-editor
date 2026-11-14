//! Adding items to a Divinity: Original Sin character's inventory.
//!
//! An item in `globals.lsf` is not one node but a handful of records that must agree:
//!
//! - an `Items/ItemFactory/Items/Item` node (template, `Stats` name, `Parent` = the holder's
//!   inventory handle, `Slot` = place in that inventory, optional `Amount`), with the same
//!   position in the list as its `Items/ItemFactory/Creators/Creator` (the item's handle,
//!   uuid and template);
//! - an `Inventories/.../Inventory/TimeItemAddedToInventory` entry (key = the item's handle)
//!   in the holder's inventory;
//! - an `EntityWorld/Entity/Component` entry carrying the item's handle.
//!
//! A new item is a copy of an item of the same kind already in the save (so the template,
//! generated stats and so on are ones the game itself wrote), with a new handle and uuid, put
//! into the target inventory's first free bag slot, and entered in all of the above. Copies
//! are only made of plain items (no containers, keys, quest variables or AI plans); where the
//! target already carries a stack of the item with an `Amount`, that stack grows instead.
//! Anything unexpected in the save's layout makes the whole edit refuse.

use super::lsf::{Lsf, NewAttr, Writer};
use std::collections::{HashMap, HashSet};

const ITEMS: [&str; 4] = ["Items", "ItemFactory", "Items", "Item"];
const ITEM_CREATORS: [&str; 4] = ["Items", "ItemFactory", "Creators", "Creator"];
const INVS: [&str; 4] = [
    "Inventories",
    "InventoryFactory",
    "Inventories",
    "Inventory",
];
const INV_CREATORS: [&str; 4] = ["Inventories", "InventoryFactory", "Creators", "Creator"];
const VIEWS: [&str; 4] = [
    "InventoryViews",
    "InventoryViewFactory",
    "InventoryViews",
    "InventoryView",
];

/// Amounts offered for stackable items.
pub const AMOUNTS: [u32; 6] = [1, 5, 10, 20, 50, 99];

/// Stats-name prefixes of things that stack even when the save holds only a single one.
const STACKING: [&str; 4] = ["CON_", "ARR_", "SCR_", "GRN_"];

struct World<'a> {
    l: &'a Lsf<'a>,
    items: Vec<usize>,
    creators: Vec<usize>,
    invs: Vec<usize>,
    inv_creators: Vec<usize>,
}

fn world<'a>(l: &'a Lsf<'a>) -> Option<World<'a>> {
    let w = World {
        l,
        items: l.at_path(&ITEMS),
        creators: l.at_path(&ITEM_CREATORS),
        invs: l.at_path(&INVS),
        inv_creators: l.at_path(&INV_CREATORS),
    };
    (!w.items.is_empty()
        && w.items.len() == w.creators.len()
        && w.invs.len() == w.inv_creators.len())
    .then_some(w)
}

pub(super) fn bytes<'a>(l: &'a Lsf, n: usize, name: &str) -> Option<&'a [u8]> {
    let a = l.attr(n, name)?;
    l.d.get(a.at..a.at + a.len)
}

fn subtree_ok(l: &Lsf, n: usize, depth: usize) -> bool {
    // ItemMachine, StatusManager and VariableManager only when empty (anything inside is
    // game state); the generated-stats subtrees are copied as they are.
    l.children(n).all(|c| match l.node_name(c) {
        "ItemMachine" | "StatusManager" | "VariableManager" => {
            depth == 0 && l.children(c).next().is_none()
        }
        "Stats" | "Generation" | "Sockets" => true,
        _ => false,
    })
}

/// Plain items that can be copied: not containers, keys or story items, in somebody's inventory.
fn copyable(l: &Lsf, i: usize) -> bool {
    l.string_of(i, "Stats").is_some_and(|s| !s.is_empty())
        && l.int_of(i, "Parent").is_some_and(|p| p != 0)
        && l.int_of(i, "Inventory").unwrap_or(0) == 0
        && l.int_of(i, "IsKey").unwrap_or(0) == 0
        && l.string_of(i, "Key").is_none_or(|k| k.is_empty())
        && l.int_of(i, "TeleportTargetOverride").unwrap_or(0) == 0
        && l.int_of(i, "Slot").is_some()
        && subtree_ok(l, i, 0)
}

/// One kind of item that can be added.
#[derive(Clone, Debug, PartialEq)]
pub struct Offer {
    pub stats: String,
    pub stackable: bool,
}

/// Stats names that stack: seen in a stack of more than one, or consumables, ammunition and coins.
fn stacking(l: &Lsf, items: &[usize]) -> HashSet<String> {
    let mut out = HashSet::new();
    for &i in items {
        let Some(s) = l.string_of(i, "Stats").filter(|s| !s.is_empty()) else {
            continue;
        };
        if l.int_of(i, "Amount").unwrap_or(0) > 1
            || STACKING.iter().any(|p| s.starts_with(p))
            || super::larian::is_gold(&s)
        {
            out.insert(s);
        }
    }
    out
}

/// The items this save can add: one entry per `Stats` name (sorted), from the items in
/// inventories. Names starting with `_` (creatures' natural weapons, characters' own gear)
/// and the inventory's sorting entries (`OC_`) are left out.
pub fn offers(plain: &[u8]) -> Vec<Offer> {
    let Some(l) = Lsf::read(plain) else {
        return vec![];
    };
    let Some(w) = world(&l) else { return vec![] };
    if !layout_ok(&w) {
        return vec![];
    }
    let stack = stacking(&l, &w.items);
    let mut ok: HashSet<String> = HashSet::new();
    for &i in &w.items {
        let Some(s) = l
            .string_of(i, "Stats")
            .filter(|s| !s.starts_with('_') && !s.starts_with("OC_"))
        else {
            continue;
        };
        if copyable(&l, i) {
            ok.insert(s);
        }
    }
    let mut v: Vec<Offer> = ok
        .into_iter()
        .map(|s| Offer {
            stackable: stack.contains(&s),
            stats: s,
        })
        .collect();
    v.sort_by(|a, b| {
        a.stats
            .to_lowercase()
            .cmp(&b.stats.to_lowercase())
            .then(a.stats.cmp(&b.stats))
    });
    v
}

/// Do the pieces an added item has to be entered in exist for the items already there?
fn layout_ok(w: &World) -> bool {
    let l = w.l;
    let Some(&root) = l.at_path(&["EntityWorld"]).first() else {
        return false;
    };
    let Some(ent) = l.children_named(root, "Entity").next() else {
        return false;
    };
    if l.child(ent, "Component").is_none() {
        return false;
    }
    // Every handle is unique, and every item has its entity.
    let comp: HashSet<i64> = l
        .children_named(root, "Entity")
        .flat_map(|e| {
            l.children_named(e, "Component")
                .filter_map(|c| l.int_of(c, "Handle"))
                .collect::<Vec<_>>()
        })
        .collect();
    let mut seen = HashSet::new();
    w.creators.iter().all(|&c| {
        l.int_of(c, "Handle")
            .is_some_and(|h| seen.insert(h) && comp.contains(&h))
            && l.attr(c, "UUID")
                .is_some_and(|a| a.len == 16 || (a.ty == 22 && a.len == 37))
    })
}

fn put_i64(v: i64) -> Vec<u8> {
    v.to_le_bytes().to_vec()
}

pub(super) fn random_guid(taken: &HashSet<[u8; 16]>) -> [u8; 16] {
    use std::hash::{BuildHasher, Hasher};
    let state = std::collections::hash_map::RandomState::new();
    let mut n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64);
    loop {
        let mut g = [0u8; 16];
        for half in g.chunks_mut(8) {
            let mut h = state.build_hasher();
            h.write_u64(n);
            n = n.wrapping_add(0x9E37_79B9_7F4A_7C15);
            half.copy_from_slice(&h.finish().to_le_bytes());
        }
        if !taken.contains(&g) {
            return g;
        }
    }
}

/// Copies `src` (its attributes, with `patch` applied to named ones, and its subtree) under `parent`.
pub(super) fn clone_node(
    l: &Lsf,
    wr: &mut Writer,
    src: usize,
    parent: usize,
    extra: &[NewAttr],
    patch: &dyn Fn(&str) -> Option<Vec<u8>>,
) -> Result<usize, String> {
    let mut attrs: Vec<NewAttr> = vec![];
    for a in l.attrs(src) {
        let bytes = match patch(l.name(a.name)) {
            Some(b) if b.len() == a.len => b,
            Some(_) => return Err(format!("{} has an unexpected size", l.name(a.name))),
            None => l.d[a.at..a.at + a.len].to_vec(),
        };
        attrs.push(NewAttr {
            name: a.name,
            ty: a.ty,
            bytes,
        });
    }
    for e in extra {
        attrs.push(NewAttr {
            name: e.name,
            ty: e.ty,
            bytes: e.bytes.clone(),
        });
    }
    let n = wr.add_node(l.node_name_ref(src), parent, &attrs)?;
    for c in l.children(src).collect::<Vec<_>>() {
        clone_node(l, wr, c, n, &[], &|_| None)?;
    }
    Ok(n)
}

/// Adds items to the inventory of the character at node `who`: each (stats name, amount).
/// Returns the new unpacked file.
pub fn add(plain: &[u8], who: usize, adds: &[(String, u32)]) -> Result<Vec<u8>, String> {
    let bad = |m: &str| {
        format!("This save's item records aren't laid out the way adding items needs ({m}), so nothing was added.")
    };
    let l = Lsf::read(plain).ok_or_else(|| bad("it doesn't read"))?;
    let w = world(&l).ok_or_else(|| bad("item tables"))?;
    if !layout_ok(&w) {
        return Err(bad("entities"));
    }
    let inv_handle = l
        .int_of(who, "Inventory")
        .filter(|&h| h != 0)
        .ok_or_else(|| bad("the character has no inventory"))?;
    let k = w
        .inv_creators
        .iter()
        .position(|&c| l.int_of(c, "Handle") == Some(inv_handle))
        .ok_or_else(|| bad("inventory"))?;
    let inv_node = w.invs[k];
    let owner = l.int_of(inv_node, "Parent").unwrap_or(0);
    let equip = l
        .int_of(inv_node, "EquipmentSlots")
        .filter(|&s| (0..=64).contains(&s))
        .unwrap_or(15);
    let mine: Vec<usize> = (0..w.items.len())
        .filter(|&j| l.int_of(w.items[j], "Parent") == Some(inv_handle))
        .collect();
    let mut slots: HashSet<i64> = mine
        .iter()
        .filter_map(|&j| l.int_of(w.items[j], "Slot"))
        .collect();

    let stack = stacking(&l, &w.items);
    let mut out = plain.to_vec();
    let mut copies: Vec<(&str, u32)> = vec![];
    for (stats, n) in adds {
        if *n == 0 {
            continue;
        }
        if *n > 1 && !stack.contains(stats) {
            return Err(format!("{stats} doesn't stack, so only one can be added"));
        }
        if stats.starts_with('_') || stats.starts_with("OC_") {
            return Err(format!("{stats} can't be added"));
        }
        let existing =
            stack
                .contains(stats)
                .then_some(())
                .and(mine.iter().map(|&j| w.items[j]).find(|&i| {
                    l.string_of(i, "Stats").as_deref() == Some(stats)
                        && l.attr(i, "Amount").is_some_and(|a| matches!(a.ty, 4 | 5))
                }));
        if let Some(i) = existing {
            let a = l.attr(i, "Amount").unwrap();
            let cur = l.int(&a).unwrap_or(0);
            let new = cur + i64::from(*n);
            if new > i64::from(i32::MAX) {
                return Err(format!("{stats}: the stack would get too big"));
            }
            out[a.at..a.at + 4].copy_from_slice(&(new as i32).to_le_bytes());
        } else {
            copies.push((stats.as_str(), *n));
        }
    }
    if copies.is_empty() {
        return Ok(out);
    }

    // Handles and uuids of everything there is, so the new ones are unused.
    let handles: Vec<i64> = w
        .creators
        .iter()
        .filter_map(|&c| l.int_of(c, "Handle"))
        .collect();
    let hi = handles
        .iter()
        .map(|h| h >> 32)
        .min()
        .ok_or_else(|| bad("handles"))?;
    let mut next_index = handles.iter().map(|h| h & 0xFFFF_FFFF).max().unwrap_or(0) + 1;
    let mut guids: HashSet<[u8; 16]> = w
        .creators
        .iter()
        .filter_map(|&c| bytes(&l, c, "UUID")?.try_into().ok())
        .collect();
    let players: HashSet<i64> = l
        .at_path(&["Characters", "CharacterFactory", "Characters", "Character"])
        .into_iter()
        .filter(|&c| l.child(c, "PlayerData").is_some())
        .filter_map(|c| l.int_of(c, "Inventory"))
        .collect();

    // Pieces to copy from.
    let item_parent = l.parent(w.items[0]).ok_or_else(|| bad("item list"))?;
    let creator_parent = l.parent(w.creators[0]).ok_or_else(|| bad("creator list"))?;
    let root = *l
        .at_path(&["EntityWorld"])
        .first()
        .ok_or_else(|| bad("entities"))?;
    let ent0 = l
        .children_named(root, "Entity")
        .next()
        .ok_or_else(|| bad("entities"))?;
    let comp0 = l.child(ent0, "Component").ok_or_else(|| bad("entities"))?;
    let times = l
        .children_named(inv_node, "TimeItemAddedToInventory")
        .collect::<Vec<_>>();
    let time0 = times.first().copied().or_else(|| {
        w.invs
            .iter()
            .find_map(|&i| l.child(i, "TimeItemAddedToInventory"))
    });
    let amount_attr = w
        .items
        .iter()
        .filter_map(|&i| l.attr(i, "Amount"))
        .find(|a| matches!(a.ty, 4 | 5) && a.len == 4);
    // The newest time among the inventory's entries; a new item is entered as added just now.
    let newest = times
        .iter()
        .filter_map(|&t| bytes(&l, t, "MapValue"))
        .filter(|b| b.len() == 8)
        .map(|b| f64::from_le_bytes(b.try_into().unwrap()))
        .fold(f64::NAN, f64::max);

    let mut wr = Writer::new(&out).ok_or_else(|| bad("node writer"))?;
    for (stats, n) in copies {
        // A copy of the item as a player carries it, if there is one.
        let of_kind = |&j: &usize| {
            l.string_of(w.items[j], "Stats").as_deref() == Some(stats) && copyable(&l, w.items[j])
        };
        let si = (0..w.items.len())
            .filter(of_kind)
            .max_by_key(|&j| {
                l.int_of(w.items[j], "Parent")
                    .is_some_and(|p| players.contains(&p))
            })
            .ok_or_else(|| format!("{stats} can't be added to this save"))?;
        let src = w.items[si];
        let multi = l.attr(src, "Amount").is_some();
        let handle = (hi << 32) | next_index;
        next_index += 1;
        let guid = random_guid(&guids);
        guids.insert(guid);
        let slot = (equip..).find(|s| !slots.contains(s)).unwrap_or(equip);
        slots.insert(slot);

        // The creator record, then the item node (the lists stay in step).
        let src_creator = w.creators[si];
        // Older saves keep the uuid as text.
        let uuid = if l.attr(src_creator, "UUID").is_some_and(|a| a.ty == 22) {
            let mut t = super::larian::guid_text(&guid).into_bytes();
            t.push(0);
            t
        } else {
            guid.to_vec()
        };
        clone_node(
            &l,
            &mut wr,
            src_creator,
            creator_parent,
            &[],
            &|name| match name {
                "Handle" => Some(put_i64(handle)),
                "UUID" => Some(uuid.clone()),
                _ => None,
            },
        )?;
        let extra: Vec<NewAttr> = match (n > 1 && !multi, amount_attr) {
            (true, Some(a)) => vec![NewAttr {
                name: a.name,
                ty: a.ty,
                bytes: (n as i32).to_le_bytes().to_vec(),
            }],
            (true, None) => return Err(bad("amounts")),
            _ => vec![],
        };
        clone_node(&l, &mut wr, src, item_parent, &extra, &|name| match name {
            "Parent" => Some(put_i64(inv_handle)),
            "Slot" => Some((slot as u16).to_le_bytes().to_vec()),
            "owner" | "OriginalOwnerCharacter" => Some(put_i64(owner)),
            "Amount" => Some((n as i32).to_le_bytes().to_vec()),
            _ => None,
        })?;
        // The inventory remembers when the item came in.
        if let Some(t0) = time0 {
            let value = if newest.is_nan() {
                None
            } else {
                Some(newest.to_le_bytes().to_vec())
            };
            clone_node(&l, &mut wr, t0, inv_node, &[], &|name| match name {
                "MapKey" => Some(put_i64(handle)),
                "MapValue" => value.clone(),
                _ => None,
            })?;
        }
        // The entity list.
        let ent = wr.add_node(l.node_name_ref(ent0), root, &[])?;
        clone_node(&l, &mut wr, comp0, ent, &[], &|name| {
            (name == "Handle").then(|| put_i64(handle))
        })?;
        // Views of this inventory that list its items (the bag view) show it too.
        for v in l.at_path(&VIEWS) {
            let parents: Vec<i64> = l
                .children_named(v, "Parents")
                .filter_map(|p| l.int_of(p, "Object"))
                .collect();
            let listed: Vec<usize> = l.children_named(v, "Items").collect();
            if parents == [inv_handle] && !listed.is_empty() && l.child(v, "Indices").is_none() {
                clone_node(&l, &mut wr, listed[listed.len() - 1], v, &[], &|name| {
                    (name == "Object").then(|| put_i64(handle))
                })?;
            }
        }
    }
    let result = wr.finish();
    // The result must read, with the new items where they belong.
    let r = Lsf::read(&result).ok_or("the edited save no longer reads, so nothing was added")?;
    let w2 = world(&r).ok_or("the edited save's item tables don't read, so nothing was added")?;
    if !layout_ok(&w2) {
        return Err("the edited save's records don't agree, so nothing was added".into());
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fmt::lsf::tests::build as lsf_build;
    use crate::fmt::lspk::{self, tests::build as pkg_build};
    use serde_json::json;

    type Node<'a> = (&'a str, i32, Vec<(&'a str, u32, Vec<u8>)>);

    fn i64v(v: i64) -> Vec<u8> {
        v.to_le_bytes().to_vec()
    }
    fn s(v: &str) -> Vec<u8> {
        let mut b = v.as_bytes().to_vec();
        b.push(0);
        b
    }

    const PLAYER: i64 = 0x40_0000_0000_0010;
    const INV: i64 = 0x140_0000_0000_0001;
    const CHEST: i64 = 0x140_0000_0000_0002;
    const POTION: i64 = 0x80_0001_0000_0005;
    const SWORD: i64 = 0x80_0002_0000_0009;

    /// A small DOS2 world: one player (made-up "Testa") carrying a stack of three potions,
    /// a chest holding a sword, and the entity and inventory records for all of it.
    fn world_nodes() -> Vec<Node<'static>> {
        let item = |stats: &str, parent: i64, slot: u16, amount: Option<i32>| {
            let mut a = vec![
                ("Flags", 24, i64v(0x4_0000_40ec)),
                ("CurrentTemplate", 31, vec![7u8; 16]),
                ("Stats", 22, s(stats)),
                ("Parent", 24, i64v(parent)),
                ("Slot", 3, slot.to_le_bytes().to_vec()),
                ("owner", 24, i64v(PLAYER)),
            ];
            if let Some(n) = amount {
                a.push(("Amount", 4, n.to_le_bytes().to_vec()));
            }
            a
        };
        let mut n: Vec<Node> = vec![
            ("EntityWorld", -1, vec![]),
            ("Entity", 0, vec![]),
            ("Component", 1, vec![("Handle", 24, i64v(POTION))]),
            ("Entity", 0, vec![]),
            ("Component", 3, vec![("Handle", 24, i64v(SWORD))]),
            ("Characters", -1, vec![]),
            ("CharacterFactory", 5, vec![]),
            ("Characters", 6, vec![]),
            ("Character", 7, vec![("Inventory", 24, i64v(INV))]),
            ("Stats", 8, vec![("IsPlayer", 19, vec![1])]),
            ("PlayerData", 8, vec![]),
            ("PlayerCustomData", 10, vec![("Name", 22, s("Testa"))]),
            ("Inventories", -1, vec![]),
            ("InventoryFactory", 12, vec![]),
            ("Creators", 13, vec![]),
            ("Creator", 14, vec![("Handle", 24, i64v(INV))]),
            ("Creator", 14, vec![("Handle", 24, i64v(CHEST))]),
            ("Inventories", 13, vec![]),
            (
                "Inventory",
                17,
                vec![
                    ("EquipmentSlots", 1, vec![15]),
                    ("Parent", 24, i64v(PLAYER)),
                ],
            ),
            (
                "TimeItemAddedToInventory",
                18,
                vec![
                    ("MapKey", 24, i64v(POTION)),
                    ("MapValue", 7, 120.5f64.to_le_bytes().to_vec()),
                ],
            ),
            (
                "Inventory",
                17,
                vec![
                    ("EquipmentSlots", 1, vec![15]),
                    ("Parent", 24, i64v(0x40_0000_0000_0099)),
                ],
            ),
            (
                "TimeItemAddedToInventory",
                20,
                vec![
                    ("MapKey", 24, i64v(SWORD)),
                    ("MapValue", 7, 3.0f64.to_le_bytes().to_vec()),
                ],
            ),
            ("Items", -1, vec![]),
            ("ItemFactory", 22, vec![]),
            ("Creators", 23, vec![]),
            (
                "Creator",
                24,
                vec![
                    ("Handle", 24, i64v(POTION)),
                    ("TemplateID", 31, vec![7u8; 16]),
                    ("UUID", 31, vec![1u8; 16]),
                ],
            ),
            (
                "Creator",
                24,
                vec![
                    ("Handle", 24, i64v(SWORD)),
                    ("TemplateID", 31, vec![8u8; 16]),
                    ("UUID", 31, vec![2u8; 16]),
                ],
            ),
            ("Items", 23, vec![]),
        ];
        n.push(("Item", 27, item("CON_Potion_Health", INV, 15, Some(3))));
        n.push(("Item", 27, item("WPN_Sword_1H", CHEST, 15, None)));
        for (k, p) in [
            ("ItemMachine", 29),
            ("StatusManager", 29),
            ("VariableManager", 29),
            ("Generation", 29),
        ] {
            n.push((k, p, vec![]));
        }
        n.push((
            "Boost",
            33,
            vec![("Object", 22, s("_Boost_Weapon_Damage_Bonus"))],
        ));
        n
    }

    fn save() -> Vec<u8> {
        let globals = lsf_build(&world_nodes(), true);
        let meta = lsf_build(
            &[
                ("MetaData", -1, vec![]),
                ("MetaData", 0, vec![("Level", 22, s("TestRegion"))]),
            ],
            true,
        );
        pkg_build(13, &[("globals.lsf", &globals), ("meta.lsf", &meta)], 0x21)
    }

    #[test]
    fn writer_adds_nodes_and_keeps_the_rest() {
        let plain = crate::fmt::lsf::unpack(&lsf_build(&world_nodes(), false)).unwrap();
        let l = Lsf::read(&plain).unwrap();
        let w = Writer::new(&plain).unwrap();
        assert_eq!(w.finish(), plain, "nothing added: the same bytes");
        let mut w = Writer::new(&plain).unwrap();
        let comp = l.at_path(&["EntityWorld", "Entity", "Component"])[0];
        let a = l.attr(comp, "Handle").unwrap();
        let e = w.add_node(l.node_name_ref(1), 0, &[]).unwrap();
        w.add_node(
            l.node_name_ref(comp),
            e,
            &[NewAttr {
                name: a.name,
                ty: 24,
                bytes: i64v(42),
            }],
        )
        .unwrap();
        assert!(w.add_node(1, 9999, &[]).is_err());
        let out = w.finish();
        let r = Lsf::read(&out).unwrap();
        let handles: Vec<i64> = r
            .at_path(&["EntityWorld", "Entity", "Component"])
            .into_iter()
            .filter_map(|c| r.int_of(c, "Handle"))
            .collect();
        assert_eq!(handles, vec![POTION, SWORD, 42]);
        // Everything that was there reads the same.
        for n in 0..l.node_count() {
            assert_eq!(l.node_name(n), r.node_name(n));
            let before: Vec<_> = l
                .attrs(n)
                .map(|a| (a.name, l.d[a.at..a.at + a.len].to_vec()))
                .collect();
            let after: Vec<_> = r
                .attrs(n)
                .map(|a| (a.name, r.d[a.at..a.at + a.len].to_vec()))
                .collect();
            assert_eq!(before, after);
        }
    }

    #[test]
    fn adds_items_through_the_screen() {
        let raw = save();
        let mut d = crate::formats::decode(&raw);
        let crate::formats::Content::Tree(t) = &mut d.content else {
            panic!("not read")
        };
        assert_eq!(
            t.choices("/Add items/WPN_Sword_1H").unwrap().as_slice(),
            ["Don't add", "Add 1"],
            "a sword doesn't stack"
        );
        assert!(t
            .choices("/Add items/CON_Potion_Health")
            .unwrap()
            .contains(&"Add 99".to_string()));
        assert_eq!(t.view.pointer("/Add items/Add to"), Some(&json!("Testa")));
        assert_eq!(
            crate::formats::encode(&d.layers, &d.content).unwrap(),
            raw,
            "no choice keeps the file"
        );
        let crate::formats::Content::Tree(t) = &mut d.content else {
            unreachable!()
        };
        t.set("/Add items/WPN_Sword_1H", json!("Add 1")).unwrap();
        t.set("/Add items/CON_Potion_Health", json!("Add 10"))
            .unwrap();
        let out = crate::formats::encode(&d.layers, &d.content).unwrap();
        let p = lspk::read(&out).unwrap();
        assert!(p.checksum_ok(&out));
        let again = crate::formats::decode(&out);
        let crate::formats::Content::Tree(t2) = &again.content else {
            panic!("re-read failed")
        };
        assert_eq!(
            t2.view.pointer("/Party/Testa/Inventory/CON_Potion_Health"),
            Some(&json!(13)),
            "the carried stack grew"
        );
        let (_, plain) = super::super::larian::open(&out).unwrap();
        let l = Lsf::read(&plain).unwrap();
        let items = l.at_path(&ITEMS);
        let creators = l.at_path(&ITEM_CREATORS);
        assert_eq!((items.len(), creators.len()), (3, 3));
        let new = items[2];
        assert_eq!(l.string_of(new, "Stats").as_deref(), Some("WPN_Sword_1H"));
        assert_eq!(l.int_of(new, "Parent"), Some(INV));
        assert_eq!(l.int_of(new, "Slot"), Some(16), "the first free bag slot");
        assert_eq!(l.int_of(new, "owner"), Some(PLAYER));
        let names: Vec<&str> = l.children(new).map(|c| l.node_name(c)).collect();
        assert_eq!(
            names,
            [
                "ItemMachine",
                "StatusManager",
                "VariableManager",
                "Generation"
            ]
        );
        let boost = l
            .child(l.child(new, "Generation").unwrap(), "Boost")
            .unwrap();
        assert_eq!(
            l.string_of(boost, "Object").as_deref(),
            Some("_Boost_Weapon_Damage_Bonus")
        );
        let h = l.int_of(creators[2], "Handle").unwrap();
        assert_eq!(
            h,
            (0x80_0001i64 << 32) | 10,
            "a new handle after the highest"
        );
        assert_ne!(
            l.attr(creators[2], "UUID")
                .map(|a| l.d[a.at..a.at + 16].to_vec()),
            Some(vec![2u8; 16])
        );
        let inv = l.at_path(&INVS)[0];
        let keys: Vec<i64> = l
            .children_named(inv, "TimeItemAddedToInventory")
            .filter_map(|t| l.int_of(t, "MapKey"))
            .collect();
        assert_eq!(keys, vec![POTION, h]);
        let comps: Vec<i64> = l
            .at_path(&["EntityWorld", "Entity", "Component"])
            .into_iter()
            .filter_map(|c| l.int_of(c, "Handle"))
            .collect();
        assert_eq!(comps, vec![POTION, SWORD, h]);
        // The chest is untouched.
        assert_eq!(
            l.children_named(l.at_path(&INVS)[1], "TimeItemAddedToInventory")
                .count(),
            1
        );
        // A second sword goes into the next slot.
        let mut d = again;
        let crate::formats::Content::Tree(t) = &mut d.content else {
            unreachable!()
        };
        t.set("/Add items/WPN_Sword_1H", json!("Add 1")).unwrap();
        let out2 = crate::formats::encode(&d.layers, &d.content).unwrap();
        let (_, plain) = super::super::larian::open(&out2).unwrap();
        let l = Lsf::read(&plain).unwrap();
        assert_eq!(l.int_of(l.at_path(&ITEMS)[3], "Slot"), Some(17));
    }

    #[test]
    fn refuses_when_records_disagree() {
        let mut nodes = world_nodes();
        // An item without its entity.
        nodes[4].2 = vec![("Handle", 24, i64v(12345))];
        let plain = crate::fmt::lsf::unpack(&lsf_build(&nodes, false)).unwrap();
        assert!(offers(&plain).is_empty());
        let l = Lsf::read(&plain).unwrap();
        let who = super::super::larian::dos2_party(&l)[0].1;
        assert!(add(&plain, who, &[("WPN_Sword_1H".into(), 1)]).is_err());
        // Something that doesn't stack.
        let plain = crate::fmt::lsf::unpack(&lsf_build(&world_nodes(), false)).unwrap();
        assert!(add(&plain, who, &[("WPN_Sword_1H".into(), 2)]).is_err());
        assert!(add(&plain, who, &[("Nothing_Like_This".into(), 1)]).is_err());
    }
}
