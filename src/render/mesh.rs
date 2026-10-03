//! Builds one static GPU mesh from the world's blocks.

use glam::{Vec3, vec3};
use sola_raylib::ffi;

use crate::rng::mix64;
use crate::world::Lot;

pub struct WorldMesh {
    pub raw: ffi::Mesh,
}

/// Each face: outward normal and its 4 corners (counter-clockwise seen from outside),
/// as unit-cube corner selectors (0 = min, 1 = max per axis).
const FACES: [(Vec3, [[f32; 3]; 4]); 6] = [
    (
        vec3(1.0, 0.0, 0.0),
        [[1., 0., 1.], [1., 0., 0.], [1., 1., 0.], [1., 1., 1.]],
    ),
    (
        vec3(-1.0, 0.0, 0.0),
        [[0., 0., 0.], [0., 0., 1.], [0., 1., 1.], [0., 1., 0.]],
    ),
    (
        vec3(0.0, 1.0, 0.0),
        [[0., 1., 1.], [1., 1., 1.], [1., 1., 0.], [0., 1., 0.]],
    ),
    (
        vec3(0.0, -1.0, 0.0),
        [[0., 0., 0.], [1., 0., 0.], [1., 0., 1.], [0., 0., 1.]],
    ),
    (
        vec3(0.0, 0.0, 1.0),
        [[0., 0., 1.], [1., 0., 1.], [1., 1., 1.], [0., 1., 1.]],
    ),
    (
        vec3(0.0, 0.0, -1.0),
        [[1., 0., 0.], [0., 0., 0.], [0., 1., 0.], [1., 1., 0.]],
    ),
];

/// Copies a slice into memory owned by raylib (freed by `UnloadMesh`).
pub(super) fn raylib_copy<T: Copy>(data: &[T]) -> *mut T {
    let bytes = std::mem::size_of_val(data);
    unsafe {
        let p = ffi::MemAlloc(bytes as u32) as *mut T;
        std::ptr::copy_nonoverlapping(data.as_ptr(), p, data.len());
        p
    }
}

impl WorldMesh {
    /// `key` distinguishes lots so outline ids differ between neighbours.
    pub fn build(lot: &Lot, key: u64) -> Self {
        let mut pos: Vec<f32> = Vec::new();
        let mut nrm: Vec<f32> = Vec::new();
        let mut uv: Vec<f32> = Vec::new();
        let mut col: Vec<u8> = Vec::new();

        for (i, b) in lot.blocks.iter().enumerate() {
            // Object id for outline detection between touching blocks; 0 is reserved for sky.
            let id = 1.0 + (mix64(key ^ i as u64) % 255) as f32;
            let size = b.aabb.size();
            for (n, corners) in FACES {
                // Faces resting on the ground can never be seen.
                if n.y < 0.0 && b.aabb.min.y >= -0.01 && b.aabb.min.y <= 0.01 {
                    continue;
                }
                for k in [0, 1, 2, 0, 2, 3] {
                    let p = b.aabb.min + Vec3::from(corners[k]) * size;
                    pos.extend_from_slice(&p.to_array());
                    nrm.extend_from_slice(&n.to_array());
                    uv.extend_from_slice(&[id / 255.0, 0.0]);
                    // Alpha carries emission; architecture does not glow.
                    col.extend_from_slice(&[b.color.r, b.color.g, b.color.b, 0]);
                }
            }
        }

        for v in &lot.plants {
            pos.extend_from_slice(&v.pos.to_array());
            nrm.extend_from_slice(&v.normal.to_array());
            uv.extend_from_slice(&[v.id as f32 / 255.0, 0.0]);
            col.extend_from_slice(&[v.color.r, v.color.g, v.color.b, v.glow]);
        }

        let count = pos.len() / 3;
        let mut raw: ffi::Mesh = unsafe { std::mem::zeroed() };
        raw.vertexCount = count as i32;
        raw.triangleCount = (count / 3) as i32;
        raw.vertices = raylib_copy(&pos);
        raw.normals = raylib_copy(&nrm);
        raw.texcoords = raylib_copy(&uv);
        raw.colors = raylib_copy(&col);
        unsafe { ffi::UploadMesh(&mut raw, false) };
        Self { raw }
    }

    pub fn triangles(&self) -> i32 {
        self.raw.triangleCount
    }
}

impl Drop for WorldMesh {
    fn drop(&mut self) {
        unsafe { ffi::UnloadMesh(self.raw) };
    }
}
