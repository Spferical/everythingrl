use std::collections::{HashMap, HashSet, VecDeque};

use crate::grid::{self, Offset, Pos, TileMap, CARDINALS};
use crate::net::{
    is_weapon_slot, Color, EquipmentSlot, IdeaGuy, ItemDefinition, MonsterDefinition, PokemonType,
};
use enum_map::{enum_map, Enum, EnumMap};
use lazy_static::lazy_static;
use rand::{seq::SliceRandom as _, SeedableRng};

pub const FOV_RANGE: i32 = 16;
pub const STARTING_DURABILITY: usize = 100;

pub const PICK_UP_MESSAGES: [&str; 5] = [
    "You see here a ",
    "You step over a ",
    "You notice a ",
    "You find a ",
    "You discover a ",
];

#[derive(Enum, PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum TileKind {
    Floor,
    Wall,
    YellowFloor,
    YellowWall,
    BloodyFloor,
    Stairs,
}

impl TileKind {
    pub fn is_opaque(self) -> bool {
        TILE_INFOS[self].opaque
    }

    pub fn is_walkable(self) -> bool {
        TILE_INFOS[self].walkable
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub struct EquipmentKind(pub usize);

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub struct EquipmentInstance {
    pub kind: EquipmentKind,
    pub item_durability: usize,
}

impl EquipmentInstance {
    pub fn new(kind: EquipmentKind, item_durability: usize) -> EquipmentInstance {
        EquipmentInstance {
            kind,
            item_durability,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Item {
    Corpse(MobKind),
    Equipment(EquipmentInstance),
    PendingCraft(EquipmentKind, EquipmentKind),
}

pub struct TileKindInfo {
    pub opaque: bool,
    pub walkable: bool,
}

lazy_static! {
    pub static ref TILE_INFOS: EnumMap<TileKind, TileKindInfo> = enum_map! {
        TileKind::Floor | TileKind::YellowFloor | TileKind::BloodyFloor | TileKind::Stairs=> TileKindInfo {
            opaque: false,
            walkable: true,
        },
        TileKind::Wall | TileKind::YellowWall => TileKindInfo {
            opaque: true,
            walkable: false,
        },
    };
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub struct Tile {
    pub kind: TileKind,
    pub item: Option<Item>,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
/// Index into World.mob_kinds.
pub struct MobKind(pub usize);

#[derive(Hash, Debug, Clone)]
pub enum MobAi {
    Idle,
    Move { dest: Pos },
}

#[derive(Hash, Debug, Clone)]
pub struct Mob {
    pub kind: MobKind,
    pub damage: usize,
    pub ai: MobAi,
}

impl Mob {
    pub fn new(kind: MobKind) -> Self {
        Self {
            kind,
            damage: 0,
            ai: MobAi::Idle,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EquipmentKindInfo {
    pub name: String,
    pub level: usize,
    pub ty: PokemonType,
    pub description: String,
    pub slot: EquipmentSlot,
}

#[derive(Debug, Clone)]
pub struct MobKindInfo {
    pub name: String,
    pub char: String,
    pub color: Color,
    pub attack_type: PokemonType,
    pub type1: PokemonType,
    pub type2: Option<PokemonType>,
    pub description: String,
    pub level: usize,
    pub seen: String,
    pub attack: String,
    pub death: String,
}

impl MobKindInfo {
    pub fn max_hp(&self) -> usize {
        self.level * 16
    }
}

/// Contains post-processed content definitions parsed from AI-generated data.
#[derive(Debug, Clone)]
pub struct WorldInfo {
    pub equip_kinds: Vec<EquipmentKindInfo>,
    pub monster_kinds: Vec<MobKindInfo>,
    pub monsters_per_level: Vec<Vec<MobKind>>,
    pub equipment_per_level: Vec<Vec<EquipmentKind>>,
    pub recipes: HashMap<(EquipmentKind, EquipmentKind), EquipmentKind>,
    pub pending_recipes: HashSet<(EquipmentKind, EquipmentKind)>,
    pub level_blurbs: Vec<String>,
}

impl WorldInfo {
    pub fn new() -> Self {
        Self {
            equip_kinds: Vec::new(),
            monster_kinds: Vec::new(),
            monsters_per_level: Vec::new(),
            equipment_per_level: Vec::new(),
            pending_recipes: HashSet::new(),
            recipes: HashMap::new(),
            level_blurbs: Vec::new(),
        }
    }

    pub fn update(&mut self, ig: &mut IdeaGuy) {
        for item in ig.items.iter().flatten() {
            if self.equip_kinds.iter().any(|e| e.name == item.name) {
                continue;
            }
            let ItemDefinition {
                name,
                level,
                ty,
                slot,
                description,
                ..
            } = item.clone();
            self.equip_kinds.push(EquipmentKindInfo {
                name,
                level,
                ty,
                description,
                slot,
            });
        }
        for mob in ig.monsters.iter().flatten() {
            if self.equip_kinds.iter().any(|m| m.name == mob.name) {
                continue;
            }
            let MonsterDefinition {
                name,
                char,
                color,
                attack_type,
                type1,
                type2,
                description,
                level,
                seen,
                attack,
                death,
            } = mob.clone();
            self.monster_kinds.push(MobKindInfo {
                name,
                char,
                color,
                attack_type,
                type1,
                type2,
                description,
                level,
                seen,
                attack,
                death,
            });
        }

        let get_monster_by_name = |name: &String| {
            self.monster_kinds
                .iter()
                .position(|k| &k.name == name)
                .map(MobKind)
        };

        self.monsters_per_level = ig
            .areas
            .iter()
            .flatten()
            .map(|area| {
                area.enemies
                    .iter()
                    .filter_map(get_monster_by_name)
                    .collect()
            })
            .collect();

        let get_equipment_by_name = |name: &String| {
            self.equip_kinds
                .iter()
                .position(|k| &k.name == name)
                .map(EquipmentKind)
        };

        self.equipment_per_level = ig
            .areas
            .iter()
            .flatten()
            .map(|area| {
                area.equipment
                    .iter()
                    .chain(area.melee_weapons.iter())
                    .filter_map(get_equipment_by_name)
                    .collect()
            })
            .collect();
        self.level_blurbs = ig
            .areas
            .iter()
            .flatten()
            .map(|area| format!("{}: {}", area.name, area.blurb.clone()))
            .collect();

        for (&(a, b), &c) in ig.recipes.iter() {
            let ek_by_name = |name: &str| {
                EquipmentKind(
                    self.equip_kinds
                        .iter()
                        .position(|ek| ek.name == name)
                        .unwrap(),
                )
            };
            let ig_item_a = &ig.items.as_ref().unwrap()[a];
            let ig_item_b = &ig.items.as_ref().unwrap()[b];
            let ig_item_c = &ig.items.as_ref().unwrap()[c];
            let ek_a = ek_by_name(&ig_item_a.name);
            let ek_b = ek_by_name(&ig_item_b.name);
            let ek_c = ek_by_name(&ig_item_c.name);
            self.recipes.insert((ek_a, ek_b), ek_c);
        }
        if let Some((a, b)) = self.pending_recipes.iter().next().cloned() {
            let ig_equip_by_name = |name: &str| {
                ig.items
                    .iter()
                    .flatten()
                    .position(|x| x.name == name)
                    .unwrap()
            };
            self.pending_recipes.remove(&(a, b));
            let ig_a = ig_equip_by_name(&self.get_equipmentkind_info(a).name);
            let ig_b = ig_equip_by_name(&self.get_equipmentkind_info(b).name);
            ig.craft(ig_a, ig_b);
        }
    }

    pub fn get_equipmentkind_info(&self, kind: EquipmentKind) -> &EquipmentKindInfo {
        &self.equip_kinds[kind.0]
    }

    pub fn get_mobkind_info(&self, kind: MobKind) -> &MobKindInfo {
        &self.monster_kinds[kind.0]
    }

    fn craft_inner(&mut self, ek1: EquipmentKind, ek2: EquipmentKind) -> Option<Item> {
        if let Some(ek3) = self.recipes.get(&(ek1, ek2)) {
            Some(Item::Equipment(EquipmentInstance {
                kind: *ek3,
                item_durability: STARTING_DURABILITY,
            }))
        } else {
            self.pending_recipes.insert((ek1, ek2));
            Some(Item::PendingCraft(ek1, ek2))
        }
    }

    pub fn craft(&mut self, item1: Item, item2: Item) -> Option<Item> {
        match (item1, item2) {
            (Item::Equipment(ei1), Item::Equipment(ei2)) => self.craft_inner(ei1.kind, ei2.kind),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct InventoryItem {
    pub item: Item,
    pub equipped: bool,
}

#[derive(Debug, Clone)]
pub struct Inventory {
    pub items: Vec<InventoryItem>,
}

impl Inventory {
    // All of these methods suck, refactor.
    fn new() -> Self {
        Self { items: vec![] }
    }

    fn damage_weapon(&mut self, wi: &WorldInfo, melee: bool) -> Option<EquipmentKindInfo> {
        if let Some(player_weapon) = self.get_equipped_weapon(wi, melee) {
            player_weapon.item_durability -= 1;
            if player_weapon.item_durability == 0 {
                let weapon_info = self.get_equipped_weapon_info(wi, melee);
                self.remove(self.get_equipped_weapon_slot(wi, melee).unwrap());
                weapon_info
            } else {
                None
            }
        } else {
            None
        }
    }

    fn damage_armor(&mut self, wi: &WorldInfo) -> Vec<EquipmentKindInfo> {
        let mut delete_idx = Vec::new();
        for (i, player_armor) in self.get_equipped_armor(wi).into_iter().enumerate() {
            player_armor.item_durability -= 1;
            if player_armor.item_durability == 0 {
                delete_idx.push(i);
            }
        }

        // Pretty important to do it in this order!
        let mut deleted_armor = Vec::new();
        for idx in delete_idx.iter() {
            deleted_armor.push(self.get_equipped_armor_info(wi)[*idx].clone());
        }
        for idx in delete_idx {
            self.remove(self.get_equipped_armor_slots(wi)[idx]);
        }

        deleted_armor
    }

    fn get_equipped_weapon(
        &mut self,
        wi: &WorldInfo,
        melee: bool,
    ) -> Option<&mut EquipmentInstance> {
        self.items
            .iter_mut()
            .filter(|x| x.equipped)
            .filter_map(|x| match x.item {
                Item::Corpse(_) => None,
                Item::PendingCraft(_, _) => None,
                Item::Equipment(ref mut i) => Some(i),
            })
            .find(|i| {
                let eki = wi.get_equipmentkind_info(i.kind);
                match eki.slot {
                    EquipmentSlot::MeleeWeapon => melee,
                    EquipmentSlot::RangedWeapon => !melee,
                    _ => false,
                }
            })
    }

    fn get_equipped_weapon_info(&self, wi: &WorldInfo, melee: bool) -> Option<EquipmentKindInfo> {
        self.items
            .iter()
            .filter(|x| x.equipped)
            .filter_map(|x| match x.item {
                Item::Equipment(ek) => Some(ek),
                _ => None,
            })
            .map(|ek| wi.get_equipmentkind_info(ek.kind))
            .find(|eki| match eki.slot {
                EquipmentSlot::MeleeWeapon => melee,
                EquipmentSlot::RangedWeapon => !melee,
                _ => false,
            })
            .cloned()
    }

    fn get_equipped_weapon_slot(&self, wi: &WorldInfo, melee: bool) -> Option<usize> {
        self.items
            .iter()
            .enumerate()
            .filter(|(_, x)| x.equipped)
            .filter_map(|(i, x)| match x.item {
                Item::Corpse(_) => None,
                Item::PendingCraft(_, _) => None,
                Item::Equipment(ek) => Some((i, ek)),
            })
            .map(|(i, ek)| (i, wi.get_equipmentkind_info(ek.kind)))
            .find(|(_, eki)| match eki.slot {
                EquipmentSlot::MeleeWeapon => melee,
                EquipmentSlot::RangedWeapon => !melee,
                _ => false,
            })
            .map(|(i, _)| i)
    }

    fn get_equipped_armor(&mut self, wi: &WorldInfo) -> Vec<&mut EquipmentInstance> {
        self.items
            .iter_mut()
            .filter(|x| x.equipped)
            .filter_map(|x| match x.item {
                Item::Corpse(_) => None,
                Item::PendingCraft(_, _) => None,
                Item::Equipment(ref mut i) => Some(i),
            })
            .filter(|i| {
                let eki = wi.get_equipmentkind_info(i.kind);
                eki.slot == EquipmentSlot::Armor
            })
            .collect()
    }

    fn get_equipped_armor_info(&self, wi: &WorldInfo) -> Vec<EquipmentKindInfo> {
        self.items
            .iter()
            .filter(|x| x.equipped)
            .filter_map(|x| match x.item {
                Item::Equipment(ek) => Some(ek),
                _ => None,
            })
            .map(|ek| wi.get_equipmentkind_info(ek.kind))
            .filter(|eki| eki.slot == EquipmentSlot::Armor)
            .cloned()
            .collect()
    }

    fn get_equipped_armor_slots(&self, wi: &WorldInfo) -> Vec<usize> {
        self.items
            .iter()
            .enumerate()
            .filter(|(_, x)| x.equipped)
            .filter_map(|(i, x)| match x.item {
                Item::Corpse(_) => None,
                Item::PendingCraft(_, _) => None,
                Item::Equipment(ek) => Some((i, ek)),
            })
            .map(|(i, ek)| (i, wi.get_equipmentkind_info(ek.kind)))
            .filter(|(_, eki)| eki.slot == EquipmentSlot::Armor)
            .map(|(i, _)| i)
            .collect()
    }

    fn sort(&mut self, wi: &WorldInfo) {
        self.items.sort_by_key(|x| match x {
            InventoryItem {
                item: Item::Equipment(ek),
                equipped,
            } => match (equipped, wi.get_equipmentkind_info(ek.kind).slot) {
                (true, EquipmentSlot::MeleeWeapon) => 1,
                (true, EquipmentSlot::RangedWeapon) => 2,
                (true, EquipmentSlot::Armor) => 3,
                (false, EquipmentSlot::MeleeWeapon) => 4,
                (false, EquipmentSlot::RangedWeapon) => 5,
                (false, EquipmentSlot::Armor) => 6,
            },
            InventoryItem {
                item: Item::PendingCraft(..),
                ..
            } => 5,
            InventoryItem {
                item: Item::Corpse(_),
                ..
            } => 6,
        });
    }
    fn add(&mut self, item: Item) -> Option<Item> {
        self.items.push(InventoryItem {
            item,
            equipped: false,
        });
        if self.items.len() > 9 {
            for i in 0..self.items.len() {
                if !self.items[i].equipped {
                    return Some(self.items.remove(i).item);
                }
            }
        }
        None
    }

    fn get(&self, i: usize) -> Option<Item> {
        self.items.get(i).map(|x| x.item)
    }

    fn remove_all(&mut self, mut indices: Vec<usize>) {
        indices.sort();
        indices.dedup();
        for i in indices.iter().rev() {
            self.remove(*i);
        }
    }
    fn remove(&mut self, i: usize) -> Option<Item> {
        if i < self.items.len() {
            Some(self.items.remove(i).item)
        } else {
            None
        }
    }
    fn toggle_equip(&mut self, i: usize, wi: &WorldInfo) -> bool {
        if i >= self.items.len() {
            eprintln!("Bad equip idx: {i}");
            false
        } else if self.items[i].equipped {
            self.items[i].equipped = false;
            true
        } else if let Item::Equipment(ek) = self.items[i].item {
            let ek_def = wi.get_equipmentkind_info(ek.kind);

            // Unequip another item if that slot is full.
            let max_per_slot = |slot: EquipmentSlot| match slot {
                EquipmentSlot::MeleeWeapon => 1,
                EquipmentSlot::RangedWeapon => 1,
                EquipmentSlot::Armor => 2,
            };
            let max = max_per_slot(ek_def.slot);
            let other_equipped_in_slot = self
                .items
                .iter()
                .enumerate()
                .filter(|(_i, x)| x.equipped)
                .filter_map(|(i, x)| {
                    if let Item::Equipment(ek) = x.item {
                        Some((i, wi.get_equipmentkind_info(ek.kind)))
                    } else {
                        None
                    }
                })
                .filter(|(_i, other_ek_def)| other_ek_def.slot == ek_def.slot)
                .map(|(i, _)| i)
                .collect::<Vec<_>>();
            if other_equipped_in_slot.len() >= max {
                self.items[other_equipped_in_slot[0]].equipped = false;
            }

            self.items[i].equipped = true;
            true
        } else {
            eprintln!("Item is not equippable");
            false
        }
    }
}

#[derive(Clone)]
pub struct World {
    pub player_pos: Pos,
    pub player_damage: usize,
    tile_map: TileMap<Tile>,
    pub world_info: WorldInfo,
    pub mobs: HashMap<Pos, Mob>,
    pub inventory: Inventory,
    pub log: VecDeque<Vec<(String, Color)>>,
    stairs: HashMap<Pos, Pos>,
    level_id: usize,
    rng: rand::rngs::SmallRng,
}

pub enum PlayerAction {
    Move(Offset),
    PickUp,
    ToggleEquip(usize),
    Drop(usize),
    Craft(usize, usize),
    Wait,
}

impl World {
    pub fn new() -> Self {
        Self {
            player_pos: Pos { x: 0, y: 0 },
            player_damage: 0,
            tile_map: TileMap::new(Tile {
                kind: TileKind::Wall,
                item: None,
            }),
            world_info: WorldInfo::new(),
            mobs: HashMap::new(),
            rng: rand::rngs::SmallRng::seed_from_u64(72),
            inventory: Inventory::new(),
            log: VecDeque::new(),
            stairs: HashMap::new(),
            level_id: 0,
        }
    }

    pub fn post_init(&mut self) {
        self.log_message(vec![(
            self.world_info.level_blurbs[0].clone(),
            Color::White,
        )]);
    }

    pub fn add_stairs(&mut self, pos: Pos, dest: Pos) {
        self.stairs.insert(pos, dest);
        self[pos].kind = TileKind::Stairs;
        self[pos].item = None;
    }

    pub fn update_defs(&mut self, ig: &mut IdeaGuy) {
        self.world_info.update(ig);
        for item in &mut self.inventory.items {
            if let Item::PendingCraft(a, b) = item.item {
                if let Some(c) = self.world_info.recipes.get(&(a, b)) {
                    item.item = Item::Equipment(EquipmentInstance::new(*c, STARTING_DURABILITY));
                }
            }
        }
    }

    pub fn log_message(&mut self, text: Vec<(String, Color)>) {
        println!(
            "{}",
            text.iter()
                .map(|(s, _)| s.to_owned())
                .collect::<Vec::<String>>()
                .join("")
        );
        self.log.push_back(text);
    }

    pub fn get_item_log_message(&self, item: &Item) -> (String, Color) {
        match item {
            Item::Corpse(mob_kind) => {
                let mob_desc = &self.get_mobkind_info(*mob_kind);
                (format!("{} Corpse", mob_desc.name), Color::Maroon)
            }
            Item::Equipment(item) => {
                let item_desc = &self.get_equipmentkind_info(item.kind);
                (item_desc.name.clone(), item_desc.ty.get_color())
            }
            Item::PendingCraft(..) => ("???".to_string(), Color::Pink),
        }
    }

    pub fn do_player_action(&mut self, action: PlayerAction) -> bool {
        let tick = match action {
            PlayerAction::Move(offset) => {
                assert!(offset.mhn_dist() == 1);
                let new_pos = self.player_pos + offset;
                if let Some(mut mob) = self.mobs.remove(&new_pos) {
                    let mki = self.get_mobkind_info(mob.kind).clone();
                    let player_weapon_info = self
                        .inventory
                        .get_equipped_weapon_info(&self.world_info, true);
                    let (att_type, att_level) = player_weapon_info
                        .clone()
                        .map(|w| (w.ty, w.level))
                        .unwrap_or((PokemonType::Normal, 0));
                    let eff = att_type.get_effectiveness2(mki.type1, mki.type2);
                    let mult = eff.get_scale();
                    let damage = (att_level + 1) * mult;

                    if let Some(destroyed_weapon) =
                        self.inventory.damage_weapon(&self.world_info, true)
                    {
                        self.log_message(vec![
                            ("Your ".into(), Color::White),
                            (
                                destroyed_weapon.name.into(),
                                destroyed_weapon.ty.get_color(),
                            ),
                            (" breaks!".into(), Color::Red),
                        ]);
                    }

                    mob.damage += damage;
                    self.log_message(vec![
                        ("You hit ".into(), Color::White),
                        (mki.name.clone(), mki.color),
                        (" for ".into(), Color::White),
                        (format!("{}", damage), Color::Red),
                    ]);
                    if mob.damage >= mki.max_hp() {
                        self.log_message(vec![(mki.death, mki.color)]);
                        self.tile_map[new_pos].item = Some(Item::Corpse(mob.kind));
                    } else {
                        self.mobs.insert(new_pos, mob);
                    }

                    true
                } else if self.tile_map[new_pos].kind.is_walkable() {
                    // Check if player walks over an item.
                    if let Some(item) = self.tile_map[new_pos].item {
                        let msg = vec![
                            (
                                PICK_UP_MESSAGES
                                    .choose(&mut self.rng)
                                    .unwrap()
                                    .to_owned()
                                    .to_owned(),
                                Color::White,
                            ),
                            self.get_item_log_message(&item),
                        ];
                        self.log_message(msg);
                    }

                    if let Some(dest) = self.stairs.get(&new_pos) {
                        self.player_pos = *dest;
                        self.mobs.remove(dest);
                        self.level_id += 1;
                        self.log_message(vec![(
                            self.world_info.level_blurbs[self.level_id].clone(),
                            Color::White,
                        )]);
                    } else {
                        self.player_pos += offset;
                    }
                    true
                } else {
                    false
                }
            }
            PlayerAction::PickUp => {
                if let Some(item) = self.tile_map[self.player_pos].item.take() {
                    if let Some(popped) = self.inventory.add(item) {
                        self.log_message(vec![
                            ("Inventory full, so swapped out ".to_owned(), Color::White),
                            self.get_item_log_message(&popped),
                            (" for ".to_owned(), Color::White),
                            self.get_item_log_message(&item),
                        ]);
                        self.tile_map[self.player_pos].item = Some(popped);
                    } else {
                        self.log_message(vec![
                            ("Picked up ".to_owned(), Color::White),
                            self.get_item_log_message(&item),
                        ]);
                    }
                    true
                } else {
                    false
                }
            }
            PlayerAction::ToggleEquip(i) => self.inventory.toggle_equip(i, &self.world_info),
            PlayerAction::Drop(i) => {
                if let Some(item) = self.inventory.remove(i) {
                    if let Some(item_on_ground) = self.tile_map[self.player_pos].item {
                        self.log_message(vec![
                            ("Swapped out ".to_owned(), Color::White),
                            self.get_item_log_message(&item),
                            (" for ".to_owned(), Color::White),
                            self.get_item_log_message(&item_on_ground),
                        ]);
                        self.inventory.add(item_on_ground);
                    } else {
                        self.log_message(vec![
                            ("Dropped ".to_owned(), Color::White),
                            self.get_item_log_message(&item),
                        ]);
                    }
                    self.tile_map[self.player_pos].item = Some(item);
                    true
                } else {
                    eprintln!("Bad drop idx: {i}");
                    false
                }
            }
            PlayerAction::Wait => true,
            PlayerAction::Craft(i, j) => {
                if i == j {
                    false
                } else if let Some(item1) = self.inventory.get(i) {
                    if let Some(item2) = self.inventory.get(j) {
                        if let Some(new_item) = self.world_info.craft(item1, item2) {
                            self.inventory.remove_all(vec![i, j]);
                            self.inventory.add(new_item);
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
        };
        self.inventory.sort(&self.world_info);
        if tick {
            self.tick();
        }
        tick
    }

    pub fn path(
        &mut self,
        start: Pos,
        end: Pos,
        maxdist: usize,
        through_walls: bool,
        around_mobs: bool,
    ) -> Option<Offset> {
        if start == end {
            return Some(Offset { x: 0, y: 0 });
        }
        let mut visited = HashSet::new();
        let mut periphery = Vec::new();
        let mut new_periphery = Vec::new();
        visited.insert(start);
        periphery.push(vec![start]);
        let mut closest_path: Option<Vec<_>> = None;
        let mut cardinals_shuffled = CARDINALS;
        cardinals_shuffled.shuffle(&mut self.rng);
        loop {
            if periphery.is_empty() || periphery[0].len() > maxdist {
                return if let Some(ref p) = closest_path {
                    if p.len() >= 2 {
                        Some(p[1] - p[0])
                    } else {
                        None
                    }
                } else {
                    None
                };
            }
            for path in periphery.drain(..) {
                let pos = *path.last().unwrap();
                let adjacent = pos
                    .adjacent_cardinal()
                    .into_iter()
                    .filter(|pos| !visited.contains(pos))
                    .filter(|pos| through_walls || self.tile_map[*pos].kind.is_walkable())
                    .filter(|pos| !around_mobs || !self.mobs.contains_key(pos))
                    .collect::<Vec<_>>();
                for pos in adjacent {
                    visited.insert(pos);
                    let mut new_path = path.clone();
                    new_path.push(pos);
                    if pos == end {
                        return Some(new_path[1] - new_path[0]);
                    }
                    match closest_path {
                        None => {
                            closest_path = Some(new_path.clone());
                        }
                        Some(ref mut p) => {
                            if (pos - end).mhn_dist() < (*p.last().unwrap() - end).mhn_dist() {
                                *p = new_path.clone();
                            }
                        }
                    }
                    new_periphery.push(new_path);
                }
            }
            std::mem::swap(&mut periphery, &mut new_periphery);
            new_periphery.clear();
        }
    }

    fn path_towards(
        &mut self,
        pos: Pos,
        target: Pos,
        through_walls: bool,
        around_mobs: bool,
        range: Option<usize>,
    ) -> Pos {
        let range = range.unwrap_or(FOV_RANGE as usize * 3);
        let off = self.path(pos, target, range, through_walls, around_mobs);
        if let Some(off) = off {
            let new_pos = pos + off;
            if !self.mobs.contains_key(&new_pos) {
                new_pos
            } else {
                pos
            }
        } else {
            pos
        }
    }

    pub fn get_visible_mobs(&self) -> Vec<Mob> {
        let fov = crate::fov::calculate_fov(self.player_pos, FOV_RANGE, self);
        let mut all_mobs: Vec<(i32, Pos, Mob)> = Vec::new();
        for pos in fov {
            if self.mobs.contains_key(&pos) {
                all_mobs.push((
                    (self.player_pos - pos).dist_squared(),
                    pos,
                    self.mobs[&pos].clone(),
                ));
            }
        }

        all_mobs.sort_by_key(|(dist_sq, pos, _)| (*dist_sq, pos.x, pos.y));
        all_mobs.iter().map(|(_, _, mob)| mob.clone()).collect()
    }

    pub fn tick(&mut self) {
        let poses = self.mobs.keys().copied().collect::<Vec<_>>();
        let fov = crate::fov::calculate_fov(self.player_pos, FOV_RANGE, self);
        for pos in poses {
            let mut mob = match self.mobs.remove(&pos) {
                Some(mob) => mob,
                None => continue,
            };
            let new_pos;
            if fov.contains(&pos) {
                if matches!(mob.ai, MobAi::Idle) {
                    let info = self.get_mobkind_info(mob.kind);
                    let mut seen_message = info.seen.clone();
                    if seen_message.ends_with("'") {
                        seen_message = format!("{}: {seen_message}", info.name);
                    }
                    self.log_message(vec![(seen_message, info.color)]);
                }
                mob.ai = MobAi::Move {
                    dest: self.player_pos,
                }
            }
            match mob.ai {
                MobAi::Idle => new_pos = pos,
                MobAi::Move { dest } => {
                    let target = self.path_towards(pos, dest, false, true, None);
                    if target == self.player_pos {
                        let mki = self.get_mobkind_info(mob.kind).clone();
                        let armor = self.inventory.get_equipped_armor_info(&self.world_info);
                        let defense1 = armor
                            .first()
                            .map(|eki| eki.ty)
                            .unwrap_or(PokemonType::Normal);
                        let defense2 = armor.get(1).map(|eki| eki.ty);
                        let eff = mki.attack_type.get_effectiveness2(defense1, defense2);
                        let mult = eff.get_scale();
                        let damage = mki.level * mult;

                        self.log_message(vec![
                            (mki.attack.clone(), mki.color),
                            (" You take ".into(), Color::White),
                            (format!("{}", damage), Color::Red),
                            (" damage!".into(), Color::White),
                        ]);

                        // See if armor is destroyed.
                        for destroyed_armor in self.inventory.damage_armor(&self.world_info) {
                            self.log_message(vec![
                                ("Your ".into(), Color::White),
                                (destroyed_armor.name.into(), destroyed_armor.ty.get_color()),
                                (" breaks!".into(), Color::Red),
                            ]);
                        }

                        self.player_damage += damage;
                        new_pos = pos;
                    } else {
                        new_pos = target;
                    }
                }
            }
            self.mobs.insert(new_pos, mob);
        }
    }

    pub fn get_player_pos(&self) -> Pos {
        self.player_pos
    }

    pub fn get_tile(&self, pos: grid::Pos) -> Tile {
        self.tile_map[pos]
    }

    pub fn get_mob(&self, pos: grid::Pos) -> Option<Mob> {
        self.mobs.get(&pos).cloned()
    }

    pub fn add_mob(&mut self, pos: grid::Pos, mob: Mob) {
        self.mobs.insert(pos, mob);
    }

    pub fn get_mobkind_info(&self, kind: MobKind) -> &MobKindInfo {
        self.world_info.get_mobkind_info(kind)
    }

    pub fn get_equipmentkind_info(&self, kind: EquipmentKind) -> &EquipmentKindInfo {
        self.world_info.get_equipmentkind_info(kind)
    }
}

pub struct Memory {
    pub tile_map: TileMap<Option<Tile>>,
    pub mobs: HashMap<Pos, Mob>,
}

impl Memory {
    pub fn new() -> Self {
        Self {
            tile_map: TileMap::new(None),
            mobs: HashMap::new(),
        }
    }
}

impl std::ops::Index<Pos> for World {
    type Output = Tile;

    fn index(&self, pos: Pos) -> &Tile {
        self.tile_map.index(pos)
    }
}

impl std::ops::IndexMut<Pos> for World {
    fn index_mut(&mut self, pos: Pos) -> &mut Tile {
        self.tile_map.index_mut(pos)
    }
}
