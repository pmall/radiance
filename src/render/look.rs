//! Mood palettes ("looks") keyed by time of day and interpolated between, one set per biome
//! (`biome::Atmosphere`), and the blend of biomes the player's sky is drawn in.
//! Colors are authored as sRGB hex and stored linear.

use glam::{Vec3, vec3};

use crate::daycycle::DayCycle;
use crate::world::biome::{BIOMES, Biome, MAX_BIOMES};

#[derive(Clone, Copy, Debug)]
pub struct Look {
    pub sky_zenith: Vec3,
    pub sky_horizon: Vec3,
    /// Color of the sun or moon disk.
    pub disk: Vec3,
    pub stars: f32,
    /// Key light (sun by day, moon by night), intensity baked in.
    pub light: Vec3,
    pub ambient_sky: Vec3,
    pub ambient_ground: Vec3,
    pub fog: Vec3,
    /// Fog density per meter at ground level.
    pub fog_density: f32,
    /// How quickly fog thins with height (per meter).
    pub fog_falloff: f32,
    /// Brightening of fog toward the key light.
    pub fog_glow: f32,
    pub exposure: f32,
    pub saturation: f32,
    pub contrast: f32,
    /// Split toning: tint pushed into shadows and highlights.
    pub shadow_tint: Vec3,
    pub highlight_tint: Vec3,
    /// Strength of plant emission and plant lights (faint by day, full at night).
    pub plant_glow: f32,
}

/// sRGB hex to linear RGB.
const fn hex(c: u32) -> Vec3 {
    vec3(srgb_channel(c >> 16), srgb_channel(c >> 8), srgb_channel(c))
}

const fn srgb_channel(c: u32) -> f32 {
    let s = (c & 0xff) as f32 / 255.0;
    // Close enough to the sRGB curve for authoring purposes, and const-friendly.
    s * s * (s * 0.2 + 0.8)
}

const NIGHT: Look = Look {
    sky_zenith: hex(0x05060f),
    sky_horizon: hex(0x221a40),
    disk: hex(0xd8e0ff),
    stars: 1.0,
    light: vec3(0.10, 0.13, 0.24),
    ambient_sky: hex(0x262c58),
    ambient_ground: hex(0x0c0a1a),
    fog: hex(0x1a1436),
    fog_density: 0.012,
    fog_falloff: 0.03,
    fog_glow: 0.3,
    exposure: 1.4,
    saturation: 1.1,
    contrast: 1.05,
    shadow_tint: hex(0x2a3a80),
    highlight_tint: hex(0xc8b8ff),
    plant_glow: 1.0,
};

const DAWN: Look = Look {
    sky_zenith: hex(0x6a7aa8),
    sky_horizon: hex(0xf0d0c8),
    disk: hex(0xfff0e0),
    stars: 0.0,
    light: vec3(1.25, 0.95, 0.80),
    ambient_sky: hex(0x8c98c4),
    ambient_ground: hex(0x4a4660),
    fog: hex(0xc4c4da),
    fog_density: 0.010,
    fog_falloff: 0.035,
    fog_glow: 0.6,
    exposure: 1.0,
    saturation: 0.8,
    contrast: 0.95,
    shadow_tint: hex(0x6878c0),
    highlight_tint: hex(0xffe0d8),
    plant_glow: 0.35,
};

const DAY: Look = Look {
    sky_zenith: hex(0x4a90b4),
    sky_horizon: hex(0xe0d0a4),
    disk: hex(0xfffbe8),
    stars: 0.0,
    light: vec3(2.1, 1.85, 1.5),
    ambient_sky: hex(0x88aec4),
    ambient_ground: hex(0x6a5e48),
    fog: hex(0xd4c090),
    fog_density: 0.0055,
    fog_falloff: 0.02,
    fog_glow: 0.5,
    exposure: 1.0,
    saturation: 1.0,
    contrast: 1.0,
    shadow_tint: hex(0x3c7890),
    highlight_tint: hex(0xfff0d0),
    plant_glow: 0.05,
};

