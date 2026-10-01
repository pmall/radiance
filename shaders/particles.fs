#version 330

// Soft additive sprites, hidden behind scene geometry by comparing against the scene depth.

in vec2 fragCorner;
in vec4 fragData;
in float fragViewZ;

uniform sampler2D texture1;  // scene depth
uniform vec2 uScreen;
uniform float uNear;
uniform float uFar;
uniform float uGlow;         // plant glow strength for the time of day
uniform float uDepthK;       // 0 in the canopy, 1 in the depths
uniform vec3 uMote;          // mote tint (display space)

out vec4 finalColor;

float linearDepth(float d)
{
    float z = d * 2.0 - 1.0;
    return 2.0 * uNear * uFar / (uFar + uNear - z * (uFar - uNear));
}

vec3 species(float h)
{
    if (h < 0.25) return vec3(0.35, 0.95, 0.9);
    if (h < 0.5) return vec3(0.55, 1.0, 0.5);
    if (h < 0.75) return vec3(1.0, 0.45, 0.85);
    return vec3(0.6, 0.6, 1.0);
}

void main()
{
    float r = length(fragCorner);
    if (r > 1.0) discard;
    vec2 uv = gl_FragCoord.xy / uScreen;
    if (fragViewZ > linearDepth(texture(texture1, uv).r)) discard;

    // A crisp core with a soft rim reads as a painted dot.
    float a = smoothstep(1.0, 0.55, r);
    float spore = fragData.y;
    float glow = max(uGlow, 0.85 * uDepthK);
    vec3 col = mix(uMote * mix(0.55, 0.2, uGlow), species(fragData.z) * 0.95, spore);
    // Spores belong to the night and the depths; motes are the daytime pollen haze.
    float vis = mix(1.0 - 0.6 * uGlow, glow, spore);
    float fade = exp(-fragViewZ * 0.012);
    finalColor = vec4(col, a * fragData.x * vis * fade * 0.85);
}
