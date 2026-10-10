//! Set dressing and points of interest: props placed by the seed where they belong in a street.
//!
//! Every lot follows the same street layout, the one `pavement` draws in `shaders/scene.fs`: a road
//! along each border (`ROAD` m deep on this lot's side, meeting the neighbor's half), a curb, then
//! the sidewalk up to the tower's facades. Each scene has a kind of place (`Slot`) and is built in
//! that place's frame:
//! - `Curb`: on the sidewalk just behind the curb, facing the road (benches, bins, signs);
//! - `Corner`: the sidewalk corner of two streets (traffic lights, signs);
//! - `Facade`: against a tower wall, facing out (dumpsters, crates, café terraces, piles left by
//!   someone moving out);
//! - `Parking`: in the road along the curb, away from intersections (old cars);
//! - `Vacant`: inside a lot with no tower (debris, a small square).
//!
//! Dense in the depths, where people lived the longest, sparse in the middle floors, nearly empty
//! in the machine-built canopy (`docs/world.md`). Some scenes are points of interest from
//! `docs/poi-catalog.md`, with flowers growing in human places (the flowers are souls).

use glam::{Vec3, vec3};

use super::city::LOT;
use super::floors;
use super::props::{Placed, Prop, mesh};
use super::{Aabb, Block, BlockKind};
use crate::rng::{Rng, hash_coords};

const SALT_DRESSING: i32 = 9;

/// Depth of the road on each side of a lot border (the curb line), as drawn by the pavement.
pub const ROAD: f32 = 3.0;
/// Gap between the curb and the front of what stands on the sidewalk behind it.
const CURB_GAP: f32 = 0.45;
/// Gap between a wall and what stands against it.
const WALL_GAP: f32 = 0.2;
/// Distance of corner furniture from the two curbs.
const CORNER_GAP: f32 = 0.6;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Slot {
    Curb,
    Corner,
    Facade,
    Parking,
    Vacant,
}

/// A prop of a scene: offset in the scene's frame (x to the right, z toward the front, meters)
/// and an extra turn. In `Facade` scenes, z is measured from the prop's own back, so props of any
/// depth stand right against the wall.
struct Part {
    prop: Prop,
    at: (f32, f32),
    turn: f32,
}

const fn part(prop: Prop, x: f32, z: f32, turn: f32) -> Part {
    Part {
        prop,
        at: (x, z),
        turn,
    }
}

/// A flower growing in a scene: (x, height, z) from the first prop's center, in the scene's frame.
type Bloom = (f32, f32, f32);

struct Scene {
    slot: Slot,
    parts: &'static [Part],
    blooms: &'static [Bloom],
    /// Weight per height band: canopy, middle, depths.
    weight: [u32; 3],
    /// Only on these floors (inclusive), if set.
    floors: Option<(usize, usize)>,
}

use Prop::*;
use std::f32::consts::{FRAC_PI_2, PI};

