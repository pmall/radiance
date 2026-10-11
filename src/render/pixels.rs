//! Pixel-art textures of the chunk materials (`world::chunks::Mat`, `docs/chunks.md`): one tile
//! of 4 m at 16 pixels per meter per material, drawn here by code (limited palettes, ordered
//! dithering, chunky joints, cracks, rivets and stains) and sampled with crisp nearest filtering
//! by `pixelMat` in `shaders/scene.fs`. Tiles wrap seamlessly. Fixed, not seeded: the seed picks
//! where materials go, not how they look.

use sola_raylib::ffi;

use crate::rng::{Rng, hash_coords};

/// Side of a material tile in pixels (4 m).
pub const TILE: usize = 64;
/// Chunk materials, in `Mat::index` order.
pub const MATERIALS: usize = 11;
/// Tower facade units (`facade_tiles`), after the materials: 4 m bays one story (4 m) high.
pub const FACADES: usize = 11;
/// Tiles in the atlas.
pub const COUNT: usize = MATERIALS + FACADES;
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
    /// Wall pixels, which the shader tints in the building's colors (alpha 255; others 0).
    wall: Vec<bool>,
}

impl Tile {
    fn new() -> Tile {
        Tile {
            px: vec![[0, 0, 0]; TILE * TILE],
            wall: vec![false; TILE * TILE],
        }
    }

