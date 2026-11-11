# World and generation

The design rules of the city: what is generated and why. Where each rule lives in the code:
`docs/architecture.md`. How hand-written chunks are written and placed: `docs/chunks.md`.

## Setting

A vast megacity where chrome, concrete and human-made infrastructure form a rigid skeleton, and
organic, engineered growth has spread over and between it: vines, roots, fungal masses, glowing
flora. Nature and technology are intertwined, not opposed. A subtle touch of the fantastic is welcome.

**Empty:** no inhabitants, no vehicles, no movement except the living world (swaying plants,
drifting particles, dripping water). The story behind it: `docs/lore.md`.

## Procedural and deterministic

- One seed generates one city: layout, architecture, plants and flowers. Every player gets a
  different city (owner: a hard requirement), and the scale is the megacity's atmosphere.
- Hand-written chunks decide how a place is organized; the seed decides which chunk goes where,
  turned how, with which slots filled, and grows every plant and flower at random on top
  (`docs/chunks.md`, "Why").
- Textures and other assets only decide how surfaces look; they never replace generation.

## The floor stack

The city is a finite stack of floors (`src/world/floors.rs`), 9 by default (`--floors N`), 36 m
apart, from the original ground (D0) to the construction front above the top deck. The player
starts on D2 with two older floors below. The stack has three zones, each recognizable without a
depth meter (architecture, materials, density, vegetation and sound all change):

- **Depths** (D0 to D2): the old human city, dark streets lit almost only by plants.
- **Middle** (D3 to D5 with 9 floors): the floors people moved up to; dimmer, scattered
  bioluminescence.
- **Canopy** (the top half of the floors above the depths): built by machines after the humans;
  towers and walkways in daylight haze, airy and readable.

**Floor profiles** (`src/world/design.rs`): every generation number that depends on the floor
lives there and nowhere else: decks and holes, plant density, flower chances per source, stairs,
props, points of interest, debris, blocks and fillers. They are written per zone and interpolated
between, so they hold for any number of floors. The owner wants full control over flower
probabilities on every floor and over every prop's placement logic and probability; `--report`
measures what a seed actually produced against the profiles (`docs/tools.md`). Props and points
of interest can be switched off (O / I, `--no-props`, `--no-pois`).

## Structure

Every lot of every floor is a chunk placed by the planner (`docs/chunks.md`). On top of the
chunks, these rules hold:

- **Holes:** openings in the floors are hole chunks (light wells, collapses, gaps, the unbuilt
  edge of the construction front); a hole opens its floor and every floor above, so holes line up
  from floor to floor like light wells and are open to the sky. Rare in the depths, common at the
  construction front (owner: holes must look consistent, few low down).
- **Light through the floors:** the renderer keeps a cover map (`render/cover.rs`) of which decks
  stand over each lot and how much sky it sees; the shaders block the sun with it at any distance,
  dim the sky light and the fog by the sky view, and light the haze where the sun gets through.
- **Craters:** round openings with nothing built above a floor (a middle floor, sometimes D2),
  whose floor is a sunlit garden of rubble. Plants grow there as lushly as in the canopy, flowers
  only a little denser than on their floor. A wide one always opens on D2 in front of the spawn,
  which faces it.
- **Stairs:** switchback stairwells (straight flights back and forth around a central wall,
  landings at each turn, rails on the open sides), each linking one floor to the next only, in the
  middle of a stair hub chunk (owner: no spiral stairs, no stair spanning several floors). Stairs
  are rare and grow rarer going up, so climbing gets harder and harder; not every part of a floor
  has one, and going down to reach another is part of the climb (owner). One always stands beside
  the start. How the player climbs beyond stairs is not built yet (`docs/game-ideas.md`).
- **Footbridges:** most gaps of one or two lots between two decks of a floor (wider would be
  physically unlikely), over a hole or a crater, get a footbridge along the lots' middle line: a
  metal walkway with railings and rusted girders underneath. It always lands on a deck at both
  ends; what stands on a landing gives way. Bridges add ways across a floor, which matters since
  stairs are rare; the owner welcomes more of them.
- **Bridges never cross** (owner): where two gaps cross, the seed ranks them and only the first
  may be built, and a skybridge passing through or just over a walkway takes the gap's place.
