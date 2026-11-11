mod audio;
mod daycycle;
mod debug;
mod player;
mod recorder;
mod render;
mod report;
mod rng;
mod stream;
mod world;

use std::process::ExitCode;

use glam::{Vec2, Vec3, vec3};
use sola_raylib::prelude::*;

use daycycle::DayCycle;
use debug::Debug;
use player::{Input, Player};
use render::look::{Blend, PRESET_TIMES, Sky};
use render::{Renderer, View};
use world::World;

const DEFAULT_SEED: u64 = 1;
const SIM_STEP: f32 = 1.0 / 120.0;
const SPAWN: Vec3 = vec3(0.0, 0.5, 0.0);
/// Late afternoon: a few minutes before dusk starts.
const START_TIME: f32 = 0.66;
/// Time scrub speed with the arrow keys, in days per second.
const SCRUB_SPEED: f32 = 0.08;
/// Seconds between two checks of the chunk files for edits (a scan of their folders).
const CHUNK_POLL: f32 = 0.5;

/// How to call the game, printed with `--help` and after a bad command line.
const USAGE: &str = "usage: radiance [seed] [--time T] [--view x,y,z,yaw,pitch] [--shot out.png] \
[--fx mask] [--bench frames] [--no-audio] [--record-for seconds] [--overlay] [--floors N] \
[--no-props] [--no-pois] [--report [radius]] [--chunk NAME] [--catalog]";

/// The command line (`USAGE`). `--shot` renders a few frames, saves a screenshot and exits (for
/// checking looks headlessly).
#[derive(Debug, Default)]
struct Args {
    /// The city's seed (`DEFAULT_SEED` when not given).
    seed: Option<u64>,
    /// Time of day, 0 midnight to 1 the next.
    time: Option<f32>,
    view: Option<[f32; 5]>,
    shot: Option<String>,
    /// Effect bit mask override (`render::FX_*`, keys in `debug::EFFECT_KEYS`), for comparing
    /// looks in shots.
    fx: Option<i32>,
    /// Render this many frames without vsync, print the average frame time and exit.
    bench: Option<u32>,
    /// Start without sound.
    no_audio: bool,
    /// Record this many seconds of the run to `recordings/`, then exit (to test the recorder).
    record_for: Option<f32>,
    /// Keep the debug overlay in a `--shot`.
    overlay: bool,
    /// Number of floors in the stack (`world::floors::init`).
    floors: Option<usize>,
    /// Start with props or points of interest switched off (O and I in game).
    no_props: bool,
    no_pois: bool,
    /// Print the design report over the lots within this radius of the spawn, and exit.
    report: Option<i32>,
    /// Show only this chunk (`NAME` or `BIOME/NAME`), in front of the spawn on a bare deck.
    chunk: Option<String>,
    /// List the chunk catalog and exit.
    catalog: bool,
    /// Print `USAGE` and exit.
    help: bool,
}

/// Lots around the spawn `--report` covers when no radius follows it.
const REPORT_RADIUS: i32 = 5;

/// A flag's value parsed as `T` and accepted by `ok`, else an error saying the flag takes `what`.
fn parse<T: std::str::FromStr>(
    flag: &str,
    v: String,
    what: &str,
    ok: impl Fn(&T) -> bool,
) -> Result<T, String> {
    v.parse()
        .ok()
        .filter(ok)
        .ok_or_else(|| format!("{flag} takes {what}, not {v:?}"))
}

