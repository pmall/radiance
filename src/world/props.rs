//! Props: photoscanned models from Poly Haven (CC0, `assets/models/polyhaven/`, imported and
//! decimated by `tools/import_props.mjs`), plus the few the city needs that no free model fits,
//! built here (the street light).
//!
//! The seed decides what goes where (`dressing.rs`); the models only give shapes and surfaces.
//! Each model keeps its own color texture, packed with the others into one atlas (`atlas`), and
//! the renderer draws every prop of a kind in one instanced call from a single mesh, so detailed
//! props cost no memory per lot. They get the same shading, outlines, fog, shadows and flower
//! light as everything else (rules in `docs/art-direction.md`).

use std::collections::HashMap;
use std::sync::OnceLock;

use glam::{Mat4, Quat, Vec3, vec3};
use sola_raylib::ffi;

/// Every prop the city can place.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Prop {
    // Street furniture.
    StreetLight,
    Bench,
    TrashCan,
    TrashCanRusty,
    UtilityBox,
    UtilityBoxWide,
    BarrierConcrete,
    BarrierJersey,
    // What people left.
    TrashBag,
    CardboardBox,
    Crate,
    CrateLong,
    BarrelRed,
    BarrelBlue,
    BarrelRusty,
    Tyre,
    Jerrycan,
    PropaneTank,
    Generator,
    HandTruck,
    Shelves,
    Television,
    Suitcase,
    PlasticChair,
    TerraceTable,
    TerraceChair,
    PicnicTable,
    Armchair,
    ArmchairGreen,
    LoungeChair,
    Sofa,
    SofaLeather,
    Wheelchair,
    BedFrame,
    ClayPot,
}

/// Where a prop's shape comes from.
enum Source {
    /// A model imported into `assets/models/polyhaven/`.
    Model(&'static [u8]),
    /// Built here.
    Built(fn() -> Vec<PropVert>),
}

/// How a prop is loaded.
struct Def {
    prop: Prop,
    source: Source,
    /// Height once placed, in meters, for the rare model not at real scale (most are).
    height: Option<f32>,
    /// Turn about the vertical, applied at load so the prop faces +z (its front, the side people
    /// use: a bench's seat, a car's nose).
    turn: f32,
    /// Collision only around a pole of this half width (a street light's arm overhangs the road).
    pole: Option<f32>,
}

macro_rules! model {
    ($file:literal) => {
        Source::Model(include_bytes!(concat!(
            "../../assets/models/polyhaven/",
            $file,
            ".glb"
        )))
    };
}

const fn def(prop: Prop, source: Source) -> Def {
    Def {
        prop,
        source,
        height: None,
        turn: 0.0,
        pole: None,
    }
}

const fn turned(prop: Prop, source: Source, turn: f32) -> Def {
    Def {
        turn,
        ..def(prop, source)
    }
}

use std::f32::consts::{FRAC_PI_2, PI};

const DEFS: &[Def] = &[
    Def {
        pole: Some(0.18),
        ..def(Prop::StreetLight, Source::Built(street_light))
    },
    def(Prop::Bench, model!("bench")),
    def(Prop::TrashCan, model!("trash_can")),
    def(Prop::TrashCanRusty, model!("trash_can_rusty")),
    def(Prop::UtilityBox, model!("utility_box")),
    def(Prop::UtilityBoxWide, model!("utility_box_wide")),
    def(Prop::BarrierConcrete, model!("barrier_concrete")),
    def(Prop::BarrierJersey, model!("barrier_jersey")),
    def(Prop::TrashBag, model!("trash_bag")),
    def(Prop::CardboardBox, model!("cardboard_box")),
    def(Prop::Crate, model!("crate")),
    def(Prop::CrateLong, model!("crate_long")),
    def(Prop::BarrelRed, model!("barrel_red")),
    def(Prop::BarrelBlue, model!("barrel_blue")),
    def(Prop::BarrelRusty, model!("barrel_rusty")),
    def(Prop::Tyre, model!("tyre")),
    def(Prop::Jerrycan, model!("jerrycan")),
    def(Prop::PropaneTank, model!("propane_tank")),
    def(Prop::Generator, model!("generator")),
    def(Prop::HandTruck, model!("hand_truck")),
    Def {
        height: Some(1.8),
        ..def(Prop::Shelves, model!("shelves"))
    },
    def(Prop::Television, model!("television")),
    def(Prop::Suitcase, model!("suitcase")),
    def(Prop::PlasticChair, model!("plastic_chair")),
    def(Prop::TerraceTable, model!("terrace_table")),
    turned(Prop::TerraceChair, model!("terrace_chair"), PI),
    turned(Prop::PicnicTable, model!("picnic_table"), FRAC_PI_2),
    def(Prop::Armchair, model!("armchair")),
    def(Prop::ArmchairGreen, model!("armchair_green")),
    def(Prop::LoungeChair, model!("lounge_chair")),
    def(Prop::Sofa, model!("sofa")),
    def(Prop::SofaLeather, model!("sofa_leather")),
    turned(Prop::Wheelchair, model!("wheelchair"), PI),
    def(Prop::BedFrame, model!("bed_frame")),
    def(Prop::ClayPot, model!("clay_pot")),
];

/// Every prop, in a stable order (the renderer keeps one mesh per prop).
pub fn all() -> impl Iterator<Item = Prop> {
    DEFS.iter().map(|d| d.prop)
}

/// A prop's triangles, standing at the origin (base at y = 0, centered in x and z), facing +z.
pub struct PropMesh {
    /// Three vertices per triangle.
    pub verts: Vec<PropVert>,
    /// Half size of the footprint (x, z) and height.
    pub half: Vec3,
    /// Half width of the collision pole, if the prop only collides around one.
    pub pole: Option<f32>,
}

#[derive(Clone, Copy)]
pub struct PropVert {
    pub pos: Vec3,
    pub normal: Vec3,
    /// sRGB color, multiplied with the texture where there is one.
    pub color: [u8; 3],
    /// Coordinates in the prop atlas (`atlas`), or `NO_UV` for untextured surfaces.
    pub uv: [f32; 2],
}

/// The prop textures packed side by side, `TILE` pixels each, RGBA8, rows from the top.
pub struct Atlas {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}

/// Side of one texture in the atlas, in pixels.
const TILE: usize = 512;
/// Tiles per atlas row.
const COLUMNS: usize = 8;

struct Image {
    w: usize,
    h: usize,
    rgba: Vec<u8>,
}

impl Image {
    /// Decodes a JPEG (raylib is built without it) or a PNG and scales it to a tile.
    fn decode_tile(bytes: &[u8], mime: &str) -> Option<Image> {
        let img = if mime.contains("png") {
            Self::decode_png(bytes)?
        } else {
            Self::decode_jpeg(bytes)?
        };
        Some(img.resized(TILE, TILE))
    }

