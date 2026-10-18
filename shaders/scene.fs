#version 330

// Scene geometry: flat cel-shaded lighting in linear HDR, plus normals and object id
// for the outline pass.

in vec3 fragWorldPos;
in vec3 fragNormal;
in vec4 fragColor;
flat in float fragId;
flat in float fragKind;      // material: 0 tower, 1 plant, 2 floor/deck, 3 bridge, 4 stair, 5 test block
in vec2 fragLoc;             // position along a vertical face, face width

uniform vec3 uLightDir;      // toward the key light
uniform vec3 uLight;         // key light color * intensity
uniform vec3 uAmbientSky;
uniform vec3 uAmbientGround;
uniform int uEffects;
uniform sampler2D texture1;  // shadow map depth
uniform mat4 uLightVP;
uniform int uShadows;
uniform float uShadowTexel;
uniform sampler2D texture2;  // plant light grid (see src/render/lights.rs)
uniform sampler2D texture4;  // photo detail 2: R roof, G foliage, B rust, A plaster
uniform vec3 uGridOrigin;
uniform vec3 uPalette[8 * 9];    // per biome (src/world/biome.rs, Palette): 8 wall paints, 1 pavement tint
uniform float uGlow;         // plant emission and light strength for the time of day
uniform sampler2D texture5;  // cover map
uniform sampler2D texture10; // chunk materials and facade units in pixel art, a row per biome (src/render/pixels.rs)
uniform sampler2D texture6;  // tower elevations (src/render/facades.rs)
uniform sampler2D texture0;  // noise (src/render/noise.rs): value noise of 1, 2, 4, 8 cells per unit
uniform vec3 uWear[2 * 8];   // per biome (src/render/pixels.rs, wear_uniform): (moss, damp, dust), (litter, puddles, markings)
uniform vec3 uCamPos;
#define COVER_TEX texture5

#include "cover.glsl"

// Height above the floor p stands on.
float aboveFloor(float y)
{
    float base = uFloorY[0];
    for (int f = 1; f < FLOORS; f++) if (f < uFloorCount && uFloorY[f] <= y + 0.5) base = uFloorY[f];
    return y - base;
}

layout(location = 0) out vec4 outColor;
layout(location = 1) out vec4 outNormal;

const int FX_CEL = 4;
const int FX_TEXTURE = 128;

// Smooth step of fixed width so band edges stay crisp but not aliased on curved surfaces.
float band(float x, float edge)
{
    float w = max(fwidth(x), 0.01);
    return smoothstep(edge - w, edge + w, x);
}

// Fraction of key light reaching p (1 = fully lit), 3x3 PCF.
float shadow(vec3 p, vec3 n)
{
    if (uShadows == 0) return 1.0;
    // Normal offset keeps surfaces from shadowing themselves at grazing angles.
    vec4 ls = uLightVP * vec4(p + n * 0.12, 1.0);
    vec3 s = ls.xyz / ls.w * 0.5 + 0.5;
    if (any(lessThan(s.xy, vec2(0.0))) || any(greaterThan(s, vec3(1.0)))) return 1.0;
    float lit = 0.0;
    for (int x = -1; x <= 1; x++)
        for (int y = -1; y <= 1; y++)
            lit += step(s.z - 0.0002, texture(texture1, s.xy + vec2(x, y) * uShadowTexel).r);
    lit /= 9.0;
    // Fade out toward the edge of the covered area.
    vec2 e = abs(s.xy - 0.5) * 2.0;
    return mix(lit, 1.0, smoothstep(0.85, 1.0, max(e.x, e.y)));
}

// Light grid layout, mirrored from src/render/lights.rs.
const int LW = 256;
const int MAX_LIGHTS = 512;
const ivec3 GRID = ivec3(32, 16, 32);
const float CELL = 6.0;
const float WALL_REACH = 1.6;  // lights.rs bins each light this much further for the walls

vec4 fetchTexel(int i)
{
    return texelFetch(texture2, ivec2(i % LW, i / LW), 0);
}