- **No corner contacts:** two decks never meet by a corner alone (owner: unnatural); one of them
  is left open, the seed picking which, unless the plan needs it. The gap is often crossed by a
  footbridge.
- **Roads** (owner: roads must make sense; `src/world/roads.rs`): a road runs along a lot border
  only where it can be a real street: decks on both sides not paved over by a chunk and not one
  building, and full crossings at both ends. So no road runs into a hole, the edge of a deck or a
  square, and none is half a road: the lots along an edge keep their sidewalk, the last road one
  lot back. Dead ends are cut back and tiny isolated networks removed.
- **Skybridges** between neighboring towers land only on solid walls, at both ends.
- **Construction front:** some towers rise past the top deck, up to the machines' tallest spires;
  only those can be unfinished frames with cranes, or lean on a taller neighbor. They are drawn far
  beyond the loaded lots, out to about a kilometer, as a skyline in the haze.

## Districts and places

A city, not a housing estate (owner):

- **Low-rise districts** where towers are rare alternate with tower clusters, so the city has
  built-up streets of row houses along every street side as well as towers.
- **Special places** (a fountain square, a shrine court, a playground, a bus terminal, an
  open-air theater...) are common enough to meet often, on every floor, and only where no tower
  rises through their floor.
- **Landmarks** (the old quarter of the depths, a station, a stadium, a mall, the machine works of
  the canopy, towers that are not housing or not whole) are met about every kilometer.
- **Chunks are big** (owner: "don't be cheap on chunk size"): a great square is 10x10 lots; one-lot
  chunks only for repetitive housing.

## Growth

- **By floor:** plants and flowers rise with the light and culminate on the canopy's first floor
  (D6 with 9 floors), then thin out on the floors above, too new for much to have grown (owner).
  The lore behind it: `docs/lore.md`.
- **Flowers grow freely:** close together where they happen to, only kept from growing into each
  other (owner: no minimum distance), up to a budget per lot and floor (the lights the renderer
  can afford).
- **Flowers per plant** (owner): a bush has one flower at most, on a stem rising from its middle;
  a hanging vine has none, one or several down its length; a wall climber may flower at every
  step. Bushes without flowers are fine (flowers are the special part); keep it easy to tune.
- **Everything random is random** (owner): stairs, props and every other surface grow plants and
  flowers by chance at the usual rates; chunks only hint where growth is denser or bare.
- **Plants never traverse blocks** (owner): no bush rooted in a wall, no leaf through a wall
  (neighbor lots included), and a flower's light stays between the deck under it and the deck
  over it.

## Streets, props and points of interest

Every lot follows one street layout: a road along each road border, a curb, the sidewalk, then
the buildings. Street and filler chunks place what stands in it, and placement is smart (owner):
every prop where it belongs in a street.

- **Street lights:** laid out like a real street, one per crossroads, on a sidewalk corner, its
  arm reaching over the crossing, always facing the road; each street keeps its lights on one side.
- **Props** stand where they belong: furniture behind the curb facing the road, leftovers against
  the facades, never anything in the road. No indoor furniture out in the street except as a story
  (someone moving out, a shop emptied); no construction props in the depths. Props stay few (owner:
  far too many before) and thin out going up.
- **Points of interest with flowers** (flowers in human places, `docs/poi-catalog.md`): an
  armchair with a flower growing from the seat, two flowers side by side on a bench, a dead garden
  with one living flower, a bed frame with a flower where someone slept. Rare on purpose (owner):
  too many would spell out that flowers are souls. The armchair always stands on the start plaza,
  facing the crater.
- The decoration still to come (streets cluttered, props in every condition, small scenes):
  `docs/poi-catalog.md`, "Decoration".

## Band character

Ideas from the brainstorm (`docs/game-ideas.md`, `docs/lore.md`) for making each zone its own
place:

- **Deeper is older.** The depths are the original human city (stone facades, arches, cobblestones,
  tram tracks, metro entrances, dry fountains, stopped clocks), broken by the foundation pillars
  of the towers above and by roots lifting the pavement. So far the old quarter landmark carries
  it; the rest of the depths shares the common look.
- **Higher is more machine.** Human-scale stairs and walkways low; colder, regular, machine-built
  structures high, with fewer human-scale routes and wider gaps. Machine intrusion into human
  spaces (columns through roads, struts across facades, cable bundles) grows with height. Not
  built yet.
