#version 450

#include "common_frag.glsl"

layout(set=COMN_UNIS + 0, binding=0) uniform texture2DArray u_texture;
layout(set=COMN_UNIS + 0, binding=1) uniform sampler u_sampler;

layout(location=COMN_INS + 0) in vec3 i_tex;
layout(location=COMN_INS + 1) in vec4 i_color;

void main() {
    // texture index rounding fix
    vec3 tex = i_tex;
    if (mod(tex.z, 1) > 0.5) {
        tex.z += 0.5;
    }
    
    // mesh fragment color
    // TODO: proper handling of color with fog
    vec4 tex_color = texture(sampler2DArray(u_texture, u_sampler), tex);
    o_color = tex_color * i_color * u_color;

    // debug day night lighting
    float day = compute_day();
    o_color = vec4(o_color.xyz * clamp(day, 0.3, 1), o_color.w);

    // finish
    apply_fog();
    apply_clipping();
}
