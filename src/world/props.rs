//! Props: models from free CC0 kits (Kenney, `assets/models/`), drawn in the city's own style.
//!
//! The seed decides what goes where (`dressing.rs`); the kits only give shapes. Their colors are
//! never shown as they are: every material is assigned a family (metal, concrete, wood, fabric,
//! paint, glass, dead plant), and only the kit color's lightness is kept and mapped onto that
//! family's ramp, so all metal looks like the same metal everywhere (rules in
//! `docs/art-direction.md`). Props are baked into their lot's mesh like plants, so they get the
//! same shading, outlines, fog, shadows and flower light as everything else.

use std::collections::HashMap;
use std::sync::OnceLock;

use glam::{Mat4, Vec3, vec3};
use sola_raylib::ffi;

/// Every prop the city can place.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Prop {
    Armchair,
    Sofa,
    BenchIndoor,
    Table,
    TableRound,
    Chair,
    CoatRack,
    Washer,
    Dryer,
    Trashcan,
    PottedPlant,
    Desk,
    DeskChair,
    Bookcase,
    BoxClosed,
    BoxOpen,
    FloorLamp,
    Television,
    Fridge,
    Bench,
    DumpsterClosed,
    DumpsterOpen,
    Barrier,
    BarrierConcrete,
    Pallet,
    Truck,
    Cone,
    RoadBarrier,
    TrafficLight,
    StreetSign,
    Sedan,
    Hatchback,
    Van,
    Taxi,
    Suv,
    Tire,
    ShoppingCart,
    Freezer,
    Barrel,
    Crate,
    Bucket,
}

/// How a prop is loaded: kit file, height in meters once placed.
struct Def {
    prop: Prop,
    glb: &'static [u8],
    /// Textures of the kit, by file name.
    textures: &'static [(&'static str, &'static [u8])],
    height: f32,
}

macro_rules! kit {
    ($kit:literal, $file:literal) => {
        include_bytes!(concat!(
            "../../assets/models/kenney/",
            $kit,
            "/",
            $file,
            ".glb"
        ))
    };
}
macro_rules! tex {
    ($kit:literal, $file:literal) => {
        (
            $file,
            include_bytes!(concat!(
                "../../assets/models/kenney/",
                $kit,
                "/Textures/",
                $file
            )) as &[u8],
        )
    };
}

const NO_TEX: &[(&str, &[u8])] = &[];
const ROADS: &[(&str, &[u8])] = &[tex!("city-kit-roads", "colormap.png")];
const CARS: &[(&str, &[u8])] = &[tex!("car-kit", "colormap.png")];
const MARKET: &[(&str, &[u8])] = &[tex!("mini-market", "colormap.png")];
const SURVIVAL: &[(&str, &[u8])] = &[tex!("survival-kit", "colormap.png")];
const RETRO: &[(&str, &[u8])] = &[
    tex!("retro-urban-kit", "concrete.png"),
    tex!("retro-urban-kit", "dirt.png"),
    tex!("retro-urban-kit", "metal.png"),
    tex!("retro-urban-kit", "metal_wall.png"),
    tex!("retro-urban-kit", "planks.png"),
    tex!("retro-urban-kit", "roof.png"),
    tex!("retro-urban-kit", "signs.png"),
    tex!("retro-urban-kit", "truck.png"),
    tex!("retro-urban-kit", "wall.png"),
];

