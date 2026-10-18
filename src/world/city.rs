//! The city skeleton: a grid of lots, each holding a tower, deck slabs on the floors of the
//! stack (`floors.rs`), stairs between floors and bridges to its neighbors. Everything about a
//! lot is a pure function of `(seed, lot x, lot z)`, so lots can be generated independently and
//! in any order, and the renderer can ask where the decks are without generating anything.
//!
//! - D0, the bottom: always solid ground. Towers rise from it.
//! - Floors above: a deck on every lot, except where a hole chunk opens its floor and every floor
//!   above (light wells, gaps, the unbuilt edge: rare low down, common at the construction front),
//!   where a crater opens, and where two decks would meet by a corner alone (`deck`).
//! - Craters: round clusters of lots where nothing is built above some floor, so the sun pours
//!   down into the dark. One always opens near the start.
//! - Stairs: switchbacks in the middle lot of a stair hub chunk, each linking one floor to the
//!   next, rarer higher up (`stair`).
//! - The construction front: the tallest towers rise past the top deck, some still unfinished.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, OnceLock};

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
const SALT_PINCH: i32 = 17;
const SALT_FOOTBRIDGE: i32 = 19;
const SALT_UNDERSIDE: i32 = 23;

/// Side, in lots, of the squares whose slabs share a tint on a floor (poured together).
const POUR: i32 = 3;

