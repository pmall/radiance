# Roadmap

Where the work stands and what comes next. The stages themselves are in `docs/vision.md`,
"Stages"; what was done and when is in git.

## Now: the city and its atmosphere

The city meets stage 1's definition of done (`docs/vision.md`). Open points:

- **To judge over time (owner):** how strong the wall wash from flowers is (`WALL_REACH`), the
  ragged leaf edges, how the low-rise districts change the feel of the city.
- **Known issue:** small bright chips flicker in the far fog of the crater.
- **Not started, from the brainstorm** (`docs/game-ideas.md`, "What the city can take now"):
  light-network veins; machine intrusion growing with height; older architecture across the
  depths beyond the old quarter (`docs/world.md`, "Band character").
- **Biomes through the floors** (possible future development, owner; not started):
  `docs/world.md`, "Biomes", "Not yet".
- **Live palette tweaking:** palettes are Rust constants (`render/look.rs`) and take a rebuild.

## Next: the game

- New biomes and points of interest first, when the owner asks for them (`docs/chunks.md`,
  "Catalog rule"). Of the catalog (`docs/poi-catalog.md`), only the flowers in human places stand
  in the city so far; the other objects (the notice board courtyard, the vending machine, the bus
  stop blocked by a column, the phone booth...) are to build.
- The melodic gradient that guides the player (`docs/guidance-ideas.md`).
- Then the story, Wick's orders and the radio dialogue, progression between floors
  (`docs/story.md`, `docs/game-ideas.md`).

## Last: voices

Text to speech for the dialogue (`docs/vision.md`, "Stages").
