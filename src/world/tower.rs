//! Towers (`docs/chunks.md`, "Towers"): hand-written tower chunks (`role: Tower`), placed by the
//! planner on one lot or a few with a height it picks (`planner::tower_at`), built here. A tower
//! is a stack of tiers rising from the bottom of the stack: solid bodies whose walls the scene
//! shader paints from the chunk's written elevations (`chunks::Facade`, `facade` in
//! `shaders/scene.fs`), or bare frames of stories. Where a deck meets the tower, the street story
//! and the two above it get their units in boxes (shop awnings, cages, balconies, laundry), from
//! the same elevations; the tiers' parts add what hangs on the walls and stands on the roofs.

use glam::{Vec3, vec3};
use sola_raylib::prelude::Color;

use super::chunks::{self, Catalog, Part, STORY, Tf, TowerDef};
use super::floors::{self, START};
use super::planner::Placement;
use super::props::Placed;
use super::{Aabb, Block, BlockKind};
use crate::rng::{Rng, hash_coords, mix64};

/// Stories over a deck whose units stand in boxes; the shader paints the others.
pub const NEAR_STORIES: usize = 2;

/// The facade paints (sRGB) a written elevation's digits 1 to 7 pick from, shifted per tower
/// (`Faces::paint`). Mirrored as `FACADE_PAINT` in `shaders/scene.fs`.
pub const PAINT: [(u8, u8, u8); 8] = [
    (150, 60, 48),
    (96, 130, 70),
    (200, 186, 150),
    (60, 120, 118),
    (190, 140, 60),
    (180, 110, 110),
    (80, 100, 130),
    (170, 90, 50),
];

/// What the renderer paints a tower body's walls with: the elevation of each face (toward +x,
/// -x, +z, -z; ids into `chunks::Catalogs::elevations`) and the tower's shift of the paints.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Faces {
    pub ids: [u16; 4],
    pub paint: u8,
}

/// The paint of a written digit on a tower (`d` 1 to 7).
pub fn paint(d: u8, shift: u8) -> (u8, u8, u8) {
    PAINT[((d + shift) % 8) as usize]
}

/// Lowest a setback may start: well above the start floor, so a tier never ends in the depths.
const SETBACK_FLOOR: f32 = 8.0;
/// Stories a tower keeps under its setbacks, at least.
const MIN_BASE_STORIES: f32 = 12.0;

const SALT_TIERS: u64 = 0x7153;

/// Local outward normals of the faces, in `Faces::ids` order, and the quarter turn that points a
/// module's +z (out of its wall) along each.
const FACES: [(Vec3, u8); 4] = [
    (Vec3::X, 1),
    (Vec3::NEG_X, 3),
    (Vec3::Z, 0),
    (Vec3::NEG_Z, 2),
];

fn face_index(n: Vec3) -> usize {
    FACES.iter().position(|(f, _)| f.dot(n) > 0.5).unwrap_or(0)
}

/// The tiers a placed tower has: (tier index, base height, top height), bottom to top. The top
/// tier ends at the placement's height; the upper tiers each take the top stories the seed picks
/// for them, when the tower is tall enough.
pub fn tiers(def: &TowerDef, p: &Placement) -> Vec<(usize, f32, f32)> {
    let top = floors::bottom() + p.stories as f32 * STORY;
    let mut rng = Rng::new(p.seed ^ SALT_TIERS);
    let mut out = Vec::new();
    let mut hi = top;
    for (i, t) in def.tiers.iter().enumerate().skip(1).rev() {
        let (a, b) = t.stories.unwrap_or((4, 8));
        let n = rng.range_i(a.min(b) as i32, a.max(b) as i32 + 1) as f32;
        let keep = rng.chance(t.chance);
        let base = hi - n * STORY;
        if keep
            && base >= floors::y(START) + SETBACK_FLOOR
            && base - floors::bottom() >= MIN_BASE_STORIES * STORY
        {
            out.push((i, base, hi));
            hi = base;
        }
    }
    out.push((0, floors::bottom(), hi));
    out.reverse();
    out
}

