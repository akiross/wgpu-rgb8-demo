#version 450

out gl_PerVertex {
    vec4 gl_Position;
};

layout(location = 0) out vec2 v_TexCoord;

void main() {
    float d = 1.0;
    vec2 position = vec2(gl_VertexIndex & 1, gl_VertexIndex / 2);
    v_TexCoord = vec2(position.x, 1.0 - position.y);
    gl_Position = vec4(2.0 * d * position - d, 0.0, 1.0);
}
