#version 330

// Square additive specks, hidden behind scene geometry by comparing against the scene depth.

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
uniform vec3 uSpores[4];     // the biome's spore colors (display space)

out vec4 finalColor;

float linearDepth(float d)
{
    float z = d * 2.0 - 1.0;
    return 2.0 * uNear * uFar / (uFar + uNear - z * (uFar - uNear));
}

vec3 species(float h)
{
    return uSpores[min(int(h * 4.0), 3)];
}

void main()
{
    vec2 uv = gl_FragCoord.xy / uScreen;
    if (fragViewZ > linearDepth(texture(texture1, uv).r)) discard;

    // A crisp square speck, like a pixel of the textures.
    float a = 1.0;
    float spore = fragData.y;
    float glow = max(uGlow, 0.85 * uDepthK);
    vec3 col = mix(uMote * mix(0.55, 0.2, uGlow), species(fragData.z) * 0.95, spore);
    // Spores belong to the night and the depths; motes are the daytime pollen haze.
    float vis = mix(1.0 - 0.6 * uGlow, glow, spore);
    float fade = exp(-fragViewZ * 0.012);
    // Drips: pale water catching the haze.
    col = mix(col, vec3(0.42, 0.5, 0.55), fragData.w);
    vis = mix(vis, 0.55, fragData.w);
    finalColor = vec4(col, a * fragData.x * vis * fade * 0.85);
}
