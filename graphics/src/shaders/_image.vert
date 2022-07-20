#version 450

layout(set=0, binding=0) uniform ustruct_canvas2d {
    mat3 u_transform;
    vec4 u_color;
    float u_clip_min_x;
    float u_clip_max_x;
    float u_clip_min_y;
    float u_clip_max_y;
};
layout(set=1, binding=0) uniform ustruct_image {
    vec2 u_tex_start;
    vec2 u_tex_extent;
};

layout(location=0) out vec2 o_pos;
layout(location=1) out vec2 o_tex;

void main() {
    int corner;
    switch (gl_VertexIndex) {
    case 0: corner = 0; break;
    case 1: corner = 2; break;
    case 2: corner = 1; break;
    case 3: corner = 0; break;
    case 4: corner = 3; break;
    case 5: corner = 2; break;
    }

    vec2 pos;
    switch (corner) {
    case 0:
        pos = vec2(0, 0);
        break;
    case 1:
        pos = vec2(1, 0);
        break;
    case 2:
        pos = vec2(1, 1);
        break;
    case 3:
        pos = vec2(0, 1);
        break;
    }
    o_tex = u_tex_start + u_tex_extent * pos;
    o_pos = (u_transform * vec3(pos, 1)).xy;

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
    gl_Position = vec4(fix * vec3(o_pos, 1), 1);
}