const SCENES: &[Scene] = &[
    // 0..=4: along the curb, facing the road.
    Scene {
        slot: Slot::Curb,
        parts: &[part(Bench, 0.0, 0.0, 0.0)],
        blooms: &[],
        weight: [0, 2, 5],
        floors: None,
    },
    Scene {
        slot: Slot::Curb,
        parts: &[part(Bench, 0.0, 0.0, 0.0), part(Trashcan, 1.4, 0.0, 0.0)],
        blooms: &[],
        weight: [0, 1, 4],
        floors: None,
    },
    Scene {
        slot: Slot::Curb,
        parts: &[part(Trashcan, 0.0, 0.0, 0.0)],
        blooms: &[],
        weight: [0, 2, 4],
        floors: None,
    },
    Scene {
        slot: Slot::Curb,
        parts: &[part(StreetSign, 0.0, 0.0, 0.0)],
        blooms: &[],
        weight: [0, 1, 2],
        floors: None,
    },
    Scene {
        // Two flowers side by side on a bench, about the distance two people sit (D1-09).
        slot: Slot::Curb,
        parts: &[part(Bench, 0.0, 0.0, 0.0)],
        blooms: &[(-0.32, 0.5, 0.05), (0.32, 0.5, 0.05)],
        weight: [0, 1, 1],
        floors: None,
    },
    // 5..=6: street corners.
    Scene {
        slot: Slot::Corner,
        parts: &[part(TrafficLight, 0.0, 0.0, 0.0)],
        blooms: &[],
        weight: [0, 0, 3],
        floors: Some((0, 2)),
    },
    Scene {
        slot: Slot::Corner,
        parts: &[part(StreetSign, 0.0, 0.0, 0.0)],
        blooms: &[],
        weight: [0, 1, 2],
        floors: None,
    },
    // 7..=18: against the facades, facing out.
    Scene {
        slot: Slot::Facade,
        parts: &[part(DumpsterClosed, 0.0, 0.0, 0.0)],
        blooms: &[],
        weight: [0, 2, 4],
        floors: None,
    },
    Scene {
        slot: Slot::Facade,
        parts: &[
            part(DumpsterOpen, 0.0, 0.0, 0.0),
            part(Pallet, 1.9, 0.0, 0.1),
        ],
        blooms: &[],
        weight: [0, 1, 3],
        floors: None,
    },
    Scene {
        slot: Slot::Facade,
        parts: &[
            part(Barrel, 0.0, 0.0, 0.0),
            part(Barrel, 0.8, 0.0, 1.0),
            part(Crate, 1.7, 0.0, 0.05),
        ],
        blooms: &[],
        weight: [1, 2, 3],
        floors: None,
    },
    Scene {
        slot: Slot::Facade,
        parts: &[
            part(Crate, 0.0, 0.0, 0.0),
            part(Crate, 1.0, 0.0, -0.1),
            part(BoxClosed, 1.9, 0.0, 0.3),
        ],
        blooms: &[],
        weight: [1, 2, 2],
        floors: None,
    },
    Scene {
        slot: Slot::Facade,
        parts: &[part(ShoppingCart, 0.0, 0.0, 0.4)],
        blooms: &[],
        weight: [0, 1, 3],
        floors: Some((1, 3)),
    },
    Scene {
        slot: Slot::Facade,
        parts: &[
            part(Trashcan, 0.0, 0.0, 0.0),
            part(BoxClosed, 0.8, 0.0, 0.3),
            part(BoxOpen, 1.6, 0.0, -0.2),
        ],
        blooms: &[],
        weight: [0, 2, 3],
        floors: None,
    },
    Scene {
        // A café terrace in front of a shop: a table, a chair on each side.
        slot: Slot::Facade,
        parts: &[
            // A quarter turn left (+) or right (-) of the frame's front: each chair faces the table.
            part(Chair, -0.85, 0.35, -FRAC_PI_2),
            part(TableRound, 0.0, 0.2, 0.0),
            part(Chair, 0.85, 0.35, FRAC_PI_2),
        ],
        blooms: &[],
        weight: [0, 1, 4],
        floors: Some((1, 3)),
    },
    Scene {
        // Someone moving out, never finished: a sofa, boxes and a lamp by the wall.
        slot: Slot::Facade,
        parts: &[
            part(Sofa, 0.0, 0.0, 0.0),
            part(BoxClosed, 1.5, 0.0, 0.2),
            part(BoxOpen, 1.5, 0.7, -0.3),
            part(FloorLamp, -1.4, 0.0, 0.0),
        ],
        blooms: &[],
        weight: [0, 0, 2],
        floors: Some((1, 2)),
    },
    Scene {
        // A shop emptied onto the sidewalk.
        slot: Slot::Facade,
        parts: &[part(Fridge, 0.0, 0.0, 0.0), part(Freezer, 1.4, 0.0, 0.0)],
        blooms: &[],
        weight: [0, 0, 1],
        floors: Some((1, 2)),
    },
    Scene {
        slot: Slot::Facade,
        parts: &[part(Tire, 0.0, 0.0, 0.0), part(Tire, 0.7, 0.0, 0.6)],
        blooms: &[],
        weight: [0, 1, 2],
        floors: Some((0, 1)),
    },
    Scene {
        // A flower growing out of an armchair left by the wall (D2-07).
        slot: Slot::Facade,
        parts: &[part(Armchair, 0.0, 0.0, 0.0)],
        blooms: &[(0.0, 0.5, 0.05)],
        weight: [0, 0, 1],
        floors: Some((1, 2)),
    },
    Scene {
        // A dead garden by the wall, one flower alive in a pot (D1-13).
        slot: Slot::Facade,
        parts: &[
            part(PottedPlant, 0.0, 0.0, 0.0),
            part(PottedPlant, 0.8, 0.0, 0.3),
            part(PottedPlant, 1.6, 0.0, -0.1),
        ],
        blooms: &[(1.6, 1.15, 0.0)],
        weight: [0, 0, 1],
        floors: Some((1, 2)),
    },
    // 19..=24: parked along the curb, on the oldest streets.
    Scene {
        slot: Slot::Parking,
        parts: &[part(Sedan, 0.0, 0.0, 0.0)],
        blooms: &[],
        weight: [0, 0, 3],
        floors: Some((0, 1)),
    },
    Scene {
        slot: Slot::Parking,
        parts: &[part(Hatchback, 0.0, 0.0, 0.0)],
        blooms: &[],
        weight: [0, 0, 2],
        floors: Some((0, 1)),
    },
    Scene {
        slot: Slot::Parking,
        parts: &[part(Van, 0.0, 0.0, 0.0)],
        blooms: &[],
        weight: [0, 0, 2],
        floors: Some((0, 1)),
    },
    Scene {
        slot: Slot::Parking,
        parts: &[part(Taxi, 0.0, 0.0, 0.0)],
        blooms: &[],
        weight: [0, 0, 1],
        floors: Some((0, 1)),
    },
    Scene {
        slot: Slot::Parking,
        parts: &[part(Suv, 0.0, 0.0, 0.0)],
        blooms: &[],
        weight: [0, 0, 1],
        floors: Some((0, 2)),
    },
    Scene {
        slot: Slot::Parking,
        parts: &[part(Truck, 0.0, 0.0, 0.0)],
        blooms: &[],
        weight: [0, 0, 1],
        floors: Some((0, 0)),
    },
    // 25..=27: lots with no tower.
    Scene {
        slot: Slot::Vacant,
        parts: &[
            part(DumpsterOpen, 0.0, 0.0, 0.0),
            part(Tire, 2.0, 0.5, 0.0),
            part(Pallet, -2.2, 0.3, 0.3),
        ],
        blooms: &[],
        weight: [0, 1, 3],
        floors: None,
    },
    Scene {
        // A small square: two benches facing each other.
        slot: Slot::Vacant,
        parts: &[part(Bench, 0.0, -1.8, 0.0), part(Bench, 0.0, 1.8, PI)],
        blooms: &[],
        weight: [0, 1, 2],
        floors: None,
    },
    Scene {
        slot: Slot::Vacant,
        parts: &[
            part(ShoppingCart, 0.0, 0.0, 0.0),
            part(BoxOpen, 1.2, 0.4, 0.5),
            part(Crate, -1.3, 0.2, 0.2),
        ],
        blooms: &[],
        weight: [0, 1, 2],
        floors: Some((0, 3)),
    },
];

