#version 330

// Drifting motes and spores. Everything is computed here from per-particle random numbers:
// no CPU simulation. Particles wrap inside a box around the camera and are drawn as billboards.

in vec3 vertexPosition;   // quad corner in [-1, 1]
in vec2 vertexTexCoord;   // random xy (position)
in vec3 vertexNormal;     // random z (position), random size, random phase
in vec4 vertexColor;      // random kind, speed, hue, spare

uniform mat4 mvp;
uniform vec3 uCamPos;
uniform vec3 uCamFwd;
uniform vec3 uCamRight;
uniform vec3 uCamUp;
uniform float uTime;

out vec2 fragCorner;
out vec4 fragData;       // x: fade, y: spore (0/1), z: hue random
out float fragViewZ;

const float BOX = 90.0;

void main()
{
    vec3 r = vec3(vertexTexCoord, vertexNormal.x);
    float phase = vertexNormal.z * 6.2831;
    float spore = step(0.82, vertexColor.r);
    float speed = 0.3 + vertexColor.g * (1.0 - 0.6 * spore);

    // Slow wind drift plus a lazy wobble; spores float upward.
    vec3 drift = vec3(0.7, mix(-0.12, 0.35, spore), 0.3) * uTime * speed
               + vec3(sin(uTime * 0.5 + phase), 0.5 * sin(uTime * 0.37 + phase * 1.7), cos(uTime * 0.43 + phase * 0.6)) * 1.4;

    vec3 local = mod(r * BOX + drift - uCamPos, BOX) - 0.5 * BOX;
    vec3 p = uCamPos + local;

    float edge = 1.0 - smoothstep(0.35, 0.5, max(abs(local.x), max(abs(local.y), abs(local.z))) / BOX);
    float dist = length(local);
    float near = smoothstep(1.0, 4.0, dist);
    float twinkle = mix(1.0, 0.55 + 0.45 * sin(uTime * 1.6 + phase * 3.0), spore);

    float size = mix(0.03, 0.08, vertexNormal.y) * (1.0 - spore) + mix(0.12, 0.3, vertexNormal.y) * spore;
    fragCorner = vertexPosition.xy;
    fragData = vec4(edge * near * twinkle, spore, vertexColor.b, 1.0);
    fragViewZ = dot(local, uCamFwd);
    vec3 wp = p + (uCamRight * vertexPosition.x + uCamUp * vertexPosition.y) * size;
    gl_Position = mvp * vec4(wp, 1.0);
}
