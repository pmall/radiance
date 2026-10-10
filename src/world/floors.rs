//! The finite stack of floors the city is built in, bottom to top. Every height rule of the city
//! (decks, height bands, the shaders' cover lookup, the souls' register) reads from here.
//!
//! Deeper is older: D0 is the original ground, each floor above was laid later on top of the
//! previous ones, and the construction front rises above the top deck. The player starts on D2,
//! with two older floors below and the whole city above (see `docs/lore.md`).

/// Number of floors.
pub const COUNT: usize = 9;

/// Height of each floor's walking surface (top of its deck), in meters.
pub const Y: [f32; COUNT] = [-80.0, -36.0, 0.0, 36.0, 72.0, 108.0, 144.0, 180.0, 216.0];

/// Floor the player wakes up on.
pub const START: usize = 2;

pub const BOTTOM: f32 = Y[0];
pub const TOP: f32 = Y[COUNT - 1];

/// Short names for the debug overlay.
pub const NAMES: [&str; COUNT] = [
    "D0 foundations",
    "D1 lower streets",
    "D2 service level",
    "D3 middle",
    "D4 middle",
    "D5 middle",
    "D6 canopy",
    "D7 canopy",
    "D8 canopy top",
];

/// Height band of a floor, indexing the biome's per-band tables: 0 canopy, 1 middle, 2 depths.
pub const fn band(floor: usize) -> usize {
    match floor {
        0..=2 => 2,
        3..=5 => 1,
        _ => 0,
    }
}

/// The floor whose walking surface is at or below `y`.
pub fn floor_at(y: f32) -> usize {
    Y.iter().rposition(|&f| f <= y + 0.5).unwrap_or(0)
}

/// Height band at `y` (see `band`).
pub fn band_at(y: f32) -> usize {
    band(floor_at(y))
}

/// 0 at the bottom floor, 1 at the top one.
pub fn depth01(y: f32) -> f32 {
    ((y - BOTTOM) / (TOP - BOTTOM)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floors_rise_and_bands_follow() {
        assert!(Y.windows(2).all(|w| w[0] < w[1]));
        assert_eq!(floor_at(Y[START] + 1.7), START);
        assert_eq!(floor_at(BOTTOM - 5.0), 0);
        assert_eq!(band_at(Y[START]), 2);
        assert_eq!(band_at(TOP + 50.0), 0);
    }
}
