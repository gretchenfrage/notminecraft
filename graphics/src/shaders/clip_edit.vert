#version 450

layout(set=0, binding=0) uniform u {
    float u_sign;
    vec3 u_affine; // TODO: pad name
};

layout(location=0) out float o_sign;
layout(location=1) out float o_z;
layout(location=2) out vec2 o_pos;

void main() {
    o_sign = u_sign; // TODO: why?

    int corner;
    switch (gl_VertexIndex) {
    case 0: corner = 0; break;
    case 1: corner = 2; break;
    case 2: corner = 1; break;
    case 3: corner = 0; break;
    case 4: corner = 3; break;
    case 5: corner = 2; break;
    }

    vec2 pos;
    switch (corner) {
    case 0:
        pos = vec2(0, 0);
        break;
    case 1:
        pos = vec2(1, 0);
        break;
    case 2:
        pos = vec2(1, 1);
        break;
    case 3:
        pos = vec2(0, 1);
        break;
    }

    // the fix matrix
    // to convert from our coordinate system, in which:
    // - <0, 0> = top left
    // - <1, 1> = bottom right
    // to vulkan's coordinate system, in which:
    // - <-1, -1> = bottom left
    // - <1, 1> = top right
    mat3 fix = mat3(
        2, 0, 0,
        0, -2, 0,
        -1, 1, 1
    ); // TODO move into instr compiler?
    gl_Position = vec4(fix * vec3(pos, 1), 1);

    //o_z = dot(pos, u_affine);
    o_z = dot(vec3(pos, 1), u_affine);

    //// TODO use instr compiler to simplify into dot oroduct?
    ////-(ax+by+d)/c
    //o_z = -(u_clip.x * pos.x + u_clip.y * pos.y + u_clip.w) / u_clip.z;

    o_pos = pos; // TODO bleh
}
