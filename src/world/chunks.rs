//! Hand-written map chunks (`docs/chunks.md`): large pieces of city written as text in
//! `assets/chunks/`, built from boxes and from modules (`assets/modules/`), placed by the seed.
//!
//! A chunk file holds one `Chunk`; a module file holds a list of `Module`s. Coordinates are meters,
//! local to the chunk (origin at its corner, `y = 0` on the walking surface of its floor). The
//! world is axis-aligned boxes, so turns are quarter turns. Files are read from disk at run time
//! and hot-reloaded (`poll`); the seed picks what `Slot`s hold.
//!
//! Status: phase 2 of the plan. Chunks can be previewed alone (`--chunk NAME`); the planner that
//! places them in the city comes next.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock, RwLock};
use std::time::SystemTime;

use glam::{Vec3, vec3};
use serde::Deserialize;
use sola_raylib::prelude::Color;

use super::Aabb;
use crate::rng::Rng;

const ASSETS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets");
/// Modules may use modules, this deep at most (a guard against a module using itself).
const MAX_DEPTH: usize = 12;

type V3 = (f32, f32, f32);
type Rect = ((f32, f32), (f32, f32));

/// Surface materials of built boxes (drawn by `built` in `shaders/scene.fs`).
#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mat {
    Stone,
    StoneDark,
    Concrete,
    Brick,
    Tile,
    Metal,
    Rust,
    Wood,
    Glass,
    Soil,
    /// Painted surface of the given sRGB color.
    Paint((u8, u8, u8)),
    /// Paint in the accent color of the module or slot using it (`accent`, `accents`): one module,
    /// many colors. Resolved to `Paint` when built.
    Accent,
}

impl Mat {
    /// Index read by the scene shader.
    pub fn index(self) -> u8 {
        match self {
            Mat::Stone => 0,
            Mat::StoneDark => 1,
            Mat::Concrete => 2,
            Mat::Brick => 3,
            Mat::Tile => 4,
            Mat::Metal => 5,
            Mat::Rust => 6,
            Mat::Wood => 7,
            Mat::Glass => 8,
            Mat::Soil => 9,
            Mat::Paint(_) | Mat::Accent => 10,
        }
    }

    /// Base color (sRGB), which the shader details.
    pub fn color(self) -> Color {
        let c = |r, g, b| Color::new(r, g, b, 255);
        match self {
            Mat::Stone => c(168, 160, 146),
            Mat::StoneDark => c(98, 96, 94),
            Mat::Concrete => c(140, 140, 136),
            Mat::Brick => c(132, 72, 58),
            Mat::Tile => c(150, 160, 156),
            Mat::Metal => c(92, 98, 108),
            Mat::Rust => c(120, 70, 44),
            Mat::Wood => c(120, 84, 56),
            Mat::Glass => c(40, 54, 64),
            Mat::Soil => c(62, 48, 36),
            Mat::Paint((r, g, b)) => c(r, g, b),
            Mat::Accent => c(150, 150, 150),
        }
    }

    /// Whether plants grow on its top.
    pub fn plantable(self) -> bool {
        !matches!(self, Mat::Glass | Mat::Metal)
    }
}

#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    X,
    Z,
}

/// Hints for the random vegetation (`growth.rs`).
#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrowthKind {
    /// Denser plants.
    Soil,
    /// No plants (a path to keep clear).
    Bare,
    /// Climbers welcome on the walls.
    Wall,
}

