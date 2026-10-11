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
use super::floors::{self, START};
use super::planner::{self, Placement, Role};
use super::props::Placed;
use super::{Aabb, Block, BlockKind};
use crate::rng::{Rng, hash_coords};

/// Side of a square lot.
pub const LOT: f32 = 24.0;

const DECK_THICKNESS: f32 = 1.2;
const BRIDGE_WIDTH: f32 = 4.0;

/// Salts keeping the per-lot random streams independent.
const SALT_BRIDGE: i32 = 5;
const SALT_STAIR: i32 = 7;
const SALT_DECK_TINT: i32 = 11;
const SALT_CHUNK: i32 = 13;

/// Side, in lots, of the squares whose slabs share a tint on a floor (poured together).
const POUR: i32 = 3;

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

/// The floor above which nothing is built on this lot, if it lies in a crater's shape
/// (`planner::crater_placement`).
pub fn crater_at(seed: u64, lx: i32, lz: i32) -> Option<usize> {
    planner::crater_placement(seed, lx, lz).map(|p| p.floor)
}

/// Whether a hole or a crater opens this lot's deck on a floor (it has none there).
pub fn opened(seed: u64, lx: i32, lz: i32, floor: usize) -> bool {
    planner::hole_at(seed, lx, lz).is_some_and(|f| floor >= f)
        || crater_at(seed, lx, lz).is_some_and(|c| floor > c)
}

/// The floors a lot's stair links (bottom, top = bottom + 1), if it has one. Every stair stands in
/// the middle lot of a stair hub chunk placed by the planner (`planner.rs`), at most one per hub
/// cell and floor, at the floor's profile chance (`design.rs`): common in the depths, rare high up,
/// so climbing gets harder and harder.
pub fn stair(seed: u64, lx: i32, lz: i32) -> Option<(usize, usize)> {
    planner::stair_hub(seed, lx, lz).map(|p| (p.floor, p.floor + 1))
}

