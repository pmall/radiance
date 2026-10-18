//! Props: what people left in the streets. Chunks place them by module name (`by_module`), whole:
//! most are box-built modules from `assets/modules/props.ron` (and the street furniture of
//! `street.ron`), drawn in the same pixel-art materials as the hand-written chunks; the street
//! light (`street_light`) is built here, as a smooth mesh, since a box pole and arm read poorly.
//!
//! Each biome builds its props from its own catalog: a biome whose folder has a module of a
//! prop's name (`bench`, or `street_light`, which then replaces the built one) gets its own
//! version of that prop, the other biomes keep theirs. Each biome's props are drawn in its own
//! pixel art (`render/pixels.rs`), so every biome has meshes of its own, even for shared props.
//!
//! The seed decides what goes where; the renderer draws every prop of a kind in one instanced call
//! per mesh (`src/render/props.rs`), and they get the same shading, outlines, fog, shadows and
//! flower light as everything else. Props are built once, when first used: editing their modules
//! takes a restart (chunks hot-reload, props do not).

use std::collections::HashMap;
use std::f32::consts::FRAC_PI_2;
use std::sync::OnceLock;

use glam::{Mat4, Quat, Vec3, vec3};

use super::biome::BIOMES;
use super::chunks::{self, Catalog};

/// Every prop the city can place.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Prop {
    // Street furniture.
    StreetLight,
    Bench,
    Bin,
    Planter,
    UtilityBox,
    // What people left.
    TrashBag,
    CardboardBox,
    Crate,
    Barrel,
    BarrelBlue,
    TerraceTable,
    TerraceChair,
    Armchair,
    Sofa,
    BedFrame,
    ClayPot,
    // Debris (`leftovers.rs`).
    DebrisLumps,
    DebrisHeap,
    DebrisSlab,
    DebrisBricks,
    DebrisFrame,
    DebrisPanel,
    DebrisPipe,
    DebrisTyre,
    DebrisPaper,
}

/// How a prop is loaded.
struct Def {
    prop: Prop,
    /// The module chunks place it by, and build it from (`assets/modules/`, or a biome's own).
    name: &'static str,
    /// Built here instead where the catalog has no module of that name.
    built: Option<fn() -> Vec<PropVert>>,
    /// Collision only around a pole of this half width (a street light's arm overhangs the road).
    pole: Option<f32>,
}

const fn module(prop: Prop, name: &'static str) -> Def {
    Def {
        prop,
        name,
        built: None,
        pole: None,
    }
}

const DEFS: &[Def] = &[
    Def {
        prop: Prop::StreetLight,
        name: "street_light",
        built: Some(street_light),
        pole: Some(0.18),
    },
    module(Prop::Bench, "bench"),
    module(Prop::Bin, "bin"),
    module(Prop::Planter, "planter"),
    module(Prop::UtilityBox, "utility_box"),
    module(Prop::TrashBag, "trash_bag"),
    module(Prop::CardboardBox, "cardboard_box"),
    module(Prop::Crate, "crate"),
    module(Prop::Barrel, "barrel"),
    module(Prop::BarrelBlue, "barrel_blue"),
    module(Prop::TerraceTable, "terrace_table"),
    module(Prop::TerraceChair, "terrace_chair"),
    module(Prop::Armchair, "armchair"),
    module(Prop::Sofa, "sofa"),
    module(Prop::BedFrame, "bed_frame"),
    module(Prop::ClayPot, "clay_pot"),
    module(Prop::DebrisLumps, "debris_lumps"),
    module(Prop::DebrisHeap, "debris_heap"),
    module(Prop::DebrisSlab, "debris_slab"),
    module(Prop::DebrisBricks, "debris_bricks"),
    module(Prop::DebrisFrame, "debris_frame"),
    module(Prop::DebrisPanel, "debris_panel"),
    module(Prop::DebrisPipe, "debris_pipe"),
    module(Prop::DebrisTyre, "debris_tyre"),
    module(Prop::DebrisPaper, "debris_paper"),
];

/// The prop a module of the catalog is drawn as, if it is one: chunks place these instanced
/// (`chunks::Built::props`) instead of building their boxes.
pub fn by_module(name: &str) -> Option<Prop> {
    DEFS.iter().find(|d| d.name == name).map(|d| d.prop)
}

