// Cover map: which decks stand over each lot around the camera (src/render/cover.rs). Included by
// scene.fs and post.fs (`#include`, src/render/shader.rs), each first defining COVER_TEX as the
// sampler holding the map.
// Tile 0 red and green: bit (f - 1) set for a deck on floor f; tiles 1..: sky view per floor;
// the last tile: the roads (src/world/roads.rs).
// Constants mirrored there and in src/world/floors.rs (MAX floors; the run uses uFloorCount).
const int COVER = 64;
const float LOT = 24.0;
const int FLOORS = 16;
const int ROADS_TILE = 1 + FLOORS / 4;
const int TILES = ROADS_TILE + 1;
uniform int uFloorCount;
uniform float uFloorY[FLOORS];
uniform vec2 uCoverOrigin;   // lot coordinates of texel (0, 0)

int coverBits(vec2 xz)
{
    ivec2 c = ivec2(floor(xz / LOT - uCoverOrigin));
    // Beyond the map, assume the city is covered.
    if (any(lessThan(c, ivec2(0))) || any(greaterThanEqual(c, ivec2(COVER)))) return 0xffff;
    ivec2 rg = ivec2(texelFetch(COVER_TEX, c, 0).rg * 255.0 + 0.5);
    return rg.x | (rg.y << 8);
}

// The floor whose walking surface is at height y or just under it (0 the ground).
int floorAt(float y)
{
    int f = 0;
    for (int k = 1; k < FLOORS; k++) if (k < uFloorCount && uFloorY[k] <= y + 0.5) f = k;
    return f;
}

// The roads of a lot on floor f (src/world/roads.rs, `bits`): x along its -x border, y along its
// -z border.
ivec2 roadsOf(ivec2 lot, int f)
{
    ivec2 c = lot - ivec2(uCoverOrigin);
    // Beyond the map, a road along every border.
    if (any(lessThan(c, ivec2(0))) || any(greaterThanEqual(c, ivec2(COVER)))) return ivec2(1);
    int bits = int(texelFetch(COVER_TEX, ivec2(ROADS_TILE * COVER + c.x, c.y), 0)[f / 4] * 255.0 + 0.5);
    bits >>= 2 * (f % 4);
    return ivec2(bits & 1, (bits >> 1) & 1);
}

// 1 if a ray from p along dir clears every deck above p, else 0.
float skyVis(vec3 p, vec3 dir)
{
    if (dir.y < 0.02) return 1.0;
    for (int f = 1; f < FLOORS; f++) {
        if (f >= uFloorCount) break;
        float y = uFloorY[f] - 0.6;
        if (y < p.y + 0.2) continue;
        vec3 q = p + dir * ((y - p.y) / dir.y);
        if (((coverBits(q.xz) >> (f - 1)) & 1) != 0) return 0.0;
    }
    return 1.0;
}

// Share of the sky p sees (0 = none, 1 = open sky), precomputed per lot and floor and blended
// between lots.
float skyView(vec3 p)
{
    int f = floorAt(p.y);
    vec2 c = clamp(p.xz / LOT - uCoverOrigin, vec2(0.5), vec2(float(COVER) - 0.5));
    // Beyond the map, assume the city is covered.
    vec2 d = p.xz / LOT - uCoverOrigin;
    if (any(lessThan(d, vec2(0.0))) || any(greaterThan(d, vec2(float(COVER))))) return 0.0;
    vec2 uv = (vec2(float((1 + f / 4) * COVER), 0.0) + c) / vec2(float(TILES * COVER), float(COVER));
    return texture(COVER_TEX, uv)[f % 4];
}

// The light under the decks (src/render/look.rs, Sky::heights): the hue the haze and the light
// take where the sky does not reach, by height in the stack (0 the ground floor, 1 the top deck),
// cold in the depths, warm in the middle, gold under the canopy; luminance 1.
uniform vec3 uHeights[3];

float stack01(float y)
{
    return clamp((y - uFloorY[0]) / max(uFloorY[max(uFloorCount - 1, 1)] - uFloorY[0], 1.0), 0.0, 1.0);
}

vec3 heightHue(float y)
{
    float h = stack01(y);
    return mix(mix(uHeights[0], uHeights[1], smoothstep(0.15, 0.45, h)), uHeights[2], smoothstep(0.55, 0.8, h));
}