/// What covers a lot on a floor. Pure, so the renderer can map the decks around the camera.
pub fn deck(seed: u64, lx: i32, lz: i32, floor: usize) -> Deck {
    // A previewed chunk stands on a bare deck at the start floor, open where it has holes.
    if chunks::preview().is_some() {
        let open = floor == START && preview_opens(lx, lz);
        return if (floor == 0 || floor == START) && !open {
            Deck::Full
        } else {
            Deck::None
        };
    }
    if floor == 0 || (floor == START && is_plaza(lx, lz)) {
        return Deck::Full;
    }
    // A hole chunk (a light well, a gap, the unbuilt edge) opens its floor and every floor above.
    if planner::hole_at(seed, lx, lz).is_some_and(|f| floor >= f) {
        return Deck::None;
    }
    // An area's or a block's floor is a full deck under it.
    if planner::area_at(seed, lx, lz).is_some_and(|a| a.floor == floor)
        || planner::blocks_at(seed, lx, lz)
            .iter()
            .any(|b| b.floor == floor)
    {
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
    Deck::Full
}

/// Whether the previewed chunk (`--chunk`) has a hole at this lot.
fn preview_opens(lx: i32, lz: i32) -> bool {
    let Some((origin, _)) = chunks::preview_footprint() else {
        return false;
    };
    let Some(p) = chunks::preview() else {
        return false;
    };
    chunks::catalog(p.biome)
        .chunks
        .get(&p.name)
        .is_some_and(|c| {
            c.placed_holes(origin, 0)
                .iter()
                .any(|&((x0, z0), (x1, z1))| lx >= x0 && lx < x1 && lz >= z0 && lz < z1)
        })
}

/// One bit per floor above the ground (bit `f - 1` for floor `f`), set where the lot has a deck.
pub fn cover_bits(seed: u64, lx: i32, lz: i32) -> u16 {
    (1..floors::count())
        .filter(|&f| deck(seed, lx, lz, f) != Deck::None)
        .fold(0, |bits, f| bits | 1 << (f - 1))
}

/// The tower standing on a lot, if any (`planner::tower_at`): its base tier's footprint from the
/// bottom of the stack to its top. Pure, so neighbors can query it for bridges.
pub fn tower(seed: u64, lx: i32, lz: i32) -> Option<Tower> {
    let p = planner::tower_at(seed, lx, lz)?;
    let b = super::tower::body(&chunks::catalog(p.biome), &p)?;
    Some(Tower {
        center: (b.min + b.max) * 0.5,
        size: b.size(),
    })
}

/// Appends every block belonging to one lot, prop collision boxes included.
pub fn generate_lot(seed: u64, lx: i32, lz: i32, out: &mut Vec<Block>) {
    generate_lot_dressed(seed, lx, lz, out);
}

/// What a lot holds besides its blocks: its props (drawn instanced) and the flowers of its points
/// of interest.
#[derive(Default, Clone)]
pub struct Dressed {
    pub props: Vec<Placed>,
    pub blooms: Vec<Vec3>,
}

/// Lots generated lately, kept since a lot's growth needs its neighbors (`growth::grow_lot`):
/// keyed by seed, lot, catalog generation and switches.
type LotKey = (u64, i32, i32, u64, bool, bool);
#[derive(Default)]
struct LotCache {
    lots: std::collections::HashMap<LotKey, std::sync::Arc<(Vec<Block>, Dressed)>>,
    order: std::collections::VecDeque<LotKey>,
}

/// Lots the cache keeps (a little more than the loaded square and its border).
const CACHED_LOTS: usize = 400;

/// `generate_lot`, also returning the lot's props and the flowers of its points of interest.
pub fn generate_lot_dressed(seed: u64, lx: i32, lz: i32, out: &mut Vec<Block>) -> Dressed {
    static CACHE: std::sync::OnceLock<std::sync::Mutex<LotCache>> = std::sync::OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    let key = (
        seed,
        lx,
        lz,
        chunks::generation(),
        super::design::props_on(),
        super::design::pois_on(),
    );
    if let Some(lot) = cache.lock().unwrap().lots.get(&key).cloned() {
        out.extend_from_slice(&lot.0);
        return lot.1.clone();
    }
    let first = out.len();
    let dressed = generate_lot_uncached(seed, lx, lz, out);
    let mut c = cache.lock().unwrap();
    if c.lots.len() >= CACHED_LOTS
        && let Some(old) = c.order.pop_front()
    {
        c.lots.remove(&old);
    }
    let lot = std::sync::Arc::new((out[first..].to_vec(), dressed.clone()));
    if c.lots.insert(key, lot).is_none() {
        c.order.push_back(key);
    }
    dressed
}

fn generate_lot_uncached(seed: u64, lx: i32, lz: i32, out: &mut Vec<Block>) -> Dressed {
    let mut dressed = Dressed::default();
    generate_structure(seed, lx, lz, out, &mut dressed);
    out.extend(dressed.props.iter().map(|p| Block {
        kind: BlockKind::Prop,
        aabb: p.aabb(),
        color: Color::new(0, 0, 0, 255),
    }));
    dressed
}

/// The lot's architecture: ground, decks, stairs, tower, bridges, chunks; the props and flowers of
/// its tower and chunks go to `dressed`.
fn generate_structure(seed: u64, lx: i32, lz: i32, out: &mut Vec<Block>, dressed: &mut Dressed) {
    let first = out.len();
    let o = lot_origin(lx, lz);
    let mid = o + vec3(LOT * 0.5, 0.0, LOT * 0.5);

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
        // One tint per few lots and floor: slabs were poured a few at a time.
        let t = floors::depth01(y);
        let lo = 60 + (36.0 * t) as i32;
        let (px, pz) = (lx.div_euclid(POUR), lz.div_euclid(POUR));
        let color = tint(
            &mut Rng::new(hash_coords(seed, px * 16 + f as i32, SALT_DECK_TINT, pz)),
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

    push_tower(seed, lx, lz, first, out, dressed);

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
            BlockKind::Floor | BlockKind::Deck | BlockKind::Tower(_) | BlockKind::Stair
        );
        !ours || !prop || !decks.iter().any(|d| d.overlaps(&b.aabb))
    });

    push_chunks(seed, lx, lz, first, out, dressed);
}

