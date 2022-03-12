#version 450

layout(set=0, binding=0) uniform u {
    mat3 u_transform;
    vec4 u_color;
};

layout(location=0) out vec4 o_color;

void main() {
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
        o_color = vec4(0, 0, 0, 1);
        break;
    case 1:
        pos = vec2(1, 0);
        o_color = vec4(1, 0, 0, 1);
        break;
    case 2:
        pos = vec2(1, 1);
        o_color = vec4(0, 1, 0, 1);
        break;
    case 3:
        pos = vec2(0, 1);
        o_color = vec4(0, 0, 1, 1);
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
    );

    gl_Position = vec4(fix * u_transform * vec3(pos, 1), 1);
    o_color = u_color * o_color;
}
