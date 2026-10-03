# Radiance

Artistic proof of concept in Rust + raylib: an infinite, seed-generated cyberpunk megacity
overgrown by bioluminescent plants, explored in first person and rendered to read like a
2D illustration. Mood and rendering prototype only — no gameplay, NPCs or UI beyond debug tools.

## Docs (read the one relevant to the task)

- `docs/vision.md`: scope, definition of done, player feel, references.
- `docs/world.md`: setting, vertical layers, procedural generation rules.
- `docs/art-direction.md`: rendering look, palettes, day/night cycle, light plants.
- `docs/audio.md`: piano notes, ambient soundscape.
- `docs/roadmap.md`: milestones, current status, debug tooling.
- `docs/biome-ideas.md`: brainstormed biomes (not implemented), for inspiration.
- `docs/feedback.md`: owner feedback and decisions so far; read it before visual or control changes.

## Hard constraints

- Rust with `sola-raylib` 6.x (binds raylib 6.0). Drop to the raw FFI (`sola_raylib::ffi`) when the
  wrapper is missing or awkward.
- **The only goal is a great final render.** Everything else is a means.
- **World generation must be procedural**: the city, its layout, architecture and plants come from
  the seed (random, infinite, extensible, deterministic). This is a hard requirement.
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
- `src/world/`: world data, collision index and generation (`biome.rs`: map zones and per-biome generation parameters; `city.rs`: lot-based city skeleton, each lot a pure function of seed + lot coords; `growth.rs`: plants grown over a lot's blocks).
- `src/render/`: stylized pipeline (scene pass into HDR + normal/id + depth targets, shadow map,
  post pass for sky, outlines, fog, grading, grain; `lights.rs` bins flower point lights into a camera grid texture, `halos.rs` draws a halo at each, `particles.rs` drifting motes and spores). `look.rs` holds the per-time-of-day palettes.
- `assets/textures/`: CC0 photo textures and the packed `detail.png` (see its README); `tools/pack_textures.py` rebuilds it.
- `shaders/`: GLSL 330 sources, hot-reloaded from disk while running (embedded as fallback).
- `src/daycycle.rs`: time-of-day clock and sun direction.
- `src/rng.rs`: seeded RNG and coordinate hashing.
- `src/debug.rs`: overlay and debug key help.

## Commands

- `cargo run -- <seed>`: run (seed optional).
- `cargo run -- <seed> --time 0.5 --view x,y,z,yaw,pitch --shot out.png`: render one frame at a
  given time of day (0 = midnight, 0.5 = noon) and camera, save it and exit. Use it to check looks.
- `--fx <mask>` sets effect bits for a shot, `--bench <frames>` prints ms/frame (no vsync).
- `cargo check` / `cargo clippy`: verify.
- Rust lives in `~/.cargo/bin`; it may need adding to `PATH` in non-login shells.
- If bindgen fails with `'stdarg.h' file not found`, the clang builtin headers are missing
  (`libclang-common-<ver>-dev`); workaround: `BINDGEN_EXTRA_CLANG_ARGS="-I/usr/lib/gcc/x86_64-linux-gnu/<ver>/include"`.

## Git

- **Never add co-authorship or AI attribution** to commits or PRs: no `Co-Authored-By` trailers, no
  "Generated with" lines. This overrides any tool default.