/// Points of interest always standing on the plaza where the player wakes up, behind the curb of
/// the street before the crater, facing it: (scene index, x, z).
const START_SCENES: &[(usize, f32, f32)] = &[(17, -7.0, -20.0), (4, 8.0, -20.2)];

/// Scenes tried per lot and floor for each slot, by height band (canopy, middle, depths).
const TRIES: [(Slot, [usize; 3]); 5] = [
    (Slot::Curb, [0, 4, 9]),
    (Slot::Corner, [0, 1, 2]),
    (Slot::Facade, [1, 3, 7]),
    (Slot::Parking, [0, 0, 3]),
    (Slot::Vacant, [0, 2, 3]),
];

/// What dressing adds to a lot.
#[derive(Default)]
pub struct Dressing {
    pub props: Vec<Placed>,
    /// Flowers growing in the scenes (points of interest).
    pub blooms: Vec<Vec3>,
}

/// Where a scene stands: its origin and the way it faces (unit, horizontal).
struct Frame {
    origin: Vec3,
    front: Vec3,
    /// z offsets start at each prop's back (scenes against a wall).
    from_back: bool,
}

impl Frame {
    fn right(&self) -> Vec3 {
        vec3(-self.front.z, 0.0, self.front.x)
    }

    /// Heading that turns a prop's front (+z) toward `front`.
    fn yaw(&self) -> f32 {
        self.front.x.atan2(self.front.z)
    }