#[derive(Clone, Copy)]
pub struct Tower {
    pub center: Vec3,
    pub size: Vec3,
    /// The top of the base tier's walls (`tower::solid_top`), under its setbacks and frames.
    pub walls_top: f32,
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

/// The corner of a lot, on the ground.
pub fn lot_origin(lx: i32, lz: i32) -> Vec3 {
    vec3(lx as f32 * LOT, 0.0, lz as f32 * LOT)
}

/// The lot holding a point.
pub fn lot_of(p: Vec3) -> (i32, i32) {
    ((p.x / LOT).floor() as i32, (p.z / LOT).floor() as i32)
}

/// A gray between `lo` and `hi`, drifting slightly cool or warm so neighboring slabs and towers do
/// not read as one mass.
pub fn tint(rng: &mut Rng, lo: i32, hi: i32) -> Color {
    let s = rng.range_i(lo, hi);
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
/// Two decks never meet by a corner alone: where they would, one of them is left open (`pinched`).
pub fn deck(seed: u64, lx: i32, lz: i32, floor: usize) -> Deck {
    static DECKS: Memo<Vec<Deck>> = OnceLock::new();
    memo(&DECKS, seed, lx, lz, || {
        opened_corners(seed, lx, lz, |x, z, f| unpinched(seed, x, z, f))
    })
    .get(floor)
    .copied()
    .unwrap_or(Deck::None)
}

/// The decks once the corners of the plan are opened, a first time (opening one may leave two
/// others meeting by a corner, which `deck` opens in turn).
fn unpinched(seed: u64, lx: i32, lz: i32, floor: usize) -> Deck {
    static DECKS: Memo<Vec<Deck>> = OnceLock::new();
    memo(&DECKS, seed, lx, lz, || {
        opened_corners(seed, lx, lz, |x, z, f| laid(seed, x, z, f).0)
    })
    .get(floor)
    .copied()
    .unwrap_or(Deck::None)
}

/// A lot's decks on every floor, open where `pinched` over the decks `before`.
fn opened_corners(
    seed: u64,
    lx: i32,
    lz: i32,
    before: impl Fn(i32, i32, usize) -> Deck,
) -> Vec<Deck> {
    (0..floors::count())
        .map(|f| {
            let kind = before(lx, lz, f);
            let has = |x: i32, z: i32| before(x, z, f) != Deck::None;
            if kind != Deck::None && pinched(seed, lx, lz, f, has) {
                Deck::None
            } else {
                kind
            }
        })
        .collect()
}

/// Whether a lot's deck gives way to leave a corner open: it meets a diagonal neighbor's deck by
/// the corner alone (no deck on the two lots between, as `has` says), and of the two it is the one
/// the seed picks. A deck the plan keeps (`laid`) never gives way, so the other one does.
fn pinched(seed: u64, lx: i32, lz: i32, floor: usize, has: impl Fn(i32, i32) -> bool) -> bool {
    let kept = |x: i32, z: i32| laid(seed, x, z, floor).1;
    if kept(lx, lz) {
        return false;
    }
    let roll = |x: i32, z: i32| hash_coords(seed, x, SALT_PINCH + 64 * floor as i32, z);
    [(-1, -1), (-1, 1), (1, -1), (1, 1)]
        .into_iter()
        .any(|(dx, dz)| {
            let (x, z) = (lx + dx, lz + dz);
            has(x, z) && !has(x, lz) && !has(lx, z) && (kept(x, z) || roll(lx, lz) < roll(x, z))
        })
}

/// Per-lot answers of the plan, asked for again and again by the rules of the lots around.
pub(super) type Memo<V> = OnceLock<Mutex<HashMap<(u64, u64, i32, i32), V>>>;

/// Lots a memo keeps before starting over.
const MEMO_LOTS: usize = 50_000;

/// The memoized answer for a lot (keyed by seed, lot and catalog generation), else `compute`d.
pub(super) fn memo<V: Clone>(
    store: &Memo<V>,
    seed: u64,
    lx: i32,
    lz: i32,
    compute: impl FnOnce() -> V,
) -> V {
    let store = store.get_or_init(Default::default);
    let key = (chunks::generation(), seed, lx, lz);
    if let Some(v) = store.lock().unwrap().get(&key) {
        return v.clone();
    }
    let v = compute();
    let mut m = store.lock().unwrap();
    if m.len() >= MEMO_LOTS {
        m.clear();
    }
    m.insert(key, v.clone());
    v
}

/// What the plan lays on a lot on a floor, before corners are opened (`deck`), and whether that
/// deck must stay: the ground, the start, a chunk's or a stair's floor.
fn laid(seed: u64, lx: i32, lz: i32, floor: usize) -> (Deck, bool) {
    static LAID: Memo<Vec<(Deck, bool)>> = OnceLock::new();
    let laid = memo(&LAID, seed, lx, lz, || {
        (0..floors::count())
            .map(|f| laid_uncached(seed, lx, lz, f))
            .collect()
    });
    laid.get(floor).copied().unwrap_or((Deck::None, true))
}

fn laid_uncached(seed: u64, lx: i32, lz: i32, floor: usize) -> (Deck, bool) {
    // A previewed chunk stands on a bare deck at the start floor, open where it has holes.
    if chunks::preview().is_some() {
        let open = floor == START && preview_opens(lx, lz);
        let kind = if (floor == 0 || floor == START) && !open {
            Deck::Full
        } else {
            Deck::None
        };
        return (kind, true);
    }
    if floor == 0 || (floor == START && is_plaza(lx, lz)) {
        return (Deck::Full, true);
    }
    // A hole chunk (a light well, a gap, the unbuilt edge) opens its floor and every floor above.
    if planner::hole_at(seed, lx, lz).is_some_and(|f| floor >= f) {
        return (Deck::None, true);
    }
    // An area's or a block's floor is a full deck under it.
    if planner::area_at(seed, lx, lz).is_some_and(|a| a.floor == floor)
        || planner::blocks_at(seed, lx, lz)
            .iter()
            .any(|b| b.floor == floor)
    {
        return (Deck::Full, true);
    }
    if let Some(f) = crater_at(seed, lx, lz) {
        if floor > f {
            return (Deck::None, true);
        }
        if floor == f {
            return (Deck::Full, true);
        }
    }
    if let Some((lo, hi)) = stair(seed, lx, lz) {
        if floor == lo {
            return (Deck::Full, true);
        }
        if floor > lo && floor <= hi {
            return (Deck::Ring, true);
        }
    }
    (Deck::Full, false)
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

/// One bit per floor (bit `f` for floor `f`), set where a chunk standing on the lot paves its deck
/// over with its own ground (`Chunk::ground`: a square, a market, a yard). No road runs there
/// (`roads.rs`).
pub fn paved(seed: u64, lx: i32, lz: i32) -> u16 {
    static PAVED: Memo<u16> = OnceLock::new();
    memo(&PAVED, seed, lx, lz, || {
        placements(seed, lx, lz)
            .iter()
            .filter(|p| p.covers(lx, lz) && paves(p) && stands_on(seed, p, (lx, lz)))
            .fold(0, |bits, p| bits | 1 << p.floor)
    })
}

/// Whether a placed chunk paves its floor over (`Chunk::ground`).
fn paves(p: &Placement) -> bool {
    chunks::catalog(p.biome)
        .chunks
        .get(&p.chunk)
        .is_some_and(|c| c.ground.is_some())
}

/// The tower standing on a lot, if any (`planner::tower_at`): its base tier's footprint from the
/// bottom of the stack to its top. Pure, so neighbors can query it for bridges.
pub fn tower(seed: u64, lx: i32, lz: i32) -> Option<Tower> {
    let p = planner::tower_at(seed, lx, lz)?;
    let cat = chunks::catalog(p.biome);
    let b = super::tower::body(&cat, &p)?;
    Some(Tower {
        center: (b.min + b.max) * 0.5,
        size: b.size(),
        walls_top: super::tower::solid_top(&cat, &p).unwrap_or(f32::MIN),
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
    lots: HashMap<LotKey, Arc<(Vec<Block>, Dressed)>>,
    order: VecDeque<LotKey>,
}

/// Lots the cache keeps (a little more than the loaded square and its border).
const CACHED_LOTS: usize = 400;

/// `generate_lot`, also returning the lot's props and the flowers of its points of interest.
pub fn generate_lot_dressed(seed: u64, lx: i32, lz: i32, out: &mut Vec<Block>) -> Dressed {
    static CACHE: OnceLock<Mutex<LotCache>> = OnceLock::new();
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
    let lot = Arc::new((out[first..].to_vec(), dressed.clone()));
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
        if kind == Deck::Ring {
            push_ring_deck(mid, y, well(seed, lx, lz), color, out);
        } else {
            push_underside(seed, lx, lz, f, out);
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

    out.extend(skybridges(seed, lx, lz));
    if chunks::preview().is_none() {
        push_footbridges(seed, lx, lz, out);
    }

    push_chunks(seed, lx, lz, first, out, dressed);

    // Things left behind, by rule: cables across the streets, debris at the foot of the walls.
    out.extend(super::leftovers::cables(seed, lx, lz));
    super::leftovers::scatter(seed, lx, lz, &out[first..], &mut dressed.props);
}

/// Depth of the girders under a deck, and of the joists between them.
const GIRDER: (f32, f32) = (1.4, 0.8);
const JOIST: (f32, f32) = (0.7, 0.35);

/// What hangs under a full deck on `floor` (a stairwell's ring has none, for the flights' head
/// room): girders along the lot's -x and -z borders (and its +x / +z edges where no deck goes on),
/// joists every 6 m between them, pipe runs that line up from lot to lot along a row, and now and
/// then a dead lamp on its rod. Nothing grows on them (`BlockKind::Hung`).
fn push_underside(seed: u64, lx: i32, lz: i32, floor: usize, out: &mut Vec<Block>) {
    use chunks::Mat;
    let o = lot_origin(lx, lz);
    let top = floors::y(floor) - DECK_THICKNESS;
    let mut put = |min: Vec3, max: Vec3, mat: Mat| {
        out.push(Block {
            aabb: Aabb { min, max },
            color: mat.color(),
            kind: BlockKind::Hung(mat),
        });
    };
    let (gd, gw) = GIRDER;
    let g = top - gd;
    put(
        vec3(o.x, g, o.z),
        vec3(o.x + LOT, top, o.z + gw),
        Mat::Concrete,
    );
    put(
        vec3(o.x, g, o.z + gw),
        vec3(o.x + gw, top, o.z + LOT),
        Mat::Concrete,
    );
    if deck(seed, lx + 1, lz, floor) == Deck::None {
        put(
            vec3(o.x + LOT - gw, g, o.z + gw),
            vec3(o.x + LOT, top, o.z + LOT),
            Mat::Concrete,
        );
    }
    if deck(seed, lx, lz + 1, floor) == Deck::None {
        put(
            vec3(o.x + gw, g, o.z + LOT - gw),
            vec3(o.x + LOT, top, o.z + LOT),
            Mat::Concrete,
        );
    }
    // Joists one way or the other, by the pour the slab belongs to.
    let (px, pz) = (lx.div_euclid(POUR), lz.div_euclid(POUR));
    let mut rng = Rng::new(hash_coords(
        seed,
        px * 16 + floor as i32,
        SALT_UNDERSIDE,
        pz,
    ));
    let (jd, jw) = JOIST;
    let along_x = rng.chance(0.5);
    for k in 1..4 {
        let t = k as f32 * 6.0 - jw * 0.5;
        if along_x {
            put(
                vec3(o.x + gw, top - jd, o.z + t),
                vec3(o.x + LOT, top, o.z + t + jw),
                Mat::Concrete,
            );
        } else {
            put(
                vec3(o.x + t, top - jd, o.z + gw),
                vec3(o.x + t + jw, top, o.z + LOT),
                Mat::Concrete,
            );
        }
    }
    // Pipes: a run along x shared by every lot of the row (and one along z by the column), under
    // the joists, two or three side by side.
    let pipe_y = top - jd;
    for (run_x, line) in [(true, lz), (false, lx)] {
        let mut r = Rng::new(hash_coords(
            seed,
            line,
            SALT_UNDERSIDE + 1,
            floor as i32 * 2 + run_x as i32,
        ));
        if !r.chance(0.3) {
            continue;
        }
        let at = r.range(3.0, LOT - 4.0).round();
        let count = r.range_i(2, 4);
        let mat = if r.chance(0.5) { Mat::Rust } else { Mat::Metal };
        for i in 0..count {
            let d = if i == 2 { 0.2 } else { 0.32 };
            let (c, y) = (at + i as f32 * 0.42, pipe_y - d);
            if run_x {
                put(
                    vec3(o.x, y, o.z + c),
                    vec3(o.x + LOT, pipe_y, o.z + c + d),
                    mat,
                );
            } else {
                put(
                    vec3(o.x + c, y, o.z),
                    vec3(o.x + c + d, pipe_y, o.z + LOT),
                    mat,
                );
            }
        }
    }
    // A dead lamp over the street, hanging from a girder's corner.
    let mut r = Rng::new(hash_coords(
        seed,
        lx,
        SALT_UNDERSIDE + 2,
        lz * 16 + floor as i32,
    ));
    if r.chance(0.45) {
        let (x, z) = (o.x + r.range(4.0, 8.0), o.z + r.range(4.0, 8.0));
        let (x, z) = if r.chance(0.5) {
            (x, o.z + 3.0)
        } else {
            (o.x + 3.0, z)
        };
        let drop = r.range(1.2, 3.0);
        put(
            vec3(x - 0.04, top - drop, z - 0.04),
            vec3(x + 0.04, top, z + 0.04),
            Mat::Metal,
        );
        put(
            vec3(x - 0.35, top - drop - 0.22, z - 0.2),
            vec3(x + 0.35, top - drop, z + 0.2),
            Mat::Metal,
        );
        put(
            vec3(x - 0.3, top - drop - 0.26, z - 0.16),
            vec3(x + 0.3, top - drop - 0.22, z + 0.16),
            Mat::Glass,
        );
    }
}

/// The lots a placement covers: corner lot and size.
#[derive(Clone, Copy)]
struct Footprint {
    origin: (i32, i32),
    size: (i32, i32),
}

impl Footprint {
    fn of(p: &Placement) -> Footprint {
        Footprint {
            origin: p.origin,
            size: p.size,
        }
    }

    /// The lot that builds a piece centered on `c`: the one holding it, or the nearest the
    /// placement covers (a crane's jib reaching out of its tower's lot), so every piece is built.
    fn owner(&self, c: Vec3) -> (i32, i32) {
        let clamp = |v: f32, o: i32, n: i32| ((v / LOT).floor() as i32).clamp(o, o + n - 1);
        (
            clamp(c.x, self.origin.0, self.size.0),
            clamp(c.z, self.origin.1, self.size.1),
        )
    }
}

/// The lots a box reaches into (touching a border is not reaching in).
fn lots_touched(a: &Aabb) -> impl Iterator<Item = (i32, i32)> {
    const E: f32 = 0.01;
    let lo = |v: f32| ((v + E) / LOT).floor() as i32;
    let hi = |v: f32| ((v - E) / LOT).floor() as i32;
    let (x0, x1, z0, z1) = (lo(a.min.x), hi(a.max.x), lo(a.min.z), hi(a.max.z));
    (z0..=z1.max(z0)).flat_map(move |z| (x0..=x1.max(x0)).map(move |x| (x, z)))
}

/// The deck slabs of a lot (a stair's ring counted whole), from the plan alone.
fn deck_slabs(seed: u64, lx: i32, lz: i32) -> Vec<Aabb> {
    let o = lot_origin(lx, lz);
    (1..floors::count())
        .filter(|&f| deck(seed, lx, lz, f) != Deck::None)
        .map(|f| {
            // Built as `generate_structure` builds them, to the last bit.
            Aabb::from_center(
                vec3(
                    o.x + LOT * 0.5,
                    floors::y(f) - DECK_THICKNESS * 0.5,
                    o.z + LOT * 0.5,
                ),
                vec3(LOT, DECK_THICKNESS, LOT),
            )
        })
        .collect()
}

/// What a lot keeps clear for the chunks of the lots around it, from the plan alone: its tower
/// (with room for what hangs on its walls), its stair, its footbridges and the skybridges crossing
/// it.
fn room(seed: u64, lx: i32, lz: i32) -> Vec<Aabb> {
    let mut out = footbridge_room(seed, lx, lz);
    out.extend(skybridges_through(seed, lx, lz));
    if let Some(t) = tower(seed, lx, lz) {
        out.push(Aabb::from_center(t.center, t.size + vec3(3.0, 0.0, 3.0)));
    }
    if let Some((lo, hi)) = stair(seed, lx, lz) {
        let w = well(seed, lx, lz);
        let mid = lot_origin(lx, lz) + vec3(LOT * 0.5, 0.0, LOT * 0.5);
        out.push(Aabb {
            min: vec3(mid.x - w.0, floors::y(lo), mid.z - w.1),
            max: vec3(mid.x + w.0, floors::y(hi) + RAIL_HEIGHT, mid.z + w.1),
        });
    }
    out
}

/// Pieces of a chunk or a tower in world coordinates: boxes, props and flowers, each in a group (a
/// module and its boxes stand or give way together).
struct Pieces {
    blocks: Vec<(Block, u32)>,
    props: Vec<(Placed, u32)>,
    blooms: Vec<(Vec3, u32)>,
}

impl Pieces {
    /// The groups touching a single lot with a piece overlapping one of `obstacles` (those
    /// spanning several lots are decided once for all of them: `spanning_blocked`).
    fn blocked(
        &self,
        obstacles: &[Aabb],
        foot: Footprint,
        test: impl Fn(&Block) -> bool,
    ) -> Vec<u32> {
        let spanning = self.spans(foot, &test);
        let boxes = self
            .blocks
            .iter()
            .filter(|(b, _)| test(b))
            .map(|(b, g)| (b.aabb, *g));
        let props = self.props.iter().map(|(p, g)| (p.aabb(), *g));
        boxes
            .chain(props)
            .filter(|(a, g)| !spanning.contains_key(g) && obstacles.iter().any(|o| o.overlaps(a)))
            .map(|(_, g)| g)
            .collect()
    }

    /// The groups spanning several lots, with every lot each one touches (or is owned by); only
    /// the blocks passing `test` count.
    fn spans(
        &self,
        foot: Footprint,
        test: impl Fn(&Block) -> bool,
    ) -> HashMap<u32, Vec<(i32, i32)>> {
        let mut lots: HashMap<u32, Vec<(i32, i32)>> = HashMap::new();
        let mut add = |g: u32, a: &Aabb| {
            let v = lots.entry(g).or_default();
            for l in lots_touched(a).chain([foot.owner((a.min + a.max) * 0.5)]) {
                if !v.contains(&l) {
                    v.push(l);
                }
            }
        };
        for (b, g) in self.blocks.iter().filter(|(b, _)| test(b)) {
            add(*g, &b.aabb);
        }
        for (p, g) in &self.props {
            add(*g, &p.aabb());
        }
        lots.retain(|_, v| v.len() > 1);
        lots
    }

    /// The groups spanning several lots that cannot stand in one of them: where `stands` says the
    /// pieces have nothing to stand on, or one overlaps what that lot's `room` keeps clear. Every
    /// lot the placement covers decides the same, so a crane never keeps its jib without its mast.
    fn spanning_blocked(
        &self,
        foot: Footprint,
        test: impl Fn(&Block) -> bool,
        stands: impl Fn((i32, i32)) -> bool,
        room: impl Fn((i32, i32)) -> Vec<Aabb>,
    ) -> Vec<u32> {
        let mut rooms: HashMap<(i32, i32), Vec<Aabb>> = HashMap::new();
        let mut out = Vec::new();
        for (g, lots) in self.spans(foot, &test) {
            let boxes: Vec<Aabb> = self
                .blocks
                .iter()
                .filter(|(b, h)| *h == g && test(b))
                .map(|(b, _)| b.aabb)
                .chain(
                    self.props
                        .iter()
                        .filter(|(_, h)| *h == g)
                        .map(|(p, _)| p.aabb()),
                )
                .collect();
            let blocked = lots.iter().any(|&l| {
                !stands(l)
                    || rooms
                        .entry(l)
                        .or_insert_with(|| room(l))
                        .iter()
                        .any(|o| boxes.iter().any(|b| o.overlaps(b)))
            });
            if blocked {
                out.push(g);
            }
        }
        out
    }

    /// Adds this lot's share: the pieces of the groups not `dropped` it owns (`Footprint::owner`),
    /// and collision for those reaching in from the lots that own them.
    fn place(
        self,
        (lx, lz): (i32, i32),
        foot: Footprint,
        dropped: &[u32],
        out: &mut Vec<Block>,
        dressed: &mut Dressed,
    ) {
        let o = lot_origin(lx, lz);
        let lot = Aabb {
            min: vec3(o.x, f32::MIN, o.z),
            max: vec3(o.x + LOT, f32::MAX, o.z + LOT),
        };
        let owns = |c: Vec3| foot.owner(c) == (lx, lz);
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
/// on a roof ending just under a deck would pierce it: those give way, and so do those in the way of
/// a skybridge, and what reaches out over another tower (a crane's jib).
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
    let built = super::tower::build(&cat, &p, seed);
    let pieces = Pieces {
        blocks: built.blocks.into_iter().zip(built.groups).collect(),
        props: built.props.into_iter().zip(built.prop_groups).collect(),
        blooms: built.blooms.into_iter().zip(built.bloom_groups).collect(),
    };
    let mut obstacles: Vec<Aabb> = out[first..]
        .iter()
        .filter(|b| b.kind == BlockKind::Deck)
        .map(|b| b.aabb)
        .collect();
    obstacles.extend(skybridges_through(seed, lx, lz));
    // What a lot keeps clear for the pieces reaching into it from several lots.
    let room = |(x, z): (i32, i32)| {
        let mut room = deck_slabs(seed, x, z);
        room.extend(skybridges_through(seed, x, z));
        if !p.covers(x, z) {
            room.extend(tower(seed, x, z).map(|t| Aabb::from_center(t.center, t.size)));
        }
        room
    };
    let foot = Footprint::of(&p);
    let test = |b: &Block| !matches!(b.kind, BlockKind::Tower(_));
    let mut dropped = pieces.blocked(&obstacles, foot, test);
    dropped.extend(pieces.spanning_blocked(foot, test, |_| true, room));
    pieces.place((lx, lz), foot, &dropped, out, dressed);
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
    let env = chunks::Env {
        seed,
        floor: p.floor,
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

/// Whether a placed chunk's pieces have something to stand on in a lot. Where two chunks cover
/// the lot on one floor, the lesser gives way (`outranks`); every chunk needs a deck to stand on,
/// but an area, and a hole over its own void (a footbridge across it, girders sticking out). A
/// hole's rim where a hole below left no deck has nothing.
fn stands_on(seed: u64, p: &Placement, (lx, lz): (i32, i32)) -> bool {
    let open = p.role == Role::Area || (p.role == Role::Hole && p.opens(lx, lz));
    if !open && deck(seed, lx, lz, p.floor) == Deck::None {
        return false;
    }
    !p.covers(lx, lz)
        || !placements(seed, lx, lz)
            .iter()
            .any(|q| q.floor == p.floor && q.outranks(p))
}

/// This lot's part of the chunks covering it (`Pieces::place`). A chunk's pieces give way, a
/// module at a time, to the lot's own architecture (a tower, a stair, a footbridge) and to the
/// towers next door; a module reaching over several lots, to what any of them holds (`room`).
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
        .filter(|b| {
            !matches!(
                b.kind,
                BlockKind::Floor | BlockKind::Deck | BlockKind::Hung(_)
            )
        })
        .map(|b| b.aabb)
        .collect();
    obstacles.extend(dressed.props.iter().map(|p| p.aabb()));
    obstacles.extend(footbridge_room(seed, lx, lz));
    obstacles.extend(skybridges_through(seed, lx, lz));
    for (nx, nz) in [(lx - 1, lz), (lx + 1, lz), (lx, lz - 1), (lx, lz + 1)] {
        if let Some(t) = tower(seed, nx, nz) {
            obstacles.push(Aabb::from_center(t.center, t.size));
        }
    }
    // The greater chunks first: the lesser give way to what they placed (a street to its filler).
    let mut all = placements(seed, lx, lz);
    all.sort_by_key(|p| p.rank());
    for p in &all {
        if !stands_on(seed, p, (lx, lz)) {
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
            .map(|(&(aabb, mat), &g)| (Block::built(aabb, mat), g))
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
        let foot = Footprint::of(p);
        let mut dropped = pieces.blocked(&obstacles, foot, |_| true);
        dropped.extend(pieces.spanning_blocked(
            foot,
            |_| true,
            |l| stands_on(seed, p, l),
            |(x, z)| room(seed, x, z),
        ));
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
        pieces.place((lx, lz), foot, &dropped, out, dressed);
    }
}

/// The skybridges between this lot's tower and the towers of its +x and +z neighbors, from the plan
/// alone, so the lots they cross can keep clear for them. Each lot pair is decided by the lower lot,
/// and two big towers facing each other across several lot pairs get the bridges of the first pair
/// only. Own random stream.
pub(super) fn skybridges(seed: u64, lx: i32, lz: i32) -> Vec<Block> {
    static BRIDGES: Memo<Vec<Block>> = OnceLock::new();
    memo(&BRIDGES, seed, lx, lz, || {
        let mut out = Vec::new();
        let (Some(a), Some(pa)) = (tower(seed, lx, lz), planner::tower_at(seed, lx, lz)) else {
            return out;
        };
        let biome = biome_at(seed, lx, lz);
        let cat = chunks::catalog(biome.name);
        let chance = biome.bridge_chance;
        let mut rng = Rng::new(hash_coords(seed, lx, SALT_BRIDGE, lz));
        for (nx, nz) in [(lx + 1, lz), (lx, lz + 1)] {
            let (Some(b), Some(pb)) = (tower(seed, nx, nz), planner::tower_at(seed, nx, nz)) else {
                continue;
            };
            let along_x = nx != lx;
            let first = if along_x {
                lz == pa.origin.1.max(pb.origin.1)
            } else {
                lx == pa.origin.0.max(pb.origin.0)
            };
            // Now and then a second one, at another height.
            let (mut heights, mut built) = (Vec::new(), Vec::new());
            for chance in [chance, chance * 0.5] {
                if rng.chance(chance) {
                    push_bridge(&mut rng, &cat, &a, &b, along_x, &mut heights, &mut built);
                }
            }
            if first {
                out.extend(built);
            }
        }
        out
    })
}

/// The boxes of the skybridges that may cross a lot: its own, its -x and -z neighbors'.
fn skybridges_through(seed: u64, lx: i32, lz: i32) -> impl Iterator<Item = Aabb> {
    [(lx, lz), (lx - 1, lz), (lx, lz - 1)]
        .into_iter()
        .flat_map(move |(x, z)| skybridges(seed, x, z))
        .map(|b| b.aabb)
}

/// Widest gap a footbridge spans, in lots (48 m; wider would be physically unlikely).
const FOOTBRIDGE_MAX: i32 = 2;
/// Chance that a gap of a footbridge's size gets one, per axis.
const FOOTBRIDGE_CHANCE: f32 = 0.7;
/// Half the width of a footbridge (`footbridge_span`), and the clearance kept around it.
const FOOTBRIDGE_HALF: f32 = 2.0;
/// How far a footbridge's landing reaches into the deck at each end, kept clear of what stands
/// there (railings, a house, a tower's wall).
const BRIDGE_LANDING: f32 = 3.0;

/// The footbridges crossing this lot on a floor, by axis (true: along x). A floor's gaps of one
/// or two lots between two decks get one now and then, along the lots' middle line, unless a
/// tower stands in the way or a wall at either end; each axis is decided for the whole gap, so
/// two bridges may cross. Pure, like the decks.
pub fn bridges_through(seed: u64, lx: i32, lz: i32, floor: usize) -> Vec<bool> {
    static BRIDGES: Memo<Vec<Vec<bool>>> = OnceLock::new();
    memo(&BRIDGES, seed, lx, lz, || {
        (0..floors::count())
            .map(|f| bridges_through_uncached(seed, lx, lz, f))
            .collect()
    })
    .get(floor)
    .cloned()
    .unwrap_or_default()
}

fn bridges_through_uncached(seed: u64, lx: i32, lz: i32, floor: usize) -> Vec<bool> {
    if floor == 0 || deck(seed, lx, lz, floor) != Deck::None {
        return Vec::new();
    }
    let crosses = |along_x: bool| {
        let lot = |i: i32| if along_x { (lx + i, lz) } else { (lx, lz + i) };
        let hole = |i: i32| {
            let (x, z) = lot(i);
            deck(seed, x, z, floor) == Deck::None
        };
        // The gap's ends: the first decks on each side, within reach.
        let back = (1..=FOOTBRIDGE_MAX).find(|&i| !hole(-i))?;
        let ahead = (1..=FOOTBRIDGE_MAX).find(|&i| !hole(i))?;
        if back + ahead - 1 > FOOTBRIDGE_MAX {
            return None;
        }
        let y = floors::y(floor);
        // No tower through the gap, and none on the landings.
        let free = (1 - back..ahead).all(|i| {
            let (x, z) = lot(i);
            tower(seed, x, z).is_none_or(|t| t.top() < y - 2.0)
        });
        let start = lot(1 - back);
        let landings = [
            (lot(-back), landing(start, along_x, false, y)),
            (lot(ahead), landing(lot(ahead - 1), along_x, true, y)),
        ];
        let clear = landings.iter().all(|&((x, z), area)| {
            tower(seed, x, z).is_none_or(|t| {
                t.top() < y - 2.0 || !Aabb::from_center(t.center, t.size).overlaps(&area)
            })
        });
        let salt = SALT_FOOTBRIDGE + 64 * floor as i32 + along_x as i32;
        let mut rng = Rng::new(hash_coords(seed, start.0, salt, start.1));
        (free && clear && rng.chance(FOOTBRIDGE_CHANCE)).then_some(along_x)
    };
    [true, false].into_iter().filter_map(crosses).collect()
}

/// The deck a footbridge leaving gap lot `(lx, lz)` lands on: `BRIDGE_LANDING` deep into the next lot,
/// ahead (toward +x or +z) or behind, from the walkway up to head height.
fn landing((lx, lz): (i32, i32), along_x: bool, ahead: bool, y: f32) -> Aabb {
    let o = lot_origin(lx, lz);
    let (u0, u1) = if ahead {
        (LOT, LOT + BRIDGE_LANDING)
    } else {
        (-BRIDGE_LANDING, 0.0)
    };
    let (v0, v1) = (
        LOT * 0.5 - FOOTBRIDGE_HALF - 0.5,
        LOT * 0.5 + FOOTBRIDGE_HALF + 0.5,
    );
    let (x, z) = if along_x {
        ((u0, u1), (v0, v1))
    } else {
        ((v0, v1), (u0, u1))
    };
    Aabb {
        min: vec3(o.x + x.0, y - 1.5, o.z + z.0),
        max: vec3(o.x + x.1, y + 2.5, o.z + z.1),
    }
}

/// What the footbridges around a lot keep clear in it, on every floor: their spans through it
/// and their landings on its decks.
fn footbridge_room(seed: u64, lx: i32, lz: i32) -> Vec<Aabb> {
    let mut out = Vec::new();
    for f in 1..floors::count() {
        let y = floors::y(f);
        for along_x in bridges_through(seed, lx, lz, f) {
            out.push(Aabb::from_center(
                lot_origin(lx, lz) + vec3(LOT * 0.5, y + 0.5, LOT * 0.5),
                if along_x {
                    vec3(LOT, 3.0, FOOTBRIDGE_HALF * 2.0)
                } else {
                    vec3(FOOTBRIDGE_HALF * 2.0, 3.0, LOT)
                },
            ));
        }
        for (dx, dz) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            let (nx, nz) = (lx + dx, lz + dz);
            if bridges_through(seed, nx, nz, f).contains(&(dx != 0)) {
                // This lot is ahead of the gap lot when the gap lot is behind it.
                out.push(landing((nx, nz), dx != 0, dx + dz < 0, y));
            }
        }
    }
    out
}

/// This lot's span of its footbridges (`footbridge_span` modules, one per lot crossed).
fn push_footbridges(seed: u64, lx: i32, lz: i32, out: &mut Vec<Block>) {
    let cat = chunks::catalog(biome_at(seed, lx, lz).name);
    for f in 1..floors::count() {
        for along_x in bridges_through(seed, lx, lz, f) {
            let o = lot_origin(lx, lz) + Vec3::Y * floors::y(f);
            // The module runs along +x from its origin, centered across: turned to run along +z.
            let tf = if along_x {
                chunks::Tf::shift(o + vec3(0.0, 0.0, LOT * 0.5))
            } else {
                chunks::Tf::shift(o + vec3(LOT * 0.5, 0.0, 0.0)).then(&chunks::Tf::turn(3))
            };
            let parts = [chunks::Part::Module {
                name: "footbridge_span".to_string(),
                at: (0.0, 0.0, 0.0),
                turn: 0,
                accent: None,
            }];
            let built = chunks::expand(&cat, &parts, &tf, 0);
            out.extend(
                built
                    .boxes
                    .into_iter()
                    .map(|(aabb, mat)| Block::built(aabb, mat)),
            );
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

/// Half size (x, z) of a lot's stairwell, the hole in the decks it climbs through.
fn well(seed: u64, lx: i32, lz: i32) -> (f32, f32) {
    if stair_along_x(seed, lx, lz) {
        WELL
    } else {
        (WELL.1, WELL.0)
    }
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

/// A walkway with low parapets between two towers (`skybridge_segment` modules of `cat`, the
/// deciding lot's biome, from wall to wall), at a height the seed picks between their tops, away
/// from the `heights` of the pair's other bridges. `along_x` is the direction from `a` to `b`.
#[allow(clippy::too_many_arguments)]
fn push_bridge(
    rng: &mut Rng,
    cat: &chunks::Catalog,
    a: &Tower,
    b: &Tower,
    along_x: bool,
    heights: &mut Vec<f32>,
    out: &mut Vec<Block>,
) {
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
    // Both ends on solid walls: under the setbacks and frames going up, which are narrower.
    let y_lo = floors::y(1) + 6.0;
    let y_hi = a.walls_top.min(b.walls_top) - 3.0;
    if y_hi <= y_lo {
        return;
    }
    let y = (rng.range(y_lo, y_hi) / 6.0).round() * 6.0;
    let y = y.clamp(y_lo, y_hi);
    // Not level with a floor, where it would run through the decks.
    if (0..floors::count()).any(|f| (y - floors::y(f)).abs() < 4.0)
        || heights.iter().any(|h| (y - h).abs() < 6.0)
    {
        return;
    }
    heights.push(y);
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
    let built = chunks::expand(cat, &parts, &tf, 0);
    out.extend(
        built
            .boxes
            .into_iter()
            .map(|(aabb, mat)| Block::built(aabb, mat)),
    );
}

/// Hand-placed blocks near spawn to test steps, jumps and ledge grabs.
fn push_movement_course(lx: i32, lz: i32, out: &mut Vec<Block>) {
    for (i, h) in [0.4_f32, 1.0, 1.6, 2.4, 3.2].iter().enumerate() {
        let c = vec3(-8.0 + i as f32 * 4.0, floors::y(START) + h * 0.5, 10.0);
        if lot_of(c) != (lx, lz) {
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
                    // A crossing of roads (`roads::crossing`), not a road going on or a sidewalk.
                    let corner = |v: f32, o: i32| o + (v > LOT * 0.5) as i32;
                    let c = (corner(pole.x, lx), corner(pole.z, lz));
                    let floor = floors::floor_at(p.pos.y);
                    assert!(
                        super::super::roads::crossing(1, c, floor),
                        "lot {lx},{lz}: a street light off the crossings, at corner {c:?}, floor {floor}"
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

    #[test]
    fn big_towers_collide_from_every_lot_they_cover() {
        let seed = 7;
        let mut lots = 0;
        for lz in -6..6 {
            for lx in -6..6 {
                let (Some(p), Some(t)) = (planner::tower_at(seed, lx, lz), tower(seed, lx, lz))
                else {
                    continue;
                };
                if p.size == (1, 1) {
                    continue;
                }
                lots += 1;
                // A point of the body in this lot, low in the stack where every tower stands.
                let (o, b) = (lot_origin(lx, lz), Aabb::from_center(t.center, t.size));
                let mid = |lo: f32, hi: f32, o: f32| (lo.max(o) + hi.min(o + LOT)) * 0.5;
                let at = vec3(
                    mid(b.min.x, b.max.x, o.x),
                    floors::bottom() + 10.0,
                    mid(b.min.z, b.max.z, o.z),
                );
                let mut blocks = Vec::new();
                generate_lot(seed, lx, lz, &mut blocks);
                assert!(
                    blocks.iter().any(|b| b.aabb.contains_xz(at)
                        && b.aabb.min.y < at.y
                        && b.aabb.max.y > at.y),
                    "lot ({lx}, {lz}): nothing to collide with in {} at {at}",
                    p.chunk
                );
            }
        }
        assert!(lots > 0, "no big tower");
    }

    #[test]
    fn skybridges_cross_nothing() {
        let seed = 1;
        let inner = |a: &Aabb| Aabb {
            min: a.min + 0.05,
            max: a.max - 0.05,
        };
        let same = |a: &Aabb, b: &Aabb| a.min == b.min && a.max == b.max;
        let mut all: Vec<((i32, i32), Aabb)> = Vec::new();
        for lz in -6..6 {
            for lx in -6..6 {
                let own: Vec<Aabb> = skybridges(seed, lx, lz).iter().map(|b| b.aabb).collect();
                all.extend(own.iter().map(|&a| ((lx, lz), a)));
                let through: Vec<Aabb> = skybridges_through(seed, lx, lz).collect();
                let mut blocks = Vec::new();
                let dressed = generate_lot_dressed(seed, lx, lz, &mut blocks);
                // Tower bodies take the bridges' ends; colliders are checked in their own lot.
                let pieces = blocks
                    .iter()
                    .filter(|b| {
                        !matches!(
                            b.kind,
                            BlockKind::Tower(_) | BlockKind::Collider | BlockKind::Prop
                        )
                    })
                    .map(|b| b.aabb)
                    .filter(|a| !own.iter().any(|o| same(o, a)))
                    .chain(dressed.props.iter().map(|p| p.aabb()));
                for a in pieces {
                    assert!(
                        !through.iter().any(|t| inner(t).overlaps(&a)),
                        "lot ({lx}, {lz}): {a:?} stands in a skybridge"
                    );
                }
            }
        }
        assert!(all.len() > 100, "only {} skybridge boxes", all.len());
        // Two bridges may meet only inside a tower they both land on (their ends sink into it).
        let bodies: Vec<Aabb> = (-7..7)
            .flat_map(|z| (-7..7).filter_map(move |x| tower(seed, x, z)))
            .map(|t| Aabb::from_center(t.center, t.size))
            .collect();
        for (i, (la, a)) in all.iter().enumerate() {
            for (lb, b) in &all[..i] {
                let meet = Aabb {
                    min: a.min.max(b.min),
                    max: a.max.min(b.max),
                };
                let buried = bodies
                    .iter()
                    .any(|t| t.min.cmple(meet.min).all() && t.max.cmpge(meet.max).all());
                assert!(
                    la == lb || !inner(a).overlaps(b) || buried,
                    "skybridges of {la:?} and {lb:?} cross"
                );
            }
        }
    }

    #[test]
    fn decks_never_meet_by_a_corner_alone() {
        let seed = 3;
        for f in 1..floors::count() {
            for lz in -12..12 {
                for lx in -12..12 {
                    let has = |x, z| deck(seed, x, z, f) != Deck::None;
                    for dx in [-1, 1] {
                        let corner = has(lx, lz)
                            && has(lx + dx, lz + 1)
                            && !has(lx + dx, lz)
                            && !has(lx, lz + 1);
                        // Unless both must stay (a stair's floor against a square's).
                        let kept = laid(seed, lx, lz, f).1 && laid(seed, lx + dx, lz + 1, f).1;
                        assert!(
                            !corner || kept,
                            "floor {f}: ({lx}, {lz}) meets ({}, {}) by a corner",
                            lx + dx,
                            lz + 1
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn footbridges_land_on_decks_at_both_ends() {
        let seed = 7;
        let mut n = 0;
        for f in 1..floors::count() {
            for lz in -12..12 {
                for lx in -12..12 {
                    for along_x in bridges_through(seed, lx, lz, f) {
                        n += 1;
                        let lot = |i: i32| if along_x { (lx + i, lz) } else { (lx, lz + i) };
                        let lands = |dir: i32| {
                            (1..=FOOTBRIDGE_MAX).any(|i| {
                                let (x, z) = lot(dir * i);
                                deck(seed, x, z, f) != Deck::None
                            })
                        };
                        assert!(
                            lands(-1) && lands(1),
                            "floor {f}: the footbridge over ({lx}, {lz}) has a loose end"
                        );
                    }
                }
            }
        }
        assert!(n > 0, "no footbridge at all");
    }
}
