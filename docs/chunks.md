# City chunks

The city is built from hand-written pieces of map (chunks) placed by the seed with rules: every
lot of every floor is a chunk. The format and its vocabulary are documented in
`src/world/chunks.rs`, the placement rules in `src/world/planner.rs`; this page says what the
system is for and how its pieces relate.

## Why

Random placement can't make a place look organized: a square needs symmetry, alignment and a
center, and a stair belongs in the middle of its stairwell, not as five stairs popping up next to
each other. The owner's direction: build the city from **large pieces of map written by hand (as
text)**, placed by the seed with **rules**, and keep **vegetation and flowers fully random** on
top. The darkness stays unpredictable because its only light sources, the flowers, are always
random; the layout gains the order and density a real city has.

Everything stays procedural in the sense the project requires: the seed decides which piece goes
where, turned how, with which slots filled, and every plant and flower. Pieces only decide how a
place is organized. The look they are drawn in: `docs/art-direction.md`, "Style target".

## Words

- **Lot:** the 24 m square of the city grid; roads run along lot borders.
- **Floor:** a deck level of the stack, 36 m apart (`docs/world.md`, "The floor stack").
- **Chunk:** a hand-written piece of map on one floor (some reach through several: stair hubs,
  holes, craters). Its **role** says how the planner places it:
  - **areas**, 6x6 lots and more: a great square, a market district, a station, a stadium;
  - **sub-areas**, openings an area leaves (`Sub`) for the planner to fill with smaller chunks of
    given labels, so a 10x10 district is designed as a whole without being one monolithic
    repetition;
  - **blocks**, a few lots a side: a courtyard, a pocket park, a special place;
  - **hubs**, which frame a stair; **holes** and **craters**, which open the floors;
  - **fillers**, one lot: housing and yards, the city's background;
  - **towers**, rising through the floors; **streets**, the lamps and curb furniture around a
    lot.
- **Module:** a reusable assembly used inside chunks: a bench, a kiosk, a stair flight, a lamp,
  a facade unit. Written once, placed many times. Every prop is a module.
- **Slot:** a spot in a chunk with a choice the seed makes: "bench, planter or nothing".
- **Label:** what a chunk is (`plaza`, `stair_hub`, `market`, ...), used by the placement rules.
- **Biome:** a collection of chunks and modules (below); each zone of the map draws from its
  biome's collection.
- **Catalog:** all chunks and modules.
- **Planner:** the code that decides which chunk goes where, from the seed and the rules.

## Chunk files

One text file (RON) per chunk; modules come as lists. Coordinates are in meters, local to the
chunk: origin at its corner, `y = 0` on the walking surface of its floor. The city is made of
axis-aligned boxes, so every turn is a quarter turn. Architecture snaps to a 0.25 m grid, small
details and props to 0.125 m. A full example: `assets/biomes/default/chunks/great_square.ron`.

The vocabulary (`chunks::Part`, `Role`, `Rate`, `TowerDef`) grows only when a chunk needs it.
Not built yet: variants the seed chooses between, module parameters (a 2 m or 4 m bench from one
module), mirrored placements.

Big chunks must be dense (terraces, stalls, canopies, level changes, sub-areas): at 240 m across,
emptiness shows.

## Catalog rule

The catalog does not grow on its own (owner): no chunks or props are written on the side. New
content comes when the owner asks for it: a **new biome**, whose chunks and props are written then,
in its own folder, or the **points of interest** of `docs/poi-catalog.md`.

## Biomes

A biome is a collection of chunks and modules; the default city is the `default` biome. On disk:

```
assets/modules/                 shared modules: props, street furniture, facade units (any biome)
assets/biomes/<name>/chunks/    the biome's chunks
assets/biomes/<name>/modules/   modules only this biome uses (win over shared ones of that name)
assets/biomes/<name>/biome.ron  optional: Biome(include: ["default"]) also draws from those
```

