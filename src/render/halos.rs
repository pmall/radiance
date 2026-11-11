//! Glowing halos at the flowers.
//!
//! One quad per light slot, uploaded once. `shaders/halos.vs` reads each light's position and
//! color from the light texture, so the halo always sits exactly where the light does.
//!
//! Every `unsafe` block here is a raylib call that needs the window open and must run on its
//! thread, where the GL context is current: the renderer is made after the window opens and
//! never leaves the main thread.

use sola_raylib::ffi;

use super::lights::MAX_LIGHTS;
use super::mesh::MeshData;
use super::overlay::{Overlay, QUAD};
use super::shader::shader_file;

pub fn new(light_texture: ffi::Texture2D) -> Overlay {
    let mut m = MeshData::default();
    for i in 0..MAX_LIGHTS {
        for c in QUAD {
            m.pos.extend_from_slice(&[c[0], c[1], 0.0]);
            m.uv.extend_from_slice(&[i as f32, 0.0]);
        }
    }
    let halos = Overlay::new(
        shader_file!("halos.vs"),
        shader_file!("halos.fs"),
        m.upload(),
    );
    // SAFETY: slot 2 is below MAX_MATERIAL_MAPS (12), the maps LoadMaterialDefault allocates.
    unsafe { (*halos.material.maps.add(2)).texture = light_texture };
    halos
}
