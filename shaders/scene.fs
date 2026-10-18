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
uniform sampler2D texture3;  // photo detail: R pavement, G asphalt, B concrete wall, A metal (src/render/surfaces.rs)
uniform vec3 uGridOrigin;
uniform vec3 uPalette[8 * 9];    // per biome (src/world/biome.rs, Palette): 8 wall paints, 1 pavement tint
uniform float uGlow;         // plant emission and light strength for the time of day
uniform sampler2D texture5;  // cover map
uniform sampler2D texture10; // chunk materials and facade units in pixel art, a row per biome (src/render/pixels.rs)
uniform sampler2D texture6;  // tower elevations (src/render/facades.rs)
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
            float x = 1.0 - dist / a.w;
            if (x <= 0.0) continue;
            float q = 0.34 * smoothstep(0.03, 0.13, x) + 0.33 * smoothstep(0.30, 0.40, x)
                    + 0.33 * smoothstep(0.65, 0.75, x);
            float facing = clamp(dot(n, d / max(dist, 1e-3)) * 0.5 + 0.6, 0.0, 1.0);
            acc += fetchTexel(i * 3 + 1).rgb * q * facing;
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

// Fine detail from photographs, centered on 1 (R pavement, G asphalt, B concrete wall, A metal).
vec4 grit(vec2 uv)
{
    return texture(texture3, uv) * 2.0;
}

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

// Whether the lot holding xz has a deck on the floor whose walking surface is at height y.
float deckAt(vec2 xz, float y)
{
    int f = floorAt(y);
    if (f == 0) return 1.0;
    return float((coverBits(xz) >> (f - 1)) & 1);
}

// Paved street or deck. Roads run along the lot borders where src/world/roads.rs lays them, 3 m
// on each side: asphalt with lane markings between curbs; the rest is sidewalk, concrete slabs
// with joints. Cracks, dust and oil stains over both; the photo detail gives the grain. All is
// worked out at the lot corner nearest the point, from the roads leaving it: the strip of a road
// beside the point, and the crossing square, road where two roads or more meet (a turn, a T, a
// crossroads, a road going on). A road with no other at its end stops short of it, at a curb.
vec3 pavement(vec3 base, vec3 p, float px)
{
    vec2 uv = p.xz;
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

    // Sidewalk: 4 m slabs with joints and a tone per slab.
    vec2 g = uv / 4.0;
    vec2 sid = floor(g);
    vec2 f = fract(g);
    float dj = min(min(f.x, 1.0 - f.x), min(f.y, 1.0 - f.y)) * 4.0;
    float joint = (1.0 - smoothstep(0.02, 0.05 + px, dj)) * (1.0 - street);
    vec3 walk = base * pal(8) * (0.82 + 0.24 * hash12(sid)) * (0.8 + 0.4 * fbm(uv * 0.07));
    walk *= mix(1.0, grit(uv / 2.4).r, 0.85) * mix(1.0, grit2(uv / 5.7).a, 0.4);
    // Road: dark asphalt.
    vec3 road = base * 0.5 * (0.85 + 0.3 * fbm(uv * 0.12)) * mix(1.0, grit(uv / 1.7).g, 0.9);
    vec3 col = mix(walk, road, street);

    // Curbs where road meets sidewalk, all on the lines 3 m off the corner's borders: along a
    // road's strip, and on the sides of the crossing square that differ from the strip next to it.
    float d = 1e3;
    if (sv > 0.5) d = min(d, length(vec2(ad.x - 3.0, max(3.0 - ad.y, 0.0))));
    if (sh > 0.5) d = min(d, length(vec2(max(3.0 - ad.x, 0.0), ad.y - 3.0)));
    if (meet != sh) d = min(d, length(vec2(ad.x - 3.0, max(ad.y - 3.0, 0.0))));
    if (meet != sv) d = min(d, length(vec2(max(ad.x - 3.0, 0.0), ad.y - 3.0)));
    float curb = 1.0 - smoothstep(0.1, 0.1 + px, d);
    col = mix(col, base * 1.15, curb * 0.9);

    // Cracks: thin contour lines of a noise field, only in patches.
    float cn = fbm(uv * 0.9 + sid * 1.7);
    float crack = (1.0 - smoothstep(0.0, 0.012 + px * 0.8, abs(cn - 0.5) * 1.6)) * smoothstep(0.62, 0.8, vnoise(uv * 0.14 + 11.0));
    col *= 1.0 - 0.5 * crack;
    // Oil and wet stains, then dust settled in blotches.
    col *= 1.0 - 0.5 * smoothstep(0.6, 0.7, fbm(uv * 0.3 + 20.0));
    float dust = smoothstep(0.52, 0.8, fbm(uv * 0.45 + 3.0));
    col = mix(col, col * vec3(1.35, 1.2, 0.95) + 0.015, dust * 0.5);
    col *= 1.0 - 0.55 * joint;

    // Lane markings: dashed yellow center lines, white edge lines, outside the crossings (on
    // through where a road just goes on).
    float wear = (0.45 + 0.55 * vnoise(uv * 3.5)) * grit(uv / 0.9).g;
    float ix = sv * max(step(3.4, ad.y), meet * (1.0 - across));
    float iz = sh * max(step(3.4, ad.x), meet * (1.0 - across));
    float dashZ = step(0.5, fract(uv.y / 3.0)) * (1.0 - smoothstep(0.07, 0.07 + px, ad.x)) * ix;
    float dashX = step(0.5, fract(uv.x / 3.0)) * (1.0 - smoothstep(0.07, 0.07 + px, ad.y)) * iz;
    float edgeZ = (1.0 - smoothstep(0.06, 0.06 + px, abs(ad.x - 2.7))) * ix;
    float edgeX = (1.0 - smoothstep(0.06, 0.06 + px, abs(ad.y - 2.7))) * iz;
    float yellow = max(dashZ, dashX);
    float white = max(edgeZ, edgeX);
    col = mix(col, vec3(0.7, 0.5, 0.06) * wear, yellow * 0.9);
    col = mix(col, vec3(0.62, 0.62, 0.6) * wear, white * 0.85);
    return col;
}

