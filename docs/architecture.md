# Architecture

How the parts fit together and where each starts. What a module does in detail is in its doc
comment (`//!` at the top of each file); this page only orients and points. The design rules the
code implements are in `docs/world.md` (generation), `docs/chunks.md` (hand-written chunks) and
`docs/art-direction.md` (the look).

## The parts

- **Main loop** (`src/main.rs`): window, command line, a fixed 120 Hz simulation step, input
  routing, debug keys (`docs/tools.md`). `src/debug.rs` draws the overlay, `src/report.rs` prints
  `--report`, `src/recorder.rs` records a run through `ffmpeg`.
- **Player** (`src/player.rs`): the first-person controller (walk, run, jump, step-up, ledge grab,
  free-fly), colliding against the world's index (`World::colliders_near`).
- **Randomness and time** (`src/rng.rs`, `src/daycycle.rs`): the seeded RNG and coordinate hashing
  every generated thing goes through; the time-of-day clock and sun direction.
- **World** (`src/world/`): the city as data, generated lot by lot.
  - `floors.rs`: the stack of floors and its zones (depths, middle, canopy).
  - `design.rs`: the floor profiles, every generation number that depends on the floor, and the
    props / points of interest switches.
  - `biome.rs`: the map's zones and everything a biome sets (`docs/world.md`, "Biomes").
  - `chunks.rs`: the chunk and module vocabulary and the catalogs; `chunk_files.rs`: where and how
    their files are stored in `assets/`, read and hot-reloaded; `planner.rs`: which chunk stands
    where (`docs/chunks.md`).
  - `city.rs`: builds a lot from the plan (ground, decks, stairs, towers, bridges, chunks);
    `tower.rs`: tower chunks; `roads.rs`: the road network; `leftovers.rs`: debris and cables;
    `growth.rs`: plants and flowers; `props.rs`: the props placed instanced; `skyline.rs`: the
    far towers beyond the loaded lots.
  - `finder.rs`: the nearest stairs and crater from the player's floor (the test mode overlay).
- **Streaming** (`src/stream.rs`): lots within `world::LOAD_RADIUS` of the player are generated
  and meshed on worker threads and uploaded a little per frame; lots past `UNLOAD_RADIUS` are
  dropped.
- **Render** (`src/render/`, pipeline in `render/mod.rs`): a shadow pass, a scene pass into HDR,
  normal/id and depth targets (`shaders/scene.*`), a post pass for sky, outlines, fog, grading and
  grain (`shaders/post.fs`), then additive overlays (particles, halos). Around it:
  - `lights.rs` bins the flower lights into a camera grid texture, `halos.rs` draws their halos;
  - `cover.rs` keeps the cover map (which decks stand over each lot, how much sky it sees, its
    roads), which light, fog and pavement read;
  - `pixels.rs` draws each biome's pixel-art atlas, `noise.rs` the noise the wear reads,
    `facades.rs` the towers' written elevations as a texture;
  - `props.rs` draws props instanced, `skyline.rs` meshes the far skyline off the main thread;
  - `look.rs` holds the time-of-day palettes and the blend of biome atmospheres.
- **Shaders** (`shaders/`): GLSL 330, hot-reloaded from disk and embedded as a fallback
  (`render/shader.rs`, `shader_file!`); `#include "file"` pulls in a shared file (`INCLUDES`),
  such as `cover.glsl` or `fade.glsl` (far draws dissolve in a dither instead of cutting). The
  constants the shaders repeat from the Rust code are checked by tests in `render/mirrors.rs`.
- **Audio** (`src/audio/`): `piano.rs` the souls' notes, `ambient.rs` the synthesized hum,
  `reverb.rs` on the master bus. Samples in `assets/audio/`.
- **Assets** (`assets/`): chunk and module files per biome (`assets/biomes/<biome>/`), shared
  modules (`assets/modules/`), piano samples. Plain text and samples only; textures are drawn by
  code.

## How a lot is built

