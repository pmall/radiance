#version 330

// Depth-only pass from the key light. Props are drawn instanced (src/render/props.rs).
in vec3 vertexPosition;
in mat4 instanceTransform;
uniform mat4 mvp;
uniform int uInstanced;

void main()
{
    vec4 p = vec4(vertexPosition, 1.0);
    if (uInstanced != 0) p = instanceTransform * p;
    gl_Position = mvp * p;
}