/// Reads the command line (without the program name). A bad one says what is wrong.
fn parse_args(words: impl IntoIterator<Item = String>) -> Result<Args, String> {
    let mut args = Args::default();
    let mut it = words.into_iter().peekable();
    while let Some(word) = it.next() {
        let flag = word.as_str();
        let mut value = || it.next().ok_or_else(|| format!("{flag} needs a value"));
        match flag {
            "--time" => {
                let range = "a time of day from 0 to 1";
                args.time = Some(parse(flag, value()?, range, |t: &f32| {
                    (0.0..=1.0).contains(t)
                })?);
            }
            "--shot" => args.shot = Some(value()?),
            "--chunk" => args.chunk = Some(value()?),
            "--floors" => {
                let (lo, hi) = (world::floors::MIN, world::floors::MAX);
                let what = format!("a number of floors from {lo} to {hi}");
                args.floors = Some(parse(flag, value()?, &what, |n| (lo..=hi).contains(n))?);
            }
            "--record-for" => {
                let what = "a number of seconds above 0";
                args.record_for = Some(parse(flag, value()?, what, |s: &f32| *s > 0.0)?);
            }
            "--bench" => {
                args.bench = Some(parse(flag, value()?, "a number of frames above 0", |n| {
                    *n > 0
                })?);
            }
            "--fx" => {
                let what = format!("an effect mask from 0 to {}", render::FX_ALL);
                let all = |m: &i32| (0..=render::FX_ALL).contains(m);
                args.fx = Some(parse(flag, value()?, &what, all)?);
            }
            "--view" => {
                let v = value()?;
                let numbers: Result<Vec<f32>, _> = v.split(',').map(str::parse).collect();
                let what = "five numbers x,y,z,yaw,pitch";
                args.view = Some(
                    numbers
                        .ok()
                        .and_then(|n| <[f32; 5]>::try_from(n).ok())
                        .ok_or_else(|| format!("--view takes {what}, not {v:?}"))?,
                );
            }
            "--report" => {
                // The radius is optional: only a number after the flag is taken as one.
                let radius = it.peek().and_then(|w| w.parse().ok());
                if radius.is_some() {
                    it.next();
                }
                args.report = Some(radius.unwrap_or(REPORT_RADIUS));
            }
            "--no-audio" => args.no_audio = true,
            "--overlay" => args.overlay = true,
            "--no-props" => args.no_props = true,
            "--no-pois" => args.no_pois = true,
            "--catalog" => args.catalog = true,
            "--help" | "-h" => args.help = true,
            _ if flag.starts_with('-') => return Err(format!("unknown flag {flag}")),
            _ => {
                if args.seed.is_some() {
                    return Err(format!("a second seed: {flag}"));
                }
                args.seed = Some(word.parse().map_err(|_| {
                    format!("{word:?} is neither a flag nor a seed (a whole number from 0)")
                })?);
            }
        }
    }
    Ok(args)
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
        boost: rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_RIGHT),
    }
}

/// The player at the spawn, or standing at `--view` (feet position, as screenshot notes give it).
/// A shot or a benchmark holds the camera exactly there in free-fly; a normal run walks from there.
fn spawn_player(view: Option<[f32; 5]>, fly: bool) -> Player {
    match view {
        Some([x, y, z, yaw, pitch]) => {
            let mut p = Player::new(vec3(x, y, z));
            if fly {
                p.toggle_fly();
            }
            p.yaw = yaw.to_radians();
            p.pitch = pitch.to_radians();
            p
        }
        None => Player::new(SPAWN),
    }
}

/// The biome around a point, as its index in `BIOMES` (what the sky is drawn in).
fn biome_variant(seed: u64, at: Vec3) -> usize {
    let (lx, lz) = world::city::lot_of(at);
    world::biome::variant_of(world::biome::biome_at(seed, lx, lz))
}

