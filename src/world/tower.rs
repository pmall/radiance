//! Towers (`docs/chunks.md`, "Towers"): hand-written tower chunks (`role: Tower`), placed by the
//! planner on one lot or a few with a height it picks (`planner::tower_at`), built here. A tower
//! is a stack of tiers rising from the bottom of the stack: solid bodies whose walls the scene
//! shader paints from the chunk's written elevations (`chunks::Facade`, `facade` in
//! `shaders/scene.fs`), or bare frames of stories. Where a deck meets the tower, the street story
//! and the two above it get their units in boxes (shop awnings, cages, balconies, laundry), from
//! the same elevations; the tiers' parts add what hangs on the walls and stands on the roofs.

use glam::{Vec3, vec3};

use super::biome::{self, Biome};
use super::chunks::{self, Catalog, Part, STORY, Tf, TowerDef};
use super::floors::{self, START};
use super::planner::Placement;
use super::props::Placed;
use super::{Aabb, Block, BlockKind};
use crate::rng::{Rng, hash_coords, mix64};

/// Stories over a deck whose units stand in boxes; the shader paints the others.
pub const NEAR_STORIES: usize = 2;

/// What the renderer paints a tower body's walls with: the elevation of each face (toward +x,
/// -x, +z, -z; ids into `chunks::Catalogs::elevations`) and the tower's shift of the paints.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Faces {
    pub ids: [u16; 4],
    pub paint: u8,
    /// The tower rises past the top deck (`over_top_deck`): the far skyline draws it too, so the
    /// fog at the loaded lots' edge leaves it be (`shaders/post.fs`).
    pub far: bool,
}

/// Whether a placed tower rises past the top deck, into the far skyline (`skyline.rs`).
pub fn over_top_deck(p: &Placement) -> bool {
    floors::bottom() + p.stories as f32 * STORY > floors::top() + 1.0
}

/// The paint of a written digit on a tower (`d` 1 to 7) in a biome (`Palette::paints`, which the
/// shader tints the walls with too).
pub fn paint(biome: &Biome, d: u8, shift: u8) -> (u8, u8, u8) {
    let [r, g, b] = biome.palette.paints[((d + shift) % 8) as usize];
    (r, g, b)
}

/// Lowest a setback may start: well above the start floor, so a tier never ends in the depths.
const SETBACK_FLOOR: f32 = 8.0;
/// Stories a tower keeps under its setbacks, at least.
const MIN_BASE_STORIES: f32 = 12.0;

const SALT_TIERS: u64 = 0x7153;
const SALT_LEAN: u64 = 0x1ea7;

/// Stories in each step of a leaning tier: it leans in steps, like the cables sag in pixels.
const LEAN_STEP: usize = 2;
/// Lowest a leaning tier starts over the top deck, clear of the cables and what stands on it.
const LEAN_ABOVE: f32 = 16.0;
/// Widest gap a leaning tier crosses to rest on its neighbor (the facing walls of two towers on
/// next lots are usually 8 m apart).
const LEAN_MAX: f32 = 16.0;

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
        let mut n = rng.range_i(a.min(b) as i32, a.max(b) as i32 + 1) as f32;
        let keep = rng.chance(t.chance);
        // A leaning tier stays above the top deck, two steps at least.
        if t.lean {
            n = n.min(((hi - floors::top() - LEAN_ABOVE) / STORY).floor());
        }
        let base = hi - n * STORY;
        if keep
            && (!t.lean || n >= (2 * LEAN_STEP) as f32)
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
    // The chunk's own size: the placement's turned back.
    let size = super::planner::turned(p.size, p.turn);
    Tf::placement(
        size,
        super::city::lot_origin(p.origin.0, p.origin.1),
        p.turn,
    )
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

