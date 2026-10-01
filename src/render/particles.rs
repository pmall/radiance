//! Drifting pollen motes and glowing spores around the camera.
//!
//! A fixed set of quads is uploaded once; `shaders/particles.vs` places, drifts and billboards
//! them from per-particle random numbers, wrapping inside a box around the camera.

use sola_raylib::ffi;

use super::shader::HotShader;
use crate::rng::Rng;

const COUNT: usize = 7000;
const CORNERS: [[f32; 2]; 6] = [
    [-1.0, -1.0],
    [1.0, -1.0],
    [1.0, 1.0],
    [-1.0, -1.0],
    [1.0, 1.0],
    [-1.0, 1.0],
];

pub struct Particles {
    pub shader: HotShader,
    pub material: ffi::Material,
    mesh: ffi::Mesh,
}

fn raylib_copy<T: Copy>(data: &[T]) -> *mut T {
    let bytes = std::mem::size_of_val(data);
    unsafe {
        let p = ffi::MemAlloc(bytes as u32) as *mut T;
        std::ptr::copy_nonoverlapping(data.as_ptr(), p, data.len());
        p
    }
}

impl Particles {
    pub fn new() -> Self {
        let shader = HotShader::new(
            Some(("particles.vs", include_str!("../../shaders/particles.vs"))),
            ("particles.fs", include_str!("../../shaders/particles.fs")),
        );
        let mut material = unsafe { ffi::LoadMaterialDefault() };
        material.shader = shader.raw;

        // The particle field is the same for every seed: it is atmosphere, not world.
        let mut rng = Rng::new(0x005E_ED0F_D057);
        let mut pos = Vec::new();
        let mut uv = Vec::new();
        let mut nrm = Vec::new();
        let mut col = Vec::new();
        for _ in 0..COUNT {
            let (x, y, z) = (rng.f32(), rng.f32(), rng.f32());
            let (size, phase) = (rng.f32(), rng.f32());
            let bytes = [
                rng.range_i(0, 256) as u8,
                rng.range_i(0, 256) as u8,
                rng.range_i(0, 256) as u8,
                255,
            ];
            for c in CORNERS {
                pos.extend_from_slice(&[c[0], c[1], 0.0]);
                uv.extend_from_slice(&[x, y]);
                nrm.extend_from_slice(&[z, size, phase]);
                col.extend_from_slice(&bytes);
            }
        }
        let count = COUNT * 6;
        let mut mesh: ffi::Mesh = unsafe { std::mem::zeroed() };
        mesh.vertexCount = count as i32;
        mesh.triangleCount = (count / 3) as i32;
        mesh.vertices = raylib_copy(&pos);
        mesh.texcoords = raylib_copy(&uv);
        mesh.normals = raylib_copy(&nrm);
        mesh.colors = raylib_copy(&col);
        unsafe { ffi::UploadMesh(&mut mesh, false) };
        Self {
            shader,
            material,
            mesh,
        }
    }

    pub fn poll(&mut self) {
        self.shader.poll();
        self.material.shader = self.shader.raw;
    }

    pub fn reload(&mut self) {
        self.shader.force_reload();
        self.material.shader = self.shader.raw;
    }

    /// Draws the field with additive blending. Call inside `BeginMode3D`.
    pub fn draw(&self) {
        unsafe {
            ffi::rlDisableDepthTest();
            ffi::rlDisableBackfaceCulling();
            ffi::BeginBlendMode(ffi::BlendMode::BLEND_ADDITIVE as i32);
            ffi::DrawMesh(self.mesh, self.material, super::IDENTITY);
            ffi::EndBlendMode();
            ffi::rlEnableBackfaceCulling();
            ffi::rlEnableDepthTest();
        }
    }
}

impl Drop for Particles {
    fn drop(&mut self) {
        unsafe { ffi::UnloadMesh(self.mesh) };
    }
}
