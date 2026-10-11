//! The towers' elevations as a texture (`chunks::Catalogs::elevations`), read by `facade` in
//! `shaders/scene.fs`: each elevation is a block of `MAX_BAYS` x `FLOOR_STORIES` texels (bay from
//! the left seen from outside, story of the floor from the street up), sixteen blocks a row. A
//! texel holds the tile painted far away, the tile painted near a deck, the paint digit and the
//! number of bays. Rebuilt when the chunk files change.

use sola_raylib::ffi;

use crate::world::chunks::{self, FLOOR_STORIES, MAX_BAYS};

/// Material slot (sampler `texture6`) of the elevations.
pub const SLOT: usize = 6;
/// Elevations per row of the texture.
const PER_ROW: usize = 16;

pub struct Facades {
    pub texture: ffi::Texture2D,
    generation: u64,
}

fn pixels() -> (usize, usize, Vec<u8>) {
    let all = chunks::catalogs();
    let n = all.elevations.len().max(1);
    let w = PER_ROW * MAX_BAYS;
    let h = n.div_ceil(PER_ROW) * FLOOR_STORIES;
    let mut rgba = vec![0u8; w * h * 4];
    for (id, e) in all.elevations.iter().enumerate() {
        let (ox, oy) = ((id % PER_ROW) * MAX_BAYS, (id / PER_ROW) * FLOOR_STORIES);
        for row in 0..FLOOR_STORIES {
            for col in 0..e.cols.min(MAX_BAYS) {
                let ((far, near, paint), _) = e.at(row, col);
                let o = ((oy + row) * w + ox + col) * 4;
                rgba[o..o + 4].copy_from_slice(&[far, near, paint, e.cols as u8]);
            }
        }
    }
    (w, h, rgba)
}

fn upload() -> ffi::Texture2D {
    let (w, h, rgba) = pixels();
    unsafe {
        let image = ffi::Image {
            data: rgba.as_ptr() as *mut _,
            width: w as i32,
            height: h as i32,
            mipmaps: 1,
            format: ffi::PixelFormat::PIXELFORMAT_UNCOMPRESSED_R8G8B8A8 as i32,
        };
        let t = ffi::LoadTextureFromImage(image);
        ffi::SetTextureFilter(t, ffi::TextureFilter::TEXTURE_FILTER_POINT as i32);
        t
    }
}

impl Facades {
    /// Requires a GL context.
    pub fn new() -> Self {
        Self {
            texture: upload(),
            generation: chunks::generation(),
        }
    }

    /// Rebuilds the texture if the catalog was reloaded. Returns true when it did.
    pub fn refresh(&mut self) -> bool {
        if self.generation == chunks::generation() {
            return false;
        }
        self.generation = chunks::generation();
        unsafe { ffi::UnloadTexture(self.texture) };
        self.texture = upload();
        true
    }
}

impl Drop for Facades {
    fn drop(&mut self) {
        unsafe { ffi::UnloadTexture(self.texture) };
    }
}
