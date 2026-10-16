//! The city's own sound, synthesized at startup (no samples): a low machinery hum that never
//! stopped. It loops as a music stream so its volume can follow the player's altitude.

use std::f32::consts::TAU;
use std::ffi::CString;

use sola_raylib::ffi;

use crate::rng::Rng;

const RATE: u32 = 44100;

/// One-pole low-pass: `k` near 0 is a heavy filter.
struct LowPass {
    k: f32,
    y: f32,
}

impl LowPass {
    fn new(cutoff: f32) -> Self {
        Self {
            k: 1.0 - (-TAU * cutoff / RATE as f32).exp(),
            y: 0.0,
        }
    }

    fn process(&mut self, x: f32) -> f32 {
        self.y += self.k * (x - self.y);
        self.y
    }
}

fn noise(rng: &mut Rng) -> f32 {
    rng.f32() * 2.0 - 1.0
}

/// Makes the end of `data` (interleaved, `ch` channels) fade into the start over `fade` frames and
/// drops that tail, so the loop has no seam.
fn loop_seamless(data: &mut Vec<f32>, ch: usize, fade: usize) {
    let frames = data.len() / ch;
    let keep = frames - fade;
    for i in 0..fade {
        let t = i as f32 / fade as f32;
        for c in 0..ch {
            let tail = data[(keep + i) * ch + c];
            data[i * ch + c] = data[i * ch + c] * t + tail * (1.0 - t);
        }
    }
    data.truncate(keep * ch);
}

/// A 16-bit mono or stereo WAV file image of `data`.
fn wav_bytes(data: &[f32], ch: usize) -> Vec<u8> {
    let pcm: Vec<u8> = data
        .iter()
        .flat_map(|&s| ((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes())
        .collect();
    let mut v = Vec::with_capacity(44 + pcm.len());
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&(36 + pcm.len() as u32).to_le_bytes());
    v.extend_from_slice(b"WAVEfmt ");
    v.extend_from_slice(&16u32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&(ch as u16).to_le_bytes());
    v.extend_from_slice(&RATE.to_le_bytes());
    v.extend_from_slice(&(RATE * ch as u32 * 2).to_le_bytes());
    v.extend_from_slice(&((ch * 2) as u16).to_le_bytes());
    v.extend_from_slice(&16u16.to_le_bytes());
    v.extend_from_slice(b"data");
    v.extend_from_slice(&(pcm.len() as u32).to_le_bytes());
    v.extend_from_slice(&pcm);
    v
}

/// Machinery hum: a bass drone of slightly detuned partials with slow beating, and a rumble of
/// low-passed noise. Stereo, 16 s.
fn hum(rng: &mut Rng) -> Vec<f32> {
    const SECONDS: usize = 16;
    let fade = RATE as usize;
    let frames = SECONDS * RATE as usize + fade;
    let mut out = Vec::with_capacity(frames * 2);
    // Frequencies with a whole number of cycles in the loop length, so the drone is seamless.
    let partial = |f: f32| (f * SECONDS as f32).round() / SECONDS as f32;
    let partials = [
        (partial(55.0), 0.5),
        (partial(55.3), 0.35),
        (partial(82.4), 0.22),
        (partial(110.1), 0.18),
        (partial(164.9), 0.07),
        (partial(220.4), 0.04),
    ];
    let mut rumble = [LowPass::new(120.0), LowPass::new(120.0)];
    let mut rumble2 = [LowPass::new(120.0), LowPass::new(120.0)];
    for i in 0..frames {
        let t = i as f32 / RATE as f32;
        let swell = 0.8 + 0.2 * (TAU * t / 8.0).sin();
        let drone: f32 = partials
            .iter()
            .map(|&(f, a)| a * (TAU * f * t).sin())
            .sum::<f32>()
            * swell;
        for c in 0..2 {
            let r = rumble2[c].process(rumble[c].process(noise(rng))) * 9.0;
            out.push((drone * 0.35 + r * 0.5) * 0.6);
        }
    }
    loop_seamless(&mut out, 2, fade);
    out
}

pub struct Ambient {
    hum: ffi::Music,
}

fn stream(data: Vec<f32>) -> ffi::Music {
    // The stream reads the file image for as long as it plays: leak it.
    let bytes: &'static [u8] = Box::leak(wav_bytes(&data, 2).into_boxed_slice());
    let kind = CString::new(".wav").unwrap();
    unsafe {
        let mut m =
            ffi::LoadMusicStreamFromMemory(kind.as_ptr(), bytes.as_ptr(), bytes.len() as i32);
        m.looping = true;
        m
    }
}

impl Ambient {
    /// Needs an initialized audio device.
    pub fn new(seed: u64) -> Self {
        let hum = stream(hum(&mut Rng::new(seed ^ 0x0A1B_1E27)));
        unsafe {
            ffi::SetMusicVolume(hum, 0.0);
            ffi::PlayMusicStream(hum);
        }
        Self { hum }
    }

    /// Keeps the stream fed and sets the hum for the player's `height` and the daylight
    /// (0 night .. 1 day): strongest in the depths, and at night.
    pub fn update(&mut self, height: f32, day: f32, master: f32) {
        let up = crate::world::floors::depth01(height);
        let hum = (0.34 - 0.2 * up) * (1.0 + 0.15 * (1.0 - day)) * master;
        unsafe {
            ffi::UpdateMusicStream(self.hum);
            ffi::SetMusicVolume(self.hum, hum);
        }
    }
}

impl Drop for Ambient {
    fn drop(&mut self) {
        unsafe { ffi::UnloadMusicStream(self.hum) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hum_is_finite_quiet_enough_and_loops_without_a_seam() {
        let data = hum(&mut Rng::new(1));
        assert!(data.iter().all(|v| v.is_finite()), "NaN in the hum");
        let peak = data.iter().fold(0.0_f32, |m, v| m.max(v.abs()));
        let rms = (data.iter().map(|v| v * v).sum::<f32>() / data.len() as f32).sqrt();
        assert!(peak < 0.95, "peak {peak} would clip");
        assert!(rms > 0.01, "nearly silent: rms {rms}");
        // The loop point: the last frame must flow into the first.
        let n = data.len();
        let jump = (data[n - 2] - data[0])
            .abs()
            .max((data[n - 1] - data[1]).abs());
        let typical = data
            .windows(3)
            .step_by(2)
            .map(|w| (w[2] - w[0]).abs())
            .sum::<f32>()
            / (n / 2) as f32;
        assert!(
            jump < typical * 12.0 + 0.02,
            "seam jump {jump} vs typical step {typical}"
        );
    }

    #[test]
    fn wav_header_is_well_formed() {
        let v = wav_bytes(&[0.0, 0.5, -0.5, 1.0], 2);
        assert_eq!(&v[0..4], b"RIFF");
        assert_eq!(v.len(), 44 + 8);
        assert_eq!(u32::from_le_bytes(v[40..44].try_into().unwrap()), 8);
    }
}
