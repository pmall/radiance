//! The planner (`docs/chunks.md`): which hand-written chunk stands where, from the seed and the
//! placement rules. Every answer is a pure function of the seed and the lot, so the city still
//! streams lot by lot. From the biggest pieces to the smallest:
//!
//! - **Areas** (a great square, a market district): the plane is cut into cells of `AREA_CELL`
//!   lots; each cell holds at most one area, entirely inside it, on one floor of the area's zones.
//!   Areas sharing a label keep their `spacing` (the cell holding the lower-priority one gives
//!   way). No tower stands in an area's footprint, on any floor.
//! - **Sub-areas:** the `Sub` openings an area leaves are filled with chunks of the labels they ask
//!   for (a garden, a café terrace in a quarter of the great square), each its own seed.
//! - **Blocks** (a pocket park, a laundry court): cells of `BLOCK_CELL` lots hold at most one per
//!   floor, at the floor's `blocks` chance (`design.rs`), entirely inside the cell, on a deck, away
//!   from areas, craters and holes.
//! - **Holes** (a light well, a gap a footbridge crosses, the unbuilt edge of the construction
//!   front): cells of `HOLE_CELL` lots hold at most one per floor, at the floor's `holes` chance; a
//!   hole chunk's `holes` open its floor and every floor above, so the city thins out going up.
//! - **Craters:** cells of `CRATER_CELL` lots hold one now and then, its shape open above its floor;
//!   one always opens in front of the spawn.
//! - **Stair hubs:** every stair stands in the middle lot of a hub chunk, which frames it (railings,
//!   lamps, benches). Cells of `HUB_CELL` lots hold at most one hub per floor, at the floor's
//!   `stair` chance, so stairs never cluster.
//! - **Fillers:** every lot left free on a floor (a deck, no tower through it, no other chunk) may
//!   hold a one-lot filler, at the floor's `fillers` chance: the housing and yards that make the
//!   city dense.
//!
//! - **Towers:** cells of `TOWER_CELL` lots hold a big tower now and then (2x1, 2x2 lots), the
//!   other lots a one-lot tower unless the biome leaves them empty; never in an area, a crater, the
//!   start's plaza or its stair lot. Each gets a height (`design::TOWER_HEIGHTS`).
//!
//! The rest of a lot (its ground, decks, stair and bridges) is built by `city.rs`.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use super::biome;
pub use super::chunks::Role;
use super::chunks::{self, Catalog, Chunk};
use super::city;
use super::floors::START;
use super::{design, floors};
use crate::rng::{Rng, hash_coords};

/// Side of an area cell, in lots.
pub const AREA_CELL: i32 = 32;
/// Side of a block cell, in lots.
pub const BLOCK_CELL: i32 = 8;
/// Side of a stair hub cell, in lots.
pub const HUB_CELL: i32 = 6;
/// Chance that an area cell holds an area.
const AREA_CHANCE: f32 = 0.6;
/// Spots a block cell tries before giving up.
const BLOCK_TRIES: usize = 10;

const SALT_AREA: i32 = 20;
const SALT_HUB: i32 = 21;
const SALT_BLOCK: i32 = 22;
const SALT_SUB: i32 = 23;
const SALT_FILLER: i32 = 24;
const SALT_TOWER: i32 = 25;
const SALT_STREET: i32 = 26;
const SALT_HOLE: i32 = 27;
const SALT_CRATER: i32 = 28;

/// Side of a hole cell, in lots.
pub const HOLE_CELL: i32 = 6;
/// Spots a hole cell tries before giving up.
const HOLE_TRIES: usize = 8;
/// Side of a crater cell, in lots.
pub const CRATER_CELL: i32 = 8;
/// The crater always opening in front of the spawn.
pub const START_CRATER: &str = "crater_start";

/// The chunk of the plaza where the player wakes up.
pub const START_CHUNK: &str = "start_plaza";

/// Side of a tower cell, in lots: the biggest tower footprint.
pub const TOWER_CELL: i32 = 2;

/// A chunk placed in the city.
#[derive(Clone, Debug)]
pub struct Placement {
    pub chunk: String,
    /// The biome whose collection the chunk comes from (`chunks::catalog`).
    pub biome: &'static str,
    pub role: Role,
    /// Corner lot of the footprint, and its size in lots (after the turn).
    pub origin: (i32, i32),
    pub size: (i32, i32),
    pub floor: usize,
    /// Quarter turns.
    pub turn: u8,
    /// Seeds the chunk's slots.
    pub seed: u64,
    /// A tower's height, in stories from the bottom of the stack (0 for other chunks).
    pub stories: u32,
}

impl Placement {
    pub fn covers(&self, lx: i32, lz: i32) -> bool {
        lx >= self.origin.0
            && lz >= self.origin.1
            && lx < self.origin.0 + self.size.0
            && lz < self.origin.1 + self.size.1
    }

    fn center(&self) -> (f32, f32) {
        (
            self.origin.0 as f32 + self.size.0 as f32 * 0.5,
            self.origin.1 as f32 + self.size.1 as f32 * 0.5,
        )
    }

    /// The middle lot of the footprint: where a hub's stair stands.
    fn middle(&self) -> (i32, i32) {
        (
            self.origin.0 + self.size.0 / 2,
            self.origin.1 + self.size.1 / 2,
        )
    }

    fn lots(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        let (x0, z0) = self.origin;
        (z0..z0 + self.size.1).flat_map(move |z| (x0..x0 + self.size.0).map(move |x| (x, z)))
    }

    /// Whether one of the chunk's holes opens this lot.
    pub fn opens(&self, lx: i32, lz: i32) -> bool {
        chunks::catalog(self.biome)
            .chunks
            .get(&self.chunk)
            .is_some_and(|c| {
                c.placed_holes(self.origin, self.turn)
                    .iter()
                    .any(|&((x0, z0), (x1, z1))| lx >= x0 && lx < x1 && lz >= z0 && lz < z1)
            })
    }

