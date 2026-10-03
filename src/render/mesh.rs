//! Builds one static GPU mesh from the world's blocks.

use glam::{Vec3, vec3};
use sola_raylib::ffi;

use crate::rng::mix64;
use crate::world::{BlockKind, Lot};

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
        // Second UV set: position along a vertical face and that face's width (0 elsewhere).
        let mut uv2: Vec<f32> = Vec::new();
        let mut col: Vec<u8> = Vec::new();

        for (i, b) in lot.blocks.iter().enumerate() {
            // Object id for outline detection between touching blocks; 0 is reserved for sky.
            let id = 1.0 + (mix64(key ^ i as u64) % 255) as f32;
            let size = b.aabb.size();
            // Shader material code: see `kind` in shaders/scene.fs (1 is plants).
            let kind = match b.kind {
                BlockKind::Tower => 0.0,
                BlockKind::Floor | BlockKind::Deck => 2.0,
                BlockKind::Bridge => 3.0,
                BlockKind::Stair => 4.0,
                BlockKind::Course => 5.0,
                BlockKind::Parapet => 6.0,
                BlockKind::Equipment => 7.0,
                BlockKind::Sign => 8.0,
                BlockKind::Mast => 9.0,
                BlockKind::Solar => 10.0,
                BlockKind::Skylight => 11.0,
                BlockKind::Lamp => 12.0,
            };
            for (n, corners) in FACES {
                // Faces resting on the ground can never be seen.
                if n.y < 0.0 && b.aabb.min.y >= -0.01 && b.aabb.min.y <= 0.01 {
                    continue;
                }
                for k in [0, 1, 2, 0, 2, 3] {
                    let p = b.aabb.min + Vec3::from(corners[k]) * size;
                    pos.extend_from_slice(&p.to_array());
                    nrm.extend_from_slice(&n.to_array());
                    uv.extend_from_slice(&[id / 255.0, kind]);
                    // Second UV set. Vertical faces: position along the face and its width in
                    // meters (billboards: both axes normalized). Horizontal faces: normalized.
                    let along = if n.x != 0.0 { 2 } else { 0 };
                    let c = corners[k];
                    let loc = if n.y != 0.0 {
                        [c[0], c[2]]
                    } else if b.kind == BlockKind::Sign {
                        // The thin edges of a panel are marked with -1: no art there.
                        [if size[along] < 1.0 { -1.0 } else { c[along] }, c[1]]
                    } else {
                        [c[along] * size[along], size[along]]
                    };
                    uv2.extend_from_slice(&loc);
                    // Alpha carries emission; architecture does not glow.
                    col.extend_from_slice(&[b.color.r, b.color.g, b.color.b, 0]);
                }
            }
        }

        for v in &lot.plants {
            pos.extend_from_slice(&v.pos.to_array());
            nrm.extend_from_slice(&v.normal.to_array());
            // uv.y = 1 marks plant geometry (architecture is 0), read by the scene shader.
            uv.extend_from_slice(&[v.id as f32 / 255.0, 1.0]);
            uv2.extend_from_slice(&[0.0, 0.0]);
            col.extend_from_slice(&[v.color.r, v.color.g, v.color.b, v.glow]);
        }

        let count = pos.len() / 3;
        let mut raw: ffi::Mesh = unsafe { std::mem::zeroed() };
        raw.vertexCount = count as i32;
        raw.triangleCount = (count / 3) as i32;
        raw.vertices = raylib_copy(&pos);
        raw.normals = raylib_copy(&nrm);
        raw.texcoords = raylib_copy(&uv);
        raw.texcoords2 = raylib_copy(&uv2);
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