// Roof: dark tar with gravel, lighter patches (skylights, panels).
vec3 roof(vec3 base, vec3 p, float px)
{
    vec2 uv = p.xz;
    float far = 1.0 - smoothstep(0.05, 0.4, px);
    vec3 col = base * 0.5 * (0.8 + 0.4 * fbm(uv * 0.2));
    col *= grit2(uv / 2.0).r;
    vec2 cell = floor(uv / 5.0);
    vec2 fc = fract(uv / 5.0) - 0.5;
    float patch = step(0.78, hash12(cell)) * boxMask(fc * 5.0, vec2(1.6, 1.1), px);
    col = mix(col, base * 1.1, patch * 0.8);
    return col * (1.0 - 0.3 * smoothstep(0.55, 0.7, fbm(uv * 0.35 + 8.0)));
}

// Underside of decks and bridges: dark concrete with beams and grime.
vec3 underside(vec3 base, vec3 p, float px)
{
    vec2 uv = p.xz;
    float beam = 1.0 - smoothstep(0.3, 0.3 + px, abs(fract(uv.x / 5.0) - 0.5) * 5.0 - 2.1);
    vec3 col = base * 0.45 * (0.75 + 0.5 * fbm(uv * 0.4));
    col *= 1.0 - 0.3 * beam;
    col *= grit(uv / 2.0).b;
    return col * (1.0 - 0.4 * smoothstep(0.55, 0.8, fbm(uv * 0.8 + 4.0)));
}

// Concrete fascia and generic vertical faces: soft blotches and grime streaks.
vec3 concrete(vec3 base, vec3 p, vec3 n, vec2 loc)
{
    float u = abs(n.y) < 0.5 ? loc.x : p.x + p.z;
    vec3 col = base * (0.8 + 0.4 * fbm(vec2(u * 0.7, p.y * 0.5)));
    col *= grit(vec2(u, p.y) / 2.5).b;
    float streak = vnoise(vec2(u * 3.0, p.y * 0.12));
    return col * (1.0 - 0.3 * smoothstep(0.55, 0.9, streak));
}

// Steel (fire escapes, brackets): painted metal with rust streaks; landings show a grating.
vec3 steel(vec3 base, vec3 p, vec3 n, float px)
{
    vec2 uv = abs(n.y) > 0.5 ? p.xz : vec2(p.x + p.z, p.y);
    vec3 col = base * (0.8 + 0.4 * fbm(uv * 1.7)) * grit(uv / 0.8).a;
    float rust = smoothstep(0.55, 0.8, vnoise(vec2(uv.x * 3.0, uv.y * 0.6) + 7.0)) * grit2(uv / 0.9).b;
    col = mix(col, vec3(0.28, 0.12, 0.05), clamp(rust * 0.5, 0.0, 0.7));
    if (n.y > 0.5) {
        vec2 d = abs(fract(uv / 0.12) - 0.5);
        float bars = 1.0 - smoothstep(0.38, 0.38 + px / 0.12, max(d.x, d.y));
        col *= 1.0 - 0.45 * bars;
    }
    return col;
}

