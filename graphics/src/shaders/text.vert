#version 450

layout(set=0, binding=0) uniform u {
    mat4 u_transform;
    vec4 u_color;
};

layout(location=0) in vec2 i_pos;
layout(location=1) in vec2 i_tex;
layout(location=2) in vec4 i_color;

layout(location=0) out vec4 o_pos;
layout(location=1) out vec2 o_tex;
layout(location=2) out vec4 o_color;

void main() {
    o_pos = (u_transform * vec4(i_pos, 0, 1));
    o_tex = i_tex;
    o_color = i_color * u_color;
    gl_Position = o_pos;
}
