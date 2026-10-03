//! The city skeleton: a grid of lots, each holding a tower, deck slabs on a few vertical
//! layers, and bridges to its neighbors. Everything about a lot is a pure function of
//! `(seed, lot x, lot z)`, so lots can be generated independently and in any order
//! (this is what makes milestone 4's chunk streaming seamless).
//!
//! Vertical layers (y in meters):
//! - upper deck, y = 0: sunlit streets and plazas; spawn is here.
//! - mid deck, y = -36: dim neon-lit level.
//! - deep floor, y = -80: dark street level.
//!
//! Towers rise from the deep floor; some are spires poking far above the upper deck.

use glam::{Vec3, vec3};
use sola_raylib::prelude::Color;

use super::biome::biome_at;
use super::{Aabb, Block, BlockKind};
use crate::rng::{Rng, hash_coords};

/// Side of a square lot.
pub const LOT: f32 = 24.0;

pub const UPPER_DECK: f32 = 0.0;
pub const MID_DECK: f32 = -36.0;
pub const DEEP_FLOOR: f32 = -80.0;

const DECK_THICKNESS: f32 = 1.2;
const BRIDGE_WIDTH: f32 = 4.0;

/// Salts keeping the per-lot random streams independent.
const SALT_TOWER: i32 = 1;
const SALT_DECOR: i32 = 2;

#[derive(Clone, Copy)]
pub struct Tower {
    pub center: Vec3,
    pub size: Vec3,
}

impl Tower {
    fn top(&self) -> f32 {
        self.center.y + self.size.y * 0.5
    }
}

/// The lots around the spawn point stay open as a plaza.
pub(super) fn is_plaza(lx: i32, lz: i32) -> bool {
    (-1..=0).contains(&lx) && (-1..=0).contains(&lz)
}

fn lot_origin(lx: i32, lz: i32) -> Vec3 {
    vec3(lx as f32 * LOT, 0.0, lz as f32 * LOT)
}

fn tint(rng: &mut Rng, lo: i32, hi: i32) -> Color {
    let s = rng.range_i(lo, hi);
    // Slight cool/warm drift so neighboring buildings don't read as one mass.
    let warm = rng.range_i(-8, 9);
    let c = |v: i32| v.clamp(0, 255) as u8;
    Color::new(c(s + warm), c(s + 6), c(s + 14 - warm), 255)
}

/// The tower standing on a lot, if any. Pure, so neighbors can query it for bridges.
pub fn tower(seed: u64, lx: i32, lz: i32) -> Option<Tower> {
    if is_plaza(lx, lz) {
        return None;
    }
    let mut rng = Rng::new(hash_coords(seed, lx, SALT_TOWER, lz));
    if rng.chance(biome_at(seed, lx, lz).tower_empty) {
        return None;
    }
    let w = rng.range(9.0, 16.0);
    let d = rng.range(9.0, 16.0);
    let top = match rng.f32() {
        r if r < 0.55 => rng.range(MID_DECK + 8.0, UPPER_DECK + 14.0),
        r if r < 0.90 => rng.range(UPPER_DECK + 14.0, 48.0),
        _ => rng.range(60.0, 110.0),
    };
    let jitter = (LOT - 6.0 - w.max(d)) * 0.5;
    let o = lot_origin(lx, lz);
    let cx = o.x + LOT * 0.5 + rng.range(-jitter, jitter);
    let cz = o.z + LOT * 0.5 + rng.range(-jitter, jitter);
    let h = top - DEEP_FLOOR;
    Some(Tower {
        center: vec3(cx, DEEP_FLOOR + h * 0.5, cz),
        size: vec3(w, h, d),
    })
}

