//! The far skyline's meshes (`world::skyline`): computed on a worker thread whenever the player
//! has moved a few lots from where the last one was made, swapped in when ready. The scene shader
//! leaves out what falls in a loaded lot (`uLoaded`), where the lot's own meshes draw the tower.

use std::sync::mpsc::{Receiver, channel};

use crate::world::{Lot, skyline};

use super::mesh::{Part, WorldMesh};

/// How far the player moves, in lots, before the skyline is made again.
const REMAKE: i32 = 3;

type Made = (u64, (i32, i32), Vec<(u8, Vec<crate::world::Block>)>);

#[derive(Default)]
pub struct Skyline {
    pub meshes: Vec<WorldMesh>,
    /// Seed and center lot of the meshes, or of the job making them.
    made: Option<(u64, (i32, i32))>,
    job: Option<Receiver<Made>>,
}

impl Skyline {
    /// Starts making the skyline around `center` when the player has moved far enough from the
    /// last one (or the seed changed), and swaps it in when done; `wait` blocks until it is.
    pub fn update(&mut self, seed: u64, center: (i32, i32), wait: bool) {
        let far = self.made.is_none_or(|(s, (x, z))| {
            s != seed || (x - center.0).abs().max((z - center.1).abs()) >= REMAKE
        });
        if far && self.job.is_none() {
            let (tx, rx) = channel();
            std::thread::spawn(move || {
                let _ = tx.send((seed, center, skyline::blocks(seed, center)));
            });
            self.job = Some(rx);
            self.made = Some((seed, center));
        }
        let Some(rx) = &self.job else {
            return;
        };
        let made = if wait {
            rx.recv().ok()
        } else {
            rx.try_recv().ok()
        };
        if let Some((s, c, groups)) = made {
            self.job = None;
            self.meshes = groups
                .into_iter()
                .filter_map(|(variant, blocks)| {
                    let key = crate::rng::hash_coords(s, c.0, 0x5c1, c.1) ^ variant as u64;
                    WorldMesh::build(&Lot::scenery(blocks, variant), key, Part::Architecture)
                })
                .collect();
        }
    }

    pub fn clear(&mut self) {
        self.meshes.clear();
        self.made = None;
        self.job = None;
    }
}