const DEFS: &[Def] = &[
    Def {
        prop: Prop::Armchair,
        glb: kit!("furniture-kit", "loungeChair"),
        textures: NO_TEX,
        height: 0.9,
    },
    Def {
        prop: Prop::Sofa,
        glb: kit!("furniture-kit", "loungeSofa"),
        textures: NO_TEX,
        height: 0.9,
    },
    Def {
        prop: Prop::BenchIndoor,
        glb: kit!("furniture-kit", "bench"),
        textures: NO_TEX,
        height: 0.5,
    },
    Def {
        prop: Prop::Table,
        glb: kit!("furniture-kit", "table"),
        textures: NO_TEX,
        height: 0.76,
    },
    Def {
        prop: Prop::TableRound,
        glb: kit!("furniture-kit", "tableRound"),
        textures: NO_TEX,
        height: 0.76,
    },
    Def {
        prop: Prop::Chair,
        glb: kit!("furniture-kit", "chair"),
        textures: NO_TEX,
        height: 0.95,
    },
    Def {
        prop: Prop::CoatRack,
        glb: kit!("furniture-kit", "coatRackStanding"),
        textures: NO_TEX,
        height: 1.8,
    },
    Def {
        prop: Prop::Washer,
        glb: kit!("furniture-kit", "washer"),
        textures: NO_TEX,
        height: 0.9,
    },
    Def {
        prop: Prop::Dryer,
        glb: kit!("furniture-kit", "dryer"),
        textures: NO_TEX,
        height: 1.05,
    },
    Def {
        prop: Prop::Trashcan,
        glb: kit!("furniture-kit", "trashcan"),
        textures: NO_TEX,
        height: 0.9,
    },
    Def {
        prop: Prop::PottedPlant,
        glb: kit!("furniture-kit", "pottedPlant"),
        textures: NO_TEX,
        height: 1.1,
    },
    Def {
        prop: Prop::Desk,
        glb: kit!("furniture-kit", "desk"),
        textures: NO_TEX,
        height: 0.76,
    },
    Def {
        prop: Prop::DeskChair,
        glb: kit!("furniture-kit", "chairDesk"),
        textures: NO_TEX,
        height: 1.0,
    },
    Def {
        prop: Prop::Bookcase,
        glb: kit!("furniture-kit", "bookcaseOpen"),
        textures: NO_TEX,
        height: 1.9,
    },
    Def {
        prop: Prop::BoxClosed,
        glb: kit!("furniture-kit", "cardboardBoxClosed"),
        textures: NO_TEX,
        height: 0.5,
    },
    Def {
        prop: Prop::BoxOpen,
        glb: kit!("furniture-kit", "cardboardBoxOpen"),
        textures: NO_TEX,
        height: 0.5,
    },
    Def {
        prop: Prop::FloorLamp,
        glb: kit!("furniture-kit", "lampRoundFloor"),
        textures: NO_TEX,
        height: 1.7,
    },
    Def {
        prop: Prop::Television,
        glb: kit!("furniture-kit", "televisionVintage"),
        textures: NO_TEX,
        height: 0.6,
    },
    Def {
        prop: Prop::Fridge,
        glb: kit!("furniture-kit", "kitchenFridgeLarge"),
        textures: NO_TEX,
        height: 1.9,
    },
    Def {
        prop: Prop::Bench,
        glb: kit!("retro-urban-kit", "detail-bench"),
        textures: RETRO,
        height: 0.85,
    },
    Def {
        prop: Prop::DumpsterClosed,
        glb: kit!("retro-urban-kit", "detail-dumpster-closed"),
        textures: RETRO,
        height: 1.3,
    },
    Def {
        prop: Prop::DumpsterOpen,
        glb: kit!("retro-urban-kit", "detail-dumpster-open"),
        textures: RETRO,
        height: 1.3,
    },
    Def {
        prop: Prop::Barrier,
        glb: kit!("retro-urban-kit", "detail-barrier-type-a"),
        textures: RETRO,
        height: 1.0,
    },
    Def {
        prop: Prop::BarrierConcrete,
        glb: kit!("retro-urban-kit", "detail-barrier-strong-damaged"),
        textures: RETRO,
        height: 0.9,
    },
    Def {
        prop: Prop::Pallet,
        glb: kit!("retro-urban-kit", "pallet"),
        textures: RETRO,
        height: 0.15,
    },
    Def {
        prop: Prop::Truck,
        glb: kit!("retro-urban-kit", "truck-grey"),
        textures: RETRO,
        height: 2.9,
    },
    Def {
        prop: Prop::Cone,
        glb: kit!("city-kit-roads", "construction-cone"),
        textures: ROADS,
        height: 0.7,
    },
    Def {
        prop: Prop::RoadBarrier,
        glb: kit!("city-kit-roads", "construction-barrier"),
        textures: ROADS,
        height: 1.0,
    },
    Def {
        prop: Prop::TrafficLight,
        glb: kit!("city-kit-roads", "traffic-light"),
        textures: ROADS,
        height: 4.2,
    },
    Def {
        prop: Prop::StreetSign,
        glb: kit!("city-kit-roads", "road-sign-street"),
        textures: ROADS,
        height: 2.8,
    },
    Def {
        prop: Prop::Sedan,
        glb: kit!("car-kit", "sedan"),
        textures: CARS,
        height: 1.45,
    },
    Def {
        prop: Prop::Hatchback,
        glb: kit!("car-kit", "hatchback-sports"),
        textures: CARS,
        height: 1.3,
    },
    Def {
        prop: Prop::Van,
        glb: kit!("car-kit", "van"),
        textures: CARS,
        height: 2.0,
    },
    Def {
        prop: Prop::Taxi,
        glb: kit!("car-kit", "taxi"),
        textures: CARS,
        height: 1.6,
    },
    Def {
        prop: Prop::Suv,
        glb: kit!("car-kit", "suv"),
        textures: CARS,
        height: 1.7,
    },
    Def {
        prop: Prop::Tire,
        glb: kit!("car-kit", "debris-tire"),
        textures: CARS,
        height: 0.65,
    },
    Def {
        prop: Prop::ShoppingCart,
        glb: kit!("mini-market", "shopping-cart"),
        textures: MARKET,
        height: 1.0,
    },
    Def {
        prop: Prop::Freezer,
        glb: kit!("mini-market", "freezer"),
        textures: MARKET,
        height: 0.9,
    },
    Def {
        prop: Prop::Barrel,
        glb: kit!("survival-kit", "barrel"),
        textures: SURVIVAL,
        height: 0.95,
    },
    Def {
        prop: Prop::Crate,
        glb: kit!("survival-kit", "box-large"),
        textures: SURVIVAL,
        height: 0.8,
    },
    Def {
        prop: Prop::Bucket,
        glb: kit!("survival-kit", "bucket"),
        textures: SURVIVAL,
        height: 0.35,
    },
];

