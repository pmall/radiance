//! Props drawn instanced: one GPU mesh per prop (`world::props`), and each frame every placement
//! near the camera in a single call per prop, so props cost no memory per lot.

use std::collections::HashMap;

use glam::{Mat4, Vec3};
use sola_raylib::ffi;

use super::mesh::raylib_copy;
use crate::world::props::{self, Placed, Prop};

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
    meshes: Vec<(Prop, ffi::Mesh)>,
}

/// The placements of one prop to draw this frame.
pub struct Batch {
    pub mesh: ffi::Mesh,
    pub transforms: Vec<ffi::Matrix>,
}

impl PropMeshes {
    /// Requires an open window (GL context).
    pub fn new() -> Self {
        let meshes = props::all().map(|p| (p, upload(p))).collect();
        Self { meshes }
    }

    /// The props of `lots` within reach, grouped by prop.
    pub fn batches<'a>(&self, lots: impl Iterator<Item = &'a [Placed]>, r: &Reach) -> Vec<Batch> {
        let mut by_prop: HashMap<Prop, Vec<ffi::Matrix>> = HashMap::new();
        for placed in lots {
            for p in placed {
                let d = p.pos - r.eye;
                let d = Vec3::new(d.x, d.y * VERTICAL, d.z);
                // Behind the camera, past the size of the biggest prop.
                let behind = r.ahead.is_some_and(|a| d.dot(a) < -5.0);
                if d.length_squared() < r.reach * r.reach && !behind {
                    by_prop
                        .entry(p.prop)
                        .or_default()
                        .push(matrix(p.transform()));
                }
            }
        }
        self.meshes
            .iter()
            .filter_map(|(prop, mesh)| {
                let transforms = by_prop.remove(prop)?;
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
            for (_, m) in &self.meshes {
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

fn matrix(m: Mat4) -> ffi::Matrix {
    let c = m.to_cols_array();
    ffi::Matrix {
        m0: c[0],
        m1: c[1],
        m2: c[2],
        m3: c[3],
        m4: c[4],
        m5: c[5],
        m6: c[6],
        m7: c[7],
        m8: c[8],
        m9: c[9],
        m10: c[10],
        m11: c[11],
        m12: c[12],
        m13: c[13],
        m14: c[14],
        m15: c[15],
    }
}

/// One prop's mesh, in the scene shader's vertex layout: uv.y is the material code (15 for chunk
/// materials in pixel art, with the material in uv2.x; 14 for plain props; the outline id comes
/// from the instance).
fn upload(prop: Prop) -> ffi::Mesh {
    let verts = &props::mesh(prop).verts;
    let mut pos = Vec::with_capacity(verts.len() * 3);
    let mut nrm = Vec::with_capacity(verts.len() * 3);
    let mut uv = Vec::with_capacity(verts.len() * 2);
    let mut uv2 = Vec::with_capacity(verts.len() * 2);
    let mut col = Vec::with_capacity(verts.len() * 4);
    for v in verts {
        pos.extend_from_slice(&v.pos.to_array());
        nrm.extend_from_slice(&v.normal.to_array());
        match v.mat {
            Some(m) => {
                uv.extend_from_slice(&[0.0, 15.0]);
                uv2.extend_from_slice(&[m as f32, 0.0]);
            }
            None => {
                uv.extend_from_slice(&[0.0, 14.0]);
                uv2.extend_from_slice(&[-1.0, -1.0]);
            }
        }
        col.extend_from_slice(&[v.color[0], v.color[1], v.color[2], 0]);
    }
    let mut raw: ffi::Mesh = unsafe { std::mem::zeroed() };
    raw.vertexCount = verts.len() as i32;
    raw.triangleCount = (verts.len() / 3) as i32;
    raw.vertices = raylib_copy(&pos);
    raw.normals = raylib_copy(&nrm);
    raw.texcoords = raylib_copy(&uv);
    raw.texcoords2 = raylib_copy(&uv2);
    raw.colors = raylib_copy(&col);
    unsafe { ffi::UploadMesh(&mut raw, false) };
    raw
}
