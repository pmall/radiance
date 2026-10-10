//! The cover map: which decks stand over the lots around the camera, and how much sky each lot
//! sees on each floor. Light falls through the city only where the gaps in the floors above line
//! up (see `city::deck`); the shadow map alone reaches too short a distance for that.
//!
//! One RGBA texture of four `SIZE` x `SIZE` tiles, one texel per lot, centered on the camera:
//! - tile 0, red: the lot's `city::cover_bits` (bit `f - 1` for a deck on floor `f`), read
//!   exactly (`texelFetch`) to trace the sun through the floors;
//! - tiles 1..=3: the sky view of the lot on floor `f` in tile `1 + f / 4`, channel `f % 4`
//!   (0 = no sky, 1 = open sky), read with linear filtering so it blends between lots.
//!
//! Rebuilt when the camera changes lot; each lot's data is cached, so a move only computes the
//! new edge. `SIZE` and the layout are mirrored in `shaders/scene.fs` and `shaders/post.fs`.

use std::collections::HashMap;

use glam::{Vec2, Vec3, vec3};
use sola_raylib::ffi;

use crate::world::{city, floors};

pub const SIZE: i32 = 64;
const TILES: i32 = 4;
/// Directions traced per lot and floor to estimate its sky view.
const VIEW_RAYS: usize = 24;

pub struct CoverMap {
    pub texture: ffi::Texture2D,
    /// Lot coordinates of texel (0, 0) of each tile.
    pub origin: (i32, i32),
    seed: Option<u64>,
    bits: HashMap<(i32, i32), u8>,
    views: HashMap<(i32, i32), [u8; floors::COUNT]>,
    rays: [Vec3; VIEW_RAYS],
    pixels: Vec<u8>,
}

/// Cosine-weighted directions over the upper hemisphere (a Fibonacci spiral), so the share of
/// them that reaches the sky is the sky's share of diffuse light.
fn view_rays() -> [Vec3; VIEW_RAYS] {
    std::array::from_fn(|i| {
        let r = ((i as f32 + 0.5) / VIEW_RAYS as f32).sqrt();
        let phi = i as f32 * 2.399_963;
        vec3(r * phi.cos(), (1.0 - r * r).sqrt(), r * phi.sin())
    })
}

fn lot_of(p: Vec2) -> (i32, i32) {
    (
        (p.x / city::LOT).floor() as i32,
        (p.y / city::LOT).floor() as i32,
    )
}

fn bits_of(cache: &mut HashMap<(i32, i32), u8>, seed: u64, lot: (i32, i32)) -> u8 {
    *cache
        .entry(lot)
        .or_insert_with(|| city::cover_bits(seed, lot.0, lot.1))
}

/// Whether a ray from `p` along `dir` clears every deck above it.
fn clears(cache: &mut HashMap<(i32, i32), u8>, seed: u64, p: Vec3, dir: Vec3) -> bool {
    (1..floors::COUNT).all(|g| {
        let y = floors::Y[g] - 0.6;
        if y < p.y + 0.2 {
            return true;
        }
        let q = p + dir * ((y - p.y) / dir.y);
        bits_of(cache, seed, lot_of(Vec2::new(q.x, q.z))) & 1 << (g - 1) == 0
    })
}

impl CoverMap {
    pub fn new() -> Self {
        let format = ffi::rlPixelFormat::RL_PIXELFORMAT_UNCOMPRESSED_R8G8B8A8 as i32;
        let (w, h) = (SIZE * TILES, SIZE);
        let id = unsafe { ffi::rlLoadTexture(std::ptr::null(), w, h, format, 1) };
        super::targets::set_params(id, ffi::RL_TEXTURE_FILTER_LINEAR);
        Self {
            texture: ffi::Texture2D {
                id,
                width: w,
                height: h,
                mipmaps: 1,
                format,
            },
            origin: (i32::MIN, i32::MIN),
            seed: None,
            bits: HashMap::new(),
            views: HashMap::new(),
            rays: view_rays(),
            pixels: vec![0; (w * h * 4) as usize],
        }
    }

    /// Sky view of a lot on every floor, 0..=255.
    fn view(&mut self, seed: u64, lot: (i32, i32)) -> [u8; floors::COUNT] {
        if let Some(v) = self.views.get(&lot) {
            return *v;
        }
        let center = Vec2::new(lot.0 as f32 + 0.5, lot.1 as f32 + 0.5) * city::LOT;
        let v = std::array::from_fn(|f| {
            let p = vec3(center.x, floors::Y[f] + 2.0, center.y);
            let open = self
                .rays
                .iter()
                .filter(|&&d| clears(&mut self.bits, seed, p, d))
                .count();
            (open * 255 / VIEW_RAYS) as u8
        });
        self.views.insert(lot, v);
        v
    }

    /// Recenters the map on the camera's lot if needed, and uploads it.
    pub fn update(&mut self, seed: u64, cam: Vec3) {
        if self.seed != Some(seed) {
            self.seed = Some(seed);
            self.bits.clear();
            self.views.clear();
            self.origin = (i32::MIN, i32::MIN);
        }
        let (cx, cz) = lot_of(Vec2::new(cam.x, cam.z));
        let origin = (cx - SIZE / 2, cz - SIZE / 2);
        if origin == self.origin {
            return;
        }
        self.origin = origin;
        // Forget lots far out of the window (sky view rays reach about 16 lots out).
        let keep = |v: i32, o: i32| (o - 24..o + SIZE + 24).contains(&v);
        self.bits
            .retain(|&(x, z), _| keep(x, origin.0) && keep(z, origin.1));
        self.views
            .retain(|&(x, z), _| keep(x, origin.0) && keep(z, origin.1));
        let row = (SIZE * TILES) as usize;
        for tz in 0..SIZE {
            for tx in 0..SIZE {
                let lot = (origin.0 + tx, origin.1 + tz);
                let base = (tz as usize * row + tx as usize) * 4;
                self.pixels[base] = bits_of(&mut self.bits, seed, lot);
                for (f, v) in self.view(seed, lot).into_iter().enumerate() {
                    let tile = 1 + f / 4;
                    self.pixels[base + tile * SIZE as usize * 4 + f % 4] = v;
                }
            }
        }
        unsafe {
            ffi::rlUpdateTexture(
                self.texture.id,
                0,
                0,
                SIZE * TILES,
                SIZE,
                self.texture.format,
                self.pixels.as_ptr() as *const _,
            );
        }
    }

    /// Texel origin for the shaders, in lots.
    pub fn origin_uniform(&self) -> Vec2 {
        Vec2::new(self.origin.0 as f32, self.origin.1 as f32)
    }

    /// Sky view at `p`, 0..1 (CPU twin of the shaders' `skyView`, without the blending).
    pub fn sky_view(&mut self, p: Vec3) -> f32 {
        let Some(seed) = self.seed else {
            return 1.0;
        };
        let v = self.view(seed, lot_of(Vec2::new(p.x, p.z)));
        v[floors::floor_at(p.y)] as f32 / 255.0
    }
}

/// Floor heights for the shaders' `uFloorY` uniform.
pub fn floor_uniform() -> [f32; floors::COUNT] {
    floors::Y
}