/// Material families, each with its own color ramp (sRGB, dark to light).
#[derive(Clone, Copy, PartialEq, Debug)]
enum Family {
    Metal,
    Concrete,
    Wood,
    Fabric,
    Plant,
    Glass,
    /// Painted surfaces: keep a muted version of the kit's hue.
    Paint,
}

impl Family {
    fn ramp(self) -> Option<([f32; 3], [f32; 3])> {
        match self {
            Family::Metal => Some(([56.0, 60.0, 70.0], [176.0, 182.0, 192.0])),
            Family::Concrete => Some(([80.0, 80.0, 82.0], [192.0, 188.0, 178.0])),
            Family::Wood => Some(([76.0, 54.0, 38.0], [176.0, 132.0, 94.0])),
            Family::Fabric => Some(([72.0, 48.0, 56.0], [168.0, 110.0, 114.0])),
            Family::Plant => Some(([48.0, 44.0, 30.0], [120.0, 104.0, 62.0])),
            Family::Glass => Some(([24.0, 32.0, 40.0], [70.0, 90.0, 104.0])),
            Family::Paint => None,
        }
    }
}

/// Family of a kit material, from its name (and, for palette-textured kits, its color).
fn family(material: &str, rgb: [f32; 3]) -> Family {
    let m = material.to_ascii_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| m.contains(w));
    if has(&["wood", "plank"]) {
        Family::Wood
    } else if has(&["carpet", "fabric", "pillow", "cloth"]) {
        Family::Fabric
    } else if has(&["plant", "leaf", "grass"]) {
        Family::Plant
    } else if has(&["glass", "window"]) {
        Family::Glass
    } else if has(&["concrete", "wall", "roof", "dirt", "stone"]) && !has(&["metal"]) {
        Family::Concrete
    } else if has(&["metal"]) {
        Family::Metal
    } else {
        // Palette textures and everything else: grays are metal, colors are paint.
        let (mx, mn) = (
            rgb[0].max(rgb[1]).max(rgb[2]),
            rgb[0].min(rgb[1]).min(rgb[2]),
        );
        if mx - mn < 0.12 {
            Family::Metal
        } else {
            Family::Paint
        }
    }
}

/// The city's color for a kit color (0..1 sRGB) of a given family.
fn remap(f: Family, rgb: [f32; 3]) -> [u8; 3] {
    let luma = 0.3 * rgb[0] + 0.59 * rgb[1] + 0.11 * rgb[2];
    let out = match f.ramp() {
        Some((dark, light)) => {
            let t = luma.clamp(0.0, 1.0).powf(0.8);
            std::array::from_fn(|i| dark[i] + (light[i] - dark[i]) * t)
        }
        None => {
            // Muted, darkened hue: old paint under dust.
            let mx = rgb[0].max(rgb[1]).max(rgb[2]).max(1e-4);
            let value = 0.3 + 0.5 * mx;
            std::array::from_fn(|i| {
                let c = rgb[i] / mx;
                let muted = 0.45 + 0.55 * c;
                muted * value * 255.0
            })
        }
    };
    out.map(|v: f32| v.clamp(0.0, 255.0) as u8)
}

/// A prop's triangles, scaled to its real height, standing at the origin (base at y = 0,
/// centered in x and z), facing -z as authored.
pub struct PropMesh {
    /// Three vertices per triangle.
    pub verts: Vec<PropVert>,
    /// Half size of the footprint (x, z) and height.
    pub half: Vec3,
}

#[derive(Clone, Copy)]
pub struct PropVert {
    pub pos: Vec3,
    pub normal: Vec3,
    pub color: [u8; 3],
}

struct Image {
    w: usize,
    h: usize,
    rgba: Vec<u8>,
}

