# City chunks: design

Status: on the `chunks` branch. Phases 2 to 4 are done: format, catalog and preview; the pixel-art
style; the planner placing areas and stair hubs in the city. Next: the first catalog. As phases land, this replaces the lot-by-lot
generation described in `docs/world.md` (see "Phases").

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

One text file per chunk in `assets/chunks/` (RON format: readable, with comments, loaded with the
`ron` crate); modules come as lists in `assets/modules/`. Coordinates are in meters, local to the
chunk: origin at its corner, `y = 0` on the walking surface of its floor, `x` and `z` along the lot
grid. The city is made of axis-aligned boxes, so every turn is a quarter turn. The real example is
`assets/chunks/great_square.ron` (10x10 lots); its header:

```ron
Chunk(
    name: "great_square",
    size: (10, 10),                  // lots: 240 m across
    zones: [Depths, Middle],         // where in the stack it may appear
    labels: ["plaza"],
    weight: 2,                       // how often, among chunks that fit
    spacing: 30,                     // lots to the nearest other plaza, at least
    ground: Some(Stone),             // paved over, inner roads included (None keeps the streets)
    parts: [ ... ],
)
```

The vocabulary (`src/world/chunks.rs`), grown only when a chunk needs it:

- `Box(at, size, mat)`: a solid box, `at` its lowest corner (collides, carries plants). Materials:
  `Stone`, `StoneDark`, `Concrete`, `Brick`, `Tile`, `Metal`, `Rust`, `Wood`, `Glass`, `Soil`,
  `Paint((r, g, b))`.
- `Module(name, at, turn)`: a module from the catalog, turned by `turn` quarter turns.
- `Slot(at, turn, pick)`: one of several modules, or nothing (`None`), picked by the seed.
- `Mirror2(axis, at, parts)`: the parts and their mirror image; two of them nested give the four-way
  symmetry of a square. `Mirror4(center, parts)`: the parts turned four times around a center (a
  pinwheel). `Repeat(from, step, count, parts)`: rhythm (columns, windows, rows of beds).
- `Stair(at, along_x)`: a switchback stairwell (as today) up to the next floor.
- `Growth(area, kind)`: hints for the random vegetation: `Soil` (six times denser; soil boxes too),
  `Bare` (nothing grows: paths, avenues), `Wall` (climbers welcome, not used yet). Flowers keep their
  own rules (below).
- Not built yet: `Sub(area, labels)` (openings for the planner), `Variant(...)` (alternatives the
  seed chooses between), module parameters (a 2 m or 4 m bench from one module).

The engine turns chunks by quarter turns when placing them (and later mirrors them), so each file
gives several looks.

**Lesson from the first chunk:** a 240 m square needs far more content than one written corner of
beds and benches: at this size, emptiness shows. Big chunks must be dense (terraces, stalls,
canopies, level changes, sub-areas), which is what the sub-areas and a larger module library are
for.

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
   hub, in the middle. They are rare, and not every walkable island of a floor has one: going
   down to reach another stair is part of the climb (owner). Hubs grow rarer going up (the floor profiles' `stair`,
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

- `--chunk NAME` (with `--view`, `--time`, `--shot` as usual): shows one chunk alone on a bare
  deck in front of the spawn, which stands at the middle of its near edge looking in. Plants and
  flowers grow on it at random as they will in the city. *(Built.)*
- `--catalog`: lists chunks and modules with their labels, zones and weights, and the files that
  failed to load. *(Built.)*
- `--report` gains chunks per label and floor, stair hubs per island, the share of holes.
- Chunk and module files hot-reload like the shaders: edit, see the city regenerate. *(Built.)*

## What changes, what stays

- **Stays:** the floor stack and its zones, the floor profiles for plants, flowers and stairs,
  growth, flower lights, the cover map, fog, the day cycle, audio, the player.
- **Replaced:** the lot generator (`city.rs`) becomes the filler while the catalog grows, then
  retires; deck sections become the planner; `dressing.rs` scenes become modules and slots;
  photoscanned props become box-built modules; shader-painted facades become facade modules.

## Phases

Each phase ends with shots the owner can judge.

1. **Design** (this document).
2. **Format and preview** *(done)*: chunk and module loading, the vocabulary above, `--chunk`
   preview, hot reload, vegetation hints. Test chunk: the great square. Big chunk boxes belong to
   the lot holding their center and reach the neighbors as collision only.
3. **Style test** *(done; the owner keeps the ink outlines and flat light bands on, and the
   pixel-art materials; plants as pixel or voxel foliage is a suggestion still open)*: every chunk material has a pixel-art
   tile (4 m at 16 pixels per meter, limited palette, ordered dithering, chunky joints, cracks,
   rivets, stains), drawn by code in `src/render/pixels.rs` and sampled with crisp nearest
   filtering. To compare: `--chunk great_square` with F3 (outlines), F5 (flat light bands) and
   F8 (grain) in game, or `--fx 255` against `--fx 234` in shots. Seen so far: without outlines
   and bands the image is softer and closer to Nivalis; the plants (smooth folded leaves) do not
   match the pixel world yet, which calls for pixel or voxel foliage.
4. **Planner** *(done, first pass: `src/world/planner.rs`)*:
   - Areas: one cell of 32x32 lots holds at most one (60 %), on one floor of its zones, turned at
     random, never near the start; same-label areas keep their spacing (the lower priority roll
     gives way). Craters give way to areas; no tower stands in an area. The area's floor is a
     full deck paved with its ground; the floors above stay as their sections are (a square in
     the depths is a dark hall under the next deck).
   - Stair hubs replace the random stairs: a cell of 6x6 lots holds at most one per floor, at the
     floor's `stair` chance (now per cell: 35 % on D0 down to about 10 % on D7: rare, owner),
     its stair in a free lot of the cell (no tower rising through that floor, a deck above, not a
     lot a lower floor's stair took), the hub chunk centered on it. One always stands beside the
     start (`planner::START_HUB`). Hub boxes skip lots without a deck there or in an area, and
     every chunk box gives way to towers and the lot's own architecture.
   - Lots under a chunk get no random props on its floor (the chunk brings its own).
   - No stair is guaranteed on every walkable island (owner: going down to reach another stair is
     fine). Bridges cross the gaps of a bridge's size instead (`city::bridges_through`).
   - Not yet: holes as chunks, filling `Sub` openings, blocks other than hubs, fillers. The planner answers are cached per cell.
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
