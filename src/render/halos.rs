//! Glowing halos at the flowers.
//!
//! One quad per light slot, uploaded once. `shaders/halos.vs` reads each light's position and
//! color from the light texture, so the halo always sits exactly where the light does.

use sola_raylib::ffi;

use super::lights::MAX_LIGHTS;
use super::mesh::raylib_copy;
use super::shader::HotShader;

const CORNERS: [[f32; 2]; 6] = [
    [-1.0, -1.0],
    [1.0, -1.0],
    [1.0, 1.0],
    [-1.0, -1.0],
    [1.0, 1.0],
    [-1.0, 1.0],
];

pub struct Halos {
    pub shader: HotShader,
    pub material: ffi::Material,
    mesh: ffi::Mesh,
}

impl Halos {
    pub fn new(light_texture: ffi::Texture2D) -> Self {
        let shader = HotShader::new(
            Some(("halos.vs", include_str!("../../shaders/halos.vs"))),
            ("halos.fs", include_str!("../../shaders/halos.fs")),
        );
        let mut material = unsafe { ffi::LoadMaterialDefault() };
        material.shader = shader.raw;
        unsafe { (*material.maps.add(2)).texture = light_texture };

        let mut pos = Vec::new();
        let mut uv = Vec::new();
        for i in 0..MAX_LIGHTS {
            for c in CORNERS {
                pos.extend_from_slice(&[c[0], c[1], 0.0]);
                uv.extend_from_slice(&[i as f32, 0.0]);
            }
        }
        let count = MAX_LIGHTS * 6;
        let mut mesh: ffi::Mesh = unsafe { std::mem::zeroed() };
        mesh.vertexCount = count as i32;
        mesh.triangleCount = (count / 3) as i32;
        mesh.vertices = raylib_copy(&pos);
        mesh.texcoords = raylib_copy(&uv);
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

    /// Draws the halos with additive blending. Call inside `BeginMode3D`.
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

impl Drop for Halos {
    fn drop(&mut self) {
        unsafe { ffi::UnloadMesh(self.mesh) };
    }
}