// Light from the luminous plants near p, banded to match the cel look.
vec3 plantLights(vec3 p, vec3 n)
{
    ivec3 c = ivec3(floor((p - uGridOrigin) / CELL));
    if (any(lessThan(c, ivec3(0))) || any(greaterThanEqual(c, GRID))) return vec3(0.0);
    int base = MAX_LIGHTS * 3 + (c.x + GRID.x * (c.y + GRID.y * c.z)) * 2;
    vec3 acc = vec3(0.0);
    for (int h = 0; h < 2; h++) {
        vec4 ids = fetchTexel(base + h);
        for (int k = 0; k < 4; k++) {
            if (ids[k] < 0.0) continue;
            int i = int(ids[k]);
            vec4 a = fetchTexel(i * 3);
            vec4 band = fetchTexel(i * 3 + 2);
            // Decks stop the light: only surfaces between the floor and ceiling of the flower.
            if (p.y < band.x - 0.1 || p.y > band.y + 0.1) continue;
            vec3 d = a.xyz - p;
            float dist = length(d);
            float facing = dot(n, d / max(dist, 1e-3));
            // Walls facing a flower catch its light further off, a softer wash up the wall.
            float wall = (1.0 - abs(n.y)) * smoothstep(0.1, 0.6, facing);
            float x = 1.0 - dist / (a.w * mix(1.0, WALL_REACH, wall));
            if (x <= 0.0) continue;
            float q = 0.34 * smoothstep(0.03, 0.13, x) + 0.33 * smoothstep(0.30, 0.40, x)
                    + 0.33 * smoothstep(0.65, 0.75, x);
            acc += fetchTexel(i * 3 + 1).rgb * q * clamp(facing * 0.5 + 0.6, 0.0, 1.0) * mix(1.0, 0.6, wall);
        }
    }
    return acc;
}


// ---- Materials (procedural, world space) ----
// Architecture is dressed by kind: tower facades (storefronts, windows, neon), paved streets with
// markings, roofs, undersides, bridge metal. Variety comes from the block id and world position,
// both fixed by the seed.

float hash12(vec2 p)
{
    vec3 q = fract(vec3(p.xyx) * 0.1031);
    q += dot(q, q.yzx + 33.33);
    return fract((q.x + q.y) * q.z);
}

float vnoise(vec2 p)
{
    vec2 i = floor(p), f = fract(p);
    f = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash12(i), hash12(i + vec2(1, 0)), f.x),
               mix(hash12(i + vec2(0, 1)), hash12(i + vec2(1, 1)), f.x), f.y);
}

float fbm(vec2 p)
{
    return 0.5 * vnoise(p) + 0.3 * vnoise(p * 2.07 + 5.3) + 0.2 * vnoise(p * 4.3 + 11.1);
}

// The same from the noise texture, one fetch: vnoise at p, 2p, 4p and 8p in its channels.
vec4 noise4(vec2 p)
{
    return textureLod(texture0, p / 8.0, 0.0);
}

float tfbm(vec2 p)
{
    return dot(noise4(p).rgb, vec3(0.5, 0.3, 0.2));
}

// Fine detail from photographs, centered on 1 (G foliage), on the plants only.
vec4 grit2(vec2 uv)
{
    return texture(texture4, uv) * 2.0;
}

// 1 inside a box of half-size `h` centered on the origin, antialiased over `aa` meters.
float boxMask(vec2 q, vec2 h, float aa)
{
    vec2 d = abs(q) - h;
    return 1.0 - smoothstep(-aa, aa, max(d.x, d.y));
}

int gVariant = 0;   // palette (biome) of the fragment's lot, set in main()

vec3 pal(int slot)
{
    return uPalette[gVariant * 9 + slot];
}

// The atlas of pixel-art tiles (src/render/pixels.rs): a row per biome, each the chunk materials,
// the facade units, the city's surfaces and a tile of swatches, 4 m per tile. Drawn from the row
// of the fragment's biome; `t` is in tiles; crisp pixels up close.
const vec2 ATLAS = vec2(27.0, 8.0);  // tiles in a row (pixels::COUNT), rows (MAX_BIOMES)
const float ROAD_TILE = 22.0;        // then sidewalk, roof, deck underside (pixels::SURFACES)
const float SIDEWALK_TILE = 23.0;
const float ROOF_TILE = 24.0;
const float UNDERSIDE_TILE = 25.0;
const int SWATCH_TILE = 26;
vec4 atlasTile(float tile, vec2 t)
{
    vec2 a = (vec2(tile, float(gVariant)) + fract(t)) / ATLAS;
    // Gradients of the unwrapped coordinates, so the tile seams never pick a blurry mip level.
    vec2 dx = dFdx(t) / ATLAS;
    vec2 dy = dFdy(t) / ATLAS;
    vec4 c = textureGrad(texture10, a, dx, dy);
    return vec4(pow(c.rgb, vec3(2.2)), c.a);
}


