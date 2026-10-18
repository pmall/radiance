#version 330

// A pixel-art halo: bright core, a mid ring and a dithered outer glow, additive, hidden by scene
// depth.

in vec2 fragCorner;
in vec4 fragColor;
in float fragViewZ;

uniform sampler2D texture1;  // scene depth
uniform vec2 uScreen;
uniform float uNear;
uniform float uFar;

out vec4 finalColor;

float linearDepth(float d)
{
    float z = d * 2.0 - 1.0;
    return 2.0 * uNear * uFar / (uFar + uNear - z * (uFar - uNear));
}

void main()
{
    float r = length(fragCorner);
    if (r > 1.0) discard;
    vec2 uv = gl_FragCoord.xy / uScreen;
    // The flower itself sits at the center, so allow for its own thickness.
    float hidden = linearDepth(texture(texture1, uv).r) - (fragViewZ - 0.5);
    float visible = smoothstep(0.0, 0.4, hidden);
    if (visible <= 0.0) discard;

    // Hard steps, and the outer glow as a dither of 2x2 screen pixels, like the textures.
    vec2 q = floor(gl_FragCoord.xy / 2.0);
    float dither = fract(dot(q, vec2(0.5, 0.25)) + 0.25 * mod(q.y, 2.0));
    float a = 0.85 * step(r, 0.2)
            + 0.3 * step(r, 0.45)
            + 0.5 * step(dither, 0.45 - 0.45 * r) * step(r, 1.0);
    vec3 col = mix(fragColor.rgb, vec3(1.0), 0.5 * (1.0 - smoothstep(0.1, 0.25, r)));
    float fade = exp(-fragViewZ * 0.008);
    finalColor = vec4(col, a * fragColor.a * visible * fade);
}
