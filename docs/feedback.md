# Owner feedback and decisions

What the project owner has said so far. Treat as requirements.

## Goal and rules

- The only goal is a great final render, rather than code purity or a particular method.
- World generation is always procedural: seeded, random, infinite, extensible, deterministic.
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

## Controls

- The owner uses an AZERTY keyboard: movement works with ZQSD, WASD and arrow keys.
- Arrow keys move the player, so time controls use Home/End (scrub) and PgUp/PgDn (speed).

## Scope

- Audio (piano notes, ambient soundscape) is skipped for now, on request.
- The first full playthrough was judged "very impressive"; the next work is milestone 8
  (visual quality pass, see `docs/roadmap.md`).

## Working notes

- No per-machine or per-session memory: the owner switches computers, so everything that must
  persist lives in this repository.