/// Appends every block belonging to one lot.
pub fn generate_lot(seed: u64, lx: i32, lz: i32, out: &mut Vec<Block>) {
    let first = out.len();
    let o = lot_origin(lx, lz);
    let mid = o + vec3(LOT * 0.5, 0.0, LOT * 0.5);
    let mut decor = Rng::new(hash_coords(seed, lx, SALT_DECOR, lz));
    let biome = biome_at(seed, lx, lz);

    // Deep floor: always present, it is the bottom of the world.
    out.push(Block {
        kind: BlockKind::Floor,
        aabb: Aabb::from_center(vec3(mid.x, DEEP_FLOOR - 1.0, mid.z), vec3(LOT, 2.0, LOT)),
        color: Color::new(44, 48, 58, 255),
    });

    let tower_here = tower(seed, lx, lz);
    // Empty lots often hold a spiral stair linking the layers, with a hole through the decks.
    let stair = tower_here.is_none() && !is_plaza(lx, lz) && decor.chance(biome.stair_chance);

    let mut levels = vec![DEEP_FLOOR];

    // Deck slabs. The plaza always has an upper deck; elsewhere gaps open onto the levels below.
    for (y, keep, shade) in [
        (UPPER_DECK, biome.deck_keep[0], (92, 112)),
        (MID_DECK, biome.deck_keep[1], (64, 80)),
    ] {
        let solid = is_plaza(lx, lz) && y == UPPER_DECK;
        if !(solid || stair || decor.chance(keep)) {
            continue;
        }
        let color = tint(&mut decor, shade.0, shade.1);
        let top = y;
        levels.push(y);
        if stair {
            push_ring_deck(mid, top, color, out);
        } else {
            out.push(Block {
                kind: BlockKind::Deck,
                aabb: Aabb::from_center(
                    vec3(mid.x, top - DECK_THICKNESS * 0.5, mid.z),
                    vec3(LOT, DECK_THICKNESS, LOT),
                ),
                color,
            });
        }
    }

    push_movement_course(lx, lz, out);

    if stair {
        push_spiral_stair(&mut decor, mid, out);
    }

    if let Some(t) = tower_here {
        push_tower(&mut decor, &t, out);
    }

    // Bridges toward +x and +z neighbors (each pair handled once, by the lower lot).
    if let Some(a) = tower(seed, lx, lz) {
        for (nx, nz) in [(lx + 1, lz), (lx, lz + 1)] {
            if let Some(b) = tower(seed, nx, nz)
                && decor.chance(biome.bridge_chance)
            {
                push_bridge(&mut decor, &a, &b, nx != lx, out);
            }
        }
    }

    push_lamps(&mut decor, o, &levels, first, out);
}

/// Street lamp posts along the sidewalks of the lot's four borders, on every deck level the lot
/// has. They are dead like the rest of the city (only flowers give light), and solid.
fn push_lamps(rng: &mut Rng, origin: Vec3, levels: &[f32], first: usize, out: &mut Vec<Block>) {
    const SIDEWALK: f32 = 3.6;
    const POLE: f32 = 0.22;
    const HEIGHT: f32 = 5.6;
    let color = Color::new(70, 76, 88, 255);
    for &y in levels {
        for side in 0..4 {
            for t in [4.0_f32, 12.0, 20.0] {
                if !rng.chance(0.3) {
                    continue;
                }
                let along = t + rng.range(-1.5, 1.5);
                // Position and the direction the lamp head leans, toward the street.
                let (x, z, lean) = match side {
                    0 => (origin.x + SIDEWALK, origin.z + along, vec3(-1.0, 0.0, 0.0)),
                    1 => (
                        origin.x + LOT - SIDEWALK,
                        origin.z + along,
                        vec3(1.0, 0.0, 0.0),
                    ),
                    2 => (origin.x + along, origin.z + SIDEWALK, vec3(0.0, 0.0, -1.0)),
                    _ => (
                        origin.x + along,
                        origin.z + LOT - SIDEWALK,
                        vec3(0.0, 0.0, 1.0),
                    ),
                };
                let pole = Aabb {
                    min: vec3(x - POLE * 0.5, y, z - POLE * 0.5),
                    max: vec3(x + POLE * 0.5, y + HEIGHT, z + POLE * 0.5),
                };
                let c = vec3(x, y + HEIGHT - 0.15, z) + lean * 0.7;
                let head = Aabb::from_center(
                    c,
                    vec3(0.4 + 1.0 * lean.x.abs(), 0.3, 0.4 + 1.0 * lean.z.abs()),
                );
                if out[first..]
                    .iter()
                    .any(|b| b.aabb.overlaps(&pole) || b.aabb.overlaps(&head))
                {
                    continue;
                }
                for aabb in [pole, head] {
                    out.push(Block {
                        kind: BlockKind::Lamp,
                        aabb,
                        color,
                    });
                }
            }
        }
    }
}