The planner makes every choice from the collection of the biome where it happens: an area or a
block from the biome of its cell's middle lot, a filler from its lot's, a sub-area from its
area's, facade units from the tower's lot. A biome without a folder draws from `default`, so a new
biome can start with only its parameters, and `include` lets it reuse the default fillers while
adding its own places. A module of a prop's name in a biome's folder replaces that prop in its
zones. Everything else a biome sets: `docs/world.md`, "Biomes".

## The planner

The planner turns the seed into a map of chunks, from the biggest pieces to the smallest, every
answer a pure function of the seed and the lot, so the city still streams lot by lot
(`src/world/planner.rs`). Each kind of chunk has its own grid of cells, at most one per cell and
floor, at the floor profile's chance: areas (at most one per cell of `AREA_CELL` lots, same labels kept apart),
then their sub-areas, blocks, holes, craters, stair hubs, towers, and fillers on what is left.
Where chunks overlap on a floor, the lesser gives way. A lot builds only its own slice of a chunk
(the boxes whose center falls in it), so big chunks cost nothing until the player comes near.

**Edges.** Every chunk border is, by default, the standard street: half a road and a curb, so any
chunk fits next to any other, wherever the road network has a road there (`docs/world.md`,
"Structure"); elsewhere the sidewalk runs to the border. A chunk may pave over its own inner roads
(`ground`), and no road borders it then. `If` parts adapt a chunk to the roads around its lot.

## Streets

`street_plain` stands on every lot and floor that no area, block, hub or hole designs, around its
tower or filler: a street light where roads cross or turn, curb furniture along its roads, a point
of interest now and then. `street_canopy` (lamps only) replaces it in the canopy. Free lots get a
filler (housing or yards), else a `vacant_lot`. The spawn's plaza is `start_plaza`.

## Holes

Hole chunks (`light_well`, `collapse`, `deck_gap`, `deck_void`, `unbuilt` at the construction
front's edge) open their floor and every floor above with their `holes`, and dress the rim
(railings, a collapsed edge, girders over the void, a fence). A footbridge may cross a gap
(`docs/world.md`, "Structure"); `towers_rise` lets towers stand in a hole; rim pieces give way
where a lower hole left no deck. Craters (`crater_small`, `crater`, `crater_start` before the
spawn) open every floor above their own, which is a sunlit garden of rubble.

## Towers

Towers rise from the bottom of the stack through the floors (`world/tower.rs`). A tower is
`tiers` stacked from the bottom (a setback takes the top stories the seed picks); each tier has a
body painted from its written **elevations**, and parts on each story, on each floor (fire
escapes, signs), against its walls where a deck is (`streets`) and on its roof. Elevations are
written as seen from outside, top story first, street story last, one letter per 4 m bay (the
letters: `chunks::UNITS`), and repeat every floor. The shader reads them from a texture; near a
deck (the street story and the two above) the same letters stand as units in boxes. That is the
Nivalis wall of apartments, made from a few dozen hand-written units.

The planner gives each tower a height (`design::TOWER_HEIGHTS`). Most are housing; the towers
labeled `landmark` are not housing or not whole (offices, glass, a hotel, a telecom shaft, a
gutted one, a breached one, a leaning one at the construction front, the tallest spires), and
`--report` lists them with a view. The towers rising past the top deck are also drawn far beyond
the loaded lots, as the skyline.

## What stays random

- Which chunk goes where, its turn, its slots and its colors.
- **Vegetation and flowers:** grown by `src/world/growth.rs` on the chunk's boxes, at the floor
  profiles' densities and chances (`docs/world.md`, "Growth"). Chunks only hint (`Growth` areas);
  they never place a flower, except a point of interest's (`Bloom`), in a slot the seed fills at
  the profile's `poi` rate.

## Open questions

- Interiors (Nivalis has bars and shops one can enter): out of scope for now, but chunks could
  hold them later.
