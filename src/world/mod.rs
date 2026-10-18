//! World geometry and generation: the city skeleton plus a spatial index for collision.

pub mod biome;
pub mod chunks;
pub mod city;
pub mod design;
pub mod floors;
pub mod growth;
pub mod leftovers;
pub mod planner;
pub mod props;
pub mod roads;
pub mod tower;

use std::collections::HashMap;

use glam::Vec3;
use sola_raylib::prelude::Color;

#[derive(Clone, Copy, Debug)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn from_center(center: Vec3, size: Vec3) -> Self {
        Self {
            min: center - size * 0.5,
            max: center + size * 0.5,
        }
    }

    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }

    pub fn overlaps(&self, o: &Aabb) -> bool {
        self.min.x < o.max.x
            && self.max.x > o.min.x
            && self.min.y < o.max.y
            && self.max.y > o.min.y
            && self.min.z < o.max.z
            && self.max.z > o.min.z
    }

    pub fn contains_xz(&self, p: Vec3) -> bool {
        p.x > self.min.x && p.x < self.max.x && p.z > self.min.z && p.z < self.max.z
    }
}

/// Faces of a unit box: outward normal and its four corners, counter-clockwise seen from outside,
/// as corner selectors (0 = min, 1 = max per axis).
pub const BOX_FACES: [(Vec3, [[f32; 3]; 4]); 6] = [
    (
        Vec3::X,
        [[1., 0., 1.], [1., 0., 0.], [1., 1., 0.], [1., 1., 1.]],
    ),
    (
        Vec3::NEG_X,
        [[0., 0., 0.], [0., 0., 1.], [0., 1., 1.], [0., 1., 0.]],
    ),
    (
        Vec3::Y,
        [[0., 1., 1.], [1., 1., 1.], [1., 1., 0.], [0., 1., 0.]],
    ),
    (
        Vec3::NEG_Y,
        [[0., 0., 0.], [1., 0., 0.], [1., 0., 1.], [0., 0., 1.]],
    ),
    (
        Vec3::Z,
        [[0., 0., 1.], [1., 0., 1.], [1., 1., 1.], [0., 1., 1.]],
    ),
    (
        Vec3::NEG_Z,
        [[1., 0., 0.], [0., 0., 0.], [0., 1., 0.], [1., 1., 0.]],
    ),
];

/// What a block is, which decides how its surfaces are painted.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BlockKind {
    /// The ground at the bottom of the world.
    Floor,
    /// Deck slabs: the streets and plazas of the floors above the ground.
    Deck,
    /// A tower body, its walls painted from its chunk's elevations.
    Tower(tower::Faces),
    /// Stair steps and landings.
    Stair,
    /// Hand-placed movement test blocks near spawn.
    Course,
    /// The wall between a stair's two lanes.
    Parapet,
    /// Steel: stair rails.
    Steel,
    /// The collision box of a prop (`props.rs`); not drawn, the prop's own mesh is.
    Prop,
    /// A box of a hand-written chunk (`chunks.rs`), in its material.
    Built(chunks::Mat),
    /// What hangs in the air: under a deck (`city::push_underside`: girders, joists, pipes, dead
    /// lamps) and across the streets (`leftovers::cables`). Drawn like `Built`; nothing grows on it.
    Hung(chunks::Mat),
    /// Collision only: the part of a big chunk box that reaches into this lot from the lot that
    /// owns and draws it.
    Collider,
}

/// A solid, drawable box.
#[derive(Clone, Copy)]
pub struct Block {
    pub aabb: Aabb,
    pub color: Color,
    pub kind: BlockKind,
}

impl Block {
    /// A box of a hand-written chunk, in its material.
    pub fn built(aabb: Aabb, mat: chunks::Mat) -> Self {
        Self {
            aabb,
            color: mat.color(),
            kind: BlockKind::Built(mat),
        }
    }
}

/// Lots kept loaded around the player (Chebyshev distance, in lots) and the distance at which
/// they are dropped; the gap avoids thrashing at the boundary.
pub const LOAD_RADIUS: i32 = 7;
const UNLOAD_RADIUS: i32 = 9;