/// Half-size of the square opening in a deck above a stair.
const STAIR_HOLE: f32 = 5.6;
const STEP_RISE: f32 = 0.5;
const STEP_RADIUS: f32 = 4.2;
const STEP_SIZE: f32 = 2.4;
const STEPS_PER_TURN: f32 = 28.0;

/// A lot-sized deck with a square hole in the middle, as four slabs.
fn push_ring_deck(mid: Vec3, top: f32, color: Color, out: &mut Vec<Block>) {
    let half = LOT * 0.5;
    let y = top - DECK_THICKNESS * 0.5;
    let band = half - STAIR_HOLE;
    let mut slab = |cx: f32, cz: f32, sx: f32, sz: f32| {
        out.push(Block {
            kind: BlockKind::Deck,
            aabb: Aabb::from_center(
                vec3(mid.x + cx, y, mid.z + cz),
                vec3(sx, DECK_THICKNESS, sz),
            ),
            color,
        });
    };
    let off = STAIR_HOLE + band * 0.5;
    slab(0.0, -off, LOT, band);
    slab(0.0, off, LOT, band);
    slab(-off, 0.0, band, STAIR_HOLE * 2.0);
    slab(off, 0.0, band, STAIR_HOLE * 2.0);
}

/// A helix of thin steps from the deep floor up to the upper deck. Each step rises less than
/// the player's step-up height, so it can simply be walked.
fn push_spiral_stair(rng: &mut Rng, mid: Vec3, out: &mut Vec<Block>) {
    let dir = if rng.chance(0.5) { 1.0 } else { -1.0 };
    let phase = rng.range(0.0, std::f32::consts::TAU);
    let color = Color::new(176, 150, 112, 255);
    let n = ((UPPER_DECK - DEEP_FLOOR) / STEP_RISE).round() as i32;
    for i in 1..=n {
        let a = phase + dir * i as f32 * std::f32::consts::TAU / STEPS_PER_TURN;
        let top = DEEP_FLOOR + i as f32 * STEP_RISE;
        let c = vec3(
            mid.x + a.cos() * STEP_RADIUS,
            top - STEP_RISE * 0.5,
            mid.z + a.sin() * STEP_RADIUS,
        );
        out.push(Block {
            kind: BlockKind::Stair,
            aabb: Aabb::from_center(c, vec3(STEP_SIZE, STEP_RISE, STEP_SIZE)),
            color,
        });
    }
}

/// A tower is a body plus an optional narrower setback tier on top.
fn push_tower(rng: &mut Rng, t: &Tower, out: &mut Vec<Block>) {
    push_tower_body(rng, t, out);
    if let Some(roof) = out.last().map(|b| b.aabb) {
        super::roof::push_roof(rng, &roof, out);
    }
}

fn push_tower_body(rng: &mut Rng, t: &Tower, out: &mut Vec<Block>) {
    let color = tint(rng, 92, 190);
    let top = t.top();
    let tall = top > UPPER_DECK + 30.0;
    if tall && rng.chance(0.7) {
        let tier = rng.range(0.25, 0.4) * (top - UPPER_DECK);
        let body_h = t.size.y - tier;
        out.push(Block {
            kind: BlockKind::Tower,
            aabb: Aabb {
                min: t.center - t.size * 0.5,
                max: vec3(
                    t.center.x + t.size.x * 0.5,
                    t.center.y - t.size.y * 0.5 + body_h,
                    t.center.z + t.size.z * 0.5,
                ),
            },
            color,
        });
        let shrink = rng.range(0.55, 0.8);
        let base_y = t.center.y - t.size.y * 0.5 + body_h;
        out.push(Block {
            kind: BlockKind::Tower,
            aabb: Aabb {
                min: vec3(
                    t.center.x - t.size.x * 0.5 * shrink,
                    base_y,
                    t.center.z - t.size.z * 0.5 * shrink,
                ),
                max: vec3(
                    t.center.x + t.size.x * 0.5 * shrink,
                    top,
                    t.center.z + t.size.z * 0.5 * shrink,
                ),
            },
            color: tint(rng, 92, 190),
        });
    } else {
        out.push(Block {
            kind: BlockKind::Tower,
            aabb: Aabb::from_center(t.center, t.size),
            color,
        });
    }
}

