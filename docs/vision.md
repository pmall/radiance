# Vision and scope

Explorable artistic proof of concept: a procedurally generated, infinite cyberpunk megacity
overgrown by bioluminescent plant life, rendered in 3D but reading like a **2D illustration**.
The player moves freely through it in first person.

**The final render is the goal.** The world is always procedurally generated from the seed; how its
surfaces are dressed (procedural shaders, authored textures, external assets) is secondary to how
good it looks.

It's a mood and rendering prototype: **no gameplay systems** (no objectives, no NPCs, no UI beyond
debug tools).

A possible future, brainstormed by the owner: a short narrative exploration game, *Last Radiance*,
built on this city (`docs/story.md`, `docs/game-ideas.md`). It is inspiration for the mood and the
world, not a change of scope.

## Definition of done

- Launch → seed-generated city → walk through it in first person.
- The city has real vertical depth: a finite stack of floors, from the original ground to the
  construction front. It streams endlessly in every horizontal direction (kept unless it gets in
  the way of more important things).
- The rendering clearly reads as stylized illustration, not default 3D.
- Bioluminescent plants light their surroundings and play a soft piano-like note when approached.
- Time of day evolves continuously through a full day/night cycle.
- The same seed always produces the same city.

## Guidance (future)

The city is huge and has no UI. The flowers guide the player: near a point of interest their notes
grow more consonant and form a motif that completes on arrival, never explained (the "melodic
gradient", chosen by the owner; see `docs/guidance-ideas.md` and `docs/audio.md`).

## Player

- First person, no visible body. The city should feel immense through the scale of its architecture.
- Walk, run, jump, with **generous ledge-grabbing** so vertical navigation is possible.
  Movement should feel smooth and pleasant.

## References

| Work | What to take from it |
| --- | --- |
| *Firewatch* | Colored fog, painted depth layers, limited palettes |
| *Sable* | Flat, illustrative shading of 3D scenes |
| *Gris* | Softness, melancholy, color as emotion |
| *Stray* | Dense, abandoned cyberpunk urban atmosphere |
| *No Man's Sky* | Procedural scale, sense of being small |
| *Mirror's Edge* | Clean first-person movement through architecture |
