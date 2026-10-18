//! The far skyline: the towers of the construction front, rising past the top deck, out to about
//! a kilometer, past the lots that are loaded. Drawn whole but alone (the decks and the lower
//! towers are not drawn that far), so they stand as silhouettes in the haze, their feet lost in
//! it. Pure, like the lots: computed off the main thread (`render::skyline`).

use super::{Block, LOAD_RADIUS, biome, chunks, floors, planner, tower};

/// How far the skyline reaches, in lots (Chebyshev distance): about a kilometer.
pub const RADIUS: i32 = 44;

/// The silhouettes of the towers standing within `RADIUS` lots of `center` and past the lots
/// loaded around it, grouped by the material variant of their biome.
pub fn blocks(seed: u64, (cx, cz): (i32, i32)) -> Vec<(u8, Vec<Block>)> {
    let mut out: Vec<(u8, Vec<Block>)> = Vec::new();
    for z in cz - RADIUS..=cz + RADIUS {
        for x in cx - RADIUS..=cx + RADIUS {
            if (x - cx).abs().max((z - cz).abs()) < LOAD_RADIUS {
                continue;
            }
            let Some(p) = planner::tower_at(seed, x, z) else {
                continue;
            };
            // Each tower once, from its corner lot, and only those over the top deck.
            let top = floors::bottom() + p.stories as f32 * chunks::STORY;
            if p.origin != (x, z) || top <= floors::top() + 1.0 {
                continue;
            }
            let cat = chunks::catalog(p.biome);
            let variant = biome::variant_of(biome::biome_at(seed, x, z)) as u8;
            let blocks = tower::silhouette(&cat, &p, seed);
            match out.iter_mut().find(|(v, _)| *v == variant) {
                Some((_, b)) => b.extend(blocks),
                None => out.push((variant, blocks)),
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_skyline_rises_over_the_top_deck() {
        let blocks: Vec<Block> = blocks(1, (0, 0)).into_iter().flat_map(|(_, b)| b).collect();
        assert!(blocks.len() > 100, "only {} skyline boxes", blocks.len());
        assert!(
            blocks
                .iter()
                .filter(|b| b.aabb.max.y > floors::top() + 30.0)
                .count()
                > 50,
            "few silhouettes over the top deck"
        );
    }
}
