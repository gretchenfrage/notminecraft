#version 450


layout(set=1, binding=0) uniform texture2D u_texture;
layout(set=1, binding=1) uniform sampler u_sampler;

layout(location=0) in float i_sign;
layout(location=1) in float i_z;

layout(location=0) out vec4 o_color;

// TODO hmmm can we, like, do this?

void main() {
    float z = texture(sampler2D(u_texture, u_sampler), i_tex).r;
    z = i_sign * max(i_sign * z, i_sign * i_z);
    o_color = vec4(z, 0, 0, 1);
}