    /// Its precedence where chunks cover a lot on one floor (lower first): areas and their
    /// sub-areas, then blocks, then hubs' surroundings, then fillers, then streets.
    pub fn rank(&self) -> u8 {
        match self.role {
            Role::Area | Role::Sub => 0,
            Role::Block | Role::Hole | Role::Crater => 1,
            Role::Hub => 2,
            Role::Filler => 3,
            Role::Street => 4,
            Role::Tower => 5,
        }
    }

    /// Whether this placement takes precedence over `other` where both cover a lot on one floor.
    /// A street and a filler share their lot (the street keeps its border), and so does a tower.
    pub fn outranks(&self, other: &Placement) -> bool {
        let shared = |r: Role| matches!(r, Role::Filler | Role::Street | Role::Tower);
        !(shared(self.role) && shared(other.role)) && self.rank() < other.rank()
    }
}

/// The biome a decision covering a cell is made in: the biome of the cell's middle lot.
fn biome_of_cell(seed: u64, cx: i32, cz: i32, side: i32) -> &'static str {
    biome::biome_at(seed, cx * side + side / 2, cz * side + side / 2).name
}

fn zone_fits(chunk: &Chunk, floor: usize) -> bool {
    let z = match floors::zone(floor) {
        floors::Zone::Depths => chunks::Zone::Depths,
        floors::Zone::Middle => chunks::Zone::Middle,
        floors::Zone::Canopy => chunks::Zone::Canopy,
    };
    chunk.zones.is_empty() || chunk.zones.contains(&z)
}

/// The catalog's chunks of a role, in a fixed order (the catalog is a hash map).
fn pool(cat: &Catalog, keep: impl Fn(&Chunk) -> bool) -> Vec<&Chunk> {
    let mut v: Vec<&Chunk> = cat.chunks.values().filter(|c| keep(c)).collect();
    v.sort_by(|a, b| a.name.cmp(&b.name));
    v
}

/// A chunk among `pool`, by weight.
fn pick<'a>(pool: &[&'a Chunk], rng: &mut Rng) -> Option<&'a Chunk> {
    let total: u32 = pool.iter().map(|c| c.weight).sum();
    if total == 0 {
        return None;
    }
    let mut roll = rng.range_i(0, total as i32) as u32;
    pool.iter().copied().find(|c| {
        let hit = roll < c.weight;
        roll = roll.saturating_sub(c.weight);
        hit
    })
}

/// A chunk's footprint once turned.
pub fn turned(size: (i32, i32), turn: u8) -> (i32, i32) {
    if turn % 2 == 1 {
        (size.1, size.0)
    } else {
        size
    }
}

/// Lots the start keeps as they are on every floor: the spawn plaza and the crater in front of it.
fn near_start(lx: i32, lz: i32) -> bool {
    (-4..=4).contains(&lx) && (-10..=2).contains(&lz)
}

/// `near_start`, and the start's stair hub around `START_HUB`.
fn near_start_or_hub(lx: i32, lz: i32) -> bool {
    near_start(lx, lz) || ((lx - START_HUB.0).abs() <= 1 && (lz - START_HUB.1).abs() <= 1)
}

/// Cell answers, keyed by (catalog generation, seed, kind, cell x, cell z, floor): the planner is
/// asked about the same cells over and over (every deck and stair query).
type Cache = Mutex<HashMap<(u64, u64, u8, i32, i32, usize), Vec<Placement>>>;

/// Cell answers the cache keeps before starting over (exploring keeps adding cells).
const CACHED_CELLS: usize = 50_000;

fn cached(
    key: (u8, i32, i32, usize),
    seed: u64,
    compute: impl FnOnce() -> Vec<Placement>,
) -> Vec<Placement> {
    static CACHE: OnceLock<Cache> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    let full = (chunks::generation(), seed, key.0, key.1, key.2, key.3);
    if let Some(p) = cache.lock().unwrap().get(&full) {
        return p.clone();
    }
    let p = compute();
    let mut c = cache.lock().unwrap();
    if c.len() >= CACHED_CELLS {
        c.clear();
    }
    c.insert(full, p.clone());
    p
}

// Areas.

/// The area a cell proposes, before spacing between cells is settled.
fn area_candidate(seed: u64, cx: i32, cz: i32) -> Option<(Placement, f32)> {
    let biome = biome_of_cell(seed, cx, cz, AREA_CELL);
    let cat = chunks::catalog(biome);
    let mut rng = Rng::new(hash_coords(seed, cx, SALT_AREA, cz));
    if !rng.chance(AREA_CHANCE) {
        return None;
    }
    let pool = pool(&cat, |c| {
        c.role() == Role::Area && c.size.0.max(c.size.1) <= AREA_CELL
    });
    let chunk = pick(&pool, &mut rng)?;
    let floors: Vec<usize> = (0..floors::count() - 1)
        .filter(|&f| zone_fits(chunk, f))
        .collect();
    if floors.is_empty() {
        return None;
    }
    let floor = floors[rng.range_i(0, floors.len() as i32) as usize];
    let turn = rng.range_i(0, 4) as u8;
    let size = turned(chunk.size, turn);
    let origin = (
        cx * AREA_CELL + rng.range_i(0, AREA_CELL - size.0 + 1),
        cz * AREA_CELL + rng.range_i(0, AREA_CELL - size.1 + 1),
    );
    let p = Placement {
        chunk: chunk.name.clone(),
        biome,
        role: Role::Area,
        origin,
        size,
        floor,
        turn,
        seed: rng.next_u64(),
        stories: 0,
    };
    // Never over the start. Craters give way to areas (`city::crater_at`).
    let clear = p.lots().all(|(x, z)| !near_start(x, z));
    clear.then(|| (p, rng.f32()))
}

/// The area of a cell, spacing settled: of two areas sharing a label closer than their spacing,
/// the one with the lower priority roll gives way.
fn area_of_cell(seed: u64, cx: i32, cz: i32) -> Option<Placement> {
    cached((0, cx, cz, 0), seed, || {
        area_of_cell_uncached(seed, cx, cz).into_iter().collect()
    })
    .pop()
}

