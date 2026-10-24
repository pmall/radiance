#version 330

// Depth-only pass from the key light. Props are drawn instanced (src/render/props.rs).
in vec3 vertexPosition;
in mat4 instanceTransform;
uniform mat4 mvp;
uniform int uInstanced;

out vec3 fragWorldPos;
flat out vec3 fragOrigin;    // where a prop stands, which its fade goes by (shadow.fs)

void main()
{
    vec4 p = vec4(vertexPosition, 1.0);
    if (uInstanced != 0) p = instanceTransform * p;
    fragWorldPos = p.xyz;
    fragOrigin = uInstanced != 0 ? instanceTransform[3].xyz : vec3(0.0);
    gl_Position = mvp * p;
}
