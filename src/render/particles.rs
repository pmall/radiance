//! Drifting pollen motes and glowing spores around the camera.
//!
//! A fixed set of quads is uploaded once; `shaders/particles.vs` places, drifts and billboards
//! them from per-particle random numbers, wrapping inside a box around the camera.

use super::mesh::MeshData;
use super::overlay::{Overlay, QUAD};
use crate::rng::Rng;

const COUNT: usize = 7000;

pub fn new() -> Overlay {
    // The particle field is the same for every seed: it is atmosphere, not world.
    let mut rng = Rng::new(0x005E_ED0F_D057);
    let mut m = MeshData::default();
    for _ in 0..COUNT {
        let (x, y, z) = (rng.f32(), rng.f32(), rng.f32());
        let (size, phase) = (rng.f32(), rng.f32());
        let bytes = [
            rng.range_i(0, 256) as u8,
            rng.range_i(0, 256) as u8,
            rng.range_i(0, 256) as u8,
            // A few are drips (`drip` in shaders/particles.vs).
            rng.range_i(0, 256) as u8,
        ];
        for c in QUAD {
            m.pos.extend_from_slice(&[c[0], c[1], 0.0]);
            m.uv.extend_from_slice(&[x, y]);
            m.nrm.extend_from_slice(&[z, size, phase]);
            m.col.extend_from_slice(&bytes);
        }
    }
    Overlay::new(
        ("particles.vs", include_str!("../../shaders/particles.vs")),
        ("particles.fs", include_str!("../../shaders/particles.fs")),
        m.upload(),
    )
}