fn area_of_cell_uncached(seed: u64, cx: i32, cz: i32) -> Option<Placement> {
    let (p, prio) = area_candidate(seed, cx, cz)?;
    let cat = chunks::catalog(p.biome);
    let chunk = cat.chunks.get(&p.chunk)?;
    let reach = (chunk.spacing as i32 / AREA_CELL) + 1;
    for nz in cz - reach..=cz + reach {
        for nx in cx - reach..=cx + reach {
            if (nx, nz) == (cx, cz) {
                continue;
            }
            let Some((q, qprio)) = area_candidate(seed, nx, nz) else {
                continue;
            };
            let qcat = chunks::catalog(q.biome);
            let Some(other) = qcat.chunks.get(&q.chunk) else {
                continue;
            };
            let shared = chunk.labels.iter().any(|l| other.labels.contains(l));
            let (a, b) = (p.center(), q.center());
            let d = ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt();
            let spacing = chunk.spacing.max(other.spacing) as f32;
            if shared && d < spacing && (qprio, (nx, nz)) > (prio, (cx, cz)) {
                return None;
            }
        }
    }
    Some(p)
}

/// The area covering a lot, on whatever floor it stands.
pub fn area_at(seed: u64, lx: i32, lz: i32) -> Option<Placement> {
    if chunks::preview().is_some() {
        return None;
    }
    let (cx, cz) = (lx.div_euclid(AREA_CELL), lz.div_euclid(AREA_CELL));
    area_of_cell(seed, cx, cz).filter(|p| p.covers(lx, lz))
}

/// The chunks filling an area's `Sub` openings: for each, a chunk carrying one of the labels it
/// asks for that fits in it (turned if need be), centered in it on whole lots.
fn subs_of_area(seed: u64, area: &Placement) -> Vec<Placement> {
    let (cx, cz) = (
        area.origin.0.div_euclid(AREA_CELL),
        area.origin.1.div_euclid(AREA_CELL),
    );
    cached((2, cx, cz, 0), seed, || subs_of(area))
}

/// `subs_of_area`, uncached: for any placement (the `--chunk` preview fills its openings too).
/// Sub-areas come from the area's biome.
pub fn subs_of(area: &Placement) -> Vec<Placement> {
    let cat = chunks::catalog(area.biome);
    let Some(chunk) = cat.chunks.get(&area.chunk) else {
        return Vec::new();
    };
    let origin =
        city::lot_origin(area.origin.0, area.origin.1) + glam::Vec3::Y * floors::y(area.floor);
    let built = chunks::build(chunk, &cat, origin, area.turn, area.seed);
    let mut out = Vec::new();
    for (i, (rect, labels)) in built.subs.iter().enumerate() {
        let lot = |v: f32| (v / city::LOT + 0.01).floor() as i32;
        let (x0, z0) = (lot(rect.min.x), lot(rect.min.z));
        let (x1, z1) = (lot(rect.max.x), lot(rect.max.z));
        let (w, d) = (x1 - x0, z1 - z0);
        let mut rng = Rng::new(hash_coords(area.seed, i as i32, SALT_SUB, 0));
        let pool = pool(&cat, |c| {
            c.labels.iter().any(|l| labels.contains(l))
                && zone_fits(c, area.floor)
                && ((c.size.0 <= w && c.size.1 <= d) || (c.size.1 <= w && c.size.0 <= d))
        });
        let Some(sub) = pick(&pool, &mut rng) else {
            continue;
        };
        let turns: Vec<u8> = (0..4)
            .filter(|&t| {
                let s = turned(sub.size, t);
                s.0 <= w && s.1 <= d
            })
            .collect();
        let turn = turns[rng.range_i(0, turns.len() as i32) as usize];
        let size = turned(sub.size, turn);
        out.push(Placement {
            chunk: sub.name.clone(),
            biome: area.biome,
            role: Role::Sub,
            origin: (x0 + (w - size.0) / 2, z0 + (d - size.1) / 2),
            size,
            floor: area.floor,
            turn,
            seed: rng.next_u64(),
            stories: 0,
        });
    }
    out
}

// Blocks.

/// The block a cell holds on a floor, if any.
fn block_of_cell(seed: u64, cx: i32, cz: i32, floor: usize) -> Option<Placement> {
    cached((3, cx, cz, floor), seed, || {
        block_of_cell_uncached(seed, cx, cz, floor)
            .into_iter()
            .collect()
    })
    .pop()
}

fn block_of_cell_uncached(seed: u64, cx: i32, cz: i32, floor: usize) -> Option<Placement> {
    if floor + 1 >= floors::count() {
        return None;
    }
    let biome = biome_of_cell(seed, cx, cz, BLOCK_CELL);
    let cat = chunks::catalog(biome);
    let mut rng = Rng::new(hash_coords(seed, cx, SALT_BLOCK + 64 * floor as i32, cz));
    if !rng.chance(design::profile(floor).blocks) {
        return None;
    }
    // The ground floor holds places but no light well: nothing lies under it.
    let pool = pool(&cat, |c| {
        c.role() == Role::Block
            && zone_fits(c, floor)
            && c.size.0.max(c.size.1) <= BLOCK_CELL
            && (floor > 0 || c.holes.is_empty())
    });
    for _ in 0..BLOCK_TRIES {
        // Another place each try: a big one may not find room where a small one does.
        let chunk = pick(&pool, &mut rng)?;
        let turn = rng.range_i(0, 4) as u8;
        let size = turned(chunk.size, turn);
        let p = Placement {
            chunk: chunk.name.clone(),
            biome,
            role: Role::Block,
            origin: (
                cx * BLOCK_CELL + rng.range_i(0, BLOCK_CELL - size.0 + 1),
                cz * BLOCK_CELL + rng.range_i(0, BLOCK_CELL - size.1 + 1),
            ),
            size,
            floor,
            turn,
            seed: rng.next_u64(),
            stories: 0,
        };
        // On a deck no hole or crater opens, clear of the hole chunks of its floor, with no tower
        // rising through it (a place needs the room).
        let fits = p.lots().all(|(x, z)| {
            !near_start_or_hub(x, z)
                && city::tower(seed, x, z).is_none_or(|t| t.top() < floors::y(floor) - 1.0)
                && !city::opened(seed, x, z, floor)
                && city::crater_at(seed, x, z).is_none()
                && area_at(seed, x, z).is_none()
                && !holes_at(seed, x, z).iter().any(|h| h.floor == floor)
        });
        if fits {
            return Some(p);
        }
    }
    None
}

