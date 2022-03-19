#version 450

layout(set=0, binding=0) uniform u {
    mat3 u_transform;
    vec4 u_color;
    float u_clip_min_x;
    float u_clip_max_x;
    float u_clip_min_y;
    float u_clip_max_y;
};

layout(location=0) in vec2 i_pos;
layout(location=1) in vec2 i_tex;
layout(location=2) in vec4 i_color;

layout(location=0) out vec2 o_pos;
layout(location=1) out vec2 o_tex;
layout(location=2) out vec4 o_color;

void main() {
    // the fix matrix
    // to convert from our coordinate system, in which:
    // - <0, 0> = top left
    // - <1, 1> = bottom right
    // to vulkan's coordinate system, in which:
    // - <-1, -1> = bottom left
    // - <1, 1> = top right
    o_pos = (u_transform * vec3(i_pos, 1)).xy;
    mat3 fix = mat3(
        2, 0, 0,
        0, -2, 0,
        -1, 1, 1
    );
    gl_Position = vec4(fix * vec3(o_pos, 1), 1);

    o_tex = i_tex;
    o_color = i_color * u_color;
}
