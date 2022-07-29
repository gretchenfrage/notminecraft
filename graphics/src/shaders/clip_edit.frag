#version 450


layout(set=0, binding=0) uniform u {
    float u_sign;
    vec4 u_clip;
};

//layout(set=1, binding=0) uniform texture2D u_texture;
//layout(set=1, binding=1) uniform sampler u_sampler;

//layout(location=0) in float i_sign;
//layout(location=1) in float i_z;
//layout(location=2) in vec2 i_pos;

layout(location=0) in vec2 i_pos;

layout(location=0) inout vec4 o_color;

// TODO hmmm can we, like, do this?

void main() {
    //float z = texture(sampler2D(u_texture, u_sampler), i_pos).r;
    //float z = o_color.r;
    //z = i_sign * max(i_sign * z, i_sign * i_z);
    //o_color = vec4(z, 0, 0, 1);

    float incumbent = o_color.r;
    float candidate = -(
        u_clip.x * i_pos.x
        + u_clip.y * i_pos.y
        + u_clip.w
    ) / u_clip.z;
    float winner = u_sign * max(incumbent * u_sign, candidate * u_sign);
    //o_color = vec4(clamp(winner, 0, 1), 0, 0, 1);
    o_color = vec4(winner < 0 ? 1 : 0, 0, 0, 1);
}
