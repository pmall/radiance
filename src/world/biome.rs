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
//! A biome is also a collection of chunks: its `name` is its folder, `assets/biomes/<name>/`
//! (`chunks.rs`, `docs/chunks.md`), and the planner places chunks from it in its zones. Its props
//! are built from its own modules of the same names (`props.rs`).
//!
//! To add a biome: define a `Biome` const (start from `DEFAULT` with `..DEFAULT`), add it to
//! `BIOMES`, and give it a folder of chunks (or none: it then draws from `default`). Everything
//! that varies per biome lives in `Biome` (its plants, its sky and fog, its textures and palette)
//! or in its folder (chunks, modules, props), so generation code never matches on a biome's name.
//!
//! Not done yet: blending across zone borders (palette and density fade, discrete choices flip).

use super::city;
use crate::render::look::{self, Looks};
use crate::render::pixels::{self, Textures};
use crate::rng::{Rng, hash_coords};

const SALT_BIOME: i32 = 4;

/// Side, in lots, of the grid cell holding one zone site. Typical zones span about twice this.
pub const SITE_LOTS: i32 = 6;

/// Colors the scene shader uses for a biome's architecture. Index in `BIOMES` selects the palette
/// (`render` uploads them all as one uniform array).
pub struct Palette {
    /// The paints of tower walls (sRGB): a written elevation's digits pick one, shifted per tower
    /// (`tower::paint`), and the shader tints the walls' pixel art with it.
    pub paints: [[u8; 3]; 8],
    /// Multiplies the base color of the pavement (linear RGB).
    pub tint: [f32; 3],
}

/// Vec3 entries per biome in the palette uniform.
pub const PALETTE_VEC3S: usize = 8 + 1;
/// Most biomes the shader's palette array and the pixel-art atlas have room for.
pub const MAX_BIOMES: usize = 8;

impl Palette {
    /// Linear RGB, paints first.
    fn flat(&self) -> impl Iterator<Item = f32> + '_ {
        let linear = |c: &[u8; 3]| c.map(|v| (v as f32 / 255.0).powf(2.2));
        self.paints
            .iter()
            .map(linear)
            .chain(std::iter::once(self.tint))
            .flatten()
    }
}

/// What a biome grows: how much of each kind of plant, the shapes of each kind (a weighted list of
/// species, the seed picking one per plant), their colors and their flowers. How dense plants and
/// flowers are by floor comes from the floor profiles (`design.rs`); these multiply it.
pub struct Plants {
    /// Flower colors (sRGB) with pick weights.
    pub flowers: &'static [(u32, [u8; 3])],
    /// Leaf and stem colors (sRGB) with pick weights; each plant varies its pick by up to
    /// `jitter` on every channel.
    pub foliage: &'static [(u32, [u8; 3])],
    pub jitter: i32,
    /// Multipliers on how many bushes, hanging vines and wall climbers a lot gets.
    pub bush_rate: f32,
    pub vine_rate: f32,
    pub climber_rate: f32,
    /// What grows on top surfaces, hangs from the edges of decks and bridges, climbs tower walls:
    /// species with pick weights. A list of one species costs no roll.
    pub bushes: &'static [(f32, Bush)],
    pub vines: &'static [(f32, Vine)],
    pub climbers: &'static [(f32, Climber)],
    pub flower: Flower,
}

/// A bush: a rosette of arching blades on a top surface (fronds, grass, reeds), with at most one
/// flower on a stem from its middle.
pub struct Bush {
    /// Size of the plant (meters).
    pub size: (f32, f32),
    /// Blades per plant, `min..max`.
    pub blades: (i32, i32),
    /// Blade length: the size times 0.9 to 1.5, times this. Up to 1.2 the blades fit the room a
    /// bush keeps from walls; longer ones keep more.
    pub length: f32,
    /// Blade width, relative to its length.
    pub width: f32,
    /// Elevation of the innermost blades (radians), and how much lower the outermost arch.
    pub upright: f32,
    pub arch: f32,
    /// How far a blade droops toward its tip, relative to its length.
    pub droop: (f32, f32),
    /// Height of the flower's stem, relative to the size.
    pub stem: (f32, f32),
}

