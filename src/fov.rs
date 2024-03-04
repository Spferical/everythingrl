use crate::{
    grid::{Offset, Pos},
    world::World,
};
use std::collections::HashSet;

/// Consider 8 quadrants on a standard graph, each one an infinitely-long
/// right triangle with one corner on the origin.
/// A QuadTransform takes an (x, y) position in the first quadrant
/// and rotates/reflects it to get the corresponding position in another.
const QUAD_TRANSFORMATIONS: [[i32; 4]; 8] = [
    [1, 0, 0, 1],
    [0, 1, 1, 0],
    [0, -1, 1, 0],
    [-1, 0, 0, 1],
    [-1, 0, 0, -1],
    [0, -1, -1, 0],
    [0, 1, -1, 0],
    [1, 0, 0, -1],
];

fn apply_quad_transform(quad: usize, off: Offset) -> Offset {
    let quad = QUAD_TRANSFORMATIONS[quad];
    Offset {
        x: off.x * quad[0] + off.y * quad[1],
        y: off.x * quad[2] + off.y * quad[3],
    }
}

/// Uses shadowcasting to return set of positions visible from pos.
pub fn calculate_fov(pos: Pos, radius: i32, world: &World) -> HashSet<Pos> {
    let mut seen = HashSet::new();
    seen.insert(pos);
    for quadrant in 0..8 {
        cast_light(&mut seen, pos, 1, 0.0, 1.0, radius, quadrant, world);
    }
    seen
}

// Recursive function to perform the shadowcasting. See
// http://www.roguebasin.com/index.php?title=FOV_using_recursive_shadowcasting
// for an explanation.
#[allow(clippy::too_many_arguments)]
fn cast_light(
    seen: &mut HashSet<Pos>,
    start_pos: Pos,
    start_y: i32,
    mut start_slope: f64,
    end_slope: f64,
    radius: i32,
    quad: usize,
    world: &World,
) {
    assert!(quad < 8);
    if start_slope > end_slope {
        return;
    }
    // stores whether the last tile we looked at was a wall
    let mut prev_blocked = false;
    let mut new_start = 0f64;
    for dy in start_y..=radius {
        for dx in 0..=dy {
            // translate relative dx, dy into absolute map position
            let offset = apply_quad_transform(quad, Offset { x: dx, y: dy });
            let pos = start_pos + offset;
            // get slopes for the extremities of the square
            let left_slope = (dx as f64 - 0.5) / (dy as f64 + 0.5);
            let right_slope = (dx as f64 + 0.5) / (dy as f64 - 0.5);
            if start_slope > right_slope || end_slope < left_slope {
                continue;
            }

            seen.insert(pos);

            if world.get_tile(pos).kind.is_opaque() {
                if prev_blocked {
                    new_start = right_slope
                } else {
                    // end of row for transparent tiles
                    prev_blocked = true;
                    cast_light(
                        seen,
                        start_pos,
                        dy + 1,
                        start_slope,
                        left_slope,
                        radius,
                        quad,
                        world,
                    );
                    new_start = right_slope;
                }
            } else if prev_blocked {
                // end of series of walls
                prev_blocked = false;
                start_slope = new_start;
            }
        }
        if prev_blocked {
            break;
        }
    }
}
