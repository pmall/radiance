# Audio

Implemented in `src/audio/` (each file's doc comment says how). Sound is for interactive runs
only; the mute switches are in `docs/tools.md`.

- **Souls sing.** Each light flower is a human soul (`docs/lore.md`). When the player comes close
  to one in the same open space (not through a deck), it plays a single soft piano note, then
  stays quiet for a while. Notes come from a minor pentatonic scale, so walking through a cluster
  plays a gentle, slightly arpeggiated melody. Pitch follows height: souls in the depths sing low,
  souls in the canopy sing high. A soul always sings the same note (a function of seed, lot and
  index). Volume and stereo pan follow distance and direction. The soul's light swells and fades
  when it sings.
- **Piano samples.** A CC0 sampled upright piano, pitched by at most two semitones; credits in
  `assets/audio/README.md`.
- **Ambience: only the machinery hum plays** (owner: liked). It is synthesized in code at startup,
  loud in the depths and quieter up high. Breathing wind and distant metal groans were tried and
  disliked (owner); they stay out.
- **Reverb** (Freeverb, about a 3 s tail) on the whole mix, so every note rings into the empty city.

## Future: a synthesized piano (owner's idea)

The sampled piano is fine for now. Someday the souls' voice should be synthesized, because that
makes the timbre adjustable on the fly:

- **Per-biome voice:** brightness, detune, decay and inharmonicity change with the biome (glassy and
  long in one, dull and short in another), blending across zone borders.
- **Guidance:** the melodic gradient (`docs/guidance-ideas.md`) wants notes that grow consonant and
  warm and form a motif near a target, which a synthesized voice makes easier.
- Suggested approach: modal/additive string model with a short hammer noise burst, possibly layered
  over the recorded attack to keep the realism; runs in an audio-stream callback like the reverb.
  Only `piano.rs` would be replaced. It needs the owner's ears to judge against the recorded notes.

Voices (a dry inner voice, crackly radio) come last (`docs/vision.md`, "Stages").