/// A vine hanging from an edge: crossed ribbons swaying sideways, leaves along it, flowers down
/// its length.
pub struct Vine {
    /// Length (meters), up to what lies below; the longest grow `lush` meters longer where the
    /// floor is dense with plants.
    pub length: (f32, f32),
    pub lush: f32,
    /// Ribbon width at the top (meters); it tapers to 0.4 of it.
    pub width: (f32, f32),
    /// How far it sways sideways at the bottom (meters).
    pub sway: (f32, f32),
    /// Chance of a leaf at each of its six joints, the leaves' size (meters) and their width
    /// (each side, relative to the size).
    pub leaves: f32,
    pub leaf: (f32, f32),
    pub leaf_width: f32,
}

/// A climber: a stem branching up a wall, leaves and flowers along it.
pub struct Climber {
    /// How high it climbs (meters), up to the roof.
    pub height: (f32, f32),
    /// Stem width at the root (meters); it thins as it climbs, branches thinner still.
    pub width: f32,
    /// Length of a step (meters), and how far its heading wanders each step (radians).
    pub step: f32,
    pub wander: f32,
    /// Chance of a branch at each step.
    pub branch: f32,
    /// Chance of a leaf at each step, the leaves' size (meters) and their width (each side,
    /// relative to the size).
    pub leaves: f32,
    pub leaf: (f32, f32),
    pub leaf_width: f32,
}

/// The shape of a biome's flowers: petals around a bright core, and the light each one gives.
pub struct Flower {
    /// Petals per flower, `min..max`.
    pub petals: (i32, i32),
    /// Petal length and width, relative to the flower's radius.
    pub length: f32,
    pub width: f32,
    /// Multiplies every flower's radius (the light's reach follows).
    pub size: f32,
    /// Brightness of its light.
    pub light: f32,
}

/// A biome's air: its sky, fog, light and grading through the day, and the particles drifting
/// in it. Around the player it is drawn in their biome's atmosphere, fading into the next one's
/// when they cross a border (`render::look::Blend`).
pub struct Atmosphere {
    pub looks: Looks,
    /// Pollen mote tint by day (display space), mixed with the sun or moon's color.
    pub pollen: [f32; 3],
    /// Colors of the glowing spores of the night and the depths (display space).
    pub spores: [[f32; 3]; 4],
    /// The color script by height: the hue of the haze and of the light where the sky does not
    /// reach, in the depths, the middle floors and the canopy (display space; only the hue counts,
    /// the brightness comes from the look's fog and ambient).
    pub heights: [[f32; 3]; 3],
}

/// Everything generation and rendering read from a biome. `DEFAULT` holds the original values of
/// the city.
pub struct Biome {
    pub name: &'static str,
    /// How often this biome is picked for a site, relative to the others.
    pub frequency: f32,
    /// Relative zone size; keep within 0.6..=1.5 so the site search (3x3 cells) stays exact.
    pub size: f32,

    pub plants: Plants,
    pub atmosphere: Atmosphere,
    /// How its materials and facade units are drawn: its own row of pixel art.
    pub textures: Textures,

    /// Chance that a lot holds no tower.
    pub tower_empty: f32,
    /// Chance that two neighboring towers are linked by a bridge.
    pub bridge_chance: f32,

    pub palette: Palette,
}

