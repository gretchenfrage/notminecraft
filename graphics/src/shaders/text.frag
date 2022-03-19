#version 450

layout(set=0, binding=0) uniform texture2D u_texture;
layout(set=0, binding=1) uniform sampler u_sampler;

layout(location=0) in vec2 i_tex;

layout(location=0) out vec4 o_color;

void main() {
    //o_color = texture(sampler2D(u_texture, u_sampler), i_tex);
    float v = texture(sampler2D(u_texture, u_sampler), i_tex).r;
    o_color = vec4(0, 0, 0, v);
    //o_color = vec4(0, 0, 0, 1);
}
