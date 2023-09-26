#version 450

#include "common_frag.glsl"

void main() {
    o_color = u_color;

    apply_fog();
    apply_clipping();
}
