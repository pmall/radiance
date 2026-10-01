//! World geometry and generation. For milestone 1 this is a seeded test scene;
//! it becomes the chunked city generator later.

pub mod test_scene;

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

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
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

/// A solid, drawable box.
pub struct Block {
    pub aabb: Aabb,
    pub color: Color,
}

pub struct World {
    pub seed: u64,
    pub blocks: Vec<Block>,
}

impl World {
    pub fn generate(seed: u64) -> Self {
        Self {
            seed,
            blocks: test_scene::generate(seed),
        }
    }

    pub fn colliders(&self) -> impl Iterator<Item = &Aabb> {
        self.blocks.iter().map(|b| &b.aabb)
    }

    pub fn collides(&self, b: &Aabb) -> bool {
        self.colliders().any(|c| c.overlaps(b))
    }
}
