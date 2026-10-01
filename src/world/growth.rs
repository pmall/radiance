//! Overgrowth: vines, wall climbers and bushes layered over a lot's architecture.
//!
//! Plants are decorative triangle geometry (no collision), derived from the lot's own blocks and
//! seeded per lot, so they stay deterministic and independent of neighbors. Growth is denser
//! toward the dark depths.

use glam::{Vec3, vec3};
use sola_raylib::prelude::Color;

use super::{Block, Light, PlantVert};
use crate::rng::{Rng, hash_coords, mix64};

const SALT_GROWTH: i32 = 3;

/// Light sources kept per lot; the renderer picks the nearest ones overall.
const MAX_LIGHTS: usize = 24;
/// Emission of luminous foliage and of flowers, out of 255.
const FOLIAGE_GLOW: u8 = 110;
const FLOWER_GLOW: u8 = 235;

/// Blocks smaller than this on both horizontal axes (stair steps, parapets) stay bare.
const MIN_HOST: f32 = 3.5;
/// Towers at least this tall get wall climbers.
const MIN_WALL: f32 = 6.0;
/// Slabs thinner than this count as decks and bridges, which vines hang from.
const MAX_SLAB: f32 = 3.0;

/// Growth multiplier by height: light in the sunlit canopy, rampant in the depths.
fn density(y: f32) -> f32 {
    match y {
        y if y > -10.0 => 0.7,
        y if y > -50.0 => 1.3,
        _ => 2.0,
    }
}

fn leaf_color(rng: &mut Rng) -> Color {
    let c = |r: i32, g: i32, b: i32| Color::new(r as u8, g as u8, b as u8, 255);
    let j = rng.range_i(-14, 15);
    match rng.range_i(0, 6) {
        0 | 1 => c(52 + j, 130 + j, 84 + j),
        2 | 3 => c(34 + j, 112 + j, 104 + j),
        4 => c(96 + j, 150 + j, 70 + j),
        _ => c(70 + j, 100 + j, 60 + j),
    }
}

fn flower_color(rng: &mut Rng) -> Color {
    match rng.range_i(0, 3) {
        0 => Color::new(214, 92, 164, 255),
        1 => Color::new(96, 214, 220, 255),
        _ => Color::new(236, 196, 96, 255),
    }
}

/// Bioluminescent species colors (sRGB).
const SPECIES: [[u8; 3]; 4] = [
    [70, 230, 210],
    [120, 240, 110],
    [240, 100, 200],
    [140, 130, 255],
];

struct Builder<'a> {
    rng: &'a mut Rng,
    out: Vec<PlantVert>,
    lights: Vec<Light>,
    id: u8,
    /// Chance that a new plant is luminous (higher in the depths).
    lum_p: f32,
    /// Species color of the current plant if it is luminous.
    lum: Option<[u8; 3]>,
    glow: u8,
}