/// The top of the base tier's walls, under any setback or frame going up: the highest a bridge
/// may land on the tower (`None` when the base tier is a bare frame).
pub fn solid_top(cat: &Catalog, p: &Placement) -> Option<f32> {
    let def = cat.chunks.get(&p.chunk)?.tower.as_ref()?;
    def.tiers.first()?.facade.as_ref()?;
    tiers(def, p).first().map(|t| t.2)
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
            self.blocks.push(Block::built(aabb, mat));
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

/// Builds a placed tower in the city of `seed` (its decks and roads, for the units and street
/// parts that need them).
pub fn build(cat: &Catalog, p: &Placement, seed: u64) -> Built {
    let deck =
        |(x, z): (i32, i32), f: usize| super::city::deck(seed, x, z, f) != super::city::Deck::None;
    let mut out = Built::default();
    let Some(def) = cat.chunks.get(&p.chunk).and_then(|c| c.tower.as_ref()) else {
        return out;
    };
    let tf = transform(p);
    let mut rng = Rng::new(p.seed);
    let shift_paint = rng.range_i(0, 8) as u8;
    let seed_of = |a: i32, b: i32, c: i32| mix64(hash_coords(p.seed, a, b, c));
    // Parts on a floor know it (their rates, which lots have a deck).
    let expand = |parts: &[Part], y: f32, slots: u64| {
        let env = chunks::Env {
            seed,
            floor: floors::floor_at(y),
        };
        let tf = tf.then(&Tf::shift(Vec3::Y * y));
        chunks::expand_in(cat, parts, &tf, slots, Some(&env))
    };

    for (k, (i, base, top)) in tiers(def, p).into_iter().enumerate() {
        let t = &def.tiers[i];
        let k = k as i32;
        let (min, max) = (
            vec3(t.at.0, base, t.at.1),
            vec3(t.at.0 + t.size.0, top, t.at.1 + t.size.1),
        );
        let stories = ((top - base) / STORY).round() as i32;
        // A leaning tier goes up in steps of `LEAN_STEP` stories, each shifted a little further
        // (world coordinates), the last by the whole lean.
        let lean = t.lean.then(|| lean(seed, p, &tf.aabb(min, max))).flatten();
        let steps = if lean.is_some() {
            (stories as usize / LEAN_STEP).max(1)
        } else {
            1
        };
        let step_of = |s: i32| (s as usize / LEAN_STEP).min(steps - 1);
        let shift = |step: usize| lean.unwrap_or(Vec3::ZERO) * ((step + 1) as f32 / steps as f32);
        if let Some(f) = &t.facade {
            let mut faces = Faces {
                ids: [0; 4],
                paint: shift_paint,
                far: over_top_deck(p),
            };
            for (fi, (n, _)) in FACES.iter().enumerate() {
                faces.ids[face_index(tf.dir(*n))] = f.ids[fi];
            }
            let color = super::city::tint(&mut rng, 92, 190);
            for step in 0..steps {
                let lo = base + (step * LEAN_STEP) as f32 * STORY;
                let hi = if step + 1 == steps {
                    top
                } else {
                    lo + LEAN_STEP as f32 * STORY
                };
                let body = tf.aabb(vec3(min.x, lo, min.z), vec3(max.x, hi, max.z));
                out.add_block(Block {
                    aabb: Aabb {
                        min: body.min + shift(step),
                        max: body.max + shift(step),
                    },
                    color,
                    kind: BlockKind::Tower(faces),
                });
            }
            if lean.is_none() {
                push_units(cat, p, &tf, t, f, (base, top), shift_paint, &deck, &mut out);
            }
        }
        // Parts in world coordinates, moved with the step they stand on.
        let moved = |mut b: chunks::Built, by: Vec3| {
            for (a, _) in &mut b.boxes {
                a.min += by;
                a.max += by;
            }
            for p in &mut b.props {
                p.pos += by;
            }
            for f in &mut b.blooms {
                *f += by;
            }
            b
        };
        if !t.each_story.is_empty() {
            for s in 0..stories {
                let b = expand(&t.each_story, base + s as f32 * STORY, seed_of(k, 1, s));
                out.add(moved(b, shift(step_of(s))), None);
            }
        }
        for f in 0..floors::count() {
            let y = floors::y(f);
            if y < base - 0.01 || y + STORY > top + 0.01 || lean.is_some() {
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
            let b = expand(&t.roof, top, seed_of(k, 4, 0));
            out.add(moved(b, shift(steps - 1)), None);
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
        let floating = need.is_some_and(|f| !deck(super::city::lot_of(c), f));
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

/// Smallest extent of a part kept in a tower's far silhouette (a mast, a crane, a sign frame).
const SILHOUETTE_PART: f32 = 8.0;

/// What a placed tower shows from far away (`world::skyline`): its bodies (leaning ones
/// included) and its big parts (masts, cranes, sign frames).
pub fn silhouette(cat: &Catalog, p: &Placement, seed: u64) -> Vec<Block> {
    build(cat, p, seed)
        .blocks
        .into_iter()
        .filter(|b| match b.kind {
            BlockKind::Tower(_) => true,
            BlockKind::Built(_) => b.aabb.size().max_element() >= SILHOUETTE_PART,
            _ => false,
        })
        .collect()
}

impl Built {
    /// Keeps the boxes, props and flowers of the groups `keep` says.
    fn retain(&mut self, keep: impl Fn(u32) -> bool) {
        // The items of `v` whose group (same order) is kept.
        fn by_group<T>(v: &mut Vec<T>, groups: &[u32], keep: &impl Fn(u32) -> bool) {
            let mut g = groups.iter();
            v.retain(|_| g.next().is_some_and(|&g| keep(g)));
        }
        by_group(&mut self.blocks, &self.groups, &keep);
        by_group(&mut self.needs, &self.groups, &keep);
        self.groups.retain(|&g| keep(g));
        by_group(&mut self.props, &self.prop_groups, &keep);
        by_group(&mut self.prop_needs, &self.prop_groups, &keep);
        self.prop_groups.retain(|&g| keep(g));
        by_group(&mut self.blooms, &self.bloom_groups, &keep);
        self.bloom_groups.retain(|&g| keep(g));
    }
}

/// Whether a placed tower stands crooked: one of its tiers leans on a neighbor (`lean`).
pub fn leans(cat: &Catalog, p: &Placement, seed: u64) -> bool {
    let Some(def) = cat.chunks.get(&p.chunk).and_then(|c| c.tower.as_ref()) else {
        return false;
    };
    let tf = transform(p);
    tiers(def, p).into_iter().any(|(i, base, top)| {
        let t = &def.tiers[i];
        let (min, max) = (
            vec3(t.at.0, base, t.at.1),
            vec3(t.at.0 + t.size.0, top, t.at.1 + t.size.1),
        );
        t.lean && lean(seed, p, &tf.aabb(min, max)).is_some()
    })
}

/// Where a leaning tier's top goes (world coordinates), `body` its box standing straight: toward
/// the facing wall of a neighbor tower whose solid walls reach about as high, by the gap between
/// them, so its top rests on that wall; the seed picks among the neighbors that qualify. Never
/// where a skybridge would be in the way. Two towers never lean on each other: the one leaned on
/// has solid walls past its top, which a leaning tower never has.
pub fn lean(seed: u64, p: &Placement, body: &Aabb) -> Option<Vec3> {
    let (ox, oz) = p.origin;
    let (w, d) = p.size;
    let around = (ox..ox + w)
        .flat_map(|x| [((x, oz - 1), Vec3::NEG_Z), ((x, oz + d), Vec3::Z)])
        .chain((oz..oz + d).flat_map(|z| [((ox - 1, z), Vec3::NEG_X), ((ox + w, z), Vec3::X)]));
    let mut found: Vec<Vec3> = Vec::new();
    for ((x, z), dir) in around {
        let Some(t) = super::city::tower(seed, x, z) else {
            continue;
        };
        let n = Aabb::from_center(t.center, t.size);
        // The gap between the facing walls, and the stretch of wall they share.
        let gap = if dir.x != 0.0 {
            if dir.x > 0.0 {
                n.min.x - body.max.x
            } else {
                body.min.x - n.max.x
            }
        } else if dir.z > 0.0 {
            n.min.z - body.max.z
        } else {
            body.min.z - n.max.z
        };
        let shared = if dir.x != 0.0 {
            body.max.z.min(n.max.z) - body.min.z.max(n.min.z)
        } else {
            body.max.x.min(n.max.x) - body.min.x.max(n.min.x)
        };
        let tall = t.walls_top >= body.max.y - LEAN_STEP as f32 * STORY * 0.5;
        if (2.0..=LEAN_MAX).contains(&gap) && shared >= 8.0 && tall && !found.contains(&(dir * gap))
        {
            found.push(dir * gap);
        }
    }
    // Skybridges stay out of the room the tier leans through.
    found.retain(|&v| {
        let swept = Aabb {
            min: body.min.min(body.min + v),
            max: body.max.max(body.max + v),
        };
        let lot = super::city::LOT;
        let (x0, x1) = (
            (swept.min.x / lot).floor() as i32 - 1,
            (swept.max.x / lot).floor() as i32,
        );
        let (z0, z1) = (
            (swept.min.z / lot).floor() as i32 - 1,
            (swept.max.z / lot).floor() as i32,
        );
        !(z0..=z1).any(|z| {
            (x0..=x1).any(|x| {
                super::city::skybridges(seed, x, z)
                    .iter()
                    .any(|b| b.aabb.overlaps(&swept))
            })
        })
    });
    if found.is_empty() {
        return None;
    }
    let mut rng = Rng::new(p.seed ^ SALT_LEAN);
    Some(found[rng.range_i(0, found.len() as i32) as usize])
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
            let lot = super::city::lot_of(tf.point(bay + n * 1.5));
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
                    let biome = biome::named(p.biome);
                    let accent = if digit > 0 {
                        paint(biome, digit, shift)
                    } else {
                        let [r, g, b] = biome.palette.paints[(seed % 8) as usize];
                        (r, g, b)
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

    #[test]
    fn leaning_towers_rest_on_a_neighbor_and_cross_nothing() {
        let seed = 1;
        let inner = |a: &Aabb| Aabb {
            min: a.min + 0.05,
            max: a.max - 0.05,
        };
        let mut n = 0;
        for lz in -30..30 {
            for lx in -30..30 {
                let Some(p) = super::super::planner::tower_at(seed, lx, lz) else {
                    continue;
                };
                let cat = chunks::catalog(p.biome);
                if p.origin != (lx, lz) || !leans(&cat, &p, seed) {
                    continue;
                }
                n += 1;
                let own = super::super::city::tower(seed, lx, lz).unwrap();
                let own = Aabb::from_center(own.center, own.size);
                let bodies: Vec<Aabb> = build(&cat, &p, seed)
                    .blocks
                    .iter()
                    .filter(|b| matches!(b.kind, BlockKind::Tower(_)))
                    .map(|b| b.aabb)
                    // The leaning steps (skybridges sink their ends into the straight base).
                    .filter(|b| b.min.y >= floors::top() + LEAN_ABOVE - 0.01)
                    .collect();
                let mut touches = false;
                for z in lz - 2..=lz + 2 {
                    for x in lx - 2..=lx + 2 {
                        for b in super::super::city::skybridges(seed, x, z) {
                            assert!(
                                !bodies.iter().any(|t| inner(t).overlaps(&b.aabb)),
                                "the tower leaning at ({lx}, {lz}) runs into a skybridge"
                            );
                        }
                        let Some(t) = super::super::city::tower(seed, x, z) else {
                            continue;
                        };
                        let t = Aabb::from_center(t.center, t.size);
                        if t.min == own.min && t.max == own.max {
                            continue;
                        }
                        assert!(
                            !bodies.iter().any(|b| inner(b).overlaps(&t)),
                            "the tower leaning at ({lx}, {lz}) runs into the one at ({x}, {z})"
                        );
                        let near = Aabb {
                            min: t.min - 0.05,
                            max: t.max + 0.05,
                        };
                        touches |= bodies.iter().any(|b| b.overlaps(&near));
                    }
                }
                assert!(touches, "the tower at ({lx}, {lz}) leans on nothing");
            }
        }
        assert!(n > 2, "only {n} leaning towers");
    }
}
