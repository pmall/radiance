# Lore

Told by the owner, then extended in a brainstorm (`docs/story.md`, the *Last Radiance* story). It is not shown in the game (no text, no UI); it exists to
guide decisions about what the city looks like and sounds like.

- **Humanity is long gone.** The city is empty of people for good. Why they disappeared stays a
  mystery.
- **Machines kept building.** The player is the consciousness of one of them, **Wick**, a search
  unit: a meta presence inside the machine that Wick cannot explain (see `docs/story.md`). The
  game's big moment is Wick panicking and breaking the fourth wall to address the player (the
  "cold stop"). That is why the city keeps getting
  taller: construction never stopped. In the brainstorm the machines follow one directive, **"no one
  left in the dark"**, which feeds itself: every new floor lights the floors above and shadows the
  ones below, which then need more canopy, so more building. The rule can never be satisfied. (Under
  it, a hidden goal: building toward a machine paradise in the sky, "Nirvanary".)
- **Each glowing flower is a human soul.** In the brainstorm: every flower grew from a human death,
  so the light of the city is the light of the people who are gone. Flowers are the only light
  sources, and the note a flower plays when approached (see `docs/audio.md`) is that soul's voice.
- **Deeper is older.** The lowest floors are the original human city, each layer above built later
  on top of the previous one. The bottom is the original ground (soil, a cut tree stump: the city
  started with a tree cut down to make a home).
- **Humanity rose with the city.** As machines built upward, people moved up to the new levels and
  abandoned the depths long before the end. So the depths are the oldest and emptiest part. This is
  why there are more flowers (souls) high up than deep down, which is also the biologically accurate
  direction (sunlight). It matches the flower chances of the floor profiles in `src/world/design.rs` (canopy > middle > depths).
- **Above the last humans, machines built for machines.** The newest floors were built after humans
  vanished. Owner's call: flowers behave like real flowers (they follow sunlight) and are, as a
  metaphor, the souls of the dead, so whether a few grow at the very top is a detail. Leaning: keep
  the construction front (bare skeletons, cranes) free of flowers, because it shows the building
  never stopped.
- **Flowers share light.** Canopy flowers absorb sunlight and pass it down through a living network
  of glowing veins and roots to the flowers in the depths. Where the network breaks, it is dark.
- **Craters** open where nothing was built above, and sunlight pours down into the dark floors.
- Plants overgrow everything: nature took the abandoned city back.

## What it means for the graphics (working notes)

- Lit windows and neon are the machines' doing: building systems still running on automatic power.
  Keep them plain, cold or warm white, sparse and mechanical, never "someone is home". Neon signs are
  old advertising still powered.
- The city is still being built. *(done: some towers end in a bare skeleton of slabs, columns and
  rebar with a frozen tower crane on top, `src/world/construction.rs`.)* Still possible: scaffolding
  on walls, stacked materials. Everything stays still (no moving machines on screen, see
  `docs/vision.md`), as if paused or seen between shifts. Visual reference for the construction
  front: Bruegel's *Tower of Babel*.
- Flowers are souls, so they should feel special next to ordinary plants: they are the only things
  that glow and the only things that sing. Their density follows altitude (many in the canopy, few in
  the depths), not building details.
- **Flowers in human places**: a flower growing out of an armchair, two side by side on a bench
  about the distance two people sit, one alive in a dead vegetable garden, one alone at a closed
  door. Odd details, never clues; the most direct way the render can say "flowers are people".
- The depths are the oldest, longest-abandoned layer: most decayed, darkest, with little light for
  plants, so little vegetation and few souls, and each one stands out. Older human architecture
  (stone, arches, cobblestones, tram tracks) with the foundation pillars of later towers punching
  through. The upper levels are the most recent human world, brighter and better kept, with the
  most souls.
- The tall, newest parts are machine-built, so they can be colder and more regular, while the human
  touches (shops, signage, balconies, fire escapes) belong to where people lived. Machine
  intrusion into human spaces (columns through roads, struts welded across facades, cables) grows
  with height.
- The sun is what people below lost: drawn, painted on ceilings, photographed, sold back as
  daylight lamps. Shafts of real sunlight through a crater are the strongest image the city has.
