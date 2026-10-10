//! The city skeleton: a grid of lots, each holding a tower, deck slabs on the floors of the
//! stack (`floors.rs`), stairs between floors and bridges to its neighbors. Everything about a
//! lot is a pure function of `(seed, lot x, lot z)`, so lots can be generated independently and
//! in any order, and the renderer can ask where the decks are without generating anything.
//!
//! - D0, the bottom: always solid ground. Towers rise from it.
//! - Floors above: a deck slab per lot, kept by chance (`Biome::deck_keep`), so gaps open onto
//!   the floors below and, where they line up, let the light down.
//! - Craters: round clusters of lots where nothing is built above some floor, so the sun pours
//!   down into the dark. One always opens near the start.
//! - Stairs: spirals in empty lots linking one to three floors, rarer higher up.
//! - The construction front: the tallest towers rise past the top deck, some still unfinished.

use glam::{Vec3, vec3};
use sola_raylib::prelude::Color;

use super::biome::biome_at;
use super::floors::{self, COUNT, START, Y};
use super::{Aabb, Block, BlockKind};
use crate::rng::{Rng, hash_coords};

/// Side of a square lot.
pub const LOT: f32 = 24.0;

const DECK_THICKNESS: f32 = 1.2;
const BRIDGE_WIDTH: f32 = 4.0;

/// Salts keeping the per-lot random streams independent.
const SALT_TOWER: i32 = 1;
const SALT_DECOR: i32 = 2;
const SALT_BRIDGE: i32 = 5;
const SALT_CRATER: i32 = 6;
const SALT_STAIR: i32 = 7;
/// Deck streams use `SALT_DECK + 16 * floor`.
const SALT_DECK: i32 = 8;

/// Side, in lots, of the grid cell that may hold one crater.
const CRATER_CELL: i32 = 8;

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

/// What covers a lot on one floor.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Deck {
    None,
    Full,
    /// A deck with a square hole for a stair passing through.
    Ring,
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

/// A crater: lots around `center` within `radius` have nothing built above `floor`.
struct Crater {
    center: (i32, i32),
    radius: i32,
    floor: usize,
}

impl Crater {
    fn contains(&self, lx: i32, lz: i32) -> bool {
        let (dx, dz) = (lx - self.center.0, lz - self.center.1);
        dx * dx + dz * dz <= self.radius * self.radius + self.radius
    }
}

/// The crater of a grid cell, if it has one.
fn crater_in_cell(seed: u64, cx: i32, cz: i32) -> Option<Crater> {
    // One always opens down to the start floor right in front of the spawn (the player wakes up
    // facing -z): a wide shaft of daylight beyond the dark plaza, the first pull upward.
    if (cx, cz) == (0, -1) {
        return Some(Crater {
            center: (0, -5),
            radius: 3,
            floor: START,
        });
    }
    let mut rng = Rng::new(hash_coords(seed, cx, SALT_CRATER, cz));
    if !rng.chance(0.45) {
        return None;
    }
    let center = (
        cx * CRATER_CELL + rng.range_i(2, CRATER_CELL - 2),
        cz * CRATER_CELL + rng.range_i(2, CRATER_CELL - 2),
    );
    let radius = if rng.chance(0.4) { 2 } else { 1 };
    // Mostly in the middle floors, sometimes all the way down to the start floor.
    let floor = if rng.chance(0.25) {
        START
    } else {
        rng.range_i(START as i32 + 1, 6) as usize
    };
    Some(Crater {
        center,
        radius,
        floor,
    })
}

/// The floor above which nothing is built on this lot, if it lies in a crater.
pub fn crater_at(seed: u64, lx: i32, lz: i32) -> Option<usize> {
    if is_plaza(lx, lz) {
        return None;
    }
    let (cx, cz) = (lx.div_euclid(CRATER_CELL), lz.div_euclid(CRATER_CELL));
    (cz - 1..=cz + 1)
        .flat_map(|z| (cx - 1..=cx + 1).map(move |x| (x, z)))
        .filter_map(|(x, z)| crater_in_cell(seed, x, z))
        .filter(|c| c.contains(lx, lz))
        .map(|c| c.floor)
        .min()
}