    fn decode_jpeg(bytes: &[u8]) -> Option<Image> {
        let mut decoder = jpeg_decoder::Decoder::new(bytes);
        let pixels = decoder.decode().ok()?;
        let info = decoder.info()?;
        let (w, h) = (info.width as usize, info.height as usize);
        let rgba = match info.pixel_format {
            jpeg_decoder::PixelFormat::RGB24 => pixels
                .as_chunks::<3>()
                .0
                .iter()
                .flat_map(|p| [p[0], p[1], p[2], 255])
                .collect(),
            jpeg_decoder::PixelFormat::L8 => pixels.iter().flat_map(|&l| [l, l, l, 255]).collect(),
            _ => return None,
        };
        Some(Image { w, h, rgba })
    }

    fn decode_png(bytes: &[u8]) -> Option<Image> {
        unsafe {
            let mut img =
                ffi::LoadImageFromMemory(c".png".as_ptr(), bytes.as_ptr(), bytes.len() as i32);
            if img.data.is_null() {
                return None;
            }
            ffi::ImageFormat(
                &mut img,
                ffi::PixelFormat::PIXELFORMAT_UNCOMPRESSED_R8G8B8A8 as i32,
            );
            let (w, h) = (img.width as usize, img.height as usize);
            let rgba = std::slice::from_raw_parts(img.data as *const u8, w * h * 4).to_vec();
            ffi::UnloadImage(img);
            Some(Image { w, h, rgba })
        }
    }

