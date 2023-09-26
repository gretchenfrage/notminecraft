#version 450

#include "common_vert.glsl"

layout(set=COMN_UNIS + 0, binding=0) uniform u2 {
    int u_tex_index;
    vec2 u_tex_start;
    vec2 u_tex_extent;
};

layout(location=COMN_OUTS + 0) out vec3 o_tex;

void main() {
    vec3 pos = unit_square_pos();
    set_pos(pos);
    gl_Position = o_pos;

    o_tex = vec3(u_tex_start + u_tex_extent * pos.xy, u_tex_index);
}
