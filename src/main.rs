mod debug;
mod player;
mod rng;
mod world;

use glam::{Vec2, Vec3, vec3};
use sola_raylib::prelude::*;

use debug::Debug;
use player::{Input, Player};
use world::World;

const DEFAULT_SEED: u64 = 1;
const SIM_STEP: f32 = 1.0 / 120.0;
const SPAWN: Vec3 = vec3(0.0, 0.5, 0.0);

fn rv(v: Vec3) -> Vector3 {
    Vector3::new(v.x, v.y, v.z)
}

fn random_seed() -> u64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    rng::mix64(nanos as u64) % 1_000_000
}

fn read_input(rl: &RaylibHandle) -> Input {
    use KeyboardKey::*;
    let axis = |pos: KeyboardKey, neg: KeyboardKey| {
        rl.is_key_down(pos) as i32 as f32 - rl.is_key_down(neg) as i32 as f32
    };
    let d = rl.get_mouse_delta();
    Input {
        wish: Vec2::new(axis(KEY_D, KEY_A), axis(KEY_W, KEY_S)),
        look: Vec2::new(d.x, d.y),
        jump_pressed: rl.is_key_pressed(KEY_SPACE),
        jump_held: rl.is_key_down(KEY_SPACE),
        run: rl.is_key_down(KEY_LEFT_SHIFT),
        down: rl.is_key_down(KEY_LEFT_CONTROL),
    }
}

fn main() {
    let seed = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_SEED);

    let (mut rl, thread) = sola_raylib::init()
        .size(1600, 900)
        .title("Radiance")
        .resizable()
        .msaa_4x()
        .vsync()
        .build();
    rl.set_exit_key(None);
    rl.disable_cursor();

    let mut world = World::generate(seed);
    let mut player = Player::new(SPAWN);
    let mut debug = Debug::new();
    let mut accumulator = 0.0;

    while !rl.window_should_close() {
        use KeyboardKey::*;
        let ctrl = rl.is_key_down(KEY_LEFT_CONTROL) || rl.is_key_down(KEY_RIGHT_CONTROL);
        if ctrl && rl.is_key_pressed(KEY_Q) {
            break;
        }

        // Mouse capture.
        if rl.is_key_pressed(KEY_ESCAPE) {
            rl.enable_cursor();
        }
        if !rl.is_cursor_hidden() && rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
            rl.disable_cursor();
        }
        let captured = rl.is_cursor_hidden();

        // Debug keys.
        if rl.is_key_pressed(KEY_F1) {
            debug.show_overlay = !debug.show_overlay;
        }
        if rl.is_key_pressed(KEY_F2) {
            player.toggle_fly();
        }
        let new_seed = if rl.is_key_pressed(KEY_R) {
            Some(random_seed())
        } else if rl.is_key_pressed(KEY_RIGHT_BRACKET) {
            Some(world.seed.wrapping_add(1))
        } else if rl.is_key_pressed(KEY_LEFT_BRACKET) {
            Some(world.seed.wrapping_sub(1))
        } else {
            None
        };
        if let Some(s) = new_seed {
            world = World::generate(s);
            if !player.is_flying() {
                player = Player::new(SPAWN);
            }
        }

        // Simulation: fixed sub-steps for stable collisions; look applied per frame.
        let mut input = if captured {
            read_input(&rl)
        } else {
            Input::default()
        };
        player.apply_look(input.look);
        accumulator = (accumulator + rl.get_frame_time()).min(0.1);
        while accumulator >= SIM_STEP {
            player.update(SIM_STEP, &input, &world);
            input.jump_pressed = false;
            accumulator -= SIM_STEP;
        }
        if player.pos.y < -100.0 {
            player = Player::new(SPAWN);
        }

        // Rendering (placeholder until the stylized pipeline lands in milestone 2).
        let eye = player.eye();
        let camera = Camera3D::perspective(
            rv(eye),
            rv(eye + player.look_dir()),
            Vector3::new(0.0, 1.0, 0.0),
            player.fov,
        );
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::new(150, 170, 185, 255));
        {
            let mut d3 = d.begin_mode3D(camera);
            for b in &world.blocks {
                let (c, s) = (b.aabb.center(), b.aabb.size());
                d3.draw_cube(rv(c), s.x, s.y, s.z, b.color);
                d3.draw_cube_wires(rv(c), s.x, s.y, s.z, Color::new(20, 22, 30, 255));
            }
        }
        debug.draw(&mut d, &world, &player);
        if !captured {
            d.draw_text(
                "click to capture mouse",
                16,
                d.get_screen_height() - 30,
                20,
                Color::RAYWHITE,
            );
        }
    }
}