// ---- Wear: what time did to the city ----
// Drawn over every surface on the textures' pixel grid (16 pixels per meter), dithered like them,
// in the biome's swatches and by its amounts (src/render/pixels.rs, Textures), and by the floor:
// the depths dusty and stained, the canopy mossy and strewn with leaves; open sky brings rain.

float gWet = 0.0;   // standing water at the fragment (groundWear), for the lighting

// A color of the biome's swatch row (0 dust, 1 litter, 2 moss, 3 water, 4 corrosion, 5 asphalt),
// `t` from dark to light.
vec3 swatch(int row, float t)
{
    int i = clamp(int(t * 4.0 + 0.5), 0, 4);
    vec3 c = texelFetch(texture10, ivec2(SWATCH_TILE * 64 + i, gVariant * 64 + row), 0).rgb;
    return pow(c, vec3(2.2));
}

// 4x4 ordered dither threshold at pixel q, in 0..1 (the textures' Bayer matrix, worked out).
float bayer2(vec2 a)
{
    a = floor(a);
    return fract(dot(a, vec2(0.5, a.y * 0.75)));
}

float bayer4(vec2 q)
{
    return bayer2(0.5 * q) * 0.25 + bayer2(q) + 1.0 / 32.0;
}

// Coverage `a` (0..1) at pixel q: all or nothing through the ordered dither up close (`fine` 1),
// a plain blend far away, where single pixels would shimmer.
float dither(float a, vec2 q, float fine)
{
    a = clamp(a, 0.0, 1.0);
    return mix(a, step(bayer4(q), a), fine);
}

// How far single pixels can be drawn: 1 up close, 0 once a screen pixel spans two texels.
float fineAt(float px)
{
    return 1.0 - smoothstep(0.06, 0.13, px);
}

// Ground: damp stains, long cracks with weeds, dust and sand in drifts, moss in patches, dead
// leaves and litter, glints of broken glass, puddles with a wet rim. `edge` (0..1) is where it
// all gathers: a gutter, the foot of a wall. Its broad shapes come from the noise texture; single
// pixels (cracks, leaves, glints) are only worked out up close.
float gOpen = 1.0;  // share of the sky the fragment sees (main)

