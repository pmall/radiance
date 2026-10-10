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

### Props and external assets: consistency rules

Points of interest and set dressing (`docs/world.md`, `docs/poi-catalog.md`) need far more objects
than code alone can model well, so props come from free external model kits. The owner leaves these
rules to the project agent. Goal: a prop from a kit must look painted by the same hand as the
procedural city. The pipeline, not the asset, guarantees that.

- **Sources.** CC0 first (e.g. Kenney, Quaternius, Poly Haven, ambientCG); anything else only with a
  license that allows it and a credit in the asset folder's README. Prefer **low-poly, stylized
  kits** with clear silhouettes over photo-scanned models: flat shapes read as illustration, dense
  scans read as photos and clash.
- **Geometry only.** A kit's own textures and colors are never shown. Each material slot of a model
  is mapped, in a plain-text table, to a **material family** (metal, concrete, glass, plastic,
  fabric, paper, wood, vegetation), with a light or dark shade.
- **One palette.** Each family has a small color ramp; the existing procedural block colors move
  onto the same ramps when the first props land, so city and props share one palette. The biome
  `Palette` and the depth band shift the ramps slightly (cold and dim deep down, warmer near the
  canopy).
- **Detail.** Surface texture comes from the same grayscale photo-detail layer as the walls,
  sampled in world space at the same scale, with capped contrast: a small prop is neither sharper
  nor blurrier than the wall behind it. (If photo detail ever reads as "photo" on props, switch it
  from a multiply to a gradient map onto the family ramp.)
- **Same rendering.** Props go through the scene shader like every block: same cel bands, outlines,
  shadows, fog, grain and flower lights. No baked or unlit kit lighting.
- **Human scale.** Props are scaled to real sizes (a door 2.1 m, a bench 0.45 m high); their size
  is what makes the architecture feel immense.
- **Seed places, asset dresses.** What appears where always comes from the seed; a prop gets a
  simple box for collision; many copies are instanced.
- **Review.** Every new prop or kit is checked with `--shot` renders, day and night, next to
  existing architecture, before moving on.

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
