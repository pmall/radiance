#version 330

// A painted halo: bright core, a mid ring and a soft outer ring, additive, hidden by scene depth.

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

    float a = 0.85 * (1.0 - smoothstep(0.16, 0.22, r))
            + 0.28 * (1.0 - smoothstep(0.42, 0.52, r))
            + 0.14 * (1.0 - smoothstep(0.80, 1.0, r));
    vec3 col = mix(fragColor.rgb, vec3(1.0), 0.5 * (1.0 - smoothstep(0.1, 0.25, r)));
    float fade = exp(-fragViewZ * 0.008);
    finalColor = vec4(col, a * fragColor.a * visible * fade);
}
