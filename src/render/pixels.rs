//! Pixel-art textures of the chunk materials (`world::chunks::Mat`, `docs/chunks.md`): one tile
//! of 4 m at 16 pixels per meter per material, drawn here by code (limited palettes, ordered
//! dithering, chunky joints, cracks, rivets and stains) and sampled with crisp nearest filtering
//! by `pixelMat` in `shaders/scene.fs`. Tiles wrap seamlessly. Fixed, not seeded: the seed picks
//! where materials go, not how they look.

use sola_raylib::ffi;

use crate::rng::{Rng, hash_coords};

/// Side of a material tile in pixels (4 m).
pub const TILE: usize = 64;
/// Materials, in `Mat::index` order.
pub const COUNT: usize = 11;
/// Material slot (sampler `texture10`) of the atlas.
pub const SLOT: usize = 10;

type Rgb = [u8; 3];

/// 4x4 ordered dither thresholds, in 0..1.
const BAYER: [[f32; 4]; 4] = [
    [0.0, 8.0, 2.0, 10.0],
    [12.0, 4.0, 14.0, 6.0],
    [3.0, 11.0, 1.0, 9.0],
    [15.0, 7.0, 13.0, 5.0],
];

fn hash(x: i32, y: i32, salt: i32) -> f32 {
    (hash_coords(0x5eed, x, salt, y) >> 40) as f32 / (1u64 << 24) as f32
}

/// Tileable value noise with cells of `cx` x `cy` pixels (each must divide `TILE`), 0..1.
fn noise(x: usize, y: usize, cx: usize, cy: usize, salt: i32) -> f32 {
    let (gx, gy) = (TILE / cx, TILE / cy);
    let (fx, fy) = (x as f32 / cx as f32, y as f32 / cy as f32);
    let (ix, iy) = (fx.floor() as usize, fy.floor() as usize);
    let (tx, ty) = (fx - ix as f32, fy - iy as f32);
    let (sx, sy) = (tx * tx * (3.0 - 2.0 * tx), ty * ty * (3.0 - 2.0 * ty));
    let h = |i: usize, j: usize| hash((i % gx) as i32, (j % gy) as i32, salt);
    let a = h(ix, iy) + (h(ix + 1, iy) - h(ix, iy)) * sx;
    let b = h(ix, iy + 1) + (h(ix + 1, iy + 1) - h(ix, iy + 1)) * sx;
    a + (b - a) * sy
}

/// A palette color for a shade `t` in 0..1, dithered between neighbors.
fn ramp(pal: &[Rgb], t: f32, x: usize, y: usize) -> Rgb {
    let n = pal.len() as f32 - 1.0;
    let v = t.clamp(0.0, 1.0) * n + (BAYER[y % 4][x % 4] / 16.0 - 0.5) * 0.9;
    pal[(v.round().clamp(0.0, n)) as usize]
}

struct Tile {
    px: Vec<Rgb>,
}

impl Tile {
    fn new() -> Tile {
        Tile {
            px: vec![[0, 0, 0]; TILE * TILE],
        }
    }

    fn fill(&mut self, f: impl Fn(usize, usize) -> Rgb) {
        for y in 0..TILE {
            for x in 0..TILE {
                self.px[y * TILE + x] = f(x, y);
            }
        }
    }

    fn set(&mut self, x: i32, y: i32, c: Rgb) {
        let (x, y) = (x.rem_euclid(TILE as i32) as usize, y.rem_euclid(TILE as i32) as usize);
        self.px[y * TILE + x] = c;
    }

    fn get(&self, x: i32, y: i32) -> Rgb {
        let (x, y) = (x.rem_euclid(TILE as i32) as usize, y.rem_euclid(TILE as i32) as usize);
        self.px[y * TILE + x]
    }

    /// Thin dark cracks wandering from random points.
    fn cracks(&mut self, count: usize, len: usize, color: Rgb, salt: u64) {
        let mut rng = Rng::new(salt);
        for _ in 0..count {
            let (mut x, mut y) = (rng.range_i(0, TILE as i32), rng.range_i(0, TILE as i32));
            let (mut dx, mut dy) = (rng.range_i(-1, 2), rng.range_i(-1, 2));
            if dx == 0 && dy == 0 {
                dx = 1;
            }
            for _ in 0..len {
                self.set(x, y, color);
                if rng.chance(0.35) {
                    dx = rng.range_i(-1, 2);
                }
                if rng.chance(0.35) {
                    dy = rng.range_i(-1, 2);
                }
                x += dx;
                y += dy;
            }
        }
    }
}

