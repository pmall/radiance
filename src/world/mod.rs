//! World geometry and generation: the city skeleton plus a spatial index for collision.

pub mod biome;
pub mod city;
pub mod growth;
pub mod roof;

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

/// What a block is, which decides how its surfaces are painted.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BlockKind {
    /// The street level at the bottom of the world.
    Floor,
    /// Deck slabs: streets and plazas on the upper and mid levels.
    Deck,
    Tower,
    /// Walkway deck and parapets between two towers.
    Bridge,
    /// Spiral stair steps.
    Stair,
    /// Hand-placed movement test blocks near spawn.
    Course,
    /// Low wall around a roof edge.
    Parapet,
    /// Rooftop machinery: air conditioners, vents, tanks, stairwell housings, billboard posts.
    Equipment,
    /// Billboard panel.
    Sign,
    /// Antenna mast.
    Mast,
    /// Rooftop solar panel array.
    Solar,
    /// Rooftop skylight.
    Skylight,
    /// Street lamp post (unlit: the city is dead, only flowers glow).
    Lamp,
}

/// A solid, drawable box.
pub struct Block {
    pub aabb: Aabb,
    pub color: Color,
    pub kind: BlockKind,
}

/// Lots kept loaded around the player (Chebyshev distance, in lots) and the distance at which
/// they are dropped; the gap avoids thrashing at the boundary.
const LOAD_RADIUS: i32 = 7;
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
    /// Leaf coordinates (across 0..1 with the midrib at 0.5, along 0..1 from root to tip), or
    /// `NO_UV` where the surface has none. Used by the shader to paint veins.
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
    pub plants: Vec<PlantVert>,
    pub lights: Vec<Light>,
}

pub struct World {
    pub seed: u64,
    lots: HashMap<LotKey, Lot>,
}

fn lot_of(v: f32) -> i32 {
    (v / city::LOT).floor() as i32
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
        let (cx, cz) = (lot_of(center.x), lot_of(center.z));
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
        for (x, z) in missing.into_iter().take(budget) {
            let mut blocks = Vec::new();
            city::generate_lot(self.seed, x, z, &mut blocks);
            let growth::Growth { plants, lights } = growth::grow_lot(self.seed, x, z, &blocks);
            self.lots.insert(
                (x, z),
                Lot {
                    blocks,
                    plants,
                    lights,
                },
            );
        }
    }

    pub fn lots(&self) -> impl Iterator<Item = (LotKey, &Lot)> {
        self.lots.iter().map(|(&k, v)| (k, v))
    }

    pub fn block_count(&self) -> usize {
        self.lots.values().map(|l| l.blocks.len()).sum()
    }

    /// Colliders in the lots touched by `b` (a superset of the ones overlapping it).
    pub fn colliders_near<'a>(&'a self, b: &Aabb) -> impl Iterator<Item = &'a Aabb> + 'a {
        let (x0, x1) = (lot_of(b.min.x), lot_of(b.max.x));
        let (z0, z1) = (lot_of(b.min.z), lot_of(b.max.z));
        (x0..=x1)
            .flat_map(move |x| (z0..=z1).map(move |z| (x, z)))
            .filter_map(|k| self.lots.get(&k))
            .flat_map(|l| l.blocks.iter())
            .map(|blk| &blk.aabb)
    }

    pub fn collides(&self, b: &Aabb) -> bool {
        self.colliders_near(b).any(|c| c.overlaps(b))
    }
}
