//! The city's own sound, synthesized at startup (no samples): a low machinery hum that never
//! stopped, slow breathing wind through the towers, and now and then a metal creak somewhere far
//! off. The two beds loop as music streams so their volumes can follow the player's altitude.

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

/// Resonant band-pass (biquad).
struct BandPass {
    b0: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl BandPass {
    fn new(freq: f32, q: f32) -> Self {
        let w = TAU * freq / RATE as f32;
        let alpha = w.sin() / (2.0 * q);
        let a0 = 1.0 + alpha;
        Self {
            b0: alpha / a0,
            a1: -2.0 * w.cos() / a0,
            a2: (1.0 - alpha) / a0,
            z1: 0.0,
            z2: 0.0,
        }
    }

    fn process(&mut self, x: f32) -> f32 {
        // Transposed direct form II; the band-pass numerator is b0 * (1, 0, -1).
        let y = self.b0 * x + self.z1;
        self.z1 = self.z2 - self.a1 * y;
        self.z2 = -self.b0 * x - self.a2 * y;
        y
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

/// Wind through the towers: band-passed noise swelling and fading like slow breathing, with a thin
/// whisper above it. Stereo with decorrelated channels, 32 s.
fn air(rng: &mut Rng) -> Vec<f32> {
    const SECONDS: usize = 32;
    let fade = 2 * RATE as usize;
    let frames = SECONDS * RATE as usize + fade;
    let mut out = Vec::with_capacity(frames * 2);
    let mut body = [BandPass::new(420.0, 0.9), BandPass::new(470.0, 0.9)];
    let mut whisper = [BandPass::new(2600.0, 2.5), BandPass::new(2900.0, 2.5)];
    for i in 0..frames {
        let t = i as f32 / RATE as f32;
        // Breathing: about one cycle every 8 s, with an uneven second harmonic.
        let breath = (0.5 + 0.5 * (TAU * t / 8.0).sin()).powf(1.6);
        let slow = 0.6 + 0.4 * (TAU * t / 32.0).sin();
        for c in 0..2 {
            let b = body[c].process(noise(rng)) * 2.4 * breath * slow;
            let w = whisper[c].process(noise(rng)) * 1.0 * (0.3 + 0.7 * breath) * slow;
            out.push((b + w * 0.35) * 0.55);
        }
    }
    loop_seamless(&mut out, 2, fade);
    out
}

/// A metal creak: a stick-slip pulse train gliding down in rate, ringing two resonances.
fn creak(rng: &mut Rng) -> Vec<f32> {
    let seconds = rng.range(1.3, 2.6);
    let frames = (seconds * RATE as f32) as usize;
    let (f1, f2) = (rng.range(260.0, 420.0), rng.range(700.0, 1100.0));
    let (mut r1, mut r2) = (BandPass::new(f1, 14.0), BandPass::new(f2, 18.0));
    let (rate0, rate1) = (rng.range(40.0, 70.0), rng.range(14.0, 28.0));
    let mut out = Vec::with_capacity(frames);
    let mut phase = 0.0f32;
    for i in 0..frames {
        let t = i as f32 / frames as f32;
        // Pulses at a falling rate, each with a little jitter.
        let rate = rate0 + (rate1 - rate0) * t;
        phase += rate / RATE as f32;
        let pulse = if phase >= 1.0 {
            phase -= 1.0 + rng.range(-0.15, 0.15);
            rng.range(0.4, 1.0)
        } else {
            0.0
        };
        let env = (t * 9.0).min(1.0) * (1.0 - t).powf(1.5);
        out.push((r1.process(pulse) * 1.6 + r2.process(pulse) * 0.9) * env);
    }
    // The resonators are narrow and ring quietly: bring the creak to a fixed loudness.
    let peak = out.iter().fold(1e-6_f32, |m, v| m.max(v.abs()));
    out.iter_mut().for_each(|v| *v *= 0.6 / peak);
    out
}

pub struct Ambient {
    hum: ffi::Music,
    air: ffi::Music,
    creaks: Vec<ffi::Sound>,
    next_creak: f32,
    rng: Rng,
}

fn stream(rng: &mut Rng, data: Vec<f32>) -> ffi::Music {
    // The stream reads the file image for as long as it plays: leak it.
    let bytes: &'static [u8] = Box::leak(wav_bytes(&data, 2).into_boxed_slice());
    let kind = CString::new(".wav").unwrap();
    let _ = rng;
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
        let mut rng = Rng::new(seed ^ 0x0A1B_1E27);
        let hum_data = hum(&mut rng);
        let air_data = air(&mut rng);
        let hum = stream(&mut rng, hum_data);
        let air = stream(&mut rng, air_data);
        let kind = CString::new(".wav").unwrap();
        let creaks = (0..6)
            .map(|_| {
                let bytes = wav_bytes(&creak(&mut rng), 1);
                unsafe {
                    let wave =
                        ffi::LoadWaveFromMemory(kind.as_ptr(), bytes.as_ptr(), bytes.len() as i32);
                    let sound = ffi::LoadSoundFromWave(wave);
                    ffi::UnloadWave(wave);
                    sound
                }
            })
            .collect();
        unsafe {
            ffi::SetMusicVolume(hum, 0.0);
            ffi::SetMusicVolume(air, 0.0);
            ffi::PlayMusicStream(hum);
            ffi::PlayMusicStream(air);
        }
        Self {
            hum,
            air,
            creaks,
            next_creak: 6.0,
            rng,
        }
    }

    /// Keeps the streams fed and sets the beds for the player's `height` and the daylight
    /// (0 night .. 1 day). The hum is strongest in the depths, the wind in the open sky.
    pub fn update(&mut self, dt: f32, height: f32, day: f32, master: f32) {
        let up = ((height + 60.0) / 100.0).clamp(0.0, 1.0);
        let hum = (0.34 - 0.2 * up) * (1.0 + 0.15 * (1.0 - day)) * master;
        let air = (0.08 + 0.3 * up * up) * (0.7 + 0.3 * day) * master;
        unsafe {
            ffi::UpdateMusicStream(self.hum);
            ffi::UpdateMusicStream(self.air);
            ffi::SetMusicVolume(self.hum, hum);
            ffi::SetMusicVolume(self.air, air);
        }
        self.next_creak -= dt;
        if self.next_creak <= 0.0 && master > 0.0 {
            self.next_creak = self.rng.range(9.0, 26.0);
            let i = self.rng.range_i(0, self.creaks.len() as i32) as usize;
            let s = self.creaks[i];
            unsafe {
                ffi::SetSoundPitch(s, self.rng.range(0.7, 1.25));
                ffi::SetSoundVolume(s, self.rng.range(0.1, 0.3) * master);
                ffi::SetSoundPan(s, self.rng.range(-0.9, 0.9));
                ffi::PlaySound(s);
            }
        }
    }
}

impl Drop for Ambient {
    fn drop(&mut self) {
        unsafe {
            ffi::UnloadMusicStream(self.hum);
            ffi::UnloadMusicStream(self.air);
            for s in &self.creaks {
                ffi::UnloadSound(*s);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats(data: &[f32]) -> (f32, f32) {
        let peak = data.iter().fold(0.0_f32, |m, v| m.max(v.abs()));
        let rms = (data.iter().map(|v| v * v).sum::<f32>() / data.len() as f32).sqrt();
        (peak, rms)
    }

    #[test]
    fn beds_are_finite_quiet_enough_and_loop_without_a_seam() {
        let mut rng = Rng::new(1);
        for (name, data) in [("hum", hum(&mut rng)), ("air", air(&mut rng))] {
            assert!(data.iter().all(|v| v.is_finite()), "{name} has NaN");
            let (peak, rms) = stats(&data);
            assert!(peak < 0.95, "{name} peak {peak} would clip");
            assert!(rms > 0.01, "{name} is nearly silent: rms {rms}");
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
                "{name} seam jump {jump} vs typical step {typical}"
            );
        }
    }

    #[test]
    fn creaks_are_finite_and_do_not_clip() {
        let mut rng = Rng::new(2);
        for _ in 0..6 {
            let c = creak(&mut rng);
            assert!(c.iter().all(|v| v.is_finite()));
            let (peak, rms) = stats(&c);
            assert!(peak < 1.0 && rms > 0.001, "creak peak {peak} rms {rms}");
        }
    }

    #[test]
    fn wav_header_is_well_formed() {
        let v = wav_bytes(&[0.0, 0.5, -0.5, 1.0], 2);
        assert_eq!(&v[0..4], b"RIFF");
        assert_eq!(v.len(), 44 + 8);
        assert_eq!(u32::from_le_bytes(v[40..44].try_into().unwrap()), 8);
    }
}
