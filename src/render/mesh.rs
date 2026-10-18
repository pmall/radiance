//! Static GPU meshes: one per lot from the world's blocks (`WorldMesh`), and the upload every
//! mesh of the renderer goes through (`MeshData`).

use glam::Vec3;
use sola_raylib::ffi;

use crate::rng::mix64;
use crate::world::{BOX_FACES, BlockKind, Lot};

/// Elevation ids the kind code can carry exactly (a float holds integers up to 2^24).
const MAX_ELEVATION: u16 = 8191;

pub struct WorldMesh {
    pub raw: ffi::Mesh,
}

/// Vertex arrays in raylib's layout; an empty array leaves that attribute out.
#[derive(Default)]
pub struct MeshData {
    pub pos: Vec<f32>,
    pub nrm: Vec<f32>,
    pub uv: Vec<f32>,
    pub uv2: Vec<f32>,
    pub col: Vec<u8>,
}

impl MeshData {
    /// Uploads the arrays as a mesh owned by raylib (freed by `UnloadMesh`).
    pub fn upload(&self) -> ffi::Mesh {
        let count = self.pos.len() / 3;
        let mut raw: ffi::Mesh = unsafe { std::mem::zeroed() };
        raw.vertexCount = count as i32;
        raw.triangleCount = (count / 3) as i32;
        raw.vertices = raylib_copy(&self.pos);
        raw.normals = raylib_copy(&self.nrm);
        raw.texcoords = raylib_copy(&self.uv);
        raw.texcoords2 = raylib_copy(&self.uv2);
        raw.colors = raylib_copy(&self.col);
        unsafe { ffi::UploadMesh(&mut raw, false) };
        raw
    }
}

/// Copies a slice into memory owned by raylib (freed by `UnloadMesh`); null if it is empty.
fn raylib_copy<T: Copy>(data: &[T]) -> *mut T {
    if data.is_empty() {
        return std::ptr::null_mut();
    }
    let bytes = std::mem::size_of_val(data);
    unsafe {
        let p = ffi::MemAlloc(bytes as u32) as *mut T;
        std::ptr::copy_nonoverlapping(data.as_ptr(), p, data.len());
        p
    }
}

/// Largest extent of a box that counts as detail (an air conditioner, a balcony, a railing post):
/// those go to the lot's detail mesh, drawn only near the camera.
const DETAIL_SIZE: f32 = 3.5;

/// Whether a block is small detail rather than architecture.
fn is_detail(kind: BlockKind, size: Vec3) -> bool {
    matches!(
        kind,
        BlockKind::Built(_) | BlockKind::Hung(_) | BlockKind::Steel
    ) && size.max_element() <= DETAIL_SIZE
}

impl WorldMesh {
    /// A lot's architecture and plants (`detail` false), or its small boxes (`detail` true); `None`
    /// if there is nothing. `key` distinguishes lots so outline ids differ between neighbours.
    pub fn build(lot: &Lot, key: u64, detail: bool) -> Option<Self> {
        let mut pos: Vec<f32> = Vec::new();
        let mut nrm: Vec<f32> = Vec::new();
        let mut uv: Vec<f32> = Vec::new();
        // Second UV set: position along a vertical face and that face's width (0 elsewhere).
        let mut uv2: Vec<f32> = Vec::new();
        let mut col: Vec<u8> = Vec::new();

        // Material variant of the lot's biome rides in the kind code (16 per variant).
        let variant = lot.variant as f32 * 16.0;
        for (i, b) in lot.blocks.iter().enumerate() {
            // Prop boxes are only collision; the props draw their own mesh below.
            if matches!(b.kind, BlockKind::Prop | BlockKind::Collider)
                || is_detail(b.kind, b.aabb.size()) != detail
            {
                continue;
            }
            // Object id for outline detection between touching blocks; 0 is reserved for sky.
            let id = 1.0 + (mix64(key ^ i as u64) % 255) as f32;
            let size = b.aabb.size();
            // Shader material code: see `kind` in shaders/scene.fs (1 is plants).
            let kind = variant
                + match b.kind {
                    BlockKind::Tower(_) => 0.0,
                    BlockKind::Floor | BlockKind::Deck => 2.0,
                    BlockKind::Stair => 4.0,
                    BlockKind::Course => 5.0,
                    BlockKind::Parapet => 6.0,
                    BlockKind::Steel => 13.0,
                    BlockKind::Built(_) | BlockKind::Hung(_) => 15.0,
                    BlockKind::Prop | BlockKind::Collider => unreachable!(),
                };
            for (n, corners) in BOX_FACES {
                // Faces resting on the ground can never be seen.
                if n.y < 0.0 && b.aabb.min.y >= -0.01 && b.aabb.min.y <= 0.01 {
                    continue;
                }
                // A tower wall carries its elevation and the tower's paint shift (`facade` in
                // shaders/scene.fs), above the variant in the kind code.
                let kind = match b.kind {
                    BlockKind::Tower(f) if n.y == 0.0 => {
                        let face = if n.x > 0.0 {
                            0
                        } else if n.x < 0.0 {
                            1
                        } else if n.z > 0.0 {
                            2
                        } else {
                            3
                        };
                        let id = f.ids[face].min(MAX_ELEVATION) as f32;
                        kind + id * 2048.0 + (f.paint % 8) as f32 * 256.0
                    }
                    _ => kind,
                };
                for k in [0, 1, 2, 0, 2, 3] {
                    let p = b.aabb.min + Vec3::from(corners[k]) * size;
                    pos.extend_from_slice(&p.to_array());
                    nrm.extend_from_slice(&n.to_array());
                    uv.extend_from_slice(&[id / 255.0, kind]);
                    // Second UV set. Vertical faces: position along the face and its width in
                    // meters (billboards: both axes normalized). Horizontal faces: normalized.
                    let along = if n.x != 0.0 { 2 } else { 0 };
                    let c = corners[k];
                    let loc = if let BlockKind::Built(m) | BlockKind::Hung(m) = b.kind {
                        // Built boxes: their material, painted in world space, and their
                        // height, which bounds the grime at their foot (`wallWear`).
                        [m.index() as f32, size.y]
                    } else if n.y != 0.0 {
                        [c[0], c[2]]
                    } else {
                        [c[along] * size[along], size[along]]
                    };
                    uv2.extend_from_slice(&loc);
                    // Alpha carries emission; architecture does not glow.
                    col.extend_from_slice(&[b.color.r, b.color.g, b.color.b, 0]);
                }
            }
        }

        for v in lot.plants().filter(|_| !detail) {
            pos.extend_from_slice(&v.pos.to_array());
            nrm.extend_from_slice(&v.normal.to_array());
            // uv.y = 1 marks plant geometry (architecture is 0), read by the scene shader.
            uv.extend_from_slice(&[v.id as f32 / 255.0, variant + 1.0]);
            uv2.extend_from_slice(&v.uv);
            col.extend_from_slice(&[v.color.r, v.color.g, v.color.b, v.glow]);
        }

        if pos.is_empty() {
            return None;
        }
        let raw = MeshData {
            pos,
            nrm,
            uv,
            uv2,
            col,
        }
        .upload();
        Some(Self { raw })
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
