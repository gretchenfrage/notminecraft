#version 450

layout(set=0, binding=0) uniform u1 {
    mat4 u_transform;
    vec4 u_color;
    mat4 u_screen_to_world;
    float u_fog_mul;
    float u_fog_add;
    float u_day_night_time;
};

layout(set=1, binding=0) uniform texture2D u_clip_min_texture;
layout(set=1, binding=1) uniform sampler u_clip_min_sampler;

layout(set=3, binding=1) uniform texture2D u_drawn_texture;
layout(set=3, binding=2) uniform sampler u_drawn_sampler;

layout(set=2, binding=0) uniform texture2D u_clip_max_texture;
layout(set=2, binding=1) uniform sampler u_clip_max_sampler; // TODO: dedupe samplers?

layout(set=4, binding=0) uniform texture2DArray u_texture;
layout(set=4, binding=1) uniform sampler u_sampler;

layout(location=0) in vec4 i_pos;
layout(location=1) in vec3 i_tex;

layout(location=0) out vec4 o_color;

void main() {
    // texture index rounding fix
    vec3 tex = i_tex;
    if (mod(tex.z, 1) > 0.5) {
        tex.z += 0.5;
    }

    vec2 clip_drawn_uv = vec2(
        i_pos.x / 2 + 0.5,
        i_pos.y / -2 + 0.5
    );

    vec4 drawn = texture(sampler2D(u_drawn_texture, u_drawn_sampler), clip_drawn_uv);
    vec4 inverted = vec4(vec3(1) - drawn.xyz, 1);
    o_color = inverted * texture(sampler2DArray(u_texture, u_sampler), tex) * u_color;

    vec4 pos = i_pos / i_pos.w;
    vec2 clip_uv = vec2(
        pos.x / 2 + 0.5,
        pos.y / -2 + 0.5
    );
    float min_z = texture(sampler2D(u_clip_min_texture, u_clip_min_sampler), clip_uv).r;
    float max_z = texture(sampler2D(u_clip_max_texture, u_clip_max_sampler), clip_uv).r;
    if (pos.z < min_z) {
        discard;
    }
    if (pos.z > max_z) {
        discard;
    }
}