    /// Box-filtered (when shrinking) or nearest (when growing) copy at another size.
    fn resized(self, w: usize, h: usize) -> Image {
        if (self.w, self.h) == (w, h) {
            return self;
        }
        let (fx, fy) = ((self.w / w).max(1), (self.h / h).max(1));
        let mut rgba = vec![0u8; w * h * 4];
        for y in 0..h {
            for x in 0..w {
                let (sx, sy) = (x * self.w / w, y * self.h / h);
                let mut sum = [0u32; 4];
                for dy in 0..fy {
                    for dx in 0..fx {
                        let i =
                            ((sy + dy).min(self.h - 1) * self.w + (sx + dx).min(self.w - 1)) * 4;
                        for (acc, &v) in sum.iter_mut().zip(&self.rgba[i..i + 4]) {
                            *acc += v as u32;
                        }
                    }
                }
                let at = (y * w + x) * 4;
                for (out, acc) in rgba[at..at + 4].iter_mut().zip(sum) {
                    *out = (acc / (fx * fy) as u32) as u8;
                }
            }
        }
        Image { w, h, rgba }
    }
}

/// Everything props need, loaded once.
struct Library {
    meshes: HashMap<Prop, PropMesh>,
    atlas: Atlas,
}

/// A vertex while loading: its texture as a tile index and the coordinates inside it.
struct Raw {
    pos: Vec3,
    normal: Vec3,
    color: [u8; 3],
    tile: Option<(usize, [f32; 2])>,
}

fn to_srgb(linear: f32) -> u8 {
    (linear.clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0).round() as u8
}

/// Reads a model's triangles, adding its textures to `tiles`.
fn read_model(glb: &[u8], tiles: &mut Vec<Image>) -> Vec<Raw> {
    let gltf = gltf::Gltf::from_slice(glb).expect("prop model");
    let blob = gltf.blob.as_deref().unwrap_or(&[]);
    // The model's images, as tile indices.
    let image_tiles: Vec<Option<usize>> = gltf
        .images()
        .map(|img| match img.source() {
            gltf::image::Source::View { view, mime_type } => {
                let bytes = &blob[view.offset()..view.offset() + view.length()];
                Image::decode_tile(bytes, mime_type).map(|i| {
                    tiles.push(i);
                    tiles.len() - 1
                })
            }
            _ => None,
        })
        .collect();

    let mut out = Vec::new();
    let mut stack: Vec<(gltf::Node, Mat4)> = gltf
        .default_scene()
        .or_else(|| gltf.scenes().next())
        .into_iter()
        .flat_map(|s| s.nodes())
        .map(|n| (n, Mat4::IDENTITY))
        .collect();
    while let Some((node, parent)) = stack.pop() {
        let m = parent * Mat4::from_cols_array_2d(&node.transform().matrix());
        stack.extend(node.children().map(|c| (c, m)));
        let Some(mesh) = node.mesh() else { continue };
        for prim in mesh.primitives() {
            let reader = prim.reader(|_| Some(blob));
            let Some(positions) = reader.read_positions() else {
                continue;
            };
            let positions: Vec<Vec3> = positions
                .map(|p| m.transform_point3(Vec3::from(p)))
                .collect();
            let normals: Vec<Vec3> = match reader.read_normals() {
                Some(n) => n
                    .map(|n| m.transform_vector3(Vec3::from(n)).normalize_or_zero())
                    .collect(),
                None => vec![Vec3::Y; positions.len()],
            };
            let uvs: Vec<[f32; 2]> = match reader.read_tex_coords(0) {
                Some(t) => t.into_f32().collect(),
                None => vec![[0.0, 0.0]; positions.len()],
            };
            let indices: Vec<usize> = match reader.read_indices() {
                Some(i) => i.into_u32().map(|i| i as usize).collect(),
                None => (0..positions.len()).collect(),
            };
            let pbr = prim.material().pbr_metallic_roughness();
            let f = pbr.base_color_factor();
            let color = [to_srgb(f[0]), to_srgb(f[1]), to_srgb(f[2])];
            let tile = pbr.base_color_texture().and_then(|t| {
                image_tiles
                    .get(t.texture().source().index())
                    .copied()
                    .flatten()
            });
            for tri in indices.as_chunks::<3>().0 {
                // Texture coordinates of a triangle wrapped into the tile together, so a triangle
                // past the texture's edge (tiled UVs) still maps continuously.
                let (u0, v0) = tri.iter().fold((f32::MAX, f32::MAX), |(u, v), &i| {
                    (u.min(uvs[i][0]), v.min(uvs[i][1]))
                });
                let shift = [u0.floor(), v0.floor()];
                for &i in tri {
                    out.push(Raw {
                        pos: positions[i],
                        normal: normals[i],
                        color,
                        tile: tile.map(|t| {
                            (
                                t,
                                [
                                    (uvs[i][0] - shift[0]).clamp(0.0, 1.0),
                                    (uvs[i][1] - shift[1]).clamp(0.0, 1.0),
                                ],
                            )
                        }),
                    });
                }
            }
        }
    }
    out
}

fn load() -> Library {
    let mut tiles = Vec::new();
    let mut raws: Vec<(&Def, Vec<Raw>)> = Vec::new();
    for d in DEFS {
        let raw = match d.source {
            Source::Model(glb) => read_model(glb, &mut tiles),
            Source::Built(build) => build()
                .into_iter()
                .map(|v| Raw {
                    pos: v.pos,
                    normal: v.normal,
                    color: v.color,
                    tile: None,
                })
                .collect(),
        };
        raws.push((d, raw));
    }

    let rows = tiles.len().div_ceil(COLUMNS).max(1);
    let (width, height) = (COLUMNS * TILE, rows * TILE);
    let mut rgba = vec![0u8; width * height * 4];
    for (i, img) in tiles.iter().enumerate() {
        let (x0, y0) = ((i % COLUMNS) * TILE, (i / COLUMNS) * TILE);
        for y in 0..img.h {
            let dst = ((y0 + y) * width + x0) * 4;
            rgba[dst..dst + img.w * 4]
                .copy_from_slice(&img.rgba[y * img.w * 4..(y + 1) * img.w * 4]);
        }
    }
    // Half a texel in from the tile's edges, so filtering never reaches the next tile.
    let inset = 0.5 / TILE as f32;
    let to_atlas = |t: usize, uv: [f32; 2]| {
        let (col, row) = ((t % COLUMNS) as f32, (t / COLUMNS) as f32);
        let u = uv[0].clamp(inset, 1.0 - inset);
        let v = uv[1].clamp(inset, 1.0 - inset);
        [(col + u) / COLUMNS as f32, (row + v) / rows as f32]
    };

    let meshes = raws
        .into_iter()
        .map(|(d, raw)| {
            let rot = Quat::from_rotation_y(d.turn);
            let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
            for v in &raw {
                let p = rot * v.pos;
                lo = lo.min(p);
                hi = hi.max(p);
            }
            let scale = d.height.map_or(1.0, |h| h / (hi.y - lo.y).max(1e-4));
            let base = vec3((lo.x + hi.x) * 0.5, lo.y, (lo.z + hi.z) * 0.5);
            let verts = raw
                .iter()
                .map(|v| PropVert {
                    pos: (rot * v.pos - base) * scale,
                    normal: rot * v.normal,
                    color: v.color,
                    uv: v.tile.map_or(super::NO_UV, |(t, uv)| to_atlas(t, uv)),
                })
                .collect();
            let size = (hi - lo) * scale;
            let mesh = PropMesh {
                verts,
                half: vec3(size.x * 0.5, size.y, size.z * 0.5),
                pole: d.pole,
            };
            (d.prop, mesh)
        })
        .collect();
    Library {
        meshes,
        atlas: Atlas {
            width,
            height,
            rgba,
        },
    }
}

fn library() -> &'static Library {
    static LIBRARY: OnceLock<Library> = OnceLock::new();
    LIBRARY.get_or_init(load)
}

