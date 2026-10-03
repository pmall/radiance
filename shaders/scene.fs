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
uniform sampler2D texture3;  // photo detail: R pavement, G asphalt, B concrete wall, A metal (src/render/surfaces.rs)
uniform vec3 uGridOrigin;
uniform float uGlow;         // plant emission and light strength for the time of day

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

float hash11(float p)
{
    p = fract(p * 0.1031);
    p *= p + 33.33;
    p *= p + p;
    return fract(p);
}

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

// 1 inside a box of half-size `h` centered on the origin, antialiased over `aa` meters.
float boxMask(vec2 q, vec2 h, float aa)
{
    vec2 d = abs(q) - h;
    return 1.0 - smoothstep(-aa, aa, max(d.x, d.y));
}

vec3 neonColor(float r)
{
    return r < 0.4 ? vec3(0.1, 0.9, 1.0) : (r < 0.75 ? vec3(1.0, 0.2, 0.75) : vec3(1.0, 0.65, 0.2));
}

vec3 windowLight(float r)
{
    return r < 0.55 ? vec3(1.0, 0.68, 0.34) : (r < 0.85 ? vec3(0.35, 0.85, 1.0) : vec3(1.0, 0.35, 0.8));
}

// Tower walls: stories of windows between piers, storefronts with neon on the street levels,
// slab bands, rain streaks, an occasional vertical neon sign. `base` is the wall color (linear).
// `emit` receives light the surface gives off. loc = (position along the face, face width).
vec3 facade(vec3 base, vec3 p, vec2 loc, float fid, float px, out vec3 emit)
{
    const float H = 4.0;
    float W = max(loc.y, 1.0);
    // 0 windows between piers, 1 ribbon, 2 slits, 3 punched grid, 4 glass curtain wall.
    int style = int(hash11(fid * 1.7) * 5.0);
    float pitch = style == 2 ? 2.0 : style == 1 ? 4.0 : style == 3 ? 2.0 : style == 4 ? 1.8
                : mix(2.6, 3.4, hash11(fid * 3.1));
    float cols = max(floor((W - 2.4) / pitch), 1.0);
    float u0 = (W - cols * pitch) * 0.5;
    float cu = (loc.x - u0) / pitch;
    float inCols = step(0.0, cu) * step(cu, cols);
    float cell = floor(clamp(cu, 0.0, cols - 0.001));
    float fu = clamp(cu, 0.0, cols) - cell;
    float yy = p.y / H;
    float story = floor(yy);
    float vy = yy - story;

    // Street levels (decks at y = -80, -36 and 0) get storefronts.
    float shop = max(max(step(-80.0, p.y) * step(p.y, -76.0), step(-36.0, p.y) * step(p.y, -32.0)),
                     step(0.0, p.y) * step(p.y, 4.0));
    vec2 q = vec2((fu - 0.5) * pitch, (vy - mix(0.52, 0.42, shop)) * H);
    vec2 halfw = style == 0 ? vec2(0.31 * pitch, 0.95)
               : style == 1 ? vec2(pitch * 0.5 - 0.15, 0.7)
               : style == 2 ? vec2(0.35, 1.45)
               : style == 3 ? vec2(0.42, 0.62) : vec2(pitch * 0.5 - 0.05, 1.7);
    halfw = mix(halfw, vec2(pitch * 0.5 - 0.3, 1.45), shop);

    // A big colored cladding panel on some buildings, in the style of painted accent blocks.
    float hasP = step(0.55, hash11(fid * 8.1));
    float pw = W * mix(0.35, 0.6, hash11(fid * 8.7));
    float pu = (W - pw) * hash11(fid * 9.1);
    float py0 = floor(mix(-60.0, 6.0, hash11(fid * 10.3)) / H) * H;
    float ph = H * floor(mix(2.0, 5.0, hash11(fid * 11.7)));
    float inP = hasP * step(pu, loc.x) * step(loc.x, pu + pw) * step(py0, p.y) * step(p.y, py0 + ph) * (1.0 - shop);
    float pr = hash11(fid * 12.9);
    vec3 pcol = pr < 0.4 ? vec3(0.75, 0.22, 0.03) : pr < 0.7 ? vec3(0.85, 0.6, 0.04)
              : pr < 0.9 ? vec3(0.04, 0.4, 0.45) : vec3(0.8, 0.8, 0.78);

    float fr = style == 4 ? 0.05 : 0.14;
    float win = boxMask(q, halfw, px) * inCols * (1.0 - inP);
    float frame = (boxMask(q, halfw + fr, px) - boxMask(q, halfw, px)) * inCols * (1.0 - inP);
    float sill = boxMask(q - vec2(0.0, -halfw.y - 0.1), vec2(halfw.x + 0.12, 0.07), px) * inCols * (1.0 - inP);

    // Wall: concrete with soft blotches, darker slab band at the foot of every story.
    float blotch = 0.88 + 0.24 * fbm(vec2(loc.x * 0.6, p.y * 0.25));
    vec3 col = base * blotch * mix(1.0, grit(vec2(loc.x, p.y) / 4.5).b, 0.55);
    col *= 1.0 - 0.14 * (1.0 - smoothstep(0.0, 0.5, vy * H));
    // Rain streaks running down from the sills, heavier lower down.
    float streak = vnoise(vec2(loc.x * 6.0 + floor(cu) * 3.0, p.y * 0.1));
    float low = 1.0 - smoothstep(-80.0, 10.0, p.y);
    col *= 1.0 - 0.3 * smoothstep(0.55, 0.9, streak) * (0.3 + 0.7 * low);

    // Glass: dark, a little lighter toward the top of each pane; some panes are lit.
    float r = hash12(vec2(cell + fid * 13.0, story));
    float litP = style == 4 ? 0.04 : mix(0.07, 0.4, shop);
    float lit = step(1.0 - litP, r);
    float shade = clamp(q.y / halfw.y * 0.5 + 0.5, 0.0, 1.0);
    vec3 glass = mix(vec3(0.012, 0.02, 0.028), vec3(0.05, 0.1, 0.12), shade);
    if (style == 4) glass = mix(vec3(0.02, 0.05, 0.08), vec3(0.09, 0.22, 0.3), shade * shade);
    vec3 wl = windowLight(hash12(vec2(story, cell + fid * 7.0) + 9.0)) * (0.55 + 0.45 * hash12(vec2(r, cell)));
    // Lit panes: blinds (horizontal slats) and a center mullion keep them from reading as flat patches.
    float slats = 0.55 + 0.45 * step(0.5, fract(q.y * 4.0 + hash12(vec2(cell, story)) * 3.0));
    float mullion = 1.0 - smoothstep(0.03, 0.03 + px, abs(q.x));
    emit = wl * lit * win * slats * (1.0 - 0.8 * mullion);
    // Cladding seams every 1.5 m on the accent panel.
    vec2 ps = abs(fract(vec2(loc.x, p.y) / 1.5) - 0.5);
    float pseam = 1.0 - smoothstep(0.015, 0.015 + px / 1.5, 0.5 - max(ps.x, ps.y));
    col = mix(col, pcol * (1.0 - 0.25 * pseam) * (0.9 + 0.2 * fbm(vec2(loc.x, p.y) * 0.8)), inP);
    col = mix(col, glass, win);
    col = mix(col, base * 0.35, frame);
    col = mix(col, base * 1.25, sill);

    // Window air conditioners under some sills, and a projecting cornice band every 4 stories.
    float acHas = step(0.9, hash12(vec2(cell + fid * 3.0, story + 40.0))) * inCols * (1.0 - shop) * (1.0 - inP) * step(float(style), 3.5);
    vec2 aq = q - vec2(0.0, -halfw.y - 0.42);
    float ac = boxMask(aq, vec2(0.42, 0.26), px) * acHas;
    float grille = 0.55 + 0.45 * step(0.5, fract(aq.y * 9.0));
    col = mix(col, vec3(0.55, 0.57, 0.6) * grille, ac);
    float band = (1.0 - smoothstep(0.0, 0.3 + px, abs(vy * H - 0.0))) * step(mod(story, 4.0), 0.5);
    col = mix(col, base * 1.2, band * (1.0 - shop));
    col *= 1.0 - 0.25 * (1.0 - smoothstep(0.0, 0.5, (vy * H - 0.3))) * step(mod(story, 4.0), 0.5) * step(0.3, vy * H) ;

    // Neon lintel above storefronts.
    float lintel = boxMask(vec2(q.x, q.y - halfw.y - 0.5), vec2(halfw.x, 0.07), px) * inCols * shop;
    vec3 lc = neonColor(hash12(vec2(cell, fid * 5.0 + story)));
    emit += lc * lintel * 1.4;
    col = mix(col, lc * 0.4, lintel);

    // A vertical neon sign on some buildings.
    float has = step(0.72, hash11(fid * 5.3));
    float su = W * (0.2 + 0.6 * hash11(fid * 9.7));
    float y0 = floor(mix(-70.0, 8.0, hash11(fid * 2.3)) / H) * H;
    float sy = (p.y - y0) / (10.0 + 14.0 * hash11(fid * 4.1));
    float sign = has * boxMask(vec2(loc.x - su, 0.0), vec2(0.11, 1.0), px) * step(0.0, sy) * step(sy, 1.0);
    vec3 sc = neonColor(hash11(fid * 6.7));
    emit += sc * sign * 1.8;
    col = mix(col, sc * 0.5, sign);
    return col;
}

