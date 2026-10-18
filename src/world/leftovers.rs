//! Things left behind, placed by rule (milestone 10, pass 3): debris at the foot of the walls, more
//! under the tower facades (lumps of concrete, bricks, a fallen window frame or panel) than along
//! the low walls (bags, boxes, papers, a tyre), never in a road; and cables sagging across the
//! streets between towers facing each other. Ground cover at the wall feet and along the curbs is
//! grown with the plants (`growth::grow_lot`, from `wall_feet` and `in_road`). Pure functions of
//! the seed and the lot, like the rest.

use glam::{Vec3, vec3};

use super::biome::{biome_at, variant_of};
use super::chunks::Mat;
use super::city::{self, Deck, LOT};
use super::props::{Placed, Prop};
use super::{Aabb, Block, BlockKind, design, floors, roads};
use crate::rng::{Rng, hash_coords};

const SALT_DEBRIS: i32 = 31;
const SALT_CABLES: i32 = 37;

/// How far a road takes from the border it runs along (`pavement` in shaders/scene.fs: 3 m, and
/// the curb), so nothing left behind lies on it.
const ROAD: f32 = 3.4;

/// The foot of a wall on a floor: where it starts, the direction along it, its length, its outward
/// normal and the floor.
#[derive(Clone, Copy, Debug)]
pub struct Foot {
    pub start: Vec3,
    pub along: Vec3,
    pub len: f32,
    pub out: Vec3,
    pub floor: usize,
    /// A tower's facade (debris fell from it), not a low wall.
    pub facade: bool,
}

/// The feet of the lot's walls: tower walls on every floor they cross where the lot has a full deck
/// (or the ground), and the walls of built boxes at least 2 m tall standing on a floor.
pub fn wall_feet(seed: u64, lx: i32, lz: i32, blocks: &[Block]) -> Vec<Foot> {
    let mut out = Vec::new();
    let walkable = |f: usize| f == 0 || city::deck(seed, lx, lz, f) == Deck::Full;
    for b in blocks {
        let a = &b.aabb;
        let size = a.size();
        let floors_of: Vec<usize> = match b.kind {
            BlockKind::Tower(_) => (0..floors::count())
                .filter(|&f| {
                    let y = floors::y(f);
                    y >= a.min.y - 0.05 && y + 3.0 < a.max.y && walkable(f)
                })
                .collect(),
            BlockKind::Built(m) if m != Mat::Glass && size.y >= 2.0 => {
                let f = floors::floor_at(a.min.y);
                if (a.min.y - floors::y(f)).abs() < 0.05 && walkable(f) {
                    vec![f]
                } else {
                    Vec::new()
                }
            }
            _ => Vec::new(),
        };
        let facade = matches!(b.kind, BlockKind::Tower(_));
        for f in floors_of {
            let y = floors::y(f);
            let faces = [
                (vec3(a.min.x, y, a.min.z), Vec3::Z, size.z, Vec3::NEG_X),
                (vec3(a.max.x, y, a.min.z), Vec3::Z, size.z, Vec3::X),
                (vec3(a.min.x, y, a.min.z), Vec3::X, size.x, Vec3::NEG_Z),
                (vec3(a.min.x, y, a.max.z), Vec3::X, size.x, Vec3::Z),
            ];
            for (start, along, len, n) in faces {
                if len >= 1.5 {
                    out.push(Foot {
                        start,
                        along,
                        len,
                        out: n,
                        floor: f,
                        facade,
                    });
                }
            }
        }
    }
    out
}

/// Whether a point of the lot lies on a road of its floor, or in a crossing's corner, where nothing
/// is left lying (corners are always kept clear: a crossing's square is road).
pub fn in_road(seed: u64, (lx, lz): (i32, i32), floor: usize, p: Vec3) -> bool {
    let o = city::lot_origin(lx, lz);
    let (u, v) = (p.x - o.x, p.z - o.z);
    let near = |d: f32| d < ROAD;
    let (w, e, s, n) = (near(u), near(LOT - u), near(v), near(LOT - v));
    if (w || e) && (s || n) {
        return true;
    }
    (w && roads::along(seed, (lx, lz), (-1, 0), floor))
        || (e && roads::along(seed, (lx, lz), (1, 0), floor))
        || (s && roads::along(seed, (lx, lz), (0, -1), floor))
        || (n && roads::along(seed, (lx, lz), (0, 1), floor))
}

