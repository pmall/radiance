//! Many small point lights, kept cheap by a camera-centered 3D grid.
//!
//! Each frame the nearest lights are binned into grid cells (a few light indices per cell) and
//! uploaded in one float texture; the scene shader looks up only the lights of its own cell.
//! Texture layout (RGBA32F, `WIDTH` texels per row): light data first (3 texels per light:
//! position + radius, color, vertical extent), then the grid (2 texels per cell: 8 light indices, -1 = empty).
//! The constants here are mirrored in `shaders/scene.fs`.

use glam::Vec3;
use sola_raylib::ffi;

use crate::world::Light;

const WIDTH: usize = 256;
pub const MAX_LIGHTS: usize = 512;
const SLOTS: usize = 8;
const GRID: [usize; 3] = [32, 16, 32];
const CELL: f32 = 6.0;
const LIGHT_TEXELS: usize = MAX_LIGHTS * 3;
const GRID_CELLS: usize = GRID[0] * GRID[1] * GRID[2];
const TEXELS: usize = LIGHT_TEXELS + GRID_CELLS * 2;
const HEIGHT: usize = TEXELS.div_ceil(WIDTH);

pub struct LightGrid {
    pub texture: ffi::Texture2D,
    pixels: Vec<f32>,
    counts: Vec<u8>,
    /// World position of the grid's minimum corner.
    pub origin: Vec3,
}

impl LightGrid {
    pub fn new() -> Self {
        let format = ffi::rlPixelFormat::RL_PIXELFORMAT_UNCOMPRESSED_R32G32B32A32 as i32;
        let id =
            unsafe { ffi::rlLoadTexture(std::ptr::null(), WIDTH as i32, HEIGHT as i32, format, 1) };
        super::targets::set_params(id, ffi::RL_TEXTURE_FILTER_NEAREST);
        Self {
            texture: ffi::Texture2D {
                id,
                width: WIDTH as i32,
                height: HEIGHT as i32,
                mipmaps: 1,
                format,
            },
            pixels: vec![0.0; WIDTH * HEIGHT * 4],
            counts: vec![0; GRID_CELLS],
            origin: Vec3::ZERO,
        }
    }

    /// Bins the nearest lights around `cam` and uploads the result. Returns lights used.
    pub fn update(&mut self, lights: impl Iterator<Item = Light>, cam: Vec3) -> usize {
        let half = Vec3::new(GRID[0] as f32, GRID[1] as f32, GRID[2] as f32) * CELL * 0.5;
        self.origin = ((cam - half) / CELL).floor() * CELL;

        let mut near: Vec<Light> = lights.collect();
        if near.len() > MAX_LIGHTS {
            near.select_nth_unstable_by(MAX_LIGHTS, |a, b| {
                a.pos
                    .distance_squared(cam)
                    .total_cmp(&b.pos.distance_squared(cam))
            });
            near.truncate(MAX_LIGHTS);
        }
        // Nearest first, so crowded cells keep the lights that matter most.
        near.sort_by(|a, b| {
            a.pos
                .distance_squared(cam)
                .total_cmp(&b.pos.distance_squared(cam))
        });

        let grid_base = LIGHT_TEXELS * 4;
        self.pixels[grid_base..].fill(-1.0);
        self.counts.fill(0);

        for (i, l) in near.iter().enumerate() {
            let t = i * 12;
            self.pixels[t..t + 4].copy_from_slice(&[l.pos.x, l.pos.y, l.pos.z, l.radius]);
            self.pixels[t + 4..t + 8].copy_from_slice(&[l.color.x, l.color.y, l.color.z, 0.0]);
            self.pixels[t + 8..t + 12].copy_from_slice(&[l.y_lo, l.y_hi, 0.0, 0.0]);

            let lo = ((l.pos - Vec3::splat(l.radius) - self.origin) / CELL).floor();
            let hi = ((l.pos + Vec3::splat(l.radius) - self.origin) / CELL).floor();
            let range = |axis: usize| {
                let (a, b) = (
                    lo[axis].max(0.0) as i32,
                    hi[axis].min(GRID[axis] as f32 - 1.0) as i32,
                );
                (hi[axis] >= 0.0).then_some(a..=b)
            };
            let (Some(xs), Some(ys), Some(zs)) = (range(0), range(1), range(2)) else {
                continue;
            };
            for z in zs {
                for y in ys.clone() {
                    for x in xs.clone() {
                        let c = x as usize + GRID[0] * (y as usize + GRID[1] * z as usize);
                        let n = self.counts[c] as usize;
                        if n < SLOTS {
                            self.pixels[grid_base + c * 8 + n] = i as f32;
                            self.counts[c] += 1;
                        }
                    }
                }
            }
        }

        unsafe {
            ffi::rlUpdateTexture(
                self.texture.id,
                0,
                0,
                WIDTH as i32,
                HEIGHT as i32,
                self.texture.format,
                self.pixels.as_ptr() as *const _,
            );
        }
        near.len()
    }
}