/// `--catalog`: every biome's chunks and modules, and the files that failed to load.
fn print_catalog() {
    let all = world::chunks::catalogs();
    let mut biomes: Vec<_> = all.biomes.values().collect();
    biomes.sort_by(|a, b| a.biome.cmp(&b.biome));
    for c in biomes {
        let mut chunks: Vec<_> = c.chunks.values().collect();
        chunks.sort_by(|a, b| a.name.cmp(&b.name));
        println!("Biome {} (assets/biomes/{}/), chunks:", c.biome, c.biome);
        for ch in chunks {
            println!(
                "  {:<24} {:>2}x{:<2} lots  {:?}  labels {:?}  zones {:?}  weight {}  spacing {}",
                ch.name,
                ch.size.0,
                ch.size.1,
                ch.role(),
                ch.labels,
                ch.zones,
                ch.weight,
                ch.spacing
            );
        }
        let mut modules: Vec<_> = c.modules.keys().collect();
        modules.sort();
        println!(
            "  modules: {}",
            modules
                .iter()
                .map(|m| m.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    for e in &all.errors {
        println!("ERROR {e}");
    }
}

fn main() -> ExitCode {
    let args = match parse_args(std::env::args().skip(1)) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("radiance: {e}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    if args.help {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let seed = args.seed.unwrap_or(DEFAULT_SEED);
    world::floors::init(args.floors.unwrap_or(world::floors::DEFAULT_COUNT));
    world::design::set_props(!args.no_props);
    world::design::set_pois(!args.no_pois);
    if args.catalog {
        print_catalog();
        return ExitCode::SUCCESS;
    }
    if let Some(name) = args.chunk.clone() {
        let Some((biome, name)) = world::chunks::find(&name) else {
            eprintln!("radiance: no chunk named {name:?}; see --catalog");
            return ExitCode::from(2);
        };
        world::chunks::set_preview(biome, name);
    }
    if let Some(radius) = args.report {
        report::run(seed, radius);
        return ExitCode::SUCCESS;
    }

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
        // SAFETY: the window was just opened on this thread.
        unsafe {
            sola_raylib::ffi::ClearWindowState(
                sola_raylib::ffi::ConfigFlags::FLAG_VSYNC_HINT as u32,
            )
        };
    }
    let mut bench_start = None;

    let mut player = spawn_player(args.view, args.shot.is_some() || args.bench.is_some());
    let mut world = World::new(seed);
    let mut renderer = Renderer::new();
    if let Some(fx) = args.fx {
        renderer.effects = fx;
    }
    let mut streamer = stream::Streamer::new();
    streamer.fill(&mut world, &mut renderer, player.pos);
    renderer.sync_skyline(world.seed, player.pos, true);
    // Sound only for interactive runs: screenshots and benchmarks stay silent.
    let mut audio = if args.shot.is_none() && args.bench.is_none() && !args.no_audio {
        audio::Audio::new(seed)
    } else {
        None
    };
    let mut cycle = DayCycle::new(args.time.unwrap_or(START_TIME));
    let mut blend = Blend::new(biome_variant(world.seed, player.pos));
    let mut debug = Debug::new();
    let mut accumulator = 0.0;
    let mut chunk_poll = 0.0;
    let mut frame = 0u32;
    // What the process exits with: a `--shot` or `--record-for` that fails says so.
    let mut exit = ExitCode::SUCCESS;
    // Recording (V): `start_recording` is picked up after the next draw, when the screen size is
    // known; finished files are written in the background.
    let mut recorder: Option<recorder::Recorder> = None;
    let mut saving: Vec<recorder::Saving> = Vec::new();
    let mut start_recording = args.record_for.is_some();
    let mut toast: Option<(String, f32)> = None;
    // P: save the next frame (without the overlay) to screenshots/.
    let mut take_screenshot = false;
    if args.shot.is_some() {
        cycle.running = false;
        debug.show_overlay = args.overlay;
    }

    'run: while !rl.window_should_close() {
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
        if rl.is_key_pressed(KEY_G) {
            debug.test_mode = !debug.test_mode;
        }
        if rl.is_key_pressed(KEY_F2) {
            player.toggle_fly();
        }
        for e in &debug::EFFECT_KEYS {
            if rl.is_key_pressed(e.key) {
                renderer.effects ^= e.bit;
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
        if rl.is_key_pressed(KEY_P) {
            take_screenshot = true;
        }
        if rl.is_key_pressed(KEY_V) {
            match recorder.take() {
                Some(r) => {
                    saving.push(r.stop());
                    toast = Some(("saving the recording...".into(), 4.0));
                }
                None => start_recording = true,
            }
        }
        if rl.is_key_pressed(KEY_F10) {
            renderer.reload_shaders();
        }
        // O / I: props / points of interest on or off; the city is generated again.
        // Chunk files edited on disk: the city is generated again.
        let mut regenerate = false;
        chunk_poll -= dt;
        if chunk_poll <= 0.0 {
            chunk_poll = CHUNK_POLL;
            regenerate = world::chunks::poll();
        }
        if rl.is_key_pressed(KEY_O) {
            world::design::set_props(!world::design::props_on());
            regenerate = true;
        }
        if rl.is_key_pressed(KEY_I) {
            world::design::set_pois(!world::design::pois_on());
            regenerate = true;
        }
        let new_seed = if rl.is_key_pressed(KEY_R) {
            Some(random_seed())
        } else if rl.is_key_pressed(KEY_RIGHT_BRACKET) || rl.is_key_pressed(KEY_KP_ADD) {
            Some(world.seed.wrapping_add(1))
        } else if rl.is_key_pressed(KEY_LEFT_BRACKET) || rl.is_key_pressed(KEY_KP_SUBTRACT) {
            Some(world.seed.wrapping_sub(1))
        } else if regenerate {
            Some(world.seed)
        } else {
            None
        };
        if let Some(s) = new_seed {
            world = World::new(s);
            if let Some(a) = audio.as_mut() {
                a.reseed(s);
            }
            renderer.clear_world();
            streamer.reset();
            if !player.is_flying() {
                player = Player::new(SPAWN);
            }
            blend = Blend::new(biome_variant(s, player.pos));
            streamer.fill(&mut world, &mut renderer, player.pos);
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
        input.boost &= debug.test_mode;
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
        streamer.update(&mut world, &mut renderer, player.pos);
        renderer.sync_world(&world);
        renderer.sync_skyline(world.seed, player.pos, false);

        blend.update(biome_variant(world.seed, player.pos), dt);
        let sky = Sky::from_cycle(&cycle, &blend);
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

        // Recording: the world as drawn, without the overlay.
        if let Some(r) = recorder.as_mut() {
            r.frame();
            if args.record_for.is_some_and(|s| r.seconds() >= s) {
                saving.push(recorder.take().unwrap().stop());
            }
        } else if start_recording {
            start_recording = false;
            let (_, w, h) = recorder::grab_screen();
            match recorder::Recorder::start(world.seed, (w, h), audio.is_some()) {
                Ok(r) => recorder = Some(r),
                Err(e) => toast = Some((format!("cannot record: {e}"), 6.0)),
            }
        }
        let mut pending = Vec::new();
        for s in saving.drain(..) {
            match s.poll() {
                Ok(Ok(path)) => {
                    println!("RECORDED {}", path.display());
                    toast = Some((format!("saved {}", path.display()), 8.0));
                    if args.record_for.is_some() {
                        break 'run;
                    }
                }
                Ok(Err(e)) => {
                    eprintln!("recording failed: {e}");
                    toast = Some((format!("recording failed: {e}"), 8.0));
                    if args.record_for.is_some() {
                        exit = ExitCode::FAILURE;
                        break 'run;
                    }
                }
                Err(s) => pending.push(s),
            }
        }
        saving = pending;

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
        if take_screenshot {
            take_screenshot = false;
            toast = Some(
                match debug::save_screenshot(world.seed, &player, &cycle, renderer.effects) {
                    Ok(path) => (format!("screenshot saved: {}", path.display()), 4.0),
                    Err(e) => (format!("screenshot failed: {e}"), 6.0),
                },
            );
        }
        // The overlay comes before the shot, which keeps it only with --overlay.
        debug.update(&world, &player);
        debug.draw(&mut d, &world, &player, &renderer, &cycle, audio.as_ref());
        if let Some(path) = &args.shot
            && frame == 5
        {
            if let Err(e) = debug::export_screen(std::path::Path::new(path)) {
                eprintln!("radiance: could not save the shot to {path}: {e}");
                exit = ExitCode::FAILURE;
            }
            break;
        }

        if let Some(r) = recorder.as_ref() {
            let secs = r.seconds() as u32;
            d.draw_circle(d.get_screen_width() - 120, 24, 8.0, Color::RED);
            d.draw_text(
                &format!("REC {:02}:{:02}", secs / 60, secs % 60),
                d.get_screen_width() - 104,
                14,
                20,
                Color::RAYWHITE,
            );
        }
        if let Some((text, left)) = toast.as_mut() {
            *left -= dt;
            d.draw_text(text, 16, d.get_screen_height() - 60, 20, Color::RAYWHITE);
            if *left <= 0.0 {
                toast = None;
            }
        }
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

    // Leaving while recording (or saving): finish the files before exiting.
    if let Some(r) = recorder.take() {
        saving.push(r.stop());
    }
    for mut s in saving {
        loop {
            match s.poll() {
                Ok(result) => {
                    match result {
                        Ok(path) => println!("RECORDED {}", path.display()),
                        Err(e) => {
                            eprintln!("recording failed: {e}");
                            exit = ExitCode::FAILURE;
                        }
                    }
                    break;
                }
                Err(back) => {
                    s = back;
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }
    }
    exit
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(line: &str) -> Result<Args, String> {
        parse_args(line.split_whitespace().map(String::from))
    }

    #[test]
    fn a_shot_command_reads_back() {
        // As screenshot notes and tools/lookdev.py write it.
        let a = parse("7 --time 0.7400 --view -96.00,36.00,72.00,0.0,-5.0 --fx 255 --shot out.png")
            .unwrap();
        assert_eq!(a.seed, Some(7));
        assert_eq!(a.time, Some(0.74));
        assert_eq!(a.view, Some([-96.0, 36.0, 72.0, 0.0, -5.0]));
        assert_eq!((a.fx, a.shot.as_deref()), (Some(255), Some("out.png")));
        let a = parse("--report --floors 9").unwrap();
        assert_eq!((a.report, a.floors), (Some(REPORT_RADIUS), Some(9)));
        assert_eq!(parse("--report 3 12").unwrap().report, Some(3));
        assert_eq!(parse("--report 3 12").unwrap().seed, Some(12));
    }

    #[test]
    fn a_bad_command_line_is_refused() {
        for bad in [
            "--view 1,2,3",
            "--view 1,2,3,4,x",
            "--time noon",
            "--time 2",
            "--floors 99",
            "--bench 0",
            "--fx 256",
            "--shot",
            "--frobnicate",
            "seven",
            "1 2",
        ] {
            assert!(parse(bad).is_err(), "{bad:?} was accepted");
        }
    }
}
