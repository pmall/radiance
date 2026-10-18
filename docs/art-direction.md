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
scene shader in world space and multiplied over the procedural colors. *(Replaced in milestone 10: streets, sidewalks, roofs, deck undersides and edges, stairs, steel
and the street light are pixel art like the chunks; the photo detail is left on the plants
only.)*

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

## Wear: abandoned, not empty

*(Milestone 10, pass 1: `groundWear` and `wallWear` in `shaders/scene.fs`.)* Abandonment is what
people left and what time did to it, so every surface is worn, drawn on the same pixel grid as
the pixel art (16 pixels per meter, ordered dither) in the biome's swatches and by its amounts
(`Textures`: `dusty`, `littered`, `puddled`, `mossy`, `damp`), and by height: the depths dusty and
stained, the canopy mossy and strewn with leaves, open sky bringing rain.

- **Ground:** damp blotches, long cracks with weeds, dust and sand in drifts, moss in patches, dead
  leaves gathered in the gutters along the curbs, glints of broken glass, puddles with a wet rim
  that mirror the sky, the sun and the flowers' light (the strongest abandoned cue). Road
  markings are worn nearly away.
- **Walls:** a ragged band of grime at the foot, moss in it where it is wet, rain streaks and the
  odd rust streak under every story's ledge, soot under the deck above.
- **Surfaces in pixel art:** roads (asphalt with patches and sealed cracks, or the biome's
  stone), sidewalks, curbstones, roofs (tar and gravel) and deck undersides (coffers between
  beams, water stains, salts) are tiles of the biome's row of the atlas (`render/pixels.rs`), like
  the chunk materials, so the street and the square share one style.
- Broad shapes come from a noise texture (`render/noise.rs`), so the wear costs no frame time;
  single pixels (cracks, leaves, glints) are only worked out up close.

## Things left behind

*(Milestone 10, pass 3: `src/world/leftovers.rs`.)* Empty becomes abandoned when things lie where
they fell. By rule, never by hand: debris at the foot of every wall, every 3 m at the floor's
`debris` chance (dense in the depths, rare at the construction front) and the biome's
`littered`: what fell from a tower facade (lumps of concrete, a heap of rubble, bricks, a window
frame, a panel) or was dropped against a low wall (bags, boxes, papers, a tyre), each turned at
random, never in a road. Weeds (three to five blades from a crack) at the foot of the walls and
behind the curbs. Cables sag across the streets between towers facing each other, one to four
per floor, drawn as short boxes that step like pixels.

## Light under the decks

*(Milestone 10, pass 1.)* Most of the city lies under a deck. There the haze, not the sky, is the
light: it is brighter than the walls standing in it, so they read as silhouettes against it
(*Blade Runner*, *Stray*), and it takes a hue per height, the biome's color script
(`Atmosphere::heights`): cold blue-green in the depths, where every flower stands out, warm amber
in the middle, gold under the canopy (the Grotto: deep blue, teal, pale aqua). The ambient light
of surfaces the sky does not reach takes the same hue. At night the haze takes the colors of the
flowers around the eye, faint where there are few, so walls still stand out in the dark. Under
the decks the eye adapts a little (the image opens up by a third). Particles are specks of a few
screen pixels and halos have hard steps and a dithered glow, like the textures.

*(Pass 2.)* Where the sun gets through a hole or a crater into the floors below, it lights the
haze as beams: the shafts are the haze in the sun that sees little sky (`fogLight` in
`shaders/post.fs`), brightest looking toward the sun, and the moon makes faint ones at night. The
deck overhead is a structure, not a flat ceiling: girders along the lot borders, joists between,
pipe runs from lot to lot, dead lamps on their rods, water dripping to the floor. A flower lights
the wall it faces further off than the ground around it (a wash up the wall), so the walls show
where the flowers are.

## Reference views

Every look change is judged on the same images: `python3 tools/lookdev.py LABEL` renders eight
views at day, dusk and night into `screenshots/lookdev/LABEL/` (a sheet per time of day), to set
side by side with an earlier label.

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
