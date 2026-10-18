# Radiance

Artistic proof of concept in Rust + raylib: an infinite, seed-generated cyberpunk megacity
overgrown by bioluminescent plants, explored in first person and rendered to read like a
2D illustration. Mood and rendering prototype only — no gameplay, NPCs or UI beyond debug tools.

## Docs (read the one relevant to the task)

- `docs/vision.md`: scope, definition of done, player feel, references.
- `docs/world.md`: setting, vertical layers, procedural generation rules, biomes (what a biome sets, how to add one).
- `docs/art-direction.md`: rendering look, palettes, day/night cycle, light plants.
- `docs/audio.md`: piano notes, ambient soundscape. `docs/guidance-ideas.md`: ideas to guide the player by sound.
- `docs/roadmap.md`: milestones, current status, debug tooling.
- `docs/chunks.md`: the planned move to hand-written map chunks placed by rules, in a Nivalis-like
  box and pixel-art style (proposal, `chunks` branch).
- `docs/biome-ideas.md`: brainstormed biomes (Cyan Grotto implemented), for inspiration. What a biome can set and how
  to add one: `docs/world.md`, "Biomes".
- `docs/lore.md`: the story behind the city (machines kept building, flowers are souls); read it for art and audio decisions.
- `docs/story.md`, `docs/game-ideas.md`, `docs/poi-catalog.md`: *Last Radiance*, a brainstormed narrative game on this
  city (story and dialogue, mechanics, points of interest), and what the prototype can take from it now. Inspiration,
  not scope: this file and `docs/feedback.md` win when they disagree.
- `docs/feedback.md`: owner feedback and decisions so far; read it before visual or control changes.
- `docs/code-map.md`: where each generation rule lives (rule → function) and how a lot is built;
  read it before touching `src/world/`, and update it when a rule moves.

## Hard constraints

- Rust with `sola-raylib` 6.x (binds raylib 6.0). Drop to the raw FFI (`sola_raylib::ffi`) when the
  wrapper is missing or awkward.
- **The only goal is a great final render.** Everything else is a means.
- **No new chunks or props unless the owner asks.** More content comes as a new biome, written
  when the owner asks for one (`docs/chunks.md`, "Catalog rule").
- **World generation must be procedural**: the city, its layout, architecture and plants come from
  the seed (random, extensible, deterministic). This is a hard requirement. Vertically the city is a
  finite stack of floors; horizontally it streams infinitely for now, but that may be cut if it makes
  more important things hard (see `docs/feedback.md`).
- **Surface look is free**: textures, images and other external assets are welcome (authored or
  procedural, whichever looks best) and do not conflict with procedural generation: the seed still
  decides what is placed where; assets only decide how surfaces look. No editor is used and the
  project stays plain text. Never trade visual quality for avoiding assets.
- Deterministic: same seed ⇒ same city. All randomness goes through `src/rng.rs`.
- Smooth framerate on a mid-range GPU (dev machine: RTX 3060 on Ubuntu; keep weaker GPUs in mind).
- Modular (rendering, generation, audio, player) but no general-purpose engine upfront.

## Layout