const STONE: [Rgb; 5] = [
    [70, 66, 62],
    [104, 98, 90],
    [138, 130, 118],
    [166, 158, 142],
    [192, 184, 166],
];
const STONE_DARK: [Rgb; 5] = [
    [34, 34, 38],
    [52, 52, 56],
    [72, 70, 72],
    [94, 92, 92],
    [116, 112, 110],
];
const CONCRETE: [Rgb; 5] = [
    [62, 64, 66],
    [88, 90, 90],
    [112, 114, 112],
    [136, 138, 134],
    [160, 160, 154],
];
const BRICK: [Rgb; 5] = [
    [64, 30, 26],
    [96, 44, 34],
    [128, 60, 44],
    [154, 78, 56],
    [176, 100, 74],
];
const TILES: [Rgb; 5] = [
    [70, 84, 84],
    [110, 128, 124],
    [150, 166, 158],
    [184, 196, 184],
    [210, 216, 202],
];
const METAL: [Rgb; 5] = [
    [40, 44, 52],
    [62, 68, 78],
    [86, 94, 104],
    [112, 120, 130],
    [140, 148, 156],
];
const RUST: [Rgb; 5] = [
    [66, 32, 20],
    [104, 50, 26],
    [140, 72, 34],
    [172, 100, 48],
    [196, 130, 70],
];
const WOOD: [Rgb; 5] = [
    [58, 38, 26],
    [84, 56, 36],
    [110, 76, 48],
    [136, 98, 62],
    [160, 120, 80],
];
const GLASS: [Rgb; 5] = [
    [18, 26, 34],
    [28, 40, 50],
    [42, 58, 70],
    [64, 84, 96],
    [110, 134, 144],
];
const SOIL: [Rgb; 5] = [
    [30, 22, 16],
    [46, 34, 24],
    [64, 48, 32],
    [84, 64, 42],
    [104, 84, 58],
];
const PAINT: [Rgb; 5] = [
    [120, 120, 120],
    [168, 168, 168],
    [200, 200, 200],
    [226, 226, 226],
    [246, 246, 246],
];

/// Paving slabs of `slab` pixels: a shade per slab, dark joints, a lit edge, specks and cracks.
fn slabs(pal: &[Rgb], slab: usize, salt: i32) -> Tile {
    let mut t = Tile::new();
    t.fill(|x, y| {
        let (sx, sy) = ((x / slab) as i32, (y / slab) as i32);
        if x % slab == 0 || y % slab == 0 {
            return pal[0];
        }
        let mut v = 0.35 + 0.3 * hash(sx, sy, salt);
        v += 0.25 * (noise(x, y, 8, 8, salt + 1) - 0.5) + 0.2 * (noise(x, y, 2, 2, salt + 2) - 0.5);
        if x % slab == 1 || y % slab == 1 {
            v += 0.15;
        }
        if hash(x as i32, y as i32, salt + 3) > 0.97 {
            v -= 0.3;
        }
        ramp(pal, v, x, y)
    });
    t.cracks(5, 14, pal[0], salt as u64);
    t
}

fn concrete() -> Tile {
    let mut t = Tile::new();
    t.fill(|x, y| {
        let mut v = 0.5 + 0.35 * (noise(x, y, 16, 16, 20) - 0.5) + 0.25 * (noise(x, y, 4, 4, 21) - 0.5);
        // Rain streaks running down from some columns.
        let streak = hash(x as i32, 0, 22);
        if streak > 0.8 {
            v -= 0.25 * (y as f32 / TILE as f32) * (noise(x, y, 1, 8, 23) + 0.3);
        }
        // Form-tie holes every meter.
        if (x % 16 == 7 || x % 16 == 8) && y % 16 == 8 {
            v = 0.0;
        }
        // Formwork joints.
        if y % 32 == 0 {
            v -= 0.2;
        }
        ramp(&CONCRETE, v, x, y)
    });
    t.cracks(3, 10, CONCRETE[0], 24);
    t
}