/// The blocks covering a lot, on every floor.
pub fn blocks_at(seed: u64, lx: i32, lz: i32) -> Vec<Placement> {
    if chunks::preview().is_some() {
        return Vec::new();
    }
    let (cx, cz) = (lx.div_euclid(BLOCK_CELL), lz.div_euclid(BLOCK_CELL));
    (0..floors::count())
        .filter_map(|f| block_of_cell(seed, cx, cz, f))
        .filter(|p| p.covers(lx, lz))
        .collect()
}

// Holes.

/// The hole chunk a cell holds on a floor, if any: placed entirely inside the cell, never on an
/// area's floor or below it, clear of towers unless it lets them rise through
/// (`Chunk::towers_rise`).
fn hole_of_cell(seed: u64, cx: i32, cz: i32, floor: usize) -> Option<Placement> {
    cached((5, cx, cz, floor), seed, || {
        hole_of_cell_uncached(seed, cx, cz, floor)
            .into_iter()
            .collect()
    })
    .pop()
}

fn hole_of_cell_uncached(seed: u64, cx: i32, cz: i32, floor: usize) -> Option<Placement> {
    if floor == 0 || chunks::preview().is_some() {
        return None;
    }
    let biome = biome_of_cell(seed, cx, cz, HOLE_CELL);
    let cat = chunks::catalog(biome);
    let mut rng = Rng::new(hash_coords(seed, cx, SALT_HOLE + 64 * floor as i32, cz));
    if !rng.chance(design::profile(floor).holes) {
        return None;
    }
    let pool = pool(&cat, |c| {
        c.role() == Role::Hole && zone_fits(c, floor) && c.size.0.max(c.size.1) <= HOLE_CELL
    });
    let chunk = pick(&pool, &mut rng)?;
    let y = floors::y(floor);
    for _ in 0..HOLE_TRIES {
        let turn = rng.range_i(0, 4) as u8;
        let size = turned(chunk.size, turn);
        let (ox, oz) = hole_grid(floor);
        let p = Placement {
            chunk: chunk.name.clone(),
            biome,
            role: Role::Hole,
            origin: (
                cx * HOLE_CELL - ox + rng.range_i(0, HOLE_CELL - size.0 + 1),
                cz * HOLE_CELL - oz + rng.range_i(0, HOLE_CELL - size.1 + 1),
            ),
            size,
            floor,
            turn,
            seed: rng.next_u64(),
            stories: 0,
        };
        // The start floor around the spawn keeps its look, and its stair what it stands on.
        let start = floor <= START + 1;
        // A rim on lots a hole below already opened simply has nothing there.
        let fits = p.lots().all(|(x, z)| {
            let open = p.opens(x, z);
            !(start && near_start_or_hub(x, z))
                && area_at(seed, x, z).is_none_or(|a| a.floor < floor)
                && (!open
                    || chunk.towers_rise
                    || city::tower(seed, x, z).is_none_or(|t| t.top() < y - 1.0))
        });
        if fits {
            return Some(p);
        }
    }
    None
}

/// How far the hole cells of a floor are shifted (lots): each floor its own grid, so the holes of
/// the floors do not pile up in the same places.
fn hole_grid(floor: usize) -> (i32, i32) {
    let f = floor as i32;
    ((f * 2) % HOLE_CELL, (f * 5) % HOLE_CELL)
}

/// The hole cell holding a lot on a floor.
fn hole_cell(lx: i32, lz: i32, floor: usize) -> (i32, i32) {
    let (ox, oz) = hole_grid(floor);
    (
        (lx + ox).div_euclid(HOLE_CELL),
        (lz + oz).div_euclid(HOLE_CELL),
    )
}

/// The hole chunks covering a lot, on every floor.
pub fn holes_at(seed: u64, lx: i32, lz: i32) -> Vec<Placement> {
    (1..floors::count())
        .filter_map(|f| {
            let (cx, cz) = hole_cell(lx, lz, f);
            hole_of_cell(seed, cx, cz, f)
        })
        .filter(|p| p.covers(lx, lz))
        .collect()
}

/// The lowest floor a hole chunk opens at this lot (no deck there or above), if any.
pub fn hole_at(seed: u64, lx: i32, lz: i32) -> Option<usize> {
    holes_at(seed, lx, lz)
        .iter()
        .filter(|p| p.opens(lx, lz))
        .map(|p| p.floor)
        .min()
}

// Craters.

/// The crater always opening in front of the spawn (the player wakes up facing -z), down to the
/// start floor: a wide shaft of daylight beyond the dark plaza, the first pull upward.
fn start_crater() -> Option<Placement> {
    let cat = chunks::catalog(chunks::DEFAULT_BIOME);
    let c = cat.chunks.get(START_CRATER)?;
    Some(Placement {
        chunk: c.name.clone(),
        biome: chunks::DEFAULT_BIOME,
        role: Role::Crater,
        origin: (-c.size.0 / 2, -5 - c.size.1 / 2),
        size: c.size,
        floor: START,
        turn: 0,
        seed: 0,
        stories: 0,
    })
}

/// The crater of a cell, entirely inside it, if it has one: on the start floor now and then,
/// mostly on a floor of the middle.
fn crater_of_cell(seed: u64, cx: i32, cz: i32) -> Option<Placement> {
    cached((6, cx, cz, 0), seed, || {
        crater_of_cell_uncached(seed, cx, cz).into_iter().collect()
    })
    .pop()
}

