//! Debug and art-iteration tools: overlay and key bindings.
//!
//! Its `unsafe` blocks are raylib calls made while drawing: the window is open, on its thread.

use sola_raylib::prelude::*;

use crate::daycycle::DayCycle;
use crate::player::Player;
use crate::render::{self, Renderer};
use crate::world::World;
use crate::world::finder::{Finder, Found, Way};

pub struct Debug {
    pub show_overlay: bool,
    /// Test mode (G): the overlay points to the nearest stairs and craters, and the right mouse
    /// button makes the player much faster.
    pub test_mode: bool,
    finder: Finder,
}

/// A way to another floor as the overlay names it.
fn way_label(found: &Found) -> String {
    let label = match found.way {
        Way::StairUp { to } => format!("stair up to D{to}"),
        Way::StairDown { to } => format!("stair down to D{to}"),
        Way::Crater { above } => format!("crater open above D{above}"),
    };
    if found.loaded {
        label
    } else {
        // Beyond the loaded area: real, but not drawn yet.
        format!("{label} (far, not loaded yet)")
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

/// A key that switches one render effect on and off.
pub struct EffectKey {
    pub key: KeyboardKey,
    /// The key as the overlay names it.
    pub label: &'static str,
    /// The effect's bit (`render::FX_*`).
    pub bit: i32,
    /// The effect as the overlay names it.
    pub name: &'static str,
}

/// Every effect toggle, in the order the overlay lists them.
pub const EFFECT_KEYS: [EffectKey; 8] = {
    use KeyboardKey::*;
    use render::*;
    const fn k(key: KeyboardKey, label: &'static str, bit: i32, name: &'static str) -> EffectKey {
        EffectKey {
            key,
            label,
            bit,
            name,
        }
    }
    [
        k(KEY_F3, "F3", FX_OUTLINES, "outlines"),
        k(KEY_F4, "F4", FX_FOG, "fog"),
        k(KEY_F5, "F5", FX_CEL, "cel"),
        k(KEY_F6, "F6", FX_SHADOWS, "shadows"),
        k(KEY_F7, "F7", FX_GRADING, "grading"),
        k(KEY_F8, "F8", FX_GRAIN, "grain"),
        k(KEY_F11, "F11", FX_PARTICLES, "particles"),
        k(KEY_X, "X", FX_TEXTURE, "texture"),
    ]
};

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
            if self.finder.found().is_empty() {
                say("  nothing within 1 km".into(), Color::RAYWHITE);
            }
            for found in self.finder.found() {
                say(
                    format!("  {}: {}", way_label(found), bearing(player, found.at)),
                    Color::RAYWHITE,
                );
            }
        }

        say("Technical".into(), HEADING);
        say(
            format!(
                "  {} fps   {:.1} M triangles   {} flower lights ({} left out of full cells)   resolution {:.0}%   sound {}",
                // SAFETY: called while drawing, the window open (module doc).
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
        let off: Vec<String> = EFFECT_KEYS
            .iter()
            .filter(|e| renderer.effects & e.bit == 0)
            .map(|e| format!("{} ({})", e.name, e.label))
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
            // SAFETY: called while drawing, the window open; `c` outlives the call.
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

/// Writes the screen as drawn so far to an image file.
pub fn export_screen(path: &std::path::Path) -> std::io::Result<()> {
    let c = std::ffi::CString::new(path.to_string_lossy().as_bytes())
        .map_err(|_| std::io::Error::other("the path holds a NUL byte"))?;
    // SAFETY: called while drawing, the window open; `c` outlives the export, and the image is
    // unloaded once.
    unsafe {
        // Draws still queued in raylib's batch must reach the screen first.
        sola_raylib::ffi::rlDrawRenderBatchActive();
        let img = sola_raylib::ffi::LoadImageFromScreen();
        let ok = sola_raylib::ffi::ExportImage(img, c.as_ptr());
        sola_raylib::ffi::UnloadImage(img);
        if !ok {
            // raylib logs why; a missing folder is the usual cause.
            return Err(std::io::Error::other("raylib could not write the image"));
        }
    }
    Ok(())
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
    export_screen(&png)?;
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
    fn every_effect_has_one_key() {
        let mut all = 0;
        for e in &EFFECT_KEYS {
            assert_eq!(e.bit.count_ones(), 1, "{}", e.name);
            assert_eq!(all & e.bit, 0, "{} twice", e.name);
            all |= e.bit;
        }
        assert_eq!(all, render::FX_ALL);
    }
}
