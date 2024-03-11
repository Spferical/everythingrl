#![allow(dead_code)]
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use rand::rngs::StdRng;
use rand::Rng;
use rand::{seq::SliceRandom, SeedableRng};

use crate::grid::{Offset, Pos, Rect, TileMap, CARDINALS};
use crate::net::{ItemKind, MapGen};
use crate::world::{self, Item, ItemInfo, ItemInstance, Mob, MobKind, TileKind, World, FOV_RANGE};

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

fn gen_offices(world: &mut World, rng: &mut impl Rng, rect: Rect) -> LevelgenResult {
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
    LevelgenResult {
        start: rooms[0].center(),
        end: rooms.iter().last().unwrap().center(),
    }
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
    pub num_armor: usize,
    pub num_weapons: usize,
    pub num_food: usize,
    pub enemies: Vec<MobKind>,
    pub items: Vec<Rc<ItemInfo>>,
    pub difficulty: usize,
}

pub fn gen_simple_rooms(
    world: &mut World,
    opts: &SimpleRoomOpts,
    rng: &mut impl Rng,
) -> LevelgenResult {
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

    LevelgenResult {
        start: rooms[0].center(),
        end: rooms.iter().last().unwrap().center(),
    }
}

pub struct LevelgenResult {
    pub start: Pos,
    pub end: Pos,
}

fn gen_dijkstra_map(world: &mut World, start: Pos) -> TileMap<i32> {
    let mut dijkstra_map = TileMap::new(i32::MAX);
    dijkstra_map[start] = 0;
    let mut visited = HashSet::new();
    let mut periphery = vec![start];
    let mut new_periphery = Vec::new();
    visited.extend(periphery.iter());
    for i in 0.. {
        if i > FOV_RANGE + 1 {
            break;
        }
        for pos in periphery.drain(..) {
            let adjacent = CARDINALS
                .iter()
                .copied()
                .map(|c| pos + c)
                .filter(|p| (*p - pos).dist_squared() <= FOV_RANGE)
                .filter(|pos| !visited.contains(pos))
                .filter(|pos| world[*pos].kind.is_walkable())
                .collect::<Vec<_>>();
            for pos in adjacent {
                dijkstra_map[pos] = i;
                visited.insert(pos);
                new_periphery.push(pos)
            }
        }
        std::mem::swap(&mut periphery, &mut new_periphery);
    }
    dijkstra_map
}

fn gen_level_mapgen(
    world: &mut World,
    buf: mapgen::MapBuffer,
    rect: Rect,
    _rng: &mut impl Rng,
) -> LevelgenResult {
    assert!(buf.width as i32 == rect.width());
    assert!(buf.height as i32 == rect.height());
    for x in 0..buf.width {
        for y in 0..buf.height {
            let pos = rect.topleft()
                + Offset {
                    x: x as i32,
                    y: y as i32,
                };
            world[pos].kind = if buf.is_walkable(x, y) {
                TileKind::Floor
            } else {
                TileKind::Wall
            }
        }
    }

    let start = buf.starting_point.unwrap();
    let start_pos = Pos {
        x: rect.topleft().x + start.x as i32,
        y: rect.topleft().y + start.y as i32,
    };

    // Mapgen assumes diagonal movement, which we don't have.
    // So, roll our own unreachable culling and exit detection.
    let dijkstra_map = gen_dijkstra_map(world, start_pos);
    let mut furthest_tile = start_pos;
    for pos in rect {
        if dijkstra_map[pos] == i32::MAX {
            world[pos].kind = TileKind::Wall;
        } else if dijkstra_map[pos] > dijkstra_map[furthest_tile] {
            furthest_tile = pos;
        }
    }

    LevelgenResult {
        start: start_pos,
        end: furthest_tile,
    }
}

