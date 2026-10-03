//! The souls' voice: sampled upright piano notes, pitched to a minor pentatonic scale and played
//! through a pool of sound aliases so notes overlap and ring out.

use std::ffi::CString;

use sola_raylib::ffi;

/// Samples (MIDI key, WAV bytes), one every minor third from D#2 to C7, mono 44.1 kHz.
macro_rules! samples {
    ($(($key:expr, $file:literal)),* $(,)?) => {
        &[$(($key, include_bytes!(concat!("../../assets/audio/piano/", $file)) as &[u8])),*]
    };
}

const SAMPLES: &[(i32, &[u8])] = samples![
    (39, "Ds2.wav"),
    (42, "Fs2.wav"),
    (47, "B2.wav"),
    (51, "Ds3.wav"),
    (54, "Fs3.wav"),
    (57, "A3.wav"),
    (60, "C4.wav"),
    (63, "Ds4.wav"),
    (66, "Fs4.wav"),
    (69, "A4.wav"),
    (72, "C5.wav"),
    (75, "Ds5.wav"),
    (78, "Fs5.wav"),
    (81, "A5.wav"),
    (84, "C6.wav"),
    (87, "Ds6.wav"),
    (90, "Fs6.wav"),
    (93, "A6.wav"),
    (96, "C7.wav"),
];

/// Voices per sample: how many times the same note can ring at once.
const VOICES: usize = 3;

/// Pitch classes of the scale: A minor pentatonic (A C D E G).
pub const SCALE: [i32; 5] = [9, 0, 2, 4, 7];

struct Sample {
    key: i32,
    voices: Vec<ffi::Sound>,
    next: usize,
}

pub struct Piano {
    samples: Vec<Sample>,
}

impl Piano {
    /// Needs an initialized audio device.
    pub fn new() -> Self {
        let kind = CString::new(".wav").unwrap();
        let samples = SAMPLES
            .iter()
            .map(|&(key, bytes)| unsafe {
                let wave =
                    ffi::LoadWaveFromMemory(kind.as_ptr(), bytes.as_ptr(), bytes.len() as i32);
                let source = ffi::LoadSoundFromWave(wave);
                ffi::UnloadWave(wave);
                let mut voices = vec![source];
                for _ in 1..VOICES {
                    voices.push(ffi::LoadSoundAlias(source));
                }
                Sample {
                    key,
                    voices,
                    next: 0,
                }
            })
            .collect();
        Self { samples }
    }

    /// Plays MIDI note `note` (may be any key; the nearest sample is pitch-shifted).
    /// `volume` 0..1, `pan` -1 (left) .. 1 (right).
    pub fn play(&mut self, note: i32, volume: f32, pan: f32) {
        let Some(sample) = self.samples.iter_mut().min_by_key(|s| (s.key - note).abs()) else {
            return;
        };
        let shift = (note - sample.key) as f32;
        // A free voice if there is one, else the oldest.
        let n = sample.voices.len();
        let slot = (0..n)
            .map(|k| (sample.next + k) % n)
            .find(|&i| !unsafe { ffi::IsSoundPlaying(sample.voices[i]) })
            .unwrap_or(sample.next);
        sample.next = (slot + 1) % n;
        let voice = sample.voices[slot];
        unsafe {
            ffi::StopSound(voice);
            ffi::SetSoundPitch(voice, 2f32.powf(shift / 12.0));
            ffi::SetSoundVolume(voice, volume);
            ffi::SetSoundPan(voice, pan);
            ffi::PlaySound(voice);
        }
    }
}

impl Drop for Piano {
    fn drop(&mut self) {
        for s in &self.samples {
            unsafe {
                // Aliases first, the source owns the data.
                for v in s.voices.iter().skip(1) {
                    ffi::UnloadSoundAlias(*v);
                }
                ffi::UnloadSound(s.voices[0]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every note the game can ask for is within two semitones of a sample (so pitch shifting by
    /// resampling stays natural).
    #[test]
    fn every_note_has_a_close_sample() {
        for note in 38..=98 {
            let near = SAMPLES.iter().map(|s| (s.0 - note).abs()).min().unwrap();
            assert!(near <= 2, "note {note} is {near} semitones from any sample");
        }
    }
}
