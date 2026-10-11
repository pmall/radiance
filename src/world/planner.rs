//! The planner (`docs/chunks.md`): which hand-written chunk stands where, from the seed and the
//! placement rules. Every answer is a pure function of the seed and the lot, so the city still
//! streams lot by lot.
//!
//! - **Areas** (chunks 6 lots wide or more: a great square, a market district): the plane is cut
//!   into cells of `AREA_CELL` lots; each cell holds at most one area, entirely inside it, on one
//!   floor of the area's zones. Areas sharing a label keep their `spacing` (the cell holding the
//!   lower-priority one gives way). No tower stands in an area's footprint, on any floor.
//! - **Stair hubs:** every stair stands in the middle lot of a hub chunk, which frames it (railings,
//!   lamps, benches). Cells of `HUB_CELL` lots hold at most one hub per floor, at the floor's
//!   `stair` chance (`design.rs`), so stairs never cluster.
//! - Everything else is still the lot generator of `city.rs`, which fills around the chunks.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use super::chunks::{self, Catalog, Chunk};
use super::city;
use super::floors::START;
use super::{design, floors};
use crate::rng::{Rng, hash_coords};

/// Side of an area cell, in lots.
pub const AREA_CELL: i32 = 32;
/// Side of a stair hub cell, in lots.
pub const HUB_CELL: i32 = 6;
/// Chance that an area cell holds an area.
const AREA_CHANCE: f32 = 0.6;
/// Chunks at least this wide (in lots) are areas.
const AREA_MIN: i32 = 6;

const SALT_AREA: i32 = 20;
const SALT_HUB: i32 = 21;

/// What a placed chunk is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    Area,
    /// Frames the stair from its floor to the next, in its middle lot.
    Hub,
}

/// A chunk placed in the city.
#[derive(Clone, Debug)]
pub struct Placement {
    pub chunk: String,
    pub role: Role,
    /// Corner lot of the footprint, and its size in lots (after the turn).
    pub origin: (i32, i32),
    pub size: (i32, i32),
    pub floor: usize,
    /// Quarter turns.
    pub turn: u8,
    /// Seeds the chunk's slots.
    pub seed: u64,
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
}

