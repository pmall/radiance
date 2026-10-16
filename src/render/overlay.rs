//! Layers drawn additively over the finished frame and hidden by the scene's depth: a fixed set of
//! quads uploaded once, which the layer's vertex shader places and billboards every frame (pollen
//! and spores, `particles.rs`; halos at the flowers, `halos.rs`).

use glam::{Vec2, Vec3};
use sola_raylib::ffi;

use super::shader::HotShader;
use super::{FAR, IDENTITY, NEAR, View};

/// The corners of one quad, as two triangles.
pub const QUAD: [[f32; 2]; 6] = [
    [-1.0, -1.0],
    [1.0, -1.0],
    [1.0, 1.0],
    [-1.0, -1.0],
    [1.0, 1.0],
    [-1.0, 1.0],
];

pub struct Overlay {
    pub shader: HotShader,
    pub material: ffi::Material,
    mesh: ffi::Mesh,
}

impl Overlay {
    /// `vs` / `fs` are (file name, embedded source), as `HotShader` takes them.
    pub fn new(
        vs: (&'static str, &'static str),
        fs: (&'static str, &'static str),
        mesh: ffi::Mesh,
    ) -> Self {
        let shader = HotShader::new(Some(vs), fs);
        let mut material = unsafe { ffi::LoadMaterialDefault() };
        material.shader = shader.raw;
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

    /// Draws the layer over the frame, hidden where the scene's `depth` is nearer. Sets the camera
    /// uniforms (with the camera's `right` and `up`) and the plants' `glow`; the layer's own
    /// uniforms are set before.
    pub fn draw(
        &mut self,
        view: &View,
        (right, up): (Vec3, Vec3),
        camera: &ffi::Camera3D,
        screen: (i32, i32),
        depth: ffi::Texture2D,
        glow: f32,
    ) {
        let s = &mut self.shader;
        s.set_vec3("uCamPos", view.pos);
        s.set_vec3("uCamFwd", view.dir);
        s.set_vec3("uCamRight", right);
        s.set_vec3("uCamUp", up);
        s.set_vec2("uScreen", Vec2::new(screen.0 as f32, screen.1 as f32));
        s.set_f32("uNear", NEAR);
        s.set_f32("uFar", FAR);
        s.set_f32("uGlow", glow);
        unsafe {
            (*self.material.maps.add(1)).texture = depth;
            ffi::rlSetClipPlanes(NEAR as f64, FAR as f64);
            ffi::BeginMode3D(*camera);
            ffi::rlDisableDepthTest();
            ffi::rlDisableBackfaceCulling();
            ffi::BeginBlendMode(ffi::BlendMode::BLEND_ADDITIVE as i32);
            ffi::DrawMesh(self.mesh, self.material, IDENTITY);
            ffi::EndBlendMode();
            ffi::rlEnableBackfaceCulling();
            ffi::rlEnableDepthTest();
            ffi::EndMode3D();
        }
    }
}

impl Drop for Overlay {
    fn drop(&mut self) {
        unsafe { ffi::UnloadMesh(self.mesh) };
    }
}
