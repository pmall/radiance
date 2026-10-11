// Imports the props' models from Poly Haven (CC0, https://polyhaven.com) into assets/models/polyhaven:
// downloads each source model (1k textures), keeps the parts listed, decimates it to a triangle
// budget, keeps only the base color texture (resized to TEXTURE px, JPEG) and writes one .glb per
// prop. The game loads these (src/world/props.rs); run again after changing the list below.
//
//   cd tools && npm install && node import_props.mjs [name ...]

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { NodeIO } from '@gltf-transform/core';
import { dedup, flatten, join, prune, simplify, textureCompress, weld } from '@gltf-transform/functions';
import { MeshoptSimplifier } from 'meshoptimizer';
import sharp from 'sharp';

const here = path.dirname(fileURLToPath(import.meta.url));
const OUT = path.join(here, '..', 'assets', 'models', 'polyhaven');
const CACHE = path.join(here, '.cache');
/// Side of the base color texture, in pixels (one tile of the game's prop atlas).
const TEXTURE = 512;

// out: file written; src: Poly Haven asset; keep: which top-level nodes to keep (all if absent);
// tris: triangle budget; error: how far the simplified surface may stray, relative to the model's
// size (0.02 unless given), for models whose fine lattices cannot meet the budget otherwise.
const PROPS = [
  // Street furniture.
  { out: 'bench', src: 'modular_street_seating', tris: 3000,
    keep: n => ['legs_double', 'legs_single', 'crossbar', 'suspended_support_01', 'back_support_r',
      'back_support_l', 'arm_rest_01', 'arm_rest_02', 'seat', 'seat_back'].includes(n) },
  { out: 'trash_can', src: 'metal_trash_can', tris: 2000, keep: n => !n.includes('rust') },
  { out: 'trash_can_rusty', src: 'metal_trash_can', tris: 2000, keep: n => n.includes('rust') },
  { out: 'utility_box', src: 'utility_box_01', tris: 1500 },
  { out: 'utility_box_wide', src: 'utility_box_02', tris: 1500 },
  { out: 'barrier_concrete', src: 'concrete_road_barrier', tris: 2000 },
  { out: 'barrier_jersey', src: 'concrete_road_barrier_02', tris: 2000 },
  // What people left.
  { out: 'trash_bag', src: 'trashbag', tris: 1200 },
  { out: 'cardboard_box', src: 'cardboard_box_01', tris: 800 },
  { out: 'crate', src: 'wooden_crate_01', tris: 1200 },
  { out: 'crate_long', src: 'wooden_crate_02', tris: 1200 },
  { out: 'barrel_red', src: 'Barrel_01', tris: 1000 },
  { out: 'barrel_blue', src: 'Barrel_02', tris: 1000 },
  { out: 'barrel_rusty', src: 'barrel_03', tris: 1000 },
  { out: 'tyre', src: 'old_tyre', tris: 800 },
  { out: 'jerrycan', src: 'metal_jerrycan', tris: 800 },
  { out: 'propane_tank', src: 'propane_tank', tris: 1000 },
  { out: 'generator', error: 0.05, src: 'portable_generator', tris: 2000 },
  { out: 'hand_truck', error: 0.05, src: 'hand_truck', tris: 1500 },
  { out: 'shelves', src: 'steel_frame_shelves_01', tris: 1500 },
  { out: 'television', src: 'Television_01', tris: 1000 },
  { out: 'suitcase', src: 'vintage_suitcase', tris: 1500, keep: n => n.startsWith('vintage_suitcase_01') },
  { out: 'plastic_chair', src: 'plastic_monobloc_chair_01', tris: 1200 },
  { out: 'terrace_table', src: 'outdoor_table_chair_set_01', tris: 1000, keep: n => n.endsWith('_table') },
  { out: 'terrace_chair', src: 'outdoor_table_chair_set_01', tris: 1000, keep: n => n.endsWith('_chair_01') },
  { out: 'picnic_table', src: 'wooden_picnic_table', tris: 2000 },
  { out: 'armchair', src: 'ArmChair_01', tris: 2000 },
  { out: 'armchair_green', src: 'GreenChair_01', tris: 2000 },
  { out: 'lounge_chair', src: 'mid_century_lounge_chair', tris: 2000 },
  { out: 'sofa', src: 'sofa_03', tris: 2500 },
  { out: 'sofa_leather', src: 'sofa_02', tris: 1500 },
  { out: 'wheelchair', src: 'wheelchair_01', tris: 2500 },
  { out: 'bed_frame', error: 0.05, src: 'old_bed_frame', tris: 2500 },
  { out: 'clay_pot', src: 'planter_pot_clay', tris: 600 },
];

const UA = { 'User-Agent': 'radiance-asset-import/1.0' };

async function download(src) {
  const dir = path.join(CACHE, src);
  const files = await (await fetch(`https://api.polyhaven.com/files/${src}`, { headers: UA })).json();
  const g = files.gltf['1k'].gltf;
  const main = path.join(dir, path.basename(new URL(g.url).pathname));
  if (fs.existsSync(main)) return main;
  const get = async (url, to) => {
    fs.mkdirSync(path.dirname(to), { recursive: true });
    const res = await fetch(url, { headers: UA });
    if (!res.ok) throw new Error(`${url}: ${res.status}`);
    fs.writeFileSync(to, Buffer.from(await res.arrayBuffer()));
  };
  for (const [rel, f] of Object.entries(g.include ?? {})) await get(f.url, path.join(dir, rel));
  await get(g.url, main);
  return main;
}

function triangles(doc) {
  let n = 0;
  for (const m of doc.getRoot().listMeshes())
    for (const p of m.listPrimitives())
      n += (p.getIndices()?.getCount() ?? p.getAttribute('POSITION').getCount()) / 3;
  return n;
}

const io = new NodeIO();
await MeshoptSimplifier.ready;
fs.mkdirSync(OUT, { recursive: true });
const only = process.argv.slice(2);
for (const p of PROPS.filter(p => only.length === 0 || only.includes(p.out))) {
  const doc = await io.read(await download(p.src));
  const scene = doc.getRoot().getDefaultScene() ?? doc.getRoot().listScenes()[0];
  if (p.keep) for (const n of scene.listChildren()) if (!p.keep(n.getName())) n.dispose();
  // Only the base color is used: the game lights props its own way.
  for (const m of doc.getRoot().listMaterials()) {
    m.setAlphaMode('OPAQUE').setNormalTexture(null).setOcclusionTexture(null).setMetallicRoughnessTexture(null)
      .setEmissiveTexture(null);
  }
  for (const e of doc.getRoot().listExtensionsUsed()) e.dispose();
  await doc.transform(prune(), dedup(), flatten(), join(), weld());
  const before = triangles(doc);
  await doc.transform(
    simplify({ simplifier: MeshoptSimplifier, ratio: Math.min(1, p.tris / before), error: p.error ?? 0.02 }),
    prune(),
    textureCompress({ encoder: sharp, targetFormat: 'jpeg', quality: 88, resize: [TEXTURE, TEXTURE] }),
  );
  const file = path.join(OUT, `${p.out}.glb`);
  await io.write(file, doc);
  console.log(`${p.out}: ${before} -> ${triangles(doc)} triangles, ${(fs.statSync(file).size / 1024).toFixed(0)} KB`);
}