impl Image {
    fn decode(png: &[u8]) -> Option<Image> {
        unsafe {
            let kind = c".png";
            let mut img = ffi::LoadImageFromMemory(kind.as_ptr(), png.as_ptr(), png.len() as i32);
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

    fn sample(&self, uv: [f32; 2]) -> [f32; 3] {
        let x = ((uv[0].rem_euclid(1.0)) * self.w as f32) as usize % self.w;
        let y = ((uv[1].rem_euclid(1.0)) * self.h as f32) as usize % self.h;
        let i = (y * self.w + x) * 4;
        [0, 1, 2].map(|c| self.rgba[i + c] as f32 / 255.0)
    }
}

fn load(def: &Def) -> PropMesh {
    let gltf = gltf::Gltf::from_slice(def.glb).expect("prop model");
    let blob = gltf.blob.as_deref().unwrap_or(&[]);
    let images: HashMap<&str, Image> = def
        .textures
        .iter()
        .filter_map(|(name, png)| Some((*name, Image::decode(png)?)))
        .collect();
    let image_of = |tex: &gltf::Texture| -> Option<&Image> {
        match tex.source().source() {
            gltf::image::Source::Uri { uri, .. } => images.get(uri.rsplit('/').next()?),
            _ => None,
        }
    };

    let mut verts = Vec::new();
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
            let mat = prim.material();
            let pbr = mat.pbr_metallic_roughness();
            let factor = pbr.base_color_factor();
            let texture = pbr
                .base_color_texture()
                .and_then(|t| image_of(&t.texture()));
            let name = mat.name().unwrap_or("");
            for &i in &indices {
                let mut rgb = [factor[0], factor[1], factor[2]];
                if let Some(img) = texture {
                    let s = img.sample(uvs[i]);
                    rgb = [rgb[0] * s[0], rgb[1] * s[1], rgb[2] * s[2]];
                }
                verts.push(PropVert {
                    pos: positions[i],
                    normal: normals[i],
                    color: remap(family(name, rgb), rgb),
                });
            }
        }
    }

    // Base on the ground, centered, scaled to the real height.
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for v in &verts {
        lo = lo.min(v.pos);
        hi = hi.max(v.pos);
    }
    let scale = def.height / (hi.y - lo.y).max(1e-4);
    let base = vec3((lo.x + hi.x) * 0.5, lo.y, (lo.z + hi.z) * 0.5);
    for v in &mut verts {
        v.pos = (v.pos - base) * scale;
    }
    let size = (hi - lo) * scale;
    PropMesh {
        verts,
        half: vec3(size.x * 0.5, size.y, size.z * 0.5),
    }
}

/// The loaded mesh of a prop (all props load on first use).
pub fn mesh(prop: Prop) -> &'static PropMesh {
    static MESHES: OnceLock<HashMap<Prop, PropMesh>> = OnceLock::new();
    let all = MESHES.get_or_init(|| DEFS.iter().map(|d| (d.prop, load(d))).collect());
    &all[&prop]
}

/// A prop placed in the world.
#[derive(Clone, Copy, Debug)]
pub struct Placed {
    pub prop: Prop,
    /// Center of the footprint, on the floor.
    pub pos: Vec3,
    /// Rotation about the vertical, in quarter turns plus a small jitter (radians).
    pub yaw: f32,
}

impl Placed {
    /// The axis-aligned box the prop occupies (its collision).
    pub fn aabb(&self) -> super::Aabb {
        let m = mesh(self.prop);
        let (s, c) = self.yaw.sin_cos();
        let hx = (m.half.x * c).abs() + (m.half.z * s).abs();
        let hz = (m.half.x * s).abs() + (m.half.z * c).abs();
        super::Aabb {
            min: self.pos - vec3(hx, 0.0, hz),
            max: self.pos + vec3(hx, m.half.y, hz),
        }
    }

    /// The prop's triangles in world space.
    pub fn world_verts(&self) -> impl Iterator<Item = PropVert> + '_ {
        let rot = glam::Quat::from_rotation_y(self.yaw);
        mesh(self.prop).verts.iter().map(move |v| PropVert {
            pos: self.pos + rot * v.pos,
            normal: rot * v.normal,
            color: v.color,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_prop_loads_at_its_height() {
        for d in DEFS {
            let m = mesh(d.prop);
            assert!(!m.verts.is_empty(), "{:?} has no triangles", d.prop);
            assert!((m.half.y - d.height).abs() < 1e-3, "{:?}", d.prop);
            assert!(m.half.x > 0.0 && m.half.z > 0.0, "{:?} is flat", d.prop);
        }
    }
}
