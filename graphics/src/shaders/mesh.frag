#version 450

layout(set=0, binding=0) uniform u {
    mat4 u_transform;
    vec4 u_color;
    mat4 u_screen_to_world;
};

layout(set=1, binding=0) uniform texture2D u_clip_min_texture;
layout(set=1, binding=1) uniform sampler u_clip_min_sampler;

layout(set=2, binding=0) uniform texture2D u_clip_max_texture;
layout(set=2, binding=1) uniform sampler u_clip_max_sampler; // TODO: dedupe samplers?

layout(set=3, binding=0) uniform texture2DArray u_texture;
layout(set=3, binding=1) uniform sampler u_sampler;

layout(location=0) in vec4 i_pos;
layout(location=1) in vec3 i_tex;
layout(location=2) in vec4 i_color;

layout(location=0) out vec4 o_color;

void main() {
    // texture index rounding fix
    vec3 tex = i_tex;
    if (mod(tex.z, 1) > 0.5) {
        tex.z += 0.5;
    }
    
    vec4 tex_color = texture(sampler2DArray(u_texture, u_sampler), tex);
    o_color = tex_color * i_color;

    vec4 fog_color;
    float fog = 0;

    if (u_screen_to_world != mat4(0)) {
        vec4 a = u_screen_to_world * i_pos;
        vec4 b = u_screen_to_world * vec4(i_pos.xy, 0, i_pos.w);
        vec3 view = (a.xyz / a.w) - (b.xyz / b.w);

        //fog_color = vec4(1, 0, 0, 1);
        fog_color = vec4(abs(normalize(view.xyz)), 1);
        fog = clamp((length(view.xz) - 100) / 100.0, 0.0, 1.0);
    }

    // TODO: for maximum correctness, color must somehow be mixed into fog when 3D scene begins
    o_color = mix(o_color, fog_color, fog);

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