    fn place(&self, p: &Part) -> Placed {
        let half = mesh(p.prop).half;
        // Depth along the front: a quarter-turned prop shows its width.
        let quarter = (p.turn.abs() - FRAC_PI_2).abs() < 0.3;
        let depth = match (self.from_back, quarter) {
            (false, _) => 0.0,
            (true, false) => half.z,
            (true, true) => half.x,
        };
        Placed {
            prop: p.prop,
            pos: self.origin + self.right() * p.at.0 + self.front * (p.at.1 + depth),
            yaw: self.yaw() + p.turn,
        }
    }
}

struct LotDressing<'a> {
    origin: Vec3,
    blocks: &'a [&'a Block],
    boxes: Vec<Aabb>,
    out: Dressing,
}

impl LotDressing<'_> {
    /// Places a scene in a frame, all or nothing: every prop fully on the deck at the frame's
    /// height, clear of everything else, and (except parked cars) off the road.
    fn try_place(&mut self, scene: &Scene, frame: &Frame) -> bool {
        let placed: Vec<Placed> = scene.parts.iter().map(|p| frame.place(p)).collect();
        let y = frame.origin.y;
        let (lo, hi) = (
            self.origin + vec3(ROAD, 0.0, ROAD),
            self.origin + vec3(LOT - ROAD, 0.0, LOT - ROAD),
        );
        for p in &placed {
            let b = p.aabb();
            let corners = [
                vec3(b.min.x, 0.0, b.min.z),
                vec3(b.max.x, 0.0, b.min.z),
                vec3(b.min.x, 0.0, b.max.z),
                vec3(b.max.x, 0.0, b.max.z),
            ];
            let on_deck = |q: Vec3| {
                self.blocks.iter().any(|d| {
                    matches!(d.kind, BlockKind::Deck | BlockKind::Floor)
                        && (d.aabb.max.y - y).abs() < 0.01
                        && d.aabb.contains_xz(q)
                })
            };
            if !corners.iter().all(|&q| on_deck(q)) {
                return false;
            }
            let in_road = b.min.x < lo.x || b.min.z < lo.z || b.max.x > hi.x || b.max.z > hi.z;
            if in_road != (scene.slot == Slot::Parking) {
                return false;
            }
            let room = Aabb {
                min: b.min + vec3(-0.15, 0.02, -0.15),
                max: b.max + vec3(0.15, 0.0, 0.15),
            };
            if self.blocks.iter().any(|o| o.aabb.overlaps(&room))
                || self.boxes.iter().any(|o| o.overlaps(&room))
            {
                return false;
            }
        }
        self.boxes.extend(placed.iter().map(|p| p.aabb()));
        if let Some(first) = placed.first() {
            for &(x, h, z) in scene.blooms {
                self.out
                    .blooms
                    .push(first.pos + frame.right() * x + frame.front * z + Vec3::Y * h);
            }
        }
        self.out.props.extend(placed);
        true
    }
}

/// Inward normal of a lot side (0: x min, 1: x max, 2: z min, 3: z max) and the point at
/// distance `d` from that side and `t` along it, from the lot origin.
fn side(i: usize, d: f32, t: f32) -> (Vec3, Vec3) {
    match i {
        0 => (Vec3::X, vec3(d, 0.0, t)),
        1 => (Vec3::NEG_X, vec3(LOT - d, 0.0, t)),
        2 => (Vec3::Z, vec3(t, 0.0, d)),
        _ => (Vec3::NEG_Z, vec3(t, 0.0, LOT - d)),
    }
}

