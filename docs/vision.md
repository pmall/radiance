# Vision and scope

Explorable artistic proof of concept: a procedurally generated, infinite cyberpunk megacity
overgrown by bioluminescent plant life, rendered in 3D but reading like a **2D illustration**.
The player moves freely through it in first person.

**The final render is the goal.** The world is always procedurally generated from the seed; how its
surfaces are dressed (procedural shaders, authored textures, external assets) is secondary to how
good it looks.

It's a mood and rendering prototype: **no gameplay systems** (no objectives, no NPCs, no UI beyond
debug tools).

## Definition of done

- Launch → seed-generated city → walk through it in first person.
- The city streams endlessly in every horizontal direction and has real vertical depth.
- The rendering clearly reads as stylized illustration, not default 3D.
- Bioluminescent plants light their surroundings and play a soft piano-like note when approached.
- Time of day evolves continuously through a full day/night cycle.
- The same seed always produces the same city.

## Guidance (future)

The city is huge and has no UI. Like the wind in *Ghost of Tsushima*, a guiding wind (sound first,
pollen and spores drifting the same way as its visual twin) leads the player toward points of
interest. See `docs/audio.md`.

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
