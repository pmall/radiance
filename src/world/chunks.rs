//! Hand-written map chunks (`docs/chunks.md`): large pieces of city written as text, built from
//! boxes and from modules, placed by the seed.
//!
//! A chunk file holds one `Chunk`; a module file holds a list of `Module`s. Coordinates are meters,
//! local to the chunk (origin at its corner, `y = 0` on the walking surface of its floor). The
//! world is axis-aligned boxes, so turns are quarter turns. Files are read from disk at run time
//! and hot-reloaded (`poll`); the seed picks what `Slot`s hold.
//!
//! Chunks and modules are grouped by biome: `assets/biomes/<biome>/chunks/` and `.../modules/`,
//! plus the shared modules of `assets/modules/` (props, street furniture) every biome can use.

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

/// How the planner places a chunk (`planner.rs`). Left out, it follows from the chunk: 6 lots wide
/// or more an area, labeled `stair_hub` a hub, one lot a filler, else a block.
#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// A large designed place, one per area cell at most.
    Area,
    /// A smaller place (a light well, a pocket park), one per block cell and floor at most.
    Block,
    /// Frames a stair, which stands in its middle lot.
    Hub,
    /// Only fills the `Sub` openings of another chunk that ask for one of its labels.
    Sub,
    /// The city's background: one lot of housing, a yard, a depot.
    Filler,
}

fn one() -> u32 {
    1
}

/// A rectangle of lots, local to a chunk: first lot (x, z), and past the last one.
pub type LotRect = ((i32, i32), (i32, i32));

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
    #[serde(default)]
    pub role: Option<Role>,
    /// Lots left open on the chunk's floor and every floor above (a light well, a collapse): no
    /// deck there, so the light falls to the floor below.
    #[serde(default)]
    pub holes: Vec<LotRect>,
    pub parts: Vec<Part>,
}

impl Chunk {
    pub fn role(&self) -> Role {
        if let Some(r) = self.role {
            r
        } else if self.size.0.max(self.size.1) >= 6 {
            Role::Area
        } else if self.labels.iter().any(|l| l == "stair_hub") {
            Role::Hub
        } else if self.size == (1, 1) {
            Role::Filler
        } else {
            Role::Block
        }
    }

