//! Biomes: big zones of the map with their own generation parameters and flower palette.
//!
//! A biome is a pure function of `(seed, lot coords)` like everything else, so zones stay
//! deterministic and infinite, and lots stay independent (nothing is streamed or stored).
//!
//! Zones come from a jittered grid of *sites*, one per `SITE_LOTS` x `SITE_LOTS` cell. Each site
//! picks a biome (weighted by `frequency`) and a lot belongs to the site nearest to it, where
//! distance is divided by the biome's `size`. That is a multiplicatively weighted Voronoi
//! diagram: a biome with a bigger `size` claims larger zones on average, so biome types can have
//! different typical zone sizes.
//!
//! To add a biome: define a `Biome` const (start from `DEFAULT` with `..DEFAULT`), add it to
//! `BIOMES`. Everything that varies per biome lives in `Biome`, so generation code never matches
//! on a biome's name.
//!
//! Not done yet: blending across zone borders (palette and density fade, discrete choices flip),
//! per-biome fog and sky tint in the post pass, and per-biome plant species.

use super::city;
use crate::rng::{Rng, hash_coords};

const SALT_BIOME: i32 = 4;

/// Side, in lots, of the grid cell holding one zone site. Typical zones span about twice this.
pub const SITE_LOTS: i32 = 6;

/// Colors the scene shader uses for a biome's architecture (linear RGB). Index in `BIOMES` selects
/// the palette (`render` uploads them all as one uniform array).
pub struct Palette {
    /// Neon strips over storefronts and vertical signs.
    pub neon: [[f32; 3]; 3],
    /// Light of lit windows: warm white, cool white, then two rarer tints.
    pub windows: [[f32; 3]; 4],
    /// Colored cladding panels on facades.
    pub accents: [[f32; 3]; 4],
    /// Billboard backgrounds.
    pub signs: [[f32; 3]; 5],
    /// Multiplies the base color of facades and pavement.
    pub tint: [f32; 3],
}

/// Vec3 entries per biome in the palette uniform.
pub const PALETTE_VEC3S: usize = 3 + 4 + 4 + 5 + 1;
/// Most biomes the shader's palette array has room for.
pub const MAX_BIOMES: usize = 8;

impl Palette {
    fn flat(&self) -> impl Iterator<Item = f32> + '_ {
        self.neon
            .iter()
            .chain(&self.windows)
            .chain(&self.accents)
            .chain(&self.signs)
            .chain(std::iter::once(&self.tint))
            .flatten()
            .copied()
    }
}

/// Everything generation reads from a biome. `DEFAULT` holds the original values of the city.
pub struct Biome {
    pub name: &'static str,
    /// How often this biome is picked for a site, relative to the others.
    pub frequency: f32,
    /// Relative zone size; keep within 0.6..=1.5 so the site search (3x3 cells) stays exact.
    pub size: f32,

    /// Flower colors (sRGB) with pick weights.
    pub flowers: &'static [(u32, [u8; 3])],
    /// Plant growth multiplier per height layer: canopy, middle, depths (see `growth::layer`).
    pub plant_density: [f32; 3],
    /// Flower multiplier per height layer.
    pub bloom: [f32; 3],
    /// Multipliers on how many bushes, hanging vines and wall climbers a lot gets.
    pub bush_rate: f32,
    pub vine_rate: f32,
    pub climber_rate: f32,

    /// Chance that a lot holds no tower.
    pub tower_empty: f32,
    /// Chance that a lot keeps its upper and its mid deck.
    pub deck_keep: [f32; 2],
    /// Chance that a tower-less lot holds a spiral stair.
    pub stair_chance: f32,
    /// Chance that two neighboring towers are linked by a bridge.
    pub bridge_chance: f32,

    pub palette: Palette,
}

/// The original look of the city: cyan and magenta flowers with some green, violet and amber,
/// vegetation and flowers both thickest in the sunlit canopy and thinnest in the near-dark depths.
pub const DEFAULT: Biome = Biome {
    name: "default",
    frequency: 1.0,
    size: 1.0,
    flowers: &[
        (2, [70, 230, 210]),
        (1, [120, 240, 110]),
        (2, [240, 100, 200]),
        (1, [140, 130, 255]),
        (1, [255, 200, 100]),
    ],
    plant_density: [1.5, 1.0, 0.45],
    bloom: [2.4, 1.0, 0.5],
    bush_rate: 1.0,
    vine_rate: 1.0,
    climber_rate: 1.0,
    tower_empty: 0.18,
    deck_keep: [0.85, 0.6],
    stair_chance: 0.7,
    bridge_chance: 0.35,
    palette: Palette {
        neon: [[0.1, 0.9, 1.0], [1.0, 0.2, 0.75], [1.0, 0.65, 0.2]],
        windows: [
            [1.0, 0.84, 0.58],
            [0.72, 0.86, 1.0],
            [0.45, 0.9, 0.95],
            [1.0, 0.45, 0.7],
        ],
        accents: [
            [0.75, 0.22, 0.03],
            [0.85, 0.6, 0.04],
            [0.04, 0.4, 0.45],
            [0.8, 0.8, 0.78],
        ],
        signs: [
            [0.9, 0.62, 0.04],
            [0.9, 0.3, 0.05],
            [0.05, 0.55, 0.7],
            [0.75, 0.1, 0.45],
            [0.85, 0.85, 0.82],
        ],
        tint: [1.0, 1.0, 1.0],
    },
};