vec3 groundWear(vec3 col, vec3 p, float px, float edge)
{
    vec3 w0 = uWear[gVariant * 2], w1 = uWear[gVariant * 2 + 1];
    vec2 q = floor(p.xz * 16.0);
    vec2 c = (q + 0.5) / 16.0;
    float fine = fineAt(px);
    float h = stack01(p.y);
    float open = gOpen;
    vec4 n1 = noise4(c * 0.11 + 3.0);
    vec4 n2 = noise4(c * 0.19 + 60.0);
    vec4 n3 = noise4(c * 0.17 + 40.0);

    // Damp stains: the ground darkened in broad blotches.
    float st = dot(noise4(c * 0.21 + 20.0).rgb, vec3(0.5, 0.3, 0.2));
    col *= 1.0 - 0.3 * dither(smoothstep(0.58, 0.64, st), q, fine);

    // Dust and sand: drifts in the depths, thinner up high, piled where things gather.
    float dust = w0.z * mix(1.1, 0.45, h) + 0.6 * edge * w0.z;
    float dn = 0.8 * dot(n1.rgb, vec3(0.5, 0.3, 0.2)) + 0.2 * n1.a;
    float dc = dither(smoothstep(0.0, 0.1, dn - 1.0 + 0.5 * dust), q, fine);
    col = mix(col, swatch(0, 0.15 + 0.7 * n2.a), dc * 0.85);

    // Moss: where light and damp meet, more toward the canopy.
    float moss = w0.x * mix(0.6, 1.3, h) * mix(0.6, 1.0, open) + 0.5 * edge * w0.x;
    float mn = 0.75 * dot(n3.rgb, vec3(0.5, 0.3, 0.2)) + 0.25 * n3.a;
    float mc = dither(smoothstep(0.0, 0.1, mn - 1.0 + 0.55 * moss), q, fine);
    col = mix(col, swatch(2, 0.1 + 0.7 * n1.b), mc);

    // Puddles: rain where the sky is open, leaks under the decks; a darker wet rim.
    float wet = w1.y * mix(0.55, 1.0, open);
    float pn = dot(n2.rgb, vec3(0.5, 0.3, 0.2)) - 0.12 * edge;
    float thr = 0.17 + 0.2 * wet;
    float pool = dither(smoothstep(0.0, 0.025, thr - pn), q, fine);
    float rim = max(dither(smoothstep(0.0, 0.025, thr + 0.03 - pn), q, fine) - pool, 0.0);
    col *= 1.0 - 0.45 * rim;
    col = mix(col, swatch(3, 0.15 + 0.3 * n3.g), pool);
    gWet = pool;

    if (fine > 0.0) {
        // Long cracks, weeds in some of them.
        float cn = fbm(c * 0.33 + 70.0);
        float crack = step(abs(cn - 0.5), 0.009) * step(0.52, noise4(c * 0.08 + 5.0).r) * fine;
        float weed = crack * step(hash12(q + 0.31), 0.25 + 0.6 * w0.x * open);
        col = mix(col, col * 0.35, crack);
        col = mix(col, swatch(2, 0.3 + 0.6 * hash12(q + 1.7)), weed);

        // Dead leaves and litter in clusters, a leaf two pixels long; glints of broken glass.
        float lit = w1.x * mix(0.6, 1.4, h);
        vec2 lq = floor(q / vec2(2.0, 1.0));
        float ln = smoothstep(0.45, 0.9, noise4(c * 0.45 + 13.0).r) * 0.035 + edge * 0.3;
        float leaf = step(hash12(lq + 0.71), lit * ln) * fine * (1.0 - pool);
        col = mix(col, swatch(1, 0.8 * hash12(lq + 3.7)), leaf);
        float glint = step(hash12(q + 9.1), 0.0015 * w0.z * (1.0 - h)) * fine * (1.0 - pool);
        col = mix(col, vec3(0.55, 0.62, 0.66), glint);
    }
    return col;
}

// Walls: a band of grime at the foot (splashed dirt and damp, moss where it is wet), rain
// streaks and the odd rust streak running down from every story's ledge, soot under the deck
// above. `u` runs along the face in meters; `tall` is the box's height (grime stays at the foot
// of low things, and only tall walls streak).
vec3 wallWear(vec3 col, vec3 p, vec3 n, float u, float tall, float px)
{
    vec3 w0 = uWear[gVariant * 2];
    vec2 q = floor(vec2(u, p.y) * 16.0);
    float fine = fineAt(px);
    float x = (q.x + 0.5) / 16.0;
    float y = aboveFloor((q.y + 0.5) / 16.0);

    // The foot: a ragged band, darker toward the ground.
    float foot = (0.25 + 0.4 * vnoise(vec2(x * 0.9, 3.0)) + 0.12 * vnoise(vec2(x * 7.0, 1.0)))
               * (0.8 + 0.6 * w0.y);
    foot = min(foot, 0.3 * tall);
    float f = dither(smoothstep(0.0, 0.06, foot - y), q, fine);
    col *= 1.0 - f * mix(0.5, 0.2, clamp(y / max(foot, 0.01), 0.0, 1.0));
    float mf = f * step(1.0 - 0.9 * w0.x, vnoise(vec2(x * 1.7, y * 3.0) + 5.0)) * step(0.3, tall);
    col = mix(col, swatch(2, 0.1 + 0.6 * vnoise(vec2(x, y) * 4.0)), mf);

    // Streaks from each story's ledge (every 4 m), two pixels wide, broken here and there.
    if (tall > 3.0) {
        float cx = floor(q.x / 2.0);
        float story = floor(y / 4.0);
        float below = (story + 1.0) * 4.0 - y;
        vec2 k = vec2(cx, story + floor(p.y / 36.0) * 17.0);
        float r = hash12(k + 0.37);
        float len = 0.3 + 3.2 * hash12(k + 5.1) * (0.6 + 0.6 * w0.y);
        float s = step(r, 0.14 + 0.1 * w0.y) * (1.0 - smoothstep(0.0, len, below));
        s *= step(0.2, hash12(q * vec2(1.0, 0.25) + 2.3)) * mix(0.6, 1.0, fine);
        col *= 1.0 - 0.45 * s;
        col = mix(col, swatch(4, 0.4 + 0.3 * hash12(k)), s * step(r, 0.025) * 0.7);
    }

    // Soot and damp under the deck above.
    int fl = floorAt(p.y);
    if (fl + 1 < uFloorCount) {
        float above = float((coverBits(p.xz + n.xz * 1.5) >> fl) & 1);
        float top = uFloorY[fl + 1] - uFloorY[fl];
        col *= 1.0 - 0.4 * above * dither(smoothstep(top - 9.0, top - 1.0, y), q, fine);
    }
    return col;
}

