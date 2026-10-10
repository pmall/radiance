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
        let keep = |best: &mut Option<(f32, String, glam::Vec3)>, p: glam::Vec3, label: String| {
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

/// "62 m behind you": distance and direction from the player's position and heading.
fn bearing(player: &Player, target: glam::Vec3) -> String {
    let v = target - player.pos;
    let (ahead, right) = (v.dot(player.forward_flat()), v.dot(player.right()));
    let sector = (right.atan2(ahead) / std::f32::consts::TAU * 8.0).round() as i32;
    let way = [
        "ahead",
        "ahead, to the right",
        "to your right",
        "behind you, to the right",
        "behind you",
        "behind you, to the left",
        "to your left",
        "ahead, to the left",
    ][sector.rem_euclid(8) as usize];
    format!("{:.0} m {way}", glam::vec2(v.x, v.z).length())
}

/// Key of each effect in `EFFECTS`, same order as the bindings in `main`.
const EFFECT_KEYS: [&str; 8] = ["F3", "F4", "F5", "F6", "F7", "F8", "F11", "X"];

pub const HELP: &[&str] = &[
    "Move: WASD / ZQSD / arrows   Shift: run   Space or left click: jump (hold toward a ledge to climb)",
    "F1: hide this panel   F2: free-fly (Space/Ctrl: up/down)   G: test mode",
    "Time: T pause   Home/End rewind/advance   PgUp/PgDn speed   1-4 dawn/day/dusk/night",
    "Seed: R random   [ ] previous/next      M mute   V record video   F12 screenshot",
    "Look: F3-F8, F11, X effects   F9 resolution   F10 reload shaders   Esc free the mouse   Ctrl+Q quit",
];

const TITLE_SIZE: i32 = 32;
const TEXT_SIZE: i32 = 18;
const LINE: i32 = 21;
const HEADING: Color = Color::new(255, 214, 120, 255);
const DIM: Color = Color::new(170, 170, 180, 255);

/// Name of the time of day at clock position `t` (0 midnight, 0.5 noon).
fn phase(t: f32) -> &'static str {
    match t {
        t if t < 0.2 => "night",
        t if t < 0.3 => "dawn",
        t if t < 0.7 => "day",
        t if t < 0.8 => "dusk",
        _ => "night",
    }
}

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
        use crate::world::{city, floors};
        let p = player.pos;
        let floor = floors::floor_at(p.y);
        let (code, name) = floors::NAMES[floor]
            .split_once(' ')
            .unwrap_or((floors::NAMES[floor], ""));

        // (text, color); the title is drawn on its own above.
        let mut lines: Vec<(String, Color)> = Vec::new();
        let mut say = |s: String, c: Color| lines.push((s, c));
        let doing = match player.state_name() {
            "ground" => "walking",
            "air" => "in the air",
            "mantle" => "climbing a ledge",
            _ => "free-flying",
        };
        say(
            format!(
                "floor {} of {} from the bottom   {:.0} m above its ground   {doing}, {:.0} m/s",
                floor + 1,
                floors::COUNT,
                p.y - floors::Y[floor],
                glam::vec2(player.vel.x, player.vel.z).length()
            ),
            Color::RAYWHITE,
        );
        say(
            format!(
                "{} {}   time {}{}   seed {}   biome {}",
                cycle.clock(),
                phase(cycle.t),
                if cycle.running { "running" } else { "paused" },
                if cycle.speed != 1.0 {
                    format!(" x{}", cycle.speed)
                } else {
                    String::new()
                },
                world.seed,
                crate::world::biome::biome_at(
                    world.seed,
                    (p.x / city::LOT).floor() as i32,
                    (p.z / city::LOT).floor() as i32
                )
                .name
            ),
            Color::RAYWHITE,
        );

        if self.test_mode {
            say(
                "Nearby (test mode: G hides, hold right click to go fast)".into(),
                HEADING,
            );
            if self.finder.found.is_empty() {
                say("  nothing within 500 m".into(), Color::RAYWHITE);
            }
            for (label, target) in &self.finder.found {
                say(
                    format!("  {label}: {}", bearing(player, *target)),
                    Color::RAYWHITE,
                );
            }
        }

        say("Technical".into(), HEADING);
        say(
            format!(
                "  {} fps   {:.1} M triangles   {} flower lights   resolution {:.0}%   sound {}",
                unsafe { sola_raylib::ffi::GetFPS() },
                renderer.triangles() as f32 / 1e6,
                renderer.lights_in_use,
                renderer.render_scale * 100.0,
                match audio {
                    Some(a) if a.muted => "muted".to_string(),
                    Some(a) => format!("on, {} notes played", a.notes_played),
                    None => "off".to_string(),
                }
            ),
            Color::RAYWHITE,
        );
        let off: Vec<String> = EFFECTS
            .iter()
            .enumerate()
            .filter(|(_, (bit, _))| renderer.effects & bit == 0)
            .map(|(i, (_, name))| format!("{name} ({})", EFFECT_KEYS[i]))
            .collect();
        say(
            if off.is_empty() {
                "  all effects on".into()
            } else {
                format!("  effects off: {}", off.join(", "))
            },
            Color::RAYWHITE,
        );
        if renderer.shaders_broken() {
            say(
                "  SHADER COMPILE ERROR (see log), using last good version".into(),
                Color::RED,
            );
        }
        say("Keys".into(), HEADING);
        for h in HELP {
            say(format!("  {h}"), DIM);
        }

        let measure = |s: &str, size: i32| {
            let c = std::ffi::CString::new(s).unwrap_or_default();
            unsafe { sola_raylib::ffi::MeasureText(c.as_ptr(), size) }
        };
        let width = lines
            .iter()
            .map(|(l, _)| measure(l, TEXT_SIZE))
            .max()
            .unwrap_or(0)
            + 24;
        let h = 16 + TITLE_SIZE + 8 + lines.len() as i32 * LINE + 8;
        d.draw_rectangle(8, 8, width, h, Color::new(0, 0, 0, 150));
        d.draw_text(code, 18, 16, TITLE_SIZE, HEADING);
        let w = measure(code, TITLE_SIZE);
        d.draw_text(name, 18 + w + 14, 16, TITLE_SIZE, Color::RAYWHITE);
        let top = 16 + TITLE_SIZE + 8;
        for (i, (l, c)) in lines.iter().enumerate() {
            d.draw_text(l, 18, top + i as i32 * LINE, TEXT_SIZE, *c);
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
