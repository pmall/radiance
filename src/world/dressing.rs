//! Set dressing and points of interest: props placed on the decks by the seed, in small scenes.
//!
//! Dense in the depths, where people lived the longest (`docs/world.md`, "Set dressing"), sparse
//! in the middle floors, rare in the machine-built canopy. Some scenes are points of interest
//! from `docs/poi-catalog.md`: silent tableaux, some with flowers growing in human places (the
//! flowers are souls, `docs/lore.md`). A few always stand on the plaza where the player wakes up.

use glam::{Vec3, vec3};

use super::floors;
use super::props::{Placed, Prop};
use super::{Aabb, Block, BlockKind};
use crate::rng::{Rng, hash_coords};

const SALT_DRESSING: i32 = 9;

/// A prop of a scene, relative to the scene's anchor and heading.
struct Part {
    prop: Prop,
    /// Offset in the scene's frame (x right, z forward), meters.
    at: (f32, f32),
    /// Extra rotation, radians.
    turn: f32,
}

const fn part(prop: Prop, x: f32, z: f32, turn: f32) -> Part {
    Part {
        prop,
        at: (x, z),
        turn,
    }
}

/// A flower growing in the scene: position in the scene's frame (x, height, z).
type Bloom = (f32, f32, f32);

struct Scene {
    parts: &'static [Part],
    blooms: &'static [Bloom],
    /// Weight per height band: canopy, middle, depths.
    weight: [u32; 3],
    /// Only on these floors (inclusive), if set.
    floors: Option<(usize, usize)>,
}

use Prop::*;
use std::f32::consts::{FRAC_PI_2 as QUARTER, PI as HALF};