/// A prop's triangles, standing at the origin (base at y = 0, centered in x and z), facing +z.
pub struct PropMesh {
    /// Three vertices per triangle.
    pub verts: Vec<PropVert>,
    /// Half size of the footprint (x, z) and height.
    pub half: Vec3,
    /// Half width of the collision pole, if the prop only collides around one.
    pub pole: Option<f32>,
    /// Where the mesh's origin (the footprint's center, on the floor) lies in its module.
    pub base: Vec3,
    /// The biome whose pixel art it is drawn in (`biome::variant_of`).
    pub biome: u8,
}

#[derive(Clone, Copy, PartialEq)]
pub struct PropVert {
    pub pos: Vec3,
    pub normal: Vec3,
    /// sRGB color.
    pub color: [u8; 3],
    /// Chunk material (`chunks::Mat::index`) painted in pixel art, or `None` for a plain surface.
    pub mat: Option<u8>,
}

/// A module's boxes as triangles, from `cat`. `None` if it has no such module.
fn module_verts(cat: &Catalog, name: &str) -> Option<Vec<PropVert>> {
    let built = chunks::build_module(cat, name)?;
    let mut verts = Vec::new();
    for (aabb, mat) in &built.boxes {
        let size = aabb.max - aabb.min;
        let c = mat.color();
        for (n, corners) in super::BOX_FACES {
            for k in [0, 1, 2, 0, 2, 3] {
                verts.push(PropVert {
                    pos: aabb.min + Vec3::from(corners[k]) * size,
                    normal: n,
                    color: [c.r, c.g, c.b],
                    mat: Some(mat.index()),
                });
            }
        }
    }
    Some(verts)
}

/// A prop's mesh as a catalog builds it, for biome `biome`: from its module there, else (for the
/// props built here) from code, else empty.
fn build(d: &Def, cat: &Catalog, biome: u8) -> PropMesh {
    let raw = module_verts(cat, d.name)
        .or_else(|| d.built.map(|build| build()))
        .unwrap_or_else(|| {
            eprintln!("props: no module {:?} for the {} biome", d.name, cat.biome);
            Vec::new()
        });
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for v in &raw {
        lo = lo.min(v.pos);
        hi = hi.max(v.pos);
    }
    if raw.is_empty() {
        (lo, hi) = (Vec3::ZERO, Vec3::splat(0.1));
    }
    let base = vec3((lo.x + hi.x) * 0.5, lo.y, (lo.z + hi.z) * 0.5);
    let verts = raw
        .into_iter()
        .map(|v| PropVert {
            pos: v.pos - base,
            ..v
        })
        .collect();
    let size = hi - lo;
    PropMesh {
        verts,
        half: vec3(size.x * 0.5, size.y, size.z * 0.5),
        pole: d.pole,
        base,
        biome,
    }
}

/// Every biome's props: their meshes, and which one each (prop, biome) uses.
pub struct Meshes {
    pub list: Vec<PropMesh>,
    of: HashMap<(Prop, u8), usize>,
}

impl Meshes {
    /// From each biome's catalog (`catalog(i)` for the biome of index `i`, `count` of them).
    fn load(count: usize, catalog: impl Fn(usize) -> std::sync::Arc<Catalog>) -> Meshes {
        let mut m = Meshes {
            list: Vec::new(),
            of: HashMap::new(),
        };
        for i in 0..count {
            let cat = catalog(i);
            for d in DEFS {
                m.of.insert((d.prop, i as u8), m.list.len());
                m.list.push(build(d, &cat, i as u8));
            }
        }
        m
    }

    /// Index in `list` of a prop's mesh in a biome (the default biome's for an unknown one).
    pub fn index(&self, prop: Prop, biome: u8) -> usize {
        self.of
            .get(&(prop, biome))
            .or_else(|| self.of.get(&(prop, 0)))
            .copied()
            .unwrap_or(0)
    }
}

/// Every biome's prop meshes (all load on first use).
pub fn meshes() -> &'static Meshes {
    static MESHES: OnceLock<Meshes> = OnceLock::new();
    MESHES.get_or_init(|| Meshes::load(BIOMES.len(), |i| chunks::catalog(BIOMES[i].name)))
}

/// The mesh of a prop in a biome (`biome::variant_of`).
pub fn mesh(prop: Prop, biome: u8) -> &'static PropMesh {
    let m = meshes();
    &m.list[m.index(prop, biome)]
}

/// The biome a catalog's props are drawn in: the biome of that name, or the default one.
pub fn biome_of(cat: &Catalog) -> u8 {
    BIOMES.iter().position(|b| b.name == cat.biome).unwrap_or(0) as u8
}

