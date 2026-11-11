# Art direction

**Core principle:** stylized 3D that reads like a 2D illustration. Shapes and color carry the image,
not detail.

## Style target: an abandoned Nivalis

Reference: *Nivalis* (2025), above all a Kowloon-like wall of apartments the owner called the
"perfect result" for the buildings (screenshots kept by the owner, outside git). What makes that
look:

- **Everything is built from boxes** with drawn textures: buildings, balconies, signs and props
  share one visual language.
- **Repeated modules, varied:** a facade is a grid of apartment units (window, cage, air
  conditioner, laundry, balcony, awning), each with its own color, following consistent rules.
- **Density and layers:** small details everywhere, bridges between towers, cables, signs, haze
  giving depth, warm light against teal shadows.

Ours is that city **abandoned**: the neon dead, the windows dark, the signs still readable but
unlit, the glowing flowers the only light. Owner: "Pixel art was never a requirement. Cartoonish /
Nivalis night look was." Textures are drawn at 64 pixels per meter (`pixels::PPM`) toward a painted
look: flat tones with a narrow dithered seam between them, crisp up close, mip levels blended far
away so no band shows.

Second reference, for density and spirit: *Mirror's Edge* (a full, dense city with no people:
punched-window facades, glass curtain walls, big colored accent panels, billboards, rooftops full
of machinery). Ours is hazier, with night and bioluminescence.

The visual pass that set today's look aimed at five things, which still hold (owner): abandoned,
not empty; one consistent look for everything; light that composes the image; a city rather than
a housing estate (`docs/world.md`); biomes recognizable at a glance.

## Rendering look

- **Cel shading** with 2–3 flat light bands, kept on (owner, after the style test).
- **Outlines:** black ink from an edge-detection post-process (`shaders/post.fs`), at every hour
  and in every biome (owner), fading into the fog with distance.
- **Paper grain** over the image.
- **Colored atmospheric fog:** distance fades into tinted gradients rather than gray, creating
  painted depth layers and hiding the draw distance.
- **Limited palettes** per mood, with strong color grading.
- **Atmospheric particles:** drifting spores and pollen, drawn as specks of a few screen pixels;
  halos have hard steps and a dithered glow, like the textures.
- **Chrome vs. life:** cold metal and dead neon against soft, glowing organic forms. This contrast
  is the visual identity.
- **Nothing pops in** (owner): every far limit fades instead of cutting (fog closing in short of
  the loaded lots' edge below the top deck, small things and their shadows dissolving in a
  dither, flower lights and halos fading with distance), and the far skyline's towers stay out of
  that fog so the far city keeps its layers.

## Architecture and surfaces

Each biome draws its own textures for every chunk material, facade unit and city surface
(`docs/world.md`, "Biomes"). Tower walls are painted from their written elevations, with real
units in boxes near the decks (`docs/chunks.md`, "Towers"). Surfaces look like a real city, not
flat boxes: floors like pavement (dust, cracks), buildings like buildings. Streets carry few
markings (owner: crossings painted everywhere was too much).

## Props

Props are **box-built modules** written as text like the chunks (`docs/chunks.md`), in the same
materials, so a bench and the square it stands in come from one hand. Two passes with external
models were rejected by the owner: stylized kits ("too baby-like"), then photoscans (realistic but
foreign to the city's look). The street light alone is a smooth mesh built in code
(`world/props.rs`), since a box pole and arm read poorly. Cars and hydrants were judged horrible
(owner) and there are none.

- **One language:** props use the chunk vocabulary and materials; nothing imported.
- **Same rendering:** props go through the scene shader like every block (cel bands, outlines,
  shadows, fog, grain, flower lights).
- **Human scale:** real sizes (a bench seat at 0.45 m, an armchair 0.9 m); their size is what
  makes the architecture feel immense.
- **Seed places, module shapes:** what appears where comes from the seed and the chunks' slots
  (placement rules: `docs/world.md`, "Streets, props and points of interest").

## Wear: abandoned, not empty

Abandonment is what people left and what time did to it, so every surface is worn, on the same
pixel grid as the textures, in the biome's swatches and by its amounts (`docs/world.md`,
"Biomes"), and by height: the depths dusty and stained, the canopy mossy and strewn with leaves,
open sky bringing rain. Drawn by `groundWear` and `wallWear` in `shaders/scene.fs`.

