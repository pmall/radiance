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
   per lot), the chance of a stair up, and the prop and car densities. Written
   per zone as values at its lowest and highest floor, interpolated in between, so they hold for
   any number of floors. **Every generation number that depends on the floor lives there.**
3. **Structure** (`city.rs`): ground, decks (`deck_keep`), craters, stairs (`stair`), towers,
   bridges.
4. **Dressing** (`dressing.rs`): street lights at the crossroads, then props placed by the street
   layout (below), at the floor's `props` and `cars` densities; points of interest drawn apart at the floor's `poi` rate.
5. **Growth** (`growth.rs`): plants at the floor's density, flowers at the floor's chances; plants
   grow around the props.

`--report [radius]` prints the profiles next to what a seed actually produced on each floor
(decks, stairs, flowers by source, props by kind): tuning is measured, not guessed. **O** and **I**
in game (or `--no-props`, `--no-pois`) switch props and points of interest off and regenerate the
city. Values are tuned with the report, then checked in game.

### The floor stack in detail

- **Decks:** the city was laid floor by floor, in sections. Each district of 6 x 6 lots is split
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
  the rest: steps and landings grow bushes, and flowers on them, at the floor's rates, the bushes
  shrinking to fit between the rails and the next flight.
  How the player climbs beyond stairs is not built yet (`docs/game-ideas.md`).
- **Footbridges:** a gap of one or two lots between two decks of a floor (24 or 48 m; wider would
  be physically unlikely) gets a footbridge 60 % of the time, along the lots' middle line, unless
  a tower stands in the way (`city::bridges_through`): a metal walkway with railings, rusted
  girders and crossbeams underneath (vines hang from them). Gaps line up from floor to floor, so
  bridges often stack across the same shaft. They add ways across a floor, which matters now
  that stairs are rare.
- **Growth by floor:** plants and flowers rise with the light and culminate on the canopy's first
  floor (D6 with 9 floors), then thin out on the floors above, too new for much to have grown
  (owner's direction).
- **Flower budget:** at most 24 flowers per lot and floor, at least 3 m apart (except within a
  point of interest).
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

The map is split into large zones (typically 8 to 12 lots across), each with a biome. A biome is a
`Biome` value in `src/world/biome.rs`: flower palette, an architecture `Palette` (neon, lit-window light,
facade accent panels, billboard colors, wall tint; uploaded as a uniform array and picked per lot by the
biome's index in `BIOMES`, so a new biome re-colors all the procedural materials with no shader work),
bush/vine/climber rates, and city parameters (empty-lot and bridge chances). What depends on the
floor (decks, plant density, flower chances, stairs) is in the floor profiles, not the biome.
`biome_at(seed, lot)` is pure, so zones stay deterministic and infinite. Zone sites come from a
jittered grid and a lot joins the nearest site (distance divided by the biome's `size`), so each biome
type has its own typical zone size. The spawn plaza is always the default biome.

Status: the machinery is in place with a single biome, `default` (the original look). Still to do:
more biomes (including a rare "rainbow" one with every flower color), blending across zone borders,
per-biome fog and sky tint, and per-biome plant species and tower styles.

## Set dressing (implemented, second pass)

Every lot follows one street layout, the one the pavement shader draws: a road along each border
(3 m deep on each lot's side), a curb, the sidewalk, then the tower's facades. `src/world/dressing.rs`
places each scene in the kind of place it belongs to, never at random:

- **Streets only between decks:** a lot border is a street on a floor only if the lot across it
  has a deck there; at the edge of a section the sidewalk runs to the edge instead (the pavement
  shader reads the cover map), so a road is never cut in half along its length. Curb, corner and
  parking spots and street lights only stand along real streets.
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
growing out of an armchair left by a wall, two flowers side by side on a bench (allowed closer than
the usual 3 m), a dead garden of clay pots with one living flower, an old bed frame against a wall
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
