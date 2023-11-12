#version 450

#include "common_frag.glsl"

layout(set=COMN_UNIS + 1, binding=0) uniform texture2DArray u_texture;
layout(set=COMN_UNIS + 1, binding=1) uniform sampler u_sampler;

layout(location=COMN_INS + 0) in vec3 i_tex;

void main() {    
    // texture index rounding fix
    vec3 tex = i_tex;
    if (mod(tex.z, 1) > 0.5) {
        tex.z += 0.5;
    }

    o_color = texture(sampler2DArray(u_texture, u_sampler), tex) * u_color;

    apply_fog();
    apply_clipping();
}