/// The transform of a placed tower chunk (heights stay absolute).
fn transform(p: &Placement) -> Tf {
    Tf::placement(
        unturned(p),
        vec3(
            p.origin.0 as f32 * super::city::LOT,
            0.0,
            p.origin.1 as f32 * super::city::LOT,
        ),
        p.turn,
    )
}

/// The chunk's own size (the placement's is turned).
fn unturned(p: &Placement) -> (i32, i32) {
    if p.turn % 2 == 1 {
        (p.size.1, p.size.0)
    } else {
        p.size
    }
}

/// The base tier's footprint, from the bottom of the stack to the tower's top: what other rules
/// keep clear of (stairs, holes, fillers) and what bridges land on.
pub fn body(cat: &Catalog, p: &Placement) -> Option<Aabb> {
    let def = cat.chunks.get(&p.chunk)?.tower.as_ref()?;
    let t = def.tiers.first()?;
    let top = floors::bottom() + p.stories as f32 * STORY;
    Some(transform(p).aabb(
        vec3(t.at.0, floors::bottom(), t.at.1),
        vec3(t.at.0 + t.size.0, top, t.at.1 + t.size.1),
    ))
}

/// A slightly drifting gray, so neighboring towers do not read as one mass.
fn tint(rng: &mut Rng, lo: i32, hi: i32) -> Color {
    let s = rng.range_i(lo, hi);
    let warm = rng.range_i(-8, 9);
    let c = |v: i32| v.clamp(0, 255) as u8;
    Color::new(c(s + warm), c(s + 6), c(s + 14 - warm), 255)
}

/// What a tower is made of, in world coordinates: its bodies and its boxes, each box with its
/// group (a module and its boxes stand or give way together) and, for the street parts, the
/// floor whose deck it needs under it.
#[derive(Default)]
pub struct Built {
    pub blocks: Vec<Block>,
    pub groups: Vec<u32>,
    pub needs: Vec<Option<usize>>,
    /// Props, drawn instanced, with their groups and needs.
    pub props: Vec<Placed>,
    pub prop_groups: Vec<u32>,
    pub prop_needs: Vec<Option<usize>>,
    /// Flowers of points of interest, with their groups.
    pub blooms: Vec<Vec3>,
    pub bloom_groups: Vec<u32>,
    next: u32,
}

impl Built {
    fn add(&mut self, b: chunks::Built, needs: Option<usize>) {
        let base = self.next;
        for ((aabb, mat), g) in b.boxes.into_iter().zip(b.groups) {
            self.blocks.push(Block {
                aabb,
                color: mat.color(),
                kind: BlockKind::Built(mat),
            });
            self.groups.push(base + g);
            self.next = self.next.max(base + g + 1);
            self.needs.push(needs);
        }
        for (p, g) in b.props.into_iter().zip(b.prop_groups) {
            self.props.push(p);
            self.prop_groups.push(base + g);
            self.next = self.next.max(base + g + 1);
            self.prop_needs.push(needs);
        }
        for (p, g) in b.blooms.into_iter().zip(b.bloom_groups) {
            self.blooms.push(p);
            self.bloom_groups.push(base + g);
            self.next = self.next.max(base + g + 1);
        }
    }

    fn add_block(&mut self, b: Block) {
        self.blocks.push(b);
        self.groups.push(self.next);
        self.next += 1;
        self.needs.push(None);
    }
}

