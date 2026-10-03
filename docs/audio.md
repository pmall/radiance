# Audio

Implemented in `src/audio/`. Sound is for interactive runs only (screenshots and benchmarks are
silent; `--no-audio` mutes a run, `M` toggles mute in game).

- **Souls sing.** Each light flower is a human soul (see `docs/lore.md`). When the player comes
  within 6.5 m of one in the same open space (not through a deck) it plays a single soft piano note,
  then stays quiet for 45 s. Notes come from A minor pentatonic, so walking through a cluster plays
  a gentle, slightly arpeggiated melody (notes of one step are staggered by 0.2 s). Pitch follows
  height: souls in the depths sing low, souls in the canopy sing high. A soul always sings the same
  note (a function of seed, lot and index). Volume and stereo pan follow distance and direction. The
  soul's light swells and fades over a few seconds when it sings.
- **Piano samples.** CC0 "Upright Piano KW" from FreePats (a living-room upright, mono, one
  velocity, a sample every minor third from D#2 to C7), pitched by at most two semitones with
  playback speed. Credits in `assets/audio/README.md`.
- **Ambience**, synthesized in code at startup (no samples): a machinery hum (detuned bass partials
  and rumble, loud in the depths, quieter up high), breathing wind (band-passed noise swelling every
  8 s, strongest in the open sky), and distant metal creaks (stick-slip pulses through two
  resonances) every 9 to 26 s at random pitch and pan.
- **Reverb** (Freeverb, about a 3 s tail) on the whole mix, so every note rings into the empty city.
- Tests check the synthesis (no NaN, no clipping, seamless loops), that the reverb decays, and that
  notes stay in the scale and follow height.

## Future: a synthesized piano (owner's idea)

The sampled piano is fine for now. Someday the souls' voice should be synthesized, because that makes
the timbre adjustable on the fly:

- **Per-biome voice:** brightness, detune, decay and inharmonicity change with the biome (glassy and
  long in one, dull and short in another), blending across zone borders.
- **Sound as guidance, like the wind in Ghost of Tsushima (owner's direction):** the sound acts as a
  guiding wind that leads the player toward points of interest in the very large city, with no UI.
  It reshapes as the player moves toward a place or object (rising pitch or brightness, faster
  pulse, direction by stereo and muffling). The visual twin would be the existing pollen motes and
  spores drifting the same way, as the wind does in that game. This needs points of interest to
  exist first: they must come from the seed (e.g. very tall unfinished towers, soul clusters, the
  rare rainbow biome, biome borders).
- Suggested approach: modal/additive string model with a short hammer noise burst, possibly layered
  over the recorded attack to keep the realism; runs in an audio-stream callback like the reverb.
  Only `piano.rs` would be replaced. It needs the owner's ears to judge against the recorded notes.
