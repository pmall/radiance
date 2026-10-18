//! `--report`: the city's design next to what a seed actually produced, floor by floor, so tuning
//! is measured instead of guessed. Prints the floor profiles (`world::design`) and, over the lots
//! around the spawn, the decks, stairs, chunks placed, flowers by source and props by kind.

use crate::world::city::LOT;
use crate::world::planner::{self, Role};
use crate::world::props::Prop;
use crate::world::{BlockKind, chunks, city, design, floors, growth, tower};

pub fn run(seed: u64, radius: i32) {
    let n = floors::count();
    let lots = ((2 * radius + 1) * (2 * radius + 1)) as f32;
    let (mut decks, mut stairs) = (vec![0usize; n], vec![0usize; n]);
    // Lots covered per floor: by an area or its sub-areas, a block, a filler; lots opened by a
    // block's hole. And the chunks placed, by name.
    let mut area_lots = vec![0usize; n];
    let mut block_lots = vec![0usize; n];
    let mut filler_lots = vec![0usize; n];
    let mut hole_lots = vec![0usize; n];
    let mut by_name: std::collections::BTreeMap<String, usize> = Default::default();
    // The designed places (areas, blocks, craters) whose corner lies in the scanned square.
    let mut places: Vec<planner::Placement> = Vec::new();
    // The landmark towers (not housing, broken, leaning), likewise.
    let mut landmarks: Vec<planner::Placement> = Vec::new();
    // Flowers: plants (stairs apart), plants on stairs, points of interest.
    let mut flowers = vec![[0usize; 3]; n];
    // Props: street furniture, what people left; points of interest.
    let mut props = vec![[0usize; 3]; n];
    for lz in -radius..=radius {
        for lx in -radius..=radius {
            for (f, d) in decks.iter_mut().enumerate() {
                *d += (city::deck(seed, lx, lz, f) != city::Deck::None) as usize;
            }
            if let Some((lo, _)) = city::stair(seed, lx, lz) {
                stairs[lo] += 1;
            }
            for p in city::placements(seed, lx, lz) {
                match p.role {
                    Role::Area => area_lots[p.floor] += 1,
                    Role::Block => block_lots[p.floor] += 1,
                    Role::Filler => filler_lots[p.floor] += 1,
                    Role::Sub
                    | Role::Hub
                    | Role::Tower
                    | Role::Street
                    | Role::Hole
                    | Role::Crater => {}
                }
                // Counted once, at the corner lot.
                if p.origin == (lx, lz) {
                    *by_name.entry(p.chunk.clone()).or_default() += 1;
                    if matches!(p.role, Role::Area | Role::Block | Role::Crater) {
                        places.push(p.clone());
                    }
                }
            }
            if let Some(p) = planner::tower_at(seed, lx, lz)
                && p.origin == (lx, lz)
            {
                *by_name.entry(p.chunk.clone()).or_default() += 1;
                let cat = chunks::catalog(p.biome);
                if cat
                    .chunks
                    .get(&p.chunk)
                    .is_some_and(|c| c.labels.iter().any(|l| l == "landmark"))
                {
                    landmarks.push(p);
                }
            }
            if let Some(f) = planner::hole_at(seed, lx, lz) {
                hole_lots[f] += 1;
            }
            let mut blocks = Vec::new();
            let dressed = city::generate_lot_dressed(seed, lx, lz, &mut blocks);
            for p in &dressed.props {
                let street = matches!(
                    p.prop,
                    Prop::StreetLight | Prop::Bench | Prop::Bin | Prop::Planter | Prop::UtilityBox
                );
                props[floors::floor_at(p.pos.y)][if street { 0 } else { 1 }] += 1;
            }
            for b in &dressed.blooms {
                props[floors::floor_at(b.y)][2] += 1;
            }
            let stair_blocks: Vec<_> = blocks
                .iter()
                .filter(|b| b.kind == BlockKind::Stair)
                .map(|b| b.aabb)
                .collect();
            for l in growth::grow_lot(seed, lx, lz, &blocks, &dressed.blooms).lights {
                let source = if dressed.blooms.iter().any(|b| b.distance(l.pos) < 1.0) {
                    2
                } else if stair_blocks
                    .iter()
                    .any(|a| a.contains_xz(l.pos) && l.pos.y > a.max.y && l.pos.y < a.max.y + 3.0)
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
    let (hole, hub, block) = (planner::HOLE_CELL, planner::HUB_CELL, planner::BLOCK_CELL);
    println!(
        "Profiles (world/design.rs): chances in %. Holes: per hole cell ({hole}x{hole} lots). Stairs: per stair hub cell ({hub}x{hub} lots). Flowers: per"
    );
    println!(
        "plant, poi per point of interest slot. Props: chance of a furniture or leftovers slot."
    );
    println!("Blocks: per block cell ({block}x{block} lots). Fillers: per free lot.");
    println!(
        "{:<20} {:>6} {:>6} {:>6} | {:>6} {:>6} {:>8} {:>6} | {:>6} | {:>6} {:>7}",
        "floor",
        "holes",
        "plants",
        "stairs",
        "bush",
        "vine",
        "climber",
        "poi",
        "props",
        "blocks",
        "fillers"
    );
    for f in (0..n).rev() {
        let p = design::profile(f);
        let pc = |x: f32| format!("{:.0}", x * 100.0);
        println!(
            "{:<20} {:>6} {:>6} {:>6} | {:>6} {:>6} {:>8} {:>6} | {:>6} | {:>6} {:>7}",
            floors::name(f),
            pc(p.holes),
            format!("x{:.2}", p.plants),
            pc(p.stair),
            pc(p.flowers.bush),
            pc(p.flowers.vine),
            format!("{:.1}", p.flowers.climber * 100.0),
            format!("{:.1}", p.flowers.poi * 100.0),
            pc(p.props),
            pc(p.blocks),
            pc(p.fillers),
        );
    }
    println!();
    println!(
        "Measured: decks, chunks and holes in % of lots, stairs in % of hub cells, then counts per lot"
    );
    println!(
        "{:<20} {:>6} {:>6} {:>6} {:>7} {:>5} {:>6} | {:>7} {:>7} {:>7} {:>7} | {:>7} {:>9} {:>5}",
        "floor",
        "decks",
        "areas",
        "blocks",
        "fillers",
        "holes",
        "stairs",
        "flowers",
        "plants",
        "stairs",
        "poi",
        "street",
        "leftovers",
        "poi"
    );
    let hub_lots = (planner::HUB_CELL * planner::HUB_CELL) as f32;
    for f in (0..n).rev() {
        let per = |x: usize| format!("{:.2}", x as f32 / lots);
        let fl = flowers[f];
        let pr = props[f];
        println!(
            "{:<20} {:>6} {:>6} {:>6} {:>7} {:>5} {:>6} | {:>7} {:>7} {:>7} {:>7} | {:>7} {:>9} {:>5}",
            floors::name(f),
            format!("{:.0}", decks[f] as f32 * 100.0 / lots),
            format!("{:.0}", area_lots[f] as f32 * 100.0 / lots),
            format!("{:.0}", block_lots[f] as f32 * 100.0 / lots),
            format!("{:.0}", filler_lots[f] as f32 * 100.0 / lots),
            format!("{:.0}", hole_lots[f] as f32 * 100.0 / lots),
            format!("{:.0}", stairs[f] as f32 * 100.0 * hub_lots / lots),
            per(fl.iter().sum()),
            per(fl[0]),
            per(fl[1]),
            per(fl[2]),
            per(pr[0]),
            per(pr[1]),
            per(pr[2]),
        );
    }
    println!();
    println!("Chunks placed (whole or in part around the spawn):");
    for (name, count) in &by_name {
        println!("  {name:<24} {count}");
    }
    println!();
    println!("Places (areas, blocks, craters), nearest first, with a view standing at the middle:");
    let middle = |p: &planner::Placement| {
        (
            (p.origin.0 as f32 + p.size.0 as f32 * 0.5) * LOT,
            (p.origin.1 as f32 + p.size.1 as f32 * 0.5) * LOT,
        )
    };
    places.sort_by(|a, b| {
        let (da, db) = (middle(a), middle(b));
        (da.0.hypot(da.1)).total_cmp(&db.0.hypot(db.1))
    });
    for p in &places {
        let (x, z) = middle(p);
        println!(
            "  {:<20} {:<8} {:>2}x{:<2} {}  --view {x:.0},{:.0},{z:.0},0,0",
            p.chunk,
            format!("{:?}", p.role),
            p.size.0,
            p.size.1,
            floors::name(p.floor),
            floors::y(p.floor),
        );
    }
    println!();
    println!(
        "Landmark towers, nearest first, with a view from a lot away (the top deck for the \
         construction front):"
    );
    let crooked = landmarks
        .iter()
        .filter(|p| tower::leans(&chunks::catalog(p.biome), p, seed))
        .count();
    println!(
        "  ({} landmark towers, {crooked} of them leaning)",
        landmarks.len()
    );
    landmarks.sort_by(|a, b| {
        let (da, db) = (middle(a), middle(b));
        (da.0.hypot(da.1)).total_cmp(&db.0.hypot(db.1))
    });
    for p in landmarks.iter().take(400) {
        let (x, z) = middle(p);
        let top = floors::bottom() + p.stories as f32 * chunks::STORY;
        let front = top > floors::top();
        let crooked = tower::leans(&chunks::catalog(p.biome), p, seed);
        let y = if front {
            floors::top()
        } else {
            floors::y(floors::START)
        };
        println!(
            "  {:<20} {:>3} m{}  --view {:.0},{y:.0},{:.0},45,25",
            p.chunk,
            (top - floors::bottom()).round(),
            if crooked { ", leaning" } else { "" },
            x + 1.5 * LOT,
            z + 1.5 * LOT,
        );
    }
}