// Whether the lot holding xz has a deck on the floor whose walking surface is at height y.
float deckAt(vec2 xz, float y)
{
    int f = floorAt(y);
    if (f == 0) return 1.0;
    return float((coverBits(xz) >> (f - 1)) & 1);
}

// Paved street or deck. Roads run along the lot borders where src/world/roads.rs lays them, 3 m
// on each side, in the biome's road surface (pixel art) with lane markings on asphalt, worn
// nearly away; curbstones on the sidewalk side, a gutter where dirt gathers on the road side; the
// rest is sidewalk. All is worked out at the lot corner nearest the point, from the roads leaving
// it: the strip of a road beside the point, and the crossing square, road where two roads or
// more meet (a turn, a T, a crossroads, a road going on). A road with no other at its end stops
// short of it, at a curb. Masks are taken at the texel's center so their edges are pixels too.
vec3 pavement(vec3 base, vec3 p, float px)
{
    vec2 uv = (floor(p.xz * 16.0) + 0.5) / 16.0;
    vec2 corner = round(uv / LOT);
    vec2 off = uv - corner * LOT;  // from the nearest corner
    vec2 ad = abs(off);
    int fl = floorAt(p.y);
    ivec2 c = ivec2(corner);
    ivec2 here = roadsOf(c, fl);
    // The roads leaving the corner: along z toward +z and -z, along x toward +x and -x.
    float zUp = float(here.x), zDown = float(roadsOf(c - ivec2(0, 1), fl).x);
    float xUp = float(here.y), xDown = float(roadsOf(c - ivec2(1, 0), fl).y);
    // The roads beside the point: along z on its side of the corner, along x on its side.
    float sv = off.y >= 0.0 ? zUp : zDown;
    float sh = off.x >= 0.0 ? xUp : xDown;
    float meet = step(1.5, zUp + zDown + xUp + xDown);  // the crossing square is road
    float across = max(zUp, zDown) * max(xUp, xDown);   // roads both ways: markings stop
    float inV = step(ad.x, 3.0), inH = step(ad.y, 3.0);
    float street = inV * inH * meet + inV * (1.0 - inH) * sv + inH * (1.0 - inV) * sh;

    // Slabs poured a few lots at a time differ a little in shade (the deck's color).
    float shade = clamp(dot(base, vec3(0.2126, 0.7152, 0.0722)) / 0.094, 0.6, 1.25);
    vec3 walk = atlasTile(SIDEWALK_TILE, p.xz / 4.0).rgb * pal(8) * shade;
    vec3 road = atlasTile(ROAD_TILE, p.xz / 4.0).rgb * shade;
    vec3 col = mix(walk, road, street);

    // Distance to the curb lines 3 m off the corner's borders: along a road's strip, and on the
    // sides of the crossing square that differ from the strip next to it.
    float d = 1e3;
    if (sv > 0.5) d = min(d, length(vec2(ad.x - 3.0, max(3.0 - ad.y, 0.0))));
    if (sh > 0.5) d = min(d, length(vec2(max(3.0 - ad.x, 0.0), ad.y - 3.0)));
    if (meet != sh) d = min(d, length(vec2(ad.x - 3.0, max(ad.y - 3.0, 0.0))));
    if (meet != sv) d = min(d, length(vec2(max(ad.x - 3.0, 0.0), ad.y - 3.0)));
    // Curbstones a quarter meter wide on the sidewalk, a dark joint at their foot on the road.
    float curb = (1.0 - street) * step(d, 0.25);
    vec3 stone = atlasTile(2.0, p.xz / 4.0).rgb * 1.2 * shade;
    col = mix(col, stone, curb);
    col *= 1.0 - 0.6 * street * step(d, 0.07);
    float gutter = street * (1.0 - smoothstep(0.1, 0.9, d));

    // Lane markings on asphalt: dashed yellow center lines, white edge lines, outside the
    // crossings (on through where a road just goes on), mostly worn away.
    if (uWear[gVariant * 2 + 1].z > 0.5) {
        float keep = step(0.5, 0.75 * vnoise(uv * 0.9 + 4.0) + 0.25 * hash12(floor(uv * 16.0)));
        float ix = sv * max(step(3.4, ad.y), meet * (1.0 - across));
        float iz = sh * max(step(3.4, ad.x), meet * (1.0 - across));
        float dashZ = step(0.5, fract(uv.y / 3.0)) * step(ad.x, 0.07) * ix;
        float dashX = step(0.5, fract(uv.x / 3.0)) * step(ad.y, 0.07) * iz;
        float edgeZ = step(abs(ad.x - 2.7), 0.07) * ix;
        float edgeX = step(abs(ad.y - 2.7), 0.07) * iz;
        col = mix(col, vec3(0.42, 0.3, 0.06), max(dashZ, dashX) * keep * 0.85);
        col = mix(col, vec3(0.4, 0.4, 0.38), max(edgeZ, edgeX) * keep * 0.75);
    }
    return groundWear(col, p, px, gutter);
}

