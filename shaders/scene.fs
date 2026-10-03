#version 330

// Scene geometry: flat cel-shaded lighting in linear HDR, plus normals and object id
// for the outline pass.

in vec3 fragWorldPos;
in vec3 fragNormal;
in vec4 fragColor;
flat in float fragId;

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
uniform vec3 uGridOrigin;
uniform float uGlow;         // plant emission and light strength for the time of day

layout(location = 0) out vec4 outColor;
layout(location = 1) out vec4 outNormal;

const int FX_CEL = 4;

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

void main()
{
    vec3 n = normalize(fragNormal);
    vec3 albedo = pow(fragColor.rgb, vec3(2.2));

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
    outColor = vec4(lit, 1.0);
    outNormal = vec4(n * 0.5 + 0.5, fragId);
}