/// How often a stair starts on each floor (bottom to the one under the top): common low in the
/// city, where people built stairs everywhere, and rarer at every floor up, where machines built
/// for machines. Each floor needs its own stair: climbing gets harder and harder.
const STAIR_START_WEIGHTS: [i32; COUNT - 1] = [6, 6, 4, 3, 2, 1, 1, 1];

/// The floors a lot's spiral stair links (bottom, top = bottom + 1), if it has one. Stairs stand
/// in empty lots, one floor each, and get rarer higher up: going down is easy, going up is not.
pub fn stair(seed: u64, lx: i32, lz: i32) -> Option<(usize, usize)> {
    if is_plaza(lx, lz) || tower(seed, lx, lz).is_some() || crater_at(seed, lx, lz).is_some() {
        return None;
    }
    let mut rng = Rng::new(hash_coords(seed, lx, SALT_STAIR, lz));
    if !rng.chance(biome_at(seed, lx, lz).stair_chance) {
        return None;
    }
    let total: i32 = STAIR_START_WEIGHTS.iter().sum();
    let mut roll = rng.range_i(0, total);
    let lo = STAIR_START_WEIGHTS
        .iter()
        .position(|&w| {
            roll -= w;
            roll < 0
        })
        .unwrap_or(0);
    Some((lo, lo + 1))
}

/// What covers a lot on a floor. Pure, so the renderer can map the decks around the camera.
pub fn deck(seed: u64, lx: i32, lz: i32, floor: usize) -> Deck {
    if floor == 0 || (floor == START && is_plaza(lx, lz)) {
        return Deck::Full;
    }
    if let Some(f) = crater_at(seed, lx, lz) {
        if floor > f {
            return Deck::None;
        }
        if floor == f {
            return Deck::Full;
        }
    }
    if let Some((lo, hi)) = stair(seed, lx, lz) {
        if floor == lo {
            return Deck::Full;
        }
        if floor > lo && floor <= hi {
            return Deck::Ring;
        }
    }
    let keep = biome_at(seed, lx, lz).deck_keep[floors::band(floor)];
    let mut rng = Rng::new(hash_coords(seed, lx, SALT_DECK + 16 * floor as i32, lz));
    if rng.chance(keep) {
        Deck::Full
    } else {
        Deck::None
    }
}

/// One bit per floor above the ground (bit `f - 1` for floor `f`), set where the lot has a deck.
pub fn cover_bits(seed: u64, lx: i32, lz: i32) -> u8 {
    (1..COUNT)
        .filter(|&f| deck(seed, lx, lz, f) != Deck::None)
        .fold(0, |bits, f| bits | 1 << (f - 1))
}

/// The tower standing on a lot, if any. Pure, so neighbors can query it for bridges.
pub fn tower(seed: u64, lx: i32, lz: i32) -> Option<Tower> {
    if is_plaza(lx, lz) || crater_at(seed, lx, lz).is_some() {
        return None;
    }
    let mut rng = Rng::new(hash_coords(seed, lx, SALT_TOWER, lz));
    if rng.chance(biome_at(seed, lx, lz).tower_empty) {
        return None;
    }
    let w = rng.range(9.0, 16.0);
    let d = rng.range(9.0, 16.0);
    let top = match rng.f32() {
        // Old buildings of the depths, long since covered by the floors above.
        r if r < 0.2 => rng.range(Y[1] + 8.0, Y[START] + 14.0),
        // Most end somewhere in the stack, a little above one of its floors.
        r if r < 0.88 => Y[rng.range_i(START as i32, COUNT as i32) as usize] + rng.range(8.0, 28.0),
        // The construction front, past the top deck.
        _ => rng.range(floors::TOP + 30.0, floors::TOP + 110.0),
    };
    let jitter = (LOT - 6.0 - w.max(d)) * 0.5;
    let o = lot_origin(lx, lz);
    let cx = o.x + LOT * 0.5 + rng.range(-jitter, jitter);
    let cz = o.z + LOT * 0.5 + rng.range(-jitter, jitter);
    let h = top - floors::BOTTOM;
    Some(Tower {
        center: vec3(cx, floors::BOTTOM + h * 0.5, cz),
        size: vec3(w, h, d),
    })
}