fn brick() -> Tile {
    let mut t = Tile::new();
    t.fill(|x, y| {
        let row = y / 4;
        let xo = x + if row % 2 == 1 { 4 } else { 0 };
        let (bx, by) = ((xo / 8) as i32, row as i32);
        if y % 4 == 3 || xo % 8 == 7 {
            // Mortar.
            let m = 0.3 + 0.3 * noise(x, y, 4, 4, 30);
            return ramp(&STONE_DARK, m + 0.3, x, y);
        }
        let mut v = 0.3 + 0.45 * hash(bx, by, 31);
        if hash(bx, by, 32) > 0.9 {
            v -= 0.3;
        }
        v += 0.15 * (noise(x, y, 2, 2, 33) - 0.5);
        if y % 4 == 0 {
            v += 0.12;
        }
        // Soot and damp, in large patches.
        v -= 0.3 * (noise(x, y, 32, 32, 34) - 0.4).max(0.0);
        ramp(&BRICK, v, x, y)
    });
    t
}

fn tiles() -> Tile {
    let mut t = Tile::new();
    t.fill(|x, y| {
        let (tx, ty) = ((x / 8) as i32, (y / 8) as i32);
        if x % 8 == 0 || y % 8 == 0 {
            return TILES[0];
        }
        // A few tiles missing: bare concrete.
        if hash(tx, ty, 40) > 0.94 {
            return ramp(&CONCRETE, 0.3 + 0.3 * noise(x, y, 2, 2, 41), x, y);
        }
        let mut v = 0.55 + 0.3 * (hash(tx, ty, 42) - 0.5) + 0.15 * (noise(x, y, 4, 4, 43) - 0.5);
        if x % 8 == 1 || y % 8 == 1 {
            v += 0.15;
        }
        ramp(&TILES, v, x, y)
    });
    t.cracks(4, 9, TILES[0], 44);
    t
}

fn metal(rust: f32) -> Tile {
    let mut t = Tile::new();
    t.fill(|x, y| {
        let mut v = 0.5 + 0.3 * (noise(x, y, 16, 1, 50) - 0.5) + 0.15 * (noise(x, y, 2, 2, 51) - 0.5);
        // Panels with seams, lit under each seam, rivets along them.
        if y % 16 == 0 || x % 32 == 0 {
            v = 0.05;
        } else if y % 16 == 1 || x % 32 == 1 {
            v += 0.2;
        }
        if (y % 16 == 2 || y % 16 == 14) && x % 8 == 4 {
            v = 1.0;
        }
        let m = ramp(&METAL, v, x, y);
        let r = noise(x, y, 8, 8, 52) * 0.6 + noise(x, y, 2, 2, 53) * 0.4 + (y as f32 / TILE as f32) * 0.2;
        if r + rust - 0.5 > 0.62 - BAYER[y % 4][x % 4] / 40.0 {
            ramp(&RUST, 0.3 + 0.6 * noise(x, y, 4, 4, 54), x, y)
        } else {
            m
        }
    });
    t
}

fn wood() -> Tile {
    let mut t = Tile::new();
    t.fill(|x, y| {
        let plank = (y / 4) as i32;
        if y % 4 == 0 {
            return WOOD[0];
        }
        let shift = (hash(plank, 0, 60) * TILE as f32) as usize;
        let mut v = 0.35 + 0.3 * hash(plank, 1, 61);
        v += 0.35 * (noise((x + shift) % TILE, y, 16, 1, 62) - 0.5);
        if y % 4 == 1 {
            v += 0.12;
        }
        // Plank ends and knots.
        if (x + shift).is_multiple_of(32) {
            v = 0.0;
        }
        if hash(x as i32, y as i32, 63) > 0.985 {
            v -= 0.35;
        }
        ramp(&WOOD, v, x, y)
    });
    t
}

