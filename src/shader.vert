#version 450

out gl_PerVertex {
    vec4 gl_Position;
};

layout(location = 0) out vec2 v_TexCoord;

void main() {
    float d = 0.9;
    /*
    vec2 position;
    switch (gl_VertexIndex) {
        case 0: {
            v_TexCoord = vec2(0, 0);
            gl_Position = vec4(-1.0, -1.0, 0.0, 1.0);
            break;
        }
        case 1: {
            v_TexCoord = vec2(1, 0);
            gl_Position = vec4(+1.0, -1.0, 0.0, 1.0);
            break;
        }
        case 2: {
            v_TexCoord = vec2(0, 1);
            gl_Position = vec4(-1.0, +1.0, 0.0, 1.0);
            break;
        }
        case 3: {
            v_TexCoord = vec2(1, 1);
            gl_Position = vec4(+1.0, +1.0, 0.0, 1.0);
            break;
        }
        default: position = vec2(0.5, 0.5); break;
    }
    */
    vec2 position = vec2(gl_VertexIndex & 1, gl_VertexIndex / 2);
    v_TexCoord = position;
    gl_Position = vec4(2.0 * d * position - d, 0.0, 1.0);
}
