#version 330

// Glowing halos at the flowers. Each quad belongs to one entry of the light texture (see
// src/render/lights.rs), so position, size and color come straight from there: no CPU work.

in vec3 vertexPosition;   // quad corner in [-1, 1]
in vec2 vertexTexCoord;   // x: light index

uniform mat4 mvp;
uniform sampler2D texture2;  // plant light grid
uniform vec3 uCamPos;
uniform vec3 uCamFwd;
uniform vec3 uCamRight;
uniform vec3 uCamUp;
uniform int uCount;
uniform float uGlow;         // plant glow strength for the time of day

out vec2 fragCorner;
out vec4 fragColor;
out float fragViewZ;

const int LW = 256;

vec4 fetchTexel(int i)
{
    return texelFetch(texture2, ivec2(i % LW, i / LW), 0);
}

void main()
{
    int i = int(vertexTexCoord.x + 0.5);
    if (i >= uCount) {
        gl_Position = vec4(2.0, 2.0, 2.0, 1.0);
        fragCorner = vec2(2.0);
        fragColor = vec4(0.0);
        fragViewZ = 0.0;
        return;
    }
    vec4 a = fetchTexel(i * 3);
    vec4 light = fetchTexel(i * 3 + 1);
    vec3 lin = light.rgb / 2.2;
    vec3 d = a.xyz - uCamPos;
    float dist = length(d);

    // Same rule as the scene shader: always strong in the lightless depths.
    float glow = mix(uGlow, max(uGlow, 0.85), 1.0 - smoothstep(-60.0, -10.0, a.y));
    // A fixed world size up close, growing a little with distance so far flowers stay visible.
    float size = max(0.5, dist * 0.01);

    fragCorner = vertexPosition.xy;
    // The halo fades out with distance, before the lights run out (lights.rs).
    fragColor = vec4(pow(lin, vec3(1.0 / 2.2)), mix(0.3, 1.0, glow) * light.w);
    fragViewZ = dot(d, uCamFwd);
    vec3 wp = a.xyz + (uCamRight * vertexPosition.x + uCamUp * vertexPosition.y) * size;
    gl_Position = mvp * vec4(wp, 1.0);
}
