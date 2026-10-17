# Art direction

> **Changing:** the owner chose a new target, an abandoned *Nivalis* (box geometry, pixel-art
> textures, dense modular facades). See `docs/chunks.md`. After its style test: pixel-art materials on
> hand-written chunks, with the ink outlines and flat light bands kept on (owner); pixel or voxel
> foliage is a suggestion still open. The sections below describe the procedural
> look, which the chunks replace step by step.

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
across; modern street lights (dead, only flowers glow) at the crossroads.

Working method: get the look as far as possible with code, then add a layer of real photo detail.
The second layer is `assets/textures/detail.png` (CC0 concrete, asphalt, wall, metal), sampled in the
scene shader in world space and multiplied over the procedural colors.

### Props and external assets: consistency rules

*(Implemented: `src/world/props.rs`, `assets/modules/props.ron`, `src/render/props.rs`.)*

Two passes with external models were rejected: stylized Kenney kits ("too baby-like"), then
photoscanned Poly Haven models (realistic, but foreign to the city's look; removed). Props are now
**box-built modules** written as text, like the chunks (`docs/chunks.md`), in the same pixel-art
materials, so a bench and the square it stands in come from one hand. The street light alone is
a smooth mesh built in code (a box pole and arm read poorly). Rules:

- **One language:** props use the chunk vocabulary and materials; nothing imported.
- **Same rendering:** props go through the scene shader like every block: same cel bands,
  outlines, shadows, fog, grain and flower lights. Drawn instanced, one mesh per prop.
- **Human scale:** real sizes (a bench seat at 0.45 m, an armchair 0.9 m); their size is what
  makes the architecture feel immense.
- **Seed places, module shapes:** what appears where comes from the seed (`dressing.rs`); a prop
  gets a simple box for collision (a pole only, for street lights).
- **Review:** every new prop is checked with `--shot` renders, day and night.

## Day/night cycle

Time of day evolves continuously. Sun direction, sky, fog color, palette and grading all follow it,
interpolating smoothly between key phases (each biome has its own looks for them, the default
biome's described here; `docs/world.md`, "Biomes"):

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
