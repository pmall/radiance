# Debug and art-iteration tools

The look is worked out by iteration, so the tools to see, measure and compare it matter as much
as the features. The command line is parsed in `src/main.rs` (`parse_args`), the keys handled in
its main loop; the overlay is `src/debug.rs`.

## Command line

`cargo run --release -- [seed] [flags]` (seed 1 when left out). `--help` prints the usage. A bad
command line (unknown flag, missing or out-of-range value, a second seed, an unknown `--chunk`)
says what is wrong and exits with 2; a `--shot` that cannot be saved or a `--record-for` that fails
exits with 1.

| Flag                   | What it does                                                                                                                     |
| ---------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| `--time T`             | Time of day, 0 = midnight, 0.5 = noon.                                                                                           |
| `--view x,y,z,yaw,pch` | Camera at that feet position. With `--shot` or `--bench` it renders there; alone it starts a normal run walking from that spot.  |
| `--shot out.png`       | Render one frame, save it and exit (silent).                                                                                     |
| `--overlay`            | Keep the debug panel in a shot.                                                                                                  |
| `--fx MASK`            | Effect bits for a shot (`debug::EFFECT_KEYS`: outlines, fog, cel, shadows, grading, grain, particles, texture; 255 = all on).    |
| `--bench N`            | Render N frames without vsync and print ms/frame and the triangles drawn in view (silent).                                       |
| `--floors N`           | Number of floors of the stack, 5 to 16 (`floors::MIN`, `MAX`); refuses any other.                                                |
| `--no-props`           | Props off (like O in game).                                                                                                      |
| `--no-pois`            | Points of interest off (like I in game).                                                                                         |
| `--report [radius]`    | Print the floor profiles next to what the seed produced, and the places and landmark towers with a `--view` for each, then exit. |
| `--chunk NAME`         | Show one chunk alone (`BIOME/NAME` for another biome's, in that biome's whole look) on a bare deck before the spawn.             |
| `--catalog`            | List every biome's chunks and modules (labels, zones, weights) and the files that failed to load.                                |
| `--no-audio`           | Mute a normal run. `RADIANCE_AUDIO_LOG=1` prints each note played.                                                               |
| `--record-for S`       | Record S seconds of the run, then exit (to test the recorder).                                                                   |

## Keys

The owner uses an AZERTY keyboard: movement works with ZQSD, WASD and the arrow keys alike, so
nothing else may sit on the arrows.

- **Move:** ZQSD / WASD / arrows, Shift to run, Space or left click to jump (and rise when
  flying), Ctrl to sink when flying. Esc frees the mouse, a click captures it, Ctrl+Q quits.
- **F1** overlay, **F2** free-fly, **G** test mode (below).
- **F3–F8, F11, X** toggle outlines, fog, cel bands, shadows, grading, grain, particles,
  textures. **F9** cycles the render scale (for weaker GPUs). **F10** reloads the shaders.
- **Time:** T pauses, PgUp / PgDn speed up / slow down, Home / End scrub, 1–4 jump to dawn,
  noon, dusk, midnight.
- **Seed:** R random, ] / [ (or keypad + / -) next and previous. **O** / **I** switch props /
  points of interest and regenerate.
- **M** mute, **P** screenshot, **V** start / stop recording.

## Overlay and test mode

The overlay must be readable by someone who is not a specialist (owner): plain words, the floor
shown large at the top, floors counted from 0 like their D names, directions as clock hours (12
o'clock straight ahead).

Test mode (G, on by default) helps find things without changing the city: stairs and craters are
generated as in the real game, however rare. The overlay gives the distance and clock direction
of the nearest stair up, stair down and crater within about a kilometer (`FIND_RADIUS`), marked
"far, not loaded yet" beyond the loaded lots; the right mouse button makes Wick four times faster.

## Screenshots, recordings, reference views

- **P** saves the frame without the overlay to `screenshots/radiance-<seed>-<n>.png`, with a
  `.txt` beside it: seed, floor, time and the `--view ... --shot` command that renders the same
  view. The owner sends these as feedback; that command reproduces what they saw.
- **V** records the screen and the sound to `recordings/radiance-<seed>-<time>.mp4` through
  `ffmpeg` (`src/recorder.rs`), without the overlay.
- **Reference views:** `python3 tools/lookdev.py LABEL` renders a fixed set of views (listed in
  the script) at day, dusk and night into `screenshots/lookdev/LABEL/`, one sheet per time of
  day, to set side by side with an earlier label. Needs a release build and Pillow.

## Hot reload

Shaders (`shaders/`) and chunk and module files (`assets/`) reload when edited, and the city
regenerates. Palettes (`render/look.rs`) and biome settings (`world/biome.rs`) are Rust constants
and take a rebuild.
