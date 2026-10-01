//! Debug and art-iteration tools: overlay and key bindings.

use sola_raylib::prelude::*;

use crate::daycycle::DayCycle;
use crate::player::Player;
use crate::render::{EFFECTS, Renderer};
use crate::world::World;

pub struct Debug {
    pub show_overlay: bool,
}

pub const HELP: &[&str] = &[
    "WASD / ZQSD / arrows move  Shift run  Space jump (hold toward a ledge to climb)",
    "F1 overlay  F2 free-fly (Space/Ctrl up/down)",
    "F3-F8/F11 toggle effects  F9 render scale  F10 reload shaders  F12 screenshot",
    "T pause time  Home/End scrub  PgUp/PgDn speed  1-4 (or numpad) dawn/day/dusk/night",
    "R random seed  [ ] or numpad -/+ previous/next seed  Esc release mouse  Ctrl+Q quit",
];

impl Debug {
    pub fn new() -> Self {
        Self { show_overlay: true }
    }

    pub fn draw(
        &self,
        d: &mut impl RaylibDraw,
        world: &World,
        player: &Player,
        renderer: &Renderer,
        cycle: &DayCycle,
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
                format!("F{} {}{}", i + 3, if on { "" } else { "-" }, name)
            })
            .collect::<Vec<_>>()
            .join("  ");
        let mut lines = vec![
            format!("seed {}", world.seed),
            format!(
                "pos {:.1} {:.1} {:.1}  {}",
                p.x,
                p.y,
                p.z,
                player.state_name()
            ),
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
