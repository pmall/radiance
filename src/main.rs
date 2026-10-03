mod audio;
mod daycycle;
mod debug;
mod player;
mod render;
mod rng;
mod world;

use glam::{Vec2, Vec3, vec3};
use sola_raylib::prelude::*;

use daycycle::DayCycle;
use debug::Debug;
use player::{Input, Player};
use render::look::{PRESET_TIMES, Sky};
use render::{EFFECTS, Renderer, View};
use world::World;

const DEFAULT_SEED: u64 = 1;
const SIM_STEP: f32 = 1.0 / 120.0;
/// Lots generated per frame while streaming, to keep frame times even.
const STREAM_BUDGET: usize = 4;
const SPAWN: Vec3 = vec3(0.0, 0.5, 0.0);
/// Late afternoon: a few minutes before dusk starts.
const START_TIME: f32 = 0.66;
/// Time scrub speed with the arrow keys, in days per second.
const SCRUB_SPEED: f32 = 0.08;

/// Command line: `radiance [seed] [--time T] [--view x,y,z,yaw,pitch] [--shot out.png] [--fx mask] [--bench frames] [--no-audio]`.
/// `--shot` renders a few frames, saves a screenshot and exits (for checking looks headlessly).
struct Args {
    seed: u64,
    time: Option<f32>,
    view: Option<[f32; 5]>,
    shot: Option<String>,
    /// Effect bit mask override (see `render::EFFECTS`), for comparing looks in shots.
    fx: Option<i32>,
    /// Render this many frames without vsync, print the average frame time and exit.
    bench: Option<u32>,
    /// Start without sound.
    no_audio: bool,
}

fn parse_args() -> Args {
    let mut args = Args {
        seed: DEFAULT_SEED,
        time: None,
        view: None,
        shot: None,
        fx: None,
        bench: None,
        no_audio: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--time" => args.time = it.next().and_then(|s| s.parse().ok()),
            "--shot" => args.shot = it.next(),
            "--no-audio" => args.no_audio = true,
            "--bench" => args.bench = it.next().and_then(|s| s.parse().ok()),
            "--fx" => args.fx = it.next().and_then(|s| s.parse().ok()),
            "--view" => {
                let v: Vec<f32> = it
                    .next()
                    .map(|s| s.split(',').filter_map(|x| x.parse().ok()).collect())
                    .unwrap_or_default();
                args.view = v.try_into().ok();
            }
            _ => {
                if let Ok(s) = a.parse() {
                    args.seed = s;
                }
            }
        }
    }
    args
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
    // QWERTY (WASD), AZERTY (ZQSD) and arrow keys all work.
    let any = |keys: &[KeyboardKey]| keys.iter().any(|&k| rl.is_key_down(k)) as i32 as f32;
    let axis = |pos: &[KeyboardKey], neg: &[KeyboardKey]| any(pos) - any(neg);
    let d = rl.get_mouse_delta();
    Input {
        wish: Vec2::new(
            axis(&[KEY_D, KEY_RIGHT], &[KEY_A, KEY_Q, KEY_LEFT]),
            axis(&[KEY_W, KEY_Z, KEY_UP], &[KEY_S, KEY_DOWN]),
        ),
        look: Vec2::new(d.x, d.y),
        jump_pressed: rl.is_key_pressed(KEY_SPACE)
            || rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT),
        jump_held: rl.is_key_down(KEY_SPACE)
            || rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT),
        run: rl.is_key_down(KEY_LEFT_SHIFT),
        down: rl.is_key_down(KEY_LEFT_CONTROL),
    }
}

fn spawn_player(view: Option<[f32; 5]>) -> Player {
    match view {
        Some([x, y, z, yaw, pitch]) => {
            let mut p = Player::new(vec3(x, y, z));
            p.toggle_fly();
            p.yaw = yaw.to_radians();
            p.pitch = pitch.to_radians();
            p
        }
        None => Player::new(SPAWN),
    }
}

