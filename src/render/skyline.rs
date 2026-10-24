//! The far skyline's meshes (`world::skyline`): computed on a worker thread whenever the player
//! has moved a few lots from where the last one was made, uploaded a batch per frame and swapped
//! in when all are. The scene shader leaves out what falls in a loaded lot (`uLoaded`), where the
//! lot's own meshes draw the tower.

use std::sync::mpsc::{Receiver, channel};

use crate::world::{Lot, skyline};

use super::mesh::{Arrays, Part, WorldMesh};

/// How far the player moves, in lots, before the skyline is made again.
const REMAKE: i32 = 3;
/// Boxes per mesh, so each uploads in a frame without a stall (a box is a few dozen vertices).
const BATCH: usize = 1500;

#[derive(Default)]
pub struct Skyline {
    pub meshes: Vec<WorldMesh>,
    /// Seed and center lot of the meshes, or of the job making them.
    made: Option<(u64, (i32, i32))>,
    job: Option<Receiver<Vec<Arrays>>>,
    /// The next skyline: its meshes uploaded so far, and the arrays still to upload.
    next: Option<(Vec<WorldMesh>, Vec<Arrays>)>,
}

impl Skyline {
    /// Starts making the skyline around `center` when the player has moved far enough from the
    /// last one (or the seed changed), uploads it a mesh per call and swaps it in when done;
    /// `wait` blocks until it is.
    pub fn update(&mut self, seed: u64, center: (i32, i32), wait: bool) {
        let far = self.made.is_none_or(|(s, (x, z))| {
            s != seed || (x - center.0).abs().max((z - center.1).abs()) >= REMAKE
        });
        if far && self.job.is_none() && self.next.is_none() {
            let (tx, rx) = channel();
            std::thread::spawn(move || {
                let _ = tx.send(arrays(seed, center));
            });
            self.job = Some(rx);
            self.made = Some((seed, center));
        }
        if let Some(rx) = &self.job {
            let made = if wait {
                rx.recv().ok()
            } else {
                rx.try_recv().ok()
            };
            if let Some(arrays) = made {
                self.job = None;
                self.next = Some((Vec::new(), arrays));
            }
        }
        let Some((done, todo)) = &mut self.next else {
            return;
        };
        while let Some(a) = todo.pop() {
            done.push(WorldMesh::upload(a));
            if !wait {
                break;
            }
        }
        if todo.is_empty() {
            self.meshes = std::mem::take(done);
            self.next = None;
        }
    }

    pub fn clear(&mut self) {
        self.meshes.clear();
        self.made = None;
        self.job = None;
        self.next = None;
    }
}

/// The skyline's vertex arrays around `center`, in batches of `BATCH` boxes of one variant.
fn arrays(seed: u64, center: (i32, i32)) -> Vec<Arrays> {
    let mut out = Vec::new();
    for (variant, blocks) in skyline::blocks(seed, center) {
        for (i, batch) in blocks.chunks(BATCH).enumerate() {
            let key = crate::rng::hash_coords(seed, center.0, 0x5c1 + i as i32, center.1)
                ^ variant as u64;
            let lot = Lot::scenery(batch.to_vec(), variant);
            out.extend(WorldMesh::arrays(&lot, key, Part::Architecture));
        }
    }
    out
}