/// Pieces of a chunk or a tower in world coordinates: boxes, props and flowers, each in a group (a
/// module and its boxes stand or give way together).
struct Pieces {
    blocks: Vec<(Block, u32)>,
    props: Vec<(Placed, u32)>,
    blooms: Vec<(Vec3, u32)>,
}

impl Pieces {
    /// The groups with a piece overlapping one of `obstacles`.
    fn blocked(&self, obstacles: &[Aabb], test: impl Fn(&Block) -> bool) -> Vec<u32> {
        let boxes = self
            .blocks
            .iter()
            .filter(|(b, _)| test(b))
            .map(|(b, g)| (b.aabb, *g));
        let props = self.props.iter().map(|(p, g)| (p.aabb(), *g));
        boxes
            .chain(props)
            .filter(|(a, _)| obstacles.iter().any(|o| o.overlaps(a)))
            .map(|(_, g)| g)
            .collect()
    }

    /// Adds this lot's share: the pieces of the groups not `dropped` whose center falls in the
    /// lot, and collision for those reaching in from the lots that own them.
    fn place(
        self,
        (lx, lz): (i32, i32),
        dropped: &[u32],
        out: &mut Vec<Block>,
        dressed: &mut Dressed,
    ) {
        let o = lot_origin(lx, lz);
        let lot = Aabb {
            min: vec3(o.x, f32::MIN, o.z),
            max: vec3(o.x + LOT, f32::MAX, o.z + LOT),
        };
        let owns = |c: Vec3| (c.x / LOT).floor() as i32 == lx && (c.z / LOT).floor() as i32 == lz;
        for (b, g) in self.blocks {
            if dropped.contains(&g) || !b.aabb.overlaps(&lot) {
                continue;
            }
            out.push(if owns((b.aabb.min + b.aabb.max) * 0.5) {
                b
            } else {
                Block {
                    kind: BlockKind::Collider,
                    ..b
                }
            });
        }
        for (p, g) in self.props {
            if dropped.contains(&g) {
                continue;
            }
            if owns(p.pos) {
                dressed.props.push(p);
            } else if p.aabb().overlaps(&lot) {
                out.push(Block {
                    kind: BlockKind::Collider,
                    aabb: p.aabb(),
                    color: Color::new(0, 0, 0, 255),
                });
            }
        }
        for (b, g) in self.blooms {
            if !dropped.contains(&g) && owns(b) {
                dressed.blooms.push(b);
            }
        }
    }
}

/// This lot's share of the tower standing on it (`tower.rs`). What hangs on the walls or stands
/// on a roof ending just under a deck would pierce it: those give way.
fn push_tower(
    seed: u64,
    lx: i32,
    lz: i32,
    first: usize,
    out: &mut Vec<Block>,
    dressed: &mut Dressed,
) {
    let Some(p) = planner::tower_at(seed, lx, lz) else {
        return;
    };
    let cat = chunks::catalog(p.biome);
    let has_deck = |(x, z): (i32, i32), f: usize| deck(seed, x, z, f) != Deck::None;
    let built = super::tower::build(&cat, &p, &has_deck);
    let pieces = Pieces {
        blocks: built.blocks.into_iter().zip(built.groups).collect(),
        props: built.props.into_iter().zip(built.prop_groups).collect(),
        blooms: built.blooms.into_iter().zip(built.bloom_groups).collect(),
    };
    let decks: Vec<Aabb> = out[first..]
        .iter()
        .filter(|b| b.kind == BlockKind::Deck)
        .map(|b| b.aabb)
        .collect();
    let dropped = pieces.blocked(&decks, |b| !matches!(b.kind, BlockKind::Tower(_)));
    pieces.place((lx, lz), &dropped, out, dressed);
}