- **Light-network veins:** glowing roots running down walls and across decks, carrying light from
  canopy flowers to those below. Not built yet.

## Biomes

The map is split into large zones, each with a biome; a biome type has its own typical zone size,
and the spawn is always in the default biome (`src/world/biome.rs`, a pure function of seed and
lot). A biome is fully configurable, not only colors and rates (owner).

A biome is a `Biome` const in `src/world/biome.rs` (start from `..DEFAULT`), listed in `BIOMES`,
and a folder `assets/biomes/<name>/`. Between them, a biome sets:

- **Its places:** its chunks and modules (`docs/chunks.md`, "Biomes"). Towers are chunks, so their
  shapes and elevations come with it; footbridges and skybridges are built from its modules too.
- **Its props:** a module in its folder named like a prop (`bench`, `bin`, `street_light`...)
  replaces that prop in its zones. Props it does not write stay the shared ones, drawn in its own
  textures.
- **Its plants** (`Plants`): flower and foliage colors, how many bushes, vines and climbers, and
  their shapes: each kind a weighted list of species, plus the flowers' shape and light. What
  depends on the floor stays in the floor profiles.
- **Its air** (`Atmosphere`): sky, fog, key light, ambient light and grading through the day
  (`Looks`, in `render/look.rs`), the colors of pollen and spores, and its color script by height
  (`docs/art-direction.md`, "Light under the decks"). The sky is drawn in the atmosphere of the
  player's biome and fades into the next one's over a few seconds at a border.
- **Its textures** (`Textures`, in `render/pixels.rs`): its own drawing of every chunk material,
  facade unit and city surface (one row of the atlas per biome, so the same chunk reads
  differently from one biome to the next): shades, patterns, and how worn everything is (dust,
  litter, puddles, moss, damp), which the shader draws over every surface. Lot meshes and props
  carry their biome, so a prop is drawn in the textures of the zone it stands in.
- **Its colors** (`Palette`): the paints of its tower walls and the tint of its pavement.
- **Its city numbers:** how often a lot has no tower, how often towers get a bridge.

Biomes now: `default` (the original city, Neon Garden in `docs/biome-ideas.md`) and `grotto`
(Cyan Grotto, about a third of the zones, `docs/biome-ideas.md`). A new biome comes only when the
owner asks for one (`docs/chunks.md`, "Catalog rule").

**Not yet:**

- **Biomes through the floors** (possible future development, owner; not started): a biome's zone
  stays a column through every floor (same map on every floor, so the city stays consistent), but
  the biome can look different in each zone of the stack, depths, middle and canopy, for a
  vertical evolution: the grotto flooded and choked with algae deep down, its bathhouse district
  in the middle, bleached and salt-crusted near the top. A biome does not have to evolve, but it
  is better for the game's consistency that most do, the default city included (older
  architecture deep, brick and stone and heavy rust, newer up high). Shape agreed so far:
  - A zone only overrides what differs from the biome's base (textures, wall paints, sky and fog,
    plant species and colors); a biome without overrides looks the same at every height. Plant
    density already follows the floor profiles, and a biome's chunks can already be kept to zones.
  - The switch happens at the decks between zones, where the player cannot see across; the sky
    fades as at a zone border when they take a stair into another zone.
  - Towers crossing a zone line change textures at that floor: an older base under newer
    stories, as the machines kept building.
  - The renderer needs room for up to three looks per biome (atlas rows, palettes, the mesh's
    material code).
- **New kinds of plants:** species vary bushes, vines and climbers widely, but a mushroom or a tree
  is new code in `world/growth.rs`, written with the biome that needs it. Likewise a new kind of
  texture pattern is new code in `render/pixels.rs`.
- **Borders** (can wait, owner): palette, plants and density flip at the zone line, the far side
  of a border is drawn in the player's own atmosphere, and an area picked in one biome is colored
  lot by lot, so it can change colors at the border.
- **Settings as text:** biome settings are Rust constants and take a rebuild (chunks and modules
  hot-reload).
- **Limits:** at most `MAX_BIOMES` biomes (the shader's palette and the atlas's rows); tower
  heights come from the floor (`design::TOWER_HEIGHTS`), not the biome.
- More biomes, including a rare "rainbow" one with every flower color, and the grotto's own chunks.