const SCENES: &[Scene] = &[
    // Street furniture and leftovers.
    Scene {
        parts: &[part(Bench, 0.0, 0.0, 0.0)],
        blooms: &[],
        weight: [1, 3, 6],
        floors: None,
    },
    Scene {
        parts: &[
            part(Trashcan, 0.0, 0.0, 0.0),
            part(BoxClosed, 0.8, 0.1, 0.3),
            part(BoxOpen, 0.7, 0.9, -0.2),
        ],
        blooms: &[],
        weight: [0, 2, 5],
        floors: None,
    },
    Scene {
        parts: &[
            part(DumpsterClosed, 0.0, 0.0, 0.0),
            part(Pallet, 2.0, 0.2, 0.2),
        ],
        blooms: &[],
        weight: [0, 2, 4],
        floors: None,
    },
    Scene {
        parts: &[part(DumpsterOpen, 0.0, 0.0, 0.0)],
        blooms: &[],
        weight: [0, 1, 3],
        floors: None,
    },
    Scene {
        parts: &[
            part(Barrel, 0.0, 0.0, 0.0),
            part(Barrel, 0.75, 0.2, 1.0),
            part(Crate, -0.2, 0.9, 0.1),
        ],
        blooms: &[],
        weight: [1, 3, 3],
        floors: None,
    },
    Scene {
        parts: &[
            part(Cone, 0.0, 0.0, 0.0),
            part(Cone, 1.2, 0.0, 0.0),
            part(RoadBarrier, 0.6, 0.6, 0.0),
        ],
        blooms: &[],
        weight: [1, 2, 3],
        floors: None,
    },
    Scene {
        parts: &[part(BarrierConcrete, 0.0, 0.0, 0.0)],
        blooms: &[],
        weight: [1, 2, 3],
        floors: None,
    },
    Scene {
        parts: &[part(Barrier, 0.0, 0.0, 0.0)],
        blooms: &[],
        weight: [0, 1, 3],
        floors: None,
    },
    Scene {
        parts: &[part(ShoppingCart, 0.0, 0.0, 0.0)],
        blooms: &[],
        weight: [0, 1, 3],
        floors: None,
    },
    Scene {
        parts: &[part(Tire, 0.0, 0.0, 0.0), part(Tire, 0.5, 0.6, 0.5)],
        blooms: &[],
        weight: [0, 1, 2],
        floors: None,
    },
    Scene {
        parts: &[part(StreetSign, 0.0, 0.0, 0.0)],
        blooms: &[],
        weight: [0, 1, 2],
        floors: None,
    },
    Scene {
        parts: &[part(TrafficLight, 0.0, 0.0, 0.0)],
        blooms: &[],
        weight: [0, 0, 2],
        floors: Some((0, 1)),
    },
    // Old cars, mostly on the oldest floors.
    Scene {
        parts: &[part(Sedan, 0.0, 0.0, 0.0)],
        blooms: &[],
        weight: [0, 0, 3],
        floors: Some((0, 1)),
    },
    Scene {
        parts: &[part(Hatchback, 0.0, 0.0, 0.0)],
        blooms: &[],
        weight: [0, 0, 2],
        floors: Some((0, 1)),
    },
    Scene {
        parts: &[part(Van, 0.0, 0.0, 0.0)],
        blooms: &[],
        weight: [0, 0, 2],
        floors: Some((0, 1)),
    },
    Scene {
        parts: &[part(Taxi, 0.0, 0.0, 0.0)],
        blooms: &[],
        weight: [0, 0, 1],
        floors: Some((0, 1)),
    },
    Scene {
        parts: &[part(Suv, 0.0, 0.0, 0.0)],
        blooms: &[],
        weight: [0, 0, 1],
        floors: Some((0, 2)),
    },
    Scene {
        parts: &[part(Truck, 0.0, 0.0, 0.0)],
        blooms: &[],
        weight: [0, 0, 1],
        floors: Some((0, 0)),
    },
    // Human places, left as they were.
    Scene {
        // A café table, two chairs facing each other.
        parts: &[
            part(TableRound, 0.0, 0.0, 0.0),
            part(Chair, 0.0, -0.9, 0.0),
            part(Chair, 0.0, 0.9, HALF),
        ],
        blooms: &[],
        weight: [0, 1, 4],
        floors: Some((1, 3)),
    },
    Scene {
        // The laundromat: a row of washers and a dryer.
        parts: &[
            part(Washer, 0.0, 0.0, 0.0),
            part(Washer, 0.75, 0.0, 0.0),
            part(Washer, 1.5, 0.0, 0.0),
            part(Dryer, 2.25, 0.0, 0.0),
        ],
        blooms: &[],
        weight: [0, 0, 2],
        floors: Some((1, 2)),
    },
    Scene {
        // An office corner.
        parts: &[
            part(Desk, 0.0, 0.0, 0.0),
            part(DeskChair, 0.0, 0.8, HALF),
            part(Bookcase, 1.6, -0.2, 0.0),
        ],
        blooms: &[],
        weight: [0, 1, 2],
        floors: Some((2, 4)),
    },
    Scene {
        // A living room out in the open.
        parts: &[
            part(Sofa, 0.0, 0.0, 0.0),
            part(Table, 0.0, 1.2, 0.0),
            part(Television, 0.0, 2.6, HALF),
            part(FloorLamp, 1.4, 0.0, 0.0),
        ],
        blooms: &[],
        weight: [0, 0, 2],
        floors: Some((1, 2)),
    },
    Scene {
        parts: &[part(CoatRack, 0.0, 0.0, 0.0)],
        blooms: &[],
        weight: [0, 1, 2],
        floors: Some((2, 3)),
    },
    Scene {
        parts: &[part(Fridge, 0.0, 0.0, 0.0), part(Freezer, 1.3, 0.1, 0.0)],
        blooms: &[],
        weight: [0, 0, 2],
        floors: Some((1, 2)),
    },
    // Points of interest with a flower (foreshadowing: flowers in human places).
    Scene {
        // A flower growing out of an armchair (D2-07).
        parts: &[part(Armchair, 0.0, 0.0, 0.0)],
        blooms: &[(0.0, 0.5, 0.05)],
        weight: [0, 0, 1],
        floors: Some((1, 2)),
    },
    Scene {
        // Two flowers side by side on a bench, about the distance two people sit (D1-09).
        parts: &[part(Bench, 0.0, 0.0, 0.0)],
        blooms: &[(-0.32, 0.5, 0.0), (0.32, 0.5, 0.0)],
        weight: [0, 1, 1],
        floors: None,
    },
    Scene {
        // A dead garden, one flower alive in a pot (D1-13).
        parts: &[
            part(PottedPlant, 0.0, 0.0, 0.0),
            part(PottedPlant, 0.8, 0.0, 0.3),
            part(PottedPlant, 1.6, 0.0, -0.1),
        ],
        blooms: &[(1.6, 1.15, -0.1)],
        weight: [0, 0, 1],
        floors: Some((1, 2)),
    },
];

/// Scenes always standing on the plaza where the player wakes up, facing the crater (-z):
/// (scene index, anchor x, anchor z, heading).
const START_SCENES: &[(usize, f32, f32, f32)] = &[(24, -7.0, -20.0, 0.0), (25, 6.0, -19.0, 0.0)];

/// Props per lot and floor, by height band: canopy, middle, depths.
const DENSITY: [usize; 3] = [1, 3, 7];

/// What dressing adds to a lot.
#[derive(Default)]
pub struct Dressing {
    pub props: Vec<Placed>,
    /// Flowers growing in the scenes (points of interest): position.
    pub blooms: Vec<Vec3>,
}

