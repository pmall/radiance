//! Towers that are still being built: machines never stopped, so some towers end in a bare
//! skeleton of floor slabs and columns with rebar sticking out, topped by a tower crane. Nothing
//! moves: the crane stands frozen in mid-job. All solid, so it can be climbed and run across.

use glam::vec3;
use sola_raylib::prelude::Color;

use super::{Aabb, Block, BlockKind};
use crate::rng::Rng;

const STORY: f32 = 4.0;
const SLAB: f32 = 0.35;
const COLUMN: f32 = 0.5;

/// Crane paint.
const YELLOW: Color = Color::new(232, 176, 24, 255);

/// Raw concrete floors and columns over `footprint` (its `y` range is the skeleton's height).
/// Returns the top slab.
pub fn push_skeleton(rng: &mut Rng, footprint: &Aabb, out: &mut Vec<Block>) -> Aabb {
    let (min, max) = (footprint.min, footprint.max);
    let stories = (((max.y - min.y) / STORY).floor() as i32).max(1);
    let top_y = min.y + stories as f32 * STORY;
    let concrete = Color::new(168, 168, 164, 255);

    // Columns on a grid, floor to top.
    let cols = |lo: f32, hi: f32| (((hi - lo) / 4.5).ceil() as i32 + 1).max(2);
    let (nx, nz) = (cols(min.x, max.x), cols(min.z, max.z));
    for ix in 0..nx {
        for iz in 0..nz {
            let x = min.x + COLUMN * 0.5 + (max.x - min.x - COLUMN) * ix as f32 / (nx - 1) as f32;
            let z = min.z + COLUMN * 0.5 + (max.z - min.z - COLUMN) * iz as f32 / (nz - 1) as f32;
            out.push(Block {
                kind: BlockKind::Parapet,
                aabb: Aabb {
                    min: vec3(x - COLUMN * 0.5, min.y, z - COLUMN * 0.5),
                    max: vec3(x + COLUMN * 0.5, top_y, z + COLUMN * 0.5),
                },
                color: concrete,
            });
            // Rebar standing up from some columns on the top slab.
            if rng.chance(0.5) {
                let h = rng.range(0.8, 2.0);
                out.push(Block {
                    kind: BlockKind::Steel,
                    aabb: Aabb {
                        min: vec3(x - 0.05, top_y, z - 0.05),
                        max: vec3(x + 0.05, top_y + h, z + 0.05),
                    },
                    color: Color::new(120, 82, 60, 255),
                });
            }
        }
    }

    // A slab at the top of every story.
    let mut top = Aabb {
        min,
        max: max.min(vec3(max.x, top_y, max.z)),
    };
    for k in 1..=stories {
        let y = min.y + k as f32 * STORY;
        top = Aabb {
            min: vec3(min.x, y - SLAB, min.z),
            max: vec3(max.x, y, max.z),
        };
        out.push(Block {
            kind: BlockKind::Parapet,
            aabb: top,
            color: concrete,
        });
    }
    top
}

/// A tower crane standing on `slab`: mast, jib with counterweight, cab, cable and hook block.
pub fn push_crane(rng: &mut Rng, slab: &Aabb, out: &mut Vec<Block>) {
    let y = slab.max.y;
    let (cx, cz) = (
        rng.range(slab.min.x + 1.5, slab.max.x - 1.5)
            .min(slab.max.x - 1.0),
        rng.range(slab.min.z + 1.5, slab.max.z - 1.5)
            .min(slab.max.z - 1.0),
    );
    let mast_h = rng.range(10.0, 18.0);
    let top = y + mast_h;
    let mut add = |min: glam::Vec3, max: glam::Vec3, color: Color| {
        out.push(Block {
            kind: BlockKind::Steel,
            aabb: Aabb { min, max },
            color,
        });
    };
    add(
        vec3(cx - 0.45, y, cz - 0.45),
        vec3(cx + 0.45, top, cz + 0.45),
        YELLOW,
    );

    // The jib runs along x or z, long side one way, counterweight the other.
    let along_x = rng.chance(0.5);
    let sign = if rng.chance(0.5) { 1.0 } else { -1.0 };
    let (jib, counter) = (rng.range(13.0, 22.0), 5.0);
    let span = |a: f32, b: f32| (a.min(b), a.max(b));
    let (lo, hi) = span(sign * -counter, sign * jib);
    let beam = |lo: f32, hi: f32, y0: f32, y1: f32, half: f32| {
        if along_x {
            (vec3(cx + lo, y0, cz - half), vec3(cx + hi, y1, cz + half))
        } else {
            (vec3(cx - half, y0, cz + lo), vec3(cx + half, y1, cz + hi))
        }
    };
    let (a, b) = beam(lo, hi, top, top + 0.7, 0.3);
    add(a, b, YELLOW);
    // Counterweight and cab.
    let (cw0, cw1) = span(sign * -counter, sign * (-counter + 1.6));
    let (a, b) = beam(cw0, cw1, top - 1.2, top + 0.7, 0.9);
    add(a, b, Color::new(150, 150, 146, 255));
    let (a, b) = beam(-0.7, 0.7, top + 0.7, top + 2.2, 0.7);
    add(a, b, YELLOW);
    // A pennant mast on top of the mast.
    add(
        vec3(cx - 0.08, top + 0.7, cz - 0.08),
        vec3(cx + 0.08, top + 3.0, cz + 0.08),
        YELLOW,
    );

    // Frozen mid-lift: cable and hook block hang from the jib.
    let at = sign * jib * rng.range(0.45, 0.85);
    let drop = rng.range(4.0, 9.0);
    let (lo, hi) = span(at - 0.04, at + 0.04);
    let (a, b) = beam(lo, hi, top - drop, top, 0.04);
    add(a, b, Color::new(40, 40, 44, 255));
    let (lo, hi) = span(at - 0.3, at + 0.3);
    let (a, b) = beam(lo, hi, top - drop - 0.6, top - drop, 0.3);
    add(a, b, YELLOW);
}
