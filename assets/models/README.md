# Models

Props for set dressing and points of interest (`src/world/props.rs`, placed by
`src/world/dressing.rs`, drawn instanced by `src/render/props.rs`).

`polyhaven/`: photoscanned models from [Poly Haven](https://polyhaven.com), CC0 (public domain, no
attribution required; credited here anyway). Each file is one prop, imported by
`tools/import_props.mjs` from the Poly Haven asset named in that script: only the parts kept,
decimated to 1 000 to 5 000 triangles, base color texture only, 512 px JPEG. Sources:
`modular_street_seating` (bench), `metal_trash_can`, `utility_box_01`,
`utility_box_02`, `concrete_road_barrier`, `concrete_road_barrier_02`, `trashbag`,
`cardboard_box_01`, `wooden_crate_01`, `wooden_crate_02`, `Barrel_01`, `Barrel_02`, `barrel_03`,
`old_tyre`, `metal_jerrycan`, `propane_tank`, `portable_generator`, `hand_truck`,
`steel_frame_shelves_01`, `Television_01`, `vintage_suitcase`, `plastic_monobloc_chair_01`,
`outdoor_table_chair_set_01`, `wooden_picnic_table`, `ArmChair_01`, `GreenChair_01`,
`mid_century_lounge_chair`, `sofa_03`, `sofa_02`, `wheelchair_01`, `old_bed_frame`,
`planter_pot_clay`.

To add a prop: add a line to `PROPS` in `tools/import_props.mjs` and run it
(`cd tools && npm install && node import_props.mjs <name>`), then add a `Prop` variant and a `Def`
line in `props.rs` (a turn if it does not face +z, a height if not at real scale) and use it in a
scene of `dressing.rs`. Check it with `--shot` renders.
