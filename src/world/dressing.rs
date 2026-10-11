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
use super::design;
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

/// What a scene is, for the switches and for how it is drawn.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// Street furniture: benches, bins, signs, traffic lights.
    Street,
    /// What people left: dumpsters, crates, carts, café terraces, piles from moving out.
    Leftovers,
    /// Parked cars (none until a fitting car model is found).
    #[allow(dead_code)]
    Vehicle,
    /// Points of interest with flowers growing in them (drawn apart, at the floor's rate).
    Poi,
}

struct Scene {
    slot: Slot,
    kind: Kind,
    parts: &'static [Part],
    blooms: &'static [Bloom],
    /// Weight per zone of the stack: depths, middle, canopy (`floors::Zone`).
    weight: [u32; 3],
}

const fn scene(slot: Slot, kind: Kind, parts: &'static [Part], weight: [u32; 3]) -> Scene {
    Scene {
        slot,
        kind,
        parts,
        blooms: &[],
        weight,
    }
}

use Prop::*;
use std::f32::consts::{FRAC_PI_2, PI};

const SCENES: &[Scene] = &[
    // Along the curb, facing the road.
    scene(Slot::Curb, Kind::Street, &[part(Bench, 0.0, 0.0, 0.0)], [5, 2, 0]),
    scene(
        Slot::Curb,
        Kind::Street,
        &[part(Bench, 0.0, 0.0, 0.0), part(Bin, 1.5, 0.0, 0.0)],
        [4, 1, 0],
    ),
    scene(Slot::Curb, Kind::Street, &[part(Bin, 0.0, 0.0, 0.0)], [3, 2, 0]),
    scene(Slot::Curb, Kind::Street, &[part(UtilityBox, 0.0, 0.0, PI)], [2, 1, 0]),
    scene(Slot::Curb, Kind::Street, &[part(Planter, 0.0, 0.0, 0.0)], [2, 1, 0]),
    // Street corners.
    scene(Slot::Corner, Kind::Street, &[part(UtilityBox, 0.0, 0.0, PI)], [2, 1, 0]),
    scene(Slot::Corner, Kind::Street, &[part(Planter, 0.0, 0.0, 0.0)], [2, 1, 0]),
    // Against the facades, facing out.
    scene(
        Slot::Facade,
        Kind::Leftovers,
        &[
            part(TrashBag, 0.0, 0.0, 0.0),
            part(TrashBag, 0.6, 0.15, 0.9),
            part(Bin, 1.3, 0.0, 0.0),
        ],
        [4, 2, 0],
    ),
    scene(
        Slot::Facade,
        Kind::Leftovers,
        &[
            part(Barrel, 0.0, 0.0, 0.0),
            part(BarrelBlue, 0.7, 0.0, 0.3),
            part(Crate, 1.6, 0.0, 0.05),
        ],
        [3, 2, 1],
    ),
    scene(
        Slot::Facade,
        Kind::Leftovers,
        &[
            part(Crate, 0.0, 0.0, 0.0),
            part(CardboardBox, 0.75, 0.0, -0.1),
            part(CardboardBox, 1.3, 0.05, 0.3),
        ],
        [2, 2, 1],
    ),
    // A café terrace in front of a shop: a table, a chair on each side, a quarter turn left (-)
    // or right (+) of the frame's front so each faces the table.
    scene(
        Slot::Facade,
        Kind::Leftovers,
        &[
            part(TerraceChair, -0.6, 0.3, -FRAC_PI_2),
            part(TerraceTable, 0.0, 0.2, 0.0),
            part(TerraceChair, 0.6, 0.3, FRAC_PI_2),
        ],
        [4, 1, 0],
    ),
    // Someone moving out, never finished: a sofa and boxes by the wall.
    scene(
        Slot::Facade,
        Kind::Leftovers,
        &[
            part(Sofa, 0.0, 0.0, 0.0),
            part(CardboardBox, 1.4, 0.1, 0.0),
            part(CardboardBox, 1.5, 0.6, 0.3),
        ],
        [2, 0, 0],
    ),
    scene(Slot::Facade, Kind::Street, &[part(UtilityBox, 0.0, 0.0, 0.0)], [2, 2, 1]),
    // Lots with no tower.
    scene(
        Slot::Vacant,
        Kind::Leftovers,
        &[
            part(Barrel, 0.0, 0.0, 0.0),
            part(TrashBag, 1.0, 0.3, 0.0),
            part(Crate, -1.0, 0.2, 0.3),
        ],
        [3, 1, 0],
    ),
    // A small square: two benches facing each other.
    scene(
        Slot::Vacant,
        Kind::Street,
        &[part(Bench, 0.0, -1.8, 0.0), part(Bench, 0.0, 1.8, PI)],
        [2, 1, 0],
    ),
    // Points of interest with flowers (foreshadowing: flowers in human places), drawn apart at
    // each floor's `Flowers::poi` rate. Last in the table, the armchair first (`START_SCENES`).
    // A flower growing out of an armchair left by a wall (D2-07).
    Scene {
        slot: Slot::Facade,
        kind: Kind::Poi,
        parts: &[part(Armchair, 0.0, 0.0, 0.0)],
        blooms: &[(0.0, 0.5, 0.05)],
        weight: [2, 1, 0],
    },
    // Two flowers side by side on a bench, about the distance two people sit (D1-09).
    Scene {
        slot: Slot::Curb,
        kind: Kind::Poi,
        parts: &[part(Bench, 0.0, 0.0, 0.0)],
        blooms: &[(-0.32, 0.58, 0.0), (0.32, 0.58, 0.0)],
        weight: [1, 1, 0],
    },
    // A dead garden by the wall, one flower alive in a pot (D1-13).
    Scene {
        slot: Slot::Facade,
        kind: Kind::Poi,
        parts: &[
            part(ClayPot, 0.0, 0.0, 0.0),
            part(ClayPot, 0.45, 0.15, 0.3),
            part(ClayPot, 0.9, 0.0, -0.1),
        ],
        blooms: &[(0.9, 0.3, 0.0)],
        weight: [2, 1, 0],
    },
    // An old bed frame against a wall, a flower where someone slept.
    Scene {
        slot: Slot::Facade,
        kind: Kind::Poi,
        parts: &[part(BedFrame, 0.0, 0.0, 0.0)],
        blooms: &[(0.0, 0.55, -0.5)],
        weight: [1, 0, 0],
    },
];

