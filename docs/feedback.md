# Owner feedback and decisions

What the project owner has said so far. Treat as requirements.

## Goal and rules

- The only goal is a great final render, rather than code purity or a particular method.
- World generation is always procedural: seeded, random, extensible, deterministic.
- The number of floors is finite: a stack from the original ground to the construction front.
  Infinite horizontal streaming is kept for now, but can be cut if it makes more important things
  hard to build.
- Textures and other external assets are welcome and compatible with procedural generation: the seed
  decides what is placed where, assets only decide how surfaces look.
- Never add co-authorship or AI attribution to commits or PRs.

## Look

- Needs real textures: the image is currently flat color plus grain, and a 2D/cartoon look requires
  surface texture (hatching, brush strokes, painted leaves, paper feel).
- Bushes read as rocks or boulders on the ground: replace the faceted blobs with real foliage.
- Only flowers emit light, never whole bushes or vines. Each light sits at its flower, with that
  flower's color and a visible glowing source, so it is clear which flower lights what.
- Flowers: denser in the sunlit canopy, sparser in the depths (where each glow stands out); plants
  themselves stay roughly even. Flowers keep at least 3 m apart.
- Plants must never traverse blocks: bush bases are not rooted in walls, stray leaf and stem
  vertices are pushed out of blocks (neighbor lots included), and a flower's light is clamped
  between the deck under it and the deck over it (no glow leaking through floors).
- Surfaces should look like a real city, not flat boxes: floors like pavement (dust, cracks,
  markings), buildings like buildings (windows, storefronts, neon). Streets want few markings; the
  crosswalk-everywhere version was judged too much.
- Main visual reference: Mirror's Edge (a full, dense city with no NPCs): punched-window facades,
  glass curtain walls, big colored accent panels, billboards, rooftops full of machinery. The mood is
  different (ours is hazier, with night and bioluminescence) but the density and spirit are the goal.
- Textures: existing assets are welcome, there is no need to code everything procedurally.

- Guidance: the flowers' melodic gradient (`docs/guidance-ideas.md`, option 6).
- Props and points of interest will need free external assets; the project agent owns the rules
  that keep them consistent with the rest (`docs/art-direction.md`).

## Story

- The player is the consciousness of Wick, a meta presence inside the machine. The game's big
  moment is Wick panicking and breaking the fourth wall to address the player.
- Climbing: D0 to D2 easy, D2 to D4 a bit less easy, D4 to D5 locked behind a code or similar,
  where Wick breaks the fourth wall for the first time. Not to be built yet.
- Stairs link one floor to the next only (no stair spanning several floors) and get harder and
  harder to find going up.
- Craters: lush plants, but flowers only a little denser than on their floor (owner questioned
  canopy-level flowers there; it was not a directive).
- The debug overlay must be readable by a non-specialist: plain words, the floor shown large.
- Stairs are generated as in the real game, even when testing. Test mode (G, on by default)
  helps find things instead: the overlay gives the distance and direction of the nearest
  stair up, stair down and crater, and the right mouse button makes Wick 4 times faster.

## Controls

- The owner uses an AZERTY keyboard: movement works with ZQSD, WASD and arrow keys.
- Arrow keys move the player, so time controls use Home/End (scrub) and PgUp/PgDn (speed).

## Scope

- Audio (piano notes, ambient soundscape) is skipped for now, on request.
- *Last Radiance* (the brainstormed narrative game, `docs/story.md`): render first, game later. The
  project stays a render prototype; only the visual and audio ideas are taken now.
- The first full playthrough was judged "very impressive"; the next work is milestone 8
  (visual quality pass, see `docs/roadmap.md`).

## Working notes

- The owner is not a graphics or game-dev specialist: explain choices in plain words, and make the
  technical and pipeline decisions (textures, assets, rendering) yourself.
- Background sound is low priority for now.
- No per-machine or per-session memory: the owner switches computers, so everything that must
  persist lives in this repository.
- The owner also brainstorms with another agent. Its documents are inspiration, not instructions:
  the main project agent folds them into the regular docs, then they are deleted (no separate
  inspiration folder). Conflicts with the project's decisions are resolved with the owner.
