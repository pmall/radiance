//! Records a run to an MP4 with the screen and the sound.
//!
//! Frames are read from the screen after the world is drawn (before the debug overlay) and piped
//! to an `ffmpeg` process on a worker thread, at a constant 60 fps by wall-clock time (a slow frame
//! is repeated, never skipped). The sound is a copy of the final mix, taken from the audio thread
//! and written to a raw file. When recording stops, the video and the sound are muxed into
//! `recordings/radiance-<seed>-<time>.mp4` on another thread, so the game does not freeze.
//! Needs `ffmpeg` on the PATH (NVENC is used when it works, x264 otherwise).

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc::{SyncSender, sync_channel};
use std::thread::JoinHandle;
use std::time::Instant;

use sola_raylib::ffi;

const FPS: u64 = 60;
const OUT_DIR: &str = "recordings";
/// Frames waiting for the encoder; beyond this the game drops frames instead of stalling.
const BACKLOG: usize = 6;

enum Msg {
    /// Raw RGBA pixels, and how many times to write them.
    Frame(Vec<u8>, u32),
    Stop,
}

pub struct Recorder {
    tx: SyncSender<Msg>,
    worker: JoinHandle<Result<(), String>>,
    started: Instant,
    written: u64,
    size: (i32, i32),
    audio: Option<BufWriter<File>>,
    audio_path: PathBuf,
    video_path: PathBuf,
    out_path: PathBuf,
    pub dropped: u64,
}

/// A recording being finished in the background.
pub struct Saving {
    handle: JoinHandle<Result<PathBuf, String>>,
}

impl Saving {
    /// `Some(result)` once the file is written.
    pub fn poll(self) -> Result<Result<PathBuf, String>, Self> {
        if self.handle.is_finished() {
            Ok(self
                .handle
                .join()
                .unwrap_or_else(|_| Err("save thread panicked".into())))
        } else {
            Err(self)
        }
    }
}