/// A prop placed in the world.
#[derive(Clone, Copy, Debug)]
pub struct Placed {
    pub prop: Prop,
    /// The biome whose version of the prop this is (`biome::variant_of`).
    pub biome: u8,
    /// Center of the footprint, on the floor.
    pub pos: Vec3,
    /// Rotation about the vertical (radians).
    pub yaw: f32,
}

impl Placed {
    /// Its mesh's index in `meshes().list`.
    pub fn mesh_index(&self) -> usize {
        meshes().index(self.prop, self.biome)
    }

    /// The axis-aligned box the prop occupies (its collision).
    pub fn aabb(&self) -> super::Aabb {
        let m = mesh(self.prop, self.biome);
        if let Some(r) = m.pole {
            // The pole stands at the module's origin, off the footprint's center.
            let pole = self.pos - Quat::from_rotation_y(self.yaw) * vec3(m.base.x, 0.0, m.base.z);
            return super::Aabb {
                min: pole - vec3(r, 0.0, r),
                max: pole + vec3(r, m.half.y, r),
            };
        }
        let (s, c) = self.yaw.sin_cos();
        let hx = (m.half.x * c).abs() + (m.half.z * s).abs();
        let hz = (m.half.x * s).abs() + (m.half.z * c).abs();
        super::Aabb {
            min: self.pos - vec3(hx, 0.0, hz),
            max: self.pos + vec3(hx, m.half.y, hz),
        }
    }

    /// Model to world transform.
    pub fn transform(&self) -> Mat4 {
        Mat4::from_rotation_translation(Quat::from_rotation_y(self.yaw), self.pos)
    }
}

/// Triangle soup builder for the props built here.
struct Builder {
    verts: Vec<PropVert>,
    color: [u8; 3],
}

impl Builder {
    /// A triangle, wound counter-clockwise seen from the side its normals face.
    fn tri(&mut self, a: Vec3, b: Vec3, c: Vec3, na: Vec3, nb: Vec3, nc: Vec3) {
        let flip = (b - a).cross(c - a).dot(na + nb + nc) < 0.0;
        let (b, c, nb, nc) = if flip { (c, b, nc, nb) } else { (b, c, nb, nc) };
        for (pos, normal) in [(a, na), (b, nb), (c, nc)] {
            self.verts.push(PropVert {
                pos,
                normal,
                color: self.color,
                mat: None,
            });
        }
    }

    /// A tube along `path` with a radius per point, `sides` around, smooth shaded, capped.
    fn tube(&mut self, path: &[Vec3], radius: &[f32], sides: usize) {
        let rings: Vec<Vec<(Vec3, Vec3)>> = (0..path.len())
            .map(|i| {
                let ahead = path[(i + 1).min(path.len() - 1)] - path[i.saturating_sub(1)];
                let t = ahead.normalize();
                let (u, v) = t.any_orthonormal_pair();
                (0..sides)
                    .map(|k| {
                        let a = std::f32::consts::TAU * k as f32 / sides as f32;
                        let n = u * a.cos() + v * a.sin();
                        (path[i] + n * radius[i], n)
                    })
                    .collect()
            })
            .collect();
        for w in rings.windows(2) {
            for k in 0..sides {
                let k1 = (k + 1) % sides;
                let (a, b, c, d) = (w[0][k], w[0][k1], w[1][k1], w[1][k]);
                self.tri(a.0, c.0, b.0, a.1, c.1, b.1);
                self.tri(a.0, d.0, c.0, a.1, d.1, c.1);
            }
        }
        let last = path.len() - 1;
        for (ring, end, before) in [
            (&rings[0], path[0], path[1]),
            (&rings[last], path[last], path[last - 1]),
        ] {
            let n = (end - before).normalize();
            for k in 0..sides {
                self.tri(end, ring[k].0, ring[(k + 1) % sides].0, n, n, n);
            }
        }
    }

    /// A box with flat faces, centered at `c`, rotated by `rot`.
    fn cuboid(&mut self, c: Vec3, half: Vec3, rot: Quat) {
        let faces = [
            (Vec3::X, Vec3::Y, Vec3::Z),
            (Vec3::NEG_X, Vec3::Y, Vec3::NEG_Z),
            (Vec3::Y, Vec3::Z, Vec3::X),
            (Vec3::NEG_Y, Vec3::X, Vec3::Z),
            (Vec3::Z, Vec3::X, Vec3::Y),
            (Vec3::NEG_Z, Vec3::Y, Vec3::X),
        ];
        for (n, u, v) in faces {
            let p = |su: f32, sv: f32| c + rot * ((n + u * su + v * sv) * half);
            let rn = rot * n;
            let (a, b, cc, d) = (p(-1.0, -1.0), p(1.0, -1.0), p(1.0, 1.0), p(-1.0, 1.0));
            self.tri(a, b, cc, rn, rn, rn);
            self.tri(a, cc, d, rn, rn, rn);
        }
    }
}