/// One instruction of a chunk or module.
#[derive(Deserialize, Clone, Debug)]
pub enum Part {
    /// A solid box: `at` its minimum corner, `size` its extent.
    Box { at: V3, size: V3, mat: Mat },
    /// A module from the catalog, its origin at `at`, turned by `turn` quarter turns, its `Accent`
    /// surfaces in `accent` (or the accent around it).
    Module {
        name: String,
        #[serde(default)]
        at: V3,
        #[serde(default)]
        turn: u8,
        #[serde(default)]
        accent: Option<(u8, u8, u8)>,
    },
    /// One of several modules (or nothing, `None`), picked by the seed, with an accent color
    /// picked among `accents` (if any).
    Slot {
        at: V3,
        #[serde(default)]
        turn: u8,
        pick: Vec<Option<String>>,
        #[serde(default)]
        accents: Vec<(u8, u8, u8)>,
    },
    /// The parts, and their mirror image across the plane `axis = at`.
    Mirror2 {
        axis: Axis,
        at: f32,
        parts: Vec<Part>,
    },
    /// The parts four times, turned by quarter turns around `center` (x, z).
    Mirror4 {
        center: (f32, f32),
        parts: Vec<Part>,
    },
    /// The parts `count` times, shifted by `from + step * i`.
    Repeat {
        #[serde(default)]
        from: V3,
        step: V3,
        count: u32,
        parts: Vec<Part>,
    },
    /// A stairwell centered at `at` (x, z), up to the next floor.
    Stair {
        at: (f32, f32),
        #[serde(default)]
        along_x: bool,
    },
    /// A vegetation hint over a rectangle (x, z corners).
    Growth { area: Rect, kind: GrowthKind },
    /// An opening the planner fills with smaller chunks of these labels (not built yet).
    Sub { area: Rect, labels: Vec<String> },
}

/// Where in the stack a chunk may stand (`floors::Zone`).
#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Zone {
    Depths,
    Middle,
    Canopy,
}

fn one() -> u32 {
    1
}

#[derive(Deserialize, Clone, Debug)]
pub struct Chunk {
    pub name: String,
    /// Footprint in lots (x, z).
    pub size: (i32, i32),
    #[serde(default)]
    pub zones: Vec<Zone>,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default = "one")]
    pub weight: u32,
    /// Least distance, in lots, to another chunk sharing a label.
    #[serde(default)]
    pub spacing: u32,
    /// The walking surface over the whole footprint, paving over the inner roads; `None` keeps the
    /// standard streets.
    #[serde(default)]
    pub ground: Option<Mat>,
    pub parts: Vec<Part>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Module {
    pub name: String,
    pub parts: Vec<Part>,
}

/// Every chunk and module, as loaded from disk.
#[derive(Default)]
pub struct Catalog {
    pub chunks: HashMap<String, Chunk>,
    pub modules: HashMap<String, Module>,
    /// Files that did not load, with the reason.
    pub errors: Vec<String>,
}

/// RON as chunk files are written: optional values without `Some(...)` (`accent: (150, 52, 40)`).
fn ron_options() -> ron::Options {
    ron::Options::default().with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME)
}

fn ron_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "ron"))
        .collect();
    files.sort();
    files
}

impl Catalog {
    fn load() -> Catalog {
        let mut c = Catalog::default();
        let root = Path::new(ASSETS);
        for path in ron_files(&root.join("modules")) {
            match std::fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|s| ron_options().from_str::<Vec<Module>>(&s).map_err(|e| e.to_string()))
            {
                Ok(list) => c
                    .modules
                    .extend(list.into_iter().map(|m| (m.name.clone(), m))),
                Err(e) => c.errors.push(format!("{}: {e}", path.display())),
            }
        }
        for path in ron_files(&root.join("chunks")) {
            match std::fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|s| ron_options().from_str::<Chunk>(&s).map_err(|e| e.to_string()))
            {
                Ok(ch) => {
                    c.chunks.insert(ch.name.clone(), ch);
                }
                Err(e) => c.errors.push(format!("{}: {e}", path.display())),
            }
        }
        for e in &c.errors {
            eprintln!("chunks: {e}");
        }
        c
    }
}

fn store() -> &'static RwLock<Arc<Catalog>> {
    static STORE: OnceLock<RwLock<Arc<Catalog>>> = OnceLock::new();
    STORE.get_or_init(|| RwLock::new(Arc::new(Catalog::load())))
}

