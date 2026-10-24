//! Many small point lights, kept cheap by a camera-centered 3D grid.
//!
//! Each frame the nearest lights are binned into grid cells (a few light indices per cell) and
//! uploaded in one float texture; the scene shader looks up only the lights of its own cell.
//! Lights fade out with distance before the grid ends or the count runs out (`update`), so none
//! switches on or off as the camera moves; their halos (`halos.rs`) reach further.
//! Texture layout (RGBA32F, `WIDTH` texels per row): light data first (3 texels per light:
//! position + radius, color + halo fade, vertical extent + light fade), then the grid (2 texels
//! per cell: 8 light indices, -1 = empty). The constants here are mirrored in `shaders/scene.fs`.

use glam::Vec3;
use sola_raylib::ffi;

use crate::world::Light;

const WIDTH: usize = 256;
pub const MAX_LIGHTS: usize = 1024;
const SLOTS: usize = 8;
const GRID: [usize; 3] = [32, 32, 32];
const CELL: f32 = 6.0;
/// Lights are gone this far from the camera: a cell short of the grid's side, which snaps by a
/// cell. Their halos, which need no grid, go on to `HALO_REACH`, short of the fog at the loaded
/// lots' edge. Both fade out over the last quarter of the way, or of the distance to the farthest
/// light kept when there are more than `MAX_LIGHTS`.
const REACH: f32 = (GRID[0] / 2 - 2) as f32 * CELL;
const HALO_REACH: f32 = 150.0;
const FADE_FROM: f32 = 0.75;
/// How much further a light reaches the walls facing it (`plantLights` in shaders/scene.fs).
const WALL_REACH: f32 = 1.6;
const LIGHT_TEXELS: usize = MAX_LIGHTS * 3;
const GRID_CELLS: usize = GRID[0] * GRID[1] * GRID[2];
const TEXELS: usize = LIGHT_TEXELS + GRID_CELLS * 2;
const HEIGHT: usize = TEXELS.div_ceil(WIDTH);

pub struct LightGrid {
    pub texture: ffi::Texture2D,
    pixels: Vec<f32>,
    counts: Vec<u8>,
    /// For each slot of each cell, how far into its light's reach the cell's center lies.
    depths: Vec<f32>,
    /// World position of the grid's minimum corner.
    pub origin: Vec3,
    /// Light indices left out of full cells last frame (`SLOTS`), for the debug overlay.
    pub overflow: usize,
}

impl LightGrid {
    pub fn new() -> Self {
        let format = ffi::rlPixelFormat::RL_PIXELFORMAT_UNCOMPRESSED_R32G32B32A32 as i32;
        let id =
            unsafe { ffi::rlLoadTexture(std::ptr::null(), WIDTH as i32, HEIGHT as i32, format, 1) };
        super::targets::set_params(id, ffi::RL_TEXTURE_FILTER_NEAREST);
        Self {
            texture: super::targets::texture(id, WIDTH as i32, HEIGHT as i32, format),
            pixels: vec![0.0; WIDTH * HEIGHT * 4],
            counts: vec![0; GRID_CELLS],
            depths: vec![0.0; GRID_CELLS * SLOTS],
            origin: Vec3::ZERO,
            overflow: 0,
        }
    }

    /// Bins the nearest lights around `cam` and uploads the result. Returns lights used.
    pub fn update(&mut self, lights: impl Iterator<Item = Light>, cam: Vec3) -> usize {
        let half = Vec3::new(GRID[0] as f32, GRID[1] as f32, GRID[2] as f32) * CELL * 0.5;
        self.origin = ((cam - half) / CELL).floor() * CELL;

        let dist = |l: &Light| l.pos.distance(cam);
        let mut near: Vec<Light> = lights.filter(|l| dist(l) < HALO_REACH).collect();
        // With more lights than fit, the reach shrinks to the farthest one kept, so they still
        // fade out before the cut.
        let mut halo_reach = HALO_REACH;
        if near.len() > MAX_LIGHTS {
            near.select_nth_unstable_by(MAX_LIGHTS, |a, b| dist(a).total_cmp(&dist(b)));
            halo_reach = dist(&near[MAX_LIGHTS]);
            near.truncate(MAX_LIGHTS);
        }
        let reach = halo_reach.min(REACH);
        near.sort_by(|a, b| dist(a).total_cmp(&dist(b)));

        let grid_base = LIGHT_TEXELS * 4;
        self.pixels[grid_base..].fill(-1.0);
        self.counts.fill(0);
        self.overflow = 0;

        for (i, l) in near.iter().enumerate() {
            let t = i * 12;
            let d = dist(l);
            let halo = 1.0 - smoothstep(FADE_FROM * halo_reach, halo_reach, d);
            let fade = 1.0 - smoothstep(FADE_FROM * reach, reach, d);
            self.pixels[t..t + 4].copy_from_slice(&[l.pos.x, l.pos.y, l.pos.z, l.radius]);
            self.pixels[t + 4..t + 8].copy_from_slice(&[l.color.x, l.color.y, l.color.z, halo]);
            self.pixels[t + 8..t + 12].copy_from_slice(&[l.y_lo, l.y_hi, fade, 0.0]);
            // Too far to light anything: only its halo shows.
            if fade <= 0.0 {
                continue;
            }

            let reach = Vec3::splat(l.radius * WALL_REACH);
            let lo = ((l.pos - reach - self.origin) / CELL).floor();
            let hi = ((l.pos + reach - self.origin) / CELL).floor();
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
                        let center =
                            self.origin + (Vec3::new(x as f32, y as f32, z as f32) + 0.5) * CELL;
                        let depth = l.pos.distance(center) / l.radius;
                        // A full cell keeps the lights strongest at its center: which ones does not
                        // hang on the camera, so none flickers as it moves.
                        let n = self.counts[c] as usize;
                        let slot = if n < SLOTS {
                            self.counts[c] += 1;
                            n
                        } else {
                            self.overflow += 1;
                            let slots = &self.depths[c * SLOTS..(c + 1) * SLOTS];
                            let (k, &weakest) = slots
                                .iter()
                                .enumerate()
                                .max_by(|a, b| a.1.total_cmp(b.1))
                                .unwrap();
                            if depth >= weakest {
                                continue;
                            }
                            k
                        };
                        self.pixels[grid_base + c * SLOTS + slot] = i as f32;
                        self.depths[c * SLOTS + slot] = depth;
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

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
