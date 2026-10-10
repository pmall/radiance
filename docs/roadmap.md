# Roadmap

## Milestones

1. **Setup:** project skeleton, window, first-person camera, basic movement on a test scene.
   *(done)*
2. **Rendering pipeline:** cel shading, outlines, colored fog, color grading on simple geometry.
   *(done: also cast shadows, banded sky, paper grain, time-of-day looks)*
3. **City skeleton:** procedural chunk of architecture with vertical layers.
   *(first pass done: 24 m lots with towers, deck slabs at y = 0 / -36 / -80, bridges; spiral stairs in empty lots link the layers.
   Replaced in milestone 9 by a stack of 9 floors.)*
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
     softer, with real foliage shapes. *(done: rosettes of arching folded fronds and narrow-blade
     tufts in `growth.rs`, denser than before)*
   - Light sources: only flowers emit light, one light per flower at the flower, with a visible halo,
     matching tint and a smaller pool (see `docs/art-direction.md`). *(done: foliage never glows;
     each flower has petals, a bright core and one light of its color; halos are billboards read from
     the light texture, `src/render/halos.rs`; pool cut to 512 lights)*
   - Textures *(done, first full pass)*: a procedural layer per block kind in `shaders/scene.fs`
     (tower facades with five window styles, storefronts, neon, accent panels; asphalt streets with
     curbs and markings; roofs, billboards, solar arrays, lamps; painted leaves with midribs and veins)
     plus a photo-detail layer from CC0 concrete, asphalt, wall and metal textures
     (`assets/textures/`, `tools/pack_textures.py`). Rooftops (parapets, machinery, masts, billboards)
     and street lamps are solid blocks. Reference: Mirror's Edge (see `docs/feedback.md`).
     Also fire escapes and blade signs on tower walls (`src/world/facade.rs`), photo detail on roofs,
     foliage and rust (`detail2.png`), and per-biome architecture palettes (`Palette` in `biome.rs`).
   - Lore pass *(done)*: unfinished tower tiers with tower cranes (the machines never stopped
     building), vegetation and flowers thickest in the canopy and thinnest in the depths (humans
     moved up as the city rose). See `docs/lore.md`.
   - Biomes *(groundwork done)*: see `docs/world.md` and `docs/biome-ideas.md`.
   - Candidates from the *Last Radiance* brainstorm *(not started)*: sunlight craters, light-network
     veins, older architecture deep and machine intrusion high, set dressing, flowers in human
     places, melodic-gradient guidance. See "What the prototype can take now" in
     `docs/game-ideas.md`.

9. **The floor stack, props and points of interest** (owner's direction after the *Last Radiance*
   brainstorm: the ambiance and visuals must carry the future story):
   1. Floor stack *(done)*: 9 floors D0–D8, start on D2 facing a crater of daylight, decks block
      the sun and sky through a cover map, fog beams, stairs rarer higher up, construction front
      past the top deck. See "The floor stack" in `docs/world.md`. About 6 ms/frame and 3.7 M
      tris from the spawn on the 3060 (was 3 ms, 1.4 M tris).
      Known issues: small bright chips flicker in the far fog of the crater; deck slabs read as
      square tiles from above; the depths band is uniform (no older architecture yet).
   2. Props *(next)*: free model kits drawn in our style (rules in `docs/art-direction.md`), set
      dressing over D0–D2 (`docs/world.md`).
   3. Points of interest: silent tableaux from `docs/poi-catalog.md` placed by the seed near the
      start (the notice board courtyard, the armchair with a flower, the bench with two flowers...).

## Debug and art-iteration tools

Important, since the purpose is art exploration. Build them alongside the features they serve.

- Change or randomize the seed and regenerate. *(done: R, [ ])*
- Control time of day: pause, speed up, and scrub to any time. *(done: T, PgUp/PgDn, Home/End, 1-4)*
- Toggle individual effects (outlines, fog, cel shading, grain, particles). *(done: F3-F8, F11 for particles, X for textures, F9 render scale)*
- Free-fly camera. *(done: F2)*
- Test mode *(done: G, on by default)*: the overlay points to the nearest stair up, stair down and
  crater within about 1 km (distance, direction in words; "far, not loaded yet" beyond the
  loaded area, where things exist but are not drawn yet); right mouse button = 4x speed. The overlay (F1) shows the
  floor large at the top, in plain words; `--overlay` keeps it in a `--shot`.
- Live-tweakable shader and palette parameters, if practical (e.g. shader hot-reload from disk).
  *(shader hot-reload done, F10 forces it; palettes still in `src/render/look.rs`)*
- Headless frame capture for checking looks: `--time`, `--view`, `--shot`, `--fx` (effect mask). *(done)*
- Screenshots for feedback *(done: P saves to `screenshots/` with a note holding the command that
  renders the same view)*.
- Record a run to MP4 with the screen and the sound. *(done: V; 1600x900 60 fps H.264 (NVENC when available) with AAC audio, saved in the background to `recordings/`; the overlay stays out of the video)*
- Frame time benchmark: `--bench N` renders N frames without vsync and prints ms/frame and triangles.
  *(done; about 3 ms/frame, ~1.4 M tris loaded, on the 3060 from the plaza)*
