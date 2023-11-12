#version 450

#include "common_frag.glsl"

layout(set=COMN_UNIS + 0, binding=1) uniform texture2D u_drawn_texture;
layout(set=COMN_UNIS + 0, binding=2) uniform sampler u_drawn_sampler;

layout(set=COMN_UNIS + 1, binding=0) uniform texture2DArray u_texture;
layout(set=COMN_UNIS + 1, binding=1) uniform sampler u_sampler;

layout(location=COMN_INS + 0) in vec3 i_tex;

void main() {
    // texture index rounding fix
    vec3 tex = i_tex;
    if (mod(tex.z, 1) > 0.5) {
        tex.z += 0.5;
    }

    vec2 clip_drawn_uv = vec2(
        i_pos.x / 2 + 0.5,
        i_pos.y / -2 + 0.5
    );

    vec4 drawn = texture(sampler2D(u_drawn_texture, u_drawn_sampler), clip_drawn_uv);
    vec4 inverted = vec4(vec3(1) - drawn.xyz, 1);
    o_color = inverted * texture(sampler2DArray(u_texture, u_sampler), tex) * u_color;

    apply_fog();
    apply_clipping();
}
