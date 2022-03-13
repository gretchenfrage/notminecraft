#version 450

layout(set=0, binding=0) uniform u {
    mat3 u_transform;
    vec4 u_color;
    float u_clip_min_x;
    float u_clip_max_x;
    float u_clip_min_y;
    float u_clip_max_y;
};

layout(location=0) out vec2 o_pos;
//layout(location=1) out vec4 o_color;

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
        o_pos = vec2(0, 0);
        //o_color = vec4(0, 0, 0, 1);
        break;
    case 1:
        o_pos = vec2(1, 0);
        //o_color = vec4(1, 0, 0, 1);
        break;
    case 2:
        o_pos = vec2(1, 1);
        //o_color = vec4(0, 1, 0, 1);
        break;
    case 3:
        o_pos = vec2(0, 1);
        //o_color = vec4(0, 0, 1, 1);
        break;
    }
    o_pos = (u_transform * vec3(o_pos, 1)).xy;
    //o_color = u_color * o_color;

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