/// Appends every block belonging to one lot.
pub fn generate_lot(seed: u64, lx: i32, lz: i32, out: &mut Vec<Block>) {
    let first = out.len();
    let o = lot_origin(lx, lz);
    let mid = o + vec3(LOT * 0.5, 0.0, LOT * 0.5);
    let mut decor = Rng::new(hash_coords(seed, lx, SALT_DECOR, lz));

    // The bottom: always solid ground.
    out.push(Block {
        kind: BlockKind::Floor,
        aabb: Aabb::from_center(
            vec3(mid.x, floors::BOTTOM - 1.0, mid.z),
            vec3(LOT, 2.0, LOT),
        ),
        color: Color::new(44, 48, 58, 255),
    });
    let mut levels = vec![floors::BOTTOM];

    // Deck slabs, lighter higher up.
    for (f, &y) in Y.iter().enumerate().skip(1) {
        let kind = deck(seed, lx, lz, f);
        if kind == Deck::None {
            continue;
        }
        let t = floors::depth01(y);
        let lo = 60 + (36.0 * t) as i32;
        let color = tint(&mut decor, lo, lo + 18);
        levels.push(y);
        if kind == Deck::Ring {
            push_ring_deck(mid, y, color, out);
        } else {
            out.push(Block {
                kind: BlockKind::Deck,
                aabb: Aabb::from_center(
                    vec3(mid.x, y - DECK_THICKNESS * 0.5, mid.z),
                    vec3(LOT, DECK_THICKNESS, LOT),
                ),
                color,
            });
        }
    }

    push_movement_course(lx, lz, out);

    if let Some((lo, hi)) = stair(seed, lx, lz) {
        push_spiral_stair(&mut decor, mid, Y[lo], Y[hi], out);
    }

    if let Some(t) = tower(seed, lx, lz) {
        let before = out.len();
        push_tower(&mut decor, &t, out);
        // Wall props go on the body (the first tower block).
        if let Some(body) = out[before..].first().map(|b| b.aabb) {
            super::facade::push_facade_props(&mut decor, &body, &levels, first, out);
        }
    }

    push_bridges(seed, lx, lz, out);

    push_lamps(seed, (lx, lz), &mut decor, &levels, first, out);

    // Roofs and wall props of towers ending just under a deck would pierce it: drop them.
    let decks: Vec<Aabb> = out[first..]
        .iter()
        .filter(|b| b.kind == BlockKind::Deck)
        .map(|b| b.aabb)
        .collect();
    let mut index = 0;
    out.retain(|b| {
        let ours = index >= first;
        index += 1;
        let prop = !matches!(
            b.kind,
            BlockKind::Floor | BlockKind::Deck | BlockKind::Tower | BlockKind::Stair
        );
        !ours || !prop || !decks.iter().any(|d| d.overlaps(&b.aabb))
    });
}

/// Bridges from this lot's tower toward its +x and +z neighbors (each pair is handled once, by the
/// lower lot). Own random stream, so a neighbor can ask where they are without generating the lot.
fn push_bridges(seed: u64, lx: i32, lz: i32, out: &mut Vec<Block>) {
    let Some(a) = tower(seed, lx, lz) else {
        return;
    };
    let chance = biome_at(seed, lx, lz).bridge_chance;
    let mut rng = Rng::new(hash_coords(seed, lx, SALT_BRIDGE, lz));
    for (nx, nz) in [(lx + 1, lz), (lx, lz + 1)] {
        if let Some(b) = tower(seed, nx, nz)
            && rng.chance(chance)
        {
            push_bridge(&mut rng, &a, &b, nx != lx, out);
        }
    }
}

