//! Time of day clock. `t` runs over [0, 1): 0 is midnight, 0.25 sunrise, 0.5 noon,
//! 0.75 sunset.

use std::f32::consts::TAU;

use glam::{Vec3, vec3};

/// Full cycle duration at speed 1, in seconds.
const DURATION: f32 = 20.0 * 60.0;

pub struct DayCycle {
    pub t: f32,
    pub running: bool,
    pub speed: f32,
}

impl DayCycle {
    pub fn new(t: f32) -> Self {
        Self {
            t,
            running: true,
            speed: 1.0,
        }
    }

    pub fn update(&mut self, dt: f32) {
        if self.running {
            self.advance(dt * self.speed / DURATION);
        }
    }

    /// Moves the clock by a fraction of a day (negative goes back).
    pub fn advance(&mut self, dt: f32) {
        self.t = (self.t + dt).rem_euclid(1.0);
    }

    /// Unit vector toward the sun. Below the horizon at night.
    pub fn sun_dir(&self) -> Vec3 {
        let a = (self.t - 0.25) * TAU;
        vec3(a.cos(), a.sin() * 0.8, 0.35).normalize()
    }

    /// How much of the day it is, from 0 (the sun well below the horizon) to 1 (well above it):
    /// a ramp through dawn and dusk, for what changes with daylight (the hum).
    pub fn daylight(&self) -> f32 {
        ((self.sun_dir().y + 0.1) / 0.4).clamp(0.0, 1.0)
    }

    pub fn clock(&self) -> String {
        let minutes = (self.t * 24.0 * 60.0) as u32;
        format!("{:02}:{:02}", minutes / 60, minutes % 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noon_is_day_and_midnight_night() {
        let at = |t| DayCycle::new(t);
        assert!(at(0.5).sun_dir().y > 0.5);
        assert!(at(0.0).sun_dir().y < -0.5);
        assert_eq!(at(0.5).daylight(), 1.0);
        assert_eq!(at(0.0).daylight(), 0.0);
        // Dawn is between, and the light grows through it.
        let (before, after) = (at(0.24).daylight(), at(0.27).daylight());
        assert!(0.0 < before && before < after && after < 1.0);
        for t in [0.0, 0.1, 0.3, 0.5, 0.8] {
            assert!((at(t).sun_dir().length() - 1.0).abs() < 1e-5);
        }
    }

    #[test]
    fn the_clock_wraps_and_reads_in_hours() {
        let mut c = DayCycle::new(0.9);
        c.advance(0.2);
        assert!((c.t - 0.1).abs() < 1e-5);
        c.advance(-0.2);
        assert!((c.t - 0.9).abs() < 1e-5);
        assert_eq!(DayCycle::new(0.5).clock(), "12:00");
        assert_eq!(DayCycle::new(0.75).clock(), "18:00");
        c.running = false;
        c.update(10.0);
        assert!((c.t - 0.9).abs() < 1e-5);
    }
}
