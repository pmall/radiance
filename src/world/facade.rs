//! Things attached to tower walls: fire escapes (solid steel platforms and ladders you can climb),
//! blade signs hanging over the street levels, and the facade units of the stories next to the
//! decks (shop awnings, air conditioners, cages, balconies, laundry: `assets/modules/facade.ron`),
//! on the grid of bays the shader paints (`facade` in `shaders/scene.fs`). Pure functions of the
//! tower block and the lot's random stream.

use glam::{Vec3, vec3};
use sola_raylib::prelude::Color;

use super::chunks;
use super::{Aabb, Block, BlockKind};
use crate::rng::Rng;

const STORY: f32 = 4.0;

/// The four wall faces of a box: outward normal and the unit vector running along the face.
fn faces(a: &Aabb) -> [(Vec3, Vec3, Vec3); 4] {
    // (outward normal, along axis, origin of the face at along = 0)
    [
        (Vec3::X, Vec3::Z, vec3(a.max.x, 0.0, a.min.z)),
        (Vec3::NEG_X, Vec3::Z, vec3(a.min.x, 0.0, a.min.z)),
        (Vec3::Z, Vec3::X, vec3(a.min.x, 0.0, a.max.z)),
        (Vec3::NEG_Z, Vec3::X, vec3(a.min.x, 0.0, a.min.z)),
    ]
}

/// A box given in wall coordinates: `u` along the face from its origin, `d` out of the wall, `y`.
fn wall_box(
    face: &(Vec3, Vec3, Vec3),
    (u0, u1): (f32, f32),
    (d0, d1): (f32, f32),
    (y0, y1): (f32, f32),
) -> Aabb {
    let (n, along, origin) = *face;
    let p = |u: f32, d: f32, y: f32| origin + along * u + n * d + Vec3::Y * y;
    let (a, b) = (p(u0, d0, y0), p(u1, d1, y1));
    Aabb {
        min: a.min(b),
        max: a.max(b),
    }
}

/// Adds fire escapes and blade signs to the tower block `tower`. `levels` are the heights of the
/// street levels (decks) of the lot; `first` is where the lot's blocks start in `out`, to keep
/// clear of them.
pub fn push_facade_props(
    rng: &mut Rng,
    tower: &Aabb,
    levels: &[f32],
    first: usize,
    out: &mut Vec<Block>,
) {
    let steel = Color::new(58, 64, 74, 255);
    let top = tower.max.y;
    let add = |aabb: Aabb, color: Color, kind: BlockKind, out: &mut Vec<Block>| {
        // Never into another block of the lot (bridges, decks).
        if !out[first..].iter().any(|b| b.aabb.overlaps(&aabb)) {
            out.push(Block { aabb, color, kind });
        }
    };

    for face in faces(tower) {
        let width = if face.1 == Vec3::X {
            tower.max.x - tower.min.x
        } else {
            tower.max.z - tower.min.z
        };
        if width < 8.0 {
            continue;
        }

        // Fire escape: a column of landings joined by ladders.
        if rng.chance(0.35) {
            let u = rng.range(1.8, width - 1.8 - 2.4);
            let start = levels[rng.range_i(0, levels.len() as i32) as usize] + STORY;
            let stories = rng.range_i(3, 8);
            for k in 0..stories {
                let y = start + k as f32 * STORY + 1.0;
                if y + STORY > top - 2.0 {
                    break;
                }
                let (u0, u1) = (u, u + 2.4);
                add(
                    wall_box(&face, (u0, u1), (0.02, 1.1), (y - 0.12, y)),
                    steel,
                    BlockKind::Steel,
                    out,
                );
                // Rails on the outer edge and the two ends.
                add(
                    wall_box(&face, (u0, u1), (1.04, 1.1), (y + 0.9, y + 0.96)),
                    steel,
                    BlockKind::Steel,
                    out,
                );
                for e in [u0, u1 - 0.06] {
                    add(
                        wall_box(&face, (e, e + 0.06), (0.02, 1.1), (y + 0.9, y + 0.96)),
                        steel,
                        BlockKind::Steel,
                        out,
                    );
                }
                // Ladder up to the next landing, alternating sides.
                let lu = if k % 2 == 0 { u0 + 0.3 } else { u1 - 0.8 };
                for rail in [lu, lu + 0.5] {
                    add(
                        wall_box(&face, (rail, rail + 0.05), (0.15, 0.2), (y, y + STORY)),
                        steel,
                        BlockKind::Steel,
                        out,
                    );
                }
            }
        }

        // Blade signs over the street levels.
        for &level in levels {
            if level + 4.5 > top || !rng.chance(0.35) {
                continue;
            }
            let u = rng.range(1.5, width - 1.5);
            let y = level + 2.9;
            let (w, h) = (rng.range(0.9, 1.4), rng.range(1.0, 1.5));
            add(
                wall_box(&face, (u - 0.06, u + 0.06), (0.9, 0.9 + w), (y, y + h)),
                Color::new(240, 240, 240, 255),
                BlockKind::Sign,
                out,
            );
            add(
                wall_box(
                    &face,
                    (u - 0.03, u + 0.03),
                    (0.02, 0.95),
                    (y + h - 0.12, y + h - 0.06),
                ),
                steel,
                BlockKind::Steel,
                out,
            );
        }
    }
}

