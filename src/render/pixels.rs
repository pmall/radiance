//! Pixel-art textures of the chunk materials (`world::chunks::Mat`, `docs/chunks.md`), of the
//! tower facade units and of the city's own surfaces (roads, sidewalks, roofs, deck undersides):
//! one tile of 4 m at 32 pixels per meter each, drawn here by code (limited palettes, ordered
//! dithering, chunky joints, cracks, rivets and stains) and sampled with crisp nearest filtering
//! by `atlasTile` in `shaders/scene.fs`. Tiles wrap seamlessly. A last tile holds swatches: the
//! colors the shader's wear (dust, litter, moss, puddles) is drawn in.
//!
//! Every biome draws its own row of tiles from its `Textures` (`Biome::textures`): the shades of
//! each material, the patterns that come in several kinds (slabs or setts, plaster or tiled walls,
//! square or arched windows, plain or glazed bricks) and the wear (moss, damp, corrosion), so the
//! same chunk reads differently from one biome to the next. Fixed, not seeded: the seed picks where
//! materials go, not how they look.

use sola_raylib::ffi;

use crate::rng::{Rng, hash_coords};
use crate::world::biome::{BIOMES, MAX_BIOMES};

/// Side of a material tile in pixels (4 m, 32 pixels per meter).
pub const TILE: usize = 128;
/// Chunk materials, in `Mat::index` order.
pub const MATERIALS: usize = 11;
/// Tower facade units (`facade_tiles`), after the materials: 4 m bays one story (4 m) high.
pub const FACADES: usize = 14;
/// The city's surfaces (`surface_tiles`), after the facade units: road, sidewalk, roof, deck
/// underside.
pub const SURFACES: usize = 4;
/// Tiles in a row of the atlas: materials, facade units, surfaces, and the swatches last.
pub const COUNT: usize = MATERIALS + FACADES + SURFACES + 1;
/// Rows of the atlas: one per biome (`biome::variant_of`), room for all of them.
pub const ROWS: usize = MAX_BIOMES;
/// Material slot (sampler `texture10`) of the atlas.
pub const SLOT: usize = 10;

/// An sRGB color.
pub type Rgb = [u8; 3];
/// Five shades of a material, dark to light.
pub type Ramp = [Rgb; 5];

/// How a biome's materials and facade units look (`Biome::textures`; start a new one from
/// `..DEFAULT_TEXTURES`). Pattern sizes are in pixels, 32 to the meter, and must divide `TILE`.
#[derive(Clone, Copy)]
pub struct Textures {
    // The shades of each material.
    pub stone: Ramp,
    pub stone_dark: Ramp,
    pub concrete: Ramp,
    pub brick: Ramp,
    /// Joints between bricks and setts.
    pub mortar: Ramp,
    pub tile: Ramp,
    pub metal: Ramp,
    /// What metal corrodes to (rust, verdigris), on metal, bars, shutters and drips.
    pub corrosion: Ramp,
    pub wood: Ramp,
    pub glass: Ramp,
    pub soil: Ramp,
    /// Moss and algae, where `mossy` and `damp` let them grow.
    pub moss: Ramp,
    /// Window frames and sills, dark to light, and window panes, dark to light.
    pub frame: [Rgb; 3],
    pub pane: [Rgb; 4],
    /// Laundry hanging in the enclosed balconies.
    pub laundry: [Rgb; 3],
    /// Road asphalt, and roof tar.
    pub asphalt: Ramp,
    /// Dust, sand and grit settled on the ground.
    pub dust: Ramp,
    /// Dead leaves and scraps of paper lying about.
    pub litter: Ramp,
    /// Standing water, dark to light (the shader adds what it reflects).
    pub water: Ramp,

    /// How stone and dark stone are laid.
    pub paving: (Paving, Paving),
    /// Roads: asphalt with lane markings, or stone without.
    pub road: Road,
    /// How sidewalks are laid, in concrete shades.
    pub sidewalk: Paving,
    /// Brick size (width, height), and whether they are glazed (a sheen, no soot).
    pub bricks: (usize, usize),
    pub glazed: bool,
    /// Side of a tile, and how much tiles differ in shade (0.3 a plain floor, 1 a mosaic).
    pub tiles: usize,
    pub mosaic: f32,
    /// How corroded `Metal` and `Rust` are, 0 to 1 (0.45 is well rusted).
    pub corroded: (f32, f32),
    /// Tower walls, plaster or tiled, and whether their windows are round-headed.
    pub wall: Wall,
    pub arched: bool,

    /// Moss in joints, on ledges and sills, in patches on soil: 0 none, 1 thick.
    pub mossy: f32,
    /// Water instead of rain: tide marks, algae running down walls, mist and beads on glass. 0
    /// none, 1 soaked.
    pub damp: f32,
    /// Wear the shader draws over every surface (`groundWear`, `wallWear` in `shaders/scene.fs`),
    /// 0 none to 1 thick: dust and sand drifts, leaves and litter, puddles.
    pub dusty: f32,
    pub littered: f32,
    pub puddled: f32,
}

/// How a road is surfaced.
#[derive(Clone, Copy)]
pub enum Road {
    /// Asphalt, patched and cracked, with lane markings.
    Asphalt,
    /// Dark stone laid this way, no markings.
    Stone(Paving),
}

/// How a stone floor is laid.
#[derive(Clone, Copy)]
pub enum Paving {
    /// Square slabs of this side, joints between them.
    Slabs(usize),
    /// Domed setts of this side in courses offset by half a sett.
    Setts(usize),
}

/// What a tower wall is faced with (tinted by the tower's paint, `Palette::paints`).
#[derive(Clone, Copy)]
pub enum Wall {
    /// Stained plaster.
    Plaster,
    /// Square tiles of this side, grout between them.
    Tiled(usize),
}

