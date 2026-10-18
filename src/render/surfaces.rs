//! The scene shader's extra samplers: where raylib binds the material's maps (cover map, tower
//! elevations, chunk pixel art) in the shader. The photographic detail that once lived here is
//! gone: every surface is pixel art (milestone 10).

use std::ffi::CString;

use sola_raylib::ffi;

/// Tells raylib where the scene shader's extra samplers are (cover map, tower elevations, chunk
/// pixel art), so DrawMesh binds the textures to them. Must be repeated whenever the scene shader is reloaded
/// (the location table is rebuilt).
pub fn bind_samplers(material: &mut ffi::Material) {
    use ffi::ShaderLocationIndex::{
        SHADER_LOC_MAP_BRDF, SHADER_LOC_MAP_EMISSION, SHADER_LOC_MAP_HEIGHT,
    };
    for (name, index) in [
        ("texture5", SHADER_LOC_MAP_EMISSION),
        ("texture6", SHADER_LOC_MAP_HEIGHT),
        ("texture10", SHADER_LOC_MAP_BRDF),
    ] {
        unsafe {
            let name = CString::new(name).unwrap();
            let loc = ffi::GetShaderLocation(material.shader, name.as_ptr());
            *material.shader.locs.add(index as usize) = loc;
        }
    }
}