/// Points of interest at the end of `SCENES`.
const POIS: usize = 4;

/// The street light, placed by its own rule (`street_lights`), never drawn at random.
const LIGHT: Scene = scene(
    Slot::Corner,
    Kind::Street,
    &[part(StreetLight, 0.0, 0.0, 0.0)],
    [0, 0, 0],
);
/// Distance of a street light's pole from the two curbs of its corner.
const LIGHT_GAP: f32 = 0.5;
const SALT_LIGHTS: i32 = 12;

/// The point of interest always standing on the plaza where the player wakes up, behind the curb
/// of the street before the crater, facing it: (scene index, x, z). Only one, so the secret of the
/// flowers is not spelled out from the first step.
const START_SCENES: &[(usize, f32, f32)] = &[(SCENES.len() - POIS, -7.0, -20.0)];

/// Props tried per lot and floor for each slot, before the floor's profile scales them
/// (`FloorProfile::props`, and `FloorProfile::cars` for parking).
const TRIES: [(Slot, f32); 5] = [
    (Slot::Curb, 3.0),
    (Slot::Corner, 1.0),
    (Slot::Facade, 3.0),
    (Slot::Parking, 1.0),
    (Slot::Vacant, 1.5),
];

/// Attempts at placing a lot's point of interest once the floor's chance says it has one.
const POI_ATTEMPTS: usize = 6;

/// What dressing adds to a lot.
#[derive(Default)]
pub struct Dressing {
    pub props: Vec<Placed>,
    /// Kind of the scene each prop belongs to (same order as `props`).
    pub kinds: Vec<Kind>,
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
    /// A point of interest with flowers was placed: one per lot at most, so they stay rare.
    poi: bool,
}

impl LotDressing<'_> {
    /// Places a scene in a frame, all or nothing: every prop fully on the deck at the frame's
    /// height, clear of everything else, and (except parked cars) off the road.
    fn try_place(&mut self, scene: &Scene, frame: &Frame) -> bool {
        if self.poi && !scene.blooms.is_empty() {
            return false;
        }
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
        self.poi |= !scene.blooms.is_empty();
        if let Some(first) = placed.first() {
            for &(x, h, z) in scene.blooms {
                self.out
                    .blooms
                    .push(first.pos + frame.right() * x + frame.front * z + Vec3::Y * h);
            }
        }
        self.out.kinds.extend(placed.iter().map(|_| scene.kind));
        self.out.props.extend(placed);
        true
    }
}

/// What the spots of a lot depend on.
struct Context {
    /// The tower's main body, whose walls the facade scenes stand against.
    tower: Option<Aabb>,
    /// No tower, no stair, not the plaza: a vacant lot.
    vacant: bool,
    /// Which sides of the lot (see `side`) are streets on the level being dressed: those with a
    /// deck across the border. The others are the edge of the deck, a sidewalk with no road.
    streets: [bool; 4],
}

