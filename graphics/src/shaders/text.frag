#version 450

layout(set=1, binding=0) uniform texture2D u_clip_min_texture;
layout(set=1, binding=1) uniform sampler u_clip_min_sampler;

layout(set=2, binding=0) uniform texture2D u_clip_max_texture;
layout(set=2, binding=1) uniform sampler u_clip_max_sampler; // TODO: dedupe samplers?

layout(set=3, binding=0) uniform texture2D u_texture;
layout(set=3, binding=1) uniform sampler u_sampler;

layout(location=0) in vec4 i_pos;
layout(location=1) in vec2 i_tex;
layout(location=2) in vec4 i_color;

layout(location=0) out vec4 o_color;

void main() {
    float a = texture(sampler2D(u_texture, u_sampler), i_tex).r;
    o_color = vec4(1, 1, 1, a) * i_color;

    vec2 clip_uv = vec2(
        i_pos.x / 2 + 0.5,
        i_pos.y / -2 + 0.5
    );
    float min_z = texture(sampler2D(u_clip_min_texture, u_clip_min_sampler), clip_uv).r;
    float max_z = texture(sampler2D(u_clip_max_texture, u_clip_max_sampler), clip_uv).r;
    if (i_pos.z < min_z) {
        discard;
    }
    if (i_pos.z > max_z) {
        discard;
    }
}