/// The original look of the city: cyan and magenta flowers with some green, violet and amber,
/// fronds and grass tufts, leafy vines and climbers.
pub const DEFAULT: Biome = Biome {
    name: "default",
    frequency: 1.0,
    size: 1.0,
    plants: Plants {
        flowers: &[
            (2, [70, 230, 210]),
            (1, [120, 240, 110]),
            (2, [240, 100, 200]),
            (1, [140, 130, 255]),
            (1, [255, 200, 100]),
        ],
        foliage: &[
            (2, [52, 130, 84]),
            (2, [34, 112, 104]),
            (1, [96, 150, 70]),
            (1, [70, 100, 60]),
        ],
        jitter: 14,
        bush_rate: 1.0,
        vine_rate: 1.0,
        climber_rate: 1.0,
        bushes: &[
            // Tufts of narrow blades, then rosettes of broad fronds.
            (
                0.3,
                Bush {
                    size: (1.0, 2.0),
                    blades: (10, 17),
                    length: 1.2,
                    width: 0.09,
                    upright: 1.3,
                    arch: 0.9,
                    droop: (0.15, 0.55),
                    stem: (0.55, 0.8),
                },
            ),
            (
                0.7,
                Bush {
                    size: (1.0, 2.0),
                    blades: (6, 10),
                    length: 1.0,
                    width: 0.3,
                    upright: 1.3,
                    arch: 0.9,
                    droop: (0.15, 0.55),
                    stem: (0.55, 0.8),
                },
            ),
        ],
        vines: &[(
            1.0,
            Vine {
                length: (3.0, 10.0),
                lush: 6.0,
                width: (0.3, 0.55),
                sway: (0.15, 0.7),
                leaves: 0.85,
                leaf: (1.2, 2.2),
                leaf_width: 0.28,
            },
        )],
        climbers: &[(
            1.0,
            Climber {
                height: (12.0, 30.0),
                width: 0.5,
                step: 1.1,
                wander: 0.4,
                branch: 0.12,
                leaves: 0.8,
                leaf: (1.0, 1.9),
                leaf_width: 0.28,
            },
        )],
        flower: Flower {
            petals: (5, 8),
            length: 1.7,
            width: 0.55,
            size: 1.0,
            light: 2.2,
        },
    },
    atmosphere: Atmosphere {
        looks: look::DEFAULT_LOOKS,
        pollen: [1.0, 0.78, 0.4],
        spores: [
            [0.35, 0.95, 0.9],
            [0.55, 1.0, 0.5],
            [1.0, 0.45, 0.85],
            [0.6, 0.6, 1.0],
        ],
        // Cold blue-green depths where every flower stands out, warm haze in the middle, gold
        // under the canopy.
        heights: [[0.42, 0.66, 0.78], [0.95, 0.74, 0.5], [1.0, 0.86, 0.56]],
    },
    tower_empty: 0.18,
    bridge_chance: 0.6,
    textures: pixels::DEFAULT_TEXTURES,
    palette: Palette {
        paints: [
            [150, 60, 48],
            [96, 130, 70],
            [200, 186, 150],
            [60, 120, 118],
            [190, 140, 60],
            [180, 110, 110],
            [80, 100, 130],
            [170, 90, 50],
        ],
        tint: [1.0, 1.0, 1.0],
    },
};

