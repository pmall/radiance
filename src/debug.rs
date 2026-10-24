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

/// How far the finder looks, in lots around the player (about 1 km). Things beyond the loaded
/// area exist (the city is a pure function of the seed) but are not drawn until the player gets
/// closer, so they are marked as such.
const FIND_RADIUS: i32 = 40;

/// Nearest stairs and crater from the player's lot and floor, recomputed when either changes (or
/// the seed, or the chunk files).
#[derive(Default)]
struct Finder {
    key: Option<(u64, u64, i32, i32, usize)>,
    /// (label, target point on the player's floor).
    found: Vec<(String, glam::Vec3)>,
}

impl Finder {
    fn update(&mut self, seed: u64, pos: glam::Vec3) {
        use crate::world::{city, floors};
        let (lx, lz) = city::lot_of(pos);
        let floor = floors::floor_at(pos.y);
        let key = (seed, crate::world::chunks::generation(), lx, lz, floor);
        if self.key == Some(key) {
            return;
        }
        self.key = Some(key);
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
        // Ring by ring outward; stop once nothing farther out can beat what was found.
        let done = |best: &Option<(f32, String, glam::Vec3)>, d: i32| {
            best.as_ref()
                .is_some_and(|b| b.0.sqrt() < (d as f32 - 0.5) * city::LOT)
        };
        for d in 0..=FIND_RADIUS {
            if done(&up, d) && done(&down, d) && done(&crater, d) {
                break;
            }
            for z in lz - d..=lz + d {
                for x in lx - d..=lx + d {
                    if (x - lx).abs().max((z - lz).abs()) != d {
                        continue;
                    }
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
        }
        // Beyond the loaded area: real, but not drawn yet.
        let loaded = |p: glam::Vec3| {
            let (x, z) = city::lot_of(p);
            (x - lx).abs().max((z - lz).abs()) <= crate::world::LOAD_RADIUS
        };
        for best in [&mut up, &mut down, &mut crater].into_iter().flatten() {
            if !loaded(best.2) {
                best.1.push_str(" (far, not loaded yet)");
            }
        }
        self.found = [up, down, crater]
            .into_iter()
            .flatten()
            .map(|(_, label, p)| (label, p))
            .collect();
    }
}

/// "62 m at 6 o'clock": distance and direction from the player's position and heading
/// (12 o'clock straight ahead, 3 o'clock to the right).
fn bearing(player: &Player, target: glam::Vec3) -> String {
    let v = target - player.pos;
    let (ahead, right) = (v.dot(player.forward_flat()), v.dot(player.right()));
    let hour = (right.atan2(ahead) / std::f32::consts::TAU * 12.0).round() as i32;
    let hour = (hour + 11).rem_euclid(12) + 1;
    format!("{:.0} m at {hour} o'clock", glam::vec2(v.x, v.z).length())
}

/// Key of each effect in `EFFECTS`, same order as the bindings in `main`.
const EFFECT_KEYS: [&str; 8] = ["F3", "F4", "F5", "F6", "F7", "F8", "F11", "X"];

const TITLE_SIZE: i32 = 32;
const TEXT_SIZE: i32 = 18;
const LINE: i32 = 21;
const HEADING: Color = Color::new(255, 214, 120, 255);

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
        let full = floors::name(floor);
        let (lx, lz) = city::lot_of(p);
        let (code, name) = full.split_once(' ').unwrap_or((&full, ""));

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
                "floor {} of {} (D0 is the bottom)   {:.0} m above its ground   {doing}, {:.0} m/s",
                floor,
                floors::count() - 1,
                p.y - floors::y(floor),
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
                crate::world::biome::biome_at(world.seed, lx, lz).name
            ),
            Color::RAYWHITE,
        );

        if self.test_mode {
            say(
                "Nearby, 12 o'clock is straight ahead (test mode: G hides, hold right click to go fast)"
                    .into(),
                HEADING,
            );
            if self.finder.found.is_empty() {
                say("  nothing within 1 km".into(), Color::RAYWHITE);
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
                "  {} fps   {:.1} M triangles   {} flower lights ({} left out of full cells)   resolution {:.0}%   sound {}",
                unsafe { sola_raylib::ffi::GetFPS() },
                renderer.triangles() as f32 / 1e6,
                renderer.lights_in_use,
                renderer.lights_left_out(),
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

/// Writes the screen as drawn so far to an image file. Returns whether it was written.
pub fn export_screen(path: &std::path::Path) -> bool {
    let Ok(c) = std::ffi::CString::new(path.to_string_lossy().as_bytes()) else {
        return false;
    };
    unsafe {
        // Draws still queued in raylib's batch must reach the screen first.
        sola_raylib::ffi::rlDrawRenderBatchActive();
        let img = sola_raylib::ffi::LoadImageFromScreen();
        let ok = sola_raylib::ffi::ExportImage(img, c.as_ptr());
        sola_raylib::ffi::UnloadImage(img);
        ok
    }
}

/// Folder of the P screenshots (gitignored).
const SCREENSHOT_DIR: &str = "screenshots";

/// Saves the frame drawn so far (call before the overlay) as `screenshots/radiance-<seed>-<n>.png`,
/// with a text file beside it: where and when it was taken, and the command that renders the
/// same view again. Returns the image path.
pub fn save_screenshot(
    seed: u64,
    player: &Player,
    cycle: &DayCycle,
    effects: i32,
) -> std::io::Result<std::path::PathBuf> {
    use crate::world::floors;
    std::fs::create_dir_all(SCREENSHOT_DIR)?;
    let base = (1..)
        .map(|n| std::path::Path::new(SCREENSHOT_DIR).join(format!("radiance-{seed}-{n:03}")))
        .find(|p| !p.with_extension("png").exists())
        .expect("a free screenshot name");
    let png = base.with_extension("png");
    if !export_screen(&png) {
        return Err(std::io::Error::other("could not write the image"));
    }
    let p = player.pos;
    let (yaw, pitch) = (player.yaw.to_degrees(), player.pitch.to_degrees());
    let view = format!("{:.2},{:.2},{:.2},{yaw:.1},{pitch:.1}", p.x, p.y, p.z);
    let note = format!(
        "seed {seed}\nfloor {}\ntime {} ({})\nposition {view} (x,y,z,yaw,pitch)\n\n\
         Render this view again:\n\
         cargo run --release -- {seed} --time {:.4} --view {view} --fx {effects} --shot out.png\n",
        floors::name(floors::floor_at(p.y)),
        cycle.clock(),
        phase(cycle.t),
        cycle.t,
    );
    std::fs::write(base.with_extension("txt"), note)?;
    Ok(png)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finder_points_to_a_stair_up_and_a_crater_from_the_spawn() {
        let mut f = Finder::default();
        f.update(1, glam::vec3(0.0, 1.7, 0.0));
        for (label, p) in &f.found {
            println!("{label} at {p}");
        }
        assert!(f.found.iter().any(|(l, _)| l.starts_with("stair up")));
        assert!(f.found.iter().any(|(l, _)| l.starts_with("crater")));
    }

    #[test]
    fn finder_finds_a_stair_up_even_from_high_floors() {
        use crate::world::floors;
        for floor in 0..floors::count() - 1 {
            let mut f = Finder::default();
            f.update(1, glam::vec3(0.0, floors::y(floor) + 0.5, 0.0));
            assert!(
                f.found.iter().any(|(l, _)| l.starts_with("stair up")),
                "no stair up from D{floor} within 1 km"
            );
        }
    }
}
