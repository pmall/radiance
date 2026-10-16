//! Glowing halos at the flowers.
//!
//! One quad per light slot, uploaded once. `shaders/halos.vs` reads each light's position and
//! color from the light texture, so the halo always sits exactly where the light does.

use sola_raylib::ffi;

use super::lights::MAX_LIGHTS;
use super::mesh::MeshData;
use super::overlay::{Overlay, QUAD};

pub fn new(light_texture: ffi::Texture2D) -> Overlay {
    let mut m = MeshData::default();
    for i in 0..MAX_LIGHTS {
        for c in QUAD {
            m.pos.extend_from_slice(&[c[0], c[1], 0.0]);
            m.uv.extend_from_slice(&[i as f32, 0.0]);
        }
    }
    let halos = Overlay::new(
        ("halos.vs", include_str!("../../shaders/halos.vs")),
        ("halos.fs", include_str!("../../shaders/halos.fs")),
        m.upload(),
    );
    unsafe { (*halos.material.maps.add(2)).texture = light_texture };
    halos
}
