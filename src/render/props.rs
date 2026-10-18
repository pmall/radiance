//! Props drawn instanced: one GPU mesh per prop and biome version (`world::props`), and each frame
//! every placement near the camera in a single call per mesh, so props cost no memory per lot.

use std::collections::HashMap;

use glam::Vec3;
use sola_raylib::ffi;

use super::mesh::MeshData;
use crate::world::props::{self, Placed, PropMesh};

/// Which props are drawn: those within `reach` meters, height differences counting `VERTICAL`
/// times (the decks hide the other floors' props), and only in front of the camera if `ahead`.
pub struct Reach {
    pub eye: Vec3,
    pub reach: f32,
    pub ahead: Option<Vec3>,
}

/// How much a height difference counts toward a prop's distance.
const VERTICAL: f32 = 1.6;

pub struct PropMeshes {
    /// In the order of `props::meshes().list`.
    meshes: Vec<ffi::Mesh>,
}

/// The placements of one prop to draw this frame.
pub struct Batch {
    pub mesh: ffi::Mesh,
    pub transforms: Vec<ffi::Matrix>,
}

impl PropMeshes {
    /// Requires an open window (GL context).
    pub fn new() -> Self {
        let meshes = props::meshes().list.iter().map(upload).collect();
        Self { meshes }
    }

    /// The props of `lots` within reach, grouped by mesh.
    pub fn batches<'a>(&self, lots: impl Iterator<Item = &'a [Placed]>, r: &Reach) -> Vec<Batch> {
        let mut by_mesh: HashMap<usize, Vec<ffi::Matrix>> = HashMap::new();
        for placed in lots {
            for p in placed {
                let d = p.pos - r.eye;
                let d = Vec3::new(d.x, d.y * VERTICAL, d.z);
                // Behind the camera, past the size of the biggest prop.
                let behind = r.ahead.is_some_and(|a| d.dot(a) < -5.0);
                if d.length_squared() < r.reach * r.reach && !behind {
                    by_mesh
                        .entry(p.mesh_index())
                        .or_default()
                        .push(super::matrix(p.transform()));
                }
            }
        }
        self.meshes
            .iter()
            .enumerate()
            .filter_map(|(i, mesh)| {
                let transforms = by_mesh.remove(&i)?;
                Some(Batch {
                    mesh: *mesh,
                    transforms,
                })
            })
            .collect()
    }
}

impl Drop for PropMeshes {
    fn drop(&mut self) {
        unsafe {
            for m in &self.meshes {
                ffi::UnloadMesh(*m);
            }
        }
    }
}

/// Draws batches with a material whose shader takes `instanceTransform` (raylib's default name).
pub fn draw(batches: &[Batch], material: ffi::Material) {
    for b in batches {
        unsafe {
            ffi::DrawMeshInstanced(
                b.mesh,
                material,
                b.transforms.as_ptr(),
                b.transforms.len() as i32,
            );
        }
    }
}

pub fn triangles(batches: &[Batch]) -> i32 {
    batches
        .iter()
        .map(|b| b.mesh.triangleCount * b.transforms.len() as i32)
        .sum()
}

/// One prop's mesh, in the scene shader's vertex layout: uv.y is the material code (15 for chunk
/// materials in pixel art, with the material in uv2.x; 14 for plain props), its biome's variant
/// above it (16 per variant, as in `mesh.rs`); the outline id comes from the instance.
fn upload(prop: &PropMesh) -> ffi::Mesh {
    let variant = prop.biome as f32 * 16.0;
    let mut m = MeshData::default();
    for v in &prop.verts {
        m.pos.extend_from_slice(&v.pos.to_array());
        m.nrm.extend_from_slice(&v.normal.to_array());
        match v.mat {
            Some(mat) => {
                m.uv.extend_from_slice(&[0.0, variant + 15.0]);
                m.uv2.extend_from_slice(&[mat as f32, 0.0]);
            }
            None => {
                m.uv.extend_from_slice(&[0.0, variant + 14.0]);
                m.uv2.extend_from_slice(&[-1.0, -1.0]);
            }
        }
        m.col
            .extend_from_slice(&[v.color[0], v.color[1], v.color[2], 0]);
    }
    m.upload()
}
