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
    Equipment,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub struct EquipmentKind(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Item {
    Corpse(MobKind),
    Equipment(EquipmentKind),
}

pub struct EquipmentDefinition {
    pub name: String,
    pub level: usize,
    pub color: Color,
    pub ty: PokemonType,
    pub slot: EquipmentSlot,
    pub description: String,
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
    pub damage: u32,
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

enum AttackEffectiveness {
    Zero,
    Quarter,
    Half,
    One,
    Two,
    Four,
}

fn pokemon_effectiveness1(attack: PokemonType, defense: PokemonType) -> AttackEffectiveness {
    use AttackEffectiveness::*;
    use PokemonType::*;
    match (attack, defense) {
        (Normal, Rock | Steel) => Half,
        (Normal, Ghost) => Zero,

        (Fire, Fire | Water | Rock | Dragon) => Half,
        (Fire, Grass | Ice | Bug | Steel) => Two,

        (Water, Water | Grass | Dragon) => Half,
        (Water, Fire | Ground | Rock) => Two,

        (Electric, Water | Flying) => Two,
        (Electric, Electric | Grass) => Half,
        (Electric, Ground) => Zero,

        (Grass, Water | Ground | Rock) => Two,
        (Grass, Fire | Grass | Poison | Flying | Bug | Dragon | Steel) => Half,

        (Ice, Grass | Ground | Flying | Dragon) => Two,
        (Ice, Fire | Water | Ice | Steel) => Half,

        (Fighting, Ice | Rock | Normal | Dark | Steel) => Two,
        (Fighting, Flying | Poison | Bug | Psychic | Fairy) => Half,
        (Fighting, Ghost) => Zero,

        (Poison, Grass | Fairy) => Two,
        (Poison, Poison | Ground | Rock | Ghost) => Half,
        (Poison, Steel) => Zero,

        (Ground, Fire | Electric | Poison | Rock | Steel) => Two,
        (Ground, Grass | Bug) => Half,
        (Ground, Flying) => Zero,

        (Flying, Grass | Fighting | Bug) => Two,
        (Flying, Electric | Rock | Steel) => Half,

        (Psychic, Fighting | Poison) => Two,
        (Psychic, Psychic | Steel) => Half,
        (Psychic, Dark) => Zero,

        (Bug, Grass | Psychic | Dark) => Two,
        (Bug, Fire | Fighting | Poison | Flying | Ghost | Steel | Fairy) => Half,

        (Rock, Fire | Ice | Flying | Bug) => Two,
        (Rock, Fighting | Ground | Steel) => Half,

        (Ghost, Psychic | Ghost) => Two,
        (Ghost, Dark) => Half,
        (Ghost, Normal) => Zero,

        (Dragon, Dragon) => Two,
        (Dragon, Steel) => Half,
        (Dragon, Fairy) => Zero,

        (Dark, Psychic | Ghost) => Two,
        (Dark, Fighting | Dark | Fairy) => Half,

        (Steel, Ice | Rock | Fairy) => Two,
        (Steel, Fire | Water | Electric | Steel) => Half,

        (Fairy, Fighting | Dragon | Dark) => Two,
        (Fairy, Fire | Poison | Steel) => Half,

        _ => One,
    }
}
fn multiply_effectiveness(
    eff1: AttackEffectiveness,
    eff2: AttackEffectiveness,
) -> AttackEffectiveness {
    use AttackEffectiveness::*;
    match (eff1, eff2) {
        (Zero, _) | (_, Zero) => Zero,
        (Half, Half) => Quarter,
        (Half, Two) | (Two, Half) => One,
        (Two, Two) => Four,
        (eff1, One) => eff1,
        (One, eff2) => eff2,
        _ => One,
    }
}

fn pokemon_effectiveness2(
    attack: PokemonType,
    defense1: PokemonType,
    defense2: Option<PokemonType>,
) -> AttackEffectiveness {
    use AttackEffectiveness::*;
    let eff1 = pokemon_effectiveness1(attack, defense1);
    let eff2 = defense2.map(|defense2| pokemon_effectiveness1(attack, defense2));
    multiply_effectiveness(eff1, eff2.unwrap_or(One))
}

#[derive(Debug, Clone)]
pub struct InventorySlot {
    pub item: Item,
    pub equipped: bool,
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

/// Contains post-processed content definitions parsed from AI-generated data.
#[derive(Debug, Clone)]
pub struct WorldInfo {
    equip_kinds: Vec<EquipmentKindInfo>,
    monster_kinds: Vec<MonsterDefinition>,
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
                EquipmentSlot::Equipment
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
            self.monster_kinds.push(mob.clone());
        }
    }

    pub fn get_random_item_kind(&self, rng: &mut impl Rng) -> EquipmentKind {
        EquipmentKind(rng.gen_range(0..self.equip_kinds.len()))
    }

    pub fn get_random_mob_kind(&self, rng: &mut impl Rng) -> MobKind {
        MobKind(rng.gen_range(0..self.monster_kinds.len()))
    }
}

#[derive(Clone)]
pub struct World {
    pub player_pos: Pos,
    pub player_damage: usize,
    tile_map: TileMap<Tile>,
    world_info: WorldInfo,
    pub mobs: HashMap<Pos, Mob>,
    pub inventory: Vec<InventorySlot>,
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
            inventory: vec![],
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

    pub fn log_message(&mut self, text: &str, color: macroquad::color::Color) {
        self.log.push_back((text.into(), color));
    }

    pub fn sort_inventory(&mut self) {
        self.inventory
            // Equipped to the top, corpses to the bottom.
            .sort_by_key(|x| (!x.equipped, matches!(x.item, Item::Corpse(_))))
    }

    pub fn do_player_action(&mut self, action: PlayerAction) -> bool {
        let tick = match action {
            PlayerAction::Move(offset) => {
                assert!(offset.mhn_dist() == 1);
                let new_pos = self.player_pos + offset;
                if let Some(mob) = self.mobs.remove(&new_pos) {
                    // TODO: more advanced combat
                    self.log_message("death has happened", macroquad::color::RED);
                    self.tile_map[new_pos].item = Some(Item::Corpse(mob.kind));
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
                    self.inventory.push(InventorySlot {
                        item,
                        equipped: false,
                    });
                    if self.inventory.len() > 9 {
                        for i in 0..self.inventory.len() {
                            if !self.inventory[i].equipped {
                                self.tile_map[self.player_pos].item =
                                    Some(self.inventory.remove(i).item);
                                break;
                            }
                        }
                    }
                    true
                } else {
                    false
                }
            }
            PlayerAction::ToggleEquip(i) => {
                if i >= self.inventory.len() {
                    eprintln!("Bad equip idx: {i}");
                    false
                } else if self.inventory[i].equipped {
                    self.inventory[i].equipped = false;
                    true
                } else if matches!(self.inventory[i].item, Item::Equipment(_)) {
                    let num_equipped = self.inventory.iter().filter(|x| x.equipped).count();
                    if num_equipped >= 2 {
                        for j in 0..self.inventory.len() {
                            if self.inventory[j].equipped {
                                self.inventory[j].equipped = false;
                                break;
                            }
                        }
                    }
                    self.inventory[i].equipped = true;
                    true
                } else {
                    eprintln!("Item is not equippable");
                    false
                }
            }
            PlayerAction::Drop(i) => {
                if i >= self.inventory.len() {
                    eprintln!("Bad drop idx: {i}");
                    false
                } else {
                    let slot = self.inventory.remove(i);
                    if let Some(item_on_ground) = self.tile_map[self.player_pos].item {
                        self.inventory.push(InventorySlot {
                            item: item_on_ground,
                            equipped: false,
                        })
                    }
                    self.tile_map[self.player_pos].item = Some(slot.item);
                    true
                }
            }
            PlayerAction::Wait => true,
        };
        self.sort_inventory();
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

    pub fn get_mobkind_info(&self, kind: MobKind) -> MonsterDefinition {
        self.world_info.monster_kinds[kind.0].clone()
    }

    pub fn get_equipmentkind_info(&self, kind: EquipmentKind) -> EquipmentKindInfo {
        self.world_info.equip_kinds[kind.0].clone()
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
