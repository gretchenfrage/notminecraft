#version 450

layout(set=0, binding=0) uniform u1 {
    mat4 u_transform;
    vec4 u_color;
    mat4 u_screen_to_world;
    float u_fog_mul;
    float u_fog_add;
    float u_day_night_time;
};
layout(set=3, binding=0) uniform u2 {
    int u_tex_index;
    vec2 u_tex_start;
    vec2 u_tex_extent;
};

layout(location=0) out vec4 o_pos;
layout(location=1) out vec3 o_tex;

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
    case 0: pos = vec2(0, 0); break;
    case 1: pos = vec2(1, 0); break;
    case 2: pos = vec2(1, 1); break;
    case 3: pos = vec2(0, 1); break;
    }

    o_pos = (u_transform * vec4(pos, 0, 1));
    gl_Position = o_pos;
    o_tex = vec3(u_tex_start + u_tex_extent * pos, u_tex_index);
}
