use std::collections::{HashMap, HashSet, VecDeque};

use crate::grid::{self, Offset, Pos, TileMap, CARDINALS};
use crate::net::{Color, IdeaGuy, ItemDefinition, MonsterDefinition, PokemonType};
use enum_map::{enum_map, Enum, EnumMap};
use lazy_static::lazy_static;
use rand::Rng;
use rand::{seq::SliceRandom as _, SeedableRng};

pub const FOV_RANGE: i32 = 16;

#[derive(Enum, PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum TileKind {
    Floor,
    Wall,
    YellowFloor,
    YellowWall,
    BloodyFloor,
}

impl TileKind {
    pub fn is_opaque(self) -> bool {
        TILE_INFOS[self].opaque
    }

    pub fn is_walkable(self) -> bool {
        TILE_INFOS[self].walkable
    }
}

#[derive(Enum, PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum EquipmentSlot {
    Weapon,
    Armor,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub struct EquipmentKind(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Item {
    Corpse(MobKind),
    Equipment(EquipmentKind),
}

pub struct TileKindInfo {
    pub opaque: bool,
    pub walkable: bool,
}

lazy_static! {
    pub static ref TILE_INFOS: EnumMap<TileKind, TileKindInfo> = enum_map! {
        TileKind::Floor | TileKind::YellowFloor | TileKind::BloodyFloor => TileKindInfo {
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
    pub color: Color,
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
}

impl MobKindInfo {
    fn max_hp(&self) -> usize {
        self.level * 16
    }

    fn base_damage(&self) -> usize {
        self.level
    }
}

/// Contains post-processed content definitions parsed from AI-generated data.
#[derive(Debug, Clone)]
pub struct WorldInfo {
    equip_kinds: Vec<EquipmentKindInfo>,
    monster_kinds: Vec<MobKindInfo>,
}

impl WorldInfo {
    pub fn new() -> Self {
        Self {
            equip_kinds: Vec::new(),
            monster_kinds: Vec::new(),
        }
    }

    pub fn update(&mut self, ig: &mut IdeaGuy) {
        let weapon_names = ig
            .areas
            .iter()
            .flatten()
            .flat_map(|area| area.melee_weapons.clone())
            .collect::<HashSet<String>>();
        for item in ig.items.iter().flatten() {
            let slot = if weapon_names.contains(&item.name) {
                EquipmentSlot::Weapon
            } else {
                EquipmentSlot::Armor
            };
            if self.equip_kinds.iter().any(|e| e.name == item.name) {
                continue;
            }
            let ItemDefinition {
                name,
                level,
                color,
                ty,
                description,
            } = item.clone();
            self.equip_kinds.push(EquipmentKindInfo {
                name,
                level,
                color,
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
            });
        }
    }

    pub fn get_random_item_kind(&self, rng: &mut impl Rng) -> EquipmentKind {
        EquipmentKind(rng.gen_range(0..self.equip_kinds.len()))
    }

    pub fn get_random_mob_kind(&self, rng: &mut impl Rng) -> MobKind {
        MobKind(rng.gen_range(0..self.monster_kinds.len()))
    }

    pub fn get_equipmentkind_info(&self, kind: EquipmentKind) -> &EquipmentKindInfo {
        &self.equip_kinds[kind.0]
    }

    pub fn get_mobkind_info(&self, kind: MobKind) -> &MobKindInfo {
        &self.monster_kinds[kind.0]
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
    fn new() -> Self {
        Self { items: vec![] }
    }

    fn get_equipped_weapon_info(&self, wi: &WorldInfo) -> Option<EquipmentKindInfo> {
        self.items
            .iter()
            .filter(|x| x.equipped)
            .filter_map(|x| match x.item {
                Item::Corpse(_) => None,
                Item::Equipment(ek) => Some(ek),
            })
            .map(|ek| wi.get_equipmentkind_info(ek))
            .filter(|eki| eki.slot == EquipmentSlot::Weapon)
            .next()
            .cloned()
    }
    fn get_equipped_armor_info(&self, wi: &WorldInfo) -> Vec<EquipmentKindInfo> {
        self.items
            .iter()
            .filter(|x| x.equipped)
            .filter_map(|x| match x.item {
                Item::Corpse(_) => None,
                Item::Equipment(ek) => Some(ek),
            })
            .map(|ek| wi.get_equipmentkind_info(ek))
            .filter(|eki| eki.slot == EquipmentSlot::Armor)
            .cloned()
            .collect()
    }

    fn sort(&mut self) {
        // Equipped to the top, corpses to the bottom.
        self.items
            .sort_by_key(|x| (!x.equipped, matches!(x.item, Item::Corpse(_))))
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
            let ek_def = wi.get_equipmentkind_info(ek);

            // Unequip another item if that slot is full.
            let max_per_slot = |slot: EquipmentSlot| match slot {
                EquipmentSlot::Weapon => 1,
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
                        Some((i, wi.get_equipmentkind_info(ek)))
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
    world_info: WorldInfo,
    pub mobs: HashMap<Pos, Mob>,
    pub inventory: Inventory,
    pub log: VecDeque<(String, macroquad::color::Color)>,
    rng: rand::rngs::SmallRng,
}

pub enum PlayerAction {
    Move(Offset),
    PickUp,
    ToggleEquip(usize),
    Drop(usize),
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
        }
    }

    pub fn update_defs(&mut self, ig: &mut IdeaGuy) {
        self.world_info.update(ig);
    }

    pub fn get_random_equipment_kind(&self, rng: &mut impl Rng) -> EquipmentKind {
        self.world_info.get_random_item_kind(rng)
    }

    pub fn get_random_mob_kind(&self, rng: &mut impl Rng) -> MobKind {
        self.world_info.get_random_mob_kind(rng)
    }

    pub fn log_message(&mut self, text: String, color: macroquad::color::Color) {
        self.log.push_back((text.into(), color));
    }

    pub fn do_player_action(&mut self, action: PlayerAction) -> bool {
        let tick = match action {
            PlayerAction::Move(offset) => {
                assert!(offset.mhn_dist() == 1);
                let new_pos = self.player_pos + offset;
                if let Some(mut mob) = self.mobs.remove(&new_pos) {
                    let mki = self.get_mobkind_info(mob.kind).clone();
                    let player_weapon_info =
                        self.inventory.get_equipped_weapon_info(&self.world_info);
                    let (att_type, att_level) = player_weapon_info
                        .map(|w| (w.ty, w.level))
                        .unwrap_or((PokemonType::Normal, 0));
                    let eff = att_type.get_effectiveness2(mki.type1, mki.type2);
                    let mult = eff.get_scale();
                    let damage = (att_level + 1) * mult;
                    mob.damage += damage;
                    self.log_message(
                        format!("You hit {} for {}!", mki.name, damage),
                        macroquad::color::RED,
                    );
                    if mob.damage >= mki.max_hp() {
                        self.log_message(format!("{} dies!", mki.name), macroquad::color::RED);
                        self.tile_map[new_pos].item = Some(Item::Corpse(mob.kind));
                    } else {
                        self.mobs.insert(new_pos, mob);
                    }
                    true
                } else if self.tile_map[new_pos].kind.is_walkable() {
                    self.player_pos += offset;
                    true
                } else {
                    false
                }
            }
            PlayerAction::PickUp => {
                if let Some(item) = self.tile_map[self.player_pos].item.take() {
                    if let Some(popped) = self.inventory.add(item) {
                        self.tile_map[self.player_pos].item = Some(popped);
                    }
                    true
                } else {
                    false
                }
            }
            PlayerAction::ToggleEquip(i) => self.inventory.toggle_equip(i, &self.world_info),
            PlayerAction::Drop(i) => {
                if let Some(item) = self.inventory.remove(i) {
                    self.tile_map[self.player_pos].item = Some(item);
                    if let Some(item_on_ground) = self.tile_map[self.player_pos].item {
                        self.inventory.add(item_on_ground);
                    }
                    true
                } else {
                    eprintln!("Bad drop idx: {i}");
                    false
                }
            }
            PlayerAction::Wait => true,
        };
        self.inventory.sort();
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
                mob.ai = MobAi::Move {
                    dest: self.player_pos,
                }
            }
            match mob.ai {
                MobAi::Idle => new_pos = pos,
                MobAi::Move { dest } => {
                    let target = self.path_towards(pos, dest, false, true, None);
                    if target == self.player_pos {
                        // TODO: combat
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
