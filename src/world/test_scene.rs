//! Seeded test scene for tuning movement: a ground slab, towers of varied
//! height, stepped plinths to climb, and elevated walkways.

use glam::{Vec3, vec3};
use sola_raylib::prelude::Color;

use super::{Aabb, Block};
use crate::rng::Rng;

const HALF_EXTENT: f32 = 120.0;

pub fn generate(seed: u64) -> Vec<Block> {
    let mut rng = Rng::new(seed);
    let mut blocks = Vec::new();

    // Ground slab (top at y = 0).
    blocks.push(Block {
        aabb: Aabb {
            min: vec3(-HALF_EXTENT, -2.0, -HALF_EXTENT),
            max: vec3(HALF_EXTENT, 0.0, HALF_EXTENT),
        },
        color: Color::new(58, 62, 70, 255),
    });

    // Towers on a jittered grid, leaving a clear plaza around the spawn point.
    let cell = 16.0;
    let n = (HALF_EXTENT / cell) as i32 - 1;
    for gx in -n..=n {
        for gz in -n..=n {
            if gx.abs() <= 1 && gz.abs() <= 1 {
                continue;
            }
            if rng.chance(0.25) {
                continue;
            }
            let w = rng.range(5.0, 11.0);
            let d = rng.range(5.0, 11.0);
            let h = if rng.chance(0.15) {
                rng.range(40.0, 90.0)
            } else {
                rng.range(2.0, 24.0)
            };
            let c = vec3(
                gx as f32 * cell + rng.range(-2.0, 2.0),
                h * 0.5,
                gz as f32 * cell + rng.range(-2.0, 2.0),
            );
            let shade = rng.range_i(70, 140) as u8;
            blocks.push(Block {
                aabb: Aabb::from_center(c, vec3(w, h, d)),
                color: Color::new(shade, shade + 6, shade + 14, 255),
            });

            // Climbable plinths stepping up the side of some towers.
            if rng.chance(0.4) {
                let steps = rng.range_i(2, 6);
                let side = if rng.chance(0.5) { 1.0 } else { -1.0 };
                for s in 0..steps {
                    let sh = 1.2 + s as f32 * 1.6;
                    if sh >= h {
                        break;
                    }
                    let sc = vec3(
                        c.x + side * (w * 0.5 + 1.2 + (steps - 1 - s) as f32 * 2.0),
                        sh * 0.5,
                        c.z,
                    );
                    blocks.push(Block {
                        aabb: Aabb::from_center(sc, vec3(2.0, sh, 3.0)),
                        color: Color::new(150, 120, 90, 255),
                    });
                }
            }
        }
    }

    // Elevated walkways spanning between rows.
    for _ in 0..14 {
        let y = rng.range(6.0, 20.0);
        let along_x = rng.chance(0.5);
        let len = rng.range(20.0, 60.0);
        let p = vec3(rng.range(-80.0, 80.0), y, rng.range(-80.0, 80.0));
        let size = if along_x {
            vec3(len, 0.6, 3.0)
        } else {
            vec3(3.0, 0.6, len)
        };
        blocks.push(Block {
            aabb: Aabb::from_center(p, size),
            color: Color::new(170, 180, 190, 255),
        });
    }

    // A few hand-placed blocks near spawn to test steps, jumps and ledge grabs.
    for (i, h) in [0.4_f32, 1.0, 1.6, 2.4, 3.2].iter().enumerate() {
        let c = Vec3::new(-8.0 + i as f32 * 4.0, h * 0.5, -10.0);
        blocks.push(Block {
            aabb: Aabb::from_center(c, vec3(3.0, *h, 3.0)),
            color: Color::new(200, 110, 160, 255),
        });
    }

    blocks
}
