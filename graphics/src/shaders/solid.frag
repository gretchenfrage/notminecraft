#version 450

layout(set=0, binding=0) uniform u {
    mat3 u_transform;
    vec4 u_color;
    float u_clip_min_x;
};

layout(location=0) in vec2 i_pos;
layout(location=1) in vec4 i_color;

layout(location=0) out vec4 o_color;

void main() {
    if (i_pos.x < u_clip_min_x) {
        discard;
    }
    o_color = i_color;
}
