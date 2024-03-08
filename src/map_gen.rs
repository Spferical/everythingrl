#![allow(dead_code)]
use std::collections::{HashMap, HashSet};

use rand::rngs::SmallRng;
use rand::Rng;
use rand::{seq::SliceRandom, SeedableRng};

use crate::grid::{Offset, Pos, Rect, CARDINALS};
use crate::world::{self, EquipmentInstance, EquipmentKind, Item, Mob, MobKind, TileKind, World};

#[derive(Debug, Clone, Copy)]
pub struct CarveRoomOpts {
    wall: TileKind,
    floor: TileKind,
    max_width: i32,
    max_height: i32,
    min_width: i32,
    min_height: i32,
}

impl From<CarveRoomOpts> for BspSplitOpts {
    fn from(opts: CarveRoomOpts) -> Self {
        Self {
            max_width: opts.max_width,
            max_height: opts.max_height,
            min_width: opts.min_width,
            min_height: opts.min_height,
        }
    }
}

pub fn carve_rooms_bsp(
    world: &mut World,
    rect: Rect,
    opts: &CarveRoomOpts,
    rng: &mut impl Rng,
) -> Vec<Rect> {
    let tree = gen_bsp_tree(rect, (*opts).into(), rng);
    let room_graph = tree.into_room_graph();
    for room in room_graph.iter() {
        fill_rect(world, room, opts.floor);
        for adj in room_graph.get_adj(room).unwrap() {
            let wall = get_connecting_wall(room, *adj).unwrap();
            let has_door = wall.into_iter().any(|pos| world[pos].kind.is_walkable());
            if !has_door {
                carve_floor(world, wall.choose(rng), 0, opts.floor);
            }
        }
    }
    room_graph.iter().collect()
}

pub fn carve_rooms_bsp_extra_loops(
    world: &mut World,
    rect: Rect,
    opts: &CarveRoomOpts,
    rng: &mut impl Rng,
    loopiness: f32,
) -> Vec<Rect> {
    let rooms = carve_rooms_bsp(world, rect, opts, rng);
    for _ in 0..((rooms.len() - 1) as f32 * loopiness) as u32 {
        loop {
            let room1 = rooms.choose(rng).unwrap();
            let room2 = rooms.choose(rng).unwrap();
            if let Some(wall) = get_connecting_wall(*room1, *room2) {
                let pos = wall.choose(rng);
                carve_floor(world, pos, 0, opts.floor);
                break;
            }
        }
    }
    rooms
}

fn get_connecting_wall(room1: Rect, room2: Rect) -> Option<Rect> {
    // one-tile-wall between them
    for (room1, room2) in &[(room1, room2), (room2, room1)] {
        // room2 right of room1
        if room1.x2 + 2 == room2.x1 {
            let y1 = room1.y1.max(room2.y1);
            let y2 = room1.y2.min(room2.y2);
            if y1 <= y2 {
                return Some(Rect {
                    x1: room1.x2 + 1,
                    x2: room1.x2 + 1,
                    y1,
                    y2,
                });
            }
        }
        // room2 under room1
        if room1.y2 + 2 == room2.y1 {
            let x1 = room1.x1.max(room2.x1);
            let x2 = room1.x2.min(room2.x2);
            if x1 <= x2 {
                return Some(Rect {
                    x1,
                    x2,
                    y1: room1.y2 + 1,
                    y2: room1.y2 + 1,
                });
            }
        }
    }
    None
}

#[derive(Clone, Copy, Debug)]
pub struct BspSplitOpts {
    max_width: i32,
    max_height: i32,
    min_width: i32,
    min_height: i32,
}

#[derive(Debug)]
pub enum BspTree {
    Split(Box<BspTree>, Box<BspTree>),
    Room(Rect),
}

impl BspTree {
    fn into_room_graph(self) -> RoomGraph {
        match self {
            BspTree::Room(rect) => {
                let mut graph = RoomGraph::new();
                graph.add_room(rect);
                graph
            }
            BspTree::Split(tree1, tree2) => {
                let mut rooms1 = tree1.into_room_graph();
                let rooms2 = tree2.into_room_graph();
                // now figure out how to bridge the trees
                rooms1.extend_bridged(rooms2);
                rooms1
            }
        }
    }
}

struct RoomGraph {
    pub room_adj: HashMap<Rect, Vec<Rect>>,
}