// Roof: tar membrane and gravel, worn like the ground.
vec3 roof(vec3 base, vec3 p, float px)
{
    return groundWear(atlasTile(ROOF_TILE, p.xz / 4.0).rgb, p, px, 0.0);
}

// Underside of decks and bridges: coffered concrete between beams, water stains and salts.
vec3 underside(vec3 base, vec3 p, float px)
{
    float shade = clamp(dot(base, vec3(0.2126, 0.7152, 0.0722)) / 0.094, 0.6, 1.25);
    return atlasTile(UNDERSIDE_TILE, p.xz / 4.0).rgb * 0.7 * shade;
}

// Slabs poured a few lots at a time differ a little in shade: the block's color against the
// usual deck color.
float deckShade(vec3 base)
{
    return clamp(dot(base, vec3(0.2126, 0.7152, 0.0722)) / 0.094, 0.6, 1.25);
}

// Boxes of hand-written chunks (src/world/chunks.rs): the tile of their material (0 stone,
// 1 dark stone, 2 concrete, 3 brick, 4 tile, 5 metal, 6 rust, 7 wood, 8 glass, 9 soil, 10 paint,
// which takes the box color), mapped in world space by the face's plane.
vec3 built(vec3 base, int mat, vec3 p, vec3 n, float tall, float px)
{
    vec3 an = abs(n);
    vec2 uv = an.y > 0.5 ? p.xz : (an.x > an.z ? vec2(p.z, -p.y) : vec2(p.x, -p.y));
    vec3 col = atlasTile(float(mat), uv / 4.0).rgb;
    col = mat == 10 ? col * base : col;
    // Glass (8) stays clean of ground wear; walls and tops weather.
    if (n.y > 0.5 && mat != 8) return groundWear(col, p, px, 0.0);
    if (an.y < 0.5) return wallWear(col, p, n, uv.x, tall, px);
    return col;
}

// Tower walls in pixel art (docs/chunks.md, "Towers"): a grid of bays 4 m wide and stories 4 m
// high, each cell the facade unit its tower chunk's elevation writes there (src/render/facades.rs:
// far tile, tile near a deck, paint), a tile of the atlas (src/render/pixels.rs: window, two
// windows, caged window, enclosed balcony, air conditioner, boarded, broken, vent, shop shutter,
// shop window, blank wall). The rows repeat every floor from its street story up. On the street
// story and the two above it, where a deck meets the wall, the units stand in boxes in front of
// plain windows (src/world/tower.rs). Abandoned: no window is lit, and now and then a far window
// is boarded up or broken. Wall pixels take the tower's color, or the biome's paint the elevation
// gives (src/world/tower.rs, `paint`). loc = (position along the face, face width).
const float FACADE_FIRST = 11.0;

