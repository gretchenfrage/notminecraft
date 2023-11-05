#version 450

layout(set=0, binding=0) uniform texture2D u_texture;
layout(set=0, binding=1) uniform sampler u_sampler;

layout(location=0) in vec2 i_pos;

layout(location=0) out vec4 o_color;

void main() {
    vec2 uv = vec2(
        i_pos.x / 2 + 0.5,
        i_pos.y / -2 + 0.5
    );
    o_color = texture(sampler2D(u_texture, u_sampler), uv);
}