impl RoomGraph {
    fn get_adj(&self, rect: Rect) -> Option<&[Rect]> {
        self.room_adj.get(&rect).map(|v| v.as_slice())
    }
    fn choose(&self, rng: &mut impl Rng) -> Option<Rect> {
        if self.room_adj.is_empty() {
            return None;
        }
        let idx = rng.gen_range(0..self.room_adj.len());
        self.room_adj.keys().nth(idx).cloned()
    }
    fn len(&self) -> usize {
        self.room_adj.len()
    }
    fn remove_room(&mut self, rect: Rect) {
        self.room_adj.retain(|r, _| *r != rect);
    }
    fn find_spatially_adjacent(&self, rect: Rect) -> Option<Rect> {
        for room in self.room_adj.keys() {
            if let Some(_wall) = get_connecting_wall(rect, *room) {
                return Some(*room);
            }
        }
        None
    }
    fn extend_bridged(&mut self, mut other: RoomGraph) {
        let mut bridged = false;
        'loop1: for (room1, ref mut adj1) in &mut self.room_adj {
            for (room2, ref mut adj2) in &mut other.room_adj {
                if get_connecting_wall(*room1, *room2).is_some() {
                    bridged = true;
                    adj1.push(*room2);
                    adj2.push(*room1);
                    break 'loop1;
                }
            }
        }
        assert!(bridged);
        self.room_adj.extend(other.room_adj);
    }
    fn new() -> Self {
        Self {
            room_adj: HashMap::new(),
        }
    }
    fn add_room(&mut self, room: Rect) {
        self.room_adj.insert(room, vec![]);
    }
    fn add_connection(&mut self, room1: Rect, room2: Rect) {
        assert!(get_connecting_wall(room1, room2).is_some());
        assert!(self.room_adj.contains_key(&room1));
        assert!(self.room_adj.contains_key(&room2));
        self.room_adj.get_mut(&room2).unwrap().push(room1);
        self.room_adj.get_mut(&room1).unwrap().push(room2);
    }
    fn add_connection_oneway(&mut self, room1: Rect, room2: Rect) {
        assert!(get_connecting_wall(room1, room2).is_some());
        assert!(self.room_adj.contains_key(&room1));
        self.room_adj.get_mut(&room1).unwrap().push(room2);
    }

    fn iter(&'_ self) -> impl Iterator<Item = Rect> + '_ {
        self.room_adj.keys().copied()
    }
}

// returns (rooms, walls between connected rooms in the bsp tree)
pub fn gen_bsp_tree(rect: Rect, opts: BspSplitOpts, rng: &mut impl Rng) -> BspTree {
    assert!(opts.min_width * 2 < opts.max_width);
    assert!(opts.min_height * 2 < opts.max_height);
    #[derive(Clone, Copy, Debug)]
    enum Split {
        X,
        Y,
        None,
    }
    let too_wide = (rect.x2 - rect.x1) > opts.max_width;
    let too_tall = (rect.y2 - rect.y1) > opts.max_height;
    let split = match (too_wide, too_tall) {
        (true, true) => *[Split::X, Split::Y].choose(rng).unwrap(),
        (true, false) => Split::X,
        (false, true) => Split::Y,
        _ => Split::None,
    };
    match split {
        Split::X => {
            let split_x =
                rng.gen_range(rect.x1 + opts.min_width + 1..(rect.x2 - opts.min_width - 1));
            let left = Rect::new(rect.x1, split_x - 1, rect.y1, rect.y2);
            let right = Rect::new(split_x + 1, rect.x2, rect.y1, rect.y2);
            BspTree::Split(
                Box::new(gen_bsp_tree(left, opts, rng)),
                Box::new(gen_bsp_tree(right, opts, rng)),
            )
        }
        Split::Y => {
            let split_y = rng.gen_range(rect.y1 + opts.min_height + 1..(rect.y2 - opts.min_height));
            let top = Rect::new(rect.x1, rect.x2, rect.y1, split_y - 1);
            let bottom = Rect::new(rect.x1, rect.x2, split_y + 1, rect.y2);
            BspTree::Split(
                Box::new(gen_bsp_tree(top, opts, rng)),
                Box::new(gen_bsp_tree(bottom, opts, rng)),
            )
        }
        Split::None => BspTree::Room(rect),
    }
}