/// Every biome the map can hold. Only the default exists for now.
pub const BIOMES: &[&Biome] = &[&DEFAULT];

/// Index of `biome` in `BIOMES`: the material variant its lots are drawn with.
pub fn variant_of(biome: &Biome) -> usize {
    BIOMES
        .iter()
        .position(|b| std::ptr::eq(*b, biome))
        .unwrap_or(0)
}

/// All palettes packed for the shader (`MAX_BIOMES` x `PALETTE_VEC3S` vec3s, unused ones zero).
pub fn palette_uniform() -> Vec<f32> {
    let mut v: Vec<f32> = BIOMES
        .iter()
        .take(MAX_BIOMES)
        .flat_map(|b| b.palette.flat())
        .collect();
    v.resize(MAX_BIOMES * PALETTE_VEC3S * 3, 0.0);
    v
}

/// The biome of a lot.
pub fn biome_at(seed: u64, lx: i32, lz: i32) -> &'static Biome {
    // The plaza around spawn is always the default biome.
    if city::is_plaza(lx, lz) {
        return &DEFAULT;
    }
    pick(BIOMES, seed, lx, lz)
}

/// `biome_at` over an explicit list (tests use their own).
fn pick<'a>(biomes: &[&'a Biome], seed: u64, lx: i32, lz: i32) -> &'a Biome {
    if let [only] = biomes {
        return only;
    }
    let total: f32 = biomes.iter().map(|b| b.frequency).sum();
    let (cx, cz) = (lx.div_euclid(SITE_LOTS), lz.div_euclid(SITE_LOTS));
    let lot = (lx as f32 + 0.5, lz as f32 + 0.5);
    let mut best = (f32::MAX, biomes[0]);
    for sz in cz - 1..=cz + 1 {
        for sx in cx - 1..=cx + 1 {
            let mut rng = Rng::new(hash_coords(seed, sx, SALT_BIOME, sz));
            let site = (
                (sx as f32 + rng.f32()) * SITE_LOTS as f32,
                (sz as f32 + rng.f32()) * SITE_LOTS as f32,
            );
            let mut roll = rng.f32() * total;
            let biome = biomes
                .iter()
                .find(|b| {
                    roll -= b.frequency;
                    roll < 0.0
                })
                .unwrap_or(&biomes[0]);
            let d = ((lot.0 - site.0).powi(2) + (lot.1 - site.1).powi(2)).sqrt() / biome.size;
            if d < best.0 {
                best = (d, biome);
            }
        }
    }
    best.1
}

/// Picks one color of a weighted palette using `rng`.
pub fn pick_color(palette: &[(u32, [u8; 3])], rng: &mut Rng) -> [u8; 3] {
    let total: i32 = palette.iter().map(|p| p.0 as i32).sum();
    let mut roll = rng.range_i(0, total);
    for &(w, c) in palette {
        roll -= w as i32;
        if roll < 0 {
            return c;
        }
    }
    palette[palette.len() - 1].1
}

#[cfg(test)]
mod tests {
    use super::*;

    const SMALL: Biome = Biome {
        name: "small",
        size: 0.7,
        ..DEFAULT
    };
    const LARGE: Biome = Biome {
        name: "large",
        size: 1.4,
        ..DEFAULT
    };

    #[test]
    fn default_is_the_only_biome_for_now() {
        for (lx, lz) in [(0, 0), (5, -9), (-40, 77)] {
            assert_eq!(biome_at(1, lx, lz).name, "default");
        }
    }

    #[test]
    fn zones_are_deterministic_contiguous_and_sized_by_type() {
        let list: &[&Biome] = &[&SMALL, &LARGE];
        let (mut n_small, mut n_large, mut changes, mut pairs) = (0, 0, 0, 0);
        for lz in -80..80 {
            for lx in -80..80 {
                let b = pick(list, 3, lx, lz);
                assert_eq!(b.name, pick(list, 3, lx, lz).name);
                match b.name {
                    "small" => n_small += 1,
                    _ => n_large += 1,
                }
                // Zones are blobs, not noise: neighbors mostly agree.
                pairs += 1;
                changes += (pick(list, 3, lx + 1, lz).name != b.name) as usize;
            }
        }
        assert!(n_small > 0 && n_large > 0, "both biomes appear");
        assert!(n_large > n_small * 2, "the larger biome claims more land");
        assert!(
            changes * 8 < pairs,
            "zones are large: {changes}/{pairs} borders"
        );
    }

    #[test]
    fn palette_pick_matches_weights() {
        let mut rng = Rng::new(1);
        let mut counts = [0; 5];
        for _ in 0..7000 {
            let c = pick_color(DEFAULT.flowers, &mut rng);
            counts[DEFAULT.flowers.iter().position(|p| p.1 == c).unwrap()] += 1;
        }
        assert!(counts[0] > counts[1] * 3 / 2 && counts[2] > counts[3] * 3 / 2);
    }
}
