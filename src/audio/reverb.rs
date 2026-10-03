//! A stereo reverb on the whole mix (Freeverb: parallel damped combs into series allpasses), so
//! every note and creak rings into the empty canyons of the city. Runs in raylib's audio thread
//! as a mixed-output processor.

use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

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

/// Limiter statistics for the debug log: frames seen, frames where the limiter changed the sound,
/// and the loudest sample before limiting (f32 bits).
static STAT_FRAMES: AtomicU64 = AtomicU64::new(0);
static STAT_LIMITED: AtomicU64 = AtomicU64::new(0);
static STAT_PEAK: AtomicU32 = AtomicU32::new(0);

/// (frames, limited frames, peak before limiting) since the last call.
pub fn take_limiter_stats() -> (u64, u64, f32) {
    (
        STAT_FRAMES.swap(0, Ordering::Relaxed),
        STAT_LIMITED.swap(0, Ordering::Relaxed),
        f32::from_bits(STAT_PEAK.swap(0, Ordering::Relaxed)),
    )
}

/// A copy of the final mix, kept while a recording runs (see `crate::recorder`).
struct Capture {
    on: bool,
    /// Interleaved stereo f32 frames not yet taken.
    samples: Vec<f32>,
    frames: u64,
    first: Option<Instant>,
    last: Option<Instant>,
}

static CAPTURE: Mutex<Capture> = Mutex::new(Capture {
    on: false,
    samples: Vec::new(),
    frames: 0,
    first: None,
    last: None,
});

/// Starts keeping a copy of everything that goes to the speakers.
pub fn start_capture() {
    if let Ok(mut c) = CAPTURE.lock() {
        *c = Capture {
            on: true,
            samples: Vec::new(),
            frames: 0,
            first: None,
            last: None,
        };
    }
}

/// Takes the samples captured since the last call (interleaved stereo f32).
pub fn take_capture() -> Vec<f32> {
    CAPTURE
        .lock()
        .map(|mut c| std::mem::take(&mut c.samples))
        .unwrap_or_default()
}

/// Stops capturing. Returns the device's real sample rate, measured from how many frames it asked
/// for over the capture's wall-clock time (the device rate is not exposed by raylib).
pub fn stop_capture() -> Option<f32> {
    let mut c = CAPTURE.lock().ok()?;
    c.on = false;
    let (first, last) = (c.first?, c.last?);
    let secs = last.duration_since(first).as_secs_f32();
    (secs > 0.5).then(|| c.frames as f32 / secs)
}

/// Called by raylib's audio thread with interleaved stereo f32 frames.
unsafe extern "C" fn process(buffer: *mut c_void, frames: u32) {
    let Some(lock) = REVERB.get() else { return };
    let data = unsafe { std::slice::from_raw_parts_mut(buffer as *mut f32, frames as usize * 2) };
    if let Ok(mut r) = lock.try_lock() {
        reverberate(&mut r, data);
    }
    // A blocking lock: the main thread only holds it for an instant, and skipping would leave a
    // hole in the recording.
    if let Ok(mut c) = CAPTURE.lock()
        && c.on
    {
        let now = Instant::now();
        c.first.get_or_insert(now);
        c.last = Some(now);
        c.frames += frames as u64;
        c.samples.extend_from_slice(data);
    }
}

fn reverberate(r: &mut Reverb, data: &mut [f32]) {
    for frame in data.as_chunks_mut::<2>().0 {
        let mono = (frame[0] + frame[1]) * 0.5;
        // Left and right tanks share the input and differ by their delay spread.
        let (l, rr) = (r.left.process(mono), r.right.process(mono));
        let (a, b) = (frame[0] + l * WET * 3.0, frame[1] + rr * WET * 3.0);
        let peak = a.abs().max(b.abs());
        STAT_FRAMES.fetch_add(1, Ordering::Relaxed);
        if peak > 0.8 {
            STAT_LIMITED.fetch_add(1, Ordering::Relaxed);
        }
        STAT_PEAK.fetch_max(peak.to_bits(), Ordering::Relaxed);
        frame[0] = soft_limit(a);
        frame[1] = soft_limit(b);
    }
}

/// Keeps loud moments (several notes ringing at once) under 1.0 without harsh clipping: linear
/// up to 0.8, then a smooth knee toward 1.
fn soft_limit(x: f32) -> f32 {
    const KNEE: f32 = 0.8;
    let a = x.abs();
    if a <= KNEE {
        x
    } else {
        x.signum() * (KNEE + (1.0 - KNEE) * ((a - KNEE) / (1.0 - KNEE)).tanh())
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
    fn soft_limit_is_transparent_below_the_knee_and_bounded_above() {
        for x in [-0.8, -0.3, 0.0, 0.5, 0.8] {
            assert_eq!(soft_limit(x), x);
        }
        let mut last = 0.0;
        for i in 0..=400 {
            let y = soft_limit(i as f32 * 0.01);
            assert!(y >= last && y < 1.0 + 1e-6, "{y}");
            last = y;
        }
    }

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
