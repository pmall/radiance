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
- Flowers: denser in the sunlit canopy, sparser in the depths (where each glow stands out).
  Plants and flowers culminate on D6 (the canopy's first floor), then are less present on D7 and
  D8. Flowers keep at least 3 m apart.
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
  harder to find going up. Spiral stairs did not fit the city: stairs are straight switchbacks
  ("Z" stairs) with landings and rails.
- Bushes without flowers are fine (flowers are the special, glowing part); the owner was unsure,
  so keep it easy to tune.
- Craters: lush plants, but flowers only a little denser than on their floor (owner questioned
  canopy-level flowers there; it was not a directive).
- The debug overlay must be readable by a non-specialist: plain words, the floor shown large.
  Floors are counted from 0 like the D names (D7 = floor 7 of 8); directions use clock hours.
- Stairs are generated as in the real game, even when testing. Rare stairs high up are fine; the
  debug panel must still show the nearest one, however far. Test mode (G, on by default)
  helps find things instead: the overlay gives the distance and clock direction of the nearest
  stair up, stair down and crater, and the right mouse button makes Wick 4 times faster.

- Props: the Kenney kits looked like a cheap mobile game ("too baby-like"); the photoscanned Poly
  Haven models that followed were removed at the owner's wish; props are now box-built modules in
  the chunks' style. What matters most is smart placement: every prop where it belongs in a
  street (no bench in the road), no construction props in the depths, no indoor furniture out in
  the street without a story. Darkness of the depths: keep it, the owner loves it.

- After the generation refactor: holes in the floors looked too randomly placed (wanted: more
  consistency, fewer holes in the lower floors; many holes at the top are fine, it is under
  construction); street lamps too many and too random (wanted: evenly spaced along the roads,
  exactly at the corners of crossroads, always facing the road); stairs only got flowers on their
  landings (wanted: stairs as part of the world, random bushes and flowers on them); props still
  toy-like (wanted: realistic assets). Done: decks by sections built up to a floor, one street
  light per crossroads on a regular layout, plants on stairs at the usual rates, photoscanned
  Poly Haven props.

- On that pass: decks "way better". Still wrong: a road cut in the middle along the edge of a deck
  section; far too many props overall (fewer for now), cars and hydrants judged horrible and far
  too frequent, cars should not be covered; stairs seemed common then dropped suddenly around D7;
  plants and flowers did not feel like they culminate on D6. The owner is not satisfied with the
  props overall and has no direction yet. Done: no road where there is no deck across, cars and
  hydrants removed, props cut to a third, a smooth stair curve, growth peaking on D6.

- Direction after that pass: fully random placement cannot make places look organized (a square
  needs symmetry). Build the city from many large hand-written chunks (text), labeled and placed
  by the seed with rules (squares far apart, one stair in the middle of a stair chunk), with
  vegetation and flowers still random: the random light sources keep exploration unpredictable.
  Style target: *Nivalis* (box/voxel geometry, pixel-art textures, dense modular facades),
  abandoned, with flowers as the only light. Think the whole system ahead, test where it matters.
  Reference screenshots stay out of git. Plan: `docs/chunks.md`.

- Style test on the great square (pixel-art materials): "this is all really good, keep going".
  Keep the outlines and flat light bands on (the agent had wrongly read "keep going" as approval
  of its suggestion to turn them off: only the owner decides). Pixel or voxel foliage: suggested,
  not decided.
- Stairs should be rare. Not every walkable island needs a stair: going down and up another one
  is fine. Bridges over gaps of a bridge's size (not so short it makes no sense, not so long it is
  physically unlikely) would add to the ambiance. Chunks should be big ("don't be cheap on chunk size": a big
  square might be 10x10 lots; 1x1 only for repetitive housing).

- After phases 4 to 6 ("this looks amazing"): the goal restated: many chunks defined by text,
  each structured by hand, not randomly, then assembled randomly; chunks so far only sit on top
  of the old random city. Next: everything is a chunk, towers first (plan in `docs/chunks.md`).
  Props built from boxes in text are liked: make as many as needed. Tests must stay fast.
  Explain status in plain words, simply.

- Biomes: in the end, a set of chunks and props per biome; a biome is a meta collection of chunks,
  today's city the default one (done: `assets/biomes/<biome>/`). From now on no chunks or props
  are created on the side: when the owner wants more, they ask for a new biome.

## How to work on generation (owner's direction)

- Think of the game as one consistent system, not fix after fix. The code must be as consistent
  as the design: modular, every rule in its place.
- Perfect control of flower probabilities on every floor; control of every prop's placement logic
  and probability; props and points of interest can be switched off; the number of floors is
  fixed per run but variable (more may come, depending on how hard climbing is).
- Flowers on benches and chairs must stay rare: too many make it obvious they are human souls.
- Everything random is random: stairs grow plants and flowers by chance, like any other spot.
- Implemented as `world/design.rs` (floor profiles), `--report`, O/I switches, `--floors N`
  (see "How the city is generated" in `docs/world.md`).

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
