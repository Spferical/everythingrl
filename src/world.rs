use std::collections::{HashMap, HashSet};

use crate::grid::{self, Offset, Pos, Rect, TileMap, CARDINALS};
use enum_map::{enum_map, Enum, EnumMap};
use lazy_static::lazy_static;
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
}

#[derive(Enum, PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum EquipmentKind {
    Sword,
}

impl EquipmentKind {
    fn get_slot(&self) -> EquipmentSlot {
        match self {
            EquipmentKind::Sword => EquipmentSlot::Weapon,
        }
    }
}

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

#[derive(Enum, PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum MobKind {
    Cat,
    Alien,
}

pub fn get_glyph(kind: MobKind) -> (char, macroquad::color::Color) {
    match kind {
        MobKind::Cat => ('c', macroquad::color::YELLOW),
        MobKind::Alien => ('a', macroquad::color::PURPLE),
    }
}

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

#[derive(Clone)]
pub struct World {
    player_pos: Pos,
    tile_map: TileMap<Tile>,
    pub mobs: HashMap<Pos, Mob>,
    pub inventory: Vec<Item>,
    equipment: Vec<Item>,
    rng: rand::rngs::SmallRng,
}

pub enum PlayerAction {
    Move(Offset),
    PickUp,
    Equip(usize),
    Unequip(usize),
    Drop(usize),
}

impl World {
    pub fn new() -> Self {
        let tile_map = TileMap::new(Tile {
            kind: TileKind::Wall,
            item: None,
        });
        let player_pos = Pos { x: 0, y: 0 };
        let mobs = HashMap::new();
        let rng = rand::rngs::SmallRng::seed_from_u64(72);
        let inventory = vec![];
        let equipment = vec![];
        Self {
            player_pos,
            tile_map,
            mobs,
            rng,
            inventory,
            equipment,
        }
    }

    pub fn do_player_action(&mut self, action: PlayerAction) -> bool {
        let tick = match action {
            PlayerAction::Move(offset) => {
                assert!(offset.mhn_dist() == 1);
                let new_pos = self.player_pos + offset;
                if let Some(mob) = self.mobs.remove(&new_pos) {
                    // TODO: more advanced combat
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
                    self.inventory.push(item);
                    true
                } else {
                    false
                }
            }
            PlayerAction::Equip(i) => {
                if i >= self.inventory.len() {
                    eprintln!("Bad equip idx: {i}");
                    false
                } else {
                    let item = self.inventory.remove(i);
                    // unequip anything in the same slot
                    if let Some(equipped_index) = self.equipment.iter().position(|x| match x {
                        Item::Equipment(ek) => ek.get_slot() == ek.get_slot(),
                        _ => false,
                    }) {
                        self.inventory.push(self.equipment.remove(equipped_index));
                    }
                    self.equipment.push(item);
                    true
                }
            }
            PlayerAction::Unequip(i) => {
                if i >= self.equipment.len() {
                    eprintln!("Bad unequip idx: {i}");
                    false
                } else {
                    let item = self.equipment.remove(i);
                    self.inventory.push(item);
                    true
                }
            }
            PlayerAction::Drop(i) => {
                if i >= self.inventory.len() {
                    eprintln!("Bad drop idx: {i}");
                    false
                } else {
                    let item = self.inventory.remove(i);
                    if let Some(item_on_ground) = self.tile_map[self.player_pos].item {
                        self.inventory.push(item_on_ground)
                    }
                    self.tile_map[self.player_pos].item = Some(item);
                    true
                }
            }
        };
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
        let mut all_mobs: Vec<(i32, Mob)> = Vec::new();
        for pos in fov {
            if self.mobs.contains_key(&pos) {
                all_mobs.push(
                    ((self.player_pos - pos).dist_squared(),
                    self.mobs[&pos].clone())
                );
            }
        }

        all_mobs.sort_by_key(|(dist_sq, _)| *dist_sq);
        all_mobs.iter().map(|(_, mob)| mob.clone()).collect()
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