/// The current catalog.
pub fn catalog() -> Arc<Catalog> {
    store().read().unwrap().clone()
}

/// Newest modification time of the catalog's files.
fn stamp() -> Option<SystemTime> {
    let root = Path::new(ASSETS);
    ["chunks", "modules"]
        .iter()
        .flat_map(|d| ron_files(&root.join(d)))
        .filter_map(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok())
        .max()
}

/// Reloads the catalog if a file changed on disk. Returns true when it did (the world must then be
/// generated again).
pub fn poll() -> bool {
    static LAST: RwLock<Option<SystemTime>> = RwLock::new(None);
    let now = stamp();
    let mut last = LAST.write().unwrap();
    if last.is_none() {
        *last = now;
        return false;
    }
    if *last == now {
        return false;
    }
    *last = now;
    *store().write().unwrap() = Arc::new(Catalog::load());
    GENERATION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    true
}

static GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Bumped on every reload, so caches built from the catalog know when to start over.
pub fn generation() -> u64 {
    GENERATION.load(std::sync::atomic::Ordering::Relaxed)
}

/// A placement transform: quarter turns and mirrors in the horizontal plane, then a shift.
#[derive(Clone, Copy)]
struct Tf {
    /// x' = m[0][0] x + m[0][1] z, z' = m[1][0] x + m[1][1] z.
    m: [[f32; 2]; 2],
    t: Vec3,
}

impl Tf {
    const ID: Tf = Tf {
        m: [[1.0, 0.0], [0.0, 1.0]],
        t: Vec3::ZERO,
    };

    fn shift(t: Vec3) -> Tf {
        Tf { t, ..Tf::ID }
    }

    /// `q` quarter turns about the origin (+x toward -z: counter-clockwise seen from above).
    fn turn(q: u8) -> Tf {
        let m = match q % 4 {
            0 => [[1.0, 0.0], [0.0, 1.0]],
            1 => [[0.0, 1.0], [-1.0, 0.0]],
            2 => [[-1.0, 0.0], [0.0, -1.0]],
            _ => [[0.0, -1.0], [1.0, 0.0]],
        };
        Tf { m, t: Vec3::ZERO }
    }

    fn mirror(axis: Axis) -> Tf {
        let m = match axis {
            Axis::X => [[-1.0, 0.0], [0.0, 1.0]],
            Axis::Z => [[1.0, 0.0], [0.0, -1.0]],
        };
        Tf { m, t: Vec3::ZERO }
    }

    fn point(&self, p: Vec3) -> Vec3 {
        vec3(
            self.m[0][0] * p.x + self.m[0][1] * p.z + self.t.x,
            p.y + self.t.y,
            self.m[1][0] * p.x + self.m[1][1] * p.z + self.t.z,
        )
    }

    /// This transform applied after `inner`.
    fn then(&self, inner: &Tf) -> Tf {
        let a = self.m;
        let b = inner.m;
        let mut m = [[0.0; 2]; 2];
        for (i, row) in m.iter_mut().enumerate() {
            for (j, v) in row.iter_mut().enumerate() {
                *v = a[i][0] * b[0][j] + a[i][1] * b[1][j];
            }
        }
        Tf {
            m,
            t: self.point(inner.t),
        }
    }

    fn aabb(&self, min: Vec3, max: Vec3) -> Aabb {
        let (a, b) = (self.point(min), self.point(max));
        Aabb {
            min: a.min(b),
            max: a.max(b),
        }
    }
}

fn v3(t: V3) -> Vec3 {
    vec3(t.0, t.1, t.2)
}

/// A chunk turned into geometry, in world coordinates.
#[derive(Default)]
pub struct Built {
    pub boxes: Vec<(Aabb, Mat)>,
    /// Stairwell centers on the walking surface, and whether the flights run along x.
    pub stairs: Vec<(Vec3, bool)>,
    pub growth: Vec<(Aabb, GrowthKind)>,
    pub subs: Vec<(Aabb, Vec<String>)>,
    /// Modules that were asked for but are not in the catalog.
    pub missing: Vec<String>,
}