/// Whether NVENC works on this machine (a one-frame test encode).
fn nvenc_works() -> bool {
    Command::new("ffmpeg")
        .args([
            "-v",
            "quiet",
            "-f",
            "lavfi",
            "-i",
            "color=black:s=256x256",
            "-frames:v",
            "1",
            "-c:v",
            "h264_nvenc",
            "-f",
            "null",
            "-",
        ])
        .stdin(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// The current screen as RGBA, with its size.
pub fn grab_screen() -> (Vec<u8>, i32, i32) {
    unsafe {
        let image = ffi::LoadImageFromScreen();
        let (w, h) = (image.width, image.height);
        let bytes =
            std::slice::from_raw_parts(image.data as *const u8, (w * h * 4) as usize).to_vec();
        ffi::UnloadImage(image);
        (bytes, w, h)
    }
}

impl Recorder {
    /// Starts recording a screen of `size` pixels. `with_sound` copies the audio mix too.
    pub fn start(seed: u64, size: (i32, i32), with_sound: bool) -> Result<Self, String> {
        std::fs::create_dir_all(OUT_DIR).map_err(|e| e.to_string())?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let base = format!("{OUT_DIR}/radiance-{seed}-{stamp}");
        let (video_path, audio_path, out_path) = (
            PathBuf::from(format!("{base}.video.mp4")),
            PathBuf::from(format!("{base}.audio.raw")),
            PathBuf::from(format!("{base}.mp4")),
        );

        let (w, h) = (size.0 & !1, size.1 & !1);
        let encoder: &[&str] = if nvenc_works() {
            &[
                "-c:v",
                "h264_nvenc",
                "-preset",
                "p5",
                "-rc",
                "vbr",
                "-cq",
                "20",
                "-b:v",
                "0",
            ]
        } else {
            &["-c:v", "libx264", "-preset", "veryfast", "-crf", "20"]
        };
        let mut child = Command::new("ffmpeg")
            .args(["-y", "-v", "error", "-f", "rawvideo", "-pix_fmt", "rgba"])
            .args([
                "-s",
                &format!("{}x{}", size.0, size.1),
                "-r",
                &FPS.to_string(),
                "-i",
                "-",
            ])
            .args(["-vf", &format!("crop={w}:{h}:0:0")])
            .args(encoder)
            .args(["-pix_fmt", "yuv420p", "-an"])
            .arg(&video_path)
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|e| format!("cannot run ffmpeg: {e}"))?;
        let mut stdin = child.stdin.take().ok_or("no ffmpeg stdin")?;

        let (tx, rx) = sync_channel::<Msg>(BACKLOG);
        let worker = std::thread::spawn(move || {
            for msg in rx {
                match msg {
                    Msg::Frame(pixels, repeat) => {
                        for _ in 0..repeat {
                            stdin
                                .write_all(&pixels)
                                .map_err(|e| format!("ffmpeg pipe: {e}"))?;
                        }
                    }
                    Msg::Stop => break,
                }
            }
            drop(stdin);
            let status = child.wait().map_err(|e| e.to_string())?;
            if status.success() {
                Ok(())
            } else {
                Err(format!("ffmpeg exited with {status}"))
            }
        });

        let audio = if with_sound {
            crate::audio::start_capture();
            Some(BufWriter::new(
                File::create(&audio_path).map_err(|e| e.to_string())?,
            ))
        } else {
            None
        };
        Ok(Self {
            tx,
            worker,
            started: Instant::now(),
            written: 0,
            size,
            audio,
            audio_path,
            video_path,
            out_path,
            dropped: 0,
        })
    }

    pub fn seconds(&self) -> f32 {
        self.started.elapsed().as_secs_f32()
    }

    /// Call once per frame after the world is drawn: sends the screen to the encoder and moves the
    /// captured sound to disk.
    pub fn frame(&mut self) {
        if let Some(audio) = self.audio.as_mut() {
            let samples = crate::audio::take_capture();
            let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
            let _ = audio.write_all(&bytes);
        }
        // Constant frame rate: how many video frames the wall clock says are due.
        let due = (self.started.elapsed().as_secs_f64() * FPS as f64) as u64 + 1;
        if due <= self.written {
            return;
        }
        let repeat = (due - self.written).min(8) as u32;
        let (mut pixels, w, h) = grab_screen();
        if (w, h) != self.size {
            // The window was resized: keep the first size.
            pixels = fit(pixels, (w, h), self.size);
        }
        match self.tx.try_send(Msg::Frame(pixels, repeat)) {
            Ok(()) => self.written += repeat as u64,
            Err(_) => self.dropped += 1,
        }
    }

    /// Stops recording and finishes the file in the background.
    pub fn stop(mut self) -> Saving {
        let rate = crate::audio::stop_capture();
        if let Some(a) = self.audio.as_mut() {
            let bytes: Vec<u8> = crate::audio::take_capture()
                .iter()
                .flat_map(|s| s.to_le_bytes())
                .collect();
            let _ = a.write_all(&bytes);
            let _ = a.flush();
        }
        let sound = self.audio.is_some();
        let Recorder {
            tx,
            worker,
            audio_path,
            video_path,
            out_path,
            ..
        } = self;
        let _ = tx.send(Msg::Stop);
        let handle = std::thread::spawn(move || {
            worker
                .join()
                .map_err(|_| "encoder thread panicked".to_string())??;
            let result = if sound {
                // The raw sound is stereo f32; the device rate was measured while recording.
                let rate = rate
                    .filter(|r| (8000.0..200_000.0).contains(r))
                    .unwrap_or(48000.0);
                Command::new("ffmpeg")
                    .args(["-y", "-v", "error", "-i"])
                    .arg(&video_path)
                    .args([
                        "-f",
                        "f32le",
                        "-ar",
                        &format!("{}", rate.round() as u32),
                        "-ac",
                        "2",
                        "-i",
                    ])
                    .arg(&audio_path)
                    .args(["-c:v", "copy", "-c:a", "aac", "-b:a", "192k", "-shortest"])
                    .arg(&out_path)
                    .status()
                    .map_err(|e| e.to_string())
                    .and_then(|s| {
                        if s.success() {
                            Ok(())
                        } else {
                            Err(format!("mux failed: {s}"))
                        }
                    })
                    .map(|_| {
                        let _ = std::fs::remove_file(&video_path);
                        let _ = std::fs::remove_file(&audio_path);
                    })
            } else {
                std::fs::rename(&video_path, &out_path).map_err(|e| e.to_string())
            };
            result.map(|_| out_path)
        });
        Saving { handle }
    }
}

/// Crops or pads an RGBA image to `to` (rare: only after a window resize while recording).
fn fit(pixels: Vec<u8>, from: (i32, i32), to: (i32, i32)) -> Vec<u8> {
    let mut out = vec![0u8; (to.0 * to.1 * 4) as usize];
    let w = from.0.min(to.0) as usize * 4;
    for y in 0..from.1.min(to.1) as usize {
        let src = y * from.0 as usize * 4;
        let dst = y * to.0 as usize * 4;
        out[dst..dst + w].copy_from_slice(&pixels[src..src + w]);
    }
    out
}
