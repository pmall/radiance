# Code map: where each rule lives

A lookup for the start of a session: which function decides what in the world generation, so a
bug in a screenshot leads straight to its code. `AGENTS.md` ("Layout") says what each file is;
this says where each rule is. Keep it current when a rule moves or a new one appears.

## How a lot is built

`city::generate_lot_dressed` (cached, `CACHED_LOTS`) → `generate_structure`, in this order:

1. Ground, then deck slabs per floor (`deck`; `push_ring_deck` around a stairwell).
2. Movement course near the spawn (`push_movement_course`).
3. The lot's stair (`stair`, `push_switchback_stair`).
4. Its tower (`push_tower` → `tower::build`); wall and roof pieces piercing a deck give way.
5. Skybridges to the +x / +z towers (`skybridges` → `push_bridge`), footbridges
   (`push_footbridges`).
6. The chunks covering the lot (`push_chunks`), greater ranks first (`Placement::rank`).

Plants come after, from the finished blocks (`growth::grow_lot`).

## Rule → function

| Rule                                                    | Where                                                                     |
| ------------------------------------------------------- | ------------------------------------------------------------------------- |
| Does a lot have a deck on a floor                       | `city::deck` (final) ← `unpinched` ← `laid` (the plan)                    |
| Decks never touch by a corner alone                     | `city::pinched`, `opened_corners` (two passes)                            |
| Holes and craters opening floors                        | `planner::hole_at`, `holes_at`, `crater_placement`; `city::opened`        |
| Which chunk stands where                                | `planner::covering` (areas, subs, blocks, holes, craters, hubs, fillers,  |
|                                                         | streets); `city::placements` (adds `--chunk` preview)                     |
| Precedence between chunks on a lot                      | `Placement::rank`, `Placement::outranks`                                  |
| Whether a chunk stands on a lot (deck, outranked)       | `city::stands_on`                                                         |
| What gives way (a module and its boxes, together)       | `Pieces::blocked` (one lot), `Pieces::spanning_blocked` (multi-lot        |
|                                                         | modules, decided once per placement), `city::room`                        |
| Which lot builds a piece reaching out of its chunk      | `Footprint::owner`                                                        |
| Towers: where, how tall                                 | `planner::tower_at`, `towers_of_cell`, `tower_stories`                    |
| Towers: tiers, setbacks, body, walls a bridge can use   | `tower::tiers`, `tower::body`, `tower::solid_top`, `city::tower`          |
| Towers: units near decks, street parts                  | `tower::build`, `push_units`                                              |
| Stairs (one per hub)                                    | `planner::stair_hub`, `hub_of_cell`; `city::stair`                        |
| Skybridges between towers, decided once per pair        | `city::skybridges`, `push_bridge` (module `skybridge_segment`),           |
|                                                         | `skybridges_through`                                                      |
| Footbridges over gaps of 1–2 lots, landings kept clear  | `city::bridges_through`, `landing`, `footbridge_room`, `push_footbridges` |
|                                                         | (module `footbridge_span`)                                                |
| Per-floor numbers (holes, stairs, fillers, props, POIs) | `design::profile`, `FloorProfile`                                         |
| Floors, heights, zones                                  | `floors::y`, `floors::zone`, `floors::count`                              |
| Per-biome numbers (bridge chance, plants)               | `biome::biome_at`, `Biome`                                                |
| Chunk text format, modules, slots, `If`                 | `chunks::Part`, `chunks::expand_in`, `Env`                                |
| Props drawn instanced                                   | `props::by_module`, `props::Placed`                                       |
| Plants and flowers                                      | `growth::grow_lot`                                                        |
| Collisions with what reaches in from the next lot       | `World::colliders_near`, `overhangs` (`world/mod.rs`)                     |

## Things to know

- **Pure per lot:** every planner and deck answer is a function of seed and lot coordinates, so a
  lot can ask about its neighbors without generating them. Never make a lot's decision depend on
  another lot's generated blocks: that is what floated crane jibs (each lot decided on its own).
- **Caches:** the planner caches cell answers (`planner::cached`), `city` memoizes per-lot plan
  answers (`memo`: `laid`, `unpinched`, `deck`, `bridges_through`) and whole lots. All are keyed by
  `chunks::generation()`, so hot-reloaded chunks invalidate them. A new neighbor-heavy rule should
  go through `memo`, or the tests slow down (they went from 1 s to 14 s without it).
- **Reproducing a screenshot:** the `.txt` beside it holds the `--view ... --shot` command. Render
  it before and after a fix; `--bench 300` on the same view gives ms/frame (7–8 ms is usual).