/// Tries to place one scene at `anchor` with heading `yaw`; all or nothing.
fn place(
    scene: &Scene,
    anchor: Vec3,
    yaw: f32,
    blocks: &[&Block],
    out: &mut Dressing,
    boxes: &mut Vec<Aabb>,
) -> bool {
    let (s, c) = yaw.sin_cos();
    // Scene frame to world: x right, z forward, rotated by yaw about the vertical.
    let to_world = |x: f32, z: f32| anchor + vec3(x * c + z * s, 0.0, -x * s + z * c);
    let placed: Vec<Placed> = scene
        .parts
        .iter()
        .map(|p| Placed {
            prop: p.prop,
            pos: to_world(p.at.0, p.at.1),
            yaw: yaw + p.turn,
        })
        .collect();
    for p in &placed {
        let b = p.aabb();
        // Stands on a deck, all four corners.
        let corners = [
            vec3(b.min.x, 0.0, b.min.z),
            vec3(b.max.x, 0.0, b.min.z),
            vec3(b.min.x, 0.0, b.max.z),
            vec3(b.max.x, 0.0, b.max.z),
        ];
        let on_deck = |q: Vec3| {
            blocks.iter().any(|d| {
                matches!(d.kind, BlockKind::Deck | BlockKind::Floor)
                    && (d.aabb.max.y - anchor.y).abs() < 0.01
                    && d.aabb.contains_xz(q)
            })
        };
        if !corners.iter().all(|&q| on_deck(q)) {
            return false;
        }
        // Clear of everything else, with a little room.
        let room = Aabb {
            min: b.min + vec3(-0.25, 0.02, -0.25),
            max: b.max + vec3(0.25, 0.0, 0.25),
        };
        if blocks.iter().any(|o| o.aabb.overlaps(&room)) || boxes.iter().any(|o| o.overlaps(&room))
        {
            return false;
        }
    }
    for p in &placed {
        boxes.push(p.aabb());
    }
    out.blooms.extend(
        scene
            .blooms
            .iter()
            .map(|&(x, y, z)| to_world(x, z) + Vec3::Y * y),
    );
    out.props.extend(placed);
    true
}

/// Props and flowers for a lot, given its blocks (prop collision blocks are ignored, so the result
/// is the same before and after they are added). Pure in `(seed, lot)`.
pub fn dress_lot(seed: u64, lx: i32, lz: i32, blocks: &[Block]) -> Dressing {
    let blocks: Vec<&Block> = blocks
        .iter()
        .filter(|b| b.kind != BlockKind::Prop)
        .collect();
    let blocks = &blocks[..];
    let mut rng = Rng::new(hash_coords(seed, lx, SALT_DRESSING, lz));
    let mut out = Dressing::default();
    let mut boxes = Vec::new();
    let origin = vec3(lx as f32, 0.0, lz as f32) * super::city::LOT;

    if super::city::is_plaza(lx, lz) {
        for &(i, x, z, yaw) in START_SCENES {
            let anchor = vec3(x, floors::Y[floors::START], z);
            let inside = |v: f32, o: f32| v >= o && v < o + super::city::LOT;
            if inside(anchor.x, origin.x) && inside(anchor.z, origin.z) {
                place(&SCENES[i], anchor, yaw, blocks, &mut out, &mut boxes);
            }
        }
    }

    // Walking surfaces of the lot: tops of its decks and ground.
    let mut levels: Vec<f32> = blocks
        .iter()
        .filter(|b| matches!(b.kind, BlockKind::Deck | BlockKind::Floor))
        .map(|b| b.aabb.max.y)
        .collect();
    levels.sort_by(f32::total_cmp);
    levels.dedup();
    for y in levels {
        let floor = floors::floor_at(y);
        let band = floors::band(floor);
        let eligible: Vec<&Scene> = SCENES
            .iter()
            .filter(|s| {
                s.weight[band] > 0 && s.floors.is_none_or(|(a, b)| (a..=b).contains(&floor))
            })
            .collect();
        let total: u32 = eligible.iter().map(|s| s.weight[band]).sum();
        if total == 0 {
            continue;
        }
        let mut placed = 0;
        for _ in 0..DENSITY[band] * 4 {
            if placed >= DENSITY[band] {
                break;
            }
            let mut roll = rng.range_i(0, total as i32) as u32;
            let scene = eligible
                .iter()
                .find(|s| {
                    let w = s.weight[band];
                    if roll < w {
                        true
                    } else {
                        roll -= w;
                        false
                    }
                })
                .unwrap_or(&eligible[0]);
            let anchor = vec3(
                origin.x + rng.range(1.0, super::city::LOT - 1.0),
                y,
                origin.z + rng.range(1.0, super::city::LOT - 1.0),
            );
            let yaw = rng.range_i(0, 4) as f32 * QUARTER + rng.range(-0.12, 0.12);
            if place(scene, anchor, yaw, blocks, &mut out, &mut boxes) {
                placed += 1;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_start_plaza_has_its_points_of_interest() {
        let mut blooms = Vec::new();
        for (lx, lz) in [(-1, -1), (0, -1)] {
            let mut blocks = Vec::new();
            super::super::city::generate_lot(1, lx, lz, &mut blocks);
            // The lot's own prop boxes are ignored: dressing it again gives the same result.
            blooms.extend(dress_lot(1, lx, lz, &blocks).blooms);
        }
        for &(_, x, z, _) in START_SCENES {
            let near = blooms
                .iter()
                .filter(|b| {
                    b.y < floors::Y[floors::START] + 2.0 && vec3(x, b.y, z).distance(**b) < 1.0
                })
                .count();
            assert!(near >= 1, "no flower at the start scene at {x},{z}");
        }
    }
}
