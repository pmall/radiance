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

## Architecture and materials

Reference: Mirror's Edge (dense city, no people). Architecture is dressed by block kind in
`shaders/scene.fs` (procedural so far): tower facades in five window styles (windows between piers,
ribbons, slits, punched grid, glass curtain wall) with storefronts and neon at street levels, colored
accent panels and vertical neon signs; paved streets with joints, cracks, dust and lane markings;
roofs with a parapet, machinery (air conditioners with fans, vents, solar arrays, skylights, pipes),
antenna masts and backlit billboards, all real blocks (`src/world/roof.rs`) so rooftops can be run
across; street lamp posts (dead, only flowers glow) along the sidewalks.

Working method: get the look as far as possible with code, then add a layer of real photo detail.
The second layer is `assets/textures/detail.png` (CC0 concrete, asphalt, wall, metal), sampled in the
scene shader in world space and multiplied over the procedural colors.

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

**Flowers are the light sources, not whole plants.** A bush, vine or climber is foliage; the small
flowers or buds growing on it emit the light. Each light sits at its flower, takes the flower's
color, has a visible glowing source (halo) and stays small and tightly tied to it, so the viewer can
see which flower casts which light.

Light plants are actual light sources affecting their surroundings. There are many small lights, so
lighting must scale (e.g. clustered/tiled forward or deferred light accumulation). The dev machine
now has an RTX 3060 (previously an Intel Arrow Lake-U iGPU); still target a smooth framerate on a
mid-range GPU, and keep the render-scale option for weaker ones.