fn crater_of_cell_uncached(seed: u64, cx: i32, cz: i32) -> Option<Placement> {
    // The start's crater stands across these cells.
    if (cx == -1 || cx == 0) && cz == -1 {
        return None;
    }
    let biome = biome_of_cell(seed, cx, cz, CRATER_CELL);
    let cat = chunks::catalog(biome);
    let mut rng = Rng::new(hash_coords(seed, cx, SALT_CRATER, cz));
    if !rng.chance(design::CRATER_CHANCE) {
        return None;
    }
    let pool = pool(&cat, |c| {
        c.role() == Role::Crater && c.size.0.max(c.size.1) <= CRATER_CELL
    });
    let chunk = pick(&pool, &mut rng)?;
    let middle: Vec<usize> = (0..floors::count())
        .filter(|&f| floors::zone(f) == floors::Zone::Middle)
        .collect();
    let floor = if rng.chance(design::CRATER_START) || middle.is_empty() {
        START
    } else {
        middle[rng.range_i(0, middle.len() as i32) as usize]
    };
    let turn = rng.range_i(0, 4) as u8;
    let size = turned(chunk.size, turn);
    Some(Placement {
        chunk: chunk.name.clone(),
        biome,
        role: Role::Crater,
        origin: (
            cx * CRATER_CELL + rng.range_i(0, CRATER_CELL - size.0 + 1),
            cz * CRATER_CELL + rng.range_i(0, CRATER_CELL - size.1 + 1),
        ),
        size,
        floor,
        turn,
        seed: rng.next_u64(),
        stories: 0,
    })
}

/// The crater whose shape takes a lot, if any (areas win over craters, and the start's plaza
/// stays).
pub fn crater_placement(seed: u64, lx: i32, lz: i32) -> Option<Placement> {
    if city::is_plaza(lx, lz) || chunks::preview().is_some() || area_at(seed, lx, lz).is_some() {
        return None;
    }
    let (cx, cz) = (lx.div_euclid(CRATER_CELL), lz.div_euclid(CRATER_CELL));
    start_crater()
        .into_iter()
        .chain(crater_of_cell(seed, cx, cz))
        .find(|p| p.covers(lx, lz) && p.opens(lx, lz))
}

// Stair hubs.

/// Middle lot of the stair hub always standing by the start (on the start floor).
pub const START_HUB: (i32, i32) = (3, 2);

/// The stair hub a cell holds on a floor (its stair going up to the next floor), if any.
fn hub_of_cell(seed: u64, cx: i32, cz: i32, floor: usize) -> Option<Placement> {
    cached((1, cx, cz, floor), seed, || {
        hub_of_cell_uncached(seed, cx, cz, floor)
            .into_iter()
            .collect()
    })
    .pop()
}

fn hub_of_cell_uncached(seed: u64, cx: i32, cz: i32, floor: usize) -> Option<Placement> {
    if floor + 1 >= floors::count() {
        return None;
    }
    let biome = biome_of_cell(seed, cx, cz, HUB_CELL);
    let cat = chunks::catalog(biome);
    let mut rng = Rng::new(hash_coords(seed, cx, SALT_HUB + 64 * floor as i32, cz));
    // The start always has a way up close by: the cell just beside the spawn plaza.
    let forced = floor == START && (cx, cz) == (0, 0);
    if !forced && !rng.chance(design::profile(floor).stair) {
        return None;
    }
    let pool = pool(&cat, |c| c.role() == Role::Hub && zone_fits(c, floor));
    // Without hub chunks in the catalog, a bare stair still stands (a one-lot hub with nothing).
    let (name, base) = match pick(&pool, &mut rng) {
        Some(c) => (c.name.clone(), c.size),
        None => (String::new(), (1, 1)),
    };
    let turn = rng.range_i(0, 4) as u8;
    let size = turned(base, turn);
    let (mx, mz) = (size.0 / 2, size.1 / 2);
    // The middle lot holds the stair: a lot of the cell free for it (no tower rising through the
    // floor, no crater, no area or block, a deck to arrive on), picked at random. The hub around
    // it may reach into the next cells; its boxes skip lots without a deck there
    // (`city::push_chunks`).
    let stair = if forced {
        START_HUB
    } else {
        // Lots already holding the stair of a lower floor's hub in this cell.
        let taken: Vec<(i32, i32)> = (0..floor)
            .filter_map(|f| hub_of_cell(seed, cx, cz, f))
            .map(|p| p.middle())
            .collect();
        let free: Vec<(i32, i32)> = (0..HUB_CELL * HUB_CELL)
            .map(|i| (cx * HUB_CELL + i % HUB_CELL, cz * HUB_CELL + i / HUB_CELL))
            .filter(|&(x, z)| {
                !city::is_plaza(x, z)
                    && (x, z) != START_HUB
                    && !taken.contains(&(x, z))
                    // A tower ending below this floor leaves the lot free above it.
                    && city::tower(seed, x, z).is_none_or(|t| t.top() < floors::y(floor) - 1.0)
                    && city::crater_at(seed, x, z).is_none()
                    && !city::opened(seed, x, z, floor + 1)
                    && area_at(seed, x, z).is_none()
                    && blocks_at(seed, x, z).is_empty()
                    && holes_at(seed, x, z).iter().all(|h| h.floor > floor + 1)
            })
            .collect();
        if free.is_empty() {
            return None;
        }
        free[rng.range_i(0, free.len() as i32) as usize]
    };
    Some(Placement {
        chunk: name,
        biome,
        role: Role::Hub,
        origin: (stair.0 - mx, stair.1 - mz),
        size,
        floor,
        turn,
        seed: rng.next_u64(),
        stories: 0,
    })
}

