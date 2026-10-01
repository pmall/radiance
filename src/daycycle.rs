//! Time of day clock. `t` runs over [0, 1): 0 is midnight, 0.25 sunrise, 0.5 noon,
//! 0.75 sunset.

use std::f32::consts::TAU;

use glam::{Vec3, vec3};

/// Full cycle duration at speed 1, in seconds.
pub const DEFAULT_DURATION: f32 = 20.0 * 60.0;

pub struct DayCycle {
    pub t: f32,
    pub running: bool,
    pub speed: f32,
    pub duration: f32,
}

impl DayCycle {
    pub fn new(t: f32) -> Self {
        Self {
            t,
            running: true,
            speed: 1.0,
            duration: DEFAULT_DURATION,
        }
    }

    pub fn update(&mut self, dt: f32) {
        if self.running {
            self.advance(dt * self.speed / self.duration);
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

    pub fn clock(&self) -> String {
        let minutes = (self.t * 24.0 * 60.0) as u32;
        format!("{:02}:{:02}", minutes / 60, minutes % 60)
    }
}