const DUSK: Look = Look {
    sky_zenith: hex(0x2c2060),
    sky_horizon: hex(0xff7a50),
    disk: hex(0xffc080),
    stars: 0.2,
    light: vec3(1.8, 0.85, 0.45),
    ambient_sky: hex(0x6a4c94),
    ambient_ground: hex(0x3a2244),
    fog: hex(0x9a4a78),
    fog_density: 0.011,
    fog_falloff: 0.025,
    fog_glow: 1.0,
    exposure: 1.2,
    saturation: 1.2,
    contrast: 1.05,
    shadow_tint: hex(0x5030a0),
    highlight_tint: hex(0xffb080),
    plant_glow: 0.85,
};

/// A biome's looks at the four times of day the cycle blends between.
#[derive(Clone, Copy, Debug)]
pub struct Looks {
    pub night: Look,
    pub dawn: Look,
    pub day: Look,
    pub dusk: Look,
}

/// The original looks of the city (the default biome's).
pub const DEFAULT_LOOKS: Looks = Looks {
    night: NIGHT,
    dawn: DAWN,
    day: DAY,
    dusk: DUSK,
};

/// The Cyan Grotto's looks: cold and aquatic, a teal fog at every hour, pale cyan days, a cold
/// teal sunset instead of the warm one, deep blue-green nights where the cyan flowers glow.
pub const GROTTO_LOOKS: Looks = Looks {
    night: Look {
        sky_zenith: hex(0x030a10),
        sky_horizon: hex(0x0e2a36),
        disk: hex(0xd0f0ff),
        stars: 0.8,
        light: vec3(0.07, 0.13, 0.20),
        ambient_sky: hex(0x163848),
        ambient_ground: hex(0x06141a),
        fog: hex(0x0c2a34),
        fog_density: 0.016,
        fog_falloff: 0.03,
        fog_glow: 0.3,
        exposure: 1.45,
        saturation: 1.05,
        contrast: 1.05,
        shadow_tint: hex(0x1a5a78),
        highlight_tint: hex(0xb8f0ff),
        plant_glow: 1.0,
    },
    dawn: Look {
        sky_zenith: hex(0x4a7a90),
        sky_horizon: hex(0xc8e0e0),
        disk: hex(0xf0fff8),
        stars: 0.0,
        light: vec3(0.95, 1.0, 1.0),
        ambient_sky: hex(0x7ab0c0),
        ambient_ground: hex(0x34505a),
        fog: hex(0x9cc8cc),
        fog_density: 0.013,
        fog_falloff: 0.035,
        fog_glow: 0.5,
        exposure: 1.0,
        saturation: 0.75,
        contrast: 0.95,
        shadow_tint: hex(0x3a8098),
        highlight_tint: hex(0xe0fff8),
        plant_glow: 0.4,
    },
    day: Look {
        sky_zenith: hex(0x3a8aa0),
        sky_horizon: hex(0xb8dcd8),
        disk: hex(0xf4fffc),
        stars: 0.0,
        light: vec3(1.7, 1.9, 1.9),
        ambient_sky: hex(0x7ab8c8),
        ambient_ground: hex(0x3c5a5c),
        fog: hex(0x88c0c4),
        fog_density: 0.0085,
        fog_falloff: 0.02,
        fog_glow: 0.4,
        exposure: 1.0,
        saturation: 0.9,
        contrast: 1.0,
        shadow_tint: hex(0x2c7a90),
        highlight_tint: hex(0xe8fff8),
        plant_glow: 0.1,
    },
    dusk: Look {
        sky_zenith: hex(0x14304a),
        sky_horizon: hex(0x5ab0b8),
        disk: hex(0xc8f0ff),
        stars: 0.25,
        light: vec3(0.8, 1.1, 1.3),
        ambient_sky: hex(0x3a6a88),
        ambient_ground: hex(0x18303c),
        fog: hex(0x2a6a78),
        fog_density: 0.013,
        fog_falloff: 0.025,
        fog_glow: 0.8,
        exposure: 1.25,
        saturation: 1.1,
        contrast: 1.05,
        shadow_tint: hex(0x204a90),
        highlight_tint: hex(0xa0f0ff),
        plant_glow: 0.85,
    },
};

