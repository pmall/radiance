# World and generation

## Setting

A vast megacity where chrome, concrete and human-made infrastructure form a rigid skeleton, and
organic, engineered growth has spread over and between it: vines, roots, fungal masses, glowing
flora. Nature and technology are intertwined, not opposed. A subtle touch of the fantastic is welcome.

**Empty:** no inhabitants, no vehicles, no movement except the living world (swaying plants,
drifting particles, flickering signs).

## Verticality defines the mood

- **Canopy (D6–D8):** towers and walkways in daylight haze, airy and readable.
- **Middle floors (D3–D5):** dimmer, lit by neon remnants and scattered bioluminescence.
- **Depths (D0–D2):** dark streets and old infrastructure, lit almost only by plants.

## How the city is generated (the system)

Generation is one system, tuned in one place. In order, for each lot (all pure functions of seed
and lot coordinates):

1. **The floor stack** (`src/world/floors.rs`): a fixed but variable number of floors, chosen at
   launch (`--floors N`, 9 by default, up to 16), 36 m apart, the player on D2 (y = 0) with two
   older floors below. Three zones: **depths** (D0 to D2, the old human city), **middle** (the
   floors people moved up to), **canopy** (the top half of the floors above the depths, built by
   machines after the humans).
2. **The floor profiles** (`src/world/design.rs`): for each floor, the chance of a deck per lot,
   the plant density, the flower chances per source (bush, vine, climber branch, point of interest
   per lot), the chance of a stair up, the prop and car densities, and the chances of a block
   chunk per block cell and of a filler per free lot (`docs/chunks.md`). Written
   per zone as values at its lowest and highest floor, interpolated in between, so they hold for
   any number of floors. **Every generation number that depends on the floor lives there.**
3. **Structure** (`city.rs`, now from the planner's chunks, see `docs/chunks.md`): ground, decks (hole chunks), craters, stairs (`stair`), towers,
   bridges, then the hand-written chunks the planner places (`planner.rs`: areas and their
   sub-areas, blocks with their holes, stair hubs, fillers; see `docs/chunks.md`).
4. **Dressing** (now street and filler chunks, `docs/chunks.md`; this was `dressing.rs`): street lights at the crossroads, then props placed by the street
   layout (below), at the floor's `props` and `cars` densities; points of interest drawn apart at the floor's `poi` rate.
5. **Growth** (`growth.rs`): plants at the floor's density, flowers at the floor's chances; plants
   grow around the props.

`--report [radius]` prints the profiles next to what a seed actually produced on each floor
(decks, chunks and holes, stairs, flowers by source, props by kind, chunks placed by name): tuning is measured, not guessed. **O** and **I**
in game (or `--no-props`, `--no-pois`) switch props and points of interest off and regenerate the
city. Values are tuned with the report, then checked in game.

### The floor stack in detail

- **Decks** (history: holes are now hole chunks, `docs/chunks.md`): the city was laid floor by floor, in sections. Each district of 6 x 6 lots is split
  into rectangular sections of one to three lots a side, and each section is built up to some
  floor (`city::built`): a deck on every floor up to it, none above. The chance that a section
  reaches a floor is the profile's `deck_keep`, which never grows going up (97 % on D1, 92 % on
  D3, 45 % on the top floor). So holes are straight-edged, line up from floor to floor like light
  wells and are open to the sky; rare in the depths, common at the construction front. All lots of
  a section share one slab tint per floor. (Owner: the first version, one coin flip per lot and
  floor, looked too random.)
- **Light under the decks:** the renderer keeps a cover map of the lots around the camera
  (`src/render/cover.rs`, room for 16 floors): which decks exist, and how much sky each lot sees on
  each floor (24 rays traced against the decks). The shaders block the sun with it (exact, at any
  distance), dim the sky light and the fog by the sky view, and march the fog so sunbeams show
  through openings.
- **Craters:** round clusters of lots with nothing built above a floor (a middle floor, sometimes
  D2), one per 8x8 lots about half the time. Plants grow there as lushly as in the canopy;
  flowers 1.5x their floor's rate (`design::CRATER_FLOWERS`). A wide one always opens on D2 right
  in front of the spawn, which faces it.
