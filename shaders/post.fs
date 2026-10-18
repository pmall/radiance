#version 330

// Composite: sky, ink outlines, colored height fog, grading, paper grain.
// Every target shares GL's bottom-left origin, so screen uv addresses them directly.

in vec2 fragTexCoord;

uniform sampler2D texture0;  // scene color (linear HDR)
uniform sampler2D uNormal;   // rgb: normal * 0.5 + 0.5, a: object id (0 = sky)
uniform sampler2D uDepth;

uniform vec2 uScreen;        // output size in pixels
uniform vec2 uTexel;         // 1 / scene target size
uniform float uNear;
uniform float uFar;
uniform vec3 uCamPos;
uniform vec3 uCamFwd;
uniform vec3 uCamRight;
uniform vec3 uCamUp;
uniform float uTanHalfFov;
uniform float uAspect;

uniform vec3 uSkyZenith;
uniform vec3 uSkyHorizon;
uniform vec3 uDisk;
uniform float uStars;
uniform vec3 uLightDir;
uniform vec3 uFog;
uniform float uFogDensity;
uniform float uFogFalloff;
uniform float uFogGlow;
uniform vec3 uGlowHaze;      // the flowers' light scattered in the haze around the eye (src/render/mod.rs)
uniform float uExposure;
uniform float uSaturation;
uniform float uContrast;
uniform vec3 uShadowTint;
uniform vec3 uHighlightTint;

uniform int uEffects;
uniform float uTime;
uniform float uFogBase;      // fog density stops growing below this height
uniform sampler2D uCover;
#define COVER_TEX uCover

#include "cover.glsl"

uniform float uLineWidth;    // in scene pixels

out vec4 finalColor;

const int FX_OUTLINES = 1;
const int FX_FOG = 2;
const int FX_GRADING = 8;
const int FX_GRAIN = 16;

