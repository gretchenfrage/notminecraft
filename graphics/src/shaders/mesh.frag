#version 450

layout(set=1, binding=0) uniform texture2D u_clip_min_texture;
layout(set=1, binding=1) uniform sampler u_clip_min_sampler;

layout(set=2, binding=0) uniform texture2D u_clip_max_texture;
layout(set=2, binding=1) uniform sampler u_clip_max_sampler; // TODO: dedupe samplers?

layout(set=3, binding=0) uniform texture2DArray u_texture;
layout(set=3, binding=1) uniform sampler u_sampler;

layout(location=0) in vec4 i_pos;
layout(location=1) in vec4 i_pos_3d;
layout(location=2) in vec3 i_tex;
layout(location=3) in vec4 i_color;

layout(location=0) out vec4 o_color;

const mat4 fog_mat = mat4(
    1.0 / 20.0, 0.0, 0.0, -1.0,
    0.0, 1.0 / 20.0, 0.0, -1.0,
    0.0, 0.0, 0.0, 0.0,
    0.0, 0.0, 0.0, 1.0
);
const float fog_min = 0.0;
const float fog_max = 1.0;

void main() {

    // texture index rounding fix
    vec3 tex = i_tex;
    if (mod(tex.z, 1) > 0.5) {
        tex.z += 0.5;
    }

    vec4 tex_color = texture(sampler2DArray(u_texture, u_sampler), tex);
    //float fog = clamp(length(fog_mat * i_pos), fog_min, fog_max);
    //float fog = clamp(length(i_pos) / 20.0, 0.0, 1.0);
    //float fog = clamp(i_pos.z / i_pos.w, 0.0, 1.0);
    vec4 pos_3d = i_pos_3d / i_pos_3d.w;
    vec4 fog_color = vec4(0.45, 0.62, 1.0, 1.0);
    float fog = clamp((length(pos_3d.xz) - 100.0) / 100.0, 0.0, 1.0);
    o_color = mix(tex_color * i_color, fog_color, fog);

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
    if (tex_color.a == 0) {
        discard;
    }
}