// Paved street or deck. Streets run along the lot borders (towers keep 3 m from them): asphalt
// with lane markings between curbs, concrete slabs with joints on the sides. Cracks, dust and
// oil stains over both; the photo detail gives the grain.
vec3 pavement(vec3 base, vec3 p, float px)
{
    vec2 uv = p.xz;
    vec2 off = uv - round(uv / 24.0) * 24.0;  // from the nearest street axes
    vec2 ad = abs(off);
    float street = step(min(ad.x, ad.y), 3.0);

    // Sidewalk: 4 m slabs with joints and a tone per slab.
    vec2 g = uv / 4.0;
    vec2 sid = floor(g);
    vec2 f = fract(g);
    float dj = min(min(f.x, 1.0 - f.x), min(f.y, 1.0 - f.y)) * 4.0;
    float joint = (1.0 - smoothstep(0.02, 0.05 + px, dj)) * (1.0 - street);
    vec3 walk = base * (0.82 + 0.24 * hash12(sid)) * (0.8 + 0.4 * fbm(uv * 0.07));
    walk *= mix(1.0, grit(uv / 2.4).r, 0.85);
    // Road: dark asphalt.
    vec3 road = base * 0.5 * (0.85 + 0.3 * fbm(uv * 0.12)) * mix(1.0, grit(uv / 1.7).g, 0.9);
    vec3 col = mix(walk, road, street);

    // Curbs between road and sidewalk (not across intersections).
    float curbZ = (1.0 - smoothstep(0.1, 0.1 + px, abs(ad.x - 3.0))) * step(3.0, ad.y);
    float curbX = (1.0 - smoothstep(0.1, 0.1 + px, abs(ad.y - 3.0))) * step(3.0, ad.x);
    col = mix(col, base * 1.15, max(curbX, curbZ) * 0.9);

    // Cracks: thin contour lines of a noise field, only in patches.
    float cn = fbm(uv * 0.9 + sid * 1.7);
    float crack = (1.0 - smoothstep(0.0, 0.012 + px * 0.8, abs(cn - 0.5) * 1.6)) * smoothstep(0.62, 0.8, vnoise(uv * 0.14 + 11.0));
    col *= 1.0 - 0.5 * crack;
    // Oil and wet stains, then dust settled in blotches.
    col *= 1.0 - 0.5 * smoothstep(0.6, 0.7, fbm(uv * 0.3 + 20.0));
    float dust = smoothstep(0.52, 0.8, fbm(uv * 0.45 + 3.0));
    col = mix(col, col * vec3(1.35, 1.2, 0.95) + 0.015, dust * 0.5);
    col *= 1.0 - 0.55 * joint;

    // Lane markings: dashed yellow center lines, white edge lines, outside the intersections.
    float wear = (0.45 + 0.55 * vnoise(uv * 3.5)) * grit(uv / 0.9).g;
    float ix = step(3.4, ad.y);
    float iz = step(3.4, ad.x);
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
    col *= grit(uv / 1.5).g;
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

// Bridge walkway: metal plates with a grid of seams; parapets get a bright top edge.
vec3 bridgeMetal(vec3 base, vec3 p, vec3 n, float px)
{
    vec2 uv = abs(n.y) > 0.5 ? p.xz : vec2(p.x + p.z, p.y);
    vec2 g = uv / 1.5;
    vec2 d = abs(fract(g) - 0.5);
    vec2 w = vec2(px / 1.5) * 1.5;
    float seam = max(1.0 - smoothstep(0.5 - 0.02 - w.x, 0.5 - 0.02 + w.x, 0.5 - (0.5 - d.x) ),
                     1.0 - smoothstep(0.5 - 0.02 - w.y, 0.5 - 0.02 + w.y, 0.5 - (0.5 - d.y)));
    vec3 col = base * (0.8 + 0.3 * fbm(uv * 0.6)) * grit(uv / 1.2).a;
    return col * (1.0 - 0.3 * seam);
}


// Rooftop machinery: louvered casing with a fan or cap on top. loc is normalized on top faces.
vec3 equipment(vec3 base, vec3 p, vec3 n, vec2 loc, float px)
{
    vec3 col = base * (0.85 + 0.25 * fbm(p.xz * 1.5 + p.y)) * grit((abs(n.y) > 0.5 ? p.xz : vec2(p.x + p.z, p.y)) / 1.3).a;
    if (n.y > 0.5) {
        vec2 c = loc - 0.5;
        float r = length(c);
        float aa = max(fwidth(r), 1e-3);
        float disc = 1.0 - smoothstep(0.38 - aa, 0.38 + aa, r);
        float inner = 1.0 - smoothstep(0.32 - aa, 0.32 + aa, r);
        float blades = 0.5 + 0.5 * step(0.0, sin(atan(c.y, c.x) * 6.0 + r * 9.0));
        col = mix(col, base * 0.4, disc);
        col = mix(col, base * 0.15 * (0.6 + 0.8 * blades), inner);
        return col;
    }
    if (n.y < -0.5) return col * 0.6;
    float u = loc.x / max(loc.y, 0.01);
    float louver = step(0.12, u) * step(u, 0.88);
    float slat = 1.0 - smoothstep(0.08, 0.08 + px * 8.0, abs(fract(p.y * 8.0) - 0.5) * 2.0 - 0.55);
    col *= 1.0 - 0.4 * louver * slat;
    col *= 1.0 - 0.25 * smoothstep(0.55, 0.9, vnoise(vec2(loc.x * 4.0, p.y * 0.3)));
    return col;
}

// Billboard art: a colored field with a round emblem and rows of blocky "text", backlit.
vec3 billboard(vec2 uv, float fid, float px, out vec3 emit)
{
    float h = hash11(fid * 3.3);
    vec3 bg = h < 0.3 ? vec3(0.9, 0.62, 0.04) : h < 0.55 ? vec3(0.9, 0.3, 0.05)
            : h < 0.75 ? vec3(0.05, 0.55, 0.7) : h < 0.9 ? vec3(0.75, 0.1, 0.45) : vec3(0.85, 0.85, 0.82);
    vec3 fg = h >= 0.9 ? vec3(0.05, 0.06, 0.08) : vec3(0.95, 0.95, 0.92);
    vec2 q = vec2((uv.x - 0.5) * 2.4, uv.y - 0.5);
    float aa = max(fwidth(q.x), 1e-3);
    vec3 col = bg;
    float ring = abs(length(q - vec2(-0.72, 0.0)) - 0.3);
    float emblem = 1.0 - smoothstep(0.07 - aa, 0.07 + aa, ring);
    emblem = max(emblem, 1.0 - smoothstep(0.12 - aa, 0.12 + aa, length(q - vec2(-0.72, 0.0))));
    float rows = 0.0;
    for (int i = 0; i < 3; i++) {
        float y = 0.2 - float(i) * 0.19;
        float inRow = step(abs(q.y - y), 0.07) * step(-0.25, q.x) * step(q.x, 1.05 - 0.25 * float(i) * hash11(fid + float(i)));
        float word = step(0.28, hash12(vec2(floor((q.x + 0.25) * 7.0), float(i) + fid)));
        rows = max(rows, inRow * word);
    }
    col = mix(col, fg, max(emblem, rows));
    float border = 1.0 - smoothstep(0.0, 0.03, min(min(uv.x, 1.0 - uv.x) * 2.4 / 2.4, min(uv.y, 1.0 - uv.y)));
    col = mix(col, bg * 0.5, border);
    emit = col * 0.9;
    return col;
}

// Antenna mast: white with darker rungs.
vec3 mast(vec3 base, vec3 p, float px)
{
    float rung = 1.0 - smoothstep(0.03, 0.03 + px, abs(fract(p.y / 0.7) - 0.5) - 0.42);
    return base * (1.0 - 0.5 * rung);
}


// Rooftop solar array: dark blue cells with silver seams and a sheen.
vec3 solar(vec3 base, vec3 p, vec3 n, float px)
{
    if (n.y < 0.5) return base * 0.6;
    vec2 g = p.xz / 0.55;
    vec2 d = abs(fract(g) - 0.5);
    float seam = 1.0 - smoothstep(0.46, 0.46 + px / 0.55, max(d.x, d.y));
    float sheen = 0.7 + 0.6 * smoothstep(0.2, 0.9, vnoise(p.xz * 0.35));
    return mix(vec3(0.5, 0.52, 0.56), base * sheen * 1.8, seam);
}

// Skylight: dark glass in a frame with a cross of bars.
vec3 skylight(vec3 base, vec3 loc3, vec3 n, vec2 loc, float px)
{
    if (n.y < 0.5) return base * 0.7;
    vec2 c = abs(loc - 0.5);
    float frame = step(0.42, max(c.x, c.y));
    float bars = 1.0 - smoothstep(0.015, 0.015 + fwidth(c.x) , min(c.x, c.y));
    return mix(base * 0.25, vec3(0.5, 0.52, 0.55), max(frame, bars));
}

// Street lamp: dull painted metal.
vec3 lamp(vec3 base, vec3 p)
{
    return base * (0.8 + 0.4 * fbm(vec2(p.x + p.z, p.y) * 1.3)) * grit(vec2(p.x + p.z, p.y)).a;
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
    int kind = int(fragKind + 0.5);
    vec3 emit = vec3(0.0);
    if (painted) {
        float px = max(length(fwidth(fragWorldPos)), 1e-4);
        float fid = fragId * 255.0;
        bool vertical = abs(n.y) < 0.5;
        if (kind == 1) {
            albedo *= paintPlant(fragWorldPos);
            if (fragLoc.x >= 0.0) albedo = leaf(albedo, fragLoc);
        } else if (kind == 0) {
            if (vertical) albedo = facade(albedo, fragWorldPos, fragLoc, fid, px, emit);
            else if (n.y > 0.0) albedo = roof(albedo, fragWorldPos, px);
            else albedo = underside(albedo, fragWorldPos, px);
        } else if (kind == 2) {
            if (n.y > 0.5) albedo = pavement(albedo, fragWorldPos, px);
            else if (n.y < -0.5) albedo = underside(albedo, fragWorldPos, px);
            else albedo = concrete(albedo, fragWorldPos, n, fragLoc);
        } else if (kind == 3) {
            albedo = bridgeMetal(albedo, fragWorldPos, n, px);
        } else if (kind == 4) {
            // Stone steps.
            vec2 suv = abs(n.y) > 0.5 ? fragWorldPos.xz : vec2(fragWorldPos.x + fragWorldPos.z, fragWorldPos.y);
            albedo *= (0.8 + 0.4 * fbm(suv * 1.5)) * grit(suv / 1.4).r;
        } else if (kind == 6) {
            albedo = concrete(albedo, fragWorldPos, n, fragLoc);
        } else if (kind == 7) {
            albedo = equipment(albedo, fragWorldPos, n, fragLoc, px);
        } else if (kind == 8) {
            if (vertical && fragLoc.x > -0.5) {
                albedo = billboard(fragLoc, fid, px, emit);
            } else {
                albedo = albedo * 0.5;
            }
        } else if (kind == 9) {
            albedo = mast(albedo, fragWorldPos, px);
        } else if (kind == 10) {
            albedo = solar(albedo, fragWorldPos, n, px);
        } else if (kind == 11) {
            albedo = skylight(albedo, fragWorldPos, n, fragLoc, px);
        } else if (kind == 12) {
            albedo = lamp(albedo, fragWorldPos);
        }
    }

    float ndl = dot(n, uLightDir);
    float sh = ndl > 0.0 ? shadow(fragWorldPos, n) : 0.0;
    float direct;
    if ((uEffects & FX_CEL) != 0) {
        // Three flat bands: shadow, half-lit, lit. Cast shadows snap to the shadow band.
        sh = smoothstep(0.3, 0.7, sh);
        direct = (0.45 * band(ndl, 0.05) + 0.55 * band(ndl, 0.5)) * sh;
    } else {
        direct = max(ndl, 0.0) * sh;
    }

    // Hemisphere ambient: sky from above, bounced ground light from below.
    vec3 ambient = mix(uAmbientGround, uAmbientSky, n.y * 0.5 + 0.5);

    // Painted vertical gradient: darker at street level, slightly lifted up high.
    float y = fragWorldPos.y;
    float grad = mix(0.55, 1.0, smoothstep(0.0, 10.0, y)) * (1.0 + 0.15 * smoothstep(10.0, 80.0, y));

    // Plants glow regardless of the hour in the lightless depths.
    float glow = mix(uGlow, max(uGlow, 0.85), 1.0 - smoothstep(-60.0, -10.0, fragWorldPos.y));
    vec3 lit = albedo * (ambient * grad + uLight * direct + plantLights(fragWorldPos, n) * glow);
    lit += albedo * fragColor.a * glow * 2.5;
    lit += emit * glow * 0.5;
    outColor = vec4(lit, 1.0);
    outNormal = vec4(n * 0.5 + 0.5, fragId);
}
