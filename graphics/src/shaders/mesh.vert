#version 450

#include "common_vert.glsl"

layout(location=COMN_INS + 0) in vec3 i_pos;
layout(location=COMN_INS + 1) in vec3 i_tex;
layout(location=COMN_INS + 2) in vec4 i_color;

layout(location=COMN_OUTS + 0) out vec3 o_tex;
layout(location=COMN_OUTS + 1) out vec4 o_color;

void main() {
    set_pos(i_pos);
    gl_Position = o_pos;
    
    o_tex = i_tex;
    o_color = i_color * u_color;
}