/// Props and flowers for a lot, given its blocks (prop collision blocks are ignored, so the result
/// is the same before and after they are added). Pure in `(seed, lot)`.
pub fn dress_lot(seed: u64, lx: i32, lz: i32, blocks: &[Block]) -> Dressing {
    let blocks: Vec<&Block> = blocks
        .iter()
        .filter(|b| b.kind != BlockKind::Prop)
        .collect();
    let mut rng = Rng::new(hash_coords(seed, lx, SALT_DRESSING, lz));
    let mut lot = LotDressing {
        origin: vec3(lx as f32, 0.0, lz as f32) * LOT,
        blocks: &blocks,
        boxes: Vec::new(),
        out: Dressing::default(),
    };
    let plaza = super::city::is_plaza(lx, lz);

    if plaza {
        for &(i, x, z) in START_SCENES {
            let inside = |v: f32, o: f32| v >= o && v < o + LOT;
            if inside(x, lot.origin.x) && inside(z, lot.origin.z) {
                let frame = Frame {
                    origin: vec3(x, floors::Y[floors::START], z),
                    front: Vec3::NEG_Z,
                    from_back: false,
                };
                lot.try_place(&SCENES[i], &frame);
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
    // The tower's main body, whose walls the facade scenes stand against.
    let tower = blocks
        .iter()
        .find(|b| b.kind == BlockKind::Tower)
        .map(|b| b.aabb);
    let vacant = tower.is_none() && !plaza && super::city::stair(seed, lx, lz).is_none();

    for y in levels {
        let floor = floors::floor_at(y);
        let band = floors::band(floor);
        let standing = tower.filter(|t| t.min.y <= y && t.max.y > y + 3.0);
        for (slot, tries) in TRIES {
            let eligible: Vec<&Scene> = SCENES
                .iter()
                .filter(|s| {
                    s.slot == slot
                        && s.weight[band] > 0
                        && s.floors.is_none_or(|(a, b)| (a..=b).contains(&floor))
                })
                .collect();
            let total: u32 = eligible.iter().map(|s| s.weight[band]).sum();
            if total == 0 {
                continue;
            }
            for _ in 0..tries[band] {
                let mut roll = rng.range_i(0, total as i32) as u32;
                let Some(&scene) = eligible.iter().find(|s| {
                    let w = s.weight[band];
                    let hit = roll < w;
                    roll = roll.saturating_sub(w);
                    hit
                }) else {
                    continue;
                };
                let s = rng.range_i(0, 4) as usize;
                let frame = match slot {
                    Slot::Curb => {
                        // Facing the road, the front edge just behind the curb.
                        let depth = mesh(scene.parts[0].prop).half.z;
                        let t = rng.range(ROAD + 1.5, LOT - ROAD - 1.5);
                        let (inward, at) = side(s, ROAD + CURB_GAP + depth, t);
                        Frame {
                            origin: lot.origin + at + Vec3::Y * y,
                            front: -inward,
                            from_back: false,
                        }
                    }
                    Slot::Corner => {
                        let (a, b) = (s & 1, 2 + (s >> 1));
                        let (na, _) = side(a, 0.0, 0.0);
                        let (nb, _) = side(b, 0.0, 0.0);
                        let d = ROAD + CORNER_GAP;
                        let at = vec3(
                            if a == 0 { d } else { LOT - d },
                            y,
                            if b == 2 { d } else { LOT - d },
                        );
                        Frame {
                            origin: lot.origin + at,
                            front: -(na + nb).normalize(),
                            from_back: false,
                        }
                    }
                    Slot::Facade => {
                        let Some(t) = standing else { continue };
                        // A face of the tower and a spot along it.
                        let (front, along) = match s {
                            0 | 1 => (
                                if s == 0 { Vec3::NEG_X } else { Vec3::X },
                                (t.min.z + 1.0, t.max.z - 1.0),
                            ),
                            _ => (
                                if s == 2 { Vec3::NEG_Z } else { Vec3::Z },
                                (t.min.x + 1.0, t.max.x - 1.0),
                            ),
                        };
                        if along.1 <= along.0 {
                            continue;
                        }
                        let u = rng.range(along.0, along.1);
                        let wall = match s {
                            0 => t.min.x - WALL_GAP,
                            1 => t.max.x + WALL_GAP,
                            2 => t.min.z - WALL_GAP,
                            _ => t.max.z + WALL_GAP,
                        };
                        Frame {
                            origin: if s < 2 {
                                vec3(wall, y, u)
                            } else {
                                vec3(u, y, wall)
                            },
                            front,
                            from_back: true,
                        }
                    }
                    Slot::Parking => {
                        // In the road along the curb, nose along the street, clear of crossings.
                        let half = mesh(scene.parts[0].prop).half;
                        let reach = ROAD + half.z + 1.5;
                        let (inward, at) =
                            side(s, ROAD - 0.25 - half.x, rng.range(reach, LOT - reach));
                        let along = vec3(inward.z, 0.0, inward.x);
                        Frame {
                            origin: lot.origin + at + Vec3::Y * y,
                            front: if rng.chance(0.5) { along } else { -along },
                            from_back: false,
                        }
                    }
                    Slot::Vacant => {
                        if !vacant {
                            continue;
                        }
                        let fronts = [Vec3::X, Vec3::NEG_X, Vec3::Z, Vec3::NEG_Z];
                        let (a, b) = (ROAD + 3.0, LOT - ROAD - 3.0);
                        Frame {
                            origin: lot.origin + vec3(rng.range(a, b), y, rng.range(a, b)),
                            front: fronts[s],
                            from_back: false,
                        }
                    }
                };
                lot.try_place(scene, &frame);
            }
        }
    }
    lot.out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dressed(seed: u64, lx: i32, lz: i32) -> Dressing {
        let mut blocks = Vec::new();
        super::super::city::generate_lot(seed, lx, lz, &mut blocks);
        // The lot's own prop boxes are ignored: dressing it again gives the same result.
        dress_lot(seed, lx, lz, &blocks)
    }

    #[test]
    fn the_start_plaza_has_its_points_of_interest() {
        let blooms: Vec<Vec3> = [(-1, -1), (0, -1)]
            .into_iter()
            .flat_map(|(lx, lz)| dressed(1, lx, lz).blooms)
            .collect();
        for &(_, x, z) in START_SCENES {
            let near = blooms
                .iter()
                .filter(|b| {
                    b.y < floors::Y[floors::START] + 2.0 && vec3(x, b.y, z).distance(**b) < 1.2
                })
                .count();
            assert!(near >= 1, "no flower at the start scene at {x},{z}");
        }
    }

    #[test]
    fn only_parked_cars_stand_in_the_road() {
        let cars = [Sedan, Hatchback, Van, Taxi, Suv, Truck];
        for seed in 1..=3 {
            for lz in -3..=3 {
                for lx in -3..=3 {
                    let o = vec3(lx as f32, 0.0, lz as f32) * LOT;
                    for p in dressed(seed, lx, lz).props {
                        let b = p.aabb();
                        let in_road = b.min.x < o.x + ROAD - 0.01
                            || b.min.z < o.z + ROAD - 0.01
                            || b.max.x > o.x + LOT - ROAD + 0.01
                            || b.max.z > o.z + LOT - ROAD + 0.01;
                        assert_eq!(in_road, cars.contains(&p.prop), "{:?} at {}", p.prop, p.pos);
                    }
                }
            }
        }
    }

    #[test]
    fn the_depths_are_dressed() {
        let n: usize = (-2..2)
            .flat_map(|lx| (-2..2).map(move |lz| (lx, lz)))
            .map(|(lx, lz)| dressed(2, lx, lz).props.len())
            .sum();
        assert!(n >= 16 * 10, "only {n} props on 16 lots");
    }
}