type LotKey = (i32, i32);

/// A vertex of decorative plant geometry (no collision).
pub struct PlantVert {
    pub pos: Vec3,
    pub normal: Vec3,
    pub color: Color,
    /// Outline id, 1..=255.
    pub id: u8,
    /// Emission strength, 0..=255; scaled by the time of day in the shader.
    pub glow: u8,
    /// Leaf coordinates (across 0..1 with the midrib at 0.5, along 0..1 from root to tip), each
    /// plus twice the leaf's size in pixels that way, or `NO_UV` where the surface has none. The
    /// shader draws the leaf in pixel art from them (`pixelLeaf`).
    pub uv: [f32; 2],
}

pub const NO_UV: [f32; 2] = [-1.0, -1.0];

/// A point light from a luminous plant.
#[derive(Clone, Copy)]
pub struct Light {
    pub pos: Vec3,
    pub radius: f32,
    /// Linear color with intensity baked in.
    pub color: Vec3,
    /// Vertical extent the light reaches: the floor under it and the ceiling over it, so it does
    /// not leak through decks into the levels above and below.
    pub y_lo: f32,
    pub y_hi: f32,
}

/// Everything generated for one lot.
pub struct Lot {
    pub blocks: Vec<Block>,
    plants: Vec<PlantVert>,
    /// Props standing in the lot (from its chunks); the renderer draws them instanced.
    pub props: Vec<props::Placed>,
    lights: Vec<Light>,
    /// Material variant (palette index) of the lot's biome.
    pub variant: u8,
    /// The boxes of its blocks reaching into the lots around (`overhangs`).
    overhangs: Vec<Aabb>,
}

impl Lot {
    pub fn plants(&self) -> impl Iterator<Item = &PlantVert> {
        self.plants.iter()
    }

    /// Its flowers' lights, each with its index: a soul's identity.
    pub fn lights(&self) -> impl Iterator<Item = (usize, &Light)> {
        self.lights.iter().enumerate()
    }
}

/// The boxes of a lot's blocks that reach into the lots around it (a skybridge, a crane's jib),
/// for the collisions there. Collider copies are left out: the lot they come from has the box.
fn overhangs((lx, lz): LotKey, blocks: &[Block]) -> Vec<Aabb> {
    const E: f32 = 0.01;
    let (lo, hi) = (city::lot_origin(lx, lz), city::lot_origin(lx + 1, lz + 1));
    blocks
        .iter()
        .filter(|b| b.kind != BlockKind::Collider)
        .map(|b| b.aabb)
        .filter(|a| {
            a.min.x < lo.x - E || a.min.z < lo.z - E || a.max.x > hi.x + E || a.max.z > hi.z + E
        })
        .collect()
}

pub struct World {
    pub seed: u64,
    lots: HashMap<LotKey, Lot>,
}

