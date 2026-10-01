//! Mood palettes ("looks") keyed by time of day and interpolated between.
//! Colors are authored as sRGB hex and stored linear.

use glam::{Vec3, vec3};

use crate::daycycle::DayCycle;

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
    pub ink: Vec3,
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
    ink: hex(0x05040c),
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
    ink: hex(0x22243a),
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
    ink: hex(0x1e2432),
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
    ink: hex(0x160a24),
    exposure: 1.2,
    saturation: 1.2,
    contrast: 1.05,
    shadow_tint: hex(0x5030a0),
    highlight_tint: hex(0xffb080),
    plant_glow: 0.85,
};

/// (time of day, look). Must be sorted; wraps around midnight.
const KEYS: &[(f32, Look)] = &[
    (0.0, NIGHT),
    (0.20, NIGHT),
    (0.27, DAWN),
    (0.36, DAY),
    (0.64, DAY),
    (0.74, DUSK),
    (0.82, NIGHT),
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
            ink: v(self.ink, o.ink),
            exposure: f(self.exposure, o.exposure),
            saturation: f(self.saturation, o.saturation),
            contrast: f(self.contrast, o.contrast),
            shadow_tint: v(self.shadow_tint, o.shadow_tint),
            highlight_tint: v(self.highlight_tint, o.highlight_tint),
            plant_glow: f(self.plant_glow, o.plant_glow),
        }
    }

    pub fn at(t: f32) -> Look {
        let i = KEYS.iter().rposition(|(kt, _)| *kt <= t).unwrap_or(0);
        let (t0, a) = KEYS[i];
        let (t1, b) = KEYS
            .get(i + 1)
            .copied()
            .unwrap_or((1.0 + KEYS[0].0, KEYS[0].1));
        let k = ((t - t0) / (t1 - t0)).clamp(0.0, 1.0);
        a.lerp(&b, k * k * (3.0 - 2.0 * k))
    }
}

/// Everything the shaders need about the current time of day.
pub struct Sky {
    pub look: Look,
    /// Unit vector toward the key light (sun or moon).
    pub light_dir: Vec3,
}

impl Sky {
    pub fn from_cycle(cycle: &DayCycle) -> Self {
        let mut look = Look::at(cycle.t);
        let sun = cycle.sun_dir();
        // The key light swaps from sun to moon at the horizon; fade it out around the swap.
        let light_dir = if sun.y >= 0.0 { sun } else { -sun };
        let fade = (light_dir.y / 0.08).clamp(0.0, 1.0);
        look.light *= fade * fade * (3.0 - 2.0 * fade);
        Self { look, light_dir }
    }
}