fn main() {
    let args = parse_args();

    let (mut rl, thread) = sola_raylib::init()
        .size(1600, 900)
        .title("Radiance")
        .resizable()
        .vsync()
        .build();
    rl.set_exit_key(None);
    if args.shot.is_none() && args.bench.is_none() {
        rl.disable_cursor();
    }
    if args.bench.is_some() {
        unsafe {
            sola_raylib::ffi::ClearWindowState(
                sola_raylib::ffi::ConfigFlags::FLAG_VSYNC_HINT as u32,
            )
        };
    }
    let mut bench_start = None;

    let mut player = spawn_player(args.view);
    let mut world = World::new(args.seed);
    world.stream(player.pos, usize::MAX);
    let mut renderer = Renderer::new();
    if let Some(fx) = args.fx {
        renderer.effects = fx;
    }
    renderer.sync_world(&world);
    // Sound only for interactive runs: screenshots and benchmarks stay silent.
    let mut audio = if args.shot.is_none() && args.bench.is_none() && !args.no_audio {
        audio::Audio::new(args.seed)
    } else {
        None
    };
    let mut cycle = DayCycle::new(args.time.unwrap_or(START_TIME));
    let mut debug = Debug::new();
    let mut accumulator = 0.0;
    let mut frame = 0u32;
    if args.shot.is_some() {
        cycle.running = false;
        debug.show_overlay = false;
    }

    while !rl.window_should_close() {
        use KeyboardKey::*;
        let dt = rl.get_frame_time();
        let ctrl = rl.is_key_down(KEY_LEFT_CONTROL) || rl.is_key_down(KEY_RIGHT_CONTROL);
        if ctrl && (rl.is_key_pressed(KEY_Q) || rl.is_key_pressed(KEY_A)) {
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
        for (i, key) in [
            KEY_F3, KEY_F4, KEY_F5, KEY_F6, KEY_F7, KEY_F8, KEY_F11, KEY_X,
        ]
        .into_iter()
        .enumerate()
        {
            if rl.is_key_pressed(key) {
                renderer.effects ^= EFFECTS[i].0;
            }
        }
        if rl.is_key_pressed(KEY_F9) {
            renderer.cycle_render_scale();
        }
        if rl.is_key_pressed(KEY_M)
            && let Some(a) = audio.as_mut()
        {
            a.toggle_mute();
        }
        if rl.is_key_pressed(KEY_F10) {
            renderer.reload_shaders();
        }
        let new_seed = if rl.is_key_pressed(KEY_R) {
            Some(random_seed())
        } else if rl.is_key_pressed(KEY_RIGHT_BRACKET) || rl.is_key_pressed(KEY_KP_ADD) {
            Some(world.seed.wrapping_add(1))
        } else if rl.is_key_pressed(KEY_LEFT_BRACKET) || rl.is_key_pressed(KEY_KP_SUBTRACT) {
            Some(world.seed.wrapping_sub(1))
        } else {
            None
        };
        if let Some(s) = new_seed {
            world = World::new(s);
            if let Some(a) = audio.as_mut() {
                a.reseed(s);
            }
            renderer.clear_world();
            if !player.is_flying() {
                player = Player::new(SPAWN);
            }
            world.stream(player.pos, usize::MAX);
        }

        // Time of day.
        if rl.is_key_pressed(KEY_T) {
            cycle.running = !cycle.running;
        }
        if rl.is_key_pressed(KEY_PAGE_UP) {
            cycle.speed = (cycle.speed * 2.0).min(256.0);
        }
        if rl.is_key_pressed(KEY_PAGE_DOWN) {
            cycle.speed = (cycle.speed * 0.5).max(1.0);
        }
        let scrub = rl.is_key_down(KEY_END) as i32 - rl.is_key_down(KEY_HOME) as i32;
        cycle.advance(scrub as f32 * SCRUB_SPEED * dt);
        for (i, keys) in [
            [KEY_ONE, KEY_KP_1],
            [KEY_TWO, KEY_KP_2],
            [KEY_THREE, KEY_KP_3],
            [KEY_FOUR, KEY_KP_4],
        ]
        .into_iter()
        .enumerate()
        {
            if keys.iter().any(|&k| rl.is_key_pressed(k)) {
                cycle.t = PRESET_TIMES[i];
            }
        }
        cycle.update(dt);
        renderer.update(dt);

        // Simulation: fixed sub-steps for stable collisions; look applied per frame.
        let mut input = if captured {
            read_input(&rl)
        } else {
            Input::default()
        };
        player.apply_look(input.look);
        accumulator = (accumulator + dt).min(0.1);
        while accumulator >= SIM_STEP {
            player.update(SIM_STEP, &input, &world);
            input.jump_pressed = false;
            accumulator -= SIM_STEP;
        }
        if player.pos.y < -100.0 {
            player = Player::new(SPAWN);
        }
        world.stream(player.pos, STREAM_BUDGET);
        renderer.sync_world(&world);

        let sky = Sky::from_cycle(&cycle);
        if let Some(a) = audio.as_mut() {
            let day = ((cycle.sun_dir().y + 0.1) / 0.4).clamp(0.0, 1.0);
            let souls = a.update(dt, player.eye(), player.right(), day, &world);
            renderer.pulse(&souls);
        }
        let view = View {
            pos: player.eye(),
            dir: player.look_dir(),
            fovy: player.fov,
        };
        let time = rl.get_time() as f32;
        let mut d = rl.begin_drawing(&thread);
        renderer.draw(&view, &sky, time);

        frame += 1;
        if let Some(n) = args.bench {
            // Skip a warm-up (shader compile, streaming) before timing.
            if frame == 60 {
                bench_start = Some(std::time::Instant::now());
            }
            if frame == 60 + n {
                let ms = bench_start.map_or(0.0, |t| t.elapsed().as_secs_f64() * 1000.0 / n as f64);
                println!(
                    "BENCH {ms:.2} ms/frame ({:.0} fps), {} tris",
                    1000.0 / ms,
                    renderer.triangles()
                );
                break;
            }
        }
        if let Some(path) = &args.shot
            && frame == 5
        {
            let c = std::ffi::CString::new(path.as_str()).unwrap();
            unsafe {
                let img = sola_raylib::ffi::LoadImageFromScreen();
                sola_raylib::ffi::ExportImage(img, c.as_ptr());
                sola_raylib::ffi::UnloadImage(img);
            }
            break;
        }

        debug.draw(&mut d, &world, &player, &renderer, &cycle, audio.as_ref());
        if !captured && args.shot.is_none() {
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