- **Ground:** damp blotches, long cracks with weeds, dust and sand in drifts, moss in patches, dead
  leaves gathered in the gutters along the curbs, glints of broken glass, puddles with a wet rim
  that mirror the sky, the sun and the flowers' light (the strongest abandoned cue). Road
  markings are worn nearly away.
- **Standing water stays puddles** (owner): flooded streets were tried in the Grotto and rejected
  ("if you can't implement it then it is probably a bad idea"), and a puddle never mirrors wholly
  (the ground shows through), or it reads as a flat slab of sky.
- **Walls:** a ragged band of grime at the foot, moss in it where it is wet, rain streaks and the
  odd rust streak under every story's ledge, soot under the deck above.
- **City surfaces:** roads (asphalt with patches and sealed cracks, or the biome's stone),
  sidewalks, curbstones, roofs and deck undersides are tiles of the biome's textures like the
  chunk materials, so the street and the square share one style.

## Plants

Plants share the pixels of the city: a leaf, frond or blade is a flat cut-out drawn in square
cells at the textures' density, in a few flat tones of its color dithered from a dark root to a
light tip, with a lighter midrib, a darker rim and a ragged edge (`pixelLeaf` in
`shaders/scene.fs`). Fronds are nearly flat, with just enough fold for volume. Flowers stay clean
and glowing, since they are the light, but in the same cells: petals cut to a stepped kite, pale
by the bright core, a darker rim (`pixelPetal`).

## Things left behind

Empty becomes abandoned when things lie where they fell. By rule, never by hand
(`world/leftovers.rs`): debris at the foot of every wall (what fell from a tower facade, or was
dropped against a low wall), never in a road; weeds at the foot of the walls and behind the curbs;
cables sagging across the streets between towers facing each other, stepped like pixels.

## Light under the decks

Most of the city lies under a deck. There the haze, not the sky, is the light: it is brighter
than the walls standing in it, so they read as silhouettes against it (*Blade Runner*, *Stray*),
and it takes a hue per height, the biome's color script (`Atmosphere::heights`): cold blue-green
in the depths, where every flower stands out, warm amber in the middle, gold under the canopy.
The ambient light of surfaces the sky does not reach takes the same hue. At night the haze takes
the colors of the flowers around the eye, faint where there are few, so walls still stand out in
the dark. Under the decks the eye adapts a little. The depths stay dark (owner: loved as it is).

Where the sun gets through a hole or a crater into the floors below, it lights the haze as beams
(`fogLight` in `shaders/post.fs`), brightest looking toward the sun; the moon makes faint ones at
night. The deck overhead is a structure, not a flat ceiling: girders along the lot borders,
joists, pipe runs, dead lamps on their rods, water dripping. A flower lights the wall it faces
further off than the ground around it, so the walls show where the flowers are.

## Day/night cycle

Time of day evolves continuously. Sun direction, sky, fog color, palette and grading all follow it,
interpolating smoothly between key phases (each biome has its own looks for them, the default
biome's described here; `docs/world.md`, "Biomes"):

- **Dawn:** cool, pale haze; plants dimming.
- **Day:** warm, hazy layers of chrome and overgrowth fading into ochre or teal fog.
- **Dusk:** saturated warm-to-violet transition; the first flowers light up.
- **Night:** dark fog; the flowers' bioluminescence (cyan, green, soft magenta) is the only light.

Flower glow follows the cycle (faint by day, strongest at night). The full cycle's length is one
parameter (`daycycle.rs`).

## Light plants

**Flowers are the light sources, not whole plants** (owner). A bush, vine or climber is foliage;
the flowers growing on it emit the light. Each light sits at its flower, takes the flower's color,
has a visible glowing source (halo) and stays small and tightly tied to it, so the viewer can see
which flower casts which light. There are many small lights, binned in a camera grid
(`render/lights.rs`) so the count scales.