/// The loaded mesh of a prop (all props load on first use).
pub fn mesh(prop: Prop) -> &'static PropMesh {
    &library().meshes[&prop]
}

/// The texture atlas the props' `uv` point into.
pub fn atlas() -> &'static Atlas {
    &library().atlas
}

/// A prop placed in the world.
#[derive(Clone, Copy, Debug)]
pub struct Placed {
    pub prop: Prop,
    /// Center of the footprint, on the floor.
    pub pos: Vec3,
    /// Rotation about the vertical (radians).
    pub yaw: f32,
}

impl Placed {
    /// The axis-aligned box the prop occupies (its collision).
    pub fn aabb(&self) -> super::Aabb {
        let m = mesh(self.prop);
        if let Some(r) = m.pole {
            return super::Aabb {
                min: self.pos - vec3(r, 0.0, r),
                max: self.pos + vec3(r, m.half.y, r),
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
                uv: super::NO_UV,
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
        for d in DEFS {
            let m = mesh(d.prop);
            assert!(!m.verts.is_empty(), "{:?} has no triangles", d.prop);
            assert!(m.half.x > 0.05 && m.half.z > 0.05, "{:?} is flat", d.prop);
            assert!(
                (0.1..10.0).contains(&m.half.y),
                "{:?} is {} m tall",
                d.prop,
                m.half.y
            );
            if let Some(h) = d.height {
                assert!((m.half.y - h).abs() < 1e-3, "{:?}", d.prop);
            }
        }
        let a = atlas();
        assert_eq!(a.rgba.len(), a.width * a.height * 4);
    }

    #[test]
    fn photoscanned_props_are_textured() {
        for d in DEFS.iter().filter(|d| matches!(d.source, Source::Model(_))) {
            let m = mesh(d.prop);
            let textured = m.verts.iter().filter(|v| v.uv[0] >= 0.0).count();
            assert!(
                textured * 2 > m.verts.len(),
                "{:?} is mostly untextured",
                d.prop
            );
        }
    }
}
