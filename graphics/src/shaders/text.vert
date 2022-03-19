#version 450

layout(location=0) in vec2 i_pos;
layout(location=1) in vec2 i_tex;

layout(location=0) out vec2 o_tex;

void main() {
    // the fix matrix
    // to convert from our coordinate system, in which:
    // - <0, 0> = top left
    // - <1, 1> = bottom right
    // to vulkan's coordinate system, in which:
    // - <-1, -1> = bottom left
    // - <1, 1> = top right
    mat3 fix = mat3(
        2, 0, 0,
        0, -2, 0,
        -1, 1, 1
    );
    gl_Position = vec4(fix * vec3(i_pos, 1), 1);

    o_tex = i_tex;
}
