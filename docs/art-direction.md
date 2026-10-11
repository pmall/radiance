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
across; modern street lights (dead, only flowers glow) at the crossroads.

Working method: get the look as far as possible with code, then add a layer of real photo detail.
The second layer is `assets/textures/detail.png` (CC0 concrete, asphalt, wall, metal), sampled in the
scene shader in world space and multiplied over the procedural colors.

### Props and external assets: consistency rules

*(Implemented: `src/world/props.rs`, `src/render/props.rs`, `tools/import_props.mjs`.)*

Points of interest and set dressing (`docs/world.md`, `docs/poi-catalog.md`) need far more objects
than code alone can model well, so props come from free external models. The owner leaves these
rules to the project agent. The first pass used stylized low-poly kits recolored to material
families; the owner judged them toy-like ("too baby-like") and asked for realistic assets, so the
rules are now:

- **Sources.** CC0 first; anything else only with a license that allows it and a credit in the
  asset folder's README. **Realistic, photoscanned models** (Poly Haven) over stylized kits: their
  worn, true-to-life surfaces are what make the dead city believable. Where no free model fits
  the setting (a modern street light), the prop is built in code, with real proportions.
- **Own textures, kept.** A model keeps its color texture (packed into one atlas), muted a little
  toward gray so it sits in the palette; normal and roughness maps are dropped (the city has its
  own lighting). Props built in code use the walls' photo detail at the same scale.
- **Budget.** Each model is decimated at import to 1 000 to 5 000 triangles with a 512 px texture,
  and drawn instanced (one mesh per model, any number of copies), within about 110 m of the camera
  (less across floors) and casting shadows within 45 m. Models whose fine lattices cannot be
  decimated (plastic crates) are left out.
- **Same rendering.** Props go through the scene shader like every block: same cel bands, outlines,
  shadows, fog, grain and flower lights. No baked or unlit lighting.
- **Human scale.** Props keep their real sizes (Poly Haven models are in meters); their size is
  what makes the architecture feel immense.
- **Fitting the setting.** Only what a dead modern city would hold: street furniture, leftovers.
  No covered cars (owner). No period pieces that break the era (Poly Haven's Victorian lanterns, for one).
- **Seed places, asset dresses.** What appears where always comes from the seed; a prop gets a
  simple box for collision (a pole only, for street lights).
- **Review.** Every new prop is checked with `--shot` renders, day and night, next to existing
  architecture, before moving on.

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
