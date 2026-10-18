//! A tileable noise texture for the scene shader (sampler `texture0`): four octaves of smooth value
//! noise, one per channel, so one fetch gives what the shader would otherwise work out with a
//! dozen hashes (`noise4` in `shaders/scene.fs`). Fixed, like the pixel art: the seed picks where
//! things go, not the grain of the noise.

use sola_raylib::ffi;

use crate::rng::hash_coords;

/// Side of the texture in texels. The shader maps 8 units of its noise onto it.
pub const SIZE: usize = 256;
/// Material slot (sampler `texture0`) of the noise.
pub const SLOT: usize = 0;

/// Cells across the texture of each channel's noise: 8, 16, 32, 64 (1, 2, 4, 8 per shader unit).
const CELLS: [usize; 4] = [8, 16, 32, 64];

/// Smooth value noise with `cells` cells across the texture, wrapping, 0..1.
fn value(x: usize, y: usize, cells: usize, salt: i32) -> f32 {
    let side = SIZE / cells;
    let (fx, fy) = (x as f32 / side as f32, y as f32 / side as f32);
    let (ix, iy) = (fx.floor() as usize, fy.floor() as usize);
    let (tx, ty) = (fx - ix as f32, fy - iy as f32);
    let (sx, sy) = (tx * tx * (3.0 - 2.0 * tx), ty * ty * (3.0 - 2.0 * ty));
    let h = |i: usize, j: usize| {
        let v = hash_coords(0x0015_e5ed, (i % cells) as i32, salt, (j % cells) as i32);
        (v >> 40) as f32 / (1u64 << 24) as f32
    };
    let a = h(ix, iy) + (h(ix + 1, iy) - h(ix, iy)) * sx;
    let b = h(ix, iy + 1) + (h(ix + 1, iy + 1) - h(ix, iy + 1)) * sx;
    a + (b - a) * sy
}

/// RGBA8 texels, a channel per octave.
pub fn pixels() -> Vec<u8> {
    let mut rgba = vec![0u8; SIZE * SIZE * 4];
    for y in 0..SIZE {
        for x in 0..SIZE {
            for (c, &cells) in CELLS.iter().enumerate() {
                let v = value(x, y, cells, c as i32);
                rgba[(y * SIZE + x) * 4 + c] = (v * 255.0).round() as u8;
            }
        }
    }
    rgba
}

/// The noise as a texture: wrapping, filtered between texels, no mipmaps (the shader reads level
/// 0). Requires a GL context.
pub fn load() -> ffi::Texture2D {
    let rgba = pixels();
    unsafe {
        let image = ffi::Image {
            data: rgba.as_ptr() as *mut _,
            width: SIZE as i32,
            height: SIZE as i32,
            mipmaps: 1,
            format: ffi::PixelFormat::PIXELFORMAT_UNCOMPRESSED_R8G8B8A8 as i32,
        };
        let texture = ffi::LoadTextureFromImage(image);
        ffi::SetTextureWrap(texture, ffi::TextureWrap::TEXTURE_WRAP_REPEAT as i32);
        ffi::SetTextureFilter(texture, ffi::TextureFilter::TEXTURE_FILTER_BILINEAR as i32);
        texture
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every channel wraps: the first column continues the last one smoothly.
    #[test]
    fn noise_wraps() {
        for (c, &cells) in CELLS.iter().enumerate() {
            let step = 1.0 / (SIZE / cells) as f32;
            let d = (value(0, 7, cells, c as i32) - value(SIZE - 1, 7, cells, c as i32)).abs();
            assert!(d <= step + 1e-3, "channel {c} jumps by {d}");
        }
    }
}
