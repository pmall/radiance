//! Overgrowth: vines, wall climbers and frond bushes layered over a lot's architecture, with
//! flowers as the only light sources (one point light per flower, in the flower's color).
//!
//! Plants are decorative triangle geometry (no collision), grown on the lot's own blocks and
//! seeded per lot, so they stay deterministic. They are checked against the blocks of the
//! neighboring lots too (decks and bridges touch across lot borders) and never pass through one:
//! a plant that would is not planted, or stops short. Plant growth is nearly even by height,
//! flowers are densest in the sunlit canopy.

use glam::{Vec3, vec3};
use sola_raylib::prelude::Color;

use super::biome::{Biome, biome_at, pick_color};
use super::{Aabb, Block, Light, PlantVert};
use crate::rng::{Rng, hash_coords, mix64};

const SALT_GROWTH: i32 = 3;

/// Light sources kept per lot; the renderer picks the nearest ones overall.
const MAX_LIGHTS: usize = 24;
/// Emission of flowers, out of 255. Foliage never glows: only flowers are light sources.
const FLOWER_GLOW: u8 = 235;
/// Minimum distance between two flowers of a lot, so their glows never pile up.
const MIN_FLOWER_GAP: f32 = 3.0;

/// Blocks smaller than this on both horizontal axes (stair steps, parapets) stay bare.
const MIN_HOST: f32 = 3.5;
/// Towers at least this tall get wall climbers.
const MIN_WALL: f32 = 6.0;
/// Slabs thinner than this count as decks and bridges, which vines hang from.
const MAX_SLAB: f32 = 3.0;

