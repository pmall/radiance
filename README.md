# Radiance

*Last Radiance* is a conceptual artistic game in Rust and raylib. Wick, an android search unit,
explores in first person a vast, seed-generated cyberpunk megacity overgrown by plants, rendered
to read like a 2D illustration, looking for the humans left in the dark. There are none: the
glowing flowers, each playing a piano note, are their souls.

## Run

```
cargo run --release -- <seed>
```

The seed is optional. Every flag and key: `docs/tools.md`.

## Docs

- `docs/vision.md`: what the game is, its stages, stage 1's definition of done, references. Read
  first.
- `docs/roadmap.md`: where the work stands and what comes next.
- `docs/architecture.md`: how the code fits together, where each part starts, which function
  holds each generation rule, the invariants. Read before touching the code.
- `docs/tools.md`: command-line flags, keys, screenshots, recordings, reference views.
- `docs/world.md`: the city's generation rules: floors, structure, districts, growth, streets,
  biomes. Read before changing generation.
- `docs/chunks.md`: the hand-written chunks the city is built from and how they are placed.
  Read before writing or placing chunks.
- `docs/art-direction.md`: the look and the owner's decisions on it. Read before any visual
  change.
- `docs/audio.md`: the souls' piano notes and the ambient sound.
- `docs/lore.md`: the story behind the city, for art and audio decisions.
- `docs/story.md`, `docs/game-ideas.md`, `docs/poi-catalog.md`: the game's design (story and
  dialogue, systems, points of interest), not built yet and still being written.
- `docs/guidance-ideas.md`: ideas to guide the player by sound, and the owner's choice.
- `docs/biome-ideas.md`: brainstormed biomes, for inspiration.