/// A walkway with low parapets between two towers. `along_x` is the direction from `a` to `b`.
fn push_bridge(rng: &mut Rng, a: &Tower, b: &Tower, along_x: bool, out: &mut Vec<Block>) {
    let (ax, cross) = if along_x { (0, 2) } else { (2, 0) };
    // Span between the facing walls, sunk slightly into both towers.
    let from = a.center[ax] + a.size[ax] * 0.5 - 0.3;
    let to = b.center[ax] - b.size[ax] * 0.5 + 0.3;
    if to - from < 1.0 {
        return;
    }
    // Needs a stretch of both facades in common to land on.
    let lo = (a.center[cross] - a.size[cross] * 0.5).max(b.center[cross] - b.size[cross] * 0.5);
    let hi = (a.center[cross] + a.size[cross] * 0.5).min(b.center[cross] + b.size[cross] * 0.5);
    if hi - lo < BRIDGE_WIDTH + 1.0 {
        return;
    }
    let min_top = a.top().min(b.top());
    let y_lo = MID_DECK + 6.0;
    let y_hi = min_top - 3.0;
    if y_hi <= y_lo {
        return;
    }
    let y = (rng.range(y_lo, y_hi) / 6.0).round() * 6.0;
    let y = y.clamp(y_lo, y_hi);
    let c = rng.range(lo + BRIDGE_WIDTH * 0.5, hi - BRIDGE_WIDTH * 0.5);
    let len = to - from;
    let mid = (from + to) * 0.5;

    let mut add = |center_ax: f32,
                   center_cross: f32,
                   y_center: f32,
                   len_ax: f32,
                   wid: f32,
                   h: f32,
                   col: Color| {
        let mut p = Vec3::ZERO;
        let mut s = Vec3::ZERO;
        p[ax] = center_ax;
        p[cross] = center_cross;
        p.y = y_center;
        s[ax] = len_ax;
        s[cross] = wid;
        s.y = h;
        out.push(Block {
            kind: BlockKind::Bridge,
            aabb: Aabb::from_center(p, s),
            color: col,
        });
    };
    let deck = Color::new(168, 178, 190, 255);
    add(mid, c, y - 0.3, len, BRIDGE_WIDTH, 0.6, deck);
    for side in [-1.0, 1.0] {
        add(
            mid,
            c + side * (BRIDGE_WIDTH * 0.5 - 0.15),
            y + 0.5,
            len,
            0.3,
            1.0,
            Color::new(140, 150, 164, 255),
        );
    }
}

/// Hand-placed blocks near spawn to test steps, jumps and ledge grabs.
fn push_movement_course(lx: i32, lz: i32, out: &mut Vec<Block>) {
    for (i, h) in [0.4_f32, 1.0, 1.6, 2.4, 3.2].iter().enumerate() {
        let c = vec3(-8.0 + i as f32 * 4.0, UPPER_DECK + h * 0.5, -10.0);
        if (c.x / LOT).floor() as i32 != lx || (c.z / LOT).floor() as i32 != lz {
            continue;
        }
        out.push(Block {
            kind: BlockKind::Course,
            aabb: Aabb::from_center(c, vec3(3.0, *h, 3.0)),
            color: Color::new(200, 110, 160, 255),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lots_are_deterministic() {
        let (mut a, mut b) = (Vec::new(), Vec::new());
        generate_lot(7, 3, -2, &mut a);
        generate_lot(7, 3, -2, &mut b);
        assert_eq!(a.len(), b.len());
        assert!(
            a.iter()
                .zip(&b)
                .all(|(x, y)| x.aabb.min == y.aabb.min && x.aabb.max == y.aabb.max)
        );
    }
}
