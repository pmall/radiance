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

## The floor stack (implemented)

The city is a finite stack of 9 floors, one table in `src/world/floors.rs`: D0 (the original
ground, y = -80) to D8 (the top deck, y = 216), 36 m apart above D1. The player wakes on **D2**
(y = 0): two older floors below, six above. Everything height-dependent reads the table (decks,
plant bands, the shaders' floor heights, the souls' register, the debug overlay's floor name).

- **Decks:** each lot has a deck on a floor by chance (`Biome::deck_keep`, per band), a pure function
  of seed, lot and floor (`city::deck`). Gaps open onto the floors below.
- **Light under the decks:** the renderer keeps a cover map of the lots around the camera
  (`src/render/cover.rs`): which decks exist, and how much sky each lot sees on each floor (24 rays
  traced against the decks). The shaders block the sun with it (exact, at any distance), dim the
  sky light and the fog by the sky view, and march the fog so sunbeams show through openings.
- **Craters:** round clusters of lots with nothing built above a floor (mostly middle floors,
  sometimes D2), one per 8x8 lots about half the time. Plants grow there as lushly as in the
  canopy; flowers, which follow the sun like real ones, only 1.5x their floor's usual rate. A wide one always opens on D2 right in front of the spawn, which faces it.
- **Stairs:** switchback stairwells in empty lots (straight flights going back and forth in two
  lanes split by a central wall, landings at each turn, rails on the open sides), each linking one
  floor to the next, rarer at every floor up (common in D0–D2, rare in the canopy): every floor
  needs its own stair, so climbing gets harder and harder. A flower on every landing (the souls of
  the people who climbed them) makes a stairwell read from afar as a column of lights.
  How the player climbs beyond stairs is not built yet (`docs/game-ideas.md`).
- **Flower budget:** at most 24 flowers per lot *and floor*, so the lower floors, grown first,
  don't use up the budget of the floors above.
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
biome's index in `BIOMES`, so a new biome re-colors all the procedural materials with no shader work), plant and flower density per height layer,
bush/vine/climber rates, and city parameters (empty-lot chance, deck, stair and bridge chances).
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

- **Curb:** on the sidewalk just behind the curb, facing the road: benches, bench and bin, bins,
  street signs.
- **Corner:** the sidewalk corner of two streets: traffic lights (D0–D2), signs.
- **Facade:** against a tower wall, facing out: dumpsters, barrels and crates, carts, bins and
  boxes, café terraces (D1–D3), a sofa with boxes and a lamp (someone moving out), a fridge and a
  freezer (a shop emptied), tires.
- **Parking:** in the road along the curb, nose along the street, clear of crossings: old cars on
  D0–D1, a truck on D0. Nothing else ever stands in the road (tested).
- **Vacant lot:** lots without a tower: debris, a small square with two benches facing each other.

About 10 to 15 props per lot and floor in the depths, a few in the middle floors, almost none in
the canopy. No indoor furniture out in the street except as a story (moving out, a shop emptied),
and no construction props in the depths (they belong to the construction front, not done yet).
Every prop stands fully on a deck, clear of everything else, with a collision box; plants grow
around them.

Points of interest with flowers (flowers in human places, `docs/poi-catalog.md`): two flowers
side by side on a bench (allowed closer than the usual 3 m), a flower growing out of an armchair
left by a wall, a dead garden with one living flower. The armchair and the bench always stand on
the start plaza, behind the curb before the crater, facing it.

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
