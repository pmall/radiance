//! Debug and art-iteration tools: overlay and key bindings.

use sola_raylib::prelude::*;

use crate::daycycle::DayCycle;
use crate::player::Player;
use crate::render::{EFFECTS, Renderer};
use crate::world::World;

pub struct Debug {
    pub show_overlay: bool,
    /// Test mode (G): the overlay points to the nearest stairs and craters, and the right mouse
    /// button makes the player much faster.
    pub test_mode: bool,
    finder: Finder,
}

/// How far the finder looks, in lots around the player.
const FIND_RADIUS: i32 = 20;

/// Nearest stairs and crater from the player's lot and floor, recomputed when either changes.
#[derive(Default)]
struct Finder {
    key: Option<(u64, i32, i32, usize)>,
    /// (label, target point on the player's floor).
    found: Vec<(String, glam::Vec3)>,
}

impl Finder {
    fn update(&mut self, seed: u64, pos: glam::Vec3) {
        use crate::world::{city, floors};
        let (lx, lz) = (
            (pos.x / city::LOT).floor() as i32,
            (pos.z / city::LOT).floor() as i32,
        );
        let floor = floors::floor_at(pos.y);
        if self.key == Some((seed, lx, lz, floor)) {
            return;
        }
        self.key = Some((seed, lx, lz, floor));
        let center = |x: i32, z: i32| {
            glam::vec3(
                (x as f32 + 0.5) * city::LOT,
                pos.y,
                (z as f32 + 0.5) * city::LOT,
            )
        };
        let mut up: Option<(f32, String, glam::Vec3)> = None;
        let mut down = up.clone();
        let mut crater = up.clone();
        let keep =
            |best: &mut Option<(f32, String, glam::Vec3)>, p: glam::Vec3, label: String| {
                let d = p.distance_squared(pos);
                if best.as_ref().is_none_or(|b| d < b.0) {
                    *best = Some((d, label, p));
                }
            };
        for z in lz - FIND_RADIUS..=lz + FIND_RADIUS {
            for x in lx - FIND_RADIUS..=lx + FIND_RADIUS {
                if let Some((lo, hi)) = city::stair(seed, x, z) {
                    if lo <= floor && floor < hi {
                        keep(&mut up, center(x, z), format!("stair up to D{hi}"));
                    }
                    if lo < floor && floor <= hi {
                        keep(&mut down, center(x, z), format!("stair down to D{lo}"));
                    }
                }
                if let Some(f) = city::crater_at(seed, x, z) {
                    keep(&mut crater, center(x, z), format!("crater open above D{f}"));
                }
            }
        }
        self.found = [up, down, crater]
            .into_iter()
            .flatten()
            .map(|(_, label, p)| (label, p))
            .collect();
    }
}

/// "130 m at 2 o'clock" from the player's position and heading.
fn bearing(player: &Player, target: glam::Vec3) -> String {
    let v = target - player.pos;
    let (ahead, right) = (v.dot(player.forward_flat()), v.dot(player.right()));
    let hour = (right.atan2(ahead) / std::f32::consts::TAU * 12.0).round() as i32;
    let hour = (hour + 11).rem_euclid(12) + 1;
    format!("{:.0} m at {hour} o'clock", glam::vec2(v.x, v.z).length())
}

/// Key of each effect in `EFFECTS`, same order as the bindings in `main`.
const EFFECT_KEYS: [&str; 8] = ["F3", "F4", "F5", "F6", "F7", "F8", "F11", "X"];

pub const HELP: &[&str] = &[
    "WASD / ZQSD / arrows move  Shift run  Space/left click jump (hold toward a ledge to climb)",
    "F1 overlay  F2 free-fly (Space/Ctrl up/down)  G test mode (finder, right click = fast)",
    "F3-F8/F11/X toggle effects  F9 render scale  F10 reload shaders  F12 screenshot",
    "T pause time  Home/End scrub  PgUp/PgDn speed  1-4 (or numpad) dawn/day/dusk/night",
    "R random seed  [ ] or numpad -/+ previous/next seed  M mute  V record video  Esc release mouse  Ctrl+Q quit",
];

impl Debug {
    pub fn new() -> Self {
        Self {
            show_overlay: true,
            test_mode: true,
            finder: Finder::default(),
        }
    }

    /// Per-frame upkeep (the test mode finder).
    pub fn update(&mut self, world: &World, player: &Player) {
        if self.test_mode {
            self.finder.update(world.seed, player.pos);
        }
    }

    pub fn draw(
        &self,
        d: &mut impl RaylibDraw,
        world: &World,
        player: &Player,
        renderer: &Renderer,
        cycle: &DayCycle,
        audio: Option<&crate::audio::Audio>,
    ) {
        if !self.show_overlay {
            return;
        }
        let p = player.pos;
        let effects = EFFECTS
            .iter()
            .enumerate()
            .map(|(i, (bit, name))| {
                let on = renderer.effects & bit != 0;
                format!("{} {}{}", EFFECT_KEYS[i], if on { "" } else { "-" }, name)
            })
            .collect::<Vec<_>>()
            .join("  ");
        let mut lines = vec![
            format!(
                "seed {}  biome {}",
                world.seed,
                crate::world::biome::biome_at(
                    world.seed,
                    (p.x / crate::world::city::LOT).floor() as i32,
                    (p.z / crate::world::city::LOT).floor() as i32
                )
                .name
            ),
            format!(
                "pos {:.1} {:.1} {:.1}  {}  {}",
                p.x,
                p.y,
                p.z,
                crate::world::floors::NAMES[crate::world::floors::floor_at(p.y)],
                player.state_name()
            ),
            match audio {
                Some(a) => format!(
                    "sound {}  {} notes",
                    if a.muted { "muted (M)" } else { "on (M mutes)" },
                    a.notes_played
                ),
                None => "sound off".to_string(),
            },
            format!(
                "speed {:.1} m/s",
                glam::vec2(player.vel.x, player.vel.z).length()
            ),
            format!(
                "time {}  {}  x{}",
                cycle.clock(),
                if cycle.running { "running" } else { "paused" },
                cycle.speed
            ),
            format!(
                "blocks {}  tris {}  lights {}  scale {}",
                world.block_count(),
                renderer.triangles(),
                renderer.lights_in_use,
                renderer.render_scale
            ),
            effects,
        ];
        if self.test_mode {
            lines.push("TEST MODE (G)  right click: fast".into());
            for (label, target) in &self.finder.found {
                lines.push(format!("  {label}: {}", bearing(player, *target)));
            }
        }
        if renderer.shaders_broken() {
            lines.push("SHADER COMPILE ERROR (see log), using last good version".into());
        }
        let h = 26 + (lines.len() + HELP.len()) as i32 * 20;
        d.draw_rectangle(8, 8, 680, h, Color::new(0, 0, 0, 140));
        d.draw_fps(16, 14);
        let help = HELP.iter().map(|s| s.to_string());
        for (i, l) in lines.iter().cloned().chain(help).enumerate() {
            let color = if i < lines.len() {
                Color::RAYWHITE
            } else {
                Color::LIGHTGRAY
            };
            d.draw_text(&l, 16, 36 + i as i32 * 20, 18, color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finder_points_to_a_stair_up_and_a_crater_from_the_spawn() {
        let mut f = Finder::default();
        f.update(7, glam::vec3(0.0, 1.7, 0.0));
        for (label, p) in &f.found {
            println!("{label} at {p}");
        }
        assert!(f.found.iter().any(|(l, _)| l.starts_with("stair up")));
        assert!(f.found.iter().any(|(l, _)| l.starts_with("crater")));
    }
}
