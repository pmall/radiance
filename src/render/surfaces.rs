//! Photographic surface detail: one packed RGBA texture (see `tools/pack_textures.py`) holding the
//! fine detail of CC0 concrete, asphalt, wall and metal photos, sampled by the scene shader with
//! world-space coordinates. Bound to material slot 3, sampler `texture3`.

use std::ffi::CString;

use sola_raylib::ffi;

const DETAIL_PNG: &[u8] = include_bytes!("../../assets/textures/detail.png");

/// Material slot and sampler of the detail texture.
pub const SLOT: usize = 3;

pub fn load() -> ffi::Texture2D {
    unsafe {
        let kind = CString::new(".png").unwrap();
        let image =
            ffi::LoadImageFromMemory(kind.as_ptr(), DETAIL_PNG.as_ptr(), DETAIL_PNG.len() as i32);
        let mut texture = ffi::LoadTextureFromImage(image);
        ffi::UnloadImage(image);
        ffi::GenTextureMipmaps(&mut texture);
        ffi::SetTextureWrap(texture, ffi::TextureWrap::TEXTURE_WRAP_REPEAT as i32);
        ffi::SetTextureFilter(
            texture,
            ffi::TextureFilter::TEXTURE_FILTER_ANISOTROPIC_8X as i32,
        );
        texture
    }
}

/// Tells raylib where the shader's detail sampler is, so DrawMesh binds the texture to it. Must be
/// repeated whenever the scene shader is reloaded (the location table is rebuilt).
pub fn bind_sampler(material: &mut ffi::Material) {
    unsafe {
        let name = CString::new("texture3").unwrap();
        let loc = ffi::GetShaderLocation(material.shader, name.as_ptr());
        let idx = ffi::ShaderLocationIndex::SHADER_LOC_MAP_ROUGHNESS as usize;
        *material.shader.locs.add(idx) = loc;
    }
}
