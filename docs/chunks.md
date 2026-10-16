# City chunks: design

Status: on the `chunks` branch. Phases 2 to 7 are done, and "everything is a chunk" (section
below) too: every lot of every floor is a chunk placed by the planner (towers, streets, fillers,
holes, craters, blocks, hubs, areas); no random architecture code is left. This replaces the
lot-by-lot generation described in `docs/world.md` (its "Decks" and "Set dressing" sections are
history).

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

Props are box-built modules too (phase 7, done).

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
- **Biome:** a collection of chunks and modules (see "Biomes"); the city's zones each draw from
  their biome's collection. Today there is one, `default`.
- **Catalog:** all chunks and modules: `assets/biomes/<biome>/chunks/` and `.../modules/`, and the
  shared modules of `assets/modules/`.
- **Planner:** the code that decides which chunk goes where, from the seed and the rules.

## Chunk files

One text file per chunk in `assets/biomes/<biome>/chunks/` (RON format: readable, with comments, loaded with the
`ron` crate); modules come as lists in `assets/modules/`. Coordinates are in meters, local to the
chunk: origin at its corner, `y = 0` on the walking surface of its floor, `x` and `z` along the lot
grid. The city is made of axis-aligned boxes, so every turn is a quarter turn. The real example is
`assets/biomes/default/chunks/great_square.ron` (10x10 lots); its header:

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
  `Paint((r, g, b))`, and `Accent`: paint in the color given by the module or slot using it.
- `Module(name, at, turn, accent)`: a module from the catalog, turned by `turn` quarter turns,
  its `Accent` surfaces in `accent`.
