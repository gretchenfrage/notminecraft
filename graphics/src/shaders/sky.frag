#version 450

#include "common_frag.glsl"

void main() {
    // special way to compute view dir
    vec4 a = u_screen_to_world * vec4(i_pos.xy, 1, i_pos.w);
    vec4 b = u_screen_to_world * vec4(i_pos.xy, 0, i_pos.w);
    vec3 view_dir = normalize((a.xyz / a.w) - (b.xyz / b.w));

    // compute color = fog color
    o_color = vec4(compute_fog_color(view_dir), 1);

    // finish
    apply_clipping();
}