fn sprinkle_items(
    world: &mut World,
    poses: &mut Vec<Pos>,
    num: usize,
    items: &Vec<Rc<ItemInfo>>,
    rng: &mut impl Rng,
) -> usize {
    for i in 0..num {
        let pos = match poses.pop() {
            Some(pos) => pos,
            None => return i,
        };
        if let Some(ii) = items.choose(rng).cloned() {
            world[pos].item = Some(Item::Instance(ItemInstance::new(
                ii,
                world::STARTING_DURABILITY,
            )));
        } else {
            return i;
        }
    }
    num
}

fn sprinkle_enemies_and_items(
    world: &mut World,
    rect: Rect,
    level_idx: usize,
    lgr: &LevelgenResult,
    sprinkle: &SprinkleOpts,
    rng: &mut impl Rng,
) -> Result<(), String> {
    let walkable_poses = rect
        .into_iter()
        .filter(|pos| world[*pos].kind.is_walkable())
        .collect::<Vec<_>>();

    let fov = crate::fov::calculate_fov(lgr.start, FOV_RANGE, world);

    let walkable_poses_out_of_fov = walkable_poses
        .iter()
        .copied()
        .filter(|pos| !fov.contains(pos))
        .collect::<Vec<_>>();

    // Sprinkle enemies/items
    let enemy_level_weight = match sprinkle.difficulty {
        0 => &[(7, 1), (2, 2), (1, 3)],
        1 => &[(3, 1), (5, 2), (1, 3)],
        _ => &[(3, 1), (3, 2), (4, 3)],
    };
    let enemies_per_level = (1..=3)
        .map(|i| {
            sprinkle
                .enemies
                .iter()
                .copied()
                .filter(|mk| world.get_mobkind_info(*mk).level == i)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    for _ in 0..sprinkle.num_enemies {
        let pos = match walkable_poses_out_of_fov.choose(rng) {
            Some(pos) => *pos,
            None => return Err("Failed to find pos out of fov".into()),
        };
        let desired_level = enemy_level_weight
            .choose_weighted(rng, |wl| wl.0)
            .unwrap()
            .1;
        if let Some(mob_info) = enemies_per_level[desired_level - 1].choose(rng) {
            // Try to pick enemies with a good level distribution.
            world.add_mob(pos, Mob::new(*mob_info));
        } else if let Some(mob_info) = sprinkle.enemies.choose(rng) {
            // Fall back to any enemy.
            world.add_mob(pos, Mob::new(*mob_info));
        } else {
            macroquad::miniquad::error!("No mobs available in level");
        }
    }

    let items_by_kind = |f: fn(ItemKind) -> bool| {
        sprinkle
            .items
            .iter()
            .filter(|ii| f(ii.kind))
            .cloned()
            .collect::<Vec<_>>()
    };
    let armor = items_by_kind(|k| k == ItemKind::Armor);
    let weapons = items_by_kind(|k| matches!(k, ItemKind::MeleeWeapon | ItemKind::RangedWeapon));
    let food = items_by_kind(|k| k == ItemKind::Armor);

    let mut item_poses = walkable_poses.clone();
    item_poses.shuffle(rng);
    for (num, items, name) in &[
        (sprinkle.num_armor, &armor, "armor"),
        (sprinkle.num_weapons, &weapons, "weapons"),
        (sprinkle.num_food, &food, "food"),
    ] {
        let placed = sprinkle_items(world, &mut item_poses, *num, items, rng);
        macroquad::miniquad::info!("{}", format!("Placed {placed}/{num} {name}"));
    }

    // sprinkle some starting items around the player if this is level 1
    if level_idx == 0 {
        let mut free_poses_near_player: Vec<Pos> = fov.iter().cloned().collect();
        free_poses_near_player.sort_by_key(|p| (*p - lgr.start).mhn_dist());
        free_poses_near_player.reverse();
        for (num, items, name) in &[
            (2, &armor, "starting armor"),
            (2, &weapons, "starting weapons"),
            (3, &food, "starting food"),
        ] {
            let placed = sprinkle_items(world, &mut free_poses_near_player, *num, items, rng);
            macroquad::miniquad::info!("{}", format!("Placed {placed}/{num} {name}"));
        }
    }
    // make some tiles bloody just for fun
    for p in walkable_poses {
        let gen = rng.gen::<f32>();
        if gen < 0.1 {
            world[p].kind = TileKind::BloodyFloor;
        } else if gen < 0.2 {
            world[p].kind = TileKind::YellowFloor;
        }
    }
    Ok(())
}

enum LevelGenType {
    SimpleRoomsAndCorridors,
    Caves,
    Hive,
    DenseRooms,
}

fn generate_level(world: &mut World, i: usize, rng: &mut StdRng) -> Result<LevelgenResult, String> {
    let algo = world.world_info.areas[i].mapgen;
    let sprinkle = SprinkleOpts {
        num_enemies: 30,
        num_armor: 20,
        num_weapons: 20,
        num_food: 20,
        enemies: world.world_info.monsters_per_level[i].clone(),
        items: world.world_info.equipment_per_level[i].clone(),
        difficulty: i,
    };
    let rect = Rect::new_centered(Pos::new(i as i32 * 80, 0), 80, 50);
    let lgr = match algo {
        MapGen::SimpleRoomsAndCorridors => {
            let opts = SimpleRoomOpts {
                rect,
                max_rooms: 30,
                min_room_size: 6,
                max_room_size: 10,
            };
            gen_simple_rooms(world, &opts, rng)
        }
        MapGen::Caves => {
            let buf = mapgen::MapBuilder::new(80, 50)
                .with(mapgen::NoiseGenerator::uniform())
                .with(mapgen::CellularAutomata::new())
                .with(mapgen::AreaStartingPosition::new(
                    mapgen::XStart::CENTER,
                    mapgen::YStart::CENTER,
                ))
                .with(mapgen::CullUnreachable::new())
                .with(mapgen::DistantExit::new())
                .build_with_rng(rng);
            gen_level_mapgen(world, buf, rect, rng)
        }
        MapGen::Hive => {
            let buf = mapgen::MapBuilder::new(80, 50)
                .with(mapgen::VoronoiHive::new())
                .with(mapgen::AreaStartingPosition::new(
                    mapgen::XStart::LEFT,
                    mapgen::YStart::TOP,
                ))
                .with(mapgen::DistantExit::new())
                .build_with_rng(rng);
            gen_level_mapgen(world, buf, rect, rng)
        }
        MapGen::DenseRooms => {
            // too dense for big rect
            let rect = Rect::new_centered(rect.center(), 40, 25);
            gen_offices(world, rng, rect)
        }
    };
    sprinkle_enemies_and_items(world, rect, i, &lgr, &sprinkle, rng).map(|_| lgr)
}

pub fn generate_world(world: &mut World, seed: u64) {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut results = vec![];
    for i in 0..world.world_info.areas.len() {
        let lgr = loop {
            let algo = world.world_info.areas[i].mapgen;
            match generate_level(world, i, &mut rng) {
                Ok(lgr) => break lgr,
                Err(e) => {
                    macroquad::miniquad::error!("{}", format!("{algo:?} levelgen failed: {e}"));
                }
            }
        };
        results.push(lgr);
    }
    world.player_pos = results[0].start;
    for i in 1..results.len() {
        world.add_stairs(results[i - 1].end, results[i].start)
    }
    // final boss room
    let fb_rect = Rect::new_centered(Pos::new(80 * 4, 0), 12, 12);
    for pos in fb_rect {
        carve_floor(world, pos, 0, TileKind::YellowFloor);
    }
    world.add_stairs(
        results.iter().last().unwrap().end,
        fb_rect.bottom_edge().choose(&mut rng),
    );
    let boss_kind = world.world_info.boss_info.as_ref().unwrap().mob_kind;
    world.add_mob(fb_rect.top_edge().center(), Mob::new(boss_kind));
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
