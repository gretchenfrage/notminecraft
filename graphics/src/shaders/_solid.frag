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

layout(location=0) out vec4 o_color;

void main() {
    o_color = u_color;
    if (i_pos.x < u_clip_min_x) {
        discard;
    }
    if (i_pos.x > u_clip_max_x) {
        discard;
    }
    if (i_pos.y < u_clip_min_y) {
        discard;
    }
    if (i_pos.y > u_clip_max_y) {
        discard;
    }
}