/// Height layer of a block, indexing the per-layer biome tables: canopy, middle, depths.
fn layer(y: f32) -> usize {
    match y {
        y if y > -10.0 => 0,
        y if y > -50.0 => 1,
        _ => 2,
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

fn shade(c: Color, k: f32) -> Color {
    let f = |v: u8| (v as f32 * k).clamp(0.0, 255.0) as u8;
    Color::new(f(c.r), f(c.g), f(c.b), c.a)
}

struct Builder<'a> {
    rng: &'a mut Rng,
    out: Vec<PlantVert>,
    lights: Vec<Light>,
    id: u8,
    /// Growth density of the block being planted.
    dens: f32,
    /// Flower multiplier of the block being planted.
    bloom: f32,
    glow: u8,
    biome: &'static Biome,
    /// Blocks near the lot, neighbors included: what plants must keep clear of.
    solids: &'a [Block],
}

impl Builder<'_> {
    /// Starts a new plant: its own outline id, so it reads separately from its host.
    fn new_plant(&mut self) {
        self.id = 1 + (mix64(self.rng.next_u64()) % 255) as u8;
        self.glow = 0;
    }

    /// A flower color from the biome's palette.
    fn flower_color(&mut self) -> Color {
        let [r, g, b] = pick_color(self.biome.flowers, self.rng);
        Color::new(r, g, b, 255)
    }

    /// Leaf or stem color of the current plant.
    fn foliage(&mut self) -> Color {
        leaf_color(self.rng)
    }

    /// Whether `p` lies strictly inside a block grown by `margin` (a point on a block's own
    /// surface is not inside it).
    fn inside_block(&self, p: Vec3, margin: f32) -> bool {
        self.solids.iter().any(|b| inside(&b.aabb, p, margin))
    }

    /// Whether a flower at `center` sits inside a block, or within a hair of its surface (which
    /// would bury the petals). Wall flowers float 0.3 off their host, so they pass.
    fn buried(&self, center: Vec3) -> bool {
        self.inside_block(center, 0.15)
    }

    /// A flower: petals around a bright core, plus the point light that sits at it and takes its
    /// color. `axis` is the way the flower faces. Skipped once the lot's light budget is spent.
    fn flower(&mut self, center: Vec3, radius: f32, color: Color, axis: Vec3) {
        let at = center + axis * radius * 0.4;
        if self.lights.len() >= MAX_LIGHTS
            || self.buried(center)
            || self
                .lights
                .iter()
                .any(|l| l.pos.distance(at) < MIN_FLOWER_GAP)
        {
            return;
        }
        let (u, v) = axis.any_orthonormal_pair();
        let petals = self.rng.range_i(5, 8);
        let spin = self.rng.range(0.0, std::f32::consts::TAU);
        let quads: Vec<[Vec3; 4]> = (0..petals)
            .map(|k| {
                let a = spin + std::f32::consts::TAU * k as f32 / petals as f32;
                let radial = u * a.cos() + v * a.sin();
                let dir = (axis * 0.5 + radial).normalize();
                let tangent = axis.cross(radial);
                let (mid, tip) = (center + dir * radius * 0.95, center + dir * radius * 1.7);
                let w = tangent * radius * 0.55;
                [center, mid + w, tip, mid - w]
            })
            .collect();
        // A flower whose petals would touch a block is not planted.
        if quads
            .iter()
            .any(|q| (0..4).any(|e| !self.clear(q[e], q[(e + 1) % 4])))
        {
            return;
        }
        let saved = std::mem::replace(&mut self.glow, FLOWER_GLOW);
        for q in &quads {
            self.quad2(q[0], q[1], q[2], q[3], color);
        }
        let core = Color::new(
            ((color.r as u16 + 255) / 2) as u8,
            ((color.g as u16 + 255) / 2) as u8,
            ((color.b as u16 + 255) / 2) as u8,
            255,
        );
        self.blob(
            center + axis * radius * 0.15,
            Vec3::splat(radius * 0.5),
            core,
        );
        self.glow = saved;

        // Decks above and below stop the light: clamp it to the open space around the flower.
        let (mut y_lo, mut y_hi) = (f32::MIN, f32::MAX);
        for b in self.solids {
            let a = &b.aabb;
            if at.x > a.min.x && at.x < a.max.x && at.z > a.min.z && at.z < a.max.z {
                if a.max.y <= at.y + 0.05 {
                    y_lo = y_lo.max(a.max.y);
                } else if a.min.y >= at.y - 0.05 {
                    y_hi = y_hi.min(a.min.y);
                }
            }
        }
        let lin = |v: u8| (v as f32 / 255.0).powf(2.2);
        self.lights.push(Light {
            y_lo,
            y_hi,
            pos: at,
            radius: 4.0 + 5.0 * radius,
            color: vec3(lin(color.r), lin(color.g), lin(color.b)) * 2.2,
        });
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

    /// Whether the segment a-b stays out of every block.
    fn clear(&self, a: Vec3, b: Vec3) -> bool {
        !self
            .solids
            .iter()
            .any(|o| segment_crosses(a, b, &o.aabb, -0.02))
    }

    /// A diamond leaf pointing along `dir`, tilted by `up`. Dropped if it would touch a block.
    fn leaf(&mut self, base: Vec3, dir: Vec3, size: f32, color: Color) {
        let side = dir.cross(Vec3::Y).normalize_or_zero() * size * 0.28;
        let lift = Vec3::Y * size * 0.12;
        let mid = base + dir * size * 0.45 + lift;
        let tip = base + dir * size + lift * 0.5;
        if !(self.clear(base, mid + side)
            && self.clear(base, mid - side)
            && self.clear(mid + side, tip)
            && self.clear(mid - side, tip))
        {
            return;
        }
        self.quad2(base, mid + side, tip, mid - side, color);
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

    /// A double-sided triangle.
    fn tri2(&mut self, a: Vec3, b: Vec3, c: Vec3, color: Color) {
        let n = (b - a).cross(c - a).normalize_or_zero();
        if n == Vec3::ZERO {
            return;
        }
        self.tri(a, b, c, n, color);
        self.tri(a, c, b, -n, color);
    }

    /// An arching blade from `base`, folded along its midrib, pointed at the tip and darker toward
    /// the root. `el` is its initial elevation. Returns the tip.
    fn frond(&mut self, base: Vec3, yaw: f32, el: f32, len: f32, w: f32, color: Color) -> Vec3 {
        const SEGS: usize = 3;
        let h = vec3(yaw.cos(), 0.0, yaw.sin());
        let side = vec3(-h.z, 0.0, h.x);
        let droop = len * self.rng.range(0.15, 0.55);
        let pts: [Vec3; SEGS + 1] = std::array::from_fn(|i| {
            let t = i as f32 / SEGS as f32;
            base + h * (len * el.cos() * t)
                + Vec3::Y * (len * el.sin() * t - droop * t * t).max(0.06 + 0.14 * len * t)
        });
        let width = |i: usize| {
            let t = i as f32 / SEGS as f32;
            w * (0.9 * (std::f32::consts::PI * t.powf(0.65)).sin().max(0.0) + 0.1 * (1.0 - t))
        };
        for i in 0..SEGS {
            let (w0, w1) = (width(i), width(i + 1));
            let (m0, m1) = (
                pts[i] + Vec3::Y * w0 * 0.15,
                pts[i + 1] + Vec3::Y * w1 * 0.15,
            );
            let c = shade(color, 0.65 + 0.5 * (i as f32 + 0.5) / SEGS as f32);
            for s in [-0.5, 0.5] {
                let (e0, e1) = (pts[i] + side * w0 * s, pts[i + 1] + side * w1 * s);
                self.tri2(e0, m0, m1, c);
                self.tri2(e0, m1, e1, c);
            }
        }
        pts[SEGS]
    }

    /// A rosette of arching fronds (or a tuft of narrow blades) sitting on a surface at `base`.
    fn bush(&mut self, base: Vec3) {
        let tuft = self.rng.chance(0.3);
        let scale = self.rng.range(1.0, 2.0);
        // Fronds reach about 1.8 * scale sideways: keep clear of any other block, so none of
        // them is squashed against a wall or rooted in one (the host's top surface is fine).
        let reach = 1.8 * scale;
        let crowded = self.solids.iter().any(|b| {
            let a = &b.aabb;
            a.min.y < base.y + 2.5 * scale
                && a.max.y > base.y + 0.05
                && base.x > a.min.x - reach
                && base.x < a.max.x + reach
                && base.z > a.min.z - reach
                && base.z < a.max.z + reach
        });
        if crowded {
            return;
        }
        self.new_plant();
        let color = self.foliage();
        let count = if tuft {
            self.rng.range_i(10, 17)
        } else {
            self.rng.range_i(6, 10)
        };
        let mut tips = Vec::new();
        for _ in 0..count {
            let yaw = self.rng.range(0.0, std::f32::consts::TAU);
            // Inner fronds stand up, outer ones arch over.
            let spread = self.rng.f32();
            let el = 1.3 - 0.9 * spread;
            let len = scale * self.rng.range(0.9, 1.5) * if tuft { 1.2 } else { 1.0 };
            let w = len * if tuft { 0.09 } else { 0.3 };
            let k = self.rng.range(0.85, 1.1);
            let root = base + vec3(yaw.cos(), 0.0, yaw.sin()) * scale * 0.15 * spread;
            tips.push(self.frond(root, yaw, el, len, w, shade(color, k)));
        }
        if self.rng.chance((0.25 * self.bloom).min(0.6)) {
            let fc = self.flower_color();
            for _ in 0..self.rng.range_i(1, 3) {
                // Of two random tips, take the higher: flowers sit on upright fronds, not on
                // the ones lying on the ground.
                let pick = |s: &mut Self| tips[s.rng.range_i(0, tips.len() as i32) as usize];
                let (t0, t1) = (pick(self), pick(self));
                let tip = if t0.y > t1.y { t0 } else { t1 };
                let r = self.rng.range(0.22, 0.34);
                self.flower(tip + Vec3::Y * r * 0.9, r, fc, Vec3::Y);
            }
        }
    }

    /// A vine hanging from `from`, swaying sideways, crossed ribbons so it has volume.
    fn hanging_vine(&mut self, from: Vec3, length: f32, out: Vec3) {
        self.new_plant();
        const SEGS: usize = 6;
        let color = self.foliage();
        let phase = self.rng.range(0.0, std::f32::consts::TAU);
        let sway = self.rng.range(0.15, 0.7);
        let w = self.rng.range(0.3, 0.55);
        // It drapes over the edge: hangs outside the face of its host, and only sways outward.
        let from = from + out * (w * 0.6 + 0.12);
        let pts: Vec<Vec3> = (0..=SEGS)
            .map(|i| {
                let t = i as f32 / SEGS as f32;
                let mut d = vec3(
                    (phase + t * 4.0).sin() * sway * t,
                    0.0,
                    (phase * 1.7 + t * 3.0).cos() * sway * t,
                );
                let inward = d.dot(out);
                if inward < 0.0 {
                    d -= 2.0 * inward * out;
                }
                from + d + Vec3::Y * (-length * t)
            })
            .collect();
        // Next to another deck (lots touch) the free side is a slab: no vine there.
        let hits = |a: Vec3, b: Vec3| {
            self.solids
                .iter()
                .any(|o| segment_crosses(a, b, &o.aabb, -0.35))
        };
        if pts.iter().any(|&p| self.inside_block(p, 0.35))
            || pts.windows(2).any(|w| hits(w[0], w[1]))
        {
            return;
        }
        self.ribbon(&pts, Vec3::X, w, w * 0.4, color);
        self.ribbon(&pts, Vec3::Z, w, w * 0.4, color);
        let leaf_col = self.foliage();
        for (i, p) in pts.iter().enumerate().skip(1) {
            if self.rng.chance(0.85) {
                let a = self.rng.range(0.0, std::f32::consts::TAU);
                // Leaves point away from the host face, never back into it.
                let mut dir = vec3(a.cos(), -0.35, a.sin());
                let inward = dir.dot(out);
                if inward < 0.0 {
                    dir -= 2.0 * inward * out;
                }
                let dir = (dir + out * 0.3).normalize();
                let size = self.rng.range(1.2, 2.2) * (1.0 - i as f32 / (SEGS + 4) as f32);
                self.leaf(*p, dir, size, leaf_col);
            }
        }
        if self.rng.chance((0.25 * self.bloom).min(0.6)) {
            let fc = self.flower_color();
            let p = pts[self.rng.range_i(2, SEGS as i32 + 1) as usize];
            let r = self.rng.range(0.2, 0.3);
            self.flower(p + Vec3::Y * r * 0.3, r, fc, Vec3::Y);
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
                // Stop short of anything in the way (a deck jutting out of the wall, a bridge):
                // the stem and its leaves would otherwise run through it.
                let last = *pts.last().unwrap();
                let ahead = p + (p - last).normalize_or_zero() * 1.2;
                if !self.clear(last, ahead) {
                    break;
                }
                pts.push(p);
                w *= 0.96;
                if self.rng.chance(0.8) {
                    let sgn = if self.rng.chance(0.5) { 1.0 } else { -1.0 };
                    let dir = (along * sgn * 0.8 + Vec3::Y * 0.3 + normal * 0.5).normalize();
                    let size = self.rng.range(1.0, 1.9);
                    self.leaf(p, dir, size, leaf_col);
                }
                if self.rng.chance(0.05 * self.bloom) {
                    let fc = self.flower_color();
                    let r = self.rng.range(0.25, 0.35);
                    self.flower(p + normal * 0.3, r, fc, normal);
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

fn inside(a: &Aabb, p: Vec3, margin: f32) -> bool {
    let m = Vec3::splat(margin);
    p.cmpgt(a.min - m).all() && p.cmplt(a.max + m).all()
}

/// Plant geometry for one lot, grown over the lot's blocks.
pub struct Growth {
    pub plants: Vec<PlantVert>,
    pub lights: Vec<Light>,
}

pub fn grow_lot(seed: u64, lx: i32, lz: i32, blocks: &[Block]) -> Growth {
    let mut rng = Rng::new(hash_coords(seed, lx, SALT_GROWTH, lz));
    // Neighboring lots matter too: decks and towers touch at lot borders, and plants must not
    // cross into them or leak light through them.
    let mut around = Vec::new();
    for dz in -1..=1 {
        for dx in -1..=1 {
            super::city::generate_lot(seed, lx + dx, lz + dz, &mut around);
        }
    }
    // Only blocks near this lot's own blocks can touch its plants. Bridges reach into the next
    // lot, so the area is the span of the blocks, not the lot's footprint.
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for blk in blocks {
        lo = lo.min(blk.aabb.min);
        hi = hi.max(blk.aabb.max);
    }
    let reach = Vec3::new(4.0, 0.0, 4.0);
    let near = Aabb {
        min: lo - reach,
        max: hi + reach,
    };
    around.retain(|b| b.aabb.overlaps(&near));
    let mut b = Builder {
        rng: &mut rng,
        out: Vec::new(),
        lights: Vec::new(),
        id: 1,
        dens: 1.0,
        bloom: 1.0,
        glow: 0,
        biome: biome_at(seed, lx, lz),
        solids: &around,
    };

    for blk in blocks {
        let a = &blk.aabb;
        let size = a.size();
        if size.x.max(size.z) < MIN_HOST {
            continue;
        }
        let layer = layer(a.max.y.min(a.min.y + 1.0));
        let dens = b.biome.plant_density[layer];
        b.dens = dens;
        b.bloom = b.biome.bloom[layer];

        // Bushes on top surfaces.
        let area = size.x * size.z;
        let mut n = area * 0.04 * dens * b.biome.bush_rate;
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
            let mut n = perimeter * 0.3 * dens * b.biome.vine_rate;
            while n > 0.0 {
                if n >= 1.0 || b.rng.chance(n) {
                    let t = b.rng.range(0.0, perimeter);
                    let (x, z, out) = perimeter_point(a.min, a.max, t);
                    let len = b.rng.range(3.0, 10.0 + 6.0 * dens);
                    // Hang only through free air: stop at whatever block lies below.
                    let top = a.max.y - 0.1;
                    let ground = b
                        .solids
                        .iter()
                        .filter(|o| {
                            let c = &o.aabb;
                            c.max.y <= top
                                && x > c.min.x - 0.8
                                && x < c.max.x + 0.8
                                && z > c.min.z - 0.8
                                && z < c.max.z + 0.8
                                && !(c.min.y <= a.min.y && c.max.y >= a.max.y)
                                && c.max.y <= a.min.y + 0.05
                        })
                        .map(|o| o.aabb.max.y)
                        .fold(f32::MIN, f32::max);
                    let len = len.min(top - ground - 0.3);
                    if len >= 2.0 {
                        b.hanging_vine(vec3(x, top, z), len, out);
                    }
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
                let count = if b.rng.chance(0.8 * b.biome.climber_rate * dens.min(1.3)) {
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

/// A point at distance `t` along the perimeter of a box's footprint, and the outward normal there.
fn perimeter_point(min: Vec3, max: Vec3, t: f32) -> (f32, f32, Vec3) {
    let (w, d) = (max.x - min.x, max.z - min.z);
    if t < w {
        (min.x + t, min.z, Vec3::NEG_Z)
    } else if t < w + d {
        (max.x, min.z + t - w, Vec3::X)
    } else if t < 2.0 * w + d {
        (max.x - (t - w - d), max.z, Vec3::Z)
    } else {
        (min.x, max.z - (t - 2.0 * w - d), Vec3::NEG_X)
    }
}

/// Whether the segment a-b passes through the interior of `bx` shrunk by `eps` (slab test).
fn segment_crosses(a: Vec3, b: Vec3, bx: &Aabb, eps: f32) -> bool {
    let (mut t0, mut t1) = (0.0_f32, 1.0_f32);
    let d = b - a;
    for k in 0..3 {
        let (lo, hi) = (bx.min[k] + eps, bx.max[k] - eps);
        if d[k].abs() < 1e-9 {
            if a[k] <= lo || a[k] >= hi {
                return false;
            }
        } else {
            let (u, v) = ((lo - a[k]) / d[k], (hi - a[k]) / d[k]);
            t0 = t0.max(u.min(v));
            t1 = t1.min(u.max(v));
            if t0 >= t1 {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::city::generate_lot;

    fn around(seed: u64, lx: i32, lz: i32) -> Vec<Block> {
        let mut v = Vec::new();
        for dz in -1..=1 {
            for dx in -1..=1 {
                generate_lot(seed, lx + dx, lz + dz, &mut v);
            }
        }
        v
    }

    #[test]
    /// Over a few hundred lots: growth is deterministic, flowers respect the light budget and
    /// spacing and never sit in a block, and no plant triangle edge passes through a block.
    fn growth_invariants() {
        let mut crossing = 0;
        for seed in 1..=3u64 {
            for lx in -5..5 {
                for lz in -5..5 {
                    let mut own = Vec::new();
                    generate_lot(seed, lx, lz, &mut own);
                    let g = grow_lot(seed, lx, lz, &own);
                    let all = around(seed, lx, lz);

                    // Determinism.
                    let g2 = grow_lot(seed, lx, lz, &own);
                    assert_eq!(g.plants.len(), g2.plants.len());
                    assert_eq!(g.lights.len(), g2.lights.len());

                    // Lights: budget, spacing, never inside a block.
                    assert!(g.lights.len() <= MAX_LIGHTS);
                    for (i, l) in g.lights.iter().enumerate() {
                        if let Some(b) = all.iter().find(|b| inside(&b.aabb, l.pos, 0.0)) {
                            panic!(
                                "seed {seed} lot {lx},{lz}: light {:?} inside block {:?}..{:?}",
                                l.pos, b.aabb.min, b.aabb.max
                            );
                        }
                        assert!(l.y_lo <= l.pos.y && l.pos.y <= l.y_hi);
                        for m in &g.lights[..i] {
                            assert!(m.pos.distance(l.pos) >= MIN_FLOWER_GAP);
                        }
                    }

                    // No triangle edge goes through a block.
                    for t in g.plants.chunks(3) {
                        let hit = all.iter().any(|b| {
                            (0..3).any(|e| {
                                segment_crosses(t[e].pos, t[(e + 1) % 3].pos, &b.aabb, 0.01)
                            })
                        });
                        crossing += hit as usize;
                    }
                }
            }
        }
        assert_eq!(crossing, 0);
    }
}
