//! The city's design in one place: what each floor is like (decks, plants, flowers, stairs, props,
//! cars) and the switches for props and points of interest. Generation tunes nothing elsewhere;
//! `--report` prints these profiles next to what a seed actually produced.
//!
//! Profiles are written per zone of the stack (`floors::Zone`), as the values at the zone's lowest
//! and highest floor, and interpolated in between, so they hold for any number of floors.
//! The lore behind the numbers (`docs/lore.md`): people lived longest in the depths and moved up
//! with the city, so their leftovers (props, cars, stairs) thin out going up; flowers follow the
//! sun like real flowers, so they grow denser toward the canopy; the canopy was built by machines
//! after the humans, so it holds almost nothing human.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use super::floors::{self, Zone};

/// Chances that a flower grows, per source, on a floor.
#[derive(Clone, Copy, Debug)]
pub struct Flowers {
    /// Per bush.
    pub bush: f32,
    /// Per hanging vine.
    pub vine: f32,
    /// Per branch of a wall climber.
    pub climber: f32,
    /// Per lot: a point of interest with flowers (an armchair, a bench, a pot).
    pub poi: f32,
}

/// Everything generation needs to know about one floor.
#[derive(Clone, Copy, Debug)]
pub struct FloorProfile {
    /// Chance that a deck section is built up to this floor (`city::built`): the share of the floor
    /// that has a deck. Never grows going up (a section with no deck on a floor has none above).
    pub deck_keep: f32,
    /// Multiplier on how many plants grow.
    pub plants: f32,
    pub flowers: Flowers,
    /// Chance that a lot without a tower holds the stair from this floor to the next.
    pub stair: f32,
    /// Multiplier on the props tried per lot (`dressing.rs`).
    pub props: f32,
    /// Cars tried per lot (parked along the curbs).
    pub cars: f32,
}

/// Most flowers per lot and floor (their lights are what the renderer can afford).
pub const MAX_FLOWERS_PER_LOT: usize = 24;
/// Least distance between two flowers, so their glows never pile up (owner's rule).
pub const MIN_FLOWER_GAP: f32 = 3.0;
/// Under a crater the sun gets in: plants grow as in the canopy, flowers this much more often
/// than on their floor.
pub const CRATER_FLOWERS: f32 = 1.5;

