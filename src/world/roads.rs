//! The road network (owner: roads must make sense). Roads run along lot borders, 3 m on each side
//! (`pavement` in `shaders/scene.fs`), and a border is a road on a floor when:
//!
//! - the lots on both sides are open: a deck, not paved over by a chunk (`city::paved`: a square,
//!   a market, a yard), and not one building (a tower over two lots or more, up to its top);
//! - both its ends are full crossings: the four lots around each end are open. So a road never
//!   runs into a hole, the edge of a deck or a square, and is never half a road; the lots along an
//!   edge keep their sidewalk to it;
//! - it leads somewhere: a road with no other road at one of its ends is cut back, `PRUNE` times
//!   over, so stubs and roads across a narrow strip of deck disappear (a very long dead end is only
//!   shortened: it ends far from the edge, closed by a curb);
//! - it belongs to a network: roads that link no more than `SMALL` lots across (a ring around a
//!   block or two on a small island of deck) are no network, and go too.
//!
//! Every answer is a pure function of the seed and the lot. The network is computed by tiles of
//! `TILE` lots, each with a margin wide enough that tiles agree along their borders, and cached.
//!
//! What a lot keeps is two bits per floor, its roads along its -x and its -z border (`Roads`);
//! the rest (crossings, curbs, lane markings) follows from the roads meeting at each corner, here
//! (`along`, `crossing`) and in the shader alike (`bits`, uploaded by `render/cover.rs`).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use super::city::{self, Deck};
use super::{chunks, floors, planner};

/// Rounds of cutting dead ends back.
const PRUNE: i32 = 4;
/// The widest network that is no network, in lots across (both ways).
const SMALL: i32 = 2;
/// Lots of margin around a tile, so that the roads inside it come out exact: a road's first answer
/// needs the lots around both its ends, each round of cutting reaches one lot further, and telling
/// a small network reaches `SMALL + 1` lots out.
const MARGIN: i32 = PRUNE + SMALL + 2;
/// Side of the tiles the network is computed by, in lots.
const TILE: i32 = 4;
/// Tiles kept before starting over.
const CACHED_TILES: usize = 16_384;

/// The roads along a lot's -x and -z borders, one bit per floor (bit `f` for floor `f`).
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Roads {
    /// Along its -x border, between it and the lot at x - 1.
    pub west: u16,
    /// Along its -z border, between it and the lot at z - 1.
    pub south: u16,
}

/// What the network needs to know of a lot.
#[derive(Clone, Copy)]
struct Ground {
    /// Floors where a road may run along it: a deck, not paved over.
    open: u16,
    /// The building over several lots standing on it: its corner lot and the floors it rises
    /// through.
    building: Option<((i32, i32), u16)>,
}

fn ground(seed: u64, lx: i32, lz: i32) -> Ground {
    static GROUND: city::Memo<Ground> = OnceLock::new();
    city::memo(&GROUND, seed, lx, lz, || {
        let decks = (0..floors::count())
            .filter(|&f| city::deck(seed, lx, lz, f) != Deck::None)
            .fold(0u16, |bits, f| bits | 1 << f);
        let building = planner::tower_at(seed, lx, lz)
            .filter(|p| p.size != (1, 1))
            .and_then(|p| {
                let top = city::tower(seed, lx, lz)?.top();
                let through = (0..floors::count())
                    .filter(|&f| floors::y(f) < top)
                    .fold(0u16, |bits, f| bits | 1 << f);
                Some((p.origin, through))
            });
        Ground {
            open: decks & !city::paved(seed, lx, lz),
            building,
        }
    })
}

/// The roads along a lot's -x and -z borders.
pub fn roads(seed: u64, lx: i32, lz: i32) -> Roads {
    let t = tile(seed, lx.div_euclid(TILE), lz.div_euclid(TILE));
    t[(lz.rem_euclid(TILE) * TILE + lx.rem_euclid(TILE)) as usize]
}