/// Builds a placed tower. `deck(lot, floor)` says whether a lot has a deck on a floor, for the
/// units and street parts that need one.
pub fn build(cat: &Catalog, p: &Placement, deck: &dyn Fn((i32, i32), usize) -> bool) -> Built {
    let mut out = Built::default();
    let Some(def) = cat.chunks.get(&p.chunk).and_then(|c| c.tower.as_ref()) else {
        return out;
    };
    let tf = transform(p);
    let mut rng = Rng::new(p.seed);
    let shift = rng.range_i(0, 8) as u8;
    let seed_of = |a: i32, b: i32, c: i32| mix64(hash_coords(p.seed, a, b, c));
    let lot_of = |v: Vec3| {
        (
            (v.x / super::city::LOT).floor() as i32,
            (v.z / super::city::LOT).floor() as i32,
        )
    };
    // Parts on a floor know it (their rates, which lots have a deck).
    let expand = |parts: &[Part], y: f32, seed: u64| {
        let floor = floors::floor_at(y);
        let has_deck = |lot: (i32, i32)| deck(lot, floor);
        let env = chunks::Env {
            floor,
            deck: &has_deck,
        };
        let tf = tf.then(&Tf::shift(Vec3::Y * y));
        chunks::expand_in(cat, parts, &tf, seed, Some(&env))
    };

    for (k, (i, base, top)) in tiers(def, p).into_iter().enumerate() {
        let t = &def.tiers[i];
        let k = k as i32;
        let (min, max) = (
            vec3(t.at.0, base, t.at.1),
            vec3(t.at.0 + t.size.0, top, t.at.1 + t.size.1),
        );
        if let Some(f) = &t.facade {
            let mut faces = Faces {
                ids: [0; 4],
                paint: shift,
            };
            for (fi, (n, _)) in FACES.iter().enumerate() {
                faces.ids[face_index(tf.dir(*n))] = f.ids[fi];
            }
            out.add_block(Block {
                aabb: tf.aabb(min, max),
                color: tint(&mut rng, 92, 190),
                kind: BlockKind::Tower(faces),
            });
            push_units(cat, p, &tf, t, f, (base, top), shift, deck, &mut out);
        }
        let stories = ((top - base) / STORY).round() as i32;
        if !t.each_story.is_empty() {
            for s in 0..stories {
                let b = expand(&t.each_story, base + s as f32 * STORY, seed_of(k, 1, s));
                out.add(b, None);
            }
        }
        for f in 0..floors::count() {
            let y = floors::y(f);
            if y < base - 0.01 || y + STORY > top + 0.01 {
                continue;
            }
            if !t.each_floor.is_empty() {
                out.add(expand(&t.each_floor, y, seed_of(k, 2, f as i32)), None);
            }
            if !t.streets.is_empty() {
                out.add(expand(&t.streets, y, seed_of(k, 3, f as i32)), Some(f));
            }
        }
        if !t.roof.is_empty() {
            out.add(expand(&t.roof, top, seed_of(k, 4, 0)), None);
        }
    }
    // Street parts only where their floor has a deck under them, and nothing inside a body (a
    // lower tier's roof under the setback): a group stands or falls as one.
    let bodies: Vec<Aabb> = out
        .blocks
        .iter()
        .filter(|b| matches!(b.kind, BlockKind::Tower(_)))
        .map(|b| b.aabb)
        .collect();
    let mut dropped: Vec<u32> = Vec::new();
    let boxes = out.blocks.iter().map(|b| b.aabb).zip(&out.needs);
    let props = out.props.iter().map(|p| p.aabb()).zip(&out.prop_needs);
    let groups = out.groups.iter().chain(&out.prop_groups);
    for ((aabb, need), &g) in boxes.chain(props).zip(groups) {
        let c = (aabb.min + aabb.max) * 0.5;
        let floating = need.is_some_and(|f| !deck(lot_of(c), f));
        let inside = bodies
            .iter()
            .any(|b| b.overlaps(&aabb) && (b.min != aabb.min || b.max != aabb.max));
        if floating || inside {
            dropped.push(g);
        }
    }
    out.retain(|g| !dropped.contains(&g));
    out
}