    /// Paints the rectangle `x0..x1` x `y0..y1` (not wall).
    fn rect(
        &mut self,
        (x0, y0): (i32, i32),
        (x1, y1): (i32, i32),
        f: impl Fn(usize, usize) -> Rgb,
    ) {
        for y in y0.max(0)..y1.min(TILE as i32) {
            for x in x0.max(0)..x1.min(TILE as i32) {
                let i = y as usize * TILE + x as usize;
                self.px[i] = f(x as usize, y as usize);
                self.wall[i] = false;
            }
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
        let (x, y) = (
            x.rem_euclid(TILE as i32) as usize,
            y.rem_euclid(TILE as i32) as usize,
        );
        self.px[y * TILE + x] = c;
    }

    fn get(&self, x: i32, y: i32) -> Rgb {
        let (x, y) = (
            x.rem_euclid(TILE as i32) as usize,
            y.rem_euclid(TILE as i32) as usize,
        );
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
        let mut v =
            0.5 + 0.35 * (noise(x, y, 16, 16, 20) - 0.5) + 0.25 * (noise(x, y, 4, 4, 21) - 0.5);
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
        let mut v =
            0.5 + 0.3 * (noise(x, y, 16, 1, 50) - 0.5) + 0.15 * (noise(x, y, 2, 2, 51) - 0.5);
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
        let r = noise(x, y, 8, 8, 52) * 0.6
            + noise(x, y, 2, 2, 53) * 0.4
            + (y as f32 / TILE as f32) * 0.2;
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
        let mut v =
            0.75 + 0.2 * (noise(x, y, 16, 16, 90) - 0.5) + 0.1 * (noise(x, y, 2, 2, 91) - 0.5);
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

// Tower facades (`facade` in shaders/scene.fs): one tile is a bay 4 m wide and a story 4 m high,
// row 0 at the top of the story. Wall pixels are a pale gray the shader tints per building and
// color block; everything else (frames, glass, cages, shutters) keeps its own colors. Abandoned:
// the windows are dark, the signs unlit.

const WALL: [Rgb; 5] = [
    [120, 120, 120],
    [150, 150, 150],
    [176, 176, 176],
    [200, 200, 200],
    [222, 222, 222],
];
const FRAME: [Rgb; 3] = [[92, 92, 88], [168, 166, 158], [206, 204, 196]];
const DARK: [Rgb; 4] = [[8, 10, 14], [14, 18, 24], [22, 28, 36], [34, 42, 52]];

/// A wall tile: stained, dithered pale gray (tinted by the shader), a slab ledge at the foot of
/// the story with a drip line under it.
fn facade_wall(salt: i32) -> Tile {
    let mut t = Tile::new();
    t.fill(|x, y| {
        let mut v = 0.55
            + 0.3 * (noise(x, y, 16, 16, salt) - 0.5)
            + 0.2 * (noise(x, y, 4, 4, salt + 1) - 0.5);
        // Grime gathering under the ledge above, rain streaks down some columns.
        v -= 0.25 * (1.0 - y as f32 / 10.0).max(0.0);
        if hash(x as i32, 0, salt + 2) > 0.82 {
            v -= 0.22 * noise(x, y, 1, 16, salt + 3);
        }
        ramp(&WALL, v, x, y)
    });
    t.wall.iter_mut().for_each(|w| *w = true);
    // The slab ledge: concrete, lit on top, dark under.
    t.rect((0, 59), (64, 64), |x, y| {
        let v = if y == 59 {
            0.95
        } else if y == 63 {
            0.1
        } else {
            0.55 + 0.25 * (noise(x, y, 4, 1, salt + 4) - 0.5)
        };
        ramp(&CONCRETE, v, x, y)
    });
    t.cracks(2, 8, WALL[0], salt as u64);
    t
}

/// Dark window glass with a faint diagonal sheen; `broken` knocks most panes out.
fn glass_px(x: usize, y: usize, broken: bool, salt: i32) -> Rgb {
    let d = (x + 64 - y % 64) % 24;
    let mut v = 0.35 + 0.25 * (noise(x, y, 8, 8, salt) - 0.5) + if d < 2 { 0.4 } else { 0.0 };
    if broken && noise(x, y, 4, 4, salt + 1) > 0.42 {
        v = 0.0;
    }
    ramp(&DARK, v, x, y)
}

/// A window of glass in a frame, `x0..x1` x `y0..y1`, a mullion and a transom, a sill under it.
fn window(t: &mut Tile, (x0, y0): (i32, i32), (x1, y1): (i32, i32), broken: bool, salt: i32) {
    t.rect((x0 - 2, y0 - 2), (x1 + 2, y1 + 2), |x, y| {
        ramp(&FRAME, 0.5 + 0.3 * (noise(x, y, 2, 2, salt) - 0.5), x, y)
    });
    t.rect((x0, y0), (x1, y1), |x, y| glass_px(x, y, broken, salt + 1));
    let mid = (x0 + x1) / 2;
    t.rect((mid - 1, y0), (mid + 1, y1), |_, _| FRAME[0]);
    let tr = y0 + (y1 - y0) / 3;
    t.rect((x0, tr), (x1, tr + 1), |_, _| FRAME[0]);
    // The sill, lit on top.
    t.rect((x0 - 3, y1 + 2), (x1 + 3, y1 + 4), |x, y| {
        if y as i32 == y1 + 2 {
            FRAME[2]
        } else {
            ramp(&CONCRETE, 0.3, x, y)
        }
    });
    if broken {
        // Shards left in the corners of the frame.
        for i in 0..4 {
            t.set(x0 + i, y0 + 3 - i, DARK[3]);
            t.set(x1 - 1 - i, y1 - 4 + i, DARK[3]);
        }
    }
}

/// Vertical bars every `step` pixels over `x0..x1` x `y0..y1`, rails top and bottom.
fn bars(t: &mut Tile, (x0, y0): (i32, i32), (x1, y1): (i32, i32), step: i32, salt: i32) {
    let bar = |x: usize, y: usize| ramp(&RUST, 0.3 + 0.5 * noise(x, y, 2, 8, salt), x, y);
    for x in (x0..x1).step_by(step as usize) {
        t.rect((x, y0), (x + 1, y1), bar);
    }
    t.rect((x0, y0), (x1, y0 + 2), bar);
    t.rect((x0, y1 - 2), (x1, y1), bar);
}

/// The facade units, in shader order: window, two narrow windows, caged window, enclosed balcony,
/// window with an air conditioner, boarded window, broken window, blank wall with a vent and a
/// pipe, shop shutter, shop window, blank wall.
fn facade_tiles() -> Vec<Tile> {
    let mut out = Vec::new();
    // 0: a window.
    let mut t = facade_wall(100);
    window(&mut t, (18, 12), (46, 46), false, 101);
    out.push(t);
    // 1: two narrow windows.
    let mut t = facade_wall(110);
    window(&mut t, (9, 14), (25, 46), false, 111);
    window(&mut t, (39, 14), (55, 46), false, 112);
    out.push(t);
    // 2: a window behind a burglar cage, a pot on its ledge.
    let mut t = facade_wall(120);
    window(&mut t, (18, 12), (46, 46), false, 121);
    bars(&mut t, (13, 8), (52, 52), 4, 122);
    t.rect((40, 46), (45, 51), |x, y| ramp(&BRICK, 0.6, x, y));
    t.rect((41, 43), (44, 46), |x, y| {
        ramp(&SOIL, 0.8 + 0.2 * hash(x as i32, y as i32, 123), x, y)
    });
    out.push(t);
    // 3: an enclosed balcony across the bay: a solid parapet below, a grille above, the dark room
    // behind with laundry hanging in it.
    let mut t = facade_wall(130);
    t.rect((2, 6), (62, 59), |x, y| {
        ramp(&DARK, 0.25 + 0.3 * noise(x, y, 8, 8, 131), x, y)
    });
    for (cx, w, c) in [
        (12, 6, [168, 60, 48]),
        (22, 5, [210, 204, 188]),
        (44, 7, [70, 110, 150]),
    ] {
        t.rect((cx, 14), (cx + w, 26), move |_, _| c);
    }
    t.rect((2, 13), (62, 14), |_, _| FRAME[0]);
    bars(&mut t, (2, 6), (62, 40), 3, 132);
    // The parapet stays wall (tinted), with a lit coping.
    for y in 40..59 {
        for x in 2..62 {
            let i = y * TILE + x;
            t.px[i] = ramp(&WALL, 0.45 + 0.25 * (noise(x, y, 8, 4, 133) - 0.5), x, y);
            t.wall[i] = true;
        }
    }
    t.rect((1, 39), (63, 41), |_, _| FRAME[2]);
    out.push(t);
    // 4: a window with an air conditioner under it, a rust drip below.
    let mut t = facade_wall(140);
    window(&mut t, (18, 10), (46, 40), false, 141);
    t.rect((22, 46), (42, 57), |x, y| {
        let v = if y % 2 == 0 && x > 24 && x < 36 {
            0.15
        } else {
            0.6 + 0.2 * (noise(x, y, 4, 4, 142) - 0.5)
        };
        ramp(&METAL, v, x, y)
    });
    t.rect((22, 46), (42, 47), |_, _| METAL[4]);
    t.rect((38, 57), (39, 64), |x, y| ramp(&RUST, 0.4, x, y));
    out.push(t);
    // 5: a window boarded up with planks.
    let mut t = facade_wall(150);
    window(&mut t, (18, 12), (46, 46), false, 151);
    t.rect((16, 12), (48, 46), |x, y| {
        let plank = y / 5;
        let v = if y % 5 == 0 {
            0.05
        } else {
            0.35 + 0.35 * hash(plank as i32, 0, 152) + 0.2 * (noise(x, y, 8, 1, 153) - 0.5)
        };
        ramp(&WOOD, v, x, y)
    });
    for y in [16, 41] {
        t.rect((14, y), (50, y + 3), |x, y| {
            ramp(&WOOD, 0.6 + 0.2 * (noise(x, y, 4, 1, 154) - 0.5), x, y)
        });
    }
    out.push(t);
    // 6: a broken window, soot above it.
    let mut t = facade_wall(160);
    for y in 0..12 {
        for x in 16..48 {
            if noise(x, y, 4, 4, 161) + y as f32 / 24.0 > 0.6 {
                t.px[y * TILE + x] = ramp(&WALL, 0.1, x, y);
            }
        }
    }
    window(&mut t, (18, 12), (46, 46), true, 162);
    out.push(t);
    // 7: blank wall, a vent grille and a pipe running down.
    let mut t = facade_wall(170);
    t.rect((26, 18), (38, 28), |x, y| {
        if y % 2 == 0 {
            DARK[1]
        } else {
            ramp(&METAL, 0.5, x, y)
        }
    });
    t.rect((54, 0), (57, 64), |x, y| {
        ramp(
            &METAL,
            if x == 54 {
                0.8
            } else {
                0.35 + 0.2 * noise(x, y, 1, 8, 171)
            },
            x,
            y,
        )
    });
    t.rect((53, 30), (58, 32), |_, _| METAL[0]);
    out.push(t);
    // 8: a shop, its shutter down: the sign band above stays wall (tinted), with unlit lettering.
    let mut t = facade_wall(180);
    t.rect((2, 20), (62, 64), |x, y| {
        let v = if y % 3 == 0 {
            0.15
        } else {
            0.55 + 0.25 * (noise(x, y, 16, 2, 181) - 0.5)
        };
        if noise(x, y, 4, 4, 182) * 0.6 + noise(x, y, 2, 2, 185) * 0.3 + (y as f32 / 64.0) * 0.3
            > 0.78
        {
            ramp(&RUST, 0.3 + 0.5 * noise(x, y, 2, 2, 183), x, y)
        } else {
            ramp(&METAL, v, x, y)
        }
    });
    t.rect((2, 18), (62, 20), |_, _| METAL[0]);
    letters(&mut t, 6, 184);
    out.push(t);
    // 9: a shop window, dark, its sign band above.
    let mut t = facade_wall(190);
    t.rect((2, 18), (62, 64), |x, y| ramp(&FRAME, 0.3, x, y));
    t.rect((5, 21), (59, 61), |x, y| glass_px(x, y, false, 191));
    t.rect((31, 21), (33, 61), |_, _| FRAME[0]);
    t.rect((5, 50), (31, 51), |_, _| FRAME[0]);
    letters(&mut t, 6, 192);
    out.push(t);
    // 10: blank wall.
    out.push(facade_wall(200));
    out
}

/// Unlit sign lettering in the sign band (rows `y0..y0 + 8`): blocky glyphs of dark paint.
fn letters(t: &mut Tile, y0: i32, salt: i32) {
    let n = 4 + (hash(0, 0, salt) * 3.0) as i32;
    let w = 5;
    let x0 = (64 - n * (w + 2)) / 2;
    for i in 0..n {
        for gy in 0..4 {
            for gx in 0..3 {
                if hash(i * 8 + gx, gy, salt + 1) > 0.45 {
                    let (x, y) = (x0 + i * (w + 2) + gx * 2 - 1, y0 + gy * 2);
                    t.rect((x, y), (x + 2, y + 2), |_, _| [40, 34, 32]);
                }
            }
        }
    }
}

/// All tiles side by side, RGBA, in `Mat::index` order, then the facade units. Alpha marks the
/// facade's wall pixels.
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
    ]
    .into_iter()
    .chain(facade_tiles())
    .collect::<Vec<_>>();
    let w = TILE * COUNT;
    let mut rgba = vec![255u8; w * TILE * 4];
    for (i, t) in tiles.iter().enumerate() {
        for y in 0..TILE {
            for x in 0..TILE {
                let c = t.px[y * TILE + x];
                let o = (y * w + i * TILE + x) * 4;
                rgba[o..o + 3].copy_from_slice(&c);
                rgba[o + 3] = if t.wall[y * TILE + x] { 255 } else { 0 };
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