/// A modern street light: a plinth, a tapered pole, an arm curving out over the road (+z) and a
/// flat luminaire head with its dead lens underneath.
fn street_light() -> Vec<PropVert> {
    let mut b = Builder {
        verts: Vec::new(),
        color: [64, 70, 78],
    };
    // Plinth and pole.
    b.tube(
        &[
            vec3(0.0, 0.0, 0.0),
            vec3(0.0, 0.45, 0.0),
            vec3(0.0, 0.5, 0.0),
        ],
        &[0.2, 0.17, 0.12],
        8,
    );
    b.tube(
        &[
            vec3(0.0, 0.5, 0.0),
            vec3(0.0, 3.5, 0.0),
            vec3(0.0, 6.9, 0.0),
        ],
        &[0.11, 0.09, 0.07],
        10,
    );
    // The arm: up and out in a smooth curve, ending nearly level.
    let arm: Vec<Vec3> = (0..=8)
        .map(|i| {
            let t = i as f32 / 8.0;
            let a = t * FRAC_PI_2 * 0.92;
            vec3(0.0, 6.4 + 1.1 * a.sin(), 1.9 * (1.0 - a.cos()) + 0.25 * t)
        })
        .collect();
    let radii: Vec<f32> = (0..arm.len())
        .map(|i| 0.065 - 0.02 * i as f32 / 8.0)
        .collect();
    b.tube(&arm, &radii, 8);
    // The head, tilted slightly up like the arm's end, and its lens.
    let end = arm[arm.len() - 1];
    let tilt = Quat::from_rotation_x(-0.08);
    let head = end + vec3(0.0, -0.02, 0.3);
    b.cuboid(head, vec3(0.17, 0.06, 0.36), tilt);
    b.color = [44, 52, 58];
    b.cuboid(head + vec3(0.0, -0.07, 0.02), vec3(0.13, 0.015, 0.28), tilt);
    b.verts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_prop_loads_standing_at_the_origin() {
        for (d, b) in DEFS
            .iter()
            .flat_map(|d| (0..BIOMES.len()).map(move |b| (d, b)))
        {
            let m = mesh(d.prop, b as u8);
            assert!(!m.verts.is_empty(), "{:?} has no triangles", d.prop);
            assert!(m.half.x > 0.05 && m.half.z > 0.05, "{:?} is flat", d.prop);
            assert!(
                // Debris lies flat: papers are a couple of centimeters thick.
                (0.005..10.0).contains(&m.half.y),
                "{:?} is {} m tall",
                d.prop,
                m.half.y
            );
        }
    }

    #[test]
    fn a_biome_restyles_the_props_it_has_modules_for() {
        let default = chunks::catalog(chunks::DEFAULT_BIOME);
        let mut own = Catalog {
            biome: "other".into(),
            modules: default.modules.clone(),
            ..Catalog::default()
        };
        // A bench of its own, and a street light replacing the one built in code.
        for name in ["bench", "street_light"] {
            let parts = vec![chunks::Part::Box {
                at: (-1.0, 0.0, -0.5),
                size: (2.0, 1.0, 1.0),
                mat: chunks::Mat::Rust,
            }];
            let name = name.to_string();
            own.modules
                .insert(name.clone(), chunks::Module { name, parts });
        }
        let own = std::sync::Arc::new(own);
        let m = Meshes::load(2, |i| if i == 0 { default.clone() } else { own.clone() });
        for d in DEFS {
            let restyled = matches!(d.prop, Prop::Bench | Prop::StreetLight);
            let (a, b) = (&m.list[m.index(d.prop, 0)], &m.list[m.index(d.prop, 1)]);
            assert_eq!(a.verts != b.verts, restyled, "{:?}", d.prop);
            assert_eq!((a.biome, b.biome), (0, 1), "drawn in its biome's pixel art");
        }
        assert_eq!(m.list[m.index(Prop::Bench, 1)].half, vec3(1.0, 1.0, 0.5));
    }
}