struct Expander<'a> {
    catalog: &'a Catalog,
    rng: Rng,
    out: Built,
    /// The accent color in effect (`Mat::Accent`).
    accent: (u8, u8, u8),
}

impl Expander<'_> {
    fn module(&mut self, name: &str, tf: &Tf, depth: usize, accent: Option<(u8, u8, u8)>) {
        let saved = self.accent;
        self.accent = accent.unwrap_or(saved);
        match self.catalog.modules.get(name) {
            Some(m) if depth < MAX_DEPTH => self.parts(&m.parts, tf, depth + 1),
            Some(_) => self.out.missing.push(format!("{name} (too deep)")),
            None => self.out.missing.push(name.to_string()),
        }
        self.accent = saved;
    }

    fn parts(&mut self, parts: &[Part], tf: &Tf, depth: usize) {
        for p in parts {
            match p {
                Part::Box { at, size, mat } => {
                    let a = v3(*at);
                    let mat = if *mat == Mat::Accent {
                        Mat::Paint(self.accent)
                    } else {
                        *mat
                    };
                    self.out.boxes.push((tf.aabb(a, a + v3(*size)), mat));
                }
                Part::Module {
                    name,
                    at,
                    turn,
                    accent,
                } => {
                    let t = tf.then(&Tf::shift(v3(*at)).then(&Tf::turn(*turn)));
                    self.module(name, &t, depth, *accent);
                }
                Part::Slot {
                    at,
                    turn,
                    pick,
                    accents,
                } => {
                    if pick.is_empty() {
                        continue;
                    }
                    let i = self.rng.range_i(0, pick.len() as i32) as usize;
                    let accent = (!accents.is_empty())
                        .then(|| accents[self.rng.range_i(0, accents.len() as i32) as usize]);
                    if let Some(name) = &pick[i] {
                        let t = tf.then(&Tf::shift(v3(*at)).then(&Tf::turn(*turn)));
                        self.module(name, &t, depth, accent);
                    }
                }
                Part::Mirror2 { axis, at, parts } => {
                    self.parts(parts, tf, depth);
                    let c = match axis {
                        Axis::X => vec3(*at, 0.0, 0.0),
                        Axis::Z => vec3(0.0, 0.0, *at),
                    };
                    let m = Tf::shift(c).then(&Tf::mirror(*axis)).then(&Tf::shift(-c));
                    self.parts(parts, &tf.then(&m), depth);
                }
                Part::Mirror4 { center, parts } => {
                    let c = vec3(center.0, 0.0, center.1);
                    for q in 0..4 {
                        let m = Tf::shift(c).then(&Tf::turn(q)).then(&Tf::shift(-c));
                        self.parts(parts, &tf.then(&m), depth);
                    }
                }
                Part::Repeat {
                    from,
                    step,
                    count,
                    parts,
                } => {
                    for i in 0..*count {
                        let shift = v3(*from) + v3(*step) * i as f32;
                        self.parts(parts, &tf.then(&Tf::shift(shift)), depth);
                    }
                }
                Part::Stair { at, along_x } => {
                    let c = tf.point(vec3(at.0, 0.0, at.1));
                    // A turn by an odd number of quarters swaps the axes.
                    let swapped = tf.m[0][0] == 0.0;
                    self.out.stairs.push((c, *along_x != swapped));
                }
                Part::Growth { area, kind } => {
                    let r = tf.aabb(vec3(area.0.0, 0.0, area.0.1), vec3(area.1.0, 4.0, area.1.1));
                    self.out.growth.push((r, *kind));
                }
                Part::Sub { area, labels } => {
                    let r = tf.aabb(vec3(area.0.0, 0.0, area.0.1), vec3(area.1.0, 0.0, area.1.1));
                    self.out.subs.push((r, labels.clone()));
                }
            }
        }
    }
}

