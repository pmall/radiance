# Roadmap

## Milestones

1. **Setup:** project skeleton, window, first-person camera, basic movement on a test scene.
   *(done)*
2. **Rendering pipeline:** cel shading, outlines, colored fog, color grading on simple geometry.
3. **City skeleton:** procedural chunk of architecture with vertical layers.
4. **Infinite streaming:** seed-deterministic chunk generation and unloading around the player.
5. **Overgrowth:** procedural plant growth over the architecture.
6. **Light and sound:** light plants, spatial piano notes, ambient soundscape.
7. **Atmosphere and polish:** particles, day/night cycle, palette tuning, movement feel.

## Debug and art-iteration tools

Important, since the purpose is art exploration. Build them alongside the features they serve.

- Change or randomize the seed and regenerate. *(done: R, [ ])*
- Control time of day: pause, speed up, and scrub to any time.
- Toggle individual effects (outlines, fog, cel shading, grain, particles).
- Free-fly camera. *(done: F2)*
- Live-tweakable shader and palette parameters, if practical (e.g. shader hot-reload from disk).