/// Cyan Grotto (`docs/biome-ideas.md`): cold, calm, aquatic. Cyan and teal flowers with the odd
/// white one, low ferns and moss, long curtains of vines, a teal fog at every hour; more empty
/// lots, fewer bridges. Its props are tiled (`assets/biomes/grotto/`).
pub const GROTTO: Biome = Biome {
    name: "grotto",
    frequency: 0.5,
    size: 1.0,
    plants: Plants {
        flowers: &[
            (4, [60, 230, 220]),
            (3, [40, 190, 235]),
            (1, [200, 250, 255]),
        ],
        foliage: &[
            (2, [30, 100, 110]),
            (2, [40, 120, 100]),
            (1, [60, 140, 140]),
            (1, [25, 80, 95]),
        ],
        jitter: 12,
        bush_rate: 0.8,
        vine_rate: 3.0,
        climber_rate: 0.6,
        bushes: &[
            // Low ferns, broad fronds lying out flat.
            (
                0.8,
                Bush {
                    size: (0.8, 1.5),
                    blades: (8, 13),
                    length: 1.1,
                    width: 0.22,
                    upright: 0.9,
                    arch: 0.7,
                    droop: (0.3, 0.7),
                    stem: (0.35, 0.5),
                },
            ),
            // Moss tufts: short, dense, fine.
            (
                0.2,
                Bush {
                    size: (0.5, 0.9),
                    blades: (14, 22),
                    length: 0.8,
                    width: 0.08,
                    upright: 1.2,
                    arch: 0.9,
                    droop: (0.2, 0.5),
                    stem: (0.6, 0.9),
                },
            ),
        ],
        vines: &[(
            1.0,
            Vine {
                length: (6.0, 16.0),
                lush: 8.0,
                width: (0.35, 0.65),
                sway: (0.05, 0.3),
                leaves: 0.95,
                leaf: (1.0, 1.8),
                leaf_width: 0.22,
            },
        )],
        climbers: &[(
            1.0,
            Climber {
                height: (8.0, 20.0),
                width: 0.3,
                step: 1.0,
                wander: 0.3,
                branch: 0.08,
                leaves: 0.6,
                leaf: (0.7, 1.3),
                leaf_width: 0.22,
            },
        )],
        flower: Flower {
            petals: (6, 9),
            length: 1.4,
            width: 0.45,
            size: 0.9,
            light: 2.4,
        },
    },
    atmosphere: Atmosphere {
        looks: look::GROTTO_LOOKS,
        pollen: [0.7, 0.95, 1.0],
        spores: [
            [0.3, 0.95, 0.95],
            [0.4, 0.8, 1.0],
            [0.6, 1.0, 0.9],
            [0.85, 0.95, 1.0],
        ],
        // Deep blue at the bottom, teal through the middle, pale aqua-green up high.
        heights: [[0.32, 0.56, 0.86], [0.46, 0.84, 0.86], [0.7, 0.96, 0.84]],
    },
    tower_empty: 0.25,
    bridge_chance: 0.45,
    textures: pixels::GROTTO_TEXTURES,
    palette: Palette {
        // Tiled walls gone cold: teals, aquas, bone white, slate.
        paints: [
            [78, 128, 132],
            [150, 184, 180],
            [198, 208, 200],
            [70, 98, 124],
            [104, 146, 126],
            [168, 192, 200],
            [52, 104, 112],
            [126, 136, 150],
        ],
        tint: [0.86, 0.96, 1.0],
    },
};

/// Every biome the map can hold.
pub const BIOMES: &[&Biome] = &[&DEFAULT, &GROTTO];

/// Index of `biome` in `BIOMES`: the material variant its lots are drawn with. By name: two
/// references to one `const` biome need not share an address.
pub fn variant_of(biome: &Biome) -> usize {
    BIOMES
        .iter()
        .position(|b| b.name == biome.name)
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

/// The biome of that name (its folder, a catalog's `biome`), or the default one.
pub fn named(name: &str) -> &'static Biome {
    BIOMES.iter().find(|b| b.name == name).unwrap_or(&&DEFAULT)
}

/// The biome of a lot.
pub fn biome_at(seed: u64, lx: i32, lz: i32) -> &'static Biome {
    // A chunk shown alone (`--chunk BIOME/NAME`) stands in its biome: its plants, props and sky.
    if let Some(p) = super::chunks::preview() {
        return named(p.biome);
    }
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

