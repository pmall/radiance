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
     and street lights are props (`src/world/props.rs`). Reference: Mirror's Edge (see `docs/feedback.md`).
     Also fire escapes and blade signs on tower walls (`src/world/facade.rs`), photo detail on roofs,
     foliage and rust (`detail2.png`), and per-biome architecture palettes (`Palette` in `biome.rs`).
   - Lore pass *(done)*: unfinished tower tiers with tower cranes (the machines never stopped
     building), vegetation and flowers thickest in the canopy and thinnest in the depths (humans
     moved up as the city rose). See `docs/lore.md`.
   - Biomes *(groundwork done: a biome sets its places, props, plants and sky; second biome: Cyan
     Grotto)*: see `docs/world.md` ("Biomes") and `docs/biome-ideas.md`.
   - Per-biome textures *(done, owner)*: each biome its own pixel art for the materials
     (concrete, brick, tile, metal, wood, glass...) and the facade units (windows, balconies,
     shops), not only its own colors: `render/pixels.rs` draws a row of the atlas per biome from
     its `Textures` (shades, patterns, wear), and the tower wall paints are the biome's. The Cyan
     Grotto's: an old bathhouse district gone cold and wet (`docs/world.md`, "Biomes").
   - Biomes through the floors *(possible future development, owner; not started)*: each biome
     stays a column but can look different in the depths, the middle and the canopy, the default
     city included (`docs/world.md`, "Biomes", "Not yet").
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
      Known issues: small bright chips flicker in the far fog of the crater; the depths band is uniform (no older architecture yet).
   2. Props *(fourth pass done)*: Kenney kits (toy-like), then Poly Haven photoscans (removed),
      replaced by box-built modules in the chunks' pixel-art materials
      (`assets/modules/props.ron`), plus a street light built in code, one per crossroads.
      Placed by the street layout (curb, corner, facade, parking, vacant lot), a third of the
      first density. No cars yet.
      2c. Consistency pass *(done)*: decks laid by sections built up to a floor (holes line up as
      light wells, rare low down), street lights on a regular layout, stairs grow plants like any
      surface (no special landing flowers).
      2b. Organizational pass *(done)*: one design module with floor profiles (`world/design.rs`),
      variable floor count, scenes declared by slot, kind and zone weights, points of interest
      drawn apart at a controlled rate, O/I switches, `--report` to measure each floor.
      8.3 ms/frame at 9 floors, 11.5 ms at 14.
   3. Points of interest *(first pass done)*: the armchair with a flower and the bench with two
      flowers on the start plaza; armchairs, benches and dead gardens with flowers elsewhere. Still
      to do: the catalog's other objects (notice board courtyard, vending machine, bus stop blocked
      by a column, phone booth...), which need models not in these kits.
      The darkness of the depths stays (owner's call).

10. **Beautiful, consistent, abandoned** (owner, after the biomes: "bland", "empty more than
    abandoned", "always habitation, I never spot a square or some special architecture"; before
    any new biome or feature). The diagnosis: nothing was left behind and nothing has aged; three
    styles in one image (pixel-art chunks, smooth photo-grained streets, smooth plants, blurred
    particles); one olive gray from the ground to the ceiling; isolated apartment towers in a sea
    of asphalt; special places so rare one is met every 2 to 3 km of a floor; biomes that differ
    only in the sky. About eight passes, each judged on the reference views
    (`tools/lookdev.py`, `docs/art-direction.md`):
    1. Reference views, wear and light *(done, to be judged by the owner)*: the reference views;
       roads, sidewalks, curbs, roofs and deck undersides in each biome's pixel art (the Grotto's
       roads cobbled, no markings); wear over every surface (dust drifts, cracks with weeds, moss,
       dead leaves gathered in the gutters, glints of glass, puddles that mirror the sky and the
       flowers; grime and moss at the foot of walls, streaks under every ledge, soot under the
       deck above), road markings worn nearly away; a color per height under the decks (cold
       depths, warm middle, gold canopy) and a haze brighter than the walls in it, so they stand
       as silhouettes; the flowers' colors in the night haze; the eye adapting under the decks;
       particles and halos as pixel specks; no outline across two lots of one deck. Frame time
       unchanged (the wear reads a noise texture).
    2. Light, continued *(done)*: sunbeams through holes and craters (the haze lit where the sun
       reaches air that sees little sky, strongest toward the sun; moonbeams at night); the deck
       undersides as a structure (girders on the lot borders, joists, pipe runs lining up along
       a row of lots, dead lamps on rods: `city::push_underside`, `BlockKind::Hung`) and water
       dripping from them (particles, only under a deck); walls facing a flower catch its light
       further off (1.6 times the pool). About 0.6 ms/frame more (8.7 ms at the spawn).
    3. Things left behind *(done)*: debris scattered by rule at the foot of every wall (more
       under the tower facades: lumps, a rubble heap, bricks, a fallen frame or panel; bags,
       boxes, papers, a tyre against low walls), never in a road (`world/leftovers.rs`, the
       floor profiles' `debris`); weeds in the cracks at the wall feet and behind the curbs;
       cables sagging across the streets between towers facing each other, stepped like pixels.
       Frame time unchanged (about 8.6 ms at the spawn).
    4. One look, finished *(done)*: stairs, the wall between their lanes, steel (rails,
       brackets), the street light and the deck edges are drawn from the biome's pixel-art
       tiles (concrete, paint) like the chunk boxes, worn the same way; the photo detail is left
       on the plants only, until pass 5.
    5. Pixel-art plants *(first pass done)*: every leaf, frond and blade is a sprite of square
       cells (20 per meter) in its leaf coordinates, which carry its size (`growth::set_leaf`,
       `pixelLeaf` in `shaders/scene.fs`): flat tones dithered root to tip, a lighter midrib,
       a darker rim, a ragged pixel edge cut out of the outer cells; fronds nearly flat
       (`FOLD`); stems in world-space cells. The photo detail layer is gone (`assets/textures`,
       `tools/pack_textures.py`). Frame time unchanged. Second pass *(done)*: the flowers' petals
       as sprites cut to a stepped kite, pale by the core, a darker rim (`pixelPetal`).
    6. The planner *(done)*: low-rise districts (6x6 lots, 40 % of them, `planner::low_rise`,
       `design::LOW_RISE`) where towers are rare, so the city alternates tower clusters and
       built-up streets; special places only where no tower rises through their floor, on every
       floor including D0 (blocks at 70 % per 8x8 cell low down, a new place each try): a
       fountain square, a shrine court, a playground, a bus terminal, a monument square, an
       open-air theater, rooftop greenhouses (about one every 280 m on D2); `street_front`, row
       houses along every street side of a lot in a pinwheel around a courtyard, backs or an
       alley on the quiet sides (`If(quiet: ..)`); fillers more common (75 % low down, 90 % in
       low-rise districts). Modules in `assets/modules/places.ron`.
    7. Landmarks *(first pass done)*: new areas `old_quarter` (the old stone city of the depths:
       a temple, two avenues with tram lines and poles, arcaded stone buildings, narrow houses
       around small squares), `station` (a train shed of stepped arches over platforms and
       tracks, the head building and its clock tower, a forecourt, a freight yard), `stadium`
       (the middle floors: stands on four sides, roofs, floodlight masts, a scoreboard), `mall`
       (galleries on three levels around a glass-roofed atrium, stopped escalators) and
       `machine_works` (the canopy: gantries printing facades, arms, conveyors); the block
       `parking_garage` (four stories, stepped ramps); the abandoned tower `tower_gutted`
       (broken and boarded windows, a burnt band, rubble). Modules in
       `assets/modules/landmarks.ron`. Areas stay one per 32x32-lot cell, so a landmark is met
       every kilometer or so. Still to do (second pass): towers that are not housing beyond the
       office tower, a tower with a gaping hole or leaning on its neighbor (needs geometry the
       tower bodies do not have), tall silhouettes above the construction front. D5 views run
       about 11 ms/frame (7.5 M tris), as before this pass.
    8. The Grotto *(done)*: its own tiled tree beds, lanterns, kiosks, fountains and drinking
       fountains (`grotto/modules/places.ron`); its landmark, the `bathhouse` (3x3 block): a
       tiled hall with arched doors under a stepped dome open at its crown, a pool ringed by
       columns, a terrace of basins and lanterns.

    Milestone 10 status: all eight passes have a first version, judged good by the owner on the
    reference views (`python3 tools/lookdev.py`, label `grotto8`). The points left after them are
    done too (label `m10done`), to be judged by the owner; only the points to judge over time
    remain. The new towers change the tower picks, so the city differs from `grotto8` in places
    (not a look change).

    - ~~Bug, bridges cross each other~~ *(fixed)*: owner's screenshot, seed 7, D2 at dusk, two
      footbridges met over lot (-6, -1). A lot over a hole now carries one footbridge at most
      (where an x and a z gap cross, the seed ranks them and the first one wins), and a footbridge
      gives way to a skybridge passing through or less than 6 m over its walkway
      (`city::bridges_through`, test `bridges_never_cross`).
    - ~~Second landmark pass (pass 7)~~ *(done)*: towers that are not housing *(done: `tower_glass`,
      `tower_hotel`, `tower_telecom`, with new facade units: curtain walls, broken curtain walls,
      ribbon windows)*; a tower with a gaping hole *(done: `tower_breached`)* or leaning on its
      neighbor *(done: `tower_leaning`, about one every 400 m at the construction front)*; tall
      silhouettes above the construction front *(done: `tower_spire`, 180 to 300 m over the top
      deck, and the far skyline: the front's towers drawn out to about a kilometer, past the
      loaded lots, `world/skyline.rs`; about 0.35 ms/frame)*.
    - ~~Maybe a second plant pass (pass 5): the flowers' petals in pixel art~~ *(done)*.
    - ~~Frame time~~ *(done)*: was 9.5 ms at the spawn and about 11 ms on D5. Plants are 95 % of
      the triangles (6 M of 6.3 M at the spawn) and every lot was drawn whole in the scene and the
      shadow pass. Now each lot's plants are a mesh of their own, lot meshes are culled to the
      camera's view and the shadow map's box, and plants cast shadows within 70 m only
      (`PLANT_SHADOW_REACH`): 5.6 ms at the spawn, 6.3 ms on the D5 stadium view, 3.2 ms on the
      top deck. What is left: shadows about 1.3 ms, the haze and its sunbeams about 1.1 ms.
    - Points to judge over time: how strong the wall wash from flowers is (`WALL_REACH`), the
      ragged leaf edges, and how the low-rise districts change the feel of the city.
    - The reference views moved in pass 6 (street-d3 moved, laundry-d1 renamed court-d1, three
      new views), so labels before `plan6` do not compare one to one on those.

## Debug and art-iteration tools

Important, since the purpose is art exploration. Build them alongside the features they serve.

- Change or randomize the seed and regenerate. *(done: R, [ ])*
- Control time of day: pause, speed up, and scrub to any time. *(done: T, PgUp/PgDn, Home/End, 1-4)*
- Toggle individual effects (outlines, fog, cel shading, grain, particles). *(done: F3-F8, F11 for particles, X for textures, F9 render scale)*
- Free-fly camera. *(done: F2)*
- Test mode *(done: G, on by default)*: the overlay points to the nearest stair up, stair down and
  crater within about 1 km (distance, clock direction, 12 o'clock straight ahead; "far, not loaded yet" beyond the
  loaded area, where things exist but are not drawn yet); right mouse button = 4x speed. The overlay (F1) shows the
  floor large at the top, in plain words; `--overlay` keeps it in a `--shot`.
- Live-tweakable shader and palette parameters, if practical (e.g. shader hot-reload from disk).
  *(shader hot-reload done, F10 forces it; palettes still in `src/render/look.rs`)*
- Headless frame capture for checking looks: `--time`, `--view`, `--shot`, `--fx` (effect mask). *(done)*
- Reference views *(done)*: `python3 tools/lookdev.py LABEL` renders eleven fixed views (spawn,
  streets on D0 and D3, a court, the great square, a shrine, the old town, a stadium, a park, the canopy, the Grotto) at day,
  dusk and night into `screenshots/lookdev/LABEL/`, with one sheet per time of day; every look
  change is judged on them, before against after. `--report` lists the places (areas, blocks,
  craters) with a `--view` to stand in each.
- Screenshots for feedback *(done: P saves to `screenshots/` with a note holding the command that
  renders the same view)*.
- Record a run to MP4 with the screen and the sound. *(done: V; 1600x900 60 fps H.264 (NVENC when available) with AAC audio, saved in the background to `recordings/`; the overlay stays out of the video)*
- Frame time benchmark: `--bench N` renders N frames without vsync and prints ms/frame and the
  triangles drawn in view. *(done; about 5.6 ms/frame at the spawn on the 3060)*
