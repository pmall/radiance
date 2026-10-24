# Code map: where each rule lives

A lookup for the start of a session: which function decides what in the world generation, so a
bug in a screenshot leads straight to its code. `AGENTS.md` ("Layout") says what each file is;
this says where each rule is. Keep it current when a rule moves or a new one appears.

## How a lot is built

`world::generate` (on the worker threads of `src/stream.rs` while running) →
`city::generate_lot_dressed` (cached, `CACHED_LOTS`) → `generate_structure`, in this order:

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

## Things to know

- **Pure per lot:** every planner and deck answer is a function of seed and lot coordinates, so a
  lot can ask about its neighbors without generating them. Never make a lot's decision depend on
  another lot's generated blocks: that is what floated crane jibs (each lot decided on its own).
- **Caches:** the planner caches cell answers (`planner::cached`), `city` memoizes per-lot plan
  answers (`memo`: `laid`, `unpinched`, `deck`, `bridges_through`) and whole lots. All are keyed by
  `chunks::generation()`, so hot-reloaded chunks invalidate them. A new neighbor-heavy rule should
  go through `memo`, or the tests slow down (they went from 1 s to 14 s without it).
- **Reproducing a screenshot:** the `.txt` beside it holds the `--view ... --shot` command. Render
  it before and after a fix; `--bench 300` on the same view gives ms/frame (5–6 ms is usual).
