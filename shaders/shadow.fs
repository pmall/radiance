#version 330

// Casters dissolve before their shadow reach like they do in the scene (`Fade` in
// src/render/mod.rs), on the shadow map's texels; the lookup's PCF softens the dither.
in vec3 fragWorldPos;
flat in vec3 fragOrigin;

uniform vec3 uFade;          // from, to (meters off the camera), weight of height differences
uniform vec3 uCamPos;
uniform int uInstanced;

out vec4 finalColor;

#include "fade.glsl"

void main()
{
    if (faded(uInstanced != 0 ? fragOrigin : fragWorldPos)) discard;
    finalColor = vec4(1.0);
}