/// The debris of a lot: clusters at the foot of its walls, every 3 m at the floor profile's
/// `debris` chance (times how littered the biome is), each piece turned at random, kept inside
/// the lot, off the roads and clear of every box and prop already there.
pub fn scatter(seed: u64, lx: i32, lz: i32, blocks: &[Block], props: &mut Vec<Placed>) {
    if !design::props_on() {
        return;
    }
    let biome = biome_at(seed, lx, lz);
    let variant = variant_of(biome) as u8;
    let litter = biome.textures.littered / 0.5 * 1.4;
    let mut rng = Rng::new(hash_coords(seed, lx, SALT_DEBRIS, lz));
    let o = city::lot_origin(lx, lz);
    let solid: Vec<Aabb> = blocks
        .iter()
        .filter(|b| {
            !matches!(
                b.kind,
                BlockKind::Floor | BlockKind::Deck | BlockKind::Hung(_)
            )
        })
        .map(|b| b.aabb)
        .collect();
    // Fell from a facade, or was dropped against a wall.
    const FACADE: &[(Prop, f32)] = &[
        (Prop::DebrisLumps, 3.0),
        (Prop::DebrisHeap, 1.5),
        (Prop::DebrisBricks, 2.0),
        (Prop::DebrisSlab, 1.0),
        (Prop::DebrisFrame, 1.0),
        (Prop::DebrisPanel, 0.7),
        (Prop::DebrisPaper, 1.5),
        (Prop::TrashBag, 0.6),
    ];
    const WALL: &[(Prop, f32)] = &[
        (Prop::TrashBag, 2.0),
        (Prop::CardboardBox, 1.5),
        (Prop::DebrisPaper, 2.0),
        (Prop::DebrisTyre, 0.6),
        (Prop::DebrisPipe, 0.8),
        (Prop::DebrisLumps, 1.0),
        (Prop::DebrisBricks, 0.8),
    ];
    let pick = |rng: &mut Rng, list: &[(Prop, f32)]| {
        let total: f32 = list.iter().map(|(_, w)| w).sum();
        let mut r = rng.range(0.0, total);
        for &(p, w) in list {
            if r < w {
                return p;
            }
            r -= w;
        }
        list[0].0
    };
    let first = props.len();
    for foot in wall_feet(seed, lx, lz, blocks) {
        let chance = design::profile(foot.floor).debris * litter;
        let steps = (foot.len / 3.0).floor() as i32;
        for k in 0..steps {
            if !rng.chance(chance) {
                continue;
            }
            let t = (k as f32 + rng.range(0.2, 0.8)) * foot.len / steps as f32;
            let at = foot.start + foot.along * t;
            let count = rng.range_i(2, 6);
            for _ in 0..count {
                let prop = pick(&mut rng, if foot.facade { FACADE } else { WALL });
                let off = rng.range(0.25, if foot.facade { 1.6 } else { 0.9 });
                let slide = rng.range(-0.9, 0.9);
                let yaw = rng.range(0.0, std::f32::consts::TAU);
                // Out from the wall past what stands against it (a shop front, a stair), up to a
                // meter further.
                for step in 0..3 {
                    let placed = Placed {
                        prop,
                        biome: variant,
                        pos: at + foot.out * (off + step as f32 * 0.5) + foot.along * slide,
                        yaw,
                    };
                    let a = placed.aabb();
                    let inside = a.min.x > o.x + 0.2
                        && a.min.z > o.z + 0.2
                        && a.max.x < o.x + LOT - 0.2
                        && a.max.z < o.z + LOT - 0.2;
                    if !inside
                        || in_road(seed, (lx, lz), foot.floor, a.min)
                        || in_road(seed, (lx, lz), foot.floor, a.max)
                    {
                        break;
                    }
                    let pad = Aabb {
                        min: a.min - vec3(0.05, -0.01, 0.05),
                        max: a.max + vec3(0.05, 0.0, 0.05),
                    };
                    if !solid.iter().any(|s| s.overlaps(&pad))
                        && !props.iter().any(|q| q.aabb().overlaps(&pad))
                    {
                        props.push(placed);
                        break;
                    }
                }
            }
        }
    }
    debug_assert!(props[first..].iter().all(|p| p.pos.y >= floors::bottom()));
}