impl Built {
    /// Keeps the boxes and props of the groups `keep` says.
    fn retain(&mut self, keep: impl Fn(u32) -> bool) {
        let mut i = 0;
        let groups = &self.groups;
        self.blocks.retain(|_| {
            i += 1;
            keep(groups[i - 1])
        });
        let mut i = 0;
        self.needs.retain(|_| {
            i += 1;
            keep(groups[i - 1])
        });
        self.groups.retain(|&g| keep(g));
        let mut i = 0;
        let groups = &self.prop_groups;
        self.props.retain(|_| {
            i += 1;
            keep(groups[i - 1])
        });
        let mut i = 0;
        self.prop_needs.retain(|_| {
            i += 1;
            keep(groups[i - 1])
        });
        self.prop_groups.retain(|&g| keep(g));
        let mut i = 0;
        let groups = &self.bloom_groups;
        self.blooms.retain(|_| {
            i += 1;
            keep(groups[i - 1])
        });
        self.bloom_groups.retain(|&g| keep(g));
    }
}

/// The units in boxes of a tier's faces, on the street story and the `NEAR_STORIES` above each
/// floor whose deck meets the face, from its elevations.
#[allow(clippy::too_many_arguments)]
fn push_units(
    cat: &Catalog,
    p: &Placement,
    tf: &Tf,
    t: &chunks::Tier,
    f: &chunks::Facade,
    (base, top): (f32, f32),
    shift: u8,
    deck: &dyn Fn((i32, i32), usize) -> bool,
    out: &mut Built,
) {
    let all = chunks::catalogs();
    let (x0, z0) = t.at;
    let (w, d) = t.size;
    for (fi, &(n, q)) in FACES.iter().enumerate() {
        let Some(e) = all.elevations.get(f.ids[fi] as usize) else {
            continue;
        };
        let width = if fi < 2 { d } else { w };
        // The face's middle at the foot, and the viewer's right along it.
        let center = vec3(x0 + w * 0.5, 0.0, z0 + d * 0.5) + n * vec3(w * 0.5, 0.0, d * 0.5);
        let right = Tf::turn(q).dir(Vec3::X);
        let u0 = (width - e.cols as f32 * STORY) * 0.5;
        for c in 0..e.cols {
            let along = -width * 0.5 + u0 + (c as f32 + 0.5) * STORY;
            let bay = center + right * along;
            let front = tf.point(bay + n * 1.5);
            let lot = (
                (front.x / super::city::LOT).floor() as i32,
                (front.z / super::city::LOT).floor() as i32,
            );
            for fl in 0..floors::count() {
                let y = floors::y(fl);
                let near = STORY * (NEAR_STORIES + 1) as f32;
                if y < base - 0.01 || y + near > top + 0.01 || !deck(lot, fl) {
                    continue;
                }
                for row in 0..=NEAR_STORIES {
                    let ((_, _, digit), module) = e.at(row, c);
                    let Some(name) = module else {
                        continue;
                    };
                    // The street story's units stand on its floor, the others on the window sill.
                    let h = if row == 0 {
                        y
                    } else {
                        y + row as f32 * STORY + 1.0
                    };
                    let seed = mix64(hash_coords(
                        p.seed,
                        fi as i32 * 64 + c as i32,
                        fl as i32,
                        row as i32,
                    ));
                    let accent = if digit > 0 {
                        paint(digit, shift)
                    } else {
                        PAINT[(seed % 8) as usize]
                    };
                    let part = Part::Module {
                        name: name.to_string(),
                        at: (bay.x, h, bay.z),
                        turn: q,
                        accent: Some(accent),
                    };
                    out.add(chunks::expand(cat, &[part], tf, seed), None);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faces_follow_the_turns() {
        // A module's +x is the viewer's right on every face: turning the frame keeps handedness.
        for (n, q) in FACES {
            let right = Tf::turn(q).dir(Vec3::X);
            let out = Tf::turn(q).dir(Vec3::Z);
            assert!((out - n).length() < 1e-5, "{n}: {out}");
            // Seen from outside (looking along -n), right = -n x up.
            let expect = (-n).cross(Vec3::Y);
            assert!((right - expect).length() < 1e-5, "{n}: {right} vs {expect}");
        }
    }
}