pub fn carve_line_drunk(
    world: &mut World,
    start: Pos,
    end: Pos,
    brush_size: u8,
    rng: &mut impl Rng,
    waviness: f64,
    tile: TileKind,
    bound: Rect,
) {
    let mut pos = start;
    while pos != end {
        let dir = if rng.gen::<f64>() < waviness {
            *CARDINALS.choose(rng).unwrap()
        } else {
            (end - pos).closest_dir()
        };
        if !bound.contains(pos + dir) {
            continue;
        }
        pos += dir;
        carve_floor(world, pos, brush_size, tile);
    }
}

pub fn carve_line(world: &mut World, start: Pos, end: Pos, brush_size: u8, tile: TileKind) {
    // based on https://www.redblobgames.com/grids/line-drawing.html (2.1)
    carve_floor(world, start, brush_size, tile);
    let mut pos = start;
    let offset = end - start;
    let (nx, ny) = (offset.x.abs(), offset.y.abs());
    let (mut ix, mut iy) = (0, 0);
    while (ix, iy) != (nx, ny) {
        if (1 + 2 * ix) * ny < (1 + 2 * iy) * nx {
            pos.x += offset.x.signum();
            ix += 1;
        } else {
            pos.y += offset.y.signum();
            iy += 1;
        }
        carve_floor(world, pos, brush_size, tile);
    }
}

pub fn carve_corridor(world: &mut World, start: Pos, end: Pos, tile: TileKind) {
    let mut pos = start;
    while pos != end {
        carve_floor(world, pos, 0, tile);
        pos += (end - pos).closest_dir();
    }
}

pub fn fill_rect(world: &mut World, rect: Rect, kind: TileKind) {
    for x in rect.x1..=rect.x2 {
        for y in rect.y1..=rect.y2 {
            let pos = Pos { x, y };
            world[pos].kind = kind;
        }
    }
}

fn gen_alien_nest(world: &mut World, rng: &mut impl Rng, entrances: &[Pos], rect: Rect) {
    // draw a bunch of lines between the entrances
    let mut interior_entrances = Vec::new();
    for &e in entrances {
        for &o in &CARDINALS {
            if rect.contains(e + o) {
                interior_entrances.push(e + o);
            }
        }
    }
    fill_rect(world, rect, TileKind::YellowWall);
    // draw lines between interior entrances
    for &e1 in &interior_entrances {
        for &e2 in interior_entrances.iter().chain(&[rect.center()]) {
            carve_line_drunk(world, e1, e2, 0, rng, 0.5, TileKind::YellowFloor, rect);
        }
    }
    // spawn some enemies
    let size = rect.width() * rect.height();
    for _ in 0..(size / 20).max(1) {
        loop {
            let pos = rect.choose(rng);
            if !world[pos].kind.is_walkable() {
                continue;
            }
            // world.add_mob(pos, Mob::new(world.get_random_mob_kind(rng)));
            break;
        }
    }
}

fn gen_offices(world: &mut World, rng: &mut impl Rng, entrances: &[Pos], rect: Rect) {
    // offices
    for entrance in entrances {
        carve_floor(world, *entrance, 1, TileKind::Floor);
    }
    let max_width = rng.gen_range(4..=rect.width().min(8));
    let min_width = max_width / 2 - 1;
    let max_height = rng.gen_range(4..=rect.width().min(8));
    let min_height = max_height / 2 - 1;
    let bsp_opts = CarveRoomOpts {
        wall: TileKind::Wall,
        floor: TileKind::Floor,
        max_width,
        max_height,
        min_width,
        min_height,
    };
    // let rect = Rect::new(rect.x1 + 1, rect.x2 - 1, rect.y1 + 1, rect.y2 - 1);
    let rooms = carve_rooms_bsp_extra_loops(world, rect, &bsp_opts, rng, 1.0);
    for room in &rooms {
        // furnish the rooms a little
        let r: f32 = rng.gen_range(0.0..1.0);
        if r < 0.30 {
            for _ in 0..10 {
                // add some spashes of blood
                let x1 = rng.gen_range(room.x1..=room.x2);
                let x2 = rng.gen_range(room.x1..=room.x2);
                let y1 = rng.gen_range(room.y1..=room.y2);
                let y2 = rng.gen_range(room.y1..=room.y2);
                if x1 < x2 && y1 < y2 {
                    fill_rect(world, Rect { x1, x2, y1, y2 }, TileKind::BloodyFloor);
                }
            }
        }
    }
    let size = rect.width() * rect.height();
    // spawn some enemies
    for _ in 0..(size / 20).max(1) {
        let room = rooms.choose(rng).unwrap();
        let x = rng.gen_range(room.x1..=room.x2);
        let y = rng.gen_range(room.y1..=room.y2);
        let _pos = Pos { x, y };
        // world.add_mob(pos, Mob::new(world.get_random_mob_kind(rng)));
    }

    carve_floor(world, Pos { x: 8, y: 0 }, 1, TileKind::Floor);
}

