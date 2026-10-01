# Art direction

**Core principle:** stylized 3D that reads like a 2D illustration. Shapes and color carry the image,
not detail.

## Rendering look

- **Cel shading** with 2–3 flat light bands.
- **Outlines** (inverted hull or edge-detection post-process).
- **Flat or lightly textured surfaces;** a subtle paper or grain overlay is welcome.
- **Colored atmospheric fog:** distance fades into tinted gradients rather than gray, creating
  painted depth layers and hiding the draw distance.
- **Limited palettes** per mood, with strong color grading.
- **Atmospheric particles:** drifting spores, pollen haze, mist.
- **Chrome vs. life:** cold reflective metal and neon against soft, glowing organic forms. This
  contrast is the visual identity.

## Day/night cycle

Time of day evolves continuously. Sun direction, sky, fog color, palette and grading all follow it,
interpolating smoothly between key phases:

- **Dawn:** cool, pale haze; neon fading, plants dimming.
- **Day:** warm, hazy layers of chrome and overgrowth fading into ochre or teal fog.
- **Dusk:** saturated warm-to-violet transition; the first plants and signs light up.
- **Night:** dark fog; bioluminescence (cyan, green, soft magenta) and neon become the only light.

Plant glow intensity follows the cycle (faint by day, strongest at night). The full cycle duration
is a single tweakable parameter, default ≈ 20 minutes.

## Light plants

Light plants are actual light sources affecting their surroundings. There are many small lights, so
lighting must scale (e.g. clustered/tiled forward or deferred light accumulation). The dev GPU is an
Intel Arrow Lake-U iGPU; target a smooth framerate on a mid-range GPU.
