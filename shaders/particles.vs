#version 330

// Drifting motes and spores. Everything is computed here from per-particle random numbers:
// no CPU simulation. Particles wrap inside a box around the camera and are drawn as billboards.

in vec3 vertexPosition;   // quad corner in [-1, 1]
in vec2 vertexTexCoord;   // random xy (position)
in vec3 vertexNormal;     // random z (position), random size, random phase
in vec4 vertexColor;      // random kind, speed, hue, drip (under 0.03)

uniform mat4 mvp;
uniform vec3 uCamPos;
uniform vec3 uCamFwd;
uniform vec3 uCamRight;
uniform vec3 uCamUp;
uniform float uTime;

uniform sampler2D texture2;  // cover map
#define COVER_TEX texture2

#include "cover.glsl"

out vec2 fragCorner;
out vec4 fragData;       // x: fade, y: spore (0/1), z: hue random, w: drip (0/1)
out float fragViewZ;

const float BOX = 90.0;
const float DRIP_BOX = 60.0;

// Water dripping from the deck above, at fixed spots of the world: a streak falling from the
// girders to the floor, only where a deck covers the lot.
void drip(vec3 r, float phase)
{
    vec3 local = vec3(mod(r.x * DRIP_BOX - uCamPos.x, DRIP_BOX), 0.0, mod(r.z * DRIP_BOX - uCamPos.z, DRIP_BOX))
               - vec3(0.5 * DRIP_BOX, 0.0, 0.5 * DRIP_BOX);
    vec3 p = vec3(uCamPos.x + local.x, 0.0, uCamPos.z + local.z);
    int f = floorAt(uCamPos.y);
    bool covered = f + 1 < uFloorCount && ((coverBits(p.xz) >> f) & 1) != 0;
    float top = covered ? uFloorY[f + 1] - 2.6 : uFloorY[f];
    float h = max(top - uFloorY[f], 0.0);
    // Falling at about 9 m/s, a few seconds apart, from one of the spots.
    float cycle = h / 9.0 + 1.5 + 3.0 * r.y;
    float t = mod(uTime + phase * 3.0, cycle) * 9.0;
    p.y = top - t;
    float live = covered && t < h ? 1.0 : 0.0;
    local.y = p.y - uCamPos.y;
    float dist = length(local);
    float w = min(0.012, dist * 0.0012);
    fragCorner = vertexPosition.xy;
    fragData = vec4(live * smoothstep(1.5, 4.0, dist) * (1.0 - smoothstep(0.38, 0.5, length(local.xz) / DRIP_BOX)), 0.0, 0.0, 1.0);
    fragViewZ = dot(local, uCamFwd);
    vec3 wp = p + uCamRight * vertexPosition.x * w + vec3(0.0, 1.0, 0.0) * vertexPosition.y * w * 14.0;
    gl_Position = mvp * vec4(wp, 1.0);
}

void main()
{
    vec3 r = vec3(vertexTexCoord, vertexNormal.x);
    float phase = vertexNormal.z * 6.2831;
    if (vertexColor.a < 0.03) {
        drip(r, phase);
        return;
    }
    float spore = step(0.82, vertexColor.r);
    float speed = 0.3 + vertexColor.g * (1.0 - 0.6 * spore);

    // Slow wind drift plus a lazy wobble; spores float upward.
    vec3 drift = vec3(0.7, mix(-0.12, 0.35, spore), 0.3) * uTime * speed
               + vec3(sin(uTime * 0.5 + phase), 0.5 * sin(uTime * 0.37 + phase * 1.7), cos(uTime * 0.43 + phase * 0.6)) * 1.4;

    vec3 local = mod(r * BOX + drift - uCamPos, BOX) - 0.5 * BOX;
    vec3 p = uCamPos + local;

    float edge = 1.0 - smoothstep(0.35, 0.5, max(abs(local.x), max(abs(local.y), abs(local.z))) / BOX);
    float dist = length(local);
    float near = smoothstep(2.0, 6.0, dist);
    float twinkle = mix(1.0, 0.55 + 0.45 * sin(uTime * 1.6 + phase * 3.0), spore);

    float size = mix(0.03, 0.08, vertexNormal.y) * (1.0 - spore) + mix(0.12, 0.3, vertexNormal.y) * spore;
    // Specks, not blurred discs: never more than a few screen pixels across, however near.
    size = min(size, dist * mix(0.0022, 0.0045, spore));
    fragCorner = vertexPosition.xy;
    fragData = vec4(edge * near * twinkle, spore, vertexColor.b, 0.0);
    fragViewZ = dot(local, uCamFwd);
    vec3 wp = p + (uCamRight * vertexPosition.x + uCamUp * vertexPosition.y) * size;
    gl_Position = mvp * vec4(wp, 1.0);
}