#[derive(Debug, Clone)]
pub struct SimpleRoomOpts {
    pub rect: Rect,
    pub max_rooms: usize,
    pub min_room_size: i32,
    pub max_room_size: i32,
}

pub struct SprinkleOpts {
    pub num_enemies: usize,
    pub num_items: usize,
    pub valid_enemies: Vec<MobKind>,
    pub valid_equipment: Vec<EquipmentKind>,
}

pub fn gen_simple_rooms(
    world: &mut World,
    opts: SimpleRoomOpts,
    sprinkle: SprinkleOpts,
    rng: &mut impl Rng,
) -> Vec<Rect> {
    // Create rooms
    let mut rooms = vec![];
    for _ in 0..opts.max_rooms {
        let w = rng.gen_range(opts.min_room_size..=opts.max_room_size);
        let h = rng.gen_range(opts.min_room_size..=opts.max_room_size);
        let x = rng.gen_range(opts.rect.x1..=opts.rect.x2 - w);
        let y = rng.gen_range(opts.rect.y1..=opts.rect.y2 - h);
        let new_room = Rect::new(x, x + w, y, y + h);
        let intersects = rooms.iter().any(|r| new_room.intersects(r));
        if !intersects {
            rooms.push(new_room);
        }
    }
    // Draw corridors
    let mut connected: HashSet<usize> = HashSet::new();
    for (i, room) in rooms.iter().enumerate() {
        if let Some(nearest_other_room) = rooms
            .iter()
            .enumerate()
            .filter(|(j, _)| i != *j && !connected.contains(j))
            .map(|(_, other)| ((room.center() - other.center()).mhn_dist(), other))
            .min_by_key(|(dist, _)| *dist)
            .map(|(_, other)| other)
        {
            carve_corridor(
                world,
                room.center(),
                nearest_other_room.center(),
                TileKind::Floor,
            );
            connected.insert(i);
        }
    }

    // Write rooms on top of corridors
    for room in rooms.iter().copied() {
        for pos in room {
            carve_floor(world, pos, 0, TileKind::Floor)
        }
    }

    // Sprinkle enemies/items
    for _ in 0..sprinkle.num_enemies {
        let room = rooms[1..].choose(rng).unwrap();
        let pos = room.choose(rng);
        world.add_mob(pos, Mob::new(*sprinkle.valid_enemies.choose(rng).unwrap()));
    }
    for _ in 0..sprinkle.num_items {
        let room = rooms[0..].choose(rng).unwrap();
        let pos = room.choose(rng);
        world[pos].item = Some(Item::Equipment(EquipmentInstance::new(
            *sprinkle.valid_equipment.choose(rng).unwrap(),
            world::STARTING_DURABILITY,
        )));
    }
    rooms
}

pub fn generate_world(world: &mut World, seed: u64) {
    let mut rng = SmallRng::seed_from_u64(seed);
    let mut rooms = vec![];
    for i in 0..3 {
        let opts = SimpleRoomOpts {
            rect: Rect::new_centered(Pos::new(i * 80, 0), 80, 50),
            max_rooms: 30,
            min_room_size: 6,
            max_room_size: 10,
        };
        let sprinkle = SprinkleOpts {
            num_enemies: 30,
            num_items: 300,
            valid_enemies: world.world_info.monsters_per_level[i as usize].clone(),
            valid_equipment: world.world_info.equipment_per_level[i as usize].clone(),
        };
        rooms.push(gen_simple_rooms(world, opts.clone(), sprinkle, &mut rng));
    }
    world.player_pos = rooms[0][0].center();
    for i in 0..2 {
        world.add_stairs(
            rooms[i][rooms[i].len() - 1].center(),
            rooms[i + 1][0].center(),
        );
    }
}

pub fn carve_floor(world: &mut World, pos: Pos, brush_size: u8, tile: TileKind) {
    let brush_size = brush_size as i32;
    let brush_floor = -brush_size / 2;
    let brush_ceil = brush_floor + brush_size;
    for dx in brush_floor..=brush_ceil {
        for dy in brush_floor..=brush_ceil {
            world[pos + Offset { x: dx, y: dy }].kind = tile;
        }
    }
}
