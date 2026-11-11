//! The constants the shaders repeat from the Rust code (GLSL cannot read them), checked here so a
//! change on one side without the other fails the tests instead of drawing a wrong picture.

use super::{FX_CEL, FX_FOG, FX_GRADING, FX_GRAIN, FX_OUTLINES, FX_TEXTURE, cover, lights, pixels};
use crate::world::biome::{MAX_BIOMES, PALETTE_VEC3S};
use crate::world::{city, floors};

const SCENE_FS: &str = include_str!("../../shaders/scene.fs");
const POST_FS: &str = include_str!("../../shaders/post.fs");
const HALOS_VS: &str = include_str!("../../shaders/halos.vs");
const COVER_GLSL: &str = include_str!("../../shaders/cover.glsl");

/// The value written after `decl = ` (e.g. `const int LW`) in a shader, up to the `;`.
fn value<'a>(file: &str, src: &'a str, decl: &str) -> &'a str {
    let start = format!("{decl} = ");
    src.lines()
        .find_map(|l| l.trim().strip_prefix(&start))
        .and_then(|rest| rest.split(';').next())
        .unwrap_or_else(|| panic!("{file} declares no `{decl}`"))
        .trim()
}

fn int(file: &str, src: &str, name: &str) -> usize {
    let v = value(file, src, &format!("const int {name}"));
    v.parse()
        .unwrap_or_else(|_| panic!("{file}: {name} = {v} is not a plain integer"))
}

fn float(file: &str, src: &str, name: &str) -> f32 {
    let v = value(file, src, &format!("const float {name}"));
    v.parse()
        .unwrap_or_else(|_| panic!("{file}: {name} = {v} is not a plain number"))
}

#[test]
fn the_light_grid_layout_matches_lights_rs() {
    for (file, src) in [("scene.fs", SCENE_FS), ("halos.vs", HALOS_VS)] {
        assert_eq!(int(file, src, "LW"), lights::WIDTH, "{file}: LW");
    }
    assert_eq!(int("scene.fs", SCENE_FS, "MAX_LIGHTS"), lights::MAX_LIGHTS);
    let [x, y, z] = lights::GRID;
    assert_eq!(
        value("scene.fs", SCENE_FS, "const ivec3 GRID"),
        format!("ivec3({x}, {y}, {z})")
    );
    assert_eq!(float("scene.fs", SCENE_FS, "CELL"), lights::CELL);
    assert_eq!(
        float("scene.fs", SCENE_FS, "WALL_REACH"),
        lights::WALL_REACH
    );
    // The shader reads a cell's light indices as two texels of four.
    assert_eq!(lights::SLOTS, 2 * 4);
}

#[test]
fn the_cover_map_layout_matches_cover_rs() {
    let f = "cover.glsl";
    assert_eq!(int(f, COVER_GLSL, "COVER"), cover::SIZE as usize);
    assert_eq!(float(f, COVER_GLSL, "LOT"), city::LOT);
    assert_eq!(int(f, COVER_GLSL, "FLOORS"), floors::MAX);
    assert_eq!(
        value(f, COVER_GLSL, "const int ROADS_TILE"),
        "1 + FLOORS / 4"
    );
    assert_eq!(cover::ROADS_TILE, 1 + floors::MAX / 4);
    assert_eq!(value(f, COVER_GLSL, "const int TILES"), "ROADS_TILE + 1");
    assert_eq!(cover::TILES as usize, cover::ROADS_TILE + 1);
}

#[test]
fn the_effect_bits_match_render_mod() {
    for (name, bit) in [("FX_CEL", FX_CEL), ("FX_TEXTURE", FX_TEXTURE)] {
        assert_eq!(
            int("scene.fs", SCENE_FS, name),
            bit as usize,
            "scene.fs: {name}"
        );
    }
    for (name, bit) in [
        ("FX_OUTLINES", FX_OUTLINES),
        ("FX_FOG", FX_FOG),
        ("FX_GRADING", FX_GRADING),
        ("FX_GRAIN", FX_GRAIN),
    ] {
        assert_eq!(
            int("post.fs", POST_FS, name),
            bit as usize,
            "post.fs: {name}"
        );
    }
}

#[test]
fn the_atlas_layout_matches_pixels_rs() {
    let f = "scene.fs";
    assert_eq!(float(f, SCENE_FS, "PPM"), pixels::PPM as f32);
    assert_eq!(value(f, SCENE_FS, "const int TILE_PX"), "int(4.0 * PPM)");
    assert_eq!(pixels::TILE, 4 * pixels::PPM);
    assert_eq!(value(f, SCENE_FS, "const float UNIT"), "PPM / 32.0");
    assert_eq!(pixels::U, pixels::PPM / 32);
    // A row of the atlas: materials, facade units, surfaces (in `SURFACES` order), swatches.
    assert_eq!(float(f, SCENE_FS, "FACADE_FIRST"), pixels::MATERIALS as f32);
    let first_surface = pixels::MATERIALS + pixels::FACADES;
    for (i, name) in ["ROAD_TILE", "SIDEWALK_TILE", "ROOF_TILE", "UNDERSIDE_TILE"]
        .into_iter()
        .enumerate()
    {
        assert_eq!(
            float(f, SCENE_FS, name),
            (first_surface + i) as f32,
            "{name}"
        );
    }
    assert_eq!(pixels::SURFACES, 4);
    assert_eq!(
        int(f, SCENE_FS, "SWATCH_TILE"),
        first_surface + pixels::SURFACES
    );
}

#[test]
fn the_palette_uniform_matches_biome_rs() {
    let (b, v) = (MAX_BIOMES, PALETTE_VEC3S);
    assert!(SCENE_FS.contains(&format!("uniform vec3 uPalette[{b} * {v}];")));
    assert!(SCENE_FS.contains(&format!("uPalette[gVariant * {v} + slot]")));
}