impl Builder<'_> {
    /// Starts a new plant: its own outline id, so it reads separately from its host.
    fn new_plant(&mut self) {
        self.id = 1 + (mix64(self.rng.next_u64()) % 255) as u8;
        self.lum = self
            .rng
            .chance(self.lum_p)
            .then(|| SPECIES[self.rng.range_i(0, SPECIES.len() as i32) as usize]);
        self.glow = if self.lum.is_some() { FOLIAGE_GLOW } else { 0 };
    }

    /// Leaf or stem color of the current plant: its species glow color, or ordinary green.
    fn foliage(&mut self) -> Color {
        match self.lum {
            Some([r, g, b]) => {
                let j = self.rng.range_i(-12, 13);
                let c = |v: u8| (v as i32 + j).clamp(0, 255) as u8;
                Color::new(c(r), c(g), c(b), 255)
            }
            None => leaf_color(self.rng),
        }
    }

    /// A small bright flower bud; flowers glow even on ordinary plants.
    fn flower_blob(&mut self, center: Vec3, radius: f32, color: Color) {
        let saved = std::mem::replace(&mut self.glow, FLOWER_GLOW);
        self.blob(center, Vec3::splat(radius), color);
        self.glow = saved;
    }

    /// A point light for the current plant, if it is luminous.
    fn light(&mut self, pos: Vec3, radius: f32) {
        if let Some([r, g, b]) = self.lum
            && self.lights.len() < MAX_LIGHTS
        {
            let lin = |v: u8| (v as f32 / 255.0).powf(2.2);
            self.lights.push(Light {
                pos,
                radius,
                color: vec3(lin(r), lin(g), lin(b)) * 2.0,
            });
        }
    }

    fn tri(&mut self, a: Vec3, b: Vec3, c: Vec3, n: Vec3, color: Color) {
        for p in [a, b, c] {
            self.out.push(PlantVert {
                pos: p,
                normal: n,
                color,
                id: self.id,
                glow: self.glow,
            });
        }
    }

    /// A quad visible from both sides.
    fn quad2(&mut self, a: Vec3, b: Vec3, c: Vec3, d: Vec3, color: Color) {
        let n = (b - a).cross(c - a).normalize_or_zero();
        if n == Vec3::ZERO {
            return;
        }
        self.tri(a, b, c, n, color);
        self.tri(a, c, d, n, color);
        self.tri(a, c, b, -n, color);
        self.tri(a, d, c, -n, color);
    }

    /// A triangle facing away from `inside`.
    fn tri_out(&mut self, a: Vec3, b: Vec3, c: Vec3, inside: Vec3, color: Color) {
        let mut n = (b - a).cross(c - a).normalize_or_zero();
        if n == Vec3::ZERO {
            return;
        }
        let (b, c) = if n.dot((a + b + c) / 3.0 - inside) < 0.0 {
            n = -n;
            (c, b)
        } else {
            (b, c)
        };
        self.tri(a, b, c, n, color);
    }

    /// A strip along `points`, `side` giving the width direction, tapering from `w0` to `w1`.
    fn ribbon(&mut self, points: &[Vec3], side: Vec3, w0: f32, w1: f32, color: Color) {
        let n = points.len();
        for i in 0..n.saturating_sub(1) {
            let t0 = i as f32 / (n - 1) as f32;
            let t1 = (i + 1) as f32 / (n - 1) as f32;
            let h0 = side * (w0 + (w1 - w0) * t0) * 0.5;
            let h1 = side * (w0 + (w1 - w0) * t1) * 0.5;
            self.quad2(
                points[i] - h0,
                points[i] + h0,
                points[i + 1] + h1,
                points[i + 1] - h1,
                color,
            );
        }
    }

    /// A diamond leaf pointing along `dir`, tilted by `up`.
    fn leaf(&mut self, base: Vec3, dir: Vec3, size: f32, color: Color) {
        let side = dir.cross(Vec3::Y).normalize_or_zero() * size * 0.28;
        let lift = Vec3::Y * size * 0.12;
        let mid = base + dir * size * 0.45 + lift;
        self.quad2(
            base,
            mid + side,
            base + dir * size + lift * 0.5,
            mid - side,
            color,
        );
    }

    /// A low-poly ellipsoid.
    fn blob(&mut self, center: Vec3, radii: Vec3, color: Color) {
        const RINGS: usize = 4;
        const SEGS: usize = 7;
        let mut verts = [[Vec3::ZERO; SEGS]; RINGS - 1];
        let jitter: Vec<f32> = (0..(RINGS - 1) * SEGS + 2)
            .map(|_| self.rng.range(0.82, 1.12))
            .collect();
        for (r, ring) in verts.iter_mut().enumerate() {
            let lat = std::f32::consts::PI * ((r + 1) as f32 / RINGS as f32 - 0.5);
            for (s, v) in ring.iter_mut().enumerate() {
                let lon = std::f32::consts::TAU * s as f32 / SEGS as f32;
                let j = jitter[r * SEGS + s];
                *v = center
                    + vec3(
                        lat.cos() * lon.cos() * radii.x,
                        lat.sin() * radii.y,
                        lat.cos() * lon.sin() * radii.z,
                    ) * j;
            }
        }
        let bottom = center - Vec3::Y * radii.y * jitter[jitter.len() - 2];
        let top = center + Vec3::Y * radii.y * jitter[jitter.len() - 1];
        for s in 0..SEGS {
            let s1 = (s + 1) % SEGS;
            self.tri_out(bottom, verts[0][s], verts[0][s1], center, color);
            for r in 0..RINGS - 2 {
                let (a, b) = (verts[r][s], verts[r][s1]);
                let (c, d) = (verts[r + 1][s1], verts[r + 1][s]);
                self.tri_out(a, b, c, center, color);
                self.tri_out(a, c, d, center, color);
            }
            self.tri_out(
                top,
                verts[RINGS - 2][s1],
                verts[RINGS - 2][s],
                center,
                color,
            );
        }
    }

    /// A few overlapping blobs sitting on a surface at `base`.
    fn bush(&mut self, base: Vec3) {
        self.new_plant();
        let color = self.foliage();
        let n = self.rng.range_i(3, 7);
        let scale = self.rng.range(1.0, 2.6);
        for _ in 0..n {
            let r = scale * self.rng.range(0.6, 1.1);
            let off = vec3(
                self.rng.range(-1.0, 1.0) * scale,
                0.0,
                self.rng.range(-1.0, 1.0) * scale,
            );
            let squash = self.rng.range(0.6, 0.9);
            self.blob(
                base + off + Vec3::Y * r * 0.45,
                vec3(r, r * squash, r),
                color,
            );
        }
        self.light(base + Vec3::Y * scale * 0.9, 6.0 + 3.0 * scale);
        if self.rng.chance(0.35) {
            let fc = flower_color(self.rng);
            for _ in 0..self.rng.range_i(2, 5) {
                let p = base
                    + vec3(
                        self.rng.range(-1.0, 1.0) * scale,
                        scale * self.rng.range(0.7, 1.2),
                        self.rng.range(-1.0, 1.0) * scale,
                    );
                self.flower_blob(p, 0.18, fc);
            }
        }
    }

    /// A vine hanging from `from`, swaying sideways, crossed ribbons so it has volume.
    fn hanging_vine(&mut self, from: Vec3, length: f32) {
        self.new_plant();
        const SEGS: usize = 6;
        let color = self.foliage();
        let phase = self.rng.range(0.0, std::f32::consts::TAU);
        let sway = self.rng.range(0.15, 0.7);
        let pts: Vec<Vec3> = (0..=SEGS)
            .map(|i| {
                let t = i as f32 / SEGS as f32;
                from + vec3(
                    (phase + t * 4.0).sin() * sway * t,
                    -length * t,
                    (phase * 1.7 + t * 3.0).cos() * sway * t,
                )
            })
            .collect();
        let w = self.rng.range(0.3, 0.55);
        self.ribbon(&pts, Vec3::X, w, w * 0.4, color);
        self.ribbon(&pts, Vec3::Z, w, w * 0.4, color);
        self.light(pts[SEGS / 2], 6.0);
        let leaf_col = self.foliage();
        for (i, p) in pts.iter().enumerate().skip(1) {
            if self.rng.chance(0.85) {
                let a = self.rng.range(0.0, std::f32::consts::TAU);
                let dir = vec3(a.cos(), -0.35, a.sin()).normalize();
                let size = self.rng.range(1.2, 2.2) * (1.0 - i as f32 / (SEGS + 4) as f32);
                self.leaf(*p, dir, size, leaf_col);
            }
        }
    }

    /// A branching climber growing up a wall. `origin` is the face's bottom-left point at the wall
    /// surface, `along` the face's horizontal unit axis, `normal` points away from the wall.
    fn climber(&mut self, origin: Vec3, along: Vec3, normal: Vec3, face_w: f32, y0: f32, y1: f32) {
        self.new_plant();
        let color = self.foliage();
        let leaf_col = self.foliage();
        let u0 = self.rng.range(0.5, face_w - 0.5);
        // (u, y, heading from straight up in radians, width, steps left, depth)
        let mut stack = vec![(u0, y0, 0.0_f32, 0.5_f32, 26, 0)];
        let mut budget = 300;
        while let Some((mut u, mut y, mut heading, mut w, mut steps, depth)) = stack.pop() {
            let mut pts = vec![origin + along * u + Vec3::Y * y + normal * 0.1];
            while steps > 0 && budget > 0 {
                steps -= 1;
                budget -= 1;
                heading = (heading + self.rng.range(-0.4, 0.4)) * 0.85;
                let step = 1.1;
                u = (u + heading.sin() * step).clamp(0.3, face_w - 0.3);
                y += heading.cos() * step;
                if y > y1 {
                    break;
                }
                let p = origin + along * u + Vec3::Y * y + normal * 0.1;
                pts.push(p);
                if pts.len() % 7 == 0 {
                    self.light(p + normal * 1.0, 6.0);
                }
                w *= 0.96;
                if self.rng.chance(0.8) {
                    let sgn = if self.rng.chance(0.5) { 1.0 } else { -1.0 };
                    let dir = (along * sgn * 0.8 + Vec3::Y * 0.3 + normal * 0.5).normalize();
                    let size = self.rng.range(1.0, 1.9);
                    self.leaf(p, dir, size, leaf_col);
                }
                if self.rng.chance(0.07) {
                    let flower = flower_color(self.rng);
                    self.flower_blob(p + normal * 0.3, 0.35, flower);
                }
                if depth < 3 && self.rng.chance(0.12) {
                    let side = if self.rng.chance(0.5) { 0.8 } else { -0.8 };
                    stack.push((u, y, heading + side, w * 0.7, steps / 2, depth + 1));
                }
            }
            self.ribbon_flat(&pts, along, normal, w, color);
        }
    }

    /// A ribbon lying on a wall: width along `side`, facing `normal`, one-sided is enough
    /// but double-sided keeps it simple and cheap.
    fn ribbon_flat(&mut self, pts: &[Vec3], side: Vec3, _normal: Vec3, w: f32, color: Color) {
        self.ribbon(pts, side, w, w * 0.6, color);
    }
}