/// The hub whose stair stands in this lot (the floor it leaves from), if any. A lot holds one
/// stair at most: of two hubs of a cell wanting the same lot, the lower floor's keeps it.
pub fn stair_hub(seed: u64, lx: i32, lz: i32) -> Option<Placement> {
    if chunks::preview().is_some() {
        return None;
    }
    let (cx, cz) = (lx.div_euclid(HUB_CELL), lz.div_euclid(HUB_CELL));
    (0..floors::count() - 1)
        .filter_map(|f| hub_of_cell(seed, cx, cz, f))
        .find(|p| p.middle() == (lx, lz))
}

/// The stair hubs whose surroundings cover a lot, on every floor.
fn hubs_at(seed: u64, lx: i32, lz: i32) -> Vec<Placement> {
    let mut out = Vec::new();
    let (cx, cz) = (lx.div_euclid(HUB_CELL), lz.div_euclid(HUB_CELL));
    // Hubs reach a lot or two past their cell: look at the neighbor cells too.
    for nz in cz - 1..=cz + 1 {
        for nx in cx - 1..=cx + 1 {
            let mut taken: Vec<(i32, i32)> = Vec::new();
            for f in 0..floors::count() - 1 {
                if let Some(p) = hub_of_cell(seed, nx, nz, f) {
                    let stair = p.middle();
                    // Same rule as `stair_hub`: a lot's stair belongs to the lowest floor.
                    if taken.contains(&stair) {
                        continue;
                    }
                    taken.push(stair);
                    if p.covers(lx, lz) {
                        out.push(p);
                    }
                }
            }
        }
    }
    out
}

// Towers.

/// Side, in lots, of the districts that are low-rise or not (`low_rise`).
pub const DISTRICT: i32 = 6;
const SALT_DISTRICT: i32 = 29;

/// Whether a lot lies in a low-rise district (`design::LOW_RISE` of them): few towers, so row
/// houses line its streets and its blocks hold the special places (squares, shrines,
/// playgrounds...). Never the start's district.
pub fn low_rise(seed: u64, lx: i32, lz: i32) -> bool {
    let (dx, dz) = (lx.div_euclid(DISTRICT), lz.div_euclid(DISTRICT));
    if (dx, dz) == (0, 0)
        || (dx, dz)
            == (
                START_HUB.0.div_euclid(DISTRICT),
                START_HUB.1.div_euclid(DISTRICT),
            )
    {
        return false;
    }
    Rng::new(hash_coords(seed, dx, SALT_DISTRICT, dz)).chance(design::LOW_RISE)
}

/// Whether a tower may stand on a lot: not in an area or a crater, not on the start's plaza or its
/// stair lot.
fn tower_may_stand(seed: u64, lx: i32, lz: i32) -> bool {
    !city::is_plaza(lx, lz)
        && (lx, lz) != START_HUB
        && area_at(seed, lx, lz).is_none()
        && city::crater_at(seed, lx, lz).is_none()
}

/// A tower's height in stories, from the shares of `design::TOWER_HEIGHTS` the chunk allows.
fn tower_stories(chunk: &Chunk, rng: &mut Rng) -> u32 {
    use chunks::Height;
    let bands = [Height::Old, Height::Stack, Height::Front];
    let allowed = |h: Height| {
        chunk
            .tower
            .as_ref()
            .is_none_or(|t| t.heights.is_empty() || t.heights.contains(&h))
    };
    let total: f32 = (0..3)
        .filter(|&i| allowed(bands[i]))
        .map(|i| design::TOWER_HEIGHTS[i])
        .sum();
    let mut roll = rng.f32() * total;
    let band = (0..3)
        .filter(|&i| allowed(bands[i]))
        .find(|&i| {
            roll -= design::TOWER_HEIGHTS[i];
            roll < 0.0
        })
        .map_or(Height::Stack, |i| bands[i]);
    let top = match band {
        Height::Old => rng.range(floors::y(1) + 8.0, floors::y(START) + 14.0),
        Height::Stack => {
            floors::y(rng.range_i(START as i32, floors::count() as i32) as usize)
                + rng.range(8.0, 28.0)
        }
        Height::Front => rng.range(floors::top() + 30.0, floors::top() + 110.0),
    };
    (((top - floors::bottom()) / chunks::STORY).round() as u32).max(3)
}

/// The towers of a cell.
fn towers_of_cell(seed: u64, cx: i32, cz: i32) -> Vec<Placement> {
    cached((4, cx, cz, 0), seed, || {
        towers_of_cell_uncached(seed, cx, cz)
    })
}

fn towers_of_cell_uncached(seed: u64, cx: i32, cz: i32) -> Vec<Placement> {
    let biome = biome_of_cell(seed, cx, cz, TOWER_CELL);
    let cat = chunks::catalog(biome);
    let pool = pool(&cat, |c| c.role() == Role::Tower && c.tower.is_some());
    let small: Vec<&Chunk> = pool.iter().copied().filter(|c| c.size == (1, 1)).collect();
    let (x0, z0) = (cx * TOWER_CELL, cz * TOWER_CELL);
    let lots: Vec<(i32, i32)> = (0..TOWER_CELL * TOWER_CELL)
        .map(|i| (x0 + i % TOWER_CELL, z0 + i / TOWER_CELL))
        .collect();
    let mut taken = vec![false; lots.len()];
    let mut out = Vec::new();
    let mut rng = Rng::new(hash_coords(seed, cx, SALT_TOWER, cz));
    // A big tower now and then, where its lots are all free.
    if let Some(c) = pick(&pool, &mut rng).filter(|c| c.size != (1, 1)) {
        let turn = rng.range_i(0, 4) as u8;
        let size = turned(c.size, turn);
        if size.0 <= TOWER_CELL && size.1 <= TOWER_CELL {
            let origin = (
                x0 + rng.range_i(0, TOWER_CELL - size.0 + 1),
                z0 + rng.range_i(0, TOWER_CELL - size.1 + 1),
            );
            let mut p = Placement {
                chunk: c.name.clone(),
                biome,
                role: Role::Tower,
                origin,
                size,
                floor: 0,
                turn,
                seed: rng.next_u64(),
                stories: 0,
            };
            p.stories = tower_stories(c, &mut rng);
            if p.lots()
                .all(|(x, z)| tower_may_stand(seed, x, z) && !low_rise(seed, x, z))
            {
                for (i, &(x, z)) in lots.iter().enumerate() {
                    taken[i] = p.covers(x, z);
                }
                out.push(p);
            }
        }
    }
    for (i, &(x, z)) in lots.iter().enumerate() {
        if taken[i] || !tower_may_stand(seed, x, z) {
            continue;
        }
        let mut rng = Rng::new(hash_coords(seed, x, SALT_TOWER + 1, z));
        if rng.chance(biome::biome_at(seed, x, z).tower_empty)
            || (low_rise(seed, x, z) && !rng.chance(design::LOW_RISE_TOWERS))
        {
            continue;
        }
        let Some(c) = pick(&small, &mut rng) else {
            continue;
        };
        let turn = rng.range_i(0, 4) as u8;
        let own = rng.next_u64();
        out.push(Placement {
            chunk: c.name.clone(),
            biome,
            role: Role::Tower,
            origin: (x, z),
            size: (1, 1),
            floor: 0,
            turn,
            seed: own,
            stories: tower_stories(c, &mut rng),
        });
    }
    out
}

