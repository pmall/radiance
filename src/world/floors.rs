//! The finite stack of floors the city is built in, bottom to top. Every height rule of the city
//! (decks, zones, the shaders' cover lookup, the souls' register) reads from here.
//!
//! The number of floors is fixed for a run but chosen at launch (`--floors N`, `init`); floors are
//! `SPACING` apart and the player starts on `START`. Deeper is older: D0 is the original ground,
//! each floor above was laid later, and the construction front rises above the top deck
//! (`docs/lore.md`). The stack is split into three zones, which the floor profiles of `design.rs`
//! are written for, so they adapt to any number of floors:
//! - **depths**: D0 to the start floor, the old human city;
//! - **middle**: the floors people moved up to as the city rose;
//! - **canopy**: the top half of the floors above the depths, built by machines after the humans.

use std::sync::OnceLock;

/// Floors when `init` is not called.
pub const DEFAULT_COUNT: usize = 9;
/// Most floors the shaders and the cover map can handle.
pub const MAX: usize = 16;
/// Height between two floors' walking surfaces, in meters.
pub const SPACING: f32 = 36.0;
/// Floor the player wakes up on, with the older floors below.
pub const START: usize = 2;

static COUNT: OnceLock<usize> = OnceLock::new();

/// Sets the number of floors for this run (before anything is generated).
pub fn init(count: usize) {
    let count = count.clamp(START + 3, MAX);
    let _ = COUNT.set(count);
}

/// Number of floors.
pub fn count() -> usize {
    *COUNT.get_or_init(|| DEFAULT_COUNT)
}

/// Height of a floor's walking surface (top of its deck), in meters. The start floor is at 0.
pub fn y(floor: usize) -> f32 {
    (floor as f32 - START as f32) * SPACING
}

pub fn bottom() -> f32 {
    y(0)
}

pub fn top() -> f32 {
    y(count() - 1)
}

/// The floor whose walking surface is at or below `y`.
pub fn floor_at(y: f32) -> usize {
    (((y + 0.5 - bottom()) / SPACING).floor().max(0.0) as usize).min(count() - 1)
}

/// 0 at the bottom floor, 1 at the top one.
pub fn depth01(y: f32) -> f32 {
    ((y - bottom()) / (top() - bottom())).clamp(0.0, 1.0)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Zone {
    Depths,
    Middle,
    Canopy,
}

/// Floors of the canopy in a stack of `count`: the top half of those above the depths (at least
/// one).
fn canopy_floors(count: usize) -> usize {
    ((count - START - 1) / 2).max(1)
}

fn zone_in(count: usize, floor: usize) -> Zone {
    if floor <= START {
        Zone::Depths
    } else if floor >= count - canopy_floors(count) {
        Zone::Canopy
    } else {
        Zone::Middle
    }
}

pub fn zone(floor: usize) -> Zone {
    zone_in(count(), floor)
}

/// Position of a floor within its zone: 0 at the zone's lowest floor, 1 at its highest.
pub fn zone_t(floor: usize) -> f32 {
    zone_t_in(count(), floor)
}

fn zone_t_in(count: usize, floor: usize) -> f32 {
    let canopy = canopy_floors(count);
    let (lo, hi) = match zone_in(count, floor) {
        Zone::Depths => (0, START),
        Zone::Middle => (START + 1, count - canopy - 1),
        Zone::Canopy => (count - canopy, count - 1),
    };
    if hi > lo {
        (floor - lo) as f32 / (hi - lo) as f32
    } else {
        0.0
    }
}

/// "D2 service level", "D5 middle", ...
pub fn name(floor: usize) -> String {
    let what = match floor {
        0 => "foundations",
        1 => "lower streets",
        START => "service level",
        f if f == count() - 1 => "canopy top",
        f => match zone(f) {
            Zone::Depths => "depths",
            Zone::Middle => "middle",
            Zone::Canopy => "canopy",
        },
    };
    format!("D{floor} {what}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floors_rise_and_zones_follow() {
        assert!((1..count()).all(|f| y(f - 1) < y(f)));
        assert_eq!(y(START), 0.0);
        assert_eq!(floor_at(y(START) + 1.7), START);
        assert_eq!(floor_at(bottom() - 5.0), 0);
        assert_eq!(floor_at(top() + 50.0), count() - 1);
        assert_eq!(zone(START), Zone::Depths);
        assert_eq!(zone(count() - 1), Zone::Canopy);
        assert!((0..count()).any(|f| zone(f) == Zone::Middle));
    }

    #[test]
    fn zones_hold_for_any_floor_count() {
        for n in START + 3..=MAX {
            let zones: Vec<Zone> = (0..n).map(|f| zone_in(n, f)).collect();
            // Depths, then middle, then canopy, each present, in that order.
            assert_eq!(zones[0], Zone::Depths);
            assert_eq!(zones[n - 1], Zone::Canopy);
            assert!(zones.contains(&Zone::Middle), "{n} floors: {zones:?}");
            assert!(zones.windows(2).all(|w| w[0] as u8 <= w[1] as u8));
            assert!((0..n).all(|f| (0.0..=1.0).contains(&zone_t_in(n, f))));
        }
    }
}
