#version 330

// Scene geometry. Attribute and matrix names are raylib's defaults so DrawMesh binds them.
in vec3 vertexPosition;
in vec2 vertexTexCoord;
in vec3 vertexNormal;
in vec4 vertexColor;
in vec2 vertexTexCoord2;     // position along a vertical face, face width

uniform mat4 mvp;
uniform mat4 matModel;
uniform mat4 matNormal;

out vec3 fragWorldPos;
out vec3 fragNormal;
out vec4 fragColor;
flat out float fragId;
flat out float fragKind;     // material: 0 tower, 1 plant, 2 floor/deck, 3 bridge, 4 stair, 5 test block
out vec2 fragLoc;

void main()
{
    fragWorldPos = (matModel * vec4(vertexPosition, 1.0)).xyz;
    fragNormal = normalize((matNormal * vec4(vertexNormal, 0.0)).xyz);
    fragColor = vertexColor;
    fragId = vertexTexCoord.x;
    fragKind = vertexTexCoord.y;
    fragLoc = vertexTexCoord2;
    gl_Position = mvp * vec4(vertexPosition, 1.0);
}