impl Context {
    /// A spot for a scene on the walking surface at height `y`, in its slot's frame.
    fn frame(&self, scene: &Scene, y: f32, rng: &mut Rng, lot: &LotDressing) -> Option<Frame> {
        let s = rng.range_i(0, 4) as usize;
        let first = mesh(scene.parts[0].prop).half;
        match scene.slot {
            Slot::Curb => {
                if !self.streets[s] {
                    return None;
                }
                // Facing the road, the front edge just behind the curb.
                let t = rng.range(ROAD + 1.5, LOT - ROAD - 1.5);
                let (inward, at) = side(s, ROAD + CURB_GAP + first.z, t);
                Some(Frame {
                    origin: lot.origin + at + Vec3::Y * y,
                    front: -inward,
                    from_back: false,
                })
            }
            Slot::Corner => {
                let (a, b) = (s & 1, 2 + (s >> 1));
                if !self.streets[a] || !self.streets[b] {
                    return None;
                }
                let (na, _) = side(a, 0.0, 0.0);
                let (nb, _) = side(b, 0.0, 0.0);
                let d = ROAD + CORNER_GAP;
                let at = vec3(
                    if a == 0 { d } else { LOT - d },
                    y,
                    if b == 2 { d } else { LOT - d },
                );
                Some(Frame {
                    origin: lot.origin + at,
                    front: -(na + nb).normalize(),
                    from_back: false,
                })
            }
            Slot::Facade => {
                // Against a face of the tower standing on this floor, at a spot along it.
                let t = self.tower.filter(|t| t.min.y <= y && t.max.y > y + 3.0)?;
                let (front, along, wall) = match s {
                    0 => (Vec3::NEG_X, (t.min.z, t.max.z), t.min.x - WALL_GAP),
                    1 => (Vec3::X, (t.min.z, t.max.z), t.max.x + WALL_GAP),
                    2 => (Vec3::NEG_Z, (t.min.x, t.max.x), t.min.z - WALL_GAP),
                    _ => (Vec3::Z, (t.min.x, t.max.x), t.max.z + WALL_GAP),
                };
                if along.1 - along.0 <= 2.0 {
                    return None;
                }
                let u = rng.range(along.0 + 1.0, along.1 - 1.0);
                Some(Frame {
                    origin: if s < 2 {
                        vec3(wall, y, u)
                    } else {
                        vec3(u, y, wall)
                    },
                    front,
                    from_back: true,
                })
            }
            Slot::Parking => {
                if !self.streets[s] {
                    return None;
                }
                // In the road along the curb, nose along the street, clear of crossings.
                let reach = ROAD + first.z + 1.5;
                let (inward, at) = side(s, ROAD - 0.25 - first.x, rng.range(reach, LOT - reach));
                let along = vec3(inward.z, 0.0, inward.x);
                Some(Frame {
                    origin: lot.origin + at + Vec3::Y * y,
                    front: if rng.chance(0.5) { along } else { -along },
                    from_back: false,
                })
            }
            Slot::Vacant => {
                if !self.vacant {
                    return None;
                }
                let fronts = [Vec3::X, Vec3::NEG_X, Vec3::Z, Vec3::NEG_Z];
                let (a, b) = (ROAD + 3.0, LOT - ROAD - 3.0);
                Some(Frame {
                    origin: lot.origin + vec3(rng.range(a, b), y, rng.range(a, b)),
                    front: fronts[s],
                    from_back: false,
                })
            }
        }
    }
}

