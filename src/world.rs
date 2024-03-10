use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;

use crate::grid::{self, Offset, Pos, TileMap, CARDINALS};
use crate::net::{
    Area, AttackEffectiveness, Color, IdeaGuy, ItemDefinition, ItemKind, MonsterDefinition,
    PokemonType,
};
use crate::render::{Animation, AnimationState, ShotAnimation};
use enum_map::{enum_map, Enum, EnumMap};
use lazy_static::lazy_static;
use rand::{seq::SliceRandom as _, SeedableRng};

pub const FOV_RANGE: i32 = 16;
pub const STARTING_DURABILITY: usize = 10;
pub const PLAYER_MAX_HEALTH: usize = 100;
pub const RELOAD_DELAY: usize = 2;

pub const PICK_UP_MESSAGES: [&str; 5] = [
    "You see here a ",
    "You step over a ",
    "You notice a ",
    "You find a ",
    "You discover a ",
];
pub const BREAK_VERBS: [&str; 5] = ["jams", "breaks", "shatters", "stops working", "crumbles"];

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

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct ItemInstance {
    pub info: Rc<ItemInfo>,
    pub item_durability: usize,
}

impl ItemInstance {
    pub fn new(info: Rc<ItemInfo>, item_durability: usize) -> ItemInstance {
        ItemInstance {
            info,
            item_durability,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Item {
    Instance(ItemInstance),
    PendingCraft(Rc<ItemInfo>, Rc<ItemInfo>),
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

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
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
    pub reload: usize,
    pub ai: MobAi,
}

#[derive(Hash, Debug, Clone)]
#[repr(u8)]
pub enum Speed {
    Slow = 1,
    Normal = 2,
    Fast = 3,
}

impl From<u8> for Speed {
    fn from(orig: u8) -> Self {
        match orig {
            1 => Speed::Slow,
            3 => Speed::Fast,
            _ => Speed::Normal,
        }
    }
}

impl Mob {
    pub fn new(kind: MobKind) -> Self {
        Self {
            kind,
            damage: 0,
            reload: RELOAD_DELAY,
            ai: MobAi::Idle,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ItemInfo {
    pub name: String,
    pub level: usize,
    pub ty: PokemonType,
    pub ty2: Option<PokemonType>,
    pub description: String,
    pub kind: ItemKind,
}

impl ItemInfo {
    pub fn get_range(&self) -> usize {
        match self.kind {
            ItemKind::RangedWeapon => 5 + self.level * 2,
            _ => 0,
        }
    }
    /// If ingested, how much does this heal?
    pub fn get_heal_amount(&self, armor_types: &[PokemonType]) -> i32 {
        use AttackEffectiveness::*;
        use PokemonType::Poison;
        let _heal_amount = 5 * self.level.pow(2);

        // Okay, I just think this is funny.
        let harm = if matches!(self.ty, Poison) {
            // Check if the player armor negates poison in any way.
            // If not, then negate the healing!
            println!("got here {:?}", self.ty);
            !armor_types.iter().any(|ty| {
                matches!(ty.get_effectiveness(Poison), Two | Four)
                    || matches!(Poison.get_effectiveness(*ty), Half | Quarter | Zero)
            })
        } else {
            false
        };

        let amt = 5 * 2i32.pow(self.level as u32);
        if harm {
            -amt
        } else {
            amt
        }
    }
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
    pub ranged: bool,
    pub speed: Speed,
}

impl MobKindInfo {
    pub fn max_hp(&self) -> usize {
        self.level * 16
    }
}

/// Contains post-processed content definitions parsed from AI-generated data.
#[derive(Debug, Clone)]
pub struct WorldInfo {
    pub areas: Vec<Area>,
    pub item_kinds: Vec<Rc<ItemInfo>>,
    pub monster_kinds: Vec<MobKindInfo>,
    pub monsters_per_level: Vec<Vec<MobKind>>,
    pub equipment_per_level: Vec<Vec<Rc<ItemInfo>>>,
    pub recipes: HashMap<(Rc<ItemInfo>, Rc<ItemInfo>), Rc<ItemInfo>>,
    pub pending_recipes: HashSet<(Rc<ItemInfo>, Rc<ItemInfo>)>,
    pub level_blurbs: Vec<String>,
}

impl WorldInfo {
    pub fn new() -> Self {
        Self {
            areas: Vec::new(),
            item_kinds: Vec::new(),
            monster_kinds: Vec::new(),
            monsters_per_level: Vec::new(),
            equipment_per_level: Vec::new(),
            pending_recipes: HashSet::new(),
            recipes: HashMap::new(),
            level_blurbs: Vec::new(),
        }
    }

    pub fn update(&mut self, ig: &mut IdeaGuy) {
        for i in self.areas.len()..ig.areas.as_ref().unwrap().len() {
            self.areas.push(ig.areas.as_ref().unwrap()[i].clone());
        }
        for item in ig.items.iter().flatten() {
            if self.item_kinds.iter().any(|e| e.name == item.name) {
                continue;
            }
            let ItemDefinition {
                name,
                level,
                ty,
                kind,
                description,
                ..
            } = item.clone();
            self.item_kinds.push(Rc::new(ItemInfo {
                name,
                level,
                ty,
                ty2: None,
                description,
                kind,
            }));
        }
        for mob in ig.monsters.iter().flatten() {
            if self.item_kinds.iter().any(|m| m.name == mob.name) {
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
                ranged,
                speed,
            } = mob.clone();
            let speed = speed.into();
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
                ranged,
                speed,
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

        let get_equipment_by_name =
            |name: &String| self.item_kinds.iter().find(|k| &k.name == name);

        self.equipment_per_level = ig
            .areas
            .iter()
            .flatten()
            .map(|area| {
                area.equipment
                    .iter()
                    .chain(area.melee_weapons.iter())
                    .chain(area.ranged_weapons.iter())
                    .chain(area.food.iter())
                    // NOTE: we may be missing some
                    .filter_map(get_equipment_by_name)
                    .cloned()
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
                self.item_kinds
                    .iter()
                    .find(|ek| ek.name == name)
                    .cloned()
                    .unwrap()
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
            self.pending_recipes.remove(&(a.clone(), b.clone()));
            let ig_a = ig_equip_by_name(&a.name);
            let ig_b = ig_equip_by_name(&b.name);
            ig.craft(ig_a, ig_b);
        }
    }

    pub fn get_mobkind_info(&self, kind: MobKind) -> &MobKindInfo {
        &self.monster_kinds[kind.0]
    }

    fn craft_inner(&mut self, ii1: Rc<ItemInfo>, ii2: Rc<ItemInfo>) -> Option<Item> {
        if let Some(ek3) = self.recipes.get(&(ii1.clone(), ii2.clone())) {
            Some(Item::Instance(ItemInstance {
                info: ek3.clone(),
                item_durability: STARTING_DURABILITY,
            }))
        } else {
            self.pending_recipes.insert((ii1.clone(), ii2.clone()));
            Some(Item::PendingCraft(ii1, ii2))
        }
    }

    pub fn craft(&mut self, item1: Item, item2: Item) -> Option<Item> {
        match (item1, item2) {
            (Item::Instance(ei1), Item::Instance(ei2)) => self.craft_inner(ei1.info, ei2.info),
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

    fn damage_weapon(&mut self, melee: bool) -> Option<Rc<ItemInfo>> {
        if let Some(player_weapon) = self.get_equipped_weapon(melee) {
            player_weapon.item_durability -= 1;
            if player_weapon.item_durability == 0 {
                let weapon_info = self.get_equipped_weapon_info(melee);
                self.remove(self.get_equipped_weapon_slot(melee).unwrap());
                weapon_info
            } else {
                None
            }
        } else {
            None
        }
    }

    fn damage_armor(&mut self) -> Vec<Rc<ItemInfo>> {
        let mut deleted = vec![];
        for player_armor in self.get_equipped_armor().into_iter() {
            player_armor.item_durability -= 1;
            if player_armor.item_durability == 0 {
                deleted.push(player_armor.info.clone());
            }
        }
        self.items.retain(|x| {
            if let Item::Instance(ref ii) = x.item {
                ii.item_durability > 0
            } else {
                true
            }
        });

        deleted
    }

    fn get_equipped_weapon(&mut self, melee: bool) -> Option<&mut ItemInstance> {
        self.items
            .iter_mut()
            .filter(|x| x.equipped)
            .filter_map(|x| match x.item {
                Item::PendingCraft(_, _) => None,
                Item::Instance(ref mut i) => Some(i),
            })
            .find(|i| match i.info.kind {
                ItemKind::MeleeWeapon => melee,
                ItemKind::RangedWeapon => !melee,
                _ => false,
            })
    }

    fn get_equipped_weapon_info(&self, melee: bool) -> Option<Rc<ItemInfo>> {
        self.items
            .iter()
            .filter(|x| x.equipped)
            .filter_map(|x| match x.item {
                Item::Instance(ref ek) => Some(ek),
                _ => None,
            })
            .map(|x| &x.info)
            .find(|info| match info.kind {
                ItemKind::MeleeWeapon => melee,
                ItemKind::RangedWeapon => !melee,
                _ => false,
            })
            .cloned()
    }

    fn get_equipped_weapon_slot(&self, melee: bool) -> Option<usize> {
        self.items
            .iter()
            .enumerate()
            .filter(|(_, x)| x.equipped)
            .filter_map(|(i, x)| match x.item {
                Item::PendingCraft(_, _) => None,
                Item::Instance(ref ek) => Some((i, ek)),
            })
            .find(|(_, eki)| match eki.info.kind {
                ItemKind::MeleeWeapon => melee,
                ItemKind::RangedWeapon => !melee,
                _ => false,
            })
            .map(|(i, _)| i)
    }

    fn get_equipped_armor(&mut self) -> Vec<&mut ItemInstance> {
        self.items
            .iter_mut()
            .filter(|x| x.equipped)
            .filter_map(|x| match x.item {
                Item::PendingCraft(_, _) => None,
                Item::Instance(ref mut i) => Some(i),
            })
            .filter(|i| i.info.kind == ItemKind::Armor)
            .collect()
    }

    fn get_equipped_armor_info(&self) -> Vec<Rc<ItemInfo>> {
        self.items
            .iter()
            .filter(|x| x.equipped)
            .filter_map(|x| match x.item {
                Item::Instance(ref ek) => Some(ek),
                _ => None,
            })
            .filter(|eki| eki.info.kind == ItemKind::Armor)
            .map(|item| item.info.clone())
            .collect()
    }

    fn get_equipped_armor_slots(&self) -> Vec<usize> {
        self.items
            .iter()
            .enumerate()
            .filter(|(_, x)| x.equipped)
            .filter_map(|(i, x)| match x.item {
                Item::PendingCraft(_, _) => None,
                Item::Instance(ref ek) => Some((i, ek)),
            })
            .filter(|(_, eki)| eki.info.kind == ItemKind::Armor)
            .map(|(i, _)| i)
            .collect()
    }

    fn sort(&mut self) {
        self.items.sort_by_key(|x| match x {
            InventoryItem {
                item: Item::Instance(ek),
                equipped,
            } => match (equipped, ek.info.kind) {
                (true, ItemKind::MeleeWeapon) => 1,
                (true, ItemKind::RangedWeapon) => 2,
                (true, ItemKind::Armor) => 3,
                (false, ItemKind::MeleeWeapon) => 4,
                (false, ItemKind::RangedWeapon) => 5,
                (false, ItemKind::Armor) => 6,
                (_, ItemKind::Food) => 7,
            },
            InventoryItem {
                item: Item::PendingCraft(..),
                ..
            } => 5,
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
        self.items.get(i).map(|x| x.item.clone())
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
    fn toggle_equip(&mut self, i: usize) -> bool {
        if i >= self.items.len() {
            eprintln!("Bad equip idx: {i}");
            false
        } else if self.items[i].equipped {
            self.items[i].equipped = false;
            true
        } else if let Item::Instance(ref ii) = self.items[i].item {
            // Unequip another item if that slot is full.
            let max_per_slot = |slot: ItemKind| match slot {
                ItemKind::MeleeWeapon => 1,
                ItemKind::RangedWeapon => 1,
                ItemKind::Armor => 2,
                ItemKind::Food => 0,
            };
            let max = max_per_slot(ii.info.kind);
            if max == 0 {
                return false;
            }
            let other_equipped_in_slot = self
                .items
                .iter()
                .enumerate()
                .filter(|(_i, x)| x.equipped)
                .filter_map(|(i, x)| {
                    if let Item::Instance(ref ii) = x.item {
                        Some((i, ii))
                    } else {
                        None
                    }
                })
                .filter(|(_i, other_ii)| other_ii.info.kind == ii.info.kind)
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
    pub untriggered_animations: Vec<AnimationState>,
    stairs: HashMap<Pos, Pos>,
    level_id: usize,
    rng: rand::rngs::SmallRng,
}

pub enum PlayerAction {
    Move(Offset),
    Fire(Offset),
    PickUp,
    Use(usize),
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
            untriggered_animations: Vec::new(),
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
            if let Item::PendingCraft(a, b) = item.item.clone() {
                if let Some(c) = self.world_info.recipes.get(&(a, b)) {
                    item.item = Item::Instance(ItemInstance::new(c.clone(), STARTING_DURABILITY));
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
            Item::Instance(item) => (item.info.name.clone(), item.info.ty.get_color()),
            Item::PendingCraft(..) => ("???".to_string(), Color::Pink),
        }
    }

    fn damage_mob(&mut self, mut mob: Mob, mob_pos: Pos, damage: usize, eff: AttackEffectiveness) {
        let mki = self.get_mobkind_info(mob.kind).clone();
        mob.damage += damage;

        let mut msg = vec![
            ("You hit ".into(), Color::White),
            (mki.name.clone(), mki.color),
            (" for ".into(), Color::White),
            (format!("{}", damage), Color::Red),
        ];
        match eff {
            AttackEffectiveness::Zero => msg.push((" It had no effect!".into(), Color::Red)),
            AttackEffectiveness::Quarter | AttackEffectiveness::Half => {
                msg.push((" It's not very effective...".into(), Color::Red))
            }
            AttackEffectiveness::Two | AttackEffectiveness::Four => {
                msg.push((" It's super effective!".into(), Color::Gold))
            }
            _ => {}
        };
        self.log_message(msg);
        if mob.damage >= mki.max_hp() {
            self.log_message(vec![(mki.death, mki.color)]);
        } else {
            self.mobs.insert(mob_pos, mob);
        }
    }

    pub fn do_player_action(&mut self, action: PlayerAction) -> bool {
        if self.player_is_dead() {
            return false;
        }
        let tick = match action {
            PlayerAction::Move(offset) => {
                assert!(offset.mhn_dist() == 1);
                let new_pos = self.player_pos + offset;
                if let Some(mob) = self.mobs.remove(&new_pos) {
                    let mki = self.get_mobkind_info(mob.kind).clone();
                    let player_weapon_info = self.inventory.get_equipped_weapon_info(true);
                    let (att_type, att_level) = player_weapon_info
                        .clone()
                        .map(|w| (w.ty, w.level))
                        .unwrap_or((PokemonType::Normal, 0));
                    let eff = att_type.get_effectiveness2(mki.type1, mki.type2);
                    let mult = eff.get_scale();
                    let damage = (att_level + 1) * mult;

                    self.damage_mob(mob, new_pos, damage, eff);

                    if let Some(destroyed_weapon) = self.inventory.damage_weapon(true) {
                        self.log_message(vec![
                            ("Your ".into(), Color::White),
                            (
                                destroyed_weapon.name.clone(),
                                destroyed_weapon.ty.get_color(),
                            ),
                            (" breaks!".into(), Color::Red),
                        ]);
                    }

                    true
                } else if self.tile_map[new_pos].kind.is_walkable() {
                    // Check if player walks over an item.
                    if let Some(ref item) = self.tile_map[new_pos].item {
                        let msg = vec![
                            (
                                PICK_UP_MESSAGES
                                    .choose(&mut self.rng)
                                    .unwrap()
                                    .to_owned()
                                    .to_owned(),
                                Color::White,
                            ),
                            self.get_item_log_message(item),
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
            PlayerAction::Fire(direction) => {
                assert!(direction.mhn_dist() == 1);
                if let Some(pwi) = self.inventory.get_equipped_weapon_info(false) {
                    let range = pwi.get_range() as i32;
                    let start_pos = self.player_pos;
                    let end_pos = self.player_pos + direction * range;
                    let mut zapped_tiles = Vec::new();
                    for (x, y) in line_drawing::Bresenham::new(
                        (start_pos.x, start_pos.y),
                        (end_pos.x, end_pos.y),
                    )
                    .skip(0)
                    {
                        let zapped_pos = Pos::new(x, y);
                        if let Some(mob) = self.mobs.remove(&zapped_pos) {
                            let mki = self.get_mobkind_info(mob.kind).clone();
                            let (att_type, att_level) = (pwi.ty, pwi.level);
                            let eff = att_type.get_effectiveness2(mki.type1, mki.type2);
                            let mult = eff.get_scale() / 2;
                            let damage = (att_level + 1) * mult;
                            self.damage_mob(mob, zapped_pos, damage, eff);
                        }
                        zapped_tiles.push(zapped_pos);
                    }

                    self.untriggered_animations.push(AnimationState::new(
                        Animation::Shot(ShotAnimation {
                            cells: zapped_tiles,
                            color: pwi.ty.get_color(),
                        }),
                        0.5,
                    ));

                    // Add some damage to the weapon.
                    if let Some(destroyed_weapon) = self.inventory.damage_weapon(false) {
                        let breaks = BREAK_VERBS.choose(&mut self.rng).unwrap().to_owned();
                        self.log_message(vec![
                            ("Your ".into(), Color::White),
                            (
                                destroyed_weapon.name.clone(),
                                destroyed_weapon.ty.get_color(),
                            ),
                            (format!(" runs out of ammo and {breaks}!"), Color::Red),
                        ]);
                    }
                    true
                } else {
                    self.log_message(vec![(
                        "You cannot fire because you do not have a ranged weapon equipped!".into(),
                        Color::White,
                    )]);
                    false
                }
            }
            PlayerAction::PickUp => {
                if let Some(item) = self.tile_map[self.player_pos].item.take() {
                    if let Some(popped) = self.inventory.add(item.clone()) {
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
            PlayerAction::Use(i) => {
                if let Some(Item::Instance(ii)) = self.inventory.get(i) {
                    use ItemKind::*;
                    match ii.info.kind {
                        Armor | MeleeWeapon | RangedWeapon => self.inventory.toggle_equip(i),
                        Food => {
                            self.inventory.remove(i).unwrap();
                            let armor_types = self
                                .inventory
                                .get_equipped_armor_info()
                                .iter()
                                .map(|a| a.ty)
                                .collect::<Vec<_>>();
                            let heal_amt = ii.info.get_heal_amount(&armor_types);
                            if heal_amt < 0 {
                                self.player_damage =
                                    self.player_damage.saturating_sub(-heal_amt as usize);
                                self.log_message(vec![(
                                    format!(
                                        "You eat a poisonous {} and lose {heal_amt} HP! Ouch!",
                                        ii.info.name
                                    ),
                                    Color::Green,
                                )]);
                            } else {
                                self.player_damage =
                                    self.player_damage.saturating_sub(heal_amt as usize);
                                self.log_message(vec![(
                                    format!("You eat a {} and gain {heal_amt} HP!", ii.info.name),
                                    Color::Green,
                                )]);
                            }
                            true
                        }
                    }
                } else {
                    false
                }
            }
            PlayerAction::Drop(i) => {
                if let Some(item) = self.inventory.remove(i) {
                    if let Some(item_on_ground) = self.tile_map[self.player_pos].item.clone() {
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
                if matches!(mob.ai, MobAi::Idle) {
                    let info = self.get_mobkind_info(mob.kind);
                    let mut seen_message = info.seen.clone();
                    if seen_message.ends_with('\'') {
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
                    // Start by determining the next position we want to move towards.
                    let target = self.path_towards(pos, dest, false, true, None);

                    let mki = self.get_mobkind_info(mob.kind).clone();
                    let armor = self.inventory.get_equipped_armor_info();
                    let defense1 = armor
                        .first()
                        .map(|eki| eki.ty)
                        .unwrap_or(PokemonType::Normal);
                    let defense2 = armor.get(1).map(|eki| eki.ty);
                    let eff = mki.attack_type.get_effectiveness2(defense1, defense2);
                    let mult = eff.get_scale();
                    let mut damage = mki.level * mult;
                    if mki.ranged {
                        damage /= 2;
                    }
                    let range = (5 + mki.level * 2) as i32;
                    let in_range = (pos - self.player_pos).dist_squared() <= range * range;
                    if mki.ranged {
                        println!("(0) {} in range -- {in_range}", mki.name);
                    }

                    // If ranged and in range and reload cooldown done
                    let mut can_fire = mki.ranged && in_range && mob.reload == 0;
                    let fire_line: Vec<_> = line_drawing::Bresenham::new(
                        (target.x, target.y),
                        (self.player_pos.x, self.player_pos.y),
                    )
                    .map(|(x, y)| Pos::new(x, y))
                    .collect();

                    // If we can't see it, also avoid it. Or if there's friendly fire.
                    can_fire &= fov.contains(&pos);
                    can_fire &= fire_line.iter().any(|&pos| self.mobs.contains_key(&pos));
                    // If melee and adjacent, then let fire.
                    can_fire |= !mki.ranged && target == self.player_pos;

                    if can_fire {
                        self.log_message(vec![
                            (mki.attack.clone(), mki.color),
                            (" You take ".into(), Color::White),
                            (format!("{}", damage), Color::Red),
                            (" damage!".into(), Color::White),
                        ]);

                        // See if armor is destroyed.
                        for destroyed_armor in self.inventory.damage_armor() {
                            self.log_message(vec![
                                ("Your ".into(), Color::White),
                                (destroyed_armor.name.clone(), destroyed_armor.ty.get_color()),
                                (" breaks!".into(), Color::Red),
                            ]);
                        }

                        if mki.ranged {
                            self.untriggered_animations.push(AnimationState::new(
                                Animation::Shot(ShotAnimation {
                                    cells: fire_line,
                                    color: mki.attack_type.get_color(),
                                }),
                                0.5,
                            ));
                            mob.reload = RELOAD_DELAY;
                        }

                        self.player_damage += damage;
                    }

                    if target == self.player_pos {
                        new_pos = pos;
                    } else {
                        new_pos = target;
                    }
                }
            }
            if mob.reload != 0 {
                mob.reload -= 1;
            }
            self.mobs.insert(new_pos, mob);
        }
        if self.player_is_dead() {
            self.log_message(vec![("YOU DIED".into(), Color::Red)]);
        }
    }

    pub fn player_is_dead(&self) -> bool {
        self.player_damage >= PLAYER_MAX_HEALTH
    }

    pub fn get_player_pos(&self) -> Pos {
        self.player_pos
    }

    pub fn get_tile(&self, pos: grid::Pos) -> Tile {
        self.tile_map[pos].clone()
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