vec3 facade(vec3 base, vec3 p, vec3 n, vec2 loc, float fid, int elev, int shift)
{
    float W = max(loc.y, 1.0);
    float cols = max(floor(W / 4.0 + 1e-3), 1.0);
    float u0 = (W - cols * 4.0) * 0.5;
    // Bays are written from the left seen from outside: right to left along the world axis on
    // the faces toward +x and -z.
    bool rev = n.x > 0.5 || n.z < -0.5;
    vec2 t = vec2((loc.x - u0) / 4.0, -p.y / 4.0);
    float ci = floor(t.x);
    // Piers at the corners, outside the grid of bays.
    if (ci < 0.0 || ci >= cols) return built(base, 2, p, n, 0.0, 1.0);
    if (rev) t.x = -t.x;
    int col = int(rev ? cols - 1.0 - ci : ci);
    int row = clamp(int(floor(aboveFloor(p.y) / 4.0 + 1e-3)), 0, 8);
    ivec2 o = ivec2((elev % 16) * 16, (elev / 16) * 9);
    int bays = max(int(texelFetch(texture6, o, 0).a * 255.0 + 0.5), 1);
    ivec4 e = ivec4(texelFetch(texture6, o + ivec2(min(col, bays - 1), row), 0) * 255.0 + 0.5);
    bool near = row <= 2 && deckAt(p.xz + n.xz * 1.5, p.y) > 0.5;
    float tile = float(near ? e.g : e.r);
    if (!near && tile < 1.5) {
        float r = hash12(vec2(ci + fid * 13.0, floor(p.y / 4.0 + 1e-3) * 1.3 + 7.0));
        tile = r < 0.04 ? 5.0 : r < 0.07 ? 6.0 : tile;
    }
    vec4 c = atlasTile(FACADE_FIRST + tile, t);
    vec3 tint = e.b == 0 ? base : pal((e.b + shift) % 8);
    return mix(c.rgb, c.rgb / 0.44 * tint, c.a);
}

// Leaf and frond painting from leaf coordinates (across with the midrib at 0.5, along from the
// root): lighter midrib, side veins slanting toward the tip, darker root, lighter edge and tip.
vec3 leaf(vec3 base, vec2 lc)
{
    float u = abs(lc.x - 0.5) * 2.0;
    float aa = max(fwidth(u), 1e-3);
    float rib = 1.0 - smoothstep(0.04, 0.04 + aa * 1.5, u);
    float slant = lc.y * 7.0 - u * 1.6;
    float veins = (1.0 - smoothstep(0.03, 0.03 + fwidth(slant) * 1.5, abs(fract(slant) - 0.5)))
                * (1.0 - smoothstep(0.75, 1.0, u)) * step(0.1, lc.y) * step(0.12, u);
    vec3 col = base * mix(0.62, 1.15, smoothstep(0.0, 0.8, lc.y)) * (1.0 + 0.18 * u * u);
    col = mix(col, base * 1.55, rib * 0.7);
    col = mix(col, base * 1.3, veins * 0.35);
    return col;
}

// Painted foliage: soft blotches of two greens, finer brush flecks.
float paintPlant(vec3 p)
{
    float m = 0.6 * vnoise(p.xz * 2.2 + p.y * 1.7) + 0.4 * vnoise(p.xy * 7.0 + p.z * 5.0);
    return 0.78 + 0.44 * m;
}

