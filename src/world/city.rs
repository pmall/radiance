//! The city skeleton: a grid of lots, each holding a tower, deck slabs on the floors of the
//! stack (`floors.rs`), stairs between floors and bridges to its neighbors. Everything about a
//! lot is a pure function of `(seed, lot x, lot z)`, so lots can be generated independently and
//! in any order, and the renderer can ask where the decks are without generating anything.
//!
//! - D0, the bottom: always solid ground. Towers rise from it.
//! - Floors above: the city was laid floor by floor, in sections. Each district of `DISTRICT` x
//!   `DISTRICT` lots is split into rectangular sections of one to three lots a side, and each
//!   section is built up to some floor (`built`): it has a deck on every floor up to that one and
//!   none above. Holes are thus straight-edged and open to the sky, lining up like light wells:
//!   rare low down, common at the construction front (the floor's `deck_keep`, `design.rs`).
//! - Craters: round clusters of lots where nothing is built above some floor, so the sun pours
//!   down into the dark. One always opens near the start.
//! - Stairs: switchbacks in lots without a tower, each linking one floor to the next, rarer higher
//!   up, only where the section is built up to the stair's top.
//! - The construction front: the tallest towers rise past the top deck, some still unfinished.

use glam::{Vec3, vec3};
use sola_raylib::prelude::Color;

use super::biome::biome_at;
use super::chunks;
use super::planner::{self, Placement, Role};
use super::design;
use super::floors::{self, START};
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
const SALT_SECTION: i32 = 10;
const SALT_SECTION_TINT: i32 = 11;
const SALT_CHUNK: i32 = 13;

/// Side, in lots, of a district: the square that is split into deck sections.
const DISTRICT: i32 = 6;
/// Longest side of a deck section, in lots.
const MAX_SECTION: i32 = 3;

/// Side, in lots, of the grid cell that may hold one crater.
const CRATER_CELL: i32 = 8;

#[derive(Clone, Copy)]
pub struct Tower {
    pub center: Vec3,
    pub size: Vec3,
}