/// The chunks covering a lot: the previewed one (`--chunk`), or the planner's (`planner.rs`).
pub fn placements(seed: u64, lx: i32, lz: i32) -> Vec<Placement> {
    if let Some(((ox, oz), size)) = chunks::preview_footprint() {
        let p = Placement {
            chunk: chunks::preview()
                .map(|p| p.name.clone())
                .unwrap_or_default(),
            biome: chunks::preview().map_or(chunks::DEFAULT_BIOME, |p| p.biome),
            role: Role::Area,
            origin: (ox, oz),
            size,
            floor: START,
            turn: 0,
            seed: hash_coords(seed, ox, SALT_CHUNK, oz),
            stories: 0,
        };
        let mut all = planner::subs_of(&p);
        all.push(p);
        all.retain(|p| p.covers(lx, lz));
        return all;
    }
    planner::covering(seed, lx, lz)
}

/// A placed chunk, built in world coordinates from its biome's collection, on its floor of the
/// city (`chunks::Env`).
fn build_placement(seed: u64, p: &Placement) -> Option<(chunks::Chunk, chunks::Built)> {
    let cat = chunks::catalog(p.biome);
    let chunk = cat.chunks.get(&p.chunk)?.clone();
    let origin = lot_origin(p.origin.0, p.origin.1) + Vec3::Y * floors::y(p.floor);
    let has_deck = |(x, z): (i32, i32)| deck(seed, x, z, p.floor) != Deck::None;
    let env = chunks::Env {
        floor: p.floor,
        deck: &has_deck,
    };
    let built = chunks::build_in(&chunk, &cat, origin, p.turn, p.seed, Some(&env));
    Some((chunk, built))
}

/// The vegetation hints (`chunks::GrowthKind`) of the chunks covering this lot.
pub fn growth_hints(seed: u64, lx: i32, lz: i32) -> Vec<(Aabb, chunks::GrowthKind)> {
    placements(seed, lx, lz)
        .iter()
        .filter_map(|p| build_placement(seed, p))
        .flat_map(|(_, b)| b.growth)
        .collect()
}

