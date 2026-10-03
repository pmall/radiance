#!/usr/bin/env python3
"""Packs the luminance detail of the CC0 Poly Haven textures in assets/textures into one RGBA
image, assets/textures/detail.png, that the scene shader samples (embedded in the binary).

detail.png channels: R pavement, G asphalt, B concrete wall, A metal plate.
detail2.png channels: R roof bitumen, G leafy grass (foliage), B rusty metal, A grey plaster. Each is high-passed (so only
fine detail is left, no lighting gradients from the photo), then centered on 0.5. The shader
multiplies its procedural colors by 2 * value, so the palette stays ours and the photo adds grit.

Sources (all CC0, polyhaven.com): concrete_pavement, worn_asphalt, concrete_slab_wall,
metal_plate_02, bitumen, leafy_grass, rusty_metal_02, grey_plaster_02.
Run: python3 tools/pack_textures.py   (needs Pillow)
"""
from PIL import Image, ImageFilter, ImageChops, ImageStat

SETS = {
    "detail.png": ["pavement", "asphalt", "wall", "metal"],
    "detail2.png": ["roof", "foliage", "rust", "plaster"],
}
SIZE = 1024
GAIN = 1.0


def detail(path):
    im = Image.open(path).convert("L").resize((SIZE, SIZE), Image.LANCZOS)
    # Blur on a 3x3 tiling so the result stays seamless.
    tiled = Image.new("L", (SIZE * 3, SIZE * 3))
    for y in range(3):
        for x in range(3):
            tiled.paste(im, (x * SIZE, y * SIZE))
    low = tiled.filter(ImageFilter.GaussianBlur(40)).crop((SIZE, SIZE, SIZE * 2, SIZE * 2))
    hp = Image.eval(ImageChops.subtract(im, low, scale=1.0, offset=128), lambda v: v)
    # Normalise contrast to a standard deviation of about 36/255.
    std = ImageStat.Stat(hp).stddev[0] or 1.0
    k = GAIN * 36.0 / std
    return hp.point(lambda v: max(0, min(255, int(128 + (v - 128) * k))))


for out_name, names in SETS.items():
    out = Image.merge("RGBA", [detail(f"assets/textures/{n}.jpg") for n in names])
    out.save(f"assets/textures/{out_name}", optimize=True)
    print("wrote", out_name, out.size)