/// Builds a chunk placed with its corner at `origin` (world, on its walking surface), turned by
/// `turn` quarter turns within its footprint. `seed` picks the slots.
pub fn build(chunk: &Chunk, catalog: &Catalog, origin: Vec3, turn: u8, seed: u64) -> Built {
    let w = chunk.size.0 as f32 * super::city::LOT;
    let d = chunk.size.1 as f32 * super::city::LOT;
    // Turn about the footprint's center, so the turned chunk covers the same lots (square chunks;
    // others swap their sides).
    let c = vec3(w * 0.5, 0.0, d * 0.5);
    let c2 = if turn % 2 == 1 {
        vec3(c.z, 0.0, c.x)
    } else {
        c
    };
    let tf = Tf::shift(origin + c2)
        .then(&Tf::turn(turn))
        .then(&Tf::shift(-c));
    let mut e = Expander {
        catalog,
        rng: Rng::new(seed),
        out: Built::default(),
        accent: (150, 150, 150),
    };
    e.parts(&chunk.parts, &tf, 0);
    e.out
}

/// A module alone, at the origin, unturned (props are built this way, `props.rs`). `None` if the
/// catalog has no such module.
pub fn build_module(catalog: &Catalog, name: &str) -> Option<Built> {
    catalog.modules.get(name)?;
    let mut e = Expander {
        catalog,
        rng: Rng::new(0),
        out: Built::default(),
        accent: (150, 150, 150),
    };
    e.module(name, &Tf::ID, 0, None);
    Some(e.out)
}

/// A chunk shown alone (`--chunk NAME`): the city around it is a bare deck.
pub struct Preview {
    pub name: String,
}

static PREVIEW: OnceLock<Preview> = OnceLock::new();

/// Shows only this chunk, in front of the spawn (before anything is generated).
pub fn set_preview(name: String) {
    let _ = PREVIEW.set(Preview { name });
}

pub fn preview() -> Option<&'static Preview> {
    PREVIEW.get()
}

/// The previewed chunk's footprint: its corner lot and size, in front of the spawn (which looks
/// toward -z from the middle of its near edge).
pub fn preview_footprint() -> Option<((i32, i32), (i32, i32))> {
    let p = preview()?;
    let size = catalog().chunks.get(&p.name)?.size;
    Some(((-size.0 / 2, -size.1), size))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transforms_compose() {
        // A quarter turn twice is a half turn.
        let a = Tf::turn(1).then(&Tf::turn(1));
        let p = a.point(vec3(1.0, 0.0, 2.0));
        assert!((p - vec3(-1.0, 0.0, -2.0)).length() < 1e-5);
        // Mirror4 around a center keeps the center.
        let c = vec3(5.0, 0.0, 5.0);
        for q in 0..4 {
            let m = Tf::shift(c).then(&Tf::turn(q)).then(&Tf::shift(-c));
            assert!((m.point(c) - c).length() < 1e-5);
        }
    }

    #[test]
    fn the_catalog_loads_and_builds() {
        let c = catalog();
        assert!(c.errors.is_empty(), "{:?}", c.errors);
        for ch in c.chunks.values() {
            let b = build(ch, &c, Vec3::ZERO, 0, 1);
            assert!(b.missing.is_empty(), "{}: missing {:?}", ch.name, b.missing);
            assert!(!b.boxes.is_empty(), "{} is empty", ch.name);
            let (w, d) = (ch.size.0 as f32 * 24.0, ch.size.1 as f32 * 24.0);
            for (a, _) in &b.boxes {
                assert!(
                    a.min.x >= -0.01
                        && a.min.z >= -0.01
                        && a.max.x <= w + 0.01
                        && a.max.z <= d + 0.01,
                    "{}: box {:?}..{:?} outside the footprint",
                    ch.name,
                    a.min,
                    a.max
                );
            }
        }
    }
}