- `src/main.rs`: window, main loop, fixed 120 Hz simulation step, input routing.
- `src/player.rs`: first-person controller (walk/run/jump, step-up, ledge grab, free-fly).
- `src/world/`: world data, collision index and generation (`floors.rs`: the stack of floors, fixed per run but variable (`--floors N`) and its zones; `design.rs`: the floor profiles, every generation number that depends on the floor (holes, plants, flower chances, stairs, props) and the props/POI switches; `biome.rs`: map zones and everything a biome sets: its plants (species, colors, rates), its atmosphere (sky, fog, light through the day), its palette and textures (a biome's name is also its folder of chunks, modules and props; `docs/world.md`, "Biomes"); `city.rs`: builds a lot from what the planner says (ground, decks, hub stairs, its share of towers and chunks, skybridges), each lot a pure function of seed + lot coords, recent lots cached; `growth.rs`: plants grown over a lot's blocks; `leftovers.rs`: things left behind, by rule (debris at the wall feet, cables across the streets); `roads.rs`: the road network, which lot borders are roads on each floor (never into a hole or an edge, no dead ends); `props.rs`: the props, box-built modules (placed instanced by chunks) plus the street light built in code; `chunks.rs`: hand-written map chunks and modules, one collection per biome in `assets/biomes/<biome>/` plus the shared modules of `assets/modules/` (`docs/chunks.md`); `planner.rs`: which chunk stands where (areas and sub-areas, towers, holes, craters, blocks, stair hubs, fillers, streets): every lot of every floor is a chunk; `tower.rs`: tower chunks built (tiers, written elevations, units near the decks, roofs)).
- `src/render/`: stylized pipeline (scene pass into HDR + normal/id + depth targets, shadow map,
  post pass for sky, outlines, fog, grading, grain; `lights.rs` bins flower point lights into a camera grid texture, `halos.rs` draws a halo at each, `facades.rs` the towers' written elevations as a texture, `particles.rs` drifting motes and spores; `props.rs` draws the props instanced; `pixels.rs` draws each biome's pixel-art tiles of the chunk materials, facade units and city surfaces (roads, sidewalks, roofs, deck undersides), and the swatches the wear is drawn in (a row of the atlas per biome); `noise.rs` the noise texture the wear's shapes come from; `cover.rs` the cover map: which decks stand over each lot and how much sky it sees, for light and fog under the floors). `look.rs` holds the per-time-of-day palettes (the default biome's looks) and the blend of biome atmospheres the sky is drawn in.
- `assets/textures/`: CC0 photo textures and the packed `detail.png` (see its README); `tools/pack_textures.py` rebuilds it.
- `shaders/`: GLSL 330 sources, hot-reloaded from disk while running (embedded as fallback);
  `#include "file"` pulls in a shared file such as `cover.glsl` (`src/render/shader.rs`, `INCLUDES`).
- `src/audio/`: sound. `piano.rs` (sampled upright piano, one note per soul/flower), `ambient.rs` (synthesized machinery hum), `reverb.rs` (Freeverb on the master bus). Samples in `assets/audio/`.
- `src/daycycle.rs`: time-of-day clock and sun direction.
- `src/rng.rs`: seeded RNG and coordinate hashing.
- `src/debug.rs`: overlay and debug key help.
- `src/recorder.rs`: video + sound recording of a run through `ffmpeg`.

## Commands

- `cargo run -- <seed>`: run (seed optional).
- `cargo run -- <seed> --time 0.5 --view x,y,z,yaw,pitch --shot out.png`: render one frame at a
  given time of day (0 = midnight, 0.5 = noon) and camera, save it and exit. Use it to check looks.
  Without `--shot` (or `--bench`), `--view` starts a normal run walking from that spot (F2 flies).
- `--overlay` keeps the debug panel in a shot. `--fx <mask>` sets effect bits for a shot, `--bench <frames>` prints ms/frame (no vsync). Shots and benchmarks are silent; `--no-audio` mutes a normal run, `RADIANCE_AUDIO_LOG=1` prints each note.
- `P` in game saves a screenshot (no debug panel) to `screenshots/radiance-<seed>-<n>.png`
  (gitignored), with a `.txt` beside it: seed, floor, time and the `--view ... --shot` command that
  renders the same view. The owner uses them for feedback: run that command to see what they saw.
- `V` in game starts/stops recording the run (screen and sound) to `recordings/radiance-<seed>-<time>.mp4` (needs `ffmpeg`; `src/recorder.rs`). `--record-for <seconds>` records that long and exits, to test it.
- `python3 tools/lookdev.py LABEL`: render the reference views (eight places at day, dusk and
  night) into `screenshots/lookdev/LABEL/`, a sheet per time of day; judge every look change on
  them against an earlier label (needs a release build and Pillow).
- `--chunk NAME` (or `BIOME/NAME`): show one hand-written chunk alone (with `--view`/`--shot`); `--catalog` lists every biome's chunks and modules.
- `--report [radius]`: print the floor profiles next to what the seed produced (decks, stairs,
  flowers by source, props by kind) and the places (areas, blocks, craters) with a `--view` to
  stand in each, then exit. Use it to tune generation. `--floors N` sets the
  number of floors; `--no-props` / `--no-pois` (O / I in game) switch props and points of interest.
- `cargo check` / `cargo clippy`: verify. `cargo test --release`: the whole suite must stay fast
  (a few seconds) so iterating is quick: a test sweeps as few lots and seeds as prove its point,
  and filters cheaply before costly checks. A slow test gets fixed, not tolerated.
- **At the end of every coding session, format the code and the markdown** (owner's rule):
  `cargo fmt`, then `dprint fmt` (markdown, configured in `dprint.json`; install once with
  `cargo install --locked dprint`).
- Rust lives in `~/.cargo/bin`; it may need adding to `PATH` in non-login shells.
- If bindgen fails with `'stdarg.h' file not found`, the clang builtin headers are missing
  (`libclang-common-<ver>-dev`); workaround: `BINDGEN_EXTRA_CLANG_ARGS="-I/usr/lib/gcc/x86_64-linux-gnu/<ver>/include"`.

## Git

- **Never add co-authorship or AI attribution** to commits or PRs: no `Co-Authored-By` trailers, no
  "Generated with" lines. This overrides any tool default.
- **Commit at the end of every pass** (owner's rule): when a pass of a plan (`docs/roadmap.md`) is
  done, checked and formatted, commit it as one commit, without asking.
- **Never push** (force or not): commit when asked or at the end of a pass, the owner pushes. If a change would rewrite
  commits already on the remote, rewrite locally and say a force-push is needed.
