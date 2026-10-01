//! Directional shadow map from the key light, following the camera.

use glam::camera::rh::proj::opengl;
use glam::camera::rh::view::look_at_mat4;
use glam::{Mat4, Vec3, vec3};
use sola_raylib::ffi;

use super::shader::HotShader;
use super::{IDENTITY, v3};

const SIZE: i32 = 4096;
/// Half the side of the square area covered, in meters.
const HALF_EXTENT: f32 = 110.0;
/// Distance from the covered area's center back to the light camera.
const BACK: f32 = 600.0;
const DEPTH_RANGE: f32 = 1200.0;

pub struct ShadowMap {
    fbo: u32,
    color_id: u32,
    pub depth: ffi::Texture2D,
    pub shader: HotShader,
    material: ffi::Material,
}

impl ShadowMap {
    pub fn new() -> Self {
        use ffi::rlFramebufferAttachTextureType::RL_ATTACHMENT_TEXTURE2D as TEX2D;
        use ffi::rlFramebufferAttachType::*;
        let shader = HotShader::new(
            Some(("shadow.vs", include_str!("../../shaders/shadow.vs"))),
            ("shadow.fs", include_str!("../../shaders/shadow.fs")),
        );
        let mut material = unsafe { ffi::LoadMaterialDefault() };
        material.shader = shader.raw;
        unsafe {
            let fbo = ffi::rlLoadFramebuffer();
            ffi::rlEnableFramebuffer(fbo);
            // A small color target keeps the framebuffer complete on strict GL 3.3 drivers.
            let gray = ffi::rlPixelFormat::RL_PIXELFORMAT_UNCOMPRESSED_GRAYSCALE as i32;
            let color_id = ffi::rlLoadTexture(std::ptr::null(), SIZE, SIZE, gray, 1);
            let depth_id = ffi::rlLoadTextureDepth(SIZE, SIZE, false);
            for id in [color_id, depth_id] {
                super::targets::set_params(id, ffi::RL_TEXTURE_FILTER_NEAREST);
            }
            ffi::rlFramebufferAttach(
                fbo,
                color_id,
                RL_ATTACHMENT_COLOR_CHANNEL0 as i32,
                TEX2D as i32,
                0,
            );
            ffi::rlFramebufferAttach(fbo, depth_id, RL_ATTACHMENT_DEPTH as i32, TEX2D as i32, 0);
            assert!(
                ffi::rlFramebufferComplete(fbo),
                "shadow framebuffer incomplete"
            );
            ffi::rlDisableFramebuffer();
            Self {
                fbo,
                color_id,
                depth: ffi::Texture2D {
                    id: depth_id,
                    width: SIZE,
                    height: SIZE,
                    mipmaps: 1,
                    format: 0,
                },
                shader,
                material,
            }
        }
    }

    pub fn texel(&self) -> f32 {
        1.0 / SIZE as f32
    }

    pub fn poll(&mut self) {
        self.shader.poll();
        self.material.shader = self.shader.raw;
    }

    /// Renders the mesh into the shadow map, covering an area around `center`.
    /// Returns the light view-projection matrix for sampling.
    pub fn render<'a>(
        &mut self,
        meshes: impl Iterator<Item = &'a ffi::Mesh>,
        center: Vec3,
        light_dir: Vec3,
    ) -> Mat4 {
        let up = if light_dir.y.abs() > 0.99 {
            Vec3::Z
        } else {
            Vec3::Y
        };
        // Snap the center to whole texels in light space so shadow edges don't shimmer.
        let texel = 2.0 * HALF_EXTENT / SIZE as f32;
        let view0 = look_at_mat4(light_dir * BACK, Vec3::ZERO, up);
        let p = view0.transform_point3(center);
        let p = vec3(
            (p.x / texel).round() * texel,
            (p.y / texel).round() * texel,
            p.z,
        );
        let center = view0.inverse().transform_point3(p);

        let eye = center + light_dir * BACK;
        let view = look_at_mat4(eye, center, up);
        let proj = opengl::orthographic(
            -HALF_EXTENT,
            HALF_EXTENT,
            -HALF_EXTENT,
            HALF_EXTENT,
            1.0,
            DEPTH_RANGE,
        );
        let camera = ffi::Camera3D {
            position: v3(eye),
            target: v3(center),
            up: v3(up),
            fovy: 2.0 * HALF_EXTENT,
            projection: ffi::CameraProjection::CAMERA_ORTHOGRAPHIC as i32,
        };
        unsafe {
            ffi::rlSetClipPlanes(1.0, DEPTH_RANGE as f64);
            ffi::BeginTextureMode(ffi::RenderTexture2D {
                id: self.fbo,
                texture: ffi::Texture2D {
                    id: self.color_id,
                    width: SIZE,
                    height: SIZE,
                    mipmaps: 1,
                    format: 0,
                },
                depth: self.depth,
            });
            ffi::rlClearColor(255, 255, 255, 255);
            ffi::rlClearScreenBuffers();
            ffi::BeginMode3D(camera);
            // Back faces only: closed boxes then never shadow their own lit faces.
            ffi::rlSetCullFace(ffi::rlCullMode::RL_CULL_FACE_FRONT as i32);
            for mesh in meshes {
                ffi::DrawMesh(*mesh, self.material, IDENTITY);
            }
            ffi::rlSetCullFace(ffi::rlCullMode::RL_CULL_FACE_BACK as i32);
            ffi::EndMode3D();
            ffi::EndTextureMode();
        }
        proj * view
    }
}

impl Drop for ShadowMap {
    fn drop(&mut self) {
        unsafe {
            ffi::rlUnloadTexture(self.color_id);
            ffi::rlUnloadFramebuffer(self.fbo);
        }
    }
}
