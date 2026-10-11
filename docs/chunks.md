# City chunks: design

Status: **proposal**, on the `chunks` branch. Nothing here is built yet. Once agreed, it replaces
the lot-by-lot generation described in `docs/world.md` step by step (see "Phases").

## Why

Random placement can't make a place look organized: a square needs symmetry, alignment and a
center, and a stair belongs in the middle of its stairwell, not as five stairs popping up next to
each other. The owner's direction: build the city from **large pieces of map written by hand (as
text)**, placed by the seed with **rules**, and keep **vegetation and flowers fully random** on
top. The darkness stays unpredictable because its only light sources, the flowers, are always
random; the layout gains the order and density a real city has.

Everything stays procedural in the sense the project requires (`AGENTS.md`): the seed decides
which piece goes where, which variant, which mirror, and every plant and flower. Pieces only
decide how a place is organized.

## Style target: an abandoned Nivalis

Reference: *Nivalis* (2025). Screenshots are kept by the owner outside git; the key one shows a
Kowloon-like wall of apartments: the "perfect result" for the buildings. What makes that look:

- **Everything is built from boxes** (a voxel style) with **pixel-art textures**: buildings,
  balconies, signs and props all share one visual language. Our engine is built on boxes already,
  so this suits it better than photoscans did.
- **Repeated modules, varied:** a facade is a grid of apartment units (window, cage, air
  conditioner, laundry, balcony, awning), each with its own color, following consistent rules.
- **Density and layers:** small details everywhere, bridges between towers, cables, signs, haze
  giving depth, warm light against teal shadows.

Ours is that city **abandoned**: the neon is dead, the windows dark, the signs still readable as
pixel art but unlit, and the glowing flowers are the only light. Concretely:

- **Grain:** architecture snapped to a 0.25 m grid, small details and props to 0.125 m.
- **Textures:** pixel art at about 16 pixels per meter, nearest filtering (crisp pixels), per
  material (concrete, brick, tile, rusted metal, painted panel, wood, glass, sign art). Generated
  by scripts from code (`tools/`), so the project stays plain text; the seed picks the colors.
- **Light:** sun and sky by day, flowers by night, both as today (cover map, fog, flower lights).
- **To decide with a style test** (phase 3): whether the ink outlines, cel bands and grain of the
  current look stay. Nivalis uses neither outlines nor cel bands.

`docs/art-direction.md` changes accordingly once the style test is done; the photoscanned props
(`assets/models/polyhaven/`) are replaced by box-built ones (phase 7).

## Words

- **Lot:** the 24 m square of the city grid (unchanged); roads run along lot borders.
- **Floor:** a deck level of the stack, 36 m apart (unchanged, `src/world/floors.rs`).
- **Chunk:** a hand-written piece of map on one floor (some span two or more floors: stair hubs,
  atriums, light wells). Chunks are big, to leave room for real places:
  - **areas**, 6x6 to 12x12 lots (144 to 288 m across): a great square, a market district, a
    station, a park around a crater, a construction yard;
  - **blocks**, 3x3 to 5x5 lots: a stair hub with its surroundings, a courtyard block, a market
    street, a light well with its railings;
  - **fillers**, 1x1 to 2x2 lots: repetitive habitation blocks, the city's background.
  A big chunk can leave **sub-areas** open for the planner to fill with smaller chunks of given
  labels (`Sub(area, labels)`), so a 10x10 district is designed as a whole without being one
  monolithic repetition.
- **Module:** a reusable assembly used inside chunks: a bench, a kiosk, a stair flight, a lamp,
  an apartment facade unit. Written once, placed many times.
- **Slot:** a spot in a chunk with a choice the seed makes: "bench, planter or nothing".
- **Label:** what a chunk is (`plaza`, `stair_hub`, `market`, ...), used by the placement rules.
- **Catalog:** all chunks and modules, in `assets/chunks/` and `assets/modules/`.
- **Planner:** the code that decides which chunk goes where, from the seed and the rules.

## Chunk files

One text file per chunk (RON format: readable, with comments, loaded with the `ron` crate).
Coordinates are in meters, local to the chunk: origin at its corner, `y = 0` on the walking
surface of its floor, `x` and `z` along the lot grid. A sketch:

```ron
// The great square: a dry fountain in the middle, tree beds and benches in rings around it,
// colonnaded towers on its four sides, avenues entering at the middle of each side.
Chunk(
    name: "great_square",
    size: (10, 10),                  // lots: 240 m across
    zones: [Depths, Middle],         // where in the stack it may appear
    labels: [Plaza],
    weight: 2,                       // how often, among chunks that fit
    spacing: 30,                     // lots to the nearest other Plaza, at least
    edges: Street,                   // its outer border is the standard street
    inner_roads: false,              // the roads between its lots are paved over
    parts: [
        Box(at: (110, 0, 110), size: (20, 0.6, 20), mat: Stone),        // the basin
        Ring(center: (120, 120), radius: 30, count: 12, parts: [      // benches facing the fountain
            Module("bench_long", at: (0, 0, 0), turn: 0),
            Slot(at: (3, 0, 0), pick: ["planter", "bin", None]),
        ]),
        Mirror4(center: (120, 120), parts: [                         // same on the 4 sides
            Module("tree_bed", at: (120, 0, 70)),
            Repeat(from: (60, 0, 28), step: (8, 0, 0), count: 16, parts: [
                Module("column", at: (0, 0, 0), params: (height: 9)), // the colonnade
            ]),
            Module("tower_colonnade", at: (48, 0, 0), params: (width: 144, depth: 22)),
            Sub(area: ((0, 0), (48, 48)), labels: [Habitation]),     // corner blocks, filled by the planner
        ]),
        Growth(area: ((112, 112), (128, 128)), kind: Soil),           // dense plants in the basin
        Growth(area: ((116, 0), (124, 240)), kind: Bare),             // keep the avenues clear
    ],
)
```

The vocabulary, kept small and grown only when a chunk needs it:

- `Box(at, size, mat)`: a solid box (collides, carries plants).
- `Module(name, at, turn, params)`: a module from the catalog.
- `Slot(at, pick)`: one of several modules, or nothing, picked by the seed.
- `Mirror2` / `Mirror4` / `Repeat` / `Ring`: symmetry and rhythm, the tools that make a place look
  organized.
- `Stair(at, to_floor)`: the stairwell (switchback flights and landings, as today) at that spot.
- `Growth(area, kind)`: hints for the random vegetation: `Soil` (denser), `Bare` (none), `Wall`
  (climbers welcome). Flowers keep their own rules (below).
- `Variant(...)`: alternatives the seed chooses between (a damaged version, a collapsed corner).

The engine mirrors and rotates chunks by quarter turns when placing them, so each file gives up to
eight looks.

## Modules

Same format, without size and placement rules: a named list of parts, with optional parameters
(length, color, count) so one `bench` module serves 2 m and 4 m benches. Modules include the
facade units that make towers (below) and every prop (benches, bins, kiosks, crates, sofas,
signs): props become box-built in the same style as the city.

## The planner

The planner turns the seed into a map of chunks, one floor at a time, from the biggest to the
smallest, every step a pure function of the seed, so the city still streams and stays
deterministic:

1. **Areas.** The plane is divided into large cells (about 32x32 lots per floor). Each cell holds
   at most one area chunk, placed entirely inside it at a seeded spot: areas never overlap and
   the planner never needs to look further than one cell. Spacing between areas of the same label
   (a great square never near another) is checked against the neighboring cells' choices, which
   are pure functions too. Multi-floor areas (an atrium through three floors) reserve the same
   footprint on every floor they span, so the floors agree.
2. **Blocks**, the same way with smaller cells (about 8x8 lots), on the lots no area took, then
   in the areas' `Sub` openings that ask for them.
3. **Stairs.** Stairs only stand in `stair_hub` chunks (blocks, or built into areas), one per
   hub, in the middle. The planner finds the walkable islands of each floor (deck lots joined by
   streets or bridges) and gives every island at least one hub up, so no part of a floor is a
   dead end by accident. Hubs grow rarer going up (the floor profiles' `stair`,
   `src/world/design.rs`), and the planned difficulty of the story (D4 to D5 locked) becomes a
   rule here when its time comes.
