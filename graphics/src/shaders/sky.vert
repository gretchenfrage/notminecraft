#version 450

#include "common_vert.glsl"

void main() {
    set_pos(unit_square_pos());
    gl_Position = o_pos;
}
