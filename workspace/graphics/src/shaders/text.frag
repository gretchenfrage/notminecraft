#version 450

#include "common_frag.glsl"

layout(set=COMN_UNIS + 0, binding=0) uniform texture2D u_texture;
layout(set=COMN_UNIS + 0, binding=1) uniform sampler u_sampler;

layout(location=COMN_INS + 0) in vec2 i_tex;
layout(location=COMN_INS + 1) in vec4 i_color;

void main() {
    float a = texture(sampler2D(u_texture, u_sampler), i_tex).r;
    o_color = vec4(1, 1, 1, a) * i_color * u_color;

    // finish
    apply_fog();
    apply_clipping();
}


    