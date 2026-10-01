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
- Geometry is generated in code; no external modeling pipeline. Free low-poly kit placeholders only
  if truly needed.