/// The tower standing on a lot, if any (a big one covers several lots).
pub fn tower_at(seed: u64, lx: i32, lz: i32) -> Option<Placement> {
    if let Some(pv) = chunks::preview() {
        // A previewed tower stands alone in front of the spawn, a few floors high.
        let ((ox, oz), size) = chunks::preview_footprint()?;
        let cat = chunks::catalog(pv.biome);
        let c = cat
            .chunks
            .get(&pv.name)
            .filter(|c| c.role() == Role::Tower)?;
        let p = Placement {
            chunk: c.name.clone(),
            biome: pv.biome,
            role: Role::Tower,
            origin: (ox, oz),
            size,
            floor: 0,
            turn: 0,
            seed: hash_coords(seed, ox, SALT_TOWER, oz),
            stories: ((floors::y(START + 1) + 24.0 - floors::bottom()) / chunks::STORY) as u32,
        };
        return p.covers(lx, lz).then_some(p);
    }
    let (cx, cz) = (lx.div_euclid(TOWER_CELL), lz.div_euclid(TOWER_CELL));
    towers_of_cell(seed, cx, cz)
        .into_iter()
        .find(|p| p.covers(lx, lz))
}

// Fillers.

/// The filler standing on a lot on a floor, given the other chunks covering the lot.
fn filler_at(seed: u64, lx: i32, lz: i32, floor: usize, others: &[Placement]) -> Option<Placement> {
    if others.iter().any(|p| p.floor == floor) {
        return None;
    }
    let mut rng = Rng::new(hash_coords(seed, lx, SALT_FILLER + 64 * floor as i32, lz));
    // Housing and yards at the floor's chance (never by the start), else a vacant lot.
    // Low-rise districts are built up: row houses, hardly a vacant lot.
    let fillers = design::profile(floor).fillers;
    let fillers = if low_rise(seed, lx, lz) {
        fillers.max(0.9)
    } else {
        fillers
    };
    let housing = rng.chance(fillers) && !near_start_or_hub(lx, lz);
    let free = city::deck(seed, lx, lz, floor) == city::Deck::Full
        && city::stair(seed, lx, lz).is_none()
        && city::tower(seed, lx, lz).is_none_or(|t| t.top() < floors::y(floor) - 1.0);
    if !free {
        return None;
    }
    let biome = biome::biome_at(seed, lx, lz).name;
    let cat = chunks::catalog(biome);
    let pool = pool(&cat, |c| {
        c.role() == Role::Filler
            && c.size == (1, 1)
            && zone_fits(c, floor)
            && c.labels.iter().any(|l| l == "vacant") != housing
    });
    let chunk = pick(&pool, &mut rng)?;
    Some(Placement {
        chunk: chunk.name.clone(),
        biome,
        role: Role::Filler,
        origin: (lx, lz),
        size: (1, 1),
        floor,
        turn: rng.range_i(0, 4) as u8,
        seed: rng.next_u64(),
        stories: 0,
    })
}

// Streets.

/// The street around a lot on a floor, given the chunks covering it: wherever the lot has a deck
/// and no area, block or hub designs it. Never turned: its lamps stand at the same corner of every
/// lot, one at each crossing.
fn street_at(seed: u64, lx: i32, lz: i32, floor: usize, others: &[Placement]) -> Option<Placement> {
    if others.iter().any(|p| p.floor == floor)
        || city::deck(seed, lx, lz, floor) == city::Deck::None
    {
        return None;
    }
    let biome = biome::biome_at(seed, lx, lz).name;
    let cat = chunks::catalog(biome);
    let pool = pool(&cat, |c| {
        c.role() == Role::Street && c.size == (1, 1) && zone_fits(c, floor)
    });
    let mut rng = Rng::new(hash_coords(seed, lx, SALT_STREET + 64 * floor as i32, lz));
    let chunk = pick(&pool, &mut rng)?;
    Some(Placement {
        chunk: chunk.name.clone(),
        biome,
        role: Role::Street,
        origin: (lx, lz),
        size: (1, 1),
        floor,
        turn: 0,
        seed: rng.next_u64(),
        stories: 0,
    })
}

/// The plaza where the player wakes up, on the start floor (`START_CHUNK`, if the catalog has it).
fn start_at(lx: i32, lz: i32) -> Option<Placement> {
    if !city::is_plaza(lx, lz) {
        return None;
    }
    let cat = chunks::catalog(chunks::DEFAULT_BIOME);
    let c = cat.chunks.get(START_CHUNK)?;
    Some(Placement {
        chunk: c.name.clone(),
        biome: chunks::DEFAULT_BIOME,
        role: Role::Block,
        origin: (-1, -1),
        size: c.size,
        floor: START,
        turn: 0,
        seed: 0,
        stories: 0,
    })
}