float hash12(vec2 p)
{
    vec3 p3 = fract(vec3(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

float noise(vec2 p)
{
    vec2 i = floor(p);
    vec2 f = fract(p);
    vec2 u = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash12(i), hash12(i + vec2(1, 0)), u.x),
               mix(hash12(i + vec2(0, 1)), hash12(i + vec2(1, 1)), u.x), u.y);
}

float luma(vec3 c) { return dot(c, vec3(0.2126, 0.7152, 0.0722)); }

float linearDepth(float d)
{
    float z = d * 2.0 - 1.0;
    return 2.0 * uNear * uFar / (uFar + uNear - z * (uFar - uNear));
}

// Soft posterization: flat steps with short smooth transitions, like painted bands.
float posterize(float x, float steps)
{
    float s = x * steps;
    float f = fract(s);
    return (floor(s) + smoothstep(0.35, 0.65, f)) / steps;
}

vec3 sky(vec3 dir)
{
    float h = dir.y;
    float g = posterize(pow(clamp(h, 0.0, 1.0), 0.5), 6.0);
    vec3 col = mix(uSkyHorizon, uSkyZenith, g);
    col = mix(col, uFog, smoothstep(0.0, -0.2, h));

    float c = dot(dir, uLightDir);
    // Broad glow around the key light, then a flat illustrated disk.
    col += uDisk * uFogGlow * (0.25 * pow(max(c, 0.0), 8.0) + 0.35 * pow(max(c, 0.0), 64.0));
    float r = cos(0.03);
    col = mix(col, uDisk * 1.6, smoothstep(r - 0.00015, r + 0.00015, c));

    if (uStars > 0.0) {
        vec2 sp = vec2(atan(dir.z, dir.x), asin(clamp(h, -1.0, 1.0))) * 140.0;
        vec2 cell = floor(sp);
        float star = step(0.985, hash12(cell));
        float d = length(fract(sp) - 0.5);
        float tw = 0.6 + 0.4 * sin(uTime * 2.0 + hash12(cell + 7.0) * 40.0);
        col += vec3(0.9, 0.9, 1.0) * star * smoothstep(0.18, 0.05, d) * tw * uStars * smoothstep(0.05, 0.3, h);
    }
    return col;
}

// Fraction of light lost to exponential height fog along a view ray.
float fogAmount(vec3 dir, float dist)
{
    // Density plateaus below the canopy instead of growing into the depths.
    bool below = uCamPos.y < uFogBase;
    float a = uFogDensity * exp(-uFogFalloff * max(uCamPos.y - uFogBase, 0.0));
    float b = below ? 0.0 : uFogFalloff * dir.y * dist;
    float k = abs(b) > 1e-4 ? (1.0 - exp(-b)) / b : 1.0;
    // A thin uniform haze on top, so tall distant structures fade too.
    float optical = a * dist * k + uFogDensity * 0.06 * dist;
    return 1.0 - exp(-optical);
}

// Light the fog gets along a view ray: x = share of it with open sky above, y = share in the
// sun, z = share in a beam: sun reaching air that sees little sky, through a hole or a crater
// into the floors below. Samples along the ray, so openings in the floors show as shafts.
vec3 fogLight(vec3 dir, float dist)
{
    const int N = 16;
    // Interleaved gradient noise: a different sample offset per pixel, hidden by the grain.
    float jitter = fract(52.9829189 * fract(dot(gl_FragCoord.xy, vec2(0.06711056, 0.00583715))));
    float reach = min(dist, 420.0);
    vec3 acc = vec3(0.0);
    for (int i = 0; i < N; i++) {
        vec3 q = uCamPos + dir * reach * (float(i) + jitter) / float(N);
        float v = skyView(q), s = skyVis(q, uLightDir);
        acc += vec3(v, s, s * (1.0 - v) * (1.0 - v));
    }
    return acc / float(N);
}

// Fog color along a view ray of length dist, f = its fog amount. Fog sinks into darkness under
// the decks, where little light gets down; where the sun gets through, it lights the fog as beams.
const float DEEP_FOG = 0.3;   // brightness of the haze where no sky reaches, against the open fog
float gBeam = 0.0;            // light of the sunbeams along the ray (fogColor), added over the fog

vec3 fogColor(vec3 dir, float dist, float f)
{
    float c = max(dot(dir, uLightDir), 0.0);
    vec3 fl = fogLight(dir, dist);
    // Under the decks the haze takes the hue of the floor (cold depths, warm middle, gold
    // canopy), brighter than the shaded walls in it, so they stand as silhouettes.
    vec3 deep = heightHue(uCamPos.y) * luma(uFog) * DEEP_FOG + uGlowHaze * 0.04;
    vec3 col = mix(deep, uFog, fl.x) + uDisk * uFogGlow * (0.1 + 0.2 * c * c + 0.3 * pow(c, 6.0)) * fl.y;
    // Sunbeams through the openings: light scattered by the haze along the lit stretch of the
    // ray, strongest looking toward the sun, against the dark under the decks.
    float lit = fl.z * min(dist, 420.0);
    gBeam = (1.0 - exp(-lit * 0.02)) * (0.35 + 0.65 * c * c + 1.2 * pow(c, 8.0));
    // Far fog blends into the sky behind it so silhouettes melt into the horizon.
    return mix(col, mix(deep, sky(dir), fl.x), smoothstep(0.4, 1.0, f));
}

void main()
{
    vec2 uv = gl_FragCoord.xy / uScreen;
    vec2 ndc = uv * 2.0 - 1.0;
    vec3 dir = normalize(uCamFwd + ndc.x * uTanHalfFov * uAspect * uCamRight + ndc.y * uTanHalfFov * uCamUp);

    float rawDepth = texture(uDepth, uv).r;
    vec3 col;

    if (rawDepth >= 1.0) {
        col = sky(dir);
        // Past the loaded city, a ray that would run into the floors above ends in their dark
        // fog, not in the sky. Level rays only see the sky from out in the open.
        float clear = dir.y < 0.02 ? step(0.9, skyView(uCamPos)) : skyVis(uCamPos, dir);
        if ((uEffects & FX_FOG) != 0 && clear < 1.0) {
            col = fogColor(dir, 600.0, 1.0);
            col += uDisk * uFogGlow * gBeam * 0.45;
        }
    } else {
        col = texture(texture0, uv).rgb;
        float z = linearDepth(rawDepth);

        if ((uEffects & FX_OUTLINES) != 0) {
            vec2 o = uTexel * uLineWidth;
            vec4 nc = texture(uNormal, uv);
            float wc = 1.0 / z;
            float edge = 0.0;

            // Silhouettes: second derivative of inverse depth (zero on any plane),
            // keeping only the near side so ink belongs to the foreground object.
            float wl = 1.0 / linearDepth(texture(uDepth, uv - vec2(o.x, 0)).r);
            float wr = 1.0 / linearDepth(texture(uDepth, uv + vec2(o.x, 0)).r);
            float wd = 1.0 / linearDepth(texture(uDepth, uv - vec2(0, o.y)).r);
            float wu = 1.0 / linearDepth(texture(uDepth, uv + vec2(0, o.y)).r);
            float lap = min(wl + wr - 2.0 * wc, wd + wu - 2.0 * wc) / wc;
            edge = max(edge, smoothstep(0.08, 0.16, -lap));

            // Creases and touching objects: normal or id changes toward +x / +y,
            // ignored when the neighbor is a nearer object (its own silhouette).
            for (int i = 0; i < 2; i++) {
                vec2 off = i == 0 ? vec2(o.x, 0) : vec2(0, o.y);
                vec4 nn = texture(uNormal, uv + off);
                float wn = i == 0 ? wr : wu;
                if (nn.a == 0.0 || wn > wc * 1.05) continue;
                float crease = 1.0 - dot(nc.xyz * 2.0 - 1.0, nn.xyz * 2.0 - 1.0);
                edge = max(edge, smoothstep(0.3, 0.6, crease));
                // Another object, unless it is the same plane going on (two lots' slabs of one
                // deck): depth continues the gradient across and the normal is the same.
                float wf = i == 0 ? wl : wd;
                bool plane = crease < 0.002 && abs(wn - 2.0 * wc + wf) < 0.02 * wc;
                if (abs(nn.a - nc.a) > 0.001 && !plane) edge = 1.0;
            }

            // Black ink at every hour, in every biome; the fog over it fades the far lines.
            col = mix(col, col * 0.05, edge);
        }

        if ((uEffects & FX_FOG) != 0) {
            float dist = z / dot(dir, uCamFwd);
            float f = fogAmount(dir, dist);
            col = mix(col, fogColor(dir, dist, f), f);
            col += uDisk * uFogGlow * gBeam * 0.45;
        }
    }

    // The eye adapts: under the decks the image opens up a little.
    float adapt = mix(1.35, 1.0, skyView(uCamPos));
    // Tonemap: gentle exponential shoulder, then to display sRGB.
    col = 1.0 - exp(-col * uExposure * adapt);
    col = pow(col, vec3(1.0 / 2.2));

    if ((uEffects & FX_GRADING) != 0) {
        float l = luma(col);
        // Split toning: a gentle hue push that keeps brightness (tints are only used for hue).
        vec3 sh = uShadowTint / max(max(uShadowTint.r, uShadowTint.g), max(uShadowTint.b, 1e-3));
        vec3 hl = uHighlightTint / max(max(uHighlightTint.r, uHighlightTint.g), max(uHighlightTint.b, 1e-3));
        vec3 m = mix(vec3(1.0), mix(sh, hl, smoothstep(0.1, 0.8, l)), 0.3);
        col *= m / luma(m);
        col = (col - 0.45) * uContrast + 0.45;
        col = mix(vec3(luma(col)), col, uSaturation);
        float v = length((uv - 0.5) * vec2(uAspect, 1.0));
        col *= mix(1.0, 0.8, smoothstep(0.5, 1.2, v));
    }

    if ((uEffects & FX_GRAIN) != 0) {
        vec2 px = gl_FragCoord.xy;
        float paper = noise(px * 0.35) * 0.5 + noise(px * 0.04) * 0.35 + noise(px * 0.008) * 0.15;
        col *= 1.0 + (paper - 0.5) * 0.08;
        col += (hash12(px + fract(uTime * 13.0) * 917.0) - 0.5) * 0.03;
    } else {
        col += (hash12(gl_FragCoord.xy) - 0.5) / 255.0;
    }

    finalColor = vec4(clamp(col, 0.0, 1.0), 1.0);
}
