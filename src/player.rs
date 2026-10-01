//! First-person controller: walk, run, jump, generous ledge grabbing, and a
//! noclip free-fly mode for debugging.

use glam::{Vec2, Vec3, vec3};

use crate::world::{Aabb, World};

const RADIUS: f32 = 0.35;
const HEIGHT: f32 = 1.8;
const EYE_HEIGHT: f32 = 1.62;
const STEP_HEIGHT: f32 = 0.55;

const WALK_SPEED: f32 = 6.0;
const RUN_SPEED: f32 = 11.0;
const GROUND_RESPONSE: f32 = 12.0;
const AIR_RESPONSE: f32 = 2.5;
const GRAVITY: f32 = 24.0;
const JUMP_SPEED: f32 = 8.2;
const MAX_FALL_SPEED: f32 = 50.0;
const COYOTE_TIME: f32 = 0.12;
const JUMP_BUFFER: f32 = 0.15;

/// How far above the feet a ledge can be and still be grabbed (generous on purpose).
const LEDGE_MIN: f32 = STEP_HEIGHT + 0.05;
const LEDGE_MAX: f32 = 2.7;
const LEDGE_PROBE: f32 = 0.5;

const FLY_SPEED: f32 = 20.0;
const FLY_FAST_MULT: f32 = 5.0;

const BASE_FOV: f32 = 75.0;
const RUN_FOV_KICK: f32 = 7.0;
const MOUSE_SENS: f32 = 0.0022;

#[derive(Default, Clone, Copy)]
pub struct Input {
    /// x: right, y: forward, each in [-1, 1].
    pub wish: Vec2,
    pub look: Vec2,
    pub jump_pressed: bool,
    pub jump_held: bool,
    pub run: bool,
    pub down: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Walk,
    Mantle {
        from: Vec3,
        to: Vec3,
        t: f32,
        duration: f32,
    },
    Fly,
}

pub struct Player {
    /// Feet position.
    pub pos: Vec3,
    pub vel: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    mode: Mode,
    grounded: bool,
    coyote: f32,
    jump_buffer: f32,
    /// Smooths the camera over step-ups so they don't snap.
    eye_offset: f32,
    /// Camera dip after a hard landing, easing back out.
    landing_dip: f32,
    bob_phase: f32,
    bob_amount: f32,
    pub fov: f32,
}

impl Player {
    pub fn new(pos: Vec3) -> Self {
        Self {
            pos,
            vel: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            mode: Mode::Walk,
            grounded: false,
            coyote: 0.0,
            jump_buffer: 0.0,
            eye_offset: 0.0,
            landing_dip: 0.0,
            bob_phase: 0.0,
            bob_amount: 0.0,
            fov: BASE_FOV,
        }
    }

    pub fn is_flying(&self) -> bool {
        self.mode == Mode::Fly
    }