fn glass() -> Tile {
    let mut t = Tile::new();
    t.fill(|x, y| {
        let mut v = 0.25 + 0.2 * (y as f32 / TILE as f32) + 0.1 * (noise(x, y, 8, 8, 70) - 0.5);
        // Diagonal reflections.
        let d = (x + TILE - y % TILE) % 32;
        if d < 3 {
            v += 0.45;
        } else if d < 5 {
            v += 0.2;
        }
        // Grime at the foot.
        if y > 52 {
            v -= 0.15 * noise(x, y, 4, 2, 71);
        }
        ramp(&GLASS, v, x, y)
    });
    t
}

fn soil() -> Tile {
    let mut t = Tile::new();
    t.fill(|x, y| {
        let v = 0.45 + 0.4 * (noise(x, y, 4, 4, 80) - 0.5) + 0.3 * (noise(x, y, 2, 2, 81) - 0.5);
        if hash(x as i32, y as i32, 82) > 0.97 {
            return ramp(&STONE, 0.5 + 0.4 * hash(x as i32, y as i32, 83), x, y);
        }
        ramp(&SOIL, v, x, y)
    });
    t.cracks(6, 12, SOIL[0], 84);
    t
}

fn paint() -> Tile {
    let mut t = Tile::new();
    t.fill(|x, y| {
        let mut v = 0.75 + 0.2 * (noise(x, y, 16, 16, 90) - 0.5) + 0.1 * (noise(x, y, 2, 2, 91) - 0.5);
        // Chipped paint showing the primer.
        if noise(x, y, 4, 4, 92) * 0.7 + noise(x, y, 2, 2, 93) * 0.3 > 0.72 {
            v = 0.15;
        }
        // Grime streaks running down.
        if hash(x as i32, 0, 94) > 0.85 {
            v -= 0.25 * (y as f32 / TILE as f32);
        }
        ramp(&PAINT, v, x, y)
    });
    // Outline the chips one pixel darker below and right, like flaking paint.
    let src = t.px.clone();
    for y in 0..TILE as i32 {
        for x in 0..TILE as i32 {
            let chip = |c: Rgb| c == PAINT[0];
            if !chip(src[y as usize * TILE + x as usize]) && chip(t.get(x - 1, y - 1)) {
                t.set(x, y, PAINT[1]);
            }
        }
    }
    t
}

/// All tiles side by side, RGBA, in `Mat::index` order.
pub fn atlas_pixels() -> (usize, usize, Vec<u8>) {
    let tiles = [
        slabs(&STONE, 16, 1),
        slabs(&STONE_DARK, 8, 10),
        concrete(),
        brick(),
        tiles(),
        metal(0.0),
        metal(0.45),
        wood(),
        glass(),
        soil(),
        paint(),
    ];
    let w = TILE * COUNT;
    let mut rgba = vec![255u8; w * TILE * 4];
    for (i, t) in tiles.iter().enumerate() {
        for y in 0..TILE {
            for x in 0..TILE {
                let c = t.px[y * TILE + x];
                let o = (y * w + i * TILE + x) * 4;
                rgba[o..o + 3].copy_from_slice(&c);
            }
        }
    }
    (w, TILE, rgba)
}

/// The atlas as a texture: crisp pixels up close, mipmapped far away. Requires a GL context.
pub fn load() -> ffi::Texture2D {
    let (w, h, rgba) = atlas_pixels();
    unsafe {
        let image = ffi::Image {
            data: rgba.as_ptr() as *mut _,
            width: w as i32,
            height: h as i32,
            mipmaps: 1,
            format: ffi::PixelFormat::PIXELFORMAT_UNCOMPRESSED_R8G8B8A8 as i32,
        };
        let mut t = ffi::LoadTextureFromImage(image);
        ffi::GenTextureMipmaps(&mut t);
        ffi::SetTextureFilter(t, ffi::TextureFilter::TEXTURE_FILTER_POINT as i32);
        t
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiles_wrap_and_fill_the_atlas() {
        let (w, h, rgba) = atlas_pixels();
        assert_eq!((w, h), (TILE * COUNT, TILE));
        assert_eq!(rgba.len(), w * h * 4);
        // Noise wraps: the first and last columns of a noise field match up smoothly.
        let d = (noise(0, 5, 8, 8, 1) - noise(TILE, 5, 8, 8, 1)).abs();
        assert!(d < 1e-6);
    }
}