/// This lot's part of the chunks covering it (`Pieces::place`). A chunk's pieces give way, a
/// module at a time, to the lot's own architecture (a tower, a stair) and to the towers next door.
fn push_chunks(
    seed: u64,
    lx: i32,
    lz: i32,
    first: usize,
    out: &mut Vec<Block>,
    dressed: &mut Dressed,
) {
    let mut obstacles: Vec<Aabb> = out[first..]
        .iter()
        .filter(|b| !matches!(b.kind, BlockKind::Floor | BlockKind::Deck))
        .map(|b| b.aabb)
        .collect();
    obstacles.extend(dressed.props.iter().map(|p| p.aabb()));
    for (nx, nz) in [(lx - 1, lz), (lx + 1, lz), (lx, lz - 1), (lx, lz + 1)] {
        if let Some(t) = tower(seed, nx, nz) {
            obstacles.push(Aabb::from_center(t.center, t.size));
        }
    }
    // The greater chunks first: the lesser give way to what they placed (a street to its filler).
    let mut all = placements(seed, lx, lz);
    all.sort_by_key(|p| p.rank());
    for p in &all {
        // Where two chunks cover the lot on one floor, the lesser gives way (`outranks`); every
        // chunk needs a deck to stand on, but an area, and a hole over its own void (a footbridge
        // across it, girders sticking out). A hole's rim where a hole below left no deck has
        // nothing.
        let open = p.role == Role::Area || (p.role == Role::Hole && p.opens(lx, lz));
        if all.iter().any(|q| q.floor == p.floor && q.outranks(p))
            || (!open && deck(seed, lx, lz, p.floor) == Deck::None)
        {
            continue;
        }
        let Some((chunk, built)) = build_placement(seed, p) else {
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
        let mut blocks: Vec<(Block, u32)> = built
            .boxes
            .iter()
            .zip(&built.groups)
            .map(|(&(aabb, mat), &g)| {
                let b = Block {
                    kind: BlockKind::Built(mat),
                    aabb,
                    color: mat.color(),
                };
                (b, g)
            })
            .collect();
        let stair_group = built
            .groups
            .iter()
            .chain(&built.prop_groups)
            .max()
            .map_or(0, |g| g + 1);
        for &(at, along_x) in &built.stairs {
            let mut stair = Vec::new();
            push_switchback_stair(at, along_x, at.y, at.y + floors::SPACING, &mut stair);
            blocks.extend(stair.into_iter().map(|b| (b, stair_group)));
        }
        let pieces = Pieces {
            blocks,
            props: built.props.into_iter().zip(built.prop_groups).collect(),
            blooms: built.blooms.into_iter().zip(built.bloom_groups).collect(),
        };
        let dropped = pieces.blocked(&obstacles, |_| true);
        obstacles.extend(
            pieces
                .blocks
                .iter()
                .filter(|(_, g)| !dropped.contains(g))
                .map(|(b, _)| b.aabb),
        );
        obstacles.extend(
            pieces
                .props
                .iter()
                .filter(|(_, g)| !dropped.contains(g))
                .map(|(p, _)| p.aabb()),
        );
        pieces.place((lx, lz), &dropped, out, dressed);
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

/// Length of a segment of the `skybridge_segment` module.
const SKYBRIDGE_STEP: f32 = 2.0;

/// A walkway with low parapets between two towers (`skybridge_segment` modules from wall to wall),
/// at a height the seed picks between their tops. `along_x` is the direction from `a` to `b`.
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
    // Segments of the walkway module from wall to wall (the last one sinks into the far wall).
    let n = ((to - from) / SKYBRIDGE_STEP).ceil() as u32;
    let start = if along_x {
        vec3(from, y, c)
    } else {
        vec3(c, y, from)
    };
    let tf = chunks::Tf::shift(start).then(&chunks::Tf::turn(if along_x { 0 } else { 3 }));
    let parts = [chunks::Part::Repeat {
        from: (0.0, 0.0, 0.0),
        step: (SKYBRIDGE_STEP, 0.0, 0.0),
        count: n,
        parts: vec![chunks::Part::Module {
            name: "skybridge_segment".to_string(),
            at: (0.0, 0.0, 0.0),
            turn: 0,
            accent: None,
        }],
    }];
    let built = chunks::expand(&chunks::shared(), &parts, &tf, 0);
    out.extend(built.boxes.into_iter().map(|(aabb, mat)| Block {
        kind: BlockKind::Built(mat),
        aabb,
        color: mat.color(),
    }));
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

    #[test]
    fn the_start_plaza_has_its_point_of_interest() {
        let mut blocks = Vec::new();
        let blooms = generate_lot_dressed(1, -1, -1, &mut blocks).blooms;
        assert!(
            blooms
                .iter()
                .any(|b| b.distance(vec3(-7.0, floors::y(START) + 0.5, -20.0)) < 0.5),
            "no flower in the start's armchair: {blooms:?}"
        );
    }

    #[test]
    fn street_lights_stand_at_the_crossroads() {
        let mut n = 0;
        for lz in -3..=3 {
            for lx in -3..=3 {
                let mut blocks = Vec::new();
                let dressed = generate_lot_dressed(1, lx, lz, &mut blocks);
                let o = lot_origin(lx, lz);
                for p in &dressed.props {
                    if p.prop != super::super::props::Prop::StreetLight {
                        continue;
                    }
                    n += 1;
                    // The pole, off the corner of a crossing by the same gap on both roads.
                    let a = p.aabb();
                    let pole = (a.min + a.max) * 0.5 - o;
                    let near = |v: f32| (v - 3.5).abs() < 0.3 || (v - LOT + 3.5).abs() < 0.3;
                    assert!(
                        near(pole.x) && near(pole.z),
                        "lot {lx},{lz}: pole at {pole}"
                    );
                }
            }
        }
        // About one per crossing and floor with a deck there.
        assert!(n > 49 * 3, "only {n} street lights on 49 lots");
    }

    #[test]
    fn the_depths_are_dressed() {
        let mut n = 0;
        for lz in -2..2 {
            for lx in 2..6 {
                let mut blocks = Vec::new();
                let dressed = generate_lot_dressed(2, lx, lz, &mut blocks);
                n += dressed
                    .props
                    .iter()
                    .filter(|p| p.pos.y <= floors::y(START) + 0.1)
                    .count();
            }
        }
        assert!(n >= 16 * 8, "only {n} props on the depths of 16 lots");
    }
}
