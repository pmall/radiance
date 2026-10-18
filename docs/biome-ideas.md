# Biome ideas

Inspiration only, except Cyan Grotto (implemented, see below). The machinery is in
`src/world/biome.rs` (see the Biomes section of `docs/world.md`: what a biome can set). Each idea is described by what it changes there: flower palette,
plant and flower density, bush/vine/climber rates, and city parameters. Size is the typical zone
size relative to the others.

Palettes, plant mix, city parameters, plant species, props, per-biome sky and fog and per-biome
textures (shades, patterns and wear of the pixel art) are all settings now; new kinds of plants and
new kinds of texture patterns are the expensive part.

From the brainstorm (`docs/game-ideas.md`): the height band should set the broad ecology (sparse
and fungal deep, dense and flowering near the sun), with some biomes only above or below a given
level. The owner's direction instead: every biome stays a column through all floors and can evolve
by section (depths, middle, canopy), the default city included (`docs/world.md`, "Biomes", "Not
yet"). The rainbow biome is extremely rare, possibly unique per city, and in the story it is where
the flowers' faintest melody leads.

## Ideas

### 1. Default: "Neon Garden"

The current look: cyan and magenta flowers with some green, violet and amber, mixed towers and
bridges. Most common and mid-sized, the baseline the others contrast with.

### 2. Cyan Grotto

- **Light:** almost all cyan and teal, with the occasional white.
- **Plants:** dense hanging vines and low ferns, few tall plants.
- **City:** more empty lots, fewer bridges.
- **Mood:** cold, calm, aquatic. Pairs well with a teal fog tint.
- **Implemented** (October 2026, `GROTTO` in `biome.rs`, `assets/biomes/grotto/`): cyan, teal and
  white flowers; low ferns and moss tufts, curtains of leafy vines three times as common, few
  climbers; a teal fog and sky at every hour (`look::GROTTO_LOOKS`, a cold teal sunset); tiled
  benches, bins, planters and street lights; a third of the zones, more empty lots, fewer
  bridges. Its places are still the default chunks (`include: ["default"]`), drawn in its own
  textures (`pixels::GROTTO_TEXTURES`): pale limestone and basalt setts with moss in the joints,
  glazed teal bricks, a mosaic of aqua and white tiles, verdigris for rust, silvered driftwood,
  tiled towers in teals and bone white with round-headed windows misted over, algae running down
  from the ledges.

### 3. Magenta Bloom

- **Light:** pink, magenta and violet.
- **Plants:** lush, many flowers at every height, including the depths.
- **City:** tight and tall.
- **Mood:** warm, overgrown, romantic.

### 4. Amber Ember

- **Light:** orange, amber and gold, with a few red flowers.
- **Plants:** sparse and dry-looking, few vines, more bare concrete.
- **City:** mostly ruined, fewer decks, many open gaps.
- **Mood:** lit by embers, almost a dying city. Needs a warm fog and sky tint to work.

### 5. Green Overgrowth

- **Light:** acid green and lime, a little yellow.
- **Plants:** the densest growth, towers almost swallowed, very high climber and vine rates.
- **City:** many stairs and bridges so you can climb through it.
- **Mood:** jungle.

### 6. Violet Deep

- **Light:** violet, indigo and rare blue.
- **Plants:** few flowers, each bright and isolated, large gaps between lights.
- **City:** very tall towers, narrow streets, many decks.
- **Mood:** vertical and dark, the glow stands out more than anywhere else. The cleanest test of
  the "lights only from flowers" idea.

### 7. Rainbow Garden (rare)

- **Light:** every flower color, evenly weighted.
- **Plants:** moderate growth with a high flower count, so the mix of colors shows.
- **Size:** small and rare, finding one feels like an event.
- **Mood:** celebratory. The cap of 24 flowers per lot means it needs a high flower density to
  read as rainbow.

### 8. Dead Zone

- **Light:** almost none, a few white or pale flowers.
- **Plants:** bare concrete, withered stems.
- **City:** the plain skeleton, large empty spans.
- **Mood:** a quiet, dark pause between busy biomes; makes the lit biomes feel brighter by contrast.

### 9. Sunken Market (layout biome)

- **Light:** mixed warm colors.
- **City:** many close decks and bridges, small blocks, parapets and stairs everywhere, very few
  tall spires.
- **Mood:** a dense, walkable maze, fun to explore on foot.

### 10. Spire Field

- **Light:** cool white and ice blue.
- **City:** widely spaced, very tall thin spires above the canopy, few decks in between.
- **Mood:** sparse and high, wide sky view from the top. Fits "airy and readable" for the upper city.

## Needing more work

- **Plant species:** hanging lanterns, tall reeds and fungus caps each need new geometry.
- **Fog and sky tint:** what makes biomes like Amber Ember and Cyan Grotto read from afar.
- **Border blending:** a smooth fade between very different biomes, e.g. Amber Ember next to
  Cyan Grotto.

## Suggested first set

Cyan Grotto, Magenta Bloom, Green Overgrowth and Rainbow Garden: palettes and density rates are
enough, no new geometry, and they cover very different moods.