// The atlas of pixel-art tiles (src/render/pixels.rs): a row per biome, each the chunk materials
// then the facade units, 4 m per tile. Drawn from the row of the fragment's biome; `t` is in
// tiles; crisp pixels up close.
const vec2 ATLAS = vec2(22.0, 8.0);  // tiles in a row (pixels::COUNT), rows (MAX_BIOMES)
vec4 atlasTile(float tile, vec2 t)
{
    vec2 a = (vec2(tile, float(gVariant)) + fract(t)) / ATLAS;
    // Gradients of the unwrapped coordinates, so the tile seams never pick a blurry mip level.
    vec2 dx = dFdx(t) / ATLAS;
    vec2 dy = dFdy(t) / ATLAS;
    vec4 c = textureGrad(texture10, a, dx, dy);
    return vec4(pow(c.rgb, vec3(2.2)), c.a);
}

// Boxes of hand-written chunks (src/world/chunks.rs): the tile of their material (0 stone,
// 1 dark stone, 2 concrete, 3 brick, 4 tile, 5 metal, 6 rust, 7 wood, 8 glass, 9 soil, 10 paint,
// which takes the box color), mapped in world space by the face's plane.
vec3 built(vec3 base, int mat, vec3 p, vec3 n)
{
    vec3 an = abs(n);
    vec2 uv = an.y > 0.5 ? p.xz : (an.x > an.z ? vec2(p.z, -p.y) : vec2(p.x, -p.y));
    vec3 col = atlasTile(float(mat), uv / 4.0).rgb;
    return mat == 10 ? col * base : col;
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
    if (ci < 0.0 || ci >= cols) return built(base, 2, p, n);
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
    if (painted) {
        float px = max(length(fwidth(fragWorldPos)), 1e-4);
        float fid = fragId * 255.0;
        bool vertical = abs(n.y) < 0.5;
        if (kind == 1) {
            albedo *= paintPlant(fragWorldPos) * mix(1.0, grit2(fragWorldPos.xz * 1.3 + fragWorldPos.y * 0.7).g, 0.6);
            if (fragLoc.x >= 0.0) albedo = leaf(albedo, fragLoc);
        } else if (kind == 0) {
            if (vertical) albedo = facade(albedo, fragWorldPos, n, fragLoc, fid, elevation, paintShift);
            else if (n.y > 0.0) albedo = roof(albedo, fragWorldPos, px);
            else albedo = underside(albedo, fragWorldPos, px);
        } else if (kind == 2) {
            if (n.y > 0.5) albedo = pavement(albedo, fragWorldPos, px);
            else if (n.y < -0.5) albedo = underside(albedo, fragWorldPos, px);
            else albedo = concrete(albedo, fragWorldPos, n, fragLoc);
        } else if (kind == 4) {
            // Stone steps.
            vec2 suv = abs(n.y) > 0.5 ? fragWorldPos.xz : vec2(fragWorldPos.x + fragWorldPos.z, fragWorldPos.y);
            albedo *= (0.8 + 0.4 * fbm(suv * 1.5)) * grit(suv / 1.4).r;
        } else if (kind == 6) {
            albedo = concrete(albedo, fragWorldPos, n, fragLoc);
        } else if (kind == 14) {
            // Props built in code (the street light): worn by the same photo detail as the walls
            // (same scale), darker at the foot.
            vec3 an = abs(n);
            vec2 puv = an.y > 0.6 ? fragWorldPos.xz : (an.x > an.z ? fragWorldPos.zy : fragWorldPos.xy);
            albedo *= mix(1.0, grit(puv / 3.0).a, 0.45) * (0.85 + 0.3 * fbm(puv * 0.8));
            albedo *= mix(0.7, 1.0, smoothstep(0.0, 0.8, aboveFloor(fragWorldPos.y)));
        } else if (kind == 13) {
            albedo = steel(albedo, fragWorldPos, n, px);
        } else if (kind == 15) {
            albedo = built(albedo, int(fragLoc.x + 0.5), fragWorldPos, n);
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
    float open = skyView(fragWorldPos + n * 0.1);
    vec3 ambient = mix(uAmbientGround, uAmbientSky, n.y * 0.5 + 0.5) * mix(0.2, 1.0, open);

    // Painted vertical gradient: darker at street level, slightly lifted up high.
    float y = aboveFloor(fragWorldPos.y);
    float grad = mix(0.55, 1.0, smoothstep(0.0, 10.0, y)) * (1.0 + 0.15 * smoothstep(10.0, 30.0, y));

    // Plants glow regardless of the hour where no sky reaches.
    float glow = mix(uGlow, max(uGlow, 0.85), 1.0 - open);
    vec3 lit = albedo * (ambient * grad + uLight * direct + plantLights(fragWorldPos, n) * glow);
    lit += albedo * fragColor.a * glow * 2.5;
    lit += emit * glow * 0.32;
    outColor = vec4(lit, 1.0);
    outNormal = vec4(n * 0.5 + 0.5, fragId);
}
