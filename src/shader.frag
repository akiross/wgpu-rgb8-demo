#version 450

layout(location = 0) in vec2 v_TexCoord;
layout(location = 0) out vec4 outColor;

layout(set = 0, binding = 0) uniform texture2D t_Color;
layout(set = 0, binding = 1) uniform sampler s_Color;

void main() {
    outColor = vec4(v_TexCoord.x, v_TexCoord.y, 0.0, 1.0);// * texture(sampler2D(t_Color, s_Color), v_TexCoord) * 0.000001;
}