/// Every chunk covering a lot, on every floor: areas and their sub-areas, blocks, stair hubs,
/// fillers and streets.
pub fn covering(seed: u64, lx: i32, lz: i32) -> Vec<Placement> {
    if chunks::preview().is_some() {
        return Vec::new();
    }
    let mut out: Vec<Placement> = Vec::new();
    out.extend(start_at(lx, lz));
    if let Some(a) = area_at(seed, lx, lz) {
        out.extend(
            subs_of_area(seed, &a)
                .into_iter()
                .filter(|s| s.covers(lx, lz)),
        );
        out.push(a);
    }
    out.extend(blocks_at(seed, lx, lz));
    out.extend(holes_at(seed, lx, lz));
    out.extend(crater_placement(seed, lx, lz));
    out.extend(hubs_at(seed, lx, lz));
    // Fillers only where nothing else stands (a hub's surroundings without a deck do not count:
    // they build nothing there).
    let taken: Vec<Placement> = out
        .iter()
        .filter(|p| p.role != Role::Hub || city::deck(seed, lx, lz, p.floor) != city::Deck::None)
        .cloned()
        .collect();
    for f in 0..floors::count() {
        out.extend(filler_at(seed, lx, lz, f, &taken));
        out.extend(street_at(seed, lx, lz, f, &taken));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn areas_keep_their_spacing_and_avoid_the_start() {
        let mut areas = Vec::new();
        for cz in -4..4 {
            for cx in -4..4 {
                if let Some(p) = area_of_cell(1, cx, cz) {
                    areas.push(p);
                }
            }
        }
        assert!(!areas.is_empty(), "no area on 64 cells");
        for (i, a) in areas.iter().enumerate() {
            for b in &areas[..i] {
                let (cata, catb) = (chunks::catalog(a.biome), chunks::catalog(b.biome));
                let (ca, cb) = (&cata.chunks[&a.chunk], &catb.chunks[&b.chunk]);
                if !ca.labels.iter().any(|l| cb.labels.contains(l)) {
                    continue;
                }
                let spacing = ca.spacing.max(cb.spacing) as f32;
                let (p, q) = (a.center(), b.center());
                let d = ((p.0 - q.0).powi(2) + (p.1 - q.1).powi(2)).sqrt();
                assert!(
                    d >= spacing,
                    "{} and {} only {d} lots apart",
                    a.chunk,
                    b.chunk
                );
            }
            for (x, z) in a.lots() {
                assert!(!near_start(x, z));
            }
        }
    }

    #[test]
    fn a_hub_stands_by_the_start() {
        for seed in 1..=8 {
            assert!(
                stair_hub(seed, START_HUB.0, START_HUB.1).is_some_and(|p| p.floor == START),
                "seed {seed}"
            );
        }
    }

    #[test]
    fn sub_areas_fill_their_openings() {
        let (mut areas, mut subs) = (0, 0);
        for cz in -3..3 {
            for cx in -3..3 {
                if let Some(a) = area_of_cell(2, cx, cz) {
                    areas += 1;
                    for s in subs_of_area(2, &a) {
                        subs += 1;
                        assert!(s.lots().all(|(x, z)| a.covers(x, z)), "{} outside", s.chunk);
                    }
                }
            }
        }
        assert!(areas == 0 || subs > 0, "{areas} areas, no sub-area");
    }

    #[test]
    fn blocks_and_holes() {
        let mut blocks = 0;
        for cz in -4..4 {
            for cx in -4..4 {
                for f in 1..floors::count() {
                    let Some(b) = block_of_cell(3, cx, cz, f) else {
                        continue;
                    };
                    blocks += 1;
                    for (x, z) in b.lots() {
                        assert!(x.div_euclid(BLOCK_CELL) == cx && z.div_euclid(BLOCK_CELL) == cz);
                        assert_eq!(city::deck(3, x, z, f), city::Deck::Full);
                    }
                }
            }
        }
        assert!(blocks > 10, "only {blocks} blocks on 64 cells");
        let mut holes = 0;
        for cz in -5..5 {
            for cx in -5..5 {
                for f in 1..floors::count() {
                    let Some(h) = hole_of_cell(3, cx, cz, f) else {
                        continue;
                    };
                    for (x, z) in h.lots() {
                        assert_eq!(hole_cell(x, z, f), (cx, cz));
                        if h.opens(x, z) {
                            holes += 1;
                            // Open on its floor and every floor above, no stair there.
                            for g in f..floors::count() {
                                assert_eq!(city::deck(3, x, z, g), city::Deck::None);
                            }
                            assert!(city::stair(3, x, z).is_none_or(|(lo, _)| lo + 1 < f));
                        }
                    }
                }
            }
        }
        assert!(holes > 30, "only {holes} hole lots on 100 cells");
    }

    #[test]
    fn fillers_stand_on_free_lots() {
        let mut n = 0;
        for lz in -12..12 {
            for lx in -12..12 {
                let all = covering(4, lx, lz);
                for p in all.iter().filter(|p| p.role == Role::Filler) {
                    n += 1;
                    assert_eq!(city::deck(4, lx, lz, p.floor), city::Deck::Full);
                    assert!(!all.iter().any(|q| q.floor == p.floor && q.outranks(p)));
                }
            }
        }
        assert!(n > 100, "only {n} fillers on 576 lots");
    }

    #[test]
    fn holes_turn_with_their_chunk() {
        let cat = chunks::catalog(chunks::DEFAULT_BIOME);
        let Some(c) = cat.chunks.values().find(|c| !c.holes.is_empty()) else {
            return;
        };
        let area = |r: &chunks::LotRect| (r.1.0 - r.0.0) * (r.1.1 - r.0.1);
        for turn in 0..4 {
            let size = turned(c.size, turn);
            for (h, r) in c.holes.iter().zip(c.placed_holes((10, 20), turn)) {
                assert_eq!(area(h), area(&r));
                assert!(r.0.0 >= 10 && r.0.1 >= 20 && r.1.0 <= 10 + size.0 && r.1.1 <= 20 + size.1);
            }
        }
    }
}
