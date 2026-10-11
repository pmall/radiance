#version 330

// Scene geometry. Attribute and matrix names are raylib's defaults so DrawMesh binds them.
in vec3 vertexPosition;
in vec2 vertexTexCoord;
in vec3 vertexNormal;
in vec4 vertexColor;
in vec2 vertexTexCoord2;     // position along a vertical face, face width
in mat4 instanceTransform;   // props, drawn instanced (src/render/props.rs)

uniform mat4 mvp;
uniform mat4 matModel;
uniform mat4 matNormal;
uniform int uInstanced;

out vec3 fragWorldPos;
out vec3 fragNormal;
out vec4 fragColor;
flat out float fragId;
flat out float fragKind;     // material: 0 tower, 1 plant, 2 floor/deck, 3 bridge, 4 stair, 5 test block
out vec2 fragLoc;

void main()
{
    vec4 local = vec4(vertexPosition, 1.0);
    vec3 normal = vertexNormal;
    fragId = vertexTexCoord.x;
    if (uInstanced != 0) {
        local = instanceTransform * local;
        normal = mat3(instanceTransform) * normal;
        // Each prop its own outline id (1..255), from where it stands.
        vec3 t = instanceTransform[3].xyz;
        float h = fract(sin(dot(t, vec3(12.9898, 78.233, 37.719))) * 43758.5453);
        fragId = (1.0 + floor(h * 254.0)) / 255.0;
    }
    fragWorldPos = (matModel * local).xyz;
    fragNormal = normalize((matNormal * vec4(normal, 0.0)).xyz);
    fragColor = vertexColor;
    fragKind = vertexTexCoord.y;
    fragLoc = vertexTexCoord2;
    gl_Position = mvp * local;
}