    pub fn state_name(&self) -> &'static str {
        match self.mode {
            Mode::Walk if self.grounded => "ground",
            Mode::Walk => "air",
            Mode::Mantle { .. } => "mantle",
            Mode::Fly => "fly",
        }
    }

    pub fn toggle_fly(&mut self) {
        self.mode = if self.mode == Mode::Fly {
            Mode::Walk
        } else {
            Mode::Fly
        };
        self.vel = Vec3::ZERO;
    }

    pub fn forward_flat(&self) -> Vec3 {
        vec3(-self.yaw.sin(), 0.0, -self.yaw.cos())
    }

    pub fn right(&self) -> Vec3 {
        vec3(self.yaw.cos(), 0.0, -self.yaw.sin())
    }

    pub fn look_dir(&self) -> Vec3 {
        let (sp, cp) = self.pitch.sin_cos();
        self.forward_flat() * cp + Vec3::Y * sp
    }

    pub fn eye(&self) -> Vec3 {
        let bob = (self.bob_phase).sin() * 0.045 * self.bob_amount;
        self.pos + Vec3::Y * (EYE_HEIGHT + self.eye_offset - self.landing_dip + bob)
    }

    fn aabb_at(pos: Vec3) -> Aabb {
        Aabb {
            min: pos - vec3(RADIUS, 0.0, RADIUS),
            max: pos + vec3(RADIUS, HEIGHT, RADIUS),
        }
    }

    /// Look is applied once per frame; simulation may be sub-stepped.
    pub fn apply_look(&mut self, look: Vec2) {
        self.yaw -= look.x * MOUSE_SENS;
        self.pitch = (self.pitch - look.y * MOUSE_SENS).clamp(-1.55, 1.55);
    }

    pub fn update(&mut self, dt: f32, input: &Input, world: &World) {
        match self.mode {
            Mode::Fly => self.update_fly(dt, input),
            Mode::Mantle {
                from,
                to,
                t,
                duration,
            } => self.update_mantle(dt, from, to, t, duration),
            Mode::Walk => self.update_walk(dt, input, world),
        }

        self.eye_offset *= (-14.0 * dt).exp();
        self.landing_dip *= (-8.0 * dt).exp();
        let running = input.run && input.wish.y > 0.1 && self.mode == Mode::Walk;
        let target_fov = BASE_FOV + if running { RUN_FOV_KICK } else { 0.0 };
        self.fov += (target_fov - self.fov) * (1.0 - (-6.0 * dt).exp());
    }

    fn update_fly(&mut self, dt: f32, input: &Input) {
        let mut dir = self.look_dir() * input.wish.y + self.right() * input.wish.x;
        if input.jump_held {
            dir += Vec3::Y;
        }
        if input.down {
            dir -= Vec3::Y;
        }
        let speed = FLY_SPEED * if input.run { FLY_FAST_MULT } else { 1.0 };
        let target = dir.normalize_or_zero() * speed;
        self.vel += (target - self.vel) * (1.0 - (-8.0 * dt).exp());
        self.pos += self.vel * dt;
        self.bob_amount = 0.0;
    }

    fn update_mantle(&mut self, dt: f32, from: Vec3, to: Vec3, t: f32, duration: f32) {
        let t = (t + dt / duration).min(1.0);
        // Rise first, then roll forward over the lip.
        let rise = smoothstep(0.0, 0.65, t);
        let over = smoothstep(0.35, 1.0, t);
        self.pos = vec3(
            from.x + (to.x - from.x) * over,
            from.y + (to.y - from.y) * rise,
            from.z + (to.z - from.z) * over,
        );
        if t >= 1.0 {
            self.mode = Mode::Walk;
            self.grounded = true;
            self.vel = self.forward_flat() * 3.0;
        } else {
            self.mode = Mode::Mantle {
                from,
                to,
                t,
                duration,
            };
        }
    }

    fn update_walk(&mut self, dt: f32, input: &Input, world: &World) {
        let fwd = self.forward_flat();
        let wish_dir = (fwd * input.wish.y + self.right() * input.wish.x).clamp_length_max(1.0);
        let speed = if input.run { RUN_SPEED } else { WALK_SPEED };
        let target = wish_dir * speed;

        let response = if self.grounded {
            GROUND_RESPONSE
        } else {
            AIR_RESPONSE
        };
        let k = 1.0 - (-response * dt).exp();
        self.vel.x += (target.x - self.vel.x) * k;
        self.vel.z += (target.z - self.vel.z) * k;

        // Jumping with coyote time and input buffering.
        self.coyote = if self.grounded {
            COYOTE_TIME
        } else {
            self.coyote - dt
        };
        self.jump_buffer = if input.jump_pressed {
            JUMP_BUFFER
        } else {
            self.jump_buffer - dt
        };
        if self.jump_buffer > 0.0 && self.coyote > 0.0 {
            self.vel.y = JUMP_SPEED;
            self.jump_buffer = 0.0;
            self.coyote = 0.0;
            self.grounded = false;
        }

        // Releasing jump early cuts the arc short for finer control.
        let gravity = if self.vel.y > 0.0 && !input.jump_held {
            GRAVITY * 2.2
        } else {
            GRAVITY
        };
        self.vel.y = (self.vel.y - gravity * dt).max(-MAX_FALL_SPEED);

        // Horizontal movement with automatic step-up.
        let horiz = vec3(self.vel.x, 0.0, self.vel.z) * dt;
        let before = self.pos;
        let vel_before = self.vel;
        let blocked = self.move_horizontal(horiz, world);
        if blocked && self.grounded {
            let raised = before + Vec3::Y * STEP_HEIGHT;
            if !world.collides(&Self::aabb_at(raised)) {
                let saved = self.pos;
                self.pos = raised;
                if self.move_horizontal(horiz, world) {
                    self.pos = saved;
                } else {
                    self.vel = vel_before;
                    let landed = self.drop_to_ground(STEP_HEIGHT, world);
                    self.eye_offset -= landed - before.y;
                }
            }
        }

        // Vertical movement.
        let was_grounded = self.grounded;
        self.grounded = false;
        let dy = self.vel.y * dt;
        self.pos.y += dy;
        let me = Self::aabb_at(self.pos);
        let impact = -self.vel.y;
        for c in world.colliders_near(&me) {
            if !me.overlaps(c) {
                continue;
            }
            if dy <= 0.0 {
                self.pos.y = c.max.y;
                self.grounded = true;
            } else {
                self.pos.y = c.min.y - HEIGHT - 1e-4;
            }
            self.vel.y = 0.0;
        }
        if self.grounded && !was_grounded && impact > 6.0 {
            self.landing_dip = (impact * 0.012).min(0.3);
        }
        // Stick to the ground when walking down small steps.
        if was_grounded && !self.grounded && self.vel.y <= 0.0 {
            let y0 = self.pos.y;
            let landed = self.drop_to_ground(STEP_HEIGHT, world);
            if landed < y0 {
                self.grounded = true;
                self.eye_offset += y0 - landed;
            }
        }

        if self.grounded {
            let hspeed = vec3(self.vel.x, 0.0, self.vel.z).length();
            self.bob_phase += hspeed * dt * 1.6;
            self.bob_amount +=
                ((hspeed / RUN_SPEED).min(1.0) - self.bob_amount) * (1.0 - (-8.0 * dt).exp());
        } else {
            self.bob_amount *= (-6.0 * dt).exp();
        }

        if !self.grounded && input.wish.y > 0.3 {
            self.try_ledge_grab(world);
        }
    }

    /// Moves along X then Z, sliding along walls. Returns whether anything blocked us.
    fn move_horizontal(&mut self, delta: Vec3, world: &World) -> bool {
        let mut blocked = false;
        for axis in [0usize, 2] {
            let d = delta[axis];
            if d == 0.0 {
                continue;
            }
            self.pos[axis] += d;
            let me = Self::aabb_at(self.pos);
            for c in world.colliders_near(&me) {
                if me.overlaps(c) {
                    self.pos[axis] = if d > 0.0 {
                        c.min[axis] - RADIUS - 1e-4
                    } else {
                        c.max[axis] + RADIUS + 1e-4
                    };
                    self.vel[axis] = 0.0;
                    blocked = true;
                }
            }
        }
        blocked
    }

    /// Lowers the player onto the highest surface within `max_drop`. Returns the new feet height.
    fn drop_to_ground(&mut self, max_drop: f32, world: &World) -> f32 {
        let probe = Aabb {
            min: self.pos - vec3(RADIUS, max_drop, RADIUS),
            max: self.pos + vec3(RADIUS, 0.01, RADIUS),
        };
        let top = world
            .colliders_near(&probe)
            .filter(|c| c.overlaps(&probe) && c.max.y <= self.pos.y + 0.01)
            .map(|c| c.max.y)
            .fold(f32::NEG_INFINITY, f32::max);
        if top.is_finite() {
            self.pos.y = top;
        }
        self.pos.y
    }

    fn try_ledge_grab(&mut self, world: &World) {
        let fwd = self.forward_flat();
        let probe = self.pos + fwd * (RADIUS + LEDGE_PROBE);
        let probe_box = Aabb {
            min: probe - Vec3::splat(0.01),
            max: probe + Vec3::splat(0.01),
        };
        let ledge = world
            .colliders_near(&probe_box)
            .filter(|c| c.contains_xz(probe))
            .map(|c| c.max.y)
            .filter(|&top| top >= self.pos.y + LEDGE_MIN && top <= self.pos.y + LEDGE_MAX)
            .fold(f32::NEG_INFINITY, f32::max);
        if !ledge.is_finite() {
            return;
        }
        // Need headroom to climb straight up, then room to stand on the lip.
        let up = vec3(self.pos.x, ledge + 0.01, self.pos.z);
        let to = vec3(probe.x, ledge + 0.01, probe.z) + fwd * 0.2;
        if world.collides(&Self::aabb_at(up)) || world.collides(&Self::aabb_at(to)) {
            return;
        }
        let height = ledge - self.pos.y;
        self.vel = Vec3::ZERO;
        self.mode = Mode::Mantle {
            from: self.pos,
            to,
            t: 0.0,
            duration: 0.22 + height * 0.09,
        };
    }
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