- **Stairs:** switchback stairwells (straight flights going back and forth around a central wall,
  landings at each turn, rails on the open sides), each linking one floor to the next, standing in
  the middle of a hand-written stair hub chunk placed by the planner (`docs/chunks.md`): at most
  one per 6x6-lot cell and floor, at the profile's `stair` chance per cell: rare everywhere (35 %
  of cells on D0, about 10 % below the top), rarer going up, so climbing gets harder and harder.
  Not every part of a floor has a stair: going down to reach another one is part of the climb. Stairs are part of the city like
  the rest: steps and landings grow bushes, and flowers on them, at the floor's rates; a bush on
  the steps keeps the fronds that fit and spills down the flight.
  How the player climbs beyond stairs is not built yet (`docs/game-ideas.md`).
- **Footbridges:** a gap of one or two lots between two decks of a floor (24 or 48 m; wider would
  be physically unlikely), over any hole or crater, gets a footbridge 70 % of the time, along the
  lots' middle line, unless a tower stands in the way or on a landing (`city::bridges_through`,
  `footbridge_span` modules): a metal walkway with railings, rusted girders and crossbeams
  underneath (vines hang from them). It always lands on a deck at both ends; what stands on a
  landing (a rim's railings, a house) gives way. Gaps line up from floor to floor, so bridges often
  stack across the same shaft. They add ways across a floor, which matters now that stairs are
  rare. Bridges never cross (owner): where an x and a z gap share a hole lot, the seed ranks the
  two and only the first may be built, if it beats every gap it crosses; a skybridge passing
  through or just over a walkway (less than 6 m above it) takes the gap's place; floors are 36 m
  apart, so bridges of two floors never touch.
- **No corner contacts:** two decks never meet by a corner alone (owner: unnatural); one of them
  is left open, the seed picking which, unless the plan needs it (a square's, a block's, a stair's
  or a crater's floor). The gap this leaves is often crossed by a footbridge.
- **Roads** (owner: roads must make sense; `src/world/roads.rs`): a road runs along a lot border,
  3 m on each side, only where it can be a real street: both lots are decks not paved over by a
  chunk (a square, a market, a yard) and not one building over several lots, and both its ends are
  full crossings (the four lots around each end are decks). So no road runs into a hole, the edge
  of a deck or a square, and none is half a road: the lots along an edge keep their sidewalk to
  it, with a road running along them one lot back. A road with no other road at one of its ends
  is cut back (four rounds), and a network no more than two lots across (a ring around a block on
  a small island) goes: what is left links crossings to crossings. The shader paints roads,
  curbs, crossings and markings from the same answer (the cover map's road tile), and street
  lights stand where roads cross or turn.
- **Skybridges** between neighboring towers (60 % per pair, sometimes a second one at another
  height) land only on the base tier's walls, under any setback or frame going up.
- **Growth by floor:** plants and flowers rise with the light and culminate on the canopy's first
  floor (D6 with 9 floors), then thin out on the floors above, too new for much to have grown
  (owner's direction).
- **Flower budget:** at most 24 flowers per lot and floor, free to grow close together (only kept
  from growing into each other).
- **Flowers per plant:** a bush has one flower at most, on a stem rising from its
  middle; a hanging vine has none, one or several
  down its length, and a wall climber may flower at every step of its climb (owner's
  direction).
- **Construction front:** about 1 tower in 8 rises 30 to 110 m past the top deck; only those can
  be unfinished skeletons with cranes. Roof and wall props that would pierce a deck are dropped.

Each band should be recognizable without a depth meter: architecture, materials, density,
vegetation and sound all change. Ideas from the brainstorm (`docs/game-ideas.md`, not implemented
unless marked):

- **Deeper is older.** Depths: the original human city (stone facades, arches, ornamented doorways,
  cobblestones, tram tracks, metro entrances, dry fountains, stopped public clocks), broken by the
  massive foundation pillars of the towers above and by roots and soil lifting the pavement.
- **Higher is more machine.** Human-scale stairs, lifts and walkways low; colder, regular,
  machine-built structures high, with fewer human-scale routes and wider gaps. Machine intrusion
  into human spaces (columns through roads, struts across facades, cable bundles) grows with height.
- **Craters** *(first pass done, see above)*: where nothing was built above, a shaft of sunlight
  reaches a middle or deep level, with lush growth underneath. Landmarks seen from far below.
- **Light-network veins:** glowing roots running down walls and across decks, carrying light from
  canopy flowers to those below.

## Procedural and deterministic

- One seed generates one city. Generation happens in chunks streamed around the player.
- Every random decision derives from the seed (and chunk coordinates) through `src/rng.rs`, never
  from time or global state.
- Organic elements use growth-inspired techniques (L-systems, space colonization, noise-driven
  growth), layered on top of a generated architectural skeleton.
- The world itself (layout, architecture, plant placement and growth) is always procedural. Textures
  and other external assets are welcome to dress that geometry whenever they make the render better;
  they are materials and building blocks, not a replacement for generation.

## Biomes

The map is split into large zones (typically 8 to 12 lots across), each with a biome. The owner's
rule: a biome is fully configurable, not only colors and rates (`docs/feedback.md`).
`biome_at(seed, lot)` is pure, so zones stay deterministic and infinite. Zone sites come from a
jittered grid and a lot joins the nearest site (distance divided by the biome's `size`), so each
biome type has its own typical zone size. The spawn plaza is always the default biome.

A biome is a `Biome` const in `src/world/biome.rs` (start from `..DEFAULT`), listed in `BIOMES`,
and a folder `assets/biomes/<name>/`. Between them, a biome sets:

- **Its places:** its chunks and modules (`docs/chunks.md`; `biome.ron` can `include: ["default"]`
  to draw from the default collection too, and a biome without a folder draws from the default
  one). Towers are chunks, so their shapes and elevations come with it; footbridges and
  skybridges are built from its modules too (`footbridge_span`, `skybridge_segment`).
- **Its props:** a module in its folder named like a prop (`bench`, `bin`, `sofa`...) replaces that
  prop in its zones; `street_light` replaces the street light built in code (`world/props.rs`).
  Props it does not write stay as the shared ones, drawn in its own textures (below).
- **Its plants** (`Plants`): flower and foliage colors, how many bushes, vines and climbers, and
  their shapes: each kind is a weighted list of species (`Bush`, `Vine`, `Climber`: sizes, blade
  counts and widths, droop, sway, leaves, branching), plus the flowers' shape and light
  (`Flower`). What depends on the floor (plant density, flower chances) stays in the floor
  profiles.
- **Its air** (`Atmosphere`): sky, fog, key light, ambient light, ink and grading at night, dawn,
  day and dusk (`Looks`, in `render/look.rs`), the colors of pollen and spores, and its color
  script by height (`heights`): the hue the haze and the light take under the decks in the
  depths, the middle and the canopy (`docs/art-direction.md`, "Light under the decks"). The sky
  is drawn in the atmosphere of the player's biome and fades into the next one's over 4 seconds
  when they cross a border (`look::Blend`).
- **Its textures** (`Textures`, in `render/pixels.rs`): its own pixel art for every chunk material
  and facade unit, one row of the atlas per biome, so the same chunk reads differently from one
  biome to the next. A biome sets the shades of each material (stone, concrete, brick, tile,
  metal and what it corrodes to, wood, glass, soil, moss, window frames and panes, laundry), the
  patterns that come in several kinds (slabs or setts, plain or glazed bricks, tile size and how
  much tiles differ, plaster or tiled tower walls, square or round-headed windows) and the wear
  (how corroded metal is, moss in joints and on ledges, damp: tide marks, algae running down,
  misted glass). It also sets the city's own surfaces: how roads are surfaced (asphalt with
  lane markings, or stone without), how sidewalks are laid, and the shades of asphalt, dust,
  litter and standing water; and how worn everything is (`dusty`, `littered`, `puddled`, with
  `mossy` and `damp`), which the shader draws over every surface (`docs/art-direction.md`,
  "Wear"). The default set is the original city's; start a new one from
  `..DEFAULT_TEXTURES`. Lot meshes and props carry their biome, so a prop is drawn in the pixel
  art of the zone it stands in.
- **Its colors** (`Palette`): the eight paints of its tower walls (the shader tints the walls' pixel
  art with them; `tower::paint` gives the same color to the units near the decks) and the tint of
  its pavement; uploaded as a uniform array and picked per lot by the biome's index in `BIOMES`.
- **Its city numbers:** how often a lot has no tower, how often towers get a bridge.
- **Its water:** `Textures::flooded`, the share of its roads under standing water.

Biomes now: `default` (the original city, Neon Garden in `docs/biome-ideas.md`) and `grotto` (Cyan
Grotto, about a third of the zones: its own plants, atmosphere, palette, textures and street
furniture in `assets/biomes/grotto/modules/props.ron`, the default chunks for its places; seed 1
has a zone two lots behind the spawn). Since milestone 10, pass 8, its roads are flooded
(`Textures::flooded`: standing water up to the curbs, `pavement` in `shaders/scene.fs`) with
stepping stones across them (its own `street_plain`), its tree beds, lamps, kiosks, fountains and
drinking fountains are tiled (`grotto/modules/places.ron`, winning over the shared modules of those
names), and its landmark is the `bathhouse`, a tiled stepped dome over a pool (a block, common
there).

**Adding one** (only when the owner asks, `docs/chunks.md`, "Catalog rule"): its `Biome` const and
its entry in `BIOMES`, its folder; then `cargo test --release` (`biomes_fit_the_renderer_and_the_zones`
checks the limits below and sane plant ranges, `every_biome_has_zones_and_the_spawn_is_default`
the zones, `every_biome_draws_its_own_tiles` its textures), `--catalog` (load errors), `--report` (where its chunks land), and shots: `--chunk
BIOME/NAME` shows a chunk in its biome's whole look; look at its zones and a border at several
`--time`s, with the overlay (it names the biome under the player).

**Not yet:**

- Biomes through the floors *(possible future development, owner; not started)*: a biome's zone
  stays a column through every floor (same map on every floor, so the city stays consistent), but
  the biome can look different in each of the three sections of the stack, the depths, the middle
  and the canopy (`floors::Zone`), for a vertical evolution: the grotto flooded and choked with
  algae deep down, its bathhouse district in the middle, bleached and salt-crusted near the top.
  A biome does not have to evolve, but it is better for the game's consistency that most do, the
  default city included (it is one biome like the others: older architecture deep, brick and
  stone and heavy rust, newer up high). Shape agreed so far:
  - A section only overrides what differs from the biome's base (textures, wall paints, sky and
    fog, plant species and colors); a biome without overrides looks the same at every height.
    Plant density already follows the floor profiles, and a biome's chunks can already be kept to
    sections (`zones`).
  - The switch happens at the decks between sections, where the player cannot see across; the
    sky fades as at a zone border (`look::Blend`) when they take a stair into another section.
  - Towers crossing a section line change pixel art at that floor (the shader knows each pixel's
    height): an older base under newer stories, as the machines kept building.
  - The renderer needs room for up to three looks per biome: atlas rows, palettes and the mesh's
    material code (see the limits below).
- New kinds of plants: species vary bushes, vines and climbers widely, but a mushroom or a tree is
  new code in `world/growth.rs`, written with the biome that needs it.
- Tower heights come from the floor (`design::TOWER_HEIGHTS`), not the biome.
- Biome settings are Rust constants: editing one takes a rebuild (chunks and modules hot-reload).
- Borders *(can wait, owner)*: palette, plants and density flip at the zone line, and the far side
  of a border is drawn in the player's own atmosphere; an area, block or hub picked in one biome
  is colored lot by lot, so it can change colors at the border.
- Textures are drawn by code from a biome's settings: a new kind of pattern (a new window shape,
  a new paving) is new code in `render/pixels.rs`, written with the biome that needs it. The
  pavement of the streets (`pavement` in `shaders/scene.fs`) is not pixel art and only takes the
  biome's tint.
- At most 8 biomes: the shader's palette and the atlas's rows (`MAX_BIOMES`, `uPalette` and
  `ATLAS` in `shaders/scene.fs`); the mesh's material code has room for 16.
- More biomes, including a rare "rainbow" one with every flower color, and the grotto's own chunks.

## Set dressing (history: replaced by street, filler and tower chunks, `docs/chunks.md`)

Every lot follows one street layout, the one the pavement shader draws: a road along each border
(3 m deep on each lot's side), a curb, the sidewalk, then the tower's facades. `src/world/dressing.rs`
places each scene in the kind of place it belongs to, never at random:

- **Streets only between decks:** a lot border is a street on a floor only if the lot across it
  has a deck there; at the edge of a section the sidewalk runs to the edge instead (the pavement
  shader reads the cover map), so a road is never cut in half along its length. Curb, corner and
  parking spots and street lights only stand along real streets. (Now the road network,
  "Roads" above.)
- **Street lights:** not drawn at random but laid out like a real street: one per crossroads (two
  streets meeting), on a sidewalk corner, its arm reaching over the crossing; each street keeps its lights on the same
  side all along it, so they line up one per lot. Built in code (a modern pole and arm; no free
  model fits). They follow the props switch (O).
- **Curb:** on the sidewalk just behind the curb, facing the road: benches, bench and bin, bins,
  utility cabinets, planters.
- **Corner:** the sidewalk corner of two streets: utility cabinets, planters.
- **Facade:** against a tower wall, facing out: trash bags, barrels and crates, boxes, café
  terraces (D1–D3), a sofa with boxes (someone moving out), utility cabinets.
- **Parking:** in the road along the curb, clear of crossings. Empty for now: no car yet. Nothing
  else ever stands in the road (tested).
- **Vacant lot:** lots without a tower: debris, a small square with two benches facing each other.

Props are box-built modules in the chunks' pixel-art materials (`assets/modules/props.ron`), the
street light a mesh built in code (`docs/art-direction.md`, "Props and external assets").

Every scene declares its slot, its kind (street furniture, leftovers, vehicles, points of
interest) and a weight per zone; how many are tried per lot follows the floor's profile (about 4
to 8 props per lot in the depths, a third of the first pass at the owner's request, thinning
through the middle, almost none in the canopy). Bushes next to props grow smaller instead of
not at all. No indoor furniture out in the street except as a story (moving out, a shop emptied),
and no construction props in the depths (they belong to the construction front, not done yet).
Every prop stands fully on a deck, clear of everything else, with a collision box; plants grow
around them.

Points of interest with flowers (flowers in human places, `docs/poi-catalog.md`): a flower
growing out of an armchair left by a wall, two flowers side by side on a bench, a dead garden of clay pots with one living flower, an old bed frame against a wall
with a flower where someone slept. They are rare on purpose (a few % of
lots, the profile's `poi`): too many would spell out that flowers are souls. Only the armchair
always stands on the start plaza, facing the crater.

### Ideas still to do (from the brainstorm's decoration lists)

Streets and interiors should be cluttered: as a starting point, rarely more than about ten meters
without passing a prop. Props vary in condition (intact, broken, overgrown, half-swallowed) and
come in small scenes (a bicycle against a railing, chairs around a café table). Growth covers them,
matching the biome. Props can host flowers (a planter, a pavement crack, a seat), but flowers are
still placed by the flower system. Everything is placed by the seed.

- **Everywhere:** lamp posts (mostly dead), benches, bins, bollards, railings, planters with dead
  plants; shop signs, dead neon fragments, street plates, faded billboards for invented brands,
  graffiti (often suns); pipes, ducts, vents, AC units, cables, manholes, drains; bicycles,
  scooters, carts, crates, bags, chairs; closed shutters, half-open doors, broken and dark windows.
- **Service levels:** lobbies with reception desks and waiting chairs, dead fluorescent tubes,
  elevator banks, maintenance hatches, powered-down cleaning robots, office desks and partitions.
- **Lower streets:** balconies, laundry lines across streets, intercoms, café terraces, stalls,
  kiosks, mailbox banks, chained bicycles, courtyards with dead trees and toys.
- **Foundations:** see "Deeper is older" above, plus old parked cars under dust and growth.