impl Tower {
    pub fn top(&self) -> f32 {
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
pub fn is_plaza(lx: i32, lz: i32) -> bool {
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
    let middle: Vec<usize> = (0..floors::count())
        .filter(|&f| floors::zone(f) == floors::Zone::Middle)
        .collect();
    let floor = if rng.chance(0.25) || middle.is_empty() {
        START
    } else {
        middle[rng.range_i(0, middle.len() as i32) as usize]
    };
    Some(Crater {
        center,
        radius,
        floor,
    })
}

/// The floor above which nothing is built on this lot, if it lies in a crater.
pub fn crater_at(seed: u64, lx: i32, lz: i32) -> Option<usize> {
    if is_plaza(lx, lz) || chunks::preview().is_some() || planner::area_at(seed, lx, lz).is_some() {
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

/// The floors a lot's stair links (bottom, top = bottom + 1), if it has one. Every stair stands in
/// the middle lot of a stair hub chunk placed by the planner (`planner.rs`), at most one per hub
/// cell and floor, at the floor's profile chance (`design.rs`): common in the depths, rare high up,
/// so climbing gets harder and harder.
pub fn stair(seed: u64, lx: i32, lz: i32) -> Option<(usize, usize)> {
    planner::stair_hub(seed, lx, lz).map(|p| (p.floor, p.floor + 1))
}

/// The deck section a lot belongs to, as its lot range `(x0, z0, x1, z1)` (half open): its
/// district split again and again along its longer side until no side is longer than
/// `MAX_SECTION`, and sometimes a little further.
fn section(seed: u64, lx: i32, lz: i32) -> (i32, i32, i32, i32) {
    let (x0, z0) = (
        lx.div_euclid(DISTRICT) * DISTRICT,
        lz.div_euclid(DISTRICT) * DISTRICT,
    );
    let mut r = (x0, z0, x0 + DISTRICT, z0 + DISTRICT);
    loop {
        let (w, h) = (r.2 - r.0, r.3 - r.1);
        let mut rng = Rng::new(hash_coords(seed, r.0 * 8 + w, SALT_SECTION, r.1 * 8 + h));
        let must = w.max(h) > MAX_SECTION;
        if w.max(h) <= 1 || !(must || rng.chance(0.35)) {
            return r;
        }
        let along_x = if w == h { rng.chance(0.5) } else { w > h };
        if along_x {
            let cut = r.0 + rng.range_i(1, w);
            r = if lx < cut {
                (r.0, r.1, cut, r.3)
            } else {
                (cut, r.1, r.2, r.3)
            };
        } else {
            let cut = r.1 + rng.range_i(1, h);
            r = if lz < cut {
                (r.0, r.1, r.2, cut)
            } else {
                (r.0, cut, r.2, r.3)
            };
        }
    }
}

/// The highest floor a lot's section is built up to: it has a deck on every floor up to this one
/// and none above. The chance that a section reaches a floor is that floor's `deck_keep`.
pub fn built(seed: u64, lx: i32, lz: i32) -> usize {
    let r = section(seed, lx, lz);
    let mut rng = Rng::new(hash_coords(
        seed,
        r.0 * 8 + r.2 - r.0,
        SALT_SECTION + 64,
        r.1 * 8 + r.3 - r.1,
    ));
    let u = rng.f32();
    (1..floors::count())
        .take_while(|&f| u < design::profile(f).deck_keep)
        .last()
        .unwrap_or(0)
}

/// What covers a lot on a floor. Pure, so the renderer can map the decks around the camera.
pub fn deck(seed: u64, lx: i32, lz: i32, floor: usize) -> Deck {
    // A previewed chunk stands on a bare deck at the start floor.
    if chunks::preview().is_some() {
        return if floor == 0 || floor == START {
            Deck::Full
        } else {
            Deck::None
        };
    }
    if floor == 0 || (floor == START && is_plaza(lx, lz)) {
        return Deck::Full;
    }
    // An area's floor is a full deck under it.
    if planner::area_at(seed, lx, lz).is_some_and(|a| a.floor == floor) {
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
    if floor <= built(seed, lx, lz) {
        Deck::Full
    } else {
        Deck::None
    }
}

/// One bit per floor above the ground (bit `f - 1` for floor `f`), set where the lot has a deck.
pub fn cover_bits(seed: u64, lx: i32, lz: i32) -> u16 {
    (1..floors::count())
        .filter(|&f| deck(seed, lx, lz, f) != Deck::None)
        .fold(0, |bits, f| bits | 1 << (f - 1))
}

/// The tower standing on a lot, if any. Pure, so neighbors can query it for bridges.
pub fn tower(seed: u64, lx: i32, lz: i32) -> Option<Tower> {
    // Areas are designed spaces, and the start's stair needs its lot.
    if chunks::preview().is_some()
        || (lx, lz) == planner::START_HUB
        || planner::area_at(seed, lx, lz).is_some()
    {
        return None;
    }
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
        r if r < 0.2 => rng.range(floors::y(1) + 8.0, floors::y(START) + 14.0),
        // Most end somewhere in the stack, a little above one of its floors.
        r if r < 0.88 => {
            floors::y(rng.range_i(START as i32, floors::count() as i32) as usize)
                + rng.range(8.0, 28.0)
        }
        // The construction front, past the top deck.
        _ => rng.range(floors::top() + 30.0, floors::top() + 110.0),
    };
    let jitter = (LOT - 6.0 - w.max(d)) * 0.5;
    let o = lot_origin(lx, lz);
    let cx = o.x + LOT * 0.5 + rng.range(-jitter, jitter);
    let cz = o.z + LOT * 0.5 + rng.range(-jitter, jitter);
    let h = top - floors::bottom();
    Some(Tower {
        center: vec3(cx, floors::bottom() + h * 0.5, cz),
        size: vec3(w, h, d),
    })
}

/// Appends every block belonging to one lot, prop collision boxes included.
pub fn generate_lot(seed: u64, lx: i32, lz: i32, out: &mut Vec<Block>) {
    generate_lot_dressed(seed, lx, lz, out);
}

/// `generate_lot`, also returning the lot's props and their flowers (`dressing.rs`).
pub fn generate_lot_dressed(
    seed: u64,
    lx: i32,
    lz: i32,
    out: &mut Vec<Block>,
) -> super::dressing::Dressing {
    let first = out.len();
    generate_structure(seed, lx, lz, out);
    if chunks::preview().is_some() {
        return super::dressing::Dressing::default();
    }
    let dressing = super::dressing::dress_lot(seed, lx, lz, &out[first..]);
    out.extend(dressing.props.iter().map(|p| Block {
        kind: BlockKind::Prop,
        aabb: p.aabb(),
        color: Color::new(0, 0, 0, 255),
    }));
    dressing
}

/// The lot's architecture: ground, decks, stairs, tower, bridges.
fn generate_structure(seed: u64, lx: i32, lz: i32, out: &mut Vec<Block>) {
    let first = out.len();
    let o = lot_origin(lx, lz);
    let mid = o + vec3(LOT * 0.5, 0.0, LOT * 0.5);
    let mut decor = Rng::new(hash_coords(seed, lx, SALT_DECOR, lz));

    // The bottom: always solid ground.
    out.push(Block {
        kind: BlockKind::Floor,
        aabb: Aabb::from_center(
            vec3(mid.x, floors::bottom() - 1.0, mid.z),
            vec3(LOT, 2.0, LOT),
        ),
        color: Color::new(44, 48, 58, 255),
    });
    let mut levels = vec![floors::bottom()];

    // Deck slabs, lighter higher up.
    for f in 1..floors::count() {
        let y = floors::y(f);
        let kind = deck(seed, lx, lz, f);
        if kind == Deck::None {
            continue;
        }
        // One tint per section and floor: a section's slab was poured in one go.
        let t = floors::depth01(y);
        let lo = 60 + (36.0 * t) as i32;
        let r = section(seed, lx, lz);
        let color = tint(
            &mut Rng::new(hash_coords(
                seed,
                r.0 * 16 + f as i32,
                SALT_SECTION_TINT,
                r.1,
            )),
            lo,
            lo + 18,
        );
        levels.push(y);
        if kind == Deck::Ring {
            let hole = if stair_along_x(seed, lx, lz) {
                WELL
            } else {
                (WELL.1, WELL.0)
            };
            push_ring_deck(mid, y, hole, color, out);
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

    if chunks::preview().is_none() {
        push_movement_course(lx, lz, out);
    }

    if let Some((lo, hi)) = stair(seed, lx, lz) {
        push_switchback_stair(
            mid,
            stair_along_x(seed, lx, lz),
            floors::y(lo),
            floors::y(hi),
            out,
        );
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

    push_chunks(seed, lx, lz, first, out);
}

/// The chunks covering a lot: the previewed one (`--chunk`), or the planner's (`planner.rs`).
pub fn placements(seed: u64, lx: i32, lz: i32) -> Vec<Placement> {
    if let Some(((ox, oz), size)) = chunks::preview_footprint() {
        let p = Placement {
            chunk: chunks::preview().map(|p| p.name.clone()).unwrap_or_default(),
            role: Role::Area,
            origin: (ox, oz),
            size,
            floor: START,
            turn: 0,
            seed: hash_coords(seed, ox, SALT_CHUNK, oz),
        };
        return if p.covers(lx, lz) { vec![p] } else { Vec::new() };
    }
    planner::covering(seed, lx, lz)
}

/// A placed chunk, built in world coordinates.
fn build_placement(p: &Placement) -> Option<(chunks::Chunk, chunks::Built)> {
    let cat = chunks::catalog();
    let chunk = cat.chunks.get(&p.chunk)?.clone();
    let origin = lot_origin(p.origin.0, p.origin.1) + Vec3::Y * floors::y(p.floor);
    let built = chunks::build(&chunk, &cat, origin, p.turn, p.seed);
    Some((chunk, built))
}

/// The vegetation hints (`chunks::GrowthKind`) of the chunks covering this lot.
pub fn growth_hints(seed: u64, lx: i32, lz: i32) -> Vec<(Aabb, chunks::GrowthKind)> {
    placements(seed, lx, lz)
        .iter()
        .filter_map(build_placement)
        .flat_map(|(_, b)| b.growth)
        .collect()
}

/// This lot's part of the chunks covering it: the boxes whose center falls in the lot, and
/// collision for those reaching in from the lots that own them. A chunk's boxes give way to the
/// lot's own architecture (a tower, a stair) and to the towers next door.
fn push_chunks(seed: u64, lx: i32, lz: i32, first: usize, out: &mut Vec<Block>) {
    let o = lot_origin(lx, lz);
    let lot = Aabb {
        min: vec3(o.x, f32::MIN, o.z),
        max: vec3(o.x + LOT, f32::MAX, o.z + LOT),
    };
    let mut obstacles: Vec<Aabb> = out[first..]
        .iter()
        .filter(|b| !matches!(b.kind, BlockKind::Floor | BlockKind::Deck))
        .map(|b| b.aabb)
        .collect();
    for (nx, nz) in [(lx - 1, lz), (lx + 1, lz), (lx, lz - 1), (lx, lz + 1)] {
        if let Some(t) = tower(seed, nx, nz) {
            obstacles.push(Aabb::from_center(t.center, t.size));
        }
    }
    let all = placements(seed, lx, lz);
    for p in &all {
        // A hub's surroundings give way to an area, and need a deck to stand on.
        if p.role == Role::Hub
            && (all.iter().any(|a| a.role == Role::Area && a.floor == p.floor)
                || deck(seed, lx, lz, p.floor) == Deck::None)
        {
            continue;
        }
        let Some((chunk, built)) = build_placement(p) else {
            continue;
        };
        let y = floors::y(p.floor);
        // Paved over: the lot's deck takes the chunk's ground material.
        if let Some(mat) = chunk.ground {
            for b in out[first..].iter_mut() {
                if b.kind == BlockKind::Deck && (b.aabb.max.y - y).abs() < 0.01 {
                    b.kind = BlockKind::Built(mat);
                    b.color = mat.color();
                }
            }
        }
        let mut blocks: Vec<Block> = built
            .boxes
            .iter()
            .map(|&(aabb, mat)| Block {
                kind: BlockKind::Built(mat),
                aabb,
                color: mat.color(),
            })
            .collect();
        for &(at, along_x) in &built.stairs {
            push_switchback_stair(at, along_x, at.y, at.y + floors::SPACING, &mut blocks);
        }
        for b in blocks {
            if !b.aabb.overlaps(&lot) || obstacles.iter().any(|o| o.overlaps(&b.aabb)) {
                continue;
            }
            let c = (b.aabb.min + b.aabb.max) * 0.5;
            let owned = (c.x / LOT).floor() as i32 == lx && (c.z / LOT).floor() as i32 == lz;
            out.push(if owned {
                b
            } else {
                Block {
                    kind: BlockKind::Collider,
                    ..b
                }
            });
        }
    }
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

// Switchback stairs: straight flights going back and forth in two lanes split by a central
// wall, with a landing at each turn and rails on the open sides.
const FLIGHT_RUN: f32 = 8.0;
const FLIGHT_STEPS: usize = 20;
/// Step height aimed for (well under the player's step-up height).
const TARGET_RISE: f32 = 0.25;
const LANE: f32 = 2.0;
const SPINE: f32 = 0.4;
const LANDING: f32 = 2.0;
const LANDING_THICKNESS: f32 = 0.3;
const RAIL_HEIGHT: f32 = 1.1;
const RAIL_THICKNESS: f32 = 0.1;
/// Half size of the stairwell: along the flights, across them. The deck above has this hole.
const WELL: (f32, f32) = (FLIGHT_RUN * 0.5 + LANDING, LANE + SPINE * 0.5);

/// Whether a lot's stair flights run along x (else along z).
fn stair_along_x(seed: u64, lx: i32, lz: i32) -> bool {
    hash_coords(seed, lx, SALT_STAIR + 1, lz) & 1 == 0
}

/// A lot-sized deck with a rectangular hole of half size `hole` (x, z) in the middle, as four
/// slabs.
fn push_ring_deck(mid: Vec3, top: f32, hole: (f32, f32), color: Color, out: &mut Vec<Block>) {
    let half = LOT * 0.5;
    let y = top - DECK_THICKNESS * 0.5;
    let mut slab = |x0: f32, x1: f32, z0: f32, z1: f32| {
        out.push(Block {
            kind: BlockKind::Deck,
            aabb: Aabb {
                min: vec3(mid.x + x0, y - DECK_THICKNESS * 0.5, mid.z + z0),
                max: vec3(mid.x + x1, y + DECK_THICKNESS * 0.5, mid.z + z1),
            },
            color,
        });
    };
    slab(-half, half, -half, -hole.1);
    slab(-half, half, hole.1, half);
    slab(-half, -hole.0, -hole.1, hole.1);
    slab(hole.0, half, -hole.1, hole.1);
}

/// A switchback stair from height `bottom` up to `top`, centered on `mid`.
fn push_switchback_stair(mid: Vec3, along_x: bool, bottom: f32, top: f32, out: &mut Vec<Block>) {
    let stone = Color::new(176, 150, 112, 255);
    let steel = Color::new(70, 76, 88, 255);
    let wall = Color::new(150, 150, 156, 255);
    let flights = ((top - bottom) / (FLIGHT_STEPS as f32 * TARGET_RISE))
        .round()
        .max(1.0) as usize;
    let rise = (top - bottom) / (flights * FLIGHT_STEPS) as f32;
    let tread = FLIGHT_RUN / FLIGHT_STEPS as f32;
    let half_run = FLIGHT_RUN * 0.5;
    // A box from stair coordinates: u along the flights, w across them.
    let mut push = |u: (f32, f32), w: (f32, f32), y: (f32, f32), kind: BlockKind, color: Color| {
        let (x, z) = if along_x { (u, w) } else { (w, u) };
        out.push(Block {
            kind,
            aabb: Aabb {
                min: vec3(mid.x + x.0, y.0, mid.z + z.0),
                max: vec3(mid.x + x.1, y.1, mid.z + z.1),
            },
            color,
        });
    };
    for i in 0..flights {
        let base = bottom + (i * FLIGHT_STEPS) as f32 * rise;
        let forward = i % 2 == 0;
        // Even flights climb toward +u in one lane, odd ones come back in the other.
        let lane = if forward {
            (-WELL.1, -SPINE * 0.5)
        } else {
            (SPINE * 0.5, WELL.1)
        };
        let rail = if forward {
            (lane.0, lane.0 + RAIL_THICKNESS)
        } else {
            (lane.1 - RAIL_THICKNESS, lane.1)
        };
        for j in 0..FLIGHT_STEPS {
            let step_top = base + (j + 1) as f32 * rise;
            let (a, b) = (j as f32 * tread, (j + 1) as f32 * tread);
            let u = if forward {
                (-half_run + a, -half_run + b)
            } else {
                (half_run - b, half_run - a)
            };
            push(
                u,
                lane,
                (step_top - rise, step_top),
                BlockKind::Stair,
                stone,
            );
            push(
                u,
                rail,
                (step_top, step_top + RAIL_HEIGHT),
                BlockKind::Steel,
                steel,
            );
        }
        // The landing at the end of the flight, railed on its open sides (the last one is level
        // with the deck around the stairwell).
        let end = base + FLIGHT_STEPS as f32 * rise;
        let u = if forward {
            (half_run, WELL.0)
        } else {
            (-WELL.0, -half_run)
        };
        push(
            u,
            (-WELL.1, WELL.1),
            (end - LANDING_THICKNESS, end),
            BlockKind::Stair,
            stone,
        );
        if i + 1 < flights {
            let y = (end, end + RAIL_HEIGHT);
            let edge = if forward {
                (WELL.0 - RAIL_THICKNESS, WELL.0)
            } else {
                (-WELL.0, -WELL.0 + RAIL_THICKNESS)
            };
            push(edge, (-WELL.1, WELL.1), y, BlockKind::Steel, steel);
            push(
                u,
                (-WELL.1, -WELL.1 + RAIL_THICKNESS),
                y,
                BlockKind::Steel,
                steel,
            );
            push(
                u,
                (WELL.1 - RAIL_THICKNESS, WELL.1),
                y,
                BlockKind::Steel,
                steel,
            );
        }
    }
    // The wall between the two lanes.
    push(
        (-half_run, half_run),
        (-SPINE * 0.5, SPINE * 0.5),
        (bottom, top),
        BlockKind::Parapet,
        wall,
    );
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
    let tall = top > floors::y(START) + 30.0;
    if tall && rng.chance(0.7) {
        let tier = rng.range(0.25, 0.4) * (top - floors::y(START));
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
        if top > floors::top() + 20.0 && rng.chance(0.5) {
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
    let y_lo = floors::y(1) + 6.0;
    let y_hi = min_top - 3.0;
    if y_hi <= y_lo {
        return;
    }
    let y = (rng.range(y_lo, y_hi) / 6.0).round() * 6.0;
    let y = y.clamp(y_lo, y_hi);
    // Not level with a floor, where it would run through the decks.
    if (0..floors::count()).any(|f| (y - floors::y(f)).abs() < 4.0) {
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
        let c = vec3(-8.0 + i as f32 * 4.0, floors::y(START) + h * 0.5, 10.0);
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
}