/// The original city's materials: worn stone and concrete, red brick, rusting steel.
pub const DEFAULT_TEXTURES: Textures = Textures {
    stone: [
        [70, 66, 62],
        [104, 98, 90],
        [138, 130, 118],
        [166, 158, 142],
        [192, 184, 166],
    ],
    stone_dark: [
        [34, 34, 38],
        [52, 52, 56],
        [72, 70, 72],
        [94, 92, 92],
        [116, 112, 110],
    ],
    concrete: [
        [62, 64, 66],
        [88, 90, 90],
        [112, 114, 112],
        [136, 138, 134],
        [160, 160, 154],
    ],
    brick: [
        [64, 30, 26],
        [96, 44, 34],
        [128, 60, 44],
        [154, 78, 56],
        [176, 100, 74],
    ],
    mortar: [
        [34, 34, 38],
        [52, 52, 56],
        [72, 70, 72],
        [94, 92, 92],
        [116, 112, 110],
    ],
    tile: [
        [70, 84, 84],
        [110, 128, 124],
        [150, 166, 158],
        [184, 196, 184],
        [210, 216, 202],
    ],
    metal: [
        [40, 44, 52],
        [62, 68, 78],
        [86, 94, 104],
        [112, 120, 130],
        [140, 148, 156],
    ],
    corrosion: [
        [66, 32, 20],
        [104, 50, 26],
        [140, 72, 34],
        [172, 100, 48],
        [196, 130, 70],
    ],
    wood: [
        [58, 38, 26],
        [84, 56, 36],
        [110, 76, 48],
        [136, 98, 62],
        [160, 120, 80],
    ],
    glass: [
        [18, 26, 34],
        [28, 40, 50],
        [42, 58, 70],
        [64, 84, 96],
        [110, 134, 144],
    ],
    soil: [
        [30, 22, 16],
        [46, 34, 24],
        [64, 48, 32],
        [84, 64, 42],
        [104, 84, 58],
    ],
    moss: [
        [30, 52, 30],
        [44, 74, 40],
        [62, 98, 50],
        [86, 122, 62],
        [114, 146, 80],
    ],
    frame: [[92, 92, 88], [168, 166, 158], [206, 204, 196]],
    pane: [[8, 10, 14], [14, 18, 24], [22, 28, 36], [34, 42, 52]],
    laundry: [[168, 60, 48], [210, 204, 188], [70, 110, 150]],
    asphalt: [
        [34, 36, 40],
        [50, 52, 56],
        [68, 69, 72],
        [88, 88, 88],
        [116, 112, 106],
    ],
    dust: [
        [74, 66, 54],
        [98, 88, 70],
        [124, 112, 90],
        [150, 136, 108],
        [174, 160, 128],
    ],
    litter: [
        [70, 42, 24],
        [112, 70, 32],
        [138, 100, 52],
        [104, 66, 44],
        [196, 188, 168],
    ],
    water: [
        [10, 14, 20],
        [18, 24, 32],
        [28, 36, 46],
        [44, 54, 64],
        [70, 82, 92],
    ],
    paving: (Paving::Slabs(32), Paving::Slabs(16)),
    road: Road::Asphalt,
    sidewalk: Paving::Slabs(32),
    bricks: (16, 8),
    glazed: false,
    tiles: 16,
    mosaic: 0.3,
    corroded: (0.0, 0.45),
    wall: Wall::Plaster,
    arched: false,
    mossy: 0.3,
    damp: 0.15,
    dusty: 0.6,
    littered: 0.5,
    puddled: 0.35,
};

/// Cyan Grotto: an old bathhouse district gone cold and wet. Pale limestone and basalt setts with
/// moss in every joint, glazed teal bricks, a mosaic of aqua and white tiles, verdigris for rust,
/// silvered driftwood; tiled towers with round-headed windows misted over, algae running down from
/// the ledges.
pub const GROTTO_TEXTURES: Textures = Textures {
    stone: [
        [62, 72, 76],
        [92, 104, 106],
        [124, 136, 134],
        [152, 164, 160],
        [180, 190, 184],
    ],
    stone_dark: [
        [24, 30, 36],
        [38, 46, 52],
        [54, 64, 70],
        [72, 84, 88],
        [94, 106, 108],
    ],
    concrete: [
        [54, 62, 68],
        [78, 88, 92],
        [102, 112, 114],
        [126, 136, 136],
        [150, 158, 156],
    ],
    brick: [
        [18, 54, 60],
        [28, 80, 84],
        [42, 108, 108],
        [68, 138, 132],
        [126, 186, 176],
    ],
    mortar: [
        [30, 36, 40],
        [46, 54, 58],
        [66, 74, 76],
        [88, 96, 96],
        [110, 116, 114],
    ],
    tile: [
        [36, 78, 86],
        [66, 128, 134],
        [116, 176, 178],
        [172, 212, 206],
        [222, 236, 228],
    ],
    metal: [
        [34, 42, 50],
        [52, 64, 74],
        [74, 88, 98],
        [100, 114, 122],
        [132, 146, 152],
    ],
    corrosion: [
        [28, 66, 62],
        [44, 100, 90],
        [68, 134, 114],
        [104, 166, 138],
        [150, 200, 170],
    ],
    wood: [
        [44, 46, 46],
        [66, 70, 68],
        [90, 94, 90],
        [116, 120, 112],
        [144, 146, 136],
    ],
    glass: [
        [14, 32, 38],
        [22, 48, 56],
        [34, 68, 76],
        [56, 96, 104],
        [110, 150, 156],
    ],
    soil: [
        [18, 20, 18],
        [28, 32, 28],
        [40, 46, 38],
        [54, 62, 50],
        [70, 80, 64],
    ],
    moss: [
        [18, 52, 46],
        [28, 76, 62],
        [42, 100, 78],
        [62, 126, 94],
        [96, 156, 116],
    ],
    frame: [[64, 80, 84], [132, 152, 152], [186, 204, 202]],
    pane: [[6, 12, 16], [10, 20, 26], [16, 32, 40], [30, 52, 62]],
    laundry: [[214, 222, 222], [96, 168, 178], [40, 66, 106]],
    asphalt: [
        [16, 22, 26],
        [26, 34, 38],
        [38, 48, 52],
        [54, 64, 68],
        [78, 88, 90],
    ],
    dust: [
        [58, 70, 68],
        [80, 94, 90],
        [104, 118, 112],
        [130, 142, 134],
        [156, 166, 156],
    ],
    litter: [
        [30, 56, 46],
        [48, 84, 64],
        [76, 110, 80],
        [110, 96, 70],
        [200, 214, 208],
    ],
    water: [
        [6, 18, 22],
        [10, 30, 36],
        [18, 46, 52],
        [32, 68, 74],
        [60, 104, 108],
    ],
    paving: (Paving::Slabs(32), Paving::Setts(16)),
    road: Road::Stone(Paving::Setts(16)),
    sidewalk: Paving::Slabs(16),
    bricks: (16, 8),
    glazed: true,
    tiles: 8,
    mosaic: 0.9,
    corroded: (0.2, 0.55),
    wall: Wall::Tiled(16),
    arched: true,
    mossy: 0.7,
    damp: 0.8,
    dusty: 0.15,
    littered: 0.3,
    puddled: 0.9,
};

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

