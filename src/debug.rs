//! Debug and art-iteration tools: overlay and key bindings.

use sola_raylib::prelude::*;

use crate::player::Player;
use crate::world::World;

pub struct Debug {
    pub show_overlay: bool,
}

pub const HELP: &[&str] = &[
    "WASD move  Shift run  Space jump (hold toward a ledge to climb)",
    "F1 overlay  F2 free-fly (Space/Ctrl up/down)",
    "R random seed  [ ] previous/next seed  Esc release mouse  Ctrl+Q quit",
];

impl Debug {
    pub fn new() -> Self {
        Self { show_overlay: true }
    }

    pub fn draw(&self, d: &mut impl RaylibDraw, world: &World, player: &Player) {
        if !self.show_overlay {
            return;
        }
        let p = player.pos;
        let lines = [
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
            format!("blocks {}", world.blocks.len()),
        ];
        let h = 26 + (lines.len() + HELP.len()) as i32 * 20;
        d.draw_rectangle(8, 8, 560, h, Color::new(0, 0, 0, 140));
        d.draw_fps(16, 14);
        for (i, l) in lines
            .iter()
            .chain(
                HELP.iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .iter(),
            )
            .enumerate()
        {
            let color = if i < lines.len() {
                Color::RAYWHITE
            } else {
                Color::LIGHTGRAY
            };
            d.draw_text(l, 16, 36 + i as i32 * 20, 18, color);
        }
    }
}
