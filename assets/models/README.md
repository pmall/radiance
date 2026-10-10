# Models

Props for set dressing and points of interest (`src/world/props.rs`, placed by
`src/world/dressing.rs`). Only the shapes are used: colors are remapped to the city's material
families (`docs/art-direction.md`).

All from [Kenney](https://kenney.nl), CC0 (public domain); each kit's `License.txt` is kept beside
its models:

- `furniture-kit`: armchair, sofa, benches, tables, chairs, coat rack, washer, dryer, trash can,
  potted plant, desk, desk chair, bookcase, cardboard boxes, floor lamp, vintage TV, fridge.
- `retro-urban-kit`: street bench, dumpsters, barriers, pallet, old truck (with its textures).
- `city-kit-roads`: cone, road barrier, traffic light, street sign.
- `car-kit`: sedan, hatchback, van, taxi, SUV, tire.
- `mini-market`: shopping cart, freezer.
- `survival-kit`: barrel, crate, bucket.

To add a prop: copy the `.glb` (and any texture it uses into the kit's `Textures/`), add a
`Prop` variant and a `Def` line (file, real height) in `props.rs`, then use it in a scene of
`dressing.rs`.
