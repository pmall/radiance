//! Lots loaded on worker threads while running: each is generated (`world::generate`, a pure
//! function of the seed and the lot) and its meshes worked out there (`render::LotMeshes`), so
//! the main thread only uploads them and never stalls when the player crosses into a new lot.

use std::collections::HashSet;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};

use glam::Vec3;

use crate::render::{LotMeshes, Renderer};
use crate::world::{self, Lot, LotKey, World};

/// Lots asked of each worker at a time: few, so the nearest missing lots are always next even
/// when the player moves fast.
const PER_WORKER: usize = 2;
/// Lots taken in per frame: uploading one's meshes takes a few milliseconds.
const PER_FRAME: usize = 1;

struct Job {
    seed: u64,
    key: LotKey,
    epoch: u64,
}

struct Done {
    key: LotKey,
    epoch: u64,
    lot: Lot,
    meshes: LotMeshes,
}

pub struct Streamer {
    jobs: Sender<Job>,
    done: Receiver<Done>,
    /// Lots asked for and not back yet.
    pending: HashSet<LotKey>,
    in_flight: usize,
    /// Bumped when the world is made anew: lots on their way from before are dropped.
    epoch: u64,
}

impl Streamer {
    pub fn new() -> Self {
        let (jobs, rx) = channel::<Job>();
        let (tx, done) = channel();
        let rx = Arc::new(Mutex::new(rx));
        // Leave a core to the main thread and one to the skyline and the sound.
        let workers = std::thread::available_parallelism()
            .map_or(1, |n| n.get().saturating_sub(2))
            .clamp(1, 4);
        for _ in 0..workers {
            let (rx, tx) = (rx.clone(), tx.clone());
            std::thread::spawn(move || {
                // Ends when the streamer is dropped and the channel closes.
                while let Ok(job) = rx.lock().unwrap().recv() {
                    let lot = world::generate(job.seed, job.key);
                    let meshes = LotMeshes::of(job.seed, job.key, &lot);
                    let done = Done {
                        key: job.key,
                        epoch: job.epoch,
                        lot,
                        meshes,
                    };
                    if tx.send(done).is_err() {
                        break;
                    }
                }
            });
        }
        Self {
            jobs,
            done,
            pending: HashSet::new(),
            in_flight: workers * PER_WORKER,
            epoch: 0,
        }
    }

    /// Forgets the lots on their way: the world was made anew (another seed, chunks reloaded).
    pub fn reset(&mut self) {
        self.epoch += 1;
        self.pending.clear();
    }

    /// Loads every lot missing around `center` on all the workers at once and waits for them,
    /// meshes uploaded: the first frame, a new seed.
    pub fn fill(&mut self, world: &mut World, renderer: &mut Renderer, center: Vec3) {
        drop(world.drop_far(center));
        for key in world.missing(center) {
            let job = Job {
                seed: world.seed,
                key,
                epoch: self.epoch,
            };
            if self.jobs.send(job).is_ok() {
                self.pending.insert(key);
            }
        }
        while !self.pending.is_empty() {
            let Ok(d) = self.done.recv() else {
                break;
            };
            if d.epoch == self.epoch && self.pending.remove(&d.key) {
                renderer.add_lot(d.key, &d.lot, d.meshes);
                renderer.finish_uploads();
                world.insert(d.key, d.lot);
            }
        }
        renderer.sync_world(world);
    }

    /// Drops the lots far from `center`, takes in those that are ready and asks for the nearest
    /// missing ones.
    pub fn update(&mut self, world: &mut World, renderer: &mut Renderer, center: Vec3) {
        // Freeing a row of lots takes a few milliseconds: done aside.
        let far = world.drop_far(center);
        if !far.is_empty() {
            std::thread::spawn(move || drop(far));
        }
        for d in self.done.try_iter().take(PER_FRAME) {
            if d.epoch != self.epoch {
                continue;
            }
            self.pending.remove(&d.key);
            if world.wants(center, d.key) {
                renderer.add_lot(d.key, &d.lot, d.meshes);
                world.insert(d.key, d.lot);
            }
        }
        let free = self.in_flight.saturating_sub(self.pending.len());
        let wanted: Vec<LotKey> = world
            .missing(center)
            .into_iter()
            .filter(|k| !self.pending.contains(k))
            .take(free)
            .collect();
        for key in wanted {
            let job = Job {
                seed: world.seed,
                key,
                epoch: self.epoch,
            };
            if self.jobs.send(job).is_ok() {
                self.pending.insert(key);
            }
        }
    }
}