`world::generate` (on the worker threads of `src/stream.rs`) → `city::generate_lot_dressed`
(cached, `CACHED_LOTS`) → `generate_structure`, in this order:

1. Ground, then deck slabs per floor (`deck`; `push_ring_deck` around a stairwell), and what
   hangs under a full slab (`push_underside`).
2. Movement course near the spawn (`push_movement_course`).
3. The lot's stair (`stair`, `push_switchback_stair`).
4. Its tower (`push_tower` → `tower::build`); wall and roof pieces piercing a deck give way.
5. Skybridges to the +x / +z towers (`skybridges` → `push_bridge`), footbridges
   (`push_footbridges`).
6. The chunks covering the lot (`push_chunks`), greater ranks first (`Placement::rank`).
7. Things left behind (`leftovers::cables`, `leftovers::scatter`).

Plants come after, from the finished blocks (`growth::grow_lot`).

## Rule → function

| Rule                                                    | Where                                                                           |
| ------------------------------------------------------- | ------------------------------------------------------------------------------- |
| Does a lot have a deck on a floor                       | `city::deck` (final) ← `unpinched` ← `laid` (the plan)                          |
| Decks never touch by a corner alone                     | `city::pinched`, `opened_corners` (two passes)                                  |
| Which lot borders are roads (network, dead ends cut)    | `roads::network` (per tile, cached `roads::roads`), `roads::ground`             |
| Where a chunk paves its floor over (no road there)      | `city::paved`                                                                   |
| Street lights at crossings, curb furniture along roads  | `roads::crossing`, `roads::along`; `If` in `chunks::streets_fit`                |
| Roads, curbs and markings painted                       | `pavement` (`shaders/scene.fs`) from `roads::bits` (`render/cover.rs`)          |
| Holes and craters opening floors                        | `planner::hole_at`, `holes_at`, `crater_placement`; `city::opened`              |
| Which chunk stands where                                | `planner::covering` (areas, subs, blocks, holes, craters, hubs, fillers,        |
|                                                         | streets); `city::placements` (adds `--chunk` preview)                           |
| Precedence between chunks on a lot                      | `Placement::rank`, `Placement::outranks`                                        |
| Whether a chunk stands on a lot (deck, outranked)       | `city::stands_on`                                                               |
| What gives way (a module and its boxes, together)       | `Pieces::blocked` (one lot), `Pieces::spanning_blocked` (multi-lot              |
|                                                         | modules, decided once per placement), `city::room`                              |
| Which lot builds a piece reaching out of its chunk      | `Footprint::owner`                                                              |
| Towers: where, how tall                                 | `planner::tower_at`, `towers_of_cell`, `tower_stories`                          |
| Towers: tiers, setbacks, body, walls a bridge can use   | `tower::tiers`, `tower::body`, `tower::solid_top`, `city::tower`                |
| Towers: units near decks, street parts                  | `tower::build`, `push_units`                                                    |
| Towers leaning on a taller neighbor (front only)        | `tower::lean` (which neighbor, how far), `tower::tiers` (`LEAN_ABOVE`),         |
|                                                         | `tower::build` (steps), `tower::leans` (`--report`)                             |
| Stairs (one per hub)                                    | `planner::stair_hub`, `hub_of_cell`; `city::stair`                              |
| Skybridges between towers, decided once per pair        | `city::skybridges`, `push_bridge` (module `skybridge_segment`),                 |
|                                                         | `skybridges_through`                                                            |
| Footbridges over gaps of 1–2 lots, landings kept clear  | `city::footbridge_gap`, `landing`, `footbridge_room`, `push_footbridges`        |
|                                                         | (module `footbridge_span`)                                                      |
| Bridges never cross (one footbridge per lot, skybridges | `city::bridges_through` (ranks crossing gaps), `footbridge_gap` (`under_sky`)   |
| clear of footbridges)                                   |                                                                                 |
| Per-floor numbers (holes, stairs, fillers, props, POIs) | `design::profile`, `FloorProfile`                                               |
| Low-rise districts (few towers, places, row houses)     | `planner::low_rise` (`design::LOW_RISE`), read in `towers_of_cell`, `filler_at` |
| Places need room: no tower through their floor          | `planner::block_of_cell_uncached` (`fits`)                                      |
| Floors, heights, zones                                  | `floors::y`, `floors::zone`, `floors::count`                                    |
| Per-biome numbers (bridge chance, plant rates)          | `biome::biome_at`, `Biome`                                                      |
| Per-biome plant species (shapes, foliage, flowers)      | `biome::Plants`, `pick_species`; read in `growth::Builder` (`bush`,             |
|                                                         | `hanging_vine`, `climber`, `flower`)                                            |
| Per-biome props (a biome's module restyles a prop)      | `props::Meshes::load`, `props::biome_of`                                        |
| Per-biome textures (pixel art of materials and units)   | `biome::Biome::textures` (`pixels::Textures`), `pixels::atlas_pixels`;          |
|                                                         | `atlasTile` (`shaders/scene.fs`) reads the lot's or prop's biome row            |
| Per-biome tower wall paints                             | `biome::Palette::paints`; `tower::paint`, `facade` (`shaders/scene.fs`)         |
| Per-biome sky, fog, light, pollen, spores               | `biome::Atmosphere`; `look::Blend` (fades at a border), `Sky::from_cycle`       |
| Chunk text format, modules, slots, `If`                 | `chunks::Part`, `chunks::expand_in`, `Env`                                      |
| Props drawn instanced                                   | `props::by_module`, `props::Placed`                                             |
| Plants and flowers                                      | `growth::grow_lot`                                                              |
| What hangs under a deck (girders, pipes, dead lamps)    | `city::push_underside` (`BlockKind::Hung`: drawn as built, grows nothing)       |
| Debris at the wall feet, never in a road                | `leftovers::scatter` (`wall_feet`, `in_road`; `design::FloorProfile::debris`)   |
| Cables across the streets between towers                | `leftovers::cables`, `push_cable` (kept out of skybridges)                      |
| Weeds at the wall feet and behind the curbs             | `growth::grow_lot` (end), `Builder::weed`                                       |
| Collisions with what reaches in from the next lot       | `World::colliders_near`, `overhangs` (`world/mod.rs`)                           |
| Far skyline: front towers out to 1 km, past loaded lots | `skyline::blocks` (`RADIUS`), `tower::silhouette`; drawn by `render::skyline`,  |
|                                                         | left out over loaded lots (`uLoaded` in `shaders/scene.fs`)                     |
| How far over the top deck a front tower rises           | `planner::tower_stories` (a chunk's `rise`, e.g. `tower_spire`)                 |
| Far draws fading instead of cutting                     | `Fade` (`render/mod.rs`, `shaders/fade.glsl`), `EDGE_FOG` (`uEdgeFog`,          |
|                                                         | `post.fs`), light fade in `lights::update`                                      |

## Invariants

- **Deterministic:** same seed, same city. Every random decision goes through `src/rng.rs`, keyed
  by the seed and coordinates, never by time or global state.
- **Pure per lot:** every planner and deck answer is a function of seed and lot coordinates, so a
  lot can ask about its neighbors without generating them, and lots generate in any order on any
  thread. A lot's decision never depends on another lot's generated blocks (that is what once
  left crane jibs floating: each lot decided on its own).
- **Big chunks belong to the lot holding their center:** each lot builds only its slice; a module
  spanning several lots stands or gives way as a whole, decided the same way in every lot from
  the plan alone.
- **Caches:** the planner caches cell answers (`planner::cached`), `city` memoizes per-lot plan
  answers (`memo`: `laid`, `unpinched`, `deck`, `bridges_through`) and whole lots. All are keyed
  by `chunk_files::generation()`, so hot-reloaded chunks invalidate them. Neighbor-heavy rules go
  through `memo`, or the test suite slows down sharply.
