//! Infinite cartesian grid library for roguelike development.
//!
//! Inspired by Jeff Lait's "Toward an Algebra of Roguelikes" talk.
//!
//! Key choices/limitations are:
//! - A map is an infinite 2d cartesian grid. Internally, it's stored in
//!   a hashmap of `NxN` chunks. Never worry about out-of-bounds again.
//! - Positions and offsets are separate types with algebraic helper methods.
//!   Also, rectangles. Write fewer 2D x/y loops!
//! - North is +y, and East is +x.
use std::{
    collections::HashMap,
    f64::consts::PI,
    ops::{Add, AddAssign, Div, Index, IndexMut, Mul, Sub},
};

use rand::Rng;

pub mod fov;
pub mod path;

const CHUNKSIZE: usize = 16;

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

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy, PartialOrd, Ord)]
/// Position of a tile.
pub struct Pos {
    /// The x-coordinate of this position, increasing from west to east.
    pub x: i32,
    /// The y-coordinate of this position, increasing from north to south.
    pub y: i32,
}

impl Pos {
    /// Creates a new Pos at coordinate (x, y).
    #[must_use]
    pub fn new(x: i32, y: i32) -> Pos {
        Pos { x, y }
    }

    /// Returns the four adjacent positions to this Pos in cardinal directions.
    #[must_use]
    pub fn adjacent_cardinal(&self) -> [Pos; 4] {
        CARDINALS.map(|c| *self + c)
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
/// An offset between two positions.
pub struct Offset {
    /// The change in the x axis this offset represents.
    pub x: i32,
    /// The change in the y axis this offset represents.
    pub y: i32,
}

impl Offset {
    /// Creates a new offset (x, y).
    #[must_use]
    pub fn new(x: i32, y: i32) -> Offset {
        Offset { x, y }
    }
    /// The number of tiles it would take to walk walk to the other end of this
    /// Offset if one is allowed to move diagonally and diagonal moves cost the
    /// same as cardinal direction moves.
    #[must_use]
    pub fn diag_walk_dist(self) -> i32 {
        self.x.abs().max(self.y.abs())
    }

    /// The Manhattan distance between two positions, x + y.
    #[must_use]
    pub fn mhn_dist(self) -> i32 {
        self.x.abs() + self.y.abs()
    }

    /// The squared euclidean distance between two positions.
    #[must_use]
    pub fn dist_squared(self) -> i32 {
        self.x * self.x + self.y * self.y
    }

    /// Returns the closest cardinal direction aligned with this offset, rounding clockwise.
    #[must_use]
    pub fn nearest_cardinal(self) -> Self {
        let angle = f64::from(self.y).atan2(f64::from(self.x));
        let mut octant = (8f64 * angle / (2f64 * PI) + 8f64) as usize;
        if octant % 2 == 1 {
            octant += 1;
        }
        octant %= 8;
        DIRECTIONS[octant]
    }

    /// Normalizes the x and y axes of this offset independently. That is,
    /// x and y are each constrained to the range [-1, 1].
    #[must_use]
    pub fn norm(self) -> Self {
        Offset {
            x: self.x.signum(),
            y: self.y.signum(),
        }
    }

    /// Rotates this offset 90 degrees clockwise.
    #[must_use]
    pub fn rot_cw(self) -> Self {
        Offset {
            x: self.y,
            y: -self.x,
        }
    }

    /// Flips this offset 180 degrees.
    #[must_use]
    pub fn flip(self) -> Self {
        self.rot_cw().rot_cw()
    }

    /// Rotates this offset 90 degrees counterclockwise.
    #[must_use]
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

/// A cardinal direction in the negative y direction.
pub const SOUTH: Offset = Offset { x: 0, y: -1 };
/// A cardinal direction in the positive y direction.
pub const NORTH: Offset = Offset { x: 0, y: 1 };
/// A cardinal direction in the negative x direction.
pub const WEST: Offset = Offset { x: -1, y: 0 };
/// A cardinal direction in the positive x direction.
pub const EAST: Offset = Offset { x: 1, y: 0 };

/// Offsets of the 8 positions immediately adjacent to a position.
///
/// Ordered by increasing angles, starting in the positive x direction.
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

/// Offsets of distance 1 in the four cardinal direction.
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
    x: i32,
    y: i32,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
struct Chunk<Tile> {
    grid: Vec<Tile>,
}

impl<Tile: Clone> Chunk<Tile> {
    fn new_filled(tile: Tile) -> Self {
        Self {
            grid: vec![tile; CHUNKSIZE * CHUNKSIZE],
        }
    }
}

/// Chunked grid of tiles, infinite in any direction.
#[derive(Debug, Clone)]
pub struct TileMap<Tile> {
    chunks: HashMap<ChunkIndex, Chunk<Tile>>,
    default_chunk: Chunk<Tile>,
}

impl<Tile: Clone> TileMap<Tile> {
    /// Creates a new `TileMap`, an infinite grid filled with `default_tile`.
    pub fn new(default_tile: Tile) -> Self {
        TileMap {
            chunks: HashMap::new(),
            default_chunk: Chunk::new_filled(default_tile),
        }
    }

    pub fn iter(&'_ self) -> impl Iterator<Item = (Pos, Tile)> + '_ {
        self.chunks.iter().flat_map(|(ci, chunk)| {
            chunk.grid.iter().enumerate().map(|(ti, tile)| {
                let pos = Pos {
                    x: ci.x * CHUNKSIZE as i32 + modulo!(ti, CHUNKSIZE) as i32,
                    y: ci.y * CHUNKSIZE as i32 + (ti / CHUNKSIZE) as i32,
                };
                (pos, tile.clone())
            })
        })
    }

    pub fn set_rect(&mut self, rect: Rect, tile: Tile) {
        for p in rect {
            self[p] = tile.clone();
        }
    }
}

impl<Tile: Clone> Index<Pos> for TileMap<Tile> {
    type Output = Tile;

    fn index(&self, pos: Pos) -> &Tile {
        let chunk_index = get_chunk_index(pos);
        let chunk = self.chunks.get(&chunk_index).unwrap_or(&self.default_chunk);
        let chunk_offset_x = modulo!(pos.x, CHUNKSIZE as i32) as usize;
        let chunk_offset_y = modulo!(pos.y, CHUNKSIZE as i32) as usize;
        let chunk_offset = chunk_offset_y * CHUNKSIZE + chunk_offset_x;
        &chunk.grid[chunk_offset]
    }
}

impl<Tile: Clone> IndexMut<Pos> for TileMap<Tile> {
    fn index_mut(&mut self, pos: Pos) -> &mut Tile {
        let chunk_index = get_chunk_index(pos);
        let chunk = self
            .chunks
            .entry(chunk_index)
            .or_insert_with(|| self.default_chunk.clone());
        let chunk_offset_x = modulo!(pos.x, CHUNKSIZE as i32) as usize;
        let chunk_offset_y = modulo!(pos.y, CHUNKSIZE as i32) as usize;
        let chunk_offset = chunk_offset_y * CHUNKSIZE + chunk_offset_x;
        &mut chunk.grid[chunk_offset]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
/// Represents a rectangle on the map.
pub struct Rect {
    /// X-coordinate of the leftmost column in the rectangle.
    pub x1: i32,
    /// Y-coordinate of the topmost row in the rectangle.
    pub y1: i32,
    /// X-coordinate of the rightmost column in the rectangle.
    pub x2: i32,
    /// Y-coordinate of the bottommost row in the rectangle.
    pub y2: i32,
}

impl Rect {
    /// Creates a new Rect centered at `center` with the given width and height.
    #[must_use]
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
    /// Returns the topleft position in this rectangle.
    #[must_use]
    pub fn topleft(&self) -> Pos {
        Pos {
            x: self.x1,
            y: self.y2,
        }
    }
    /// Returns the topright position in this rectangle.
    #[must_use]
    pub fn topright(&self) -> Pos {
        Pos {
            x: self.x2,
            y: self.y2,
        }
    }
    /// Returns the bottomleft position in this rectangle.
    #[must_use]
    pub fn bottomleft(&self) -> Pos {
        Pos {
            x: self.x1,
            y: self.y1,
        }
    }
    /// Returns the bottomright position in this rectangle.
    #[must_use]
    pub fn bottomright(&self) -> Pos {
        Pos {
            x: self.x2,
            y: self.y1,
        }
    }
    /// Returns the number of colums in this rectangle.
    #[must_use]
    pub fn width(&self) -> i32 {
        self.x2 - self.x1 + 1
    }
    /// Returns the number of rows in this rectangle.
    #[must_use]
    pub fn height(&self) -> i32 {
        self.y2 - self.y1 + 1
    }
    /// Chooses a random position in this rectangle.
    pub fn choose(&self, rng: &mut impl Rng) -> Pos {
        let x = rng.gen_range(self.x1..=self.x2);
        let y = rng.gen_range(self.y1..=self.y2);
        Pos { x, y }
    }
    /// Chooses a random position on the edge of this rectangle.
    ///
    /// Avoids corners, if possible. Good for placing doors in rooms.
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
        let num_valid_squares = valid_rects.iter().map(Rect::len).sum::<usize>();
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
    /// Creates a new rectangle covering colums [`x1`, `x2`] and rows
    /// [`y1`, `y2`].
    #[must_use]
    pub fn new(x1: i32, x2: i32, y1: i32, y2: i32) -> Self {
        assert!(x1 <= x2 && y1 <= y2);
        Rect { x1, y1, x2, y2 }
    }
    /// Creates a 1-tile rectangle containing only `pos`.
    #[must_use]
    pub fn smol(pos: Pos) -> Self {
        Self::new(pos.x, pos.x, pos.y, pos.y)
    }
    /// Creates the smallest rectangle containing all `positions`.
    #[must_use]
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
    /// Returns a rectangle expanded `amt` tiles in each cardinal direction.
    #[must_use]
    pub fn expand(mut self, amt: i32) -> Self {
        assert!(amt >= 0, "Cannot expand a rectangle by a negative amount.");
        self.x1 -= amt;
        self.x2 += amt;
        self.y1 -= amt;
        self.y2 += amt;
        self
    }
    #[must_use]
    pub fn expand_x(mut self, amt: i32) -> Self {
        assert!(amt >= 0, "Cannot expand a rectangle by a negative amount.");
        self.x1 -= amt;
        self.x2 += amt;
        self
    }
    #[must_use]
    pub fn expand_y(mut self, amt: i32) -> Self {
        assert!(amt >= 0, "Cannot expand a rectangle by a negative amount.");
        self.y1 -= amt;
        self.y2 += amt;
        self
    }

    /// Returns a rectangle shrunk `amt` tiles in each cardinal direction.
    /// Will not shrink a rectangle below size 1; a 1-size rectangle
    /// will be returned instead.
    #[must_use]
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
    /// Returns true if this rectangle contains `pos`.
    #[must_use]
    pub fn contains(&self, pos: Pos) -> bool {
        pos.x >= self.x1 && pos.x <= self.x2 && pos.y >= self.y1 && pos.y <= self.y2
    }
    /// Returns the center of this rectangle. Rounds to the topleft in case of
    /// even width and/or height.
    #[must_use]
    pub fn center(&self) -> Pos {
        Pos {
            x: avg!(self.x1, self.x2),
            y: avg!(self.y1, self.y2),
        }
    }
    /// Returns the number of tiles contained in this rectangle.
    #[allow(clippy::len_without_is_empty)]
    #[must_use]
    pub fn len(&self) -> usize {
        self.width() as usize * self.height() as usize
    }
    /// Returns the 1-height rectangle along the bottom edge of `self`.
    #[must_use]
    pub fn bottom_edge(&self) -> Rect {
        Rect {
            x1: self.x1,
            y1: self.y1,
            x2: self.x2,
            y2: self.y1,
        }
    }
    /// Returns the 1-height rectangle along the top edge of `self`.
    #[must_use]
    pub fn top_edge(&self) -> Rect {
        Rect {
            x1: self.x1,
            y1: self.y2,
            x2: self.x2,
            y2: self.y2,
        }
    }
    /// Returns the 1-width rectangle along the left edge of `self`.
    #[must_use]
    pub fn left_edge(&self) -> Rect {
        Rect {
            x1: self.x1,
            x2: self.x1,
            y1: self.y1,
            y2: self.y2,
        }
    }
    /// Returns the 1-width rectangle along the right edge of `self`.
    #[must_use]
    pub fn right_edge(&self) -> Rect {
        Rect {
            x1: self.x2,
            x2: self.x2,
            y1: self.y1,
            y2: self.y2,
        }
    }

    /// Returns whether `self` intersects `other` at any position.
    #[must_use]
    pub fn intersects(&self, other: &Rect) -> bool {
        self.x1 <= other.x2 && self.x2 >= other.x1 && self.y1 <= other.y2 && self.y2 >= other.y1
    }

    #[must_use]
    pub fn intersect(&self, other: &Rect) -> Option<Rect> {
        self.intersects(other).then_some(Rect {
            x1: self.x1.max(other.x1),
            y1: self.y1.max(other.y1),
            x2: self.x2.min(other.x2),
            y2: self.y2.min(other.y2),
        })
    }
    #[must_use]
    pub fn shift_to_right_of(&self, other: Rect) -> Rect {
        let offset_x = ((other.x2 + 1) - self.x1).max(0);
        *self + Offset::new(offset_x, 0)
    }
}

impl Add<Offset> for Rect {
    type Output = Rect;

    fn add(self, rhs: Offset) -> Self::Output {
        let Rect { x1, y1, x2, y2 } = self;
        Self {
            x1: x1 + rhs.x,
            y1: y1 + rhs.y,
            x2: x2 + rhs.x,
            y2: y2 + rhs.y,
        }
    }
}

/// Iterator over the positions in a rectangle. Goes row-by-row from the
/// topleft.
pub struct RectIter {
    rect: Rect,
    idx: i32,
}

impl Iterator for RectIter {
    type Item = Pos;
    fn next(&mut self) -> Option<Pos> {
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

#[cfg(feature = "bevy15")]
mod bevy15 {
    use super::*;
    impl From<Pos> for bevy15_math::IVec2 {
        fn from(pos: Pos) -> Self {
            let Pos { x, y } = pos;
            Self { x, y }
        }
    }
    impl From<Offset> for bevy15_math::IVec2 {
        fn from(offset: Offset) -> Self {
            let Offset { x, y } = offset;
            Self { x, y }
        }
    }

    impl From<bevy15_math::IVec2> for Pos {
        fn from(ivec: bevy15_math::IVec2) -> Self {
            let bevy15_math::IVec2 { x, y } = ivec;
            Self { x, y }
        }
    }
    impl From<Rect> for bevy15_math::IRect {
        fn from(value: Rect) -> Self {
            let Rect { x1, y1, x2, y2 } = value;
            Self::new(x1, y1, x2, y2)
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nearest_cardinal() {
        assert_eq!(Offset { x: 1, y: -30 }.nearest_cardinal(), SOUTH);
        assert_eq!(Offset { x: 30, y: 1 }.nearest_cardinal(), EAST);

        // Rounds clockwise.
        assert_eq!(Offset { x: 1, y: 1 }.nearest_cardinal(), NORTH);
        assert_eq!(Offset { x: -1, y: 1 }.nearest_cardinal(), WEST);
        assert_eq!(Offset { x: -1, y: -1 }.nearest_cardinal(), SOUTH);
        assert_eq!(Offset { x: 1, y: -1 }.nearest_cardinal(), EAST);
    }

    #[test]
    fn test_rect_trivial() {
        let origin = Pos { x: 0, y: 0 };
        let r = Rect::new_centered(origin, 1, 1);
        assert_eq!(r, Rect::smol(origin));
        assert_eq!(r, Rect::new_containing(&[origin]));
        assert_eq!(r, r.shrink(1));
        assert_eq!(r, r.expand(1).shrink(1));
        assert_eq!(r.topleft(), origin);
        assert_eq!(r.topright(), origin);
        assert_eq!(r.bottomleft(), origin);
        assert_eq!(r.bottomright(), origin);
        assert_eq!(r.width(), 1);
        assert_eq!(r.height(), 1);
        let mut rng = rand::thread_rng();
        assert_eq!(r.choose(&mut rng), origin);
        assert_eq!(r.choose_edge(&mut rng), origin);
        assert_eq!(r.center(), origin);
        assert!(r.contains(origin));
        assert_eq!(r.len(), 1);
        assert_eq!(r, r.bottom_edge());
        assert_eq!(r, r.top_edge());
        assert_eq!(r, r.left_edge());
        assert_eq!(r, r.right_edge());
        assert!(r.intersects(&r));
        assert_eq!(r.into_iter().collect::<Vec<_>>(), vec![origin]);
    }

    #[test]
    fn test_rect_3x3() {
        let r = Rect::new_centered(Pos::new(10, 20), 3, 3);
        assert_eq!(
            r,
            Rect::new_containing(&[Pos::new(9, 19), Pos::new(11, 21)])
        );
        assert_eq!(r, Rect::new(9, 11, 19, 21));
        assert_eq!(r.shrink(1), Rect::smol(Pos::new(10, 20)));
        assert_eq!(r.shrink(5), Rect::smol(Pos::new(10, 20)));
        assert_eq!(r.expand(5), Rect::new(4, 16, 14, 26));
        assert_eq!(r.topleft(), Pos::new(9, 21));
        assert_eq!(r.topright(), Pos::new(11, 21));
        assert_eq!(r.bottomleft(), Pos::new(9, 19));
        assert_eq!(r.bottomright(), Pos::new(11, 19));
        assert_eq!(r.width(), 3);
        assert_eq!(r.height(), 3);
        assert_eq!(r.len(), 9);
        let mut rng = rand::thread_rng();
        assert!(r.contains(r.choose_edge(&mut rng)));
        assert!(r.contains(r.choose(&mut rng)));
        assert!(r.choose_edge(&mut rng) != r.center());
        assert_eq!(
            r.bottom_edge(),
            Rect::new_containing(&[r.bottomleft(), r.bottomright()])
        );
        assert_eq!(
            r.top_edge(),
            Rect::new_containing(&[r.topleft(), r.topright()])
        );
        assert_eq!(
            r.left_edge(),
            Rect::new_containing(&[r.topleft(), r.bottomleft()])
        );
        assert_eq!(
            r.right_edge(),
            Rect::new_containing(&[r.bottomright(), r.topright()])
        );

        let mut expected_positions = Vec::new();
        for x in 9..=11 {
            for y in 19..=21 {
                expected_positions.push(Pos::new(x, y));
            }
        }
        expected_positions.sort();
        let mut positions = r.into_iter().collect::<Vec<_>>();
        positions.sort();
        assert_eq!(positions, expected_positions);

        for pos in r {
            assert!(r.intersects(&Rect::smol(pos)));
            assert!(r.contains(pos));
        }

        assert!(r.intersects(&r.left_edge()));
        assert!(r.intersects(&r.right_edge()));
        assert!(r.intersects(&r.bottom_edge()));
        assert!(r.intersects(&r.top_edge()));
        assert!(r.intersects(&Rect::new(11, 13, 21, 2000)));
        assert!(r.intersects(&Rect::new(-4000, 10, 9, 20)));
        assert!(!r.intersects(&Rect::new(0, 0, 0, 0)));
        assert!(!r.intersects(&Rect::new(11, 13, 22, 2000)));
    }
}
