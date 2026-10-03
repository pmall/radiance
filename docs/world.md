# World and generation

## Setting

A vast megacity where chrome, concrete and human-made infrastructure form a rigid skeleton, and
organic, engineered growth has spread over and between it: vines, roots, fungal masses, glowing
flora. Nature and technology are intertwined, not opposed. A subtle touch of the fantastic is welcome.

**Empty:** no inhabitants, no vehicles, no movement except the living world (swaying plants,
drifting particles, flickering signs).

## Verticality defines the mood

- **Upper city:** a canopy of towers and walkways in daylight haze, airy and readable.
- **Middle levels:** dimmer, lit by neon remnants and scattered bioluminescence.
- **Depths:** dark streets and old infrastructure, lit almost only by plants.

## Procedural and deterministic

- One seed generates one city. Generation happens in chunks streamed around the player.
- Every random decision derives from the seed (and chunk coordinates) through `src/rng.rs`, never
  from time or global state.
- Organic elements use growth-inspired techniques (L-systems, space colonization, noise-driven
  growth), layered on top of a generated architectural skeleton.
- The world itself (layout, architecture, plant placement and growth) is always procedural. Textures
  and other external assets are welcome to dress that geometry whenever they make the render better;
  they are materials and building blocks, not a replacement for generation.
  if truly needed.

## Biomes

The map is split into large zones (typically 8 to 12 lots across), each with a biome. A biome is a
`Biome` value in `src/world/biome.rs`: flower palette, plant and flower density per height layer,
bush/vine/climber rates, and city parameters (empty-lot chance, deck, stair and bridge chances).
`biome_at(seed, lot)` is pure, so zones stay deterministic and infinite. Zone sites come from a
jittered grid and a lot joins the nearest site (distance divided by the biome's `size`), so each biome
type has its own typical zone size. The spawn plaza is always the default biome.

Status: the machinery is in place with a single biome, `default` (the original look). Still to do:
more biomes (including a rare "rainbow" one with every flower color), blending across zone borders,
per-biome fog and sky tint, and per-biome plant species and tower styles.