/// Whether a road runs along the border of `lot` toward `dir` (one step along x or z) on a floor.
pub fn along(seed: u64, lot: (i32, i32), dir: (i32, i32), floor: usize) -> bool {
    let bit = 1 << floor;
    let (x, z) = lot;
    match dir {
        (-1, 0) => roads(seed, x, z).west & bit != 0,
        (1, 0) => roads(seed, x + 1, z).west & bit != 0,
        (0, -1) => roads(seed, x, z).south & bit != 0,
        (0, 1) => roads(seed, x, z + 1).south & bit != 0,
        _ => false,
    }
}

/// Whether roads cross or turn at a lot corner (corner `(x, z)` is the -x-z corner of lot
/// `(x, z)`): at least one leaves it along x and one along z (a turn, a T, a crossroads). Where a
/// street light stands.
pub fn crossing(seed: u64, corner: (i32, i32), floor: usize) -> bool {
    let bit = 1 << floor;
    let (x, z) = corner;
    let along_z = (roads(seed, x, z).west | roads(seed, x, z - 1).west) & bit != 0;
    let along_x = (roads(seed, x, z).south | roads(seed, x - 1, z).south) & bit != 0;
    along_z && along_x
}

/// A lot's roads for the renderer: floor `f` at bits `2f` (its -x border) and `2f + 1` (its -z
/// border), so each byte holds four floors.
pub fn bits(seed: u64, lx: i32, lz: i32) -> u32 {
    let r = roads(seed, lx, lz);
    (0..floors::MAX).fold(0, |bits, f| {
        bits | ((r.west >> f & 1) as u32) << (2 * f) | ((r.south >> f & 1) as u32) << (2 * f + 1)
    })
}

/// The roads of the lots of a tile, cached.
fn tile(seed: u64, tx: i32, tz: i32) -> Arc<Vec<Roads>> {
    type Tiles = HashMap<(u64, u64, i32, i32), Arc<Vec<Roads>>>;
    static TILES: OnceLock<Mutex<Tiles>> = OnceLock::new();
    let store = TILES.get_or_init(Default::default);
    let key = (chunks::generation(), seed, tx, tz);
    if let Some(t) = store.lock().unwrap().get(&key) {
        return t.clone();
    }
    let t = Arc::new(network(seed, tx * TILE, tz * TILE));
    let mut m = store.lock().unwrap();
    if m.len() >= CACHED_TILES {
        m.clear();
    }
    m.insert(key, t.clone());
    t
}