fn zone_fits(chunk: &Chunk, floor: usize) -> bool {
    let z = match floors::zone(floor) {
        floors::Zone::Depths => chunks::Zone::Depths,
        floors::Zone::Middle => chunks::Zone::Middle,
        floors::Zone::Canopy => chunks::Zone::Canopy,
    };
    chunk.zones.is_empty() || chunk.zones.contains(&z)
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

/// Lots the start keeps as they are on every floor: the spawn plaza and the crater in front of it.
fn near_start(lx: i32, lz: i32) -> bool {
    (-4..=4).contains(&lx) && (-10..=2).contains(&lz)
}

/// The area a cell proposes, before spacing between cells is settled.
fn area_candidate(seed: u64, cat: &Catalog, cx: i32, cz: i32) -> Option<(Placement, f32)> {
    let mut rng = Rng::new(hash_coords(seed, cx, SALT_AREA, cz));
    if !rng.chance(AREA_CHANCE) {
        return None;
    }
    let mut pool: Vec<&Chunk> = cat
        .chunks
        .values()
        .filter(|c| c.size.0.max(c.size.1) >= AREA_MIN && c.size.0.max(c.size.1) <= AREA_CELL)
        .collect();
    pool.sort_by(|a, b| a.name.cmp(&b.name));
    let chunk = pick(&pool, &mut rng)?;
    let floors: Vec<usize> = (0..floors::count() - 1)
        .filter(|&f| zone_fits(chunk, f))
        .collect();
    if floors.is_empty() {
        return None;
    }
    let floor = floors[rng.range_i(0, floors.len() as i32) as usize];
    let turn = rng.range_i(0, 4) as u8;
    let size = if turn % 2 == 1 {
        (chunk.size.1, chunk.size.0)
    } else {
        chunk.size
    };
    let origin = (
        cx * AREA_CELL + rng.range_i(0, AREA_CELL - size.0 + 1),
        cz * AREA_CELL + rng.range_i(0, AREA_CELL - size.1 + 1),
    );
    let p = Placement {
        chunk: chunk.name.clone(),
        role: Role::Area,
        origin,
        size,
        floor,
        turn,
        seed: rng.next_u64(),
    };
    // Never over the start. Craters give way to areas (`city::crater_at`).
    let clear = (origin.0..origin.0 + size.0)
        .all(|x| (origin.1..origin.1 + size.1).all(|z| !near_start(x, z)));
    clear.then(|| (p, rng.f32()))
}

/// Cell answers, keyed by (catalog generation, seed, kind, cell x, cell z, floor): the planner is
/// asked about the same cells over and over (every deck and stair query).
type Cache = Mutex<HashMap<(u64, u64, u8, i32, i32, usize), Option<Placement>>>;

fn cached(
    key: (u8, i32, i32, usize),
    seed: u64,
    compute: impl FnOnce() -> Option<Placement>,
) -> Option<Placement> {
    static CACHE: OnceLock<Cache> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    let full = (chunks::generation(), seed, key.0, key.1, key.2, key.3);
    if let Some(p) = cache.lock().unwrap().get(&full) {
        return p.clone();
    }
    let p = compute();
    cache.lock().unwrap().insert(full, p.clone());
    p
}

/// The area of a cell, spacing settled: of two areas sharing a label closer than their spacing,
/// the one with the lower priority roll gives way.
fn area_of_cell(seed: u64, cat: &Catalog, cx: i32, cz: i32) -> Option<Placement> {
    cached((0, cx, cz, 0), seed, || area_of_cell_uncached(seed, cat, cx, cz))
}

fn area_of_cell_uncached(seed: u64, cat: &Catalog, cx: i32, cz: i32) -> Option<Placement> {
    let (p, prio) = area_candidate(seed, cat, cx, cz)?;
    let chunk = cat.chunks.get(&p.chunk)?;
    let reach = (chunk.spacing as i32 / AREA_CELL) + 1;
    for nz in cz - reach..=cz + reach {
        for nx in cx - reach..=cx + reach {
            if (nx, nz) == (cx, cz) {
                continue;
            }
            let Some((q, qprio)) = area_candidate(seed, cat, nx, nz) else {
                continue;
            };
            let Some(other) = cat.chunks.get(&q.chunk) else {
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
    let cat = chunks::catalog();
    let (cx, cz) = (lx.div_euclid(AREA_CELL), lz.div_euclid(AREA_CELL));
    area_of_cell(seed, &cat, cx, cz).filter(|p| p.covers(lx, lz))
}

/// Middle lot of the stair hub always standing by the start (on the start floor).
pub const START_HUB: (i32, i32) = (3, 2);

/// The stair hub a cell holds on a floor (its stair going up to the next floor), if any.
fn hub_of_cell(seed: u64, cat: &Catalog, cx: i32, cz: i32, floor: usize) -> Option<Placement> {
    cached((1, cx, cz, floor), seed, || {
        hub_of_cell_uncached(seed, cat, cx, cz, floor)
    })
}

fn hub_of_cell_uncached(
    seed: u64,
    cat: &Catalog,
    cx: i32,
    cz: i32,
    floor: usize,
) -> Option<Placement> {
    if floor + 1 >= floors::count() {
        return None;
    }
    let mut rng = Rng::new(hash_coords(seed, cx, SALT_HUB + 64 * floor as i32, cz));
    // The start always has a way up close by: the cell just beside the spawn plaza.
    let forced = floor == START && (cx, cz) == (0, 0);
    if !forced && !rng.chance(design::profile(floor).stair) {
        return None;
    }
    let mut pool: Vec<&Chunk> = cat
        .chunks
        .values()
        .filter(|c| c.labels.iter().any(|l| l == "stair_hub") && zone_fits(c, floor))
        .collect();
    pool.sort_by(|a, b| a.name.cmp(&b.name));
    // Without hub chunks in the catalog, a bare stair still stands (a one-lot hub with nothing).
    let (name, base) = match pick(&pool, &mut rng) {
        Some(c) => (c.name.clone(), c.size),
        None => (String::new(), (1, 1)),
    };
    let turn = rng.range_i(0, 4) as u8;
    let size = if turn % 2 == 1 { (base.1, base.0) } else { base };
    let (mx, mz) = (size.0 / 2, size.1 / 2);
    // The middle lot holds the stair: a lot of the cell free for it (no tower rising through the
    // floor, no crater, not
    // in an area, a deck to arrive on), picked at random. The hub around it may reach into the
    // next cells; its boxes skip lots without a deck there (`city::push_chunks`).
    let stair = if forced {
        START_HUB
    } else {
        // Lots already holding the stair of a lower floor's hub in this cell.
        let taken: Vec<(i32, i32)> = (0..floor)
            .filter_map(|f| hub_of_cell(seed, cat, cx, cz, f))
            .map(|p| (p.origin.0 + p.size.0 / 2, p.origin.1 + p.size.1 / 2))
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
                    && city::built(seed, x, z) > floor
                    && area_at(seed, x, z).is_none()
            })
            .collect();
        if free.is_empty() {
            return None;
        }
        free[rng.range_i(0, free.len() as i32) as usize]
    };
    Some(Placement {
        chunk: name,
        role: Role::Hub,
        origin: (stair.0 - mx, stair.1 - mz),
        size,
        floor,
        turn,
        seed: rng.next_u64(),
    })
}

/// The hub whose stair stands in this lot (the floor it leaves from), if any. A lot holds one
/// stair at most: of two hubs of a cell wanting the same lot, the lower floor's keeps it.
pub fn stair_hub(seed: u64, lx: i32, lz: i32) -> Option<Placement> {
    if chunks::preview().is_some() {
        return None;
    }
    let cat = chunks::catalog();
    let (cx, cz) = (lx.div_euclid(HUB_CELL), lz.div_euclid(HUB_CELL));
    (0..floors::count() - 1)
        .filter_map(|f| hub_of_cell(seed, &cat, cx, cz, f))
        .find(|p| (p.origin.0 + p.size.0 / 2, p.origin.1 + p.size.1 / 2) == (lx, lz))
}

/// Every chunk covering a lot (on any floor): areas and stair hubs.
pub fn covering(seed: u64, lx: i32, lz: i32) -> Vec<Placement> {
    if chunks::preview().is_some() {
        return Vec::new();
    }
    let cat = chunks::catalog();
    let mut out: Vec<Placement> = area_at(seed, lx, lz).into_iter().collect();
    let (cx, cz) = (lx.div_euclid(HUB_CELL), lz.div_euclid(HUB_CELL));
    // Hubs reach a lot or two past their cell: look at the neighbor cells too.
    for nz in cz - 1..=cz + 1 {
        for nx in cx - 1..=cx + 1 {
            let mut taken: Vec<(i32, i32)> = Vec::new();
            for f in 0..floors::count() - 1 {
                if let Some(p) = hub_of_cell(seed, &cat, nx, nz, f) {
                    let stair = (p.origin.0 + p.size.0 / 2, p.origin.1 + p.size.1 / 2);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn areas_keep_their_spacing_and_avoid_the_start() {
        let cat = chunks::catalog();
        let mut areas = Vec::new();
        for cz in -4..4 {
            for cx in -4..4 {
                if let Some(p) = area_of_cell(1, &cat, cx, cz) {
                    areas.push(p);
                }
            }
        }
        assert!(!areas.is_empty(), "no area on 64 cells");
        for (i, a) in areas.iter().enumerate() {
            for b in &areas[..i] {
                let (ca, cb) = (&cat.chunks[&a.chunk], &cat.chunks[&b.chunk]);
                if !ca.labels.iter().any(|l| cb.labels.contains(l)) {
                    continue;
                }
                let spacing = cat.chunks[&a.chunk].spacing.max(cat.chunks[&b.chunk].spacing) as f32;
                let (p, q) = (a.center(), b.center());
                let d = ((p.0 - q.0).powi(2) + (p.1 - q.1).powi(2)).sqrt();
                assert!(d >= spacing, "{} and {} only {d} lots apart", a.chunk, b.chunk);
            }
            for x in a.origin.0..a.origin.0 + a.size.0 {
                for z in a.origin.1..a.origin.1 + a.size.1 {
                    assert!(!near_start(x, z));
                }
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
}