#[derive(Clone, Copy)]
enum Phase {
    Night,
    Dawn,
    Day,
    Dusk,
}

/// (time of day, look). Must be sorted; wraps around midnight.
const KEYS: &[(f32, Phase)] = &[
    (0.0, Phase::Night),
    (0.20, Phase::Night),
    (0.27, Phase::Dawn),
    (0.36, Phase::Day),
    (0.64, Phase::Day),
    (0.74, Phase::Dusk),
    (0.82, Phase::Night),
];

/// Times for the debug jump keys: dawn, day, dusk, night.
pub const PRESET_TIMES: [f32; 4] = [0.27, 0.5, 0.74, 0.0];

impl Look {
    fn lerp(&self, o: &Look, k: f32) -> Look {
        let v = |a: Vec3, b: Vec3| a.lerp(b, k);
        let f = |a: f32, b: f32| a + (b - a) * k;
        Look {
            sky_zenith: v(self.sky_zenith, o.sky_zenith),
            sky_horizon: v(self.sky_horizon, o.sky_horizon),
            disk: v(self.disk, o.disk),
            stars: f(self.stars, o.stars),
            light: v(self.light, o.light),
            ambient_sky: v(self.ambient_sky, o.ambient_sky),
            ambient_ground: v(self.ambient_ground, o.ambient_ground),
            fog: v(self.fog, o.fog),
            fog_density: f(self.fog_density, o.fog_density),
            fog_falloff: f(self.fog_falloff, o.fog_falloff),
            fog_glow: f(self.fog_glow, o.fog_glow),
            exposure: f(self.exposure, o.exposure),
            saturation: f(self.saturation, o.saturation),
            contrast: f(self.contrast, o.contrast),
            shadow_tint: v(self.shadow_tint, o.shadow_tint),
            highlight_tint: v(self.highlight_tint, o.highlight_tint),
            plant_glow: f(self.plant_glow, o.plant_glow),
        }
    }
}

impl Looks {
    fn get(&self, phase: Phase) -> &Look {
        match phase {
            Phase::Night => &self.night,
            Phase::Dawn => &self.dawn,
            Phase::Day => &self.day,
            Phase::Dusk => &self.dusk,
        }
    }

    /// The look at time of day `t`.
    pub fn at(&self, t: f32) -> Look {
        let i = KEYS.iter().rposition(|(kt, _)| *kt <= t).unwrap_or(0);
        let (t0, a) = KEYS[i];
        let (t1, b) = KEYS
            .get(i + 1)
            .copied()
            .unwrap_or((1.0 + KEYS[0].0, KEYS[0].1));
        let k = ((t - t0) / (t1 - t0)).clamp(0.0, 1.0);
        self.get(a).lerp(self.get(b), k * k * (3.0 - 2.0 * k))
    }
}

/// Seconds the sky takes to turn to a biome's atmosphere once the player has crossed into it.
const FADE: f32 = 4.0;

/// How much each biome's atmosphere counts in the sky right now: the player's biome, fading in
/// over `FADE` seconds when they cross into another. (Borders themselves do not blend: the far
/// side of one is drawn in the player's atmosphere.)
#[derive(Clone, Copy, Debug)]
pub struct Blend {
    /// Per biome (`biome::variant_of`), summing to 1.
    weights: [f32; MAX_BIOMES],
}

impl Blend {
    /// All in one biome, `variant` (`biome::variant_of`).
    pub fn new(variant: usize) -> Self {
        let mut weights = [0.0; MAX_BIOMES];
        weights[variant] = 1.0;
        Self { weights }
    }

    /// `dt` seconds spent in biome `variant`: it gains on the others.
    pub fn update(&mut self, variant: usize, dt: f32) {
        let step = dt / FADE;
        for (i, w) in self.weights.iter_mut().enumerate() {
            if i != variant {
                *w = (*w - step).max(0.0);
            }
        }
        let others: f32 = (self.weights.iter().sum::<f32>() - self.weights[variant]).max(0.0);
        self.weights[variant] = 1.0 - others;
    }

