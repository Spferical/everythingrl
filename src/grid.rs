#![allow(unused)]
use std::{
    f64::consts::PI,
    ops::{Add, AddAssign, Div, Index, IndexMut, Mul, Sub},
};

use indexmap::IndexMap;
use rand::Rng;

pub const CHUNKSIZE: usize = 16;

macro_rules! avg {
    ($n: expr, $d: expr) => {
        ($n + $d) / 2
    };
}

macro_rules! round_down {
    ($n:expr, $d:expr) => {
        if $n >= 0 {
            ($n / $d) * $d
        } else {
            (($n - $d + 1) / $d) * $d
        }
    };
}

macro_rules! modulo {
    ($n:expr, $d:expr) => {
        (($n % $d) + $d) % $d
    };
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub struct Pos {
    pub x: i32,
    pub y: i32,
}

impl Pos {
    pub fn new(x: i32, y: i32) -> Pos {
        Pos { x, y }
    }

    pub fn adjacent_cardinal(&self) -> [Pos; 4] {
        CARDINALS.map(|c| *self + c)
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub struct Offset {
    pub x: i32,
    pub y: i32,
}

impl Offset {
    pub fn diag_dist(self) -> i32 {
        self.x.abs().max(self.y.abs())
    }
    pub fn mhn_dist(self) -> i32 {
        self.x.abs() + self.y.abs()
    }
    pub fn dist_squared(self) -> i32 {
        self.x * self.x + self.y * self.y
    }

    /// Returns the closest cardinal direction aligned with this offset.
    pub fn closest_dir(self) -> Self {
        let angle = (self.y as f64).atan2(self.x as f64);
        let mut octant = (8f64 * angle / (2f64 * PI) + 8f64) as usize % 8;
        if octant % 2 == 1 {
            octant -= 1;
        }
        DIRECTIONS[octant]
    }

    pub fn norm(self) -> Self {
        Offset {
            x: self.x.signum(),
            y: self.y.signum(),
        }
    }

    pub fn rot_cw(self) -> Self {
        Offset {
            x: self.y,
            y: -self.x,
        }
    }

    pub fn flip(self) -> Self {
        self.rot_cw().rot_cw()
    }

    pub fn rot_ccw(self) -> Self {
        self.flip().rot_cw()
    }
}

impl Mul<i32> for Offset {
    type Output = Offset;

    fn mul(self, x: i32) -> Offset {
        Offset {
            x: self.x * x,
            y: self.y * x,
        }
    }
}

impl Div<i32> for Offset {
    type Output = Offset;
    fn div(self, x: i32) -> Offset {
        Offset {
            x: self.x / x,
            y: self.y / x,
        }
    }
}

pub const NORTH: Offset = Offset { x: 0, y: -1 };
pub const SOUTH: Offset = Offset { x: 0, y: 1 };
pub const WEST: Offset = Offset { x: -1, y: 0 };
pub const EAST: Offset = Offset { x: 1, y: 0 };

// Ordered by increasing angles, starting in the positive x direction.
pub const DIRECTIONS: [Offset; 8] = [
    Offset { x: 1, y: 0 },
    Offset { x: 1, y: 1 },
    Offset { x: 0, y: 1 },
    Offset { x: -1, y: 1 },
    Offset { x: -1, y: 0 },
    Offset { x: -1, y: -1 },
    Offset { x: 0, y: -1 },
    Offset { x: 1, y: -1 },
];

pub const CARDINALS: [Offset; 4] = [
    Offset { x: 0, y: 1 },
    Offset { x: 0, y: -1 },
    Offset { x: 1, y: 0 },
    Offset { x: -1, y: 0 },
];

impl Add<Offset> for Pos {
    type Output = Pos;

    fn add(self, offset: Offset) -> Pos {
        Pos {
            x: self.x + offset.x,
            y: self.y + offset.y,
        }
    }
}

impl Sub<Pos> for Pos {
    type Output = Offset;

    fn sub(self, other: Pos) -> Offset {
        Offset {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl Sub<Offset> for Pos {
    type Output = Pos;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn sub(self, offset: Offset) -> Pos {
        self + offset.flip()
    }
}

impl AddAssign<Offset> for Pos {
    fn add_assign(&mut self, o: Offset) {
        self.x += o.x;
        self.y += o.y;
    }
}

fn get_chunk_index(pos: Pos) -> ChunkIndex {
    ChunkIndex {
        x: round_down!(pos.x, CHUNKSIZE as i32) / CHUNKSIZE as i32,
        y: round_down!(pos.y, CHUNKSIZE as i32) / CHUNKSIZE as i32,
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
struct ChunkIndex {
    pub x: i32,
    pub y: i32,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub struct Chunk<Tile> {
    grid: [[Tile; CHUNKSIZE]; CHUNKSIZE],
}

impl<Tile: Copy> Chunk<Tile> {
    fn new_filled(tile: Tile) -> Self {
        Self {
            grid: [[tile; CHUNKSIZE]; CHUNKSIZE],
        }
    }
}

/// Chunked grid of tiles, infinite in any direction.
#[derive(Debug, Clone)]
pub struct TileMap<Tile> {
    chunks: IndexMap<ChunkIndex, Chunk<Tile>>,
    default_chunk: Chunk<Tile>,
}

impl<Tile: Copy> TileMap<Tile> {
    pub fn new(default_tile: Tile) -> Self {
        TileMap {
            chunks: IndexMap::new(),
            default_chunk: Chunk::new_filled(default_tile),
        }
    }
}

impl<Tile: Copy> Index<Pos> for TileMap<Tile> {
    type Output = Tile;

    fn index(&self, pos: Pos) -> &Tile {
        let chunk_index = get_chunk_index(pos);
        let chunk = self.chunks.get(&chunk_index).unwrap_or(&self.default_chunk);
        let chunk_offset_x = modulo!(pos.x, CHUNKSIZE as i32);
        let chunk_offset_y = modulo!(pos.y, CHUNKSIZE as i32);
        &chunk.grid[chunk_offset_x as usize][chunk_offset_y as usize]
    }
}

impl<Tile: Copy> IndexMut<Pos> for TileMap<Tile> {
    fn index_mut(&mut self, pos: Pos) -> &mut Tile {
        let chunk_index = get_chunk_index(pos);
        let chunk = self.chunks.entry(chunk_index).or_insert(self.default_chunk);
        let chunk_offset_x = modulo!(pos.x, CHUNKSIZE as i32);
        let chunk_offset_y = modulo!(pos.y, CHUNKSIZE as i32);
        &mut chunk.grid[chunk_offset_x as usize][chunk_offset_y as usize]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rect {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
}

impl Rect {
    pub fn new_centered(center: Pos, width: i32, height: i32) -> Self {
        assert!(width >= 1 && height >= 1);
        // w=1 => [x, x]
        // w=2 => [x-1, x]
        // w=3 => [x-1, x+1]
        // w=4 => [x-2, x+1]
        // w=5 => [x-2, x+2]
        // ...
        Rect {
            x1: center.x - width / 2,
            x2: center.x + (width - 1) / 2,

            y1: center.y - height / 2,
            y2: center.y + (height - 1) / 2,
        }
    }
    pub fn topleft(&self) -> Pos {
        Pos {
            x: self.x1,
            y: self.y1,
        }
    }
    pub fn topright(&self) -> Pos {
        Pos {
            x: self.x2,
            y: self.y1,
        }
    }
    pub fn bottomleft(&self) -> Pos {
        Pos {
            x: self.x1,
            y: self.y2,
        }
    }
    pub fn bottomright(&self) -> Pos {
        Pos {
            x: self.x2,
            y: self.y2,
        }
    }
    pub fn width(&self) -> i32 {
        self.x2 - self.x1 + 1
    }
    pub fn height(&self) -> i32 {
        self.y2 - self.y1 + 1
    }
    pub fn choose(&self, rng: &mut impl Rng) -> Pos {
        let x = rng.gen_range(self.x1..=self.x2);
        let y = rng.gen_range(self.y1..=self.y2);
        Pos { x, y }
    }
    /// Choose random position on the edge of this rectangle.
    /// Avoids corners.
    pub fn choose_edge(&self, rng: &mut impl Rng) -> Pos {
        if self.width() <= 2 && self.height() <= 2 {
            return self.choose(rng);
        }
        let mut valid_rects = vec![];
        if self.width() > 2 {
            // Top and bottom have valid squares.
            valid_rects.push(Rect {
                x1: self.x1 + 1,
                x2: self.x2 - 1,
                y1: self.y1,
                y2: self.y1,
            });
            valid_rects.push(Rect {
                x1: self.x1 + 1,
                x2: self.x2 - 1,
                y1: self.y2,
                y2: self.y2,
            });
        }
        if self.height() > 2 {
            // Left and right have valid squares.
            valid_rects.push(Rect {
                x1: self.x1,
                x2: self.x1,
                y1: self.y1 + 1,
                y2: self.y2 - 1,
            });
            valid_rects.push(Rect {
                x1: self.x2,
                x2: self.x2,
                y1: self.y1 + 1,
                y2: self.y2 - 1,
            });
        }
        let num_valid_squares = valid_rects.iter().map(|r| r.len()).sum::<usize>();
        let rand = rng.gen_range(0..num_valid_squares);
        let mut running_len = 0;
        for rect in valid_rects {
            running_len += rect.len();
            if running_len > rand {
                return rect.choose(rng);
            }
        }
        unreachable!()
    }
    pub fn new(x1: i32, x2: i32, y1: i32, y2: i32) -> Self {
        assert!(x1 <= x2 && y1 <= y2);
        Rect { x1, y1, x2, y2 }
    }
    pub fn smol(pos: Pos) -> Self {
        Self::new(pos.x, pos.x, pos.y, pos.y)
    }
    pub fn new_containing(positions: &[Pos]) -> Self {
        assert!(!positions.is_empty());
        let mut min_x = i32::MAX;
        let mut max_x = i32::MIN;
        let mut min_y = i32::MAX;
        let mut max_y = i32::MIN;
        for pos in positions {
            min_x = min_x.min(pos.x);
            max_x = max_x.max(pos.x);
            min_y = min_y.min(pos.y);
            max_y = max_y.max(pos.y);
        }
        Self::new(min_x, max_x, min_y, max_y)
    }
    pub fn expand(mut self, amt: i32) -> Self {
        assert!(amt >= 0);
        self.x1 -= amt;
        self.x2 += amt;
        self.y1 -= amt;
        self.y2 += amt;
        self
    }
    pub fn shrink(mut self, amt: i32) -> Self {
        self.x1 += amt;
        self.x2 -= amt;
        if self.x2 < self.x1 {
            self.x1 = (self.x1 + self.x2) / 2;
            self.x2 = self.x1;
        }
        self.y1 += amt;
        self.y2 -= amt;
        if self.y2 < self.y1 {
            self.y1 = (self.y1 + self.y2) / 2;
            self.y2 = self.y1;
        }
        self
    }
    pub fn contains(&self, pos: Pos) -> bool {
        pos.x >= self.x1 && pos.x <= self.x2 && pos.y >= self.y1 && pos.y <= self.y2
    }
    pub fn center(&self) -> Pos {
        Pos {
            x: avg!(self.x1, self.x2),
            y: avg!(self.y1, self.y2),
        }
    }
    pub fn len(&self) -> usize {
        self.width() as usize * self.height() as usize
    }
    pub fn bottom_edge(&self) -> Rect {
        Rect {
            x1: self.x1,
            y1: self.y2,
            x2: self.x2,
            y2: self.y2,
        }
    }
    pub fn top_edge(&self) -> Rect {
        Rect {
            x1: self.x1,
            y1: self.y1,
            x2: self.x2,
            y2: self.y1,
        }
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        self.x1 <= other.x2 && self.x2 >= other.x1 && self.y1 <= other.y2 && self.y2 >= other.y1
    }
}

pub struct RectIter {
    rect: Rect,
    idx: i32,
}

impl Iterator for RectIter {
    type Item = Pos;
    fn next(&mut self) -> std::option::Option<Pos> {
        let width = self.rect.width();
        let height = self.rect.height();
        if self.idx >= width * height {
            None
        } else {
            let x = self.rect.x1 + (self.idx % width);
            let y = self.rect.y1 + (self.idx / width);
            self.idx += 1;
            Some(Pos { x, y })
        }
    }
}

impl IntoIterator for Rect {
    type Item = Pos;
    type IntoIter = RectIter;
    fn into_iter(self) -> Self::IntoIter {
        RectIter { rect: self, idx: 0 }
    }
}
