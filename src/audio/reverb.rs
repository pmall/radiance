//! A stereo reverb on the whole mix (Freeverb: parallel damped combs into series allpasses), so
//! every note and creak rings into the empty canyons of the city. Runs in raylib's audio thread
//! as a mixed-output processor.

use std::ffi::c_void;
use std::sync::{Mutex, OnceLock};

use sola_raylib::ffi;

const COMBS: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
const ALLPASSES: [usize; 4] = [556, 441, 341, 225];
const STEREO_SPREAD: usize = 23;
/// The device runs at its native rate, almost always 48 kHz; the tunings are for 44.1 kHz.
const RATE_SCALE: f32 = 48000.0 / 44100.0;

const ROOM: f32 = 0.9;
const DAMP: f32 = 0.35;
const WET: f32 = 0.3;
const FIXED_GAIN: f32 = 0.015;

struct Comb {
    buf: Vec<f32>,
    pos: usize,
    store: f32,
}

impl Comb {
    fn new(len: usize) -> Self {
        Self {
            buf: vec![0.0; len],
            pos: 0,
            store: 0.0,
        }
    }

    fn process(&mut self, x: f32) -> f32 {
        let out = self.buf[self.pos];
        self.store = out * (1.0 - DAMP) + self.store * DAMP;
        self.buf[self.pos] = x + self.store * (0.7 + 0.28 * ROOM);
        self.pos = (self.pos + 1) % self.buf.len();
        out
    }
}

struct Allpass {
    buf: Vec<f32>,
    pos: usize,
}

impl Allpass {
    fn new(len: usize) -> Self {
        Self {
            buf: vec![0.0; len],
            pos: 0,
        }
    }

    fn process(&mut self, x: f32) -> f32 {
        let delayed = self.buf[self.pos];
        let out = delayed - x;
        self.buf[self.pos] = x + delayed * 0.5;
        self.pos = (self.pos + 1) % self.buf.len();
        out
    }
}

struct Channel {
    combs: Vec<Comb>,
    allpasses: Vec<Allpass>,
}

impl Channel {
    fn new(spread: usize) -> Self {
        let scale = |n: usize| ((n + spread) as f32 * RATE_SCALE) as usize;
        Self {
            combs: COMBS.iter().map(|&n| Comb::new(scale(n))).collect(),
            allpasses: ALLPASSES.iter().map(|&n| Allpass::new(scale(n))).collect(),
        }
    }

    fn process(&mut self, x: f32) -> f32 {
        let input = x * FIXED_GAIN;
        let mut out: f32 = self.combs.iter_mut().map(|c| c.process(input)).sum();
        for a in &mut self.allpasses {
            out = a.process(out);
        }
        out
    }
}

struct Reverb {
    left: Channel,
    right: Channel,
}

static REVERB: OnceLock<Mutex<Reverb>> = OnceLock::new();

/// Called by raylib's audio thread with interleaved stereo f32 frames.
unsafe extern "C" fn process(buffer: *mut c_void, frames: u32) {
    let Some(lock) = REVERB.get() else { return };
    let Ok(mut r) = lock.try_lock() else { return };
    let data = unsafe { std::slice::from_raw_parts_mut(buffer as *mut f32, frames as usize * 2) };
    for frame in data.as_chunks_mut::<2>().0 {
        let mono = (frame[0] + frame[1]) * 0.5;
        // Left and right tanks share the input and differ by their delay spread.
        let (l, rr) = (r.left.process(mono), r.right.process(mono));
        frame[0] += l * WET * 3.0;
        frame[1] += rr * WET * 3.0;
    }
}

/// Starts the reverb on the master bus. Needs an initialized audio device.
pub fn attach() {
    REVERB.get_or_init(|| {
        Mutex::new(Reverb {
            left: Channel::new(0),
            right: Channel::new(STEREO_SPREAD),
        })
    });
    unsafe { ffi::AttachAudioMixedProcessor(Some(process)) };
}

/// Stops the reverb (before closing the audio device).
pub fn detach() {
    unsafe { ffi::DetachAudioMixedProcessor(Some(process)) };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impulse_response_rings_then_decays_and_stays_bounded() {
        let mut ch = Channel::new(0);
        let n = 48000 * 12;
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            out.push(ch.process(if i == 0 { 1.0 } else { 0.0 }));
        }
        assert!(out.iter().all(|v| v.is_finite() && v.abs() < 2.0));
        let energy = |a: usize, b: usize| out[a..b].iter().map(|v| v * v).sum::<f32>();
        let early = energy(0, 48000);
        let late = energy(48000 * 10, n);
        assert!(early > 0.0, "no reverb output");
        assert!(
            late < early * 0.01,
            "tail does not decay: {late} vs {early}"
        );
        // A tail that lasts: still audible after 2 s.
        assert!(energy(48000 * 2, 48000 * 3) > early * 1e-4);
    }
}