/// Picks one species of a weighted list using `rng`; a list of one costs no roll, so a biome
/// adding a second species to one kind leaves the other kinds' plants as they were.
pub fn pick_species<'a, T>(species: &'a [(f32, T)], rng: &mut Rng) -> &'a T {
    if let [(_, only)] = species {
        return only;
    }
    let total: f32 = species.iter().map(|s| s.0).sum();
    let mut roll = rng.f32() * total;
    for (w, s) in species {
        if roll < *w {
            return s;
        }
        roll -= w;
    }
    &species[species.len() - 1].1
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

    /// What a new biome must respect: the shader's palette slots, the site search's size range, a
    /// unique name, flowers to pick from.
    #[test]
    fn biomes_fit_the_renderer_and_the_zones() {
        assert!(
            BIOMES.len() <= MAX_BIOMES,
            "the palette holds {MAX_BIOMES} biomes"
        );
        assert!(BIOMES.iter().any(|b| b.name == DEFAULT.name));
        for (i, b) in BIOMES.iter().enumerate() {
            assert!(
                BIOMES[..i].iter().all(|o| o.name != b.name),
                "two {}",
                b.name
            );
            assert_eq!(variant_of(b), i);
            assert!((0.6..=1.5).contains(&b.size), "{}: size {}", b.name, b.size);
            assert!(b.frequency > 0.0, "{} is never picked", b.name);
            let p = &b.plants;
            assert!(
                p.flowers.iter().any(|f| f.0 > 0),
                "{} has no flower colors",
                b.name
            );
            assert!(p.foliage.iter().any(|f| f.0 > 0), "{}: no foliage", b.name);
            let ordered = |(lo, hi): (f32, f32)| lo <= hi;
            let picks = |weights: &mut dyn Iterator<Item = f32>| {
                weights.fold((0, true), |(n, ok), w| (n + 1, ok && w > 0.0))
            };
            for (kind, (n, ok)) in [
                ("bushes", picks(&mut p.bushes.iter().map(|s| s.0))),
                ("vines", picks(&mut p.vines.iter().map(|s| s.0))),
                ("climbers", picks(&mut p.climbers.iter().map(|s| s.0))),
            ] {
                assert!(n > 0 && ok, "{}: {kind} need species of weight > 0", b.name);
            }
            for (_, s) in p.bushes {
                assert!(ordered(s.size) && ordered(s.droop) && ordered(s.stem));
                assert!(s.blades.0 > 0 && s.blades.0 < s.blades.1, "{}", b.name);
            }
            for (_, s) in p.vines {
                assert!(ordered(s.length) && ordered(s.width) && ordered(s.leaf));
            }
            for (_, s) in p.climbers {
                assert!(ordered(s.height) && ordered(s.leaf) && s.step > 0.1);
            }
            let f = &p.flower;
            assert!(f.petals.0 >= 3 && f.petals.0 < f.petals.1, "{}", b.name);
        }
    }

    /// The player wakes up in the default biome, and every biome has zones in every city, the
    /// default one the most.
    #[test]
    fn every_biome_has_zones_and_the_spawn_is_default() {
        for seed in 1..=3 {
            assert_eq!(biome_at(seed, 0, 0).name, DEFAULT.name);
            let mut lots = vec![0; BIOMES.len()];
            for lz in (-60..60).step_by(2) {
                for lx in (-60..60).step_by(2) {
                    lots[variant_of(biome_at(seed, lx, lz))] += 1;
                }
            }
            assert!(lots.iter().all(|&n| n > 0), "seed {seed}: {lots:?}");
            assert!(
                lots[0] == *lots.iter().max().unwrap(),
                "seed {seed}: {lots:?}"
            );
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
            let flowers = DEFAULT.plants.flowers;
            let c = pick_color(flowers, &mut rng);
            counts[flowers.iter().position(|p| p.1 == c).unwrap()] += 1;
        }
        assert!(counts[0] > counts[1] * 3 / 2 && counts[2] > counts[3] * 3 / 2);
    }

    #[test]
    fn species_pick_matches_weights_and_one_costs_no_roll() {
        let (mut a, mut b) = (Rng::new(5), Rng::new(5));
        assert_eq!(*pick_species(&[(1.0, 'x')], &mut a), 'x');
        assert_eq!(a.next_u64(), b.next_u64(), "a single species rolls nothing");
        let list = [(0.25, 0), (0.75, 1)];
        let ones: usize = (0..4000).map(|_| *pick_species(&list, &mut a)).sum();
        assert!((2800..3200).contains(&ones), "{ones} of 4000");
    }
}
