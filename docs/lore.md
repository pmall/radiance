# Lore

Told by the owner. It is not shown in the game (no text, no UI); it exists to guide decisions about
what the city looks like and sounds like.

- **Humanity is long gone.** The city is empty of people for good.
- **Machines kept building**, and the player is one of them. That is why the city keeps getting
  taller: construction never stopped, nobody ever told it to.
- **Each glowing flower is a human soul.** Flowers are the only light sources, and the piano note a
  flower plays when approached (see `docs/audio.md`) is that soul's voice.
- **Humanity rose with the city.** As machines built upward, people moved up to the new levels and
  abandoned the depths long before the end. So the depths are the oldest and emptiest part, and the
  top is where humans lived last. This is why there are more flowers (souls) high up than deep down,
  which is also the biologically accurate direction (sunlight). It matches `bloom()` in
  `src/world/biome.rs` (canopy > middle > depths).
- Plants overgrow everything: nature took the abandoned city back.

## What it means for the graphics (working notes)

- Lit windows and neon are the machines' doing: building systems still running on automatic power.
  Keep them plain, cold or warm white, sparse and mechanical, never "someone is home". Neon signs are
  old advertising still powered.
- The city is still being built. Good signs of that: cranes and scaffolding on roofs, unfinished
  upper floors with an open structure, stacked materials, tall masts. They stay still (no moving
  machines on screen, see `docs/vision.md`), as if paused or seen between shifts.
- Flowers are souls, so they should feel special next to ordinary plants: they are the only things
  that glow and the only things that sing. Their density follows altitude (many in the canopy, few in
  the depths), not building details.
- The depths are the oldest, longest-abandoned layer: most decayed, darkest, with little light for
  plants, so little vegetation and few souls, and each one stands out. The upper levels are the most recent human world, brighter and
  better kept, with the most souls.
- The tall, newest parts are machine-built, so they can be colder and more regular, while the human
  touches (shops, signage, balconies, fire escapes) belong to where people last lived, high up.