/// Moss at a pixel, if the biome grows it there (`Textures::mossy`): `room` is how welcoming the
/// spot is, near 1 in joints and on ledges (moss in most of them), lower on open faces (patches).
fn moss(s: &Textures, x: usize, y: usize, room: f32, salt: i32) -> Option<Rgb> {
    if s.mossy <= 0.0 {
        return None;
    }
    let n = 0.45 * noise(x, y, 64, 64, salt)
        + 0.35 * noise(x, y, 16, 16, salt + 1)
        + 0.2 * noise(x, y, 4, 4, salt + 3);
    (n + room * s.mossy > 1.2 - BAYER[y % 4][x % 4] / 40.0)
        .then(|| ramp(&s.moss, 0.2 + 0.7 * noise(x, y, 8, 8, salt + 2), x, y))
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
        self.area((x0, y0), (x1, y1), |_, _| true, f);
    }

    /// Paints the pixels of the rectangle `x0..x1` x `y0..y1` that `keep` accepts (not wall).
    fn area(
        &mut self,
        (x0, y0): (i32, i32),
        (x1, y1): (i32, i32),
        keep: impl Fn(i32, i32) -> bool,
        f: impl Fn(usize, usize) -> Rgb,
    ) {
        for y in y0.max(0)..y1.min(TILE as i32) {
            for x in x0.max(0)..x1.min(TILE as i32) {
                if keep(x, y) {
                    let i = y as usize * TILE + x as usize;
                    self.px[i] = f(x as usize, y as usize);
                    self.wall[i] = false;
                }
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

    /// Moss over rows `y0..y1` wherever it takes (`moss`), not wall.
    fn moss(&mut self, s: &Textures, (y0, y1): (usize, usize), room: f32, salt: i32) {
        for y in y0..y1.min(TILE) {
            for x in 0..TILE {
                if let Some(c) = moss(s, x, y, room, salt) {
                    self.px[y * TILE + x] = c;
                    self.wall[y * TILE + x] = false;
                }
            }
        }
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

/// Gray shades of painted surfaces and of tower walls, which the shader tints (the box's color,
/// the tower's paint).
const PAINT: [Rgb; 5] = [
    [120, 120, 120],
    [168, 168, 168],
    [200, 200, 200],
    [226, 226, 226],
    [246, 246, 246],
];

/// A stone floor in shades `pal`, laid as `how` says.
fn paving(s: &Textures, pal: &Ramp, how: Paving, salt: i32) -> Tile {
    match how {
        Paving::Slabs(side) => slabs(s, pal, side, salt),
        Paving::Setts(side) => setts(s, pal, side, salt),
    }
}

/// Paving slabs of `slab` pixels: a shade per slab, dark joints, a lit edge, specks and cracks.
fn slabs(s: &Textures, pal: &Ramp, slab: usize, salt: i32) -> Tile {
    let mut t = Tile::new();
    t.fill(|x, y| {
        let (sx, sy) = ((x / slab) as i32, (y / slab) as i32);
        if x % slab == 0 || y % slab == 0 {
            return moss(s, x, y, 1.0, salt + 5).unwrap_or(pal[0]);
        }
        if let Some(m) = moss(s, x, y, 0.4, salt + 5) {
            return m;
        }
        let mut v = 0.35 + 0.3 * hash(sx, sy, salt);
        v += 0.25 * (noise(x, y, 16, 16, salt + 1) - 0.5)
            + 0.2 * (noise(x, y, 2, 2, salt + 2) - 0.5);
        if x % slab == 1 || y % slab == 1 {
            v += 0.15;
        }
        if hash(x as i32, y as i32, salt + 3) > 0.97 {
            v -= 0.3;
        }
        ramp(pal, v, x, y)
    });
    t.cracks(5, 28, pal[0], salt as u64);
    t
}

/// Setts of `sett` pixels in courses offset by half a sett: a shade per sett, domed (lit on top
/// and left, dark at the bottom and right, two pixels each), corners cut round, mortar or moss
/// between.
fn setts(s: &Textures, pal: &Ramp, sett: usize, salt: i32) -> Tile {
    let mut t = Tile::new();
    t.fill(|x, y| {
        let course = y / sett;
        let xo = (x + if course % 2 == 1 { sett / 2 } else { 0 }) % TILE;
        let (u, v) = (xo % sett, y % sett);
        let edge = |a: usize| a.min(sett - a);
        if u == 0 || v == 0 || edge(u) + edge(v) <= 3 {
            return moss(s, x, y, 1.0, salt + 5)
                .unwrap_or_else(|| ramp(&s.mortar, 0.15 + 0.3 * noise(x, y, 8, 8, salt), x, y));
        }
        let mut sh = 0.3 + 0.4 * hash((xo / sett) as i32, course as i32, salt + 1);
        sh += 0.15 * (noise(x, y, 2, 2, salt + 2) - 0.5);
        let dome = |a: usize| -> f32 {
            match a {
                1 => 0.2,
                2 => 0.1,
                a if a == sett - 1 => -0.2,
                a if a == sett - 2 => -0.1,
                _ => 0.0,
            }
        };
        sh += dome(u).max(dome(v)).max(0.0) + dome(u).min(dome(v)).min(0.0);
        ramp(pal, sh, x, y)
    });
    t.cracks(3, 16, pal[0], salt as u64);
    t
}

fn concrete(s: &Textures) -> Tile {
    let mut t = Tile::new();
    t.fill(|x, y| {
        let mut v =
            0.5 + 0.35 * (noise(x, y, 32, 32, 20) - 0.5) + 0.25 * (noise(x, y, 8, 8, 21) - 0.5);
        // Rain streaks running down from some columns.
        let streak = hash(x as i32, 0, 22);
        if streak > 0.8 {
            v -= 0.25 * (y as f32 / TILE as f32) * (noise(x, y, 1, 16, 23) + 0.3);
        }
        // Form-tie holes every meter, a lit lip under each.
        if (14..18).contains(&(x % 32)) && (15..18).contains(&(y % 32)) {
            v = if y % 32 == 17 { 0.7 } else { 0.0 };
        }
        // Formwork joints, moss in them.
        if y % 64 == 0 {
            if let Some(m) = moss(s, x, y, 0.9, 29) {
                return m;
            }
            v -= 0.2;
        }
        if s.damp > 0.0 {
            // Algae running down from the formwork joints.
            let run = 1.0 - (y % 64) as f32 / 48.0;
            if hash(x as i32, 0, 26) > 1.0 - 0.3 * s.damp && noise(x, y, 1, 16, 27) < run {
                return ramp(&s.moss, 0.15 + 0.4 * noise(x, y, 4, 8, 28), x, y);
            }
            // A tide mark: a dark line, a pale line of salt over it.
            let d = y as f32 - (40.0 + 10.0 * noise(x, 0, 32, 1, 25));
            if (0.0..2.0).contains(&d) {
                v -= 0.3 * s.damp;
            } else if (-1.0..0.0).contains(&d) {
                v += 0.2 * s.damp;
            }
        }
        ramp(&s.concrete, v, x, y)
    });
    t.cracks(3, 20, s.concrete[0], 24);
    t
}

fn brick(s: &Textures) -> Tile {
    let (bw, bh) = s.bricks;
    let mut t = Tile::new();
    t.fill(|x, y| {
        let row = y / bh;
        let xo = x + if row % 2 == 1 { bw / 2 } else { 0 };
        let (bx, by) = ((xo / bw) as i32, row as i32);
        if y % bh == bh - 1 || xo % bw == bw - 1 {
            // Mortar, or moss in it.
            if let Some(m) = moss(s, x, y, 1.0, 35) {
                return m;
            }
            let m = 0.3 + 0.3 * noise(x, y, 8, 8, 30);
            return ramp(&s.mortar, m + 0.3, x, y);
        }
        let mut v = 0.3 + 0.45 * hash(bx, by, 31);
        if hash(bx, by, 32) > 0.9 {
            v -= 0.3;
        }
        v += 0.15 * (noise(x, y, 2, 2, 33) - 0.5);
        if y % bh == 0 {
            v += 0.12;
        }
        if s.glazed {
            // A sheen along the top of each brick, a glint at its corner.
            if y % bh == 0 && xo % bw < bw / 2 {
                v += 0.2;
            }
            if y % bh == 0 && xo % bw == 1 {
                v = 1.0;
            }
        } else {
            // Soot and damp, in large patches.
            v -= 0.3 * (noise(x, y, 64, 64, 34) - 0.4).max(0.0);
        }
        ramp(&s.brick, v, x, y)
    });
    t
}

fn tiles(s: &Textures) -> Tile {
    let n = s.tiles;
    let mut t = Tile::new();
    t.fill(|x, y| {
        let (tx, ty) = ((x / n) as i32, (y / n) as i32);
        if x % n == 0 || y % n == 0 {
            return moss(s, x, y, 0.9, 45).unwrap_or(s.tile[0]);
        }
        // A few tiles missing: bare concrete.
        if hash(tx, ty, 40) > 0.94 {
            return ramp(&s.concrete, 0.3 + 0.3 * noise(x, y, 2, 2, 41), x, y);
        }
        let mut v =
            0.55 + s.mosaic * (hash(tx, ty, 42) - 0.5) + 0.15 * (noise(x, y, 8, 8, 43) - 0.5);
        if x % n == 1 || y % n == 1 {
            v += 0.15;
        }
        ramp(&s.tile, v, x, y)
    });
    t.cracks(4, 18, s.tile[0], 44);
    t
}

fn metal(s: &Textures, rust: f32) -> Tile {
    let mut t = Tile::new();
    t.fill(|x, y| {
        let mut v =
            0.5 + 0.3 * (noise(x, y, 32, 1, 50) - 0.5) + 0.15 * (noise(x, y, 2, 2, 51) - 0.5);
        // Panels with seams, lit under each seam, rivets along them: domes of two by two
        // pixels, lit at the top left.
        if y % 32 == 0 || x % 64 == 0 {
            v = 0.05;
        } else if y % 32 == 1 || x % 64 == 1 {
            v += 0.2;
        }
        let (ry, rx) = (y % 32, x % 16);
        if (ry == 4 || ry == 5 || ry == 27 || ry == 28) && (rx == 8 || rx == 9) {
            v = if (ry == 4 || ry == 27) && rx == 8 {
                1.0
            } else {
                0.25
            };
        }
        let m = ramp(&s.metal, v, x, y);
        let r = noise(x, y, 16, 16, 52) * 0.6
            + noise(x, y, 2, 2, 53) * 0.4
            + (y as f32 / TILE as f32) * 0.2;
        if r + rust - 0.5 > 0.62 - BAYER[y % 4][x % 4] / 40.0 {
            ramp(&s.corrosion, 0.3 + 0.6 * noise(x, y, 8, 8, 54), x, y)
        } else {
            m
        }
    });
    t
}

fn wood(s: &Textures) -> Tile {
    let mut t = Tile::new();
    t.fill(|x, y| {
        let plank = (y / 8) as i32;
        if y % 8 == 0 {
            return moss(s, x, y, 0.8, 64).unwrap_or(s.wood[0]);
        }
        let shift = (hash(plank, 0, 60) * TILE as f32) as usize;
        let mut v = 0.35 + 0.3 * hash(plank, 1, 61);
        v += 0.35 * (noise((x + shift) % TILE, y, 32, 1, 62) - 0.5);
        if y % 8 == 1 {
            v += 0.12;
        }
        // Plank ends and knots.
        if (x + shift).is_multiple_of(64) {
            v = 0.0;
        }
        if hash(x as i32, y as i32, 63) > 0.985 {
            v -= 0.35;
        }
        ramp(&s.wood, v, x, y)
    });
    t
}

fn glass(s: &Textures) -> Tile {
    let mut t = Tile::new();
    t.fill(|x, y| {
        let mut v = 0.25 + 0.2 * (y as f32 / TILE as f32) + 0.1 * (noise(x, y, 16, 16, 70) - 0.5);
        // Diagonal reflections.
        let d = (x + TILE - y % TILE) % 64;
        if d < 6 {
            v += 0.45;
        } else if d < 10 {
            v += 0.2;
        }
        // Grime at the foot.
        if y > 104 {
            v -= 0.15 * noise(x, y, 8, 4, 71);
        }
        if s.damp > 0.0 {
            // Mist gathering low on the pane, beads of water.
            v += s.damp * 0.35 * (y as f32 / TILE as f32) * noise(x, y, 8, 8, 72);
            if hash(x as i32, y as i32, 73) > 1.0 - 0.03 * s.damp {
                v = 1.0;
            }
        }
        ramp(&s.glass, v, x, y)
    });
    t
}

fn soil(s: &Textures) -> Tile {
    let mut t = Tile::new();
    t.fill(|x, y| {
        if let Some(m) = moss(s, x, y, 0.7, 85) {
            return m;
        }
        let v = 0.45 + 0.4 * (noise(x, y, 8, 8, 80) - 0.5) + 0.3 * (noise(x, y, 2, 2, 81) - 0.5);
        if hash(x as i32, y as i32, 82) > 0.97 {
            return ramp(&s.stone, 0.5 + 0.4 * hash(x as i32, y as i32, 83), x, y);
        }
        ramp(&s.soil, v, x, y)
    });
    t.cracks(6, 24, s.soil[0], 84);
    t
}

fn paint(s: &Textures) -> Tile {
    let mut t = Tile::new();
    t.fill(|x, y| {
        let mut v =
            0.75 + 0.2 * (noise(x, y, 32, 32, 90) - 0.5) + 0.1 * (noise(x, y, 2, 2, 91) - 0.5);
        // Chipped paint showing the primer.
        if noise(x, y, 8, 8, 92) * 0.7 + noise(x, y, 2, 2, 93) * 0.3 > 0.72 {
            v = 0.15;
        }
        // Grime streaks running down.
        if hash(x as i32, 0, 94) > 0.85 {
            v -= 0.25 * (y as f32 / TILE as f32);
        }
        if s.damp > 0.0 && v > 0.15 {
            // A tide mark, water stains under it.
            let d = y as f32 - (80.0 + 12.0 * noise(x, 0, 32, 1, 95));
            if (0.0..2.0).contains(&d) {
                v -= 0.3 * s.damp;
            } else if d > 0.0 {
                v -= 0.15 * s.damp * noise(x, y, 8, 16, 96);
            }
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

/// Shade of a tiled wall's pixel, tiles of `n`: dark grout, a shade per tile, its top and left
/// edges lit.
fn tiled(n: usize, x: usize, y: usize, salt: i32) -> f32 {
    if x.is_multiple_of(n) || y.is_multiple_of(n) {
        return 0.3;
    }
    let lit = if x % n == 1 || y % n == 1 { 0.08 } else { 0.0 };
    0.6 + 0.25 * (hash((x / n) as i32, (y / n) as i32, salt) - 0.5)
        + 0.15 * (noise(x, y, 32, 32, salt + 1) - 0.5)
        + lit
}

/// A wall tile: stained, dithered pale gray (tinted by the shader), plaster or tiles, a slab
/// ledge at the foot of the story with a drip line under it; in a damp biome algae running down
/// from the ledge above, moss on its own.
fn facade_wall(s: &Textures, salt: i32) -> Tile {
    let mut t = Tile::new();
    t.fill(|x, y| {
        let mut v = match s.wall {
            Wall::Plaster => {
                0.55 + 0.3 * (noise(x, y, 32, 32, salt) - 0.5)
                    + 0.2 * (noise(x, y, 8, 8, salt + 1) - 0.5)
            }
            Wall::Tiled(n) => tiled(n, x, y, salt),
        };
        // Grime gathering under the ledge above, rain streaks down some columns.
        v -= 0.25 * (1.0 - y as f32 / 20.0).max(0.0);
        if hash(x as i32, 0, salt + 2) > 0.82 {
            v -= 0.22 * noise(x, y, 1, 32, salt + 3);
        }
        ramp(&WALL, v, x, y)
    });
    t.wall.iter_mut().for_each(|w| *w = true);
    if s.damp > 0.0 {
        for x in 0..TILE {
            if hash(x as i32, 0, salt + 5) < 1.0 - 0.18 * s.damp {
                continue;
            }
            let len = 20.0 + 72.0 * hash(x as i32, 1, salt + 6);
            for y in 0..118 {
                let run = 1.0 - y as f32 / len;
                if noise(x, y, 1, 8, salt + 7) < run {
                    t.px[y * TILE + x] = ramp(&s.moss, 0.15 + 0.45 * run, x, y);
                    t.wall[y * TILE + x] = false;
                }
            }
        }
    }
    // The slab ledge: concrete, lit on top, dark under; moss on it.
    t.rect((0, 118), (128, 128), |x, y| {
        let v = match y {
            118 => 0.95,
            126 => 0.3,
            127 => 0.1,
            _ => 0.55 + 0.25 * (noise(x, y, 8, 1, salt + 4) - 0.5),
        };
        ramp(&s.concrete, v, x, y)
    });
    t.moss(s, (114, 118), 0.6, salt + 8);
    t.moss(s, (118, 122), 1.0, salt + 8);
    t.cracks(2, 16, WALL[0], salt as u64);
    t
}

/// Window glass, dark with a faint diagonal sheen; `broken` knocks most panes out. Damp mists it
/// over low down and beads it.
fn glass_px(s: &Textures, x: usize, y: usize, broken: bool, salt: i32) -> Rgb {
    let d = (x + TILE - y % TILE) % 48;
    let mut v = 0.35 + 0.25 * (noise(x, y, 16, 16, salt) - 0.5) + if d < 4 { 0.4 } else { 0.0 };
    if broken && noise(x, y, 8, 8, salt + 1) > 0.42 {
        v = 0.0;
    } else if s.damp > 0.0 {
        v +=
            s.damp * 0.6 * ((y as f32 - 32.0) / 64.0).clamp(0.0, 1.0) * noise(x, y, 8, 4, salt + 2);
        if hash(x as i32, y as i32, salt + 3) > 1.0 - 0.04 * s.damp {
            v = 1.0;
        }
    }
    ramp(&s.pane, v, x, y)
}

/// A window of glass in a frame, `x0..x1` x `y0..y1`, a sill under it: square with a mullion and
/// a transom, or (`Textures::arched`) round-headed under a keystone, the transom where the arch
/// springs. Moss takes the sill where the biome grows it.
fn window(
    s: &Textures,
    t: &mut Tile,
    (x0, y0): (i32, i32),
    (x1, y1): (i32, i32),
    broken: bool,
    salt: i32,
) {
    // The arch: a half circle over the opening, its center `r` under the head.
    let r = (x1 - x0) as f32 * 0.5;
    let (cx, cy) = ((x0 + x1) as f32 * 0.5, y0 as f32 + r);
    let inside = |grow: f32| {
        move |x: i32, y: i32| {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            !s.arched || py >= cy || (px - cx).powi(2) + (py - cy).powi(2) <= (r + grow).powi(2)
        }
    };
    t.area((x0 - 4, y0 - 4), (x1 + 4, y1 + 4), inside(4.0), |x, y| {
        ramp(&s.frame, 0.5 + 0.3 * (noise(x, y, 4, 4, salt) - 0.5), x, y)
    });
    t.area((x0, y0), (x1, y1), inside(0.0), |x, y| {
        glass_px(s, x, y, broken, salt + 1)
    });
    let mid = (x0 + x1) / 2;
    t.area((mid - 1, y0), (mid + 1, y1), inside(0.0), |_, _| s.frame[0]);
    let tr = if s.arched {
        cy as i32
    } else {
        y0 + (y1 - y0) / 3
    };
    t.rect((x0, tr), (x1, tr + 2), |_, _| s.frame[0]);
    if s.arched {
        t.rect((mid - 4, y0 - 6), (mid + 4, y0), |x, y| {
            ramp(&s.frame, if y as i32 == y0 - 6 { 1.0 } else { 0.6 }, x, y)
        });
    }
    // The sill, lit on top, shadowed under.
    t.rect((x0 - 6, y1 + 4), (x1 + 6, y1 + 8), |x, y| {
        match y as i32 - y1 {
            4 => s.frame[2],
            7 => ramp(&s.concrete, 0.1, x, y),
            _ => ramp(&s.concrete, 0.3, x, y),
        }
    });
    if s.mossy > 0.0 {
        for x in x0 - 6..x1 + 6 {
            if let Some(c) = moss(s, x as usize, (y1 + 4) as usize, 1.0, salt + 5) {
                t.set(x, y1 + 4, c);
                // Now and then a strand hanging under the sill.
                if hash(x, y1, salt + 6) > 0.7 {
                    let len = 1 + (hash(x, y1, salt + 7) * 4.0) as i32;
                    for y in y1 + 8..y1 + 8 + len {
                        t.set(x, y, c);
                    }
                }
            }
        }
    }
    if broken {
        // Shards left in the corners of the frame.
        let top = if s.arched { cy as i32 } else { y0 };
        for i in 0..8 {
            t.set(x0 + i, top + 6 - i, s.pane[3]);
            t.set(x1 - 1 - i, y1 - 8 + i, s.pane[3]);
        }
    }
}

/// Vertical bars every `step` pixels over `x0..x1` x `y0..y1`, two pixels thick and lit on the
/// left, rails top and bottom.
fn bars(
    s: &Textures,
    t: &mut Tile,
    (x0, y0): (i32, i32),
    (x1, y1): (i32, i32),
    step: i32,
    salt: i32,
) {
    let bar = |x: usize, y: usize| ramp(&s.corrosion, 0.3 + 0.5 * noise(x, y, 4, 16, salt), x, y);
    let lit = |x: usize, y: usize| ramp(&s.corrosion, 0.6 + 0.4 * noise(x, y, 4, 16, salt), x, y);
    for x in (x0..x1).step_by(step as usize) {
        t.rect((x, y0), (x + 1, y1), lit);
        t.rect((x + 1, y0), (x + 2, y1), bar);
    }
    t.rect((x0, y0), (x1, y0 + 4), bar);
    t.rect((x0, y0), (x1, y0 + 1), lit);
    t.rect((x0, y1 - 4), (x1, y1), bar);
    t.rect((x0, y1 - 4), (x1, y1 - 3), lit);
}

/// The facade units, in shader order: window, two narrow windows, caged window, enclosed balcony,
/// window with an air conditioner, boarded window, broken window, blank wall with a vent and a
/// pipe, shop shutter, shop window, blank wall, curtain wall, curtain wall with panes gone,
/// ribbon window.
fn facade_tiles(s: &Textures) -> Vec<Tile> {
    let mut out = Vec::new();
    // 0: a window.
    let mut t = facade_wall(s, 100);
    window(s, &mut t, (36, 24), (92, 92), false, 101);
    out.push(t);
    // 1: two narrow windows.
    let mut t = facade_wall(s, 110);
    window(s, &mut t, (18, 28), (50, 92), false, 111);
    window(s, &mut t, (78, 28), (110, 92), false, 112);
    out.push(t);
    // 2: a window behind a burglar cage, a pot on its ledge.
    let mut t = facade_wall(s, 120);
    window(s, &mut t, (36, 24), (92, 92), false, 121);
    bars(s, &mut t, (26, 16), (104, 104), 8, 122);
    t.rect((80, 92), (90, 102), |x, y| {
        ramp(&s.brick, if x == 80 { 0.85 } else { 0.55 }, x, y)
    });
    t.rect((82, 86), (88, 92), |x, y| {
        ramp(&s.soil, 0.8 + 0.2 * hash(x as i32, y as i32, 123), x, y)
    });
    out.push(t);
    // 3: an enclosed balcony across the bay: a solid parapet below, a grille above, the dark room
    // behind with laundry hanging in it.
    let mut t = facade_wall(s, 130);
    t.rect((4, 12), (124, 118), |x, y| {
        ramp(&s.pane, 0.25 + 0.3 * noise(x, y, 16, 16, 131), x, y)
    });
    for (cx, w, c) in [
        (24, 12, s.laundry[0]),
        (44, 10, s.laundry[1]),
        (88, 14, s.laundry[2]),
    ] {
        // Hanging cloth, a fold down its middle, a shadow along its hem.
        t.rect((cx, 28), (cx + w, 52), move |x, y| {
            let fold = x as i32 == cx + w / 2 || y == 51;
            let k = if fold { 0.72 } else { 1.0 };
            [c[0], c[1], c[2]].map(|v| (v as f32 * k) as u8)
        });
    }
    t.rect((4, 26), (124, 28), |_, _| s.frame[0]);
    bars(s, &mut t, (4, 12), (124, 80), 6, 132);
    // The parapet stays wall (tinted), with a lit coping.
    for y in 80..118 {
        for x in 4..124 {
            let i = y * TILE + x;
            let v = match s.wall {
                Wall::Plaster => 0.45 + 0.25 * (noise(x, y, 16, 8, 133) - 0.5),
                Wall::Tiled(n) => tiled(n, x, y, 133) - 0.1,
            };
            t.px[i] = ramp(&WALL, v, x, y);
            t.wall[i] = true;
        }
    }
    t.rect((2, 78), (126, 82), |_, y| {
        if y == 81 { s.frame[1] } else { s.frame[2] }
    });
    t.moss(s, (78, 80), 1.0, 134);
    out.push(t);
    // 4: a window with an air conditioner under it, a drip of corrosion below.
    let mut t = facade_wall(s, 140);
    window(s, &mut t, (36, 20), (92, 80), false, 141);
    t.rect((44, 92), (84, 114), |x, y| {
        let v = if y % 3 == 0 && x > 48 && x < 72 {
            0.15
        } else {
            0.6 + 0.2 * (noise(x, y, 8, 8, 142) - 0.5)
        };
        ramp(&s.metal, v, x, y)
    });
    t.rect((44, 92), (84, 94), |_, _| s.metal[4]);
    t.rect((44, 112), (84, 114), |_, _| s.metal[0]);
    t.rect((76, 114), (78, 128), |x, y| ramp(&s.corrosion, 0.4, x, y));
    out.push(t);
    // 5: a window boarded up with planks, two battens nailed across.
    let mut t = facade_wall(s, 150);
    window(s, &mut t, (36, 24), (92, 92), false, 151);
    t.rect((32, 24), (96, 92), |x, y| {
        let plank = y / 10;
        let v = if y % 10 == 0 {
            0.05
        } else {
            0.35 + 0.35 * hash(plank as i32, 0, 152) + 0.2 * (noise(x, y, 16, 1, 153) - 0.5)
        };
        ramp(&s.wood, v, x, y)
    });
    for y in [32, 82] {
        t.rect((28, y), (100, y + 6), |x, y| {
            let v = 0.6 + 0.2 * (noise(x, y, 8, 1, 154) - 0.5);
            ramp(&s.wood, if y % 6 == 5 { v - 0.35 } else { v }, x, y)
        });
        for x in [32, 64, 94] {
            t.set(x, y + 2, s.metal[0]);
        }
    }
    out.push(t);
    // 6: a broken window, soot above it.
    let mut t = facade_wall(s, 160);
    for y in 0..24 {
        for x in 32..96 {
            if noise(x, y, 8, 8, 161) + y as f32 / 48.0 > 0.6 {
                t.px[y * TILE + x] = ramp(&WALL, 0.1, x, y);
            }
        }
    }
    window(s, &mut t, (36, 24), (92, 92), true, 162);
    out.push(t);
    // 7: blank wall, a louvered vent and a pipe running down.
    let mut t = facade_wall(s, 170);
    t.rect((52, 36), (76, 56), |x, y| match y % 4 {
        0 => s.pane[1],
        1 => ramp(&s.metal, 0.8, x, y),
        _ => ramp(&s.metal, 0.5, x, y),
    });
    t.rect((108, 0), (114, 128), |x, y| {
        let v = match x {
            108 => 0.8,
            113 => 0.15,
            _ => 0.35 + 0.2 * noise(x, y, 1, 16, 171),
        };
        ramp(&s.metal, v, x, y)
    });
    t.rect((106, 60), (116, 64), |_, _| s.metal[0]);
    out.push(t);
    // 8: a shop, its shutter down: the sign band above stays wall (tinted), with unlit lettering.
    let mut t = facade_wall(s, 180);
    t.rect((4, 40), (124, 128), |x, y| {
        let v = match y % 6 {
            0 => 0.15,
            1 => 0.75,
            _ => 0.55 + 0.25 * (noise(x, y, 32, 4, 181) - 0.5),
        };
        if noise(x, y, 8, 8, 182) * 0.6 + noise(x, y, 2, 2, 185) * 0.3 + (y as f32 / 128.0) * 0.3
            > 0.78
        {
            ramp(&s.corrosion, 0.3 + 0.5 * noise(x, y, 2, 2, 183), x, y)
        } else {
            ramp(&s.metal, v, x, y)
        }
    });
    t.rect((4, 36), (124, 40), |_, _| s.metal[0]);
    letters(&mut t, 12, 184);
    out.push(t);
    // 9: a shop window, dark, its sign band above.
    let mut t = facade_wall(s, 190);
    t.rect((4, 36), (124, 128), |x, y| ramp(&s.frame, 0.3, x, y));
    t.rect((10, 42), (118, 122), |x, y| glass_px(s, x, y, false, 191));
    t.rect((62, 42), (66, 122), |_, _| s.frame[0]);
    t.rect((10, 100), (62, 102), |_, _| s.frame[0]);
    letters(&mut t, 12, 192);
    out.push(t);
    // 10: blank wall.
    out.push(facade_wall(s, 200));
    // 11: a curtain wall, glass from slab to slab, and 12: the same with panes fallen out.
    out.push(curtain(s, false, 210));
    out.push(curtain(s, true, 220));
    // 13: a ribbon window, glazed from edge to edge of the bay so the bays run on as one band.
    let mut t = facade_wall(s, 230);
    t.rect((0, 28), (128, 32), |_, _| s.frame[0]);
    t.rect((0, 32), (128, 80), |x, y| glass_px(s, x, y, false, 231));
    for x in [0, 32, 64, 96] {
        t.rect((x, 32), (x + 3, 80), |_, _| s.frame[1]);
    }
    t.rect((0, 80), (128, 84), |x, y| match y {
        80 => s.frame[2],
        83 => ramp(&s.concrete, 0.1, x, y),
        _ => ramp(&s.concrete, 0.3, x, y),
    });
    out.push(t);
    out
}

/// A curtain wall: glass from the slab up in panes 2 m wide, each catching the sky a little
/// differently, slim mullions and a transom, a spandrel band at the slab (wall, so a paint digit
/// colors it); `broken` knocks some panes out to the dark floor behind, shards left in the frame.
fn curtain(s: &Textures, broken: bool, salt: i32) -> Tile {
    let mut t = facade_wall(s, salt);
    for pane in 0..4 {
        let (x0, y0) = ((pane % 2) * 64, if pane < 2 { 0 } else { 36 });
        let y1 = if pane < 2 { 36 } else { 104 };
        let shade = hash(pane, 0, salt + 1);
        let gone = broken && hash(pane, 1, salt + 2) > 0.45;
        t.rect((x0, y0), (x0 + 64, y1), |x, y| {
            if gone {
                // The dark floor behind, the ceiling's edge a line at the top.
                let v = if y < y0 as usize + 4 { 0.3 } else { 0.05 };
                return ramp(&s.pane, v, x, y);
            }
            let d = (x + TILE - y % TILE) % 56;
            let sheen = if d < 6 { 0.3 } else { 0.0 };
            let v = 0.25 + 0.5 * shade + 0.15 * (y as f32 / TILE as f32) + sheen;
            ramp(
                &s.pane,
                v + 0.1 * (noise(x, y, 16, 16, salt + 3) - 0.5),
                x,
                y,
            )
        });
        if gone {
            for i in 0..10 {
                t.set(x0 + 4 + i, y1 - 4 - i, s.pane[3]);
                t.set(x0 + 58 - i, y0 + 4 + i / 2, s.pane[2]);
            }
        }
    }
    // Mullions (the bay's edges meet in one), the transom, and the spandrel over the slab (left
    // as wall, tinted).
    for (x0, x1) in [(0, 1), (62, 64), (127, 128)] {
        t.rect((x0, 0), (x1, 104), |_, _| s.frame[1]);
    }
    t.rect((0, 34), (128, 36), |_, _| s.frame[0]);
    t.rect((0, 102), (128, 104), |_, _| s.frame[2]);
    t
}

/// Unlit sign lettering in the sign band (rows `y0..y0 + 16`): blocky glyphs of dark paint.
fn letters(t: &mut Tile, y0: i32, salt: i32) {
    let n = 4 + (hash(0, 0, salt) * 3.0) as i32;
    let w = 10;
    let x0 = (128 - n * (w + 4)) / 2;
    for i in 0..n {
        for gy in 0..4 {
            for gx in 0..3 {
                if hash(i * 8 + gx, gy, salt + 1) > 0.45 {
                    let (x, y) = (x0 + i * (w + 4) + gx * 4 - 2, y0 + gy * 4);
                    t.rect((x, y), (x + 4, y + 4), |_, _| [40, 34, 32]);
                }
            }
        }
    }
}

// The city's own surfaces (`pavement`, `roof` and `underside` in shaders/scene.fs), which the
// shader wears further on top (dust, litter, moss, puddles).

/// Asphalt: dark binder with light aggregate showing through, patches of newer tar sealed by a
/// darker line, crack lines; moss in some cracks where the biome grows it.
fn asphalt(s: &Textures, salt: i32) -> Tile {
    let mut rng = Rng::new(salt as u64);
    let patches: Vec<(i32, i32, i32, i32)> = (0..2)
        .map(|_| {
            (
                rng.range_i(0, TILE as i32),
                rng.range_i(0, TILE as i32),
                rng.range_i(24, 56),
                rng.range_i(16, 40),
            )
        })
        .collect();
    // 0 outside every patch, 1 on a patch's border, 2 inside.
    let patch = |x: usize, y: usize| {
        patches
            .iter()
            .map(|&(px, py, w, h)| {
                let dx = (x as i32 - px).rem_euclid(TILE as i32);
                let dy = (y as i32 - py).rem_euclid(TILE as i32);
                if dx >= w || dy >= h {
                    0
                } else if dx == 0 || dy == 0 || dx == w - 1 || dy == h - 1 {
                    1
                } else {
                    2
                }
            })
            .max()
            .unwrap_or(0)
    };
    let mut t = Tile::new();
    t.fill(|x, y| {
        let mut v = 0.42
            + 0.16 * (noise(x, y, 32, 32, salt + 1) - 0.5)
            + 0.1 * (noise(x, y, 8, 8, salt + 2) - 0.5);
        match patch(x, y) {
            1 => v = 0.05,
            2 => v = 0.25 + 0.08 * (noise(x, y, 16, 16, salt + 3) - 0.5),
            _ => {
                let a = hash(x as i32, y as i32, salt + 4);
                if a > 0.94 {
                    v += 0.25;
                } else if a < 0.05 {
                    v -= 0.18;
                }
            }
        }
        ramp(&s.asphalt, v, x, y)
    });
    t.cracks(4, 36, s.asphalt[0], salt as u64 + 5);
    for y in 0..TILE {
        for x in 0..TILE {
            if t.px[y * TILE + x] == s.asphalt[0]
                && let Some(m) = moss(s, x, y, 0.9, salt + 6)
            {
                t.px[y * TILE + x] = m;
            }
        }
    }
    t
}

/// A flat roof: tar membrane in rolls a meter wide, a raised seam between them lit on one side,
/// gravel strewn over it, moss in patches.
fn roof(s: &Textures) -> Tile {
    let mut t = Tile::new();
    t.fill(|x, y| {
        if let Some(m) = moss(s, x, y, 0.45, 155) {
            return m;
        }
        let mut v =
            0.5 + 0.2 * (noise(x, y, 32, 32, 150) - 0.5) + 0.12 * (noise(x, y, 2, 2, 151) - 0.5);
        if x % 32 == 0 {
            v = 0.85;
        } else if x % 32 == 1 {
            v = 0.15;
        }
        if hash(x as i32, y as i32, 152) > 0.86 {
            return ramp(
                &s.concrete,
                0.35 + 0.5 * hash(x as i32, y as i32, 153),
                x,
                y,
            );
        }
        ramp(&s.asphalt, v, x, y)
    });
    t.cracks(2, 20, s.asphalt[0], 154);
    t
}

/// The underside of a deck: cast concrete in coffers between beams, a main beam along one edge of
/// the tile and a rib along the other, lit on one side and shadowed on the other; rings of water
/// stains in the coffers, pale mineral streaks along the beams, algae where it is damp.
fn underside(s: &Textures) -> Tile {
    const BEAM: usize = 12;
    const RIB: usize = 6;
    let mut t = Tile::new();
    t.fill(|x, y| {
        let beam = x < BEAM;
        let rib = y < RIB;
        let mut v = if beam || rib {
            0.6 + 0.15 * (noise(x, y, 16, 16, 160) - 0.5)
        } else {
            0.3 + 0.15 * (noise(x, y, 32, 32, 161) - 0.5)
        };
        if x == BEAM || y == RIB {
            v = 0.05;
        } else if x == BEAM - 1 || y == RIB - 1 {
            v += 0.2;
        }
        if !beam && !rib {
            // Rings of water stains, spreading from where the slab leaks.
            let w = noise(x, y, 32, 32, 162);
            if w > 0.55 && (w * 14.0).fract() < 0.18 {
                v -= 0.15;
            }
            if w > 0.6 && s.damp > 0.0 && noise(x, y, 4, 4, 163) > 1.0 - 0.5 * s.damp {
                return ramp(&s.moss, 0.1 + 0.3 * noise(x, y, 8, 8, 164), x, y);
            }
        } else if noise(x, y, 2, 32, 165) > 0.78 {
            // Efflorescence: salts the water leaves on its way out.
            v = 0.95;
        }
        v += 0.1 * (noise(x, y, 2, 2, 166) - 0.5);
        ramp(&s.concrete, v, x, y)
    });
    t.cracks(3, 24, s.concrete[0], 167);
    t
}

/// Road, sidewalk, roof and deck underside, in `SURFACES` order.
fn surface_tiles(s: &Textures) -> Vec<Tile> {
    let road = match s.road {
        Road::Asphalt => asphalt(s, 120),
        Road::Stone(how) => paving(s, &s.stone_dark, how, 130),
    };
    vec![
        road,
        paving(s, &s.concrete, s.sidewalk, 140),
        roof(s),
        underside(s),
    ]
}

/// Rows of the swatch tile, each a ramp of five colors from its first pixel (`swatch` in
/// shaders/scene.fs).
const SWATCHES: usize = 6;

/// The swatch tile: the ramps the shader's wear is drawn in, a row each: dust, litter, moss,
/// water, corrosion, asphalt.
fn swatches(s: &Textures) -> Tile {
    let rows: [&Ramp; SWATCHES] = [
        &s.dust,
        &s.litter,
        &s.moss,
        &s.water,
        &s.corrosion,
        &s.asphalt,
    ];
    let mut t = Tile::new();
    for (r, ramp) in rows.iter().enumerate() {
        for (i, c) in ramp.iter().enumerate() {
            t.set(i as i32, r as i32, *c);
        }
    }
    t
}

/// Every biome's wear amounts for the shader (`uWear`): two vec3s a biome, (moss, damp, dust) and
/// (litter, puddles, lane markings), unused biomes zero.
pub fn wear_uniform() -> Vec<f32> {
    let mut v: Vec<f32> = BIOMES
        .iter()
        .take(MAX_BIOMES)
        .flat_map(|b| {
            let s = &b.textures;
            let markings = matches!(s.road, Road::Asphalt) as u8 as f32;
            [s.mossy, s.damp, s.dusty, s.littered, s.puddled, markings]
        })
        .collect();
    v.resize(MAX_BIOMES * 6, 0.0);
    v
}

/// A biome's row of tiles: the materials in `Mat::index` order, the facade units, the surfaces,
/// the swatches.
fn row(s: &Textures) -> Vec<Tile> {
    [
        paving(s, &s.stone, s.paving.0, 1),
        paving(s, &s.stone_dark, s.paving.1, 10),
        concrete(s),
        brick(s),
        tiles(s),
        metal(s, s.corroded.0),
        metal(s, s.corroded.1),
        wood(s),
        glass(s),
        soil(s),
        paint(s),
    ]
    .into_iter()
    .chain(facade_tiles(s))
    .chain(surface_tiles(s))
    .chain(std::iter::once(swatches(s)))
    .collect()
}

/// The atlas, RGBA: a row of tiles per biome (`biome::variant_of`), rows past the last biome
/// left empty. Alpha marks the facade's wall pixels.
pub fn atlas_pixels() -> (usize, usize, Vec<u8>) {
    let (w, h) = (TILE * COUNT, TILE * ROWS);
    let mut rgba = vec![0u8; w * h * 4];
    for (r, biome) in BIOMES.iter().enumerate().take(ROWS) {
        for (i, t) in row(&biome.textures).iter().enumerate() {
            for y in 0..TILE {
                for x in 0..TILE {
                    let c = t.px[y * TILE + x];
                    let o = ((r * TILE + y) * w + i * TILE + x) * 4;
                    rgba[o..o + 3].copy_from_slice(&c);
                    rgba[o + 3] = if t.wall[y * TILE + x] { 255 } else { 0 };
                }
            }
        }
    }
    (w, h, rgba)
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
        assert_eq!((w, h), (TILE * COUNT, TILE * ROWS));
        assert_eq!(rgba.len(), w * h * 4);
        // Noise wraps: the first and last columns of a noise field match up smoothly.
        let d = (noise(0, 5, 8, 8, 1) - noise(TILE, 5, 8, 8, 1)).abs();
        assert!(d < 1e-6);
    }

    /// Every biome's patterns tile the 4 m tile and its wear stays in range, and each biome draws
    /// a row of its own.
    #[test]
    fn every_biome_draws_its_own_tiles() {
        let divides = |n: usize| n >= 2 && TILE.is_multiple_of(n);
        let mut rows: Vec<Vec<Rgb>> = Vec::new();
        for b in BIOMES {
            let s = &b.textures;
            let road = match s.road {
                Road::Stone(p) => Some(p),
                Road::Asphalt => None,
            };

            for p in [s.paving.0, s.paving.1, s.sidewalk].into_iter().chain(road) {
                let (Paving::Slabs(n) | Paving::Setts(n)) = p;
                assert!(divides(n) && n >= 4, "{}: paving of {n}", b.name);
            }
            if let Wall::Tiled(n) = s.wall {
                assert!(divides(n) && n >= 3, "{}: wall tiles of {n}", b.name);
            }
            let (bw, bh) = s.bricks;
            assert!(divides(bw) && divides(bh) && bw >= 4, "{}: bricks", b.name);
            assert!(divides(s.tiles) && s.tiles >= 3, "{}: tiles", b.name);
            for f in [
                s.mosaic,
                s.corroded.0,
                s.corroded.1,
                s.mossy,
                s.damp,
                s.dusty,
                s.littered,
                s.puddled,
            ] {
                assert!((0.0..=1.0).contains(&f), "{}: {f} out of 0..1", b.name);
            }
            let tiles = row(s);
            assert_eq!(tiles.len(), COUNT, "{}: tiles in a row", b.name);
            let px: Vec<Rgb> = tiles.into_iter().flat_map(|t| t.px).collect();
            assert!(!rows.contains(&px), "{} draws another biome's row", b.name);
            rows.push(px);
        }
    }
}