    /// The biomes that count, with their weights.
    fn parts(&self) -> impl Iterator<Item = (f32, &'static Biome)> + '_ {
        BIOMES
            .iter()
            .zip(self.weights)
            .filter(|(_, w)| *w > 0.0)
            .map(|(b, w)| (w, *b))
    }

    /// The weighted mean of `f` over the biomes that count (exactly `f` of the one biome when
    /// only one does).
    fn mean<T>(&self, f: impl Fn(&'static Biome) -> T, lerp: impl Fn(&T, &T, f32) -> T) -> T {
        let mut parts = self.parts();
        let (w0, b0) = parts.next().unwrap_or((1.0, BIOMES[0]));
        let (mut acc, mut total) = (f(b0), w0);
        for (w, b) in parts {
            total += w;
            acc = lerp(&acc, &f(b), w / total);
        }
        acc
    }
}

/// Everything the shaders need about the current time of day and the air around the player.
pub struct Sky {
    pub look: Look,
    /// Unit vector toward the key light (sun or moon).
    pub light_dir: Vec3,
    /// Tint of the day's pollen motes and colors of the night's spores (display space).
    pub pollen: Vec3,
    pub spores: [Vec3; 4],
    /// The color script by height (`Atmosphere::heights`): depths, middle, canopy, linear, each
    /// scaled to a luminance of 1.
    pub heights: [Vec3; 3],
}

impl Sky {
    /// The sky at the cycle's time of day, in the atmosphere of the biomes of `blend`.
    pub fn from_cycle(cycle: &DayCycle, blend: &Blend) -> Self {
        let mut look = blend.mean(|b| b.atmosphere.looks.at(cycle.t), |a, b, k| a.lerp(b, k));
        let sun = cycle.sun_dir();
        // The key light swaps from sun to moon at the horizon; fade it out around the swap.
        let light_dir = if sun.y >= 0.0 { sun } else { -sun };
        let fade = (light_dir.y / 0.08).clamp(0.0, 1.0);
        look.light *= fade * fade * (3.0 - 2.0 * fade);
        let pollen = blend.mean(|b| Vec3::from(b.atmosphere.pollen), |a, b, k| a.lerp(*b, k));
        let spores = blend.mean(
            |b| b.atmosphere.spores.map(Vec3::from),
            |a, b, k| std::array::from_fn(|i| a[i].lerp(b[i], k)),
        );
        let heights = blend
            .mean(
                |b| b.atmosphere.heights.map(Vec3::from),
                |a, b, k| std::array::from_fn(|i| a[i].lerp(b[i], k)),
            )
            .map(|c| {
                let c = c.powf(2.2);
                c / c.dot(vec3(0.2126, 0.7152, 0.0722)).max(1e-4)
            });
        Self {
            look,
            light_dir,
            pollen,
            spores,
            heights,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sky_turns_to_a_new_biome_within_the_fade() {
        let mut blend = Blend::new(0);
        assert_eq!(blend.weights[0], 1.0);
        let other = 1.min(MAX_BIOMES - 1);
        blend.update(other, FADE * 0.5);
        let sum: f32 = blend.weights.iter().sum();
        assert!((sum - 1.0).abs() < 1e-5, "weights sum to {sum}");
        assert!((blend.weights[other] - 0.5).abs() < 1e-5);
        blend.update(other, FADE);
        assert_eq!(blend.weights[other], 1.0);
        assert_eq!(blend.weights[0], 0.0);
    }

    #[test]
    fn one_biome_is_drawn_in_exactly_its_looks() {
        let blend = Blend::new(0);
        for t in [0.0, 0.3, 0.5, 0.77] {
            let a = blend.mean(|b| b.atmosphere.looks.at(t), |a, b, k| a.lerp(b, k));
            let b = BIOMES[0].atmosphere.looks.at(t);
            assert_eq!(a.fog, b.fog);
            assert_eq!(a.sky_zenith, b.sky_zenith);
        }
    }
}
