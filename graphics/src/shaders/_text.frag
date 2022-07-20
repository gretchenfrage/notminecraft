#version 450

layout(set=0, binding=0) uniform u {
    mat3 u_transform;
    vec4 u_color;
    float u_clip_min_x;
    float u_clip_max_x;
    float u_clip_min_y;
    float u_clip_max_y;
};

layout(set=1, binding=0) uniform texture2D u_texture;
layout(set=1, binding=1) uniform sampler u_sampler;

layout(location=0) in vec2 i_pos;
layout(location=1) in vec2 i_tex;
layout(location=2) in vec4 i_color;

layout(location=0) out vec4 o_color;

void main() {
    float v = texture(sampler2D(u_texture, u_sampler), i_tex).r;
    o_color = vec4(1, 1, 1, v) * i_color;
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
