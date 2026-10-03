//! Photographic surface detail: two packed RGBA textures (see `tools/pack_textures.py`) holding the
//! fine detail of CC0 photos, sampled by the scene shader with world-space coordinates.

use std::ffi::CString;

use sola_raylib::ffi;

const DETAIL_PNG: &[u8] = include_bytes!("../../assets/textures/detail.png");
const DETAIL2_PNG: &[u8] = include_bytes!("../../assets/textures/detail2.png");

/// Material slots (samplers `texture3`, `texture4`) of the two detail textures.
pub const SLOTS: [usize; 2] = [3, 4];

/// The detail textures, in slot order.
pub fn load() -> [ffi::Texture2D; 2] {
    [load_png(DETAIL_PNG), load_png(DETAIL2_PNG)]
}

fn load_png(png: &[u8]) -> ffi::Texture2D {
    unsafe {
        let kind = CString::new(".png").unwrap();
        let image = ffi::LoadImageFromMemory(kind.as_ptr(), png.as_ptr(), png.len() as i32);
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

/// Tells raylib where the shader's detail samplers are, so DrawMesh binds the textures to them.
/// Must be repeated whenever the scene shader is reloaded (the location table is rebuilt).
pub fn bind_samplers(material: &mut ffi::Material) {
    use ffi::ShaderLocationIndex::{SHADER_LOC_MAP_OCCLUSION, SHADER_LOC_MAP_ROUGHNESS};
    for (name, index) in [
        ("texture3", SHADER_LOC_MAP_ROUGHNESS),
        ("texture4", SHADER_LOC_MAP_OCCLUSION),
    ] {
        unsafe {
            let name = CString::new(name).unwrap();
            let loc = ffi::GetShaderLocation(material.shader, name.as_ptr());
            *material.shader.locs.add(index as usize) = loc;
        }
    }
}