    /// Its holes once placed with corner lot `origin` and turned by `turn` quarter turns (as
    /// `build` turns it), in world lots.
    pub fn placed_holes(&self, origin: (i32, i32), turn: u8) -> Vec<LotRect> {
        let (w, d) = self.size;
        self.holes
            .iter()
            .map(|&((x0, z0), (x1, z1))| {
                // Lot (x, z) turned as `Tf::turn` turns points about the footprint's center.
                let lot = |x: i32, z: i32| match turn % 4 {
                    0 => (x, z),
                    1 => (z, w - 1 - x),
                    2 => (w - 1 - x, d - 1 - z),
                    _ => (d - 1 - z, x),
                };
                let (a, b) = (lot(x0, z0), lot(x1 - 1, z1 - 1));
                (
                    (origin.0 + a.0.min(b.0), origin.1 + a.1.min(b.1)),
                    (origin.0 + a.0.max(b.0) + 1, origin.1 + a.1.max(b.1) + 1),
                )
            })
            .collect()
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct Module {
    pub name: String,
    pub parts: Vec<Part>,
}

/// The chunks and modules one biome draws from, as loaded from disk: a biome is a collection of
/// chunks (`docs/chunks.md`, "Biomes"). Modules are the shared ones (`assets/modules/`), those of
/// the biomes it includes, then its own, later ones winning on a name.
#[derive(Default)]
pub struct Catalog {
    pub biome: String,
    pub chunks: HashMap<String, Chunk>,
    pub modules: HashMap<String, Module>,
}

/// A biome's own file (`assets/biomes/<name>/biome.ron`, optional).
#[derive(Deserialize, Default)]
struct BiomeFile {
    /// Biomes whose chunks and modules this one also draws from (its own win on a name).
    #[serde(default)]
    include: Vec<String>,
}

/// Biome whose catalog stands in for a biome without a folder.
pub const DEFAULT_BIOME: &str = "default";

/// Every biome's catalog.
#[derive(Default)]
pub struct Catalogs {
    pub biomes: HashMap<String, Arc<Catalog>>,
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

/// Biome folders under `<root>/biomes/`, sorted.
fn biome_dirs(root: &Path) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(root.join("biomes"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    dirs
}

fn read<T: serde::de::DeserializeOwned>(path: &Path, errors: &mut Vec<String>) -> Option<T> {
    match std::fs::read_to_string(path)
        .map_err(|e| e.to_string())
        .and_then(|s| ron_options().from_str::<T>(&s).map_err(|e| e.to_string()))
    {
        Ok(v) => Some(v),
        Err(e) => {
            errors.push(format!("{}: {e}", path.display()));
            None
        }
    }
}

fn read_modules(dir: &Path, errors: &mut Vec<String>) -> HashMap<String, Module> {
    ron_files(dir)
        .iter()
        .filter_map(|p| read::<Vec<Module>>(p, errors))
        .flatten()
        .map(|m| (m.name.clone(), m))
        .collect()
}

impl Catalogs {
    fn load() -> Catalogs {
        Catalogs::load_from(Path::new(ASSETS))
    }

    /// The catalogs of an assets folder (`assets/`, or a test's own).
    fn load_from(root: &Path) -> Catalogs {
        let mut errors = Vec::new();
        let shared = read_modules(&root.join("modules"), &mut errors);
        // Each biome's own files first, then the includes resolved.
        struct Own {
            include: Vec<String>,
            chunks: HashMap<String, Chunk>,
            modules: HashMap<String, Module>,
        }
        let mut own: HashMap<String, Own> = HashMap::new();
        for dir in biome_dirs(root) {
            let name = dir.file_name().unwrap().to_string_lossy().to_string();
            let file = dir.join("biome.ron");
            let include = if file.exists() {
                read::<BiomeFile>(&file, &mut errors)
                    .unwrap_or_default()
                    .include
            } else {
                Vec::new()
            };
            let chunks = ron_files(&dir.join("chunks"))
                .iter()
                .filter_map(|p| read::<Chunk>(p, &mut errors))
                .map(|c| (c.name.clone(), c))
                .collect();
            let modules = read_modules(&dir.join("modules"), &mut errors);
            own.insert(
                name,
                Own {
                    include,
                    chunks,
                    modules,
                },
            );
        }
        // A biome's whole collection: what it includes (recursively, each once), then its own.
        fn gather<'a>(
            name: &str,
            own: &'a HashMap<String, Own>,
            seen: &mut Vec<String>,
            out: &mut Vec<&'a Own>,
        ) {
            if seen.iter().any(|s| s == name) {
                return;
            }
            seen.push(name.to_string());
            if let Some(o) = own.get(name) {
                for i in &o.include {
                    gather(i, own, seen, out);
                }
                out.push(o);
            }
        }
        let mut biomes = HashMap::new();
        for name in own.keys() {
            let mut layers = Vec::new();
            gather(name, &own, &mut Vec::new(), &mut layers);
            let mut c = Catalog {
                biome: name.clone(),
                modules: shared.clone(),
                ..Catalog::default()
            };
            for o in layers {
                c.chunks
                    .extend(o.chunks.iter().map(|(k, v)| (k.clone(), v.clone())));
                c.modules
                    .extend(o.modules.iter().map(|(k, v)| (k.clone(), v.clone())));
            }
            biomes.insert(name.clone(), Arc::new(c));
        }
        for (name, o) in &own {
            for i in o.include.iter().filter(|i| !own.contains_key(*i)) {
                errors.push(format!("biome {name} includes {i}, which has no folder"));
            }
        }
        if !biomes.contains_key(DEFAULT_BIOME) {
            errors.push(format!("no assets/biomes/{DEFAULT_BIOME}/ folder"));
            biomes.insert(
                DEFAULT_BIOME.to_string(),
                Arc::new(Catalog {
                    biome: DEFAULT_BIOME.to_string(),
                    modules: shared,
                    ..Catalog::default()
                }),
            );
        }
        for e in &errors {
            eprintln!("chunks: {e}");
        }
        Catalogs { biomes, errors }
    }
}

fn store() -> &'static RwLock<Arc<Catalogs>> {
    static STORE: OnceLock<RwLock<Arc<Catalogs>>> = OnceLock::new();
    STORE.get_or_init(|| RwLock::new(Arc::new(Catalogs::load())))
}

/// Every biome's catalog, as currently loaded.
pub fn catalogs() -> Arc<Catalogs> {
    store().read().unwrap().clone()
}

/// The catalog of a biome (`biome::Biome::name`); a biome without a folder draws from the
/// default one.
pub fn catalog(biome: &str) -> Arc<Catalog> {
    let all = catalogs();
    all.biomes
        .get(biome)
        .or_else(|| all.biomes.get(DEFAULT_BIOME))
        .cloned()
        .unwrap_or_default()
}

/// The shared modules alone (with the default biome's): props and facade units are built from
/// these.
pub fn shared() -> Arc<Catalog> {
    catalog(DEFAULT_BIOME)
}

/// Newest modification time of the catalog's files.
fn stamp() -> Option<SystemTime> {
    let root = Path::new(ASSETS);
    let mut dirs = vec![root.join("modules")];
    for d in biome_dirs(root) {
        dirs.extend([d.join("chunks"), d.join("modules"), d.clone()]);
    }
    dirs.iter()
        .flat_map(|d| ron_files(d))
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
    *store().write().unwrap() = Arc::new(Catalogs::load());
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
    build_module_seeded(catalog, name, 0)
}

/// `build_module`, its slots picked by `seed`.
pub fn build_module_seeded(catalog: &Catalog, name: &str, seed: u64) -> Option<Built> {
    catalog.modules.get(name)?;
    let mut e = Expander {
        catalog,
        rng: Rng::new(seed),
        out: Built::default(),
        accent: (150, 150, 150),
    };
    e.module(name, &Tf::ID, 0, None);
    Some(e.out)
}

/// A chunk shown alone (`--chunk NAME` or `--chunk BIOME/NAME`): the city around it is a bare
/// deck.
pub struct Preview {
    pub name: String,
    pub biome: &'static str,
}

static PREVIEW: OnceLock<Preview> = OnceLock::new();

/// Finds a chunk by `NAME` (in the default biome first, then the others) or `BIOME/NAME`: its
/// biome, if it exists.
pub fn find(spec: &str) -> Option<(String, String)> {
    let all = catalogs();
    if let Some((b, n)) = spec.split_once('/') {
        return all
            .biomes
            .get(b)
            .filter(|c| c.chunks.contains_key(n))
            .map(|_| (b.to_string(), n.to_string()));
    }
    let mut names: Vec<&String> = all.biomes.keys().collect();
    names.sort_by_key(|b| (b.as_str() != DEFAULT_BIOME, b.as_str()));
    names
        .into_iter()
        .find(|b| all.biomes[*b].chunks.contains_key(spec))
        .map(|b| (b.clone(), spec.to_string()))
}

/// Shows only this chunk, in front of the spawn (before anything is generated).
pub fn set_preview(biome: String, name: String) {
    let biome: &'static str = Box::leak(biome.into_boxed_str());
    let _ = PREVIEW.set(Preview { name, biome });
}

pub fn preview() -> Option<&'static Preview> {
    PREVIEW.get()
}

/// The previewed chunk's footprint: its corner lot and size, in front of the spawn (which looks
/// toward -z from the middle of its near edge).
pub fn preview_footprint() -> Option<((i32, i32), (i32, i32))> {
    let p = preview()?;
    let size = catalog(p.biome).chunks.get(&p.name)?.size;
    Some(((-size.0 / 2, -size.1), size))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn biomes_include_other_biomes() {
        let root = std::env::temp_dir().join(format!("radiance-biomes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let write = |path: &str, text: &str| {
            let p = root.join(path);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, text).unwrap();
        };
        let chunk = |name: &str| {
            format!("Chunk(name: \"{name}\", size: (1, 1), parts: [Module(name: \"post\")])")
        };
        write("modules/a.ron", "[Module(name: \"post\", parts: [])]");
        write("biomes/default/chunks/house.ron", &chunk("house"));
        write("biomes/rust/biome.ron", "(include: [\"default\"])");
        write("biomes/rust/chunks/scrap.ron", &chunk("scrap"));
        write(
            "biomes/rust/modules/m.ron",
            "[Module(name: \"post\", parts: [Box(at: (0.0, 0.0, 0.0), size: (1.0, 1.0, 1.0), mat: Rust)])]",
        );
        let all = Catalogs::load_from(&root);
        let _ = std::fs::remove_dir_all(&root);
        assert!(all.errors.is_empty(), "{:?}", all.errors);
        let (d, r) = (&all.biomes["default"], &all.biomes["rust"]);
        assert!(d.chunks.contains_key("house") && !d.chunks.contains_key("scrap"));
        assert!(r.chunks.contains_key("house") && r.chunks.contains_key("scrap"));
        // The biome's own module wins over the shared one of the same name.
        assert!(d.modules["post"].parts.is_empty());
        assert_eq!(r.modules["post"].parts.len(), 1);
    }

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
        let all = catalogs();
        assert!(all.errors.is_empty(), "{:?}", all.errors);
        assert!(all.biomes.contains_key(DEFAULT_BIOME));
        // Every biome of the map has its collection (its own folder, or the default one).
        for b in crate::world::biome::BIOMES {
            assert!(
                !catalog(b.name).chunks.is_empty(),
                "biome {} has no chunks",
                b.name
            );
        }
        for c in all.biomes.values() {
            for ch in c.chunks.values() {
                let b = build(ch, c, Vec3::ZERO, 0, 1);
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
}
