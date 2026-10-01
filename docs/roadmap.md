# Roadmap

## Milestones

1. **Setup:** project skeleton, window, first-person camera, basic movement on a test scene.
   *(done)*
2. **Rendering pipeline:** cel shading, outlines, colored fog, color grading on simple geometry.
   *(done: also cast shadows, banded sky, paper grain, time-of-day looks)*
3. **City skeleton:** procedural chunk of architecture with vertical layers.
   *(first pass done: 24 m lots with towers, deck slabs at y = 0 / -36 / -80, bridges; spiral stairs in empty lots link the layers)*
4. **Infinite streaming:** seed-deterministic chunk generation and unloading around the player.
   *(done: lots load within 7 lots of the player, unload past 9, 4 per frame; one mesh per lot)*
5. **Overgrowth:** procedural plant growth over the architecture.
   *(first pass done: bushes, hanging vines and branching wall climbers from `src/world/growth.rs`,
   denser toward the depths; no collision, no glow yet. ~3.8 M tris loaded, 3.7 ms/frame on the 3060)*
6. **Light and sound:** light plants, spatial piano notes, ambient soundscape.
   *(light plants done: ~1 in 8 plants in the canopy and ~1 in 3 in the depths are luminous (cyan, green,
   magenta, violet), emit and cast banded point lights; up to 1024 nearest lights binned in a camera
   grid (`src/render/lights.rs`), 3.8 ms/frame on the 3060. Glow follows `plant_glow` in the looks,
   always strong in the depths. Sound pending)*
7. **Atmosphere and polish:** particles, day/night cycle, palette tuning, movement feel.
   *(done: GPU-driven pollen motes and glowing spores (`src/render/particles.rs`), full day/night cycle,
   dusk brightened, 4096 shadow map, landing dip. Not done: audio, which was skipped on request)*

8. **Visual quality pass (feedback after the first full playthrough):**
   - Plants: bushes read as boulders (faceted blobs); replace with leaf cards or fronds, smaller and
     softer, with real foliage shapes.
   - Light sources: only flowers emit light, one light per flower at the flower, with a visible halo,
     matching tint and a smaller pool (see `docs/art-direction.md`).
   - Textures: the image is flat vertex color plus grain. Add real surface texture (hatching and
     brush strokes in shadow bands, concrete and metal detail, painted leaves with veins, a stronger
     paper or canvas feel). Texturing is compatible with procedural generation: the seed still drives placement, textures drive surface look. Authored/external or procedural, whichever looks best.

## Debug and art-iteration tools

Important, since the purpose is art exploration. Build them alongside the features they serve.

- Change or randomize the seed and regenerate. *(done: R, [ ])*
- Control time of day: pause, speed up, and scrub to any time. *(done: T, PgUp/PgDn, Home/End, 1-4)*
- Toggle individual effects (outlines, fog, cel shading, grain, particles). *(done: F3-F8 and F11 for particles, F9 render scale)*
- Free-fly camera. *(done: F2)*
- Live-tweakable shader and palette parameters, if practical (e.g. shader hot-reload from disk).
  *(shader hot-reload done, F10 forces it; palettes still in `src/render/look.rs`)*
- Headless frame capture for checking looks: `--time`, `--view`, `--shot`. *(done)*