impl World {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            lots: HashMap::new(),
        }
    }

    /// Loads lots around `center` (nearest first, at most `budget` per call) and drops far ones.
    pub fn stream(&mut self, center: Vec3, budget: usize) {
        let (cx, cz) = city::lot_of(center);
        self.lots
            .retain(|&(x, z), _| (x - cx).abs().max((z - cz).abs()) <= UNLOAD_RADIUS);

        let mut missing = Vec::new();
        for x in cx - LOAD_RADIUS..=cx + LOAD_RADIUS {
            for z in cz - LOAD_RADIUS..=cz + LOAD_RADIUS {
                if !self.lots.contains_key(&(x, z)) {
                    missing.push((x, z));
                }
            }
        }
        missing.sort_by_key(|&(x, z)| (x - cx).pow(2) + (z - cz).pow(2));
        for key in missing.into_iter().take(budget) {
            self.load(key);
        }
    }

    /// Generates a lot and grows its plants.
    fn load(&mut self, (x, z): LotKey) {
        let mut blocks = Vec::new();
        let dressed = city::generate_lot_dressed(self.seed, x, z, &mut blocks);
        let growth = growth::grow_lot(self.seed, x, z, &blocks, &dressed.blooms);
        let lot = Lot {
            overhangs: overhangs((x, z), &blocks),
            blocks,
            plants: growth.plants,
            props: dressed.props,
            lights: growth.lights,
            variant: biome::variant_of(biome::biome_at(self.seed, x, z)) as u8,
        };
        self.lots.insert((x, z), lot);
    }

    pub fn lots(&self) -> impl Iterator<Item = (LotKey, &Lot)> {
        self.lots.iter().map(|(&k, v)| (k, v))
    }

    /// Colliders that may overlap `b`: those of the lots it touches, and those reaching into it
    /// from the lots around (no block reaches further than the next lot).
    pub fn colliders_near<'a>(&'a self, b: &Aabb) -> impl Iterator<Item = &'a Aabb> + 'a {
        let ((x0, z0), (x1, z1)) = (city::lot_of(b.min), city::lot_of(b.max));
        (x0 - 1..=x1 + 1)
            .flat_map(move |x| (z0 - 1..=z1 + 1).map(move |z| (x, z)))
            .filter_map(|k| Some((k, self.lots.get(&k)?)))
            .flat_map(move |((x, z), l)| {
                let touched = (x0..=x1).contains(&x) && (z0..=z1).contains(&z);
                let (blocks, reaching): (&[Block], &[Aabb]) = if touched {
                    (&l.blocks, &[])
                } else {
                    (&[], &l.overhangs)
                };
                blocks.iter().map(|blk| &blk.aabb).chain(reaching)
            })
    }

    pub fn collides(&self, b: &Aabb) -> bool {
        self.colliders_near(b).any(|c| c.overlaps(b))
    }
}

#[cfg(test)]
impl World {
    /// A world of these lots' blocks alone (no plants, no props drawn), for collision tests.
    pub fn of_blocks(seed: u64, lots: impl IntoIterator<Item = (LotKey, Vec<Block>)>) -> World {
        let lots = lots
            .into_iter()
            .map(|(k, blocks)| {
                let lot = Lot {
                    overhangs: overhangs(k, &blocks),
                    blocks,
                    plants: Vec::new(),
                    props: Vec::new(),
                    lights: Vec::new(),
                    variant: 0,
                };
                (k, lot)
            })
            .collect();
        World { seed, lots }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_collide_in_every_lot_they_reach_into() {
        let seed = 1;
        let world = World::of_blocks(
            seed,
            (-5..5)
                .flat_map(|z| (-5..5).map(move |x| (x, z)))
                .map(|(x, z)| {
                    let mut blocks = Vec::new();
                    city::generate_lot(seed, x, z, &mut blocks);
                    ((x, z), blocks)
                }),
        );
        let mut reaching = 0;
        for (&(lx, lz), lot) in &world.lots {
            for b in &lot.overhangs {
                // Its part in each other lot it reaches into: something the player can stand on.
                let ((x0, z0), (x1, z1)) = (city::lot_of(b.min + 0.01), city::lot_of(b.max - 0.01));
                for (x, z) in (x0..=x1).flat_map(|x| (z0..=z1).map(move |z| (x, z))) {
                    if (x, z) == (lx, lz) || !world.lots.contains_key(&(x, z)) {
                        continue;
                    }
                    reaching += 1;
                    let (lo, hi) = (city::lot_origin(x, z), city::lot_origin(x + 1, z + 1));
                    let part = Aabb {
                        min: b.min.max(Vec3::new(lo.x, b.min.y, lo.z)),
                        max: b.max.min(Vec3::new(hi.x, b.max.y, hi.z)),
                    };
                    assert!(
                        world
                            .colliders_near(&part)
                            .any(|c| c.min == b.min && c.max == b.max),
                        "a box of lot ({lx}, {lz}) does not collide in ({x}, {z}): {b:?}"
                    );
                }
            }
        }
        assert!(
            reaching > 100,
            "only {reaching} boxes reach into another lot"
        );
    }
}