/// Width of a bay of the facade grid, and height of a story (as in `facade`, `shaders/scene.fs`).
const BAY: f32 = 4.0;
/// Stories above a street level whose windows get real units (the shader paints them plain).
const NEAR_STORIES: usize = 2;

/// Adds the facade units (from `biome`'s collection) of the stories next to each street level in
/// `levels`: on the street
/// story, shop awnings and sign boxes; on the `NEAR_STORIES` above, what each window has in front
/// of it. Bays as the shader lays them out: `BAY` wide, centered on the face.
pub fn push_facade_units(
    rng: &mut Rng,
    biome: &str,
    tower: &Aabb,
    levels: &[f32],
    first: usize,
    out: &mut Vec<Block>,
) {
    let cat = chunks::catalog(biome);
    for face in faces(tower) {
        let width = if face.1 == Vec3::X {
            tower.max.x - tower.min.x
        } else {
            tower.max.z - tower.min.z
        };
        let cols = (width / BAY).floor().max(1.0) as i32;
        let u0 = (width - cols as f32 * BAY) * 0.5;
        for &level in levels {
            // Only where the tower stands on this level and rises past its near stories.
            if tower.min.y > level + 0.5 || tower.max.y < level + BAY * (NEAR_STORIES + 1) as f32 {
                continue;
            }
            for c in 0..cols {
                let u = u0 + (c as f32 + 0.5) * BAY;
                let shop = if rng.chance(0.65) {
                    "facade_shop"
                } else {
                    "facade_awningless"
                };
                let mut units = vec![(shop, level)];
                for s in 1..=NEAR_STORIES {
                    units.push(("facade_unit", level + s as f32 * BAY + 1.0));
                }
                for (name, y) in units {
                    let Some(built) = chunks::build_module_seeded(&cat, name, rng.next_u64())
                    else {
                        continue;
                    };
                    for (b, mat) in built.boxes {
                        let aabb = wall_box(
                            &face,
                            (u + b.min.x, u + b.max.x),
                            (b.min.z, b.max.z),
                            (y + b.min.y, y + b.max.y),
                        );
                        // Never into another block of the lot (bridges, decks, fire escapes).
                        if !out[first..].iter().any(|o| o.aabb.overlaps(&aabb)) {
                            out.push(Block {
                                aabb,
                                color: mat.color(),
                                kind: BlockKind::Built(mat),
                            });
                        }
                    }
                }
            }
        }
    }
}
