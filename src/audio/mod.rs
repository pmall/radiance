//! Sound: each light flower is a human soul, and it sings a soft piano note when the machine
//! (the player) comes close. Notes come from a minor pentatonic scale, so walking through a
//! cluster plays a gentle melody; souls high in the city sing high, those in the depths low. A
//! synthesized hum, wind and distant creaks fill the city, and a reverb on the whole mix makes it
//! feel enormous and empty.

mod ambient;
mod piano;
mod reverb;

pub use reverb::{start_capture, stop_capture, take_capture};

use std::collections::HashMap;

use glam::Vec3;
use sola_raylib::ffi;

use crate::rng::{Rng, hash_coords};
use crate::world::World;
use crate::world::city::LOT;

/// Distance at which a soul starts to sing.
const REACH: f32 = 6.5;
/// A soul stays quiet for this long after singing, in seconds.
const COOLDOWN: f32 = 45.0;
/// Delay between notes started by one step into a cluster, so they arpeggiate.
const STAGGER: f32 = 0.2;
/// Most notes waiting to sound.
const MAX_QUEUE: usize = 6;
const SALT_NOTE: i32 = 7;

/// A soul that has just started to sing: its lot and its index in that lot's lights.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Soul {
    pub lot: (i32, i32),
    pub index: usize,
}

struct Pending {
    at: f32,
    note: i32,
    volume: f32,
    pan: f32,
}

pub struct Audio {
    piano: piano::Piano,
    ambient: ambient::Ambient,
    seed: u64,
    clock: f32,
    last_sung: HashMap<Soul, f32>,
    queue: Vec<Pending>,
    rng: Rng,
    pub muted: bool,
    /// Notes played since the start, for the debug overlay.
    pub notes_played: u32,
}

impl Audio {
    /// Opens the audio device. `None` when there is no usable device.
    pub fn new(seed: u64) -> Option<Self> {
        unsafe {
            ffi::InitAudioDevice();
            if !ffi::IsAudioDeviceReady() {
                return None;
            }
        }
        reverb::attach();
        Some(Self {
            piano: piano::Piano::new(),
            ambient: ambient::Ambient::new(seed),
            seed,
            clock: 0.0,
            last_sung: HashMap::new(),
            queue: Vec::new(),
            rng: Rng::new(seed ^ 0x50_1A60),
            muted: false,
            notes_played: 0,
        })
    }

    /// A new world: souls sing again.
    pub fn reseed(&mut self, seed: u64) {
        self.seed = seed;
        self.last_sung.clear();
        self.queue.clear();
    }

    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
    }

    /// Advances the sound: starts the notes of souls the player has come near, plays the ones due,
    /// and sets the ambient beds for the player's height and the daylight. `right` is the
    /// listener's right-hand direction. Returns the souls that started to sing this call.
    pub fn update(
        &mut self,
        dt: f32,
        eye: Vec3,
        right: Vec3,
        day: f32,
        world: &World,
    ) -> Vec<Soul> {
        self.clock += dt;
        let master = if self.muted { 0.0 } else { 1.0 };
        self.ambient.update(dt, eye.y, day, master);

        let mut started = Vec::new();
        if !self.muted {
            let here = ((eye.x / LOT).floor() as i32, (eye.z / LOT).floor() as i32);
            for (key, lot) in world.lots() {
                if (key.0 - here.0).abs() > 1 || (key.1 - here.1).abs() > 1 {
                    continue;
                }
                for (index, light) in lot.lights.iter().enumerate() {
                    let d = light.pos.distance(eye);
                    // Only souls in the same open space: not through a deck.
                    if d > REACH || eye.y < light.y_lo - 0.5 || eye.y > light.y_hi + 0.5 {
                        continue;
                    }
                    let soul = Soul { lot: key, index };
                    if self.clock - self.last_sung.get(&soul).copied().unwrap_or(-COOLDOWN)
                        < COOLDOWN
                    {
                        continue;
                    }
                    if self.queue.len() >= MAX_QUEUE {
                        break;
                    }
                    self.last_sung.insert(soul, self.clock);
                    let nth = started.len() as f32;
                    let to = (light.pos - eye).normalize_or_zero();
                    self.queue.push(Pending {
                        at: self.clock + nth * STAGGER + self.rng.range(0.0, 0.12),
                        note: note_for(self.seed, soul, light.pos.y, &mut self.rng),
                        // The samples are recorded loud: keep the souls' voices soft next to the ambience.
                        volume: 0.25 + 0.4 * (1.0 - d / REACH),
                        pan: (to.dot(right) * 0.85).clamp(-1.0, 1.0),
                    });
                    started.push(soul);
                }
            }
            if self.last_sung.len() > 600 {
                let now = self.clock;
                self.last_sung.retain(|_, t| now - *t < COOLDOWN);
            }
        }

        let clock = self.clock;
        let log = std::env::var_os("RADIANCE_AUDIO_LOG").is_some();
        let piano = &mut self.piano;
        let mut played = 0;
        self.queue.retain(|p| {
            if p.at > clock {
                return true;
            }
            if log {
                eprintln!("NOTE {} vol {:.2} pan {:.2}", p.note, p.volume, p.pan);
            }
            piano.play(p.note, p.volume, p.pan);
            played += 1;
            false
        });
        self.notes_played += played;
        started
    }
}

/// The note (MIDI key) a soul sings: a pitch of the scale chosen from where the soul is, in a
/// register that follows its height (low in the depths, high in the canopy), always the same one
/// for the same soul.
fn note_for(seed: u64, soul: Soul, height: f32, rng: &mut Rng) -> i32 {
    let h = hash_coords(seed, soul.lot.0, SALT_NOTE + soul.index as i32, soul.lot.1);
    let pitch_class = piano::SCALE[(h % 5) as usize];
    // Register center: D3 deep down to A5 high up, a little spread so neighbors differ.
    let t = ((height + 80.0) / 130.0).clamp(0.0, 1.0);
    let center = 50.0 + 30.0 * t + ((h >> 8) % 9) as f32 - 4.0;
    let _ = rng;
    // The key with that pitch class closest to the center.
    (0..128)
        .filter(|k| k % 12 == pitch_class)
        .min_by(|a, b| {
            (*a as f32 - center)
                .abs()
                .total_cmp(&(*b as f32 - center).abs())
        })
        .unwrap_or(60)
}

impl Drop for Audio {
    fn drop(&mut self) {
        reverb::detach();
        // Ambient and piano free their sounds as they drop, before the device closes.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_stay_in_the_scale_follow_height_and_repeat() {
        let mut rng = Rng::new(0);
        let (mut deep, mut high) = (0.0, 0.0);
        for index in 0..60 {
            let soul = Soul {
                lot: (3, -2),
                index,
            };
            let low = note_for(5, soul, -75.0, &mut rng);
            let up = note_for(5, soul, 40.0, &mut rng);
            assert!(piano::SCALE.contains(&(low % 12)) && piano::SCALE.contains(&(up % 12)));
            assert_eq!(
                low,
                note_for(5, soul, -75.0, &mut rng),
                "same soul, same note"
            );
            deep += low as f32;
            high += up as f32;
        }
        assert!(
            high > deep + 60.0 * 15.0,
            "high souls should sing higher: {high} vs {deep}"
        );
    }
}
