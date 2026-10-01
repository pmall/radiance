//! Offscreen scene targets: HDR color, normal + object id, and a sampleable depth texture.

use sola_raylib::ffi;

pub struct SceneTargets {
    pub fbo: u32,
    pub color: ffi::Texture2D,
    pub normal: ffi::Texture2D,
    pub depth: ffi::Texture2D,
    pub width: i32,
    pub height: i32,
}

fn texture(id: u32, width: i32, height: i32, format: i32) -> ffi::Texture2D {
    ffi::Texture2D {
        id,
        width,
        height,
        mipmaps: 1,
        format,
    }
}

pub(super) fn set_params(id: u32, filter: u32) {
    unsafe {
        ffi::rlTextureParameters(id, ffi::RL_TEXTURE_MIN_FILTER as i32, filter as i32);
        ffi::rlTextureParameters(id, ffi::RL_TEXTURE_MAG_FILTER as i32, filter as i32);
        ffi::rlTextureParameters(
            id,
            ffi::RL_TEXTURE_WRAP_S as i32,
            ffi::RL_TEXTURE_WRAP_CLAMP as i32,
        );
        ffi::rlTextureParameters(
            id,
            ffi::RL_TEXTURE_WRAP_T as i32,
            ffi::RL_TEXTURE_WRAP_CLAMP as i32,
        );
    }
}

impl SceneTargets {
    pub fn new(width: i32, height: i32) -> Self {
        use ffi::rlFramebufferAttachTextureType::RL_ATTACHMENT_TEXTURE2D as TEX2D;
        use ffi::rlFramebufferAttachType::*;
        use ffi::rlPixelFormat::*;
        unsafe {
            let fbo = ffi::rlLoadFramebuffer();
            ffi::rlEnableFramebuffer(fbo);

            let hdr = RL_PIXELFORMAT_UNCOMPRESSED_R16G16B16A16 as i32;
            let rgba8 = RL_PIXELFORMAT_UNCOMPRESSED_R8G8B8A8 as i32;
            let color_id = ffi::rlLoadTexture(std::ptr::null(), width, height, hdr, 1);
            let normal_id = ffi::rlLoadTexture(std::ptr::null(), width, height, rgba8, 1);
            let depth_id = ffi::rlLoadTextureDepth(width, height, false);
            set_params(color_id, ffi::RL_TEXTURE_FILTER_LINEAR);
            set_params(normal_id, ffi::RL_TEXTURE_FILTER_NEAREST);
            set_params(depth_id, ffi::RL_TEXTURE_FILTER_NEAREST);

            ffi::rlFramebufferAttach(
                fbo,
                color_id,
                RL_ATTACHMENT_COLOR_CHANNEL0 as i32,
                TEX2D as i32,
                0,
            );
            ffi::rlFramebufferAttach(
                fbo,
                normal_id,
                RL_ATTACHMENT_COLOR_CHANNEL1 as i32,
                TEX2D as i32,
                0,
            );
            ffi::rlFramebufferAttach(fbo, depth_id, RL_ATTACHMENT_DEPTH as i32, TEX2D as i32, 0);
            // rlFramebufferAttach unbinds the framebuffer; draw buffers are per-FBO state.
            ffi::rlEnableFramebuffer(fbo);
            ffi::rlActiveDrawBuffers(2);
            assert!(
                ffi::rlFramebufferComplete(fbo),
                "scene framebuffer incomplete"
            );
            ffi::rlDisableFramebuffer();

            Self {
                fbo,
                color: texture(color_id, width, height, hdr),
                normal: texture(normal_id, width, height, rgba8),
                depth: texture(depth_id, width, height, 0),
                width,
                height,
            }
        }
    }

    /// Raylib view of the framebuffer, for `BeginTextureMode`.
    pub fn render_texture(&self) -> ffi::RenderTexture2D {
        ffi::RenderTexture2D {
            id: self.fbo,
            texture: self.color,
            depth: self.depth,
        }
    }
}

impl Drop for SceneTargets {
    fn drop(&mut self) {
        unsafe {
            ffi::rlUnloadTexture(self.color.id);
            ffi::rlUnloadTexture(self.normal.id);
            // Also deletes the attached depth texture.
            ffi::rlUnloadFramebuffer(self.fbo);
        }
    }
}
