// Dissolving a draw instead of cutting it at its reach (`Fade` in src/render/mod.rs): a
// screen-door dither between uFade.x and uFade.y meters off the camera, height differences
// counting uFade.z times. Included by scene.fs and shadow.fs, which declare uFade and uCamPos.

// 4x4 ordered dither threshold at pixel q, in 0..1 (the textures' Bayer matrix, worked out).
float bayer2(vec2 a)
{
    a = floor(a);
    return fract(dot(a, vec2(0.5, a.y * 0.75)));
}

float bayer4(vec2 q)
{
    return bayer2(0.5 * q) * 0.25 + bayer2(q) + 1.0 / 32.0;
}

// Whether the fragment of a thing at p is dissolved away.
bool faded(vec3 p)
{
    vec3 d = p - uCamPos;
    d.y *= uFade.z;
    return bayer4(gl_FragCoord.xy) > 1.0 - smoothstep(uFade.x, uFade.y, length(d));
}