/// The roads of the `TILE` x `TILE` lots from `(x0, z0)`, worked out over a margin around them.
fn network(seed: u64, x0: i32, z0: i32) -> Vec<Roads> {
    const W: i32 = TILE + 2 * MARGIN;
    let (ox, oz) = (x0 - MARGIN, z0 - MARGIN);
    let idx = |i: i32, j: i32| (j * W + i) as usize;
    let inside = |i: i32, j: i32| (0..W).contains(&i) && (0..W).contains(&j);
    let grounds: Vec<Ground> = (0..W)
        .flat_map(|j| (0..W).map(move |i| (i, j)))
        .map(|(i, j)| ground(seed, ox + i, oz + j))
        .collect();
    let open = |i: i32, j: i32| {
        if inside(i, j) {
            grounds[idx(i, j)].open
        } else {
            0
        }
    };
    // The floors where two neighboring lots are one building.
    let one_building = |a: usize, b: usize| match (grounds[a].building, grounds[b].building) {
        (Some((oa, through)), Some((ob, _))) if oa == ob => through,
        _ => 0,
    };
    // Corner (i, j), the -x-z corner of lot (i, j): a full crossing where its four lots are open.
    let full = |i: i32, j: i32| open(i - 1, j - 1) & open(i, j - 1) & open(i - 1, j) & open(i, j);

    let mut west = vec![0u16; (W * W) as usize];
    let mut south = vec![0u16; (W * W) as usize];
    for j in 0..W {
        for i in 0..W {
            let k = idx(i, j);
            if i > 0 {
                west[k] = open(i - 1, j)
                    & open(i, j)
                    & !one_building(idx(i - 1, j), k)
                    & full(i, j)
                    & full(i, j + 1);
            }
            if j > 0 {
                south[k] = open(i, j - 1)
                    & open(i, j)
                    & !one_building(idx(i, j - 1), k)
                    & full(i, j)
                    & full(i + 1, j);
            }
        }
    }

    // Dead ends cut back: a road stays where another road leaves each of its ends.
    for _ in 0..PRUNE {
        let w = |i: i32, j: i32| if inside(i, j) { west[idx(i, j)] } else { 0 };
        let s = |i: i32, j: i32| if inside(i, j) { south[idx(i, j)] } else { 0 };
        let mut next_west = west.clone();
        let mut next_south = south.clone();
        for j in 0..W {
            for i in 0..W {
                let k = idx(i, j);
                // Along z, from corner (i, j) to (i, j + 1).
                next_west[k] &= (w(i, j - 1) | s(i, j) | s(i - 1, j))
                    & (w(i, j + 1) | s(i, j + 1) | s(i - 1, j + 1));
                // Along x, from corner (i, j) to (i + 1, j).
                next_south[k] &= (s(i - 1, j) | w(i, j) | w(i, j - 1))
                    & (s(i + 1, j) | w(i + 1, j) | w(i + 1, j - 1));
            }
        }
        west = next_west;
        south = next_south;
    }

    // Whether the network reaching corner (i, j) on floor f spans `SMALL` lots or less both ways,
    // walked from that corner: the same answer from any corner of it.
    let w = |i: i32, j: i32| if inside(i, j) { west[idx(i, j)] } else { 0 };
    let s = |i: i32, j: i32| if inside(i, j) { south[idx(i, j)] } else { 0 };
    let small = |f: usize, start: (i32, i32)| {
        let bit = 1u16 << f;
        let (mut lo, mut hi) = (start, start);
        let mut seen = vec![start];
        let mut todo = vec![start];
        while let Some((i, j)) = todo.pop() {
            let next = [
                (w(i, j), (i, j + 1)),
                (w(i, j - 1), (i, j - 1)),
                (s(i, j), (i + 1, j)),
                (s(i - 1, j), (i - 1, j)),
            ];
            for (road, c) in next {
                if road & bit == 0 || seen.contains(&c) {
                    continue;
                }
                if (c.0 - start.0).abs() > SMALL || (c.1 - start.1).abs() > SMALL {
                    return false;
                }
                lo = (lo.0.min(c.0), lo.1.min(c.1));
                hi = (hi.0.max(c.0), hi.1.max(c.1));
                seen.push(c);
                todo.push(c);
            }
        }
        hi.0 - lo.0 <= SMALL && hi.1 - lo.1 <= SMALL
    };

    (0..TILE)
        .flat_map(|b| (0..TILE).map(move |a| (a, b)))
        .map(|(a, b)| {
            let (i, j) = (MARGIN + a, MARGIN + b);
            let k = idx(i, j);
            // Both of the lot's roads leave its -x-z corner: one network.
            let mut keep = west[k] | south[k];
            for f in 0..floors::MAX {
                if keep >> f & 1 != 0 && small(f, (i, j)) {
                    keep &= !(1 << f);
                }
            }
            Roads {
                west: west[k] & keep,
                south: south[k] & keep,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every road of a window of lots on every floor: the lot it borders, the step across, the
    /// floor.
    fn all_roads(seed: u64, r: i32) -> Vec<((i32, i32), (i32, i32), usize)> {
        let mut out = Vec::new();
        for lz in -r..r {
            for lx in -r..r {
                let here = roads(seed, lx, lz);
                for f in 0..floors::count() {
                    if here.west >> f & 1 != 0 {
                        out.push(((lx, lz), (-1, 0), f));
                    }
                    if here.south >> f & 1 != 0 {
                        out.push(((lx, lz), (0, -1), f));
                    }
                }
            }
        }
        out
    }

    #[test]
    fn tiles_agree_with_their_neighbors() {
        // Windows straddling tile borders, worked out on their own, find the same roads.
        for (seed, (x0, z0)) in [(1, (3, -5)), (7, (-6, 2)), (7, (10, 13))] {
            let shifted = network(seed, x0, z0);
            for b in 0..TILE {
                for a in 0..TILE {
                    let (x, z) = (x0 + a, z0 + b);
                    assert_eq!(
                        shifted[(b * TILE + a) as usize],
                        roads(seed, x, z),
                        "seed {seed}: lot ({x}, {z})"
                    );
                }
            }
        }
    }

    #[test]
    fn roads_run_between_full_crossings() {
        // Never into a hole, the edge of a deck, a square or a building: the lots on both sides
        // of a road and around both its ends are open.
        for seed in [1, 7] {
            let all = all_roads(seed, 12);
            assert!(all.len() > 1000, "seed {seed}: only {} roads", all.len());
            for ((lx, lz), (dx, dz), f) in all {
                // The road's corners: lot (lx, lz)'s -x-z corner and the next one along it.
                let (ex, ez) = (dz.abs(), dx.abs());
                for (cx, cz) in [(lx, lz), (lx + ex, lz + ez)] {
                    for (x, z) in [(cx - 1, cz - 1), (cx, cz - 1), (cx - 1, cz), (cx, cz)] {
                        let g = ground(seed, x, z);
                        assert!(
                            g.open >> f & 1 != 0,
                            "seed {seed}, floor {f}: the road along ({lx}, {lz}) toward \
                             ({dx}, {dz}) touches lot ({x}, {z})"
                        );
                    }
                }
                let (a, b) = (ground(seed, lx, lz), ground(seed, lx + dx, lz + dz));
                let one = matches!((a.building, b.building), (Some((p, t)), Some((q, _))) if p == q && t >> f & 1 != 0);
                assert!(
                    !one,
                    "seed {seed}, floor {f}: a road through a building at ({lx}, {lz})"
                );
            }
        }
    }

    #[test]
    fn every_network_spans_more_than_a_couple_of_blocks() {
        for seed in [1, 7] {
            for ((lx, lz), _, f) in all_roads(seed, 8) {
                // Walk the network from the road's first corner until it proves wide enough.
                let start = (lx, lz);
                let (mut seen, mut todo) = (vec![start], vec![start]);
                let (mut lo, mut hi) = (start, start);
                while let Some((x, z)) = todo.pop() {
                    if hi.0 - lo.0 > SMALL || hi.1 - lo.1 > SMALL {
                        break;
                    }
                    let next = [
                        (roads(seed, x, z).west, (x, z + 1)),
                        (roads(seed, x, z - 1).west, (x, z - 1)),
                        (roads(seed, x, z).south, (x + 1, z)),
                        (roads(seed, x - 1, z).south, (x - 1, z)),
                    ];
                    for (road, c) in next {
                        if road >> f & 1 != 0 && !seen.contains(&c) {
                            lo = (lo.0.min(c.0), lo.1.min(c.1));
                            hi = (hi.0.max(c.0), hi.1.max(c.1));
                            seen.push(c);
                            todo.push(c);
                        }
                    }
                }
                assert!(
                    hi.0 - lo.0 > SMALL || hi.1 - lo.1 > SMALL,
                    "seed {seed}, floor {f}: a small network around ({lx}, {lz})"
                );
            }
        }
    }

    #[test]
    fn roads_lead_somewhere() {
        // A road ending with no other road at its end only remains far out along a long strip.
        for seed in [1, 7] {
            let all = all_roads(seed, 12);
            let ends = all
                .iter()
                .filter(|&&((lx, lz), (dx, dz), f)| {
                    let (ex, ez) = (dz.abs(), dx.abs());
                    // Another road at corner c: two or more meet there.
                    let others = |(x, z): (i32, i32)| {
                        let leaving = [
                            roads(seed, x, z).west,
                            roads(seed, x, z - 1).west,
                            roads(seed, x, z).south,
                            roads(seed, x - 1, z).south,
                        ];
                        leaving.iter().filter(|&&m| m >> f & 1 != 0).count() >= 2
                    };
                    !others((lx, lz)) || !others((lx + ex, lz + ez))
                })
                .count();
            assert!(
                ends * 50 < all.len(),
                "seed {seed}: {ends} dead ends among {} roads",
                all.len()
            );
        }
    }
}
