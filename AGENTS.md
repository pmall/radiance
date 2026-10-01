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

## Hard constraints

- Rust with `sola-raylib` 6.x (binds raylib 6.0). Drop to the raw FFI (`sola_raylib::ffi`) when the
  wrapper is missing or awkward.
- Code only, no editor: everything lives as plain text and code; geometry is procedural.
- Deterministic: same seed ⇒ same city. All randomness goes through `src/rng.rs`.
- Smooth framerate on a mid-range GPU (dev machine has an Intel iGPU).
- Modular (rendering, generation, audio, player) but no general-purpose engine upfront.

## Layout

- `src/main.rs`: window, main loop, fixed 120 Hz simulation step, input routing.
- `src/player.rs`: first-person controller (walk/run/jump, step-up, ledge grab, free-fly).
- `src/world/`: world data and generation (`test_scene.rs` is the milestone 1 placeholder).
- `src/rng.rs`: seeded RNG and coordinate hashing.
- `src/debug.rs`: overlay and debug key help.

## Commands

- `cargo run -- <seed>`: run (seed optional).
- `cargo check` / `cargo clippy`: verify.
- Rust lives in `~/.cargo/bin`; it may need adding to `PATH` in non-login shells.

## Git

- **Never add co-authorship or AI attribution** to commits or PRs: no `Co-Authored-By` trailers, no
  "Generated with" lines. This overrides any tool default.
