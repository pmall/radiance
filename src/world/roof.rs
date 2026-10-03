//! Rooftops: a parapet around the edge and the usual clutter (air conditioners, vents, water
//! tanks, stairwell housings, antenna masts, billboards). All of it is solid blocks, so rooftops
//! are places to run across and jump between. Pure function of the roof's rectangle and the lot's
//! random stream.

use glam::vec3;
use sola_raylib::prelude::Color;

use super::{Aabb, Block, BlockKind};
use crate::rng::Rng;

const PARAPET_THICKNESS: f32 = 0.4;
const PARAPET_HEIGHT: f32 = 0.9;
/// Clear space between the roof edge and any equipment.
const EDGE_MARGIN: f32 = 1.6;
/// Equipment is dropped on a grid of cells this wide, one prop per cell at most.
const CELL: f32 = 4.5;

fn light_gray(rng: &mut Rng) -> Color {
    let v = rng.range_i(188, 232) as u8;
    Color::new(v, v, v.saturating_add(6), 255)
}

/// Dresses the roof whose top face is the top of `roof`.
pub fn push_roof(rng: &mut Rng, roof: &Aabb, out: &mut Vec<Block>) {
    let y = roof.max.y;
    let (w, d) = (roof.max.x - roof.min.x, roof.max.z - roof.min.z);
    let height = y - super::city::DEEP_FLOOR;
    let mut placed: Vec<Aabb> = Vec::new();
    let add = |aabb: Aabb, color: Color, kind: BlockKind, out: &mut Vec<Block>| {
        out.push(Block { aabb, color, kind });
    };

    // Parapet: two full strips and two shorter ones between them.
    let (t, h) = (PARAPET_THICKNESS, PARAPET_HEIGHT);
    let wall = Color::new(214, 216, 222, 255);
    for (min, max) in [
        (
            vec3(roof.min.x, y, roof.min.z),
            vec3(roof.max.x, y + h, roof.min.z + t),
        ),
        (
            vec3(roof.min.x, y, roof.max.z - t),
            vec3(roof.max.x, y + h, roof.max.z),
        ),
        (
            vec3(roof.min.x, y, roof.min.z + t),
            vec3(roof.min.x + t, y + h, roof.max.z - t),
        ),
        (
            vec3(roof.max.x - t, y, roof.min.z + t),
            vec3(roof.max.x, y + h, roof.max.z - t),
        ),
    ] {
        add(Aabb { min, max }, wall, BlockKind::Parapet, out);
    }

    // Equipment on a jittered grid.
    let (x0, z0) = (roof.min.x + EDGE_MARGIN, roof.min.z + EDGE_MARGIN);
    let (nx, nz) = (
        ((w - 2.0 * EDGE_MARGIN) / CELL).floor().max(0.0) as i32,
        ((d - 2.0 * EDGE_MARGIN) / CELL).floor().max(0.0) as i32,
    );
    for iz in 0..nz {
        for ix in 0..nx {
            if !rng.chance(0.42) {
                continue;
            }
            let roll = rng.f32();
            // Footprint (x, z) and height by kind of equipment.
            let (sx, sz, sy) = if roll < 0.42 {
                (
                    rng.range(1.4, 2.2),
                    rng.range(1.1, 1.8),
                    rng.range(0.9, 1.3),
                )
            } else if roll < 0.62 {
                (
                    rng.range(2.6, 3.2),
                    rng.range(2.2, 2.8),
                    rng.range(1.2, 1.6),
                )
            } else if roll < 0.76 {
                (0.7, 0.7, rng.range(2.0, 3.5))
            } else if roll < 0.9 {
                (2.4, 2.4, rng.range(2.8, 3.4))
            } else {
                (rng.range(3.2, 4.0), rng.range(2.8, 3.4), 2.6)
            };
            let c = vec3(
                x0 + (ix as f32 + 0.5) * CELL + rng.range(-0.5, 0.5) * (CELL - sx).max(0.0) * 0.5,
                y,
                z0 + (iz as f32 + 0.5) * CELL + rng.range(-0.5, 0.5) * (CELL - sz).max(0.0) * 0.5,
            );
            let aabb = Aabb {
                min: vec3(c.x - sx * 0.5, y, c.z - sz * 0.5),
                max: vec3(c.x + sx * 0.5, y + sy, c.z + sz * 0.5),
            };
            if placed.iter().any(|p| p.overlaps(&aabb)) {
                continue;
            }
            placed.push(aabb);
            let color = light_gray(rng);
            add(aabb, color, BlockKind::Equipment, out);
        }
    }

    // Antenna mast: tall on the high roofs.
    if w > 5.0 && d > 5.0 && rng.chance(if height > 120.0 { 1.0 } else { 0.45 }) {
        let len = if height > 120.0 {
            rng.range(14.0, 26.0)
        } else {
            rng.range(5.0, 11.0)
        };
        let c = vec3(
            rng.range(roof.min.x + 2.0, roof.max.x - 2.0),
            y,
            rng.range(roof.min.z + 2.0, roof.max.z - 2.0),
        );
        let aabb = Aabb {
            min: vec3(c.x - 0.18, y, c.z - 0.18),
            max: vec3(c.x + 0.18, y + len, c.z + 0.18),
        };
        if !placed.iter().any(|p| p.overlaps(&aabb)) {
            placed.push(aabb);
            add(aabb, Color::new(236, 236, 240, 255), BlockKind::Mast, out);
        }
    }

    // Billboard on posts along one edge, facing out.
    if w >= 9.0 && d >= 9.0 && rng.chance(0.3) {
        let along_x = rng.chance(0.5);
        let near_min = rng.chance(0.5);
        let len = (if along_x { w } else { d } * 0.6).min(10.0);
        let (panel_h, post_h, thick) = (4.2, 1.5, 0.35);
        let centre = if along_x {
            (roof.min.x + roof.max.x) * 0.5
        } else {
            (roof.min.z + roof.max.z) * 0.5
        };
        let edge = if along_x {
            if near_min {
                roof.min.z + 1.4
            } else {
                roof.max.z - 1.4
            }
        } else if near_min {
            roof.min.x + 1.4
        } else {
            roof.max.x - 1.4
        };
        let boxed = |a0: f32, a1: f32, e0: f32, e1: f32, y0: f32, y1: f32| {
            if along_x {
                Aabb {
                    min: vec3(a0, y0, e0),
                    max: vec3(a1, y1, e1),
                }
            } else {
                Aabb {
                    min: vec3(e0, y0, a0),
                    max: vec3(e1, y1, a1),
                }
            }
        };
        let panel = boxed(
            centre - len * 0.5,
            centre + len * 0.5,
            edge - thick * 0.5,
            edge + thick * 0.5,
            y + post_h,
            y + post_h + panel_h,
        );
        if !placed.iter().any(|p| p.overlaps(&panel)) {
            add(panel, Color::new(240, 240, 240, 255), BlockKind::Sign, out);
            for s in [-0.38, 0.38] {
                let a = centre + len * s;
                let post = boxed(a - 0.15, a + 0.15, edge - 0.15, edge + 0.15, y, y + post_h);
                add(
                    post,
                    Color::new(150, 154, 162, 255),
                    BlockKind::Equipment,
                    out,
                );
            }
        }
    }
}
