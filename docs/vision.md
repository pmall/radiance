# Vision and scope

*Last Radiance*: a conceptual artistic game. Wick, an android search unit, is sent on an absurd
mission into the depths of a vast cyberpunk megacity overgrown by plants: find the humans left in
the dark. There are none. The glowing flowers each play a piano note, and they are the souls of the
long-gone humans. The player is a presence inside Wick, pulling it upward against its orders. Story,
dialogue and the narrative escalation: `docs/story.md`; systems: `docs/game-ideas.md`; points of
interest: `docs/poi-catalog.md`.

The city is procedurally generated from the seed, so every player gets a different city, and its
scale is what makes it a megacity. It is rendered in 3D but reads like a **2D illustration**; how
surfaces are dressed (procedural shaders, authored textures, external assets) is secondary to how
good it looks.

## Stages

1. **The city and its atmosphere** (now): generation, the look, light, the flowers and their notes.
   The game must look and feel good before anything else.
2. **The game** (next): new biomes and points of interest first, then the story, Wick's orders and
   the radio dialogue, progression between floors.
3. **Voices** (last): very good text to speech for the dialogue (maybe ElevenLabs).

## Definition of done for stage 1

- Launch → seed-generated city → walk through it in first person.
- The city has real vertical depth: a finite stack of floors, from the original ground to the
  construction front, each floor vast. It streams endlessly in every horizontal direction today,
  but endlessness is not a goal: bounded floors are fine if the game needs them.
- The rendering clearly reads as stylized illustration, not default 3D.
- Bioluminescent plants light their surroundings and play a soft piano-like note when approached.
- Time of day evolves continuously through a full day/night cycle.
- The same seed always produces the same city.

## Guidance (future)

The city is huge and has no map or markers. The flowers guide the player: near a point of interest their notes
grow more consonant and form a motif that completes on arrival, never explained (the "melodic
gradient", chosen by the owner; see `docs/guidance-ideas.md` and `docs/audio.md`).

## Player

- First person, no visible body. The city should feel immense through the scale of its architecture.
- Walk, run, jump, with **generous ledge-grabbing** so vertical navigation is possible.
  Movement should feel smooth and pleasant.

## References

| Work            | What to take from it                                |
| --------------- | --------------------------------------------------- |
| *Firewatch*     | Colored fog, painted depth layers, limited palettes |
| *Sable*         | Flat, illustrative shading of 3D scenes             |
| *Gris*          | Softness, melancholy, color as emotion              |
| *Stray*         | Dense, abandoned cyberpunk urban atmosphere         |
| *No Man's Sky*  | Procedural scale, sense of being small              |
| *Mirror's Edge* | Clean first-person movement through architecture    |