/// Street lights stand at the crossroads, one per crossing, on a corner of the sidewalk with
/// their arm reaching over the crossing. Each street keeps its lights on the same side all along
/// it, so they line up evenly, one every lot. A lot holds the lights of the corners where both its
/// streets have their lights on its side.
fn street_lights(seed: u64, (lx, lz): (i32, i32), y: f32, ctx: &Context, lot: &mut LotDressing) {
    for (cx, cz) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
        // The street along z at this corner has its lights east of it (the lot whose west border
        // it is) or west; the street along x, south or north.
        let east = hash_coords(seed, lx + cx, SALT_LIGHTS, 0) & 1 == 1;
        let south = hash_coords(seed, 0, SALT_LIGHTS + 1, lz + cz) & 1 == 1;
        // Only where two streets cross: both of the corner's sides are streets.
        if east != (cx == 0)
            || south != (cz == 0)
            || !ctx.streets[cx as usize]
            || !ctx.streets[2 + cz as usize]
        {
            continue;
        }
        let d = ROAD + LIGHT_GAP;
        let at = vec3(
            if cx == 0 { d } else { LOT - d },
            y,
            if cz == 0 { d } else { LOT - d },
        );
        let front = vec3(2.0 * cx as f32 - 1.0, 0.0, 2.0 * cz as f32 - 1.0).normalize();
        let frame = Frame {
            origin: lot.origin + at,
            front,
            from_back: false,
        };
        lot.try_place(&LIGHT, &frame);
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
        poi: false,
    };
    let props_on = design::props_on();
    let pois_on = design::pois_on();
    let plaza = super::city::is_plaza(lx, lz);

    if plaza && pois_on {
        for &(i, x, z) in START_SCENES {
            let inside = |v: f32, o: f32| v >= o && v < o + LOT;
            if inside(x, lot.origin.x) && inside(z, lot.origin.z) {
                let frame = Frame {
                    origin: vec3(x, floors::y(floors::START), z),
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

    let mut ctx = Context {
        tower,
        vacant,
        streets: [true; 4],
    };
    // Floors where a hand-written chunk covers the lot: it brings its own furniture.
    let chunked: Vec<usize> = super::city::placements(seed, lx, lz)
        .iter()
        .map(|p| p.floor)
        .collect();
    for y in levels {
        let floor = floors::floor_at(y);
        if chunked.contains(&floor) {
            continue;
        }
        ctx.streets = [(lx - 1, lz), (lx + 1, lz), (lx, lz - 1), (lx, lz + 1)]
            .map(|(x, z)| super::city::deck(seed, x, z, floor) != super::city::Deck::None);
        let zone = floors::zone(floor) as usize;
        let profile = design::profile(floor);
        let pick = |rng: &mut Rng, wanted: &dyn Fn(&Scene) -> bool| -> Option<&'static Scene> {
            let pool: Vec<&'static Scene> = SCENES
                .iter()
                .filter(|s| wanted(s) && s.weight[zone] > 0)
                .collect();
            let total: u32 = pool.iter().map(|s| s.weight[zone]).sum();
            if total == 0 {
                return None;
            }
            let mut roll = rng.range_i(0, total as i32) as u32;
            pool.into_iter().find(|s| {
                let hit = roll < s.weight[zone];
                roll = roll.saturating_sub(s.weight[zone]);
                hit
            })
        };
        // Street lights first: they follow the street layout, not chance.
        if props_on {
            street_lights(seed, (lx, lz), y, &ctx, &mut lot);
        }
        // A point of interest next, at the floor's rate, so it gets a good spot.
        if pois_on
            && rng.chance(profile.flowers.poi)
            && let Some(scene) = pick(&mut rng, &|s| s.kind == Kind::Poi)
        {
            for _ in 0..POI_ATTEMPTS {
                let frame = ctx.frame(scene, y, &mut rng, &lot);
                if frame.is_some_and(|f| lot.try_place(scene, &f)) {
                    break;
                }
            }
        }
        if !props_on {
            continue;
        }
        for (slot, tries) in TRIES {
            // Fractional tries happen by chance.
            let rate = if slot == Slot::Parking {
                profile.cars
            } else {
                profile.props
            };
            let want = tries * rate;
            let n = want as usize + rng.chance(want.fract()) as usize;
            for _ in 0..n {
                let Some(scene) = pick(&mut rng, &|s| s.slot == slot && s.kind != Kind::Poi) else {
                    continue;
                };
                if let Some(frame) = ctx.frame(scene, y, &mut rng, &lot) {
                    lot.try_place(scene, &frame);
                }
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
                    b.y < floors::y(floors::START) + 2.0 && vec3(x, b.y, z).distance(**b) < 1.2
                })
                .count();
            assert!(near >= 1, "no flower at the start scene at {x},{z}");
        }
    }

    #[test]
    fn only_parked_cars_stand_in_the_road() {
        let cars: [Prop; 0] = [];
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
    fn street_lights_stand_at_the_crossroads() {
        let mut n = 0;
        for lz in -3..=3 {
            for lx in -3..=3 {
                let o = vec3(lx as f32, 0.0, lz as f32) * LOT;
                for p in dressed(1, lx, lz)
                    .props
                    .iter()
                    .filter(|p| p.prop == StreetLight)
                {
                    n += 1;
                    let d = ROAD + LIGHT_GAP;
                    let at_corner =
                        |v: f32, o: f32| (v - o - d).abs() < 1e-3 || (v - o - LOT + d).abs() < 1e-3;
                    assert!(
                        at_corner(p.pos.x, o.x) && at_corner(p.pos.z, o.z),
                        "{}",
                        p.pos
                    );
                }
            }
        }
        // About one per crossing and floor with a deck there.
        assert!(n > 49 * 2, "only {n} street lights on 49 lots");
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
