//! The nearest ways to another floor from where the player stands: the stair up, the stair down
//! and the crater, within about a kilometer. Things beyond the loaded lots exist (the city is a
//! pure function of the seed) but are not drawn until the player gets closer, so each is marked
//! loaded or not. The debug overlay shows them in test mode (`debug.rs`).

use glam::Vec3;

use super::{LOAD_RADIUS, chunks, city, floors};

/// How far the finder looks, in lots around the player (about 1 km).
pub const FIND_RADIUS: i32 = 40;

/// A way to another floor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Way {
    /// A stair up to floor `to`.
    StairUp { to: usize },
    /// A stair down to floor `to`.
    StairDown { to: usize },
    /// A crater, open above floor `above`.
    Crater { above: usize },
}

/// One way found: what, where (the center of its lot, at the player's height), and whether its
/// lot is loaded.
#[derive(Clone, Copy, Debug)]
pub struct Found {
    pub way: Way,
    pub at: Vec3,
    pub loaded: bool,
}

/// Nearest stairs and crater from the player's lot and floor, recomputed when either changes (or
/// the seed, or the chunk files).
#[derive(Default)]
pub struct Finder {
    key: Option<(u64, u64, i32, i32, usize)>,
    found: Vec<Found>,
}

impl Finder {
    /// What was found at the last `update`: the stair up, the stair down and the crater, those
    /// within `FIND_RADIUS`, in that order.
    pub fn found(&self) -> &[Found] {
        &self.found
    }

    pub fn update(&mut self, seed: u64, pos: Vec3) {
        let (lx, lz) = city::lot_of(pos);
        let floor = floors::floor_at(pos.y);
        let key = (seed, chunks::generation(), lx, lz, floor);
        if self.key == Some(key) {
            return;
        }
        self.key = Some(key);
        let center = |x: i32, z: i32| {
            glam::vec3(
                (x as f32 + 0.5) * city::LOT,
                pos.y,
                (z as f32 + 0.5) * city::LOT,
            )
        };
        let mut up: Option<(f32, Way, Vec3)> = None;
        let mut down = up;
        let mut crater = up;
        let keep = |best: &mut Option<(f32, Way, Vec3)>, p: Vec3, way: Way| {
            let d = p.distance_squared(pos);
            if best.as_ref().is_none_or(|b| d < b.0) {
                *best = Some((d, way, p));
            }
        };
        // Ring by ring outward; stop once nothing farther out can beat what was found.
        let done = |best: &Option<(f32, Way, Vec3)>, d: i32| {
            best.as_ref()
                .is_some_and(|b| b.0.sqrt() < (d as f32 - 0.5) * city::LOT)
        };
        for d in 0..=FIND_RADIUS {
            if done(&up, d) && done(&down, d) && done(&crater, d) {
                break;
            }
            for z in lz - d..=lz + d {
                for x in lx - d..=lx + d {
                    if (x - lx).abs().max((z - lz).abs()) != d {
                        continue;
                    }
                    if let Some((lo, hi)) = city::stair(seed, x, z) {
                        if lo <= floor && floor < hi {
                            keep(&mut up, center(x, z), Way::StairUp { to: hi });
                        }
                        if lo < floor && floor <= hi {
                            keep(&mut down, center(x, z), Way::StairDown { to: lo });
                        }
                    }
                    if let Some(f) = city::crater_at(seed, x, z) {
                        keep(&mut crater, center(x, z), Way::Crater { above: f });
                    }
                }
            }
        }
        let loaded = |p: Vec3| {
            let (x, z) = city::lot_of(p);
            (x - lx).abs().max((z - lz).abs()) <= LOAD_RADIUS
        };
        self.found = [up, down, crater]
            .into_iter()
            .flatten()
            .map(|(_, way, at)| Found {
                way,
                at,
                loaded: loaded(at),
            })
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn points_to_a_stair_up_and_a_crater_from_the_spawn() {
        let mut f = Finder::default();
        f.update(1, glam::vec3(0.0, 1.7, 0.0));
        for found in f.found() {
            println!("{found:?}");
        }
        let has = |ok: fn(&Way) -> bool| f.found().iter().any(|found| ok(&found.way));
        assert!(has(|w| matches!(w, Way::StairUp { .. })));
        assert!(has(|w| matches!(w, Way::Crater { .. })));
    }

    #[test]
    fn finds_a_stair_up_even_from_high_floors() {
        for floor in 0..floors::count() - 1 {
            let mut f = Finder::default();
            f.update(1, glam::vec3(0.0, floors::y(floor) + 0.5, 0.0));
            assert!(
                f.found()
                    .iter()
                    .any(|found| matches!(found.way, Way::StairUp { .. })),
                "no stair up from D{floor} within 1 km"
            );
        }
    }
}