- `Slot(at, turn, pick, accents)`: one of several modules, or nothing (`None`), picked by the
  seed, with an accent color picked among `accents`: one shophouse module, every shop its color.
  Optional values need no `Some(...)` (RON's implicit-some extension), except in `pick` lists.
- `Mirror2(axis, at, parts)`: the parts and their mirror image; two of them nested give the four-way
  symmetry of a square. `Mirror4(center, parts)`: the parts turned four times around a center (a
  pinwheel). `Repeat(from, step, count, parts)`: rhythm (columns, windows, rows of beds).
- `Stair(at, along_x)`: a switchback stairwell (as today) up to the next floor.
- `Growth(area, kind)`: hints for the random vegetation: `Soil` (six times denser; soil boxes too),
  `Bare` (nothing grows: paths, avenues), `Wall` (climbers welcome, not used yet). Flowers keep their
  own rules (below).
- `Sub(area, labels)`: an opening (whole lots) the planner fills with a chunk carrying one of the
  labels, turned to fit and centered (the great square's quarters: `plaza_quarter`).
- Header fields: `role` (`Area`, `Block`, `Hub`, `Sub`, `Filler`; left out it follows from the
  size: 6 lots or more an area, `stair_hub` a hub, one lot a filler, else a block) and `holes`
  (lot rectangles left open on the chunk's floor and every floor above: light wells, collapses).
- Not built yet: `Variant(...)` (alternatives the seed chooses between), module parameters (a 2 m
  or 4 m bench from one module).

The engine turns chunks by quarter turns when placing them (and later mirrors them), so each file
gives several looks.

**Lesson from the first chunk:** a 240 m square needs far more content than one written corner of
beds and benches: at this size, emptiness shows. Big chunks must be dense (terraces, stalls,
canopies, level changes, sub-areas), which is what the sub-areas and a larger module library are
for.

## Catalog rule (owner)

The catalog does not grow on its own: no new chunks or props are written unless the owner asks.
When the owner wants more content, they ask for a **new biome**, and that biome's chunks and props
are written then, in its own folder. The `default` biome is what exists now.

## Biomes

A biome is a meta collection of chunks: in the end every biome has its own set of chunks and props,
and the default city is the `default` biome. On disk:

```
assets/modules/                 shared modules: props, street furniture, facade units (any biome)
assets/biomes/default/chunks/   the default biome's chunks
assets/biomes/default/modules/  modules only this biome uses (win over shared ones of that name)
assets/biomes/<name>/biome.ron  optional: Biome(include: ["default"]) also draws from those
```

The map's zones come from `src/world/biome.rs` (where each biome's flowers, densities and palette
live, under the same name as its folder). The planner makes every choice from the collection of the
biome where it happens: an area or a block from the biome of its cell's middle lot, a filler from
its lot's, a sub-area from its area's; facade units from the tower's lot. A biome without a folder
draws from `default`, so a new biome can start with only its parameters, and `include` lets it
reuse the default fillers while adding its own places. `--chunk BIOME/NAME` previews a biome's
chunk; `--catalog` lists every biome's collection.

Still to do when a second biome arrives: biome parameters as text next to the chunks (now Rust
constants in `biome.rs`), and borders between biomes (today a chunk simply belongs to one side).

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
4. **Holes are chunks.** Openings in the floors come from chunks labeled `light_well`,
   `collapse` or `construction_yard`, sized and placed by rules (rare low down, common at the
   construction front), instead of bare missing sections. A hole then always looks designed:
   railings, a collapsed edge, scaffolding. *(First pass: blocks with `holes` exist alongside the
   deck sections, which still make most holes; moving the profiles' `deck_keep` over to hole
   chunks is still to do.)*
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

*(First pass built.)* The tower bodies are still single boxes, painted by the scene shader
(`facade` in `shaders/scene.fs`) as a grid of 4 m bays and 4 m stories, each cell a pixel-art
facade unit of the atlas (`src/render/pixels.rs`: window, two windows, caged window, enclosed
balcony, air conditioner, boarded, broken, vent; shops on the street story of every floor), units
running in vertical stacks with the odd one out, the walls tinted in color blocks per face (runs of
bays and stories, faded paints or bare concrete). Abandoned: no window lit, signs unlit. Where the
player sees them up close, the two stories over each deck the tower crosses, the shader paints plain
windows and `world/facade.rs` places real units from `assets/modules/facade.ron` on the same grid
(air conditioners, cages, balconies, laundry, shop awnings, sign boxes). The plan below stays the
target for later passes.

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

- `--chunk NAME` or `--chunk BIOME/NAME` (with `--view`, `--time`, `--shot` as usual): shows one chunk alone on a bare
  deck in front of the spawn, which stands at the middle of its near edge looking in. Plants and
  flowers grow on it at random as they will in the city. *(Built.)*
- `--catalog`: lists chunks and modules with their labels, zones and weights, and the files that
  failed to load. *(Built.)*
- `--report` gives, per floor, the lots covered by areas, blocks and fillers and opened by holes,
  and the chunks placed by name. *(Built.)*
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
4. **Planner** *(done: `src/world/planner.rs`)*:
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
   - Sub-areas: an area's `Sub` openings get a chunk of the labels they ask for (role `Sub`).
   - Blocks: a cell of 8x8 lots holds at most one per floor, at the floor's `blocks` chance
     (`design.rs`), entirely inside the cell, away from areas, craters and the start, on lots
     built up to its floor (its floor becomes a full deck); a block's `holes` open its floor and
     every floor above, never with a tower rising through, never over a lower block's hole. Hubs
     avoid blocks; footbridges avoid designed holes.
   - Fillers: every lot left free on a floor (a full deck, no stair, no tower through the floor,
     no other chunk) holds a one-lot filler at the floor's `fillers` chance (50 % in the depths
     down to 8 % at the top). A filler takes the inside of the lot; the street around keeps its
     props and lights.
   - Where chunks overlap on a floor, the lesser gives way: areas and sub-areas, then blocks,
     then hubs' surroundings, then fillers. The planner's cell answers are cached.
5. **First catalog** *(done)*: areas `great_square` (10x10, depths and middle; its four quarters
   are sub-areas: `plaza_garden`, `plaza_terrace`, `plaza_bandstand`, `plaza_market`),
   `market_district` (8x6, depths and middle: shophouse lanes, stalls, flags, a covered hall),
   `construction_yard` (6x6, middle and canopy: fence, crane, skeletons, stacks, cabins); blocks
   `light_well` and `collapse` (3x3 with a hole in the middle), `pocket_park` and
   `laundry_court` (2x2); stair hubs `stair_hub_square` and `stair_hub_market`; fillers
   `housing_row` (walk-ups back to back, each its color), `housing_block` (five stories),
   `laundry_yard`, `depot` (middle and canopy); modules for street furniture, architecture,
   housing (walk-ups, balconies, boarded windows, roof clutter), railings, rubble, parasols, a
   pergola, a bandstand, construction.
6. **Facade modules** for towers *(first pass done, see "Towers")*.
7. **Props as modules** *(done)*: props are modules of `assets/modules/props.ron` drawn
   instanced in pixel-art materials; the photoscans, their import tool and dependencies are gone.
8. **More content comes as new biomes**, only when the owner asks for one (see "Catalog rule").

## Everything is a chunk *(done)*

The owner's goal: many chunks defined by text, each structured by hand, assembled randomly. Plants
and flowers stay fully random (`growth.rs`). What the random code used to build is now chunks of
the `default` biome:

- **Towers** (role `Tower`, `world/tower.rs`): `tower_slab`, `tower_point`, `tower_wall`,
  `tower_office`, `tower_narrow`, `tower_old` (heights `Old` only), `tower_unfinished` (the
  construction front: a frame of stories and a crane), `tower_twin` (2x1 lots) and `tower_block`
  (2x2). A tower is `tiers` stacked from the bottom of the stack (a setback takes the top
  `stories` the seed picks, at its `chance`); each tier has a body painted from its written
  **elevations** and parts `each_story`, `each_floor` (fire escapes, signs), `streets` (what stands
  against its walls, only where a deck is) and `roof`. Elevations are written as seen from outside,
  top story first, street story last, one letter per 4 m bay (`chunks::UNITS`: `w` window, `d`
  two windows, `c` cage, `b` balcony, `a`/`A` air conditioner, `l` laundry, `u` the seed's pick,
  `x` boarded, `k` broken, `v` vent, `-` blank, `s`/`S`/`h`/`H` shops), a digit after a letter
  paints the unit in one of the tower's colors. The rows repeat every floor from its street story
  up. The shader reads them from a texture (`render/facades.rs`) instead of rolling units; near a
  deck (the street story and the two above) the units stand in boxes from the same letters. The
  planner gives each tower a height (`design::TOWER_HEIGHTS`) and, in cells of 2x2 lots, sometimes
  a big tower.
- **Streets** (role `Street`, never turned): `street_plain` on every lot and floor no area, block,
  hub or hole designs, around its tower or filler: a street light at the crossing of the lot's
  -x/-z corner (one per crossing), curb furniture at the floor's `props` chance, a point of
  interest at its `poi` chance; `street_canopy` (lamps only) in the canopy. Free lots get a filler:
  housing at the floor's `fillers` chance, else a `vacant_lot`. The spawn's plaza is
  `start_plaza`, with the armchair and its flower. `dressing.rs` is gone.
- **Holes** (role `Hole`, one per 6x6-lot cell and floor at the floor's `holes` chance, each floor
  its own shifted grid): `light_well`, `collapse`, `deck_gap` (a footbridge across, or railings),
  `deck_void`, `unbuilt` (the construction front's edge: fences, girders over the void, a crane).
  A hole opens its floor and every floor above; `towers_rise` lets towers stand in it. Rim pieces
  give way where a lower hole left no deck. Deck sections and `deck_keep` are gone; the measured
  deck shares stay close to the old ones (`--report 12`: about 98 % on D2, 75 % on D5, 35 % on D8).
- **Craters** (role `Crater`): `crater_small`, `crater`, and `crater_start` always in front of the
  spawn; their `holes` (the round shape) open every floor above their own, which is a sunlit
  garden of rubble. Footbridges over every gap of one or two lots (`deck_gap` included) and skybridges
  between towers stay rules (`city::bridges_through`, `city::push_bridge`) built from the
  `footbridge_span` and `skybridge_segment` modules.
- **Giving way, once per placement:** a module whose boxes reach over several lots (a crane, a long
  railing) stands or gives way as a whole, decided the same in every lot from the plan alone
  (decks, towers, stairs, footbridges of each lot it touches: `Pieces::spanning_blocked`); a piece
  reaching out of the placement is built by the nearest lot it covers. Before, each lot decided
  with its own obstacles, which left crane jibs floating without their mast.

**New vocabulary:** `If(street: [..], edge: [..], parts)` (by which sides of the lot have a deck on
this floor), `Bloom(at)` (a point of interest's flower), slot `rate: Props | Poi` (the floor
profile's chances and the O / I switches). Modules that are props (`props::by_module`, including
`street_light`) are placed instanced, drawn within 110 m. A module or slot's boxes stand or give
way together (groups).

**Performance:** each lot's small boxes (largest side under 3.5 m: units, rails, furniture) go to a
detail mesh drawn within 110 m and shadowed within 45 m; frame time stays where it was (about 7.1
to 8.0 ms on the usual views). Growth indexes blocks in 4 m columns and generated lots are cached,
so a lot grows in about 2.5 ms (was 11); the test suite runs in under a second.

**Rules kept:** same seed, same city; every planner answer a pure function of the seed and the lot;
big chunks' boxes belong to the lot holding their center.

## Open questions

- Outlines, cel bands and paper grain in the new style (phase 3): outlines and bands stay
  (owner).
- Lit windows and neon: the abandoned towers have none now (as this plan says), so at night the
  flowers are the only light on the facades; to check with the owner.
- Interiors (Nivalis has bars and shops one can enter): out of scope for now, but chunks could
  hold them later.
- Infinite streaming or a fixed map: the planner supports both; to decide when the catalog
  exists.