3. **Stairs.** Stairs only stand in `stair_hub` chunks, one per hub, in the middle. The planner
   finds the walkable islands of each floor (deck lots joined by streets or bridges) and gives
   every island at least one hub up, so no part of a floor is a dead end by accident. Hubs grow
   rarer going up (the floor profiles' `stair`, `src/world/design.rs`), and the planned
   difficulty of the story (D4 to D5 locked) becomes a rule here when its time comes.
4. **Holes are chunks.** Openings in the floors come from chunks labeled `light_well`,
   `collapse` or `construction_yard`, sized and placed by rules (rare low down, common at the
   construction front; the profiles' `deck_keep` becomes their share), instead of bare missing
   sections. A hole then always looks designed: railings, a collapsed edge, scaffolding.
5. **Fill the rest** with fillers (habitation blocks, tower bases, small yards), weighted by
   zone, with neighbor affinities where they help (a market near a square, workshops near the
   construction front).

**Streaming.** The world still loads lot by lot around the player. A lot asks the planner which
chunk covers it (cheap: the cell's choice), and builds only its own slice of that chunk: the boxes
whose center falls in the lot, so big chunks cost nothing until the player comes near.
6. **Edges.** Every chunk border is, by default, the standard street: half a road and a curb, as
   today, so any chunk fits next to any other. A border with no deck across it becomes a sidewalk
   edge with a railing (already the rule for roads). A chunk may pave over its own inner roads.

## Towers

Towers rise through many floors. Inside an area, the chunk places them (they are part of its
design); elsewhere the filler places them, and a tower keeps its footprint on every floor it
crosses. They are built from **facade modules**: per building, the planner picks a facade style (a set of compatible units and a color
palette) and fills the facade grid with units by rules (shops on the street story, a consistent
column rhythm, balconies in stacks, air conditioners under windows, the odd unit missing or
boarded up). That is the Nivalis wall of apartments, made from a few dozen hand-written units.
Roofs and the construction front follow the same idea with their own modules.

## What stays random

- Which chunk goes where, its mirror and rotation, its variants and slots, its colors.
- **Vegetation and flowers, exactly as today:** grown by `src/world/growth.rs` on the chunk's
  boxes, at the floor profiles' densities and chances, at least 3 m apart, at most 24 per lot and
  floor. Chunks only hint (`Growth` areas); they never place a flower. Points of interest (a
  flower in an armchair) become slots that the seed fills at the profiles' `poi` rate.

## Rendering and performance

- Chunks produce boxes, as lots do today: collision, plants, shadows, the cover map and outlines
  keep working unchanged.
- First version: each placed chunk is built into its lot meshes like today. If the box count gets
  heavy, modules (and later whole chunk variants) are built once and drawn instanced, as the
  props already are (`src/render/props.rs`).
- Pixel textures go into one atlas, sampled with nearest filtering.

## Tools

- `--chunk NAME [--variant N] --shot out.png`: renders one chunk alone on a neutral stage, day
  or night, to review it before it enters the catalog.
- `--catalog`: lists chunks and modules with their labels, zones and weights.
- `--report` gains chunks per label and floor, stair hubs per island, the share of holes.
- Chunk and module files hot-reload like the shaders: edit, see the city regenerate.

## What changes, what stays

- **Stays:** the floor stack and its zones, the floor profiles for plants, flowers and stairs,
  growth, flower lights, the cover map, fog, the day cycle, audio, the player.
- **Replaced:** the lot generator (`city.rs`) becomes the filler while the catalog grows, then
  retires; deck sections become the planner; `dressing.rs` scenes become modules and slots;
  photoscanned props become box-built modules; shader-painted facades become facade modules.

## Phases

Each phase ends with shots the owner can judge.

1. **Design** (this document).
2. **Format and preview:** chunk and module loading, the vocabulary above, `--chunk` preview,
   hot reload. One test chunk.
3. **Style test:** pixel textures and the 0.25 m grain on that chunk, next to the current look.
   The owner picks; outlines and cel bands are decided here.
4. **Planner:** rare chunks with spacing, stair hubs per island, holes as chunks, the current
   generator as filler.
5. **First catalog:** a great square (10x10), a market district (8x6), two stair hub blocks, a
   light well block, a construction yard area, a few habitation fillers. Check that rules plus
   random flowers give the intended feel.
6. **Facade modules** for towers.
7. **Props as modules;** the photoscans are removed.
8. **Grow the catalog.**

## Open questions

- Outlines, cel bands and paper grain in the new style (phase 3).
- Interiors (Nivalis has bars and shops one can enter): out of scope for now, but chunks could
  hold them later.
- Infinite streaming or a fixed map: the planner supports both; to decide when the catalog
  exists.
