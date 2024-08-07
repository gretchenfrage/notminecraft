#version 450

layout(location=0) out vec2 o_pos;

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
    case 0: pos = vec2(0, 0); break;
    case 1: pos = vec2(1, 0); break;
    case 2: pos = vec2(1, 1); break;
    case 3: pos = vec2(0, 1); break;
    }

    pos.x = pos.x * 2 - 1;
    pos.y = pos.y * 2 - 1;

    // the fix matrix
    // to convert from our coordinate system, in which:
    // - <0, 0> = top left
    // - <1, 1> = bottom right
    // to vulkan's coordinate system, in which:
    // - <-1, -1> = bottom left
    // - <1, 1> = top right
    //mat3 fix = mat3(
    //    2, 0, 0,
    //    0, -2, 0,
    //    -1, 1, 1
    //); // TODO move into instr compiler?
    //gl_Position = vec4(fix * vec3(pos, 1), 1);
    //gl_Position = vec4(fix * vec3(o_pos.xy, 1), o_pos.z, 1);
    gl_Position = vec4(pos, 0, 1);
    o_pos = pos;
}