/// Cables between this lot's tower and the towers of its +x and +z neighbors where a street lies
/// between them: on each floor both walls rise through, at the floor's chance, one to four sagging
/// wires from wall to wall, a few meters over the street and well under the deck above. Drawn as
/// short boxes stepping down and up like pixels (`BlockKind::Hung`); the lower lot builds them.
pub fn cables(seed: u64, lx: i32, lz: i32) -> Vec<Block> {
    let mut out = Vec::new();
    let Some(a) = city::tower(seed, lx, lz) else {
        return out;
    };
    let bridges: Vec<Aabb> = city::skybridges(seed, lx, lz)
        .iter()
        .map(|b| b.aabb)
        .collect();
    for (dx, dz) in [(1, 0), (0, 1)] {
        let Some(b) = city::tower(seed, lx + dx, lz + dz) else {
            continue;
        };
        let (ax, cross) = if dx == 1 { (0, 2) } else { (2, 0) };
        let from = a.center[ax] + a.size[ax] * 0.5;
        let to = b.center[ax] - b.size[ax] * 0.5;
        let gap = to - from;
        if !(3.0..=30.0).contains(&gap) {
            continue;
        }
        let lo = (a.center[cross] - a.size[cross] * 0.5).max(b.center[cross] - b.size[cross] * 0.5);
        let hi = (a.center[cross] + a.size[cross] * 0.5).min(b.center[cross] + b.size[cross] * 0.5);
        if hi - lo < 2.0 {
            continue;
        }
        let top = a.walls_top.min(b.walls_top);
        let bottom = (a.center.y - a.size.y * 0.5).max(b.center.y - b.size.y * 0.5);
        for f in 0..floors::count() {
            let y = floors::y(f);
            let ceiling = if f + 1 < floors::count() {
                floors::y(f + 1) - 4.5
            } else {
                y + 20.0
            };
            let (y_lo, y_hi) = (y + 5.0, (y + 14.0).min(ceiling).min(top - 1.0));
            if y < bottom - 0.1 || y_hi < y_lo + 1.0 {
                continue;
            }
            let mut rng = Rng::new(hash_coords(seed, lx * 2 + dx, SALT_CABLES + f as i32, lz));
            if !rng.chance(0.45) {
                continue;
            }
            for _ in 0..rng.range_i(1, 5) {
                let c = rng.range(lo + 0.5, hi - 0.5);
                let h0 = rng.range(y_lo, y_hi);
                let h1 = (h0 + rng.range(-1.5, 1.5)).clamp(y_lo, y_hi);
                let sag = gap * rng.range(0.04, 0.12);
                let mut wire = Vec::new();
                push_cable(&mut wire, ax == 0, (from, h0), (to, h1), c, sag);
                let span = wire.iter().fold(wire[0].aabb, |s, w| Aabb {
                    min: s.min.min(w.aabb.min),
                    max: s.max.max(w.aabb.max),
                });
                // Never through a skybridge between the same towers.
                if !bridges.iter().any(|b| b.overlaps(&span)) {
                    out.extend(wire);
                }
            }
        }
    }
    out
}

/// A wire from `(from, h0)` to `(to, h1)` along x (or z), at `c` across, sagging `sag` in its
/// middle: boxes 0.5 m long, each spanning the height its stretch falls, so they join.
fn push_cable(
    out: &mut Vec<Block>,
    along_x: bool,
    (from, h0): (f32, f32),
    (to, h1): (f32, f32),
    c: f32,
    sag: f32,
) {
    const T: f32 = 0.05;
    let n = ((to - from) / 0.5).ceil().max(1.0) as usize;
    let y = |t: f32| h0 + (h1 - h0) * t - sag * 4.0 * t * (1.0 - t);
    let mat = Mat::Paint((34, 34, 36));
    for i in 0..n {
        let (t0, t1) = (i as f32 / n as f32, (i + 1) as f32 / n as f32);
        let (u0, u1) = (from + (to - from) * t0, from + (to - from) * t1);
        let (ya, yb) = (y(t0), y(t1));
        let (y0, y1) = (ya.min(yb) - T * 0.5, ya.max(yb) + T * 0.5);
        let aabb = if along_x {
            Aabb {
                min: vec3(u0, y0, c - T * 0.5),
                max: vec3(u1, y1, c + T * 0.5),
            }
        } else {
            Aabb {
                min: vec3(c - T * 0.5, y0, u0),
                max: vec3(c + T * 0.5, y1, u1),
            }
        };
        out.push(Block {
            aabb,
            color: mat.color(),
            kind: BlockKind::Hung(mat),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debris_stays_off_the_roads_and_clear_of_boxes() {
        let seed = 7;
        let mut placed = 0;
        for lz in -3..3 {
            for lx in -3..3 {
                let mut blocks = Vec::new();
                let dressed = city::generate_lot_dressed(seed, lx, lz, &mut blocks);
                let solid: Vec<Aabb> = blocks
                    .iter()
                    .filter(|b| {
                        !matches!(
                            b.kind,
                            BlockKind::Floor
                                | BlockKind::Deck
                                | BlockKind::Hung(_)
                                | BlockKind::Prop
                        )
                    })
                    .map(|b| b.aabb)
                    .collect();
                for p in dressed.props.iter().filter(|p| {
                    matches!(
                        p.prop,
                        Prop::DebrisLumps
                            | Prop::DebrisBricks
                            | Prop::DebrisPaper
                            | Prop::DebrisFrame
                    )
                }) {
                    placed += 1;
                    let a = p.aabb();
                    let f = floors::floor_at(a.min.y);
                    assert!(
                        !in_road(seed, (lx, lz), f, a.min),
                        "debris in a road: {a:?}"
                    );
                    assert!(
                        !solid.iter().any(|s| s.overlaps(&a)),
                        "debris in a box: {a:?}"
                    );
                }
            }
        }
        assert!(placed > 10, "only {placed} pieces of debris");
    }

    #[test]
    fn cables_hang_between_towers_clear_of_bridges() {
        let seed = 7;
        let mut wires = 0;
        for lz in -4..4 {
            for lx in -4..4 {
                let cables = cables(seed, lx, lz);
                wires += cables.len();
                let bridges = city::skybridges(seed, lx, lz);
                for c in &cables {
                    assert!(
                        !bridges.iter().any(|b| b.aabb.overlaps(&c.aabb)),
                        "a cable through a bridge"
                    );
                }
            }
        }
        assert!(wires > 100, "only {wires} cable segments");
    }
}