/// Plant geometry for one lot, grown over the lot's blocks.
pub struct Growth {
    pub plants: Vec<PlantVert>,
    pub lights: Vec<Light>,
}

pub fn grow_lot(seed: u64, lx: i32, lz: i32, blocks: &[Block]) -> Growth {
    let mut rng = Rng::new(hash_coords(seed, lx, SALT_GROWTH, lz));
    let mut b = Builder {
        rng: &mut rng,
        out: Vec::new(),
        lights: Vec::new(),
        id: 1,
        lum_p: 0.0,
        lum: None,
        glow: 0,
    };

    for blk in blocks {
        let a = &blk.aabb;
        let size = a.size();
        if size.x.max(size.z) < MIN_HOST {
            continue;
        }
        let dens = density(a.max.y.min(a.min.y + 1.0));
        b.lum_p = 0.12 + 0.2 * (dens - 0.7);

        // Bushes on top surfaces.
        let area = size.x * size.z;
        let mut n = area * 0.02 * dens;
        while n > 0.0 {
            if n >= 1.0 || b.rng.chance(n) {
                let p = vec3(
                    b.rng
                        .range(a.min.x + 1.0, (a.max.x - 1.0).max(a.min.x + 1.01)),
                    a.max.y,
                    b.rng
                        .range(a.min.z + 1.0, (a.max.z - 1.0).max(a.min.z + 1.01)),
                );
                b.bush(p);
            }
            n -= 1.0;
        }

        // Hanging vines from the edges of decks and bridges.
        if size.y < MAX_SLAB && a.min.y > super::city::DEEP_FLOOR + 3.0 {
            let perimeter = 2.0 * (size.x + size.z);
            let mut n = perimeter * 0.3 * dens;
            while n > 0.0 {
                if n >= 1.0 || b.rng.chance(n) {
                    let t = b.rng.range(0.0, perimeter);
                    let (x, z) = perimeter_point(a.min, a.max, t);
                    let len = b.rng.range(3.0, 10.0 + 6.0 * dens);
                    b.hanging_vine(vec3(x, a.max.y - 0.1, z), len);
                }
                n -= 1.0;
            }
        }

        // Climbers up tower walls, rooted at one of the layers below the roof.
        if size.y > MIN_WALL {
            let faces = [
                (
                    vec3(a.min.x, 0.0, a.min.z),
                    Vec3::Z,
                    vec3(-1.0, 0.0, 0.0),
                    size.z,
                ),
                (
                    vec3(a.max.x, 0.0, a.min.z),
                    Vec3::Z,
                    vec3(1.0, 0.0, 0.0),
                    size.z,
                ),
                (
                    vec3(a.min.x, 0.0, a.min.z),
                    Vec3::X,
                    vec3(0.0, 0.0, -1.0),
                    size.x,
                ),
                (
                    vec3(a.min.x, 0.0, a.max.z),
                    Vec3::X,
                    vec3(0.0, 0.0, 1.0),
                    size.x,
                ),
            ];
            for (origin, along, normal, w) in faces {
                let count = if b.rng.chance(0.8 * dens.min(1.3)) {
                    b.rng.range_i(1, 4)
                } else {
                    0
                };
                for _ in 0..count {
                    let roots = [
                        super::city::DEEP_FLOOR,
                        super::city::MID_DECK,
                        super::city::UPPER_DECK,
                    ];
                    let start = roots[b.rng.range_i(0, 3) as usize];
                    if start + 3.0 > a.max.y || start < a.min.y {
                        continue;
                    }
                    let top = (start + b.rng.range(12.0, 30.0)).min(a.max.y - 0.5);
                    b.climber(origin, along, normal, w, start + 0.2, top);
                }
            }
        }
    }
    Growth {
        plants: b.out,
        lights: b.lights,
    }
}

/// A point at distance `t` along the perimeter of a box's footprint.
fn perimeter_point(min: Vec3, max: Vec3, t: f32) -> (f32, f32) {
    let (w, d) = (max.x - min.x, max.z - min.z);
    if t < w {
        (min.x + t, min.z)
    } else if t < w + d {
        (max.x, min.z + t - w)
    } else if t < 2.0 * w + d {
        (max.x - (t - w - d), max.z)
    } else {
        (min.x, max.z - (t - 2.0 * w - d))
    }
}