/// Profiles at the lowest and highest floor of each zone.
///
/// Plants and flowers rise with the light and culminate on the canopy's first floor, then thin
/// out toward the construction front, too new for much to have grown yet. Stairs grow rarer
/// smoothly all the way up.
const fn zone_profiles(zone: Zone) -> (FloorProfile, FloorProfile) {
    match zone {
        Zone::Depths => (
            FloorProfile {
                deck_keep: 0.97,
                plants: 0.45,
                flowers: Flowers {
                    bush: 0.2,
                    vine: 0.2,
                    climber: 0.04,
                    poi: 0.03,
                },
                stair: 0.16,
                props: 1.0,
                cars: 1.0,
            },
            FloorProfile {
                deck_keep: 0.95,
                plants: 0.5,
                flowers: Flowers {
                    bush: 0.22,
                    vine: 0.22,
                    climber: 0.045,
                    poi: 0.03,
                },
                stair: 0.12,
                props: 0.9,
                cars: 0.9,
            },
        ),
        Zone::Middle => (
            FloorProfile {
                deck_keep: 0.92,
                plants: 0.8,
                flowers: Flowers {
                    bush: 0.28,
                    vine: 0.28,
                    climber: 0.055,
                    poi: 0.02,
                },
                stair: 0.1,
                props: 0.7,
                cars: 0.6,
            },
            FloorProfile {
                deck_keep: 0.8,
                plants: 1.2,
                flowers: Flowers {
                    bush: 0.4,
                    vine: 0.4,
                    climber: 0.08,
                    poi: 0.01,
                },
                stair: 0.065,
                props: 0.4,
                cars: 0.2,
            },
        ),
        Zone::Canopy => (
            FloorProfile {
                deck_keep: 0.65,
                plants: 1.6,
                flowers: Flowers {
                    bush: 0.65,
                    vine: 0.65,
                    climber: 0.13,
                    poi: 0.0,
                },
                stair: 0.05,
                props: 0.12,
                cars: 0.0,
            },
            FloorProfile {
                deck_keep: 0.45,
                plants: 0.8,
                flowers: Flowers {
                    bush: 0.3,
                    vine: 0.3,
                    climber: 0.06,
                    poi: 0.0,
                },
                stair: 0.03,
                props: 0.05,
                cars: 0.0,
            },
        ),
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn blend(a: &FloorProfile, b: &FloorProfile, t: f32) -> FloorProfile {
    let f = |x: f32, y: f32| lerp(x, y, t);
    FloorProfile {
        deck_keep: f(a.deck_keep, b.deck_keep),
        plants: f(a.plants, b.plants),
        flowers: Flowers {
            bush: f(a.flowers.bush, b.flowers.bush),
            vine: f(a.flowers.vine, b.flowers.vine),
            climber: f(a.flowers.climber, b.flowers.climber),
            poi: f(a.flowers.poi, b.flowers.poi),
        },
        stair: f(a.stair, b.stair),
        props: f(a.props, b.props),
        cars: f(a.cars, b.cars),
    }
}

/// The profile of a floor.
pub fn profile(floor: usize) -> &'static FloorProfile {
    static ALL: OnceLock<Vec<FloorProfile>> = OnceLock::new();
    let all = ALL.get_or_init(|| {
        let mut v: Vec<FloorProfile> = (0..floors::count())
            .map(|f| {
                let (lo, hi) = zone_profiles(floors::zone(f));
                blend(&lo, &hi, floors::zone_t(f))
            })
            .collect();
        // The share of the floor with a deck only shrinks going up.
        for f in 1..v.len() {
            v[f].deck_keep = v[f].deck_keep.min(v[f - 1].deck_keep);
        }
        // A lot holds one stair at most: keep the chances summing to at most 1.
        let total: f32 = v.iter().take(floors::count() - 1).map(|p| p.stair).sum();
        if total > 1.0 {
            v.iter_mut().for_each(|p| p.stair /= total);
        }
        // No stair leaves the top floor.
        if let Some(top) = v.last_mut() {
            top.stair = 0.0;
        }
        v
    });
    &all[floor.min(all.len() - 1)]
}

/// The roll that picks a lot's stair (`city::stair`): the profile's chance, made up for the lots
/// whose section is not built up to the stair's top (where the stair is dropped).
pub fn stair_roll(floor: usize) -> f32 {
    static ALL: OnceLock<Vec<f32>> = OnceLock::new();
    let all = ALL.get_or_init(|| {
        let n = floors::count();
        let mut v: Vec<f32> = (0..n)
            .map(|f| match f + 1 < n {
                true => profile(f).stair / profile(f + 1).deck_keep.max(0.05),
                false => 0.0,
            })
            .collect();
        let total: f32 = v.iter().sum();
        if total > 1.0 {
            v.iter_mut().for_each(|w| *w /= total);
        }
        v
    });
    all.get(floor).copied().unwrap_or(0.0)
}

/// The profile used for growth under a crater, where the sun gets in.
pub fn sunlit(floor: usize) -> FloorProfile {
    let p = profile(floor);
    let canopy = zone_profiles(Zone::Canopy).0;
    // Flowers come per plant: compensate for the extra plants, so the crater ends up with
    // `CRATER_FLOWERS` times its floor's flowers.
    let k = CRATER_FLOWERS * p.plants / canopy.plants;
    let f = |x: f32| (x * k).min(1.0);
    FloorProfile {
        plants: canopy.plants,
        flowers: Flowers {
            bush: f(p.flowers.bush),
            vine: f(p.flowers.vine),
            climber: f(p.flowers.climber),
            ..p.flowers
        },
        ..*p
    }
}

static PROPS: AtomicBool = AtomicBool::new(true);
static POIS: AtomicBool = AtomicBool::new(true);

/// Whether props are placed (debug switch, O). Changing it calls for regenerating the world.
pub fn props_on() -> bool {
    PROPS.load(Ordering::Relaxed)
}

/// Whether points of interest are placed (debug switch, I).
pub fn pois_on() -> bool {
    POIS.load(Ordering::Relaxed)
}

pub fn set_props(on: bool) {
    PROPS.store(on, Ordering::Relaxed);
}

pub fn set_pois(on: bool) {
    POIS.store(on, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiles_follow_the_lore() {
        let n = floors::count();
        let (bottom, top) = (profile(0), profile(n - 1));
        assert!(
            top.flowers.bush > bottom.flowers.bush,
            "flowers follow the sun"
        );
        assert!(
            top.props < bottom.props,
            "human leftovers thin out going up"
        );
        assert_eq!(top.cars, 0.0, "no cars in the machine-built canopy");
        assert_eq!(top.stair, 0.0, "no stair above the top");
        let stairs: f32 = (0..n).map(|f| profile(f).stair).sum();
        assert!(stairs <= 1.0 + 1e-4);
        let peak = (0..n).find(|&f| floors::zone(f) == Zone::Canopy).unwrap();
        assert!(
            (0..n).all(|f| profile(f).flowers.bush <= profile(peak).flowers.bush),
            "flowers culminate on the canopy's first floor"
        );
        assert!(
            (1..n - 1).all(|f| profile(f).stair <= profile(f - 1).stair),
            "stairs grow rarer going up"
        );
        assert!(
            (1..n).all(|f| profile(f).deck_keep <= profile(f - 1).deck_keep),
            "decks thin out going up"
        );
    }
}