/// Street lamp posts along the sidewalks of the lot's four borders, on every deck level the lot
/// has. They are dead like the rest of the city (only flowers give light), and solid.
fn push_lamps(
    seed: u64,
    (lx, lz): (i32, i32),
    rng: &mut Rng,
    levels: &[f32],
    first: usize,
    out: &mut Vec<Block>,
) {
    let origin = lot_origin(lx, lz);
    // Bridges from the neighbors that end in this lot cross its streets too.
    let mut incoming = Vec::new();
    push_bridges(seed, lx - 1, lz, &mut incoming);
    push_bridges(seed, lx, lz - 1, &mut incoming);
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
                    .chain(&incoming)
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

/// A helix of thin steps from height `bottom` up to `top`. Each step rises less than the
/// player's step-up height, so it can simply be walked.
fn push_spiral_stair(rng: &mut Rng, mid: Vec3, bottom: f32, top: f32, out: &mut Vec<Block>) {
    let dir = if rng.chance(0.5) { 1.0 } else { -1.0 };
    let phase = rng.range(0.0, std::f32::consts::TAU);
    let color = Color::new(176, 150, 112, 255);
    let n = ((top - bottom) / STEP_RISE).round() as i32;
    for i in 1..=n {
        let a = phase + dir * i as f32 * std::f32::consts::TAU / STEPS_PER_TURN;
        let step_top = bottom + i as f32 * STEP_RISE;
        let c = vec3(
            mid.x + a.cos() * STEP_RADIUS,
            step_top - STEP_RISE * 0.5,
            mid.z + a.sin() * STEP_RADIUS,
        );
        out.push(Block {
            kind: BlockKind::Stair,
            aabb: Aabb::from_center(c, vec3(STEP_SIZE, STEP_RISE, STEP_SIZE)),
            color,
        });
    }
}

/// A tower is a body plus an optional narrower setback tier on top. Some towers are still being
/// built: their upper tier is a bare skeleton of slabs and columns with a crane on it.
fn push_tower(rng: &mut Rng, t: &Tower, out: &mut Vec<Block>) {
    match push_tower_body(rng, t, out) {
        Some(top_slab) => super::construction::push_crane(rng, &top_slab, out),
        None => {
            if let Some(roof) = out.last().map(|b| b.aabb) {
                super::roof::push_roof(rng, &roof, out);
            }
        }
    }
}

/// Pushes the tower's blocks. Returns the top slab if the tower ends in an unfinished skeleton.
fn push_tower_body(rng: &mut Rng, t: &Tower, out: &mut Vec<Block>) -> Option<Aabb> {
    let color = tint(rng, 92, 190);
    let top = t.top();
    let tall = top > Y[START] + 30.0;
    if tall && rng.chance(0.7) {
        let tier = rng.range(0.25, 0.4) * (top - Y[START]);
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
        // Only the construction front, past the top deck, is still being built.
        if top > floors::TOP + 20.0 && rng.chance(0.5) {
            let min = vec3(
                t.center.x - t.size.x * 0.5 * shrink,
                base_y,
                t.center.z - t.size.z * 0.5 * shrink,
            );
            let max = vec3(
                t.center.x + t.size.x * 0.5 * shrink,
                top,
                t.center.z + t.size.z * 0.5 * shrink,
            );
            return Some(super::construction::push_skeleton(
                rng,
                &Aabb { min, max },
                out,
            ));
        }
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
    None
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
    let y_lo = Y[1] + 6.0;
    let y_hi = min_top - 3.0;
    if y_hi <= y_lo {
        return;
    }
    let y = (rng.range(y_lo, y_hi) / 6.0).round() * 6.0;
    let y = y.clamp(y_lo, y_hi);
    // Not level with a floor, where it would run through the decks.
    if Y.iter().any(|&f| (y - f).abs() < 4.0) {
        return;
    }
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
        let c = vec3(-8.0 + i as f32 * 4.0, Y[START] + h * 0.5, 10.0);
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

    #[test]
    fn a_stair_up_from_the_start_floor_is_never_far() {
        for seed in 1..=8 {
            let near = (-8..=8)
                .flat_map(|lz| (-8..=8).map(move |lx| (lx, lz)))
                .filter_map(|(lx, lz)| stair(seed, lx, lz))
                .any(|(lo, hi)| lo <= START && hi > START);
            assert!(
                near,
                "seed {seed}: no stair up from the start floor within 8 lots"
            );
        }
    }

    #[test]
    fn lamps_stand_clear_of_everything_else() {
        for seed in 1..=2u64 {
            let mut all = Vec::new();
            for lz in -4..4 {
                for lx in -4..4 {
                    generate_lot(seed, lx, lz, &mut all);
                }
            }
            for l in all.iter().filter(|b| b.kind == BlockKind::Lamp) {
                assert!(
                    !all.iter()
                        .any(|b| b.kind != BlockKind::Lamp && b.aabb.overlaps(&l.aabb)),
                    "lamp at {:?} overlaps a block",
                    l.aabb.min
                );
            }
        }
    }
}
