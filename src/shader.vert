#version 450

out gl_PerVertex {
    vec4 gl_Position;
};

void main() {
    float d = 0.9;
    /*
    vec2 position;
    switch (gl_VertexIndex) {
        case 0: position = vec2(-d, -d); break;
        case 1: position = vec2(+d, -d); break;
        case 2: position = vec2(-d, +d); break;
        case 3: position = vec2(+d, +d); break;
        default: position = vec2(0.5, 0.5); break;
    }
    */
    vec2 position = 2.0 * d * vec2(gl_VertexIndex & 1, gl_VertexIndex / 2) - d;
    gl_Position = vec4(position, 0.0, 1.0);
}