void main()
{
    vec3 n = normalize(fragNormal);
    vec3 albedo = pow(fragColor.rgb, vec3(2.2));
    bool textured = (uEffects & FX_TEXTURE) != 0;
    // Flowers (emissive) stay clean and graphic.
    bool painted = textured && fragColor.a < 0.01;
    int kindCode = int(fragKind + 0.5);
    int kind = kindCode % 16;
    gVariant = (kindCode / 16) % 16;
    // Tower walls: the tower's paint shift and the face's elevation (src/render/mesh.rs).
    int paintShift = (kindCode / 256) % 8;
    int elevation = kindCode / 2048;
    vec3 emit = vec3(0.0);
    float open = skyView(fragWorldPos + n * 0.1);
    gOpen = open;
    if (painted) {
        float px = max(length(fwidth(fragWorldPos)), 1e-4);
        float fid = fragId * 255.0;
        bool vertical = abs(n.y) < 0.5;
        if (kind == 1) {
            albedo *= paintPlant(fragWorldPos) * mix(1.0, grit2(fragWorldPos.xz * 1.3 + fragWorldPos.y * 0.7).g, 0.6);
            if (fragLoc.x >= 0.0) albedo = leaf(albedo, fragLoc);
        } else if (kind == 0) {
            if (vertical)
                albedo = wallWear(facade(albedo, fragWorldPos, n, fragLoc, fid, elevation, paintShift),
                                  fragWorldPos, n, fragLoc.x, 99.0, px);
            else if (n.y > 0.0) albedo = roof(albedo, fragWorldPos, px);
            else albedo = underside(albedo, fragWorldPos, px);
        } else if (kind == 2) {
            if (n.y > 0.5) albedo = pavement(albedo, fragWorldPos, px);
            else if (n.y < -0.5) albedo = underside(albedo, fragWorldPos, px);
            // The deck's edge: its concrete fascia, stained like a wall.
            else albedo = built(albedo, 2, fragWorldPos, n, 1.2, px) * deckShade(albedo);
        } else if (kind == 4) {
            // Stair steps and landings: cast concrete, worn on the treads.
            albedo = built(albedo, 2, fragWorldPos, n, 0.0, px) * deckShade(albedo);
        } else if (kind == 6) {
            // The wall between a stair's lanes.
            albedo = built(albedo, 2, fragWorldPos, n, fragLoc.y, px);
        } else if (kind == 14) {
            // Props built in code (the street light): painted metal in pixel art, mapped by the
            // face's plane like the boxes, darker at the foot.
            albedo = built(albedo, 10, fragWorldPos, n, 0.0, px) * 1.35;
            albedo *= mix(0.7, 1.0, smoothstep(0.0, 0.8, aboveFloor(fragWorldPos.y)));
        } else if (kind == 13) {
            // Steel (stair rails, brackets): painted metal in pixel art, rust where it weathers.
            albedo = built(albedo, 10, fragWorldPos, n, 0.0, px) * 1.35;
        } else if (kind == 15) {
            albedo = built(albedo, int(fragLoc.x + 0.5), fragWorldPos, n, fragLoc.y, px);
        }
    }

    float ndl = dot(n, uLightDir);
    float sh = ndl > 0.0 ? shadow(fragWorldPos, n) * skyVis(fragWorldPos + n * 0.1, uLightDir) : 0.0;
    float direct;
    if ((uEffects & FX_CEL) != 0) {
        // Three flat bands: shadow, half-lit, lit. Cast shadows snap to the shadow band.
        sh = smoothstep(0.3, 0.7, sh);
        direct = (0.45 * band(ndl, 0.05) + 0.55 * band(ndl, 0.5)) * sh;
    } else {
        direct = max(ndl, 0.0) * sh;
    }

    // Hemisphere ambient: sky from above, bounced ground light from below. Under the decks of
    // the floors above, little of the sky gets in.
    vec3 hemi = mix(uAmbientGround, uAmbientSky, n.y * 0.5 + 0.5);
    vec3 deep = heightHue(fragWorldPos.y) * dot(hemi, vec3(0.2126, 0.7152, 0.0722)) * 0.2;
    vec3 ambient = mix(deep, hemi, open);

    // Painted vertical gradient: darker at street level, slightly lifted up high.
    float y = aboveFloor(fragWorldPos.y);
    float grad = mix(0.55, 1.0, smoothstep(0.0, 10.0, y)) * (1.0 + 0.15 * smoothstep(10.0, 30.0, y));

    // Plants glow regardless of the hour where no sky reaches.
    float glow = mix(uGlow, max(uGlow, 0.85), 1.0 - open);
    vec3 flowers = plantLights(fragWorldPos, n);
    vec3 lit = albedo * (ambient * grad + uLight * direct + flowers * glow);
    lit += albedo * fragColor.a * glow * 2.5;
    // Standing water mirrors the sky, the flowers' light and the sun, more so at grazing angles.
    if (gWet > 0.0) {
        vec3 v = normalize(fragWorldPos - uCamPos);
        float fres = 0.1 + 0.9 * pow(1.0 - clamp(-dot(v, n), 0.0, 1.0), 5.0);
        vec3 r = reflect(v, n);
        vec3 mirror = mix(uAmbientGround, uAmbientSky, 0.8) * mix(0.15, 1.0, open) * 1.3
                    + flowers * glow * 0.7
                    + uLight * pow(max(dot(r, uLightDir), 0.0), 150.0) * sh * 3.0;
        lit = mix(lit, mirror, fres * gWet);
    }
    lit += emit * glow * 0.32;
    outColor = vec4(lit, 1.0);
    outNormal = vec4(n * 0.5 + 0.5, fragId);
}
