#version 450

#include "common_vert.glsl"

void main() {
    set_pos(vec3(gl_VertexIndex, 0, 0));
    gl_Position = o_pos;
}
