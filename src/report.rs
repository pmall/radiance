//! `--report`: the city's design next to what a seed actually produced, floor by floor, so tuning
//! is measured instead of guessed. Prints the floor profiles (`world::design`) and, over the lots
//! around the spawn, the decks, stairs, flowers by source and props by kind.

use crate::world::dressing::{self, Kind};
use crate::world::{BlockKind, city, design, floors, growth};

pub fn run(seed: u64, radius: i32) {
    let n = floors::count();
    let lots = ((2 * radius + 1) * (2 * radius + 1)) as f32;
    let (mut decks, mut stairs) = (vec![0usize; n], vec![0usize; n]);
    // Lots that can hold a stair: no tower, no crater, not the plaza.
    let mut stair_lots = 0usize;
    // Flowers: plants, stair landings, points of interest.
    let mut flowers = vec![[0usize; 3]; n];
    // Props by kind: street, leftovers, vehicles, points of interest.
    let mut props = vec![[0usize; 4]; n];
    for lz in -radius..=radius {
        for lx in -radius..=radius {
            for (f, d) in decks.iter_mut().enumerate() {
                *d += (city::deck(seed, lx, lz, f) != city::Deck::None) as usize;
            }
            stair_lots += (city::tower(seed, lx, lz).is_none()
                && city::crater_at(seed, lx, lz).is_none()
                && !city::is_plaza(lx, lz)) as usize;
            if let Some((lo, _)) = city::stair(seed, lx, lz) {
                stairs[lo] += 1;
            }
            let mut blocks = Vec::new();
            city::generate_lot(seed, lx, lz, &mut blocks);
            let dressing = dressing::dress_lot(seed, lx, lz, &blocks);
            for (p, k) in dressing.props.iter().zip(&dressing.kinds) {
                let i = match k {
                    Kind::Street => 0,
                    Kind::Leftovers => 1,
                    Kind::Vehicle => 2,
                    Kind::Poi => 3,
                };
                props[floors::floor_at(p.pos.y)][i] += 1;
            }
            let landings: Vec<_> = blocks
                .iter()
                .filter(|b| {
                    let s = b.aabb.size();
                    b.kind == BlockKind::Stair && s.x > 1.5 && s.z > 1.5
                })
                .map(|b| b.aabb)
                .collect();
            for l in growth::grow_lot(seed, lx, lz, &blocks).lights {
                let source = if dressing.blooms.iter().any(|b| b.distance(l.pos) < 1.0) {
                    2
                } else if landings
                    .iter()
                    .any(|a| a.contains_xz(l.pos) && (l.pos.y - a.max.y).abs() < 1.0)
                {
                    1
                } else {
                    0
                };
                flowers[floors::floor_at(l.pos.y)][source] += 1;
            }
        }
    }

    println!(
        "Radiance report: seed {seed}, {n} floors, {lots} lots around the spawn. Props {}, points of interest {}.",
        if design::props_on() { "on" } else { "off" },
        if design::pois_on() { "on" } else { "off" },
    );
    println!();
    println!(
        "Profiles (world/design.rs): chances in %. Stairs: per lot without a tower. Flowers: per"
    );
    println!(
        "plant or landing, poi per lot. Props: multiplier of the tries per lot. Cars: tries per lot."
    );
    println!(
        "{:<20} {:>6} {:>6} {:>6} | {:>6} {:>6} {:>8} {:>8} {:>6} | {:>6} {:>5}",
        "floor",
        "decks",
        "plants",
        "stairs",
        "bush",
        "vine",
        "climber",
        "landing",
        "poi",
        "props",
        "cars"
    );
    for f in (0..n).rev() {
        let p = design::profile(f);
        let pc = |x: f32| format!("{:.0}", x * 100.0);
        println!(
            "{:<20} {:>6} {:>6} {:>6} | {:>6} {:>6} {:>8} {:>8} {:>6} | {:>6} {:>5}",
            floors::name(f),
            pc(p.deck_keep),
            format!("x{:.2}", p.plants),
            pc(p.stair),
            pc(p.flowers.bush),
            pc(p.flowers.vine),
            format!("{:.1}", p.flowers.climber * 100.0),
            pc(p.flowers.landing),
            format!("{:.1}", p.flowers.poi * 100.0),
            format!("x{:.2}", p.props),
            format!("{:.2}", p.cars),
        );
    }
    println!();
    println!(
        "Measured: decks in % of lots, stairs in % of lots without a tower, then counts per lot"
    );
    println!(
        "{:<20} {:>6} {:>6} | {:>7} {:>7} {:>7} {:>7} | {:>7} {:>9} {:>6} {:>5}",
        "floor",
        "decks",
        "stairs",
        "flowers",
        "plants",
        "landing",
        "poi",
        "street",
        "leftovers",
        "cars",
        "poi"
    );
    for f in (0..n).rev() {
        let per = |x: usize| format!("{:.2}", x as f32 / lots);
        let fl = flowers[f];
        let pr = props[f];
        println!(
            "{:<20} {:>6} {:>6} | {:>7} {:>7} {:>7} {:>7} | {:>7} {:>9} {:>6} {:>5}",
            floors::name(f),
            format!("{:.0}", decks[f] as f32 * 100.0 / lots),
            format!("{:.0}", stairs[f] as f32 * 100.0 / stair_lots.max(1) as f32),
            per(fl.iter().sum()),
            per(fl[0]),
            per(fl[1]),
            per(fl[2]),
            per(pr[0]),
            per(pr[1]),
            per(pr[2]),
            per(pr[3]),
        );
    }
}
